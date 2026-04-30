# Page Audit: Affiliate referrals

Date: 2026-04-27
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/referrals`
Template: `frontend/platform/affiliate-referrals.html`
JavaScript: inline script in `frontend/platform/affiliate-referrals.html`; declared `frontend/platform/static/js/affiliate-referrals.js` is missing
CSS: declared `frontend/platform/static/css/affiliate-referrals.css` is missing
Backend Routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`

---

## Summary

`/affiliate/referrals` is partially implemented. The protected page route exists and unauthenticated users are redirected to login. The data API exists and correctly requires an active affiliate before returning referral email, status, holdback, and commission data. However, the HTML page is accessible to any authenticated user while the API is active-affiliate-only, the page silently hides API failures, rendered referral rows use `innerHTML` with user-controlled email data, and the page declares missing static assets that return `404`.

Final status is `needs_recheck`.

---

## Tested Scope

- Reviewed `frontend/platform/affiliate-referrals.html`.
- Reviewed shared `frontend/platform/components/head.html` asset loading and `frontend/platform/components/investor-topbar.html` export button.
- Reviewed `backend/src/rewards/mod.rs` route registration.
- Reviewed `backend/src/rewards/routes.rs` page, referral-list API, and commission-export API.
- Reviewed affiliate schema migrations `database/072_affiliate_core_system.sql`, `database/073_affiliate_profile_data.sql`, `database/074_affiliate_indexes.sql`, and `database/076_affiliate_system_gaps.sql`.
- Started local backend with `cargo run` and ran unauthenticated curl smoke checks.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/affiliate/referrals` | Registered in rewards router. |
| Template | `frontend/platform/affiliate-referrals.html` | Main page shell, inline controller, table, tabs, search. |
| Component | `frontend/platform/components/investor-topbar.html` | Adds Export CSV and Back to Dashboard controls for this variant. |
| Component | `frontend/platform/components/head.html` | Loads declared `extra_css` and `extra_js`. |
| JS | `frontend/platform/static/js/affiliate-referrals.js` | Declared by template but file does not exist. |
| CSS | `frontend/platform/static/css/affiliate-referrals.css` | Declared by template but file does not exist. |
| Backend page route | `GET /affiliate/referrals` | `page_affiliate_referrals`, protected by session only. |
| Backend API route | `GET /api/affiliate/referrals` | Requires active affiliate and returns referral details. |
| Backend API route | `GET /api/affiliate/commissions/export` | Server CSV/JSON export exists but is not wired to the page export button. |
| Database table | `affiliates` | Active affiliate gate. |
| Database table | `affiliate_referrals` | Referral state, referred user, holdback expiry. |
| Database table | `affiliate_commissions` | Commission status and cents amount. |
| Database table | `users` | Source of referred email shown on the page. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Loading state | `#referrals-loading`, template lines 21-28 | Show while API loads. | Yes | API-backed | Unverified authenticated; hides in `finally`. |
| Content wrapper | `#referrals-content`, template line 30 | Hidden until load completes. | Yes | API-backed | Unverified authenticated; shown even after API failure. |
| Export CSV | topbar variant, `onclick="exportReferralCSV()"` | Download referral/commission CSV. | Yes, client-side only | Server export API exists but unused | Asset/runtime unauthenticated smoke only; implementation is not backend-backed. |
| Back to Dashboard | `/affiliate/dashboard` link | Navigate back to affiliate dashboard. | Link | Page route exists | Not clicked in authenticated session. |
| All Referrals tab | `.page-tab`, `switchReferralTab('all')` | Show all rows. | Yes | Uses loaded API data | Not authenticated-runtime verified. |
| Under Holdback tab | `.page-tab`, `switchReferralTab('under_holdback')` | Show holdback rows. | Yes | Uses API status values | Not authenticated-runtime verified. |
| Payable tab | `.page-tab`, `switchReferralTab('payable')` | Show payable commission rows. | Yes | Uses commission status values | Not authenticated-runtime verified. |
| Completed tab | `.page-tab`, `switchReferralTab('paid')` | Show paid rows. | Yes | Uses commission status values | Not authenticated-runtime verified. |
| Search input | `#referral-search`, `onkeyup="filterReferrals()"` | Filter by ID, email, or status. | Yes | Uses loaded API data | Label missing; placeholder says date but date is not searched. |
| Referrals table | `#referrals-table-body` | Render status, registration date, holdback expiry, commission. | Yes | `GET /api/affiliate/referrals` | Rows use unsafe `innerHTML`. |
| Empty state | table row colspan 4 | Show no-data message. | Yes | No backend dependency | Also shown after API errors, causing false empty state. |

---

## Frontend Findings

### P1 - Referral rows render API data with `innerHTML`

Location:

- Template: `frontend/platform/affiliate-referrals.html:129`
- JS: inline `renderTable()`

Problem:

Referral rows are built with template strings and assigned to `tbody.innerHTML`. The row includes `r.email`, `r.status`, date fields, and amounts from `/api/affiliate/referrals`. Referred user email is user-originated account data and should not be interpolated into HTML.

Expected:

Build rows with DOM methods and `textContent`, or strictly escape every dynamic value before interpolation.

Evidence:

The API joins `users.email` as `referred_email` in `backend/src/rewards/routes.rs:1158` and the frontend inserts it into `innerHTML` in `frontend/platform/affiliate-referrals.html:136`.

Recommended fix:

Move the inline controller into a real `affiliate-referrals.js` file and render table cells with `document.createElement()` and `textContent`.

### P1 - API failures are shown as an empty successful table

Location:

- Template: `frontend/platform/affiliate-referrals.html:174`
- Backend API: `backend/src/rewards/routes.rs:1127`

Problem:

The page fetches `/api/affiliate/referrals`, ignores non-OK responses, logs network exceptions only to the console, then always hides the loader and shows the table. A pending, suspended, terminated, or non-affiliate authenticated user gets a shell that looks empty rather than a clear “active affiliate required” error or redirect.

Expected:

Non-OK responses should render a visible error state with retry and, for `403`, a clear affiliate-status action such as returning to onboarding/dashboard.

Evidence:

The page route is only session-protected in `page_affiliate_referrals`, while the API requires `affiliates.status = 'active'`. Unauthenticated curl confirmed `GET /api/affiliate/referrals` returns `401 {"error":"Invalid session"}`.

Recommended fix:

Gate the page route to active affiliates or handle API `401/403/5xx` explicitly in the page UI.

### P2 - Declared page-specific static assets are missing

Location:

- Template declaration: `frontend/platform/affiliate-referrals.html:2`
- Template declaration: `frontend/platform/affiliate-referrals.html:3`
- Loader: `frontend/platform/components/head.html:213`

Problem:

The template declares `affiliate-referrals` in `extra_css` and `extra_js`, but neither `frontend/platform/static/css/affiliate-referrals.css` nor `frontend/platform/static/js/affiliate-referrals.js` exists. The actual page logic is inline.

Expected:

Either create the declared files and move page-specific CSS/JS into them, or remove the declarations.

Evidence:

`curl -I http://localhost:8888/static/js/affiliate-referrals.js` and `curl -I http://localhost:8888/static/css/affiliate-referrals.css` both returned `404 Not Found`.

Recommended fix:

Create the missing static assets and keep the inline script/style minimal, or remove the unused declarations and update tracker metadata.

### P2 - Search, tab, and export controls have weak accessibility and inaccurate behavior

Location:

- Template: `frontend/platform/affiliate-referrals.html:36`
- Template: `frontend/platform/affiliate-referrals.html:45`
- Component: `frontend/platform/components/investor-topbar.html:112`

Problem:

Tabs are anchors with inline `onclick` and no `role="tab"`, `aria-selected`, or keyboard model. The search field has no visible or programmatic label, and its placeholder says “Search ID or Date...” but the filter only searches referral ID, email, and status. Export uses `alert()` for no-data feedback.

Expected:

Use button/tabpanel semantics or proper tab roles, add a label or `aria-label`, align placeholder text with actual searchable fields, and show no-data/export feedback inline.

Evidence:

The filter string in `renderTable()` includes only `referral_id`, `email`, and `status`.

Recommended fix:

Replace anchor tabs with accessible buttons, update active state attributes, add a search label, and replace `alert()` with a status region.

### P2 - Client CSV export is not backed by the existing server export API

Location:

- Component: `frontend/platform/components/investor-topbar.html:112`
- JS: `frontend/platform/affiliate-referrals.html:147`
- Backend API: `backend/src/rewards/routes.rs:941`

Problem:

The page export button serializes whatever rows are already in browser memory. It ignores date filters, pagination, backend export authorization response shape, and CSV escaping. A server export endpoint exists, but this page does not use it.

Expected:

Export should either call `/api/affiliate/commissions/export?format=csv` or clearly export only the filtered on-screen rows with proper CSV escaping.

Evidence:

`exportReferralCSV()` builds a data URI locally, while `api_affiliate_commissions_export` exists separately.

Recommended fix:

Wire the button to the backend export endpoint, or rename it to “Export loaded referrals” and escape CSV cells.

---

## Backend Findings

### P1 - Page route and API authorization contracts are inconsistent

Location:

- Page route: `backend/src/rewards/routes.rs:134`
- API route: `backend/src/rewards/routes.rs:1127`

Problem:

`GET /affiliate/referrals` is available to any authenticated user, but `GET /api/affiliate/referrals` is active-affiliate-only. This produces a broken shell for pending or inactive affiliates and makes access behavior inconsistent with the sensitive referral details being displayed.

Expected:

The page route should enforce the same active-affiliate state as the API, or server-render a deliberate pending/blocked state.

### P2 - Server commission export swallows DB errors and uses float formatting for cents

Location:

- Backend: `backend/src/rewards/routes.rs:941`

Problem:

The existing export endpoint uses `.unwrap_or_default()` for the main export query and `.unwrap_or(0)` for the count query, so DB failures can appear as an empty export. Its CSV amount formatting converts cents through `amount_cents as f64 / 100.0`.

Expected:

DB failures should propagate as errors, and cent amounts should be formatted with integer math to preserve the platform money invariant.

Evidence:

Static review of `api_affiliate_commissions_export`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page route | `curl -i http://localhost:8888/affiliate/referrals` | Redirect to login | `303 See Other`, `location: /auth/login` | Pass |
| Unauthenticated API route | `curl -i http://localhost:8888/api/affiliate/referrals` | JSON auth failure | `401 Unauthorized`, `{"error":"Invalid session"}` | Pass |
| Missing JS asset | `curl -I http://localhost:8888/static/js/affiliate-referrals.js` | `200` if declared | `404 Not Found` | Fail |
| Missing CSS asset | `curl -I http://localhost:8888/static/css/affiliate-referrals.css` | `200` if declared | `404 Not Found` | Fail |
| Active affiliate page load | Login as active affiliate and open page | Data loads, no console errors | Not run; no authenticated fixture/session available in this audit | Blocked |
| Tabs/search/export | Use controls with active affiliate data | Correct filtering/export and accessible state | Not run; no authenticated fixture/session available in this audit | Blocked |

---

## Security Findings

- P1: Referred email data is rendered with `innerHTML`, creating an avoidable XSS risk.
- P1: Page and API gates differ; sensitive referral details are API-protected, but the page route should not expose a broken sensitive-data shell to inactive affiliates.
- P2: Client CSV export serializes personal data from browser memory without server-side export auditability or robust CSV escaping.

---

## Database Findings

- Required tables exist: `affiliates`, `affiliate_referrals`, `affiliate_commissions`, `users`.
- Required indexes exist for `affiliate_referrals(affiliate_id, status)`, `affiliate_referrals(referred_user_id)`, and `affiliate_commissions(affiliate_id, status)`.
- Monetary commission values use `BIGINT` cents in `affiliate_commissions.provisional_amount_cents`.
- No database write is performed by the page itself.

---

## Missing Tests

- Authenticated active-affiliate E2E for page load, referral rows, tabs, search, empty state, and export.
- Pending/suspended/non-affiliate E2E verifying the page route and API show a clear blocked state.
- XSS regression test with an encoded or hostile referred-user display/email fixture, ensuring rows use text rendering.
- Static regression test that declared `extra_css` and `extra_js` assets exist.
- API test for `/api/affiliate/referrals` response shape and active-affiliate authorization.

---

## Recommended Fix Order

1. Align `/affiliate/referrals` page authorization with `/api/affiliate/referrals`, or render a deliberate inactive-affiliate state.
2. Replace `innerHTML` table rendering with DOM/text rendering and move inline JS to a real static asset.
3. Add visible API error, retry, and empty states.
4. Create or remove the declared page CSS/JS assets.
5. Wire CSV export to the backend endpoint or make the client export clearly scoped and CSV-safe.
6. Add active-affiliate and inactive-affiliate E2E coverage.

---

## Final Status

`fixed, needs authenticated browser recheck`

Reason: The 2026-04-28 fix pass addressed the documented implementation issues. A fixture-backed active-affiliate browser/E2E pass is still recommended because this run could not verify the full authenticated UI with real referral data.

---

## Fix Update - 2026-04-28

Fixed:

- PAGE-ISSUE-0349: Referral rows now render with DOM APIs and `textContent` in `frontend/platform/static/js/affiliate-referrals.js`; no referral API data is interpolated through `innerHTML`.
- PAGE-ISSUE-0350: `GET /affiliate/referrals` now checks `affiliates.status = 'active'` before serving the page and redirects inactive authenticated users to `/affiliate/onboarding`.
- PAGE-ISSUE-0351: Referral API failures now render a visible live-region error state instead of a false empty table.
- PAGE-ISSUE-0352: Added real `frontend/platform/static/js/affiliate-referrals.js` and `frontend/platform/static/css/affiliate-referrals.css` assets.
- PAGE-ISSUE-0353: Replaced inline anchor tabs with button tabs using `role="tablist"`, `role="tab"`, `aria-selected`, arrow-key navigation, and a labeled search input with accurate placeholder copy.
- PAGE-ISSUE-0354: Export now calls the existing authorized `/api/affiliate/commissions/export?format=csv&limit=200` endpoint. The backend export now propagates DB errors, escapes CSV cells, formats cents with integer math, and computes JSON page counts without float math.

Remaining issues / verification gaps:

- Active-affiliate browser/E2E coverage now exists in `tests/e2e/test_affiliate_referrals.py`, covering seeded referral rows, hostile email safe rendering, tabs, search, backend CSV export, and inactive-affiliate redirect. The test still needs a successful runtime execution once a local backend is reachable.
- `cargo check` is currently blocked by unrelated repository errors outside this fix, including missing `aes_gcm`, existing auth TOTP field mismatches, and other dirty-worktree compile issues. The affiliate-referrals targeted static tests pass.

Verification run:

- `node --check frontend/platform/static/js/affiliate-referrals.js`
- `python3 -m pytest tests/admin/test_affiliate_route_contract_static.py -q`
- `python3 -m py_compile tests/e2e/test_affiliate_referrals.py`
- `python3 -m pytest tests/e2e/test_affiliate_referrals.py -q` attempted; blocked because `http://localhost:8888/health` was not reachable.
- `cargo check` attempted; blocked by unrelated compile errors.
- `cargo fmt --check` attempted; blocked by pre-existing formatting drift in unrelated dirty files.
