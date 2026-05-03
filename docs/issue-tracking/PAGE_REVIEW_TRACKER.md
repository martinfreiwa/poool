# Page Review Tracker

## Purpose
This document is the human-readable view of `docs/issue-tracking/page-review-tracker.yml`. It tracks every known page route, frontend page template, and supporting HTML template that should be considered during security, accessibility, E2E, and functional reviews.

## How To Update
- Update `docs/issue-tracking/page-review-tracker.yml`; do not hand-edit generated tables in this report.
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
Generated: 2026-04-28

| Metric | Count |
| --- | --- |
| Discovered page routes | 233 |
| Discovered page templates | 135 |
| Discovered supporting templates | 48 |
| Missing tracker routes | 0 |
| Missing page template entries | 0 |
| Missing supporting template entries | 0 |
| Tracker references to missing files | 0 |
| Pages with stale reviews | 0 |
| Pages with not-reviewed categories | 44 |
| Pages with last_tested_date set | 117 |

## Coverage Summary
| Dimension | Counts |
| --- | --- |
| Business risk | critical: 16, high: 80, low: 42, medium: 25 |
| Access level | admin: 70, authenticated user: 54, developer: 13, public: 23, unknown or needs verification: 2, verified investor: 1 |
| Open issues | 75 |

## Page Inventory
| ID | Name | URL | Last Tested | Access | Sensitivity | Risk | Review Statuses | Stale | Issues |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| admin.admins | Admins | /admin/admins | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, needs runtime recheck, e2e: needs recheck, functional: issues found | - | 8 |
| admin.affiliate-applications | Affiliate Applications | /admin/affiliate-applications | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, needs runtime recheck, e2e: reviewed, functional: fixed | - | 12 |
| admin.affiliate-compliance-route | Affiliate compliance route | /admin/affiliate-compliance | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: not applicable, e2e: reviewed, functional: fixed | - | 2 |
| admin.affiliate-finance | Affiliate Finance | /admin/affiliate-finance | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 7 |
| admin.affiliate-fraud-route | Affiliate fraud route | /admin/affiliate-fraud | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, needs browser recheck, functional: fixed | - | 5 |
| admin.approvals | Approvals | /admin/approvals | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs browser recheck, functional: fixed | - | 5 |
| admin.asset-details | Asset Details | /admin/asset-details | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 7 |
| admin.asset-tokenize | Asset Tokenize | /admin/asset-tokenize | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 8 |
| admin.assets | Assets | /admin/assets | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| admin.audit-logs | Audit Logs | /admin/audit-logs | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| admin.blockchain-contract-detail | Blockchain Contract Detail | /admin/blockchain-contract-detail | 2026-04-26 | admin | admin-only data | critical | security: passed, accessibility: passed, e2e: passed, functional: passed | - | 9 |
| admin.blockchain-contracts | Blockchain Contracts | /admin/blockchain-contracts | 2026-04-28 | admin | admin-only data | critical | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 5 |
| admin.blockchain-sync | Blockchain Sync | /admin/blockchain-sync | 2026-04-28 | admin | admin-only data | critical | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.blockchain-treasury | Blockchain Treasury | /admin/blockchain-treasury | 2026-04-28 | admin | admin-only data | critical | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 1 |
| admin.community.amas | Community AMAS | /admin/community/amas | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.community.announcements | Community Announcements | /admin/community/announcements | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.community.badges | Community Badges | /admin/community/badges | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.community.challenges | Community Challenges | /admin/community/challenges | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.community.circle-detail | Community Circle Detail | /admin/community/circle-detail | 2026-04-25 | admin | admin-only data | high | security: needs recheck, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 5 |
| admin.community.circles | Community Circles | /admin/community/circles | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| admin.community.comments | Community Comments | /admin/community/comments | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 8 |
| admin.community.index | Community Index | /admin/community/ | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.community.leaderboard | Community Leaderboard | /admin/community/leaderboard | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 9 |
| admin.community.post-detail | Community Post Detail | /admin/community/post-detail | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 6 |
| admin.community.posts | Community Posts | /admin/community/posts | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 1 |
| admin.community.reports | Community Reports | /admin/community/reports | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 8 |
| admin.community.user-detail | Community User Detail | /admin/community/user-detail | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 3 |
| admin.community.users | Community Users | /admin/community/users | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.deposits | Deposits | /admin/deposits | 2026-04-28 | admin | admin-only data | critical | security: fixed, needs runtime recheck, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.developer-submission-review | Developer Submission Review | /admin/developer-submission-review | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, needs browser recheck, e2e: static regression covered, runtime blocked, functional: fixed | - | 5 |
| admin.developer-submissions | Developer Submissions | /admin/developer-submissions | 2026-04-27 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 5 |
| admin.dividends | Dividends | /admin/dividends | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: fixed, needs runtime recheck, e2e: fixed, needs runtime recheck, functional: fixed | - | 8 |
| admin.email-marketing | Email Marketing | /admin/email-marketing | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.index | Index | /admin/ | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.kyc | KYC | /admin/kyc | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 7 |
| admin.marketplace.alerts | Marketplace Alerts | /admin/marketplace/alerts | 2026-04-25 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 7 |
| admin.marketplace.analytics | Marketplace Analytics | /admin/marketplace/analytics | 2026-04-25 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 8 |
| admin.marketplace.approvals | Marketplace Approvals | /admin/marketplace/approvals | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: needs recheck, e2e: fixed, functional: fixed | - | 8 |
| admin.marketplace.compliance | Marketplace Compliance | /admin/marketplace/compliance | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| admin.marketplace.fees | Marketplace Fees | /admin/marketplace/fees | - | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 9 |
| admin.marketplace.index | Marketplace Index | /admin/marketplace/ | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.marketplace.orderbook | Marketplace Orderbook | /admin/marketplace/orderbook | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.marketplace.orders | Marketplace Orders | /admin/marketplace/orders | 2026-04-28 | admin | admin-only data | critical | security: needs recheck, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 5 |
| admin.marketplace.p2p | Marketplace P2P | /admin/marketplace/p2p | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 8 |
| admin.marketplace.primary-escrow | Marketplace Primary Escrow | /admin/marketplace/primary-escrow | 2026-04-28 | admin | admin-only data | critical | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.marketplace.reconciliation | Marketplace Reconciliation | /admin/marketplace/reconciliation | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.marketplace.settings | Marketplace Settings | /admin/marketplace/settings | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 10 |
| admin.marketplace.trades | Marketplace Trades | /admin/marketplace/trades | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.notifications | Notifications | /admin/notifications | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.orders | Orders | /admin/orders | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.reports | Reports | /admin/reports | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 7 |
| admin.rewards | Rewards | /admin/rewards | 2026-04-26 | admin | admin-only data | high | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 3 |
| admin.roles | Roles | /admin/roles | 2026-04-26 | admin | admin-only data | high | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.settings | Settings | /admin/settings | 2026-04-26 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.support | Support | /admin/support | 2026-04-26 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.support-ticket | Support Ticket | /admin/support-ticket | 2026-04-26 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| admin.system | System | /admin/system | 2026-04-26 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 2 |
| admin.treasury | Treasury | /admin/treasury | 2026-04-24 | admin | admin-only data | critical | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| admin.user-details | User Details | /admin/user-details | 2026-04-28 | admin | admin-only data | high | security: fixed, accessibility: needs recheck, e2e: needs browser recheck, functional: fixed | - | 2 |
| admin.users | Users | /admin/users | 2026-04-26 | admin | admin-only data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 2 |
| affiliate.dashboard | Affiliate dashboard | /affiliate/dashboard | 2026-04-28 | authenticated user | personal data | medium | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 5 |
| affiliate.materials | Affiliate materials | /affiliate/materials | 2026-04-28 | authenticated user | personal data | medium | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| affiliate.onboarding | Affiliate onboarding | /affiliate/onboarding | 2026-04-28 | authenticated user | personal data | medium | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 8 |
| affiliate.promo | Affiliate promo | /affiliate | 2026-04-28 | authenticated user | personal data | medium | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 5 |
| affiliate.referrals | Affiliate referrals | /affiliate/referrals | 2026-04-28 | authenticated user | personal data | medium | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| affiliate.settings | Affiliate settings | /affiliate/settings | 2026-04-28 | authenticated user | personal data | high | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 7 |
| assets.commodities-marketplace | Commodities marketplace | /commodities-marketplace | 2026-04-28 | authenticated user | public | medium | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| assets.commodities-tab | Commodities tab fragment | /commodities-marketplace/tab | 2026-04-28 | authenticated user | public | medium | security: reviewed, accessibility: fixed, e2e: fixed, functional: fixed | - | 4 |
| assets.commodity | Commodity detail | /commodity | 2026-04-28 | authenticated user | public | medium | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| assets.marketplace | Marketplace | /marketplace | 2026-04-28 | authenticated user | public | medium | security: reviewed, accessibility: fixed, E2E verified, e2e: E2E verified, functional: fixed | - | 5 |
| assets.marketplace-tab | Marketplace tab fragment | /marketplace/tab | 2026-04-28 | authenticated user | public | medium | security: reviewed, accessibility: fixed, E2E verified, e2e: E2E verified, functional: fixed | - | 4 |
| assets.property | Property detail | /property | 2026-04-28 | authenticated user | public | medium | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 5 |
| assets.property-public | Public property detail | /p/:slug | 2026-04-28 | public | public | medium | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 6 |
| auth.auth-2fa | 2FA verify | /auth/2fa | 2026-04-28 | authenticated user | personal data | low | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 4 |
| auth.auth-2fa-setup | 2FA setup | /auth/2fa/setup | 2026-04-28 | authenticated user | personal data | low | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 7 |
| auth.auth-forgot-password | Forgot password | /auth/forgot-password | 2026-04-28 | public | personal data | low | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 5 |
| auth.auth-login | Login | /auth/login | 2026-04-28 | public | personal data | low | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 3 |
| auth.auth-reset-password | Reset password | /auth/reset-password | 2026-04-28 | public | personal data | low | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 4 |
| auth.auth-signup | Signup | /auth/signup | 2026-04-28 | public | personal data | low | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 6 |
| auth.auth-verify-email | Verify email | /auth/verify-email | 2026-04-28 | public | personal data | low | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 3 |
| auth.google | Google OAuth redirect | /auth/google | 2026-04-28 | public | personal data | low | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 2 |
| auth.google-callback | Google OAuth callback | /auth/google/callback | 2026-04-28 | public | personal data | low | security: fixed, accessibility: not applicable, e2e: fixed, functional: fixed | - | 3 |
| auth.logout | Logout | /auth/logout | 2026-04-28 | authenticated user | personal data | low | security: fixed, accessibility: needs recheck, e2e: needs recheck, functional: fixed | - | 2 |
| blog.article | Blog article | /blog/:slug | 2026-04-28 | public | public | low | security: fixed, accessibility: fixed, e2e: fixed, needs browser coverage, functional: fixed | - | 5 |
| blog.category | Blog category | /blog/category/:slug | 2026-04-28 | public | public | low | security: fixed, accessibility: fixed, e2e: fixed, functional: fixed | - | 3 |
| blog.index | Blog index | /blog | 2026-04-28 | public | public | low | security: reviewed, accessibility: fixed, e2e: fixed, functional: fixed | - | 3 |
| cart.cart | Cart | /cart | 2026-04-27 | authenticated user | financial data | high | security: issues found, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 4 |
| community.community | Community | /community | 2026-04-28 | authenticated user | personal data | medium | security: reviewed, accessibility: reviewed, e2e: needs recheck, functional: reviewed | - | 7 |
| community.partial-announcements | Community announcements partial | /community/partials/announcements/list | 2026-04-28 | authenticated user | personal data | medium | security: reviewed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 4 |
| community.partial-feed | Community feed partial | /community/partials/feed/list | 2026-04-28 | authenticated user | personal data | medium | security: reviewed, accessibility: fixed, e2e: fixed, functional: fixed | - | 5 |
| community.partial-tab | Community tab partial | /community/partials/:tab | 2026-04-28 | authenticated user | personal data | medium | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 4 |
| community.post | Community post | /community/post/:id | 2026-04-28 | authenticated user | personal data | medium | security: needs recheck, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 6 |
| developer.add-asset | Developer add asset | /developer/add-asset | 2026-04-27 | developer | personal data | high | security: reviewed, accessibility: issues found, e2e: needs recheck, functional: reviewed | - | 1 |
| developer.application-form | Developer application form | /developer/application-form | 2026-04-25 | authenticated user | personal data | high | security: needs recheck, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 3 |
| developer.asset-detail | Developer asset detail | /developer/asset-detail | 2026-04-27 | developer | personal data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 2 |
| developer.assets | Developer assets | /developer/assets | 2026-04-27 | developer | personal data | high | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| developer.dashboard | Developer dashboard | /developer/dashboard | 2026-04-28 | developer | personal data | high | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: needs recheck | - | 0 |
| developer.document-upload-step3 | Developer document upload | /developer/document-upload-step3 | 2026-04-28 | developer | personal data | high | security: reviewed, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 2 |
| developer.fragment-assets | Developer assets fragment | /developer/dashboard/fragments/assets | 2026-04-28 | developer | personal data | high | security: issues found, accessibility: needs recheck, e2e: issues found, functional: needs recheck | - | 1 |
| developer.fragment-chart | Developer chart fragment | /developer/dashboard/fragments/chart | 2026-04-28 | developer | personal data | high | security: issues found, accessibility: needs recheck, e2e: issues found, functional: needs recheck | - | 1 |
| developer.property-content | Developer property content | /developer/property-content | 2026-04-28 | developer | personal data | high | security: reviewed, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 2 |
| developer.root | Developer root redirect | /developer | 2026-04-28 | developer | personal data | high | security: reviewed, accessibility: not applicable, e2e: reviewed, functional: reviewed | - | 0 |
| developer.settings | Developer settings | /developer/settings | 2026-04-28 | developer | personal data | high | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| developer.submission-success | Developer submission success | /developer/submission-success | 2026-04-28 | developer | personal data | high | security: reviewed, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 1 |
| developer.submissions | Developer submissions | /developer/submissions | 2026-04-28 | developer | personal data | high | security: fixed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 3 |
| developer.support | Developer support | /developer/support | 2026-04-28 | developer | personal data | high | security: fixed, accessibility: fixed, e2e: fixed, needs runtime recheck, functional: fixed | - | 7 |
| kyc.identity-verification | Identity verification | /kyc | 2026-04-28 | authenticated user | KYC/identity data | high | security: issues found, accessibility: needs recheck, e2e: needs recheck, functional: issues found | - | 9 |
| leaderboard.leaderboard | Leaderboard | /leaderboard | 2026-04-28 | authenticated user | public | medium | security: reviewed, accessibility: fixed, e2e: needs recheck, functional: fixed | - | 5 |
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
| admin.blog | Blog CMS | /admin/blog | 2026-04-27 | admin | admin-only data | high | security: reviewed, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 2 |
| admin.blog-editor | Blog Editor | /admin/blog-editor | 2026-04-27 | admin | admin-only data | high | security: issues found, accessibility: issues found, e2e: needs recheck, functional: issues found | - | 3 |
| admin.blog-persona | Blog Persona | /admin/blog-persona | 2026-04-27 | admin | admin-only data | high | security: reviewed, accessibility: issues found, e2e: needs recheck, functional: reviewed | - | 1 |
| admin.blog-strategy | Blog Strategy | /admin/blog-strategy | 2026-04-27 | admin | admin-only data | high | security: reviewed, accessibility: issues found, e2e: needs recheck, functional: reviewed | - | 1 |
| admin.templates-icons | Admin Icon Templates | /admin/templates/icons | - | admin | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| design.fonts-template | Fonts Template | /fonts-template.html | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| design.forms-template | Forms Template | /forms-template.html | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| design.overlays-template | Overlays Template | /overlays-template.html | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| design.statistics-template | Statistics Template | /statistics-template.html | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
| design.table-template | Table Template | /table-template.html | - | authenticated user | public | low | security: not reviewed, accessibility: not reviewed, e2e: not reviewed, functional: not reviewed | - | 0 |
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
| admin.admins | PAGE-ISSUE-0023 | security_review | medium | open | Admin directory staff PII and security posture reads are not audit logged | unassigned |
| admin.community.post-detail | PAGE-ISSUE-0427 | security_review | high | open | Post detail APIs lack community permission checks | unassigned |
| admin.community.post-detail | PAGE-ISSUE-0148 | security_review | high | open | Admin post detail renderer injects community fields as HTML | unassigned |
| admin.community.post-detail | PAGE-ISSUE-0149 | functional_review | high | open | Post moderation mutations are non-transactional and weakly audited | unassigned |
| admin.community.post-detail | PAGE-ISSUE-0150 | functional_review | medium | open | Tag update accepts unvalidated arbitrary tag arrays | unassigned |
| admin.community.post-detail | PAGE-ISSUE-0151 | accessibility_review | medium | open | Moderation actions rely on prompt alert confirm flows | unassigned |
| admin.community.post-detail | PAGE-ISSUE-0152 | security_review | medium | open | Admin post detail loads unused external CDN scripts | unassigned |
| admin.community.posts | PAGE-ISSUE-0139 | security_review | high | open | Community posts admin table injects unescaped user content | unassigned |
| admin.email-marketing | PAGE-ISSUE-0560 | security_review | high | open | Email campaign and template changes lack audit, approval, and abuse controls | unassigned |
| admin.kyc | PAGE-ISSUE-0160 | security_review | high | open | KYC routes lack KYC-specific permission gates | unassigned |
| admin.kyc | PAGE-ISSUE-0161 | security_review | high | open | KYC document signed URLs are overbroad and best-effort audited | unassigned |
| admin.kyc | PAGE-ISSUE-0162 | security_review | high | open | KYC decisions lack audit logs and atomic side effects | unassigned |
| admin.kyc | PAGE-ISSUE-0163 | functional_review | medium | open | KYC backend failures render as empty states | unassigned |
| admin.kyc | PAGE-ISSUE-0164 | security_review | medium | open | Document viewer injects signed URL data with innerHTML | unassigned |
| admin.kyc | PAGE-ISSUE-0165 | accessibility_review | medium | open | KYC modals and sort controls lack keyboard semantics | unassigned |
| admin.kyc | PAGE-ISSUE-0166 | functional_review | medium | open | Rejection reason validation is client-side only | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0167 | functional_review | high | open | Alert API failures render fake operational alerts | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0168 | security_review | high | open | Alert rows render database text with innerHTML | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0169 | security_review | high | open | Marketplace alert routes lack permission gates | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0170 | security_review | high | open | Alert actions lack audit logs and missing-row checks | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0171 | functional_review | medium | open | Alert list database failures return empty data | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0172 | functional_review | medium | open | Alert status transition semantics are weak | unassigned |
| admin.marketplace.alerts | PAGE-ISSUE-0173 | accessibility_review | medium | open | Alerts page lacks loading empty and error states | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0205 | security_review | high | open | Fee management routes do not enforce marketplace.manage | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0206 | functional_review | high | open | Fee controls show success without persistence | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0207 | functional_review | high | open | Fee list API masks database failures as empty state | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0208 | functional_review | high | open | Fee resolver ignores accepted developer fee scope | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0209 | functional_review | medium | open | Active fee configuration validation is ambiguous | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0210 | security_review | medium | open | Stored fee data renders through raw HTML | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0211 | security_review | medium | open | Fee mutations are not audit logged | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0212 | functional_review | medium | open | Settlement and minimum fee fields lack backend support | unassigned |
| admin.marketplace.fees | PAGE-ISSUE-0213 | accessibility_review | low | open | Fee page tabs lack accessible tab semantics | unassigned |
| admin.marketplace.orders | PAGE-ISSUE-0528 | security_review | high | needs recheck | Marketplace orders APIs lack granular permission checks | unassigned |
| admin.marketplace.orders | PAGE-ISSUE-0529 | security_review | high | needs recheck | Admin order cancel is not locked or audited | unassigned |
| admin.marketplace.orders | PAGE-ISSUE-0530 | functional_review | high | needs recheck | Open orders page renders mock financial orders on API failure | unassigned |
| admin.marketplace.orders | PAGE-ISSUE-0531 | security_review | high | needs recheck | Open order rows render backend values through innerHTML | unassigned |
| admin.marketplace.orders | PAGE-ISSUE-0532 | functional_review | medium | needs recheck | Marketplace orders pagination is not reachable | unassigned |
| admin.marketplace.reconciliation | PAGE-ISSUE-0002 | functional_review | high | open | Reconciliation page displays mock mismatch data when the API fails |  |
| admin.rewards | PAGE-ISSUE-0279 | security_review | high | open | Rewards management mutations lack granular permissions | unassigned |
| admin.rewards | PAGE-ISSUE-0280 | functional_review | medium | open | Rewards affiliate approval button calls an unregistered endpoint | unassigned |
| admin.rewards | PAGE-ISSUE-0281 | security_review | medium | open | Rewards application links do not restrict URL schemes | unassigned |
| admin.roles | PAGE-ISSUE-0282 | functional_review | medium | open | Roles page falls back to demo data instead of showing authorization failure | unassigned |
| admin.settings | PAGE-ISSUE-0283 | security_review | high | open | Platform settings and maintenance actions lack granular authorization | unassigned |
| admin.support | PAGE-ISSUE-0284 | security_review | high | open | Support ticket list and bulk update APIs lack support permissions and audit logs | unassigned |
| admin.support-ticket | PAGE-ISSUE-0285 | security_review | high | open | Support ticket detail and reply actions lack support permissions and durable audit | unassigned |
| admin.system | PAGE-ISSUE-0286 | functional_review | high | open | System dashboard calls unregistered jobs, webhooks, sessions, and reset routes | unassigned |
| admin.system | PAGE-ISSUE-0287 | security_review | high | open | System maintenance and session operations lack granular authorization and audit | unassigned |
| admin.users | PAGE-ISSUE-0290 | functional_review | medium | open | Tracked clean URL /admin/users returns 404 instead of the users page | unassigned |
| admin.users | PAGE-ISSUE-0291 | security_review | high | open | User directory exposes PII and status mutation without granular user permissions | unassigned |
| auth.auth-signup | PAGE-ISSUE-0460 | functional_review | medium | open | Verification email delivery lacks outbox retry worker | unassigned |
| cart.cart | PAGE-ISSUE-0387 | security_review | high | open | Populated cart HTML uses incomplete manual escaping for asset data | unassigned |
| cart.cart | PAGE-ISSUE-0388 | security_review | high | open | Cart quantity update fails open when availability lock cannot be read | unassigned |
| cart.cart | PAGE-ISSUE-0389 | accessibility_review | medium | open | Generated cart item controls lack robust accessible labels | unassigned |
| community.partial-tab | PAGE-ISSUE-0506 | e2e_review | low | needs recheck | Dynamic community tab E2E coverage is incomplete | unassigned |
| developer.add-asset | PAGE-ISSUE-0390 | accessibility_review | medium | open | Asset type selection is mouse-only and not semantic | unassigned |
| developer.asset-detail | PAGE-ISSUE-0391 | functional_review | high | open | Asset detail destructive/publish controls are placeholders that imply success | unassigned |
| developer.asset-detail | PAGE-ISSUE-0392 | security_review | medium | open | Developer cap table renders admin user-detail links | unassigned |
| developer.document-upload-step3 | PAGE-ISSUE-0483 | accessibility_review | medium | open | Document upload controls lack complete accessible names | unassigned |
| developer.document-upload-step3 | PAGE-ISSUE-0484 | functional_review | medium | open | Document upload page renders hardcoded demo document rows before JS cleanup | unassigned |
| developer.fragment-assets | PAGE-ISSUE-0482 | security_review | medium | open | Assets fragment returns HTTP 200 for unauthenticated requests | unassigned |
| developer.fragment-chart | PAGE-ISSUE-0481 | security_review | medium | open | Chart fragment returns HTTP 200 for unauthenticated requests | unassigned |
| developer.property-content | PAGE-ISSUE-0485 | functional_review | medium | open | Property media upload copy does not match accepted formats or limits | unassigned |
| developer.property-content | PAGE-ISSUE-0488 | accessibility_review | medium | open | Generated property image remove buttons have no accessible names | unassigned |
| developer.settings | PAGE-ISSUE-0487 | functional_review | medium | open | Developer logo upload UI advertises SVG files that the backend rejects | unassigned |
| developer.submission-success | PAGE-ISSUE-0486 | functional_review | low | open | Submission success WhatsApp contact link points to a placeholder | unassigned |
| kyc.identity-verification | PAGE-ISSUE-0550 | security_review | medium | open | KYC email delivery still lacks durable outbox | unassigned |
| kyc.identity-verification | PAGE-ISSUE-0566 | functional_review | low | open | KYC upload can orphan private object after DB failure | unassigned |
| marketplace.tax-report | PAGE-ISSUE-0003 | functional_review | low | open | Tax report route requires format despite route comment and path contract |  |
| admin.blog | PAGE-ISSUE-0380 | accessibility_review | medium | open | Blog CMS taxonomy form fields lack explicit labels | unassigned |
| admin.blog | PAGE-ISSUE-0381 | functional_review | medium | open | Blog CMS exposes controls that require permissions the page does not require | unassigned |
| admin.blog-editor | PAGE-ISSUE-0382 | security_review | medium | open | Blog cover upload lacks server-side image type validation | unassigned |
| admin.blog-editor | PAGE-ISSUE-0383 | accessibility_review | medium | open | Blog editor URL override fields lack field-specific labels | unassigned |
| admin.blog-editor | PAGE-ISSUE-0384 | functional_review | medium | open | Blog editor exposes publish/archive actions without checking granular permissions | unassigned |
| admin.blog-persona | PAGE-ISSUE-0385 | accessibility_review | low | open | Blog persona output textarea lacks an explicit label | unassigned |
| admin.blog-strategy | PAGE-ISSUE-0386 | accessibility_review | low | open | Blog strategy output textarea lacks an explicit label | unassigned |

## Ambiguous Or Needs Verification
| ID | Kind | Path/Route | Reason | Follow-up |
| --- | --- | --- | --- | --- |
| platform.profile-template | route/template | /profile -> frontend/platform/profile.html | Route handler exists, but the referenced template is missing. | Confirm whether /profile should be removed, redirected, or backed by a template. |
| admin.affiliate-fraud-template | route/template | /admin/affiliate-fraud -> admin/affiliate-fraud.html | Backend generic route expects admin/affiliate-fraud.html, but the repo has admin/admin-affiliate-fraud.html. | Rename the template or add an explicit route for the existing file. |
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
