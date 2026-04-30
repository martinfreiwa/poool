# Page Audit: Commodity Detail

Date: 2026-04-27
Status: fixed_needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/commodity`, `/commodity/:slug`
Template: `frontend/platform/commodity.html`
JavaScript: `frontend/platform/static/js/property-detail.js`, `frontend/platform/static/js/property-detail-cart.js`, `frontend/platform/static/js/property-price-sticky.js`, `frontend/platform/static/js/legal-enhancements.js`, `frontend/platform/static/js/pie-chart.js`
CSS: `frontend/platform/static/css/property-detail.css`, `frontend/platform/static/css/commodity-info-section.css`, `frontend/platform/static/css/security-compliance-section.css`, `frontend/platform/static/css/operator-section.css`, `frontend/platform/static/css/financials-distribution-section.css`, `frontend/platform/static/css/mobile-commodity-detail.css`
Backend Routes: `backend/src/assets/mod.rs`, `backend/src/assets/routes.rs`, `backend/src/cart/routes.rs`

---

## Summary

The authenticated commodity detail page is substantially wired: it renders asset data from `assets`, gallery images from `asset_images`, roadmap rows from `asset_milestones`, and submits the investment amount to the real `/cart/add` backend flow. The page is not production-ready because the commodity route can render any published asset slug through the commodity template, page data failures are masked as empty/default states, the documents section is static instead of driven by `asset_documents`, and several interactive UI controls are click-only without keyboard/ARIA state.

Runtime browser verification was blocked because no backend was listening on `localhost:8888` during this audit. Static JS syntax and the targeted commodity display-data Rust test passed.

## Fix Update - 2026-04-28

All six documented code issues from this audit are fixed locally:

| Issue | Status | Fix Summary |
|------|--------|-------------|
| PAGE-ISSUE-0397 | Fixed | `/commodity/:slug` now requires `a.asset_type = 'commodity'` and returns a safe 404 for non-commodity or missing assets. |
| PAGE-ISSUE-0398 | Fixed | Main asset, milestone, similar-asset, platform-fee, and template failures now use explicit 404/5xx handling with generic user-facing responses. |
| PAGE-ISSUE-0399 | Fixed | The page now queries `asset_documents` and renders real document rows or a truthful empty state instead of hardcoded disabled placeholders. |
| PAGE-ISSUE-0400 | Fixed | Gallery and FAQ triggers are semantic buttons; FAQ state updates `aria-expanded`, and the lightbox returns focus on close. |
| PAGE-ISSUE-0401 | Fixed | Add-to-cart backend failures now redirect with explicit error codes, and the detail page shows inline error messages instead of making failures look successful. |
| PAGE-ISSUE-0402 | Fixed | Platform-fee display now parses percent values into basis points and calculates displayed fee totals with integer cents. |

Remaining verification/documented issues:

- Authenticated browser/E2E recheck is still required for commodity-only routing, gallery/lightbox keyboard behavior, FAQ expansion, document downloads, and add-to-cart success/failure states.
- Full Rust verification is blocked by unrelated compile errors in `backend/src/developer/routes.rs`, `backend/src/blog/routes.rs`, `backend/src/blog/mod.rs`, and `backend/src/rewards/routes.rs`.
- Full `cargo fmt --check` is blocked by unrelated formatting drift in `backend/src/admin/deposits.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/reports.rs`, `backend/src/community/routes.rs`, `backend/src/community/service.rs`, `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, and `backend/src/rewards/service.rs`.

---

## Tested Scope

- Reviewed `frontend/platform/commodity.html` and included commodity/property components.
- Reviewed shared page controllers: gallery/lightbox, financial tabs, FAQ accordion, YouTube modal, sticky price card, add-to-cart, cookie consent, and pie chart.
- Reviewed `GET /commodity`, `GET /commodity/:slug`, and `POST /cart/add`.
- Reviewed `assets`, `asset_images`, `asset_milestones`, `asset_documents`, and `cart_items` schema dependencies.
- Checked existing tests for commodity detail coverage.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/commodity` | Authenticated route; no slug/id fallback is not handled explicitly. |
| URL | `/commodity/:slug` | Authenticated route; accepts any published asset slug today. |
| Template | `frontend/platform/commodity.html` | Main commodity detail page. |
| Component | `frontend/platform/components/property/gallery.html` | Image gallery and lightbox trigger. |
| Component | `frontend/platform/components/property/documents.html` | Static disabled documents list. |
| Component | `frontend/platform/components/property/faq-commodity.html` | FAQ accordion content. |
| Component | `frontend/platform/components/property/contact-commodity.html` | `/support` link. |
| JS | `frontend/platform/static/js/property-detail.js` | Gallery, tabs, FAQ, document modal, video modal. |
| JS | `frontend/platform/static/js/property-detail-cart.js` | Quick-add and `/cart/add` submit. |
| JS | `frontend/platform/static/js/property-price-sticky.js` | Sticky right price card. |
| JS | `frontend/platform/static/js/pie-chart.js` | ApexCharts split chart. |
| Backend page route | `GET /commodity`, `GET /commodity/:slug` | Registered in `backend/src/assets/mod.rs`; handler in `backend/src/assets/routes.rs`. |
| Backend action route | `POST /cart/add` | Real cart mutation with auth, KYC, CSRF middleware, row lock, and transaction. |
| Database table | `assets` | Commodity asset fields and route lookup. |
| Database table | `asset_images` | Gallery images. |
| Database table | `asset_milestones` | Roadmap rows. |
| Database table | `asset_documents` | Exists but not used by visible document list. |
| Database table | `cart_items` | Add-to-cart persistence. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Marketplace link | `#property-breadcrumbs a[href="/marketplace"]` | Navigate to marketplace. | Link | Yes | Not runtime-tested. |
| Gallery image tiles | `.gallery-image[onclick]` | Open lightbox at clicked image. | Inline `openLightbox()` | Uses rendered images | Not runtime-tested; click-only divs. |
| View all photos | `#gallery-view-all-button` | Open lightbox. | Inline `openLightbox(0)` | Uses rendered images | Not runtime-tested. |
| Virtual tour group | `.gallery-btn-group[onclick]` | Show "coming soon" toast. | Inline `openVirtualTour()` | No backend | Not runtime-tested. |
| Video thumbnail | `#video-play-button` | Open YouTube modal. | `property-detail.js` | Uses `video_url` via extracted ID | Not runtime-tested. |
| Financial tabs | `.financial-tab[data-tab]` | Toggle cost/yield panels. | `property-detail.js` | Rendered data only | JS syntax passed. |
| Location iframe | `.location-map iframe` | Display map. | Native iframe | Uses `google_maps_url` | Not runtime-tested. |
| Document download buttons | `.download-button[disabled]` | Either download real documents or clearly unavailable. | Disabled only | No detail-page document query | Static disabled placeholder. |
| FAQ items | `.faq-item-content` | Expand/collapse answer. | Click listener | No backend | Not runtime-tested; no button/ARIA state. |
| Contact us link | `.contact-button[href="/support"]` | Navigate to support. | Link | Support route exists elsewhere | Not runtime-tested. |
| Investment amount input | `#investment-amount-input` | User enters USD amount. | Read by cart JS | `/cart/add` parses amount server-side | Not runtime-tested. |
| Quick amount buttons | `.quick-add-btn` | Add USD amount to input. | `property-detail-cart.js` | No backend until submit | JS syntax passed. |
| Add to cart | `#add-to-cart-main-btn` | Submit to `/cart/add`, redirect to cart/KYC/login. | `fetch("/cart/add")` | Yes; auth, KYC, transaction | Not runtime-tested. |
| Pie chart | `#financials-pie-chart-dynamic` | Render operator/POOOL split chart. | `pie-chart.js` + CDN ApexCharts | Rendered data only | JS syntax passed. |
| Cookie consent | `#cookie-consent-banner` | Accept/reject cookie preference. | `legal-enhancements.js` | localStorage only | JS syntax passed. |

---

## Frontend Findings

### P2 - Gallery and FAQ controls are not keyboard-accessible controls

Location:

- Template: `frontend/platform/components/property/gallery.html`
- Template: `frontend/platform/components/property/faq-commodity.html`
- JS: `frontend/platform/static/js/property-detail.js`

Problem:

Gallery tiles are clickable `div` elements with inline `onclick`, and FAQ rows are clickable `div` elements without `button`, `tabindex`, `aria-expanded`, or keyboard handlers. Lightbox open/close works through global functions and Escape/arrow keys after opening, but the trigger controls are not semantic.

Expected:

Gallery triggers and FAQ headers should be real buttons or links with accessible names and expanded/current state where applicable.

Evidence:

Static review found `.gallery-image onclick="openLightbox(...)"` and `.faq-item-content` click listeners only.

Recommended fix:

Convert gallery tiles and FAQ headers to `<button type="button">` or add equivalent keyboard and ARIA handling. Keep lightbox focus inside the modal while open.

---

### P2 - Document section is static and not backed by asset documents

Location:

- Template: `frontend/platform/components/property/documents.html`
- Backend: `backend/src/assets/routes.rs`
- Database: `asset_documents`

Problem:

The visible documents are hardcoded labels with disabled download buttons. The page route never loads `asset_documents`, even though the table exists and admin detail APIs can list documents.

Expected:

Commodity detail should either render real published investor documents for the asset or hide the section until documents are available. Disabled placeholders should not look like asset-specific disclosure material.

Evidence:

Static review found hardcoded "Smart Contract", "Token Registration", and "Articles_of_Organization_LLC.pdf" with disabled buttons; no `asset_documents` query exists in `page_commodity`.

Recommended fix:

Add a public/authenticated asset-document query with safe allowed document types and signed/download URLs, or replace the section with an explicit unavailable empty state.

---

### P2 - Add-to-cart failure handling can send users to cart without useful error context

Location:

- JS: `frontend/platform/static/js/property-detail-cart.js`
- Backend: `backend/src/cart/routes.rs`

Problem:

The frontend redirects to `/cart` on fetch errors. The backend also redirects to `/cart` on asset-not-found, transaction begin failure, and cart upsert failure. This can make a failed add look like a successful cart navigation.

Expected:

The page should show a visible failure state or redirect with a specific error query. Backend failures should not be indistinguishable from success.

Evidence:

Static review found `.catch(() => window.location.href = "/cart")` and backend fallbacks returning `Redirect::to("/cart")` for several failure branches.

Recommended fix:

Return explicit error redirects or JSON for fetch requests, and show an inline error/toast while keeping the user on the asset page when cart persistence fails.

---

## Backend Findings

### P1 - Commodity route can render non-commodity assets

Location:

- Backend: `backend/src/assets/routes.rs`
- Route: `GET /commodity/:slug`

Problem:

`page_commodity` queries `assets` with `WHERE a.slug = $1 AND a.published = true` but does not require `a.asset_type = 'commodity'`. Any published property slug can be rendered through the commodity template, mixing real-estate data into commodity-specific fixed ROI, operator, roadmap, and agricultural disclosure UI.

Expected:

`/commodity/:slug` should only render published commodity assets. Non-commodity slugs should return a safe 404 or redirect to the canonical detail route.

Evidence:

Static route review of `page_commodity` found no `asset_type = 'commodity'` predicate. The separate commodity list/tab queries do include `a.asset_type = 'commodity'`.

Recommended fix:

Add `AND a.asset_type = 'commodity'` to the detail lookup and add a route-level regression test for a published non-commodity slug.

---

### P1 - Detail route masks database failures and missing slugs as empty/default page data

Location:

- Backend: `backend/src/assets/routes.rs`
- Template: `frontend/platform/commodity.html`

Problem:

The asset, milestones, similar-assets, and platform-fee queries use `unwrap_or_default()` / fallback values. If the main asset query fails, `display_data` becomes `None` and the handler still renders `commodity.html`. If no slug/id is provided, the same path is attempted. Template render errors are returned as HTML containing the internal render error text.

Expected:

Database failures should produce safe 5xx handling through the app error path, and missing/not-found slugs should return 404 or redirect. Internal template error details should not be returned to users.

Evidence:

Static review found `.fetch_optional(...).await.unwrap_or_default()`, additional `.fetch_all(...).await.unwrap_or_default()` calls, and `Html(format!("<h1>Internal Server Error</h1><p>{}</p>", e))`.

Recommended fix:

Propagate DB errors through `AppError`, return 404 for missing slugs/assets, and log template errors without embedding internal details in the response.

---

### P2 - Commodity display fee uses float arithmetic for money-like page totals

Location:

- Backend: `backend/src/assets/models.rs`
- Backend: `backend/src/assets/routes.rs`
- Template: `frontend/platform/commodity.html`

Problem:

The displayed platform fee and total investment cost are calculated with `f64` from `platform_fee_percent`. This is display-only, but it can drift from checkout calculations and violates the platform's integer/decimal money standard for user-facing financial figures.

Expected:

Use integer basis points or `Decimal` for fee display so the asset page and checkout can share exact rounding behavior.

Evidence:

Static review found `platform_fee_pct: f64`, `v.parse::<f64>()`, and `((total_value_dollars as f64) * fee_pct / 100.0).round() as i64`.

Recommended fix:

Store/display platform fee as basis points or parse into `Decimal`, then format the result using the same rounding policy as checkout.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page smoke | `curl -I --max-time 3 http://localhost:8888/commodity` | Redirect to `/auth/login` if server is running. | Could not connect to `localhost:8888`. | Blocked |
| Unauthenticated slug smoke | `curl -I --max-time 3 http://localhost:8888/commodity/premium-bali-rice` | Redirect to `/auth/login` if server is running. | Could not connect to `localhost:8888`. | Blocked |
| JS syntax | `node --check` for page JS files | No syntax errors. | Passed. | Pass |
| Targeted Rust commodity model test | `cargo test assets::models::tests::test_commodity --manifest-path backend/Cargo.toml` | Commodity display conversion test passes. | 1 targeted test passed. | Pass |

---

## Security Findings

- P1: Route confusion allows published non-commodity assets to render through commodity-specific investment disclosure.
- P1: Main route error handling can expose template-render error strings in HTML responses.
- CSRF for `/cart/add` is covered by the global middleware and global fetch interceptor in `components/head.html`; this was verified statically, not at runtime.
- Description fields are sanitized in developer update paths before storage, but the template still renders generated `long_description` with `| safe`; keep this safe only if all write paths remain sanitized.

---

## Database Findings

- `assets` has the commodity-specific fields needed by the page and uses integer cents for monetary values.
- `asset_images` and `asset_milestones` are queried for gallery and roadmap data.
- `asset_documents` exists but is not queried by the detail page.
- `cart_items` is updated by `/cart/add` inside a transaction after locking the selected asset row.
- The commodity detail route should use the existing `idx_assets_type` and `idx_assets_slug` semantics by filtering both slug and `asset_type`.

---

## Missing Tests

- Route test: `/commodity/:slug` returns 404 or canonical redirect for a published non-commodity asset.
- Route test: `/commodity/:slug` returns 404 for missing slug and does not render `commodity.html` with `asset = None`.
- Failure-path test: DB/template errors do not leak internal strings to HTML responses.
- E2E test: authenticated commodity page loads, gallery opens/closes, financial tabs toggle, FAQ is keyboard accessible, and Add to cart redirects correctly for approved KYC.
- E2E test: document section renders real document rows or a truthful empty state.
- Unit test: commodity fee display uses exact Decimal/basis-point rounding.

---

## Recommended Fix Order

1. Add `asset_type = 'commodity'`, 404 handling, and safe DB/template error propagation in `page_commodity`.
2. Replace static document placeholders with real asset-document rendering or an explicit unavailable state.
3. Improve add-to-cart failure UX so persistence failures are visible.
4. Convert gallery/FAQ triggers to semantic controls and add modal focus management.
5. Align commodity fee display with exact backend fee rounding.

---

## Final Status

`fixed_needs_recheck`

Reason: The documented code defects are fixed locally. Runtime status remains `needs_recheck` until an authenticated browser/E2E pass verifies the commodity route, document download path, gallery/FAQ keyboard behavior, and add-to-cart flows.
