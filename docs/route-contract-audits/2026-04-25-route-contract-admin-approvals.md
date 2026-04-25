# Route/API Contract Audit: Admin Approvals

Date: 2026-04-25

Selected scope: Admin Approvals page and four-eyes approval API contract.

Page route: `/admin/approvals`, `/admin/approvals.html`

Frontend:

- `frontend/platform/admin/approvals.html`
- `frontend/platform/static/js/admin-approvals.js`
- `frontend/platform/static/js/admin-sidebar-loader.js`
- `frontend/platform/static/js/user-data.js`

Backend:

- `backend/src/admin/mod.rs`
- `backend/src/admin/approvals.rs`
- `backend/src/admin/extractors.rs`
- `database/010_advanced_rbac.sql`
- `database/006_admin_settings.sql`

## Route/File Map

| UI / API surface | Frontend caller | Backend route / handler | Contract status |
| --- | --- | --- | --- |
| Page render | `/admin/approvals`, `/admin/approvals.html` | `backend/src/admin/mod.rs` -> `page_admin_generic` | Exists and admin-protected. |
| List approvals | `fetch("/api/admin/approvals")` | `GET /api/admin/approvals` -> `api_admin_approvals_list` | Exists; response shape matches UI. |
| Create approval request | `POST /api/admin/approvals` | `api_admin_approvals_create` | Exists; body shape mostly matches, but entity ID validation is too weak. |
| Approve request | `POST /api/admin/approvals/:id/approve` | `api_admin_approvals_approve` | Exists; body/response shape matches, but execution contract is unsafe under concurrency. |
| Reject request | `POST /api/admin/approvals/:id/reject` | `api_admin_approvals_reject` | Exists; body/response shape matches, but mutation contract lacks fine-grained permission and durable write checks. |
| Sidebar pending badge | `fetch("/api/admin/approvals")` in sidebar loader | Same list endpoint | Exists; expects `approvals` array and counts client-side. |

## Frontend Action Inventory

Links/navigation:

- Breadcrumb link to `/admin/`.
- Sidebar link from `admin-sidebar-loader.js` to `/admin/approvals.html`.

Form:

- `#new-request-form` submits JSON to `POST /api/admin/approvals`.
- Request body: `{ action_type, entity_type, entity_id, payload }`.
- `entity_id` is a UUID-like text input, but frontend does not require it and sends `undefined` when blank.
- Payload is free-form JSON from `#req-payload`, defaulting to `{}`.

Fetch/API calls:

- `GET /api/admin/approvals`
  - Expected response: `{ approvals: Approval[], pending_count?: number }`.
  - UI consumes `approvals[].id`, `action_type`, `entity_type`, `entity_id`, `payload`, `status`, `rejection_reason`, `requester_name`, `requester_email`, `approver_name`, `approver_email`, `created_at`.
- `POST /api/admin/approvals`
  - Expected success: `{ status: "pending", approval_id, message }`.
  - Expected error: `{ error }`.
- `POST /api/admin/approvals/:id/approve`
  - Body: `{}`.
  - Expected success: `{ status: "approved", message, result }`.
  - Expected error: `{ error }`.
- `POST /api/admin/approvals/:id/reject`
  - Body: `{ reason }`.
  - Expected success: `{ status: "rejected", message }`.
  - Expected error: `{ error }`.

CSRF/auth expectations:

- `user-data.js` defines `getCsrfToken()`, but `admin-approvals.js` does not send any CSRF header on state-changing POSTs.
- Backend handlers in this selected scope authenticate via `AdminUser` or manual `is_admin`, but no CSRF verification was visible in the route handlers.

## Backend Route Inventory

Registered in `backend/src/admin/mod.rs`:

- `GET /admin/approvals`
- `GET /admin/approvals.html`
- `GET /api/admin/approvals`
- `POST /api/admin/approvals`
- `POST /api/admin/approvals/:id/approve`
- `POST /api/admin/approvals/:id/reject`

Handler response shapes:

- List returns `{ approvals, pending_count }`.
- Create returns `{ status, approval_id, message }`.
- Approve returns `{ status, message, result }`.
- Reject returns `{ status, message }`, or JSON `{ error }` on failure.

Executor actions currently accepted by backend:

- `deposit.confirm`
- `deposit.cancel`
- `balance.adjust`
- `user.suspend`
- `user.delete`
- `kyc.override`
- `kyc.reject`
- `treasury.payout`
- `settings.update`
- `submission.approve`
- `submission.reject`
- `dividend.process`

Actions exposed by the page:

- All above except `dividend.process`.

## Mismatches And Issues

### Critical: Approval execution can double-run sensitive actions

Evidence:

- `api_admin_approvals_approve` reads the pending row without `SELECT ... FOR UPDATE`.
- It executes the business action before marking the approval request approved.
- It ignores failures while updating `admin_approval_requests` and inserting the audit log.

Impact:

- Two checkers can race the same pending request and execute financial/admin mutations more than once.
- The UI can show success while the approval request remains pending or unaudited.

Recommended fix:

1. Move approve/reject state transitions into one transaction.
2. Lock the approval request row with `FOR UPDATE`.
3. Atomically transition `pending -> approved/rejected` before or within the same durable execution flow.
4. Treat status and audit-log write failures as hard failures.
5. Add an idempotency/execution record for irreversible financial/admin actions.

### High: State-changing approval APIs lack CSRF contract

Evidence:

- Frontend POSTs send only `Content-Type: application/json`.
- The shared `getCsrfToken()` helper exists, but this page does not use it.
- The selected backend handlers do not visibly require or validate a CSRF token.

Impact:

- Authenticated admin sessions may be exposed to cross-site request forgery for create, approve, and reject operations.

Recommended fix:

1. Require the standard CSRF header/cookie contract on all three POST endpoints.
2. Add the header from `admin-approvals.js`.
3. Return a consistent JSON 403 shape that the page can display.

### High: Approval APIs only require generic admin access

Evidence:

- List, create, and approve use `AdminUser`, but do not call `require_permission`.
- Reject manually checks only `is_admin`.
- UI permission guard references `approvals.manage`, but backend does not enforce it.

Impact:

- Any generic admin can create or execute balance adjustments, KYC overrides, user deletion, settings updates, treasury payouts, and asset publication.

Recommended fix:

1. Require `approvals.manage` for queue access.
2. Require action-specific permissions for create and approve, for example `financials.payout.approve`, `kyc.override`, `assets.publish`, `roles.edit`, or a dedicated mapping.
3. Keep the permission mapping server-side; the UI guard is only advisory.

### High: Entity/action contract accepts invalid or mismatched IDs

Evidence:

- Frontend lets admins pair any `action_type` with any `entity_type`.
- Backend ignores `_entity_type` during execution.
- Backend parses invalid `entity_id` with `and_then(|s| s.parse().ok())`, silently storing `NULL`.

Impact:

- Invalid form submissions become pending requests that fail only at approval time.
- Reviewers have weak assurance that a request describes the object that will actually be mutated.

Recommended fix:

1. Reject invalid UUID text at create time.
2. Define an action-to-entity-type allowlist.
3. Require `entity_id` at create time for actions whose executor requires it.
4. Optionally return an action-specific schema to the frontend instead of a free-form generic form.

### High: Advertised executors can fail or report false success

Evidence:

- `treasury.payout` ignores insert failures with `.await.ok()` and still returns success.
- `settings.update` writes `platform_settings.settings` and `id = 1`, but the schema is key/value columns `key`, `value`, `value_type`, `description`, `updated_at`, `updated_by`.

Impact:

- Admins can receive false success for treasury payout.
- Settings approvals appear available in the UI but do not match the database schema.

Recommended fix:

1. Remove or disable unsupported executor options until they have real contracts.
2. Fail loudly on all executor write errors.
3. Align `settings.update` with the key/value settings schema or route it through the existing settings service.

### Medium: List endpoint hides database/query failures

Evidence:

- `GET /api/admin/approvals` uses `fetch_all(...).await.unwrap_or_default()`.

Impact:

- Operational failures are returned as HTTP 200 with an empty queue; frontend treats that as success.

Recommended fix:

1. Return a 500 JSON error on query failure.
2. Keep the frontend error state for non-2xx responses.

### Medium: Frontend mutation handling allows double-clicks and assumes JSON errors

Evidence:

- Approve/reject/create buttons are not disabled while requests are in flight.
- The code immediately calls `await resp.json()` on all POST responses.

Impact:

- Double-clicks can submit duplicate requests before backend protection is fixed.
- Non-JSON error pages or empty responses become generic network errors, making operational diagnosis harder.

Recommended fix:

1. Disable and label in-flight mutation buttons.
2. Parse JSON defensively; fall back to status text for non-JSON responses.

## Missing Routes

No missing backend routes were found for the selected page's visible fetch calls or page navigation.

## Dead UI Actions

- No completely dead UI action was found: every visible create/approve/reject action has a backend route.
- Several actions are effectively unsafe or unsupported by contract, especially `treasury.payout` and `settings.update`.
- `dividend.process` is accepted by backend and queued from treasury code, but it is not exposed in the generic approvals page action dropdown.

## Unused Backend Routes Noticed In Scope

- No unused route in the selected `/api/admin/approvals` route group was identified.
- `dividend.process` is an accepted backend action not exposed by this page; it appears to be created from `backend/src/admin/treasury.rs`.

## Severity Summary

| Severity | Count |
| --- | ---: |
| Critical | 1 |
| High | 4 |
| Medium | 2 |
| Low | 0 |
| Info | 0 |

Total issues: 7.

## Recommended Fix Order

1. Fix approve/reject transactionality, row locking, audit durability, and idempotency.
2. Add CSRF enforcement and frontend headers for all approval POSTs.
3. Add backend fine-grained permissions and action-specific permission mapping.
4. Validate action/entity/body contracts at create time.
5. Remove or fix unsupported executor options.
6. Return real list endpoint errors instead of empty success.
7. Add frontend in-flight and defensive response parsing states.

## Verification Performed

- Static route and contract review of selected frontend/backend files.
- `node --check frontend/platform/static/js/admin-approvals.js` passed.
- Schema inspection of `database/006_admin_settings.sql` and `database/010_advanced_rbac.sql`.

No authenticated mutation was executed because this automation is documentation-only and must not modify production application code or data.
