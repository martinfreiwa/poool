# Page Audit: Marketplace Index

Date: 2026-04-26
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/`
Template: `frontend/platform/admin/marketplace/index.html`
JavaScript: `frontend/platform/static/js/mp-index.js`
CSS: `frontend/platform/static/css/mp-index.css`, `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

`/admin/marketplace/` is implemented as a real admin marketplace overview route with KPI, recent-trade, and health APIs. The 2026-04-26 fix pass removed the production mock fallback, safe-rendered recent trades, aligned marketplace view permissions, made health failures visible, and added authenticated E2E coverage.

---

## Tested Scope

- Reviewed `frontend/platform/admin/marketplace/index.html`.
- Reviewed `frontend/platform/static/js/mp-index.js`.
- Reviewed `frontend/platform/static/css/mp-index.css`.
- Reviewed route registration in `backend/src/admin/mod.rs`.
- Reviewed marketplace page authorization in `backend/src/admin/pages.rs`.
- Reviewed `/api/admin/marketplace/stats`, `/recent-trades`, and `/health` in `backend/src/admin/marketplace.rs`.
- Reviewed marketplace table migrations: `database/050_marketplace_orders.sql` and `database/051_trade_history.sql`.
- Checked for related tests covering the overview APIs.
- Ran JavaScript syntax check and backend compile check.
- Ran authenticated Playwright E2E with seeded admin session and trade fixture.
- Ran JavaScript, Python, Rust format, and Rust compile checks.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/` | Registered through `page_admin_generic`. |
| URL alias | `/admin/marketplace/index.html` | Registered. |
| Template | `frontend/platform/admin/marketplace/index.html` | KPI cards, live trades table, system health grid. |
| JS | `frontend/platform/static/js/mp-index.js` | Fetches stats, recent trades, and health; renders explicit loading/error states. |
| CSS | `frontend/platform/static/css/mp-index.css` | Live KPI pulse and trade-side badge styling. |
| Backend page route | `GET /admin/marketplace/` | `backend/src/admin/mod.rs`; `backend/src/admin/pages.rs`. |
| Backend API route | `GET /api/admin/marketplace/stats` | Requires `marketplace.view`; returns `MarketplaceStats`. |
| Backend API route | `GET /api/admin/marketplace/recent-trades` | Requires `marketplace.view`; returns recent `AdminTrade` rows. |
| Backend API route | `GET /api/admin/marketplace/health` | Requires `marketplace.view`; returns explicit component health. |
| Database table | `market_orders` | Open orders, pending review count, active users. |
| Database table | `trade_history` | Volume, trade count, fees, recent trades, last trade. |
| Database table | `assets`, `users` | Joined for recent trade asset names and user emails. |
| Redis key | `marketplace:trading_enabled` | Used for trading status. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `a[href="/admin/"]` | Navigate to admin dashboard. | Link navigation. | Route exists. | Static verified; runtime blocked. |
| Marketplace breadcrumb | `a[href="/admin/marketplace/"]` | Stay on marketplace overview. | Link navigation. | Route exists. | Static verified; runtime blocked. |
| Topbar health dots | `.admin-health-indicators` | Reflect matching engine, DB, and WebSocket status. | Yes; `renderHealth()`. | `GET /api/admin/marketplace/health`. | Authenticated E2E passed. |
| Trading Status KPI | `#kpi-trading-status` | Show live/halted/unknown from Redis-backed stats. | `renderKPIs()`. | `GET /api/admin/marketplace/stats`. | Authenticated E2E passed. |
| Open Orders KPI | `#kpi-open-orders` | Show open/partially filled order count. | `renderKPIs()`. | `market_orders` count. | Authenticated E2E passed. |
| 24h Volume KPI | `#kpi-volume` | Show integer-cent 24h volume as USD. | `renderKPIs()`. | `trade_history` sum using integer cents. | Authenticated E2E passed. |
| Pending Approvals KPI | `#kpi-pending` | Show pending review count. | `renderKPIs()`. | `market_orders.status = 'pending_review'`. | Authenticated E2E passed. |
| KPI trend/subtext labels | `.admin-kpi-change`, `.admin-kpi-subtext` | Match the loaded API data or neutral state. | Yes; `renderKPIs()`. | Existing stats fields. | Authenticated E2E passed. |
| Recent Live Trades table | `#live-trades-body` | Render recent trades from DB, or real empty/error state. | `renderTrades()` using DOM text nodes. | `GET /api/admin/marketplace/recent-trades`. | Authenticated E2E passed with HTML-like asset title. |
| View All trades | `a[href="/admin/marketplace/trades.html"]` | Navigate to full trades page. | Link navigation. | Route exists. | Static verified; runtime blocked. |
| System Health grid | `#health-grid` | Render DB, Redis, matching, WebSocket, queue, last-trade status. | `renderHealth()`. | `GET /api/admin/marketplace/health`. | Authenticated E2E passed. |
| Auto refresh | `setInterval(..., 30000)` | Refresh API-backed cards every 30 seconds. | Yes. | Same APIs. | Static verified; refresh silently skips failed calls. |

---

## Frontend Findings

### P1 - API failures and authorization failures are replaced with mock operational data

Status: fixed 2026-04-26

Location:

- Template: `frontend/platform/admin/marketplace/index.html:67`, `frontend/platform/admin/marketplace/index.html:79`, `frontend/platform/admin/marketplace/index.html:91`, `frontend/platform/admin/marketplace/index.html:103`
- JS: `frontend/platform/static/js/mp-index.js:13-21`, `frontend/platform/static/js/mp-index.js:144-188`, `frontend/platform/static/js/mp-index.js:200-208`
- Backend: `backend/src/admin/pages.rs:178-188`, `backend/src/admin/marketplace.rs:214-218`, `backend/src/admin/marketplace.rs:316-319`

Problem:

The page allows users with `marketplace.view`, `marketplace.manage`, or `marketplace.compliance` to render the overview, but the stats and recent-trades APIs require `marketplace.view`. If a compliance/manage-only admin can render the page but receives API 403s, `fetchJSON()` returns `null` and the initial load calls `useMockData()`, showing realistic "LIVE" KPIs, mock trades, and green health data.

Expected:

The page should show a visible error or permission message when required APIs fail. Permission gates should be aligned across the page and APIs, and production admin pages should not show mock operational data.

Evidence:

Static review shows `fetchJSON()` swallows all non-2xx responses and initial load uses mock data whenever either stats or health is missing. `page_admin_generic` has a broader marketplace page guard than the API handlers.

Fix:

Removed `useMockData()`, converted initial and refresh loads to `Promise.allSettled()`, rendered visible per-section errors, and made the overview page require `marketplace.view` to match the stats/recent-trades/health APIs.

### P1 - Recent trades render database-backed labels through `innerHTML`

Status: fixed 2026-04-26

Location:

- JS: `frontend/platform/static/js/mp-index.js:69-91`
- Backend: `backend/src/admin/marketplace.rs:321-348`

Problem:

`renderTrades()` builds table rows with template-string `innerHTML` using `asset_name`, `buyer_email`, and `seller_email` from the database. Asset titles are developer-controlled content, and admin pages are high-value targets.

Expected:

Rows should be created with DOM APIs and user/database values assigned via `textContent`, or every interpolated value must be escaped by a shared helper.

Evidence:

The recent-trades API joins `assets.title` and `users.email`, then `mp-index.js` interpolates those values into `<td>` and `<code>` HTML without escaping.

Fix:

Replaced table-string rendering with DOM row construction and `textContent`. The authenticated E2E seeds an asset title containing `<img src=x onerror=alert(1)>` and verifies no image element is created.

### P2 - KPI trend and health indicators can remain stale or misleading after real data loads

Status: fixed 2026-04-26

Location:

- Template: `frontend/platform/admin/marketplace/index.html:42-45`, `frontend/platform/admin/marketplace/index.html:80`, `frontend/platform/admin/marketplace/index.html:92`
- JS: `frontend/platform/static/js/mp-index.js:26-51`, `frontend/platform/static/js/mp-index.js:96-140`

Problem:

The cards update primary KPI values, but the trend labels stay hardcoded (`12% vs yesterday`, `$342K vs 24h ago`) and the topbar health dots are always green. These visible signals can contradict the API response.

Expected:

Either compute these values from backend fields or remove/hide them until real data exists. The topbar health dots should be driven by `/health` or rendered neutrally until loaded.

Evidence:

`renderKPIs()` only updates the four value nodes and one pending color; no code updates the hardcoded trend/subtext labels or `.admin-health-indicators`.

Fix:

Replaced hardcoded trend labels with API-backed or neutral text and wired the topbar health dots to the health API response.

### P3 - Mock pending-approval data uses the wrong response property

Status: fixed 2026-04-26

Location:

- JS: `frontend/platform/static/js/mp-index.js:144-150`

Problem:

`useMockData()` passes `pending_approvals`, but `renderKPIs()` reads `pending_reviews`. If mock fallback runs, the pending approvals KPI renders `0` while the mock intent is `3`.

Expected:

If mock mode remains for local development, mock objects should match the backend response contract.

Evidence:

Static review of `renderKPIs()` and `useMockData()`.

Fix:

Removed production mock data.

---

## Backend Findings

### P1 - Health endpoint masks database failures and hardcodes critical component status

Status: fixed 2026-04-26

Location:

- Backend: `backend/src/admin/marketplace.rs:830-885`

Problem:

`GET /api/admin/marketplace/health` ignores DB check errors with `.ok().flatten()`, ignores last-trade query errors, defaults queue depth to `0` on DB failure with `unwrap_or(0)`, hardcodes `matching_engine_status` to `healthy`, and returns `active_ws_connections: 0` as a TODO. The UI then renders those values as a system health panel.

Expected:

Health should report explicit component errors and degraded states. If DB checks fail, the endpoint should return a failed component or non-2xx operator-safe error rather than healthy-looking data.

Evidence:

Static review shows failed SQL checks are discarded and matching/WebSocket values are not measured.

Fix:

The health handler now propagates DB errors, requires successful queue and last-trade queries, returns `database_connected`, `websocket_status`, and derived matching status, and no longer defaults failed DB checks to healthy values. A timestamp decode bug found by E2E was also fixed by decoding `last_trade_at` as `DateTime<Utc>`.

### P2 - Health API lacks marketplace-specific permission enforcement

Status: fixed 2026-04-26

Location:

- Backend: `backend/src/admin/marketplace.rs:831-833`
- Backend route: `backend/src/admin/mod.rs:715-718`

Problem:

Stats and recent trades require `marketplace.view`, but health accepts any `AdminUser`. Marketplace health data is admin-only operational data and should use the same page/API permission model.

Expected:

`api_admin_marketplace_health` should call `admin.require_permission(&state.db, "marketplace.view").await?` or a documented marketplace operations permission.

Evidence:

The handler parameter is named `_admin` and no `require_permission()` call exists.

Fix:

`GET /api/admin/marketplace/health` now requires `marketplace.view`.

### P2 - Redis trading-status command failures default to `LIVE`

Status: fixed 2026-04-26

Location:

- Backend: `backend/src/admin/marketplace.rs:275-293`

Problem:

When Redis exists but the `GET marketplace:trading_enabled` command fails, `.unwrap_or(None)` converts the error into `None`, and the status becomes `LIVE`. That hides an inability to verify the trading kill-switch.

Expected:

Connection or command failures should produce `UNKNOWN` or degraded status, not `LIVE`.

Evidence:

The connection error branch returns `UNKNOWN`, but the command error path is swallowed before the `match status.as_deref()` branch.

Fix:

Redis `GET marketplace:trading_enabled` errors are now logged and returned as `UNKNOWN`; only an absent key or explicit true value returns `LIVE`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/mp-index.js` | Parses successfully. | Passed with no output. | Pass |
| Python syntax | `python3 -m py_compile tests/e2e/test_admin_marketplace_index.py` | Parses successfully. | Passed with no output. | Pass |
| Rust format | `cargo fmt --manifest-path backend/Cargo.toml --check` | Formatting passes. | Passed. | Pass |
| Backend compile | `cargo check --manifest-path backend/Cargo.toml` | Backend compiles. | Finished successfully. | Pass |
| Authenticated E2E | `BASE_URL=http://127.0.0.1:8888 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_marketplace_index.py -q` | Real stats/trades/health render, no mock fallback, safe HTML-like title rendering, no console/network failures. | `1 passed`. | Pass |

---

## Security Findings

- Fixed: Recent trades now use DOM text nodes for database-backed asset titles and email-derived labels.
- Fixed: `/api/admin/marketplace/health` now requires `marketplace.view`.
- Fixed: API failures render visible error states instead of mock data.
- CSRF is not applicable for this page's reviewed APIs because they are read-only `GET` requests.

---

## Database Findings

- `market_orders` and `trade_history` exist with integer-cent monetary fields and relevant indexes for the overview queries.
- Stats and recent-trades queries use integer-cent arithmetic for monetary values.
- The health endpoint now propagates database query failures instead of reporting healthy-looking defaults.
- No database writes are performed by the page, so transaction/locking review was not applicable.

---

## Missing Tests

- Added `tests/e2e/test_admin_marketplace_index.py` for authenticated overview coverage.
- Add an authorization test for `/admin/marketplace/`, `/api/admin/marketplace/stats`, `/recent-trades`, and `/health` covering admin, marketplace.view, marketplace.manage, and marketplace.compliance roles.
- Add a frontend rendering test for recent trades with an asset title containing HTML-like characters, verifying it renders as text.
- Add backend tests for `/api/admin/marketplace/health` degraded DB/Redis behavior and matching/WebSocket component status once real counters exist.
- Add a mobile/keyboard smoke test for the overview page and marketplace sidebar navigation.

---

## Recommended Fix Order

1. Remove production mock fallback and replace it with visible loading/error states.
2. Fix `/api/admin/marketplace/health` to enforce marketplace permission and report real degraded component status.
3. Replace recent-trade `innerHTML` rendering with DOM-safe text rendering.
4. Align page and API permissions for marketplace overview.
5. Replace hardcoded trend labels and topbar green dots with API-backed or neutral states.

---

## Final Status

`completed`

Reason: The documented findings were fixed and verified with focused syntax, format, compile, and authenticated E2E checks.
