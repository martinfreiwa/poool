# Developer Operations To Dividends And Investor Portfolio

Purpose: Verify monthly/annual developer reporting, admin operational review, dividend creation, and investor portfolio/transaction readback.

Roles: Developer, Admin, Investor.

Primary pages:
- Developer: `/developer/operations`, `/developer/villas/:asset_id/operations/new`, `/developer/villas/:asset_id/annual/:year`, `/developer/asset-detail`
- Admin: `/admin/assets`, `/admin/asset-details`, `/admin/treasury`, `/admin/dividends`, `/admin/reports`, `/admin/audit-logs`
- Investor: `/portfolio`, `/transactions`, `/transactions/:id`, `/tax-report`, `/property/:slug`

Prerequisites:
- A published asset exists with at least one investor holding.
- Developer owns the asset.
- Admin can review operations and create/approve dividends.
- Test evidence files exist.

Steps:
1. As Developer, open `/developer/operations`.
2. Verify year tabs, filter tabs, urgent-submission banner, monthly matrix, mobile cards, and report links.
3. Open a missing or draft month via `/developer/villas/:asset_id/operations/new?period=YYYY-MM`.
4. Capture browser console errors and failed network requests; fail the workflow on unexpected `4xx/5xx` requests.
5. Verify the page derives `asset_id` from the URL and does not call `/api/developer/villas/:asset_id/operations/operations`.
6. Enter gross rental, nights available/booked, standard expenses, at least one custom expense, and notes.
7. Verify live calculations update ADR, occupancy, OpEx, net, reserves, platform/withholding, and distributable estimates.
8. Save draft and reload the page.
9. Verify every field and custom expense persisted.
10. Upload required evidence documents, remove one queued file, then upload it again.
11. Submit for approval.
12. Verify submitted state locks fields or clearly prevents unsaved mutation.
13. Open `/developer/operations` and verify the month status moved to submitted/review.
14. As Admin, open the related admin asset/operations review surface from `/admin/assets` or `/admin/asset-details`.
15. Verify revenue, occupancy, expenses, custom expenses, computed amounts, evidence documents, and developer notes.
16. Reject the report once with a disposable correction note if the UI supports revision.
17. As Developer, revise and resubmit.
18. As Admin, approve the operations report.
19. As Developer, open `/developer/villas/:asset_id/annual/:year`.
20. Submit one CapEx event, one forecast suggestion, and one annual tax/report document.
21. As Admin, verify annual data appears in the relevant admin review/reporting surface.
22. Open `/admin/dividends`.
23. Calculate a distribution for the approved reporting period.
24. Verify eligible holdings, total distributable amount, withholding/platform amounts, and per-investor cents.
25. Approve and execute the dividend using a disposable local fixture.
26. Open `/admin/treasury` and verify cash movement and ledger rows.
27. As Investor, open `/portfolio` and verify income/performance moved as expected.
28. Open `/transactions` and `/transactions/:id` for the dividend transaction.
29. Generate/download `/tax-report` for the relevant year and verify the dividend/tax data appears.
30. Verify `/property/:slug` live performance data reflects the approved operations/forecast where applicable.
31. Verify audit logs for developer submit, admin approve, dividend calculate, dividend approve, and dividend execute.

Expected Result:
- Operations reports are draftable, submitted, reviewable, revisable, and approvable.
- Dividend calculations are based on approved data only.
- Investor portfolio, transactions, and tax report reflect executed dividends.
- Audit and treasury records line up with integer-cent financial movements.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Save draft | Developer can reload without data loss. |
| Submit without documents | Blocked or clearly marked incomplete. |
| Admin rejection | Developer can revise; investor sees no dividend. |
| Approved report | Admin can calculate dividend from approved period. |
| Dividend execution | Investor transaction and treasury ledger match exactly. |
| Duplicate execution | Second execution is blocked or idempotent. |

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
