# Investor Workflows

Purpose: Cover investor-only workflows and edge cases that sit around the cross-role purchase, wallet, support, affiliate, and trading flows.

Roles: Investor, Admin where a readback or approval is required.

Primary pages:
- `/marketplace`, `/marketplace/tab`, `/commodities-marketplace`, `/commodities-marketplace/tab`, `/property/:slug`, `/commodity/:slug`
- `/cart`, `/checkout`, `/payment-in-progress`, `/payment-success`
- `/wallet`, `/transactions`, `/transactions/:id`, `/portfolio`, `/tax-report`, `/leaderboard`
- `/settings`, `/settings/notifications/community`, `/account-deletion`
- `/support`, `/rewards`, `/rewards-v2`, `/affiliate/*`
- `/marketplace-secondary`, `/marketplace-trading-v2`, `/marketplace-trading-v3`, `/my-trading`, `/trade-success`

Backend/API surfaces:
- Cart, checkout, wallet deposit/withdrawal, payment methods, portfolio cancellation, leaderboard preferences/refresh, settings, support, rewards/affiliate, marketplace orders, P2P offers.

Prerequisites:
- Investor account with controlled KYC state, wallet balance, holdings, referral state, and leaderboard preferences.
- Test assets exist for real estate, commodities, primary purchase, and secondary trading.

Steps:
1. Verify real-estate and commodity marketplace filters, search, available/funded tabs, HTMX tab endpoints, card carousel, empty states, and clear-search behavior.
2. Verify property and commodity detail media, gallery/video, financial/operator/security/document sections, calculator, amount presets, invalid amounts, and add-to-cart.
3. Verify cart empty state, add/update/remove, mixed asset cart, mobile checkout button, disclosure checks, duplicate submit guard, insufficient funds, annual-limit rejection, payment-in-progress, and payment-success.
4. Verify wallet load/retry/empty/error states, KYC blockers, add-card/bank validation, deposit, withdrawal, high-value step-up, cancel withdrawal, and transaction links.
5. Verify portfolio holdings, asset links, charts, NFT-to-wallet action, cancellation eligibility, KYC/marketplace empty links, and post-purchase readback.
6. Verify transaction filters, date ranges, pagination, transaction detail, related order/deposit/withdrawal links, and tax-report generation/download.
7. Verify leaderboard filters, pagination, preference save, hidden/anonymized user behavior, admin refresh visibility, and demo-data guard.
8. Verify settings search, profile/address/identity/preferences/social/developer fields, avatar/logo uploads, Web3 wallet controls, 2FA/password/email changes, data export, and account deletion request.
9. Verify support FAQ, ticket create, attachment, reply, reopen, status filters, and CSAT through the cross-role support handoff.
10. Verify rewards, referral links, affiliate onboarding/dashboard/referrals/materials/settings/legal pages, invalid codes, exports, and payout request states.
11. Verify secondary-market discovery, trading v2/v3 shells, buy/sell validation, order submit/cancel, P2P actions, trade success, my-trading filters, history/export, and admin settlement readback.

Expected Result:
- Every investor surface either completes the intended action, shows a validation/authorization error, or is explicitly read-only/disabled.
- Investor UI, backend state, and admin readback agree after reload.

Coverage Matrix:

| Area | Expected Result |
|------|-----------------|
| Discovery | Filters, search, cards, and detail pages are consistent. |
| Purchase | Cart/checkout state is validated and idempotent. |
| Wallet | Deposits, withdrawals, methods, and transactions are traceable. |
| Portfolio/tax | Holdings and reports reflect completed events. |
| Settings/security | Personal data and sensitive actions persist or require step-up. |
| Rewards/affiliate | Attribution and payout states are visible and exportable. |
| Trading | Orders, fills, cancellations, and settlement state are consistent. |

Negative Cases:
- Unverified KYC, insufficient funds, below-minimum amount, sold-out asset, duplicate submit, invalid payment method, invalid social URL, unauthorized admin refresh, stale transaction ID, cancelled order, and account deletion cancellation.

Audit / DB / Financial Checks:
- All wallet/order/trading values use integer cents.
- Wallet balance, held balance, order, investment, transaction, affiliate commission, and audit rows align.
- User-visible state is reloaded after every mutation.

Cleanup:
- Cancel open orders, reverse test deposits/withdrawals where supported, remove disposable payment methods, restore settings/preferences, and document retained financial fixtures.

