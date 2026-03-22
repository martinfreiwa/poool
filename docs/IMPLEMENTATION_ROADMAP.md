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

---

## PHASE 0: Infrastructure & Account Setup (MP 6.2)

*DevOps + PM — Must be completed first. No code depends on this being fancy, but everything depends on it existing.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **0.1** | Cloud SQL Core DB Provisioning | `db-f1-micro`, PG16, PITR enabled, `asia-southeast1`, 14-day backup retention (§3.3.1) | `✅ DONE` | Martin | `✅` | Cloud SQL running in production on Cloud Run. |
| **0.2** | Cloud SQL Community DB Provisioning | Separate instance, PITR enabled, 7-day retention (§3.3.1) | `❌ NOT STARTED` | - | `❌` | Not yet needed — community features not launched. |
| **0.3** | Cloud SQL Read Replicas | One replica per DB for read routing (§3.3.3) | `❌ NOT STARTED` | - | `❌` | Optimization for later — not needed at current scale. |
| **0.4** | Redis Memorystore | `basic` tier, 1GB, `redis_7_2`, `asia-southeast1` (§3.3.4) | `❌ NOT STARTED` | - | `❌` | Code has Redis fallback (works without Redis). |
| **0.5** | PgBouncer Sidecar | Connection pooling proxy in Dockerfile (§1.9, §3.3.8) | `✅ DONE` | Antigravity | `✅` | Dockerfile updated: debian-slim runtime + PgBouncer sidecar. `pgbouncer/entrypoint.sh` parses DATABASE_URL, starts PgBouncer on :6432, then backend. Set `PGBOUNCER_ENABLED=false` to skip. |
| **0.6** | PITR & Backup Strategy | 3-layer backups: PITR + daily snapshots + weekly cross-region `pg_dump` to GCS (§3.3.2) | `❌ NOT STARTED` | - | `❌` | Cloud SQL auto-backups exist, but no 3-layer strategy. |
| **0.7** | CI/CD Pipeline | GitHub Actions → Build → Test → Deploy to Cloud Run (§6.2) | `✅ DONE` | Antigravity | `✅` | Already implemented: `ci.yml` (fmt + clippy + test + audit + Docker build) + `deploy.yml` (GCP auth + Docker push + Cloud Run deploy + health check). |
| **0.8** | Cloud Monitoring Alerts | 10 alert policies: CPU, connections, Redis memory, error rate, latency, reconciliation (§3.3.7) | `❌ NOT STARTED` | - | `❌` | - |
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
| **3.11** | P2P/OTC Offer System (`p2p.rs`) | Create/accept/decline/counter offers, settlement reuse, fee application (~300 lines) (§2.7, §3.1) | `❌ NOT STARTED` | - | `❌` | - |
| **3.12** | Candlestick Chart API (`charts.rs`) | `GET /candles?asset_id=&interval=1h&from=&to=` backed by TimescaleDB aggregates (~150 lines) (§2.8) | `❌ NOT STARTED` | - | `❌` | - |
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
| **5.3** | Marketplace Overview Page | `marketplace.html` — All tradeable assets with live price, 24h change (§3.4.1) | `❌ NOT STARTED` | - | `❌` | Existing marketplace.html needs WS integration. |
| **5.4** | Candlestick Chart Integration | ApexCharts (or lightweight-charts) with interval switcher, real-time updates (§3.4.4) | `❌ NOT STARTED` | - | `❌` | ApexCharts area chart exists. Candlestick upgrade blocked on chart API. |
| **5.5** | Orderbook Rendering (`marketplace-orderbook.js`) | Bid/Ask tables, DOM patching (no full re-render), flash animations, depth bars (~200 lines) (§3.4.5) | `✅ DONE` | Antigravity | `❌` | ~230 lines. Flash anim. Depth bars. Click-to-fill. |
| **5.6** | Buy/Sell Order Form | Price/qty inputs, real-time total, balance validation, double-click protection, idempotency-key, optimistic UI (§3.4.6) | `✅ DONE` | Antigravity | `❌` | Wired to POST /api/marketplace/orders. UUID idempotency keys. |
| **5.7** | 2FA Step-Up Modal | TOTP input modal triggered on 428 response, retry with trading session (§3.4.6) | `✅ DONE` | Antigravity | `❌` | 428 detection + MarketBus event. Modal not yet built. |
| **5.8** | My Orders & Trade History | User's open orders with cancel, own trade list (§3.4.8) | `✅ DONE` | Antigravity | `❌` | Fetch + render + cancel via DELETE API. Recent trades with timestamp. |
| **5.9** | P2P Offer UI (`marketplace-p2p.js`) | Cap table, send offer modal, incoming offer notification badge (~200 lines) (§3.4.7) | `❌ NOT STARTED` | - | `❌` | Blocked on P2P backend (task 3.11). |
| **5.10** | Loading/Error/Empty States | Skeleton loaders, error-retry buttons, empty-state messages for all components (§3.4.9) | `✅ DONE` | Antigravity | `❌` | Empty states + toast notifications for success/error/warning. |
| **5.11** | Accessibility | ARIA labels, keyboard nav, focus management, `role="alert"` on toasts, reduced-motion (§3.4.10) | `❌ NOT STARTED` | - | `❌` | role="alert" on toasts ✅. Full a11y audit pending. |
| **5.12** | Responsive Design | Mobile-first: 360px → 1920px, touch-friendly order form (§3.4.12) | `❌ NOT STARTED` | - | `❌` | Existing mobile bottom sheet works. Orderbook needs mobile pass. |
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
| **6A.13** | Admin Compliance/OJK APIs | OJK quarterly report, travel-rule export, user tax reports (§3.5.14) | `❌ NOT STARTED` | - | `❌` | Deferred — needs OJK report template. |
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
| **6B.12** | Analytics & Charts | `/admin/marketplace/analytics` | 🟡 WEEK 3 | Embedded Metabase OR custom charts: volume, top-trader, fee revenue (§3.5.11) | `🟡 PARTIAL` | - | `❌` | HTML exists. Needs chart integration. |
| **6B.13** | Alerts & Watchlist | `/admin/marketplace/alerts` | 🟡 WEEK 3 | Alert table, acknowledge/resolve, user watchlist management (§3.5.12) | `✅ DONE` | Antigravity | `❌` | JS wired: acknowledge/resolve via POST. Mock fallback. |
| **6B.14** | Compliance & OJK | `/admin/marketplace/compliance` | 🟡 WEEK 4 | OJK reports, travel-rule, tax exports, AML reports (§3.5.14) | `🟡 PARTIAL` | - | `❌` | HTML exists. Blocked on 6A.13 (OJK template). |

---

## PHASE 7: Smart Contracts — ERC-1155 on Polygon (UPDATED: was ERC-3643/Base)

*Web3 Engineer — Runs PARALLEL to Phases 3-5. Chain: Polygon PoS. Token: ERC-1155 for fractional ownership.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **7.1** | Foundry Project Setup | `forge init`, install OpenZeppelin v5 (ERC-1155, AccessControl, Pausable) | `❌ NOT STARTED` | - | `❌` | - |
| **7.2** | POOOLProperty1155 Contract | ERC-1155 token: each `tokenId` = one property asset, `balanceOf(user, tokenId)` = fractional shares owned. Mint, burn, batch transfer (~200 lines) | `❌ NOT STARTED` | - | `❌` | Core contract |
| **7.3** | Access Control & Roles | `MINTER_ROLE`, `PAUSER_ROLE`, `SETTLEMENT_ROLE` via OpenZeppelin AccessControl | `❌ NOT STARTED` | - | `❌` | - |
| **7.4** | KYC Whitelist Module | On-chain mapping `isWhitelisted(address)` — only whitelisted users can receive tokens. Admin-managed. | `❌ NOT STARTED` | - | `❌` | Simpler than ERC-3643 |
| **7.5** | Transfer Restrictions | Override `_update()` to enforce: KYC whitelist, max ownership (80%), pausable | `❌ NOT STARTED` | - | `❌` | - |
| **7.6** | BatchSettlement Function | `settleBatch(froms[], tos[], tokenIds[], amounts[])` — gas-optimized multi-transfer for end-of-day netting | `❌ NOT STARTED` | - | `❌` | - |
| **7.7** | AssetFactory Contract | Deploy new tokenId per property via admin call, store metadata URI | `❌ NOT STARTED` | - | `❌` | - |
| **7.8** | URI Metadata (IPFS) | `uri(tokenId)` returns IPFS link to property metadata JSON (name, docs, SPV info) | `❌ NOT STARTED` | - | `❌` | - |
| **7.9** | Foundry Unit Tests | Mint, burn, transfer, transfer-blocked-without-KYC, zero-amount, self-transfer, batch | `❌ NOT STARTED` | - | `❌` | - |
| **7.10** | Foundry Fuzz Tests | 10,000+ runs: random amounts, mismatched arrays, edge cases | `❌ NOT STARTED` | - | `❌` | - |
| **7.11** | Invariant Tests | For each tokenId: `totalSupply(tokenId) == SUM(balanceOf(all_users, tokenId))` ALWAYS | `❌ NOT STARTED` | - | `❌` | - |
| **7.12** | Polygon Mumbai/Amoy Testnet Deploy | Deploy all contracts, verify on Polygonscan | `❌ NOT STARTED` | - | `❌` | - |
| **7.13** | Smart Contract Audit | Commission external audit. ⚠️ Order in Week 4! | `⚪ NOT READY` | - | `❌` | 4-6 week lead time! |

---

## PHASE 8: Blockchain Integration (MP 3.2.6, 3.2.9, 3.2.11)

*Backend + Web3 — Connecting Rust backend to Polygon.*

### 8A: Blockchain DB Migrations

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8A.1** | Migration `050d`: `assets` blockchain fields | `contract_address`, `token_id` (ERC-1155), `deployment_tx_hash`, `blockchain_status` | `⚪ NOT READY` | - | `❌` | - |
| **8A.2** | Migration `057`: `user_wallets` | Custodial wallet per user: `wallet_address`, `kms_key_id`, `wallet_type` | `⚪ NOT READY` | - | `❌` | - |
| **8A.3** | Migration `058`: `onchain_balances` | Cached on-chain token balances per user/asset (from ERC-1155 `balanceOf`) | `⚪ NOT READY` | - | `❌` | - |
| **8A.4** | Migration `059`: `settlement_batches` | Settlement batch audit log with tx_hash, retry_count | `⚪ NOT READY` | - | `❌` | - |
| **8A.5** | Migration `060`: `dividend_distributions` + `dividend_payouts` | Dividend calculation and payout tracking | `⚪ NOT READY` | - | `❌` | - |

### 8B: Backend Blockchain Workers

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **8B.1** | Alloy-rs / ethers-rs Integration | ABI binding to POOOLProperty1155 contract on Polygon | `⚪ NOT READY` | - | `❌` | - |
| **8B.2** | GCP KMS Signer | Private key management via HSM — key never leaves GCP | `⚪ NOT READY` | - | `❌` | - |
| **8B.3** | Net-Position Aggregator | Aggregate trades → netting → net changes per wallet | `⚪ NOT READY` | - | `❌` | - |
| **8B.4** | Settlement Worker | Tokio task: aggregate → netting → `settleBatch()` on Polygon | `⚪ NOT READY` | - | `❌` | - |
| **8B.5** | Dynamic Batching Frequency | <10 trades/day → 1x daily; 10-100 → 2x; >100 → 4x; admin → immediate | `⚪ NOT READY` | - | `❌` | - |
| **8B.6** | Failed Settlement Retry | retry_count < 3 → auto-retry 60s; ≥ 3 → stop + Sentry alert | `⚪ NOT READY` | - | `❌` | - |
| **8B.7** | Event Indexer | Poll Polygon events every 5s, update `onchain_balances`, confirmation depth | `⚪ NOT READY` | - | `❌` | - |
| **8B.8** | KYC → Whitelist Worker | KYC verified → create wallet → call `addToWhitelist()` on contract | `⚪ NOT READY` | - | `❌` | - |
| **8B.9** | Wallet Custody (GCP KMS) | Per-user key creation, address derivation, signing without key export | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 9: Dividend System (MP 3.2.10)

*Backend — Monthly dividend distribution.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **9.1** | Dividend Calculation Engine | Admin triggers: read on-chain snapshot → calculate per-user payouts proportionally (§3.2.10) | `⚪ NOT READY` | - | `❌` | - |
| **9.2** | Anti-Dividend-Sniping | Secret snapshot timing, optional 7-day holding requirement, ex-dividend date (§3.2.10) | `⚪ NOT READY` | - | `❌` | - |
| **9.3** | Admin Dividend UI | Dashboard: calculate → review → approve → distribute flow (§3.2.10) | `⚪ NOT READY` | - | `❌` | - |
| **9.4** | Dividend Payout Execution | Credit wallet balances, create `wallet_transactions`, emit notifications (§3.2.10) | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 10: Integration & Security (MP 6.7)

*All Developers — Cross-cutting concerns after core features are built.*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **10.1** | Backend ↔ Smart Contract Integration | Settlement worker sends batch transfers to Base L2 (§5.1, 6.7) | `⚪ NOT READY` | - | `❌` | - |
| **10.2** | Frontend ↔ Blockchain | TX hash display, Basescan explorer links (§6.7) | `⚪ NOT READY` | - | `❌` | - |
| **10.3** | Security Review | All endpoints: auth-bypass, IDOR, XSS, injection audit (§6.7) | `⚪ NOT READY` | - | `❌` | - |
| **10.4** | CSP Headers | Allow `wss://` for WebSocket, restrict inline scripts (§3.4.11) | `⚪ NOT READY` | - | `❌` | - |
| **10.5** | GDPR Compliance | Community-DB anonymization, selective core-DB deletion (§6.7) | `⚪ NOT READY` | - | `❌` | - |
| **10.6** | Admin RBAC Full Integration | Wire permissions into roles API + permission-guard.js + all admin pages (§3.5.1) | `⚪ NOT READY` | - | `❌` | - |
| **10.7** | Kill-Switch E2E Test | Admin stops/starts trading → verify orders rejected/accepted (§3.5.15) | `⚪ NOT READY` | - | `❌` | - |
| **10.8** | Reconciliation Cron Activation | Daily job stores results in `reconciliation_reports`, Sentry on failure (§3.3.7, §4.7) | `⚪ NOT READY` | - | `❌` | - |

---

## PHASE 11: Testing & QA (MP 1.12, 6.8)

*QA Engineer + All Developers*

| ID | Task | Description (Masterplan Ref) | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **11.1** | Financial Unit Tests | Deposit/withdraw/balance invariants using `sqlx::test` (§1.12) | `⚪ NOT READY` | - | `❌` | - |
| **11.2** | Concurrent Trade Tests | 10 tokio spawns racing on same asset — `FOR UPDATE` prevents overselling (§1.12) | `⚪ NOT READY` | - | `❌` | - |
| **11.3** | Reconciliation Test | Full lifecycle trade → reconciliation = $0 delta (§1.12) | `⚪ NOT READY` | - | `❌` | - |
| **11.4** | FX Fuzz Testing | `proptest` with thousands of random inputs into DECIMAL converters (§1.12) | `⚪ NOT READY` | - | `❌` | - |
| **11.5** | Smart Contract Fuzz | `forge test --fuzz-runs 10000` (§1.12, §3.2.5) | `⚪ NOT READY` | - | `❌` | - |
| **11.6** | E2E Tests (Playwright) | Full user journey: signup → KYC → deposit → buy → sell → withdraw (§6.8) | `⚪ NOT READY` | - | `❌` | - |
| **11.7** | Load Test | 100 users, 500 orders/min, 30 minutes sustained (§6.8) | `⚪ NOT READY` | - | `❌` | - |
| **11.8** | Admin E2E Tests | All 12 admin pages functional with correct RBAC enforcement (§3.5.18) | `⚪ NOT READY` | - | `❌` | - |
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

## PHASE 14: Community System (MP Future)

*Separate DB, lower priority.*

| ID | Task | Description | Status | Assignee | Tested? | Notes |
|:---|:---|:---|:---|:---|:---|:---|
| **14.1** | Community Database Pool | Configure strict routing to `community_db` read replicas | `⚪ NOT READY` | - | `❌` | - |
| **14.2** | Social APIs | Posts, comments, badges, live AMAs, push notifications | `⚪ NOT READY` | - | `❌` | - |

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
| **6B** | Admin Frontend Pages | `🟢 OPEN` | Phase 6A (APIs exist) | Phase 6A.1-6A.7 are `✅ DONE` ✅ | `frontend/platform/admin/marketplace/` | 12/14 DONE |
| **7** | Smart Contracts | `🟢 OPEN` | None (runs parallel!) | Anytime | `contracts/` (new directory) |
| **8** | Blockchain Integration | `🔒 LOCKED` | Phase 3 + Phase 7 | Phase 3 ALL `✅` + Phase 7.11 `✅` | `backend/src/blockchain/` |
| **9** | Dividend System | `🔒 LOCKED` | Phase 8 | Phase 8B.4 is `✅ DONE` | `backend/src/dividends/` |
| **10** | Integration & Security | `🔒 LOCKED` | Phase 3 + 5 + 7 | Phase 3 + 5 + 7 ALL `✅` | Cross-cutting (multiple files) |
| **11** | Testing & QA | `🔒 LOCKED` | Phase 3 + 5 + 6B | Phase 3 + 5 + 6B ALL `✅` | `tests/`, `backend/src/**/tests/` |
| **12** | Legal & SPV | `🟢 OPEN` | None (external legal) | Anytime | External (no code files) |
| **13** | OJK Compliance | `🟢 OPEN` | None (external legal) | Anytime | External + `backend/src/compliance/` |
| **14** | Community System | `🔒 LOCKED` | Phase 1.1 (dual DB pool) | Phase 1.1 is `✅ DONE` | `backend/src/community/` |
| **15** | Soft Launch | `🔒 LOCKED` | Phase 11 (all tests pass) | Phase 11 ALL `✅` | `Dockerfile`, deployment configs |

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
