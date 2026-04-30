# Page Audit: Approvals

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/approvals`
Template: `frontend/platform/admin/approvals.html`
JavaScript: `frontend/platform/static/js/admin-approvals.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page CSS
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/approvals.rs`

---

## Summary

The `/admin/approvals` page is present, protected by admin authentication, loads its page script, and has backend endpoints for listing, creating, approving, and rejecting four-eyes requests. The page is not production-ready for financial/admin-critical actions: approval execution is not locked or atomically marked approved, fine-grained permissions are not enforced server-side, some advertised executors are broken or silently report success, and queue list failures can appear as an empty queue.

Final status is `needs_recheck`.

---

## Tested Scope

- Static review of `frontend/platform/admin/approvals.html`.
- Static review of `frontend/platform/static/js/admin-approvals.js`.
- Static review of route registration in `backend/src/admin/mod.rs` and generic page rendering in `backend/src/admin/pages.rs`.
- Static review of `backend/src/admin/approvals.rs` list/create/approve/reject/executor logic.
- Database schema verification for `admin_approval_requests` in `database/010_advanced_rbac.sql` and local PostgreSQL.
- Unauthenticated runtime smoke against local backend on `localhost:8888`.
- JavaScript syntax check with Node.

Authenticated destructive approval flows were not executed because the audit is documentation-only and approving/rejecting four-eyes actions can mutate financial/admin state.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/approvals` | Clean admin URL registered. |
| URL alias | `/admin/approvals.html` | HTML alias registered. |
| Template | `frontend/platform/admin/approvals.html` | Admin shell, KPI cards, maker form, status tabs, list container. |
| Component | `frontend/platform/admin/components/sidebar.html` | Included by template; badge updated by JS/sidebar loader. |
| JS | `frontend/platform/static/js/admin-approvals.js` | Owns load, filter, create, approve, reject, copy payload. |
| Shared JS | `frontend/platform/static/js/user-data.js` | Provides `getCsrfToken` and loads `poool-confirm.js`. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF header to mutating fetches and hides nav items client-side. |
| CSS | `frontend/platform/static/css/admin.css` | Admin shell/components. |
| CSS | `frontend/platform/static/css/bundle.css` | Global bundled platform CSS. |
| Backend page route | `GET /admin/approvals` | `page_admin_generic`, requires `AdminUser`. |
| Backend page route | `GET /admin/approvals.html` | `page_admin_generic`, requires `AdminUser`. |
| Backend API route | `GET /api/admin/approvals` | Lists latest 200 approval requests. |
| Backend API route | `POST /api/admin/approvals` | Creates maker approval request. |
| Backend API route | `POST /api/admin/approvals/:id/approve` | Executes action and marks request approved. |
| Backend API route | `POST /api/admin/approvals/:id/reject` | Marks request rejected. |
| Database table | `admin_approval_requests` | Exists locally with expected core columns. |
| Database table | `audit_logs` | Used for request/approval/rejection audit records. |
| Related tables | `wallets`, `wallet_transactions`, `deposit_requests`, `users`, `kyc_records`, `assets`, `developer_projects`, `platform_settings`, `dividend_payouts` | Touched by action executors depending on `action_type`. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin link | `.admin-breadcrumbs a[href="/admin/"]` | Navigate to admin dashboard. | Link | Yes | Static pass. |
| Global search | `#admin-global-search` | Admin search suggestions/results. | Shared `admin-global-search.js` | API support not audited here | Present; not page-specific. |
| KPI Pending | `#kpi-pending` | Show count of pending requests. | Yes, from list response | Yes, indirectly | Static pass. |
| KPI Approved | `#kpi-approved` | Show count of approved requests in latest 200 rows. | Yes | Yes, indirectly | Static pass, limited to returned rows. |
| KPI Rejected | `#kpi-rejected` | Show count of rejected requests in latest 200 rows. | Yes | Yes, indirectly | Static pass, limited to returned rows. |
| Action type select | `#req-action-type` | Choose action to submit. | Yes | Partially | Broken for some action contracts; see findings. |
| Entity type select | `#req-entity-type` | Choose target table/entity. | Yes | Weakly | Backend stores value but does not validate action/entity compatibility. |
| Entity ID input | `#req-entity-id` | Optional UUID target. | Yes | Partially | Invalid UUID silently becomes `NULL`. |
| Payload textarea | `#req-payload` | JSON payload for executor. | Yes, JSON.parse validation | Partially | No action-specific client guidance or server schema validation. |
| Submit for Approval | `#new-request-form` submit | POST create request and reload queue. | Yes | Yes | Static pass; mutating runtime not executed. |
| Filter All | `.approval-tab[data-filter=""]` | Show all loaded approvals. | Yes | Client-only | Static pass. |
| Filter Pending | `.approval-tab[data-filter="pending"]` | Show pending approvals. | Yes | Client-only | Static pass. |
| Filter Approved | `.approval-tab[data-filter="approved"]` | Show approved approvals. | Yes | Client-only | Static pass. |
| Filter Rejected | `.approval-tab[data-filter="rejected"]` | Show rejected approvals. | Yes | Client-only | Static pass. |
| Approval cards | `.approval-card[data-id]` | Render action, entity, status, requester, payload, reviewer. | Yes | Yes | Static pass. |
| View Payload disclosure | `.approval-card__payload details` | Expand JSON payload. | Browser native | Yes | Static pass. |
| Copy JSON | `.approval-btn--copy` | Copy payload to clipboard. | Yes | No backend needed | Static pass. |
| Approve & Execute | `window._approveRequest(id)` | Confirm, POST approve, reload. | Yes | Yes, unsafe | Needs backend fix/recheck. |
| Reject | `window._rejectRequest(id)` | Capture reason, POST reject, reload. | Yes | Yes, unsafe | Needs UX/backend recheck. |
| Loading state | `#approvals-list` initial spinner | Show while loading. | Yes | No backend needed | Static pass. |
| Empty state | `#approvals-list .admin-empty-state` | Show when no filtered requests. | Yes | No backend needed | Static pass. |
| Error state | `#approvals-list` error HTML | Show load failure. | Yes | Backend currently masks DB failures | Partially broken. |

---

## Frontend Findings

### P2 - Reject flow uses native prompt and has weak accessible confirmation

Location:

- Template: `frontend/platform/admin/approvals.html`
- JS: `frontend/platform/static/js/admin-approvals.js:195`

Problem:

Reject uses `prompt()` rather than the platform confirmation/modal pattern. It has no focus-managed dialog, structured validation, or accessible error state, and the approve/reject buttons are not disabled while requests are in flight.

Expected:

Use a POOOL modal with `role="dialog"`, labelled fields, focus management, visible validation, loading/disabled states, and retry-safe submit behavior.

Evidence:

`_rejectRequest` calls `prompt("Rejection reason (required):")`; approve/reject POST handlers leave buttons clickable while the request is pending.

Recommended fix:

Replace native prompt with shared modal/confirm primitives and disable the card action buttons during each mutation.

---

## Backend Findings

### P0 - Approval execution is not locked or atomic and can double-execute financial/admin actions

Location:

- Backend: `backend/src/admin/approvals.rs:172`
- Backend: `backend/src/admin/approvals.rs:214`
- Backend: `backend/src/admin/approvals.rs:221`

Problem:

Approve reads a pending request without `FOR UPDATE`, executes the business action, then marks the request approved in a separate statement whose error is ignored. Two checker requests can both read `pending` and both execute before either status update lands. If execution succeeds but the status update or audit insert fails, the API can return success while the request remains pending or unaudited.

Expected:

Approval should lock the request row inside a transaction, transition status atomically with action execution/audit logging, and make duplicate approval attempts impossible. Financial actions must not be able to execute twice.

Evidence:

`SELECT ... FROM admin_approval_requests WHERE id = $1` has no lock. The action is executed before `UPDATE admin_approval_requests`, and both update and audit writes are assigned to `let _`.

Recommended fix:

Move approval processing into a single transaction with `SELECT ... FOR UPDATE`, status guard, action execution that accepts the transaction, required audit insert, and a committed approved status. Add concurrent approval tests for balance adjustment and dividend processing.

### P1 - Approval APIs do not enforce fine-grained server-side permissions

Location:

- Backend: `backend/src/admin/approvals.rs:16`
- Backend: `backend/src/admin/approvals.rs:69`
- Backend: `backend/src/admin/approvals.rs:162`
- Backend: `backend/src/admin/approvals.rs:256`

Problem:

The page navigation hides Approvals behind `approvals.manage`, and the schema defines specialized permissions such as `financials.payout.approve`, but the backend endpoints only require generic admin access. Any admin role that passes `AdminUser` can list, create, approve, or reject requests, including requests for financial payouts, balance adjustments, user deletion, KYC override, settings changes, and asset publication.

Expected:

Server-side permission checks should gate the page and each API action. Action-specific permissions should be enforced for sensitive action types, for example finance approval for payouts/balance adjustments, compliance approval for KYC overrides, and admin management approval for user suspension/deletion.

Evidence:

Handlers accept `AdminUser` or manual `is_admin` only; no `require_permission("approvals.manage")` or action-specific permission check is present.

Recommended fix:

Use `AdminUser::require_permission` for the page/list/create/reject baseline and an action-to-permission map before approve execution.

### P1 - Advertised executors are incomplete or can report false success

Location:

- Template: `frontend/platform/admin/approvals.html:410`
- Backend: `backend/src/admin/approvals.rs:526`
- Backend: `backend/src/admin/approvals.rs:544`

Problem:

The form advertises `treasury.payout` and `settings.update`, but executor support is not reliable. `treasury.payout` ignores the insert result into `treasury_transactions` and still returns `{"payout_processed": true}` if the table is missing or the insert fails. `settings.update` updates `platform_settings.settings` where `id = 1`, but the actual table is key/value shaped with columns `key`, `value`, `value_type`, `description`, `updated_at`, and `updated_by`.

Expected:

Every selectable action should map to a real, schema-valid executor. Failed executor writes must fail the approval and leave a clear error.

Evidence:

Database schema in `database/006_admin_settings.sql` has no `settings` or `id` column on `platform_settings`. No migration defining `treasury_transactions` was found by `rg`; the executor explicitly calls `.await.ok()` on that insert.

Recommended fix:

Remove unsupported actions from the UI until implemented, or implement typed action handlers with schema-checked SQL and required success rows. Never ignore executor write failures.

### P2 - Queue list can silently render an empty approval queue on DB failure

Location:

- Backend: `backend/src/admin/approvals.rs:20`
- Backend: `backend/src/admin/approvals.rs:38`

Problem:

The list endpoint uses `.unwrap_or_default()` after `fetch_all`. A database/schema/query failure returns HTTP 200 with an empty `approvals` array, causing admins to believe the approval queue is empty.

Expected:

The endpoint should return an error status and preserve the frontend error state when the queue cannot be loaded.

Evidence:

The query result is converted to an empty vector on any error.

Recommended fix:

Propagate the database error through `ApiError::Internal` after logging operational context, and add an E2E/API test that a forced query failure does not look like an empty queue.

### P2 - Request creation accepts invalid targets that can only fail later

Location:

- JS: `frontend/platform/static/js/admin-approvals.js:232`
- Backend: `backend/src/admin/approvals.rs:117`

Problem:

The UI labels `Entity ID (UUID)`, but invalid UUID strings are accepted by the backend as `NULL` because parsing uses `.and_then(|s| s.parse().ok())`. Entity type is also not validated against the selected action type. This lets admins create pending requests that cannot execute, or that use the wrong target type for a sensitive action.

Expected:

If an action requires a UUID, invalid or missing IDs should be rejected at creation time. Action/entity compatibility and action payload schema should be validated before inserting the approval request.

Evidence:

`entity_id` parse errors are silently mapped to `None`; `entity_type` is stored but not used by the executor.

Recommended fix:

Add an action schema table or static validator that defines required entity type, required entity ID, payload fields, and executor permission for each action.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-approvals.js` | No syntax errors. | Passed. | Pass |
| Local schema check | Query `information_schema.columns` for `admin_approval_requests`. | Required columns exist. | `id`, `requester_id`, `approver_id`, `action_type`, `entity_type`, `entity_id`, `payload`, `status`, `rejection_reason`, `expires_at`, `created_at`, `updated_at` exist. | Pass |

## 2026-04-28 Fix Pass

Status: fixed, needs authenticated maker/checker E2E recheck.

Fixed issues:

- PAGE-ISSUE-0044: Approval execution now locks the request row, claims `pending` requests as `processing`, rejects duplicate claims, and requires final status/audit writes.
- PAGE-ISSUE-0045: List/create/approve/reject now require `approvals.manage`, and create/approve/reject enforce action-specific permissions from the action contract.
- PAGE-ISSUE-0046: Unsupported `treasury.payout` is disabled instead of advertised as executable, and `settings.update` writes the real `platform_settings` key/value schema.
- PAGE-ISSUE-0047: Queue list database/query failures now propagate as errors instead of returning a false empty queue.
- PAGE-ISSUE-0048: Reject flow now uses an accessible modal with labelled reason input and live validation, and approval buttons enter disabled/busy states during mutations.

Verification:

- `python3 -m pytest tests/admin/test_admin_approvals_static.py -q`
- `node --check frontend/platform/static/js/admin-approvals.js`
- `rustfmt --edition 2021 --check backend/src/admin/approvals.rs`

Remaining:

- Authenticated maker/checker browser or API E2E still needs to recheck create, approve, reject, duplicate approve conflict, and action-specific permission denial with real admin roles.
| Local queue count check | `SELECT status, COUNT(*) FROM admin_approval_requests GROUP BY status`. | Safe read only. | Local DB had one `expired` request. | Pass |
| Unauthenticated page | `curl http://localhost:8888/admin/approvals` | Protected route rejects unauthenticated access. | HTTP 401. | Pass |
| Unauthenticated API | `curl http://localhost:8888/api/admin/approvals` | Protected API rejects unauthenticated access. | HTTP 401. | Pass |
| Authenticated create/approve/reject | Submit/approve/reject with admin sessions. | Mutations work once and are audited. | Not executed; destructive financial/admin mutations are unsafe for documentation-only run. | Not run |

---

## Security Findings

- P0: Approval execution can race and double-run financial/admin actions.
- P1: Fine-grained permissions are missing server-side.
- P1: Some action executors can falsely report success or fail from schema mismatch.
- P2: DB list failures can hide the approval queue.
- P2: Invalid UUID/entity/action payload validation is deferred until approval time.

CSRF is partially covered by `admin-permission-guard.js`, which injects `X-CSRF-Token` into mutating fetches when the cookie is available. Server-side CSRF behavior was not exhaustively audited for these handlers in this run.

---

## UX and Accessibility Findings

- The page has labels for the maker form and visible loading/empty/error states.
- Status tabs are buttons with text labels and are keyboard-focusable by default.
- The approve path uses the platform confirm helper when loaded through `user-data.js`.
- The reject path uses native `prompt()` and should be replaced with a focus-managed modal.
- Pending action buttons need in-flight disabled/loading states to prevent repeat clicks.
- Mobile CSS stacks the maker grid and card headers; no authenticated browser viewport pass was performed.

---

## Automated Test Coverage

Existing tests touch the page/API only lightly:

- `tests/admin/test_admin_dashboard.py` checks `/admin/approvals.html` page accessibility in a broad admin page smoke suite.
- `tests/admin/test_admin_features.py` calls `/api/admin/approvals`, but expects a `pending` field and a detail route that the current API does not provide.

Recommended tests:

- API test for unauthorized, generic-admin-without-permission, and permitted admin access.
- Concurrent approve test proving only one approval execution can commit.
- Balance adjustment approval test verifying exact wallet cents and audit log in one transaction.
- `settings.update` and `treasury.payout` contract tests or removal tests if these actions remain disabled.
- Frontend E2E for create validation, invalid JSON, invalid UUID, copy payload, filters, approve confirm, reject modal, and error states.

---

## Commands Run

```bash
node --check frontend/platform/static/js/admin-approvals.js
psql -d poool -Atc "SELECT column_name || ':' || data_type || ':' || is_nullable FROM information_schema.columns WHERE table_name='admin_approval_requests' ORDER BY ordinal_position;"
psql -d poool -Atc "SELECT COUNT(*) FROM admin_approval_requests; SELECT status, COUNT(*) FROM admin_approval_requests GROUP BY status ORDER BY status;"
curl -s -o /tmp/poool_approvals_unauth.html -w '%{http_code} %{redirect_url}\n' http://localhost:8888/admin/approvals
curl -s -o /tmp/poool_approvals_api_unauth.json -w '%{http_code}\n' http://localhost:8888/api/admin/approvals
```

---

## Recommended Next Fixes

1. Make approval execution transactional and row-locked before approving any financial/admin action.
2. Add server-side baseline and action-specific permission checks.
3. Remove or implement broken `treasury.payout` and `settings.update` executors.
4. Propagate queue list DB errors instead of returning an empty queue.
5. Replace native reject prompt with a real modal and add disabled/loading states.
