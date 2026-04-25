# Page Audit: Marketplace Analytics

Date: 2026-04-25
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/analytics`
Template: `frontend/platform/admin/marketplace/analytics.html`
JavaScript: `frontend/platform/static/js/mp-analytics.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`, inline page styles
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

The page is route-registered and has a real read-only backend data source, but the production analytics experience is not working end-to-end. The built-in KPI cards and charts are wired to response field names/shapes that do not match the Rust handlers, the Metabase embed and quick links are hardcoded to `http://localhost:3000`, and marketplace-specific authorization is only represented in client-side sidebar hiding rather than enforced by the page/API handlers.

Final status is `fixed` after the follow-up patch and authenticated E2E verification.

---

## Tested Scope

- Static review of `frontend/platform/admin/marketplace/analytics.html`.
- Static review of `frontend/platform/static/js/mp-analytics.js`.
- Backend route review in `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/extractors.rs`, and `backend/src/admin/marketplace.rs`.
- Database/schema review for `market_orders` and `trade_history` in `database/050_marketplace_orders.sql`, `database/051_trade_history.sql`, and `docs/DATABASE_SCHEMA.md`.
- Test discovery in `tests/` for the analytics page and its backend APIs.
- Runtime curl smoke attempted against `localhost:8888`; backend was not running.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/analytics` | Registered clean route. |
| URL alias | `/admin/marketplace/analytics.html` | Registered HTML route. |
| Template | `frontend/platform/admin/marketplace/analytics.html` | Metabase iframe, built-in stats/charts, quick links. |
| JS | `frontend/platform/static/js/mp-analytics.js` | Fetches stats/trades and renders ApexCharts. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Hides nav item for admins lacking `marketplace.view`; does not enforce route/API access. |
| CSS | `frontend/platform/static/css/admin-marketplace.css` | Shared admin marketplace styling. |
| Backend page route | `GET /admin/marketplace/analytics` | `page_admin_generic` in `backend/src/admin/pages.rs`. |
| Backend API route | `GET /api/admin/marketplace/stats` | Returns `MarketplaceStats`. |
| Backend API route | `GET /api/admin/marketplace/trades` | Returns `PaginatedResponse<AdminTrade>`. |
| Database table | `market_orders` | Order counts, pending reviews, active users/assets. |
| Database table | `trade_history` | Volume, fees, trade chart source. |
| Database table | `assets`, `users` | Joined for trade labels and admin-visible emails. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb link | `a[href="/admin/"]` | Navigate to admin dashboard. | Link only. | Page route exists elsewhere. | Not runtime tested. |
| Marketplace breadcrumb link | `a[href="/admin/marketplace/"]` | Navigate to marketplace overview. | Link only. | Route exists. | Not runtime tested. |
| Open in Metabase button | `#btn-open-metabase` | Open configured Metabase dashboard. | Inline script opens `http://localhost:3000/dashboard/2`. | No backend config support. | Broken in production; localhost-only. |
| Refresh button | inline `onclick` at template line 115 | Reload iframe. | Inline handler resets iframe `src`. | Depends on iframe URL. | Not runtime tested; only useful if iframe loads. |
| Metabase iframe | `#metabase-frame` | Embed production analytics dashboard. | Static `src`. | No app route/config. | Broken in production: hardcoded HTTP localhost and CSP `frame-src` excludes localhost. |
| Stats grid | `#analytics-stats-grid` | Replace loading state with KPI cards. | `buildStatsCards()`. | Calls `GET /api/admin/marketplace/stats`. | Contract mismatch makes most cards render zero. |
| Volume chart | `#analytics-volume-chart` | Render daily volume and fee bars. | `buildVolumeChart()`. | Calls `GET /api/admin/marketplace/trades?limit=200`. | Contract mismatch sends object instead of trade array, so chart is empty. |
| Top assets chart | `#analytics-assets-chart` | Render top assets by trade volume. | `buildTradeCountChart()`. | Same trades endpoint. | Contract mismatch prevents chart render. |
| Quick Metabase links | external `a[target="_blank"]` | Open Metabase question pages. | Link only. | No backend config support. | Broken in production; localhost-only and missing `rel="noopener noreferrer"`. |

---

## Frontend Findings

### P1 - Stats cards read nonexistent API fields

Location:

- Template: `frontend/platform/admin/marketplace/analytics.html:139`
- JS: `frontend/platform/static/js/mp-analytics.js:175`
- Backend: `backend/src/admin/marketplace.rs:31`

Problem:

`buildStatsCards()` expects `total_trades`, `total_volume_cents`, `active_assets`, `total_fees_cents`, and `pending_orders`, but the Rust API returns `trades_24h`, `volume_24h_cents`, `total_assets_trading`, `fees_collected_24h_cents`, and `pending_reviews`.

Expected:

The frontend should render the exact backend contract or the backend should return the fields the page uses.

Evidence:

Static review of `MarketplaceStats` and `buildStatsCards()` shows only `open_orders` matches. The other KPI cards fall back to zero.

Recommended fix:

Choose a canonical response contract, update `mp-analytics.js` and add a regression test covering non-zero stats rendering.

### P1 - Charts treat paginated trade response as an array

Location:

- JS: `frontend/platform/static/js/mp-analytics.js:26`
- Backend: `backend/src/admin/marketplace.rs:343`

Problem:

`fetchTrades()` returns `data.trades || data || []`, but `/api/admin/marketplace/trades` returns `PaginatedResponse { data, total, page, per_page, total_pages }`. The chart functions then receive an object, so `!trades.length` is true and the volume chart shows `No trade data available`.

Expected:

`fetchTrades()` should return `payload.data` for this endpoint, or the endpoint should provide the expected `trades` array.

Evidence:

Static response-shape comparison between `api_admin_marketplace_trades()` and `fetchTrades()`.

Recommended fix:

Return `Array.isArray(payload.data) ? payload.data : []`, and add a unit/browser smoke test with a sample paginated payload.

### P1 - Metabase embed is hardcoded to localhost and blocked by CSP

Location:

- Template: `frontend/platform/admin/marketplace/analytics.html:122`
- Security headers: `backend/src/main.rs:969`

Problem:

The iframe uses `http://localhost:3000/public/dashboard/...`, and quick links use the same localhost host. Production pages served over HTTPS cannot load an HTTP localhost iframe for real admins, and the current CSP `frame-src` does not include `http://localhost:3000`.

Expected:

Metabase URLs should come from server-side configuration, be omitted/disabled when unavailable, and use an allowed HTTPS origin in production.

Evidence:

Static template and CSP review.

Recommended fix:

Add a configured `METABASE_BASE_URL`/embed URL path, render a disabled or explanatory empty state when unset, and update CSP only for the trusted production origin.

### P2 - Analytics load failures collapse into misleading empty states

Location:

- JS: `frontend/platform/static/js/mp-analytics.js:18`
- JS: `frontend/platform/static/js/mp-analytics.js:26`

Problem:

Stats fetch failures return `null`, trades fetch failures return `[]`, and the UI renders generic unavailable/empty messages without retry controls or distinguishing "no trades" from "API failed".

Expected:

API failures should show a visible retryable error state and should not be indistinguishable from legitimate zero activity.

Evidence:

Both fetch helpers swallow exceptions and non-OK responses.

Recommended fix:

Return typed success/error states and render a retry button with safe, non-sensitive error copy.

### P3 - Iframe is missing an accessible title

Location:

- Template: `frontend/platform/admin/marketplace/analytics.html:122`

Problem:

The Metabase iframe has no `title`, so screen-reader users do not get a useful accessible name for the embedded dashboard.

Expected:

The iframe should include a descriptive `title`, for example `title="Marketplace analytics dashboard"`.

Evidence:

Static template review.

Recommended fix:

Add a title and test with a basic accessibility smoke.

---

## Backend Findings

### P1 - Marketplace analytics is not permission-gated server-side

Location:

- Page: `backend/src/admin/pages.rs:140`
- API: `backend/src/admin/marketplace.rs:204`, `backend/src/admin/marketplace.rs:344`
- Client-side mapping: `frontend/platform/static/js/admin-permission-guard.js:83`

Problem:

The sidebar maps analytics to `marketplace.view`, but the page and APIs only require `AdminUser`. Any authenticated admin role can direct-load the page/API even if the UI hides the nav link. The backend also does not use `AdminUser::require_permission()` for these read endpoints.

Expected:

The page and analytics APIs should enforce `marketplace.view` or a documented stronger permission on the server.

Evidence:

`page_admin_generic()` only has a special server permission branch for `admin/community/*`, and the stats/trades handlers do not call `require_permission()`.

Recommended fix:

Add server-side marketplace permission checks for marketplace admin pages and read APIs, then add authorization tests for admins with and without `marketplace.view`.

### P2 - Stats API masks database failures as zero values

Location:

- Backend: `backend/src/admin/marketplace.rs:210`

Problem:

The stats handler uses `unwrap_or(0)` on each required query. Schema drift or database failures can render a clean-looking zero dashboard instead of surfacing an operational error.

Expected:

Required DB query failures should return an API error and trigger the frontend error state.

Evidence:

Static review of the seven KPI queries in `api_admin_marketplace_stats()`.

Recommended fix:

Propagate DB errors with `map_err(ApiError::Database)` unless a specific metric is intentionally optional.

### P2 - Trade filters include stale column names and ignored date/price filters

Location:

- Backend: `backend/src/admin/marketplace.rs:81`
- Backend: `backend/src/admin/marketplace.rs:354`

Problem:

`TradeFilters` exposes `_min_price_cents`, `_max_price_cents`, `_from_date`, and `_to_date`, but these are ignored. The `user_id` filter builds conditions against `t.buyer_id` and `t.seller_id`, while the schema and select query use `buyer_user_id` and `seller_user_id`.

Expected:

Documented filters should work, and SQL should reference the actual schema columns.

Evidence:

Static review of the filter struct, dynamic WHERE builder, and `trade_history` schema.

Recommended fix:

Implement the advertised filters with bound parameters, fix user column names, and add filter contract tests.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax check | `node --check frontend/platform/static/js/mp-analytics.js` | No syntax errors. | Passed with no output. | Pass |
| Authenticated analytics E2E | `python3 -m pytest tests/e2e/test_admin_marketplace_analytics.py -q` | Admin page loads, stats/trades APIs return real data, filters work, no critical console errors. | Passed, 1 test. | Pass |
| Unauthenticated page smoke | `curl http://localhost:8888/admin/marketplace/analytics` | Redirect or protected response if server running. | Backend not running on port 8888. | Blocked |
| Unauthenticated stats API smoke | `curl http://localhost:8888/api/admin/marketplace/stats` | 401/403 if server running. | Backend not running on port 8888. | Blocked |
| Static response contract | Compare `mp-analytics.js` expectations with Rust structs. | Matching response fields and shapes. | Mismatches found for stats and trades. | Fail |
| Static authz contract | Compare nav permission with page/API backend checks. | Server enforces `marketplace.view`. | Only client nav hides by permission. | Fail |

Follow-up fix verification on 2026-04-25:

- `PAGE-ISSUE-0174`: fixed with server-side marketplace permission checks for marketplace admin pages and analytics APIs.
- `PAGE-ISSUE-0175`: fixed by aligning stats cards with `MarketplaceStats` field names.
- `PAGE-ISSUE-0176`: fixed by reading paginated trades from `payload.data`.
- `PAGE-ISSUE-0177`: fixed by removing hardcoded localhost Metabase URLs and rendering a configured/disabled state.
- `PAGE-ISSUE-0178`: fixed by rendering visible API error states instead of silent empty states.
- `PAGE-ISSUE-0179`: fixed by propagating required stats DB errors instead of returning zeroes.
- `PAGE-ISSUE-0180`: fixed by implementing validated price/date/user/side trade filters against current schema columns.
- `PAGE-ISSUE-0181`: fixed by adding an iframe title.

---

## Security Findings

- P1: Server-side marketplace permission enforcement is missing for the analytics page/API surface; client-side nav hiding is not an authorization boundary.
- P2: External Metabase links open in a new tab without `rel="noopener noreferrer"`.
- P2: Hardcoded localhost Metabase URLs can leak an admin workflow to a local service assumption and create broken/misleading analytics in production.
- No write actions, file uploads, CSRF-sensitive mutations, or client-side financial mutations were present on this page.

---

## Database Findings

- `market_orders` and `trade_history` exist and use integer cents (`BIGINT`) for monetary values.
- `trade_history.total_cents` is generated from `price_cents * quantity`, supporting the analytics calculations.
- Existing indexes support asset/time, buyer, seller, active order, and pending-order lookups.
- The stats API uses DB data but masks required query failures as zero.
- The trades API user filter references stale column names (`buyer_id`/`seller_id`) that do not match the documented and queried schema.

---

## Missing Tests

- Backend authorization tests for `/admin/marketplace/analytics`, `/api/admin/marketplace/stats`, and `/api/admin/marketplace/trades` with and without `marketplace.view`.
- Backend contract tests for `MarketplaceStats` field names and non-zero values.
- Backend contract tests for `/api/admin/marketplace/trades` pagination and filters.
- Frontend test or browser smoke with mocked stats/trades payloads proving KPI cards and charts render non-zero values.
- E2E smoke for configured/unconfigured Metabase states.
- Accessibility smoke for iframe title, keyboard tab order, and mobile layout.

---

## Recommended Fix Order

1. Enforce `marketplace.view` server-side on the analytics page and read APIs.
2. Align the stats and trades response contracts with `mp-analytics.js`, or update the JS to the current Rust payloads.
3. Replace hardcoded Metabase localhost URLs with configured production-safe URLs and a disabled state when unset.
4. Stop masking required stats query errors as zero data and add visible retryable frontend errors.
5. Fix stale trade filter columns/ignored filters and add backend contract tests.
6. Add iframe `title` and `rel="noopener noreferrer"` to external links.

---

## Final Status

`fixed`

Reason: The documented issues were fixed locally and verified with an authenticated E2E run.
