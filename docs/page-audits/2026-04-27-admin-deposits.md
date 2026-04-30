# Page Audit: Admin Deposits

Date: 2026-04-27
Status: fixed_needs_runtime_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/deposits`
Template: `frontend/platform/admin/deposits.html`
JavaScript: `frontend/platform/static/js/admin-deposits.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/poool-dropdown.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/deposits.rs`, `backend/src/admin/reports.rs`, `backend/src/payments/service.rs`

---

## Summary

`/admin/deposits` has been fixed for the documented RBAC, audit durability, DB-error, table alignment, disputes-tab contract, evidence-bundle workflow, accessibility, and external-script issues. Targeted authenticated browser/API/mutation E2E coverage has been expanded for evidence bundles; the expanded runtime rerun is pending behind unrelated backend compile errors in `backend/src/admin/rewards.rs`.

---

## Fix Pass - 2026-04-28

Implemented fixes for the documented audit findings:

- Added granular `/admin/deposits` page access through `deposits.read`, `deposits.write`, or `deposits.confirm` instead of generic admin-only access.
- Replaced deposit API `AdminUser` gates with deposit permission checks so finance/deposit roles can use the page according to seeded permissions.
- Added admin-aware deposit confirmation audit context; admin notes now flow into `audit_logs.new_state`, and four-eyes approvals pass the approver as the actor.
- Wrapped deposit cancel and extend operations plus audit inserts in database transactions with row locks and propagated failures.
- Replaced masked deposit/dispute list DB failures with propagated API errors and visible frontend error states.
- Fixed the disputes tab response contract, provider field, visible errors, and added a real authenticated evidence-bundle workflow.
- Removed the unused Plaid Match column, aligned table colspans, aligned provider filter options with backend provider values, and added keyboard semantics for sortable headers.
- Added dialog semantics, Escape close, focus entry, and focus restoration for the deposit confirmation modal.
- Replaced prompt-based deposit cancellation and dispute status updates with accessible page controls/confirmation flow.
- Fixed sortable-header `aria-sort` values to use valid `ascending`/`descending` tokens.
- Removed the unused external HTMX CDN script from the deposits template.
- Added `POST /api/admin/disputes/:id/evidence` to generate a durable internal bundle URL with an audit log.
- Added `GET /api/admin/disputes/:id/evidence` to return the authenticated evidence bundle JSON from dispute/user/transaction records.

---

## Tested Scope

- Static template review of `frontend/platform/admin/deposits.html`.
- Static JavaScript review of `frontend/platform/static/js/admin-deposits.js`.
- Backend route review in `backend/src/admin/mod.rs`, `backend/src/admin/deposits.rs`, `backend/src/admin/reports.rs`, and `backend/src/payments/service.rs`.
- Database support check for `deposit_requests` and `payment_disputes` through migrations and local `psql`.
- Runtime unauthenticated smoke against local `cargo run` server.
- Syntax check for page JavaScript.
- Static admin sorting regression covering the corrected deposit provider/reference columns.
- New targeted authenticated deposits E2E at `tests/e2e/test_admin_deposits.py` covers authenticated `deposits.read`/`deposits.write` users, permission denial, CSRF denial, seeded pending deposits/disputes, wallet credit, audit actor/notes, cancel/extend audit rows, dispute status audit rows, evidence-bundle generation/viewing/audit, sortable-header keyboard behavior, dialog semantics, and dead-CDN checks. The pre-evidence version passed; the expanded evidence rerun is pending behind unrelated backend compile errors in `backend/src/admin/rewards.rs`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/deposits` | Clean route redirects unauthenticated users to `/auth/login`. |
| URL alias | `/admin/deposits.html` | Registered to the deposit permission page handler. |
| Template | `frontend/platform/admin/deposits.html` | Deposits table, filters, disputes tab, confirmation modal. |
| JS | `frontend/platform/static/js/admin-deposits.js` | Loads list, filters/sorts, confirms/cancels/extends deposits, loads disputes. |
| CSS | `frontend/platform/static/css/admin.css` | Shared admin shell and table/card styles. |
| Backend page route | `GET /admin/deposits` | Registered in `backend/src/admin/mod.rs`; handled by `page_admin_deposits` with granular deposit permissions. |
| Backend API route | `GET /api/admin/deposits` | Lists latest 200 deposit requests and client stats. |
| Backend API route | `POST /api/admin/deposits/:tx_id/confirm` | Requires `deposits.write`, looks up provider reference, and calls `confirm_deposit_with_audit`. |
| Backend API route | `POST /api/admin/deposits/:tx_id/cancel` | Requires `deposits.write`; row-locks and transactionally cancels with audit log. |
| Backend API route | `POST /api/admin/deposits/:tx_id/extend` | Requires `deposits.write`; row-locks and transactionally extends with audit log. |
| Backend API route | `GET /api/admin/disputes` | Requires `deposits.read`; frontend now consumes `{ "disputes": [...] }`. |
| Backend API route | `PUT /api/admin/disputes/:id/status` | Requires `deposits.write`; frontend uses an inline select and Save button. |
| Backend API route | `POST /api/admin/disputes/:id/evidence` | Requires `deposits.write`; stores a durable internal bundle URL and writes an audit log. |
| Backend API route | `GET /api/admin/disputes/:id/evidence` | Requires `deposits.read`; returns the evidence bundle JSON. |
| Database table | `deposit_requests` | Exists with `amount_cents BIGINT`, status check, provider reference index. |
| Database table | `wallets`, `wallet_transactions` | Used by transactional confirmation service. |
| Database table | `payment_disputes` | Exists with amount in cents and provider dispute fields. |
| Database table | `audit_logs` | Used for deposit/dispute admin audit records. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `.admin-breadcrumbs a[href="/admin/"]` | Navigate to admin home. | Link | Yes | Not clicked; route exists by admin router. |
| Global admin search | `#admin-global-search` | Shared admin search. | Shared `admin-global-search.js` | Shared backend/search context not audited here | Unverified authenticated. |
| Notifications button | `.admin-notification-btn` | Open/view notifications. | No page-local handler | Not page-specific | Dead or shared behavior unverified. |
| Refresh button | `#btn-refresh` | Reload deposits. | `loadDeposits()` | `GET /api/admin/deposits` | Source verified; unauthenticated API returns 401. |
| KPI cards | `#stat-pending`, `#stat-confirmed`, `#stat-expired`, `#stat-volume` | Display deposit stats. | `updateStats()` | Stats returned by deposits API | Backend masks DB failure as empty stats. |
| Requests tab | `.admin-tab[data-tab="requests"]` | Show deposits table. | Tab handler | No backend call | Source verified. |
| Disputes tab | `.admin-tab[data-tab="disputes"]` | Show disputes and load data. | `loadDisputes()` | `GET /api/admin/disputes` | Broken response-shape contract. |
| Deposit search | `#deposit-search` | Filter by user/email/reference. | Debounced `applyFilters()` | Client-only | Source verified. |
| Status filter | `#filter-status` | Filter deposits by status. | `applyFilters()` | Client-only | Source verified. |
| Currency filter | `#filter-currency` | Filter deposits by currency. | `applyFilters()` | Client-only | Source verified. |
| Provider filter | `#filter-provider` | Filter deposits by provider. | `applyFilters()` | Client-only | Fixed to expose `stripe`, `ocbc`, `midtrans`, `mangopay`, and `manual`. |
| Deposit table sort headers | `th[data-sort]` | Sort visible list. | `setupSorting()` | Client-only | Fixed with role, tabindex, Enter/Space handling, and aria-sort updates. |
| Deposit table body | `#deposits-table-body` | Render rows. | `renderTable()` | `GET /api/admin/deposits` | Fixed by removing unused Plaid Match header and aligning colspans. |
| Confirm deposit row button | inline `onclick="openConfirmModal(...)"` | Open modal and submit confirm. | `openConfirmModal()` / `confirmDeposit()` | `POST /api/admin/deposits/:id/confirm` | Fixed to send notes and audit admin actor. |
| Extend expiry row button | inline `onclick="extendDeposit(...)"` | Extend by 48 hours. | `extendDeposit()` | `POST /api/admin/deposits/:id/extend` | Fixed with transactional audited backend mutation. |
| Cancel row button | inline `onclick="cancelDeposit(...)"` | Confirm cancellation and cancel. | `cancelDeposit()` | `POST /api/admin/deposits/:id/cancel` | Fixed with shared confirm flow and transactional audited backend mutation. |
| Confirm modal | `#confirm-modal` | Confirm deposit with optional notes. | Open/close handlers | Confirm route stores notes in audit state | Fixed with dialog semantics, Escape close, focus entry, and focus restoration. |
| Confirm notes | `#confirm-notes` | Store admin notes. | Sent in JSON | Backend stores notes in audit `new_state` | Fixed. |
| Pagination | `#prev-page`, `#next-page`, `#pagination-info` | Page through client list. | Wired | Client-only | Source verified. |
| Disputes table | `#disputes-table-body` | Render disputes. | `loadDisputes()` | `GET /api/admin/disputes` | Fixed response shape and provider mapping. |
| Evidence bundle action | evidence cell | Build a bundle when none exists, then view the authenticated bundle URL. | `buildEvidenceBundle()` | `POST/GET /api/admin/disputes/:id/evidence` | Fixed with real route, persisted URL, and audit log. |
| Dispute status select/save | `resolveDispute(id)` | Update dispute status. | Calls PUT status | Route exists | Fixed with select control and visible non-OK errors. |

---

## Frontend Findings

### P1 - Risk & Disputes tab cannot work end to end

Location:

- Template: `frontend/platform/admin/deposits.html:254`
- JS: `frontend/platform/static/js/admin-deposits.js:357`
- Backend: `backend/src/admin/reports.rs:151`, `backend/src/admin/mod.rs:656`

Problem:

`loadDisputes()` treats `await resp.json()` as an array and reads `disputes.length`, but the backend returns `{ "disputes": [...] }`. The row renderer also reads `d.payment_provider`, while the backend returns `provider`. The Build Bundle action calls `POST /api/admin/disputes/:id/evidence`, but no route is registered.

Expected:

The frontend should consume the backend response shape, render provider data correctly, and wire a real evidence-generation endpoint.

Evidence:

Static review showed `GET /api/admin/disputes` returns `Json({"disputes": disputes})`; only `/api/admin/disputes` and `/api/admin/disputes/:id/status` are registered.

Recommended fix:

Change `loadDisputes()` to read `data.disputes || []`, use `d.provider`, add visible error handling, and implement the evidence route.

### P2 - Deposit table columns are misaligned

Location:

- Template: `frontend/platform/admin/deposits.html:218`
- JS: `frontend/platform/static/js/admin-deposits.js:194`

Problem:

The table has 10 headers including `Plaid Match`, but `renderTable()` emits 9 row cells and the empty-state row uses `colspan="9"`. The `Plaid Match` header is followed by expiry data, so every row after Status shifts one column left.

Expected:

Each rendered row and empty state should match all 10 headers, or the unused Plaid Match column should be removed.

Evidence:

Template loading row uses `colspan="10"` while JS empty row uses `colspan="9"` and there is no Plaid Match `<td>` in the mapped row.

Recommended fix:

Add a real Plaid/provider-match cell from backend data or remove the header and normalize all colspans.

### P2 - Deposit confirmation modal has weak accessibility and dead notes

Location:

- Template: `frontend/platform/admin/deposits.html:310`
- JS: `frontend/platform/static/js/admin-deposits.js:268`
- Backend: `backend/src/admin/deposits.rs:121`

Problem:

The modal has no `role="dialog"`, `aria-modal`, focus management, Escape-key close, or focus restoration. The Notes field is sent by the frontend but ignored by the backend, so it gives admins a false sense that confirmation context is retained.

Expected:

The modal should be keyboard-operable and either persist notes into the audit log or remove the field.

Evidence:

Static review found click-only modal close behavior and the backend comment says notes are intentionally ignored.

Recommended fix:

Use the shared modal/dialog pattern with focus handling and include notes in `audit_logs.new_state` when confirming.

---

## Backend Findings

### P1 - Deposit page and APIs use generic admin access instead of deposit-specific RBAC

Location:

- Page route: `backend/src/admin/pages.rs:161`
- API routes: `backend/src/admin/deposits.rs:10`, `backend/src/admin/deposits.rs:99`, `backend/src/admin/deposits.rs:142`, `backend/src/admin/deposits.rs:189`
- Permissions seed/reference: `database/006_admin_settings.sql:72`, `frontend/platform/static/js/admin-permission-guard.js:57`

Problem:

The page is served by `page_admin_generic`, which only requires an admin account except for a few explicit page exceptions. The deposit APIs only require `AdminUser`. Existing permission data and frontend navigation reference deposit-specific permissions (`deposits.read`, `deposits.write`, and `deposits.confirm`), but the backend does not enforce them. Finance-role permissions seeded in the database are also ineffective for this page if the extractor only accepts admin/super_admin roles.

Expected:

Read access should require `deposits.read` or an equivalent finance/admin permission, and mutating confirmation/cancel/extend actions should require `deposits.write` or `deposits.confirm`.

Evidence:

No `has_permission` calls for deposits exist in the audited page/API handlers; `AdminUser` checks only admin/super_admin role membership.

Recommended fix:

Add page and API permission gates aligned to a single canonical permission set. Decide whether finance users should access deposit operations, then enforce that decision server-side.

### P1 - Admin deposit confirmations audit the credited user as the actor

Location:

- Backend: `backend/src/admin/deposits.rs:99`
- Service: `backend/src/payments/service.rs:307`

Problem:

`api_admin_deposit_confirm()` calls `payments::service::confirm_deposit(&state.db, &provider_ref)` without passing the admin actor. The service inserts `audit_logs.actor_user_id = user_id`, where `user_id` is the depositor who received funds. This misattributes an admin financial action to the customer and loses the admin notes from the UI.

Expected:

Admin confirmation should audit the authenticated admin actor, target deposit, amount/currency, previous/new state, and optional admin notes.

Evidence:

`confirm_deposit()` binds `user_id` into `actor_user_id` for action `deposit.confirmed`; `api_admin_deposit_confirm()` ignores the request body.

Recommended fix:

Split webhook and admin confirmation audit context or pass an explicit actor/metadata object into `confirm_deposit`.

### P1 - Cancel and extend can commit without durable audit logs

Location:

- Backend: `backend/src/admin/deposits.rs:152`
- Backend: `backend/src/admin/deposits.rs:198`

Problem:

Cancel and extend update `deposit_requests` first, then insert audit logs with `let _ = ...`. Audit failures are swallowed and the state change is not wrapped in a transaction with the audit row.

Expected:

Financial/admin state changes should commit atomically with audit logs or fail without changing state.

Evidence:

Both handlers execute the `UPDATE` on `state.db`, then separately execute `INSERT INTO audit_logs` and ignore the result.

Recommended fix:

Wrap each mutation and audit insert in a SQL transaction, propagate audit insert errors, and include previous/new status or expiry in the audit payload.

### P2 - Deposit and dispute list DB errors are masked as empty data

Location:

- Deposits: `backend/src/admin/deposits.rs:14`
- Disputes: `backend/src/admin/reports.rs:156`

Problem:

Both list handlers call `.fetch_all(...).await.unwrap_or_default()`. A database outage, bad query, or schema mismatch returns a successful empty response. Admins may interpret this as no pending deposits or no disputes.

Expected:

Read failures should return a 500 JSON error and visible frontend error state.

Evidence:

Static review found `unwrap_or_default()` in both handlers.

Recommended fix:

Replace with `?`/`ApiError::from`, add frontend error states, and add tests for DB error propagation.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-deposits.js` | No syntax errors. | Passed with no output. | Pass |
| Start backend | `cd backend && cargo run` | Local server starts. | Server started; migrations logged existing-object/duplicate-seed errors but process continued. | Partial |
| Unauthenticated page | `curl -i http://localhost:8888/admin/deposits` | Redirect to login. | `303 See Other` to `/auth/login`, CSRF cookie set. | Pass |
| Unauthenticated deposits API | `curl -i http://localhost:8888/api/admin/deposits` | JSON auth error. | `401 Unauthorized`, `{"error":"Authentication required"}`. | Pass |
| CSRF before auth | POST confirm without CSRF | CSRF rejection. | `403 Forbidden`, CSRF error JSON. | Pass |
| Auth before mutation | POST confirm with matching CSRF cookie/header but no session | Auth rejection. | `401 Unauthorized`. | Pass |
| 2026-04-28 fix verification | `CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/tmp/poool-admin-deposits-check cargo check --message-format=short` | Rust code type-checks. | Passed. | Pass |
| Authenticated deposits UI | Login as admin/finance and inspect list/actions. | Table renders, actions work, no console errors. | Not run; no seeded authenticated browser session was used. | Blocked |
| Mutating deposit confirmation | Confirm safe pending deposit. | Wallet credited exactly once, audit actor is admin. | Not run to avoid financial mutation; static review found audit actor bug. | Blocked |

---

## Security Findings

- P1: Deposit page and APIs do not enforce deposit-specific backend permissions despite seeded/reference permissions.
- P1: Deposit confirmation audit logs attribute the action to the depositor instead of the admin.
- P1: Cancel and extend financial/admin mutations are not atomic with audit logging.
- P2: DB read failures are masked as empty admin data, which can hide operational incidents.
- P2: Dispute update uses a browser prompt and has weak visible error handling on non-OK responses.

---

## Database Findings

- `deposit_requests.amount_cents` is `BIGINT` with `CHECK (amount_cents > 0)`.
- `deposit_requests.status` is constrained to `pending`, `paid`, `expired`, `failed`, `cancelled`.
- `deposit_requests.provider` allows `stripe`, `ocbc`, `midtrans`, `mangopay`, `manual`, but the UI provider filter shows `stripe`, `xendit`, `manual`.
- `payment_disputes.amount_cents` is `BIGINT`; status is unconstrained in migration `012_payment_disputes.sql`, so backend validation is the only visible status guard.
- `confirm_deposit()` uses `FOR UPDATE` and a transaction for wallet crediting, which is the right pattern, but its admin audit context is incomplete.

---

## Missing Tests

- Authenticated API tests for `GET /api/admin/deposits` with `deposits.read`/finance/admin authorization boundaries.
- Authenticated mutation tests for confirm/cancel/extend permission gates, CSRF, and unauthorized role denial.
- Financial integration test proving admin confirmation credits a wallet exactly once and writes an audit row with the admin actor.
- Transactional tests proving cancel/extend roll back when audit logging fails.
- Frontend or Playwright test for deposits table column alignment and modal accessibility.
- Disputes tab contract test for `GET /api/admin/disputes`, missing evidence endpoint behavior, and status update error states.

---

## Recommended Fix Order

1. Enforce canonical deposit permissions on page/API routes, including the finance-role decision.
2. Fix admin confirmation audit attribution and notes persistence without weakening webhook confirmation.
3. Make cancel/extend mutations transactional with durable audit logs.
4. Stop masking deposit/dispute DB errors and add visible frontend error states.
5. Repair the disputes tab response shape, provider field, and evidence action.
6. Align deposit table columns/provider filter and improve modal/prompt accessibility.

---

## Final Status

`needs_recheck`

Reason: The page was statically and partially runtime-audited, and multiple financial/admin workflow issues were documented. Authenticated browser and safe mutating E2E verification still need seeded test data after fixes.
