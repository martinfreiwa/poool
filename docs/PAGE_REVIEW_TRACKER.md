# Page Review Tracker

## Purpose
This document is the human-readable view of `docs/page-review-tracker.yml`. It tracks every known page route, frontend page template, and supporting HTML template that should be considered during security, accessibility, E2E, and functional reviews.

## How To Update
- Update `docs/page-review-tracker.yml`; do not hand-edit generated tables in this report.
- Add a new page entry whenever a new Axum page route or `frontend/platform` page template is added.
- Set `url_path`, `source_template`, `backend_route`, `related_js`, `related_css`, access level, sensitivity, risk, `last_tested_date`, owner, and notes.
- Record a review by updating the relevant review category with status, reviewer, date, commit SHA, due date, evidence, and findings summary.
- Record an issue under the affected page's `issues` list with the required issue fields.
- Regenerate this report with `python3 scripts/audit_page_review_tracker.py --write-md`.
- Audit without writing with `python3 scripts/audit_page_review_tracker.py`.

## Status Legend
- `not reviewed`
- `reviewed`
- `issues found`
- `fixed`
- `needs recheck`
- `stale`
- `not applicable`

## Severity Legend
- `critical`
- `high`
- `medium`
- `low`
- `info`

## Freshness Policy
- **critical**: Review every 30 days or after any meaningful code change.
- **high**: Review every 60 days or after any meaningful code change.
- **medium**: Review every 90 days or after any meaningful code change.
- **low**: Review every 180 days or after any meaningful code change.
- **Pull requests**: Any page touched in a PR should be checked against this tracker before release.

## Recommended Review Capabilities
| Capability | Status | Purpose |
| --- | --- | --- |
| Browser Use | already installed | Best for opening local pages, clicking through flows, screenshots, responsive checks, and console errors. |
| Playwright/E2E review skill | useful if available | Useful because this tracker needs repeatable page testing and route coverage checks. |
| Accessibility/a11y audit skill | useful if available | Useful for keyboard, focus, labels, contrast, modal/dropdown checks. |
| Security review skill | useful if available | Useful for auth/authorization, CSRF, sensitive-data leaks, upload handling, and admin-route exposure. |

## Review Checklists
### Security Review
- [ ] authentication required where expected
- [ ] authorization and role checks verified server-side
- [ ] CSRF protection checked for state-changing actions
- [ ] no sensitive data leaked into HTML, JS, logs, or URLs
- [ ] all financial logic verified server-side
- [ ] all monetary values handled as integer cents
- [ ] form inputs validated server-side
- [ ] file uploads validated where applicable
- [ ] rate limits or abuse controls considered
- [ ] no unsafe unwraps or panics in production paths

### Accessibility Review
- [ ] keyboard navigation works
- [ ] focus states are visible
- [ ] form fields have labels
- [ ] buttons and links have accessible names
- [ ] color contrast meets expected standards
- [ ] headings are semantic and ordered
- [ ] errors are announced or visible clearly
- [ ] modals/dropdowns are usable with keyboard
- [ ] page works at mobile and desktop sizes

### E2E Review
- [ ] happy path tested
- [ ] important failure states tested
- [ ] authentication/authorization behavior tested
- [ ] redirects tested
- [ ] form validation tested
- [ ] relevant backend state verified
- [ ] no console errors during normal use
- [ ] page works with realistic data

### Functional Review
- [ ] page loads successfully
- [ ] all primary actions work
- [ ] empty/loading/error states work
- [ ] navigation links work
- [ ] server-rendered data is correct
- [ ] client-side JS behavior works
- [ ] responsive layout works
- [ ] no obvious visual regressions

## Audit Snapshot
Generated: 2026-04-25

| Metric | Count |
| --- | --- |
| Discovered page routes | 226 |
| Discovered page templates | 134 |
| Discovered supporting templates | 48 |
| Missing tracker routes | 19 |
| Missing page template entries | 4 |
| Missing supporting template entries | 0 |
| Tracker references to missing files | 0 |
| Pages with stale reviews | 0 |
| Pages with not-reviewed categories | 134 |
| Pages with last_tested_date set | 18 |

## Coverage Summary
| Dimension | Counts |
| --- | --- |
| Business risk | critical: 16, high: 76, low: 36, medium: 25 |
| Access level | admin: 65, authenticated user: 49, developer: 13, public: 23, unknown or needs verification: 2, verified investor: 1 |
| Open issues | 18 |

## Page Inventory
| ID | Name | URL | Last Tested | Access | Sensitivity | Risk | Review Statuses | Stale | Issues |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| admin.admins | Admins | /admin/admins | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 7 |
| admin.affiliate-applications | Affiliate Applications | /admin/affiliate-applications | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 8 |
| admin.affiliate-compliance-route | Affiliate compliance route | /admin/affiliate-compliance | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.affiliate-finance | Affiliate Finance | /admin/affiliate-finance | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.affiliate-fraud-route | Affiliate fraud route | /admin/affiliate-fraud | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.approvals | Approvals | /admin/approvals | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.asset-details | Asset Details | /admin/asset-details | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.asset-tokenize | Asset Tokenize | /admin/asset-tokenize | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.assets | Assets | /admin/assets | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.audit-logs | Audit Logs | /admin/audit-logs | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.blockchain-contract-detail | Blockchain Contract Detail | /admin/blockchain-contract-detail | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.blockchain-contracts | Blockchain Contracts | /admin/blockchain-contracts | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.blockchain-sync | Blockchain Sync | /admin/blockchain-sync | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.blockchain-treasury | Blockchain Treasury | /admin/blockchain-treasury | 2026-04-24 | admin | admin-only data | critical | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 1 |
| admin.community.amas | Community AMAS | /admin/community/amas | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.announcements | Community Announcements | /admin/community/announcements | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.badges | Community Badges | /admin/community/badges | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.challenges | Community Challenges | /admin/community/challenges | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.circle-detail | Community Circle Detail | /admin/community/circle-detail | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.circles | Community Circles | /admin/community/circles | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.comments | Community Comments | /admin/community/comments | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.index | Community Index | /admin/community/ | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.leaderboard | Community Leaderboard | /admin/community/leaderboard | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.post-detail | Community Post Detail | /admin/community/post-detail | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.posts | Community Posts | /admin/community/posts | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.reports | Community Reports | /admin/community/reports | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.user-detail | Community User Detail | /admin/community/user-detail | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.community.users | Community Users | /admin/community/users | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.deposits | Deposits | /admin/deposits | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.developer-submission-review | Developer Submission Review | /admin/developer-submission-review | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.developer-submissions | Developer Submissions | /admin/developer-submissions | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.dividends | Dividends | /admin/dividends | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.email-marketing | Email Marketing | /admin/email-marketing | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.index | Index | /admin/ | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.kyc | KYC | /admin/kyc | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.alerts | Marketplace Alerts | /admin/marketplace/alerts | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.analytics | Marketplace Analytics | /admin/marketplace/analytics | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.approvals | Marketplace Approvals | /admin/marketplace/approvals | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.compliance | Marketplace Compliance | /admin/marketplace/compliance | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.fees | Marketplace Fees | /admin/marketplace/fees | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.index | Marketplace Index | /admin/marketplace/ | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.orderbook | Marketplace Orderbook | /admin/marketplace/orderbook | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.orders | Marketplace Orders | /admin/marketplace/orders | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.marketplace.p2p | Marketplace P2P | /admin/marketplace/p2p | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.primary-escrow | Marketplace Primary Escrow | /admin/marketplace/primary-escrow | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.marketplace.reconciliation | Marketplace Reconciliation | /admin/marketplace/reconciliation | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.marketplace.settings | Marketplace Settings | /admin/marketplace/settings | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.marketplace.trades | Marketplace Trades | /admin/marketplace/trades | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.notifications | Notifications | /admin/notifications | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.orders | Orders | /admin/orders | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.reports | Reports | /admin/reports | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.rewards | Rewards | /admin/rewards | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.roles | Roles | /admin/roles | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.settings | Settings | /admin/settings | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.support | Support | /admin/support | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.support-ticket | Support Ticket | /admin/support-ticket | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.system | System | /admin/system | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.treasury | Treasury | /admin/treasury | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.user-details | User Details | /admin/user-details | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| admin.users | Users | /admin/users | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| affiliate.dashboard | Affiliate dashboard | /affiliate/dashboard | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| affiliate.materials | Affiliate materials | /affiliate/materials | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| affiliate.onboarding | Affiliate onboarding | /affiliate/onboarding | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| affiliate.promo | Affiliate promo | /affiliate | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| affiliate.referrals | Affiliate referrals | /affiliate/referrals | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| affiliate.settings | Affiliate settings | /affiliate/settings | - | authenticated user | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| assets.commodities-marketplace | Commodities marketplace | /commodities-marketplace | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| assets.commodities-tab | Commodities tab fragment | /commodities-marketplace/tab | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| assets.commodity | Commodity detail | /commodity | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| assets.marketplace | Marketplace | /marketplace | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| assets.marketplace-tab | Marketplace tab fragment | /marketplace/tab | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| assets.property | Property detail | /property | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: needs recheck | - | 1 |
| assets.property-public | Public property detail | /p/:slug | - | public | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-2fa | 2FA verify | /auth/2fa | - | authenticated user | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-2fa-setup | 2FA setup | /auth/2fa/setup | - | authenticated user | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-forgot-password | Forgot password | /auth/forgot-password | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-login | Login | /auth/login | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-reset-password | Reset password | /auth/reset-password | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-signup | Signup | /auth/signup | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.auth-verify-email | Verify email | /auth/verify-email | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.google | Google OAuth redirect | /auth/google | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.google-callback | Google OAuth callback | /auth/google/callback | - | public | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| auth.logout | Logout | /auth/logout | - | authenticated user | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| blog.article | Blog article | /blog/:slug | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| blog.category | Blog category | /blog/category/:slug | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| blog.index | Blog index | /blog | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| cart.cart | Cart | /cart | - | authenticated user | financial data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: fixed | - | 1 |
| community.community | Community | /community | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: needs recheck | - | 1 |
| community.partial-announcements | Community announcements partial | /community/partials/announcements/list | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| community.partial-feed | Community feed partial | /community/partials/feed/list | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| community.partial-tab | Community tab partial | /community/partials/:tab | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| community.post | Community post | /community/post/:id | - | authenticated user | personal data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.add-asset | Developer add asset | /developer/add-asset | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.application-form | Developer application form | /developer/application-form | - | authenticated user | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.asset-detail | Developer asset detail | /developer/asset-detail | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.assets | Developer assets | /developer/assets | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.dashboard | Developer dashboard | /developer/dashboard | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.document-upload-step3 | Developer document upload | /developer/document-upload-step3 | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.fragment-assets | Developer assets fragment | /developer/dashboard/fragments/assets | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.fragment-chart | Developer chart fragment | /developer/dashboard/fragments/chart | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.property-content | Developer property content | /developer/property-content | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.root | Developer root redirect | /developer | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.settings | Developer settings | /developer/settings | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.submission-success | Developer submission success | /developer/submission-success | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.submissions | Developer submissions | /developer/submissions | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| developer.support | Developer support | /developer/support | - | developer | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| kyc.identity-verification | Identity verification | /kyc | - | authenticated user | KYC/identity data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| leaderboard.leaderboard | Leaderboard | /leaderboard | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: fixed | - | 1 |
| legal.cookies | Cookies policy | /cookies | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| legal.currency | Currency policy | /currency-policy | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| legal.privacy | Privacy policy | /privacy-policy | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| legal.terms | Terms | /terms | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| marketplace.my-trading | My trading | /my-trading | 2026-04-24 | authenticated user | financial data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| marketplace.secondary | Secondary marketplace | /marketplace-secondary | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: issues found | - | 1 |
| marketplace.tax-report | Tax report | /tax-report | 2026-04-24 | authenticated user | financial data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| marketplace.trade-success | Trade success | /trade-success | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| marketplace.trading-v2 | Marketplace trading V2 | /marketplace-trading-v2 | 2026-04-24 | authenticated user | financial data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| marketplace.trading-v3 | Marketplace trading V3 | /marketplace-trading-v3 | 2026-04-24 | authenticated user | financial data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 2 |
| payments.checkout | Checkout | /checkout | 2026-04-24 | verified investor | financial data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| payments.in-progress | Payment in progress | /payment-in-progress | - | authenticated user | financial data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: fixed | - | 1 |
| payments.success | Payment success | /payment-success | - | authenticated user | financial data | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: fixed | - | 1 |
| platform.profile | Profile | /profile | - | authenticated user | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| platform.root | Platform root redirect | / | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| platform.welcome | Welcome | /welcome | - | authenticated user | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| portfolio.portfolio | Portfolio | /portfolio | - | authenticated user | financial data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| portfolio.transactions | Transactions | /transactions | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| rewards.referral | Referral landing | /rewards/:code | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| rewards.rewards | Rewards | /rewards | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: needs recheck | - | 2 |
| rewards.tier | Tier redirect | /tier | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| rewards.v2 | Rewards V2 | /rewards-v2 | - | authenticated user | public | medium | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| settings.account-deletion | Account deletion | /account-deletion | - | authenticated user | personal data | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| settings.settings | Settings | /settings | - | authenticated user | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.403 | 403 | /403.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.404 | 404 | /404.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.500 | 500 | /500.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.admin-admin-affiliate-fraud | Admin Affiliate Fraud | /admin/admin-affiliate-fraud.html | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.admin-asset-change-requests | Asset Change Requests | /admin/asset-change-requests.html | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.admin-asset-change-review | Asset Change Review | /admin/asset-change-review.html | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.admin-pending-settlements | Pending Settlements | /admin/pending-settlements.html | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.admin-storage | Storage | /admin/storage.html | - | admin | admin-only data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.aml-kyc-policy | Aml KYC Policy | /aml-kyc-policy.html | - | public | KYC/identity data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.commodities-preview | Commodities Preview | /commodities-preview.html | - | unknown or needs verification | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.gdpr-data-request | GDPR Data Request | /gdpr-data-request.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.imprint | Imprint | /imprint.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.index | Index | /index.html | - | unknown or needs verification | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.landing | Landing | /landing.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.maintenance | Maintenance | /maintenance.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.poool-app-home | Poool App Home | /poool_app_home.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| static.poool-app-ssr | Poool App Ssr | /poool_app_ssr.html | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| support.support | Support | /support | - | authenticated user | personal data | high | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: issues found | - | 1 |
| wallet.wallet | Wallet | /wallet | 2026-04-24 | authenticated user | financial data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| www.home | WWW landing page | / | - | public | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| www.platform-redirect | WWW platform redirect | /platform | - | public | public | low | security: not applicable, accessibility: not applicable, e2e: not applicable, functional: not applicable | - | 0 |

## Open Issues
| Page | Issue ID | Category | Severity | Status | Title | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| admin.admins | PAGE-ISSUE-0017 | functional_review | high | open | Admin invitations cannot be accepted end-to-end | unassigned |
| admin.admins | PAGE-ISSUE-0018 | functional_review | high | open | Invite role dropdown can send roles rejected by backend | unassigned |
| admin.admins | PAGE-ISSUE-0019 | security_review | high | open | Admin status update lacks fine-grained authorization and session revocation | unassigned |
| admin.admins | PAGE-ISSUE-0020 | security_review | high | open | Session revocation lacks fine-grained authorization and protected-target guards | unassigned |
| admin.admins | PAGE-ISSUE-0021 | security_review | high | open | Force password reset can report success after ignored DB failures | unassigned |
| admin.admins | PAGE-ISSUE-0022 | security_review | medium | open | Invitation revoke/resend lack durable audit and state-transition handling | unassigned |
| admin.admins | PAGE-ISSUE-0023 | security_review | medium | open | Admin directory staff PII and security posture reads are not audit logged | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0024 | functional_review | high | open | Approve modal inputs are ignored by backend approval handler | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0025 | functional_review | high | open | Pending applications API can silently return empty on DB error | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0026 | functional_review | medium | open | Details modal and KPI cards show incomplete/placeholder data | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0027 | security_review | medium | open | Inline onclick and HTML string rendering increase XSS/injection surface | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0028 | accessibility_review | medium | open | Modals lack baseline dialog accessibility and keyboard handling | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0029 | functional_review | low | open | Unauthenticated admin page GET returns JSON 401 instead of redirecting to login | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0030 | security_review | high | open | Affiliate approval/rejection APIs lack fine-grained permission checks | unassigned |
| admin.affiliate-applications | PAGE-ISSUE-0031 | security_review | medium | open | Reject flow is not transactional with audit logging | unassigned |
| admin.blockchain-treasury | PAGE-ISSUE-0001 | security_review | critical | open | Emergency pause/unpause sends an empty CSRF header |  |
| admin.marketplace.reconciliation | PAGE-ISSUE-0002 | functional_review | high | open | Reconciliation page displays mock mismatch data when the API fails |  |
| marketplace.tax-report | PAGE-ISSUE-0003 | functional_review | low | open | Tax report route requires format despite route comment and path contract |  |

## Ambiguous Or Needs Verification
| ID | Kind | Path/Route | Reason | Follow-up |
| --- | --- | --- | --- | --- |
| missing.design-doc | documentation | docs/DESIGN.md | AGENTS.md requires this file, but it is not present in the current checkout. | Restore or update AGENTS.md to point at the current design reference. |
| platform.profile-template | route/template | /profile -> frontend/platform/profile.html | Route handler exists, but the referenced template is missing. | Confirm whether /profile should be removed, redirected, or backed by a template. |
| admin.affiliate-fraud-template | route/template | /admin/affiliate-fraud -> admin/affiliate-fraud.html | Backend generic route expects admin/affiliate-fraud.html, but the repo has admin/admin-affiliate-fraud.html. | Rename the template or add an explicit route for the existing file. |
| admin.affiliate-compliance-template | route/template | /admin/affiliate-compliance | Backend generic route exists, but no matching template exists. | Add the missing template or remove the route. |
| admin.static-fallback-pages | access-control | /admin/asset-change-requests.html, /admin/asset-change-review.html, /admin/pending-settlements.html, /admin/storage.html | These admin-looking pages have no explicit server-side admin route in backend/src/admin/mod.rs and may rely on static fallback plus client-side guards. | Add server-side admin routes or confirm static fallback exposure is acceptable. |
| static-html-fallback | routing | frontend/platform/**/*.html | platform_router fallback serves files under frontend/platform directly, so some templates may be reachable without their clean Axum route semantics. | Confirm which .html direct URLs are intended public surface and block internal templates if needed. |

## Supporting Templates
| ID | Type | Path | Notes |
| --- | --- | --- | --- |
| template.admin-components-sidebar | admin_component | frontend/platform/admin/components/sidebar.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.admin-templates-icons | admin_component | frontend/platform/admin/templates/icons.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-auth-head | component | frontend/platform/components/auth-head.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-blog-footer | component | frontend/platform/components/blog-footer.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-blog-head | component | frontend/platform/components/blog-head.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-blog-header | component | frontend/platform/components/blog-header.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-developer-assets | component | frontend/platform/components/developer-assets.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-developer-chart | component | frontend/platform/components/developer-chart.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-developer-sidebar-template | component | frontend/platform/components/developer-sidebar-template.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-developer-topbar | component | frontend/platform/components/developer-topbar.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-head | component | frontend/platform/components/head.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-investor-topbar | component | frontend/platform/components/investor-topbar.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-kyc-banner | component | frontend/platform/components/kyc-banner.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-macros | component | frontend/platform/components/macros.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-mobile-kyc-banner | component | frontend/platform/components/mobile-kyc-banner.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-mobile-menu | component | frontend/platform/components/mobile-menu.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-contact | component | frontend/platform/components/property/contact.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-contact-commodity | component | frontend/platform/components/property/contact-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-documents | component | frontend/platform/components/property/documents.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-faq | component | frontend/platform/components/property/faq.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-faq-commodity | component | frontend/platform/components/property/faq-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-funding-timeline | component | frontend/platform/components/property/funding-timeline.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-gallery | component | frontend/platform/components/property/gallery.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-how-it-works | component | frontend/platform/components/property/how-it-works.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-how-it-works-commodity | component | frontend/platform/components/property/how-it-works-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-investment-type | component | frontend/platform/components/property/investment-type.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-leasing-strategy | component | frontend/platform/components/property/leasing-strategy.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-modals | component | frontend/platform/components/property/modals.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-operational-strategy-commodity | component | frontend/platform/components/property/operational-strategy-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-operator-commodity | component | frontend/platform/components/property/operator-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-reviews | component | frontend/platform/components/property/reviews.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-risk-notification | component | frontend/platform/components/property/risk-notification.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-risk-notification-commodity | component | frontend/platform/components/property/risk-notification-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-roadmap-commodity | component | frontend/platform/components/property/roadmap-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-property-security-compliance-commodity | component | frontend/platform/components/property/security-compliance-commodity.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-sidebar | component | frontend/platform/components/sidebar.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-sidebar-developer | component | frontend/platform/components/sidebar-developer.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.components-sidebar-developer-template | component | frontend/platform/components/sidebar-developer-template.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-ama | partial | frontend/platform/partials/community_ama.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-announcements | partial | frontend/platform/partials/community_announcements.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-announcements-list | partial | frontend/platform/partials/community_announcements_list.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-challenges | partial | frontend/platform/partials/community_challenges.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-circle | partial | frontend/platform/partials/community_circle.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-feed | partial | frontend/platform/partials/community_feed.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-post-card | partial | frontend/platform/partials/community_post_card.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.partials-community-post-list | partial | frontend/platform/partials/community_post_list.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.templates-pdf-base | pdf_template | frontend/platform/templates/pdf-base.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
| template.templates-pdf-tax-report | pdf_template | frontend/platform/templates/pdf-tax-report.html | Supporting MiniJinja/static HTML template; review with pages that include or serve it. |
