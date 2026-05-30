# Developer Workflows

Purpose: Cover developer-only workflows and page-level edge cases beyond the cross-role asset, operations, and affiliate-team business flows.

Roles: Developer, Admin for review/readback, Investor for public listing readback.

Primary pages:
- `/developer`, `/developer/onboarding`, `/developer/dashboard`, `/developer/assets`, `/developer/add-asset`, `/developer/application-form`, `/developer/document-upload-step3`, `/developer/property-content`, `/developer/submission-success`, `/developer/submissions`, `/developer/asset-detail`
- `/developer/operations`, `/developer/villas/:asset_id/operations/new`, `/developer/villas/:asset_id/annual/:year`
- `/developer/ranking`, `/developer/settings`, `/developer/support`
- `/developer/affiliate-team`, `/developer/affiliate-team/members`, `/developer/affiliate-team/customers`, `/developer/affiliate-team/products`, `/developer/affiliate-team/settings`, `/developer/affiliate-team/analytics`, `/developer/affiliate-team/tier`

Backend/API surfaces:
- Developer application, drafts, document/image uploads, draft submit/duplicate/delete, change requests, operations, annual data, CapEx, forecasts, developer settings, support, affiliate team APIs.

Prerequisites:
- Developer account exists with role and owned assets in draft, submitted, approved, and live states.
- Admin account can review developer submissions/changes/operations.
- Upload fixtures include valid and invalid images/documents.

Steps:
1. Verify `/developer` redirects to `/developer/dashboard` and non-developer access is blocked.
2. Verify dashboard KPIs, chart/assets fragments, review banner dismiss, top-performing/attention rows, and navigation.
3. Run developer onboarding with valid, missing, duplicate, and invalid personal/portfolio values.
4. Verify asset table search/tabs/selection, preview, view/edit links, empty states, and row action authorization.
5. Run asset wizard save/exit/next/previous, currency formatting, required validation, document/image upload/remove/reorder, media/video/projection validation, submit/tokenize handoff, submission success, duplicate/delete/resubmit.
6. Verify asset detail view/edit modes, pending-change panels, investors/financials/documents/images/milestones, public-content edits, and change-request status.
7. Run operations dashboard filters/year/matrix, monthly report draft/submit, custom expenses, upload evidence, locked state, annual CapEx/forecast/document upload, and admin review readback.
8. Verify developer ranking load, filters/navigation, empty/error states, and role visibility.
9. Verify developer settings identity/logo/profile/public links/security parity, authorization, and investor public listing readback.
10. Verify developer support ticket lifecycle through admin support workflow.
11. Verify affiliate-team overview, members, customers, products, settings, analytics, tier, CSV/PDF exports, invitation/approval/removal, public join URL, banking validation, and admin finance readback.

Expected Result:
- Developer pages handle draft, submitted, approved, rejected, empty, unauthorized, and error states.
- Developer changes are never public until approved by Admin where required.

Coverage Matrix:

| Area | Expected Result |
|------|-----------------|
| Dashboard/nav | Role-gated dashboard and fragments load. |
| Onboarding | Developer application creates reviewable state. |
| Assets/submissions | Drafts, uploads, submissions, and changes persist. |
| Operations/annual | Reports and evidence are reviewable by Admin. |
| Settings/support | Developer identity and support context persist. |
| Affiliate team | Team attribution, exports, analytics, and finance readback work. |

Negative Cases:
- Non-developer access, missing required fields, invalid uploads, unauthorized asset ownership, duplicate draft submit, admin rejection, invalid banking/slug, removed team member attribution, and failed chart/API loads.

Audit / DB / Financial Checks:
- Draft/asset/operation rows, storage objects, audit logs, and public asset readback match.
- Money-like fields are integer cents or basis points, never floats.

Cleanup:
- Remove disposable drafts/assets/team invites/uploads or leave them with `Workflow Test` prefix and documented status.

## Local Run Evidence

- 2026-05-29: [Developer workflow run](./developer-workflow-run-2026-05-29.md) fixed and verified affiliate-team bank-edit step-up handling, annual-data mobile stacking, onboarding pending redirect, submissions search/bulk controls, operations draft edit hydration, and support create/reply/reopen/rate-limit coverage. Clean Developer full block passed: `89 passed, 2 skipped`.
