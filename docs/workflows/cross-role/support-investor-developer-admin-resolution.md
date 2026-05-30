# Investor And Developer Support To Admin Resolution

Purpose: Verify support ticket lifecycle across investor/developer contexts and admin response.

Roles: Investor, Developer, Admin.

Primary pages:
- `/support`
- `/developer/support`
- `/admin/support`
- `/admin/support-ticket`
- `/admin/audit-logs`

Prerequisites:
- Investor and developer test accounts exist.
- Admin can manage support tickets.
- Test attachment files exist.

Steps:
1. As Investor, open `/support`.
2. Search FAQ and verify filtering does not alter ticket state.
3. Create a ticket with subject, category, priority, message, and valid attachment.
4. Try an invalid/oversized attachment and verify the ticket is either not created or clearly warns that attachment failed.
5. Reload `/support` and verify ticket appears with correct status and attachment.
6. As Developer, open `/developer/support` and create a developer-context ticket.
7. Verify developer ticket is labeled or scoped to developer context.
8. As Admin, open `/admin/support`.
9. Filter by status/category/priority and find both tickets.
10. Open investor ticket via `/admin/support-ticket`.
11. Assign the ticket, reply, change status, and inspect attachment.
12. Verify Investor sees admin reply and status update.
13. Investor replies, then Admin sees the new response.
14. Admin closes the ticket.
15. Investor reopens it, then rates resolution with CSAT after final close.
16. Repeat admin reply/status path for developer-context ticket.
17. Verify audit log records admin assignment/status/reply actions without leaking attachment secrets.

Expected Result:
- Investor and developer support flows share reliable ticket mechanics but preserve context.
- Admin can triage, reply, close, and the user can reopen/rate.
- Attachment success/failure is explicit and atomic where required.

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
