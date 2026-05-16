# POOOL Live Site Action Map

Audit date: 2026-05-16  
Basis: rendered local live platform at `http://localhost:8888`, authenticated with a local test session for protected pages. Public production entrypoints `https://www.poool.app/` and `https://platform.poool.app/` were spot-checked for reachability. The in-app browser blocked direct localhost navigation, so this inventory is based on server-rendered HTML, route output, forms, links, buttons, inputs, and page scripts.

## Shared Navigation

Most authenticated investor/developer pages expose the same global actions:

- Switch between investor areas: Properties, Commodities, Wallet, Portfolio, Rewards, Affiliate, Cart, Leaderboard, Community, Settings, Support.
- Switch to developer areas when the user has the role: Dashboard, My Assets, Submissions, Operations, Ranking, Settings, Support, Add Asset.
- Use global search from sidebars/topbars where present.
- Open notification and cart controls on mobile.
- Open/close the mobile menu.
- Use profile/session controls through topbar/sidebar UI.

## Public Pages

| Page | What the user can do |
| --- | --- |
| `/` Landing page | Open section anchors (`Why us`, `How it works`, `Market`, `FAQ`), switch language (`ID`, `EN`), open mobile menu, sign in, sign up/start investing, browse property cards, move through property card image carousels, open public property details, open blog, open marketplace, open WhatsApp contact, open legal footer links. |
| `/id/` Indonesian landing | View Indonesian marketing page. The rendered static build did not expose parseable anchor/button actions in the server HTML; expected actions mirror the landing SPA language/navigation controls. |
| `/p/sunset-luxury-villa` Public property | Open gallery images/lightbox, view all photos, use investment calculator sliders, switch financial tabs (`Property cost`, `Rental income`, `Live performance`), enter investment amount, quick-add preset amounts, follow sign-up CTA with `returnTo`, use mobile amount controls, open legal/footer links. Document download is shown but disabled. |
| `/p/boutique-resort-ubud` Public property | Same public-property actions as above, plus developer website/social links. |
| `/blog` Blog index | Navigate blog header, open category filters, open article cards, sign in, open social/footer links, open mobile blog menu. |
| `/blog/:slug` Blog article | Read article, navigate back to blog/categories, open sign-in CTA, use footer/social links. |
| `/blog/category/:slug` Blog category | Filter article list by category, open article cards, sign in, use blog navigation. |
| `/terms` | Read Terms and Conditions. Main page action is navigation back through shared shell/sidebar if authenticated. |
| `/privacy-policy` | Read Privacy Policy; use any embedded contact/legal links. |
| `/currency-policy` | Read Currency Policy. |
| `/cookies` | Read Cookie Policy; use embedded privacy/contact links if present. |
| `/imprint` | Currently renders the 404 page locally. Actions: go to marketplace, go back, contact support, sign in. |
| `/gdpr-data-request` | Currently renders the 404 page locally. Actions: go to marketplace, go back, contact support, sign in. |

## Authentication And Account Entry

| Page | What the user can do |
| --- | --- |
| `/auth/login` | Enter email and password, toggle password visibility, enable `Remember me`, submit login, log in with Google, open forgot-password flow, open signup, go back to public site, move testimonial carousel. If already authenticated, this redirects to `/marketplace`. |
| `/auth/signup` | Enter email, enter password, accept terms/privacy checkbox, submit account creation, sign up with Google, open Terms & Conditions, open Privacy Policy, switch to login, go back to landing. |
| `/auth/forgot-password` | Enter email, submit password-reset request, return to login. |
| `/auth/reset-password` | Enter new password, confirm password, submit reset token form, return to login. |
| `/auth/verify-email` | Resend verification email, return to login. |
| `/auth/2fa` | Enter six-digit authenticator code, submit verification, log out. |
| `/auth/2fa/setup` | Currently returned `500` in the local authenticated audit. Intended route: enroll authenticator setup. |
| `/auth/2fa/step-up` | Redirected into `/auth/2fa/setup`, which currently returned `500` locally. Intended route: fresh 2FA confirmation before sensitive action. |
| `/logout` | Confirm sign-out transition and return to login/home flow. |
| `/welcome` | Continue to marketplace or start KYC verification. |

## Investor Pages

This section is intentionally route-level and action-oriented. It includes the primary investor app, community subpages, trading surfaces, rewards/affiliate surfaces reachable from the investor shell, and deeper dynamic pages.

| Page / route | Screen or state reviewed | User controls and inputs | User actions | Result / next step |
| --- | --- | --- | --- | --- |
| `/marketplace` | Authenticated real-estate marketplace list. | Available/Funded tabs, location select, investment select, property-type select, search field, clear/search buttons, property-card carousel buttons. | Filter listings, search by term/location/type, clear filters, switch funding state, move card images, open a card/title. | Listing grid updates; property cards navigate to `/property/:slug`. |
| `/marketplace/tab` | HTMX-backed marketplace tab endpoint. | Same filters as the marketplace list, used by tab switches. | Request filtered available/funded content without a full page reload. | Replaces the marketplace card grid. Not a standalone user destination. |
| `/commodities-marketplace` | Authenticated commodities marketplace list. | Available/Funded tabs, location filter, term filter, commodity-type filter, search, clear/search, More filters where visible. | Filter commodity offerings, search, switch funding state, open commodity card. | Listing grid updates; commodity cards navigate to `/commodity/:slug`. |
| `/commodities-marketplace/tab` | HTMX-backed commodities tab endpoint. | Same commodity filters as the list page. | Refresh commodity results for a selected tab/filter combination. | Replaces commodity marketplace content. Not a standalone user destination. |
| `/property/:slug` | Authenticated property detail, e.g. `/property/boutique-resort-ubud`. | Gallery thumbnails, lightbox controls, calculator sliders and editable values, financial tabs, USD/IDR live-performance toggle, as-of date/latest control, amount input, quick-add buttons, Add to cart, developer website/social links, document/download buttons. | Inspect photos, open/close lightbox, calculate projected return, switch cost/rental/live-performance panels, change currency/date, enter an investment amount, use presets, add investment to cart, open external developer references. | Cart line is created or validation is shown; user continues to `/cart` or reviews more detail. |
| `/p/:slug` | Public property detail used before login/signup. | Same visual detail controls as authenticated detail, plus signup/login return-path CTAs. | Inspect gallery/calculator/documents, enter amount, start signup/login from the property page. | User is sent to auth with `returnTo` so they can come back to the property after login/signup. |
| `/commodity/:slug` | Authenticated commodity detail, e.g. `/commodity/organic-cacao-bali-2026`. | Media/gallery, video play modal where available, amount input, Add to cart, financial/operator/security/roadmap/document sections, contact/legal links. | Review commodity economics, inspect operator/security details, open media, enter amount, add commodity allocation to cart. | Cart line is created or validation is shown; user continues to cart/checkout. |
| `/cart` | Empty cart and populated cart states. | Empty-state links, line-item amount controls, remove forms/buttons, add-more card, Proceed to checkout button, mobile checkout button. | Browse properties/commodities, start bundling, view portfolio, adjust investment amounts, remove items, proceed to checkout. | Empty cart routes user back to discovery; populated cart routes to `/checkout` after client/server validation. |
| `/cart/add`, `/cart/update`, `/cart/remove` | Form/API targets triggered from detail/cart pages. | Hidden asset/amount fields, amount/quantity values, remove item ID. | Add item, update amount, remove item. | Redirects back to cart or returns JSON/error state depending caller. |
| `/checkout` | Checkout with non-empty cart; redirected to `/cart` when empty in the local pass. | Required disclosure checkboxes, payment method tile, USD/IDR bank currency buttons, proof-of-transfer upload, bank reference copy buttons, Confirm Payment, summary/timer panel. | Review items/fees/tax, accept disclosures, select bank-transfer currency, copy IBAN/BIC/account/reference, upload proof, confirm payment. | Submits checkout via JS; success routes to `/payment-in-progress` or `/payment-success`, errors stay inline. |
| `/payment-in-progress` | Pending bank transfer/order status. | Status page CTAs and shared navigation. | Review pending payment status, continue browsing, navigate to wallet/portfolio/marketplace. | User waits for admin/payment confirmation and can monitor via transactions/portfolio. |
| `/payment-success` | Successful payment confirmation. | Success CTAs and shared navigation. | Continue to portfolio/marketplace/wallet, inspect order/payment result. | Purchase flow closes; holding should appear in portfolio after settlement/state update. |
| `/wallet` | Wallet overview with loading/error/empty/content states. | Retry, Deposit, Withdraw, Make a deposit, View payment methods, Start verification, Add method, Add card, Add bank, transaction detail links, See all transactions. Deposit form amount. Withdrawal payment-method select. Card holder/card/expiry/CVV fields. Bank account holder/country/bank/label fields. | Retry data load, start KYC, add payment methods, enter deposit amount, confirm deposit, select withdrawal method, submit withdrawal, open transaction details. | Creates deposit/withdrawal requests or payment methods; errors include KYC, safety, insufficient funds, or failed request states. |
| `/wallet/deposit`, `/wallet/withdraw` | Server targets triggered by wallet modals/forms. | Amount, selected payment method, CSRF/session context. | Submit deposit or withdrawal request. | Redirects to `/wallet` with success/error query state. |
| `/portfolio`, `/portfolio.html` | Investor holdings and asset performance. | Holdings rows/cards, chart/data views, asset detail links, NFT-to-wallet buttons where eligible, cancellation controls for cancellable investments, empty-state marketplace/KYC links. | Inspect total value and holdings, open property/asset details, add NFT token to MetaMask, cancel eligible investment, navigate to marketplace/KYC. | User reaches asset detail/trading/marketplace, wallet integration prompt opens, or cancellation request is sent. |
| `/transactions` | Transaction list. | Type/status filters, date-from/date-to inputs, clear filters, row links, pagination. | Filter history, clear filters, open a transaction. | List updates; row opens `/transactions/:id`. |
| `/transactions/:id` | Transaction detail. | Back/navigation links and document/detail actions where data exists. | Inspect single transaction metadata, status, amount, related order/deposit/withdrawal context. | User can return to list or follow related wallet/order navigation. |
| `/tax-report` | Investor tax report/export page. | Report period controls and export/download actions where available. | Generate or download tax/reporting data for holdings and transactions. | Produces a tax/report artifact or shows unavailable/empty state. |
| `/leaderboard`, `/leaderboard.html` | Investor leaderboard. | Retry, Explore Marketplace, metric/timeframe/tier/search/per-page controls where enabled, pagination, admin-only refresh, profile preference controls such as visibility/display options. | Review rank, change page size, paginate rankings, start investing from empty state, update visibility/display preferences, refresh if admin. | Ranking table changes; preferences persist to leaderboard settings endpoint. |
| `/community` | Community shell with feed, search, notifications, circles, challenges, AMAs, and DMs. | Topbar search, tab buttons, search form, type filters, date/min-engagement filters, notification list, preferences link, ban appeal button, moderation history, composer, reactions, comments, bookmarks, report menu, DM compose controls. | Switch tabs, search users/posts/hashtags, filter search results, follow/unfollow users, open posts/profiles, submit ban appeal, view moderation history, create/react/comment/bookmark/report posts, mark notifications read, start or continue DMs. | HTMX/API content loads into tabs; deep links open post/profile/hashtag routes; moderation/notification/DM actions persist through community APIs. |
| `/community/post/:id` | Single community post detail. | Post body controls, reaction/bookmark/report buttons, comment form/list, comment edit/delete/reaction controls for eligible users. | Read focused post, react, bookmark, report, comment, edit/delete own comments, open author/profile/hashtags/assets. | Updates post interaction counts and comment thread; links navigate to profile/hashtag/asset pages. |
| `/community/hashtag/:tag` | Hashtag feed page. | Feed controls and post cards scoped to a tag. | Browse posts for one hashtag, open posts/authors/assets, react/comment through feed controls. | User stays in filtered hashtag context or navigates into individual posts/profiles. |
| `/community/me` | Own community profile. | Profile tabs, edit profile link, badge/post/activity/media sections, follower/following lists. | Inspect public profile as others see it, review own activity, open edit form. | Navigates to `/community/me/edit` or profile-related lists/items. |
| `/community/me/edit` | Community profile editor. | Display-name/bio/location/social/profile fields, avatar/banner upload controls, notification/privacy/community identity fields where present, save/cancel. | Edit public community identity, upload profile assets, save profile. | Updates profile via `/api/community/profile`; returns to own profile or shows validation errors. |
| `/community/u/:user_id` | Other user profile. | Follow/unfollow, message/block/mute/report actions where available, profile tabs. | View another member, follow/unfollow, start DM, block/mute, inspect posts/comments/media/activity. | Relationship/moderation state changes or user navigates to posts/DMs. |
| `/community/badge/:id` | Badge detail. | Badge metadata and related member/list links. | Inspect badge requirement/award context, navigate back to profile/community. | Read-only badge detail except shared navigation. |
| `/settings/notifications/community` | Community notification preferences. | Notification preference toggles/checkboxes for community events, save/reset controls. | Configure what community notifications the user receives. | Preferences are saved to the community notification preferences API. |
| `/settings`, `/settings-2`, `/settings-3` | Investor account settings. | Settings search, profile-completion banner dismiss, section nav, avatar upload, profile/address/identity/security/Web3/preferences/leaderboard/social/developer/data privacy forms, save buttons. | Search/jump to settings section, update core profile, address, identity fields, password/security choices, Web3 wallet binding, leaderboard visibility/display name, social links, developer identity links, export data, start deletion. | Account settings persist through settings APIs; destructive flow navigates to account deletion. |
| `/account-deletion` | Account deletion request. | Warning/confirmation fields, password/confirmation input where required, cancel/delete actions. | Review consequences, cancel, or submit deletion request. | Account deletion workflow begins or user returns to settings. |
| `/support` | Investor support center. | FAQ search, ticket tabs/filters, subject/category/priority/message fields, attachment input, submit ticket, reply forms, attach screenshot/file, reopen, CSAT rating. | Search FAQ, create support ticket, attach files/screenshots, view ticket thread, reply, reopen closed ticket, rate resolution. | Ticket list/thread updates through support APIs; support team is notified. |
| `/rewards` | Referral and rewards dashboard. | Referral link/code copy/share, stats cards, commissions table/date filters, payout settings form, tier/campaign sections, marketing/material links. | Copy/share invite link, inspect rewards/tiers/commissions, filter commission history, save payout settings, open affiliate onboarding/dashboard. | Referral link can be distributed; payout/tax settings persist; tier section can be opened via `/tier`. |
| `/rewards-v2` | Alternate rewards overview. | Reward module navigation, CTA links, stats/overview controls. | Review premium rewards overview and jump to relevant reward modules. | User navigates to rewards, affiliate, tier, or marketplace surfaces. |
| `/rewards/:code`, `/r/:code` | Referral entry route. | Route is driven by referral code in URL. | Visitor lands through a referral link. | Referral cookie is set and user is redirected toward signup. |
| `/tier` | Tier shortcut. | Redirect target only. | Open tier progress. | Redirects to `/rewards#tier`. |
| `/affiliate` | Investor-accessible affiliate program landing. | Apply/start CTAs, login/signup/dashboard CTAs, legal-policy links. | Read program benefits, start onboarding, open legal pages, log in/sign up. | User enters `/affiliate/onboarding` or existing affiliate dashboard. |
| `/affiliate/onboarding` | Affiliate compliance wizard. | Traffic source, audience size, URL, phone, KYC step, tax ID/company fields, policy modal links, required acknowledgements, quiz answers, back/continue/save/submit. | Complete affiliate profile, start KYC, enter tax data, accept terms/policies, answer compliance quiz, submit application. | Application is stored; user moves to dashboard/rewards depending approval/state. |
| `/affiliate/dashboard` | Affiliate partner dashboard. | Metrics, referral link/copy/share, payout request where eligible, referrals/materials/settings navigation. | Inspect performance, copy/share links, request payout, open referrals/materials/settings. | Partner manages ongoing affiliate activity. |
| `/affiliate/referrals` | Referral/customer/payout list; redirected to onboarding for incomplete state in the local pass. | Filters/export/link controls once onboarded. | Review referred users/conversions/payouts, filter/export records. | Data table updates or export downloads. |
| `/affiliate/materials` | Approved marketing materials. | Material cards, copy/download/open controls. | Copy approved copy, download assets, open policies. | User obtains compliant marketing assets. |
| `/affiliate/settings` | Affiliate payout/tax/postback settings. | Payout method/bank/tax fields, postback URL/settings, save controls, tax-document upload where enabled. | Configure affiliate payouts, tax identity, tracking/postback settings. | Settings persist through affiliate APIs. |
| `/affiliate/terms`, `/affiliate/code-of-conduct`, `/affiliate/marketing-materials`, `/affiliate/qualified-referral-payout`, `/affiliate/tax`, `/affiliate/privacy-notice`, `/affiliate/complaints` | Affiliate legal/policy pages. | In-page legal links and shared navigation. | Read program terms, conduct rules, material policy, payout qualification, tax SOP, privacy notice, complaints procedure. | Read-only compliance reference; user returns to onboarding/dashboard. |
| `/marketplace-secondary` | Secondary-market overview. | Search/filter/listing controls, trading/detail links. | Discover secondary listings, filter/search, open trading surface. | User moves into trading pages. |
| `/marketplace-trading-v2` | Earlier secondary trading interface. | Trading controls, order/list tabs, filters. | Review secondary-market orders and place/manage trading actions where available. | User submits or navigates trading actions. |
| `/marketplace-trading-v3` | Current trading screen. | Asset/orderbook/trade panels, buy/sell mode controls, amount/price inputs, P2P/order tabs, chart/trade history controls. | Inspect orderbook/chart/trades, choose buy/sell, enter order values, submit order, review P2P offers. | Order is placed or validation is shown; success routes to `/trade-success`. |
| `/my-trading` | User's secondary-market activity. | Status tabs/filters, order rows, cancel/manage buttons, history/export controls where present. | Review own open orders/trades, filter activity, cancel eligible open orders, inspect history. | Orders update/cancel through trading APIs; history remains visible. |
| `/trade-success` | Trading success confirmation. | Continue CTAs to trading, marketplace, portfolio. | Confirm placed order and choose next destination. | User returns to portfolio/trading/marketplace. |
| `/welcome` | Post-signup investor entry. | Continue to marketplace, start KYC/verification CTAs. | Choose first post-signup path. | User proceeds to marketplace or verification. |

## Developer Pages

This section includes the standard developer shell, the asset-submission wizard, asset management, operations reporting, annual data, and the developer affiliate-team module.

| Page / route | Screen or state reviewed | User controls and inputs | User actions | Result / next step |
| --- | --- | --- | --- | --- |
| `/developer` | Developer root route. | None on the route itself. | Open developer area. | Redirects to `/developer/dashboard`. |
| `/developer/onboarding` | Three-step developer application wizard. | Personal fields: first name, last name, email, phone, WhatsApp, nationality, residence country, LinkedIn/website. Portfolio controls: asset-count radio cards, property-type check cards, location check cards, estimated asset value, monthly income, bio. Review screen and Submit & Access Dashboard. | Enter developer identity, select portfolio scale/types/locations, add experience text, review application, submit. | Posts application to `/api/developer/apply`; user is sent to `/developer/dashboard` with manual review banner state. |
| `/developer/dashboard` | Developer KPI dashboard. | Review-banner dismiss, KPI cards, sales chart, activity snapshot, top-performing/attention asset rows, Manage assets/View assets links. | Review funding/raised/remaining/asset metrics, dismiss review notice, open asset detail rows, navigate to assets. | Asset rows open `/developer/asset-detail?id=:asset_id`; links open `/developer/assets`. |
| `/developer/dashboard/fragments/chart` and `/developer/dashboard/fragments/assets` | HTMX dashboard fragments. | Fragment endpoints supply chart/assets content. | Trigger dashboard refresh/load portions of dashboard. | Replaces chart or asset components; not standalone user pages. |
| `/developer/assets` | Developer asset management table and preview panel. | Search input, All/Available/Funded tabs, sortable-style asset table, row focus/click, view icon, edit icon, preview View asset/Edit content links, empty-state Add first asset/View submissions. | Search assets, filter by status, select row to update preview, view asset, open edit mode, start first listing. | Opens `/developer/asset-detail?id=:asset_id` or `/developer/asset-detail?id=:asset_id&edit=1`; empty state starts `/developer/add-asset`. |
| `/developer/add-asset` | Step 1 of add-asset wizard. | Asset-type cards: Real Estate selectable; Commercial Property, Commodities, Business, Startups, Land/Plots marked coming soon; Next Step. | Select supported asset type and continue. | Continues to `/developer/application-form`; unsupported cards are informational/disabled. |
| `/developer/application-form` | Step 2 property and financial information. | Required fields: property name, property type, area, address, lease type, lease term, land size, building size, bedrooms, bathrooms, status, year built/renovated, purchase price, minimum share price. Optional/extra fields: city, country. Previous, Save & Exit, Next Step. | Enter core asset facts and pricing, format currency fields, save progress, go back, continue. | Draft is created/updated through developer draft APIs; next step is `/developer/document-upload-step3`. |
| `/developer/document-upload-step3` | Step 3 document upload. | Five document sections with upload/dropzone and delete controls: proof of title, legal basis of title, permits, tax documentation, KYC/corporate structure. Accepted files include PDF/DOC/DOCX/ZIP/JPG/PNG/WebP. Previous, Save & Exit, Next Step. | Upload or drag documents per category, remove uploaded files, save progress, continue. | Draft document set is stored; next step is `/developer/property-content`. |
| `/developer/property-content` | Step 4 listing content, media, and projections. | Asset title, short description, full description, Google Maps link, location description, image gallery/upload/dropzone, YouTube URL, rental yield, capital appreciation, investor profit share, occupancy rate, total expected return, Previous, Save & Exit, Submit & Tokenize. | Write public listing content, upload media, add video, enter return assumptions, save, submit for tokenization/review. | Draft is updated; final submit moves asset into submitted/review state and then `/developer/submission-success`. |
| `/developer/submission-success` | Wizard completion. | Confirmation links/buttons to dashboard/submissions/add another asset and contact channels where shown. | Confirm submission, return to dashboard, view submissions, start another asset, contact support/social channels. | Asset waits for admin review; developer returns to management area. |
| `/developer/submissions` | Draft/submission management. | Status stat cards (all, draft, submitted, in review, revision, approved, rejected), search, sort dropdown, sortable table columns, row selection, select all, bulk delete, per-row actions, pagination, empty-state Add first asset/View assets. | Filter by review state, search, sort, select draft rows, bulk delete drafts, open/edit submissions, duplicate/resubmit/delete eligible items. | Calls draft list/duplicate/delete/submit APIs; selected row opens relevant asset/draft workflow. |
| `/developer/asset-detail` | Asset detail and edit/change-request surface. | Query-driven asset selector `?id=...`, view/edit mode, pending-change panels, metrics, investors, financials, documents, images, milestones, public-content fields where edit is enabled. | Review asset performance and content, open edit mode, make allowed draft/approved edits, inspect pending changes and documents. | Updates draft or reverts approved asset to review-required state via developer draft/change APIs. |
| `/developer/operations` | Operations dashboard matrix. | Loading/error/empty states, urgent submissions banner with dismiss/chips, year tabs, filter tabs (All, Needs action, Missing docs, Rejected), monthly matrix cells, mobile cards, legend, asset/dashboard links. | Review missing/draft/review/published stats, switch year/filter, click month/asset cell, open needed report, dismiss action banner. | Opens `/developer/villas/:asset_id/operations/new?period=YYYY-MM` or existing report/detail depending cell state. |
| `/developer/villas/:asset_id/operations/new` | Monthly operations report. | Gross rental, nights available/booked, expense inputs for cleaning/maintenance/utilities/staff/security/pool/garden/pest/property tax/insurance/accounting/internet/CapEx/other, custom expense rows, notes/hidden computed metrics, live distributable/occupancy summary, Save draft, Submit for approval, document dropzone/queue, doc-type select, remove queued file, Upload documents, Download existing docs. | Enter monthly revenue/occupancy/expenses, add/remove custom expenses, let page recompute ADR/occupancy/opex/net/reserves/platform/withholding/distributable values, save draft, upload evidence, submit for approval. | Draft log is created/updated; document upload requires saved draft; submit locks editable fields and sends report to admin review. |
| `/developer/villas/:asset_id/annual/:year` | Annual data page for a villa/year. | Back link to asset detail, annual summary, CapEx form (event date, amount in IDR cents, category, description, evidence document UUID), Forecast suggestion form (occupancy bps, ADR cents, rent growth bps, expense inflation bps, appreciation bps, exit yield bps, notes), annual document type select, file input, Upload & link, Download existing documents. | Submit CapEx event for admin approval, submit forecast inputs, upload annual tax/report documents, review submitted CapEx/forecast/document lists. | Posts to CapEx/forecast/annual-document APIs; rows show pending/approved status and downloadable documents. |
| `/developer/ranking` | Developer ranking/leaderboard. | Ranking table and leaderboard-style filters/navigation. | Review developer performance rank and compare metrics. | Read-only ranking view plus shared navigation. |
| `/developer/settings` | Developer-context settings. | Shared settings controls plus developer profile/links fields. | Update developer identity, company/profile values, public links, security/profile settings. | Settings persist through the same settings APIs, including developer-specific profile/link endpoints. |
| `/developer/support` | Developer-context support. | Same ticket/FAQ/reply/attachment/CSAT controls as investor support. | Create or manage support tickets from developer context. | Support ticket APIs update ticket state. |
| `/developer/affiliate-team` | Developer affiliate-team overview. | Team shell navigation, overview KPIs, invite modal available from shell. | Review team affiliate performance, open members/customers/products/settings/analytics/tier, invite a team member. | Navigates to subpages or sends invitation. |
| `/developer/affiliate-team/members` | Team members list. | Export CSV, Invite Member, shared invite modal, members table, approve/remove row actions. Invite modal email field, Cancel, Send Invitation, token preview. | Export members, invite a registered user by email, approve pending member, remove active/pending member. | Calls affiliate team member APIs; removed members lose future business-link attribution; invitation preview shows token/link. |
| `/developer/affiliate-team/customers` | Customers acquired by team links. | Member filter select, Export CSV, customers table. | Filter customers by active member, export customer attribution data. | Table reloads with selected member; CSV downloads current view. |
| `/developer/affiliate-team/products` | Product/asset sales via team links. | Export CSV, product sales table. | Inspect revenue by sold asset/product and export product-sales data. | CSV downloads current product-sales view. |
| `/developer/affiliate-team/settings` | Team identity, public join page, payout/banking settings, overview. | Team display name, public slug, public join URL copy/open, bank account holder, IBAN, BIC, bank name, country, Discard changes, Save changes, team status/public status/bank status indicators. | Rename team, configure public slug/join page, copy/open join URL, configure payout banking, discard or save changes. | Settings footer tracks dirty state; save persists affiliate-team settings and payout destination data. |
| `/developer/affiliate-team/analytics` | Affiliate-team analytics dashboard. | Date presets (7d, 30d, this month, last month, YTD, all time, custom), from/to date inputs, day/week/month resolution, KPI tiles, links to products/members/affiliate payout, charts, by-member/by-asset tables, Export CSV/PDF controls. | Change reporting range/resolution, inspect revenue/commission/conversion/qualified/pending/payable/paid/members metrics, open related members/products/payout pages, export data or print PDF. | Analytics APIs reload overview/timeseries/member/product data; exports download CSV or open print dialog for PDF. |
| `/developer/affiliate-team/tier` | Team tier/progression page. | Tier status, progress metrics, tier action/navigation controls. | Review team tier status and progression requirements. | Informational; user can adjust team activity/settings from related subpages. |

## Admin Pages

| Page | What the user can do |
| --- | --- |
| `/admin/` | View admin dashboard cards, global metrics, charts, alerts, quick links, refresh/search/actions. |
| `/admin/users` | Search/filter users, inspect list, open user details, export or apply user actions where available. |
| `/admin/user-details` | Inspect selected user, update/review roles/status/KYC/transactions/support context. |
| `/admin/kyc` | Filter KYC queue, open documents, approve/reject/request updates, review AML status. |
| `/admin/support` | Filter tickets, open ticket detail, assign/status/reply workflows. |
| `/admin/support-ticket` | Load ticket, reply, change status, inspect conversation and attachments. |
| `/admin/developer-submissions` | Filter submission queue, open reviews, bulk/status actions. |
| `/admin/developer-submission-review` | Review submission, upload admin documents/images, edit public page content, add badges/leasing/risk/milestones, save changes, approve/reject/request changes. |
| `/admin/assets` | Search/filter live assets, toggle featured/published states, open asset details, manage funding/status. |
| `/admin/asset-details` | Edit asset metadata/content/images/documents/milestones, manage publication/funding and admin asset actions. |
| `/admin/asset-change-requests` | Filter change requests, assign/unassign, approve/reject, bulk approve/reject. |
| `/admin/asset-tokenize` | Local route redirected to admin dashboard; tokenize actions are available through blockchain/tokenize candidate pages. |
| `/admin/orders` | Inspect orders/investments, filter/search/export, open order detail, approve/reject order flows where available. |
| `/admin/deposits` | Filter deposit requests, confirm/cancel/extend, manage disputes/evidence, refresh/export. |
| `/admin/treasury` | View treasury/financial overview, refresh/export financial data. |
| `/admin/rewards` | Manage referral campaigns/tiers/commissions, search/filter/export, approve payout flows. |
| `/admin/dividends` | Calculate distributions, create/approve/execute/cancel dividend distributions. |
| `/admin/audit-logs` | Search/filter audit events, export CSV, open detail modal. |
| `/admin/reports` | Generate/download reports and exports by report type/date range. |
| `/admin/notifications` | Create/send/manage notification templates and notification center items. |
| `/admin/settings` | Edit platform settings, legal versions, feature flags, operational configs. |
| `/admin/system` | Inspect health checks, worker state, operational toggles, run/retry system actions. |
| `/admin/storage` | Refresh storage analytics, review upload/KYC storage links, navigate to KYC queue. |
| `/admin/admins` | Invite admins, manage admin directory, resend/revoke invitations, adjust admin roles. |
| `/admin/roles` | Edit RBAC matrix, reset/save permissions, create role, clone role, toggle permission checkboxes. |
| `/admin/email-marketing` | Manage email templates/campaigns, create/update/send/test email flows. |
| `/admin/approvals` | Review approval queue, approve/reject items, filter queue. |
| `/admin/blog` | Manage article list, search/filter, create/import/publish/archive/restore blog content. |
| `/admin/blog-editor` | Create/edit article fields, upload/select assets, save draft, publish/unpublish. |
| `/admin/blog-persona` | Configure blog persona/tone/profile values and save strategy inputs. |
| `/admin/blog-strategy` | Manage content strategy, keywords, briefs, planning actions. |
| `/admin/blockchain-treasury` | Inspect blockchain treasury, run primary settlement, view blocker breakdowns. |
| `/admin/blockchain-contracts` | View live contracts overview and open contract detail. |
| `/admin/blockchain-contract-detail` | Inspect selected contract, pause/unpause where permitted. |
| `/admin/blockchain-sync` | Inspect Web3 sync/health and trigger sync actions. |
| `/admin/pending-settlements` | Review pending settlements, filter/export, run/retry settlement actions. |
| `/admin/affiliate-applications` | Review affiliate applications, approve/reject, open applicant detail/modals. |
| `/admin/affiliate-finance` | Review affiliate finance board, payout/commission board actions. |
| `/admin/affiliate-fraud` | Inspect fraud visualization signals. |
| `/admin/admin-affiliate-fraud` | Same fraud visualizer surface under alternate admin path. |
| `/admin/affiliate-teams` | Inspect developer affiliate teams and team lifecycle actions. |
| `/admin/templates/icons` | Browse platform icon library. |

## Admin Community Pages

| Page | What the user can do |
| --- | --- |
| `/admin/community/` | View community overview and navigate community admin modules. |
| `/admin/community/amas` | Create AMA, view questions, answer questions, change AMA status, toggle featured, close modals. |
| `/admin/community/announcements` | Create/edit announcements, publish/archive, refresh list. |
| `/admin/community/appeals` | Review ban appeals, approve/reject, add notes. |
| `/admin/community/badges` | Create/manage badges, search/filter badges, edit badge data. |
| `/admin/community/challenges` | Create/manage challenges, toggle challenge state, edit requirements/rewards. |
| `/admin/community/circles` | Create/manage circles, inspect circle health, open circle detail. |
| `/admin/community/circle-detail` | Inspect selected circle, manage members/settings/moderation context. |
| `/admin/community/comments` | Moderate global comments, search/filter, hide/delete/pin where available. |
| `/admin/community/leaderboard` | Refresh leaderboard, open XP modal, enter user UUID/XP amount/action/description, submit/cancel. |
| `/admin/community/posts` | Search/filter posts and open post detail. |
| `/admin/community/post-detail` | Lock/unlock thread, hide post, edit tags, moderate comments. |
| `/admin/community/reports` | Refresh reports, open moderation action modal, enter notes, confirm/cancel action. |
| `/admin/community/settings` | Edit community settings and save config. |
| `/admin/community/users` | Search/filter community users, warn/ban/mute/open user detail. |
| `/admin/community/user-detail` | Inspect community user, apply moderation actions, review profile/activity. |
| `/admin/community/verified-owner-requests` | Review verified-owner requests, approve/reject, refresh queue. |

## Admin Marketplace Pages

| Page | What the user can do |
| --- | --- |
| `/admin/marketplace/` | View marketplace overview metrics, refresh, navigate marketplace admin modules. |
| `/admin/marketplace/alerts` | Manage alerts/watchlist, filter/search, save/share views, export. |
| `/admin/marketplace/analytics` | Analyze trading metrics, filter date/asset, export/share views. |
| `/admin/marketplace/approvals` | Review pending marketplace approvals. |
| `/admin/marketplace/compliance` | Generate OJK/compliance reports, filter/export evidence. |
| `/admin/marketplace/fees` | View/edit fee settings, save marketplace fee configuration. |
| `/admin/marketplace/orderbook` | Inspect live orderbook, refresh/rebuild, filter by asset/side, export. |
| `/admin/marketplace/orders` | Rebuild book, inspect order metrics, save/share views, filter status/side/page size, toggle autorefresh, export CSV, group by user, choose columns, cancel/export selected orders, paginate. |
| `/admin/marketplace/p2p` | Review P2P offers, filter/search, approve/reject/cancel/flag offers where available. |
| `/admin/marketplace/primary-escrow` | Review primary escrow engine, run/retry/reconcile settlement actions, filter/export. |
| `/admin/marketplace/reconciliation` | Run balance reconciliation, review mismatches, filter/export, resolve/retry rows. |
| `/admin/marketplace/settings` | Configure marketplace switches, trading limits, fees, circuit-breaker-like settings. |
| `/admin/marketplace/trades` | Filter/search trade history, export, inspect trade detail, choose columns/views. |

## Discovered Workflows

1. Visitor to account: `/` -> `Start Investing` or `Sign up` -> `/auth/signup` -> accept terms/privacy -> account creation -> `/welcome` -> marketplace or KYC.
2. Visitor to property with return path: `/` -> property card -> `/p/:slug` -> enter amount/quick-add -> `Sign up to invest` with `returnTo=/p/:slug` -> after signup/login, return to intended property.
3. Returning user: `/auth/login` -> email/password or Google -> `/marketplace`; if 2FA/step-up is required, user enters authenticator code before continuing.
4. Password recovery: `/auth/login` -> `Forgot password?` -> enter email -> reset email -> `/auth/reset-password` -> set and confirm new password -> login.
5. Investor primary purchase: `/marketplace` -> filter/search -> `/property/:slug` or `/commodity/:slug` -> choose amount -> `Add to cart` -> `/cart` -> `/checkout` -> payment/bank transfer -> `/payment-in-progress` -> `/payment-success` -> portfolio/wallet.
6. Investor wallet funding: `/wallet` -> `Deposit` -> enter amount -> confirm deposit; or add card/bank account before deposit/withdrawal.
7. Investor portfolio/trading: `/portfolio` -> open holding/detail -> secondary market/trading route -> place order -> `/trade-success` -> monitor in `/my-trading`.
8. Investor settings: `/settings` -> update profile/address/identity/security/Web3/preferences/social -> save; for destructive privacy action -> `/account-deletion`.
9. Community participation: `/community` -> browse feed/announcements/circles/challenges/AMAs -> search/filter -> create/send messages or submit appeal if banned -> profile edit at `/community/me/edit`.
10. Support: `/support` -> create ticket with category/message/attachments -> reply/reopen/rate ticket as it progresses.
11. Affiliate onboarding: `/affiliate` -> `/affiliate/onboarding` -> profile -> KYC -> tax profile -> accept policies -> quiz -> submit application -> dashboard/materials/settings.
12. Developer asset submission: `/developer/add-asset` -> `/developer/application-form` -> `/developer/property-content` -> `/developer/document-upload-step3` -> submit/tokenize -> `/developer/submission-success` -> admin review queue.
13. Developer operations: `/developer/operations` -> monthly operations form -> fill revenue/occupancy/expenses/documents -> save draft/submit -> admin operations review.
14. Developer affiliate team: `/developer/affiliate-team` -> manage members/customers/products/settings/analytics/tier.
15. Admin developer approval: `/admin/developer-submissions` -> `/admin/developer-submission-review` -> edit page content/assets/documents/milestones -> approve/reject/request changes -> asset publication/tokenization flow.
16. Admin KYC/order/payment operations: `/admin/kyc`, `/admin/orders`, `/admin/deposits`, `/admin/treasury` -> filter queues -> inspect detail -> approve/reject/confirm/cancel/export as applicable.
17. Admin marketplace operations: `/admin/marketplace/*` -> monitor orderbooks/orders/trades/P2P/escrow/reconciliation -> rebuild/refresh/export/cancel/reconcile as needed.
18. Admin community moderation: `/admin/community/reports` or posts/comments/users -> inspect report/user/post -> enter notes -> warn/mute/ban/hide/resolve; AMAs/challenges/badges/circles are managed from their own admin pages.
19. Admin content workflow: `/admin/blog` -> `/admin/blog-editor` -> draft/edit article -> upload/select assets -> publish/unpublish/archive; strategy/persona pages support planning inputs.
20. Admin access governance: `/admin/admins` -> invite/manage admins; `/admin/roles` -> edit RBAC matrix and save permission model.

## Notable Findings From The Walkthrough

- Public production `https://www.poool.app/` returns the landing page; `https://platform.poool.app/` redirects unauthenticated users to `/auth/login`.
- `/imprint`, `/gdpr-data-request`, and `/aml-kyc-policy` are linked/declared in places but locally render 404 unless additional deployment routing exists.
- `/auth/2fa/setup` and step-up setup returned `500` locally during this pass.
- `/checkout` redirects to `/cart` when the cart is empty.
- `/affiliate/referrals` redirected to `/affiliate/onboarding` for the audited user state.
- `/property/sunset-luxury-villa` is missing for authenticated detail route, while `/p/sunset-luxury-villa` renders public detail successfully.
- Many admin detail pages are parameter/context dependent and render loading shells without query params; the action map documents their available shell controls and intended detail workflows.
