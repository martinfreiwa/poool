# KYC To Investment Limit And Checkout Eligibility

Purpose: Verify how authentication, KYC, admin review, investment limits, and checkout eligibility interact.

Roles: Public Visitor, Investor, Admin.

Primary pages:
- `/auth/signup`
- `/auth/verify-email`
- `/welcome`
- `/kyc`
- `/settings`
- `/marketplace`
- `/property/:slug`
- `/cart`
- `/checkout`
- `/admin/kyc`
- `/admin/users`
- `/admin/user-details`

Prerequisites:
- KYC provider can run locally or manual review fallback is enabled.
- Admin can review KYC and update annual investment limits.
- A purchasable asset exists.

Steps:
1. As Public Visitor, sign up through `/auth/signup`.
2. Verify email verification state through `/auth/verify-email`.
3. Open `/welcome` and choose KYC/start verification path.
4. Open `/kyc` and submit the minimum test-safe identity data or manual review request.
5. As Admin, open `/admin/kyc`.
6. Find the submitted KYC record and inspect identity/document/AML state.
7. Reject once with an update request and verify Investor sees actionable rejection.
8. Investor updates KYC and resubmits.
9. Admin approves KYC.
10. Investor opens `/settings` and verifies identity/KYC state is reflected.
11. Admin opens `/admin/users` and `/admin/user-details` for the investor.
12. Set a low annual investment limit.
13. Investor opens `/marketplace`, adds an asset amount above the limit to cart, and opens `/checkout`.
14. Verify checkout rejects over-limit purchase with a clear message.
15. Admin raises or clears the limit.
16. Investor retries checkout with an allowed amount and verifies eligibility passes.
17. Verify audit logs for KYC reject/approve and limit update.

Expected Result:
- KYC state controls checkout eligibility.
- Annual investment limits block and allow checkout correctly.
- Investor/admin views agree after reload.

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
