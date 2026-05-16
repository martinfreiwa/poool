# POOOL Live Site Action Test Log

Test date: 2026-05-16  
Basis: `docs/LIVE_SITE_ACTION_MAP.md`  
Target: local live platform at `http://localhost:8888`  
Tester: Codex  
Scope: page render checks, internal link checks, visible UI controls, non-destructive workflow checks, and explicit safe skips for financial, destructive, admin, and outbound-message actions.

## Status Legend

| Status | Meaning |
| --- | --- |
| `PASS` | The page, link, control, or workflow step worked in the live run. |
| `FAIL` | The page, link, control, or workflow step did not work or could not be found. |
| `SAFE-SKIP` | The control was identified, but the final action was not submitted because it would create a financial, destructive, admin, support, email, public-post, or account-changing side effect. |
| `SKIPPED` | The route or action needed a missing dynamic fixture or was intentionally excluded from a GET/link check. |
| `RETEST` | A browser interaction needs a repeat pass because the local backend restarted during the browser session. |

## Run Summary

| Time | Test group | Basis | Result |
| --- | --- | --- | --- |
| `2026-05-16T16:41:56+02:00` to `2026-05-16T16:42:19+02:00` | Route/page render audit | 181 Action Map routes | `167 PASS`, `4 FAIL`, `10 SKIPPED` |
| `2026-05-16T16:41:56+02:00` to `2026-05-16T16:42:19+02:00` | Internal link audit | 4,213 rendered link records | `3,248 PASS`, `0 FAIL`, `965 SKIPPED` external/action/scheme links |
| `2026-05-16T16:42:20+02:00` to `2026-05-16T16:44:00+02:00` | Dynamic follow-up | Blog slug/category, community post/hashtag, valid tax report query | Blog/category/post/hashtag and `tax-report?year=2026&format=pdf` passed; badge detail still failed |
| `2026-05-16T17:02:33+02:00` to `2026-05-16T17:02:44+02:00` | Critical browser workflow pass | Login, Remember me, marketplace controls, cart, selected protected pages | Login and Remember me passed; later checks were interrupted by local backend restart |

Environment note: the local server ran through `cargo-watch`. During the browser interaction pass the backend restarted and, because the local environment reports missing `SESSION_SECRET_OR_JWT_SECRET`, existing sessions became invalid. The full route/link audit completed before that instability and is the primary coverage source for "all pages/all links".

## Route Failures

| Time | Page / route | Example tested | HTTP | Final URL | Result | What failed |
| --- | --- | --- | --- | --- | --- | --- |
| `2026-05-16T16:42:04+02:00` | `/auth/2fa/setup` | `/auth/2fa/setup` | `500` | `/auth/2fa/setup` | `FAIL` | 2FA setup page returned server error. |
| `2026-05-16T16:42:04+02:00` | `/auth/2fa/step-up` | `/auth/2fa/step-up` | `500` | `/auth/2fa/setup` | `FAIL` | Step-up redirected to setup, then setup returned server error. |
| `2026-05-16T16:42:07+02:00` | `/tax-report` | `/tax-report` | `400` | `/tax-report` | `FAIL` | Bare route fails because required query params are missing. Follow-up `/tax-report?year=2026&format=pdf` passed. |
| `2026-05-16T16:42:08+02:00` | `/community/badge/:id` | `/community/badge/cf024cf8-b07c-4b37-8507-aad364140cbf` | `404` | same | `FAIL` | Badge detail fixture returned not found. Follow-up with another known badge id also returned `404`. |

## Dynamic Routes Rechecked

| Time | Page / route | Action tested | Result | Evidence |
| --- | --- | --- | --- | --- |
| `2026-05-16T16:44:00+02:00` | `/blog/real-estate-101` | Open concrete blog article from blog index links | `PASS` | HTTP `200`, article HTML rendered. |
| `2026-05-16T16:44:00+02:00` | `/blog/category/investment-guides` | Open concrete blog category from blog index links | `PASS` | HTTP `200`, category HTML rendered. |
| `2026-05-16T16:44:00+02:00` | `/community/post/c14928f6-b120-44a2-8c50-892eeeb903b9` | Open concrete community post from feed API fixture | `PASS` | HTTP `200`, detail page rendered. |
| `2026-05-16T16:44:00+02:00` | `/community/hashtag/hashtags` | Open concrete hashtag page | `PASS` | HTTP `200`, hashtag page rendered. |
| `2026-05-16T16:44:00+02:00` | `/tax-report?year=2026&format=pdf` | Open tax report with required query parameters | `PASS` | HTTP `200`, PDF-template HTML rendered. |
| `2026-05-16T16:44:00+02:00` | `/community/badge/2f90c212-bb54-45a8-818e-abaec5e5e0a9` | Open concrete badge detail page | `FAIL` | HTTP `404`, JSON body `Badge not found`. |

## Critical Browser Interaction Tests

| Time | Page | Action tested | Result | Evidence / failure |
| --- | --- | --- | --- | --- |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Page load | `PASS` | HTTP `200`, final URL `/auth/login`, title `Login - POOOL`. |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Login form visible | `PASS` | `#email-input` found. |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Enter email | `PASS` | Email field fill worked. |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Enter password | `PASS` | Password field fill worked. |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Show/hide password button | `FAIL` | Expected `#password-toggle` not found in the browser DOM. Snapshot earlier showed a `Show password` button, so selector/markup needs recheck. |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Password visibility effect | `FAIL` | Password input type stayed `password` because the toggle control was not found by the automated selector. |
| `2026-05-16T17:02:33+02:00` | `/auth/login` | Enable Remember me | `PASS` | `#remember-checkbox` checked successfully. |
| `2026-05-16T17:02:34+02:00` | `/auth/login` | Submit login with Remember me | `PASS` | Final URL `/marketplace`; `poool_session` cookie present. |
| `2026-05-16T17:02:34+02:00` | `/auth/login` | Verify Remember me persistence | `PASS` | `poool_session` had future expiry `1781535753.491449`, not a session-only cookie. |
| `2026-05-16T17:02:35+02:00` | `/marketplace` | Page load after login | `PASS` | HTTP `200`, final URL `/marketplace`, title `Marketplace - POOOL`. |
| `2026-05-16T17:02:35+02:00` | `/marketplace` | Search field visible | `PASS` | 2 search inputs found. |
| `2026-05-16T17:02:35+02:00` | `/marketplace` | Available/Funded tabs visible | `PASS` | 2 tab buttons found. |
| `2026-05-16T17:02:35+02:00` | `/marketplace` | Search field fill | `PASS` | Search input filled with test term. |
| `2026-05-16T17:02:35+02:00` | `/marketplace` | Available/Funded tab click | `PASS` | Tab button clicked. |
| `2026-05-16T17:02:37+02:00` | `/property/boutique-resort-ubud` | Page load | `PASS` | HTTP `200`, title `Property Details - POOOL`. |
| `2026-05-16T17:02:37+02:00` | `/property/boutique-resort-ubud` | Detail buttons visible | `PASS` | 35 button controls found. |
| `2026-05-16T17:02:37+02:00` | `/property/boutique-resort-ubud` | Amount/rechner field visible | `FAIL` | No `input[type="number"]` or amount-named input found by browser selector. Needs UI/selector recheck because Action Map expects calculator/amount controls. |
| `2026-05-16T17:02:38+02:00` | `/cart` | Page load | `PASS` | HTTP `200`, title `Cart - POOOL`. |
| `2026-05-16T17:02:38+02:00` | `/cart` | Cart action controls visible | `PASS` | 18 button/checkout/marketplace controls found. |
| `2026-05-16T17:02:38+02:00` | `/wallet` | Page load | `RETEST` | Browser received `ERR_CONNECTION_REFUSED`; local backend restarted mid-run. Route audit had already rendered `/wallet` successfully. |
| `2026-05-16T17:02:38+02:00` | `/wallet` | Deposit/Withdraw controls | `RETEST` | Could not test after restart. Financial submit remains `SAFE-SKIP`. |
| `2026-05-16T17:02:39+02:00` | `/community` | Page load | `PASS` | HTTP `200`, title `Offline - POOOL`. |
| `2026-05-16T17:02:39+02:00` | `/community` | Community controls visible | `PASS` | At least one search/button/tab control found. |
| `2026-05-16T17:02:43+02:00` | `/community` | Search fill | `FAIL` | Search selector timed out while page was in offline state. |
| `2026-05-16T17:02:44+02:00` | `/settings` | Page load | `PASS` | HTTP `200`, title `Offline - POOOL`. |
| `2026-05-16T17:02:44+02:00` | `/settings` | Settings form visible | `FAIL` | No visible input/save/form selector found because page rendered offline state. |
| `2026-05-16T17:02:44+02:00` | `/settings` | Save settings | `SAFE-SKIP` | No profile/security/account change submitted. |
| `2026-05-16T17:02:44+02:00` | `/developer/dashboard` | Page load | `RETEST` | `ERR_CONNECTION_REFUSED`; local backend restart. Route audit had developer pages covered before restart. |
| `2026-05-16T17:02:44+02:00` | `/developer/assets` | Page load | `RETEST` | `ERR_CONNECTION_REFUSED`; local backend restart. Route audit had developer pages covered before restart. |
| `2026-05-16T17:02:44+02:00` | `/admin/` | Page load | `RETEST` | `ERR_CONNECTION_REFUSED`; local backend restart. Route audit had admin pages covered before restart. |

## Per-Section Action Coverage

| Section | Pages covered | Actions tested | Result |
| --- | --- | --- | --- |
| Shared navigation | Authenticated shells across investor/developer/admin pages | Sidebar/topbar links, internal href targets, static assets, logout skipped as session action | `PASS` for internal links; logout `SKIPPED` |
| Public pages | `/`, `/id/`, `/p/...`, `/blog`, blog article/category, legal pages | CTAs, auth links, property links, category/article links, legal/footer links | `PASS`; `/imprint` and `/gdpr-data-request` intentionally render 404 pages with recovery actions |
| Auth pages | `/auth/login`, `/auth/signup`, `/auth/forgot-password`, `/auth/reset-password`, `/auth/verify-email`, `/auth/2fa`, `/auth/2fa/setup`, `/auth/2fa/step-up` | Inputs, Remember me, password reset/verification actions, auth links, 2FA routes | Login and Remember me `PASS`; password/verification sends `SAFE-SKIP`; 2FA setup/step-up `FAIL` |
| Investor marketplace | `/marketplace`, `/marketplace/tab`, `/commodities-marketplace`, `/commodities-marketplace/tab` | Search/filter/tabs/card links | Route/link `PASS`; marketplace search and tabs browser `PASS` |
| Investor asset detail | `/property/:slug`, `/p/:slug`, `/commodity/:slug` | Gallery/detail buttons, calculator/amount controls, add-to-cart target | Pages `PASS`; authenticated property amount input not found in browser spot-check; add-to-cart final submit `SAFE-SKIP` |
| Cart and checkout | `/cart`, `/checkout`, `/payment-in-progress`, `/payment-success`, cart action endpoints | Cart links/buttons, checkout route, payment confirmation flow | Pages `PASS`; checkout/payment submit and cart mutations `SAFE-SKIP` |
| Wallet and payments | `/wallet`, `/wallet/deposit`, `/wallet/withdraw` | Deposit/withdraw/payment-method controls and endpoints | Route audit `PASS` for `/wallet`; browser control recheck interrupted by restart; submit endpoints `SAFE-SKIP` |
| Portfolio, transactions, reporting | `/portfolio`, `/portfolio.html`, `/transactions`, `/transactions/:id`, `/tax-report` | Holdings links, filters, transaction detail, tax export | Pages `PASS` except bare `/tax-report` `FAIL`; valid tax query `PASS` |
| Community | `/community`, `/community/post/:id`, `/community/hashtag/:tag`, `/community/me`, `/community/me/edit`, `/community/u/:user_id`, `/community/badge/:id`, community notifications | Tabs/search/detail/profile links, post/comment/DM/report actions | Post and hashtag dynamic pages `PASS`; badge detail `FAIL`; public communication actions `SAFE-SKIP` |
| Settings/support/rewards/affiliate | `/settings*`, `/account-deletion`, `/support`, `/rewards*`, `/affiliate*` | Forms, support ticket controls, referral links, onboarding wizard, legal links | Route/link `PASS`; saves/tickets/applications/deletion `SAFE-SKIP` |
| Trading | `/marketplace-secondary`, `/marketplace-trading-v2`, `/marketplace-trading-v3`, `/my-trading`, `/trade-success` | Trading navigation, buy/sell controls, order/history pages | Route/link `PASS`; order placement `SAFE-SKIP` |
| Developer dashboard/assets | `/developer`, `/developer/onboarding`, `/developer/dashboard`, `/developer/assets`, `/developer/add-asset` | Developer nav, KPI links, asset search/tabs, add asset type selection | Route/link `PASS`; browser recheck for deeper developer controls interrupted after restart |
| Developer submission wizard | `/developer/application-form`, `/developer/document-upload-step3`, `/developer/property-content`, `/developer/submission-success`, `/developer/submissions`, `/developer/asset-detail` | Field entry, uploads, draft actions, submit/tokenize/review actions | Route/link `PASS`; file upload/save/submit/tokenize `SAFE-SKIP` |
| Developer operations | `/developer/operations`, `/developer/villas/:asset_id/operations/new`, `/developer/villas/:asset_id/annual/:year` | Year/filter/report links, monthly report fields, annual upload/forms | Route/link `PASS`; report submit/upload `SAFE-SKIP` |
| Developer affiliate team | `/developer/affiliate-team*` | Members/customers/products/settings/analytics/tier controls, export/invite/save actions | Route/link `PASS`; invite/remove/approve/save `SAFE-SKIP` |
| Admin core | `/admin/*` core admin pages | Search/filter/export/approve/reject/settings/admin actions | Route/link `PASS`; mutating admin actions `SAFE-SKIP` |
| Admin community | `/admin/community/*` | Moderation, badges, AMAs, appeals, reports, user actions | Route/link `PASS`; mutating moderation actions `SAFE-SKIP` |
| Admin marketplace | `/admin/marketplace/*` | Orderbook/orders/p2p/reconciliation/settings controls | Route/link `PASS`; settlement/cancel/approve actions `SAFE-SKIP` |

## Workflow Tests

| Workflow | Steps tested | Result | Notes |
| --- | --- | --- | --- |
| Visitor to login | Landing/auth links -> `/auth/login` -> fill email/password -> Remember me -> submit | `PASS` | Final URL `/marketplace`; persistent session cookie verified. |
| Password recovery | `/auth/forgot-password` -> fill email -> identify submit | `SAFE-SKIP` | Reset email not sent intentionally. |
| Signup entry | `/auth/signup` -> fill email -> check terms -> verify login/legal links | `SAFE-SKIP` | No new account created. |
| Browse marketplace | `/marketplace` -> search field -> Available/Funded tab | `PASS` | Search fill and tab click passed. |
| Property detail review | `/property/boutique-resort-ubud` -> detail controls -> amount/rechner control lookup | `FAIL` | Page loaded, but expected amount/rechner input was not found by browser selector. |
| Cart/checkout | `/cart` -> cart controls -> `/checkout` route | `PASS` with `SAFE-SKIP` | Payment confirmation not submitted. |
| Wallet funding | `/wallet` -> deposit/withdraw controls | `RETEST` | Browser pass interrupted by local backend restart; financial submit remains skipped. |
| Community reading | `/community/post/:id`, `/community/hashtag/:tag` | `PASS` | Concrete post and hashtag routes rendered. |
| Community interaction | Search/post/comment/DM/report actions | `SAFE-SKIP` / partial `FAIL` | Search fill timed out in offline state; no public communication submitted. |
| Settings update | `/settings` -> settings controls -> save | `SAFE-SKIP` / partial `FAIL` | Page rendered offline state in browser pass; no settings saved. |
| Developer asset submission | Add asset wizard, application, documents, content, submissions | `PASS` for route/link audit; `SAFE-SKIP` final submits | No draft, upload, submit, or tokenize action triggered. |
| Developer operations | Operations dashboard, monthly report, annual data | `PASS` for route/link audit; `SAFE-SKIP` final submits | No report or annual data submitted. |
| Developer affiliate team | Overview, members, customers, products, analytics, settings, tier | `PASS` for route/link audit; `SAFE-SKIP` mutating actions | No invite, approve, remove, save, or banking update. |
| Admin review/moderation | Admin dashboards, queues, orders, community and marketplace admin pages | `PASS` for route/link audit; `SAFE-SKIP` mutating actions | No approve/reject/delete/send/settlement/admin config action triggered. |

## Retest Queue

| Priority | Item | Reason |
| --- | --- | --- |
| High | `/auth/2fa/setup` and `/auth/2fa/step-up` | Both returned server errors in the route audit. |
| High | Authenticated property amount/rechner control | Action Map expects amount/calculator controls; browser selector found none on `/property/boutique-resort-ubud`. |
| Medium | `/community/badge/:id` | Concrete badge detail route returned `404`; needs a valid fixture or route/API fix. |
| Medium | `/wallet`, deeper developer pages, and `/admin/` browser interactions | Browser interaction pass was interrupted by local backend restart. Route/link audit passed, but controls need a stable-session recheck. |
| Medium | Community and settings offline states | Browser rendered `Offline - POOOL` for `/community` and `/settings`; route audit passed, but interactive controls need a stable online recheck. |
| Low | Bare `/tax-report` | Route returns `400`; valid `year` and `format` query works. Decide whether bare route should redirect, show a form, or stay invalid. |

## Retest Pass 2026-05-16 17:22 CEST

Retest target: stable local backend at `http://localhost:8890`. This avoided the `cargo-watch` restarts that invalidated the earlier browser session.

### Failed Items Rechecked

| Time | Item | Retest action | Result | Updated assessment |
| --- | --- | --- | --- | --- |
| `2026-05-16T17:09:xx+02:00` | `/auth/2fa/setup` | Authenticated GET | `FAIL` | Still returns HTTP `500` with `{"error":"An unexpected error occurred. Please try again."}`. `/health` reports missing `TOTP_SECRET_ENCRYPTION_KEY_OR_ENCRYPTION_KEY`, so local 2FA is not usable until env is configured. |
| `2026-05-16T17:09:xx+02:00` | `/auth/2fa/step-up` | Authenticated GET | `FAIL` | Still redirects into `/auth/2fa/setup` and returns HTTP `500`; same TOTP env blocker. |
| `2026-05-16T17:09:xx+02:00` | `/tax-report` | Authenticated GET without query params | `FAIL` | Still HTTP `400`: `missing field year`. This route is not a standalone page today. |
| `2026-05-16T17:09:xx+02:00` | `/tax-report?year=2026&format=pdf` | Authenticated GET with required params | `PASS` | Valid report URL renders HTTP `200`. |
| `2026-05-16T17:09:xx+02:00` | `/community/badge/cf024cf8-b07c-4b37-8507-aad364140cbf` | Authenticated GET using old fixture id from core DB | `FAIL` | HTTP `404`; old fixture came from core `poool.badges`, but the route reads `poool_community.badges`. |
| `2026-05-16T17:21:xx+02:00` | `/community/badge/13c980a2-0255-4308-bbb6-fcab4be4b85c` | Authenticated GET using valid `poool_community.badges` id | `PASS` | Badge page and `/api/community/badges/:id` both returned HTTP `200`. Previous badge failure was a bad test fixture, not a route failure. |
| `2026-05-16T17:09:xx+02:00` | `/auth/login` password visibility | Browser click `#toggle-password` | `PASS` | Previous test used wrong selector. Actual id is `#toggle-password`; it changes `#password-input` from `password` to `text`. |
| `2026-05-16T17:09:xx+02:00` | `/property/boutique-resort-ubud` amount/rechner controls | Browser selector broadened to amount/data/add controls | `PASS` | 4 matching amount/add-investment controls found. Previous selector was too narrow. |
| `2026-05-16T17:09:xx+02:00` | `/community` search | Browser fill on stable session | `PASS` | Community page rendered normally and search field fill worked. Earlier `Offline - POOOL` state was caused by unstable session/server restart. |
| `2026-05-16T17:09:xx+02:00` | `/settings` controls | Browser control scan | `PASS` | 58 visible settings form/save controls found. |
| `2026-05-16T17:14:xx+02:00` | `/wallet` page and controls | Authenticated HTTP/HTML scan on stable session | `PASS` | HTTP `200`, title `Wallet - POOOL`; deposit, withdraw, and add-payment controls were present in rendered HTML. Financial submit remains `SAFE-SKIP`. |
| `2026-05-16T17:09:xx+02:00` | `/developer/dashboard` | Browser page/control scan | `PASS` | HTTP `200`; developer dashboard controls found. |
| `2026-05-16T17:09:xx+02:00` | `/developer/assets` | Browser page/control scan | `PASS` | HTTP `200`; search/button/table controls found. |
| `2026-05-16T17:09:xx+02:00` | `/admin/` | Browser page/control scan | `PASS` | HTTP `200`; admin controls found. |

### Skipped Items Rechecked

| Time | Item | Retest action | Result | Updated assessment |
| --- | --- | --- | --- | --- |
| `2026-05-16T17:09:xx+02:00` | `/cart/add` | GET only | `PASS` | HTTP `405`, confirming this is a POST/action endpoint. Mutating POST remains `SAFE-SKIP`. |
| `2026-05-16T17:09:xx+02:00` | `/cart/update` | GET only | `PASS` | HTTP `405`, confirming this is a POST/action endpoint. Mutating POST remains `SAFE-SKIP`. |
| `2026-05-16T17:09:xx+02:00` | `/cart/remove` | GET only | `PASS` | HTTP `405`, confirming this is a POST/action endpoint. Mutating POST remains `SAFE-SKIP`. |
| `2026-05-16T17:09:xx+02:00` | `/wallet/deposit` | GET only | `PASS` | HTTP `405`, confirming this is a POST/action endpoint. Financial submit remains `SAFE-SKIP`. |
| `2026-05-16T17:09:xx+02:00` | `/wallet/withdraw` | GET only | `PASS` | HTTP `405`, confirming this is a POST/action endpoint. Financial submit remains `SAFE-SKIP`. |
| `2026-05-16T17:16:xx+02:00` | `/logout` | Authenticated GET | `PASS` | Route logs out and lands on login page. It is a session-mutating GET, so it should stay separated from passive route audits. |
| `2026-05-16T17:09:xx+02:00` | `/settings` save/delete | Controls present, no submit | `SAFE-SKIP` | Save/delete remain intentionally unsubmitted. |
| `2026-05-16T17:09:xx+02:00` | `/wallet` deposit/withdraw submit | Controls present from HTML/curl; financial submit not sent | `SAFE-SKIP` | No deposit/withdraw request was created. |

## Affiliate Tracking Audit 2026-05-16 17:22 CEST

### Code Path Verified

| Step | Code path | Current behavior |
| --- | --- | --- |
| Link click | `GET /r/:code` and `GET /rewards/:code` in `backend/src/rewards/routes.rs` | Stores `poool_referral=code|subid|utm_source`, redirects to `/auth/signup`, and records first click in `referral_clicks` with `link_id` when not deduped by IP/code. |
| Signup | `backend/src/auth/routes.rs::signup_submit` | Reads form referral code or `poool_referral`, parses `code|subid|utm_source`, then calls `attribute_affiliate_referral`. |
| Attribution | `backend/src/rewards/service.rs::attribute_affiliate_referral` | Looks up active `affiliate_links`, blocks self/same-team fraud cases, inserts one `affiliate_referrals` row with `link_id`, `attribution_user_id`, `payout_user_id`, `sub_id`, and `utm_source`. |
| Commission tracking | `backend/src/rewards/service.rs::check_and_track_affiliate_commission` | On completed wallet/order payment, should insert `affiliate_commissions` with `gross_amount_cents`, `provisional_amount_cents`, currency, link id, attribution user, and payout user. |
| Counter tracking | DB trigger `trg_affiliate_commissions_counter_sync_*` | Intended to update `affiliate_live_counters`, but current local DB function body is broken; see failing test below. |

### Private vs Business Link Registration Test

| Time | Flow | Test | Result | Evidence |
| --- | --- | --- | --- | --- |
| `2026-05-16T17:20:xx+02:00` | Private/personal link | Click `/r/{personal_code}?subid=...&utm_source=codex-e2e` | `PASS` | Redirected `303` to `/auth/signup`; `poool_referral` cookie contained `code|subid|utm`. |
| `2026-05-16T17:20:xx+02:00` | Private/personal link | Register throwaway user after click | `PASS` | Signup returned HTTP `200`; user was created. Throwaway fixture was removed after verification. |
| `2026-05-16T17:20:xx+02:00` | Private/personal link | Verify `affiliate_referrals` row | `PASS` | Row status `registered`; `link_id` matched personal link; `attribution_user_id == payout_user_id`; `sub_id` and `utm_source` persisted. |
| `2026-05-16T17:20:xx+02:00` | Business/team link | Click `/r/{team_business_code}?subid=...&utm_source=codex-e2e` | `PASS` | Redirected `303` to `/auth/signup`; `poool_referral` cookie contained `code|subid|utm`. |
| `2026-05-16T17:20:xx+02:00` | Business/team link | Register throwaway user after click | `PASS` | Signup returned HTTP `200`; user was created. Throwaway fixture was removed after verification. |
| `2026-05-16T17:20:xx+02:00` | Business/team link | Verify `affiliate_referrals` row | `PASS` | Row status `registered`; `link_id` matched team link; `attribution_user_id` was the team member; `payout_user_id` was the developer/team owner; `sub_id` and `utm_source` persisted. |

### Affiliate Integration Tests

| Command / test | Result | What it proves |
| --- | --- | --- |
| `DATABASE_URL=postgres://martin@localhost/poool cargo test --test affiliate_team_integration personal_attribution_writes_self_payout -- --ignored --exact --test-threads=1 --nocapture` | `PASS` | Personal/private link attribution writes payout to the affiliate themself. |
| `DATABASE_URL=postgres://martin@localhost/poool cargo test --test affiliate_team_integration team_business_attribution_splits_attribution_from_payout -- --ignored --exact --test-threads=1 --nocapture` | `PASS` | Business/team link attribution correctly separates reporting attribution from developer payout. |
| `DATABASE_URL=postgres://martin@localhost/poool cargo test --test affiliate_team_integration commission_rate_branches_by_link_type -- --ignored --exact --test-threads=1 --nocapture` | `FAIL` | Commission/revenue insert currently crashes in DB trigger before the expected personal/team commission assertions can pass. |

### Affiliate Blocking Finding

| Severity | Area | Finding | Evidence | Impact |
| --- | --- | --- | --- | --- |
| High | Affiliate commission and revenue counters | Current DB trigger functions reference `new_table` / `old_table`, while installed triggers define transition aliases `new_rows` / `old_rows`. | Failed test error: `relation "new_table" does not exist` in `affiliate_commissions_counter_sync_ins()`. `pg_get_triggerdef` shows `REFERENCING NEW TABLE AS new_rows`; `pg_get_functiondef` still uses `FROM new_table nt`. | Click and registration tracking work, but first paid conversion cannot reliably write `affiliate_commissions` / `affiliate_live_counters`. That means Einnahmen/Umsaetze/Provisionen from both private and business links are currently blocked at commission tracking. |

There is already a local migration file named `database/178_trigger_transition_table_alias_fix.sql` that rewrites the functions to use `new_rows` / `old_rows`. It was not applied to the current local DB at test time.

## Affiliate Migration Follow-Up 2026-05-16 17:37 CEST

Target DB: `postgres://martin@localhost/poool`.

| Time | Action | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T17:36:xx+02:00` | Applied `database/169_commission_round2_fixes.sql` | `PASS` | Migration executed successfully; backfill completed. |
| `2026-05-16T17:36:xx+02:00` | Applied `database/170_affiliate_commission_currency.sql` | `PASS` | Migration executed successfully; existing currency columns were skipped where already present, constraints/functions were refreshed. |
| `2026-05-16T17:36:xx+02:00` | Re-applied `database/178_trigger_transition_table_alias_fix.sql` after `169/170` | `PASS` | Trigger functions now reference `new_rows` / `old_rows`; `new_table` / `old_table` no longer appear in installed trigger functions. |
| `2026-05-16T17:36:xx+02:00` | Marked `169_commission_round2_fixes.sql` and `170_affiliate_commission_currency.sql` in `_schema_migrations` | `PASS` | Prevents backend restarts from reapplying `169/170` after the hotfix and clobbering the corrected function bodies. |
| `2026-05-16T17:36:xx+02:00` | `cargo test --test affiliate_team_integration commission_rate_branches_by_link_type -- --ignored --exact --test-threads=1 --nocapture` | `PASS` | Personal and business commission inserts now complete; live-counter trigger no longer crashes. |
| `2026-05-16T17:37:xx+02:00` | `cargo test --test affiliate_team_integration personal_attribution_writes_self_payout -- --ignored --exact --test-threads=1 --nocapture` | `PASS` | Personal/private link attribution still writes payout to the affiliate themself. |
| `2026-05-16T17:37:xx+02:00` | `cargo test --test affiliate_team_integration team_business_attribution_splits_attribution_from_payout -- --ignored --exact --test-threads=1 --nocapture` | `PASS` | Business/team link attribution still separates attribution user from payout user. |
| `2026-05-16T17:38:xx+02:00` | `cargo test --test affiliate_team_integration -- --ignored --test-threads=1 --nocapture` | `PASS` | Full affiliate team integration suite passed: 13 passed, 0 failed. |

Updated affiliate conclusion: click and signup attribution work for private and business links, and the local DB now also supports paid conversion commission tracking and affiliate live-counter updates.

## Affiliate Checkout E2E Follow-Up 2026-05-16 17:56 CEST

Target local backend: `http://localhost:8890`.

### Localhost Full Affiliate Checkout

| Time | Flow | Test | Result | Evidence |
| --- | --- | --- | --- | --- |
| `2026-05-16T17:50:xx+02:00` | Private/personal affiliate link | Click `/r/{personal_code}?subid=...`, register via `/auth/signup`, verify email/KYC locally, fund test wallet, add one cart item, submit wallet checkout | `PASS` | Signup wrote `affiliate_referrals` with correct `link_id`, `attribution_user_id == payout_user_id`, `sub_id`, and `utm_source`. Checkout created completed order `ORD-20260516155055-71cfb9`; commission `23` cents on `4725` cents gross at `50` bps; `affiliate_live_counters` revenue and commission deltas matched exactly; affiliate dashboard personal context increased by `23` cents. |
| `2026-05-16T17:50:xx+02:00` | Business/team affiliate link | Click `/r/{team_business_code}?subid=...`, register via `/auth/signup`, verify email/KYC locally, fund test wallet, add one cart item, submit wallet checkout | `PASS` | Signup wrote `affiliate_referrals` with attribution user as the team member and payout user as the developer. Checkout created completed order `ORD-20260516155055-de98b4`; commission `212` cents on `4725` cents gross at `450` bps; `affiliate_live_counters` revenue and commission deltas matched exactly; attribution-user business dashboard and payout-user all-context dashboard both increased by `212` cents. |
| `2026-05-16T17:52:xx+02:00` | Cleanup | Remove E2E users, orders, commissions, referrals, wallet rows, dashboard sessions, and referral clicks; restore asset availability | `PASS` | Post-cleanup checks: `test_users=0`, `test_orders=0`, `test_commissions=0`, `codex_clicks=0`, `dash_sessions=0`; `renovation-flip-canggu` token availability restored to `5498`. |
| `2026-05-16T17:53:xx+02:00` | Regression suite | `cargo test --test affiliate_team_integration -- --ignored --test-threads=1 --nocapture` | `PASS` | Full suite passed again after the E2E run: 13 passed, 0 failed. |

### Localhost Negative Referral Checks

| Time | Check | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T17:54:xx+02:00` | Invalid referral code `/r/CODEXINVALID...` | `PASS` | Returned `303` to `/auth/signup`; no `referral_clicks` row was inserted. Note: route still sets a local `poool_referral` cookie for the invalid code, but later signup attribution ignores it because no active link/affiliate exists. |
| `2026-05-16T17:54:xx+02:00` | First-touch cookie behavior | `PASS` | First click set `poool_referral` for the personal code; second click on a business code did not overwrite the cookie. Both click rows were recorded and then removed. |

### Live Read-Only Smoke

| Time | URL | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/health` | `PASS` | HTTP `200`, JSON health endpoint reachable. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/` | `PASS` | Followed to `/auth/login`, HTTP `200`, title `Login - POOOL`. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/auth/login` | `PASS` | HTTP `200`, title `Login - POOOL`. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/auth/signup` | `PASS` | HTTP `200`, title `Sign up - POOOL`. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/marketplace` | `PASS` | Anonymous request returns HTTP `303` to `/auth/login`, expected protected-page behavior. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/affiliate/dashboard` | `PASS` | Anonymous request returns HTTP `303` to `/auth/login`, expected protected-page behavior. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/affiliate/code-of-conduct` | `PASS` | HTTP `200`, public affiliate legal page reachable. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/affiliate/qualified-referral-payout` | `PASS` | HTTP `200`, public affiliate policy page reachable. |
| `2026-05-16T17:55:xx+02:00` | `https://platform.poool.app/r/CODEXINVALIDLIVE` | `PASS` | HTTP `303` to `/auth/signup`; no active affiliate code was used on live. |
| `2026-05-16T17:55:xx+02:00` | `https://www.poool.app/` | `PASS` | HTTP `200`, marketing home reachable. |
| `2026-05-16T17:55:xx+02:00` | `https://poool.finance/` and `/id/` | `FAIL` | DNS resolution failed for `poool.finance`; no HTTP request reached the site. |

## Action Map Coverage Gap Review 2026-05-16 18:04 CEST

Compared source: `docs/LIVE_SITE_ACTION_MAP.md`. This review separates route/link coverage from true action/workflow coverage. The route audit covered the site broadly, but several Action Map items are still only verified as rendered controls, HTTP routes, or intentionally skipped side-effect actions.

### Coverage Position

| Area | Current coverage | Missing or under-tested coverage | Priority |
| --- | --- | --- | --- |
| Route and internal-link reachability | Broad local route/link pass: 181 Action Map routes audited, 4,213 internal link records checked. | Exact route names are grouped in this log instead of listed one by one for every admin/developer/public subpage, so future retests should keep per-route evidence for high-risk areas. | Medium |
| Public marketing and public property pages | Public route/link reachability and production `www.poool.app` smoke passed. | Landing anchors, language switch, mobile menu, property card carousels, WhatsApp/contact links, public-property gallery/lightbox/calculator tabs, mobile amount controls, and blog category/social/footer interactions still need browser-level interaction tests. `poool.finance` DNS failed. | Medium |
| Authentication | Email/password login, password visibility toggle, Remember me cookie persistence, logout, signup during affiliate E2E, and protected-page redirects passed. | Generic signup without referral, Google OAuth login/signup, forgot-password email delivery, reset-token flow, verify-email resend, `/welcome` decisions, and testimonial carousel are not fully workflow-tested. 2FA setup and step-up remain blocked by missing TOTP env. | High |
| Investor marketplace and asset discovery | Real-estate marketplace search/tabs, property detail controls, cart presence, and affiliate-driven checkout passed locally. | Commodities marketplace filters/detail, media/video controls, commodity add-to-cart, public property calculator/lightbox, card image carousel behavior, external developer/social links, and validation around invalid amount/min/max values are still not covered deeply. | High |
| Cart and checkout | Full local wallet checkout passed through private and business affiliate links; commission, revenue counters, dashboard deltas, cleanup, and regression suite passed. | Non-affiliate checkout, mixed cart, cart add/update/remove UI POSTs, disclosure-negative validation, insufficient wallet funds, empty-cart redirect, duplicate-submit/idempotency, bank-transfer USD/IDR selection, proof upload, reference copy buttons, `/payment-in-progress`, and `/payment-success` post-payment UX still need focused tests. | High |
| Wallet and payment methods | Wallet page and controls rendered; deposit/withdraw endpoints correctly reject GET. | Deposit submit, withdraw submit, KYC blocker states, insufficient funds, add-card fields, add-bank fields, payment-method validation, transaction-detail links, and retry/error states are still `SAFE-SKIP` or render-only. | High |
| Portfolio, transactions, reports, leaderboard | Routes and valid tax-report query rendered. | Holdings row actions, NFT-to-wallet, cancellation eligibility, transaction filters/date/pagination/detail, report generation/download, leaderboard filters/pagination/preference save/admin refresh are not workflow-tested. Bare `/tax-report` still returns 400 by design. | Medium |
| Community | Community route, search, post detail, hashtag detail, own/profile routes, and valid badge detail passed after fixture correction. | Composer, post create, react, bookmark, comment, comment edit/delete/reaction, report, follow/unfollow, DM, block/mute, ban appeal, notification mark-read/preferences save, profile edit/avatar/banner upload, and moderation-history actions remain largely `SAFE-SKIP`. | Medium |
| Settings, support, account deletion | Settings/support routes and controls rendered. | Profile/address/identity/security/Web3/social save actions, data export, account deletion request, support ticket creation, attachment upload, reply/reopen, and CSAT rating remain intentionally unsubmitted. | Medium |
| Rewards and affiliate | Private and business referral click, signup attribution, paid conversion, commission rates, live counters, dashboard deltas, invalid code, and first-touch behavior passed locally. Public affiliate policy pages passed live. | Affiliate onboarding wizard, compliance quiz, KYC/tax steps, payout request, payout settings, referrals/materials exports/downloads, postback settings, and live authenticated affiliate conversion are not yet tested. The migration files must be deployed before live paid-conversion confidence. | High |
| Secondary trading | Trading routes/link shells passed. | Buy/sell mode, orderbook/chart interactions, amount/price validation, submit order, cancel order, P2P offer actions, history/export, and `/trade-success` after a real order are not workflow-tested. | High |
| Developer dashboard and submission | Developer dashboard/assets controls passed after stable-session retest; route/link audit covered wizard and operations pages. | Developer onboarding submit, add-asset type selection, application form save/next, document uploads/removals, property content media/video/projection validation, submit/tokenize, submission duplicate/delete/resubmit, asset edit/change request, operations monthly report save/submit, evidence upload, annual CapEx/forecast/document upload are still render-only or `SAFE-SKIP`. | High |
| Developer affiliate team | Routes and shells passed; business-link attribution and payout split passed locally through backend integration/E2E. | Member invite/approve/remove, customers/products/member filters, CSV/PDF export, settings dirty-state/discard/save, public join URL copy/open, banking validation, analytics date/resolution controls, and tier progression are not fully browser/workflow-tested. | High |
| Admin core | Admin root retested; route/link shells for admin pages passed. | KYC approve/reject, support assignment/reply, submission review approve/reject/request changes, asset publish/feature/funding edits, deposits confirm/cancel, orders approve/reject/export, rewards payouts, dividends calculate/approve/execute, reports, notifications, platform settings, system actions, storage links, admins/roles/RBAC, email campaigns, blog editor publish/archive, blockchain settlement/sync, affiliate application/finance/team actions remain `SAFE-SKIP`. | High |
| Admin community | Route/link shells passed. | AMA create/answer/status, announcements publish/archive, appeals approve/reject, badge/challenge/circle CRUD, comment/post moderation, leaderboard XP modal, reports moderation action, community settings save, user warn/ban/mute, verified-owner approve/reject remain `SAFE-SKIP`. | High |
| Admin marketplace | Route/link shells passed. | Orderbook rebuild/refresh/export, order cancel/export selected, P2P approve/reject/cancel/flag, escrow run/retry/reconcile, reconciliation resolve/retry, fees/settings save, compliance report generation, analytics filtering/export, alerts save/share remain `SAFE-SKIP`. | High |
| Live site | Read-only live smoke passed for platform auth pages, affiliate public policy pages, protected redirects, and `www.poool.app`. | No live authenticated investor/developer/admin tests, no live affiliate signup/checkout, no live wallet/checkout/trading/admin mutations. `poool.finance` DNS failure remains open. | High |

### Items No Longer Open After Retest

| Previous gap | Current status | Evidence |
| --- | --- | --- |
| Login password toggle selector | `PASS` | Actual selector is `#toggle-password`; it toggles `#password-input` to text. |
| Authenticated property amount/rechner controls | `PASS` | Broader selector found amount/add-investment controls. |
| Community offline/search failure | `PASS` | Stable-session retest rendered normal community page and search filled. |
| Settings offline/form failure | `PASS` for render/control scan | 58 settings controls found; save remains intentionally skipped. |
| Wallet/developer/admin restart interruptions | `PASS` for render/control scan | Stable-session retests rendered wallet, developer dashboard/assets, and admin root. |
| Community badge detail fixture | `PASS` | Valid `poool_community.badges` id returned HTTP `200`; old core badge fixture was invalid. |
| Affiliate paid conversion blocker | `PASS` locally | Migrations plus full affiliate E2E verified commission inserts and live-counter deltas. |

### Remaining True Blockers

| Blocker | Why it matters | Required next action |
| --- | --- | --- |
| 2FA setup and step-up return HTTP `500` locally | Sensitive actions cannot be fully tested without 2FA enrollment/confirmation. | Configure `TOTP_SECRET_ENCRYPTION_KEY_OR_ENCRYPTION_KEY`, restart backend, retest `/auth/2fa/setup`, `/auth/2fa`, and `/auth/2fa/step-up` plus one sensitive action that triggers step-up. |
| `poool.finance` DNS fails | Public production entrypoint cannot be reached. | Decide whether `poool.finance` is still a supported domain; if yes, fix DNS and rerun `/` plus `/id/`. |
| Affiliate migration deployment gap | Local paid-conversion tracking is fixed; live conversion tracking is not proven until migrations are deployed. | Deploy migrations `169`, `170`, and the final trigger alias fix `178`, then run a controlled live/staging affiliate conversion with cleanup or refund-safe fixture. |
| Mutating admin/financial/community actions are mostly `SAFE-SKIP` | These are the highest business-risk workflows and were deliberately not submitted in broad crawl tests. | Build isolated seeded E2E fixtures and run focused mutation tests with cleanup around checkout, wallet, trading, developer submissions, support, community, and admin queues. |

### Recommended Next Test Batches

| Batch | Scope | Concrete tests to run next | Target |
| --- | --- | --- | --- |
| 1 | Auth and security | 2FA setup/verify/step-up, forgot/reset token with test outbox, verify-email resend, generic signup without referral, Google OAuth smoke if credentials exist. | Local first, then live/staging read-safe OAuth. |
| 2 | Checkout variants | Cart add/update/remove POSTs, mixed asset cart, disclosure validation, insufficient wallet, duplicate submit, bank-transfer USD/IDR, proof upload, payment-in-progress/success. | Local with DB cleanup. |
| 3 | Wallet and payment methods | Add card validation, add bank validation, deposit submit, withdraw submit, KYC blocker, insufficient funds, transaction detail links. | Local with test user and cleanup. |
| 4 | Developer workflows | Onboarding submit, add asset wizard save/continue/upload/submit, operations monthly report save/submit/upload, annual CapEx/forecast/document upload. | Local with developer fixture. |
| 5 | Developer affiliate team | Invite/approve/remove member, settings save/discard, banking validation, analytics date/resolution, CSV/PDF exports, member/customer/product filters. | Local with developer/team fixture. |
| 6 | Community workflows | Create post, comment, reaction, bookmark, report, follow/unfollow, DM, block/mute, profile edit/upload, notification preference save, ban appeal. | Local with two user fixtures. |
| 7 | Trading and portfolio | Place buy/sell order, validation failures, cancel order, P2P action, trade-success, portfolio row/detail, NFT wallet action where supported. | Local with seeded holdings. |
| 8 | Admin operations | KYC approve/reject, support reply/status, developer review approve/reject/request changes, order/deposit actions, dividends, rewards payouts, marketplace reconciliation/order cancel, community moderation, RBAC/admin invite. | Local with admin fixture and explicit cleanup. |
| 9 | Public/browser interactions | Landing menu/anchors/language/carousels, public property lightbox/calculator/mobile controls, blog category/social/footer, legal embedded links. | Local and live read-only. |
| 10 | Live confidence pass | Authenticated live smoke for investor/developer/admin read-only pages, then one controlled affiliate/signup/conversion after migration deployment. | Live/staging with approved test accounts. |

## Production Readiness Batch 1 Start 2026-05-16 18:23 CEST

### Changes Made

| Area | Change | Production value |
| --- | --- | --- |
| Auth 2FA | Added `backend/tests/auth_2fa_http.rs`, an ignored DB-backed HTTP integration test for `/auth/2fa/setup` and `/auth/2fa/step-up` through the real Axum router. | Prevents 2FA setup/step-up regressions from being hidden behind generic `500` responses. |
| Auth 2FA template rendering | Enabled MiniJinja's `json` feature in `backend/Cargo.toml`. | Fixes `tojson` rendering for step-up 2FA and other templates that safely embed server values into JavaScript. |
| Auth 2FA session rotation | Removed the hard dependency on missing `user_sessions.updated_at` in `rotate_session_token`. | 2FA enrollment no longer 500s when app code is deployed before the optional session timestamp migration. |
| Wallet bank modal | Restored the POOOL trust/brand intro in the add-bank modal. | Sensitive bank-account collection now shows the expected trust context and the regression test covers it. |
| Checkout static regression | Updated the cart FX assertion to require `payments::service::get_usd_to_idr_rate_i64().await` instead of the old static fallback. | Keeps the test aligned with production behavior: FX comes from the backend service, not hardcoded frontend/default data. |

### Verification

| Time | Command / check | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T18:18:20+02:00` | `DATABASE_URL=postgres://martin@localhost/poool RUST_LOG=info cargo test --test auth_2fa_http -- --ignored --test-threads=1 --nocapture` | `PASS` | 2 tests passed. Setup rendered QR/manual secret, submitted a valid TOTP code, stored encrypted `enc:v1` secret, and step-up accepted a valid code. |
| `2026-05-16T18:19:xx+02:00` | `DATABASE_URL=postgres://martin@localhost/poool cargo test --test affiliate_team_integration -- --ignored --test-threads=1 --nocapture` | `PASS` | 13 tests passed; personal/business attribution, commission rate branches, live-counter trigger path, team membership, and self-referral guards remain green. |
| `2026-05-16T18:20:xx+02:00` | Local DB trigger inspection | `PASS` | `affiliate_commissions_counter_sync_*` function bodies no longer contain `new_table`/`old_table`; installed triggers reference `new_rows`/`old_rows`. |
| `2026-05-16T18:20:xx+02:00` | Local `_schema_migrations` inspection | `PASS` | `169_commission_round2_fixes.sql`, `170_affiliate_commission_currency.sql`, and `178_trigger_transition_table_alias_fix.sql` are marked applied locally. |
| `2026-05-16T18:22:xx+02:00` | `python3 -m pytest tests/test_checkout_e2e_remediation_static.py tests/test_wallet_deposit_modal_static.py tests/test_wallet_bank_modal_static.py tests/test_wallet_payment_methods_static.py tests/test_payment_in_progress_static.py` | `PASS` | 15 focused checkout/wallet/payment-progress static regressions passed. |
| `2026-05-16T18:23:xx+02:00` | `DATABASE_URL=postgres://martin@localhost/poool cargo check` | `PASS` | Backend compiled successfully with the MiniJinja JSON feature and auth service change. |

### Updated Blocker Status

| Previous blocker | Current status | Remaining action |
| --- | --- | --- |
| 2FA setup and step-up return local `500` | Fixed in code and covered by `auth_2fa_http`. | Set `TOTP_SECRET_ENCRYPTION_KEY` in every runtime environment and run the ignored DB test as part of pre-release checks. |
| Affiliate local paid-conversion trigger crash | Fixed locally; regression suite still green. | Deploy migrations `169`, `170`, and `178` to staging/live, then run one controlled live/staging affiliate conversion. |
| Checkout/wallet static baseline | Green after aligning stale FX test and restoring bank-modal trust UI. | Next step is a real mutation E2E for cart add/update/remove, bank transfer, proof upload, deposit, and withdrawal with cleanup. |

## Production Readiness Batch 2 Start 2026-05-16 18:52 CEST

### Changes Made

| Area | Change | Production value |
| --- | --- | --- |
| Checkout and cart HTTP integration | Added `backend/tests/checkout_wallet_http.rs`, an ignored DB-backed router test for `/cart/add`, `/cart/update`, `/cart/remove`, `/checkout`, `/payment-success`, and `/payment-in-progress`. | Converts render-only cart/checkout coverage into workflow-level checks with database assertions and cleanup. |
| Checkout guard coverage | Covered missing-disclosure rejection, insufficient-wallet rejection, successful wallet checkout, idempotency replay, and bank-transfer acknowledgement/proof requirements. | Verifies that checkout blocks unsafe submissions before mutation, completes valid wallet purchases exactly once, and routes bank-transfer purchases into pending review. |
| Wallet and payment methods | Covered invalid card token rejection, bank-account save masking/tokenization behavior, deposit-init idempotency, deposit proof upload, over-withdrawal rejection, and withdrawal idempotency. | Raises wallet/payment-method coverage from visible controls to actual money-flow and side-effect verification. |
| Local file/GCS test isolation | The new test state forces local-only proof placeholders/uploads by clearing `gcs_bucket` inside the test `AppState`. | Keeps local HTTP tests deterministic and independent from workstation GCS credentials while leaving production config behavior unchanged. |
| Backend compile blockers | Fixed narrow build blockers in the new inbox/metrics/notification code: `notifications.rs` now uses `query_as::<_, InboxRow>` for the paginated branches, and public Rustdocs were added for the inbox router and metrics statics. | Restores `cargo check` under the repo's `#![deny(missing_docs)]` policy without changing notification or metrics behavior. |

### Verification

| Time | Command / check | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T18:52:45+02:00` | `DATABASE_URL=postgres://martin@localhost/poool RUST_LOG=info cargo test --test checkout_wallet_http -- --ignored --test-threads=1 --nocapture` | `PASS` | 2 tests passed. Cart add/update/remove, disclosure rejection, insufficient wallet, wallet checkout idempotency, bank-transfer proof flow, payment success/in-progress pages, payment-method validation, deposit init/proof submit, over-withdrawal block, and withdrawal idempotency all passed. |
| `2026-05-16T18:58:35+02:00` | `DATABASE_URL=postgres://martin@localhost/poool cargo check` | `PASS` | Backend compiled successfully after the new HTTP integration test and the narrow inbox/metrics/notification compile fixes. |

### Updated Coverage Status

| Previous gap | Current status | Remaining action |
| --- | --- | --- |
| Cart add/update/remove UI POSTs | Covered through direct HTTP router posts and DB assertions. | Browser-level plus/minus visual behavior can still be checked later. |
| Checkout disclosure-negative validation | Covered. Missing general disclosures return `400` and do not create orders, clear carts, or deduct wallets. | Add referral-specific disclosure-negative test in an affiliate-referral checkout fixture. |
| Insufficient wallet checkout | Covered. Wallet balance and order count remain unchanged. | Add IDR wallet insufficiency branch after FX/live-rate stabilization. |
| Wallet checkout duplicate-submit/idempotency | Covered. Replayed `Idempotency-Key` returns the same order number without a second order or second wallet debit. | Add concurrent double-submit stress test if needed. |
| Bank-transfer USD proof/ack flow | Covered locally. Missing ack/proof returns `400`; valid proof creates a pending order and routes to `/payment-in-progress`. | Add IDR bank-transfer branch and admin approval-to-investment allocation flow. |
| Deposit flow | Covered for `/api/wallet/deposit/init` and `/wallet/deposit/:id/submit`, including proof upload and idempotency. | Add source-of-funds threshold and required supporting-document tests. |
| Withdrawal flow | Covered for over-withdrawal rejection, successful pending withdrawal, ledger row, and idempotency replay. | Add high-value 2FA-required withdrawal, daily-cap, velocity-freeze, held-balance, and KYC-blocker tests. |
| Payment methods | Covered for invalid card token rejection and bank add masking/non-sensitive token behavior. | Add delete/default methods plus stronger server-side bank-field validation tests. |

## Production Readiness Batch 3 Start 2026-05-16 19:10 CEST

### Changes Made

| Area | Change | Production value |
| --- | --- | --- |
| Step-up 2FA durability | Added `database/186_step_up_sessions.sql` and updated `backend/src/auth/step_up.rs` so successful TOTP step-up writes a 15-minute PostgreSQL session as a durable fallback while Redis remains the fast path. | Redis is optional in app config; high-value withdrawals no longer become impossible in environments without Redis after a valid TOTP step-up. |
| High-value withdrawal workflow | Extended `backend/tests/checkout_wallet_http.rs` with a DB-backed router test that blocks a `$600.00` withdrawal before step-up, verifies TOTP through `/api/wallet/step-up/verify`, then allows the withdrawal exactly once. | Covers the sensitive-action workflow that was previously only partially tested at the auth layer. |
| Withdrawal fee alignment | Applied existing local migration `184_withdrawal_fee.sql` and updated the integration assertions to read `withdrawal_fee_cents` dynamically. | Keeps local schema aligned with handler behavior and verifies withdrawals debit `amount + fee` while preserving `fee_cents` on the request row. |
| Deposit source-of-funds | Added source-of-funds tests for missing reason, `other` without detail, large deposit init with `sof_doc_required`, submit without supporting doc, and submit with both proof and SoF document. | Covers AML threshold gates and document enforcement, not only the visible deposit modal path. |
| Referral checkout and bank approval | Added referral fixture coverage: general-only disclosures are rejected for referred users; bank transfer with all referral disclosures creates a pending order; admin approval allocates the investment, completes the order, moves referral to holdback, and creates one USD commission. | Verifies the affiliate-specific checkout compliance path and the manual bank-transfer approval-to-investment workflow. |
| Compile and cleanup blockers | Fixed `page_referral_landing` to return a consistent `Response` shape and extended checkout-wallet fixture cleanup for `affiliate_live_counters`. | Restores build stability and prevents DB fixture leakage from affiliate commission trigger side effects. |

### Verification

| Time | Command / check | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T19:33:51+02:00` | `DATABASE_URL=postgres://martin@localhost/poool RUST_LOG=info cargo test --test checkout_wallet_http -- --ignored --test-threads=1 --nocapture` | `PASS` | 5 tests passed. Existing cart/checkout/wallet paths stayed green; new coverage passed for high-value withdrawal step-up without Redis, deposit SoF thresholds/docs, referral-specific disclosures, bank-transfer approval allocation, and affiliate commission tracking. |
| `2026-05-16T19:34:xx+02:00` | `DATABASE_URL=postgres://martin@localhost/poool cargo check` | `PASS` | Backend compiled successfully after the step-up fallback, referral route compile fix, and expanded checkout-wallet tests. |
| `2026-05-16T19:35:xx+02:00` | Local DB migration/fixture inspection | `PASS` | `_schema_migrations` contains `184_withdrawal_fee.sql` and `186_step_up_sessions.sql`; `users` contains `0` `@checkout-wallet-http.test` fixtures after the final test run. |

### Updated Coverage Status

| Previous gap | Current status | Remaining action |
| --- | --- | --- |
| Referral-specific checkout disclosure-negative test | Covered. Referred users who accept only the three general disclosures receive `400` and no order is created. | Add a browser-level check that the referral disclosures render only for referred users. |
| Bank-transfer admin approval-to-investment allocation | Covered for USD bank transfer. Pending bank order creates no investment; `approve_order` completes the order, allocates the investment, and tracks one affiliate commission. | Add IDR bank-transfer branch and admin UI-button route coverage. |
| Deposit source-of-funds threshold and required supporting document | Covered. Missing reason/detail/doc are blocked and valid proof plus supporting doc persists both paths. | Add admin review/approval path for high-value deposits and source-of-funds document download authorization. |
| High-value 2FA-required withdrawal | Covered. `$600.00` withdrawal blocks until TOTP step-up succeeds, then creates one pending withdrawal with fee preserved. | Add daily-cap, velocity-freeze, held-balance, KYC-blocker, and Redis replay-guard tests. |
| Redis-optional step-up behavior | Fixed with PostgreSQL fallback and covered locally with `redis: None`. | Run one staging/live smoke where Redis is configured to confirm Redis path and replay guard behavior. |
| Withdrawal fee schema drift | Local DB fixed by applying existing `184_withdrawal_fee.sql`; tests now assert dynamic configured fee behavior. | Ensure staging/live migration state includes `184_withdrawal_fee.sql` before deploying current wallet handler. |

## Production Readiness Batch 4 Start 2026-05-16 19:45 CEST

### Changes Made

| Area | Change | Production value |
| --- | --- | --- |
| Withdrawal safety regression coverage | Added a DB-backed HTTP test covering KYC rejection, held-balance availability, daily cap, withdrawal velocity auto-freeze, and new-account cooldown. | Verifies each blocker redirects with the expected reason and does not debit the wallet or create an extra withdrawal request. |
| Velocity freeze schema drift | Added `database/193_users_status_frozen_constraint.sql` to ensure existing databases allow `users.status = 'frozen'`. | Fixes a production-risk drift where the velocity guard could return `withdraw_velocity_frozen` but fail to persist the actual account freeze. |
| Freeze observability | Updated `backend/src/wallet/safety.rs` so failed auto-freezes are logged as errors instead of being silently ignored. | Keeps the withdrawal blocked while surfacing the compliance-state write failure for operators. |
| Compile unblocker | Added the missing `Query` import for the affiliate public leaderboard handler in `backend/src/rewards/routes.rs`. | Restores backend compilation without changing affiliate behavior. |

### Verification

| Time | Command / check | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T19:52:47+02:00` | `DATABASE_URL=postgres://martin@localhost/poool RUST_LOG=info cargo test --test checkout_wallet_http -- --ignored --test-threads=1 --nocapture` | `PASS` | 6 tests passed. New withdrawal safety test verified KYC blocker, held-balance block, daily cap, velocity auto-freeze with persisted `users.status='frozen'`, and new-account cooldown after valid step-up session. |
| `2026-05-16T19:53:xx+02:00` | `DATABASE_URL=postgres://martin@localhost/poool cargo check` | `PASS` | Backend compiled successfully after migration/test/safety changes and the rewards import fix. |
| `2026-05-16T19:53:xx+02:00` | Local DB migration/fixture inspection | `PASS` | `_schema_migrations` contains `193_users_status_frozen_constraint.sql`; `users_status_check` now permits `frozen`; `users` contains `0` `@checkout-wallet-http.test` fixtures after cleanup. |

### Updated Coverage Status

| Previous gap | Current status | Remaining action |
| --- | --- | --- |
| Daily-cap withdrawal blocker | Covered. Existing same-day withdrawals plus a new request above the cap redirect with `withdraw_daily_cap` and do not mutate balance or create a second request. | Add admin alert/dashboard visibility checks for cap violations later. |
| Velocity freeze | Covered and fixed. More than 3 recent withdrawals redirects with `withdraw_velocity_frozen` and now persists `users.status='frozen'`. | Add user-facing unfreeze-request API/UI coverage and admin review action coverage. |
| Held-balance blocker | Covered. Cash balance with held funds cannot be withdrawn beyond available balance. | Add a marketplace open-order fixture to prove held balance comes from an actual order path. |
| KYC withdrawal blocker | Covered. Pending KYC redirects with `kyc_required` and leaves wallet/request tables unchanged. | Add browser-level wallet alert check for the KYC error message. |
| New-account cooldown | Covered. Freshly verified user with valid step-up session is blocked above the cooldown cap. | Add boundary tests for exactly-at-cap and after-cooldown expiry. |

## Production Readiness Batch 5 Start 2026-05-16 20:05 CEST

### Changes Made

| Area | Change | Production value |
| --- | --- | --- |
| Frozen-user self-service auth | Added a narrow `get_user_by_session_allowing_frozen` path for frozen-account self-service only. Standard protected routes still require active users. | Fixes the discovered dead-end where a velocity-frozen user could not call the intended `/api/wallet/unfreeze-request` endpoint because normal auth filtered out `frozen` users. |
| Unfreeze request atomicity | Made `/api/wallet/unfreeze-request` insert the compliance alert and stamp `users.unfreeze_requested_at` in one transaction, returning an error if either write fails. | Prevents a false success where a user thinks review was filed but no compliance queue item or user marker was persisted. |
| Admin reactivation cleanup | Updated admin user-status changes so restoring a user to `active` or `suspended` clears `frozen_at`, `frozen_reason`, and `unfreeze_requested_at`; setting `frozen` stamps missing freeze metadata. | Avoids stale freeze-review markers blocking a future legitimate request or confusing admin/user views after reactivation. |
| Freeze-review regression coverage | Added a DB-backed HTTP test covering active-user `not_frozen`, frozen-user request, note trimming/truncation, duplicate request rate limit, compliance-alert details, admin reactivation, and audit logging. | Converts the freeze-review flow from route existence into verified user/admin workflow behavior. |

### Verification

| Time | Command / check | Result | Evidence |
| --- | --- | --- | --- |
| `2026-05-16T20:14:53+02:00` | `DATABASE_URL=postgres://martin@localhost/poool RUST_LOG=info cargo test --test checkout_wallet_http -- --ignored --test-threads=1 --nocapture` | `PASS` | 7 tests passed. New freeze-review test verified frozen-user auth, compliance alert creation, duplicate guard, admin status restore, metadata cleanup, and `admin.user_status_update` audit log. |
| `2026-05-16T20:15:xx+02:00` | `DATABASE_URL=postgres://martin@localhost/poool cargo check` | `PASS` | Backend compiled successfully after auth, wallet, admin, and test changes. |
| `2026-05-16T20:16:xx+02:00` | Local fixture inspection and cleanup | `PASS` | `users` contains `0` `@checkout-wallet-http.test` fixtures; local test upload folders from the run were removed. |

### Updated Coverage Status

| Previous gap | Current status | Remaining action |
| --- | --- | --- |
| User-facing unfreeze-request API | Covered and fixed. Frozen users can authenticate only to the self-service endpoint and file a compliance review request atomically. | Add browser-level wallet banner/button coverage for frozen users. |
| Duplicate unfreeze requests | Covered. A second request within 24h returns `already_requested` and does not create another compliance alert. | Add after-24h boundary behavior if product wants repeat review requests. |
| Admin reactivation after freeze | Covered. Admin status restore clears freeze metadata and writes an audit log. | Add compliance-alert close action into the same admin review workflow. |
| Freeze metadata lifecycle | Covered for admin reactivation. | Add admin manual-freeze reason input later, instead of the current backend fallback `admin_manual`. |
