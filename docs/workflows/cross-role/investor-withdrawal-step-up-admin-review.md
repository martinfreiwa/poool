# Investor Withdrawal With Step-Up And Admin Review

Purpose: Verify sensitive withdrawal behavior, 2FA step-up, admin review, and wallet/transaction effects.

Roles: Investor, Admin.

Primary pages:
- `/wallet`
- `/wallet/withdraw`
- `/auth/2fa/setup`
- `/auth/2fa/step-up`
- `/transactions`
- `/admin/treasury`
- `/admin/approvals`
- `/admin/audit-logs`

Prerequisites:
- Investor has a funded wallet and a valid payment method.
- 2FA test secret configuration is available locally.
- Admin can review withdrawal or approval requests.

Steps:
1. As Investor, open `/wallet` and record available balance.
2. Verify add-bank/add-card controls and payment method selection.
3. If no payment method exists, add a disposable valid method and verify validation for invalid fields.
4. Start a small withdrawal below any high-value threshold.
5. Verify submission creates a pending withdrawal or clear success state.
6. Start a high-value withdrawal that requires step-up.
7. Verify redirect to `/auth/2fa/step-up` or setup flow if 2FA is not enrolled.
8. Complete `/auth/2fa/setup` with the test authenticator secret when needed.
9. Submit an invalid 2FA code and verify rejection.
10. Submit a valid 2FA code and continue the withdrawal.
11. Verify wallet held/available balances reflect pending withdrawal correctly.
12. As Admin, open `/admin/approvals` or `/admin/treasury` depending the implemented review surface.
13. Verify withdrawal details, payment method, user identity, KYC state, amount, and step-up timestamp.
14. Reject one disposable withdrawal and verify Investor sees rejected/cancelled state and funds return.
15. Submit a second valid withdrawal and approve it.
16. Verify Investor `/wallet` and `/transactions/:id` show completed withdrawal.
17. Verify treasury and audit log records show admin actor and no sensitive payment details leak.

Expected Result:
- Sensitive withdrawals require fresh 2FA step-up.
- Rejection restores funds; approval debits exactly once.
- Admin review and audit trail are complete.

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
