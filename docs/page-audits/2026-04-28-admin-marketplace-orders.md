# Page Audit: Marketplace Orders

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/orders`
Template: `frontend/platform/admin/marketplace/orders.html`
JavaScript: `frontend/platform/static/js/mp-orders.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

The page loads for an authenticated admin session and the list API returns the expected paginated JSON shape, but the open-order workflow is not production-ready. Direct marketplace order APIs use only generic `AdminUser` extraction instead of explicit marketplace permissions, the admin-cancel path mutates financial hold state without row locking/audit-log coverage, and the frontend masks any API failure with mock orders that still show a working-looking Cancel Order flow.

Final status is `needs_recheck`.

---

## Tested Scope

- Static template review of `frontend/platform/admin/marketplace/orders.html`.
- JavaScript review of `frontend/platform/static/js/mp-orders.js` and shared modal/toast behavior in `frontend/platform/static/js/mp-toast.js`.
- Backend route review of `GET /admin/marketplace/orders`, `GET /api/admin/marketplace/orders`, and `DELETE /api/admin/marketplace/orders/:order_id`.
- Database support review for `market_orders`, `wallets.held_balance_cents`, and `audit_logs`.
- Runtime smoke with local backend on `localhost:8888`: unauthenticated page/API/DELETE checks and temporary authenticated read-only page/API checks.
- No real order cancellation was submitted.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/orders` | Generic admin template route renders page. |
| URL alias | `/admin/marketplace/orders.html` | Same page. |
| Template | `frontend/platform/admin/marketplace/orders.html` | KPI cards and open orders table shell. |
| JS | `frontend/platform/static/js/mp-orders.js` | Loads orders, renders rows, opens cancel modal, submits DELETE. |
| JS | `frontend/platform/static/js/mp-toast.js` | Toast and modal helper. |
| CSS | `frontend/platform/static/css/admin-marketplace.css` | Marketplace admin styling. |
| Backend page route | `GET /admin/marketplace/orders` | Registered in `backend/src/admin/mod.rs`, served by `page_admin_generic`. |
| Backend API route | `GET /api/admin/marketplace/orders` | Paginated list of open/partially-filled `market_orders`. |
| Backend API route | `DELETE /api/admin/marketplace/orders/:order_id` | Admin-cancel order and release held balance. |
| Database table | `market_orders` | Source of order list and cancel status. |
| Database table | `wallets` | `held_balance_cents` released for buy-order cancellation. |
| Database table | `audit_logs` | Required by schema for critical actions, but cancel handler does not insert a row. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb: Admin | `nav.admin-breadcrumbs a[href="/admin/"]` | Navigate to admin dashboard. | Link only. | Page route exists. | Not clicked; static route present. |
| Admin breadcrumb: Marketplace | `nav.admin-breadcrumbs a[href="/admin/marketplace/"]` | Navigate to marketplace admin overview. | Link only. | Page route exists. | Not clicked; static route present. |
| Health indicators | `.admin-health-dot` | Show matching engine/database status. | No dynamic binding. | No backend health API used. | Static green indicators only. |
| Total Open KPI | `#kpi-total-open` | Show total open orders. | Set by `renderOrders()`. | `GET /api/admin/marketplace/orders` returns `total`. | Authenticated API returned `total: 0`; KPI wiring not browser-click verified. |
| Total Held Balance KPI | `#kpi-held-balance` | Show held balance across loaded orders. | Calculates client-side from loaded page only. | API returns cents per order, not aggregate across all pages. | Static concern: KPI undercounts when more than one page exists. |
| Avg. Order Age KPI | `#kpi-avg-age` | Show average order age. | Calculates client-side from loaded page only. | API returns `created_at`. | Static concern: page-scope average only. |
| Orders table | `#orders-table`, `#orders-body` | Render open order rows. | Uses `innerHTML` rows. | API returns order fields. | Authenticated API returned empty JSON; empty DOM not browser-verified. |
| Empty state | `#orders-body` empty branch | Show "No open orders". | Wired. | Depends on API success. | API returned empty; browser DOM not inspected. |
| Cancel Order buttons | `.btn-cancel-order` | Open cancellation modal for each order. | Bound after row render. | DELETE route exists. | Not submitted; static issues remain. |
| Cancel modal reason field | `#cancel-reason` in `mpModal` body | Require legal reason. | Client checks non-empty only. | Backend accepts optional reason and defaults if absent. | Static mismatch: backend does not enforce reason. |
| Cancel modal confirm | `.mp-modal-confirm` | Submit DELETE and reload list. | Wired. | Route exists, global CSRF applies. | Unauthenticated DELETE without CSRF returned 403. |
| Toasts | `.mp-toast` | Show success/error. | Wired with textContent. | N/A | `node --check` passed. |

---

## Frontend Findings

### P1 - API failures are replaced with fake actionable orders

Location:

- JS: `frontend/platform/static/js/mp-orders.js:17`
- JS: `frontend/platform/static/js/mp-orders.js:204`

Problem:

`loadOrders()` catches every API failure, logs a warning, generates mock orders, and renders active-looking Cancel Order buttons. In production this can make auth failures, permission failures, database failures, or response-shape regressions look like a populated orderbook. If an admin then cancels one of these rows, the mock branch removes it locally and displays a success toast even though no backend mutation occurred.

Expected:

Production admin pages should show an explicit error state for failed live data, disable financial actions, and never render mock financial records outside a clearly separated demo/test mode.

Evidence:

The catch branch sets `usingMockData = true`, calls `generateMockOrders()`, and `openCancelModal()` returns success locally for mock data.

Recommended fix:

Remove production mock fallback from `mp-orders.js`, render an error row with retry, and keep mock data behind an explicit development-only flag.

### P1 - Live order data is inserted with `innerHTML`

Location:

- JS: `frontend/platform/static/js/mp-orders.js:72`
- JS: `frontend/platform/static/js/mp-orders.js:95`

Problem:

Rows are constructed with template literals and assigned to `tbody.innerHTML`. Backend values such as `asset_name`, `user_email` prefix fallback, `side`, `order_type`, `status`, and timestamps are interpolated without escaping. Asset titles and email local parts are database-controlled values, so this is a stored admin-XSS risk.

Expected:

Render dynamic values with `textContent` or an escaping helper before inserting HTML. Reserve static HTML only for developer-controlled badge markup.

Evidence:

`asset = o.asset_name || o.asset_id.substring(0, 8)` is inserted into `<td>${asset}</td>` and `user` is inserted into `<code>${user}</code>`.

Recommended fix:

Build table rows with DOM APIs or add a small `escapeHtml` helper and apply it to every backend-derived value before HTML insertion.

### P2 - Pagination state exists but no controls are rendered

Location:

- Template: `frontend/platform/admin/marketplace/orders.html:75`
- JS: `frontend/platform/static/js/mp-orders.js:10`
- JS: `frontend/platform/static/js/mp-orders.js:201`

Problem:

The API is paginated and the JS stores `currentPage`, `totalPages`, and `totalOrders`, but the template has no pagination controls and `currentPage` is never changed. Admins can only inspect the first 25 open orders.

Expected:

Render pagination controls, disable them during loads, and reload using the requested page.

Evidence:

`PAGE_SIZE` is fixed at 25 and `loadOrders()` requests only the current page, but no UI element changes `currentPage`.

Recommended fix:

Add previous/next controls and a total count label, or raise the page size only if the backend explicitly supports the expected operational volume.

---

## Backend Findings

### P1 - Marketplace orders APIs lack granular marketplace permission checks

Location:

- Backend: `backend/src/admin/marketplace.rs:715`
- Backend: `backend/src/admin/marketplace.rs:781`
- Backend page gate: `backend/src/admin/pages.rs:241`

Problem:

The page gate restricts marketplace pages to admins with `marketplace.view`, `marketplace.manage`, or `marketplace.compliance`, but both direct API handlers only extract `AdminUser`. Unlike adjacent marketplace endpoints, `GET /api/admin/marketplace/orders` does not require `marketplace.view`, and `DELETE /api/admin/marketplace/orders/:order_id` does not require `marketplace.manage`.

Expected:

The list route should require `marketplace.view` or `marketplace.manage`. The cancel mutation should require `marketplace.manage` and likely exclude compliance-only users. Page gates and API gates must match the actual action sensitivity.

Evidence:

The handlers receive `_admin: AdminUser` / `admin: AdminUser` but do not call `admin.require_permission(...)`.

Recommended fix:

Add explicit permission checks to both handlers and add authenticated authorization tests for a generic admin without marketplace permissions, a view-only marketplace admin, a compliance-only admin, and a marketplace manager.

### P1 - Admin cancel is not locked, audited, or ledger-complete

Location:

- Backend: `backend/src/admin/marketplace.rs:790`
- Backend: `backend/src/admin/marketplace.rs:807`
- Backend: `backend/src/admin/marketplace.rs:839`
- Schema: `docs/DATABASE_SCHEMA.md` `audit_logs`

Problem:

The cancel handler verifies status outside the transaction, does not use `SELECT ... FOR UPDATE`, updates the order without a status predicate, releases held balance with `GREATEST(held_balance_cents - $1, 0)`, and does not insert an immutable `audit_logs` row. There is also no wallet transaction or reconciliation record for the hold release. For a critical admin financial mutation, this leaves race, double-submit, auditability, and reconciliation gaps.

Expected:

Within one transaction, lock the order row, validate cancellable status, update only if the row is still cancellable, release the exact held amount with a checked update, write an audit log with previous/new state, and record the financial hold-release event in the ledger/reconciliation model used by marketplace orders.

Evidence:

The handler reads `status` before `db.begin()`, later re-reads order fields without `FOR UPDATE`, updates `wallets.held_balance_cents` with `GREATEST`, and only writes a tracing log after commit.

Recommended fix:

Model the cancel path after the marketplace approval/rejection handlers that lock the order and insert audit logs, then add concurrency and idempotency tests.

### P2 - Backend accepts missing cancellation reason

Location:

- Backend: `backend/src/admin/marketplace.rs:820`
- JS: `frontend/platform/static/js/mp-orders.js:160`

Problem:

The UI marks the cancellation reason as required and blocks blank submissions, but the backend accepts missing `reason` and substitutes `"Admin cancellation"`. Any direct API client can bypass the reason requirement for a sensitive admin action.

Expected:

Backend validation should require a non-empty, bounded reason and return a 400 response for missing/blank/too-long values.

Evidence:

`body.reason.as_deref().unwrap_or("Admin cancellation")` is used for persistence and logging.

Recommended fix:

Validate `reason.trim()` server-side, enforce a length range, and store the sanitized value.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page smoke | `curl http://localhost:8888/admin/marketplace/orders` | Redirect to login | `303 See Other`, `Location: /auth/login` | Pass |
| Unauthenticated list API smoke | `curl /api/admin/marketplace/orders?page=1&per_page=25` | 401 JSON | `401 Unauthorized`, `{"error":"Authentication required"}` | Pass |
| Unauthenticated DELETE without CSRF | `curl -X DELETE /api/admin/marketplace/orders/000...` | CSRF/auth rejection | `403 Forbidden`, CSRF error JSON | Pass |
| Temporary authenticated page smoke | Inserted local short-lived admin session, GET page | 200 HTML | `200`, 4308 bytes, CSRF cookie set | Pass |
| Temporary authenticated list API smoke | Same local admin session, GET API | 200 paginated JSON | `{"data":[],"total":0,"page":1,"per_page":2,"total_pages":0}` | Pass |
| Cancel success path | Submit DELETE against safe seeded open order | Exact cancel, hold release, audit log | Not run to avoid mutating order state in page audit | Not run |
| Browser/mobile/keyboard modal test | Open page in browser and exercise modal | No console errors, focus trap works, responsive table usable | Not run; no Browser Use session used for this audit | Not run |

---

## Security Findings

- P1: Direct APIs rely on generic admin role instead of explicit marketplace permissions.
- P1: Stored XSS risk from backend-derived order values rendered through `innerHTML`.
- P1: Admin cancel lacks immutable audit-log persistence for a sensitive financial action.
- P2: Server-side cancellation reason validation is missing.
- CSRF middleware blocks unauthenticated DELETE requests without a valid token; this passed runtime smoke.

---

## Database Findings

- `market_orders` exists with integer `price_cents`, integer `quantity`, cancellable statuses, `cancel_reason`, and useful indexes.
- `wallets.held_balance_cents` exists with non-negative and held-less-than-balance constraints.
- `audit_logs` exists and is documented as immutable/compliance-required, but the admin open-order cancel handler does not write to it.
- The cancel handler does not create a `wallet_transactions` row for the held-balance release, so ledger/reconciliation evidence is incomplete.

---

## Missing Tests

- Authenticated API authorization tests for generic admin, marketplace view-only, marketplace compliance, and marketplace manager roles.
- Admin cancel integration test covering exact `market_orders.status`, `cancel_reason`, `wallets.held_balance_cents`, audit log row, and expected response.
- Admin cancel concurrency/idempotency test for duplicate DELETE requests against the same open buy order.
- Frontend/browser test that API failure renders an error state, not mock financial orders.
- Frontend/browser XSS fixture test for asset titles and email local parts rendered in the open orders table.
- Browser keyboard/mobile test for cancel modal focus behavior, Escape close, reason validation, and loading/duplicate-submit prevention.

---

## Recommended Fix Order

1. Add `marketplace.view`/`marketplace.manage` permission checks to list and cancel APIs, then test role boundaries.
2. Rework admin cancel into a locked, audited transaction with exact held-balance release and ledger/reconciliation evidence.
3. Remove production mock fallback and render safe API error states with disabled actions.
4. Replace `innerHTML` row rendering for backend values with safe DOM/text rendering.
5. Add pagination controls and modal/browser coverage.

---

## Fix Pass Update

Date: 2026-04-28
Status: implemented locally, needs recheck

Implemented fixes:

- `GET /api/admin/marketplace/orders` now requires `marketplace.view`.
- `DELETE /api/admin/marketplace/orders/:order_id` now requires `marketplace.manage`.
- Admin cancel now requires a non-empty bounded reason, locks the order with `FOR UPDATE`, updates only active orders, releases exact held cash or held tokens with checked updates, writes `audit_logs`, and removes the order from Redis orderbook best-effort after commit.
- `mp-orders.js` no longer renders mock financial orders on API failure; it renders an explicit error row and toast.
- Open-order rows now use DOM/text rendering for backend-derived values instead of template-literal `innerHTML`.
- Cancel DELETE now sends `X-CSRF-Token` from the `csrf_token` cookie.
- Pagination controls were added to the page and wired to the paginated API.
- Added `tests/e2e/test_admin_marketplace_orders.py` covering cancel hold release/audit, missing reason rejection, safe rendering, and pagination.

Remaining issues:

- Full Rust compile/test verification is still pending. `cargo check` in both the default target directory and a temporary `CARGO_TARGET_DIR` could not complete because concurrent Rust builds were holding locks or the compile process was killed during dependency compilation.
- The new E2E tests have not been executed against a freshly compiled backend for the same reason.
- Browser/mobile keyboard verification for the cancel modal remains pending.

Verification completed:

- `node --check frontend/platform/static/js/mp-orders.js`
- `python3 -m py_compile tests/e2e/test_admin_marketplace_orders.py`
- `cd backend && cargo fmt --check`
- `git diff --check` for touched implementation files

---

## Final Status

`needs_recheck`

Reason: The identified implementation gaps have local fixes, but Rust compile/test and the new authenticated E2E verification still need a clean run after the current concurrent build contention clears.
