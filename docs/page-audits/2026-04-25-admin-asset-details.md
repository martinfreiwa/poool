# Page Audit: Admin Asset Details

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/asset-details`
Template: `frontend/platform/admin/asset-details.html`
JavaScript: `frontend/platform/static/js/admin-asset-details.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page styles
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/assets.rs`

---

## Summary

`/admin/asset-details` was partially implemented at audit time. Follow-up fixes on 2026-04-25 replaced the fake-success controls that had backend support gaps, added asset-specific permission checks, added transactional audit logging for supported mutations, returned secured document download links, propagated dependent data query failures, and updated the stale smoke-test selector.

Final status is `needs_recheck`.

Severity counts: P0 0, P1 3, P2 4, P3 1.

Follow-up fix status: all documented findings were addressed in the local working tree; authenticated browser and safe mutation fixture recheck are still recommended.

---

## Tested Scope

- Reviewed `frontend/platform/admin/asset-details.html`.
- Reviewed `frontend/platform/static/js/admin-asset-details.js`.
- Reviewed route registration in `backend/src/admin/mod.rs`.
- Reviewed backend data/mutation handlers in `backend/src/admin/assets.rs`.
- Checked admin permission extractor behavior in `backend/src/admin/extractors.rs`.
- Checked relevant schema support in `database/001_initial_schema.sql`, `database/full_migration.sql`, and `docs/DATABASE_SCHEMA.md`.
- Checked existing admin E2E smoke coverage in `tests/admin/test_admin_dashboard.py`.
- Runtime smoke used unauthenticated curl only; no destructive asset mutations were submitted.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/asset-details` | Registered page route behind `AdminUser`. |
| URL alias | `/admin/asset-details.html` | Registered alias. |
| Template | `frontend/platform/admin/asset-details.html` | Static admin shell, tabs, settings, danger zone. |
| JS | `frontend/platform/static/js/admin-asset-details.js` | Fetches detail JSON and wires tabs/actions. |
| Shared JS | `user-data.js`, `admin-permission-guard.js`, `admin-global-search.js`, `admin-theme.js`, `poool-dropdown*.js` | Loaded by page. |
| CSS | `admin.css`, `bundle.css`, `fonts.css`, `poool-dropdown.css`, inline styles | Admin visual system plus page-specific inline CSS. |
| Backend page route | `GET /admin/asset-details`, `GET /admin/asset-details.html` | `page_admin_generic`. |
| Backend API route | `GET /api/admin/assets/:asset_id/detail` | Returns asset, investors, financials, docs, images, milestones, orders. |
| Backend API route | `POST /api/admin/assets/:asset_id/toggle-featured` | Real mutation, but generic admin only. |
| Missing backend routes | publish toggle, funding status save, freeze trading, unpublish, archive | UI exposes these actions but no route is registered. |
| Database tables | `assets`, `asset_images`, `asset_documents`, `asset_milestones`, `asset_financials`, `investments`, `orders`, `order_items`, `users`, `user_profiles` | Required tables exist. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `a[href="/admin/"]`, `a[href="/admin/assets"]` | Navigate back to admin and assets list. | Yes, link navigation. | Yes, page routes exist. | Not authenticated-tested; routes are registered. |
| Refresh | `#btn-refresh` | Reload current asset detail data. | Yes, calls `loadAsset()`. | Yes, `GET /api/admin/assets/:id/detail`. | Syntax passed; unauth API requires auth. |
| Loading state | `#loading-overlay` | Show while detail API loads or show error. | Yes. | N/A | Unauthenticated API returns 401; UI would show HTTP error. |
| Asset header | `#asset-title-main`, `#asset-location`, badges, stats | Display asset title, location, funding, valuation, token price, yield, type. | Yes. | Mostly yes. | Static route/API review only. |
| Funding bar | `#funding-label`, `#funding-pct`, `#funding-bar` | Display sold tokens and percent funded. | Yes. | Yes. | Not data-fixture tested. |
| Tabs | `.asset-tab[data-tab]` | Switch visible tab panel. | Yes, click delegation. | N/A | Syntax passed. |
| Overview | `#asset-description`, `#property-details`, `#financial-summary`, `#quick-stats` | Render read-only asset metadata and counts. | Yes. | Yes. | Static review passed. |
| Media gallery | `#media-grid`, `#video-link` | Render images and video tour link. | Yes. | Images yes; video URL from assets. | URL handling needs frontend hardening. |
| Documents | `#documents-list` | Render data room document links. | Partially. | Partially; API omits file URL/title. | Broken: links fall back to `#`. |
| Financials table | `#financials-tbody` | Render monthly performance. | Yes. | Yes. | Backend silently hides query failures. |
| Milestones | `#milestones-list` | Render roadmap items. | Yes. | Yes. | Backend silently hides query failures. |
| Cap table | `#captable-tbody` | Render investors and link user details. | Yes. | Yes. | Exposes personal/financial admin data under generic admin role. |
| Orders table | `#orders-tbody` | Render orders for this asset. | Yes. | Yes. | Exposes order/user data under generic admin role. |
| Featured toggle | `#toggle-featured` | Persist landing-page featured state. | Yes. | Yes, `POST /api/admin/assets/:id/toggle-featured`. | Real route exists; lacks fine-grained permission/audit. |
| Published toggle | `#toggle-published` | Persist marketplace visibility. | Fake only. | No. | Broken: toggles locally and shows pending toast. |
| Funding status select | `#select-funding-status` | Persist funding status. | No change handler. | No route from this page. | Dead UI. |
| Freeze trading | `#btn-freeze` | Freeze trading after confirmation. | Fake only. | No. | Broken: success toast only. |
| Unpublish from marketplace | `#btn-unpublish` | Persist unpublish after confirmation. | Fake only. | No. | Broken: success toast only. |
| Archive asset | `#btn-archive` | Persist archive after confirmation. | Fake only. | No. | Broken: success toast only. |

---

## Frontend Findings

### P1 - Settings and danger controls show fake success

Fix status: fixed in local working tree. Published and unpublish now use `PATCH /api/admin/assets/:asset_id/publication`; freeze and archive are disabled because there is no schema-backed workflow yet.

Location:

- Template: `frontend/platform/admin/asset-details.html:668`
- JS: `frontend/platform/static/js/admin-asset-details.js:408`, `frontend/platform/static/js/admin-asset-details.js:414`

Problem:

The Published toggle, Freeze Trading, Unpublish, and Archive controls do not call a backend API. `togglePublished()` toggles local CSS and shows "Published status toggled (save pending)", while `dangerAction()` confirms and then shows a success-like toast without mutation or failure state.

Expected:

Either remove/disable these controls until product and backend contracts exist, or wire them to authenticated, permission-checked, audited backend routes with explicit success/error states.

Evidence:

No registered Axum routes exist for publish toggle, freeze trading, archive, or funding status update in `backend/src/admin/mod.rs`. The JS contains placeholder comments and toast-only behavior.

Recommended fix:

Add real routes such as `PATCH /api/admin/assets/:id/publication`, `PATCH /api/admin/assets/:id/funding-status`, `POST /api/admin/assets/:id/freeze-trading`, and `POST /api/admin/assets/:id/archive`, or mark the controls disabled with explanatory copy.

### P1 - Document links cannot open actual data room files

Fix status: fixed in local working tree. The detail API now returns document `id`, `title`, `file_size`, and `url` pointing at the existing secured `/api/documents/:id/download` route.

Location:

- JS: `frontend/platform/static/js/admin-asset-details.js:248`
- Backend: `backend/src/admin/assets.rs:133`, `backend/src/admin/assets.rs:197`

Problem:

The document renderer expects `d.url`, but the backend only returns `document_type` and `file_size`. Every View link falls back to `#`, so the Data Room looks available while no document can be opened.

Expected:

The API should return safe document identifiers, titles, and signed or proxied document URLs, or the UI should render disabled document rows when links are unavailable.

Evidence:

The SQL selects only `document_type, file_size_bytes` from `asset_documents`, and JSON maps only `document_type` plus `file_size`.

Recommended fix:

Return `title`, `file_url` via a safe signed/proxy access path, and file metadata. Avoid exposing raw private GCS paths if documents are sensitive.

### P2 - Funding status select is dead UI

Fix status: fixed in local working tree. The select now uses database-valid status values and persists through `PATCH /api/admin/assets/:asset_id/funding-status`.

Location:

- Template: `frontend/platform/admin/asset-details.html:712`
- JS: `frontend/platform/static/js/admin-asset-details.js:383`

Problem:

The select is populated from `a.funding_status`, but no `change` listener or save button exists. Admin users can change it visually, then navigate away with no warning and no persistence.

Expected:

The control should either save through a backend route with validation and audit logging, or be disabled/read-only.

Evidence:

`renderSettings()` sets `fundingSelect.value`; no code observes changes to `#select-funding-status`.

Recommended fix:

Add an explicit Save Settings action with disabled/loading/error states, or remove editable affordance.

### P2 - Failed featured toggle requests are silent and optimistic state is incomplete

Fix status: fixed in local working tree. The toggle now disables in-flight, surfaces non-2xx/network errors, and syncs state from the backend response.

Location:

- JS: `frontend/platform/static/js/admin-asset-details.js:394`

Problem:

If `POST /api/admin/assets/:id/toggle-featured` returns a non-2xx response, the UI does nothing. Network errors are logged only to console. The control is not disabled in-flight, so rapid clicks can submit duplicate toggles and end in an unexpected final state.

Expected:

Non-2xx responses should show an error toast, prevent duplicate submits while in-flight, and reload authoritative asset state after mutation.

Evidence:

The handler checks `if (resp.ok)` and has no `else` branch. The catch only calls `console.error(e)`.

Recommended fix:

Track pending state, disable the toggle during the request, parse error JSON, show user-visible errors, and call `loadAsset()` after success.

### P2 - Asset image URL is inserted into inline CSS without CSS-safe escaping

Fix status: fixed in local working tree. Media cards are now built with DOM APIs and `style.backgroundImage` after URL validation.

Location:

- JS: `frontend/platform/static/js/admin-asset-details.js:224`

Problem:

Image URLs are inserted into `style="background-image:url('...')"` using HTML escaping, not CSS escaping. Admin/GCS-sourced URLs are usually controlled, but a quote or crafted value could break the style attribute.

Expected:

Use DOM APIs and assign `element.style.backgroundImage = "url(...)"` after validating or encoding the URL, or render `<img>` elements with `src` assigned via DOM property.

Evidence:

`esc(img.url)` escapes HTML but does not make a string safe inside a CSS `url('...')` context.

Recommended fix:

Build media cards with `document.createElement()` and `style.backgroundImage = \`url("${safeUrl}")\`` after URL validation, or use real image elements.

---

## Backend Findings

### P1 - Asset detail and feature mutation lack fine-grained permissions and audit logging

Fix status: fixed in local working tree. Asset list/detail require `assets.view`; featured/publication require `assets.publish`; funding status requires `assets.edit`; supported mutations write `audit_logs` in the same DB transaction. Migration `database/080_admin_asset_permissions.sql` grants these permissions to `admin` and `super_admin`.

Location:

- Backend: `backend/src/admin/assets.rs:85`, `backend/src/admin/assets.rs:58`
- Extractor: `backend/src/admin/extractors.rs:167`

Problem:

The detail route exposes cap table investors, user links, orders, and financials to any generic admin/super_admin session. The featured mutation also accepts generic admin only and does not write an audit log.

Expected:

Sensitive read routes should enforce `assets.view` or equivalent. Mutations should enforce `assets.publish` or `assets.edit`, write durable audit logs, and include actor, target, previous value, new value, and request metadata.

Evidence:

`AdminUser` supports `require_permission()`, and `admin/access.rs` defines asset permission names, but `api_admin_asset_detail()` and `api_admin_toggle_featured()` never call it.

Recommended fix:

Call `admin.require_permission(&state.db, "assets.view").await?` for detail reads and `assets.publish` or `assets.edit` for feature/publication changes. Add audit logging in the same transaction for mutations.

### P2 - Detail API silently hides dependent data failures

Fix status: fixed in local working tree. Dependent queries now propagate database errors instead of returning empty fallback sections.

Location:

- Backend: `backend/src/admin/assets.rs:116`

Problem:

Investor, financial, document, image, milestone, and order subqueries use `unwrap_or_default()`. If one of those queries fails, the API still returns 200 with an empty section, making operators think the asset has no related records.

Expected:

Admin pages should distinguish "no records" from "could not load records", especially for cap table, financial, and order data.

Evidence:

Every secondary query in `api_admin_asset_detail()` defaults to an empty vector on error.

Recommended fix:

Propagate query failures as a structured 5xx response, or return per-section error metadata that the UI can show clearly.

### P3 - Existing admin page test selector is stale

Fix status: fixed in local working tree. The smoke test now checks `asset-content`, which exists in the template.

Location:

- Test: `tests/admin/test_admin_dashboard.py:523`
- Template: `frontend/platform/admin/asset-details.html:326`

Problem:

The broad admin dashboard smoke test calls `/admin/asset-details.html?id=1` and looks for `asset-sc-details`, but the template does not contain that selector.

Expected:

The test should use a real UUID fixture and assert stable selectors such as `#asset-content`, `#loading-overlay`, or `#asset-tabs`, then validate the detail API behavior separately.

Evidence:

`rg` found no `asset-sc-details` selector in the admin asset detail template or JS.

Recommended fix:

Update the test fixture and selector, and add contract coverage for the JSON response shape.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-asset-details.js` | No syntax errors. | Passed with no output. | Pass |
| Unauthenticated page guard | `curl -I 'http://localhost:8888/admin/asset-details?id=00000000-0000-0000-0000-000000000000'` | 401/redirect/login guard. | Returned `401 Unauthorized` from local server. | Pass |
| Unauthenticated API guard | `curl -I http://localhost:8888/api/admin/assets/00000000-0000-0000-0000-000000000000/detail` | 401 unauthorized. | Returned `401 Unauthorized` with security headers. | Pass |
| Authenticated page render | Open page with admin session and real asset ID. | Page loads, fetch succeeds, tabs render data. | Not run; no authenticated admin browser session was used in this documentation-only run. | Not run |
| Featured toggle mutation | Click Featured on a safe fixture. | Permission checked, persisted, audit logged, UI reloads. | Not run; destructive mutation intentionally skipped. Static review found permission/audit gaps. | Not run |
| Danger actions | Click Freeze/Unpublish/Archive on a safe fixture. | Persisted or clearly disabled. | Not run. Static review shows toast-only behavior. | Fail by static review |
| Mobile/keyboard smoke | Use mobile viewport and keyboard through tabs/toggles. | No overlap; keyboard operable. | Not run. Static review found div toggles without keyboard semantics. | Needs recheck |

---

## Automated Test Coverage

Existing coverage is weak for this page:

- `tests/admin/test_admin_dashboard.py` includes `/admin/asset-details.html?id=1`, but the expected selector appears stale and the ID is not a UUID-shaped fixture.
- No targeted Rust test was found for `GET /api/admin/assets/:asset_id/detail`.
- No targeted Rust test was found for `POST /api/admin/assets/:asset_id/toggle-featured`.
- No E2E test was found for document link rendering, settings controls, or danger-zone actions.

Recommended tests:

- API contract test for detail JSON shape, including `documents[].url` once implemented.
- Permission tests for generic admin without `assets.view`/`assets.publish`.
- Audit-log test for featured/publish/archive mutations.
- Browser test with a real asset fixture covering all tabs, empty states, and mobile layout.

---

## Security, UX, and Data Integrity Notes

- Money fields use cents in backend/database (`*_cents`) and are formatted client-side only for display.
- No client-side financial mutations were found on this page.
- The only real current mutation (`toggle-featured`) is single-table, but still needs permission and audit logging.
- Data room documents are likely sensitive; direct document URLs should be signed, proxied, or otherwise authorization-checked.
- Toggle controls use `div` elements without button semantics, keyboard handling, `role="switch"`, or `aria-checked`.
- Tabs are real `<button>` elements but do not set ARIA tab roles or keyboard arrow navigation.
- Page-level inline styles are heavy and should eventually move into admin CSS for maintainability.

---

## Recommended Fix Order

1. Disable or remove fake mutation controls until real backend contracts exist.
2. Add asset-specific permission checks and audit logging to detail/mutation routes.
3. Fix document API response and safe document access.
4. Replace silent backend `unwrap_or_default()` section failures with visible errors.
5. Add accessible switch/tab semantics and in-flight/error states.
6. Update stale admin E2E selector and add route/API contract tests.
