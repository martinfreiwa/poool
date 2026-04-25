# Page Audit: Admin Assets

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/assets`
Template: `frontend/platform/admin/assets.html`
JavaScript: `frontend/platform/static/js/admin-assets.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`, `frontend/platform/static/css/poool-dropdown.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/assets.rs`

---

## Summary

`/admin/assets` is implemented as a protected admin page for viewing published assets and toggling the featured flag. The backend list and mutation routes exist and use admin permission checks; the featured mutation is transactional and audit-logged.

The page still needs recheck because the list endpoint can silently convert database failures into an empty asset list, the frontend leaves users in a loading state on API errors or missing `assets.view`, the featured toggle has no visible failure/loading feedback, and sortable/action table controls have accessibility gaps.

---

## Tested Scope

- Reviewed `frontend/platform/admin/assets.html`.
- Reviewed `frontend/platform/static/js/admin-assets.js`.
- Reviewed `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/assets.rs`, and `backend/src/auth/csrf.rs`.
- Reviewed schema/docs for `assets`, asset indexes, and admin asset permissions.
- Checked existing admin API/sorting/security tests referencing `/api/admin/assets`.
- Ran JS syntax check and unauthenticated HTTP smoke checks against the local server on `localhost:8888`.
- Did not submit mutating featured toggles because no authenticated safe admin fixture was available in this audit run.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/assets` | Clean route registered through generic admin page renderer. |
| Route alias | `/admin/assets.html` | Registered in `backend/src/admin/mod.rs`. |
| Template | `frontend/platform/admin/assets.html` | KPI cards, filters, sortable table, pagination. |
| JS | `frontend/platform/static/js/admin-assets.js` | Loads assets, filters/sorts/paginates, toggles featured. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF headers to mutating admin fetches and hides nav by permissions. |
| Backend page route | `GET /admin/assets` | `page_admin_generic`; requires `AdminUser`, not `assets.view`. |
| Backend API route | `GET /api/admin/assets` | Requires `assets.view`; returns `{ assets: [...] }`. |
| Backend API route | `POST /api/admin/assets/:asset_id/toggle-featured` | Requires `assets.publish`; row lock + transaction + audit log. |
| Related API route | `GET /api/admin/assets/:asset_id/detail` | Used by asset details page, not by this page. |
| Related API route | `PATCH /api/admin/assets/:asset_id/publication` | Used by asset details page, not by this page. |
| Related API route | `PATCH /api/admin/assets/:asset_id/funding-status` | Used by asset details page, not by this page. |
| Database table | `assets` | Published asset list, featured flag, funding status, cents/BPS fields. |
| Database table | `audit_logs` | Featured mutation audit trail. |
| Database table | `admin_permissions` / roles | `assets.view`, `assets.edit`, `assets.publish`. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin link | `a[href="/admin/"]` | Navigate to admin dashboard. | Link | `GET /admin/` | Not runtime-clicked; route exists. |
| Global search | `#admin-global-search` | Shared admin-wide search. | Shared `admin-global-search.js` | Reads admin APIs including assets. | Not authenticated-runtime tested. |
| KPI cards | `#stat-total`, `#stat-funding`, `#stat-funded`, `#stat-aum`, `#stat-tokens-sold` | Update after asset API load. | Yes, `updateStats()` | `GET /api/admin/assets` | Works by static review when API returns array. |
| Asset search | `#asset-search` | Debounced local filtering by title/city/slug. | Yes | No backend needed after initial load. | Static review only. |
| Type filter | `#filter-type` | Filter by asset type. | Yes | No backend needed after initial load. | Static review only. |
| Status filter | `#filter-status` | Filter by funding status. | Yes | No backend needed after initial load. | Missing `payout_pending` option. |
| Featured checkbox | `#filter-featured` | Filter to featured assets. | Yes | No backend needed after initial load. | Static review only. |
| Asset table headers | `th[data-sort]` | Sort local table. | Mouse click only | No backend needed after initial load. | Keyboard/ARIA gap. |
| Loading row | `#assets-table-body` initial row | Show while API loads. | Initial HTML only | N/A | Can remain forever on API failure. |
| Empty state row | rendered by `renderTable()` | Show no matching filters. | Yes | N/A | Static review only; pagination info not reset. |
| Previous page | `#prev-page` | Page back locally. | Yes | N/A | Static review only. |
| Next page | `#next-page` | Page forward locally. | Yes | N/A | Static review only. |
| Featured toggle | rendered `button onclick="toggleFeatured(...)"` | Toggle asset featured flag. | Yes | `POST /api/admin/assets/:id/toggle-featured` | Backend wired; no visible loading/error state. |
| View marketplace link | rendered `a[href="/property/{slug}"][target="_blank"]` | Open public/authenticated property detail. | Link | `GET /property/:slug` | Static review only; missing `rel="noopener noreferrer"`. |

---

## Frontend Findings

### P2 - API failures leave the table stuck loading

Location:

- Template: `frontend/platform/admin/assets.html:207`
- JS: `frontend/platform/static/js/admin-assets.js:67`

Problem:

`loadAssets()` does nothing for non-OK responses and only logs caught network errors. The loading row remains visible forever, KPI cards stay as dashes, and admins get no retry or permission-denied message. This affects database/API failures and valid admin users who lack `assets.view`.

Expected:

Render a visible error row, clear/update the count and pagination, and distinguish `401/403` from generic server failures. A retry action would be appropriate.

Evidence:

Unauthenticated curl to `/api/admin/assets` returned `401 {"error":"Authentication required"}`. Static review shows the `else {}` branch is empty and catch does not update the DOM.

Recommended fix:

Add a `renderErrorState(message)` path in `loadAssets()` for non-OK and caught failures, and guard `allAssets` so non-array JSON cannot reach `applyFilters()`.

### P2 - Featured toggle gives no visible loading or failure feedback

Location:

- Template-generated action: `frontend/platform/static/js/admin-assets.js:237`
- JS: `frontend/platform/static/js/admin-assets.js:251`

Problem:

The toggle button submits a state-changing request, but the UI never disables the button, never shows progress, and only logs failures to the console. A failed permission check, CSRF failure, 404, or database error looks like a no-op.

Expected:

Disable the clicked button while the request is in flight, show success or failure feedback, and keep the previous visible state on failure.

Evidence:

`toggleFeatured()` calls `fetch(..., { method: "POST" })`, reloads on `resp.ok`, and otherwise only calls `console.error("Failed to toggle featured status")`. CSRF is supplied indirectly by `admin-permission-guard.js`, so the main gap is user-visible state handling.

Recommended fix:

Pass the clicked button into the handler or use delegated events, apply an in-flight state, parse backend error JSON, and render an admin toast/inline error.

### P2 - Sortable and icon-only table controls are not keyboard/ARIA complete

Location:

- Template: `frontend/platform/admin/assets.html:193`
- JS: `frontend/platform/static/js/admin-assets.js:32`
- Rendered actions: `frontend/platform/static/js/admin-assets.js:221`

Problem:

Sortable `<th>` cells are made clickable with JavaScript but are not keyboard-focusable and do not expose `aria-sort`. The featured star indicator and action icons rely on `title`/SVG only, which is weaker than explicit accessible names.

Expected:

Sortable headers should use buttons or `tabindex="0"` with Enter/Space support and `aria-sort`. Icon-only buttons/links should have explicit `aria-label` values; decorative SVGs should be `aria-hidden="true"`.

Evidence:

`setupSorting()` only registers `click` handlers and mutates `style.cursor`. Rendered action buttons have `title` but no `aria-label`.

Recommended fix:

Use real button content inside sortable headers or add keyboard handlers and ARIA state. Add `aria-label="Toggle featured for {asset title}"` and `aria-label="View {asset title} on marketplace"`.

### P3 - Status filter cannot select every displayed backend status

Location:

- Template: `frontend/platform/admin/assets.html:163`
- JS: `frontend/platform/static/js/admin-assets.js:307`

Problem:

The filter dropdown omits `payout_pending`, while `statusBadge()` and the backend status update endpoint support it. If assets reach that state, admins can see the status but cannot filter for it.

Expected:

The status filter should include every status the page can display for live assets.

Evidence:

HTML options include upcoming, funding_open, funding_in_progress, funded, rented, and exited. `statusBadge()` also includes `payout_pending`.

Recommended fix:

Add `Payout Pending` with value `payout_pending` to `#filter-status`.

### P3 - External property links omit noopener protection

Location:

- JS: `frontend/platform/static/js/admin-assets.js:240`

Problem:

Rendered property links use `target="_blank"` without `rel="noopener noreferrer"`.

Expected:

New-tab links should include `rel="noopener noreferrer"`.

Evidence:

The rendered anchor is `<a href="/property/${esc(a.slug)}" target="_blank" ...>`.

Recommended fix:

Add `rel="noopener noreferrer"` to the rendered anchor.

---

## Backend Findings

### P1 - Asset list endpoint silently masks database failures as an empty successful list

Location:

- Backend: `backend/src/admin/assets.rs:21`

Problem:

`GET /api/admin/assets` uses `.unwrap_or_default()` after `fetch_all()`. If the database query fails, the handler returns `200` with an empty `assets` list instead of an error. That can make operators believe all live assets disappeared or that there are no assets to manage.

Expected:

Database errors should propagate through `ApiError::Database`, return a non-2xx API response, and be visible to operators through logs/Sentry.

Evidence:

Static review shows `.fetch_all(&state.db).await.unwrap_or_default()` in the list handler. Other asset detail/mutation handlers use `.map_err(ApiError::Database)?`.

Recommended fix:

Replace `.unwrap_or_default()` with `.map_err(ApiError::Database)?`, add structured logging if needed, and add an API test for database/query error propagation if the test harness supports it.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-assets.js` | No syntax errors. | No output, exit 0. | Pass |
| Unauthenticated page protection | `curl -I http://localhost:8888/admin/assets` | Reject unauthenticated access. | `401 Unauthorized` JSON response headers. | Pass |
| Unauthenticated API protection | `curl -i http://localhost:8888/api/admin/assets` | Reject unauthenticated access. | `401 {"error":"Authentication required"}`. | Pass |
| Authenticated list load | Browser/admin session with assets fixture. | Table, KPI, filters, sort, pagination load. | Not run; no safe admin session supplied. | Not run |
| Featured toggle mutation | Click featured star on safe fixture. | Transaction updates `assets.featured`, writes audit log, UI refreshes. | Not run; mutating action intentionally skipped. | Not run |

---

## Security Findings

- Page and API require an authenticated admin session; unauthenticated smoke checks returned 401.
- `GET /api/admin/assets` requires `assets.view`.
- `POST /api/admin/assets/:id/toggle-featured` requires `assets.publish`, locks the asset row, updates in a DB transaction, and writes an audit log.
- CSRF protection is globally enforced for mutating methods. This page relies on `admin-permission-guard.js` to inject `X-CSRF-Token` into admin fetch calls.
- Finding: new-tab property links should include `rel="noopener noreferrer"`.
- No financial arithmetic is performed client-side beyond display calculations from integer cents and token counts.

---

## Database Findings

- `assets.total_value_cents` and `token_price_cents` are `BIGINT`; the page displays these integer cents as USD.
- `annual_yield_bps` is stored as integer basis points and displayed as a percent.
- `assets.featured`, `published`, and `funding_status` support the list and toggle behavior.
- `idx_assets_status`, `idx_assets_type`, `idx_assets_slug`, and `idx_assets_published_featured` exist in migrations/docs.
- The featured toggle uses `SELECT ... FOR UPDATE`, updates `assets`, inserts into `audit_logs`, and commits atomically.
- Finding: the list query error path masks DB failures with `.unwrap_or_default()`.

---

## Missing Tests

- Authenticated browser/E2E coverage for `/admin/assets` with realistic published asset data.
- API test for `GET /api/admin/assets` permission requirements and response shape.
- API test that `POST /api/admin/assets/:id/toggle-featured` requires `assets.publish`, updates `assets.featured`, and writes one audit log entry.
- Frontend/browser test for API 403/500 error rendering and retry behavior.
- Accessibility test for sortable headers, icon-only action labels, keyboard pagination, and mobile table behavior.

---

## Recommended Fix Order

1. Make `GET /api/admin/assets` propagate DB errors instead of returning an empty success response.
2. Add visible frontend error and permission-denied states for asset loading and featured toggle failures.
3. Add in-flight state and accessible labels/keyboard support for table sort and icon actions.
4. Add the missing `payout_pending` filter option and `rel="noopener noreferrer"` to marketplace links.
5. Add authenticated E2E/API coverage for the list and featured toggle workflows.

---

## Fix Follow-up

2026-04-25: Safe code fixes were applied after this audit.

- PAGE-ISSUE-0067 fixed: `GET /api/admin/assets` now propagates database errors through `ApiError::Database`.
- PAGE-ISSUE-0068 fixed: asset load failures now render a visible error row, retry action, KPI reset, and status message.
- PAGE-ISSUE-0069 fixed: featured toggles now disable the clicked button during submit and show visible success/error feedback.
- PAGE-ISSUE-0070 fixed: sortable headers now expose keyboard handling and `aria-sort`; icon-only actions now have explicit labels.
- PAGE-ISSUE-0071 fixed: property links opened in a new tab now use `rel="noopener noreferrer"`.
- PAGE-ISSUE-0072 fixed: the status filter now includes `payout_pending`.

Remaining verification: authenticated browser/API recheck with a safe admin fixture should still exercise list loading, filters, sorting, pagination, and the featured-toggle DB/audit-log mutation.

---

## Final Status

`needs_recheck`

Reason: The documented code-level gaps have been patched, but authenticated browser and mutation verification still need to run with a safe admin fixture.
