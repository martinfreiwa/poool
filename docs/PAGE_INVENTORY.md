# POOOL Page Inventory

Generated on 2026-05-11 from:

- `frontend/platform/**/*.html`
- `frontend/www/**/*.html`
- registered Axum page routes in `backend/src/**/*.rs`

This inventory is intentionally page-focused. API routes, static asset mounts, upload mounts, and websocket routes are excluded from the main page list.

## Summary

| Group | Count |
|---|---:|
| Standalone page templates | 154 |
| Non-standalone templates, components, partials, PDFs, and archive files | 57 |
| Registered dynamic page route patterns | 13 |

Notes:

- Template-derived paths are extensionless filenames, for example `frontend/platform/login.html` is listed as `/login`.
- Some templates are mounted by Rust under a different canonical route. Auth templates are registered below `/auth`, for example `/auth/login`.
- Public marketing pages in `frontend/www` are listed separately from the platform templates.

## Public And Platform Pages

| Page | Template |
|---|---|
| `/` | `frontend/platform/index.html` |
| `/403` | `frontend/platform/403.html` |
| `/404` | `frontend/platform/404.html` |
| `/500` | `frontend/platform/500.html` |
| `/account-deletion` | `frontend/platform/account-deletion.html` |
| `/aml-kyc-policy` | `frontend/platform/aml-kyc-policy.html` |
| `/auth-2fa` | `frontend/platform/auth-2fa.html` |
| `/auth-2fa-setup` | `frontend/platform/auth-2fa-setup.html` |
| `/auth-2fa-step-up` | `frontend/platform/auth-2fa-step-up.html` |
| `/cart` | `frontend/platform/cart.html` |
| `/checkout` | `frontend/platform/checkout.html` |
| `/commodities-marketplace` | `frontend/platform/commodities-marketplace.html` |
| `/commodities-preview` | `frontend/platform/commodities-preview.html` |
| `/commodity` | `frontend/platform/commodity.html` |
| `/community` | `frontend/platform/community.html` |
| `/community-badge` | `frontend/platform/community-badge.html` |
| `/community-hashtag` | `frontend/platform/community-hashtag.html` |
| `/community-profile` | `frontend/platform/community-profile.html` |
| `/cookies` | `frontend/platform/cookies.html` |
| `/currency-policy` | `frontend/platform/currency-policy.html` |
| `/forgot-password` | `frontend/platform/forgot-password.html` |
| `/gdpr-data-request` | `frontend/platform/gdpr-data-request.html` |
| `/imprint` | `frontend/platform/imprint.html` |
| `/kyc` | `frontend/platform/kyc.html` |
| `/landing` | `frontend/platform/landing.html` |
| `/landing-v2` | `frontend/platform/landing-v2.html` |
| `/leaderboard` | `frontend/platform/leaderboard.html` |
| `/login` | `frontend/platform/login.html` |
| `/maintenance` | `frontend/platform/maintenance.html` |
| `/marketplace` | `frontend/platform/marketplace.html` |
| `/marketplace-secondary` | `frontend/platform/marketplace-secondary.html` |
| `/marketplace-trading-v2` | `frontend/platform/marketplace-trading-v2.html` |
| `/marketplace-trading-v3` | `frontend/platform/marketplace-trading-v3.html` |
| `/my-trading` | `frontend/platform/my-trading.html` |
| `/payment-in-progress` | `frontend/platform/payment-in-progress.html` |
| `/payment-success` | `frontend/platform/payment-success.html` |
| `/poool_app_home` | `frontend/platform/poool_app_home.html` |
| `/poool_app_ssr` | `frontend/platform/poool_app_ssr.html` |
| `/portfolio` | `frontend/platform/portfolio.html` |
| `/privacy-policy` | `frontend/platform/privacy-policy.html` |
| `/property` | `frontend/platform/property.html` |
| `/property-public` | `frontend/platform/property-public.html` |
| `/reset-password` | `frontend/platform/reset-password.html` |
| `/rewards` | `frontend/platform/rewards.html` |
| `/rewards-v2` | `frontend/platform/rewards-v2.html` |
| `/settings` | `frontend/platform/settings.html` |
| `/settings-notifications-community` | `frontend/platform/settings-notifications-community.html` |
| `/signup` | `frontend/platform/signup.html` |
| `/support` | `frontend/platform/support.html` |
| `/terms` | `frontend/platform/terms.html` |
| `/trade-success` | `frontend/platform/trade-success.html` |
| `/transaction-detail` | `frontend/platform/transaction-detail.html` |
| `/transactions` | `frontend/platform/transactions.html` |
| `/verify-email` | `frontend/platform/verify-email.html` |
| `/wallet` | `frontend/platform/wallet.html` |
| `/welcome` | `frontend/platform/welcome.html` |

## Blog Pages

| Page | Template |
|---|---|
| `/blog/` | `frontend/platform/blog/index.html` |
| `/blog/article` | `frontend/platform/blog/article.html` |

## Marketing WWW Pages

| Page | Template |
|---|---|
| `/en/` | `frontend/www/en/index.html` |
| `/id/` | `frontend/www/id/index.html` |

## Affiliate Pages

| Page | Template |
|---|---|
| `/affiliate-code-of-conduct` | `frontend/platform/affiliate-code-of-conduct.html` |
| `/affiliate-complaints` | `frontend/platform/affiliate-complaints.html` |
| `/affiliate-dashboard` | `frontend/platform/affiliate-dashboard.html` |
| `/affiliate-marketing-materials` | `frontend/platform/affiliate-marketing-materials.html` |
| `/affiliate-materials` | `frontend/platform/affiliate-materials.html` |
| `/affiliate-onboarding` | `frontend/platform/affiliate-onboarding.html` |
| `/affiliate-privacy-notice` | `frontend/platform/affiliate-privacy-notice.html` |
| `/affiliate-promo` | `frontend/platform/affiliate-promo.html` |
| `/affiliate-qualified-referral-payout` | `frontend/platform/affiliate-qualified-referral-payout.html` |
| `/affiliate-referrals` | `frontend/platform/affiliate-referrals.html` |
| `/affiliate-settings` | `frontend/platform/affiliate-settings.html` |
| `/affiliate-tax` | `frontend/platform/affiliate-tax.html` |
| `/affiliate-terms` | `frontend/platform/affiliate-terms.html` |

## Developer Pages

| Page | Template |
|---|---|
| `/developer/add-asset` | `frontend/platform/developer/add-asset.html` |
| `/developer/application-form` | `frontend/platform/developer/application-form.html` |
| `/developer/asset-detail` | `frontend/platform/developer/asset-detail.html` |
| `/developer/assets` | `frontend/platform/developer/assets.html` |
| `/developer/dashboard` | `frontend/platform/developer/dashboard.html` |
| `/developer/document-upload-step3` | `frontend/platform/developer/document-upload-step3.html` |
| `/developer/property-content` | `frontend/platform/developer/property-content.html` |
| `/developer/submission-success` | `frontend/platform/developer/submission-success.html` |
| `/developer/submissions` | `frontend/platform/developer/submissions.html` |

## Admin Pages

| Page | Template |
|---|---|
| `/admin/` | `frontend/platform/admin/index.html` |
| `/admin/admin-affiliate-fraud` | `frontend/platform/admin/admin-affiliate-fraud.html` |
| `/admin/admins` | `frontend/platform/admin/admins.html` |
| `/admin/affiliate-applications` | `frontend/platform/admin/affiliate-applications.html` |
| `/admin/affiliate-finance` | `frontend/platform/admin/affiliate-finance.html` |
| `/admin/affiliate-fraud` | `frontend/platform/admin/affiliate-fraud.html` |
| `/admin/approvals` | `frontend/platform/admin/approvals.html` |
| `/admin/asset-change-requests` | `frontend/platform/admin/asset-change-requests.html` |
| `/admin/asset-change-review` | `frontend/platform/admin/asset-change-review.html` |
| `/admin/asset-details` | `frontend/platform/admin/asset-details.html` |
| `/admin/asset-tokenize` | `frontend/platform/admin/asset-tokenize.html` |
| `/admin/assets` | `frontend/platform/admin/assets.html` |
| `/admin/audit-logs` | `frontend/platform/admin/audit-logs.html` |
| `/admin/blockchain-contract-detail` | `frontend/platform/admin/blockchain-contract-detail.html` |
| `/admin/blockchain-contracts` | `frontend/platform/admin/blockchain-contracts.html` |
| `/admin/blockchain-sync` | `frontend/platform/admin/blockchain-sync.html` |
| `/admin/blockchain-treasury` | `frontend/platform/admin/blockchain-treasury.html` |
| `/admin/blog` | `frontend/platform/admin/blog.html` |
| `/admin/blog-editor` | `frontend/platform/admin/blog-editor.html` |
| `/admin/blog-persona` | `frontend/platform/admin/blog-persona.html` |
| `/admin/blog-strategy` | `frontend/platform/admin/blog-strategy.html` |
| `/admin/community/` | `frontend/platform/admin/community/index.html` |
| `/admin/community/amas` | `frontend/platform/admin/community/amas.html` |
| `/admin/community/announcements` | `frontend/platform/admin/community/announcements.html` |
| `/admin/community/appeals` | `frontend/platform/admin/community/appeals.html` |
| `/admin/community/badges` | `frontend/platform/admin/community/badges.html` |
| `/admin/community/challenges` | `frontend/platform/admin/community/challenges.html` |
| `/admin/community/circle-detail` | `frontend/platform/admin/community/circle-detail.html` |
| `/admin/community/circles` | `frontend/platform/admin/community/circles.html` |
| `/admin/community/comments` | `frontend/platform/admin/community/comments.html` |
| `/admin/community/leaderboard` | `frontend/platform/admin/community/leaderboard.html` |
| `/admin/community/post-detail` | `frontend/platform/admin/community/post-detail.html` |
| `/admin/community/posts` | `frontend/platform/admin/community/posts.html` |
| `/admin/community/reports` | `frontend/platform/admin/community/reports.html` |
| `/admin/community/settings` | `frontend/platform/admin/community/settings.html` |
| `/admin/community/user-detail` | `frontend/platform/admin/community/user-detail.html` |
| `/admin/community/users` | `frontend/platform/admin/community/users.html` |
| `/admin/community/verified-owner-requests` | `frontend/platform/admin/community/verified-owner-requests.html` |
| `/admin/deposits` | `frontend/platform/admin/deposits.html` |
| `/admin/developer-submission-review` | `frontend/platform/admin/developer-submission-review.html` |
| `/admin/developer-submissions` | `frontend/platform/admin/developer-submissions.html` |
| `/admin/dividends` | `frontend/platform/admin/dividends.html` |
| `/admin/email-marketing` | `frontend/platform/admin/email-marketing.html` |
| `/admin/kyc` | `frontend/platform/admin/kyc.html` |
| `/admin/marketplace/` | `frontend/platform/admin/marketplace/index.html` |
| `/admin/marketplace/alerts` | `frontend/platform/admin/marketplace/alerts.html` |
| `/admin/marketplace/analytics` | `frontend/platform/admin/marketplace/analytics.html` |
| `/admin/marketplace/approvals` | `frontend/platform/admin/marketplace/approvals.html` |
| `/admin/marketplace/compliance` | `frontend/platform/admin/marketplace/compliance.html` |
| `/admin/marketplace/fees` | `frontend/platform/admin/marketplace/fees.html` |
| `/admin/marketplace/orderbook` | `frontend/platform/admin/marketplace/orderbook.html` |
| `/admin/marketplace/orders` | `frontend/platform/admin/marketplace/orders.html` |
| `/admin/marketplace/p2p` | `frontend/platform/admin/marketplace/p2p.html` |
| `/admin/marketplace/primary-escrow` | `frontend/platform/admin/marketplace/primary-escrow.html` |
| `/admin/marketplace/reconciliation` | `frontend/platform/admin/marketplace/reconciliation.html` |
| `/admin/marketplace/settings` | `frontend/platform/admin/marketplace/settings.html` |
| `/admin/marketplace/trades` | `frontend/platform/admin/marketplace/trades.html` |
| `/admin/notifications` | `frontend/platform/admin/notifications.html` |
| `/admin/orders` | `frontend/platform/admin/orders.html` |
| `/admin/pending-settlements` | `frontend/platform/admin/pending-settlements.html` |
| `/admin/reports` | `frontend/platform/admin/reports.html` |
| `/admin/rewards` | `frontend/platform/admin/rewards.html` |
| `/admin/roles` | `frontend/platform/admin/roles.html` |
| `/admin/settings` | `frontend/platform/admin/settings.html` |
| `/admin/storage` | `frontend/platform/admin/storage.html` |
| `/admin/support` | `frontend/platform/admin/support.html` |
| `/admin/support-ticket` | `frontend/platform/admin/support-ticket.html` |
| `/admin/system` | `frontend/platform/admin/system.html` |
| `/admin/templates/icons` | `frontend/platform/admin/templates/icons.html` |
| `/admin/treasury` | `frontend/platform/admin/treasury.html` |
| `/admin/user-details` | `frontend/platform/admin/user-details.html` |
| `/admin/users` | `frontend/platform/admin/users.html` |

## Registered Dynamic Page Routes

These are URL patterns served by Rust handlers. They do not map one-to-one to static template filenames.

| Route pattern | Source |
|---|---|
| `/blog/:slug` | `backend/src/blog/mod.rs:24` |
| `/blog/category/:slug` | `backend/src/blog/mod.rs:25` |
| `/commodity/:slug` | `backend/src/assets/mod.rs:27` |
| `/community/badge/:id` | `backend/src/lib.rs:1192` |
| `/community/hashtag/:tag` | `backend/src/lib.rs:1188` |
| `/community/partials/:tab` | `backend/src/lib.rs:1201` |
| `/community/post/:id` | `backend/src/lib.rs:1187` |
| `/community/u/:user_id` | `backend/src/lib.rs:1191` |
| `/p/:slug` | `backend/src/assets/mod.rs:25`, `backend/src/lib.rs:1094` |
| `/property/:slug` | `backend/src/assets/mod.rs:23` |
| `/r/:code` | `backend/src/rewards/mod.rs:18` |
| `/rewards/:code` | `backend/src/rewards/mod.rs:17` |
| `/transactions/:id` | `backend/src/wallet/mod.rs:18` |

## Registered Page Route Aliases Without Matching Template Paths

These routes are handled by Rust and usually render an existing template, redirect, or return a flow-specific response.

| Route | Source |
|---|---|
| `/affiliate` | `backend/src/rewards/mod.rs:21` |
| `/affiliate/complaints` | `backend/src/rewards/mod.rs:46` |
| `/affiliate/dashboard` | `backend/src/rewards/mod.rs:23` |
| `/affiliate/materials` | `backend/src/rewards/mod.rs:25` |
| `/affiliate/onboarding` | `backend/src/rewards/mod.rs:22` |
| `/affiliate/referrals` | `backend/src/rewards/mod.rs:24` |
| `/affiliate/settings` | `backend/src/rewards/mod.rs:26` |
| `/affiliate/tax` | `backend/src/rewards/mod.rs:41` |
| `/affiliate/terms` | `backend/src/rewards/mod.rs:28` |
| `/auth/2fa` | `backend/src/auth/routes.rs:77` |
| `/auth/2fa/setup` | `backend/src/auth/routes.rs:78` |
| `/auth/2fa/step-up` | `backend/src/auth/routes.rs:79` |
| `/auth/forgot-password` | `backend/src/auth/routes.rs:84` |
| `/auth/google` | `backend/src/auth/routes.rs:81` |
| `/auth/google/callback` | `backend/src/auth/routes.rs:82` |
| `/auth/login` | `backend/src/auth/routes.rs:75` |
| `/auth/logout` | `backend/src/auth/routes.rs:80` |
| `/auth/reset-password` | `backend/src/auth/routes.rs:88` |
| `/auth/signup` | `backend/src/auth/routes.rs:76` |
| `/auth/verify-email` | `backend/src/auth/routes.rs:91` |
| `/blog/feed.xml` | `backend/src/blog/mod.rs:23` |
| `/developer/settings` | `backend/src/developer/mod.rs:48` |
| `/developer/support` | `backend/src/developer/mod.rs:49` |
| `/fonts-template.html` | `backend/src/lib.rs:1216` |
| `/forms-template.html` | `backend/src/lib.rs:1213` |
| `/logout` | `backend/src/lib.rs:1139` |
| `/overlays-template.html` | `backend/src/lib.rs:1215` |
| `/profile` | `backend/src/lib.rs:1180` |
| `/settings-2` | `backend/src/settings/mod.rs:17` |
| `/settings-3` | `backend/src/settings/mod.rs:18` |
| `/statistics-template.html` | `backend/src/lib.rs:1212` |
| `/table-template.html` | `backend/src/lib.rs:1214` |
| `/tax-report` | `backend/src/lib.rs:1210` |
| `/tier` | `backend/src/rewards/mod.rs:19` |

## Non-Standalone Templates

These files exist in the template tree but are components, partials, PDF templates, or archived designs rather than standalone application pages.

| Template path | Type |
|---|---|
| `frontend/platform/_archive/cards-template.html` | archive |
| `frontend/platform/_archive/fonts-template.html` | archive |
| `frontend/platform/_archive/forms-template.html` | archive |
| `frontend/platform/_archive/landing-improved.html` | archive |
| `frontend/platform/_archive/landing-v2-testimonials.html` | archive |
| `frontend/platform/_archive/overlays-template.html` | archive |
| `frontend/platform/_archive/statistics-template.html` | archive |
| `frontend/platform/_archive/table-template.html` | archive |
| `frontend/platform/admin/components/property-page-editor.html` | component |
| `frontend/platform/admin/components/sidebar.html` | component |
| `frontend/platform/components/auth-head.html` | component |
| `frontend/platform/components/blog-footer.html` | component |
| `frontend/platform/components/blog-head.html` | component |
| `frontend/platform/components/blog-header.html` | component |
| `frontend/platform/components/developer-assets.html` | component |
| `frontend/platform/components/developer-chart.html` | component |
| `frontend/platform/components/developer-sidebar-template.html` | component |
| `frontend/platform/components/developer-topbar.html` | component |
| `frontend/platform/components/head.html` | component |
| `frontend/platform/components/investor-topbar.html` | component |
| `frontend/platform/components/kyc-banner.html` | component |
| `frontend/platform/components/macros.html` | component |
| `frontend/platform/components/mobile-kyc-banner.html` | component |
| `frontend/platform/components/mobile-menu.html` | component |
| `frontend/platform/components/property/contact-commodity.html` | component |
| `frontend/platform/components/property/contact.html` | component |
| `frontend/platform/components/property/documents.html` | component |
| `frontend/platform/components/property/faq-commodity.html` | component |
| `frontend/platform/components/property/faq.html` | component |
| `frontend/platform/components/property/funding-timeline.html` | component |
| `frontend/platform/components/property/gallery.html` | component |
| `frontend/platform/components/property/how-it-works-commodity.html` | component |
| `frontend/platform/components/property/how-it-works.html` | component |
| `frontend/platform/components/property/investment-type.html` | component |
| `frontend/platform/components/property/leasing-strategy.html` | component |
| `frontend/platform/components/property/modals.html` | component |
| `frontend/platform/components/property/operational-strategy-commodity.html` | component |
| `frontend/platform/components/property/operator-commodity.html` | component |
| `frontend/platform/components/property/reviews.html` | component |
| `frontend/platform/components/property/risk-notification-commodity.html` | component |
| `frontend/platform/components/property/risk-notification.html` | component |
| `frontend/platform/components/property/roadmap-commodity.html` | component |
| `frontend/platform/components/property/security-compliance-commodity.html` | component |
| `frontend/platform/components/sidebar-developer-template.html` | component |
| `frontend/platform/components/sidebar-developer.html` | component |
| `frontend/platform/components/sidebar.html` | component |
| `frontend/platform/partials/community_ama.html` | partial |
| `frontend/platform/partials/community_announcements.html` | partial |
| `frontend/platform/partials/community_announcements_list.html` | partial |
| `frontend/platform/partials/community_challenges.html` | partial |
| `frontend/platform/partials/community_circle.html` | partial |
| `frontend/platform/partials/community_disabled.html` | partial |
| `frontend/platform/partials/community_feed.html` | partial |
| `frontend/platform/partials/community_post_card.html` | partial |
| `frontend/platform/partials/community_post_list.html` | partial |
| `frontend/platform/templates/pdf-base.html` | pdf-template |
| `frontend/platform/templates/pdf-tax-report.html` | pdf-template |

## Rebuild Commands

Use these commands to refresh this inventory manually:

```bash
find frontend/platform frontend/www -type f -name '*.html' | sort
```

```bash
rg -n '\\.route\\("/[^"]+"|route_service\\("/|nest_service\\("' backend/src -g '*.rs'
```
