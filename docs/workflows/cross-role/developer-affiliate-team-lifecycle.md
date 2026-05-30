# Developer Affiliate Team Lifecycle

Purpose: Verify developer affiliate-team creation/management, member invitation, public join behavior, attribution, analytics, exports, and admin oversight.

Roles: Developer Team Owner, Team Member, Public Visitor, Investor, Admin.

Primary pages:
- `/developer/affiliate-team`
- `/developer/affiliate-team/members`
- `/developer/affiliate-team/customers`
- `/developer/affiliate-team/products`
- `/developer/affiliate-team/settings`
- `/developer/affiliate-team/analytics`
- `/developer/affiliate-team/tier`
- `/affiliate-team-invitation-accept` when available
- `/admin/affiliate-teams`
- `/admin/affiliate-finance`

Prerequisites:
- Developer has affiliate-team owner privileges.
- A registered user can be invited as team member.
- Purchasable asset exists.

Steps:
1. As Developer Owner, open `/developer/affiliate-team`.
2. Verify KPIs, payout pipeline, trend chart, funnel chart, members-at-risk, member breakdown, and asset breakdown.
3. Open `/developer/affiliate-team/settings`.
4. Change team display name and public slug using a workflow prefix.
5. Test invalid slug/bank values and verify validation.
6. Save valid settings and reload.
7. Copy/open public join URL and verify it does not expose private owner data.
8. Open `/developer/affiliate-team/members`.
9. Invite a registered user by email.
10. Verify invitation token/link preview and outbound email state if locally captured.
11. As Team Member, accept invitation and verify team membership.
12. As Owner, approve pending member if approval is required.
13. Remove a disposable pending/active member and verify future attribution is blocked or no longer assigned to that member.
14. Generate a team/business referral link for the active member.
15. As Public Visitor, use the link, sign up, and complete checkout.
16. As Owner, open `/developer/affiliate-team/customers` and filter by member.
17. Open `/developer/affiliate-team/products` and verify purchased asset/product revenue.
18. Open `/developer/affiliate-team/analytics`; change date presets, custom range, and day/week/month resolution.
19. Export CSV and PDF/print where supported.
20. Open `/developer/affiliate-team/tier` and verify tier/progression reflects team activity.
21. As Admin, open `/admin/affiliate-teams` and verify team lifecycle state, members, links, and revenue.
22. Open `/admin/affiliate-finance` and verify payout routing goes to team owner.

Expected Result:
- Team settings persist and public join/invite flows work.
- Member attribution drives customers/products/analytics/tier.
- Admin finance can distinguish team owner payout from member attribution.

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
