# E2E Coverage Gap Audit: Admin Approvals

Date: 2026-04-25
Automation: Daily POOOL E2E Coverage Gap Tracker
Selected scope: `/admin/approvals` unified four-eyes approval queue and `/api/admin/approvals` route group
Status: not adequate for production release

## Scope

This run audits exactly one high-risk admin workflow: the maker/checker approval queue at `/admin/approvals` and `/admin/approvals.html`, backed by `backend/src/admin/approvals.rs` and `frontend/platform/static/js/admin-approvals.js`.

The flow can create, approve, and reject requests for financial, KYC, user, asset, and settings actions. Because those mutations can affect money, permissions, account status, compliance state, and published assets, E2E coverage must prove authorization, CSRF, four-eyes constraints, backend state changes, and audit durability.

## Existing Coverage Found

| File | Existing coverage | Gap |
| --- | --- | --- |
| `tests/admin/test_admin_dashboard.py` | Broad authenticated admin smoke calls `/admin/approvals.html` and checks the page renders in the admin shell. | Does not exercise Playwright UI behavior, create/approve/reject mutations, API state verification, CSRF, roles, or mobile/keyboard flows. |
| `tests/admin/test_admin_features.py` | Calls `GET /api/admin/approvals` in a broad admin API smoke. | Expects legacy `pending` and `/api/admin/approvals/{id}` detail behavior, so it does not validate the current `{ approvals, pending_count }` contract. No mutation coverage. |
| `tests/test_e2e_affiliate_full_funnel.py` | Covers separate affiliate admin approval API. | Not the four-eyes approval queue. |
| `tests/test_e2e_affiliate.py` | Uses direct DB override for affiliate approval. | Not the four-eyes approval queue. |
| `tests/test_e2e_kyc_registration.py` | Uses direct DB KYC injection. | Not the four-eyes approval queue. |

Related documentation evidence:

- `docs/page-audits/2026-04-25-admin-approvals.md`
- `docs/route-contract-audits/2026-04-25-route-contract-admin-approvals.md`
- `docs/issue-tracking/page-review-tracker.yml` issues `PAGE-ISSUE-0044` through `PAGE-ISSUE-0048`

## Missing Coverage

The existing test suite does not provide production-grade E2E coverage for:

- Happy path: maker creates `balance.adjust` or `settings.update`, different checker approves, page refreshes, request becomes approved, and DB state plus `audit_logs` match.
- Rejection path: different checker rejects with required reason, page renders rejected state, DB records `rejection_reason`, and audit log is written.
- Four-eyes rule: requester cannot approve or reject their own request.
- Authorization: unauthenticated users, non-admin users, admins without `approvals.manage`, and admins lacking action-specific permissions are rejected server-side.
- CSRF: create, approve, and reject fail without a valid `X-CSRF-Token` and pass with one.
- Validation failures: invalid UUID, mismatched `action_type`/`entity_type`, missing required entity ID, malformed payload JSON, missing reject reason, and unsupported action such as `treasury.payout`.
- Backend state verification: wallet balance cents, wallet transaction rows, platform setting rows, request status, approver ID, and audit-log rows.
- Concurrency/idempotency: two checker approvals against the same pending request result in exactly one committed execution and one rejected/conflict response.
- UI behavior: filters, KPI counts, copy payload, in-flight disabled buttons, reject modal focus/keyboard behavior, error state on failed list API, and mobile layout.
- Console/network failures: no uncaught errors during normal load/mutation; non-2xx JSON responses are visible to admins.

## Suggested Test Files And Names

Primary Playwright/API test file:

- `tests/e2e/test_admin_approvals.py`

Suggested tests:

- `test_admin_approvals_page_requires_admin_session`
- `test_admin_approvals_create_reject_flow_records_reason_and_audit`
- `test_admin_approvals_balance_adjust_requires_different_checker_and_updates_cents_once`
- `test_admin_approvals_reject_modal_keyboard_and_required_reason`
- `test_admin_approvals_validation_blocks_invalid_uuid_and_action_entity_mismatch`
- `test_admin_approvals_post_requires_csrf_token`
- `test_admin_approvals_requires_baseline_and_action_specific_permissions`
- `test_admin_approvals_concurrent_approve_executes_once`
- `test_admin_approvals_list_error_shows_retryable_error_state`

Backend-focused companion test file if Playwright setup is too heavy for the concurrency case:

- `backend/src/admin/approvals_tests.rs`
- `concurrent_balance_adjust_approval_executes_once`
- `approval_requires_action_specific_permission`
- `approval_create_reject_audit_rows_are_transactional`

## Test Data Required

- Maker admin with `approvals.manage` plus the relevant action permission.
- Checker admin with `approvals.manage` plus the relevant action permission.
- Admin role without `approvals.manage`.
- Admin with `approvals.manage` but without a finance/action-specific permission.
- Investor user with a cash wallet starting at a known integer-cent balance.
- Safe platform setting key for `settings.update`.
- Pending approval request fixture for modal/filter tests.
- Helper to clear only automation-created `admin_approval_requests`, wallet transactions, and audit rows.

All financial assertions must use integer cents. Tests should create isolated fixture data and avoid production-like destructive actions such as `user.delete`, live KYC overrides, or real treasury payouts.

## Priority Order

1. Cover authorization, CSRF, four-eyes self-approval/self-rejection blocks, and create/reject validation.
2. Cover one safe financial mutation, preferably `balance.adjust`, with exact cents and audit-log verification.
3. Cover concurrent approval/idempotency for the same pending request.
4. Cover UI filters, KPI counts, copy payload, reject modal keyboard behavior, and mutation in-flight states.
5. Cover list API failure/error UI and mobile smoke.

## Production Adequacy

Coverage is not adequate for production release. The broad admin smoke verifies the page can load, but it does not prove the maker/checker workflow protects money movement, permissions, auditability, or CSRF-sensitive state changes.

Minimum recommended regression suite before release:

- `test_admin_approvals_page_requires_admin_session`
- `test_admin_approvals_create_reject_flow_records_reason_and_audit`
- `test_admin_approvals_balance_adjust_requires_different_checker_and_updates_cents_once`
- `test_admin_approvals_post_requires_csrf_token`
- `test_admin_approvals_requires_baseline_and_action_specific_permissions`
- `test_admin_approvals_concurrent_approve_executes_once`

## Commands Run

Documentation/source audit only. No production application code was modified and no tests were added.

Read/search commands included:

```bash
rg -n "admin/approvals|approvals|approval" tests backend/src frontend/platform docs -g '!target'
rg --files tests backend/src frontend/platform | rg 'approvals|admin.*approv|test_.*approv|approval'
sed -n '1,820p' backend/src/admin/approvals.rs
sed -n '1,460p' frontend/platform/static/js/admin-approvals.js
sed -n '380,590p' frontend/platform/admin/approvals.html
sed -n '500,540p' tests/admin/test_admin_dashboard.py
sed -n '390,435p' tests/admin/test_admin_features.py
```
