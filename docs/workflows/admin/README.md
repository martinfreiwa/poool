# Admin Workflows

Purpose: Cover admin-only operational workflows and page-level edge cases that are not fully represented by cross-role business flows.

Roles: Admin, Second Admin for maker/checker, Developer/Investor for readback.

Primary pages:
- `/admin/`, `/admin/users`, `/admin/user-details`, `/admin/kyc`, `/admin/support`, `/admin/support-ticket`
- `/admin/developer-submissions`, `/admin/developer-submission-review`, `/admin/assets`, `/admin/asset-details`, `/admin/asset-change-requests`, `/admin/asset-tokenize`
- `/admin/orders`, `/admin/deposits`, `/admin/treasury`, `/admin/rewards`, `/admin/dividends`, `/admin/approvals`
- `/admin/audit-logs`, `/admin/reports`, `/admin/notifications`, `/admin/settings`, `/admin/system`, `/admin/storage`, `/admin/admins`, `/admin/roles`
- `/admin/blog`, `/admin/blog-editor`, `/admin/blog-persona`, `/admin/blog-strategy`, `/admin/email-marketing`
- `/admin/blockchain-treasury`, `/admin/blockchain-contracts`, `/admin/blockchain-contract-detail`, `/admin/blockchain-sync`, `/admin/pending-settlements`
- `/admin/marketplace/*`, `/admin/community/*`, `/admin/affiliate-*`, `/admin/affiliate-teams`

Backend/API surfaces:
- Admin user/role/RBAC, KYC, support, submissions/assets, finance, orders/deposits/withdrawals, rewards/affiliate, dividends, approvals, reports, notification/email, settings/system/storage, marketplace, blockchain, villa ops, community.

Prerequisites:
- Admin accounts with distinct permissions.
- Disposable user, investor, developer, asset, order, deposit, ticket, community, and marketplace fixtures.

Steps:
1. Verify admin root metrics/cards/charts/alerts, quick links, search, and role-gated navigation.
2. Verify users list/detail search/filter/export, profile/status/roles/sessions/password reset/investment limit actions, and investor readback.
3. Verify KYC approve/reject/request-update and AML/compliance rescreen flows.
4. Verify support assignment/status/reply/bulk actions and user readback.
5. Verify developer submissions, asset edit/publish/feature/funding/document/image/milestone actions, change requests, and tokenization handoff.
6. Verify orders, deposits, withdrawals, treasury, rewards, affiliate finance, dividends, and approvals with maker/checker and audit trails.
7. Verify marketplace orderbook/orders/trades/P2P/escrow/reconciliation/settings/alerts/compliance/fees operations and investor trading readback.
8. Verify community admin workflows by referencing `docs/workflows/community/README.md`.
9. Verify blog/content/email/notification/report/storage/system/settings/admins/roles pages, exports, test sends, maintenance actions, RBAC denial, and audit logs.
10. Verify blockchain treasury/contracts/detail/sync/pending settlements actions in local/staging-only mode.

Expected Result:
- Admin pages enforce least privilege, mutate only intended records, and leave auditable state.
- User/developer/investor-facing pages reflect approved admin changes after reload.

Coverage Matrix:

| Area | Expected Result |
|------|-----------------|
| Core/users/RBAC | Permissions and user actions are scoped and auditable. |
| Compliance/support | Queues update and users see outcomes. |
| Assets/developer | Review/publication/change states are consistent. |
| Finance | Money mutations use transactions and integer cents. |
| Marketplace/blockchain | Settlement and operational actions are traceable. |
| Content/system | Publish/send/export/maintenance actions are authorized. |

Negative Cases:
- Admin without permission, duplicate approval, stale queue item, invalid UUID, missing rejection reason, failed export, failed upload, disabled unsupported action, destructive action cancellation, and live-production mutation without explicit approval.

Audit / DB / Financial Checks:
- Every admin mutation records actor, target, timestamp, action, and safe metadata.
- Balance/order/deposit/dividend/commission changes reconcile exactly in cents.

Cleanup:
- Revert disposable admin changes, cancel test jobs/orders, remove test content, close test tickets, and document intentionally retained audit records.

Run History:
- 2026-05-29: [Admin/Ops workflow run](./admin-ops-workflow-run-2026-05-29.md) verified dashboard, deposits, KYC, dividends, support, notifications, reports, settings, and layout browser coverage. The run fixed default-team upsert races, affiliate rollup conflict keys, Circle Ops partial-index conflict predicates, dividend rejection log levels, and local-only reconciliation/invariant noise.
