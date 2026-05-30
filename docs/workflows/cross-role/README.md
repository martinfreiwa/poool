# Cross-Role Workflows

These workflows test how POOOL behaves when investors, developers, admins, and public visitors interact with the same business object across multiple pages.

Use this folder when a feature cannot be verified from one role alone. Examples: a developer submits an asset, an admin reviews it, then an investor buys it; an investor creates a support ticket, an admin replies, then the investor rates the resolution.

Do not store passwords, API keys, session cookies, payment proof files with real bank data, or downloaded personal data in this folder.

## Standard Test Accounts

Use disposable local or staging accounts with clear labels:

| Role | Required state |
|------|----------------|
| Public visitor | No active session. |
| Investor | Email verified, KYC state controlled by the workflow, wallet balance controlled by the workflow. |
| Developer | Developer role enabled, allowed to create assets and operations reports. |
| Admin | Has the exact permissions needed for the workflow under test. Avoid using a super-admin unless the workflow is explicitly testing global admin behavior. |
| Second admin | Required for four-eyes or maker/checker flows. |
| Affiliate/team member | Required for referral and developer affiliate-team workflows. |

## Execution Rules

1. Run workflows against local fixtures first. Use staging/live only after the local workflow is deterministic and cleanup is documented.
2. Prefix created records with `Workflow Test` plus a timestamp so they can be found and removed.
3. Keep all money in integer cents and verify database rows use cents, not floats.
4. For every mutating action, verify the actor, target record, audit log, user-visible state, and rollback/cleanup path.
5. For every upload, verify file type, size validation, storage record, visible link, download/open behavior, and failed-upload state.
6. For every admin approval/rejection, verify the submitting user sees the new state after reload.
7. For every role boundary, repeat one direct URL/API attempt with an unauthorized user and expect `401`, redirect to login, or explicit `403`.
8. Never run destructive, financial, or outbound-message actions against real users unless the user explicitly approves that exact live/staging action.

## Workflow Index

Run these in order for a broad cross-role release pass:

1. [Developer Asset To Investor Purchase](./developer-asset-to-investor-purchase.md)
2. [Developer Change Request To Investor Update](./developer-change-request-to-investor-update.md)
3. [Developer Operations To Dividends And Investor Portfolio](./developer-operations-to-dividends-investor-portfolio.md)
4. [Investor Wallet Deposit To Admin Treasury Approval](./investor-wallet-deposit-admin-treasury.md)
5. [Investor Withdrawal With Step-Up And Admin Review](./investor-withdrawal-step-up-admin-review.md)
6. [Affiliate Referral To Investor Checkout And Payout](./affiliate-referral-checkout-payout.md)
7. [Developer Affiliate Team Lifecycle](./developer-affiliate-team-lifecycle.md)
8. [Investor Support To Admin Resolution](./support-investor-developer-admin-resolution.md)
9. [KYC To Investment Limit And Checkout Eligibility](./kyc-investment-limit-checkout.md)
10. [Secondary Trading To Admin Marketplace Settlement](./secondary-trading-admin-settlement.md)
11. [Community Report To Admin Moderation And Appeal](./community-report-admin-appeal.md)
12. [Admin Content Publish To Public And Investor Readback](./admin-content-publish-readback.md)
13. [Account Security And Data Rights](./account-security-data-rights-admin-audit.md)
14. [Live Read-Only Confidence Pass](./live-read-only-confidence-pass.md)

## Local Run Evidence

- 2026-05-29: [Cross-role workflow run](./cross-role-workflow-run-2026-05-29.md) executed the hybrid user lifecycle browser/E2E path and fixed the issues found during the run.

## Page Coverage Map

| Page surface | Covered by workflow |
|--------------|---------------------|
| `/`, `/id/`, `/p/:slug`, `/blog`, `/blog/:slug`, legal pages | Admin Content Publish; Live Read-Only Confidence; KYC Checkout Eligibility |
| `/auth/login`, `/auth/signup`, `/auth/google`, `/auth/forgot-password`, `/auth/reset-password`, `/auth/verify-email`, `/auth/2fa`, `/auth/2fa/setup`, `/auth/2fa/step-up`, `/welcome`, `/logout` | Account Security; KYC Checkout Eligibility; Wallet Withdrawal Step-Up; Affiliate Referral Checkout |
| `/marketplace`, `/marketplace/tab`, `/commodities-marketplace`, `/commodities-marketplace/tab`, `/property/:slug`, `/commodity/:slug` | Developer Asset To Investor Purchase; KYC Checkout Eligibility; Live Read-Only Confidence |
| `/cart`, `/cart/add`, `/cart/update`, `/cart/remove`, `/checkout`, `/payment-in-progress`, `/payment-success` | Developer Asset To Investor Purchase; Affiliate Referral Checkout; KYC Checkout Eligibility |
| `/wallet`, `/wallet/deposit`, `/wallet/withdraw` | Wallet Deposit; Wallet Withdrawal Step-Up; Affiliate Referral Checkout |
| `/portfolio`, `/transactions`, `/transactions/:id`, `/tax-report`, `/leaderboard` | Developer Asset To Investor Purchase; Operations To Dividends; Secondary Trading Settlement |
| `/settings`, `/settings/notifications/community`, `/account-deletion` | Account Security; Community Report And Appeal |
| `/support`, `/developer/support`, `/admin/support`, `/admin/support-ticket` | Investor Support To Admin Resolution |
| `/rewards`, `/rewards-v2`, `/rewards/:code`, `/r/:code`, `/affiliate/*` | Affiliate Referral Checkout; Developer Affiliate Team Lifecycle |
| `/marketplace-secondary`, `/marketplace-trading-v2`, `/marketplace-trading-v3`, `/my-trading`, `/trade-success` | Secondary Trading To Admin Marketplace Settlement |
| `/developer`, `/developer/dashboard`, `/developer/onboarding`, `/developer/assets`, `/developer/add-asset`, `/developer/application-form`, `/developer/document-upload-step3`, `/developer/property-content`, `/developer/submission-success`, `/developer/submissions`, `/developer/asset-detail` | Developer Asset To Investor Purchase; Developer Change Request |
| `/developer/operations`, `/developer/villas/:asset_id/operations/new`, `/developer/villas/:asset_id/annual/:year` | Developer Operations To Dividends |
| `/developer/ranking`, `/developer/settings` | Developer Asset To Investor Purchase; Developer Change Request |
| `/developer/affiliate-team`, `/developer/affiliate-team/members`, `/developer/affiliate-team/customers`, `/developer/affiliate-team/products`, `/developer/affiliate-team/settings`, `/developer/affiliate-team/analytics`, `/developer/affiliate-team/tier` | Developer Affiliate Team Lifecycle |
| `/admin/`, `/admin/users`, `/admin/user-details`, `/admin/kyc`, `/admin/approvals`, `/admin/audit-logs`, `/admin/reports`, `/admin/settings`, `/admin/system`, `/admin/storage`, `/admin/admins`, `/admin/roles` | KYC Checkout Eligibility; Account Security; Live Read-Only Confidence |
| `/admin/developer-submissions`, `/admin/developer-submission-review`, `/admin/assets`, `/admin/asset-details`, `/admin/asset-change-requests`, `/admin/asset-tokenize` | Developer Asset To Investor Purchase; Developer Change Request |
| `/admin/orders`, `/admin/deposits`, `/admin/treasury`, `/admin/rewards`, `/admin/dividends` | Wallet Deposit; Affiliate Referral Checkout; Operations To Dividends |
| `/admin/marketplace/*` | Secondary Trading To Admin Marketplace Settlement |
| `/admin/blog`, `/admin/blog-editor`, `/admin/blog-persona`, `/admin/blog-strategy`, `/admin/email-marketing`, `/admin/notifications` | Admin Content Publish To Public And Investor Readback |
| `/community/*`, `/admin/community/*` | Community Report To Admin Moderation And Appeal plus existing Community workflows. |

## Evidence To Capture

For every workflow run, record:

| Evidence | Required detail |
|----------|-----------------|
| Run metadata | Date, environment, commit SHA, database name, backend URL, browser/device viewport. |
| Test actors | Role labels only, never passwords or session cookies. |
| Created records | Asset IDs, order IDs, ticket IDs, report IDs, transaction IDs, and cleanup status. |
| UI proof | Final visible status on each role page after reload. |
| Browser health proof | Console errors and failed network requests for every loaded page, especially dashboards and pages that bootstrap data from route parameters. |
| API/database proof | Key rows and amounts, especially status transitions and integer-cent money values. |
| Audit proof | Audit log actor, action, target, timestamp, and sensitive-data redaction. |
| Cleanup | Exact records deleted, reverted, cancelled, or intentionally retained. |
