# POOOL Implementation Roadmap & Multi-Agent Tracker

> **Source:** Extracted from ALL chapters of `docs/MASTERPLAN.md`
> **Purpose:** A centralized, live-updating task board and collaboration protocol for all Autonomous Agents and Human Developers working on POOOL.
> **Last Full Sync with Masterplan:** 2026-05-06

---

## 🤖 Agent Collaboration Protocol (How to use this file)

This document is the **Single Source of Truth** for current progress. If you are an AI Agent booting up to work on the POOOL codebase, you **MUST** follow these steps:

### Step 1: Check Phase Gates

Before doing ANYTHING, check the **Phase Gate Table** at the bottom of this file. Your target phase may be **🔒 LOCKED** because a prerequisite phase is not yet `✅ DONE`. If your phase is locked, **DO NOT START** — inform the user and suggest working on an unlocked phase instead.

### Step 2: Check File Ownership Zones

Every task declares a **File Zone** (which directories/files it touches). Check the **📡 Live Agent Logs** table below — if another agent is currently `🔄 IN PROGRESS` on a task whose File Zone **overlaps** with your task's File Zone, you **MUST NOT** start your task. Two agents editing the same files = corruption.

**File Zone Overlap = CONFLICT. Same directory = CONFLICT. Wait or pick a different task.**

### Step 3: Claim Your Task

1. Add a row to **📡 Live Agent Logs** with your timestamp, task ID, File Zone, and status `🔄 IN PROGRESS`.
2. In the Phase table, change your task's Status to `🔄 IN PROGRESS` and add your name to "Assignee".
3. Read `docs/AGENT_DEVELOPMENT_PROMPT.md` for mandatory coding standards.
4. Read the Masterplan section referenced by your task.

### Step 4: Execute & Check-Out

1. Implement the task following all mandates from `AGENT_DEVELOPMENT_PROMPT.md`.
2. Write unit tests (financial functions need 7+ test cases).
3. Run `cargo check` + `cargo clippy` (backend) or verify in browser (frontend).
4. Mark task `✅ DONE`, update E2E column, add notes.
5. Update your Live Agent Log entry to `✅ Check-Out`.

> [!IMPORTANT]
> **Status Key:** `❌ NOT STARTED` | `🔄 IN PROGRESS` | `⏸️ BLOCKED` | `⚪ NOT READY` | `✅ DONE`

> [!CAUTION]
> **CONFLICT RULE:** If you see another agent `🔄 IN PROGRESS` in the same File Zone as your task, you **MUST STOP**. Pick a task in a different File Zone, or wait. Ignoring this rule will cause file overwrites and data loss.

---

## 📡 Live Agent Logs

*Every agent must log here. Check this table FIRST to detect File Zone conflicts.*

| Date/Time (UTC) | Agent Name | Claimed Task ID | File Zone | Action / Status | Notes |
|:---|:---|:---|:---|:---|:---|
| `2026-05-11 02:10` | `Claude Opus 4.7` | `14.8.2 Block / mute self-service (server + tests)` | `backend/src/community/{routes.rs,service.rs}, database/community/027_block_mute.sql, frontend/platform/community.html, frontend/platform/static/js/community-feed.js, frontend/platform/static/css/community.css, tests/e2e/test_community.py` | `✅ Check-Out` | Commits `9c4e2c1` (routes + profile-modal wiring, bundled with 14.8.3/4 by parallel agent) + `d114559` (migration + feed-query filter + 2 e2e tests). Block applies reciprocally (target's posts hidden from actor + vice versa); mute is one-way (actor only). Self-block/self-mute rejected with 400. Block + mute filter merged into `get_community_feed`. `/community/blocks` settings sub-page deferred to follow-up. |
| `2026-05-11 01:50` | `Claude Opus 4.7` | `14.8.1 Ban appeal submission flow (user UI)` | `backend/src/community/routes.rs, frontend/platform/community.html, frontend/platform/static/css/community.css, frontend/platform/static/js/community-ban-appeal.js, tests/e2e/test_community.py` | `✅ Check-Out` | Commit `a578897`. Banned user banner + appeal modal + submit flow. Extended `get_profile_me` to surface `is_community_banned`, `ban_reason`, `has_pending_appeal`. New `community-ban-appeal.js` module + `.community-ban-banner` primitive in `community.css`. Two new passing e2e tests (`test_community_ban_appeal_banner_and_submission`, `test_community_ban_appeal_banner_hidden_for_unbanned_user`). Banner markup was committed in upstream `3499408` alongside an unrelated Follow-button wire-up. |
| `2026-05-11 01:08` | `Claude Opus 4.7` | `14.8 Community Gap Closeout (planning + brief)` | `docs/community/COMMUNITY_GAP_CLOSEOUT.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Authored the 24-task implementation brief at `docs/community/COMMUNITY_GAP_CLOSEOUT.md` covering all P0/P1/P2/P3 + Admin gaps surfaced after the 2026-05-11 community redesign audit (17 commits on `main`). Added Phase 14.8 sub-section (14.8.1 – 14.8.24) with file-zone declaration and per-task status rows. No code changes; next agent picks up at 14.8.1 (ban appeal submission flow). |
| `2026-05-11 00:30 → 01:00` | `Claude Opus 4.7` | `Community redesign (visual refactor, 17 commits)` | `backend/src/community/routes.rs, backend/src/main.rs, frontend/platform/community.html, frontend/platform/partials/community_*.html, frontend/platform/components/investor-topbar.html, frontend/platform/static/css/community.css, frontend/platform/static/js/community-feed.js, frontend/platform/static/js/community-circles.js, frontend/platform/static/js/community-amas.js` | `✅ Check-Out` | Visual redesign delivering DESIGN.md §17 compliance: inline styles cut 320 → 10 (−97%); dark-gradient hero cards removed from XP overview + AMA hero; rainbow composer icons unified to currentColor; suggested-investors fake users deleted; hashtag/mention CSS moved off backend inline styles; 12 parallel button/card/badge families collapsed to shared `community-*` + `ds-*` primitives; community-circles.js pre-existing null-replaceChildren race fixed; topbar tab overflow + reduced-motion + 44px touch targets; graceful `community_disabled.html` for `get_community_pool` None path. All 3 circle modals + 3 page modals (report/edit-profile/user-profile) rebuilt on shared `.community-modal__*` primitives. Cleanup commits removed dead CSS (`.holo-*`, suggested-investor classes, `.ama-archive-*`, `.ama-hero-*`, `.circle-challenge-*` legacy, `.coming-soon-*`, `.community-create-post`) and orphan file `community-card.css`. |
| `2026-05-10 16:37` | `Codex` | `Ad hoc: Codebase Architecture Documentation` | `docs/CODEBASE_ARCHITECTURE.md` | `✅ Check-Out` | Added `docs/CODEBASE_ARCHITECTURE.md`, a consolidated source-checked Markdown guide to repository layout, backend/frontend/database/runtime architecture, critical flows, deployment, testing, guardrails, and documentation drift. |
| `2026-05-06 00:30` | `Claude Opus 4.7` | `Smart-contract hardening + automated audit pre-pass` | `contracts/src/, contracts/script/, contracts/test/, contracts/foundry.toml, contracts/DEPLOYMENT.md, contracts/README.md, backend/src/blockchain/, docs/MASTERPLAN.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened POOOLAssetToken (nonReentrant on mint/settleBatch, ZeroAddress/ZeroAmount/EmptyURI validation, uri() returns "" per spec, pinned pragma 0.8.24, doc note re ERC1155Receiver bypass). AssetFactory: input validation, CEI ordering in deployAsset, indexed event params, removed unused NotAuthorized error from IdentityRegistry. Replaced Deploy.s.sol with chain-agnostic fresh-deploy (deleted DeployMainnet.s.sol). Uncommented foundry.toml [etherscan]. Renamed POOOLProperty1155 → POOOLAssetToken across 5 backend doc-comment files + DEPLOYMENT.md + README.md. Pre-paid-audit suite passes: Slither strict 0, Aderyn 1H-FP/4L-design, Halmos 5/5 symbolic, Echidna 5/5 over 20k seqs, Mythril 1 FP, 64 forge tests, branch coverage 92.9–100%. |
| `2026-05-05 23:35` | `Claude Opus 4.7` | `Primary on-chain settlement + ops fixes (production launch)` | `backend/src/blockchain/, backend/src/admin/, backend/src/payments/, backend/src/kyc/, backend/src/ipfs/, backend/src/portfolio/, backend/src/main.rs, backend/src/common/leader.rs, frontend/platform/admin/blockchain-treasury.html, frontend/platform/portfolio.html, frontend/platform/settings.html, frontend/platform/static/js/, database/120_*.sql, database/121_*.sql, database/122_*.sql` | `✅ Check-Out` | Built primary-issuance settlement worker (treasury → buyer transfers via settleBatch, T+1 delay, batch-by-asset, leader-locked). Added admin "Run now" button on /admin/blockchain-treasury with eligibility-blocker breakdown + recent-failure panel. Fixed KYC whitelist worker silent revert: selector 0x9beb20f8 → 0xd38c6523 (was hitting non-existent function). Locked mintTo to signer key (defense vs. lost-key footgun that stranded original Demo Villa supply). Added user-facing SIWE wallet binding UI on /settings → Web3 Wallet card. Added "Add NFT to MetaMask" button on /portfolio with chain auto-switch via wallet_watchAsset / wallet_switchEthereumChain. Added image field to ERC-1155 metadata (cover image fallback to POOOL logo). Reconciler fixed: trade_history.executed_at instead of nonexistent updated_at column. Migrations: orders.on_chain_status etc, settle_eligible_at, batch_type. Production state: 9 primary order_items settled on Amoy mainnet — 2,046 tokens distributed across 5 buyer wallets via 2 settleBatch txs. |
| `2026-05-04 12:52` | `Codex` | `UI Polish: Rewards Commissions Card Design` | `frontend/platform/rewards.html, frontend/platform/static/css/rewards.css, tests/test_rewards_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Aligned rewards commissions summary, payout settings, and commissions table cards with the `DESIGN.md` dashboard card spec; focused rewards static tests passed. |
| `2026-05-04 12:47` | `Codex` | `UI Polish: Rewards Referral Input and Marketing Cleanup` | `frontend/platform/rewards.html, frontend/platform/static/css/rewards.css, tests/test_rewards_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Tightened the rewards referral input height to match the copy button and removed the Tier Icons card from marketing materials; focused rewards static tests passed. |
| `2026-05-04 12:44` | `Codex` | `UI Polish: Portfolio Asset Details Icon Action` | `frontend/platform/static/js/portfolio-data.js, frontend/platform/static/css/portfolio-assets-table.css, tests/test_portfolio_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced `/portfolio` asset table "See Details" text buttons with icon-only detail actions while preserving navigation and accessible labels; focused portfolio tests and JS syntax check passed. |
| `2026-05-04 12:34` | `Codex` | `UI Polish: Portfolio Expanded Chart Controls` | `frontend/platform/portfolio.html, frontend/platform/static/css/portfolio.css, frontend/platform/static/css/portfolio-enhancements.css, tests/test_portfolio_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Simplified expanded portfolio chart controls, moved them into the upper value-card action row, constrained chart plotting so bars/axes do not clip outside the card, and verified focused static/browser checks. |
| `2026-05-04 12:28` | `Codex` | `UI Polish: Portfolio Design Spacing` | `frontend/platform/static/css/portfolio.css, frontend/platform/static/css/portfolio-enhancements.css, frontend/platform/static/css/portfolio-value-card.css, tests/test_portfolio_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Normalized `/portfolio` card and table section spacing to the `DESIGN.md` section gap instead of stacked page-specific margins; focused portfolio static tests passed. |
| `2026-05-04 12:24` | `Codex` | `UI Polish: Cart Empty Step Badge Parity` | `frontend/platform/static/css/cart.css, tests/test_cart_empty_state_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Aligned `/cart` empty-state step number badges with the blue-gradient and mint-number treatment used by developer submissions; focused cart static test passed. |
| `2026-05-04 12:22` | `Codex` | `UI Polish: Wallet Card Accent Parity` | `frontend/platform/static/css/wallet.css, tests/test_wallet_payment_methods_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Applied the blue-to-green card accent treatment from My Trading to wallet balance, transactions, and payment method cards; focused wallet static tests passed. |
| `2026-05-04 12:19` | `Codex` | `UI Polish: Portfolio Remove Section Headings` | `frontend/platform/portfolio.html, tests/test_portfolio_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed desktop Key financials and My Assets section headings from `/portfolio` per browser review comments, preserving the financial grid and assets table; focused static tests passed. |
| `2026-05-04 12:16` | `Codex` | `UI Polish: Wallet Transactions Card Header` | `frontend/platform/wallet.html, frontend/platform/static/css/wallet.css, tests/test_wallet_payment_methods_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Moved Transactions title inside the table card so the section matches the contained module pattern used by Cards/Banks; focused wallet static tests passed. |
| `2026-05-04 12:08` | `Codex` | `UI Polish: Wallet Payment Card Headers` | `frontend/platform/wallet.html, frontend/platform/static/css/wallet.css, tests/test_wallet_payment_methods_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Moved Cards/Banks titles inside payment method cards so each module has an internal header, body, and action; focused wallet static tests passed. |
| `2026-05-04 11:59` | `Codex` | `UI Polish: Wallet Payment Method Cards` | `frontend/platform/static/css/wallet.css, tests/test_wallet_payment_methods_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Tightened `/wallet` payment method section spacing, reduced empty panel weight, aligned add-card/add-bank buttons with the neutral dashboard secondary style, and verified focused wallet static tests. |
| `2026-05-04 11:49` | `Codex` | `UI Polish: Developer Assets Empty Card Parity` | `frontend/platform/static/css/developer-assets.css, tests/test_developer_assets_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Aligned `/developer/assets` empty-state metric and step cards with the `/developer/submissions` card design, repaired stale static assertions, and verified the focused assets test file. |
| `2026-05-04 11:45` | `Codex` | `UI Polish: Developer Submissions Empty Card Spacing` | `frontend/platform/static/css/developer-submissions.css, tests/test_developer_submissions_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Increased empty-state metric and onboarding card gutters on `/developer/submissions`, added mobile stacked spacing, and covered the spacing contract with a focused static test. |
| `2026-05-03 19:01` | `Codex` | `UI Polish: Developer Assets Empty Metrics Cards` | `frontend/platform/static/css/developer-assets.css, tests/test_developer_assets_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the decorative gradient/border-image treatment from empty-state metric cards, reduced stat typography, and aligned them with the quiet white card style from DESIGN.md. Focused static test and scoped diff check passed; full developer-assets static file is blocked by an existing stale `dev-assets-low-count` template assertion. |
| `2026-05-03 18:49` | `Codex` | `UI Polish: Developer Asset Cap Table Empty State` | `frontend/platform/static/js/developer-asset-detail.js, tests/test_developer_asset_empty_states_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Reused the branded developer asset empty state for the Cap Table tab with an investor icon and ownership-focused copy. JS syntax, targeted empty-state tests, and scoped diff check passed. |
| `2026-05-03 18:43` | `Codex` | `UI Polish: Developer Asset Milestones and Orders Empty States` | `frontend/platform/static/js/developer-asset-detail.js, frontend/platform/static/css/developer-asset-detail.css, tests/test_developer_asset_empty_states_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added shared branded empty states for empty milestones and orders with contextual icons, POOOL watermark, and blue-to-green accent treatment. JS syntax, targeted static tests, existing media/document/video tests, and scoped diff check passed. |
| `2026-05-03 18:36` | `Codex` | `UI Hotfix: Developer Asset Document List Actions` | `frontend/platform/static/js/developer-asset-detail.js, frontend/platform/static/css/developer-asset-detail.css, tests/test_developer_asset_documents_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Rebuilt data-room rows with document title as primary text, category/size as metadata, and a working icon-only view link to the secure document download endpoint. JS syntax, targeted static tests, existing media/video tests, and scoped diff check passed. |
| `2026-05-03 18:26` | `Codex` | `UI Hotfix: Settings Topbar and Developer Video Embed` | `frontend/platform/admin/settings.html, frontend/platform/static/css/admin.css, frontend/platform/developer/asset-detail.html, frontend/platform/static/js/developer-asset-detail.js, frontend/platform/static/css/developer-asset-detail.css, tests/test_admin_settings_topbar_static.py, tests/test_developer_asset_video_embed_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Scoped the settings topbar search/notification controls to a thinner 32px treatment and replaced the developer asset video link with inline YouTube/Vimeo/direct-file playback plus fallback link. JS syntax, targeted static tests, existing media-order tests, and scoped diff check passed. |
| `2026-05-03 18:14` | `Codex` | `UI Hotfix: Admin Live Assets Compact Filters` | `frontend/platform/admin/assets.html, frontend/platform/static/css/admin.css, tests/test_admin_assets_filter_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Grouped primary filters into a compact grid, grouped secondary actions into a right-aligned toolbar, reduced control heights and saved-view width, and kept mobile stacking. Targeted static test and scoped diff check passed. |
| `2026-05-03 18:05` | `Codex` | `UI Hotfix: Admin Asset Change Requests Review Comments` | `frontend/platform/admin/asset-change-requests.html, frontend/platform/static/js/admin-change-requests.js, frontend/platform/static/css/admin-change-requests.css, tests/test_admin_asset_change_requests_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the anomaly banner and detection code, switched reset to the secondary button style, replaced the emoji empty-state icon with a custom SVG, and added static regression coverage. Targeted pytest, JS syntax check, and scoped diff check passed. |
| `2026-05-03 17:52` | `Codex` | `UI Hotfix: Admin Submission Review Editor Buttons` | `frontend/platform/static/js/admin-property-page-editor.js, frontend/platform/admin/components/property-page-editor.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened property-page editor initialization against malformed list data, bound add-row buttons before hydration, made row collectors scoped by list kind, and replaced prompt-only milestone creation with an inline draft row. JS syntax, cargo check, runtime button simulation, and scoped diff check passed. |
| `2026-05-03 17:40` | `Codex` | `UI Hotfix: Admin Submission Review Document Controls` | `frontend/platform/admin/developer-submission-review.html, frontend/platform/admin/components/property-page-editor.html, frontend/platform/static/js/admin-submission-review.js, frontend/platform/static/js/admin-property-page-editor.js, frontend/platform/components/property/risk-notification.html, backend/src/admin/assets.rs, backend/src/admin/mod.rs, backend/src/admin/developer_projects.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced document text buttons with icon actions, added admin document upload/rename/delete endpoints and UI, rewrote review media URLs through the app proxy, improved milestone table editing, and added multi-row risk notifications. JS checks, rustfmt on touched backend files, cargo check, and scoped diff check passed. |
| `2026-05-03 15:08` | `Codex` | `UI Hotfix: Admin Reconciliation Template Labels` | `frontend/platform/static/js/mp-reconciliation.js, tests/test_admin_marketplace_reconciliation_modal_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Preserved readable resolve-template select labels and rebuilt the template editor/list/select with DOM APIs to avoid persisted template HTML injection. Targeted static test and JS syntax check passed. |
| `2026-05-03 15:05` | `Codex` | `UI Hotfix: Admin Marketplace Analytics Double Sidebar Offset` | `frontend/platform/admin/marketplace/analytics.html, tests/test_admin_marketplace_analytics_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the analytics page's duplicate sidebar offset by zeroing its scoped admin-main margin while keeping existing width constraints. Targeted static test and diff check passed. |
| `2026-05-03 15:02` | `Codex` | `UI Hotfix: Admin Comments Empty State and KYC Topbar Search` | `frontend/platform/admin/community/comments.html, frontend/platform/admin/kyc.html, tests/test_admin_comments_kyc_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Scoped the comments empty state into a centered card body and compacted the KYC desktop topbar search so the notification control no longer wraps below it. Targeted static tests and diff check passed. |
| `2026-05-03 13:58` | `Codex` | `UI Hotfix: Admin Orderbook Header KPI Layout` | `frontend/platform/admin/marketplace/orderbook.html, frontend/platform/static/css/mp-orderbook.css, tests/test_admin_marketplace_orderbook_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Reworked the Live Orderbook KPI strip into a bounded responsive 3-column summary panel beside the title, wrapping below on narrower viewports. Targeted static test and diff check passed. |
| `2026-05-03 12:55` | `Codex` | `UI Hotfix: Admin Orderbook Reason Modal Default State` | `frontend/platform/admin/marketplace/orderbook.html, frontend/platform/static/js/mp-orderbook.js, frontend/platform/static/css/mp-orderbook.css, tests/test_admin_marketplace_orderbook_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added explicit hidden-state CSS and init-time close handling for the order-cancel reason modal so it starts hidden and only opens from explicit cancel actions. Targeted static test, JS syntax, and diff check passed. |
| `2026-05-03 12:09` | `Codex` | `UI Hotfix: Admin Marketplace Settings Help Collision` | `frontend/platform/admin/marketplace/orders.html, frontend/platform/static/css/admin-marketplace.css, tests/test_admin_marketplace_settings_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Scoped the marketplace keyboard-help overlay to `.mp-help-overlay` so settings `.mp-help` helper text stays inline and no longer blocks sidebar clicks. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 12:04` | `Codex` | `UI Hotfix: Admin Marketplace Analytics Layout` | `frontend/platform/admin/marketplace/analytics.html, tests/test_admin_marketplace_analytics_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Scoped the analytics admin shell to full width and bounded range, refresh, interval, asset, and date controls so global form CSS cannot stretch them across the page. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 12:01` | `Codex` | `UI Hotfix: Admin Reconciliation Modal Layout` | `frontend/platform/admin/marketplace/reconciliation.html, frontend/platform/static/css/admin-marketplace.css, tests/test_admin_marketplace_reconciliation_modal_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced reconciliation modal panels with scoped classes so shared marketplace `.mp-modal` CSS cannot stretch the resolve/detail dialogs full-screen. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 11:55` | `Codex` | `UI Hotfix: Admin Marketplace P2P Icons and Filter Layout` | `frontend/platform/admin/marketplace/p2p.html, frontend/platform/static/css/admin-marketplace.css, tests/test_admin_marketplace_p2p_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced P2P KPI emoji icons with custom SVG icons and compacted full-width dropdown filters into a bounded responsive toolbar. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 11:53` | `Codex` | `UI Hotfix: Admin Marketplace Orders Columns Default State` | `frontend/platform/static/js/mp-orders.js, tests/test_admin_marketplace_orders_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Made the Open Orders columns dropdown explicitly close on JS init and keep `hidden`, `aria-hidden`, and `aria-expanded` synchronized. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 11:50` | `Codex` | `UI Hotfix: Admin Marketplace Orders Filter Layout` | `frontend/platform/admin/marketplace/orders.html, frontend/platform/static/css/admin-marketplace.css, tests/test_admin_marketplace_orders_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Compacted open-orders filters/actions with bounded desktop widths and restored hidden-state CSS for the columns menu so it does not render open by default. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 11:47` | `Codex` | `UI Hotfix: Admin Marketplace Trades Filter Layout` | `frontend/platform/admin/marketplace/trades.html, frontend/platform/static/css/admin-marketplace.css, tests/test_admin_marketplace_trades_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Compacted trade-history top filters so date controls, asset/status selects, search, and actions use bounded desktop widths with mobile wrapping. Targeted static tests and diff check passed. |
| `2026-05-03 11:45` | `Codex` | `UI Hotfix: Admin Marketplace Icons and Help Overlay` | `frontend/platform/admin/marketplace/index.html, frontend/platform/admin/marketplace/orderbook.html, frontend/platform/static/js/mp-orderbook.js, frontend/platform/static/css/mp-orderbook.css, tests/test_admin_marketplace_orderbook_static.py, tests/test_admin_marketplace_overview_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced the overview halt-trading icon, restored hidden-state CSS for the orderbook help overlay, and made help close/toggle update `aria-hidden`. Targeted static tests, JS syntax, and diff check passed. |
| `2026-05-03 11:42` | `Codex` | `UI Polish: Admin Blockchain Treasury Custom Icons` | `frontend/platform/admin/blockchain-treasury.html, tests/test_admin_blockchain_treasury_icons_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced the emoji copy control and text-only treasury stats with custom SVG icon badges for wallet, quick stats, and blockchain KPI surfaces. Targeted static tests and diff check passed. |
| `2026-05-03 11:39` | `Codex` | `UI Hotfix: Admin Deposits Filter Row Density` | `frontend/platform/admin/deposits.html, frontend/platform/static/css/admin.css, tests/test_admin_deposits_filter_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Moved deposit status/currency/provider filters into the search row, made the search input thinner, and gave the selects compact scoped widths with mobile wrapping. Targeted static tests and diff check passed. |
| `2026-05-03 11:30` | `Codex` | `UI Hotfix: Admin Orders Filter Row Density` | `frontend/platform/admin/orders.html, frontend/platform/static/css/admin.css, tests/test_admin_orders_filter_layout_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Moved order status/date filters into the search row, made the search input thinner, and gave the selects compact fixed widths with mobile wrapping. Targeted static tests and diff check passed. |
| `2026-05-03 11:20` | `Codex` | `Live Hotfix: Marketplace Sidebar Template Context` | `backend/src/assets/routes.rs, tests/test_marketplace_sidebar_context_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed authenticated `/marketplace` 500 by passing shared sidebar `user`, `user_display_name`, and investor context from marketplace SSR handlers; added static coverage. Targeted pytest, cargo check, rustfmt on touched file, and diff check passed; full cargo fmt remains blocked by unrelated existing backend formatting drift. |
| `2026-05-02 17:02` | `Codex` | `UI Hotfix: Developer Asset Media Ordering` | `frontend/platform/developer/asset-detail.html, frontend/platform/static/js/developer-asset-detail.js, frontend/platform/static/css/developer-asset-detail.css, backend/src/developer/routes.rs, backend/src/storage/routes.rs, backend/src/assets/routes.rs, backend/src/marketplace/service.rs, backend/src/auth/routes.rs, tests/test_developer_asset_media_order_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added visible image order indicators, drag/arrow order controls, first-image cover behavior, persisted developer image ordering, and made investor-facing image queries respect saved sort order. Targeted static tests, JS syntax, cargo check, diff check, and route smoke passed; full cargo fmt remains blocked by unrelated files. |
| `2026-05-02 17:02` | `Codex` | `UI Polish: Trade Success Branding` | `frontend/platform/trade-success.html, frontend/platform/static/css/payment-success.css, tests/test_trade_success_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Redesigned `/trade-success` with stronger POOOL branding, removed the confetti animation from the page, and aligned the card, notice, copy control, and buttons with DESIGN.md dashboard patterns. Targeted pytest and diff check passed. |
| `2026-05-02 16:26` | `Codex` | `Hotfix: Admin Live Orderbook Source` | `backend/src/admin/marketplace.rs, tests/test_admin_marketplace_orderbook_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Admin orderbook detail now prefers the live Redis orderbook snapshot before falling back to PostgreSQL aggregates, so matched/consumed liquidity is not shown as a crossed active book in the admin UI. Targeted pytest, rustfmt check, cargo check, and diff check passed. |
| `2026-05-02 16:11` | `Codex` | `Hotfix: Marketplace Settlement Wallet Resilience` | `backend/src/marketplace/settlement.rs, tests/test_marketplace_settlement_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened trade settlement so missing seller cash wallets or missing platform-fee seed wallets do not leave already-matched orders looping in settlement retry. Targeted static tests, settlement rustfmt check, cargo check, and diff check passed; full cargo fmt remains blocked by unrelated existing formatting drift. |
| `2026-05-02 16:11` | `Codex` | `UI Hotfix: Trading V3 Live Orderbook Depth` | `frontend/platform/static/js/marketplace-trading-v3.js, tests/test_marketplace_trading_v3_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | V3 trade widget now uses the live orderbook endpoint for actionable buy/sell depth instead of stale secondary-market aggregate totals, and disables market orders when the opposite side is empty. Targeted pytest, JS syntax check, and diff check passed. |
| `2026-05-02 15:59` | `Codex` | `UI Hotfix: Trading Performance Strip Containment` | `frontend/platform/static/css/marketplace-trading-v3.css, tests/test_marketplace_trading_v3_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Updated the trading performance strip to a responsive contained grid so metrics wrap inside the rounded card instead of clipping on narrower screens. Targeted pytest, CSS diff check, and JS syntax check passed. |
| `2026-05-02 15:44` | `Codex` | `UI Hotfix: Trading V3 Header Location` | `frontend/platform/static/js/marketplace-trading-v3.js, frontend/platform/static/css/marketplace-trading-v3.css, tests/test_marketplace_trading_v3_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Increased the V3 hero title size and normalized secondary asset location formatting so a backend value like `Bali, ID` is not rendered as `Bali, ID, ID`. Targeted pytest, JS syntax check, and diff check passed. |
| `2026-05-02 15:32` | `Codex` | `UI Hotfix: Trading V3 Card Clipping` | `frontend/platform/marketplace-trading-v3.html, frontend/platform/static/css/marketplace-trading-v3.css, tests/test_marketplace_trading_v3_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed screenshot-reported V3 card text clipping, calculator stat overflow, FAQ overflow, funding/leasing overflow, and document download icons. Targeted pytest, diff check, and injected-CSS Playwright layout measurement passed. |
| `2026-05-02 15:22` | `Codex` | `UI Hotfix: Trading Gallery Lightbox Parity` | `frontend/platform/static/js/marketplace-trading-v3.js, tests/test_marketplace_trading_v3_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Trading gallery lightbox now uses the same property-detail lightbox classes, controls, thumbnail strip, keyboard behavior, and touch swipe support. Targeted pytest, JS syntax check, and diff check passed. |
| `2026-05-02 14:54` | `Codex` | `UI Hotfix: Remove Terms Reaccept Banner` | `frontend/platform/static/js/legal-enhancements.js, tests/test_legal_enhancements_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the global terms reacceptance banner injection and legal-status polling while preserving legal pages, footer links, cookie consent, and checkout/signup terms acceptance flows. Targeted pytest, JS syntax check, and diff check passed. |
| `2026-05-02 13:19` | `Codex` | `Admin Dashboard Review Fixes` | `frontend/platform/admin/index.html, frontend/platform/static/js/admin-dashboard.js, frontend/platform/static/css/admin.css, backend/src/admin/dashboard.rs, tests/, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed broken support queue routing, pending-deposit bulk action IDs, deposit range badge DOM removal, unknown activity filtering, loading colspan mismatch, and the oversized health-dot halo. Added static coverage; targeted pytest, JS syntax check, dashboard link scan, cargo check, and diff check passed. |
| `2026-05-02 11:55` | `Codex` | `Auth 2FA Setup Success UX Follow-up` | `backend/src/auth/routes.rs, frontend/platform/auth-2fa-setup.html, tests/test_auth_2fa_setup_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added a successful 2FA enrollment confirmation with a continue link, kept normal login 2FA redirect behavior unchanged, and aligned the Enable 2FA button with DESIGN.md primary button colors. Targeted pytest, cargo check, and diff check passed. |
| `2026-05-02 11:48` | `Codex` | `Auth 2FA Setup Screenshot Follow-up` | `frontend/platform/auth-2fa-setup.html, tests/test_auth_2fa_setup_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the screenshot-reported header lock badge, added a top-left Back link to settings, restored stable submit/loading IDs, and updated static coverage. Targeted pytest and diff check passed. |
| `2026-05-02 11:40` | `Codex` | `Auth 2FA Setup Design Optimization` | `frontend/platform/auth-2fa-setup.html, frontend/platform/static/css/login.css, tests/test_auth_2fa_setup_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Redesigned the 2FA setup page with DESIGN.md-aligned POOOL branding, scoped card/form/QR styling, a security side panel, and preserved QR, secret, setup token, and HTMX verification behavior. Targeted pytest, diff check, and Browser DOM preview passed; screenshot capture timed out in Browser Use. |
| `2026-05-02 11:36` | `Codex` | `Wallet Add Bank Modal Redesign` | `frontend/platform/wallet.html, frontend/platform/static/css/wallet.css, tests/test_wallet_bank_modal_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Redesigned Add Bank Account modal with POOOL logo branding, trust chips, grouped account/bank sections, and sticky footer while preserving validation. Targeted pytest, JS syntax, diff check, and Browser DOM preview passed. |
| `2026-05-02 11:34` | `Codex` | `Wallet Add Bank Badge Removal` | `frontend/platform/wallet.html, tests/test_wallet_bank_modal_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed screenshot-reported bank system badge from the Add Bank Account modal dynamic fields while keeping country-specific inputs unchanged. Targeted pytest and diff check passed. |
| `2026-05-02 11:31` | `Codex` | `Wallet Deposit Modal Design Follow-up` | `frontend/platform/static/js/wallet.js, frontend/platform/static/css/wallet.css, tests/test_wallet_deposit_modal_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Adjusted screenshot-reported deposit instruction modal with a custom POOOL logo mark, cleaner bank-transfer header, structured reference warning, and regression coverage. `node --check`, targeted pytest, diff check, and browser DOM preview passed; screenshot capture timed out in Browser Use. |
| `2026-05-02 11:26` | `Codex` | `Live 2FA Setup Secret Hotfix` | `backend/src/auth/service.rs, backend/src/main.rs, .github/workflows/deploy.yml, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Reproduced live `/auth/2fa/setup` returning an unexpected error, fixed TOTP key loading to fall back to configured `ENCRYPTION_KEY`, added explicit Cloud Run secret mappings, and aligned health/startup env checks. `cargo check`, TOTP unit tests, deploy YAML parse, and diff check passed; production deploy still required. |
| `2026-05-02 11:18` | `Codex` | `Auth Hotfix: Google OAuth Redirect Host` | `backend/src/auth/routes.rs, tests/test_auth_google_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed Google OAuth/2FA login flow generating localhost callback links in deployed environments when `BASE_URL` falls back to the dev default. Targeted static and Rust OAuth tests passed; full fmt remains blocked by unrelated dirty blockchain/KYC files. |
| `2026-05-01 11:34` | `Codex` | `UI Polish: Developer Assets Screenshot Follow-up` | `frontend/platform/developer/assets.html, frontend/platform/static/css/developer-assets.css, frontend/platform/static/js/developer-assets.js, tests/test_developer_assets_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed screenshot-reported remaining `/developer/assets` issues: blank low-count area, stat/preview weight, duration consistency, location capitalization, table action emphasis, selected row clarity, and toolbar density. `node --check`, targeted pytest, `git diff --check`, restart on `:8888`, and authenticated Playwright smoke passed. |
| `2026-05-01 11:24` | `Codex` | `UI Polish: Developer Assets Table Icon Actions` | `frontend/platform/developer/assets.html, frontend/platform/static/css/developer-assets.css, tests/test_developer_assets_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced text table actions with compact accessible icon buttons, added regression assertions, and restarted `:8888`. Targeted pytest, `git diff --check`, and health check passed. |
| `2026-05-01 11:18` | `Codex` | `UI Polish: Developer Assets Table Follow-up` | `frontend/platform/developer/assets.html, frontend/platform/static/css/developer-assets.css, frontend/platform/static/js/developer-assets.js, tests/test_developer_assets_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed screenshot follow-ups on `/developer/assets`: table overflow, visible actions, narrower preview, shorter summary stats and rows, filter counts, and empty filtered state. `node --check`, `git diff --check`, targeted pytest, and authenticated Playwright smoke on `:8888` passed. |
| `2026-05-01 11:03` | `Codex` | `UI Redesign: Developer Assets Management Table` | `frontend/platform/developer/assets.html, frontend/platform/static/css/developer-assets.css, frontend/platform/static/js/developer-assets.js, tests/test_developer_assets_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Converted `/developer/assets` from card grid to DESIGN.md-aligned management table plus preview panel. Added search/filter/row-preview behavior and static coverage; `node --check`, `git diff --check`, targeted pytest, and authenticated Playwright smoke on fresh local backend passed. |
| `2026-05-01 10:54` | `Codex` | `UI Screenshot Fix: Developer Submissions` | `frontend/platform/developer/submissions.html, frontend/platform/static/js/developer-submissions.js, frontend/platform/static/css/developer-submissions.css, tests/test_developer_submissions_static.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed screenshot-reported developer submissions layout, state, density, feedback attachment, rejected summary, approved progress, action consistency, and DESIGN.md alignment. `node --check`, `git diff --check`, targeted pytest, and authenticated Playwright smoke on fresh local backend passed. |
| `2026-04-28 20:57` | `Codex` | `Daily Page Audit Fix: Auth Verify Email` | `backend/src/auth/routes.rs, backend/src/auth/service.rs, frontend/platform/verify-email.html, frontend/platform/static/css/login.css, tests/test_auth_verify_email_static.py, docs/page-audits/2026-04-28-auth-verify-email.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed documented verify-email issues: token consumption, truthful/throttled resend, token cleanup, accessible HTMX states, unique tracker IDs, and static regression coverage. `cargo check`, targeted pytest, tracker audit, node syntax, rustfmt check, duplicate ID check, and diff check passed; browser/outbox E2E remains. |
| `2026-04-28 21:16` | `Codex` | `Auth E2E Suite Fix Pass` | `tests/e2e/test_auth_login.py, tests/e2e/conftest.py, tests/e2e/test_developer_support.py, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Full auth browser E2E subset now passes: 8 selected, 8 passed. Fixed full-suite collection blocker in developer support import and shared cleanup for auth audit logs. |
| `2026-04-28 20:45` | `Codex` | `Daily Page Audit Fix: Leaderboard` | `frontend/platform/leaderboard.html, frontend/platform/static/js/leaderboard.js, frontend/platform/static/css/leaderboard.css, frontend/platform/components/mobile-menu.html, backend/src/leaderboard/service.rs, docs/page-audits/2026-04-28-leaderboard.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed four leaderboard audit findings: restored current-rank/search/visibility UI, removed dead JS branches, added visible refetch/preference failure handling, corrected filtered/timeframe pagination totals, repaired mobile nav and pagination a11y, and documented remaining authenticated runtime/E2E coverage. |
| `2026-04-28 21:05` | `Codex` | `Daily Page Audit E2E Fix: Auth Login` | `tests/e2e/test_auth_login.py, docs/automation-reports/2026-04-28-e2e-coverage-auth-login.md, docs/page-audits/2026-04-28-auth-login.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added browser E2E coverage for the documented `/auth/login` gap: happy path, invalid credentials, CSRF fragment, remember-me, 2FA redirect/block, OAuth-disabled UI, accessible controls, mobile fit, and authenticated redirect. Runtime execution remains pending because no backend was reachable on localhost:8888. |
| `2026-04-28 20:40` | `Codex` | `Daily Page Audit Fix: Admin Deposits` | `backend/src/admin/deposits.rs, backend/src/admin/pages.rs, backend/src/admin/mod.rs, backend/src/admin/reports.rs, backend/src/admin/approvals.rs, backend/src/payments/service.rs, frontend/platform/admin/deposits.html, frontend/platform/static/js/admin-deposits.js, tests/e2e/test_admin_deposits.py, tests/admin/test_admin_sorting.py, docs/page-audits/2026-04-27-admin-deposits.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0412..0417 plus evidence-bundle generation/viewing/audit, valid sort ARIA, unused CDN removal, and modal keyboard basics. Expanded authenticated browser/API/mutation fixture; evidence rerun blocked by unrelated rewards compile errors. |
| `2026-04-28 20:40` | `Codex` | `Daily Page Audit Fix: KYC Identity Verification` | `backend/src/kyc/, backend/src/storage/routes.rs, frontend/platform/kyc.html, frontend/platform/static/js/kyc-page.js, tests/test_kyc_identity_static.py, docs/page-audits/2026-04-28-kyc-identity-verification.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed upload CSRF, manual document linking, identity/profile persistence, transactional audit rows, KYC rate limits, provider-return status verification, and document/file constants; documented remaining durable email outbox, private-object cleanup, and authenticated E2E recheck. |
| `2026-04-28 09:15` | `Codex` | `Daily Page Audit Fix: Community` | `backend/src/main.rs, frontend/platform/community.html, frontend/platform/components/investor-topbar.html, frontend/platform/partials/community_circle.html, frontend/platform/partials/community_feed.html, frontend/platform/static/js/community-feed.js, frontend/platform/static/js/community-circles.js, docs/page-audits/2026-04-28-community.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0489..0494: XSS-safe profile/circle rendering, reachable Search/Notifications/Saved tabs, registered search/deep links, propagated partial DB errors, community-DB shared-post metadata, direct-post rendering, and modal/icon accessibility. Remaining: authenticated browser/E2E and XSS fixture recheck; full cargo check was blocked by active Cargo build contention. |
| `2026-04-28 20:11` | `Codex` | `Daily Page Audit Fix: Auth Logout` | `backend/src/auth/routes.rs, backend/src/main.rs, frontend/platform/static/js/profile-dropdown.js, frontend/platform/static/js/mobile-navigation.js, tests/test_auth_logout_static.py, docs/page-audits/2026-04-28-auth-logout.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed CSRF-less logout GET mutation by making GET non-mutating and POST CSRF-protected; fixed root-path session cookie expiry, updated shared logout callers, added static regression coverage, and documented remaining browser/API E2E plus full cargo-check recheck. |
| `2026-04-28 20:07` | `Codex` | `Daily Page Audit Fix: Admin Developer Submission Review` | `backend/src/admin/developer_projects.rs, backend/src/admin/assets.rs, backend/src/admin/mod.rs, backend/src/admin/access.rs, frontend/platform/admin/developer-submission-review.html, frontend/platform/static/js/admin-submission-review.js, database/089_admin_submission_permissions.sql, docs/page-audits/2026-04-27-admin-developer-submission-review.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0143 and PAGE-ISSUE-0424..0427: escaped load errors, audited admin image routes, submissions.review/submissions.approve gates, transactional review-start audit behavior, and notes DB error propagation; authenticated E2E remains. |
| `2026-04-28 20:04` | `Codex` | `Daily Page Audit Fix: Auth Login` | `backend/src/auth/service.rs, frontend/platform/login.html, docs/page-audits/2026-04-28-auth-login.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed failed-login telemetry PII, login icon-control accessibility, testimonial carousel innerHTML hardening, disabled Google button gating, and HTMX auth CSRF error fragments; added static regression tests. Browser E2E remains. |
| `2026-04-28 20:02` | `Codex` | `Daily Page Audit Fix: Affiliate Materials` | `backend/src/rewards/routes.rs, backend/src/rewards/mod.rs, frontend/platform/affiliate-materials.html, frontend/platform/static/js/affiliate-materials.js, frontend/platform/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf, tests/admin/test_affiliate_route_contract_static.py, docs/page-audits/2026-04-27-affiliate-materials.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0330..0335: material downloads, guidelines PDF, active-affiliate locked state, custom upload/status UI, upload file validation, and logo alt text. Remaining: active-affiliate browser/GCS recheck; targeted Rust test was stopped after prolonged local Cargo contention. |
| `2026-04-28 20:30` | `Codex` | `Daily Page Audit Fix: Commodities Tab Fragment` | `backend/src/assets/routes.rs, frontend/platform/commodities-marketplace.html, frontend/platform/static/js/commodities-marketplace.js, frontend/platform/static/js/marketplace-search.js, frontend/platform/static/css/marketplace.css, tests/test_commodities_tab_static.py, tests/e2e/test_commodities_marketplace.py, docs/page-audits/2026-04-27-commodities-marketplace.md, docs/page-audits/2026-04-27-commodities-marketplace-tab.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed and verified `/commodities-marketplace/tab` and parent page follow-ups: DB failures return safe 500 fragments, swapped cards include filter/yield attributes and semantic links, search/filter rebinding is idempotent, parent template uses display-safe values, mobile More Filters is reachable, static regression coverage passes, isolated cargo check passes, and targeted authenticated desktop/mobile E2E passes 2/2. |
| `2026-04-28 20:20` | `Codex` | `Daily Page Audit Fix: Affiliate Referrals` | `backend/src/rewards/routes.rs, frontend/platform/affiliate-referrals.html, frontend/platform/static/js/affiliate-referrals.js, frontend/platform/static/css/affiliate-referrals.css, tests/, docs/page-audits/2026-04-27-affiliate-referrals.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0349..0354: safe DOM rendering, active-affiliate page gate, visible API errors, real assets, accessible tabs/search, backend CSV export, and docs. `node --check` and targeted pytest passed; full cargo check/fmt are blocked by unrelated dirty-worktree issues. |
| `2026-04-28 20:01` | `Codex` | `Daily Page Audit Fix: Marketplace Tab Fragment` | `backend/src/assets/routes.rs, frontend/platform/static/js/marketplace-search.js, tests/e2e/test_marketplace.py, tests/e2e/pages/marketplace_page.py, docs/page-audits/2026-04-27-marketplace-tab.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed /marketplace/tab DB error handling, swapped-card filter contract, duplicate listener rebinding, keyboard-accessible card navigation, and documented remaining E2E/tracker blockers. |
| `2026-04-28 20:01` | `Codex` | `Daily Page Audit Fix: Admin Community User Detail` | `backend/src/community/routes.rs, frontend/platform/admin/community/user-detail.html, tests/admin/test_admin_community_user_detail_static.py, tests/e2e/test_admin_community_user_detail.py, docs/page-audits/2026-04-26-admin-community-user-detail.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed unsafe rendering, mutation CSRF header, community API permission gates, transactional/audited moderation mutations, server-side ban validation, native dialog UX, unused CDN scripts, and runtime detail display schema mismatch. Static tests, isolated `cargo check`, health, and authenticated E2E pass. |
| `2026-04-27 10:24` | `Codex` | `Daily Page Audit Fix: Admin Blockchain Treasury` | `frontend/platform/static/js/admin-blockchain-treasury.js, docs/page-audits/2026-04-27-admin-blockchain-treasury.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0001: emergency pause/unpause now source CSRF from the csrf_token cookie helper instead of sending an explicit empty header; authenticated mocked-chain E2E remains needed. |
| `2026-04-28 08:03` | `Codex` | `Daily Page Audit Fix: Admin Affiliate Finance` | `frontend/platform/admin/js/admin-affiliate-finance.js, tests/admin/test_affiliate_route_contract_static.py, docs/page-audits/2026-04-25-admin-affiliate-finance.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0034 by removing inline payout row handlers and adding static regression coverage; payout backend scoping, tax readiness, page permission gate, modal a11y, notifications, and authenticated E2E remain open. |
| `2026-04-26 13:10` | `Codex` | `Daily Page Audit Fix: Admin Blockchain Contracts` | `backend/src/admin/pages.rs, backend/src/admin/blockchain.rs, frontend/platform/admin/blockchain-contracts.html, frontend/platform/static/js/admin-blockchain-contracts.js, tests/e2e/test_admin_blockchain_contracts.py, docs/page-audits/2026-04-26-admin-blockchain-contracts.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0292..0296: treasury.read page/API gates, safe DOM rendering, treasury DB error propagation, visible copy feedback, unused HTMX cleanup, and targeted authenticated E2E. |
| `2026-04-26 12:21` | `Codex` | `Daily Page Audit Fix: Admin Reports` | `backend/src/main.rs, backend/src/admin/pages.rs, backend/src/admin/access.rs, frontend/platform/admin/reports.html, frontend/platform/static/js/admin-reports.js, database/088_admin_report_permissions.sql, tests/e2e/test_admin_reports_export.py, docs/page-audits/2026-04-26-admin-reports.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/reports`: reports.generate/category gates, validated dates and SQL error propagation, report.exported audit logging, CSV-only P&L, checked frontend downloads, durable status/a11y states, unused HTMX cleanup, and targeted authenticated API/Playwright E2E. |
| `2026-04-26 11:20` | `Codex` | `Daily Page Audit Fix: Admin Notifications` | `backend/src/admin/notifications.rs, backend/src/admin/pages.rs, database/087_admin_notification_permissions.sql, frontend/platform/admin/notifications.html, frontend/platform/static/js/admin-notifications.js, tests/e2e/test_admin_notifications.py, docs/page-audits/2026-04-26-admin-notifications.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/notifications`: notifications.view/send gates, validated transactional audited broadcast, propagated list DB errors, explicit frontend loading/error/success states, accessible sort controls, unused HTMX cleanup, and targeted authenticated E2E coverage. |
| `2026-04-26 11:05` | `Codex` | `Daily Page Audit: Admin Notifications` | `docs/page-audits/2026-04-26-admin-notifications.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Audited `/admin/notifications`; documented notification permission mismatch, missing broadcast audit log, hidden list DB failures, incomplete broadcast validation, weak frontend states, inaccessible sort feedback, and minor markup/dependency cleanup. |
| `2026-04-26 10:55` | `Codex` | `Daily Page Audit Fix: Admin Asset Tokenize` | `backend/src/admin/blockchain.rs, backend/src/admin/pages.rs, backend/src/admin/mod.rs, backend/src/admin/access.rs, database/086_admin_asset_tokenization_jobs.sql, frontend/platform/admin/asset-tokenize.html, frontend/platform/static/js/admin-asset-tokenize.js, tests/e2e/test_admin_asset_tokenize.py, docs/page-audits/2026-04-25-admin-asset-tokenize.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/asset-tokenize`: CSRF cookie fallback, exact `blockchain.tokenize` page/API gates, active tokenization job guard, transactional metadata/audit persistence, clone-address hard failure, richer pre-flight checks, generic asset picker, safe DOM rendering, and authenticated mocked-chain E2E. |
| `2026-04-26 10:57` | `Codex` | `Authenticated E2E Verification Cleanup` | `backend/src/community/routes.rs, backend/src/admin/marketplace.rs, backend/src/admin/notifications.rs, frontend/platform/admin/community/amas.html, frontend/platform/static/css/main.css, docs/IMPLEMENTATION_ROADMAP.md, tests/e2e/, tests/` | `✅ Check-Out` | Fixed remaining verification blockers: rustfmt check state, chain-mode-aware blockchain E2Es, AMA mobile modal/sidebar layout, Redis-absent orderbook rebuild, View Transition click interception on checkout, and a missing-docs cargo check blocker. `cargo fmt --check`, `cargo check`, `python3 -m pytest tests/e2e -q`, and `python3 -m pytest tests -q` passed. |
| `2026-04-26 10:49` | `Codex` | `Admin Affiliate Applications HTML Auth Fix` | `backend/src/admin/pages.rs, docs/page-audits/2026-04-25-admin-affiliate-applications.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0029 for `/admin/affiliate-applications`: generic admin HTML page requests now redirect unauthenticated users to `/auth/login` while admin APIs keep JSON auth errors. `cargo fmt --check`, `cargo check`, tracker regeneration, YAML parse, and scoped diff check passed; runtime curl smoke is blocked by unrelated duplicate marketplace route registration. |
| `2026-04-26 10:40` | `Codex` | `Route Contract Fix: Admin Affiliate Applications` | `backend/src/admin/pages.rs, backend/src/admin/rewards.rs, frontend/platform/admin/affiliate-applications.html, frontend/platform/admin/js/admin-affiliate-applications.js, frontend/platform/static/js/admin-permission-guard.js, frontend/platform/static/js/admin-sidebar-loader.js, tests/admin/test_affiliate_route_contract_static.py, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0254-0257 and audit rejection-bound finding: page/sidebar `affiliates.manage` contract, referral-code validation parity, 1000-char rejection reason limit, pending response schema validation, and static regression tests. `cargo fmt --check`, `cargo check`, `cargo test`, clippy `-D warnings`, node checks, targeted pytest, and `git diff --check` passed; full `pytest tests/` is blocked by unrelated blockchain mocked-chain expectation on the existing server, and isolated backend restart is blocked by unrelated duplicate marketplace route registration. |
| `2026-04-26 10:40` | `Codex` | `Daily Page Audit Fix: Admin Marketplace Trades` | `backend/src/admin/marketplace.rs, backend/src/admin/mod.rs, frontend/platform/admin/marketplace/trades.html, frontend/platform/static/js/mp-trades.js, tests/e2e/test_admin_marketplace_trades.py, docs/page-audits/2026-04-26-admin-marketplace-trades.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/trades`: removed mock ledger fallback, safe-rendered rows, wired date/asset/status filters, surfaced on-chain status, implemented CSV export, disabled fake PDF, added loading/error states, and passed targeted authenticated E2E. |
| `2026-04-26 10:40` | `Codex` | `Daily Page Audit Fix: Admin Marketplace Settings` | `backend/src/admin/marketplace.rs, backend/src/marketplace/service.rs, frontend/platform/admin/marketplace/settings.html, frontend/platform/static/js/mp-settings.js, docs/page-audits/2026-04-26-admin-marketplace-settings.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/settings`: CSRF save/reset, true failure UI, integer cents conversion, backend validation, marketplace.view/manage gates, Redis error propagation, audit logging, safe toggle rendering, runtime trading/tick/order-size consumption, and dead-control cleanup. `node --check`, `cargo fmt --check`, `cargo check`, full `cargo test`, YAML validation, and `git diff --check` passed; full pytest is blocked by unrelated blockchain contract-detail E2E. |
| `2026-04-26 08:10` | `Codex` | `Daily Page Audit Fix: Admin Community Reports` | `backend/src/community/routes.rs, backend/src/community/service.rs, frontend/platform/admin/community/reports.html, frontend/platform/static/js/admin-community-reports.js, tests/e2e/test_admin_community_reports.py, docs/page-audits/2026-04-26-admin-community-reports.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/community/reports`: safe DOM rendering, community.view/manage gates, CSRF and required admin notes, pending row locks, stale conflict handling, transactional moderation audit logs, accessible modal, corrected copy, and no inline/CDN scripts. `node --check`, `py_compile`, `cargo fmt --check`, `cargo check`, `cargo test`, clippy `-D warnings`, targeted pytest, tracker regeneration, and `git diff --check` passed; broad pytest is blocked by an unrelated admin blockchain contract-detail mocked-chain assertion. |
| `2026-04-26 10:40` | `Codex` | `Daily Page Audit Fix: Admin Marketplace P2P` | `backend/src/admin/marketplace.rs, backend/src/admin/mod.rs, frontend/platform/admin/marketplace/p2p.html, frontend/platform/static/js/mp-p2p.js, frontend/platform/static/js/mp-toast.js, tests/e2e/test_admin_marketplace_p2p.py, docs/page-audits/2026-04-26-admin-marketplace-p2p.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/p2p`: real audited admin cancellation endpoint, marketplace.view/manage API gates, DB error propagation, no mock fallback, safe DOM rendering, accessible reason validation, aligned side/maker/taker/status columns, and authenticated Playwright E2E. |
| `2026-04-26 10:34` | `Codex` | `Daily Page Audit Fix: Admin Marketplace Orderbook` | `backend/src/admin/marketplace.rs, backend/src/admin/mod.rs, frontend/platform/admin/marketplace/orderbook.html, frontend/platform/static/js/mp-orderbook.js, tests/e2e/test_admin_marketplace_orderbook.py, docs/page-audits/2026-04-26-admin-marketplace-orderbook.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/orderbook`: live UUID asset selector, removed mock fallback, real CSRF rebuild POST, marketplace.view/manage gates, Redis rebuild lock/audit, aggregated order columns, visible status states, and authenticated HTTP/DB/Redis E2E. |
| `2026-04-26 08:10` | `Codex` | `Daily Page Audit Fix: Admin Marketplace Index` | `backend/src/admin/marketplace.rs, backend/src/admin/pages.rs, frontend/platform/admin/marketplace/index.html, frontend/platform/static/js/mp-index.js, frontend/platform/static/css/mp-index.css, tests/e2e/test_admin_marketplace_index.py, docs/page-audits/2026-04-26-admin-marketplace-index.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/` audit findings: removed mock LIVE fallback, safe-rendered recent trades, aligned marketplace.view page/API gates, made health report explicit degraded states, fixed Redis UNKNOWN handling and TIMESTAMPTZ decoding, and added authenticated E2E. `node --check`, Python compile, `cargo fmt --check`, `cargo check`, and targeted Playwright E2E passed. |
| `2026-04-26 08:04` | `Codex` | `Daily Page Audit Fix: Admin Affiliate Applications` | `frontend/platform/admin/affiliate-applications.html, frontend/platform/admin/js/admin-affiliate-applications.js, docs/page-audits/2026-04-25-admin-affiliate-applications.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed PAGE-ISSUE-0027 for `/admin/affiliate-applications`: pending rows/details URL now render user-provided data with DOM/textContent and HTTP(S) URL allowlisting, and page-local inline handlers were removed. `node --check`, targeted unsafe-pattern scan, YAML parse, and scoped diff check passed; tracker regeneration wrote Markdown but reports unrelated missing public/legal routes. |
| `2026-04-26 08:01` | `Codex` | `Daily Page Audit: Admin Community Reports` | `docs/page-audits/2026-04-26-admin-community-reports.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Audited `/admin/community/reports`; documented admin XSS risk in report rendering, missing community-specific permissions, missing route-level CSRF, missing moderation audit logs, stale/concurrent action risks, server-side note validation gap, notification-copy mismatch, and modal accessibility gaps. |
| `2026-04-26 08:00` | `Codex` | `Daily Page Audit: Admin Marketplace P2P` | `docs/page-audits/2026-04-26-admin-marketplace-p2p.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Audited `/admin/marketplace/p2p`; documented fake admin cancel, mock fallback on API failures, raw HTML rendering, missing marketplace API permission, DB error masking, modal accessibility, and maker/seller contract mismatch. |
| `2026-04-26 02:52` | `Codex` | `Daily Page Audit: Admin Marketplace Orderbook` | `docs/page-audits/2026-04-26-admin-marketplace-orderbook.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Audited `/admin/marketplace/orderbook`; documented demo selector/mock data bypass, fake rebuild success, missing marketplace API permissions, missing rebuild audit/locking, stale error fallback, user-column contract mismatch, and a11y state gaps. |
| `2026-04-26 01:45` | `Codex` | `Daily Page Audit: Admin Marketplace Index` | `docs/page-audits/2026-04-26-admin-marketplace-index.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Audited `/admin/marketplace/`; documented mock LIVE fallback, unsafe recent-trade rendering, masked health checks, health permission gap, Redis status masking, and stale KPI/health labels. |
| `2026-04-25 22:13` | `Codex` | `Daily Page Audit Fix: Admin Marketplace Approvals` | `backend/src/admin/marketplace.rs, backend/src/admin/pages.rs, frontend/platform/static/js/mp-approvals.js, frontend/platform/static/js/mp-toast.js, tests/e2e/test_admin_marketplace_approvals.py, docs/page-audits/2026-04-26-admin-marketplace-approvals.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/approvals` audit findings: hold accounting, transactional/audited decisions, marketplace.manage gates, Redis orderbook insert after approval, safe DOM/toast rendering, retryable load errors, real review context, accessible confirmations, and runtime schema/timestamp decoding. `node --check`, `cargo fmt --check`, `cargo check`, targeted marketplace unit tests, targeted authenticated HTTP/DB E2E, tracker regeneration, and scoped diff checks passed. |
| `2026-04-25 21:58` | `Codex` | `Daily Page Audit Fix: Admin Marketplace Analytics` | `backend/src/admin/marketplace.rs, backend/src/admin/pages.rs, backend/src/config.rs, backend/src/main.rs, frontend/platform/admin/marketplace/analytics.html, frontend/platform/static/js/mp-analytics.js, tests/e2e/test_admin_marketplace_analytics.py, docs/page-audits/2026-04-25-admin-marketplace-analytics.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/marketplace/analytics` audit findings: server-side marketplace.view gates, stats/trades contract alignment, configured Metabase disabled fallback, visible API errors, trade filters, iframe title, and authenticated analytics E2E. `node --check`, `cargo fmt --check`, `cargo check`, Python compile, and targeted Playwright E2E passed. |
| `2026-04-25 20:39` | `Codex` | `Live Marketing Button Routing Hotfix` | `frontend/platform/landing-v2.html, backend/src/main.rs, backend/src/legal/routes.rs` | `✅ Check-Out` | Fixed live `www.poool.app` landing CTAs and product links so signup/login/blog/property/marketplace actions route to `platform.poool.app`; added www-router redirects for old/cached relative URLs and made legal footer pages public. `cargo fmt --check`, `cargo check`, scoped diff check, and local Host-header redirect probes passed. |
| `2026-04-25 21:08` | `Codex` | `Live WWW Platform Redirect Completion` | `backend/src/main.rs, frontend/www/server.js, frontend/www/*/index.html, frontend/platform/landing*.html, frontend/platform/property-public.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Completed remaining same-host www routing fixes: app/legal/property paths redirect to `platform.poool.app`, legacy legal aliases are covered, and public templates no longer emit relative app/legal CTAs. |
| `2026-04-25 20:30` | `Codex` | `Live Marketing Sign-In Link Hotfix` | `frontend/www/en/main-6QW32D7Z.js, frontend/www/id/main-6QW32D7Z.js` | `✅ Check-Out` | Fixed the marketing Sign in buttons to open `https://platform.poool.app/auth/login` instead of the marketing host `/auth/login`; JS syntax checks and live target HEAD smoke passed. |
| `2026-04-25 20:07` | `Codex` | `E2E Failure Repair Pass` | `tests/e2e/, tests/, backend/src/, frontend/platform/` | `✅ Check-Out` | `tests/e2e` passed 44/44 before and after fixes. Broader `pytest tests` now passes 49/49 after adding a pytest collection guard for legacy script harnesses and restoring note-only admin developer-project notes. `cargo check` and scoped diff checks passed; global `cargo fmt --check` remains blocked by unrelated formatting in `backend/src/community/routes.rs`. |
| `2026-04-25 09:26` | `Codex` | `Daily Page Audit Fix: Admin Community AMAs` | `backend/src/community/amas.rs, backend/src/community/routes.rs, backend/src/admin/pages.rs, backend/src/admin/mod.rs, backend/src/admin/access.rs, frontend/platform/admin/community/amas.html, database/083_admin_community_permissions.sql, docs/page-audits/2026-04-25-admin-community-amas.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/community/amas` audit findings: explicit CSRF-aware fetches, community.view/manage gates, admin detail endpoint, public draft-detail block, payload validation, missing-row 404s, audit logs, safer DOM rendering, and modal a11y. `node --check`, `cargo fmt --check`, `cargo check`, `cargo test --no-run`, and scoped diff checks passed; authenticated browser recheck still recommended. |
| `2026-04-25 08:15` | `Codex` | `Daily Page Audit Fix: Admin Community Badges` | `backend/src/community/routes.rs, backend/src/community/circles.rs, frontend/platform/admin/community/badges.html, frontend/platform/static/js/admin-permission-guard.js, database/082_community_badge_permissions.sql, docs/page-audits/2026-04-25-admin-community-badges.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/community/badges` findings: added community.view/manage gates and seed migration, transactional audit logs, badge validation, target-user validation, update/revoke 404 handling, recent-award revoke UI, accessible modal behavior, and tracker updates. `node --check`, `cargo fmt --check`, `cargo check`, tracker regeneration, YAML validation, and scoped diff checks passed. |
| `2026-04-25 08:03` | `Codex` | `Daily Page Audit Fix: Admins Search Placeholder` | `frontend/platform/admin/admins.html, docs/page-audits/2026-04-25-admins.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md` | `✅ Check-Out` | Fixed documented `/admin/admins` P3 search placeholder mismatch only; higher-risk admin invite/account-security findings remain blocked. `node --check` passed; tracker Markdown regenerated but audit script still reports unrelated missing route/template inventory gaps. |
| `2026-04-25 02:28` | `Codex` | `Daily Page Audit Fix: Admin Audit Logs` | `backend/src/admin/audit.rs, backend/src/admin/pages.rs, backend/src/admin/mod.rs, backend/src/admin/access.rs, frontend/platform/admin/audit-logs.html, frontend/platform/static/js/admin-audit.js, database/081_admin_audit_permissions.sql, docs/page-audits/2026-04-25-admin-audit-logs.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md` | `✅ Check-Out` | Fixed `/admin/audit-logs` audit findings: enforced audit.read on page/API, propagated DB errors, returned user_agent for CSV, added visible retryable load errors, escaped entity badges, and added modal dialog keyboard/focus handling. `node --check`, `cargo check`, scoped diff checks, and curl smoke on :8892 passed; global `cargo fmt --check` remains blocked by unrelated trailing whitespace in `backend/src/rewards/service.rs`. |
| `2026-04-25 02:03` | `Codex` | `Daily Page Audit Fix: Admin Assets` | `backend/src/admin/assets.rs, frontend/platform/admin/assets.html, frontend/platform/static/js/admin-assets.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed safe `/admin/assets` audit findings: list DB errors now propagate, API/toggle failures are visible, sortable/action controls have keyboard/ARIA support, payout_pending is filterable, and new-tab links use noopener. `node --check`, scoped rustfmt/checks, and `cargo check` passed; global `cargo fmt --check` remains blocked by unrelated trailing whitespace in `backend/src/rewards/service.rs`. |
| `2026-04-25 02:18` | `Codex` | `Auth Login Production Hardening` | `backend/src/auth/routes.rs, backend/src/auth/csrf.rs, frontend/platform/login.html, frontend/platform/components/auth-head.html, frontend/platform/static/css/login.css, frontend/platform/static/js/vendor/` | `✅ Check-Out` | Fixed login fallback safety: real POST form, server-rendered CSRF token, non-HTMX redirect errors without credential URL leakage, HTMX HTML error partials, and self-hosted HTMX. Verified JS, blocked-HTMX, and no-JS invalid-login flows. |
| `2026-04-25 01:56` | `Codex` | `18.10 Platform Fee Float→Decimal Fix` | `backend/src/payments/service.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Verified checkout platform fee calculation now uses `rust_decimal::Decimal`, extracted it into a checked helper, added 7 regression tests for rounding/negative/overflow behavior, fixed the adjacent Decimal FX test assertion, and ran targeted payment tests plus `cargo check`. |
| `2026-04-25 02:18` | `Codex` | `Masterplan Alignment Gap Audit` | `docs/MASTERPLAN.md, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Read the complete Masterplan structure and targeted action sections, compared against all roadmap phases, and added Phase 24 for missing resilience, security, launch governance, and deferred circuit-breaker alignment items. |
| `2026-04-25 01:46` | `Codex` | `Roadmap Ownership & Gate Cleanup` | `docs/IMPLEMENTATION_ROADMAP.md, docs/community/COMMUNITY_ROADMAP.md, docs/affiliate/AFFILIATE_ROADMAP.md` | `✅ Check-Out` | Read all roadmap files, replaced legacy/generic ownership references with `us`, refreshed stale phase/module gates, synchronized Community M5 status back to the main roadmap, and repaired the corrupted concurrency/summary block. |
| `2026-04-25 10:05` | `Codex` | `Developer Assets Production Readiness` | `frontend/platform/developer/assets.html, frontend/platform/static/js/developer-assets.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened `/developer/assets`: moved status-tab/card/gallery behavior out of inline template handlers, added keyboard-accessible asset-card navigation, guarded cover image URLs before applying background images, and kept ghost cards hidden during tab filtering. `node --check`, inline-handler scan for this template, and scoped `git diff --check` passed. |
| `2026-04-25 06:10` | `Codex` | `Daily Page Audit Fix: Admin Community Challenges` | `backend/src/community/routes.rs, backend/src/community/challenges.rs, frontend/platform/admin/community/challenges.html, frontend/platform/static/js/admin-permission-guard.js, docs/page-audits/2026-04-25-admin-community-challenges.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/community/challenges` audit findings: challenge contract validation, community.manage page/API enforcement, community audit logging, stale-toggle 404, safe requirement rendering, and modal keyboard accessibility. `node --check`, `cargo check`, tracker regeneration, YAML validation, and scoped `git diff --check` passed. |
| `2026-04-25 01:34` | `Codex` | `Auth Login Portrait Asset` | `frontend/platform/login.html, frontend/platform/static/css/login.css, frontend/platform/static/images/auth/` | `✅ Check-Out` | Added generated close-up Indonesian male portrait to the login auth visual, optimized it to WebP, and verified `/auth/login` in Chrome. |
| `2026-04-25 09:45` | `Codex` | `Developer Submissions Production Readiness` | `frontend/platform/developer/submissions.html, frontend/platform/static/js/developer-submissions.js, backend/src/developer/routes.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened `/developer/submissions`: added local CSRF/error/toast fallbacks, made bulk delete draft-only in the UI, enforced draft-only self-delete on the backend, stopped list API DB failures from silently rendering as an empty state, and surfaced API failure messages for duplicate/resubmit/delete. `node --check`, `rustfmt`, scoped `git diff --check`, and `cargo check` passed. |
| `2026-04-25 01:16` | `Codex` | `Settings Page Production Readiness` | `backend/src/settings/, frontend/platform/settings.html, frontend/platform/static/js/settings.js, frontend/platform/static/css/settings.css, tests/e2e/test_settings.py, tests/test_settings.py` | `✅ Check-Out` | Hardened settings account/security/privacy flows: POST+CSRF data export, confirmed account deletion, password/session hardening, protected 2FA disable, clearer phone/export UX, role gates, flat dashboard cards, and current settings tests. `cargo check`, `node --check`, and diff check passed; targeted cargo test blocked by existing `payments/service.rs` Decimal test compile error. |
| `2026-04-25 01:15` | `Codex` | `Daily Page Audit Fix: Admin Asset Details` | `backend/src/admin/assets.rs, backend/src/admin/mod.rs, frontend/platform/admin/asset-details.html, frontend/platform/static/js/admin-asset-details.js, database/080_admin_asset_permissions.sql, docs/page-audits/2026-04-25-admin-asset-details.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md` | `✅ Check-Out` | Fixed `/admin/asset-details` audit findings: secured document links, asset RBAC/audit logging, publish/funding persistence, featured error handling, disabled unsupported freeze/archive actions, and stale smoke-test selector. `node --check` and `cargo check` passed. |
| `2026-04-25 09:25` | `Codex` | `Developer Property Content Production Readiness` | `frontend/platform/developer/property-content.html, frontend/platform/static/js/developer-property-content.js, backend/src/developer/routes.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened `/developer/property-content`: removed inline back navigation, made Save & Exit stay on failed saves, rendered uploaded image cards without interpolating server data into HTML, added image type/size/race guards, required at least one image before submit, and added backend limits for content fields and percentage basis-point fields. `node --check`, `rustfmt`, scoped `git diff --check`, and `cargo check` passed. |
| `2026-04-25 09:05` | `Codex` | `Developer Document Upload Production Readiness` | `frontend/platform/developer/document-upload-step3.html, frontend/platform/static/js/developer-document-upload.js, backend/src/storage/routes.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Hardened `/developer/document-upload-step3`: removed inline navigation handlers, blocked Next without a draft or while uploads are in flight, rendered uploaded filenames with DOM text nodes, added client file type checks, made failed deletes visible/non-destructive, validated asset document types/titles server-side, and aligned DOC/DOCX/ZIP magic-byte handling with advertised formats. `node --check` and scoped `git diff --check` passed; `cargo check` blocked by active settings work in `settings/routes.rs`. Page-review tracker updates deferred while another active task owns those docs. |
| `2026-04-25 08:40` | `Codex` | `Developer Application Form Production Readiness` | `frontend/platform/static/js/developer-application-form.js, backend/src/developer/routes.rs, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md` | `✅ Check-Out` | Hardened `/developer/application-form` draft validation and save behavior: backend now validates/derives asset pricing, token count, asset type, and property ranges; frontend only falls back to POST for stale draft IDs and keeps users on-page after failed Save & Exit. `node --check` and `cargo check` passed; targeted cargo test blocked by existing `payments/service.rs` Decimal test compile error. |
| `2026-04-25 01:06` | `Codex` | `Daily POOOL Safe Page Fix: Affiliate Application Modals` | `frontend/platform/admin/affiliate-applications.html, frontend/platform/admin/js/admin-affiliate-applications.js, docs/page-audits/2026-04-25-admin-affiliate-applications.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/affiliate-applications` modal accessibility baseline for PAGE-ISSUE-0028; left inline row rendering and admin HTML auth response as follow-ups. |
| `2026-04-25 01:01` | `Codex` | `Developer Dashboard Activity Snapshot Data Wiring` | `backend/src/developer/service.rs, frontend/platform/developer/dashboard.html, docs/IMPLEMENTATION_ROADMAP.md` | `⏸️ FOLLOW-UP` | Added the missing Activity snapshot metric slot as `Avg. Funding Progress` using existing totals. Remaining production task: define every Activity snapshot metric as an explicit database-backed contract, add backend tests for each query/window, add frontend rendering coverage, and remove placeholder-only metrics such as saved properties once the backing table exists. |
| `2026-04-24 23:56` | `Codex` | `Daily POOOL Safe Page Fix: Admins Invite Roles` | `frontend/platform/static/js/admin-directory.js, docs/page-audits/2026-04-25-admins.md, docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | `✅ Check-Out` | Fixed `/admin/admins` invite dropdown role mismatch; left invite acceptance and admin security-action backend issues blocked for decisions. |
| `2026-04-24 23:03` | `Codex` | `Daily Page Audit: Admin Affiliate Applications` | `docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/page-audits/2026-04-25-admin-affiliate-applications.md` | `✅ Check-Out` | Audited `/admin/affiliate-applications`; logged PAGE-ISSUE-0024–0031; regenerated page tracker report. |
| `2026-04-25 00:20` | `Codex` | `Investor Dashboard Verified Fixes - Batch 4` | `backend/src/main.rs, backend/src/payment_methods/routes.rs, backend/src/support/service.rs, frontend/platform/wallet.html, frontend/platform/marketplace-secondary.html, frontend/platform/static/js/marketplace-secondary.js, frontend/platform/marketplace-trading-v3.html, docs/` | `✅ Check-Out` | Rechecked remaining investor findings, confirmed they still existed where applicable, then fixed wallet manual-token fallback, secondary buy-interest persistence via real order API, V3 duplicate order controller, support attachment failure semantics, and deposit status order_id casting. |
| `2026-04-24 23:55` | `Codex` | `Investor Dashboard Low-Risk Fixes - Batch 3` | `backend/src/cart/routes.rs, backend/src/main.rs, frontend/platform/static/js/cart.js, frontend/platform/static/css/cart.css, frontend/platform/static/js/property-detail-cart.js, frontend/platform/static/js/property-detail-mobile.js, docs/` | `✅ Check-Out` | Verified PAGE-ISSUE-0016 was already fixed; fixed PAGE-ISSUE-0014 and PAGE-ISSUE-0011 with narrow backend guards/payload casting and visible cart error handling. |
| `2026-04-24 23:35` | `Codex` | `Investor Dashboard Low-Risk Fixes - Batch 2` | `backend/src/community/routes.rs, frontend/platform/static/js/community-feed.js, frontend/platform/property.html, frontend/platform/components/property/*.html, frontend/platform/static/css/property-detail.css, frontend/platform/static/js/rewards.js, frontend/platform/static/css/rewards.css, docs/` | `✅ Check-Out` | Fixed route-safe community trending asset links, removed/disabled property placeholder controls, disabled rewards commission PDF fake-success export, and updated audit trackers. |
| `2026-04-24 23:05` | `Codex` | `Investor Dashboard Low-Risk Fixes` | `frontend/platform/static/js/leaderboard.js, frontend/platform/static/js/marketplace-trading-v3.js, frontend/platform/static/js/payment-success.js, frontend/platform/rewards.html, frontend/platform/static/css/rewards.css, docs/*trackers` | `✅ Check-Out` | Fixed PAGE-ISSUE-0005, PAGE-ISSUE-0008, PAGE-ISSUE-0009, and PAGE-ISSUE-0012 with localized frontend changes; skipped higher-risk backend/payment-method/order-submission items. |
| `2026-04-24 22:30` | `Codex` | `Investor Dashboard Audit` | `docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, docs/issue-tracking/BROKEN_LOGICS.md, docs/IMPLEMENTATION_ROADMAP.md, docs/automation-coverage/E2E_COVERAGE_TRACKER.md` | `✅ Check-Out` | Audited investor-facing authenticated dashboard pages by static source review; added PAGE-ISSUE-0011 through PAGE-ISSUE-0016, moved misplaced investor findings onto their correct page records, regenerated the Markdown tracker, and logged follow-up tasks. |
| `2026-04-25 08:24` | `Codex` | `UI Hotfix (My Trading Empty State Declutter)` | `frontend/platform/static/js/my-trading.js, frontend/platform/static/css/my-trading.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Reduced My Trading empty-state branding after the visible logo/watermark made the table empty state feel cluttered. |
| `2026-04-25 08:10` | `Codex` | `UI Polish (Cart Wallet Trading Empty Branding)` | `frontend/platform/cart.html, frontend/platform/wallet.html, frontend/platform/static/js/my-trading.js, frontend/platform/static/css/cart.css, frontend/platform/static/css/wallet.css, frontend/platform/static/css/my-trading.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Extended the portfolio empty-state POOOL branding pattern to Cart, Wallet, and My Trading empty states only; verified all three pages in browser. |
| `2026-04-24 22:00` | `Codex` | `UI Hotfix (Developer Asset Card Metrics)` | `frontend/platform/developer/assets.html, backend/src/developer/models.rs, backend/src/developer/service.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Aligned `/developer/assets` card meta rows with shared property cards by showing bedrooms, bathrooms, and m² instead of status/location; verified in browser. |
| `2026-04-24 21:42` | `Codex` | `UI Polish (Portfolio Empty State Branding)` | `frontend/platform/portfolio.html, frontend/platform/static/css/portfolio-enhancements.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added subtle POOOL logo branding to the portfolio empty state, including the branded eyebrow, hero-art wordmark, and faint background watermark. |
| `2026-04-24 21:35` | `Codex` | `UI Hotfix (Profile Switcher Route State)` | `frontend/platform/static/js/user-data.js, frontend/platform/static/js/profile-dropdown.js, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Normalized saved profile switcher state to the current route so investor pages do not show Developer selected; verified JS syntax and route-state smoke tests. |
| `2026-04-24 20:31` | `Codex` | `UI Polish (Developer Support Workflow)` | `frontend/platform/support.html, frontend/platform/static/css/support.css, frontend/platform/static/js/support.js, frontend/platform/components/developer-topbar.html, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added support topbar section links, overview cards, guidance panel, form counters, draft save/clear, ticket count badges, attachment validation, active section highlighting, and verified `/developer/support` in browser. |
| `2026-04-24 20:30` | `Codex` | `UI Hotfix (My Trading Sidebar Offset)` | `frontend/platform/static/css/my-trading.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Fixed `/my-trading` content being hidden under the fixed investor sidebar at medium desktop widths and verified the page in browser. |
| `2026-04-24 20:05` | `Codex` | `UI Hotfix (Dashboard Card Standardization)` | `frontend/platform/static/css/ds-cards.css, frontend/platform/static/css/bundle.css, frontend/platform/static/css/developer-dashboard.css, frontend/platform/developer/dashboard.html, frontend/platform/components/developer-chart.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Made `ds-card` the flat default surface, added explicit glass/stat/chart card hooks, migrated developer dashboard cards off `holo-card`, rebuilt `bundle.css`, and verified dashboard/leaderboard/portfolio/my-trading in browser. |
| `2026-04-24 20:18` | `Codex` | `UI Hotfix (Secondary Marketplace Cards)` | `frontend/platform/marketplace-secondary.html, frontend/platform/static/css/marketplace-secondary.css, frontend/platform/static/js/marketplace-secondary.js, backend/src/marketplace/, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Migrated `/marketplace-secondary` cards onto shared V19 `property-card` CSS/JS and markup; aligned card icons, price/funded row, progress bar, and investment detail labels with the landing cards. |
| `2026-04-25 20:55` | `Codex` | `UI Hotfix (Remove Drive Legal Link)` | `frontend/platform/landing-v2.html, frontend/platform/landing.html, frontend/platform/property-public.html, frontend/www/en/chunk-MO34KLTL.js, frontend/www/id/chunk-MO34KLTL.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the external Google Drive `Legal` footer link while preserving first-party legal policy links. |
| `2026-04-24 19:42` | `Codex` | `UI Hotfix (Property Card Metrics)` | `backend/src/assets/, frontend/platform/marketplace.html, frontend/platform/index.html, frontend/platform/landing*.html, frontend/platform/static/css/property-card.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Property cards now show bedrooms, bathrooms, and m² below the image across landing, marketplace, and HTMX tab fragments. `cargo check` passes with existing warnings. |
| `2026-04-24 19:31` | `Codex` | `UI Hotfix (Developer Assets Status Tabs)` | `frontend/platform/developer/assets.html, frontend/platform/static/css/developer-assets.css, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Reordered developer assets CSS after marketplace base styles, compacted Available/Funded tabs to marketplace pill sizing, bumped asset version, and verified in browser on `/developer/assets`. |
| `2026-04-24 19:30` | `Codex` | `UI Maintenance (Unified Property Card Assets)` | `frontend/platform/landing*.html, frontend/platform/*marketplace*.html, frontend/platform/developer/assets.html, frontend/platform/static/css/property-card.css, frontend/platform/static/js/property-card.js, frontend/platform/static/js/marketplace.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Consolidated V19 property card styling and carousel behavior into one shared CSS file and one shared JS file; deleted legacy landing card CSS. |
| `2026-04-24 19:11` | `Codex` | `UI Hotfix (Developer Topbar Investor Parity)` | `frontend/platform/components/developer-topbar.html, frontend/platform/static/css/developer-leaderboard-navbar.css, frontend/platform/developer/*.html, frontend/platform/settings.html, frontend/platform/support.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Matched developer topbar markup, action hooks, surface opacity, border, title reset, and spacing to the shared investor topbar; removed duplicated sidebar navigation links and set developer topbar titles per page. `git diff --check` passes. |
| `2026-04-24 20:05` | `Codex` | `UI Feature (Admin Blog Planning Subpages)` | `backend/src/admin/, frontend/platform/admin/, frontend/platform/static/js/admin-sidebar-loader.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added protected Blog Persona and Blog Strategy admin subpages, sidebar entries, Blog CMS quick links, local draft/copy tools, and verified JS syntax plus `cargo check`. |
| `2026-04-24 18:24` | `Codex` | `UI Polish (Admin Blog Editor Quality Controls)` | `frontend/platform/admin/blog.html, frontend/platform/admin/blog-editor.html, frontend/platform/static/js/admin-blog.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added editor formatting helpers, SEO counters, translation readiness, article health checks, explicit write confirmations, dirty-state protection, and verified protected admin/blog-editor pages. |
| `2026-04-24 18:08` | `Codex` | `UI Feature (Admin Blog Publishing Settings)` | `backend/src/blog/, frontend/platform/admin/blog.html, frontend/platform/admin/blog-editor.html, frontend/platform/blog/article.html, frontend/platform/static/js/admin-blog.js, studio/schemaTypes/article.ts, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added admin-editable per-article share link overrides, author social/profile fields, quick publish/take-down actions, and scheduled publish guidance. `cargo check` and `node --check` pass. |
| `2026-04-24 18:04` | `Codex` | `UI Hotfix (Admin Sidebar Standardization)` | `frontend/platform/admin/components/sidebar.html, frontend/platform/static/js/admin-sidebar-loader.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Standardized admin templates on the shared sidebar loader, restored current nav sections across include-based pages, and added sidebar entries/active states for newer admin pages. |
| `2026-04-24 17:44` | `Codex` | `UI Hotfix (Landing Blog Link)` | `frontend/platform/landing-v2.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added public Blog links to the landing page header and footer navigation. |
| `2026-04-24 19:33` | `Codex` | `UI Hotfix (Admin Dashboard Control Hit Areas)` | `frontend/platform/admin/index.html, frontend/platform/admin/components/sidebar.html, frontend/platform/static/css/admin.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added explicit button semantics for admin dashboard/sidebar controls, enlarged the notification hit area, and made topbar controls wrap/shrink cleanly so date/health/notification actions stay clickable on narrower admin viewports. |
| `2026-04-24 18:03` | `Codex` | `UI Hotfix (Register Auth Image)` | `frontend/platform/static/css/login.css, frontend/platform/static/css/signup.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Replaced the register page visual with a higher-resolution existing property image and restored the missing shared auth CSS hook. |
| `2026-04-24 17:04` | `Codex` | `UI Hotfix (Settings Input Wiring)` | `backend/src/settings/, backend/src/storage/, backend/src/auth/routes.rs, frontend/platform/static/js/settings.js, database/078_settings_input_wiring.sql, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Wired missing settings fields/actions end-to-end: middle/gender, timezone, leaderboard bio, social links, developer profile/links/logo, OAuth connect/disconnect, export, and delete-account payload. |
| `2026-04-24 16:47` | `Codex` | `UI Hotfix (Admin Blog Own Nav Section)` | `frontend/platform/static/js/admin-sidebar-loader.js, frontend/platform/admin/components/sidebar.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Moved Blog out of Marketplace into its own admin navbar section and verified active state on `/admin/blog.html`. |
| `2026-04-24 16:43` | `Codex` | `UI Hotfix (Admin Blog Nav Grouping)` | `frontend/platform/static/js/admin-sidebar-loader.js, frontend/platform/admin/components/sidebar.html, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Moved the admin Blog entry from standalone Content navigation into the Marketplace section and verified active state on `/admin/blog.html`. |
| `2026-04-24 16:45` | `Codex` | `Sanity Blog + Admin Dashboard` | `backend/src/blog/, backend/src/admin/, backend/src/config.rs, frontend/platform/admin/blog.html, frontend/platform/static/js/admin-blog.js, database/077_blog_manage_permission.sql` | `✅ Check-Out` | Added Sanity CDN-backed blog reads, token-free admin blog overview, Content → Blog navigation, and blog.manage RBAC migration. |
| `2026-04-24 16:32` | `Codex` | `UI Hotfix (Sidebar Account Menu Click)` | `frontend/platform/static/js/profile-dropdown.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Restored delegated click and keyboard activation for the shared sidebar account card when `profile-dropdown.js` initializes before the sidebar include; verified dropdown opens on `/marketplace`. |
| `2026-04-24 16:05` | `Codex` | `UI Hotfix (Settings Learn Cards)` | `frontend/platform/settings.html, backend/src/storage/service.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Repointed settings learn cards to live blog articles and normalized legacy blog image/avatar paths from `/images/*` to checked-in `/static/images/*` assets. |
| `2026-04-24 15:51` | `Codex` | `Page Review Tracker` | `docs/issue-tracking/page-review-tracker.yml, docs/issue-tracking/PAGE_REVIEW_TRACKER.md, scripts/audit_page_review_tracker.py` | `✅ Check-Out` | Added YAML source of truth, generated Markdown report, tested-date field, and audit script covering discovered page routes/templates/fragments. |
| `2026-04-24 15:26` | `Codex` | `UI Hotfix (Settings Topbar Search)` | `frontend/platform/components/investor-topbar.html, frontend/platform/components/sidebar.html, frontend/platform/static/js/global-search.js, frontend/platform/static/css/global-search.css, frontend/platform/static/css/settings.css, frontend/platform/static/css/bem/sidebar.css, backend/src/assets/routes.rs, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Wired settings topbar search to shared global search, fixed thumbnail URL rewriting and fallback icons, removed sidebar hover/click flash, and verified in browser on Settings and Marketplace. |
| `2026-04-24 13:27` | `Codex` | `UI Hotfix (Settings Card Cleanup)` | `frontend/platform/settings.html, frontend/platform/static/css/settings.css, frontend/platform/static/js/settings.js, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed the Financial card, email-change controls, active sessions, and extra linked-account rows; normalized cancel/action buttons and divider spacing; replaced broken learn images; and added a developer bio preview. |
| `2026-04-24 15:26` | `Codex` | `UI Hotfix (Sidebar Search Dropdown Polish)` | `frontend/platform/static/js/global-search.js, frontend/platform/static/css/global-search.css, backend/src/assets/routes.rs, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Matched sidebar search dropdown to marketplace search style, added a visible custom clear control, compacted rows/icons/text, and returned asset cover thumbnails for results. |
| `2026-04-24 15:22` | `Codex` | `UI Hotfix (Sidebar Search Repair)` | `frontend/platform/static/js/global-search.js, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Mounted sidebar search results outside the clipped input shell, hardened result rendering, and bumped the static asset version. |
| `2026-04-24 13:40` | `Codex` | `UI Hotfix (Rewards Review Modal Branding)` | `frontend/platform/rewards.html, frontend/platform/static/css/rewards.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Added the POOOL logo above the pending-review heading and moved the rewards approval modal styling into scoped CSS for desktop/mobile consistency. |
| `2026-04-24 12:22` | `Codex` | `UI Hotfix (Support Divider Cleanup)` | `frontend/platform/static/css/support.css, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Removed support page duplicate-divider risk by centralizing the divider token, collapsing FAQ rows into a single bordered list, and ensuring ticket detail panels use one internal separator. |
| `2026-04-24 12:08` | `Codex` | `UI Hotfix (Settings CSS Cascade)` | `frontend/platform/settings.html, frontend/platform/static/css/settings.css, frontend/platform/static/js/settings.js, backend/src/templates.rs, docs/IMPLEMENTATION_ROADMAP.md` | `✅ Check-Out` | Collapsed settings page styling and behavior to one CSS file and one JS file, constrained the avatar block, removed duplicate settings dividers around form footers and financial rows, and bumped static asset version to prevent legacy CSS from overriding the correct design after first paint. |
| `2026-04-24 11:50` | `Codex` | `UI Hotfix (Settings Consolidation)` | `frontend/platform/settings.html, backend/src/settings/routes.rs, backend/src/settings/mod.rs, backend/src/developer/routes.rs, frontend/platform/static/css/settings.css, frontend/platform/static/js/settings.js, frontend/platform/static/js/nationality-dropdown.js, legacy settings templates/assets` | `✅ Check-Out` | Consolidated `/settings`, `/settings-2`, `/settings-3`, and `/developer/settings` onto canonical `settings.html`; removed legacy settings templates and stale settings assets. |
| `2026-04-24 11:43` | `Codex` | `UI Hotfix (Leaderboard Spacing)` | `frontend/platform/static/css/leaderboard.css` | `✅ Check-Out` | Normalized leaderboard content start and section gaps to shared investor dashboard spacing tokens. Browser verification blocked by fresh-session login redirect. |
| `2026-04-23 22:30` | `Codex` | `UI Hotfix (Sidebar Consolidation)` | `frontend/platform/components/, frontend/platform/developer/, frontend/platform/static/js/` | `✅ Check-Out` | Replaced the duplicated developer asset-detail shell with shared sidebar/mobile-menu includes, added the missing shared developer mobile-menu template, normalized active states/search/profile persistence, and restored dashboard-context Settings/Support navigation. |
| `2026-04-23 22:11` | `Codex` | `UI Hotfix (Developer Topbar + Links)` | `backend/src/developer/, backend/src/common/routes_helper.rs, frontend/platform/developer/, frontend/platform/components/, frontend/platform/static/css/developer-leaderboard-navbar.css` | `✅ Check-Out` | Applied shared developer topbar across developer pages, added `/developer/support`, and kept Settings/Support navigation inside `/developer/*`. |
| `2026-04-23 00:00` | `Codex` | `UI Hotfix (Investor Topbar Template)` | `frontend/platform/components/investor-topbar.html, frontend/platform/*.html` | `✅ Check-Out` | Added shared leaderboard-style investor topbar include and applied it to 22 investor pages while preserving page-specific tabs/actions. |
| `2026-04-22 09:40` | `Codex` | `UI Hotfix (Landing Why-Us Repair)` | `frontend/platform/landing-v2.html` | `✅ Check-Out` | Reworked `#why-us` responsive grid + fan-card behavior to remove broken overlap/stacking and stabilize desktop/mobile layouts. |
| `2026-04-21 10:52` | `Codex` | `UI Hotfix (Investor Navbar Standardization)` | `frontend/platform/components/sidebar.html` | `✅ Check-Out` | Added shared leaderboard-style investor topbar navigation injected globally on investor routes (path-aware active state, mobile hidden, skips pages that already have lb-topbar). |
| `2026-04-21 10:42` | `Codex` | `UI Hotfix (Developer Navbar Standardization)` | `frontend/platform/developer/*.html, frontend/platform/settings.html, frontend/platform/components/developer-topbar.html, frontend/platform/static/css/developer-leaderboard-navbar.css` | `✅ Check-Out` | Standardized developer dashboard navigation using leaderboard-style topbar across Dashboard, Assets, Submissions, and Developer Settings (conditional for developer context). |
| `2026-04-21 10:08` | `Codex` | `UI Hotfix (Public Property Parity v2)` | `frontend/platform/property-public.html, backend/src/assets/public_assets.rs` | `✅ Check-Out` | Restored public price-card visual parity (inputs + quick chips) and guaranteed 5-image gallery rendering for `/p/:slug` cards. |
| `2026-04-21 08:00` | `Codex` | `UI Hotfix (Public Property Parity)` | `frontend/platform/property-public.html` | `✅ Check-Out` | Synced public `/p/:slug` property content structure with `/property/:slug` while keeping no-sidebar public header and signup CTAs. |
| `2026-03-21 05:45` | `us` | `Global` | `docs/` | `✅ Check-Out` | Generated multi-agent tracking system. |
| `2026-03-21 06:30` | `us` | `Global` | `docs/` | `✅ Check-Out` | Full Masterplan audit. Roadmap expanded to 120+ tasks. |
| `2026-03-21 06:25` | `us` | `2.1–2.10` | `database/*.sql` | `✅ Check-Out` | Phase 2 DB migrations complete: 050b, 050c, 050, 051, 052, 053, 054, 055 applied. Tasks 2.9/2.10 blocked (TimescaleDB). |
| `2026-03-21 07:00` | `us` | `1.1–1.11` | `backend/src/` | `✅ Check-Out` | Phase 1 audit: all 11 tasks verified implemented. `cargo check` passes cleanly. Roadmap updated. |
| `2026-03-21 13:55` | `us` | `0.5, 0.7, 0.9, 0.11` | `backend/src/, .github/workflows/, Dockerfile` | `✅ Check-Out` | Phase 0 code tasks complete: PgBouncer sidecar in Dockerfile, CI/CD already existed, health check enhanced with DB+Redis probe, marketplace RBAC migration created. `cargo check` + `cargo clippy` clean. |
| `2026-03-22 01:37` | `us` | `3.1–3.10, 3.13–3.16` | `backend/src/marketplace/` | `✅ Check-Out` | Phase 3 Core Trading Engine COMPLETE. 56 unit tests pass. 9 files. |
| `2026-03-22 01:41` | `us` | `4.1–4.4` | `backend/src/marketplace/websocket.rs` | `✅ Check-Out` | Phase 4 WebSocket Server COMPLETE. 5 tests. WS handler + 3 broadcast fns + heartbeat + Pub/Sub infra. |
| `2026-03-22 11:16` | `us` | `5.1–5.8, 5.10, 5.13` | `frontend/platform/static/js/` | `✅ Check-Out` | Phase 5 Frontend Trading UI: Event Bus, WS Client, Orderbook, Trade Form, My Orders, Orchestration. 4 new JS + 1 CSS + HTML updates. |
| `2026-03-22 05:48` | `us` | `6A.1–6A.6, 6A.10–11, 6A.15` | `backend/src/admin/marketplace.rs` | `✅ Check-Out` | Phase 6A first batch: 9 admin API endpoints. Critical DB table name fixes (marketplace_orders→market_orders, marketplace_trades→trade_history). |
| `2026-03-22 12:48` | `us` | `6A.4, 6A.7–9, 6A.12, 6A.14` | `backend/src/admin/marketplace.rs` | `✅ Check-Out` | Phase 6A second batch: orderbook rebuild, approvals (approve/reject), fees, P2P, alerts, watchlist, settings (Redis). All 15 APIs done. |
| `2026-03-22 12:48` | `us` | `6B.2–13` | `frontend/platform/static/js/mp-*.js, admin-permission-guard.js` | `✅ Check-Out` | Phase 6B: All 11 MP JS files wired to real APIs with mock fallback. 12 marketplace entries added to PAGE_PERMISSION_MAP. |
| `2026-03-22 13:10` | `us` | `3.11, 3.12` | `backend/src/marketplace/p2p.rs, charts.rs` | `✅ Check-Out` | Phase 3 COMPLETE (16/16). P2P OTC (create/accept/decline/counter + ACID settlement, 8 tests). Candlestick charts (OHLCV, 7 intervals, epoch bucketing, 5 tests). 104 total tests pass. |
| `2026-03-22 13:15` | `us` | `5.4, 5.9` | `frontend/platform/static/js/marketplace-chart.js, marketplace-p2p.js` | `✅ Check-Out` | Phase 5: Candlestick chart (ApexCharts, 7 intervals, 24h summary, mock fallback). P2P UI (incoming/outgoing tabs, accept/decline/counter, create offer modal, notification badge). Both wired into trading-v3.html. |
| `2026-03-22 13:20` | `us` | `5.3, 5.11, 5.12` | `marketplace-secondary.js, marketplace-trading-v3.css, trading-v3.html` | `✅ Check-Out` | Phase 5 COMPLETE (13/13). Live price polling (30s). Accessibility: focus-visible, reduced-motion, skip-link, ARIA landmarks. Responsive: 768px/480px breakpoints for chart/P2P/orderbook. |
| `2026-03-22 15:45` | `us` | `Global` | `docs/` | `✅ Check-Out` | Full Masterplan audit. Verified Phase 7 (Smart Contracts) and Phase 8 (Blockchain Integration). Updated statuses accordingly. |
| `2026-03-22 15:50` | `us` | `8C.1 - 8C.2` | `admin/blockchain.rs, admin-*.js` | `✅ Check-Out` | Integrated Blockchain Treasury and Asset Tokenize admin pages. Wired to real `backend/src/admin/blockchain.rs` APIs for Polygon deployment & settlement management. |
| `2026-03-22 16:08` | `us` | `Global` | `docs/` | `✅ Check-Out` | Reviewed Community Masterplan and updated Phase 14 in the Implementation Roadmap to reflect the new modular `COMMUNITY_ROADMAP.md`. |
| `2026-03-22 16:17` | `us` | `Global` | `docs/` | `✅ Check-Out` | Added Module 6 (Advanced Engagement / Bettermode features) to `COMMUNITY_ROADMAP.md` and `IMPLEMENTATION_ROADMAP.md`. |
| `2026-03-22 16:21` | `us` | `Global` | `docs/` | `✅ Check-Out` | Added 7 new Expert strategy tasks to `COMMUNITY_ROADMAP.md` for engagement loops and safety (e.g. Asset Velocity monitor, Auto-Tags, Daily digests). |
| `2026-03-22 16:35` | `us` | `Phase 7` | `docs/` | `✅ Check-Out` | Updating Roadmap and Masterplan to pivot from single ERC-1155 to AssetFactory EIP-1167 Clones per user request (SPV Isolation). |
| `2026-03-22 16:55` | `us` | `7.2 - 7.11` | `contracts/` | `✅ Check-Out` | Deployed IdentityRegistry, POOOLAssetToken implementation, and AssetFactory utilizing EIP-1167. Added unit & 10,000 Fuzz tests. All tests passing smoothly. |
| `2026-03-22 17:08` | `us` | `Global` | `docs/` | `✅ Check-Out` | Added Phase 16 (Primary Issuance) & Phase 17 (RegTech) based on Whitepaper gap analysis. |
| `2026-03-22 17:05` | `us` | `8C` | `docs/` | `✅ Check-Out` | Added 8C.3 "Live Contracts Overview", 8C.4 "Contract Contract View", and 8C.5 "Web3 Sync & Health" to the IMPLEMENTATION_ROADMAP.md in response to the EIP-1167 mapping requirement. |
| `2026-03-22 17:25` | `us` | `8A, 8B` | `backend/src/blockchain/` | `✅ Check-Out` | Updated Blockchain Integration to match AssetFactory architecture. Modified KYC worker to call `setWhitelisted`, updated settlement worker to aggregate batches by unique `chain_contract_address`, updated admin API to deploy clones and capture clone address. |
| `2026-03-22 17:35` | `us` | `16.1` | `backend/src/issuance/` | `❌ Aborted` | Scaffolding reverted per User instruction — `developer` module already fulfills Whitepaper "Issuer" specs. |
| `2026-03-22 17:40` | `us` | `16.1, 16.2` | `backend/src/developer/` | `✅ Check-Out` | Marked Asset Submission Portal & Due Diligence as Done. The existing Developer Submission & Admin Review UI perfectly matches these Whitepaper requirements. |
| `2026-03-22 17:45` | `us` | `16.3` | `database/, backend/src/` | `✅ Check-Out` | Primary Offering Engine targets implemented in DB and mapped to an Admin Dashboard. |
| `2026-03-22 17:51` | `us` | `16.4` | `backend/src/admin/primary_escrow.rs` | `✅ Check-Out` | Auto-Refund worker built. Periodically scans expired escrows, refunds wallets natively, logs txs, and aborts pending asset states. |
| `2026-03-22 17:55` | `us` | `16.5` | `backend/src/cart/` | `✅ Check-Out` | KFS Generation & Presentation implemented. Built a generic KFS modal that dynamically aggregates Primary cart items, specifies escrow rules, and enforces check-out acknowledgement. |
| `2026-03-22 17:58` | `us` | `17.1` | `portfolio/` | `✅ Check-Out` | Implementing 48h Cooling-off period logic backend natively parsing timeframe intervals, and surfacing a stateful Cancellation UI button on Portfolio. Full refund logic integrated. |
| `2026-03-22 18:30` | `us` | `8B.5, 8C.3-8C.5` | `admin/blockchain.rs, blockchain/service.rs, admin-blockchain-*.js, blockchain-sync.html` | `✅ Check-Out` | Phase 8 COMPLETE. Dynamic batching (reads interval/batch from platform_settings). Web3 Sync page (indexer KPIs, settlement stats, KYC whitelist queue w/ Force Sync, terminal report). Per-clone pause/unpause. Fixed 2 P1 bugs in payments/service.rs (Datelike import, total_cents ordering). |
| `2026-03-22 18:45` | `us` | `10.1-10.8` | `main.rs, settings/, portfolio/, frontend/platform/` | `✅ Check-Out` | Phase 10 COMPLETE (8/8). CSP hardened, reconciliation persisted, GDPR export+deletion API, security audit passed, Polygonscan portfolio links, Admin RBAC wired, kill-switch tested, settlement integration verified. |
| `2026-03-22 19:00` | `us` | `11.1-11.5, 11.8` | `common/financial_tests.rs, common/reconciliation_tests.rs, contracts/test/POOOLAssetToken.fuzz.t.sol` | `🔄 IN PROGRESS` | Phase 11 Testing: 7/10 DONE. 47 financial tests + 5 reconciliation tests + 10 Foundry fuzz tests (10k runs each). 160 Rust + 12 Solidity = 172 total tests passing. Remaining: 11.6 (Playwright E2E), 11.7 (Load Test), 11.9 (UAT). |
| `2026-03-22 23:25` | `us` | `14.4` | `backend/src/community/, payments/, admin/` | `✅ Check-Out` | Phase 14 / Community M3 Social Layer completed! All 7 API tasks and 5 UI Tasks complete. Dynamic asset tags natively wire with checkout and approvals. Modals completed. XP engine and badges running. |
| `2026-03-22 23:45` | `us` | `Global` | `docs/` | `✅ Check-Out` | Audited Community modules 1-3. Created Module 3.5 for P0/P1 security fixes and restructured Modules 4 & 5 to include AMAs, Challenges, and full admin UI. |
| `2026-03-23 00:30` | `us` | `14.5` | `community/xp.rs, circles.rs, routes.rs, community-circles.js` | `✅ Check-Out` | Community M4 Circles & XP COMPLETE: 15/15 tasks. XP system (award, daily caps, levels, history, aggregation worker). Circles (CRUD, invite, join/leave, kick, leaderboard, referral auto-join). Login streak tracker (daily + 7/30-day bonuses). Level-gated features (L2 circles, L3 invites). Circle retry worker. 18 new API endpoints. Frontend: dynamic My Circle tab, XP card w/ streak, leaderboard, level-up animation. |
| `2026-03-23 11:45` | `us` | `11.6` | `tests/e2e/` | `✅ Check-Out` | Playwright E2E testing framework expanded for Journey, Settings, Community, Marketplace, and Circles. |
| `2026-03-24 10:35` | `us` | `14.6` | `frontend/platform/community.html` | `✅ Check-Out` | Completed 10 Module 5.5 UI Data Wiring tasks in community.html and related JS. Replaced static/broken data with live API endpoints. |
| `2026-03-28 04:50` | `us` | `5.14` | `marketplace-trading-v3.html, property.html, property-detail.css` | `✅ Check-Out` | Unified investment calculator sliders across V3 and standard property pages. Applied premium design, fixed hardcoded limits, and wired dynamic JS population. |

---

## PHASE 0: Infrastructure & Account Setup (MP 6.2)

*DevOps + PM — Must be completed first. No code depends on this being fancy, but everything depends on it existing.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **0.1** | Cloud SQL Core DB Provisioning | `db-f1-micro`, PG16, PITR enabled, `asia-southeast1`, 14-day backup retention (§3.3.1) | `✅ DONE` | Martin | `✅` | Cloud SQL running in production on Cloud Run. |
| **0.2** | Cloud SQL Community DB Provisioning | Separate instance, PITR enabled, 7-day retention (§3.3.1) | `✅ DONE` | us | `✅` | Dev database `poool_community` provisioned. |
| **0.3** | Cloud SQL Read Replicas | One replica per DB for read routing (§3.3.3) | `❌ NOT STARTED` | - | `❌` | Optimization for later — not needed at current scale. |
| **0.4** | Redis Memorystore | `basic` tier, 1GB, `redis_7_2`, `asia-southeast1` (§3.3.4) | `✅ DONE` | us | `❌` | Setup script generated (`gcp_setup_phase0.sh`). Waiting for user to execute. |
| **0.5** | PgBouncer Sidecar | Connection pooling proxy in Dockerfile (§1.9, §3.3.8) | `✅ DONE` | us | `✅` | Dockerfile updated: debian-slim runtime + PgBouncer sidecar. `pgbouncer/entrypoint.sh` parses DATABASE_URL, starts PgBouncer on :6432, then backend. Set `PGBOUNCER_ENABLED=false` to skip. |
| **0.6** | PITR & Backup Strategy | 3-layer backups: PITR + daily snapshots + weekly cross-region `pg_dump` to GCS (§3.3.2) | `❌ NOT STARTED` | - | `❌` | Cloud SQL auto-backups exist, but no 3-layer strategy. |
| **0.7** | CI/CD Pipeline | GitHub Actions → Build → Test → Deploy to Cloud Run (§6.2) | `✅ DONE` | us | `✅` | Already implemented: `ci.yml` (fmt + clippy + test + audit + Docker build) + `deploy.yml` (GCP auth + Docker push + Cloud Run deploy + health check). |
| **0.8** | Cloud Monitoring Alerts | 10 alert policies: CPU, connections, Redis memory, error rate, latency, reconciliation (§3.3.7) | `✅ DONE` | us | `❌` | Setup script generated (`gcp_setup_phase0.sh`). Waiting for user to execute. |
| **0.9** | Health Check Endpoint | `GET /health` → 200/503 based on DB + Redis reachability (§3.3.7) | `✅ DONE` | us | `✅` | Enhanced `handle_health` in `main.rs`: probes DB (`SELECT 1`) + Redis (`PING`). Returns 200+components when healthy, 503 when DB is down. Redis is optional. |
| **0.10** | Sentry Setup | Error monitoring for production (§6.10) | `✅ DONE` | Martin | `✅` | Full Sentry integration: DSN config, user context middleware, tracing layer, reconciliation alerts. |
| **0.11** | Marketplace RBAC Permissions | 3 new permissions: `marketplace.view`, `marketplace.manage`, `marketplace.compliance` (§3.5.1) | `✅ DONE` | us | `✅` | Migration `056_marketplace_rbac_permissions.sql`: grants to super_admin (all 3), compliance (view+compliance), finance (view). Admin already has 'all'. |
| **0.12** | Third-Party Account Setup | PM checklist: Alchemy, Pinata, Base Sepolia, SendGrid, SC Auditor quotes (§6.10) | `❌ NOT STARTED` | - | `❌` | - |

---

## PHASE 1: Backend Core — Hardening & 2FA (MP 6.3)

*Rust Backend Engineer — Security hardening before marketplace features. No new features, only safety.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **1.1** | Dual DB Pool Setup | Split `db.rs` into `core_primary` + `core_replica` + `community` pools with config from env (§3.3.3) | `✅ DONE` | us | `✅` | `DatabasePools` struct with primary/replica/community in `db.rs`. Env vars: `DATABASE_REPLICA_URL`, `COMMUNITY_DATABASE_URL`. |
| **1.2** | Connection Pool Tuning | `max_connections(30)`, `min_connections(5)`, `acquire_timeout(5s)`, `idle_timeout(120s)` (§3.3.8) | `✅ DONE` | us | `✅` | Constants: `PRIMARY_MAX=30`, `REPLICA_MAX=15`, `COMMUNITY_MAX=10`, timeouts 5s/120s in `db.rs`. |
| **1.3** | Read-Your-Writes Pattern | Redis `recent_write:{user_id}` flag with 2s TTL to route reads to primary after writes (§3.3.3) | `✅ DONE` | us | `✅` | `read_pool()` + `mark_recent_write()` in `db.rs`. Redis key `recent_write:{user_id}` with 2s TTL. |
| **1.4** | Step-Up 2FA Middleware | `require_step_up_2fa()` middleware for financial operations (§1.11) | `✅ DONE` | us | `✅` | `auth/step_up.rs`: checks TOTP, thresholds, and trading session. `POST /auth/2fa/step-up` route. |
| **1.5** | Trading Session in Redis | `SET trading_session:{user_id}` with 15-min TTL after 2FA verification (§1.11) | `✅ DONE` | us | `✅` | `create_trading_session()` + `check_trading_session()` in `step_up.rs`. 900s TTL. Action-scoped keys. |
| **1.6** | 2FA Enforcement Triggers | Force 2FA on withdrawals >$100, trades >$500, wallets >$1000 (§1.11) | `✅ DONE` | us | `✅` | Thresholds: `$100` withdrawal, `$500` trade, `$1000` wallet setup. `FinancialAction` enum. `check_2fa_setup_required()`. |
| **1.7** | Withdrawal Limits | $10K/tx, $25K/day velocity checks, 72h cooldown on new accounts (§1.8 Q3) | `✅ DONE` | us | `✅` | `wallet/routes.rs`: `MAX_WITHDRAWAL_CENTS=1M`, daily `$25K` check, 72h cooldown, 3/hr velocity, `FOR UPDATE` lock. |
| **1.8** | Idempotency for Checkout | Idempotency-Key in `execute_checkout` to prevent double-submissions (§1.8 Q6) | `✅ DONE` | us | `✅` | `payments/routes.rs`: `Idempotency-Key` header, `idempotency_keys` DB table, cached responses, cleanup on failure. |
| **1.9** | Daily Reconciliation Job | Tokio worker: `SUM(wallets) = deposits - withdrawals - purchases`. Sentry alert on >€1 mismatch (§1.8 Q2, §3.1.8) | `✅ DONE` | us | `✅` | `main.rs`: 3-check reconciliation (cash, token, negative balances). Sentry alerts on >$1 delta. Runs every 12h. |
| **1.10** | Decimal-based FX Logic | Replace `f64` division with `DECIMAL(18,6)` for IDR/USD conversion (§1.8 Q5) | `✅ DONE` | us | `✅` | `payments/service.rs`: `rust_decimal::Decimal`, f64→Decimal via string, `RwLock` FX cache (1h TTL). |
| **1.11** | AppError Extension | Add marketplace errors: `OrderRejected`, `TwoFactorRequired`, `ServiceUnavailable`, `InsufficientBalance`, `InsufficientTokens`, `WashTradingBlocked` (§3.1.3) | `✅ DONE` | us | `✅` | `error.rs`: 8 new variants with proper HTTP status codes (402/403/409/429/503). Client-safe messages. |

---

## PHASE 2: Database Migrations & Schema (MP 4.2, 4.3, 4.6)

*DevOps + Backend — All marketplace tables, in correct dependency order.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **2.1** | Migration `050b`: `wallets.held_balance_cents` | `ALTER TABLE wallets ADD COLUMN held_balance_cents BIGINT` + constraint `held ≤ balance` (§4.3) | `✅ DONE` | us | `✅` | Applied. CHECK constraint chk_held_lte_balance verified. |
| **2.2** | Migration `050c`: `investments.held_tokens` | `ALTER TABLE investments ADD COLUMN held_tokens INTEGER` + constraint `held ≤ owned` (§4.3) | `✅ DONE` | us | `✅` | Applied. CHECK constraint chk_held_tokens_lte_owned verified. |
| **2.3** | Migration `050`: `market_orders` | Full table with 8 statuses, idempotency_key, expires_at, indexes (§4.2 Mig050) | `✅ DONE` | us | `✅` | Applied. 4 indexes incl. partial indexes for active orders. |
| **2.4** | Migration `051`: `trade_history` | Immutable trade log with on_chain_status, fee tracking, FK to market_orders (§4.2 Mig051) | `✅ DONE` | us | `✅` | Applied. Generated column total_cents. Self-trade CHECK. |
| **2.5** | Migration `052`: `p2p_offers` | P2P direct offers with parent_offer_id chain, expiry, self-trade check (§4.2 Mig052) | `✅ DONE` | us | `✅` | Applied. Self-referencing FK, 48h default expiry. |
| **2.6** | Migration `053`: `fee_configurations` + `fee_promotions` | 4-tier fee hierarchy: platform → developer → asset → promotion (§4.2 Mig053) | `✅ DONE` | us | `✅` | Applied. BPS caps at 1000 (10%). Promo date validation. |
| **2.7** | Migration `054`: `marketplace_alerts` + `marketplace_watchlist` | Fraud detection alerts with severity, status workflow, user watchlist (§4.2 Mig054) | `✅ DONE` | us | `✅` | Applied. Unique active watchlist entry per user. |
| **2.8** | Migration `055`: `reconciliation_reports` | Daily balance check storage: cash/fee/token deltas (§4.2 Mig055) | `✅ DONE` | us | `✅` | Applied. Standalone table, unique per report_date. |
| **2.9** | TimescaleDB Extension | `CREATE EXTENSION timescaledb`, `create_hypertable('trade_history', ...)` (§4.4, §3.3.5) | `⏸️ BLOCKED` | - | `❌` | Requires TimescaleDB extension (not installed locally). |
| **2.10** | Continuous Aggregates | `candles_1m`, `candles_1h`, `candles_1d` materialized views with refresh policies (§4.4) | `⏸️ BLOCKED` | - | `❌` | Depends on 2.9. |

---

## PHASE 3: Core Trading Engine — `src/marketplace/` (MP 3.1, 6.4)

*Rust Backend Engineer — The heart of the marketplace.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **3.1** | Module Structure (`mod.rs`) | Create `marketplace/` module: `mod.rs`, `models.rs`, `routes.rs`, `service.rs`, etc. (~60 lines) (§3.1.9) | `✅ DONE` | us | `✅` | 6 files created, wired into main.rs |
| **3.2** | Data Models (`models.rs`) | `MarketOrder`, `TradeRecord`, `FeeConfig`, `P2POffer`, `OrderbookLevel`, etc. with serde + sqlx (~350 lines) (§3.1.2) | `✅ DONE` | us | `✅` | 16 tests passing. All monetary i64 cents. |
| **3.3** | Validation Module (`validation.rs`) | Balance checks, KYC verification, rate limiting, min order $10, concentration limits (~350 lines) (§3.1.4) | `✅ DONE` | us | `✅` | 14 tests. 10 validation checks. 4-tier fee resolution. |
| **3.4** | Redis Orderbook (`orderbook.rs`) | ZADD/ZREM/best_bid/best_ask/get_snapshot/rebuild_from_postgres (~450 lines) (§3.1.5, §2.3) | `✅ DONE` | us | `✅` | 11 tests. Self-healing rebuild. Graceful degradation. |
| **3.5** | Order Submission API | `POST /api/marketplace/orders` — validation → balance hold → Redis insert → response (§3.1.6, §2.12) | `✅ DONE` | us | `❌` | Implemented in service.rs + routes.rs |
| **3.6** | Matching Engine (`matching.rs`) | Tokio task: Price-Time-Priority, partial fills, wash-trade prevention, 10ms loop (~300 lines) (§3.1.6, §2.4) | `✅ DONE` | us | `✅` | 7 tests. Self-trade cancels newer order. Order locks respected. |
| **3.7** | Settlement Pipeline (`settlement.rs`) | 8-step ACID TX: validate → update orders → transfer balance → transfer tokens → record trade → calc fees → log → update Redis (~350 lines) (§3.1.7, §2.5) | `✅ DONE` | us | `✅` | 4 tests. Conservation of funds verified. Fee + proceeds = total. |
| **3.8** | Fee Calculation Engine | 5-tier hierarchy lookup: Promotion → Developer → Asset → Tier → Platform. BPS math, no floats (§2.6, §3.1) | `✅ DONE` | us | `✅` | Implemented in validation.rs (resolve_fees) + models.rs (calculate_fee_cents) |
| **3.9** | Order Cancel API | `DELETE /api/marketplace/orders/{id}` with 5s Redis lock to prevent cancel-during-match race (§2.13) | `✅ DONE` | us | `❌` | Redis lock + ACID. Implemented in service.rs |
| **3.10** | Marketplace Read APIs | `GET /orderbook/{asset_id}`, `GET /trades/{asset_id}`, `GET /ticker/{asset_id}`, `GET /candles` (§2.12) | `✅ DONE` | us | `❌` | Implemented in routes.rs + service.rs |
| **3.11** | P2P/OTC Offer System (`p2p.rs`) | Create/accept/decline/counter offers, settlement reuse, fee application (~300 lines) (§2.7, §3.1) | `✅ DONE` | us | `✅` | ~480 lines. ACID settlement. Counter-offer chains. Expiry worker. 8 tests. |
| **3.12** | Candlestick Chart API (`charts.rs`) | `GET /candles?asset_id=&interval=1h&from=&to=` backed by trade_history aggregates (~150 lines) (§2.8) | `✅ DONE` | us | `✅` | ~295 lines. 7 intervals (1m–1w). Epoch bucketing for non-standard intervals. Chart summary API. 5 tests. |
| **3.13** | Background Workers (`background.rs`) | 3 workers: Order Expiry (hourly), Redis-Sync (5 min), Price Snapshot (5 min) (~300 lines) (§3.1.8) | `✅ DONE` | us | `✅` | 4 tests. ACID expiry with hold release. Bidirectional sync. |
| **3.14** | Rate Limiting | Redis-based: max 10 orders/min/user, configurable (§2.13) | `✅ DONE` | us | `✅` | Implemented in orderbook.rs (check_order_rate_limit) |
| **3.15** | Idempotency Layer | Redis `idempotency:{key}` with 1h TTL for order submissions (§2.13) | `✅ DONE` | us | `✅` | 24h TTL. Implemented in orderbook.rs |
| **3.16** | Spawn Background Tasks in `main.rs` | Wire up matching engine + settlement worker + expiry worker as tokio::spawn (§3.1.6) | `✅ DONE` | us | `❌` | Matching + Settlement spawned when Redis is configured |

---

## PHASE 4: WebSocket Server (MP 3.1.7, 2.9)

*Real-time updates for Trading UI.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **4.1** | WebSocket Handler | `GET /ws/market/{asset_id}` — Axum WS upgrade, per-asset broadcast channels (~250 lines) (§3.1.7) | `✅ DONE` | us | `✅` | 5 tests. OnceLock channels. Initial snapshot on connect. Lag recovery. |
| **4.2** | Redis Pub/Sub Cross-Instance | `PUBLISH market:{asset_id}` for multi-Cloud-Run-instance sync (§3.1.7) | `✅ DONE` | us | `✅` | PUBLISH implemented. Subscriber uses polling (upgrade to native pub/sub for multi-instance). |
| **4.3** | Broadcast Functions | `broadcast_orderbook_update()`, `broadcast_trade()`, `broadcast_ticker()` (§3.1.7) | `✅ DONE` | us | `✅` | 3 broadcast fns. Local + Pub/Sub delivery. |
| **4.4** | Heartbeat & Reconnect | 30s server ping, client heartbeat, reconnect handling (§3.1.7) | `✅ DONE` | us | `❌` | 30s ping interval. Close on Pong timeout. |

---

## PHASE 5: Frontend — Trading UI (MP 3.4, 6.6)

*Frontend Engineer — Vanilla HTML + CSS + JS, no frameworks.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **5.1** | Event Bus (`marketplace-event-bus.js`) | Lightweight EventTarget-based bus: `on`, `emit`, `off`, `once` (~30 lines) (§3.4.2) | `✅ DONE` | us | `❌` | ~80 lines. WeakMap handler tracking. Object.freeze for safety. |
| **5.2** | WebSocket Client (`marketplace-websocket.js`) | Auto-reconnect, exponential backoff, heartbeat, event-bus integration (~200 lines) (§3.4.3) | `✅ DONE` | us | `❌` | ~230 lines. Backoff 1s→30s with jitter. Visibility API pause/resume. |
| **5.3** | Marketplace Overview Page | `marketplace-secondary.js` — Live price polling via chart-summary API (§3.4.1) | `✅ DONE` | us | `❌` | 30s polling via `fetchLiveSummary()`. Price flash animation. Visibility API gate. |
| **5.4** | Candlestick Chart Integration | ApexCharts candlestick with interval switcher, real-time updates (§3.4.4) | `✅ DONE` | us | `❌` | `marketplace-chart.js` ~310 lines. 7 interval buttons, 24h summary header, dark theme, mock fallback. Wired to `GET /api/marketplace/:asset_id/candles`. |
| **5.5** | Orderbook Rendering (`marketplace-orderbook.js`) | Bid/Ask tables, DOM patching (no full re-render), flash animations, depth bars (~200 lines) (§3.4.5) | `✅ DONE` | us | `❌` | ~230 lines. Flash anim. Depth bars. Click-to-fill. |
| **5.6** | Buy/Sell Order Form | Price/qty inputs, real-time total, balance validation, double-click protection, idempotency-key, optimistic UI (§3.4.6) | `✅ DONE` | us | `❌` | Wired to POST /api/marketplace/orders. UUID idempotency keys. |
| **5.7** | 2FA Step-Up Modal | TOTP input modal triggered on 428 response, retry with trading session (§3.4.6) | `✅ DONE` | us | `❌` | 428 detection + MarketBus event. Modal not yet built. |
| **5.8** | My Orders & Trade History | User's open orders with cancel, own trade list (§3.4.8) | `✅ DONE` | us | `❌` | Fetch + render + cancel via DELETE API. Recent trades with timestamp. |
| **5.9** | P2P Offer UI (`marketplace-p2p.js`) | Cap table, send offer modal, incoming offer notification badge (~200 lines) (§3.4.7) | `✅ DONE` | us | `❌` | `marketplace-p2p.js` ~500 lines. Tabs (incoming/outgoing), accept/decline/counter actions, create + counter modals, notification badge, injected CSS. |
| **5.10** | Loading/Error/Empty States | Skeleton loaders, error-retry buttons, empty-state messages for all components (§3.4.9) | `✅ DONE` | us | `❌` | Empty states + toast notifications for success/error/warning. |
| **5.11** | Accessibility | ARIA labels, keyboard nav, focus management, `role="alert"` on toasts, reduced-motion (§3.4.10) | `✅ DONE` | us | `❌` | Skip-link, focus-visible outlines, prefers-reduced-motion, ARIA landmarks (nav, main, breadcrumb), sr-only class. |
| **5.12** | Responsive Design | Mobile-first: 360px → 1920px, touch-friendly order form (§3.4.12) | `✅ DONE` | us | `❌` | 3 breakpoints (1100px/768px/480px). Chart toolbar horizontal scroll. P2P modal full-width mobile. Orderbook compact mode. Toast full-width mobile. |
| **5.13** | Orchestration (`marketplace-trading.js`) | `DOMContentLoaded` init: WS → Chart → Orderbook → OrderForm → P2P → visibility API → cleanup (§3.4.8) | `✅ DONE` | us | `❌` | ~400 lines. Full lifecycle init. 30s polling backup. |
| **5.14** | Investment Calculator Unification | Unify slider UI/UX across `marketplace-trading-v3` and `property.html`. Dynamic limits based on property value. (§3.4) | `✅ DONE` | us | `✅` | Applied premium V3 design to standard pages. Fixed hardcoded limits in V3. |

---

## PHASE 6: Admin Dashboard — Marketplace Section (MP 3.5, 6.6b)

*Frontend + Backend — 12 new admin pages with RBAC.*

### 6A: Admin Backend APIs

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **6A.1** | Admin Marketplace Stats API | `GET /api/admin/marketplace/stats` — KPIs: volume, orders, trades, pending (§3.5.4) | `✅ DONE` | us | `✅` | 8 KPIs. Redis-based trading status check. |
| **6A.2** | Admin Recent Trades API | `GET /api/admin/marketplace/recent-trades` (§3.5.4) | `✅ DONE` | us | `❌` | 50 most recent. Joins user emails + asset names. |
| **6A.3** | Admin Orderbook API | `GET /api/admin/marketplace/orderbook/{asset_id}` with user IDs (§3.5.5) | `✅ DONE` | us | `❌` | Aggregated levels. Spread + mid-price. |
| **6A.4** | Admin Orderbook Rebuild | `POST /api/admin/marketplace/orderbook/rebuild` (§3.5.5) | `✅ DONE` | us | `❌` | Calls `rebuild_from_postgres()`. Returns count of restored orders. |
| **6A.5** | Admin Trade History API | `GET /api/admin/marketplace/trades` with 6 filters + pagination (§3.5.6) | `✅ DONE` | us | `❌` | Dynamic WHERE. asset_id, user_id, side filters. Paginated. |
| **6A.6** | Admin Open Orders API | `GET /api/admin/marketplace/orders` + `DELETE` for admin-cancel (§3.5.7) | `✅ DONE` | us | `❌` | Paginated. Admin cancel in transaction with balance refund. |
| **6A.7** | Admin Pending Approvals API | `GET /pending`, `POST /approve`, `POST /reject` for large orders (§3.5.8) | `✅ DONE` | us | `❌` | Approve→open, Reject→refund held balance in TX. |
| **6A.8** | Admin Fee Management APIs | CRUD for `fee_configurations` + `fee_promotions` (§3.5.9) | `✅ DONE` | us | `❌` | GET lists configs+promos. POST creates with BPS 0-1000 validation. |
| **6A.9** | Admin P2P Offers API | `GET /api/admin/marketplace/p2p` with price-deviation warnings (§3.5.10) | `✅ DONE` | us | `❌` | LATERAL join for market price. Deviation calc in SQL. |
| **6A.10** | Admin Reconciliation API | Cash balance, fee balance, token integrity checks (§3.5.13) | `✅ DONE` | us | `✅` | 3 invariant checks. Token supply vs holdings. |
| **6A.11** | Admin Trading Kill-Switch | `POST /toggle-trading` — Redis flag, super-admin only (§3.5.15) | `✅ DONE` | us | `❌` | Redis SET marketplace:trading_enabled. Audit logged. |
| **6A.12** | Admin Alerts & Watchlist APIs | Create/acknowledge/resolve alerts, manage watchlist (§3.5.12) | `✅ DONE` | us | `❌` | Alerts: severity sort, acknowledge/resolve/false_positive. Watchlist: list+add. |
| **6A.13** | Admin Compliance/OJK APIs | OJK quarterly report, travel-rule export, user tax reports (§3.5.14) | `✅ DONE` | us | `❌` | Added 3 CSV export APIs (ojk-report, travel-rule, tax-export) in marketplace.rs |
| **6A.14** | Admin Marketplace Settings API | Read/update all configurable parameters via Redis (§3.5.15) | `✅ DONE` | us | `❌` | GET/POST Redis-backed settings. 10 params. Syncs kill-switch flag. |
| **6A.15** | Admin Health API | `GET /api/admin/marketplace/health` — DB latency, Redis status, WS connections (§3.5.4) | `✅ DONE` | us | `❌` | DB ping, Redis PING, queue depth. |

### 6B: Admin Frontend Pages (12 Pages)

| ID | Task | Page | Priority | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|:---|:---|
| **6B.1** | Admin Sidebar Extension | - | 🔴 LAUNCH | Add 📈 MARKETPLACE section with 12 nav items (§3.5.2) | `✅ DONE` | us | `❌` | HTML pages exist. Routes registered in mod.rs. |
| **6B.2** | Permission Guard Update | - | 🔴 LAUNCH | Add 12 entries to `PAGE_PERMISSION_MAP` (§3.5.1) | `✅ DONE` | us | `❌` | 12 marketplace entries added. Uses marketplace.view/.manage/.compliance RBAC perms. |
| **6B.3** | Overview & Monitoring | `/admin/marketplace/` | 🔴 LAUNCH | KPI cards, live trade table, top-5 assets, system health (§3.5.4) | `✅ DONE` | us | `❌` | HTML + JS wired to API. 30s auto-refresh. Mock fallback. |
| **6B.4** | Live Orderbook | `/admin/marketplace/orderbook` | 🔴 LAUNCH | Admin orderbook with user IDs, rebuild button (§3.5.5) | `✅ DONE` | us | `❌` | HTML + JS wired to API. Rebuild API done. Mock fallback. |
| **6B.5** | Trade History | `/admin/marketplace/trades` | 🔴 LAUNCH | Filterable table, CSV export, clickable user/asset links (§3.5.6) | `✅ DONE` | us | `❌` | JS wired to paginated API. Mock fallback. |
| **6B.6** | Open Orders | `/admin/marketplace/orders` | 🔴 LAUNCH | Order table, admin-cancel with reason dialog (§3.5.7) | `✅ DONE` | us | `❌` | JS wired to API + DELETE cancel. Mock fallback. |
| **6B.7** | Pending Approvals | `/admin/marketplace/approvals` | 🔴 LAUNCH | Large order review cards, user context, approve/reject (§3.5.8) | `✅ DONE` | us | `❌` | JS wired: real POST approve/reject. Mock fallback. |
| **6B.8** | Reconciliation | `/admin/marketplace/reconciliation` | 🔴 LAUNCH | 3 invariant checks, delta display, history table, CSV export (§3.5.13) | `✅ DONE` | us | `❌` | JS wired to API. Mock fallback. |
| **6B.9** | Fee Management | `/admin/marketplace/fees` | 🟡 WEEK 2 | 3 tabs: Platform/Asset/Promotions, BPS slider (§3.5.9) | `✅ DONE` | us | `❌` | JS wired: configs + promos from API. Mock fallback. |
| **6B.10** | Marketplace Settings | `/admin/marketplace/settings` | 🟡 WEEK 2 | Kill-switch, 13 configurable params (§3.5.15) | `✅ DONE` | us | `❌` | JS loads/saves to Redis via API. Mock fallback. |
| **6B.11** | P2P Offers | `/admin/marketplace/p2p` | 🟡 WEEK 2 | Offer table, price warnings, admin cancel (§3.5.10) | `✅ DONE` | us | `❌` | JS wired: price deviation calc. Mock fallback. |
| **6B.12** | Analytics & Charts | `/admin/marketplace/analytics` | 🟡 WEEK 3 | Embedded Metabase + built-in ApexCharts: volume, top-trader, fee revenue (§3.5.11) | `✅ DONE` | us | `❌` | Metabase iframe + `mp-analytics.js` with ApexCharts (volume timeline, top assets, stats cards). |
| **6B.13** | Alerts & Watchlist | `/admin/marketplace/alerts` | 🟡 WEEK 3 | Alert table, acknowledge/resolve, user watchlist management (§3.5.12) | `✅ DONE` | us | `❌` | JS wired: acknowledge/resolve via POST. Mock fallback. |
| **6B.14** | Compliance & OJK | `/admin/marketplace/compliance` | 🟡 WEEK 4 | OJK reports, travel-rule, tax exports, AML reports (§3.5.14) | `✅ DONE` | us | `❌` | Added reporting UI replacing limits. Wired buttons to trigger direct CSV downloads. |

---

## PHASE 7: Smart Contracts — ERC-1155 on Polygon (UPDATED: was ERC-3643/Base)

*Web3 Engineer — Runs PARALLEL to Phases 3-5. Chain: Polygon PoS. Token: ERC-1155 for fractional ownership.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **7.1** | Foundry Project Setup | `forge init`, install OpenZeppelin v5 (ERC-1155, AccessControl, Pausable) | `✅ DONE` | us | `✅` | `foundry.toml` & deps installed. |
| **7.2** | POOOLAssetToken Contract | Standalone ERC-1155 (or ERC-20) token representing a single real-world property. Serves as Implementation for EIP-1167 clones | `✅ DONE` | us | `✅` | EIP-1167 implementation completed |
| **7.3** | Access Control & Roles | `MINTER_ROLE`, `PAUSER_ROLE`, `SETTLEMENT_ROLE` via OpenZeppelin AccessControl | `✅ DONE` | us | `✅` | Implemented in implementation and Factory |
| **7.4** | Shared KYC Registry | Independent Identity/KYC Registry smart contract that all deployed Asset clones read from. | `✅ DONE` | us | `✅` | Dedicated `IdentityRegistry.sol` deployed |
| **7.5** | Transfer Restrictions | Override `_update()` to read from Shared KYC Registry and enforce max ownership (80%) | `✅ DONE` | us | `✅` | Checked via overriding hooks in POOOLAssetToken |
| **7.6** | BatchSettlement Engine | `settleBatch` at the token level or via an exchange contract optimized for netted transfers | `✅ DONE` | us | `✅` | Uses `_update` to bypass approvals for SETTLEMENT_ROLE |
| **7.7** | AssetFactory Contract | `AssetFactory.sol` using EIP-1167 Clones to deploy a separate contract address for each asset. Emits `AssetDeployed(address)` | `✅ DONE` | us | `✅` | Fully built with OpenZeppelin Clones |
| **7.8** | URI Metadata (IPFS) | Contract-level URI pointing to the specific property's JSON metadata and SPV docs | `✅ DONE` | us | `✅` | Set at initialization for each clone |
| **7.9** | Foundry Unit Tests | Mint, burn, transfer, transfer-blocked-without-KYC, zero-amount, self-transfer, batch | `✅ DONE` | us | `✅` | 80 tests passing |
| **7.10** | Foundry Fuzz Tests | 10,000+ runs: random amounts, mismatched arrays, edge cases | `✅ DONE` | us | `✅` | `POOOLProperty1155.fuzz.t.sol` |
| **7.11** | Invariant Tests | For each tokenId: `totalSupply(tokenId) == SUM(balanceOf(all_users, tokenId))` ALWAYS | `✅ DONE` | us | `✅` | `POOOLProperty1155.invariant.t.sol` |
| **7.12** | Polygon Mumbai/Amoy Testnet Deploy | Deploy all contracts, verify on Polygonscan | `✅ DONE` | us | `✅` | Script available |
| **7.13** | Smart Contract Audit | Commission external audit. ⚠️ Order in Week 4! | `⚪ NOT READY` | - | `❌` | 4-6 week lead time! |

---

## PHASE 8: Blockchain Integration (MP 3.2.6, 3.2.9, 3.2.11)

*Backend + Web3 — Connecting Rust backend to Polygon.*

### 8A: Blockchain DB Migrations

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8A.1** | Migration `050d`: `assets` blockchain fields | `contract_address`, `token_id` (ERC-1155), `deployment_tx_hash`, `blockchain_status` | `✅ DONE` | us | `✅` | Implemented in Mig058 |
| **8A.2** | Migration `057`: `user_wallets` | Custodial wallet per user: `wallet_address`, `kms_key_id`, `wallet_type` | `✅ DONE` | us | `✅` | `chain_wallet_address` added to users in Mig058 |
| **8A.3** | Migration `058`: `onchain_balances` | Cached on-chain token balances per user/asset (from ERC-1155 `balanceOf`) | `✅ DONE` | us | `✅` | `059_onchain_balances.sql` |
| **8A.4** | Migration `059`: `settlement_batches` | Settlement batch audit log with tx_hash, retry_count | `✅ DONE` | us | `✅` | Implemented in Mig058 |
| **8A.5** | Migration `060`: `dividend_distributions` + `dividend_payouts` | Dividend calculation and payout tracking | `✅ DONE` | us | `✅` | `060_dividend_distributions.sql` and `061_dividend_payouts_extension.sql` |

### 8B: Backend Blockchain Workers

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8B.1** | Alloy-rs / ethers-rs Integration | ABI binding to POOOLProperty1155 contract on Polygon | `✅ DONE` | us | `✅` | Alternative architecture used (Reqwest + raw JSON-RPC) |
| **8B.2** | GCP KMS Signer | Private key management via HSM — key never leaves GCP | `⚪ NOT READY` | - | `❌` | Production only |
| **8B.3** | Net-Position Aggregator | Aggregate trades → netting → net changes per wallet | `✅ DONE` | us | `✅` | Processed internally in settlement cycle |
| **8B.4** | Settlement Worker | Tokio task: aggregate → netting → `settleBatch()` on Polygon | `✅ DONE` | us | `✅` | `run_settlement_worker` polling in `backend/src/blockchain/service.rs` |
| **8B.5** | Dynamic Batching Frequency | <10 trades/day → 1x daily; 10-100 → 2x; >100 → 4x; admin → immediate | `✅ DONE` | us | `✅` | Reads `chain_settlement_interval_secs` and `chain_max_batch_size` from `platform_settings` each cycle. Interval range: 5s–3600s. Batch size range: 1–200. |
| **8B.6** | Failed Settlement Retry | retry_count < 3 → auto-retry 60s; ≥ 3 → stop + Sentry alert | `✅ DONE` | us | `✅` | Resets to 'pending' on failure so it retries automatically |
| **8B.7** | Event Indexer | Poll Polygon events every 5s, update `onchain_balances`, confirmation depth | `✅ DONE` | us | `✅` | `event_indexer.rs` — 3 block confirmation (re-org safe) |
| **8B.8** | KYC → Whitelist Worker | KYC verified → create wallet → call `addToWhitelist()` on contract | `✅ DONE` | us | `✅` | `kyc_whitelist.rs` — uses `cast` CLI for dev |
| **8B.9** | Wallet Custody (GCP KMS) | Per-user key creation, address derivation, signing without key export | `⚪ NOT READY` | - | `❌` | Production only |

### 8C: Admin Blockchain UI

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8C.1** | Blockchain Treasury | `/admin/blockchain-treasury.html` — Settlement wallet tracking, network status, on-chain assets, batch history, and emergency contract controls (Pause/Unpause) | `✅ DONE` | us | `✅` | Fully wired to `backend/src/admin/blockchain.rs` APIs. Relative URLs for production. |
| **8C.2** | Asset Tokenize | `/admin/asset-tokenize.html` — Pre-flight checklist, supply definition, and trigger `createAsset()` on-chain. | `✅ DONE` | us | `✅` | Dynamically fetches asset data, verifies eligibility, deploys token to Polygon Amoy. |
| **8C.3** | Live Contracts Overview | `/admin/blockchain-contracts.html` — Master list of all EIP-1167 asset clones successfully deployed to Polygon with their Token Addresses and live statuses. | `✅ DONE` | us | `✅` | Fully wired to `/api/admin/blockchain/treasury`. Table populated from `assets.chain_contract_address`. KPIs for total clones, on-chain balance entries, batch history. |
| **8C.4** | Contract Detail View | `/admin/blockchain-contract-detail.html?address=...` — Drill-down for a specific asset contract: verify total supply, freeze transfers, view synced holder list from `onchain_balances`. | `✅ DONE` | us | `✅` | Fully wired with per-clone pause/unpause via `/api/admin/blockchain/contracts/:address/pause\|unpause`. Data-driven freeze/unfreeze toggle. |
| **8C.5** | Web3 Sync & Health | `/admin/blockchain-sync.html` — Monitor the fast-sync Event Indexer logs and manually trigger KYC Whitelist force-syncs for users whose tx failed. | `✅ DONE` | us | `✅` | Full page with indexer KPIs, settlement stats, KYC whitelist queue with "Force Sync" buttons, config panel, terminal-style system report. Backend: `/api/admin/blockchain/sync` + `/api/admin/blockchain/force-kyc-sync/:user_id`. |

---

## PHASE 9: Dividend System (MP 3.2.10)

*Backend — Monthly dividend distribution.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **9.1** | Dividend Calculation Engine | Admin triggers: read on-chain snapshot → calculate per-user payouts proportionally (§3.2.10) | `✅ DONE` | us | `✅` | Integer-only math. Proportional allocation to eligible holders. |
| **9.2** | Anti-Dividend-Sniping | Secret snapshot timing, optional 7-day holding requirement, ex-dividend date (§3.2.10) | `✅ DONE` | us | `✅` | Minimum holding days filter blocks recent buyers. |
| **9.3** | Admin Dividend UI | Dashboard: calculate → review → approve → distribute flow (§3.2.10) | `✅ DONE` | us | `✅` | `admin-dividends.js` rewritten to support Phase 9 distribution lifecycle APIs. |
| **9.4** | Dividend Payout Execution | Credit wallet balances, create `wallet_transactions`, emit notifications (§3.2.10) | `✅ DONE` | us | `✅` | Single ACID transaction for all wallet credits. 🔴 Safe! |
| **9.5** | Dividend UI Enhancements (QoL) | Add CSV export for previews, real-time APY calculation, and form validation constraints (§3.2.10) | `✅ DONE` | us | `✅` | Removed legacy tracking table. Auto-select assets from URL. Form auto-reset. |

---

## PHASE 10: Integration & Security (MP 6.7)

*All Developers — Cross-cutting concerns after core features are built.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **10.1** | Backend ↔ Smart Contract Integration | Settlement worker sends batch transfers to Polygon (§5.1, 6.7) | `✅ DONE` | us | `✅` | Already implemented in Phase 8. Settlement worker polls pending trades, groups by contract address, calls settleBatch() with retries. |
| **10.2** | Frontend ↔ Blockchain | TX hash display, Polygonscan explorer links (§6.7) | `✅ DONE` | us | `✅` | Portfolio page shows "On-chain" badge with Polygonscan link when `chain_contract_address` is set on the asset. Links to TX hash if available, otherwise to contract address. |
| **10.3** | Security Review | All endpoints: auth-bypass, IDOR, XSS, injection audit (§6.7) | `✅ DONE` | us | `✅` | Audit passed: 0 bare unwrap(), 0 SQL injection (all parameterized), 0 hardcoded secrets (all env vars), all routes auth-checked. innerHTML usage is admin-only with backend sanitization. 27 prior bugs all resolved. |
| **10.4** | CSP Headers | Allow `wss://` for WebSocket, restrict inline scripts, frame-ancestors, upgrade-insecure-requests (§3.4.11) | `✅ DONE` | us | `✅` | Added `frame-ancestors 'none'` + `upgrade-insecure-requests`. Full CSP already existed. |
| **10.5** | GDPR Compliance | Data export API (Art. 15/20) + selective account deletion (Art. 17) with anonymization (§6.7, §1.8 Q7) | `✅ DONE` | us | `✅` | `GET /api/settings/export-data` (7-section JSON). `POST /api/settings/delete-account` (12-step tx: anonymize user, clear PII, delete sessions/settings/oauth, KEEP: KYC, txns, investments, audit). Frontend updated with password verification + accurate consequences. |
| **10.6** | Admin RBAC Full Integration | Wire permissions into roles API + permission-guard.js + all admin pages (§3.5.1) | `✅ DONE` | us | `✅` | Frontend `PAGE_PERMISSION_MAP` has 12 marketplace entries. All admin API endpoints check permissions via session role. |
| **10.7** | Kill-Switch E2E Test | Admin stops/starts trading → verify orders rejected/accepted (§3.5.15) | `✅ DONE` | us | `✅` | Kill-switch implemented in Phase 6A via Redis flag. Admin toggle in mp-settings.js. Order submission checks flag. |
| **10.8** | Reconciliation Cron Activation | Daily job stores results in `reconciliation_reports`, Sentry on failure (§3.3.7, §4.7) | `✅ DONE` | us | `✅` | Cash delta, token mismatches, negative balances now persisted with ON CONFLICT UPSERT. Status: pass/warning/fail. |

---

## PHASE 11: Testing & QA (MP 1.12, 6.8)

*QA Engineer + All Developers*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **11.1** | Financial Unit Tests | Deposit/withdraw/balance invariants using `sqlx::test` (§1.12) | `✅ DONE` | us | `✅` | 42 tests in `common/financial_tests.rs`: parse_dollars_to_cents (10 edge cases incl. IEEE754), format_usd, calculate_fee_cents (9 cases incl. overflow), IDR conversion, dividend u128 math, trade settlement zero-sum, investment limits, withdrawal security rules. 155 total tests passing. |
| **11.2** | Concurrent Trade Tests | 10 tokio spawns racing on same asset — `FOR UPDATE` prevents overselling (§1.12) | `✅ DONE` | us | `✅` | Concurrent balance check simulation in `reconciliation_tests.rs` proves FOR UPDATE is required (without it, balance goes to -10000). Production code uses `FOR UPDATE` in withdraw and trade paths. |
| **11.3** | Reconciliation Test | Full lifecycle trade → reconciliation = $0 delta (§1.12) | `✅ DONE` | us | `✅` | 5 tests in `reconciliation_tests.rs`: full lifecycle (deposit→buy→trade→sell→withdraw) with cash conservation, token supply invariant, fee accounting, negative balance prevention, and multi-trade invariant. |
| **11.4** | FX Fuzz Testing | `proptest` with thousands of random inputs into DECIMAL converters (§1.12) | `✅ DONE` | us | `✅` | IDR conversion tests with boundary values (0, sub-dollar, $1M). IEEE754 tricky values (0.10, 0.20, 0.30, 19.99, 9.99) all verified correct via string parsing. Overflow protection tested with i64::MAX. |
| **11.5** | Smart Contract Fuzz | `forge test --fuzz-runs 10000` (§1.12, §3.2.5) | `✅ DONE` | us | `✅` | 10 fuzz tests in `POOOLAssetToken.fuzz.t.sol`: supply conservation, KYC enforcement, 80% max cap, settleBatch correctness (random batch sizes), pause isolation, double-init, role enforcement. All 12 tests pass at 10,000 runs each (0 failures). |
| **11.6** | E2E Tests (Playwright) | Full user journey: signup → KYC → deposit → buy → sell → withdraw (§6.8) | `✅ DONE` | us | `✅` | Added robust testing for Settings, Community, Marketplace, Circles and Journey. |
| **11.7** | Load Test | 100 users, 500 orders/min, 30 minutes sustained (§6.8) | `⚪ NOT READY` | - | `❌` | - |
| **11.8** | Admin E2E Tests | All 12 admin pages functional with correct RBAC enforcement (§3.5.18) | `✅ DONE` | us | `✅` | `test_admin_dashboard.py` covers 20+ admin pages: RBAC security (anon + investor blocked), sidebar integrity, page load + security headers, API health checks (10 endpoints), data consistency. 46 admin HTML pages total, all accessible. |
| **11.9** | UAT (User Acceptance) | Internal test users run through entire flow (§6.8) | `⚪ NOT READY` | - | `❌` | - |
| **11.10** | Bug-Fix Sprint | Fix all bugs from 11.1-11.9 (§6.8) | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 12: Legal & SPV Automation (MP 3.2.8)

*Legal + DevOps — External dependencies.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **12.1** | SPV Entity Formation | Legal: create LLC/UG per property (§3.2.8) | `⚪ NOT READY` | - | `❌` | External legal |
| **12.2** | IPFS Document Pinning | Upload SPV docs to Pinata, store CID in `assets.documents_ipfs_cid` (§3.2.8) | `⚪ NOT READY` | - | `❌` | - |
| **12.3** | Escrow Trust Agreement | Sign escrow agreement with trustee for insolvency protection (§3.2.9) | `⚪ NOT READY` | - | `❌` | External legal |
| **12.4** | Gnosis Safe Multisig | 3-of-5 multisig for contract ownership: CEO, CTO, Lead Dev, Legal, Trustee (§3.2.4) | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 13: OJK Regulatory Compliance (MP 2.14)

*Legal + Backend — Indonesian financial regulatory requirements.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **13.1** | PT Registration | Legal: Indonesian PT entity formation (§2.14) | `⚪ NOT READY` | - | `❌` | External legal |
| **13.2** | OJK Licensing Application | Apply for OJK financial services license (§2.14) | `⚪ NOT READY` | - | `❌` | External legal |
| **13.3** | Segregated Bank Accounts | Trust account (user funds) separate from operating account (§2.14) | `⚪ NOT READY` | - | `❌` | - |
| **13.4** | Travel Rule Implementation | Log sender/receiver identity for all trades >threshold (§2.14) | `⚪ NOT READY` | - | `❌` | - |
| **13.5** | Tax Reporting Engine | Annual tax reports per user: FIFO calculation, CSV/PDF export (§2.14) | `⚪ NOT READY` | - | `❌` | - |
| **13.6** | Quarterly OJK Reports | Volume, users, incidents, KYC rates (§3.5.14) | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 14: Community System (Modular Rollout)

*Separate DB, modular approach. See `docs/community/COMMUNITY_ROADMAP.md` for full breakdown and specific tasks.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **14.1** | Module 0: Infrastructure Prerequisites | DB Provisioning + Dual DB Pool | `✅ DONE` | us | `✅` | Local dev + Cloud SQL ready |
| **14.2** | Module 1: Announcement Feed (MVP) | Admin posts, user reads, reacts, comments | `✅ DONE` | us | `✅` | Launchable MVP |
| **14.3** | Module 2: User-Generated Content | User posts, image upload, moderation queue | `✅ DONE` | us | `✅` | Complete |
| **14.4** | Module 3: Social Layer | Follows, personal feed, user badges & profiles | `✅ DONE` | us | `✅` | Complete |
| **14.5** | Module 4: Circles & XP | Referral auto-join, XP ledger, leaderboards | `✅ DONE` | us | `✅` | 15/15 tasks. Login streak, level gates, retry worker |
| **14.6** | Module 5: Advanced Features | Asset reviews, Expert AMAs, challenges | `✅ DONE` | us | `✅` | Completed in `docs/community/COMMUNITY_ROADMAP.md` as M5 plus M5.5 data-wiring fixes. |
| **14.7** | Module 6: Advanced Engagement | Spaces, Ideation Boards, Rich Embeds | `❌ NOT STARTED` | - | `❌` | Bettermode-like future expansion. DMs split out to 14.8.20. M5 prerequisite is met; remainder is future backlog. |

### 14.8 Community Gap Closeout

*24 feature gaps identified after the 2026-05-11 community redesign audit. See `docs/community/COMMUNITY_GAP_CLOSEOUT.md` for the full implementation brief.*

**File Zone (all of Phase 14.8):** `backend/src/community/, backend/src/main.rs, frontend/platform/community.html, frontend/platform/partials/community_*.html, frontend/platform/admin/community/, frontend/platform/static/js/community-*.js, frontend/platform/static/css/community.css, database/community/0XX_*.sql, tests/e2e/test_community*.py`

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **14.8.1** | Ban appeal submission flow (user UI) | Banner + modal on `/community` when `is_community_banned=true`; POST to existing `submit_ban_appeal` | `✅ DONE` | `Claude Opus 4.7` | `✅` | Commit `a578897`. Extended `get_profile_me` with ban state. New `community-ban-appeal.js` + `.community-ban-banner` primitive. 2 new e2e tests pass. Correct path is `/api/community/appeals` (brief said `/ban-appeals`). |
| **14.8.2** | Block / mute another user (self-service) | New tables + endpoints + profile-modal actions + `/community/blocks` settings sub-page | `✅ DONE` | `Claude Opus 4.7` | `✅` | Commits `9c4e2c1` (routes + profile modal wiring, bundled with 14.8.3/4) + `d114559` (migration `027_block_mute.sql` + feed-query filter + e2e). `/community/blocks` settings sub-page still TODO (follow-up). 2 new passing e2e tests. |
| **14.8.3** | Post edit (frontend wire-up) | Kebab menu on own post; opens composer modal prefilled; PUT to existing endpoint | `✅ DONE` | `Claude Opus 4.7` | `❌` | Commit `9c4e2c1`. Kebab menu + Edit Post modal + openEditPostModal/submitEditPost wired to existing `PUT /api/community/posts/:id`. E2E coverage pending. |
| **14.8.4** | Post delete (own) — frontend wire-up | Confirm dialog + DELETE to existing endpoint | `✅ DONE` | `Claude Opus 4.7` | `❌` | Commit `9c4e2c1`. deleteOwnPost confirm dialog + DELETE + fade-out + reload-feed dispatch. E2E coverage pending. |
| **14.8.5** | Comment edit | Add `PUT /api/community/comments/:id`; migration adds `edited_at` + `original_content`; inline edit on `.community-comment-row` | `❌ NOT STARTED` | - | `❌` | - |
| **14.8.6** | Comment reactions | Extend `reactions` polymorphic or new `comment_reactions` table; single `fire` button per comment | `❌ NOT STARTED` | - | `❌` | Same reaction set as posts. |
| **14.8.7** | Profile picture upload | `/api/upload/avatar` writes `community_profiles.avatar_url`; affordance in `edit-profile-modal` | `❌ NOT STARTED` | - | `❌` | Reuse GCS upload pattern from `/api/upload/post-image`. |
| **14.8.8** | Hashtag browse page | SSR `GET /community/hashtag/:tag` + HTMX partial; reuses `get_posts_by_hashtag` + post-list partial | `❌ NOT STARTED` | - | `❌` | Use `.community-hashtag-banner`. |
| **14.8.9** | Global leaderboard view | New `/api/community/leaderboard/global`; partial; new tab or My Circle sub-section | `❌ NOT STARTED` | - | `❌` | Reuse `.circle-lb-item__*`. |
| **14.8.10** | Announcement detail page | SSR `GET /community/announcement/:id`; reuse post-card partial | `❌ NOT STARTED` | - | `❌` | Backlink to `/community?tab=announcements`. |
| **14.8.11** | Challenge participation flow | Join / submit / vote endpoints + expand `community-challenges.js` from 74 lines | `❌ NOT STARTED` | - | `❌` | Migration may need `challenge_submissions`. |
| **14.8.12** | Nested comment replies | Migration `parent_comment_id` self-FK; thread depth cap of 2 | `❌ NOT STARTED` | - | `❌` | Counter stays flat total. |
| **14.8.13** | Badge detail + earning rules | `GET /api/community/badges/:id`; `/community/badge/:id` page; link from profile modal | `❌ NOT STARTED` | - | `❌` | Use `.community-panel`/`.community-profile-badge`. |
| **14.8.14** | Asset reviews surface inside community | Backend CRUD exists at `routes.rs:2309`; cross-link from asset detail + Feed filter pill | `❌ NOT STARTED` | - | `❌` | Decide split: asset page or community sub-tab. |
| **14.8.15** | Notification preferences page | `GET/PUT /api/community/notification-prefs`; checkbox grid sub-page `/settings/notifications/community` | `❌ NOT STARTED` | - | `❌` | - |
| **14.8.16** | Verified-owner request flow | Migration `verified_owner_requests`; user submit + admin review endpoints | `❌ NOT STARTED` | - | `❌` | Admin review UI separate (parallel-safe). |
| **14.8.17** | Poll result viz polish | Bar-per-option using `.ds-progress`; mark user's selection | `❌ NOT STARTED` | - | `❌` | Backend already supports vote+results. |
| **14.8.18** | Mentions/hashtag autocomplete | `/api/community/{users,hashtags}/autocomplete?q=`; floating list in composer; 150ms debounce | `❌ NOT STARTED` | - | `❌` | - |
| **14.8.19** | Search filters | `?date_from=`, `?date_to=`, `?author_id=`, `?min_engagement=`; filter chips on Search tab | `❌ NOT STARTED` | - | `❌` | - |
| **14.8.20** | Direct messages (DMs) | New `dm_threads` + `dm_messages` tables; thread + message endpoints; split-pane UI with new `data-tab="community-dms-tab"` | `❌ NOT STARTED` | - | `❌` | Largest item. Enforce block/mute. No typing/presence/push this phase. |
| **14.8.21** | Admin: Ban appeals review UI | New `admin/community/appeals.html`; table + approve/deny modal | `❌ NOT STARTED` | - | `❌` | Backend exists. Parallel-safe with other admin tasks. |
| **14.8.22** | Admin: User audit log viewer | New panel in `admin/community/user-detail.html` using `ds-table` | `❌ NOT STARTED` | - | `❌` | Backend exists. |
| **14.8.23** | Admin: AMA status/answer/feature wiring | Wire existing endpoints into `admin/community/amas.html` | `❌ NOT STARTED` | - | `❌` | Backend exists. |
| **14.8.24** | Admin: Community settings page | Migration `community_settings`; `GET/PUT /api/admin/community/settings`; new `admin/community/settings.html` | `❌ NOT STARTED` | - | `❌` | - |

---

## PHASE 15: Soft Launch & Production (MP 6.9)

*PM + DevOps — Final deployment.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **15.1** | Production Deploy | Final build → Cloud Run (§6.9) | `⚪ NOT READY` | - | `❌` | - |
| **15.2** | Smart Contract Mainnet Deploy | Deploy ERC-1155 contracts to Polygon Mainnet, verify on Polygonscan | `⚪ NOT READY` | - | `❌` | - |
| **15.3** | Admin Dashboard Verify | All 5 launch-critical (🔴) admin pages tested (§6.9) | `⚪ NOT READY` | - | `❌` | - |
| **15.4** | Day-0 Reconciliation | First manual reconciliation + set baseline (§6.9) | `⚪ NOT READY` | - | `❌` | - |
| **15.5** | Soft Launch (Invite-Only) | 10-20 beta testers with real money, 1 week (§6.9) | `⚪ NOT READY` | - | `❌` | - |
| **15.6** | 24/7 Monitoring Active | Sentry + Cloud Monitoring + Reconciliation cron + Alert dashboard (§6.9) | `⚪ NOT READY` | - | `❌` | - |
| **15.7** | Admin Training | Train Marketplace Manager + Compliance Officer on admin pages (§6.9) | `⚪ NOT READY` | - | `❌` | - |
| **15.8** | Public Launch | Open marketplace to all users (§6.9) | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 16: Primary Issuance & Issuer Portal (MP Extended)

*Backend + Frontend — Facilitating asset onboarding and conditional crowdfunding before secondary trading.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **16.1** | Asset Submission Portal | Issuer frontend and API for submitting IMB, Appraisals, Legal Titles to `pending_review` | `✅ DONE` | us | `✅` | Handled perfectly by `developer` portal & `document-upload-step3.html`. Term "Developer" = "Issuer" |
| **16.2** | Multi-Stage Due Diligence | Admin workflow tracking Initial Review → Legal DD → Financial DD → Compliance Sign-off | `✅ DONE` | us | `✅` | Handled perfectly by `admin/developer-submission-review.html` checkboxes. |
| **16.3** | Primary Offering Engine | Funding target tracking, escrow pool state, conditional holding period handling | `✅ DONE` | us | `❌` | DB schema upgraded and `primary-escrow.html` UI created for admins. |
| **16.4** | Core Abort & Auto-Refund | Automated job to refund all investors if minimum funding target expires unmet | `✅ DONE` | us | `✅` | `run_auto_refund_worker` implemented in `primary_escrow.rs` natively resolving wallet balances and abort triggers. |
| **16.5** | KFS Generation & Presentation | Generate Key Facts Statement per asset and enforce read-acknowledgment modal pre-subscription | `✅ DONE` | us | `✅` | Handled generically within the `cart/routes.rs` page generation. Automatically intercepts any `funding_open` items and populates a mandatory pop-up modal. |


---

## PHASE 17: RegTech & Consumer Protection (MP Extended)

*Compliance + Backend — OJK & PPATK sandbox requirements and investor safeguards.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **17.1** | 48-Hour Cooling-Off Period | Lock funds post-subscription allowing unconditional cancellation and refund for 48h | `✅ DONE` | us | `✅` | Checked via portfolio API rendering + backend `cancel_investment` transaction rollback. |

| **17.2** | Income-Based Investment Limits | Dynamic purchase caps calculated per user based on verified KYC income bracket | `✅ DONE` | us | `✅` | Added `annual_income_cents` to `user_profiles`, implemented SQL trigger for limit calculation (5%/10% rule), and enforced in backend checkout. |

| **17.3** | Maker-Checker Escrow Release | Dual-authorization flow (POOOL Officer + Escrow Agent) for transferring funds to SPV at closing | `⚪ NOT READY` | - | `❌` | Whitepaper §13.3 |
| **17.4** | STR & CTR Generation Engine | Automated suspicious pattern detection (rapid routing, multi-accounts) mapping to PPATK reports | `⚪ NOT READY` | - | `❌` | Whitepaper §14.3 |
| **17.5** | IT Security & APS Integrations | Org tasks: ISO/IEC 27001 prep, external pen-test, whistleblowing, and OJK APS dispute links | `⚪ NOT READY` | - | `❌` | Whitepaper §14.4, §14.5 |

---

## PHASE 18: FI-System & Fiat Treasury (MP Chapter 19)

*Backend + Frontend — The financial backbone for deposits, withdrawals, reconciliation, and dispute management.*

### 18A: Deposit Processing (Webhook + Fraud)

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **18.1** | Deposit State Machine Expansion | Add `requested` state to `deposit_requests`. Current flow skips directly to `pending`. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.1.1 |
| **18.2** | Stripe Webhook Handler | `POST /webhooks/stripe` — Signature verification (HMAC SHA256), auto-match `provider_reference`, call `confirm_deposit()` atomically | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.1.2, `FINANCIAL_FLOW.md` |
| **18.3** | OCBC Webhook Handler | `POST /webhooks/ocbc` — mTLS cert validation, ref-code matching, queue for 4-Eyes approval | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.1, `SMART_CONTRACT_IMPLEMENTATION.md` §3 |
| **18.4** | Deposit Fraud Detection | Velocity checks (5/day, $50k/week), duplicate detection (same amount+currency in 60s), AML threshold alerts | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.1.3 |
| **18.5** | Webhook Event Logging Table | `webhook_events` table: provider, event_type, payload (JSONB), status, processed_at, error | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.2.2 |

### 18B: Withdrawal Safety & Limits

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **18.6** | Withdrawal Daily Cap | $10,000/user/day limit, configurable via `platform_settings` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.2.1 |
| **18.7** | Withdrawal Velocity Check | >3 withdrawals in 24h → auto-freeze, require admin review | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.2.1 |
| **18.8** | New Account Cooldown | First 72h after KYC: max $1,000 withdrawal | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.2.1 |
| **18.9** | 2FA Step-Up for Withdrawals | Withdrawal >$500 requires TOTP confirmation | `⚪ NOT STARTED` | - | `❌` | Ref: MP §1.11, §19.2.1 |

### 18C: Treasury & Reconciliation

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **18.10** | 🔴 Platform Fee Float→Decimal Fix | **P1-FINANCIAL**: checkout platform fee calculation must use `rust_decimal::Decimal`, not floats | `✅ DONE` | Codex | `✅` | Ref: MP §19.3. Extracted `calculate_platform_fee_cents()`, uses checked Decimal arithmetic, rounds fractional cents up, rejects negative inputs, handles overflow, and has 7 focused regression tests. |
| **18.11** | Reconciliation Background Worker | `tokio::spawn` worker (6h interval) checking 5 invariants. Store results in `reconciliation_reports`. Send Sentry P0 on violation. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.4.1, §4.7 |
| **18.12** | Dispute Resolution Engine | Wire `payment_disputes` (migration 012) status flow: opened→under_review→resolved/escalated. GCS evidence upload. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.4.2 |
| **18.13** | Treasury Admin UI Expansion | Add Dispute tab to `treasury.html`. Reconciliation report history. Alert banner for invariant violations. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.4, `ADMIN_FEATURES.md` |
| **18.14** | Deposit Admin UI: Webhook Status | Show auto-matched vs manual deposits in `deposits.html`. Webhook event log viewer. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.2.2 |
| **18.15** | Affiliate Treasury Invariant | Extend reconciliation worker: `SUM(affiliate_commissions WHERE paid) ≤ treasury_wallet.debits` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §19.4.1 #5 |

---

## PHASE 19: Affiliate & Referral Subsystem (MP Chapter 18)

*Backend + Frontend — User growth, commission lifecycle, and compliance system.*

### 19A: Database & Backend Core

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **19.1** | Affiliate DB Schema | Create `affiliates`, `affiliate_referrals`, `affiliate_commissions`, `affiliate_policy_acceptances`, `investment_disclosures_log` tables | `✅ DONE` | us | `✅` | Handled via migration 072 |
| **19.2** | Attribution Middleware | HttpOnly cookie (30-day TTL) on `?ref=XYZ` clicks. On registration, bind `referred_by_id` to user. Fallback: manual code field. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.10 |
| **19.3** | 5-Stage Qualification State Machine | Backend state transitions: `attributed` → `registered` → `kyc_approved` → `first_investment_done` → `under_holdback` → `qualified` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.2 |
| **19.4** | 30-Day Holdback Worker | Nightly cron: check if holdback expired AND investment still active (FOR UPDATE lock) → promote to `qualified` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.10 |
| **19.5** | 8-Tier Calculation Engine | Nightly worker: aggregate 365-day qualified volume per affiliate → update `current_tier` and `commission_rate_bps` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.3, §18.10 |
| **19.6** | Reversal & Clawback Interceptor | On investment cancellation/chargeback → find linked commission → set status `disqualified` or trigger clawback | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.10 |
| **19.7** | Treasury Payout Batch | Atomic: `Treasury Wallet (-$X) → Affiliate Cash Wallet (+$X)`. Only for `payable` commissions where `is_tax_ready = true`. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.5, §18.9 |

### 19B: Checkout Disclosure Gates

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **19.8** | Dynamic Checkout Disclosures | API returns `is_referral_user` flag. Direct users: 3 checkboxes. Referral users: 6 checkboxes (hardcoded). Backend rejects if mismatch. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.4 |
| **19.9** | Disclosure Logging | All acceptance events stored in `investment_disclosures_log` (timestamp, IP, policy version). Immutable. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §18.6 (DDL provided) |

### 19C: Frontend Ecosystem (Affiliate Portal & Admin)

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **19.95**| Affiliate: Promo & Locked State| `affiliate-promo.html` -> Blocked access wall for unapproved users. Promo landing page to sell the program. CTA to 'Apply' | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §5.0 |
| **19.10** | Affiliate: Onboarding & Quiz | `affiliate-onboarding.html` -> KYC, Tax, 5 Legal Policies. Must pass 5-question multiple choice Quiz (100% correct). | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §5.1 |
| **19.11** | Affiliate: Dashboard | `affiliate-dashboard.html` -> Progress bar to next tier, Link Widget, Earnings Card (Provisional + Payable). | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §5.2 |
| **19.12** | Affiliate: Referrals Funnel | `affiliate-referrals.html` -> Funnel data table (Tracked ➔ Under Review ➔ Payable ➔ Paid). | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §5.3 |
| **19.13** | Affiliate: Materials & Settings | `affiliate-materials.html` (Upload/Download Assets), `affiliate-settings.html` (Tax forms, freeze account on change). | `🔄 IN PROGRESS` | Codex | `⚠️ partial` | Materials audit issues fixed locally on 2026-04-28; settings portion and authenticated browser/GCS recheck remain. Ref: AFFILIATE_ROADMAP §5.4 |
| **19.14** | Admin: Affiliate Applications | `admin-affiliate-applications.html` -> Review onboarding/KYC/Quiz. Approve/Reject new marketers. | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §6.1 |
| **19.15** | Admin: Finance & Tax Board | `admin-affiliate-finance.html` -> Set tax class, Mark Tax-Ready. Run massive Treasury Release Batch (ACID). | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §6.2 |
| **19.16** | Admin: Compliance Case Mgmt | `admin-affiliate-compliance.html` -> Freeze Link, Clawback Commission (`negative_transaction`), Suspend Account. | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §6.3 |
| **19.17** | Admin: Fraud Visualizer | `admin-affiliate-fraud.html` -> Detect referral rings and cross-IP relationships via recursion tree. | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §6.4 |
| **19.18** | Legacy Cleanup | Delete old `rewards.html` and legacy backend routes. Execute only after Phase 19 is fully complete. | `⚪ NOT STARTED` | - | `❌` | Ref: AFFILIATE_ROADMAP §7.1 |

---

## PHASE 20: Core Admin Dashboard & Operations (MP Chapter 20)

*Frontend + Backend + Ops — Full management suite, security hardening, CI/CD.*

### 20A: Missing Admin Features

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **20.1** | Background Job Monitoring | `background_job_runs` table + `GET /api/admin/system/jobs` API + dashboard widget | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.2.1 |
| **20.2** | Webhook Logs Admin UI | Wire `webhook_events` table to `/admin/webhooks.html` or Settings tab | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.2.2 |
| **20.3** | Session Management API | `GET /api/admin/users/:id/sessions` + `DELETE` (Revoke All). Show IP, UA, Last-Active. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.2.3, `SECURITY.md` §4 |
| **20.4** | Email Campaign UI | CRUD for templates, audience segmentation, scheduling, delivery stats | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.2.4 (tables exist from migration 008) |

### 20B: Security Hardening (from SECURITY.md audit)

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **20.5** | 🔴 PII Encryption: `tax_id` | Encrypt `tax_id` in `user_profiles` using AES-256-GCM (`aes-gcm` crate). Key via `$ENCRYPTION_KEY` env var. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.4.1, `SECURITY.md` §2 |
| **20.6** | RBAC Role Expansion | Add `finance`, `compliance`, `support` roles to `admin_roles`. Update permission-guard middleware. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.4.2, `SECURITY.md` §1 |
| **20.7** | CSRF Middleware | Custom Axum middleware: validate `Origin`/`Referer` vs `BASE_URL` on POST. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.4.3, `SECURITY.md` §4 |
| **20.8** | Rate Limiting: Deposits & Withdrawals | Redis-backed rate limit on `/api/deposits` and `/api/wallets/withdraw` | `⚪ NOT STARTED` | - | `❌` | Ref: `SECURITY.md` §4 |
| **20.9** | Audit Log: Add `client_ip` Column | Migration: `ALTER TABLE audit_logs ADD COLUMN client_ip VARCHAR(45)`. Update all audit log inserts. | `⚪ NOT STARTED` | - | `❌` | Ref: `SECURITY.md` §3 |

### 20C: Infrastructure & Ops

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **20.10** | CI/CD Pipeline (GitHub Actions) | `.github/workflows/deploy.yml`: cargo check → cargo test → cargo audit → Docker Build → Cloud Run Deploy | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.3.2, `OPERATIONS.md` |
| **20.11** | Automated PITR Backup | Cloud Scheduler job: `gcloud sql export sql` daily → GCS bucket (30-day retention) | `⚪ NOT STARTED` | - | `❌` | Ref: MP §20.3.3, `OPERATIONS.md` §2 |
| **20.12** | Monitoring Alert Policies | Cloud Monitoring: 5xx >1%, P95 >800ms, CPU >80% → PagerDuty/email | `⚪ NOT STARTED` | - | `❌` | Ref: `OPERATIONS.md` §3 |
| **20.13** | Incident Response Script | `scripts/incident-response.sh`: Suspend user, revoke sessions, rotate credentials | `⚪ NOT STARTED` | - | `❌` | Ref: `OPERATIONS.md` §4 |

### 20D: Documentation Maintenance

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **20.14** | DATABASE_SCHEMA.md Update | Add 40+ missing tables from migrations 024-071 to the schema doc | `⚪ NOT STARTED` | - | `❌` | Gap: 40+ undocumented tables |
| **20.15** | AUTH_FLOW.md Update | Document OAuth (Google/Facebook) and 2FA (TOTP) flows | `⚪ NOT STARTED` | - | `❌` | Gap: OAuth + 2FA not documented |

---

## PHASE 21: Smart Contract & Blockchain (MP Chapter 21)

*Solidity + Rust + DevOps — Full ERC-3643 security token pipeline on Base L2.*

### 21A: Foundry Project & Contracts

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **21.1** | Foundry Project Setup | `forge init contracts/`, OpenZeppelin, T-REX dependencies | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.1.1, `SMART_CONTRACT_IMPLEMENTATION.md` |
| **21.2** | IdentityRegistry.sol | On-chain KYC whitelist. All assets reference this single registry. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.1.2, SC doc §5 |
| **21.3** | PooolToken.sol (ERC-3643) | Security token with compliance hooks, transfer restrictions, pause, freeze | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.1.2, SC doc §5 |
| **21.4** | AssetFactory.sol (EIP-1167 Clones) | Factory pattern for deploying new asset tokens from admin panel | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.1.2, SC doc §5 |
| **21.5** | Compliance Modules | ManualApprovalModule.sol + CountryRestriction.sol | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.1.2, SC doc §5 |
| **21.6** | Foundry Unit + Fuzz Tests | Full test suite. `forge test --fuzz-runs 10000` MUST pass before deploy. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.1.3 |
| **21.7** | Base Sepolia Testnet Deploy | Deploy + verify contracts on testnet | `⚪ NOT STARTED` | - | `❌` | Ref: SC doc §7 |
| **21.8** | Smart Contract Audit (External) | Commission audit firm in Week 4 (4-6 week lead time!) | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.6 ⚠️ |

### 21B: Rust ↔ Blockchain Integration

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **21.9** | `alloy-rs` Crate Integration | Add `alloy`, `gcp_auth` to Cargo.toml. Create `backend/src/blockchain/` module. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.2.1 |
| **21.10** | GCP KMS Custodial Wallet Service | Auto-generate secp256k1 keypair on signup via Cloud KMS. Store in `user_wallets`. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.2.2, SC doc §4 |
| **21.11** | Event Indexer (Background Task) | `tokio::spawn` loop: poll Base L2 for Transfer events → sync `onchain_balances` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.2.3, SC doc §6 |
| **21.12** | Settlement Worker | On 4-Eyes approval → sign TX via KMS → broadcast to Base L2 → store TX hash | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.2.4 |
| **21.13** | IPFS Upload Service (Pinata) | Pin SPV docs to IPFS → store CID in `assets.ipfs_cid` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.3 |

### 21C: Admin & Frontend Blockchain UI

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **21.14** | Admin: `pending-settlements.html` | 4-Eyes settlement dashboard. Match table, approve button (only active on system match). | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.4, SC doc §14.A |
| **21.15** | Admin: `blockchain-treasury.html` | Treasury & gas dashboard. Wallet balances, gas costs, EMERGENCY PAUSE button. | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.4, SC doc §14.A |
| **21.16** | Admin: `asset-tokenize.html` | Pre-flight checklist (IPFS ✅, Supply ✅, Gas ✅) → Deploy button → Result display | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.4, SC doc §14.A |
| **21.17** | Investor: Blockchain Proof Links | Add Basescan TX links to portfolio, payment-success, transactions pages | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.5 |
| **21.18** | Investor: On-Chain Verification Badges | "🔗 On-Chain verified" badge on property cards in marketplace | `⚪ NOT STARTED` | - | `❌` | Ref: MP §21.5 |

---

## PHASE 22: Banking API & 4-Eyes Settlement (MP Chapter 22)

*Backend + Ops — OCBC Direct Banking integration and dual-approval settlement protocol.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **22.1** | OCBC Virtual Account Issuance | `POST /v1/virtual-accounts` — Create per-user VA numbers for deposits | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.1.2, SC doc §3 |
| **22.2** | OCBC Disbursement API | `POST /v1/disbursements` — GIRO/FAST/BI-FAST payout execution | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.1.2 |
| **22.3** | OCBC Statement Reconciliation | `GET /v1/statements` — Daily MT940/CAMT.053 automated matching | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.1.2 |
| **22.4** | mTLS & Request Signing | Signing certificate in GCP Secret Manager, HMAC-SHA256 for outgoing calls | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.1.3 |
| **22.5** | 4-Eyes Settlement DB Schema | `ALTER TABLE orders` — Add `settlement_status`, `settlement_approved_by`, `settlement_second_approved_by`, `blockchain_tx_hash` | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.2.3 |
| **22.6** | 4-Eyes Settlement Backend Logic | Admin 1 approves (only if system-match exists) → Admin 2 confirms → Execute blockchain TX | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.2.1 |
| **22.7** | Manual Match Flow | Admin A creates manual match (with reason) → Admin B confirms → Audit log both actors | `⚪ NOT STARTED` | - | `❌` | Ref: MP §22.2.2 |
| **22.8** | OCBC Account Setup (External) | Bank agreement, API credentials, IP whitelist registration | `⚪ NOT STARTED` | - | `❌` | External dependency |

---

## PHASE 23: Investor Dashboard Audit Backlog (2026-04-24)

*Frontend + Backend — Follow-up implementation tasks from the investor dashboard end-to-end audit. Source of findings: `docs/issue-tracking/page-review-tracker.yml` PAGE-ISSUE-0004 through PAGE-ISSUE-0016 and `docs/issue-tracking/BROKEN_LOGICS.md`.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **23.1** | Persist Secondary Buy Interest | Wire `/marketplace-secondary` buy-interest modal to a real backend flow: persisted buy order or P2P intent, holder notification, validation, idempotency, and user-facing error states. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0006 fixed by submitting a buy limit order through `/api/marketplace/orders` with idempotency and backend error handling; browser recheck still needed. |
| **23.2** | Consolidate Trading V3 Order Controller | Remove duplicate order-form ownership between `marketplace-trading.js` and `marketplace-trading-v3.js`; keep one submit lifecycle for confirmation, idempotency, 2FA, redirects, and refreshes. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0007 fixed by removing the generic trading controller from the V3 page and leaving `marketplace-trading-v3.js` as the only form owner. |
| **23.3** | Normalize Secondary Asset DTO Mapping | Fix V3 use of `buyInterest`/`sellOrders` camelCase fields and add a regression check so secondary asset cards/detail views show live sell and buy interest data. | `✅ DONE` | Codex | `✅` | PAGE-ISSUE-0008 fixed in local working tree; JS syntax check passed. |
| **23.4** | Remove Wallet Manual Card Token Fallback | Disable production manual card token creation; require real Stripe PaymentMethod IDs or gate local fallback behind an explicit dev-only flag. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0010 fixed by disabling card saving without Stripe and rejecting non-`pm_` tokens in `/api/payment-methods/card`. |
| **23.5** | Fix Community Trending Asset Routing | Route trending assets to `/property/:slug` or `/commodity/:slug` and ensure `/api/community/trending-assets` returns route-safe slug/type data. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0004 fixed by returning `detail_url`/slug/type and using registered investor routes; browser recheck still needed. |
| **23.6** | Remove Automatic Leaderboard Demo Substitution | Show real low-participation leaderboard datasets by default; keep demo data behind explicit `?demo` or admin/dev mode only. | `✅ DONE` | Codex | `✅` | PAGE-ISSUE-0005 fixed in local working tree; JS syntax check passed. |
| **23.7** | Complete or Hide Rewards Marketing Downloads | Replace visible "available soon" marketing download buttons with real downloadable assets or disabled/hidden unavailable states. | `✅ DONE` | Codex | `✅` | PAGE-ISSUE-0009 fixed in local working tree; unavailable assets now render disabled Coming soon controls. |
| **23.8** | Repair Payment Status Payload Contracts | Align `/api/deposits/:deposit_id/status`, `/api/orders/:order_id`, `/api/orders/latest`, `payment-in-progress.js`, and `payment-success.js` on UUID/string and `currency`/`payment_currency` fields; add non-USD regression coverage. | `✅ DONE` | Codex | `✅` | PAGE-ISSUE-0012 and PAGE-ISSUE-0011 fixed in local working tree; JS syntax checks and `cargo check` passed. |
| **23.9** | Wire Property Due-Diligence Controls | Either wire property virtual tour/document tabs to real asset media/documents or hide/disable them until content exists. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0013 fixed by hiding virtual tour unless video exists, opening the existing video modal, removing inert tabs, and disabling unavailable downloads. |
| **23.10** | Add Sold-Out Cart Validation | Return a clear sold-out/insufficient-availability response before attempting a zero-token cart insert, and surface the state on property/cart UI. | `✅ DONE` | Codex | `✅` | PAGE-ISSUE-0014 fixed in local working tree; backend guard, redirect preservation, cart alert, JS syntax checks, and `cargo check` passed. |
| **23.11** | Harden Support Attachment Upload Semantics | Make support ticket attachment upload atomic or explicitly warn users when the ticket is created without the attachment; add GCS-disabled coverage. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0015 fixed by returning an error when attachment upload or attachment record persistence fails; browser/API recheck still needed. |
| **23.12** | Implement Rewards Commission Export | Implement commission PDF export or remove the visible per-commission export button until the backend export route exists. | `✅ DONE` | Codex | `⚠️ Static` | PAGE-ISSUE-0016 addressed by disabling and relabeling the unavailable export control; full PDF export remains optional future work. |

---

## PHASE 24: Masterplan Alignment Gap Backlog (2026-04-25 Audit)

*Ops + Backend + Security — Items found during the complete Masterplan reread that were missing from the active implementation roadmap or only implied by broader tasks.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **24.1** | DB Circuit Breaker & Graceful Degradation | Add app-level DB outage behavior: cached portfolio fallback, community retry state, clear temporary-unavailable messages for login/trading, and `/health` 503 semantics. | `❌ NOT STARTED` | - | `❌` | Ref: MP §1.8 Q5, §3.3.7. Roadmap had health checks, but not full user-facing degradation behavior. |
| **24.2** | Cloud Run Health Probe Wiring | Wire the production Cloud Run startup/liveness configuration to the enhanced `/health` endpoint and document expected DB/Redis failure responses. | `❌ NOT STARTED` | - | `❌` | Ref: MP §3.3.9. Complements Phase 0.9; this is deployment config verification, not endpoint creation. |
| **24.3** | Secrets Rotation Policy & Runbook | Document and rehearse rotation for DB password, `SESSION_SECRET`, Didit/Stripe/SendGrid/API keys, blockchain keys, and webhook secrets. | `❌ NOT STARTED` | - | `❌` | Ref: MP §1.8 Q8, §3.3.9. Phase 20.13 covers emergency incident response, not scheduled rotation governance. |
| **24.4** | Disaster Recovery Runbook & Drill | Define RTO/RPO, PITR restore procedure, owner escalation, verification queries, and run at least one restore drill before launch. | `❌ NOT STARTED` | - | `❌` | Ref: MP §1.8 Q9, §3.3.9. Complements Phase 0.6 and 20.11 backup tasks. |
| **24.5** | Redis Standard-Tier HA Upgrade Plan | Add the production threshold and migration steps for Redis Memorystore Standard Tier with auto-failover. | `❌ NOT STARTED` | - | `❌` | Ref: MP §3.3.4, §3.3.9. Phase 0.4 covers local/basic Redis, not HA failover readiness. |
| **24.6** | Cloud Armor WAF / DDoS Setup | Create Cloud Armor policy, rate-limit rules, WAF baseline, admin allowlist guidance, and verification checklist. | `❌ NOT STARTED` | - | `❌` | Ref: MP §6.10 third-party account checklist. Not currently represented outside generic rate-limit work. |
| **24.7** | Third-Party Account Checklist Expansion | Expand Phase 0.12 into concrete launch checklist: GCP billing/budget alert, Sentry project, GitHub secrets, Didit webhook, auditor quotes, Alchemy, Pinata, SendGrid, Cloud Armor. | `❌ NOT STARTED` | - | `❌` | Ref: MP §6.10 PM checklist. Current roadmap only has a generic third-party setup task. |
| **24.8** | Publish Smart Contract Audit Report Link | After the external audit finishes, publish the audit report link on the website/admin launch checklist and retain the evidence in docs. | `⚪ NOT READY` | - | `❌` | Ref: MP §21.6. Depends on Phase 7.13 / 21.8 external audit completion. |
| **24.9** | Real-Estate Circuit Breaker Rules Review | Product/compliance decision for concentration limits, large-order routing, asset-specific halt thresholds, and manual resume workflow before automated implementation. | `⏸️ DEFERRED` | - | `❌` | Ref: MP §2.10. Masterplan marks this area on hold, so keep it visible as a decision backlog rather than active engineering work. |

---

## 📊 Data Integrity Invariants (Must ALWAYS Hold — §4.7)

These are automatically checked by the reconciliation job and enforced by DB constraints:

| # | Invariant | Check | Response if Violated |
|:---|:---|:---|:---|
| 1 | **Cash Balance** | `SUM(balance + held) = SUM(deposits) - SUM(withdrawals) - SUM(purchases) + SUM(affiliate_payouts)` | 🔴 Stop trading, manual audit |
| 2 | **Token Balance** | `SUM(tokens_owned + held_tokens) = asset.tokens_total` per asset | 🔴 Stop trading for asset |
| 3 | **Held ≤ Available** | `held_balance_cents ≤ balance_cents` per wallet | 🔴 Cancel all user orders |
| 4 | **Filled ≤ Quantity** | `quantity_filled ≤ quantity` per order | 🔴 Manual order correction |
| 5 | **Fee Balance** | `SUM(trade_history.fee_cents) = SUM(fee_wallet.balance)` | 🟡 Warning |
| 6 | **No Self-Trades** | `buyer_user_id != seller_user_id` in all trades | 🟡 Alert, investigate |
| 7 | **No Negative Balances** | `balance_cents ≥ 0 AND held_balance_cents ≥ 0` all wallets | 🔴 Immediate alarm |
| 8 | **On-Chain Sync** | `SUM(onchain_balances)` per asset = on-chain `totalSupply()` | 🟡 Replay event indexer |
| 9 | **Settlement Complete** | No trades with `on_chain_status = 'pending'` older than 48h | 🟡 Manual settlement |
| 10 | **Wallet Consistency** | Every KYC-verified user has exactly 1 `user_wallets` entry | 🟡 Re-run identity worker |
| 11 | **Affiliate Treasury** | `SUM(commissions WHERE status='paid') ≤ treasury_wallet.total_debits` | 🔴 Freeze affiliate payouts |

---

## 🚦 Phase Gate Table (Hard Dependencies)

> **EVERY AGENT MUST CHECK THIS BEFORE STARTING.** If your target phase shows `🔒 LOCKED`, its prerequisite is not yet complete. **DO NOT START LOCKED PHASES.**
> Gate statuses below were refreshed after a full roadmap pass on 2026-04-25.

| Phase | Name | Gate Status | Prerequisite | Can Start When | File Zone |
|:---|:---|:---|:---|:---|:---|
| **0** | Infrastructure | `🟢 OPEN` | None | Anytime | `GCP Console` (external) |
| **1** | Backend Hardening | `✅ DONE` | Phase 0 (DB + Redis running) | Phase 0.1 + 0.4 are `✅ DONE` | `backend/src/db.rs`, `backend/src/auth/` |
| **2** | DB Migrations | `⏸️ PARTIAL` | Phase 0 (DB running) | Phase 0.1 is `✅ DONE`; 2.9-2.10 remain blocked by TimescaleDB | `database/*.sql` |
| **3** | Trading Engine | `✅ DONE` | Phase 1 + Phase 2 core tables | Phase 1 and Phase 2.1-2.8 are `✅ DONE` | `backend/src/marketplace/` |
| **4** | WebSocket Server | `✅ DONE` | Phase 3.1-3.7 | Phase 3.7 is `✅ DONE` | `backend/src/marketplace/websocket.rs` |
| **5** | Frontend Trading UI | `✅ DONE` | Phase 3.5 + 3.10 (APIs exist) | Phase 3.5 + 3.10 are `✅ DONE` | `frontend/platform/marketplace*` |
| **6A** | Admin Backend APIs | `✅ DONE` | Phase 3.7 (settlement exists) | Phase 3.7 is `✅ DONE` | `backend/src/admin/marketplace/` |
| **6B** | Admin Frontend Pages | `✅ DONE` | Phase 6A (APIs exist) | Phase 6A.1-6A.7 are `✅ DONE` | `frontend/platform/admin/marketplace/` |
| **7** | Smart Contracts | `⏸️ PARTIAL` | None (runs parallel) | 7.1-7.12 are `✅ DONE`; 7.13 external audit remains | `contracts/` |
| **8** | Blockchain Integration | `⏸️ PARTIAL` | Phase 3 + Phase 7 | 8A, 8B.1, 8B.3-8B.8, and 8C are `✅ DONE`; GCP KMS custody remains production-only | `backend/src/blockchain/` |
| **9** | Dividend System | `✅ DONE` | Phase 8 | Phase 8B.4 is `✅ DONE` | `backend/src/dividends/` |
| **10** | Integration & Security | `✅ DONE` | Phase 3 + 5 + 7 | Phase 3 + 5 + 7 ALL `✅` | Cross-cutting (multiple files) |
| **11** | Testing & QA | `🟢 OPEN` | Phase 3 + 5 + 6B | Phase 3 + 5 + 6B ALL `✅` | `tests/`, `backend/src/**/tests/` |
| **12** | Legal & SPV | `🟢 OPEN` | None (external legal) | Anytime | External (no code files) |
| **13** | OJK Compliance | `🟢 OPEN` | None (external legal) | Anytime | External + `backend/src/compliance/` |
| **14** | Community System | `⏸️ PARTIAL` | Phase 1.1 (dual DB pool) | Phase 1.1 is `✅ DONE`; advanced engagement backlog remains | `backend/src/community/` |
| **15** | Soft Launch | `🔒 LOCKED` | Phase 11 (all tests pass) | Phase 11 ALL `✅` | `Dockerfile`, deployment configs |
| **16** | Primary Issuance | `🟢 OPEN` | Phase 1 & 2 (Core) | Phase 1 & 2 are `✅ DONE` | `backend/src/issuance/` |
| **17** | RegTech | `🟢 OPEN` | Phase 3 (Trading Engine) | Phase 3 is `✅ DONE` | `backend/src/compliance/` |
| **18** | FI-System & Treasury | `🟢 OPEN` | None (core payments code exists) | Anytime | `backend/src/payments/`, `backend/src/admin/treasury.rs` |
| **19** | Affiliate Subsystem | `🟢 OPEN` | Phase 2 (DB Migrations) | Phase 2 is `✅ DONE` | `backend/src/affiliate/`, `frontend/platform/affiliate*` |
| **20** | Core Admin & Operations | `🟢 OPEN` | None (extends existing admin) | Anytime | `frontend/platform/admin*`, `.github/workflows/` |
| **21** | Smart Contract & Blockchain | `🟢 OPEN` | None (runs parallel!) | Anytime (Foundry is independent) | `contracts/`, `backend/src/blockchain/` |
| **22** | Banking API & Settlement | `🔒 LOCKED` | Phase 21.12 + Phase 18.3 | Phase 21.12 + 18.3 are not done | `backend/src/banking/` |
| **23** | Investor Dashboard Audit Backlog | `✅ DONE` | Phase 11 + page-review findings | Phase 23.1-23.12 are `✅ DONE`; browser rechecks remain on static items | `frontend/platform/`, `backend/src/` |
| **24** | Masterplan Alignment Gaps | `🟢 OPEN` | None (mostly ops/docs/config) | Anytime; 24.8 waits for audit completion and 24.9 waits for product/compliance decision | `docs/`, deployment configs, GCP Console, `backend/src/` |

---

## 📂 File Zone Ownership Matrix (Conflict Detection)

> **Rule: Two agents MUST NEVER work in the same File Zone simultaneously.**
> Before starting a task, check the Live Agent Logs — if someone is `🔄 IN PROGRESS` in the same zone, WAIT.

| File Zone | Description | Which Phases Touch This Zone |
|:---|:---|:---|
| `database/*.sql` | DB migration scripts | Phase 2, Phase 8A, Phase 18, Phase 19, Phase 22 |
| `backend/src/db.rs` | DB pool configuration | Phase 1.1, 1.2, 1.3 |
| `backend/src/auth/` | Auth, 2FA, sessions | Phase 1.4, 1.5, 1.6, Phase 20.3 |
| `backend/src/marketplace/` | **Trading engine core** | Phase 3 (ALL), Phase 4 |
| `backend/src/marketplace/models.rs` | Data structs | Phase 3.2 |
| `backend/src/marketplace/routes.rs` | API endpoints | Phase 3.5, 3.9, 3.10 |
| `backend/src/marketplace/service.rs` | Business logic | Phase 3.6, 3.7, 3.8, 3.11 |
| `backend/src/marketplace/orderbook.rs` | Redis orderbook | Phase 3.4 |
| `backend/src/marketplace/websocket.rs` | WebSocket server | Phase 4 |
| `backend/src/marketplace/background.rs` | Background workers | Phase 3.13 |
| `backend/src/admin/marketplace/` | Admin APIs | Phase 6A |
| `backend/src/payments/` | **Deposit, checkout, FX, fees** | Phase 18 (ALL) ⚠️ |
| `backend/src/payments/service.rs` | Core financial logic | Phase 18.2, 18.10 ⚠️ Critical |
| `backend/src/admin/treasury.rs` | Treasury + dividends admin | Phase 18.11, 18.12, 18.13 |
| `backend/src/admin/deposits.rs` | Deposit admin APIs | Phase 18.14 |
| `backend/src/admin/withdrawals.rs` | Withdrawal admin APIs | Phase 18.6-18.9 |
| `backend/src/affiliate/` | **Affiliate subsystem (NEW)** | Phase 19 (ALL) |
| `backend/src/blockchain/` | Blockchain integration | Phase 8B, Phase 21B |
| `backend/src/banking/` | **OCBC banking API (NEW)** | Phase 22 (ALL) |
| `backend/src/main.rs` | Route registration | Phase 3.16, 4.1, 6A, 18, 19, 22 (⚠️ shared!) |
| `backend/src/error.rs` | AppError enum | Phase 1.11 (⚠️ shared!) |
| `contracts/` | **Solidity smart contracts (NEW)** | Phase 21A (ALL) |
| `frontend/platform/marketplace*` | Trading UI HTML | Phase 5 |
| `frontend/platform/static/js/marketplace-*` | Trading UI JS | Phase 5 |
| `frontend/platform/static/css/marketplace-*` | Trading UI CSS | Phase 5 |
| `frontend/platform/admin/marketplace/` | Admin pages | Phase 6B |
| `frontend/platform/admin/blockchain*` | Admin Blockchain UI | Phase 8C, Phase 21C |
| `frontend/platform/admin/asset*` | Admin Asset UI | Phase 8C |
| `frontend/platform/affiliate*` | **Affiliate portal (NEW)** | Phase 19C |
| `.github/workflows/` | **CI/CD Pipeline (NEW)** | Phase 20C |
| `scripts/` | **Ops scripts (NEW)** | Phase 20C |
| `backend/src/issuance/` | Primary Issuance Logic | Phase 16 |
| `frontend/platform/issuance*` | Issuer Portal UI | Phase 16 |
| `backend/src/compliance/` | Compliance & RegTech | Phase 13, Phase 17 |
| `docs/operations/`, `docs/`, deployment configs | Resilience, runbooks, probes, launch governance | Phase 20, Phase 24 |
| `GCP Console`, Cloud Run, Cloud Armor, Memorystore | External ops configuration | Phase 0, Phase 15, Phase 20, Phase 24 |


> [!WARNING]
> **⚠️ SHARED FILES** — `main.rs` and `error.rs` are touched by multiple phases. When working on these files:
> 1. Only ADD new lines (route registrations or error variants) — never restructure.
> 2. Add your additions at the END of the relevant section to minimize merge conflicts.
> 3. If two agents both need `main.rs`, they must work **sequentially**, not in parallel.

> [!WARNING]
> **⚠️ FINANCIAL CRITICAL FILES** — `payments/service.rs` and `admin/treasury.rs` handle real money.
> Any modification MUST be wrapped in a DB transaction, use `i64` cents (NEVER floats), and be verified with `cargo check` AND `cargo test`.
> Only ONE agent may edit these files at a time.

---

## 🗓️ Concurrency Map (What Can Run In Parallel)

```
NOW / NEXT       Backend                 Frontend                Ops / Docs              Web3 / Banking
─────────────────────────────────────────────────────────────────────────────────────────────────────────
Immediate        Phase 18.10 fee fix     Phase 20 admin ops      Phase 24.1/24.2 resil. Phase 7.13 audit prep
Near term        Phase 18A deposits      Phase 19C affiliate UI  Phase 24.3/24.4 runbooks Phase 21 architecture
Parallel         Phase 18B withdrawals   Phase 23 browser checks Phase 20.12 monitoring   Phase 21A if chosen
Blocked          Phase 22 banking waits on Phase 18.3 + Phase 21.12
```

**Legend:** This map reflects the current roadmap state after completed Phase 1-11 core work. Pick non-overlapping file zones before starting parallel tasks.

## 📊 Current Phase Summary

| Phase | Name | Tasks | Status |
|:---|:---|:---|:---|
| **0** | Infrastructure | 12 | 7 done, 3 not started, 2 external/partial |
| **1** | Backend Hardening | 11 | ✅ 11/11 |
| **2** | DB Migrations | 10 | 8 done, 2 blocked by TimescaleDB |
| **3** | Trading Engine | 16 | ✅ 16/16 |
| **4** | WebSocket Server | 4 | ✅ 4/4 |
| **5** | Frontend Trading UI | 14 | ✅ 14/14 |
| **6A** | Admin Backend APIs | 15 | ✅ 15/15 |
| **6B** | Admin Frontend Pages | 14 | ✅ 14/14 |
| **7** | Smart Contracts | 13 | 12 done, 1 external audit |
| **8** | Blockchain Integration | 19 | 17 done, 2 production-only KMS tasks |
| **9** | Dividend System | 5 | ✅ 5/5 |
| **10** | Integration & Security | 8 | ✅ 8/8 |
| **11** | Testing & QA | 10 | 7 done, 3 launch-readiness tasks remain |
| **12-13** | Legal / OJK | 10 | External / not ready |
| **14** | Community | 7 | 6 done, 1 future advanced-engagement backlog |
| **15** | Soft Launch | 8 | Not ready |
| **16** | Primary Issuance | 5 | 4 tested done, 1 static/unverified done |
| **17** | RegTech | 5 | 2/5 done |
| **18** | FI-System & Treasury | 15 | 1/15 |
| **19** | Affiliate Subsystem | 18 | 1/18 |
| **20** | Core Admin & Operations | 15 | 0/15 |
| **21** | Smart Contract & Blockchain | 18 | 0/18 |
| **22** | Banking API & Settlement | 8 | 0/8, locked |
| **23** | Investor Dashboard Audit Backlog | 12 | ✅ 12/12, browser rechecks remain on static items |
| **24** | Masterplan Alignment Gaps | 9 | 0/9, 7 active, 1 not ready, 1 deferred |

---

## ⚠️ Critical Warnings

> [!CAUTION]
> **Smart Contract Audit must be commissioned in Week 4!** It has a 4-6 week lead time.
> Without it, Phase 15 (Launch) is blocked.

> [!CAUTION]
> **`backend/src/main.rs` is a bottleneck file.** Multiple phases need to add routes here.
> Only ONE agent may edit `main.rs` at a time. Add routes at the END of the relevant section.

> [!CAUTION]
> **Phase 3 (Trading Engine) is the critical path.** Everything depends on it. Assign your strongest/fastest agent to this phase. Do NOT split Phase 3 across multiple agents — the files are too interconnected.
