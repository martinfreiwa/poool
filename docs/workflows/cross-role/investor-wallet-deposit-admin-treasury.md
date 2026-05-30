# Investor Wallet Deposit To Admin Treasury Approval

Purpose: Verify investor deposit creation, admin deposit review, treasury effects, and investor wallet/transaction readback.

Roles: Investor, Admin.

Primary pages:
- `/wallet`
- `/wallet/deposit`
- `/transactions`
- `/transactions/:id`
- `/admin/deposits`
- `/admin/treasury`
- `/admin/audit-logs`

Prerequisites:
- Investor account is active and KYC state is known.
- Admin can manage deposits and treasury.
- Use a small disposable amount in cents.

Steps:
1. As Investor, open `/wallet` and record starting available and held balances.
2. Open deposit modal/action.
3. Submit an invalid amount and verify validation blocks it.
4. Submit a valid manual/bank-transfer deposit request.
5. Verify wallet shows pending deposit state or redirects back with success query state.
6. Open `/transactions` and verify pending deposit is visible if the UI exposes it.
7. As Admin, open `/admin/deposits`.
8. Search/filter for the investor or deposit reference.
9. Open the deposit action/detail modal.
10. Verify amount, currency, provider, reference, evidence/proof, user, and timestamp.
11. Cancel one disposable deposit and verify the investor sees cancelled/failed state.
12. Create a second valid deposit as Investor.
13. As Admin, confirm the deposit.
14. Verify `/admin/treasury` reflects the cash movement.
15. As Investor, reload `/wallet` and verify available balance increased exactly once.
16. Open `/transactions/:id` and verify detail matches the deposit.
17. Verify audit logs attribute confirmation to the admin, not the depositor.
18. Try confirming the same deposit again and verify it is blocked or idempotent.

Expected Result:
- Deposits move pending -> paid/cancelled with correct actor attribution.
- Wallet balance changes exactly once and only after admin confirmation.
- Treasury and wallet transaction rows match in integer cents.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Invalid amount | Rejected before deposit request creation. |
| Pending deposit | Visible to investor/admin with consistent reference. |
| Cancel deposit | No wallet credit; status is cancelled. |
| Confirm deposit | Wallet and treasury update once. |
| Duplicate confirm | No duplicate credit. |
| Unauthorized admin | `403` for deposit management. |

Backend/API surfaces:
- See `docs/workflows/WORKFLOW_COVERAGE_MATRIX.md` for the complete route-to-workflow mapping.
- Mutating APIs used by this workflow must be verified for authorization, validation, idempotency where applicable, and reload/readback across roles.


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
