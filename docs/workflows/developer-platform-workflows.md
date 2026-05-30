# Developer Platform Workflows

Purpose: Verify developer dashboard, onboarding, asset management, submission wizard, operations reporting, annual data, ranking, settings, support, and developer affiliate-team entrypoints before or alongside cross-role approval workflows.

Roles: Developer; Admin and Investor for referenced cross-role readback.

Primary pages:
- `/developer`, `/developer/onboarding`, `/developer/dashboard`, `/developer/dashboard/fragments/chart`, `/developer/dashboard/fragments/assets`
- `/developer/assets`, `/developer/add-asset`, `/developer/application-form`, `/developer/document-upload-step3`, `/developer/property-content`, `/developer/submission-success`, `/developer/submissions`, `/developer/asset-detail`
- `/developer/operations`, `/developer/villas/:asset_id/operations/new`, `/developer/villas/:asset_id/annual/:year`, `/developer/ranking`, `/developer/settings`, `/developer/support`
- `/developer/affiliate-team`, `/developer/affiliate-team/members`, `/developer/affiliate-team/customers`, `/developer/affiliate-team/products`, `/developer/affiliate-team/settings`, `/developer/affiliate-team/analytics`, `/developer/affiliate-team/tier`

Backend/API surfaces:
- `backend/src/developer/*`, including routes, drafts, change requests, villa operations, annual data, and affiliate team APIs.
- `backend/src/storage` for developer document/media upload paths.
- Cross-role references: `developer-asset-to-investor-purchase.md`, `developer-change-request-to-investor-update.md`, `developer-operations-to-dividends-investor-portfolio.md`, `developer-affiliate-team-lifecycle.md`, and `support-investor-developer-admin-resolution.md`.

Prerequisites:
- Developer account has developer role and controlled approval state.
- Disposable asset/report/team fixtures use `Workflow Test` timestamped names.
- Valid image, PDF, DOC/DOCX/ZIP, and invalid upload samples are available.
- Admin account can review submissions, change requests, operations reports, CapEx/forecast suggestions, and affiliate teams.

Steps:
1. Open `/developer`; verify redirect to dashboard, shell navigation, review banner state, KPI cards, chart/assets fragments, empty/error states, and asset row links.
2. Run developer onboarding with missing required fields, valid personal/portfolio data, review screen, submit, reload, and dashboard readback.
3. Run asset management table: search, status tabs, row focus, view/edit actions, empty Add first asset, and submissions table filtering/sorting/select/bulk-delete for drafts.
4. Run add-asset wizard: select only supported asset type, complete property/financial fields, save and reload draft, upload required documents, reject invalid uploads, delete/re-upload, add listing content/media/projections, save, and submit.
5. Run asset detail edit/change-request branch for approved assets; verify pending change panels, admin approval requirement, and investor readback through cross-role workflow.
6. Run operations dashboard and monthly report; enter revenue, nights, expense rows, custom expenses, notes, document uploads, save draft, submit for approval, and verify locked state.
7. Run annual data page; submit CapEx amount in IDR cents, forecast bps/cents fields, upload annual document, and verify admin queue/readback.
8. Run developer ranking/settings/support; verify read-only ranking, developer identity/link persistence, support ticket lifecycle, and role authorization.
9. Run affiliate-team pages; invite member, approve/remove, filter customers, export customers/products/members, edit slug/banking settings, change analytics range/resolution, export CSV/PDF, and review tier page.
10. For banking settings, verify sensitive bank-account/IBAN edits return HTTP `428` step-up 2FA before update, and do not fall through as generic validation errors.
11. For annual data on mobile, verify the CapEx/forecast/document grid computes to a single-column layout and long UUID/document fields remain usable.

Expected Result:
- Developer can manage drafts, submissions, reporting, settings, support, and team attribution within ownership boundaries.
- Admin-dependent final states are verified by cross-role workflows after role switch and reload.
- Uploads and financial fields enforce type/size/cents/bps constraints.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Dashboard/shell | Developer pages load with correct nav, fragments, empty/error states, and role gating. |
| Onboarding | Valid application is saved; incomplete or unauthorized submit is blocked. |
| Asset draft/submission | Draft persists, required documents/media validate, submit moves to review. |
| Change request | Approved-asset edits require admin approval before investor readback changes. |
| Operations/annual data | Reports, CapEx, forecast, and documents enter admin review with cents/bps intact. |
| Settings/support | Developer-specific profile/link/ticket data persists and audits where required. |
| Affiliate team | Invitations, attribution views, exports, settings, analytics, and tier pages remain consistent. |

Negative Cases:
- Non-developer direct URL/API access.
- Missing asset required fields, unsupported asset type, invalid currency/cents input.
- Invalid document/media upload type/size and orphan upload rollback.
- Non-owner edit/delete/resubmit attempt.
- Operations submit without saved draft/documents where required.
- Affiliate team slug collision, invalid bank data, removing owner, or moving member without permission.

Audit / DB / Financial Checks:
- Asset purchase/minimum share values, operations revenue/expenses, CapEx, forecasts, commissions, and payouts are stored in integer cents or basis points, never floats.
- Developer/admin mutating actions write audit logs with actor, target, prior status, new status, and notes.
- Upload records include type, size, storage key/link, owner, document category, and failed-upload cleanup.
- Cross-role reload verifies developer/admin/investor state after every approval or rejection.

Cleanup:
- Delete or archive `Workflow Test` drafts/assets/reports/teams where safe.
- Remove uploaded test files and exported artifacts.
- Revert developer profile/link/support changes and clear pending invitations.
