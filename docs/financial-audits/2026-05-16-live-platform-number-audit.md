# Live Platform Number Audit - 2026-05-16

Audit target: `https://platform.poool.app`

Session/account observed via `/api/me`: `support@traffic-creator.com`, role `super_admin`.

Audit scope:

- Investor navigation reachable in the live session: `/marketplace`, `/commodities-marketplace`, `/marketplace-secondary`, `/my-trading`, `/wallet`, `/portfolio`, `/rewards`, `/cart`, `/leaderboard`, `/community`, `/settings`, `/support`.
- Follow-up investor/dashboard-connected pages checked in the same live session: `/transactions`, one `/transactions/:id` detail page, `/checkout`, `/tier`, `/affiliate`, `/affiliate/dashboard`, `/affiliate/referrals`, `/affiliate/materials`, `/affiliate/settings`, `/settings/notifications/community`, `/account-deletion`, `/kyc`, community tab views, and direct GET checks for `/payment-success`, `/payment-in-progress`, `/trade-success`.
- Developer area checked in the live session: `/developer/dashboard`, `/developer/assets`, `/developer/submissions`, `/developer/operations`, `/developer/affiliate-team`, `/developer/affiliate-team/*`, `/developer/ranking`, `/developer/settings`, `/developer/support`, `/developer/add-asset`, `/developer/application-form`, `/developer/onboarding`, `/developer/property-content`, `/developer/document-upload-step3`, and representative `/developer/asset-detail?id=...` pages.
- Full detail follow-up pass requested after the first audit: all discovered investor-facing `/property/:slug` detail pages, both discovered `/commodity/:slug` detail pages, all 12 `/marketplace-trading-v3?asset=:slug` secondary-market detail pages, `/marketplace-trading-v2`, and community profile/edit/hashtag/badge detail pages.
- Mutating workflow dry-runs: `/checkout`, order/P2P cancellation surfaces, support ticket submission, developer submissions, developer asset edits, deep developer Annual/Operations flows, and community profile edits were inspected without submitting or changing production state.
- API spot checks were performed by opening GET JSON endpoints in the authenticated browser session.

Not in scope for this pass:

- Admin, auth, legal, blog, and affiliate legal/policy documents.
- Actual production mutations: no checkout confirmation, order/P2P cancel, ticket submit/reopen/reply, profile save, verified-owner request, draft submit/delete/duplicate, or asset edit was executed.
- Independent production database queries. Where a page has no JSON endpoint, the audit classifies whether values are rendered from backend SQL/template data or hardcoded by reading the local route/template code.

## Status Legend

| Status | Meaning |
| --- | --- |
| OK | Displayed number matched the checked backend/API source or backend-rendered source path. |
| Mismatch | Displayed number disagrees with another checked source or the label is misleading. |
| Hardcoded | Number is static in HTML/JS/config and will not change from live data. |
| Dynamic but unverified | Rendered from backend/template data, but no separate live API source was available in this pass. |
| Not checked | Route/page not covered in this pass. |

## Executive Findings

| Severity | Page | Finding | Evidence | Dynamic vs hardcoded |
| --- | --- | --- | --- | --- |
| High | `/portfolio` | Portfolio has conflicting current value sources. Main portfolio value shows `$609,500`, but villa summary shows `USD 959,000.00` and `Flat`. `/api/portfolio` says `total_value_cents=60950000`, `total_purchase_cents=95900000`, `total_appreciation_cents=-34950000`. `/api/investors/me/portfolio-villa-summary` says `current_value_cents=95900000`, `unrealised_pnl_cents=0`. | Main card and My Trading agree with `/api/portfolio`; villa summary endpoint disagrees. | Dynamic, but inconsistent between endpoints. |
| High | `/marketplace-trading-v3?asset=*` | Trading detail pages fabricate/derive performance values instead of reading real price history. All 12 checked assets showed fixed `3 months +4.2%`, `6 months +8.7%`, `12 months +14.9%`. `NET RETURN` is `roi * 0.75`, so `8%` becomes `6.0%`, `14%` becomes `10.5%`, and `0%` assets still show the same positive 3/6/12 month performance. | Full pass over 12 assets from `/api/marketplace/secondary/assets`; `frontend/platform/static/js/marketplace-trading-v3.js` maps `netReturn: (rawAsset.roi * 0.75).toFixed(1)` and derives fixed 3m/6m/12m values. | Hardcoded formula, not live historical data. |
| High | `/marketplace-trading-v2` | Trading V2 is a static mock page and ignores `?asset=`. It always shows `Bali Villa Canggu #12`, `$105.00`, `12.4%`, `87%`, `1,000`, fake investors, `5%` fee, `$5.25`, `$110.25`, and `USD 350,000`. | Checked `/marketplace-trading-v2` and `/marketplace-trading-v2?asset=grand-pavilion-ubud-estate`; values were identical and unrelated to the selected asset. | Hardcoded/mock. |
| Medium | `/my-trading` | Tab label/count is misleading and the cancel UI is exposed for filled orders. `Open Orders 21` and the table show `Cancel` buttons for 20 `filled` rows, while the summary card says `1 Open Orders`. `/api/marketplace/orders/mine` returned 21 orders: 1 `partially_filled`, 20 `filled`. | `renderSummaryCards()` filters open-like statuses; `renderOpenOrders()` sets tab count to `state.orders.length` and renders all statuses with an unconditional cancel button. Backend cancellation correctly rejects non-active orders. | Dynamic data, wrong filtering/action rendering. |
| High | `/developer/villas/:asset_id/operations/new` | Monthly Operations can be saved and submitted with all financial inputs empty/zero. UI fields are not required; `submitForApproval()` calls `saveDraft()` first; backend only validates period range and then allows `status='submitted'`. | Checked live form for May 2026: all main operation inputs had `required=false`; backend `api_developer_villa_operations_create`/`submit` has no nonzero or nights validation beyond `period_year/month`. | Dynamic workflow, weak validation. |
| Medium | `/developer/villas/:asset_id/operations/:log_id` | Operations dashboard JS links existing draft/rejected/submitted periods to `/developer/villas/:asset_id/operations/:log_id`, but no GET page route is registered. A direct GET returned 404. | `developer-operations-dashboard.js` builds those links; `backend/src/developer/mod.rs` registers only `/operations/new` as a page route and `PUT` APIs for `/:log_id`. | Route gap; dynamic links will break once logs exist. |
| Medium | `/developer/villas/:asset_id/annual/:year` | Forecast suggestions can be submitted with all forecast fields empty and no notes. UI fields are optional and backend inserts a `submitted` suggestion without checking that at least one field is present. | `developer-annual-data.js` builds nullable payload; `forecast_suggestions.rs` validates only `forecast_year`. | Dynamic workflow, weak validation. |
| Medium | `/property/:slug` | Property detail pages mix dynamic asset values with static/formula calculator values. Many calculators share `$100,000` default investment and `$61,051` value appreciation; several pages show `net yield` higher than `gross yield`, including `19.20%` net on `12.00%` gross and `25.00%` net on `0.00%` gross. | Full pass over 8 property detail pages. | Mixed: asset values dynamic/backend-rendered; calculator and some derived yield copy need source-of-truth validation. |
| Medium | `/commodity/premium-bali-rice-q2-2026` | Rice commodity detail shows `450 Trees` as the underlying asset, which appears copied from the cacao template. | Rice page showed `$50`, `12.00%`, `6 months`, `450 Trees`, `$50,000`. | Dynamic/template-rendered field or hardcoded copy needs correction; visible value is wrong for rice. |
| Medium | `/property/demo-villa-investment-01-...` | Demo Villa page has conflicting property facts: metadata says `2` beds, `2` baths, `600 m²`; body copy says `3-bedroom`, `300-square-meter land`, `220 square meters`, and `three and a half bathrooms`. | Same live page contains both sets of values. | Dynamic content conflict or stale hardcoded description. |
| High | `/community/hashtag/hashtags`, `/community/badge/2f90c212-bb54-45a8-818e-abaec5e5e0a9` | Community detail pages crash at render time, while their APIs return data. Hashtag page errors on `partials/community_post_list.html`; badge page errors on missing MiniJinja `truncate` filter. | `/api/community/hashtags/hashtags` returned 1 post; `/api/community/badges/...` returned `holder_count=5` and 5 recent holders. | Dynamic APIs OK; SSR templates broken. |
| Medium | `/cart` | IDR conversion currently matches API/config at `15500`, but frontend recalculations use hardcoded `IDR_RATE = 15500` in JS. If the backend rate changes, quantity edits will drift. | `/api/cart` returned `usd_to_idr_rate=15500`; `frontend/platform/static/js/cart.js` defines `const IDR_RATE = 15500` in multiple update paths. | Hardcoded frontend constant. |
| Low | `/rewards` | Referral amounts currently match API, but JS has hardcoded fallback amounts: friend `3000` cents, user `3000` cents, required investment `100000` cents. | `/api/rewards` returned those values; `rewards-service.js` falls back to the same constants if API fields are absent. | Dynamic when API present; hardcoded fallback risk. |
| Low | `/property/...demo-apartment...` | Calculator default and timeline helper values are static UX assumptions: `$2,000`, `5 years`, `2 weeks`, `4 weeks`. Yield and price data are backend-rendered. | Visible detail page and `property.html`/`property-detail.js` use fixed calculator defaults and timeline text. | Mixed: financial asset values dynamic, calculator/timeline defaults hardcoded. |
| Low | `/support` | SLA and form constraints are hardcoded in UI but match backend support logic today: normal `~4 hours`, high `~2 hours`, low `~24 hours`, attachment `5MB`, subject `255`, message `20/5000`. | `support.html` and `support.js`; backend `support/db.rs` documents urgent/high/normal/low breach hours and `support/service.rs` uses `MAX_ATTACHMENT_BYTES = 5 * 1024 * 1024`. | Hardcoded policy mirrored in backend. |
| High | `/developer/dashboard` | Visible KPI counters stay at zero even though the page carries correct dynamic `data-final-value` attributes and `/api/developer/dashboard/stats` returns real values. Examples: visible `Total Assets 0`, `Funding Target $0`, `Amount Raised $0k`; API/final values are `2`, `$1.5M`, `$27.0k`. | Rechecked after waiting 3.5s and another 8s; DOM still showed `0` while `data-final-value` remained correct. | Dynamic backend values exist, but frontend counter animation leaves wrong visible values. |
| Medium | `/affiliate` | Affiliate promo page contains a hardcoded 8-tier commission ladder and calculator. The inline comment says thresholds are placeholders pending legal sign-off. | Visible values include `0.50%` through `4.50%`, `$5,000` through `$2,500,000`, default calculator `USD 10,000`, `5 referrals`, `60` yearly referrals, `$600,000`, `$6,600`. | Hardcoded mirror, not live API-driven. |
| Medium | `/payment-success`, `/payment-in-progress` | Direct GET routes load `/api/orders/latest`. Latest order is `completed` bank order `ORD-20260506011524-776729`, total `210000` cents, but `/payment-success` labels it `Payment In Progress` because JS treats any bank payment as pending. `/payment-in-progress` labels the same order `Payment Confirmed`. | `/api/orders/latest` returned `status=completed`, `payment_method=bank`, `total_cents=210000`; visible route titles disagree. | Dynamic latest-order data, but status logic/route semantics are inconsistent. |
| Medium | `/developer/affiliate-team/*` | Base team metrics render zeros correctly for an empty team, but several live routes/endpoints expected by code are missing: `/developer/affiliate-team/analytics`, `/developer/affiliate-team/members`, `/api/developer/affiliate/team/analytics/overview`, `/api/developer/affiliate/team/analytics/timeseries` return 404. | Live base `/developer/affiliate-team` loaded; team API returned `active_members=0`. Missing routes returned the platform 404 page. | Mixed: base metrics dynamic; analytics/member subroutes not deployed/reachable. |
| Low | `/checkout` | Current amounts match `/api/cart`, but checkout has its own `displayIdrRate()` fallback of `15500`. It also rounds the same nonzero funding progress to `0%`, while `/cart` shows `1%` for the same item. | `/api/cart` returned one item, `total_cents=400000`, `usd_to_idr_rate=15500`, sold `2000/500000 = 0.4%`; checkout showed `USD 4,000.00`, `IDR 62,000,000`, `0% funded`. | Dynamic cart data with hardcoded FX fallback and inconsistent rounding. |

## Remediation Progress

Status: local code changes made on 2026-05-16; not yet deployed to the live site.

| Finding | Local remediation | Files changed | Verification |
| --- | --- | --- | --- |
| `/cart` and `/checkout` hardcoded USD→IDR fallback/update rate | `/api/cart` and server-rendered checkout JSON now use `payments::service::get_usd_to_idr_rate_i64()` instead of returning `DEFAULT_USD_TO_IDR_RATE_I64`. Cart server render passes the backend rate as `data-usd-to-idr-rate`. Cart/checkout JS no longer falls back to `15500`; if no backend rate is available, IDR subtitles render empty instead of fabricated. | `backend/src/cart/routes.rs`, `backend/src/payments/routes.rs`, `backend/src/payments/service.rs`, `frontend/platform/static/js/cart.js`, `frontend/platform/checkout.html` | `cargo fmt`, `cargo check --manifest-path backend/Cargo.toml`, `node --check frontend/platform/static/js/cart.js`. Local protected pages redirect to `/auth/login` without a local session, so browser UI confirmation was not completed. |
| `/marketplace-trading-v3?asset=*` fake performance/net return and fee fallback | V3 no longer derives `netReturn` from `roi * 0.75`, no longer fabricates 3/6/12 month performance from fixed price multipliers, and no longer defaults missing platform fee to `5%`. It reads optional API percent/bps fields when present and shows `—` when no real source exists. | `frontend/platform/static/js/marketplace-trading-v3.js`, `frontend/platform/marketplace-trading-v3.html` | `node --check frontend/platform/static/js/marketplace-trading-v3.js`; hardcoded formula scan no longer finds the old fake performance formulas. |
| `/my-trading` open-orders count/table/cancel mismatch | Open Orders now filters to backend-active statuses only (`open`, `partially_filled`), uses the same filtered set for the count and bulk selection, and clears selected IDs that are no longer active after refresh. Filled orders no longer render in the open-orders table, so they no longer get cancel buttons there. | `frontend/platform/static/js/my-trading.js` | `node --check frontend/platform/static/js/my-trading.js`; backend contract checked against `MarketOrder::is_active()` and `DELETE /api/marketplace/orders/:id` docs. |

## Checked Pages

### `/marketplace` - Properties

Status: Dynamic but unverified against independent API.

Source path: `backend/src/assets/routes.rs::page_marketplace` queries `assets`, `asset_images`, investments counts and token sums, then renders `frontend/platform/marketplace.html` with `PropertyDisplayData`.

Checked visible numbers:

| Number | Context | Status | Notes |
| --- | --- | --- | --- |
| `2`, `1`, `72 sqm` | Demo Apartment 01 card metadata | Dynamic but unverified | From asset DB fields `bedrooms`, `bathrooms`, `building_size_sqm`. |
| `USD 500,000` | Demo Apartment 01 price | Dynamic but unverified | From `total_value_cents`. |
| `0.0% funded` | Demo Apartment 01 funding | Dynamic but unverified | Derived from tokens sold/available in backend display model. |
| `2`, `2`, `600 sqm` | Demo Villa 01 card metadata | Dynamic but unverified | From asset DB fields. |
| `USD 1,000,000` | Demo Villa 01 price | Dynamic but unverified | From `total_value_cents`. |
| `3.0% funded` | Demo Villa 01 funding | Dynamic but unverified | Derived from token data. |

Open item: add a read-only asset-detail/listing API or admin reconciliation query for independent production DB validation.

### `/commodities-marketplace`

Status: Dynamic but unverified against independent API.

Source path: `backend/src/assets/routes.rs::page_commodities_marketplace` queries commodity assets and renders `CommodityDisplayData`.

Checked visible numbers:

| Number | Context | Status | Notes |
| --- | --- | --- | --- |
| `60 ha`, `USD 80,000`, `40% funded`, `12 months`, `0.00% projected return`, `15.00% projected annualised net return` | Organic Cacao card | Dynamic but unverified | From commodity asset DB fields/display model. |
| `60 ha`, `USD 50,000`, `65% funded`, `6 months`, `0.00% projected return`, `12.00% projected annualised net return` | Premium Bali Rice card | Dynamic but unverified | From commodity asset DB fields/display model. |

### `/marketplace-secondary`

Status: OK for checked counts and listing values.

API checked: `/api/marketplace/secondary/assets`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `12` | All Assets | API array length `12` | OK | Dynamic from API. |
| `1` | With Offers | `assets.filter(sellOrders > 0).length = 1` | OK | Dynamic client calculation. |
| `1` | Buy Interest | `assets.filter(buyInterest > 0).length = 1` | OK | Dynamic client calculation. |
| `11` | No Offers | `assets.filter(sellOrders === 0).length = 11` | OK | Dynamic client calculation. |
| `USD 500/share`, `100% funded`, `60 months`, `12% projected return`, `8% annualised net return` | Grand Pavilion card | API first asset `price=50000`, `fundingProgressPct=100`, `termMonths=60`, `capitalAppreciationBps=1200`, `roi=8` | OK | Card values are dynamic. |

Risk: card sparklines are generated in JS if used; no real price history was validated for this page.

### `/marketplace-trading-v3?asset=grand-pavilion-ubud-estate`

Status: Mismatch and hardcoded formula risk.

APIs checked: `/api/marketplace/secondary/assets`, `/api/marketplace/:asset/orderbook` path is attempted by JS.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `$500.00/share` | Hero and ticker share price | `price=50000` cents | OK | Dynamic from secondary asset API or orderbook fallback. |
| `+8%` / `8% annual yield` | Yield | API `roi=8` | OK | Dynamic from API. |
| `USD 12,000,000` | Property value | API `propertyValue=1200000000` cents | OK | Dynamic from API. |
| `24,000 / 24,000` | Available | API `totalSupply=24000`, current sell order fallback `0` for this asset | OK | Dynamic calculation, though terminology should be reviewed. |
| `6.0%` | Net return | API/listing annualised net return is `8%` | Mismatch | JS hardcodes `roi * 0.75`; not sourced from API. |
| `+4.2%`, `+8.7%`, `+14.9%` | 3/6/12 month performance | No checked API source | Hardcoded | JS derives from fixed current-price multipliers, not historical data. |
| `5%`, `USD 600,000`, `USD 12,600,000` | Platform fee and total investment cost | Uses `window.POOOL_FEE_PCT || 5` and property value | Hardcoded fallback | Fee should come from platform settings/API for this frontend path. |

### `/my-trading`

Status: Mostly OK, with one count mismatch.

APIs checked: `/api/portfolio`, `/api/investors/me/positions-nav`, `/api/marketplace/orders/mine`, `/api/marketplace/trades/mine`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `$609,500.00` | Portfolio value | `/api/portfolio.total_value_cents=60950000` | OK | Dynamic. |
| `-$349,500.00` | All-time return | `/api/portfolio.total_appreciation_cents=-34950000` | OK | Dynamic. |
| `+10.00%` | Annual yield | `/api/portfolio.annual_yield_bps=1000` | OK | Dynamic. |
| `$0.00` | Monthly income | `/api/portfolio.monthly_income_cents=0` | OK | Dynamic. |
| `1` | Summary open orders | 1 order in open-like statuses (`partially_filled`) | OK | Dynamic. |
| `32` | Completed trades summary | `/api/marketplace/trades/mine` length `32` | OK | Dynamic. |
| `2` | My assets summary | `/api/portfolio.investments.length=2` | OK | Dynamic. |
| `0` | Active buy interests | no outgoing P2P offers observed | OK | Dynamic. |
| `Open Orders 21` plus `Cancel` on filled rows | Open orders tab/table | `/api/marketplace/orders/mine` length `21`, status distribution `1 partially_filled`, `20 filled` | Mismatch | Label says open but count includes filled orders; cancel buttons are rendered even for `filled` rows that backend will reject. |

### `/wallet`

Status: OK.

APIs checked: `/api/wallet/balance`, `/api/wallet/transactions`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `USD 1,367,189.99` | Cash balance | `cash_display=USD 1,367,189.99` | OK | Dynamic. |
| `USD 0.00` | Rewards balance | `rewards_display=USD 0.00` | OK | Dynamic. |
| `5` | Transaction count | `/api/wallet/transactions.total=5` | OK | Dynamic, not shown as a header in current visible sample but table has 5 rows. |
| `USD 9,000.00`, `USD 500,000.00`, `USD 9.99` | Deposit rows | Transaction `amount_cents` values `900000`, `50000000`, `999` | OK | UI formats with `USD` and grouping; API amount_display lacks grouping but cents match. |
| `01 May 2026`, `22 Mar 2026` | Deposit dates | API `created_at` dates | OK | Dynamic formatted dates. |

### `/portfolio`

Status: Mismatch between dynamic endpoints.

APIs checked: `/api/portfolio`, `/api/investors/me/portfolio-villa-summary`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `$609,500` | Main portfolio value | `/api/portfolio.total_value_cents=60950000` | OK | Dynamic. |
| `-36.4%` | Main portfolio appreciation | `-34950000 / 95900000 = -36.44%` | OK | Dynamic calculation. |
| `Rp 10.720.495.500` | IDR subtitle | `$609,500 * inferred 17,589` | Needs source check | The FX rate is not obvious from checked API output. |
| `$0` | Monthly income | `/api/portfolio.monthly_income_cents=0` | OK | Dynamic. |
| `$0` | Total rental income | `/api/portfolio.total_rental_cents=0` | OK | Dynamic. |
| `$-349,500` | Total appreciation | `/api/portfolio.total_appreciation_cents=-34950000` | OK | Dynamic. |
| `2` | Number of properties | `/api/portfolio.investment_count=2` | OK | Dynamic. |
| `99%` | Occupancy rate | `/api/portfolio.occupancy_rate_bps=9934` | OK | Rounded display. |
| `10.0%` | Annual rental yield | `/api/portfolio.annual_yield_bps=1000` | OK | Dynamic. |
| `USD 959,000.00` | Villa summary current value | `/api/investors/me/portfolio-villa-summary.current_value_cents=95900000` but `/api/portfolio.total_value_cents=60950000` | Mismatch | Summary endpoint disagrees with main portfolio source. |
| `Flat` | Villa summary PnL | Summary endpoint `unrealised_pnl_cents=0`; `/api/portfolio` says `-34950000` | Mismatch | Same root cause as current value conflict. |

### `/rewards`

Status: OK with hardcoded fallback risk.

API checked: `/api/rewards`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `USD 959,000` | Invested last 12 months | `invested_12m=95900000` cents | OK | Dynamic. |
| `Maximum Tier Reached` / progress complete | `progress_pct=100`, `tier_target=null` | OK | Dynamic. |
| `USD 0` | Total rewards, cashback, referrals, promotions | all API values `0` | OK | Dynamic. |
| `USD 30`, `USD 30`, `USD 1,000` | Referral card | API `friend_reward_cents=3000`, `user_reward_cents=3000`, `investment_required_cents=100000` | OK | Dynamic today; JS fallback hardcodes same values. |

### `/cart`

Status: OK for current render; hardcoded FX risk in JS updates.

API checked: `/api/cart`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `1` | Cart item count | `/api/cart.count=1` | OK | Dynamic. |
| `2`, `1`, `72 sqm` | Cart item metadata | API item `bedrooms=2`, `bathrooms=1`, `building_size_sqm=72` | OK | Dynamic. |
| `$1` | Share price | API item `token_price_cents=100` | OK | Dynamic. |
| `10.0%` | Yield | API item `annual_yield_bps=1000` | OK | Dynamic. |
| `USD 4,000.00` | Item total | `4000 tokens * $1` | OK | Dynamic calculation. |
| `1% funded` | Funding text | `(500000 - 498000) / 500000 = 0.4%`, rounded/min display `1%` | OK | Dynamic calculation with minimum nonzero display rule. |
| `4000 x $1` | Summary quantity | API `tokens_quantity=4000`, `token_price_cents=100` | OK | Dynamic. |
| `Rp 62.000.000` | IDR total | API `usd_to_idr_rate=15500`, `$4000 * 15500` | OK | Hardcoded JS update risk if quantity changes. |
| `09:59` | Reservation timer | Client countdown | Dynamic client state | Timer is not a backend financial value. |

### `/leaderboard`

Status: OK for checked values.

APIs checked: `/api/leaderboard`, `/api/leaderboard/me`, `/api/leaderboard/preferences`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `#2` | Your standing | `/api/leaderboard/me.rank=2` | OK | Dynamic. |
| `EUR 959.0K` | Holdings summary | `metric_value=95900000` cents | OK | Currency symbol is EUR in UI although platform data is cents from invested metric; confirm intended display currency. |
| `2` | Assets summary | `asset_count=2` | OK | Dynamic. |
| `10.0%` | Yield summary | `portfolio_roi_bps=1000` | OK | Dynamic. |
| `0` | Affiliates summary | `affiliate_count=0` | OK | Dynamic. |
| `5` | Total participants | `/api/leaderboard.total_participants=5` | OK | Dynamic. |
| `16.146.080,00 EUR`, `8 assets`, `+9.1%` | Rank 1 card | First ranking `total_invested_cents=1614608000`, `asset_count=8`, `portfolio_roi_bps=912` | OK | Dynamic formatted values. |
| `100% Direct`, `5% Referral` | Rank 1 legend | Not present in checked API ranking payload | Hardcoded/unknown | Needs source check; likely decorative/static split. |

### `/community`

Status: OK for checked values.

APIs checked: `/api/community/profile/me`, `/api/community/hashtags/trending`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `0 posts`, `0 followers`, `0 following` | My profile card | `post_count=0`, `follower_count=0`, `following_count=0` | OK | Dynamic. |
| `1 post`, `1 post` | Trending hashtags | API returned two hashtags with `post_count=1` | OK | Dynamic. |
| `2 badges` | Profile API | API `badges.length=2` | Not visibly audited | API observed but visible extraction did not capture badge count text. |

### `/settings`

Status: Mostly hardcoded validation/UI constraints plus dynamic profile/account data.

API checked: `/api/settings`.

Checked visible numbers:

| Number | Context | Source | Status | Notes |
| --- | --- | --- | --- | --- |
| `Web3`, `2FA` | Section labels/actions | Static UI labels | Hardcoded | Not business metrics. |
| `0xfD32...9fF0` | Wallet address | `/api/me`/settings profile chain wallet surface | Dynamic | Address itself was visible and should remain dynamic. |
| `UTC-09:00` | Timezone | `/api/settings.timezone=America/Anchorage` | OK | Dynamic selected value. |
| `0/300` | Community bio counter | UI maxlength counter | Hardcoded constraint/dynamic count | Counter is client state. |
| `400x400`, `2 MB` | Developer logo upload guidance | Static UI constraint | Hardcoded | If enforced elsewhere, keep constants centralized. |
| `Real Estate 101` | Learning link title | Static content | Hardcoded | Not user/account data. |

### `/support`

Status: OK for ticket counts; SLA/form constraints are hardcoded policy.

API checked: `/api/support/tickets`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `All 2`, `Open 0`, `Resolved 2` | Ticket tabs | `/api/support/tickets.tickets.length=2`, both `closed` | OK | Dynamic. |
| `Apr 30 19:10` | Ticket created date | API first ticket `created_at=2026-04-30 17:10:39+00`; browser/local display +2h | OK | Dynamic formatted local time. |
| `~4 hours`, `~2 hours`, `~24 hours` | SLA guidance | Backend `support/db.rs` uses normal/high/low breach hours | OK but hardcoded | UI and backend currently match. |
| `0/255`, `20`, `0/5000`, `5MB` | Form constraints | HTML attributes/JS/backend validation | OK but hardcoded | Backend also enforces attachment max. |
| `24-48 hours` | KYC help text | Static copy | Hardcoded | Not tied to a live SLA endpoint. |

## Additional Investor Pages Checked

### `/transactions`

Status: OK.

API checked: `/api/wallet/transactions`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `5` | Transaction rows | API `total=5`, `count=5` | OK | Dynamic. |
| `01 May 2026`, `22 Mar 2026` | Transaction dates | API `created_at` dates | OK | Dynamic formatted dates. |
| `+ USD 9,000.00`, `+ USD 500,000.00`, `+ USD 9.99` | Transaction rows | API `amount_cents=900000`, `50000000`, `999` | OK | Dynamic. |

### `/transactions/57a53f0b-303c-4b1f-8eea-0e91e0204bfa`

Status: OK.

API checked: `/api/wallet/transactions/57a53f0b-303c-4b1f-8eea-0e91e0204bfa`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `USD 9,000.00` | Transaction amount | `amount_cents=900000`, `amount_display=USD 9,000.00` | OK | Dynamic. |
| `01 May 2026 at 15:27 UTC` | Created / paid at | API `date_full` and deposit detail rows | OK | Dynamic. |
| `02 May 2026 at 03:42 UTC` | Expires | API deposit detail row | OK | Dynamic. |
| `STRIPE-20260501034223-2175429c` | Reference | API deposit detail row | OK | Dynamic. |

### `/checkout`

Status: OK for current values; hardcoded FX fallback and rounding inconsistency.

APIs checked: `/api/cart`, `/api/wallet/balance`, `/api/payment-methods`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `2 Beds`, `1 Bath`, `72 m²`, `120 m² land` | Checkout item metadata | `/api/cart.items[0]` | OK | Dynamic. |
| `$1`, `10.0%` | Share price and annual yield | `token_price_cents=100`, `annual_yield_bps=1000` | OK | Dynamic. |
| `USD 4,000.00`, `IDR 62,000,000` | Item total | `total_cents=400000`, `usd_to_idr_rate=15500` | OK | Dynamic today. |
| `0% funded` | Funding progress | `2000 / 500000 = 0.4%` sold | Needs consistency review | Rounded to `0%` on checkout; `/cart` uses a minimum nonzero display and shows `1%`. |
| `15:00` | Reservation timer | Client-side countdown | Dynamic client state | Not a backend financial value. |
| `0` | Payment methods | `/api/payment-methods.payment_methods.length=0` | OK | Dynamic. |

Code risk: `checkout.html` uses `displayIdrRate()` fallback `15500` if the backend-provided cart rate is missing.

### `/tier` and Rewards Tabs

Status: OK for checked values; same fallback risk as `/rewards`.

APIs checked: `/api/rewards`, `/api/rewards/tiers`, `/api/rewards/commissions`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `Premium`, `USD 959,000`, `100%` | Current tier/progress | `/api/rewards.tier_name=Premium`, `invested_12m=95900000`, `progress_pct=100` | OK | Dynamic. |
| `5` | Rewards tiers | `/api/rewards/tiers` returned 5 tiers | OK | Dynamic tier list for rewards tab. |
| `0` | Commissions | `/api/rewards/commissions.commissions.length=0` | OK | Dynamic. |
| `USD 5`, `USD 50`, `USD 100`, `USD 150`, `USD 200` | Tier referral reward cards | `/api/rewards/tiers` tier data | OK for sampled visible values | Dynamic when API loads. |

### `/affiliate` and Affiliate Subpages

Status: Mixed: public promo numbers are hardcoded; private affiliate dashboard APIs are correctly gated for this account.

APIs checked: `/api/affiliate/dashboard`, `/api/affiliate/referrals`, `/api/affiliate/settings`, `/api/affiliate/materials`, `/api/affiliate/subid-stats`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `$12M+`, `24`, `8`, `4.50%` | Affiliate promo hero stats | No live API source | Hardcoded | Marketing/program copy, not account data. |
| `0.50%` to `4.50%`, `$5,000` to `$2,500,000` | 8-tier ladder | Inline JS/HTML hardcoded mirror | Hardcoded | Comment says thresholds are placeholders pending legal sign-off. |
| `USD 10,000`, `5`, `60`, `$600,000`, `$6,600` | Earnings calculator defaults/results | Inline JS hardcoded defaults and formulas | Hardcoded | Not connected to `/api/rewards/tiers` or affiliate API. |
| Affiliate dashboard/referrals/materials/settings | Private affiliate pages | APIs returned active-affiliate errors | OK/gated | Account is not an active affiliate; `/affiliate/dashboard` and `/affiliate/referrals` redirect to onboarding. |
| `$50.00`, `Jan 31st` | Affiliate settings threshold/tax copy | Static UI copy | Hardcoded | Page is gated; values are policy copy. |

### `/settings/notifications/community`, `/account-deletion`, `/kyc`

Status: Mostly no financial/account metrics; static policy/constraint values.

APIs checked: `/api/community/notifications/preferences`, `/api/community/notifications/unread-count`, `/api/kyc/status`, `/api/kyc/provider`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `11` | Community unread notifications | `/api/community/notifications/unread-count.count=11` | API checked only | No visible count captured on notifications settings page. |
| `approved`, `manual` | KYC status/provider | `/api/kyc/status.status=approved`, `provider=manual` | OK | KYC page had no visible numeric values in the captured state. |
| `GDPR Article 17`, `5-10 years`, `30 days`, `10 seconds` | Account deletion copy | Static policy/process copy | Hardcoded | Not a financial number; retention/UX timing should be kept policy-backed. |

### Community Tab Views

Status: OK for checked API-backed values; several views are empty for this account.

APIs checked: `/api/community/profile/me`, `/api/community/xp`, `/api/community/xp/history?page=1`, `/api/community/circles/me`, `/api/community/circles/leaderboard`, `/api/community/leaderboard?limit=50&period=all`, `/api/community/members`, `/api/community/bookmarks`, `/api/community/dms/threads`, `/api/community/challenges`, `/api/community/amas`.

Checked visible/API numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `Level 10`, `45,495 XP`, `2,147,438,152 XP to next level`, `1-day streak` | My Circle XP card | `/api/community/xp` | OK | Dynamic. |
| `20` | XP history entries | `/api/community/xp/history?page=1.entries.length=20` | OK | Dynamic API count. |
| `1 members`, `68,760 XP` | Circle leaderboard | `/api/community/circles/leaderboard.circles[0]` | OK | Dynamic. |
| `#1`, `#2`, `68,760 XP`, `45,495 XP`, `15` rows | Community leaderboard | `/api/community/leaderboard` | OK | Dynamic. |
| `15`, `30` | Members list/page size | `/api/community/members.members.length=15`, `page_size=30` | OK | Dynamic. |
| `5` | Challenges | `/api/community/challenges.challenges.length=5` | OK | Dynamic API; visible capture did not enumerate all cards. |
| `0` | AMAs, bookmarks, DM threads | corresponding APIs returned empty arrays | OK | Dynamic empty states. |

### Payment/Trade Result Pages

Status: Mixed; direct GET result pages are dynamic but semantically inconsistent without explicit IDs.

APIs checked: `/api/orders/latest`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `ORD-20260506011524-776729`, `$2,100.00`, `4`, `$500.00`, `$2,000.00` | `/payment-success` and `/payment-in-progress` direct GET | `/api/orders/latest` returned same order, `status=completed`, `payment_method=bank`, `total_cents=210000`, one item `4 * 50000` cents | OK data, mismatched status labels | `/payment-success` says `Payment In Progress`; `/payment-in-progress` says `Payment Confirmed`. |
| `1-3 business days` | Bank transfer processing copy | Static JS copy | Hardcoded | Policy/SLA copy. |
| `$0.00`, `0 share` | `/trade-success` direct GET without session data | sessionStorage/URL fallback absent | Hardcoded fallback | Direct page load has placeholder trade values; meaningful values require order/session context. |

## Developer Area Checked

### `/developer/dashboard`

Status: Mismatch in visible KPI counters.

API checked: `/api/developer/dashboard/stats`.

Checked numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `0`, `$0`, `$0k`, `$0` | Visible top KPI cards | API/final values are `2`, `$1.5M`, `$27.0k`, `$1.5M` | Mismatch | Counter animation leaves the text at zero. |
| `2`, `$1.5M`, `$27.0k`, `$1.5M` | `data-final-value` attributes | `/api/developer/dashboard/stats` | OK source, not visible | Backend/template data is correct. |
| `16`, `6`, `3`, `12.5%`, `1.8%`, `$9.0k` | Secondary metrics in API | `/api/developer/dashboard/stats` | Source OK | Visible counters were also stuck at zero for sampled secondary metrics. |

### `/developer/assets`

Status: OK.

API/source checked: `/api/developer/dashboard/stats`, backend-rendered `developer_assets`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `2`, `$1.5M`, `$27.0k`, `2` | Summary cards | dashboard stats API | OK | Dynamic. |
| `All 2`, `Available 2`, `Funded 0` | Filters | two listed assets, none fully funded | OK | Client count from rendered rows. |
| `2.5%`, `$975.0k left`, `USD 1,000,000`, `30 yrs`, `$25.0k` | Demo Villa row | developer asset/stats data | OK | Dynamic backend-rendered row. |
| `0.4%`, `$498.0k left`, `USD 500,000`, `25 yrs`, `$2.0k` | Demo Apartment row | developer asset/stats data | OK | Dynamic backend-rendered row. |

### `/developer/submissions`

Status: OK.

API checked: `/api/developer/drafts`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `7 total` | Submission count | API `total=7` | OK | Dynamic. |
| `#APP-9F82AF`, `5/5` | Demo Apartment submission | API first item `id=9f82af...`, `submission_step=5` | OK | Dynamic. |
| `5/5`, `2/5` | Submission step chips | API `submission_step` values | OK | Dynamic. |

### `/developer/operations`

Status: OK for dashboard counts; deep edit/submit flows have validation and route risks.

APIs checked: `/api/developer/operations/dashboard?year=2026`, `/api/developer/villas/:asset_id/asset-config`, operation/annual/capex/forecast GET endpoints for the first asset.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `1`, `0`, `0`, `0`, `0` | Missing/draft/review/published/docs stats | API one asset, zero periods/documents | OK | Dynamic client calculation from operations API. |
| `due Jun 28`, `43 days left` | Required submission banner | Listed month May 2026, current date May 16 2026, deadline 28th of next month | OK | Client date calculation. |
| `2026`, `May 2026`, `0 / 1` | Matrix/year/listed period | API `year=2026`, listed month `5`, one expected current-period submission | OK | Dynamic. |
| `500 bps`, `0`, `0`, `IDR`, `USD` | Asset config API | `/asset-config` | OK | API checked; not all values are visible in dashboard. |

### Deep Developer Annual / Operations Flows

Status: Dry-run checked without mutating production. Read-only API values are OK; submit/edit paths need validation and route fixes.

Live context checked:

| Item | Value | Status | Notes |
| --- | --- | --- | --- |
| Operations dashboard year | `2026` | OK | `/api/developer/operations/dashboard?year=2026`. |
| Developer operations assets | `1` | OK | Only `Demo Apartment 01 - Investment` appears in the operations dashboard. |
| Asset ID | `9f82af5d-449a-4c9c-8a2c-b210d6b1016c` | OK | Used for deep-flow checks. |
| Listed period | `May 2026` | OK | `listed_year=2026`, `listed_month=5`. |
| Annual document year | `2025` | OK | Dashboard API returns `annual_doc_year=2025`, `annual_doc_uploaded=false`. |
| Existing operation rows | `0` | OK | `/api/developer/villas/:asset_id/operations?year=2026` and `?month=5` returned empty arrays. |
| Asset operations config | `reserve_pct_bps=500`, `platform_pct=0`, `withholding_tax_bps=0`, `mgmt_fee_bps=0`, `IDR`, `USD` | OK | `/api/developer/villas/:asset_id/asset-config`. |

Monthly Operations page checked: `/developer/villas/9f82af5d-449a-4c9c-8a2c-b210d6b1016c/operations/new?year=2026&month=5`.

| Number/constraint | Context | Status | Notes |
| --- | --- | --- | --- |
| `0 - 31` | Nights available/booked placeholders for May 2026 | OK client-side | Frontend clamps values above days-in-month. Backend does not independently validate nights range in checked code. |
| `0.0%` | Empty occupancy preview | OK current render | Preview starts at zero because all inputs are empty. |
| `500 bps` reserve | Computed preview source | OK | From asset config API. |
| `Save draft first` | Documents panel lock | OK | Period docs remain locked until a draft `log_id` exists. |
| `20 MB` | Period document upload max | OK | UI says max 20 MB; backend uses `MAX_PERIOD_DOC_BYTES = 20 * 1024 * 1024`. |
| All operation amount/night fields | Form validation | Mismatch/risk | Inputs are not required. Backend create/update allows zero values; submit path saves draft first, then submits. This can create a submitted zero-operations month. |

Annual pages checked: `/developer/villas/9f82af5d-449a-4c9c-8a2c-b210d6b1016c/annual/2026` and `/annual/2025`.

| Number/constraint | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `2026`, `0`, `0`, `USD 0.00`, `0 IDR` | Annual 2026 rollup | `/annual/2026/summary` returned `months_published=0`, all sums `0`, `approved_capex_count=0` | OK | Dynamic empty state. |
| `2025`, `0`, `0`, `USD 0.00`, `0 IDR` | Annual 2025 rollup | `/annual/2025/summary` returned `months_published=0`, all sums `0`, `approved_capex_count=0` | OK | Dynamic empty state. |
| `0` | CapEx events | `/capex?year=2026` and `/capex?year=2025` returned empty arrays | OK | Dynamic empty state. |
| `0` | Forecast suggestions | `/forecast/2026/suggestions` and `/forecast/2025/suggestions` returned empty arrays | OK | Dynamic empty state. |
| `0` | Annual documents | `/annual/2026/documents` and `/annual/2025/documents` returned empty arrays | OK | Dynamic empty state. |
| `Amount (IDR cents)` min `1`, required date/amount/description | CapEx form | UI and backend validate amount/description | OK | Backend rejects `amount_idr_cents <= 0` and empty description. |
| `10000 = 100%`, min/max `0-10000` on occupancy | Forecast form | UI only for occupancy max | Needs backend validation | Backend validates only year; all fields may be `null`. |
| `20 MB` | Annual tax/report upload | UI and backend | OK | Backend rejects files over 20 MB and unsupported document MIME types. |

Route check:

| Route | Result | Status | Notes |
| --- | --- | --- | --- |
| `/developer/villas/:asset_id/operations/:log_id` | 404 on direct GET | Mismatch/risk | Dashboard JS builds this URL for existing periods, but backend does not register a GET page route. Current asset has no logs, so no live user can hit it from this account yet. |

### `/developer/asset-detail?id=...`

Status: OK for checked asset data, with the same low-percentage rounding caveat.

APIs checked: `/api/developer/assets/9f82af5d-449a-4c9c-8a2c-b210d6b1016c`, `/api/developer/assets/3c8f13ff-f227-44e7-a58c-bdb8684ca9fe`.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `$500,000.00`, `$1.00`, `10.0%`, `2,000 of 500,000`, `498,000` | Demo Apartment detail | API `total_value_cents=50000000`, `token_price_cents=100`, `annual_yield_bps=1000`, `tokens_total=500000`, `tokens_available=498000` | OK | Dynamic. |
| `0%` | Demo Apartment progress | `2000 / 500000 = 0.4%` | Needs consistency review | Rounded to zero; `/cart` uses minimum nonzero display. |
| `$1,000,000.00`, `$500.00`, `10.0%`, `50 of 2,000`, `1,950` | Demo Villa detail | API `total_value_cents=100000000`, `token_price_cents=50000`, `annual_yield_bps=1000`, `tokens_total=2000`, `tokens_available=1950` | OK | Dynamic. |
| `13`, `5`, `5`, `0` | Villa docs/images/milestones/financial records | API documents `13`, images `5`, milestones `5`, financials `0` | OK | Dynamic. |
| `14`, `5`, `0`, `0` | Apartment docs/images/milestones/financial records | API documents `14`, images `5`, milestones `0`, financials `0` | OK | Dynamic. |

### `/developer/affiliate-team` and Subpages

Status: Base page OK for empty team; live route/API gaps remain.

APIs checked: `/api/developer/affiliate/team`, `/members`, `/customers`, `/products`, `/summary`, `/by-member`, analytics endpoints.

Checked visible numbers:

| Number | Context | API/source value | Status | Notes |
| --- | --- | --- | --- | --- |
| `0`, `$0`, `$0`, `$0`, `$0` | Base affiliate-team KPI cards | `active_members=0`, empty members/customers/products/summary APIs | OK | Dynamic empty team defaults. |
| `0` | Members/customers/products | API rows arrays length `0` | OK | Dynamic. |
| `/developer/affiliate-team/members`, `/developer/affiliate-team/analytics` | Live pages | 404 | Not checked | Base page is used as current members page; analytics page is not live. |
| `/api/developer/affiliate/team/analytics/overview`, `/timeseries` | Live APIs | 404 | Not checked | JS references these endpoints, but live route is missing. |

### Developer Form/Utility Pages

Status: Mostly hardcoded UX constraints, not live metrics.

Checked pages: `/developer/add-asset`, `/developer/application-form`, `/developer/onboarding`, `/developer/property-content`, `/developer/document-upload-step3`.

Checked visible numbers:

| Number | Context | Status | Notes |
| --- | --- | --- | --- |
| `$1 per share` | Application form minimum | Hardcoded policy/validation copy | Should be centralized if product rules change. |
| `STEP 1 OF 3`, `1 property`, `2-5`, `6-10`, `10+` | Developer onboarding wizard | Hardcoded UX flow/options | Not account metrics. |
| `1 photo`, `8-16 recommended`, `800x400px` | Property content upload guidance | Hardcoded constraint/copy | Needs backend enforcement/source-of-truth check if used for validation. |
| `20 MB`, document steps `1-6`, `25 years` | Document upload requirements | Hardcoded constraint/copy | Not dynamic. |

### `/developer/ranking`, `/developer/settings`, `/developer/support`

Status: Same as the corresponding investor pages.

Notes:

- `/developer/ranking` embeds the same leaderboard values as `/leaderboard`; checked standing `#2`, holdings `€959.0K`, `2 assets`, `10.0%`, `0 affiliates`, and table values matched leaderboard APIs. The `100% Direct` / `5% Referral` legend remains hardcoded/unknown.
- `/developer/settings` uses the same settings data as `/settings`; dynamic account values and hardcoded profile constraints matched the investor settings observations.
- `/developer/support` uses the same support surface as `/support`; ticket counts `All 2`, `Open 0`, `Resolved 2` matched `/api/support/tickets`.

## Full Property / Commodity Detail Follow-up

Status: Mixed. Core property values generally render from backend/template data, but calculators, timelines, and some narrative facts are static/formula-driven and not independently tied to a live financial source in this pass.

Checked property detail pages:

| Page | Key visible numbers checked | Status | Notes |
| --- | --- | --- | --- |
| `/property/demo-apartment-01---investment-6016e69e-15d0-4547-996a-ca560541f3fa` | `5 photos`, `2`, `1`, `72 m²`, `10.00%` rental/gross yield, `9.50%` net yield, `USD 4,166`/m², `0% funded`, `USD 1,102 in 5 years`, `$2,000` default investment | Mixed | Asset numbers are backend-rendered; calculator/timeline values are static/formula. |
| `/property/demo-villa-investment-01-4a0273be-671d-4c7d-8868-df536443f6d5` | `5 photos`, `2`, `2`, `600 m²`, `Completed - May 3, 2026`, `USD 111,051 in 5 years`, `$100,000`, `$61,051`, `$50,000` | Mismatch | Metadata conflicts with copy saying `3-bedroom`, `300-square-meter land`, `220 square meters`, and `three and a half bathrooms`. |
| `/property/modern-surf-villa-canggu` | `10 photos`, `4`, `4`, `380 m²`, `12.00%` rental/gross yield, `19.20%` net yield, `USD 1,916`/m², `76% funded`, `60-month`, `USD 121,051 in 5 years` | Needs review | Net yield exceeds gross yield; calculator repeats the shared `$100,000`/`$61,051` model. |
| `/property/boutique-resort-ubud` | `12 photos`, `12`, `14`, `1200 m²`, `14.00%` rental/gross yield, `17.90%` net yield, `USD 1,140`/m², `64% funded`, `48-month`, `USD 131,051 in 5 years` | Needs review | Net yield exceeds gross yield; calculator appears formula/static. |
| `/property/renovation-flip-canggu` | `6 photos`, `3`, `3`, `220 m²`, `0.00%` rental/gross yield, `25.00%` net yield, `USD 1,125`/m², `45% funded`, `18-month`, `USD 66,051 in 5 years` | Mismatch | `25.00%` net yield on `0.00%` gross yield is not credible without a separate explanation/source. |
| `/property/luxury-clifftop-villa-uluwatu` | `4`, `5`, `450 m²`, `10.50%` rental/gross yield, `16.92%` net yield, `USD 1,667`/m², `50%`, `89%`, Jan/Feb/Mar/Apr/Jun 2026 milestones, `36-month`, `USD 113,551 in 5 years` | Needs review | Net yield exceeds gross yield; milestone/timeline values are page-rendered but not cross-checked against an independent API. |
| `/property/vacation-rental-villa-uluwatu` | `8 photos`, `3`, `3`, `280 m²`, `13.00%` rental/gross yield, `18.05%` net yield, `USD 1,570`/m², `92% funded`, `36-month`, `USD 126,051 in 5 years` | Needs review | Net yield exceeds gross yield; calculator appears formula/static. |
| `/property/new-development-seminyak` | `10 photos`, `8`, `8`, `1200.00 m²`, `8.50%` rental/gross yield, `19.23%` net yield, `USD 1,500`/m², `30% funded`, `24-month`, `USD 103,551 in 5 years` | Needs review | Net yield exceeds gross yield; calculator appears formula/static. |

Checked commodity detail pages:

| Page | Key visible numbers checked | Status | Notes |
| --- | --- | --- | --- |
| `/commodity/organic-cacao-bali-2026` | `$80`, `15.00% fixed`, `12 months`, `450 Trees`, `$80,000`, `15.00% ROI`, `$200,000-$300,000`, `$100,000`, investor payout `$60,000`, `55%`, `10%`, `85%`, `15%`, ERC-1155 | Dynamic but unverified | Values are rendered consistently for cacao; no independent commodity API was found in this pass. |
| `/commodity/premium-bali-rice-q2-2026` | `$50`, `12.00% fixed`, `6 months`, `450 Trees`, `$50,000`, `12.00% ROI`, `$150,000-$220,000`, `$80,000`, investor payout `$40,000`, `60%`, `10%`, `86%`, `14%`, ERC-1155 | Mismatch | `450 Trees` appears copied from cacao and is wrong for a rice investment. |

## Trading Detail Follow-up

Status: V3 asset identity/prices mostly match `/api/marketplace/secondary/assets`; V3 performance history and net return are formula/static. V2 is a static mock.

Checked V3 trading detail pages:

| Asset | API price/value/ROI | Visible price/value/yield | Mismatches |
| --- | --- | --- | --- |
| `grand-pavilion-ubud-estate` | `$500`, `USD 12,000,000`, `8%` | `$500.00`, `USD 12,000,000`, `8%`, `NET RETURN 6.0%`, `24,000 / 24,000` | Net return is `roi * 0.75`; fixed `+4.2%/+8.7%/+14.9%`. |
| `boutique-resort-ubud` | `$285`, `USD 2,850,000`, `14%` | `$285.00`, `USD 2,850,000`, `14%`, `NET RETURN 10.5%`, `10,000 / 10,000` | Same formula/static performance. |
| `new-development-seminyak` | `$180`, `USD 1,800,000`, `8.5%` | `$180.00`, `USD 1,800,000`, `8.5%`, `NET RETURN 6.4%`, `10,000 / 10,000` | Same formula/static performance. |
| `luxury-pool-villa-canggu-funded` | `$95`, `USD 950,000`, `11%` | `$95.00`, `USD 950,000`, `11%`, `NET RETURN 8.3%`, `10,000 / 10,000` | Same formula/static performance. |
| `demo-apartment-01---investment-...` | `$1`, `USD 500,000`, `10%` | `$1.00`, `USD 500,000`, `10%`, `NET RETURN 7.5%`, `500,000 / 500,000` | Same formula/static performance. |
| `modern-surf-villa-canggu` | `$115`, `USD 1,150,000`, `12%` | `$115.00`, `USD 1,150,000`, `12%`, `NET RETURN 9.0%`, `10,000 / 10,000` | Same formula/static performance. |
| `beachfront-retreat-sanur-exited` | `$65`, `USD 650,000`, `0%` | `$65.00`, `USD 650,000`, `0%`, `NET RETURN 0.0%`, `10,000 / 10,000` | Still shows positive fixed 3/6/12 month performance on an exited/0% ROI asset. |
| `vacation-rental-villa-uluwatu` | `$78.50`, `USD 785,000`, `13%` | `$78.50`, `USD 785,000`, `13%`, `NET RETURN 9.8%`, `10,000 / 10,000` | Same formula/static performance. |
| `renovation-flip-canggu` | `$45`, `USD 450,000`, `0%` | `$45.00`, `USD 450,000`, `0%`, `NET RETURN 0.0%`, `10,000 / 10,000` | Still shows positive fixed 3/6/12 month performance on a 0% ROI asset. |
| `luxury-clifftop-villa-uluwatu` | `$133.40`, `USD 1,334,000`, `10.5%` | `$133.40`, `USD 1,334,000`, `10.5%`, `NET RETURN 7.9%`, `10,000 / 10,000` | Same formula/static performance. |
| `villa-pillada-horadada-...` | `$830`, `USD 1,000,000`, `10%`, `sellOrders=1086`, `totalSupply=2000` | `$830.00`, `USD 1,000,000`, `10%`, `NET RETURN 7.5%`, `1,006 / 2,000` | Available `1,006` does not equal `2,000 - 1,086 = 914`; needs orderbook/hold source check. |
| `demo-villa-investment-01-...` | `$500`, `USD 1,000,000`, `10%` | `$500.00`, `USD 1,000,000`, `10%`, `NET RETURN 7.5%`, `2,000 / 2,000` | Same formula/static performance. |

Checked V2 trading detail:

| Page | Status | Notes |
| --- | --- | --- |
| `/marketplace-trading-v2` and `/marketplace-trading-v2?asset=grand-pavilion-ubud-estate` | Hardcoded/mock | Both routes showed the same `Bali Villa Canggu #12`, `$105.00`, `12.4%`, `87%`, `1,000`, fake investor rows, `5%` fee, `$5.25`, `$110.25`, and `USD 350,000`; the `asset` parameter is ignored. |

## Community Profile / Edit Follow-up

Status: Profile counts are dynamic for checked profile APIs; hashtag and badge detail pages have SSR crashes. Profile edit has client constraints and mutating save endpoints that were not executed.

| Page/API | Checked values | Status | Notes |
| --- | --- | --- | --- |
| `/community/me` | `45,495 XP`, `Level 10`, `0 followers`, `0 following`, `0 posts`, `1 streak` | Mostly OK | Visible profile count values align with community surfaces; `/api/community/profile/me` confirms `0/0/0` counts and `2` badges but does not expose XP/level/streak. |
| `/community/u/50e5ca84-fe1a-44a3-a70b-9f48ad019219` | Same Martin profile values as `/community/me` | OK | Public profile resolves to the same user and counts. |
| `/community/u/6e4c37a6-64a0-429a-bfe5-9a712c682011` | `68,760 XP`, `Level 10`, `0 followers`, `0 following`, `0 posts`, `1 streak` | OK for checked profile counts | API confirms `0/0/0` counts and `3` badges; XP/level/streak came from rendered profile/community XP surfaces. |
| `/community/hashtag/hashtags` | API has `1` post, `0` comments, `0` reactions | Page crash | SSR page returns `Internal Server Error: could not render include...`; API is dynamic and returns data. |
| `/community/badge/2f90c212-bb54-45a8-818e-abaec5e5e0a9` | API badge `display_order=2`, `holder_count=5`, `recent_holders.length=5` | Page crash | SSR page returns `Internal Server Error: unknown filter: filter truncate is unknown`. |
| `/community/me/edit` | Bio `6 / 160`, flair `0 / 24`, photo max `5 MB`, verify note max `2000`, asset options `2` | OK constraints, no mutation | Save uses `PUT /api/community/profile`; photo/proof upload uses `POST /api/upload/avatar` and `/api/upload/post-image`; verified owner uses `POST /api/community/verified-owner-requests`. |

## Mutating Workflow Dry-run Checks

No live mutation was executed. These are UI/API/codepath checks only.

| Workflow | Dry-run result | Status | Notes |
| --- | --- | --- | --- |
| Checkout confirmation | Visible values still match `/api/cart`: `USD 4,000.00`, `IDR 62,000,000`, fee `USD 200.00`, total `USD 4,200.00`. `Confirm Payment` is disabled until 3 required risk/KFS checkboxes are checked; proof upload is optional and says max `5MB`. | OK with existing FX/rounding caveats | No button clicked. Bank terms checkbox is present but not HTML-required. |
| Order cancellation | My Trading shows 21 cancel buttons in the open-orders table, including 20 `filled` orders. Backend `DELETE /api/marketplace/orders/:id` only cancels `open`/`partially_filled` and would reject `filled`. | UI mismatch | The UI should hide/disable cancel for non-active statuses and fix the table filter/count. |
| P2P/buy-interest cancellation | `/api/marketplace/p2p/offers/outgoing` returned empty; UI count is `0`. Code only renders cancel for `pending` or `countered` offers. | OK in current account | No outgoing offers to cancel. |
| Support ticket submission | UI constraints match backend: subject min `5`, max `255`; message min `20`, max `5000`; attachment max `5MB`; priorities `low/normal/high/urgent`; categories match backend. `/api/support/tickets` returned `2` closed tickets; UI tabs show `All 2`, `Open 0`, `Resolved 2`. | OK | Submit/reopen/reply not executed. |
| Developer submissions | `/api/developer/drafts` returned `total=7`; UI submissions section renders dynamic drafts/statuses. Draft submit path is `PUT /api/developer/draft/:id` then `POST /api/developer/draft/:id/submit`; delete is allowed only for `draft` and blocked for active investors. | OK dry-run | No submit/delete/duplicate executed. |
| Developer asset edits | Asset edit save path is `PUT /api/developer/assets/:id`. Draft/submitted assets apply directly; approved/live assets create admin review change requests. | OK dry-run | No asset edit executed. |
| Developer monthly operations | `/operations/new?year=2026&month=5` loads and computes zero preview from config, but Save/Submit can create and submit an all-zero row because inputs are optional and backend validates only the period. | Validation risk | No draft/save/submit executed. |
| Developer annual data | `/annual/2026` and `/annual/2025` load dynamic zero summaries; CapEx has required amount/date/description, but forecast suggestions can submit all-null payloads. | Partial validation risk | No CapEx, forecast, or document upload executed. |
| Community profile edits | `/community/me/edit` loads current profile and uses `PUT /api/community/profile`; privacy toggles auto-save on change. Bio/flair counters are client constraints; backend truncates flair to 24 chars, but no backend 160-char bio cap was confirmed in the checked code. | Needs backend policy check | No profile save/toggle/upload/request executed. |

## Unchecked Pages / Areas

These routes/pages should get follow-up passes before calling the whole platform complete:

| Area | Examples | Reason not checked |
| --- | --- | --- |
| Admin | `/admin`, `/admin/users`, `/admin/assets`, `/admin/marketplace/*`, `/admin/treasury`, `/admin/deposits`, `/admin/orders`, `/admin/rewards`, `/admin/support` | Large separate admin surface with many financial numbers and privileged workflows. |
| Affiliate legal/policy docs | `/affiliate/terms`, `/affiliate/code-of-conduct`, `/affiliate/marketing-materials`, `/affiliate/qualified-referral-payout`, `/affiliate/tax`, `/affiliate/privacy-notice`, `/affiliate/complaints` | Mostly policy text, but contains thresholds/periods that need a separate legal-copy audit. |
| Actual mutating execution | Checkout confirmation, order/P2P cancel, support ticket submit/reopen/reply, profile save/upload, verified-owner request, developer draft submit/delete/duplicate, asset edit save, monthly operations save/submit, CapEx/forecast submit, document upload | Intentionally not executed on live production; this pass inspected UI, GET APIs, and local code paths only. |
| Auth/legal/blog | login/signup/reset, policy pages, blog pages | Mostly static copy, but legal amounts/terms still need separate review if they contain financial claims. |

## Recommended Fix Order

1. Fix the `/portfolio` source-of-truth conflict. Either make `/api/investors/me/portfolio-villa-summary` use the same current valuation logic as `/api/portfolio`, or remove the duplicate summary cards until both endpoints agree.
2. Replace the hardcoded trading performance formulas in `marketplace-trading-v3.js` with real API fields. Until then, hide `3 months`, `6 months`, `12 months`, and derived `netReturn` values. Batch 1 local remediation completed: optional API fields are read when present; otherwise placeholders are shown instead of fabricated values.
3. Remove or clearly retire `/marketplace-trading-v2`; it is static mock data and ignores the selected asset.
4. Fix `/my-trading` open-orders table: filter the table/count to active statuses and hide/disable cancel for `filled`, `cancelled`, and `expired` orders. Batch 1 local remediation completed.
5. Centralize FX rate use in cart frontend. Use the backend-provided `usd_to_idr_rate` or a shared config endpoint instead of `const IDR_RATE = 15500`. Batch 1 local remediation completed: cart and checkout now consume the backend FX service output.
6. Fix property detail calculators/yield formulas or add explicit backend fields. Investigate pages where net yield exceeds gross yield and the shared `$61,051` value-appreciation figure.
7. Fix commodity detail copy/data for rice showing `450 Trees`.
8. Fix Community hashtag and badge SSR templates; both APIs return data but the pages crash.
9. Remove hardcoded rewards referral fallbacks or make them explicit product-config defaults delivered by API.
10. Fix `/developer/dashboard` KPI rendering so visible text uses the same values as `data-final-value` and `/api/developer/dashboard/stats`.
11. Add hard backend validation for developer monthly operations: reject all-zero submissions, require credible gross/occupancy data before `submitted`, enforce `nights_booked <= nights_available`, and reject negative financial inputs.
12. Register a GET page route for `/developer/villas/:asset_id/operations/:log_id` or change dashboard links to `/operations/new?year=&month=` so existing drafts/submissions do not 404.
13. Validate developer forecast suggestions so at least one forecast field or note is present; add bounds for bps fields server-side.
14. Replace the hardcoded affiliate promo ladder/calculator with backend tier/config data, or mark it explicitly as static legal/product copy.
15. Fix payment result status logic for completed bank-transfer orders and avoid loading unrelated `/api/orders/latest` data when the route has no explicit order/deposit ID.
16. Deploy/register the missing developer affiliate analytics/member routes or remove dead links/references.
17. Create a follow-up audit pass for admin pages and affiliate legal documents.
