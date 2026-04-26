# Page Audit: Marketplace Approvals

Date: 2026-04-26
Status: fixed; UI accessibility browser recheck recommended
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/approvals`
Template: `frontend/platform/admin/marketplace/approvals.html`
JavaScript: `frontend/platform/static/js/mp-approvals.js`, `frontend/platform/static/js/mp-toast.js`, `frontend/platform/static/js/admin-permission-guard.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`, `backend/src/marketplace/service.rs`

---

## Summary

The page shell and route exist, and the frontend can load a pending-approval list from `/api/admin/marketplace/approvals`. The approval workflow is not production-ready. The reject endpoint has a critical financial bug that can inflate a buyer wallet balance and fails to release sell-order token holds. The approve endpoint also changes an order to `open` without a transaction, row lock, audit log, or orderbook insertion/broadcast. The API group relies only on the broad `AdminUser` extractor instead of enforcing `marketplace.manage`, and the UI hides API failures with mock approvals while rendering backend data through template-string `innerHTML`.

Fix status: fixed in local working tree on 2026-04-26; authenticated HTTP/DB E2E recheck passed. UI accessibility browser recheck remains recommended.

Final status: `fixed_backend_e2e`; UI accessibility browser recheck recommended.

---

## Fix Applied

Date fixed: 2026-04-26

Files changed:

- `backend/src/admin/marketplace.rs`
- `backend/src/admin/pages.rs`
- `frontend/platform/static/js/mp-approvals.js`
- `frontend/platform/static/js/mp-toast.js`
- `tests/e2e/test_admin_marketplace_approvals.py`

What changed:

- Reject now releases buy holds by decrementing `held_balance_cents` only, never by increasing `balance_cents`.
- Reject now releases sell holds by decrementing `investments.held_tokens`.
- Approve/reject now require `marketplace.manage`, lock the order row, run in a transaction, check state, and write durable audit logs.
- Approve now inserts the opened order into Redis orderbook and broadcasts an orderbook update after the DB transaction commits.
- Approval list now returns `review_reason` and `supply_impact_bps`.
- Approval list now decodes real `assets.title` and `TIMESTAMPTZ` values correctly in the local PostgreSQL schema.
- The approvals page now renders backend data with DOM APIs/text nodes instead of template-string `innerHTML`.
- Mock approval fallback was removed and replaced with a visible retryable load error.
- Approve/reject now use a focus-managed confirmation dialog, reject requires a reason, and mutation buttons expose disabled/`aria-busy` states.
- Added an authenticated E2E fixture that seeds pending buy/sell orders and verifies approval/rejection via the real admin API.

Verification:

- `node --check frontend/platform/static/js/mp-approvals.js && node --check frontend/platform/static/js/mp-toast.js`
- `cargo fmt --check`
- `cargo check`
- `cargo test --bin poool-backend admin::marketplace::tests`
- `BASE_URL=http://localhost:8888 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_marketplace_approvals.py -q`

Runtime result: targeted authenticated HTTP/DB E2E passed (`1 passed`). Local Redis was not configured, so the test verified orderbook visibility through the admin orderbook endpoint and asserted the API exposes `orderbook_synced`.

---

## Tested Scope

- Read required automation and project standards.
- Selected exactly one page from `docs/page-review-tracker.yml`: `admin.marketplace.approvals`.
- Reviewed the page template, shared admin sidebar/permission behavior, page JS, toast helper, backend page route, backend API routes, marketplace order creation/hold logic, migrations, and existing tests.
- Ran JavaScript syntax checks for the page scripts.
- Attempted local runtime page/API smoke tests on `localhost:8888`; blocked because no backend was listening.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/approvals` | Registered clean route. |
| URL alias | `/admin/marketplace/approvals.html` | Registered alias. |
| Template | `frontend/platform/admin/marketplace/approvals.html` | Shell, empty state, JS includes. |
| JS | `frontend/platform/static/js/mp-approvals.js` | Fetches list, renders cards, approve/reject actions. |
| JS | `frontend/platform/static/js/mp-toast.js` | Toast and modal utility; modal not used by this page. |
| JS | `frontend/platform/static/js/admin-permission-guard.js` | Sidebar permission hiding and fetch CSRF interceptor. |
| Backend page route | `GET /admin/marketplace/approvals` | `page_admin_generic`; page access allows marketplace view/manage/compliance. |
| Backend API route | `GET /api/admin/marketplace/approvals` | Lists `market_orders.status = 'pending_review'`. |
| Backend API route | `POST /api/admin/marketplace/approvals/:order_id/approve` | Sets status to `open`. |
| Backend API route | `POST /api/admin/marketplace/approvals/:order_id/reject` | Sets status to `rejected` and attempts hold release. |
| Database table | `market_orders` | Statuses include `pending_review` and `rejected`; pending index exists. |
| Database table | `wallets` | `held_balance_cents` tracks buy-order holds. |
| Database table | `investments` | `held_tokens` tracks sell-order holds. |
| Database table | `audit_logs` | Not written by marketplace approval decisions. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `admin-breadcrumbs`, lines 31-37 | Navigate to admin and marketplace index. | Link navigation. | Page routes exist. | Not runtime tested. |
| Health dots | `.admin-health-indicators`, lines 40-43 | Show system status. | Static only. | No page-specific backend. | Static indicator only. |
| Approvals grid | `#approvals-grid`, line 54 | Render pending approval cards. | Yes, `render()`. | Yes, list API. | Static review only. |
| Empty state | `#approvals-empty`, lines 59-65 | Show when no pending orders. | Yes. | Depends on list API. | Static review only. |
| Approval card | `.mp-approval-card`, JS lines 71-119 | Show order, user, asset, reason, quantity, unit price, total value, supply impact. | Yes, via `innerHTML`. | Partially. API omits real reason and supply impact. | XSS/inaccuracy risk. |
| Approve button | `.btn-approve`, JS lines 89-91 | Approve pending order and queue execution. | Yes. | Partially; backend sets status `open` only. | Broken production semantics. |
| Reject button | `.btn-reject`, JS lines 92-94 | Reject order and release holds. | Yes. | Broken for buy and sell holds. | Critical financial issue. |
| Toasts | `mpToast()`, `mp-toast.js` | Show success/error/warning. | Yes. | No backend dependency. | Syntax checked. |

---

## Frontend Findings

### P1 - Approval Cards Render Backend Data Through `innerHTML`

Location:

- JS: `frontend/platform/static/js/mp-approvals.js:49-120`
- Shared JS: `frontend/platform/static/js/mp-toast.js:33-36`

Problem:

The page interpolates `asset_name`, `user_email`-derived `userName`, IDs, and toast messages into `innerHTML`. If any backend field contains unexpected HTML, this admin page can render it as markup.

Expected:

Render backend data with DOM APIs and `textContent`, or sanitize every dynamic value before insertion. Toast messages should also render message text via `textContent`.

Evidence:

`render()` builds a template literal from API data and assigns it to `grid.innerHTML`; `mpToast()` interpolates `message` into `toast.innerHTML`.

Recommended fix:

Replace approval-card rendering with element creation or a strict escaping helper. Update `mpToast()` to create child nodes and set `.textContent` for the message.

### P2 - API Failures Are Hidden Behind Mock Approval Data

Location:

- JS: `frontend/platform/static/js/mp-approvals.js:13-30`
- JS: `frontend/platform/static/js/mp-approvals.js:205-216`

Problem:

If the approval API fails, the page logs a warning and renders three fake approval cards. On an admin financial-control page, fake pending orders can be mistaken for real production state.

Expected:

Show a visible retryable error state and keep the grid empty. Mock data should be unavailable in production.

Evidence:

`loadApprovals()` catches any API error, sets `approvals = [...MOCK_APPROVALS]`, and sets `usingMockData = true`.

Recommended fix:

Remove production mock fallback. Render an error banner with a retry button and include the failed status/message.

### P2 - Review Context Is Incomplete And Sometimes Inaccurate

Location:

- JS: `frontend/platform/static/js/mp-approvals.js:66-68`
- Backend: `backend/src/admin/marketplace.rs:1049-1067`

Problem:

The UI shows `Supply Impact` as `—%` for real API data and hardcodes the reason as `Flagged for admin review`. Admins cannot see whether the order exceeded value, concentration, or supply thresholds.

Expected:

The API should return a durable review reason, concentration/supply impact, order type, filled quantity, and current hold state, or the UI should omit unsupported fields.

Evidence:

The list API returns only order/user/asset/side/price/quantity/created_at. The JS sets `supplyPct = '—'` and a generic reason for real data.

Recommended fix:

Persist and return the review trigger from order creation, or compute a truthful read-only summary in the list API.

### P3 - Mutation UX Lacks Confirmation And Accessible Busy State

Location:

- JS: `frontend/platform/static/js/mp-approvals.js:139-177`

Problem:

Approve/reject execute immediately, disable only the clicked button by CSS class, and do not set `disabled`, `aria-busy`, or confirmation text. Reject uses a generic reason, not an admin-entered reason.

Expected:

Use a focus-managed confirmation dialog, require a rejection reason, disable both card action buttons while mutating, and expose busy/error state accessibly.

Evidence:

`handleAction()` posts `{ reason: "Admin approved" }` or `{ reason: "Admin rejected" }` without confirmation or user-entered reason.

Recommended fix:

Use the existing admin modal/confirm pattern, add a reason textarea for reject, and set semantic disabled/aria state during POST.

---

## Backend Findings

### P0 - Rejecting Pending Buy Orders Inflates Wallet Balance And Sell Rejections Do Not Release Tokens

Location:

- Backend: `backend/src/admin/marketplace.rs:1128-1198`
- Hold creation: `backend/src/marketplace/service.rs:230-255`
- Schema: `database/050b_alter_wallets_held_balance.sql`, `database/050c_alter_investments_held_tokens.sql`

Problem:

Order creation places a buy hold by increasing `wallets.held_balance_cents` only; it does not subtract from `wallets.balance_cents`. The reject path subtracts the hold and also adds the same amount to `balance_cents`, creating money. For sell orders, the reject path never decrements `investments.held_tokens`, so tokens remain locked after rejection.

Expected:

Rejecting a buy order should reduce `held_balance_cents` only. Rejecting a sell order should reduce `held_tokens` only. Both should verify affected rows and never make total wallet balance larger.

Evidence:

`service.rs` line 234 increments `held_balance_cents`; `marketplace.rs` line 1175 decrements `held_balance_cents` and increments `balance_cents`. The reject handler has no sell-side `held_tokens` release.

Recommended fix:

Update reject logic in one transaction with `SELECT ... FOR UPDATE`, release the appropriate hold based on `side`, verify row counts, and add regression tests for buy and sell pending-review rejection.

### P1 - Approving Pending Orders Is Not Atomic And Does Not Insert The Order Into The Live Orderbook

Location:

- Backend: `backend/src/admin/marketplace.rs:1082-1125`
- Orderbook insertion pattern: `backend/src/marketplace/service.rs:271-287`

Problem:

Approve reads status without `FOR UPDATE`, updates the order outside a transaction, does not check affected rows, does not write an audit log, and does not insert/broadcast the newly open order to Redis/WebSocket. The order may sit invisible to the matching engine until a later sync/rebuild path catches it.

Expected:

Approve should lock the order row, verify status, update status, write an audit log, commit atomically, then insert into Redis/broadcast or explicitly invoke the matching/orderbook refresh path with observable failure handling.

Evidence:

`api_admin_marketplace_approve_order()` uses `fetch_optional(db)` then `UPDATE ... execute(db)`, with no transaction, row lock, audit write, Redis insert, or broadcast.

Recommended fix:

Move approval into a transaction, lock the order, update with a `WHERE status = 'pending_review'`, write `audit_logs`, commit, then call the same orderbook insertion/broadcast behavior used for normal open orders.

### P1 - Marketplace Approval APIs Lack Fine-Grained `marketplace.manage` Enforcement

Location:

- Backend: `backend/src/admin/marketplace.rs:1043-1046`
- Backend: `backend/src/admin/marketplace.rs:1083-1088`
- Backend: `backend/src/admin/marketplace.rs:1128-1133`
- Sidebar permission intent: `frontend/platform/static/js/admin-permission-guard.js:76-78`

Problem:

The frontend navigation treats approvals as `marketplace.manage`, but the list, approve, and reject endpoints only require the broad `AdminUser` extractor. This bypasses the fine-grained permission model used elsewhere in admin marketplace APIs.

Expected:

The list endpoint should require `marketplace.view` or `marketplace.manage`; approve/reject should require `marketplace.manage`. Page access should not allow compliance-only admins into a manage-only workflow.

Evidence:

The approval handlers accept `AdminUser` but never call `admin.require_permission(...)`.

Recommended fix:

Add explicit permission checks and cover unauthorized admin roles in API tests.

### P1 - Marketplace Approval Decisions Are Not Audit Logged

Location:

- Backend: `backend/src/admin/marketplace.rs:1082-1198`

Problem:

Approve/reject decisions only emit application logs. There is no durable `audit_logs` record with actor, order, previous state, new state, and reason.

Expected:

Financial/admin marketplace decisions should write audit rows in the same transaction as the state transition.

Evidence:

No `INSERT INTO audit_logs` appears in either marketplace approval handler.

Recommended fix:

Insert audit records for `marketplace.order.approved` and `marketplace.order.rejected` inside the transaction.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/mp-approvals.js && node --check frontend/platform/static/js/mp-toast.js` | Scripts parse. | Passed with no output. | Pass |
| Page smoke | `curl http://localhost:8888/admin/marketplace/approvals` | 302/401/200 depending auth. | Could not connect to localhost:8888. | Blocked |
| API smoke | `curl http://localhost:8888/api/admin/marketplace/approvals` | 401 unauthenticated or data when authenticated. | Could not connect to localhost:8888. | Blocked |
| Existing test coverage search | `rg` for marketplace approvals tests | Targeted tests exist. | No targeted marketplace approvals E2E/API tests found. | Fail |

---

## Security Review

- Authentication: backend page and API require `AdminUser`, so unauthenticated users should be blocked.
- Authorization: insufficient. Approval APIs do not enforce `marketplace.manage`; page gate also allows `marketplace.compliance` into the approvals page.
- CSRF: page fetches rely on the global `admin-permission-guard.js` fetch interceptor for `X-CSRF-Token`; direct explicit headers are absent in `mp-approvals.js`.
- XSS: unsafe dynamic `innerHTML` rendering exists for approval cards and toast messages.
- Sensitive data: user email-derived display names are shown on a high-sensitivity admin page; this should be intentional and permissioned.
- Financial integrity: reject flow is critically unsafe for wallet balance and sell-token holds.
- Auditability: approval decisions have no durable audit records.

---

## UX and Accessibility Review

- Empty state exists.
- Loading state for initial API load is missing.
- API error state is replaced by mock data.
- Approve/reject buttons have text labels, but use decorative symbols and no semantic disabled state.
- No focus-managed confirmation dialog for financial admin actions.
- Reject reason is not collected from the admin.
- Mobile/responsive behavior was not runtime-tested.

---

## Automated Test Coverage

No targeted test coverage was found for:

- `GET /api/admin/marketplace/approvals`
- `POST /api/admin/marketplace/approvals/:order_id/approve`
- `POST /api/admin/marketplace/approvals/:order_id/reject`
- buy-order hold release on rejection
- sell-order hold release on rejection
- Redis/orderbook visibility after approval
- marketplace.manage authorization
- approval/rejection audit rows
- frontend error state and unsafe rendering regression

Recommended tests:

- Rust integration tests for approve/reject state transitions and hold accounting.
- API authorization tests for admins without `marketplace.manage`.
- E2E test with seeded pending buy and sell orders verifying UI actions, DB state, audit logs, and visible error handling.

---

## Severity Counts

| Severity | Count |
|----------|-------|
| P0 / critical | 1 |
| P1 / high | 4 |
| P2 / medium | 2 |
| P3 / low | 1 |

---

## Recommended Fix Order

1. Fix reject hold release accounting for buy and sell orders, with tests.
2. Make approve/reject transactional, row-locked, audited, and row-count checked.
3. Enforce `marketplace.manage` for approve/reject and align page access.
4. Ensure approved orders enter Redis/orderbook/matching immediately or fail visibly.
5. Remove mock fallback and add real loading/error states.
6. Replace unsafe `innerHTML` rendering with safe DOM rendering.
7. Add confirmation/reason modal and accessible mutation states.
