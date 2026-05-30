# Admin Operations Workflows

Purpose: Verify admin core dashboard, users, KYC, approvals, finance, marketplace, content, reports, storage, RBAC, system, blockchain, support, and audit surfaces that coordinate or approve platform state.

Roles: Admin, Second Admin for maker/checker, Investor/Developer/Public Visitor for readback branches.

Primary pages:
- `/admin/`, `/admin/users`, `/admin/user-details`, `/admin/kyc`, `/admin/support`, `/admin/support-ticket`, `/admin/approvals`, `/admin/audit-logs`, `/admin/reports`, `/admin/settings`, `/admin/system`, `/admin/storage`, `/admin/admins`, `/admin/roles`
- `/admin/developer-submissions`, `/admin/developer-submission-review`, `/admin/assets`, `/admin/asset-details`, `/admin/asset-change-requests`, `/admin/asset-change-review`, `/admin/asset-tokenize`
- `/admin/orders`, `/admin/deposits`, `/admin/treasury`, `/admin/rewards`, `/admin/dividends`, `/admin/pending-settlements`
- `/admin/marketplace/`, `/admin/marketplace/alerts`, `/admin/marketplace/analytics`, `/admin/marketplace/approvals`, `/admin/marketplace/compliance`, `/admin/marketplace/fees`, `/admin/marketplace/orderbook`, `/admin/marketplace/orders`, `/admin/marketplace/p2p`, `/admin/marketplace/primary-escrow`, `/admin/marketplace/reconciliation`, `/admin/marketplace/settings`, `/admin/marketplace/trades`
- `/admin/blog`, `/admin/blog-editor`, `/admin/blog-persona`, `/admin/blog-strategy`, `/admin/email-marketing`, `/admin/notifications`
- `/admin/blockchain-treasury`, `/admin/blockchain-contracts`, `/admin/blockchain-contract-detail`, `/admin/blockchain-sync`, `/admin/templates/icons`

Backend/API surfaces:
- `backend/src/admin/mod.rs` and domain modules under `backend/src/admin/**`.
- `backend/src/payments`, `backend/src/kyc`, `backend/src/support`, `backend/src/marketplace`, `backend/src/developer`, `backend/src/rewards`, and `backend/src/storage` for admin-mediated actions.
- Community admin surfaces are referenced in `docs/workflows/community/README.md` and `docs/workflows/cross-role/community-report-admin-appeal.md`.

Prerequisites:
- Admin account has the minimum permission for each module; use a second admin when maker/checker applies.
- Target records are disposable `Workflow Test` users/assets/orders/deposits/tickets/reports/content.
- Starting counts and balances are recorded before financial approvals.
- Outbound email/notification actions are run only in local/staging-safe mode unless explicitly approved.

Steps:
1. Open dashboard and core admin pages; verify metrics, refresh/search/filter/export controls, user detail links, loading/empty/error states, and unauthorized direct URL behavior.
2. Run KYC queue: inspect documents, approve, reject, request updates, review AML state, then reload investor checkout eligibility.
3. Run support: filter tickets, assign, reply, change status, resolve, reopen branch, and verify user CSAT readback.
4. Run developer/asset approvals: review submissions, upload admin documents/images, edit content, request changes, approve/reject, tokenize/publish, and verify developer/investor readback.
5. Run orders/deposits/treasury/rewards/dividends: filter/export queues, approve/reject/cancel/confirm, calculate distributions, execute/cancel payouts, and verify cents reconciliation.
6. Run marketplace admin: refresh/rebuild orderbook, cancel/reconcile/settle orders/trades/P2P/escrow, edit fees/settings, save/share views, export, and inspect compliance reports.
7. Run content/notification/email: draft, save, publish/send/test, archive/unpublish/revert, and reload public/investor surfaces.
8. Run RBAC/admins/system/storage/blockchain: invite/revoke admins, edit roles, inspect health/storage links, trigger safe sync/pin/pause/unpause operations, and verify audit logs.
9. For every admin mutation, reload the submitting role page and audit log before continuing.

Expected Result:
- Admin modules mutate only authorized records, generate audit logs, and produce role-specific readback after reload.
- Financial/admin approvals are idempotent, use integer cents, and keep wallet/order/treasury/commission/dividend rows reconciled.
- Upload and storage admin views show scoped links without leaking private object paths.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Queue filters/exports | Admin lists remain searchable, filterable, paginated, and exportable. |
| Approval/rejection | Target user/developer/investor state changes after reload and audit row exists. |
| Financial approval | Balances and transactions reconcile once in cents. |
| Marketplace settlement | Orderbook/trade/escrow/reconciliation state is consistent after retry/rebuild. |
| Content publish | Public/investor readback updates only after publish and reverts on cleanup. |
| RBAC/system | Unauthorized admin gets `403`; system actions are audited and safe-mode aware. |

Negative Cases:
- Admin lacks permission or attempts direct API call outside role.
- Duplicate approval/settlement/payout/confirm action.
- Missing rejection notes where required.
- Invalid upload type/size, private document link exposed, or storage object missing.
- Export on empty filters and network failure during save/send.
- Live/outbound mutation attempted without explicit approval.

Audit / DB / Financial Checks:
- Every admin mutation writes audit log actor, action, target, prior/new state, timestamp, and notes with sensitive-data redaction.
- All order/deposit/withdrawal/treasury/reward/dividend values are `BIGINT` cents and wrapped in DB transactions where multi-table.
- Admin upload metadata includes owner, target record, MIME type, size, storage key, and access scope.
- Maker/checker actions record both maker and approver where applicable.

Cleanup:
- Revert platform settings, role changes, admin invitations, content publishes, and marketplace settings.
- Cancel or reverse disposable financial records through approved local cleanup paths.
- Delete test uploads/exports where policy allows; retain audit logs unless the environment is disposable.
