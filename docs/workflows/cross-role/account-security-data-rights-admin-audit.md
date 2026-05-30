# Account Security And Data Rights

Purpose: Verify account security changes, data export/deletion request, admin auditability, and role boundaries.

Roles: Investor, Developer, Admin.

Primary pages:
- `/settings`
- `/account-deletion`
- `/auth/2fa/setup`
- `/auth/2fa`
- `/auth/2fa/step-up`
- `/auth/forgot-password`
- `/auth/reset-password`
- `/admin/users`
- `/admin/user-details`
- `/admin/audit-logs`
- `/admin/reports`

Prerequisites:
- Disposable investor and developer accounts exist.
- 2FA test configuration is available.
- Admin can inspect users and audit logs.

Steps:
1. As Investor, open `/settings`.
2. Change safe profile/preferences/social fields and verify persistence after reload.
3. Download data export and verify JSON parses and has expected top-level keys without raw secrets.
4. Start 2FA setup, submit invalid code, then valid code.
5. Log out and verify `/auth/2fa` challenge appears on next login if required.
6. Trigger `/auth/forgot-password`, use local/staging reset token, and complete `/auth/reset-password`.
7. Verify old password no longer works and new password does.
8. Start `/account-deletion` but cancel before final destructive submit.
9. If explicitly running disposable deletion, submit deletion request and verify account is disabled/anonymized according to policy.
10. As Admin, open `/admin/users` and `/admin/user-details`.
11. Verify user security/KYC/settings context, but no raw password/secret is visible.
12. Open `/admin/audit-logs` and verify profile export, 2FA, password reset, and deletion-request events.
13. Open `/admin/reports` and verify exports require the correct permissions.
14. Repeat key settings checks as Developer through `/developer/settings` if developer-specific identity/link fields are in scope.

Expected Result:
- Security-sensitive changes require correct authentication and are auditable.
- Data export works without secret leakage.
- Deletion is discoverable, cancellable, and only executed with explicit disposable-account approval.

Backend/API surfaces:
- See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for the complete route-to-workflow mapping.
- Mutating APIs used by this workflow must be verified for authorization, validation, idempotency where applicable, and reload/readback across roles.


Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Happy path | The workflow reaches the final cross-role state and every role sees the expected state after reload. |
| Authorization boundary | Non-owner or wrong-role direct page/API access returns login redirect, `401`, or explicit `403`. |
| Validation failure | Missing, malformed, stale, duplicate, or out-of-state input is rejected without partial persistence. |
| Audit/readback | Mutating action writes expected audit/DB rows and the next role sees the update only after reload. |
| Cleanup | Disposable `Workflow Test` data can be reverted, archived, or intentionally retained with a note. |

Negative Cases:
- Unauthorized direct page/API access by each non-owner role.
- Missing required fields, invalid state transition, duplicate submit, stale record, and network failure.
- For uploads, invalid file type, oversize file, missing storage object, and inaccessible download link.
- For financial flows, malformed amount, insufficient balance, duplicate approval/settlement, and cents mismatch.


Audit / DB / Financial Checks:
- Verify every admin action writes an audit row with actor, action, target, timestamp, prior/new state where available, and redacted sensitive values.
- Verify all monetary values are stored as integer cents (`BIGINT`/`i64`) and any percentage values use basis points where modeled that way.
- Verify multi-table financial writes are transactional and duplicate submits are idempotent or explicitly blocked.
- Verify uploaded files record MIME type, size, owner/target, storage key/link, access scope, success state, and failed-upload cleanup.
- After every cross-role transition, reload the new role's page and verify the visible state from the database/API, not stale client state.


Cleanup:
- Revert or archive every `Workflow Test` record created by this workflow using approved local cleanup paths.
- Remove temporary uploaded files and downloaded artifacts where policy allows.
- Restore account, wallet, role, feature-flag, notification, and content settings changed during the run.
- Retain audit logs unless the environment is fully disposable and the cleanup runbook explicitly truncates them.
