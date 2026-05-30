# Developer Change Request To Investor Update

Purpose: Verify post-approval asset edits, admin change-request review, and investor-facing readback after approval.

Roles: Developer, Admin, Investor.

Primary pages:
- `/developer/assets`
- `/developer/asset-detail`
- `/developer/submissions`
- `/admin/asset-change-requests`
- `/admin/asset-change-review`
- `/admin/asset-details`
- `/property/:slug`
- `/portfolio`

Prerequisites:
- A published workflow asset exists and is visible to an investor.
- Developer owns the asset.
- Admin can approve/reject asset change requests.
- Investor has either purchased the asset or can view it in marketplace.

Steps:
1. As Investor, open `/property/:slug` and record the current title, summary, media count, document list, financial metrics, and risk text.
2. As Developer, open `/developer/assets`, search for the asset, select it, and open `/developer/asset-detail?id=:asset_id&edit=1`.
3. Change one public content field, one financial/projection field, and one media/document field.
4. Save the edit and verify the page shows pending-change or review-required state.
5. Open `/developer/submissions` and verify the change request is listed with the correct status.
6. As Investor, reload `/property/:slug` and verify unapproved changes are not visible.
7. As Admin, open `/admin/asset-change-requests`, filter/search for the asset, and open `/admin/asset-change-review`.
8. Compare previous and proposed values field by field.
9. Reject one disposable attempted change with a reason, then verify Developer sees the rejection reason and Investor still sees the old approved value.
10. Developer submits a corrected change request.
11. Admin approves the corrected request.
12. Admin opens `/admin/asset-details` and verifies the approved values are now canonical.
13. Investor reloads `/property/:slug` and verifies the approved values are visible.
14. If the investor owns the asset, open `/portfolio` and verify linked asset names/details remain consistent.
15. Verify audit logs include developer submit, admin reject, developer resubmit, and admin approve.

Expected Result:
- Pending changes never leak to investor pages before approval.
- Admin can compare old/new values and reject/approve with durable audit history.
- Approved changes propagate to investor-facing pages and developer/admin asset detail pages.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Content-only change | Investor readback updates after approval only. |
| Financial metric change | Values remain valid and use basis points/cents where applicable. |
| Media/document change | New file links work; removed files disappear after approval only. |
| Rejection | Developer sees reason; public page remains unchanged. |
| Unauthorized edit | Non-owner developer cannot submit changes. |
| Unauthorized approval | Admin without permission gets `403`. |

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
