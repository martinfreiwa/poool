# Affiliate Referral To Investor Checkout And Payout

Purpose: Verify referral attribution from link click through signup, checkout, commission creation, affiliate dashboard readback, and admin finance/payout review.

Roles: Public Visitor, Investor, Affiliate, Developer Team Owner when applicable, Admin.

Primary pages:
- `/r/:code`
- `/rewards/:code`
- `/auth/signup`
- `/affiliate/dashboard`
- `/affiliate/referrals`
- `/affiliate/materials`
- `/affiliate/settings`
- `/rewards`
- `/checkout`
- `/admin/rewards`
- `/admin/affiliate-finance`
- `/admin/affiliate-applications`
- `/admin/audit-logs`

Prerequisites:
- One personal/private affiliate link exists.
- One developer team/business affiliate link exists if testing team attribution.
- A purchasable asset and funded investor checkout path exist.
- Admin can review affiliate applications and finance.

Steps:
1. As Public Visitor, open `/r/:code?subid=workflow-test&utm_source=workflow`.
2. Verify redirect to signup and referral cookie/state is set without exposing secrets.
3. Create a new investor account through `/auth/signup`.
4. Verify signup attribution creates a registered referral for the correct link.
5. Complete email/KYC prerequisites needed for checkout.
6. Add an asset to cart and complete `/checkout`.
7. Verify order completion creates a commission with gross amount, provisional amount, currency, link ID, attribution user, payout user, and status.
8. As Affiliate, open `/affiliate/dashboard` and verify counters changed by exactly the expected cents.
9. Open `/affiliate/referrals` and verify referred user/conversion appears with filters/export if available.
10. Open `/affiliate/settings` and verify payout/tax/postback settings are present; save disposable payout settings if safe.
11. As Admin, open `/admin/rewards` and verify commission/campaign/tier data.
12. Open `/admin/affiliate-finance` and verify payout eligibility, holdback/status, currency, and audit details.
13. Approve or mark payout using a disposable local fixture.
14. Verify affiliate dashboard payout status changes after reload.
15. Repeat steps 1-14 with a developer team/business link.
16. Verify business link separates attribution user from payout user and updates developer team analytics.

Expected Result:
- Referral click and signup attribution survive checkout.
- Commission cents, currency, link type, attribution user, and payout user are correct.
- Affiliate/admin/developer team surfaces agree.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Personal link | Attribution user and payout user are the same. |
| Team link | Attribution user is member; payout user is team owner. |
| Invalid code | Redirects safely without attribution. |
| Self-referral | Blocked or flagged according to fraud rules. |
| Checkout failure | No paid commission is created. |
| Duplicate checkout | One commission only. |

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
