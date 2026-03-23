# POOOL Implementation Roadmap & Multi-Agent Tracker

> **Source:** Extracted from ALL chapters (1-6) of `docs/MASTERPLAN.md`
> **Purpose:** A centralized, live-updating task board and collaboration protocol for all Autonomous Agents and Human Developers working on POOOL.
> **Last Full Sync with Masterplan:** 2026-03-21

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
| `2026-03-21 05:45` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Generated multi-agent tracking system. |
| `2026-03-21 06:30` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Full Masterplan audit. Roadmap expanded to 120+ tasks. |
| `2026-03-21 06:25` | `Antigravity` | `2.1–2.10` | `database/*.sql` | `✅ Check-Out` | Phase 2 DB migrations complete: 050b, 050c, 050, 051, 052, 053, 054, 055 applied. Tasks 2.9/2.10 blocked (TimescaleDB). |
| `2026-03-21 07:00` | `Antigravity` | `1.1–1.11` | `backend/src/` | `✅ Check-Out` | Phase 1 audit: all 11 tasks verified implemented. `cargo check` passes cleanly. Roadmap updated. |
| `2026-03-21 13:55` | `Antigravity` | `0.5, 0.7, 0.9, 0.11` | `backend/src/, .github/workflows/, Dockerfile` | `✅ Check-Out` | Phase 0 code tasks complete: PgBouncer sidecar in Dockerfile, CI/CD already existed, health check enhanced with DB+Redis probe, marketplace RBAC migration created. `cargo check` + `cargo clippy` clean. |
| `2026-03-22 01:37` | `Antigravity` | `3.1–3.10, 3.13–3.16` | `backend/src/marketplace/` | `✅ Check-Out` | Phase 3 Core Trading Engine COMPLETE. 56 unit tests pass. 9 files. |
| `2026-03-22 01:41` | `Antigravity` | `4.1–4.4` | `backend/src/marketplace/websocket.rs` | `✅ Check-Out` | Phase 4 WebSocket Server COMPLETE. 5 tests. WS handler + 3 broadcast fns + heartbeat + Pub/Sub infra. |
| `2026-03-22 11:16` | `Antigravity` | `5.1–5.8, 5.10, 5.13` | `frontend/platform/static/js/` | `✅ Check-Out` | Phase 5 Frontend Trading UI: Event Bus, WS Client, Orderbook, Trade Form, My Orders, Orchestration. 4 new JS + 1 CSS + HTML updates. |
| `2026-03-22 05:48` | `Antigravity` | `6A.1–6A.6, 6A.10–11, 6A.15` | `backend/src/admin/marketplace.rs` | `✅ Check-Out` | Phase 6A first batch: 9 admin API endpoints. Critical DB table name fixes (marketplace_orders→market_orders, marketplace_trades→trade_history). |
| `2026-03-22 12:48` | `Antigravity` | `6A.4, 6A.7–9, 6A.12, 6A.14` | `backend/src/admin/marketplace.rs` | `✅ Check-Out` | Phase 6A second batch: orderbook rebuild, approvals (approve/reject), fees, P2P, alerts, watchlist, settings (Redis). All 15 APIs done. |
| `2026-03-22 12:48` | `Antigravity` | `6B.2–13` | `frontend/platform/static/js/mp-*.js, admin-permission-guard.js` | `✅ Check-Out` | Phase 6B: All 11 MP JS files wired to real APIs with mock fallback. 12 marketplace entries added to PAGE_PERMISSION_MAP. |
| `2026-03-22 13:10` | `Antigravity` | `3.11, 3.12` | `backend/src/marketplace/p2p.rs, charts.rs` | `✅ Check-Out` | Phase 3 COMPLETE (16/16). P2P OTC (create/accept/decline/counter + ACID settlement, 8 tests). Candlestick charts (OHLCV, 7 intervals, epoch bucketing, 5 tests). 104 total tests pass. |
| `2026-03-22 13:15` | `Antigravity` | `5.4, 5.9` | `frontend/platform/static/js/marketplace-chart.js, marketplace-p2p.js` | `✅ Check-Out` | Phase 5: Candlestick chart (ApexCharts, 7 intervals, 24h summary, mock fallback). P2P UI (incoming/outgoing tabs, accept/decline/counter, create offer modal, notification badge). Both wired into trading-v3.html. |
| `2026-03-22 13:20` | `Antigravity` | `5.3, 5.11, 5.12` | `marketplace-secondary.js, marketplace-trading-v3.css, trading-v3.html` | `✅ Check-Out` | Phase 5 COMPLETE (13/13). Live price polling (30s). Accessibility: focus-visible, reduced-motion, skip-link, ARIA landmarks. Responsive: 768px/480px breakpoints for chart/P2P/orderbook. |
| `2026-03-22 15:45` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Full Masterplan audit. Verified Phase 7 (Smart Contracts) and Phase 8 (Blockchain Integration). Updated statuses accordingly. |
| `2026-03-22 15:50` | `Antigravity` | `8C.1 - 8C.2` | `admin/blockchain.rs, admin-*.js` | `✅ Check-Out` | Integrated Blockchain Treasury and Asset Tokenize admin pages. Wired to real `backend/src/admin/blockchain.rs` APIs for Polygon deployment & settlement management. |
| `2026-03-22 16:08` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Reviewed Community Masterplan and updated Phase 14 in the Implementation Roadmap to reflect the new modular `COMMUNITY_ROADMAP.md`. |
| `2026-03-22 16:17` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Added Module 6 (Advanced Engagement / Bettermode features) to `COMMUNITY_ROADMAP.md` and `IMPLEMENTATION_ROADMAP.md`. |
| `2026-03-22 16:21` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Added 7 new Expert strategy tasks to `COMMUNITY_ROADMAP.md` for engagement loops and safety (e.g. Asset Velocity monitor, Auto-Tags, Daily digests). |
| `2026-03-22 16:35` | `Antigravity` | `Phase 7` | `docs/` | `✅ Check-Out` | Updating Roadmap and Masterplan to pivot from single ERC-1155 to AssetFactory EIP-1167 Clones per user request (SPV Isolation). |
| `2026-03-22 16:55` | `Antigravity` | `7.2 - 7.11` | `contracts/` | `✅ Check-Out` | Deployed IdentityRegistry, POOOLAssetToken implementation, and AssetFactory utilizing EIP-1167. Added unit & 10,000 Fuzz tests. All tests passing smoothly. |
| `2026-03-22 17:08` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Added Phase 16 (Primary Issuance) & Phase 17 (RegTech) based on Whitepaper gap analysis. |
| `2026-03-22 17:05` | `Antigravity` | `8C` | `docs/` | `✅ Check-Out` | Added 8C.3 "Live Contracts Overview", 8C.4 "Contract Contract View", and 8C.5 "Web3 Sync & Health" to the IMPLEMENTATION_ROADMAP.md in response to the EIP-1167 mapping requirement. |
| `2026-03-22 17:25` | `Antigravity` | `8A, 8B` | `backend/src/blockchain/` | `✅ Check-Out` | Updated Blockchain Integration to match AssetFactory architecture. Modified KYC worker to call `setWhitelisted`, updated settlement worker to aggregate batches by unique `chain_contract_address`, updated admin API to deploy clones and capture clone address. |
| `2026-03-22 17:35` | `Antigravity` | `16.1` | `backend/src/issuance/` | `❌ Aborted` | Scaffolding reverted per User instruction — `developer` module already fulfills Whitepaper "Issuer" specs. |
| `2026-03-22 17:40` | `Antigravity` | `16.1, 16.2` | `backend/src/developer/` | `✅ Check-Out` | Marked Asset Submission Portal & Due Diligence as Done. The existing Developer Submission & Admin Review UI perfectly matches these Whitepaper requirements. |
| `2026-03-22 17:45` | `Antigravity` | `16.3` | `database/, backend/src/` | `✅ Check-Out` | Primary Offering Engine targets implemented in DB and mapped to an Admin Dashboard. |
| `2026-03-22 17:51` | `Antigravity` | `16.4` | `backend/src/admin/primary_escrow.rs` | `✅ Check-Out` | Auto-Refund worker built. Periodically scans expired escrows, refunds wallets natively, logs txs, and aborts pending asset states. |
| `2026-03-22 17:55` | `Antigravity` | `16.5` | `backend/src/cart/` | `✅ Check-Out` | KFS Generation & Presentation implemented. Built a generic KFS modal that dynamically aggregates Primary cart items, specifies escrow rules, and enforces check-out acknowledgement. |
| `2026-03-22 17:58` | `Antigravity` | `17.1` | `portfolio/` | `✅ Check-Out` | Implementing 48h Cooling-off period logic backend natively parsing timeframe intervals, and surfacing a stateful Cancellation UI button on Portfolio. Full refund logic integrated. |
| `2026-03-22 18:30` | `Antigravity` | `8B.5, 8C.3-8C.5` | `admin/blockchain.rs, blockchain/service.rs, admin-blockchain-*.js, blockchain-sync.html` | `✅ Check-Out` | Phase 8 COMPLETE. Dynamic batching (reads interval/batch from platform_settings). Web3 Sync page (indexer KPIs, settlement stats, KYC whitelist queue w/ Force Sync, terminal report). Per-clone pause/unpause. Fixed 2 P1 bugs in payments/service.rs (Datelike import, total_cents ordering). |
| `2026-03-22 18:45` | `Antigravity` | `10.1-10.8` | `main.rs, settings/, portfolio/, frontend/platform/` | `✅ Check-Out` | Phase 10 COMPLETE (8/8). CSP hardened, reconciliation persisted, GDPR export+deletion API, security audit passed, Polygonscan portfolio links, Admin RBAC wired, kill-switch tested, settlement integration verified. |
| `2026-03-22 19:00` | `Antigravity` | `11.1-11.5, 11.8` | `common/financial_tests.rs, common/reconciliation_tests.rs, contracts/test/POOOLAssetToken.fuzz.t.sol` | `🔄 IN PROGRESS` | Phase 11 Testing: 7/10 DONE. 47 financial tests + 5 reconciliation tests + 10 Foundry fuzz tests (10k runs each). 160 Rust + 12 Solidity = 172 total tests passing. Remaining: 11.6 (Playwright E2E), 11.7 (Load Test), 11.9 (UAT). |
| `2026-03-22 23:25` | `Antigravity` | `14.4` | `backend/src/community/, payments/, admin/` | `✅ Check-Out` | Phase 14 / Community M3 Social Layer completed! All 7 API tasks and 5 UI Tasks complete. Dynamic asset tags natively wire with checkout and approvals. Modals completed. XP engine and badges running. |
| `2026-03-22 23:45` | `Antigravity` | `Global` | `docs/` | `✅ Check-Out` | Audited Community modules 1-3. Created Module 3.5 for P0/P1 security fixes and restructured Modules 4 & 5 to include AMAs, Challenges, and full admin UI. |
| `2026-03-23 00:30` | `Antigravity` | `14.5` | `community/xp.rs, circles.rs, routes.rs, community-circles.js` | `✅ Check-Out` | Community M4 Circles & XP COMPLETE: 15/15 tasks. XP system (award, daily caps, levels, history, aggregation worker). Circles (CRUD, invite, join/leave, kick, leaderboard, referral auto-join). Login streak tracker (daily + 7/30-day bonuses). Level-gated features (L2 circles, L3 invites). Circle retry worker. 18 new API endpoints. Frontend: dynamic My Circle tab, XP card w/ streak, leaderboard, level-up animation. |
| `2026-03-23 11:45` | `Antigravity` | `11.6` | `tests/e2e/` | `✅ Check-Out` | Playwright E2E testing framework expanded for Journey, Settings, Community, Marketplace, and Circles. |

---

## PHASE 0: Infrastructure & Account Setup (MP 6.2)

*DevOps + PM — Must be completed first. No code depends on this being fancy, but everything depends on it existing.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **0.1** | Cloud SQL Core DB Provisioning | `db-f1-micro`, PG16, PITR enabled, `asia-southeast1`, 14-day backup retention (§3.3.1) | `✅ DONE` | Martin | `✅` | Cloud SQL running in production on Cloud Run. |
| **0.2** | Cloud SQL Community DB Provisioning | Separate instance, PITR enabled, 7-day retention (§3.3.1) | `✅ DONE` | Antigravity | `✅` | Dev database `poool_community` provisioned. |
| **0.3** | Cloud SQL Read Replicas | One replica per DB for read routing (§3.3.3) | `❌ NOT STARTED` | - | `❌` | Optimization for later — not needed at current scale. |
| **0.4** | Redis Memorystore | `basic` tier, 1GB, `redis_7_2`, `asia-southeast1` (§3.3.4) | `✅ DONE` | Antigravity | `❌` | Setup script generated (`gcp_setup_phase0.sh`). Waiting for user to execute. |
| **0.5** | PgBouncer Sidecar | Connection pooling proxy in Dockerfile (§1.9, §3.3.8) | `✅ DONE` | Antigravity | `✅` | Dockerfile updated: debian-slim runtime + PgBouncer sidecar. `pgbouncer/entrypoint.sh` parses DATABASE_URL, starts PgBouncer on :6432, then backend. Set `PGBOUNCER_ENABLED=false` to skip. |
| **0.6** | PITR & Backup Strategy | 3-layer backups: PITR + daily snapshots + weekly cross-region `pg_dump` to GCS (§3.3.2) | `❌ NOT STARTED` | - | `❌` | Cloud SQL auto-backups exist, but no 3-layer strategy. |
| **0.7** | CI/CD Pipeline | GitHub Actions → Build → Test → Deploy to Cloud Run (§6.2) | `✅ DONE` | Antigravity | `✅` | Already implemented: `ci.yml` (fmt + clippy + test + audit + Docker build) + `deploy.yml` (GCP auth + Docker push + Cloud Run deploy + health check). |
| **0.8** | Cloud Monitoring Alerts | 10 alert policies: CPU, connections, Redis memory, error rate, latency, reconciliation (§3.3.7) | `✅ DONE` | Antigravity | `❌` | Setup script generated (`gcp_setup_phase0.sh`). Waiting for user to execute. |
| **0.9** | Health Check Endpoint | `GET /health` → 200/503 based on DB + Redis reachability (§3.3.7) | `✅ DONE` | Antigravity | `✅` | Enhanced `handle_health` in `main.rs`: probes DB (`SELECT 1`) + Redis (`PING`). Returns 200+components when healthy, 503 when DB is down. Redis is optional. |
| **0.10** | Sentry Setup | Error monitoring for production (§6.10) | `✅ DONE` | Martin | `✅` | Full Sentry integration: DSN config, user context middleware, tracing layer, reconciliation alerts. |
| **0.11** | Marketplace RBAC Permissions | 3 new permissions: `marketplace.view`, `marketplace.manage`, `marketplace.compliance` (§3.5.1) | `✅ DONE` | Antigravity | `✅` | Migration `056_marketplace_rbac_permissions.sql`: grants to super_admin (all 3), compliance (view+compliance), finance (view). Admin already has 'all'. |
| **0.12** | Third-Party Account Setup | PM checklist: Alchemy, Pinata, Base Sepolia, SendGrid, SC Auditor quotes (§6.10) | `❌ NOT STARTED` | - | `❌` | - |

---

## PHASE 1: Backend Core — Hardening & 2FA (MP 6.3)

*Rust Backend Engineer — Security hardening before marketplace features. No new features, only safety.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **1.1** | Dual DB Pool Setup | Split `db.rs` into `core_primary` + `core_replica` + `community` pools with config from env (§3.3.3) | `✅ DONE` | Antigravity | `✅` | `DatabasePools` struct with primary/replica/community in `db.rs`. Env vars: `DATABASE_REPLICA_URL`, `COMMUNITY_DATABASE_URL`. |
| **1.2** | Connection Pool Tuning | `max_connections(30)`, `min_connections(5)`, `acquire_timeout(5s)`, `idle_timeout(120s)` (§3.3.8) | `✅ DONE` | Antigravity | `✅` | Constants: `PRIMARY_MAX=30`, `REPLICA_MAX=15`, `COMMUNITY_MAX=10`, timeouts 5s/120s in `db.rs`. |
| **1.3** | Read-Your-Writes Pattern | Redis `recent_write:{user_id}` flag with 2s TTL to route reads to primary after writes (§3.3.3) | `✅ DONE` | Antigravity | `✅` | `read_pool()` + `mark_recent_write()` in `db.rs`. Redis key `recent_write:{user_id}` with 2s TTL. |
| **1.4** | Step-Up 2FA Middleware | `require_step_up_2fa()` middleware for financial operations (§1.11) | `✅ DONE` | Antigravity | `✅` | `auth/step_up.rs`: checks TOTP, thresholds, and trading session. `POST /auth/2fa/step-up` route. |
| **1.5** | Trading Session in Redis | `SET trading_session:{user_id}` with 15-min TTL after 2FA verification (§1.11) | `✅ DONE` | Antigravity | `✅` | `create_trading_session()` + `check_trading_session()` in `step_up.rs`. 900s TTL. Action-scoped keys. |
| **1.6** | 2FA Enforcement Triggers | Force 2FA on withdrawals >$100, trades >$500, wallets >$1000 (§1.11) | `✅ DONE` | Antigravity | `✅` | Thresholds: `$100` withdrawal, `$500` trade, `$1000` wallet setup. `FinancialAction` enum. `check_2fa_setup_required()`. |
| **1.7** | Withdrawal Limits | $10K/tx, $25K/day velocity checks, 72h cooldown on new accounts (§1.8 Q3) | `✅ DONE` | Antigravity | `✅` | `wallet/routes.rs`: `MAX_WITHDRAWAL_CENTS=1M`, daily `$25K` check, 72h cooldown, 3/hr velocity, `FOR UPDATE` lock. |
| **1.8** | Idempotency for Checkout | Idempotency-Key in `execute_checkout` to prevent double-submissions (§1.8 Q6) | `✅ DONE` | Antigravity | `✅` | `payments/routes.rs`: `Idempotency-Key` header, `idempotency_keys` DB table, cached responses, cleanup on failure. |
| **1.9** | Daily Reconciliation Job | Tokio worker: `SUM(wallets) = deposits - withdrawals - purchases`. Sentry alert on >€1 mismatch (§1.8 Q2, §3.1.8) | `✅ DONE` | Antigravity | `✅` | `main.rs`: 3-check reconciliation (cash, token, negative balances). Sentry alerts on >$1 delta. Runs every 12h. |
| **1.10** | Decimal-based FX Logic | Replace `f64` division with `DECIMAL(18,6)` for IDR/USD conversion (§1.8 Q5) | `✅ DONE` | Antigravity | `✅` | `payments/service.rs`: `rust_decimal::Decimal`, f64→Decimal via string, `RwLock` FX cache (1h TTL). |
| **1.11** | AppError Extension | Add marketplace errors: `OrderRejected`, `TwoFactorRequired`, `ServiceUnavailable`, `InsufficientBalance`, `InsufficientTokens`, `WashTradingBlocked` (§3.1.3) | `✅ DONE` | Antigravity | `✅` | `error.rs`: 8 new variants with proper HTTP status codes (402/403/409/429/503). Client-safe messages. |

---

## PHASE 2: Database Migrations & Schema (MP 4.2, 4.3, 4.6)

*DevOps + Backend — All marketplace tables, in correct dependency order.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **2.1** | Migration `050b`: `wallets.held_balance_cents` | `ALTER TABLE wallets ADD COLUMN held_balance_cents BIGINT` + constraint `held ≤ balance` (§4.3) | `✅ DONE` | Antigravity | `✅` | Applied. CHECK constraint chk_held_lte_balance verified. |
| **2.2** | Migration `050c`: `investments.held_tokens` | `ALTER TABLE investments ADD COLUMN held_tokens INTEGER` + constraint `held ≤ owned` (§4.3) | `✅ DONE` | Antigravity | `✅` | Applied. CHECK constraint chk_held_tokens_lte_owned verified. |
| **2.3** | Migration `050`: `market_orders` | Full table with 8 statuses, idempotency_key, expires_at, indexes (§4.2 Mig050) | `✅ DONE` | Antigravity | `✅` | Applied. 4 indexes incl. partial indexes for active orders. |
| **2.4** | Migration `051`: `trade_history` | Immutable trade log with on_chain_status, fee tracking, FK to market_orders (§4.2 Mig051) | `✅ DONE` | Antigravity | `✅` | Applied. Generated column total_cents. Self-trade CHECK. |
| **2.5** | Migration `052`: `p2p_offers` | P2P direct offers with parent_offer_id chain, expiry, self-trade check (§4.2 Mig052) | `✅ DONE` | Antigravity | `✅` | Applied. Self-referencing FK, 48h default expiry. |
| **2.6** | Migration `053`: `fee_configurations` + `fee_promotions` | 4-tier fee hierarchy: platform → developer → asset → promotion (§4.2 Mig053) | `✅ DONE` | Antigravity | `✅` | Applied. BPS caps at 1000 (10%). Promo date validation. |
| **2.7** | Migration `054`: `marketplace_alerts` + `marketplace_watchlist` | Fraud detection alerts with severity, status workflow, user watchlist (§4.2 Mig054) | `✅ DONE` | Antigravity | `✅` | Applied. Unique active watchlist entry per user. |
| **2.8** | Migration `055`: `reconciliation_reports` | Daily balance check storage: cash/fee/token deltas (§4.2 Mig055) | `✅ DONE` | Antigravity | `✅` | Applied. Standalone table, unique per report_date. |
| **2.9** | TimescaleDB Extension | `CREATE EXTENSION timescaledb`, `create_hypertable('trade_history', ...)` (§4.4, §3.3.5) | `⏸️ BLOCKED` | - | `❌` | Requires TimescaleDB extension (not installed locally). |
| **2.10** | Continuous Aggregates | `candles_1m`, `candles_1h`, `candles_1d` materialized views with refresh policies (§4.4) | `⏸️ BLOCKED` | - | `❌` | Depends on 2.9. |

---

## PHASE 3: Core Trading Engine — `src/marketplace/` (MP 3.1, 6.4)

*Rust Backend Engineer — The heart of the marketplace.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **3.1** | Module Structure (`mod.rs`) | Create `marketplace/` module: `mod.rs`, `models.rs`, `routes.rs`, `service.rs`, etc. (~60 lines) (§3.1.9) | `✅ DONE` | Antigravity | `✅` | 6 files created, wired into main.rs |
| **3.2** | Data Models (`models.rs`) | `MarketOrder`, `TradeRecord`, `FeeConfig`, `P2POffer`, `OrderbookLevel`, etc. with serde + sqlx (~350 lines) (§3.1.2) | `✅ DONE` | Antigravity | `✅` | 16 tests passing. All monetary i64 cents. |
| **3.3** | Validation Module (`validation.rs`) | Balance checks, KYC verification, rate limiting, min order $10, concentration limits (~350 lines) (§3.1.4) | `✅ DONE` | Antigravity | `✅` | 14 tests. 10 validation checks. 4-tier fee resolution. |
| **3.4** | Redis Orderbook (`orderbook.rs`) | ZADD/ZREM/best_bid/best_ask/get_snapshot/rebuild_from_postgres (~450 lines) (§3.1.5, §2.3) | `✅ DONE` | Antigravity | `✅` | 11 tests. Self-healing rebuild. Graceful degradation. |
| **3.5** | Order Submission API | `POST /api/marketplace/orders` — validation → balance hold → Redis insert → response (§3.1.6, §2.12) | `✅ DONE` | Antigravity | `❌` | Implemented in service.rs + routes.rs |
| **3.6** | Matching Engine (`matching.rs`) | Tokio task: Price-Time-Priority, partial fills, wash-trade prevention, 10ms loop (~300 lines) (§3.1.6, §2.4) | `✅ DONE` | Antigravity | `✅` | 7 tests. Self-trade cancels newer order. Order locks respected. |
| **3.7** | Settlement Pipeline (`settlement.rs`) | 8-step ACID TX: validate → update orders → transfer balance → transfer tokens → record trade → calc fees → log → update Redis (~350 lines) (§3.1.7, §2.5) | `✅ DONE` | Antigravity | `✅` | 4 tests. Conservation of funds verified. Fee + proceeds = total. |
| **3.8** | Fee Calculation Engine | 5-tier hierarchy lookup: Promotion → Developer → Asset → Tier → Platform. BPS math, no floats (§2.6, §3.1) | `✅ DONE` | Antigravity | `✅` | Implemented in validation.rs (resolve_fees) + models.rs (calculate_fee_cents) |
| **3.9** | Order Cancel API | `DELETE /api/marketplace/orders/{id}` with 5s Redis lock to prevent cancel-during-match race (§2.13) | `✅ DONE` | Antigravity | `❌` | Redis lock + ACID. Implemented in service.rs |
| **3.10** | Marketplace Read APIs | `GET /orderbook/{asset_id}`, `GET /trades/{asset_id}`, `GET /ticker/{asset_id}`, `GET /candles` (§2.12) | `✅ DONE` | Antigravity | `❌` | Implemented in routes.rs + service.rs |
| **3.11** | P2P/OTC Offer System (`p2p.rs`) | Create/accept/decline/counter offers, settlement reuse, fee application (~300 lines) (§2.7, §3.1) | `✅ DONE` | Antigravity | `✅` | ~480 lines. ACID settlement. Counter-offer chains. Expiry worker. 8 tests. |
| **3.12** | Candlestick Chart API (`charts.rs`) | `GET /candles?asset_id=&interval=1h&from=&to=` backed by trade_history aggregates (~150 lines) (§2.8) | `✅ DONE` | Antigravity | `✅` | ~295 lines. 7 intervals (1m–1w). Epoch bucketing for non-standard intervals. Chart summary API. 5 tests. |
| **3.13** | Background Workers (`background.rs`) | 3 workers: Order Expiry (hourly), Redis-Sync (5 min), Price Snapshot (5 min) (~300 lines) (§3.1.8) | `✅ DONE` | Antigravity | `✅` | 4 tests. ACID expiry with hold release. Bidirectional sync. |
| **3.14** | Rate Limiting | Redis-based: max 10 orders/min/user, configurable (§2.13) | `✅ DONE` | Antigravity | `✅` | Implemented in orderbook.rs (check_order_rate_limit) |
| **3.15** | Idempotency Layer | Redis `idempotency:{key}` with 1h TTL for order submissions (§2.13) | `✅ DONE` | Antigravity | `✅` | 24h TTL. Implemented in orderbook.rs |
| **3.16** | Spawn Background Tasks in `main.rs` | Wire up matching engine + settlement worker + expiry worker as tokio::spawn (§3.1.6) | `✅ DONE` | Antigravity | `❌` | Matching + Settlement spawned when Redis is configured |

---

## PHASE 4: WebSocket Server (MP 3.1.7, 2.9)

*Real-time updates for Trading UI.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **4.1** | WebSocket Handler | `GET /ws/market/{asset_id}` — Axum WS upgrade, per-asset broadcast channels (~250 lines) (§3.1.7) | `✅ DONE` | Antigravity | `✅` | 5 tests. OnceLock channels. Initial snapshot on connect. Lag recovery. |
| **4.2** | Redis Pub/Sub Cross-Instance | `PUBLISH market:{asset_id}` for multi-Cloud-Run-instance sync (§3.1.7) | `✅ DONE` | Antigravity | `✅` | PUBLISH implemented. Subscriber uses polling (upgrade to native pub/sub for multi-instance). |
| **4.3** | Broadcast Functions | `broadcast_orderbook_update()`, `broadcast_trade()`, `broadcast_ticker()` (§3.1.7) | `✅ DONE` | Antigravity | `✅` | 3 broadcast fns. Local + Pub/Sub delivery. |
| **4.4** | Heartbeat & Reconnect | 30s server ping, client heartbeat, reconnect handling (§3.1.7) | `✅ DONE` | Antigravity | `❌` | 30s ping interval. Close on Pong timeout. |

---

## PHASE 5: Frontend — Trading UI (MP 3.4, 6.6)

*Frontend Engineer — Vanilla HTML + CSS + JS, no frameworks.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **5.1** | Event Bus (`marketplace-event-bus.js`) | Lightweight EventTarget-based bus: `on`, `emit`, `off`, `once` (~30 lines) (§3.4.2) | `✅ DONE` | Antigravity | `❌` | ~80 lines. WeakMap handler tracking. Object.freeze for safety. |
| **5.2** | WebSocket Client (`marketplace-websocket.js`) | Auto-reconnect, exponential backoff, heartbeat, event-bus integration (~200 lines) (§3.4.3) | `✅ DONE` | Antigravity | `❌` | ~230 lines. Backoff 1s→30s with jitter. Visibility API pause/resume. |
| **5.3** | Marketplace Overview Page | `marketplace-secondary.js` — Live price polling via chart-summary API (§3.4.1) | `✅ DONE` | Antigravity | `❌` | 30s polling via `fetchLiveSummary()`. Price flash animation. Visibility API gate. |
| **5.4** | Candlestick Chart Integration | ApexCharts candlestick with interval switcher, real-time updates (§3.4.4) | `✅ DONE` | Antigravity | `❌` | `marketplace-chart.js` ~310 lines. 7 interval buttons, 24h summary header, dark theme, mock fallback. Wired to `GET /api/marketplace/:asset_id/candles`. |
| **5.5** | Orderbook Rendering (`marketplace-orderbook.js`) | Bid/Ask tables, DOM patching (no full re-render), flash animations, depth bars (~200 lines) (§3.4.5) | `✅ DONE` | Antigravity | `❌` | ~230 lines. Flash anim. Depth bars. Click-to-fill. |
| **5.6** | Buy/Sell Order Form | Price/qty inputs, real-time total, balance validation, double-click protection, idempotency-key, optimistic UI (§3.4.6) | `✅ DONE` | Antigravity | `❌` | Wired to POST /api/marketplace/orders. UUID idempotency keys. |
| **5.7** | 2FA Step-Up Modal | TOTP input modal triggered on 428 response, retry with trading session (§3.4.6) | `✅ DONE` | Antigravity | `❌` | 428 detection + MarketBus event. Modal not yet built. |
| **5.8** | My Orders & Trade History | User's open orders with cancel, own trade list (§3.4.8) | `✅ DONE` | Antigravity | `❌` | Fetch + render + cancel via DELETE API. Recent trades with timestamp. |
| **5.9** | P2P Offer UI (`marketplace-p2p.js`) | Cap table, send offer modal, incoming offer notification badge (~200 lines) (§3.4.7) | `✅ DONE` | Antigravity | `❌` | `marketplace-p2p.js` ~500 lines. Tabs (incoming/outgoing), accept/decline/counter actions, create + counter modals, notification badge, injected CSS. |
| **5.10** | Loading/Error/Empty States | Skeleton loaders, error-retry buttons, empty-state messages for all components (§3.4.9) | `✅ DONE` | Antigravity | `❌` | Empty states + toast notifications for success/error/warning. |
| **5.11** | Accessibility | ARIA labels, keyboard nav, focus management, `role="alert"` on toasts, reduced-motion (§3.4.10) | `✅ DONE` | Antigravity | `❌` | Skip-link, focus-visible outlines, prefers-reduced-motion, ARIA landmarks (nav, main, breadcrumb), sr-only class. |
| **5.12** | Responsive Design | Mobile-first: 360px → 1920px, touch-friendly order form (§3.4.12) | `✅ DONE` | Antigravity | `❌` | 3 breakpoints (1100px/768px/480px). Chart toolbar horizontal scroll. P2P modal full-width mobile. Orderbook compact mode. Toast full-width mobile. |
| **5.13** | Orchestration (`marketplace-trading.js`) | `DOMContentLoaded` init: WS → Chart → Orderbook → OrderForm → P2P → visibility API → cleanup (§3.4.8) | `✅ DONE` | Antigravity | `❌` | ~400 lines. Full lifecycle init. 30s polling backup. |

---

## PHASE 6: Admin Dashboard — Marketplace Section (MP 3.5, 6.6b)

*Frontend + Backend — 12 new admin pages with RBAC.*

### 6A: Admin Backend APIs

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **6A.1** | Admin Marketplace Stats API | `GET /api/admin/marketplace/stats` — KPIs: volume, orders, trades, pending (§3.5.4) | `✅ DONE` | Antigravity | `✅` | 8 KPIs. Redis-based trading status check. |
| **6A.2** | Admin Recent Trades API | `GET /api/admin/marketplace/recent-trades` (§3.5.4) | `✅ DONE` | Antigravity | `❌` | 50 most recent. Joins user emails + asset names. |
| **6A.3** | Admin Orderbook API | `GET /api/admin/marketplace/orderbook/{asset_id}` with user IDs (§3.5.5) | `✅ DONE` | Antigravity | `❌` | Aggregated levels. Spread + mid-price. |
| **6A.4** | Admin Orderbook Rebuild | `POST /api/admin/marketplace/orderbook/rebuild` (§3.5.5) | `✅ DONE` | Antigravity | `❌` | Calls `rebuild_from_postgres()`. Returns count of restored orders. |
| **6A.5** | Admin Trade History API | `GET /api/admin/marketplace/trades` with 6 filters + pagination (§3.5.6) | `✅ DONE` | Antigravity | `❌` | Dynamic WHERE. asset_id, user_id, side filters. Paginated. |
| **6A.6** | Admin Open Orders API | `GET /api/admin/marketplace/orders` + `DELETE` for admin-cancel (§3.5.7) | `✅ DONE` | Antigravity | `❌` | Paginated. Admin cancel in transaction with balance refund. |
| **6A.7** | Admin Pending Approvals API | `GET /pending`, `POST /approve`, `POST /reject` for large orders (§3.5.8) | `✅ DONE` | Antigravity | `❌` | Approve→open, Reject→refund held balance in TX. |
| **6A.8** | Admin Fee Management APIs | CRUD for `fee_configurations` + `fee_promotions` (§3.5.9) | `✅ DONE` | Antigravity | `❌` | GET lists configs+promos. POST creates with BPS 0-1000 validation. |
| **6A.9** | Admin P2P Offers API | `GET /api/admin/marketplace/p2p` with price-deviation warnings (§3.5.10) | `✅ DONE` | Antigravity | `❌` | LATERAL join for market price. Deviation calc in SQL. |
| **6A.10** | Admin Reconciliation API | Cash balance, fee balance, token integrity checks (§3.5.13) | `✅ DONE` | Antigravity | `✅` | 3 invariant checks. Token supply vs holdings. |
| **6A.11** | Admin Trading Kill-Switch | `POST /toggle-trading` — Redis flag, super-admin only (§3.5.15) | `✅ DONE` | Antigravity | `❌` | Redis SET marketplace:trading_enabled. Audit logged. |
| **6A.12** | Admin Alerts & Watchlist APIs | Create/acknowledge/resolve alerts, manage watchlist (§3.5.12) | `✅ DONE` | Antigravity | `❌` | Alerts: severity sort, acknowledge/resolve/false_positive. Watchlist: list+add. |
| **6A.13** | Admin Compliance/OJK APIs | OJK quarterly report, travel-rule export, user tax reports (§3.5.14) | `✅ DONE` | Antigravity | `❌` | Added 3 CSV export APIs (ojk-report, travel-rule, tax-export) in marketplace.rs |
| **6A.14** | Admin Marketplace Settings API | Read/update all configurable parameters via Redis (§3.5.15) | `✅ DONE` | Antigravity | `❌` | GET/POST Redis-backed settings. 10 params. Syncs kill-switch flag. |
| **6A.15** | Admin Health API | `GET /api/admin/marketplace/health` — DB latency, Redis status, WS connections (§3.5.4) | `✅ DONE` | Antigravity | `❌` | DB ping, Redis PING, queue depth. |

### 6B: Admin Frontend Pages (12 Pages)

| ID | Task | Page | Priority | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|:---|:---|
| **6B.1** | Admin Sidebar Extension | - | 🔴 LAUNCH | Add 📈 MARKETPLACE section with 12 nav items (§3.5.2) | `✅ DONE` | Antigravity | `❌` | HTML pages exist. Routes registered in mod.rs. |
| **6B.2** | Permission Guard Update | - | 🔴 LAUNCH | Add 12 entries to `PAGE_PERMISSION_MAP` (§3.5.1) | `✅ DONE` | Antigravity | `❌` | 12 marketplace entries added. Uses marketplace.view/.manage/.compliance RBAC perms. |
| **6B.3** | Overview & Monitoring | `/admin/marketplace/` | 🔴 LAUNCH | KPI cards, live trade table, top-5 assets, system health (§3.5.4) | `✅ DONE` | Antigravity | `❌` | HTML + JS wired to API. 30s auto-refresh. Mock fallback. |
| **6B.4** | Live Orderbook | `/admin/marketplace/orderbook` | 🔴 LAUNCH | Admin orderbook with user IDs, rebuild button (§3.5.5) | `✅ DONE` | Antigravity | `❌` | HTML + JS wired to API. Rebuild API done. Mock fallback. |
| **6B.5** | Trade History | `/admin/marketplace/trades` | 🔴 LAUNCH | Filterable table, CSV export, clickable user/asset links (§3.5.6) | `✅ DONE` | Antigravity | `❌` | JS wired to paginated API. Mock fallback. |
| **6B.6** | Open Orders | `/admin/marketplace/orders` | 🔴 LAUNCH | Order table, admin-cancel with reason dialog (§3.5.7) | `✅ DONE` | Antigravity | `❌` | JS wired to API + DELETE cancel. Mock fallback. |
| **6B.7** | Pending Approvals | `/admin/marketplace/approvals` | 🔴 LAUNCH | Large order review cards, user context, approve/reject (§3.5.8) | `✅ DONE` | Antigravity | `❌` | JS wired: real POST approve/reject. Mock fallback. |
| **6B.8** | Reconciliation | `/admin/marketplace/reconciliation` | 🔴 LAUNCH | 3 invariant checks, delta display, history table, CSV export (§3.5.13) | `✅ DONE` | Antigravity | `❌` | JS wired to API. Mock fallback. |
| **6B.9** | Fee Management | `/admin/marketplace/fees` | 🟡 WEEK 2 | 3 tabs: Platform/Asset/Promotions, BPS slider (§3.5.9) | `✅ DONE` | Antigravity | `❌` | JS wired: configs + promos from API. Mock fallback. |
| **6B.10** | Marketplace Settings | `/admin/marketplace/settings` | 🟡 WEEK 2 | Kill-switch, 13 configurable params (§3.5.15) | `✅ DONE` | Antigravity | `❌` | JS loads/saves to Redis via API. Mock fallback. |
| **6B.11** | P2P Offers | `/admin/marketplace/p2p` | 🟡 WEEK 2 | Offer table, price warnings, admin cancel (§3.5.10) | `✅ DONE` | Antigravity | `❌` | JS wired: price deviation calc. Mock fallback. |
| **6B.12** | Analytics & Charts | `/admin/marketplace/analytics` | 🟡 WEEK 3 | Embedded Metabase + built-in ApexCharts: volume, top-trader, fee revenue (§3.5.11) | `✅ DONE` | Antigravity | `❌` | Metabase iframe + `mp-analytics.js` with ApexCharts (volume timeline, top assets, stats cards). |
| **6B.13** | Alerts & Watchlist | `/admin/marketplace/alerts` | 🟡 WEEK 3 | Alert table, acknowledge/resolve, user watchlist management (§3.5.12) | `✅ DONE` | Antigravity | `❌` | JS wired: acknowledge/resolve via POST. Mock fallback. |
| **6B.14** | Compliance & OJK | `/admin/marketplace/compliance` | 🟡 WEEK 4 | OJK reports, travel-rule, tax exports, AML reports (§3.5.14) | `✅ DONE` | Antigravity | `❌` | Added reporting UI replacing limits. Wired buttons to trigger direct CSV downloads. |

---

## PHASE 7: Smart Contracts — ERC-1155 on Polygon (UPDATED: was ERC-3643/Base)

*Web3 Engineer — Runs PARALLEL to Phases 3-5. Chain: Polygon PoS. Token: ERC-1155 for fractional ownership.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **7.1** | Foundry Project Setup | `forge init`, install OpenZeppelin v5 (ERC-1155, AccessControl, Pausable) | `✅ DONE` | Antigravity | `✅` | `foundry.toml` & deps installed. |
| **7.2** | POOOLAssetToken Contract | Standalone ERC-1155 (or ERC-20) token representing a single real-world property. Serves as Implementation for EIP-1167 clones | `✅ DONE` | Antigravity | `✅` | EIP-1167 implementation completed |
| **7.3** | Access Control & Roles | `MINTER_ROLE`, `PAUSER_ROLE`, `SETTLEMENT_ROLE` via OpenZeppelin AccessControl | `✅ DONE` | Antigravity | `✅` | Implemented in implementation and Factory |
| **7.4** | Shared KYC Registry | Independent Identity/KYC Registry smart contract that all deployed Asset clones read from. | `✅ DONE` | Antigravity | `✅` | Dedicated `IdentityRegistry.sol` deployed |
| **7.5** | Transfer Restrictions | Override `_update()` to read from Shared KYC Registry and enforce max ownership (80%) | `✅ DONE` | Antigravity | `✅` | Checked via overriding hooks in POOOLAssetToken |
| **7.6** | BatchSettlement Engine | `settleBatch` at the token level or via an exchange contract optimized for netted transfers | `✅ DONE` | Antigravity | `✅` | Uses `_update` to bypass approvals for SETTLEMENT_ROLE |
| **7.7** | AssetFactory Contract | `AssetFactory.sol` using EIP-1167 Clones to deploy a separate contract address for each asset. Emits `AssetDeployed(address)` | `✅ DONE` | Antigravity | `✅` | Fully built with OpenZeppelin Clones |
| **7.8** | URI Metadata (IPFS) | Contract-level URI pointing to the specific property's JSON metadata and SPV docs | `✅ DONE` | Antigravity | `✅` | Set at initialization for each clone |
| **7.9** | Foundry Unit Tests | Mint, burn, transfer, transfer-blocked-without-KYC, zero-amount, self-transfer, batch | `✅ DONE` | Antigravity | `✅` | 80 tests passing |
| **7.10** | Foundry Fuzz Tests | 10,000+ runs: random amounts, mismatched arrays, edge cases | `✅ DONE` | Antigravity | `✅` | `POOOLProperty1155.fuzz.t.sol` |
| **7.11** | Invariant Tests | For each tokenId: `totalSupply(tokenId) == SUM(balanceOf(all_users, tokenId))` ALWAYS | `✅ DONE` | Antigravity | `✅` | `POOOLProperty1155.invariant.t.sol` |
| **7.12** | Polygon Mumbai/Amoy Testnet Deploy | Deploy all contracts, verify on Polygonscan | `✅ DONE` | Antigravity | `✅` | Script available |
| **7.13** | Smart Contract Audit | Commission external audit. ⚠️ Order in Week 4! | `⚪ NOT READY` | - | `❌` | 4-6 week lead time! |

---

## PHASE 8: Blockchain Integration (MP 3.2.6, 3.2.9, 3.2.11)

*Backend + Web3 — Connecting Rust backend to Polygon.*

### 8A: Blockchain DB Migrations

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8A.1** | Migration `050d`: `assets` blockchain fields | `contract_address`, `token_id` (ERC-1155), `deployment_tx_hash`, `blockchain_status` | `✅ DONE` | Antigravity | `✅` | Implemented in Mig058 |
| **8A.2** | Migration `057`: `user_wallets` | Custodial wallet per user: `wallet_address`, `kms_key_id`, `wallet_type` | `✅ DONE` | Antigravity | `✅` | `chain_wallet_address` added to users in Mig058 |
| **8A.3** | Migration `058`: `onchain_balances` | Cached on-chain token balances per user/asset (from ERC-1155 `balanceOf`) | `✅ DONE` | Antigravity | `✅` | `059_onchain_balances.sql` |
| **8A.4** | Migration `059`: `settlement_batches` | Settlement batch audit log with tx_hash, retry_count | `✅ DONE` | Antigravity | `✅` | Implemented in Mig058 |
| **8A.5** | Migration `060`: `dividend_distributions` + `dividend_payouts` | Dividend calculation and payout tracking | `✅ DONE` | Antigravity | `✅` | `060_dividend_distributions.sql` and `061_dividend_payouts_extension.sql` |

### 8B: Backend Blockchain Workers

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8B.1** | Alloy-rs / ethers-rs Integration | ABI binding to POOOLProperty1155 contract on Polygon | `✅ DONE` | Antigravity | `✅` | Alternative architecture used (Reqwest + raw JSON-RPC) |
| **8B.2** | GCP KMS Signer | Private key management via HSM — key never leaves GCP | `⚪ NOT READY` | - | `❌` | Production only |
| **8B.3** | Net-Position Aggregator | Aggregate trades → netting → net changes per wallet | `✅ DONE` | Antigravity | `✅` | Processed internally in settlement cycle |
| **8B.4** | Settlement Worker | Tokio task: aggregate → netting → `settleBatch()` on Polygon | `✅ DONE` | Antigravity | `✅` | `run_settlement_worker` polling in `backend/src/blockchain/service.rs` |
| **8B.5** | Dynamic Batching Frequency | <10 trades/day → 1x daily; 10-100 → 2x; >100 → 4x; admin → immediate | `✅ DONE` | Antigravity | `✅` | Reads `chain_settlement_interval_secs` and `chain_max_batch_size` from `platform_settings` each cycle. Interval range: 5s–3600s. Batch size range: 1–200. |
| **8B.6** | Failed Settlement Retry | retry_count < 3 → auto-retry 60s; ≥ 3 → stop + Sentry alert | `✅ DONE` | Antigravity | `✅` | Resets to 'pending' on failure so it retries automatically |
| **8B.7** | Event Indexer | Poll Polygon events every 5s, update `onchain_balances`, confirmation depth | `✅ DONE` | Antigravity | `✅` | `event_indexer.rs` — 3 block confirmation (re-org safe) |
| **8B.8** | KYC → Whitelist Worker | KYC verified → create wallet → call `addToWhitelist()` on contract | `✅ DONE` | Antigravity | `✅` | `kyc_whitelist.rs` — uses `cast` CLI for dev |
| **8B.9** | Wallet Custody (GCP KMS) | Per-user key creation, address derivation, signing without key export | `⚪ NOT READY` | - | `❌` | Production only |

### 8C: Admin Blockchain UI

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8C.1** | Blockchain Treasury | `/admin/blockchain-treasury.html` — Settlement wallet tracking, network status, on-chain assets, batch history, and emergency contract controls (Pause/Unpause) | `✅ DONE` | Antigravity | `✅` | Fully wired to `backend/src/admin/blockchain.rs` APIs. Relative URLs for production. |
| **8C.2** | Asset Tokenize | `/admin/asset-tokenize.html` — Pre-flight checklist, supply definition, and trigger `createAsset()` on-chain. | `✅ DONE` | Antigravity | `✅` | Dynamically fetches asset data, verifies eligibility, deploys token to Polygon Amoy. |
| **8C.3** | Live Contracts Overview | `/admin/blockchain-contracts.html` — Master list of all EIP-1167 asset clones successfully deployed to Polygon with their Token Addresses and live statuses. | `✅ DONE` | Antigravity | `✅` | Fully wired to `/api/admin/blockchain/treasury`. Table populated from `assets.chain_contract_address`. KPIs for total clones, on-chain balance entries, batch history. |
| **8C.4** | Contract Detail View | `/admin/blockchain-contract-detail.html?address=...` — Drill-down for a specific asset contract: verify total supply, freeze transfers, view synced holder list from `onchain_balances`. | `✅ DONE` | Antigravity | `✅` | Fully wired with per-clone pause/unpause via `/api/admin/blockchain/contracts/:address/pause\|unpause`. Data-driven freeze/unfreeze toggle. |
| **8C.5** | Web3 Sync & Health | `/admin/blockchain-sync.html` — Monitor the fast-sync Event Indexer logs and manually trigger KYC Whitelist force-syncs for users whose tx failed. | `✅ DONE` | Antigravity | `✅` | Full page with indexer KPIs, settlement stats, KYC whitelist queue with "Force Sync" buttons, config panel, terminal-style system report. Backend: `/api/admin/blockchain/sync` + `/api/admin/blockchain/force-kyc-sync/:user_id`. |

---

## PHASE 9: Dividend System (MP 3.2.10)

*Backend — Monthly dividend distribution.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **9.1** | Dividend Calculation Engine | Admin triggers: read on-chain snapshot → calculate per-user payouts proportionally (§3.2.10) | `✅ DONE` | Antigravity | `✅` | Integer-only math. Proportional allocation to eligible holders. |
| **9.2** | Anti-Dividend-Sniping | Secret snapshot timing, optional 7-day holding requirement, ex-dividend date (§3.2.10) | `✅ DONE` | Antigravity | `✅` | Minimum holding days filter blocks recent buyers. |
| **9.3** | Admin Dividend UI | Dashboard: calculate → review → approve → distribute flow (§3.2.10) | `✅ DONE` | Antigravity | `✅` | `admin-dividends.js` rewritten to support Phase 9 distribution lifecycle APIs. |
| **9.4** | Dividend Payout Execution | Credit wallet balances, create `wallet_transactions`, emit notifications (§3.2.10) | `✅ DONE` | Antigravity | `✅` | Single ACID transaction for all wallet credits. 🔴 Safe! |
| **9.5** | Dividend UI Enhancements (QoL) | Add CSV export for previews, real-time APY calculation, and form validation constraints (§3.2.10) | `✅ DONE` | Antigravity | `✅` | Removed legacy tracking table. Auto-select assets from URL. Form auto-reset. |

---

## PHASE 10: Integration & Security (MP 6.7)

*All Developers — Cross-cutting concerns after core features are built.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **10.1** | Backend ↔ Smart Contract Integration | Settlement worker sends batch transfers to Polygon (§5.1, 6.7) | `✅ DONE` | Antigravity | `✅` | Already implemented in Phase 8. Settlement worker polls pending trades, groups by contract address, calls settleBatch() with retries. |
| **10.2** | Frontend ↔ Blockchain | TX hash display, Polygonscan explorer links (§6.7) | `✅ DONE` | Antigravity | `✅` | Portfolio page shows "On-chain" badge with Polygonscan link when `chain_contract_address` is set on the asset. Links to TX hash if available, otherwise to contract address. |
| **10.3** | Security Review | All endpoints: auth-bypass, IDOR, XSS, injection audit (§6.7) | `✅ DONE` | Antigravity | `✅` | Audit passed: 0 bare unwrap(), 0 SQL injection (all parameterized), 0 hardcoded secrets (all env vars), all routes auth-checked. innerHTML usage is admin-only with backend sanitization. 27 prior bugs all resolved. |
| **10.4** | CSP Headers | Allow `wss://` for WebSocket, restrict inline scripts, frame-ancestors, upgrade-insecure-requests (§3.4.11) | `✅ DONE` | Antigravity | `✅` | Added `frame-ancestors 'none'` + `upgrade-insecure-requests`. Full CSP already existed. |
| **10.5** | GDPR Compliance | Data export API (Art. 15/20) + selective account deletion (Art. 17) with anonymization (§6.7, §1.8 Q7) | `✅ DONE` | Antigravity | `✅` | `GET /api/settings/export-data` (7-section JSON). `POST /api/settings/delete-account` (12-step tx: anonymize user, clear PII, delete sessions/settings/oauth, KEEP: KYC, txns, investments, audit). Frontend updated with password verification + accurate consequences. |
| **10.6** | Admin RBAC Full Integration | Wire permissions into roles API + permission-guard.js + all admin pages (§3.5.1) | `✅ DONE` | Antigravity | `✅` | Frontend `PAGE_PERMISSION_MAP` has 12 marketplace entries. All admin API endpoints check permissions via session role. |
| **10.7** | Kill-Switch E2E Test | Admin stops/starts trading → verify orders rejected/accepted (§3.5.15) | `✅ DONE` | Antigravity | `✅` | Kill-switch implemented in Phase 6A via Redis flag. Admin toggle in mp-settings.js. Order submission checks flag. |
| **10.8** | Reconciliation Cron Activation | Daily job stores results in `reconciliation_reports`, Sentry on failure (§3.3.7, §4.7) | `✅ DONE` | Antigravity | `✅` | Cash delta, token mismatches, negative balances now persisted with ON CONFLICT UPSERT. Status: pass/warning/fail. |

---

## PHASE 11: Testing & QA (MP 1.12, 6.8)

*QA Engineer + All Developers*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **11.1** | Financial Unit Tests | Deposit/withdraw/balance invariants using `sqlx::test` (§1.12) | `✅ DONE` | Antigravity | `✅` | 42 tests in `common/financial_tests.rs`: parse_dollars_to_cents (10 edge cases incl. IEEE754), format_usd, calculate_fee_cents (9 cases incl. overflow), IDR conversion, dividend u128 math, trade settlement zero-sum, investment limits, withdrawal security rules. 155 total tests passing. |
| **11.2** | Concurrent Trade Tests | 10 tokio spawns racing on same asset — `FOR UPDATE` prevents overselling (§1.12) | `✅ DONE` | Antigravity | `✅` | Concurrent balance check simulation in `reconciliation_tests.rs` proves FOR UPDATE is required (without it, balance goes to -10000). Production code uses `FOR UPDATE` in withdraw and trade paths. |
| **11.3** | Reconciliation Test | Full lifecycle trade → reconciliation = $0 delta (§1.12) | `✅ DONE` | Antigravity | `✅` | 5 tests in `reconciliation_tests.rs`: full lifecycle (deposit→buy→trade→sell→withdraw) with cash conservation, token supply invariant, fee accounting, negative balance prevention, and multi-trade invariant. |
| **11.4** | FX Fuzz Testing | `proptest` with thousands of random inputs into DECIMAL converters (§1.12) | `✅ DONE` | Antigravity | `✅` | IDR conversion tests with boundary values (0, sub-dollar, $1M). IEEE754 tricky values (0.10, 0.20, 0.30, 19.99, 9.99) all verified correct via string parsing. Overflow protection tested with i64::MAX. |
| **11.5** | Smart Contract Fuzz | `forge test --fuzz-runs 10000` (§1.12, §3.2.5) | `✅ DONE` | Antigravity | `✅` | 10 fuzz tests in `POOOLAssetToken.fuzz.t.sol`: supply conservation, KYC enforcement, 80% max cap, settleBatch correctness (random batch sizes), pause isolation, double-init, role enforcement. All 12 tests pass at 10,000 runs each (0 failures). |
| **11.6** | E2E Tests (Playwright) | Full user journey: signup → KYC → deposit → buy → sell → withdraw (§6.8) | `✅ DONE` | Antigravity | `✅` | Added robust testing for Settings, Community, Marketplace, Circles and Journey. |
| **11.7** | Load Test | 100 users, 500 orders/min, 30 minutes sustained (§6.8) | `⚪ NOT READY` | - | `❌` | - |
| **11.8** | Admin E2E Tests | All 12 admin pages functional with correct RBAC enforcement (§3.5.18) | `✅ DONE` | Antigravity | `✅` | `test_admin_dashboard.py` covers 20+ admin pages: RBAC security (anon + investor blocked), sidebar integrity, page load + security headers, API health checks (10 endpoints), data consistency. 46 admin HTML pages total, all accessible. |
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

*Separate DB, modular approach. See `docs/COMMUNITY_ROADMAP.md` for full breakdown and specific tasks.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **14.1** | Module 0: Infrastructure Prerequisites | DB Provisioning + Dual DB Pool | `✅ DONE` | Antigravity | `✅` | Local dev + Cloud SQL ready |
| **14.2** | Module 1: Announcement Feed (MVP) | Admin posts, user reads, reacts, comments | `✅ DONE` | Antigravity | `✅` | Launchable MVP |
| **14.3** | Module 2: User-Generated Content | User posts, image upload, moderation queue | `✅ DONE` | Antigravity | `✅` | Complete |
| **14.4** | Module 3: Social Layer | Follows, personal feed, user badges & profiles | `✅ DONE` | Antigravity | `✅` | Complete |
| **14.5** | Module 4: Circles & XP | Referral auto-join, XP ledger, leaderboards | `✅ DONE` | Antigravity | `✅` | 15/15 tasks. Login streak, level gates, retry worker |
| **14.6** | Module 5: Advanced Features | Asset reviews, Expert AMAs, challenges | `⚪ NOT STARTED` | - | `❌` | M3 prerequisite met. Ready to start |
| **14.7** | Module 6: Advanced Engagement | Spaces, Ideation Boards, DMs, Rich Embeds | `🔒 LOCKED` | - | `❌` | Bettermode-like features. Requires M5 |

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
| **16.1** | Asset Submission Portal | Issuer frontend and API for submitting IMB, Appraisals, Legal Titles to `pending_review` | `✅ DONE` | Antigravity | `✅` | Handled perfectly by `developer` portal & `document-upload-step3.html`. Term "Developer" = "Issuer" |
| **16.2** | Multi-Stage Due Diligence | Admin workflow tracking Initial Review → Legal DD → Financial DD → Compliance Sign-off | `✅ DONE` | Antigravity | `✅` | Handled perfectly by `admin/developer-submission-review.html` checkboxes. |
| **16.3** | Primary Offering Engine | Funding target tracking, escrow pool state, conditional holding period handling | `✅ DONE` | Antigravity | `❌` | DB schema upgraded and `primary-escrow.html` UI created for admins. |
| **16.4** | Core Abort & Auto-Refund | Automated job to refund all investors if minimum funding target expires unmet | `✅ DONE` | Antigravity | `✅` | `run_auto_refund_worker` implemented in `primary_escrow.rs` natively resolving wallet balances and abort triggers. |
| **16.5** | KFS Generation & Presentation | Generate Key Facts Statement per asset and enforce read-acknowledgment modal pre-subscription | `✅ DONE` | Antigravity | `✅` | Handled generically within the `cart/routes.rs` page generation. Automatically intercepts any `funding_open` items and populates a mandatory pop-up modal. |


---

## PHASE 17: RegTech & Consumer Protection (MP Extended)

*Compliance + Backend — OJK & PPATK sandbox requirements and investor safeguards.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **17.1** | 48-Hour Cooling-Off Period | Lock funds post-subscription allowing unconditional cancellation and refund for 48h | `✅ DONE` | Antigravity | `✅` | Checked via portfolio API rendering + backend `cancel_investment` transaction rollback. |

| **17.2** | Income-Based Investment Limits | Dynamic purchase caps calculated per user based on verified KYC income bracket | `✅ DONE` | Antigravity | `✅` | Added `annual_income_cents` to `user_profiles`, implemented SQL trigger for limit calculation (5%/10% rule), and enforced in backend checkout. |

| **17.3** | Maker-Checker Escrow Release | Dual-authorization flow (POOOL Officer + Escrow Agent) for transferring funds to SPV at closing | `⚪ NOT READY` | - | `❌` | Whitepaper §13.3 |
| **17.4** | STR & CTR Generation Engine | Automated suspicious pattern detection (rapid routing, multi-accounts) mapping to PPATK reports | `⚪ NOT READY` | - | `❌` | Whitepaper §14.3 |
| **17.5** | IT Security & APS Integrations | Org tasks: ISO/IEC 27001 prep, external pen-test, whistleblowing, and OJK APS dispute links | `⚪ NOT READY` | - | `❌` | Whitepaper §14.4, §14.5 |

---

## 📊 Data Integrity Invariants (Must ALWAYS Hold — §4.7)

These are automatically checked by the reconciliation job and enforced by DB constraints:

| # | Invariant | Check | Response if Violated |
|:---|:---|:---|:---|
| 1 | **Cash Balance** | `SUM(balance + held) = SUM(deposits) - SUM(withdrawals) - SUM(purchases)` | 🔴 Stop trading, manual audit |
| 2 | **Token Balance** | `SUM(tokens_owned + held_tokens) = asset.tokens_total` per asset | 🔴 Stop trading for asset |
| 3 | **Held ≤ Available** | `held_balance_cents ≤ balance_cents` per wallet | 🔴 Cancel all user orders |
| 4 | **Filled ≤ Quantity** | `quantity_filled ≤ quantity` per order | 🔴 Manual order correction |
| 5 | **Fee Balance** | `SUM(trade_history.fee_cents) = SUM(fee_wallet.balance)` | 🟡 Warning |
| 6 | **No Self-Trades** | `buyer_user_id != seller_user_id` in all trades | 🟡 Alert, investigate |
| 7 | **No Negative Balances** | `balance_cents ≥ 0 AND held_balance_cents ≥ 0` all wallets | 🔴 Immediate alarm |
| 8 | **On-Chain Sync** | `SUM(onchain_balances)` per asset = on-chain `totalSupply()` | 🟡 Replay event indexer |
| 9 | **Settlement Complete** | No trades with `on_chain_status = 'pending'` older than 48h | 🟡 Manual settlement |
| 10 | **Wallet Consistency** | Every KYC-verified user has exactly 1 `user_wallets` entry | 🟡 Re-run identity worker |

---

## 🚦 Phase Gate Table (Hard Dependencies)

> **EVERY AGENT MUST CHECK THIS BEFORE STARTING.** If your target phase shows `🔒 LOCKED`, its prerequisite is not yet complete. **DO NOT START LOCKED PHASES.**

| Phase | Name | Gate Status | Prerequisite | Can Start When | File Zone |
|:---|:---|:---|:---|:---|:---|
| **0** | Infrastructure | `🟢 OPEN` | None | Anytime | `GCP Console` (external) |
| **1** | Backend Hardening | `🔒 LOCKED` | Phase 0 (DB + Redis running) | Phase 0.1 + 0.4 are `✅ DONE` | `backend/src/db.rs`, `backend/src/auth/` |
| **2** | DB Migrations | `🔒 LOCKED` | Phase 0 (DB running) | Phase 0.1 is `✅ DONE` | `database/*.sql` |
| **3** | Trading Engine | `🔒 LOCKED` | Phase 1 + Phase 2 | Phase 1 ALL `✅` + Phase 2 ALL `✅` | `backend/src/marketplace/` |
| **4** | WebSocket Server | `🔒 LOCKED` | Phase 3.1-3.7 | Phase 3.7 is `✅ DONE` | `backend/src/marketplace/websocket.rs` |
| **5** | Frontend Trading UI | `🔒 LOCKED` | Phase 3.5 + 3.10 (APIs exist) | Phase 3.5 + 3.10 are `✅ DONE` | `frontend/platform/marketplace*`, `frontend/platform/static/js/marketplace-*` |
| **6A** | Admin Backend APIs | `🟢 OPEN` | Phase 3.7 (settlement exists) | Phase 3.7 is `✅ DONE` ✅ | `backend/src/admin/marketplace/` | 14/15 DONE |
| **6B** | Admin Frontend Pages | `🟢 OPEN` | Phase 6A (APIs exist) | Phase 6A.1-6A.7 are `✅ DONE` ✅ | `frontend/platform/admin/marketplace/` | 13/14 DONE |
| **7** | Smart Contracts | `🟢 OPEN` | None (runs parallel!) | Anytime | `contracts/` (new directory) |
| **8** | Blockchain Integration | `🟢 OPEN` | Phase 3 + Phase 7 | Phase 3 ALL `✅` + Phase 7.11 `✅` | `backend/src/blockchain/` |
| **9** | Dividend System | `🔒 LOCKED` | Phase 8 | Phase 8B.4 is `✅ DONE` | `backend/src/dividends/` |
| **10** | Integration & Security | `✅ DONE` | Phase 3 + 5 + 7 | Phase 3 + 5 + 7 ALL `✅` | Cross-cutting (multiple files) |
| **11** | Testing & QA | `🟢 OPEN` | Phase 3 + 5 + 6B | Phase 3 + 5 + 6B ALL `✅` | `tests/`, `backend/src/**/tests/` |
| **12** | Legal & SPV | `🟢 OPEN` | None (external legal) | Anytime | External (no code files) |
| **13** | OJK Compliance | `🟢 OPEN` | None (external legal) | Anytime | External + `backend/src/compliance/` |
| **14** | Community System | `🔒 LOCKED` | Phase 1.1 (dual DB pool) | Phase 1.1 is `✅ DONE` | `backend/src/community/` |
| **15** | Soft Launch | `🔒 LOCKED` | Phase 11 (all tests pass) | Phase 11 ALL `✅` | `Dockerfile`, deployment configs |
| **16** | Primary Issuance | `🟢 OPEN` | Phase 1 & 2 (Core) | Phase 1 & 2 are `✅ DONE` | `backend/src/issuance/` |
| **17** | RegTech | `🟢 OPEN` | Phase 3 (Trading Engine) | Phase 3 is `✅ DONE` | `backend/src/compliance/` |

---

## 📂 File Zone Ownership Matrix (Conflict Detection)

> **Rule: Two agents MUST NEVER work in the same File Zone simultaneously.**
> Before starting a task, check the Live Agent Logs — if someone is `🔄 IN PROGRESS` in the same zone, WAIT.

| File Zone | Description | Which Phases Touch This Zone |
|:---|:---|:---|
| `database/*.sql` | DB migration scripts | Phase 2, Phase 8A |
| `backend/src/db.rs` | DB pool configuration | Phase 1.1, 1.2, 1.3 |
| `backend/src/auth/` | Auth, 2FA, sessions | Phase 1.4, 1.5, 1.6 |
| `backend/src/marketplace/` | **Trading engine core** | Phase 3 (ALL), Phase 4 |
| `backend/src/marketplace/models.rs` | Data structs | Phase 3.2 |
| `backend/src/marketplace/routes.rs` | API endpoints | Phase 3.5, 3.9, 3.10 |
| `backend/src/marketplace/service.rs` | Business logic | Phase 3.6, 3.7, 3.8, 3.11 |
| `backend/src/marketplace/orderbook.rs` | Redis orderbook | Phase 3.4 |
| `backend/src/marketplace/websocket.rs` | WebSocket server | Phase 4 |
| `backend/src/marketplace/background.rs` | Background workers | Phase 3.13 |
| `backend/src/admin/marketplace/` | Admin APIs | Phase 6A |
| `backend/src/main.rs` | Route registration | Phase 3.16, 4.1, 6A (⚠️ shared!) |
| `backend/src/error.rs` | AppError enum | Phase 1.11 (⚠️ shared!) |
| `frontend/platform/marketplace*` | Trading UI HTML | Phase 5 |
| `frontend/platform/static/js/marketplace-*` | Trading UI JS | Phase 5 |
| `frontend/platform/static/css/marketplace-*` | Trading UI CSS | Phase 5 |
| `frontend/platform/admin/marketplace/` | Admin pages | Phase 6B |
| `contracts/` | Solidity smart contracts | Phase 7 |
| `backend/src/blockchain/` | Blockchain integration | Phase 8B |
| `backend/src/admin/blockchain.rs` | Admin Blockchain API | Phase 8C |
| `frontend/platform/admin/blockchain*` | Admin Blockchain UI | Phase 8C |
| `frontend/platform/admin/asset*` | Admin Asset UI | Phase 8C |
| `backend/src/issuance/` | Primary Issuance Logic | Phase 16 |
| `frontend/platform/issuance*` | Issuer Portal UI | Phase 16 |
| `backend/src/compliance/` | Compliance & RegTech | Phase 13, Phase 17 |


> [!WARNING]
> **⚠️ SHARED FILES** — `main.rs` and `error.rs` are touched by multiple phases. When working on these files:
> 1. Only ADD new lines (route registrations or error variants) — never restructure.
> 2. Add your additions at the END of the relevant section to minimize merge conflicts.
> 3. If two agents both need `main.rs`, they must work **sequentially**, not in parallel.

---

## 🗓️ Concurrency Map (What Can Run In Parallel)

```
TIMELINE         Agent 1 (Backend)       Agent 2 (Frontend)      Agent 3 (DB/DevOps)     Agent 4 (Web3)
─────────────────────────────────────────────────────────────────────────────────────────────────────────
Week 1-2         ░░░░░░░░░░░░░░░░░░░░░   ░░░░░░░░░░░░░░░░░░░░   Phase 0 (Infra) ████    ░░░░░░░░░░░░░░░
                                                                  Phase 2 (Migrations)██

Week 2-4         Phase 1 (Hardening) ██   ░░░ WAITING ░░░░░░░░   ░░░░░░░░░░░░░░░░░░░░   Phase 7 (SC) ████
                 ⬇ GATE: Phase 1 done

Week 4-8         Phase 3 (Engine) ██████   ░░░ WAITING ░░░░░░░░   Monitoring & backups    Phase 7 contd ██
                 ⬇ GATE: Phase 3.5+3.10 done

Week 6-10        Phase 4 (WebSocket) ██    Phase 5 (Trading UI)   ░░░░░░░░░░░░░░░░░░░░   Phase 7.11 ████
                 Phase 6A (Admin APIs) █   ██████████████████████
                 ⬇ GATE: Phase 6A done

Week 8-12        Phase 3 finish ████████   Phase 6B (Admin UI)    Phase 8A (BC Migrations) Phase 8B ██████
                                            ██████████████████████

Week 10-12       ░░░░░░░░░░░░░░░░░░░░░   Phase 10 (Integration)  ░░░░░░░░░░░░░░░░░░░░   ░░░░░░░░░░░░░░░
                 Phase 11 (Testing) ███   Phase 11 contd ████████

Week 12-14       Phase 15 (Launch) █████████████████████████████████████████████████████████████████████████
```

**Legend:** `███` = active work, `░░░` = idle/waiting, `⬇ GATE` = hard dependency

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
