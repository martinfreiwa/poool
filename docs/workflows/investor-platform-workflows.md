# Investor Platform Workflows

Purpose: Verify authenticated investor discovery, cart, checkout, wallet, payment methods, portfolio, transactions, tax reports, rewards, support, settings, leaderboard, and secondary trading UI that can be run without crossing into developer/admin ownership unless referenced.

Roles: Investor; Admin only for approval/readback branches referenced from cross-role workflows.

Primary pages:
- `/marketplace`, `/marketplace/tab`, `/commodities-marketplace`, `/commodities-marketplace/tab`, `/property/:slug`, `/commodity/:slug`
- `/cart`, `/checkout`, `/payment-in-progress`, `/payment-success`, `/wallet`, `/portfolio`, `/transactions`, `/transactions/:id`, `/transaction-detail`, `/tax-report`
- `/leaderboard`, `/settings`, `/settings-2`, `/settings-3`, `/settings/notifications/community`, `/account-deletion`, `/support`
- `/rewards`, `/rewards-v2`, `/tier`, `/marketplace-secondary`, `/marketplace-trading-v2`, `/marketplace-trading-v3`, `/my-trading`, `/trade-success`

Backend/API surfaces:
- `backend/src/assets/mod.rs`, `backend/src/cart`, `backend/src/payments`, `backend/src/payment_methods`, `backend/src/portfolio`, `backend/src/wallet`, `backend/src/settings`, `backend/src/support`, `backend/src/rewards`, `backend/src/marketplace`.
- Cross-role references: `docs/workflows/cross-role/developer-asset-to-investor-purchase.md`, `investor-wallet-deposit-admin-treasury.md`, `investor-withdrawal-step-up-admin-review.md`, `secondary-trading-admin-settlement.md`, and `kyc-investment-limit-checkout.md`.

Prerequisites:
- Investor account is email verified; KYC state is explicitly chosen per case.
- Wallet balance, payment methods, cart, holdings, and orders are disposable or resettable.
- Test upload file for proof-of-transfer and support attachments; valid/invalid type and size variants are available.
- Use integer-cent amounts and keep a starting wallet/portfolio snapshot.

Steps:
1. Open marketplace and commodities pages; exercise tabs, filters, search, clear buttons, card image carousels, HTMX tab endpoints, and detail links.
2. Open property and commodity detail pages; exercise gallery/video, calculator, financial tabs, currency/date toggles, document links, amount input, quick-add amounts, and add-to-cart.
3. Open cart; update amounts, remove/re-add lines, validate empty and populated states, then proceed to checkout.
4. On checkout, verify disclosures, payment method selection, USD/IDR bank transfer controls, reference copy controls, proof upload, confirm payment, duplicate-submit behavior, and empty-cart redirect.
5. Open wallet; exercise retry, deposit, withdraw, payment method add/delete/default, KYC gate, failed validation, and transaction links.
6. Open portfolio and transaction pages; verify holdings, detail links, cancellation/NFT controls where eligible, filters, pagination, and transaction detail readback.
7. Open leaderboard; test metric/timeframe/tier/search/page-size/pagination controls and visibility/display-name preference persistence.
8. Open settings; update safe profile/address/identity/preference/leaderboard/social/developer fields, upload avatar, export data, and route to account deletion without destructive final submit unless explicitly approved.
9. Open support; search FAQ, create ticket, attach file, reply, reopen, and rate resolution using disposable test data.
10. Open rewards and secondary trading surfaces; validate referral copy/share, payout settings, trading filters, buy/sell validation, order cancellation, exports, and success pages. Use cross-role settlement workflows for final money/holding changes.

Expected Result:
- Investor actions persist only through intended APIs, reload correctly, and show loading/empty/error/success states.
- All money uses integer cents in API/DB checks; UI formatting never becomes the source of financial truth.
- Uploads enforce file type/size, create storage links, and show failure states without orphaning files.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Discovery filters | Lists update without losing canonical detail links. |
| Cart mutation | Add/update/remove is persisted and survives reload. |
| Checkout validation | Missing disclosures, invalid amount, empty cart, duplicate submit, and upload failure are blocked. |
| Deposit/withdrawal | Requests move through pending/admin-reviewed states via cross-role workflow. |
| Payment method | Add/default/delete respects ownership and validation. |
| Portfolio/transactions | Holdings and transaction rows match order/wallet source rows. |
| Settings/account data | Safe changes persist; destructive flow requires explicit confirmation and audit. |
| Support | Ticket lifecycle supports attachment, reply, reopen, and CSAT. |
| Secondary trading | Orders/trades are created/cancelled/settled only through trading APIs and admin settlement. |

Negative Cases:
- Unverified or KYC-blocked investor attempts checkout/withdrawal.
- Amount below minimum, above available allocation, insufficient balance, or malformed currency input.
- Unsupported proof/attachment file type or size.
- Unauthorized direct API call for another user's cart, wallet, payment method, order, ticket, or holding.
- Network failure during save/confirm/upload.

Audit / DB / Financial Checks:
- Cart/order/wallet/holding/transaction rows reconcile exactly in cents.
- Financial writes are transactionally linked and idempotency prevents duplicate checkout/deposit/withdrawal credits.
- Support, settings export/deletion, payment admin approval, and trading settlement have audit rows where applicable.
- Uploaded avatar, proof, and support attachment metadata include content type, size, owner, storage key/link, and access control.

Cleanup:
- Remove disposable cart lines, orders, payment methods, tickets, uploads, and trading orders.
- Reverse or mark test wallet/portfolio/transaction rows according to the local finance cleanup policy.
- Restore investor settings and leaderboard visibility values.
