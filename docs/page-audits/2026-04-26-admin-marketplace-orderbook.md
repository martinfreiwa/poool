# Page Audit: Marketplace Orderbook

Date: 2026-04-26
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/orderbook`
Template: `frontend/platform/admin/marketplace/orderbook.html`
JavaScript: `frontend/platform/static/js/mp-orderbook.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/mp-orderbook.css`, `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

`/admin/marketplace/orderbook` has been fixed after the audit. The selector now loads live backend assets with UUID values, mock orderbook fallback was removed, the rebuild button POSTs to the real backend endpoint with CSRF and confirmation, orderbook APIs enforce `marketplace.view`/`marketplace.manage`, and Redis rebuilds are locked and durably audit logged.

Final status is `fixed`; authenticated HTTP/DB/Redis E2E passed.

---

## Tested Scope

- Reviewed `frontend/platform/admin/marketplace/orderbook.html`.
- Reviewed `frontend/platform/static/js/mp-orderbook.js` and shared `mp-toast.js`.
- Reviewed admin route registration in `backend/src/admin/mod.rs`.
- Reviewed marketplace page authorization in `backend/src/admin/pages.rs`.
- Reviewed `/api/admin/marketplace/orderbook/:asset_id` and `/api/admin/marketplace/orderbook/rebuild` in `backend/src/admin/marketplace.rs`.
- Reviewed marketplace order schema in `database/050_marketplace_orders.sql`.
- Checked local PostgreSQL table support for `assets` and `market_orders`.
- Checked for existing tests covering the page/API.
- Ran JavaScript syntax check.
- Tried local curl smokes for page/API/rebuild; runtime was blocked because no backend was listening on `localhost:8888`.
- Follow-up fix verification added and passed: `tests/e2e/test_admin_marketplace_orderbook.py`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/orderbook` | Registered through `page_admin_generic`. |
| URL alias | `/admin/marketplace/orderbook.html` | Registered. |
| Template | `frontend/platform/admin/marketplace/orderbook.html` | Live asset selector shell, rebuild button, status region, bids/asks tables. |
| JS | `frontend/platform/static/js/mp-orderbook.js` | Loads live asset options, orderbook data, and rebuild API; no mock fallback. |
| JS | `frontend/platform/static/js/mp-toast.js` | Displays response-aware rebuild notifications. |
| CSS | `frontend/platform/static/css/mp-orderbook.css` | Spread bar, depth rows, stats styling. |
| Backend page route | `GET /admin/marketplace/orderbook` | Allows marketplace view/manage/compliance permissions. |
| Backend API route | `GET /api/admin/marketplace/orderbook/assets` | Lists live selector assets; requires `marketplace.view`. |
| Backend API route | `GET /api/admin/marketplace/orderbook/:asset_id` | Aggregates active orders by price level; requires `marketplace.view`; returns asset metadata. |
| Backend API route | `POST /api/admin/marketplace/orderbook/rebuild` | Rebuilds Redis orderbook from PostgreSQL; requires `marketplace.manage`; lock/audit protected. |
| Database table | `market_orders` | Active orders queried by `asset_id`, `side`, `status`, `price_cents`. |
| Database table | `assets` | Exists locally, but the page does not load live assets. |
| Redis | Orderbook sorted sets | Rebuild endpoint depends on configured Redis. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `a[href="/admin/"]` | Navigate to admin dashboard. | Link navigation. | Route exists. | Static verified; runtime blocked. |
| Marketplace breadcrumb | `a[href="/admin/marketplace/"]` | Navigate to marketplace index. | Link navigation. | Route exists. | Static verified; runtime blocked. |
| Asset selector | `#asset-selector` | Select a real tradable asset and load its orderbook. | Loads `/assets` API and uses UUID values. | API exists and requires `marketplace.view`. | E2E verified. |
| Rebuild Orderbook button | `#btn-rebuild-orderbook` | POST rebuild request, show real success/error, reload data. | Uses confirmation, CSRF, busy state, and real POST. | Backend POST requires `marketplace.manage`, lock, audit. | E2E verified. |
| Selected asset label | `#selected-asset-name` | Show selected live asset name. | Updated from API `asset_title`. | API returns metadata. | E2E verified. |
| Stats panel | `#ob-stats` | Show mid price and order count from live data. | DOM-rendered. | API returns mid price and counts. | E2E verified via JSON. |
| Spread bar | `#spread-bar` | Show best bid/ask, spread, bid/ask volume. | DOM-rendered with loading/error states. | API returns enough numeric data. | E2E verified via JSON. |
| Bids table | `#bids-body` | Show live aggregated bid levels and order counts. | DOM-rendered. | API returns aggregated counts. | E2E verified. |
| Asks table | `#asks-body` | Show live aggregated ask levels and order counts. | DOM-rendered. | API returns aggregated counts. | E2E verified. |
| Empty states | table fallback rows | Show no bids/no asks when API has no active orders. | DOM-rendered. | API can return empty arrays. | Static verified. |
| Error state | `#orderbook-status` | Show API/permission/Redis failure. | Visible status region. | API returns errors. | Static verified. |
| Loading state | `#orderbook-status`, disabled controls | Show pending network/rebuild state. | Implemented with disabled and `aria-busy`. | N/A. | Static verified. |

---

## Frontend Findings

### P1 - Page renders mock orderbook instead of real assets/API data

Location:

- Template: `frontend/platform/admin/marketplace/orderbook.html:43-49`
- JS: `frontend/platform/static/js/mp-orderbook.js:11-38`, `frontend/platform/static/js/mp-orderbook.js:180-205`
- Backend: `backend/src/admin/marketplace.rs:665-752`

Problem:

The asset selector contains hardcoded demo slug values such as `bali-villa`, while `loadOrderbook()` only calls `/api/admin/marketplace/orderbook/:asset_id` when the selected value matches a UUID regex. Every shipped selector option therefore bypasses the real API and renders mock bid/ask data with fake user IDs and prices.

Expected:

The selector should be populated from real assets/orderbook assets with UUID values, or the backend should accept the same identifiers the frontend sends. Production admin pages should not fall back to demo market data.

Evidence:

Static review shows the API path is only reached for UUID values. Local DB has real UUID asset IDs, but the template options are static slugs. The API handler parses the path with `ApiError::parse_uuid()`.

Recommended fix:

Load tradable assets from a backend API or server-render them into the template as UUID values. Remove production mock fallback, and show a visible empty/error state when no live orderbook data can be loaded.

### P1 - Rebuild button shows false success without calling rebuild API

Location:

- Template: `frontend/platform/admin/marketplace/orderbook.html:52-55`
- JS: `frontend/platform/static/js/mp-orderbook.js:217-223`
- Backend: `backend/src/admin/marketplace.rs:758-782`

Problem:

The `Rebuild Orderbook` button calls `mpButtonAction()` with a success message and then reloads the current mock/live view. It never sends `POST /api/admin/marketplace/orderbook/rebuild`, never includes CSRF headers, and never handles Redis/API failure.

Expected:

Clicking the button should require confirmation, POST to the real endpoint with CSRF protection, disable while pending, show response-aware success/error feedback, and reload live data only after the backend confirms the rebuild.

Evidence:

Static review found no `fetch()` call in the rebuild click handler. The real backend route exists but is unused by the page.

Recommended fix:

Replace the simulated helper with an async `fetch('/api/admin/marketplace/orderbook/rebuild', { method: 'POST', headers: csrfHeaders })` path. Use a confirmation modal because this mutates Redis operational state.

### P2 - API failures and invalid asset values degrade into mock or stale UI

Location:

- JS: `frontend/platform/static/js/mp-orderbook.js:189-205`

Problem:

When the selected value is a UUID but the API fails, the error is only logged to the console and the page falls back to mock data keyed by `currentAsset`. If the selected UUID is not in `MOCK_ORDERBOOKS`, the old table can remain visible because `renderFromMock()` returns early.

Expected:

API errors should clear stale data and render a visible error state with retry. Authorization failures must not be replaced by demo prices.

Evidence:

The `catch` branch only calls `console.warn()`. Afterward, `usingMockData` is set and `renderFromMock()` is called, but unknown assets return without clearing the DOM.

Recommended fix:

Track `loading`, `error`, `empty`, and `loaded` states explicitly. Clear bids/asks/spread/stats before rendering errors, and remove mock mode from production.

### P2 - Dynamic table rendering uses raw HTML patterns

Location:

- JS: `frontend/platform/static/js/mp-orderbook.js:55-63`, `frontend/platform/static/js/mp-orderbook.js:81-87`, `frontend/platform/static/js/mp-orderbook.js:91-135`, `frontend/platform/static/js/mp-orderbook.js:137-177`

Problem:

Most page sections are rendered with template-string `innerHTML`. Today the real API provides numeric values and counts, but the page design and mock data include user identifiers. If asset names, user labels, or richer admin context are later wired through this code, stored values can be interpolated unsafely in an admin session.

Expected:

Use DOM construction and `textContent` for dynamic values, or centralize escaping before HTML insertion. Keep style widths numeric-clamped.

Evidence:

`renderBids()`, `renderAsks()`, `renderSpread()`, and stats rendering assign HTML strings directly.

Recommended fix:

Refactor the renderers to create table rows/cells with DOM APIs. Clamp depth percentages between `0` and `100`.

### P2 - Page promises admin-only user identification but backend returns only aggregated counts

Location:

- Template: `frontend/platform/admin/marketplace/orderbook.html:10`, `frontend/platform/admin/marketplace/orderbook.html:85-87`, `frontend/platform/admin/marketplace/orderbook.html:101-103`
- JS: `frontend/platform/static/js/mp-orderbook.js:100`, `frontend/platform/static/js/mp-orderbook.js:123`
- Backend: `backend/src/admin/marketplace.rs:674-714`

Problem:

The meta description and table headers indicate an admin orderbook view with user identification, and mock data displays `USR-*` labels. The real API groups by price and returns only `order_count`, so real data mode cannot populate the `User` columns.

Expected:

Either the UI should be labeled as aggregated depth with `Orders` columns, or the backend should return a safe admin-only drilldown/details contract for user/order context.

Evidence:

The SQL groups by `price_cents` only and returns `price_cents`, `total_quantity`, and `order_count`.

Recommended fix:

Rename the table columns to `Orders` for aggregated depth, or add a separate permissioned endpoint/modal for orders at a price level.

### P3 - Basic accessibility states are missing for the live data controls

Location:

- Template: `frontend/platform/admin/marketplace/orderbook.html:43-55`, `frontend/platform/admin/marketplace/orderbook.html:66-72`, `frontend/platform/admin/marketplace/orderbook.html:90-107`

Problem:

The asset selector has no visible or programmatic label beyond the surrounding comment. Dynamic stats, spread, and table body updates do not expose `aria-live` status. The rebuild button has no pending `aria-busy` state, and there is no keyboard/focus consideration for the recommended confirmation/error flow.

Expected:

Admin operational controls should have labels, loading/error announcements, and disabled/busy states during async actions.

Evidence:

Static template/JS review found no label, `aria-live`, `aria-busy`, or error region.

Recommended fix:

Add a real label or `aria-label` for the selector, status regions for async data, and response-aware disabled/busy state for rebuild.

---

## Backend Findings

### P1 - Orderbook APIs lack marketplace-specific permission checks

Location:

- Page guard: `backend/src/admin/pages.rs:178-188`
- API routes: `backend/src/admin/mod.rs:704-709`
- Handlers: `backend/src/admin/marketplace.rs:665-782`

Problem:

Both orderbook API handlers accept `AdminUser` but do not call `require_permission()`. This means any active `admin`/`super_admin` session can read orderbook data and trigger Redis rebuild, even though the page is part of the marketplace permission model and similar marketplace APIs now enforce `marketplace.view` or `marketplace.manage`.

Expected:

`GET /api/admin/marketplace/orderbook/:asset_id` should require `marketplace.view` or a stricter operations permission. `POST /api/admin/marketplace/orderbook/rebuild` should require `marketplace.manage` and probably super-admin/ops policy if rebuilds can disrupt matching.

Evidence:

The orderbook handler names the extractor `_admin` and never uses it. The rebuild handler uses `admin.user.id` only for tracing.

Recommended fix:

Add explicit permission checks and authorization regression tests for allowed and forbidden roles.

### P1 - Redis rebuild is not durably audited and has weak operational safeguards

Location:

- Backend: `backend/src/admin/marketplace.rs:758-782`
- Redis rebuild implementation: `backend/src/marketplace/orderbook.rs:582-642`

Problem:

Forcing an orderbook rebuild mutates live Redis operational state by clearing and restoring orderbook keys, but the admin endpoint only emits a tracing warning. There is no `audit_logs` row, no idempotency/concurrency guard at the API layer, and the current page lacks confirmation.

Expected:

Operational state-changing admin actions should be durably audit logged with actor, action, and result. Rebuild should reject concurrent rebuild attempts or expose a safe lock/status.

Evidence:

Static review of the endpoint found no database audit insert or lock. The frontend also does not call the endpoint.

Recommended fix:

Require `marketplace.manage`, add an advisory/Redis lock around rebuild, write an `audit_logs` row after completion/failure, and expose clear response state to the UI.

### P2 - Orderbook API does not validate asset existence or return asset metadata

Location:

- Backend: `backend/src/admin/marketplace.rs:665-752`
- JS: `frontend/platform/static/js/mp-orderbook.js:45-46`

Problem:

The API accepts any valid UUID and returns an empty orderbook if no asset exists for that ID. It also returns only the UUID, forcing the UI to display `Asset <uuid-prefix>` in real API mode.

Expected:

Unknown assets should return `404`, and successful responses should include safe asset metadata needed by the page, such as title, slug, and symbol if available.

Evidence:

The handler parses the UUID, queries `market_orders`, and never checks `assets`.

Recommended fix:

Join or prefetch `assets` for the requested ID. Return `404` for unknown assets and include display metadata in the JSON response.

---

## Follow-up Fix Verification

Implemented on 2026-04-26:

- Added `GET /api/admin/marketplace/orderbook/assets` and changed the selector to load live UUID asset options.
- Removed production mock orderbook fallback from `mp-orderbook.js`.
- Added asset metadata and unknown-asset `404` behavior to `GET /api/admin/marketplace/orderbook/:asset_id`.
- Added `marketplace.view` enforcement to orderbook asset/read APIs.
- Added `marketplace.manage` enforcement, Redis lock, and durable `audit_logs` rows to rebuild.
- Rewired `Rebuild Orderbook` to POST with CSRF, confirmation, busy state, real success/error handling, and reload.
- Replaced dynamic orderbook table/spread/stat rendering with DOM node construction and `textContent`.
- Relabeled aggregated user columns to `Orders` and added selector/status accessibility states.
- Added authenticated E2E coverage in `tests/e2e/test_admin_marketplace_orderbook.py`.

All PAGE-ISSUE-0221 through PAGE-ISSUE-0227 are fixed locally and verified by authenticated HTTP/DB/Redis E2E.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/mp-orderbook.js` | File parses. | Passed with no output. | Pass |
| Local page smoke | `curl http://localhost:8888/admin/marketplace/orderbook` | Page or auth redirect. | Connection failed; backend not running. | Blocked |
| Local orderbook API smoke | `curl http://localhost:8888/api/admin/marketplace/orderbook/<uuid>` | JSON or auth error. | Connection failed; backend not running. | Blocked |
| Local rebuild API smoke | `curl -X POST http://localhost:8888/api/admin/marketplace/orderbook/rebuild` | JSON or auth error. | Connection failed; backend not running. | Blocked |
| DB schema support | `psql` checks for `assets`, `market_orders`, active order columns | Required tables/columns exist. | Local DB: `assets=18`, `market_orders=0`; required order columns exist. | Pass |
| Test coverage search | `rg "mp-orderbook|admin_marketplace_orderbook|marketplace/orderbook|orderbook/rebuild" tests backend/src` | Existing tests cover page and APIs. | Only approval E2E references orderbook visibility; no dedicated orderbook/rebuild UI/auth tests found. | Gap |
| Authenticated orderbook E2E | `BASE_URL=http://localhost:8888 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_marketplace_orderbook.py -q` | Seed admin/session/asset/orders, load page with CSRF, verify live assets/orderbook, rebuild Redis, verify audit row. | Passed: `1 passed in 0.23s`. | Pass |

---

## Security, Data Integrity, And Compliance Notes

- Orderbook prices are integer cents in the backend (`price_cents BIGINT`) and quantities are integer token counts.
- The page does not perform financial mutations directly, but Redis rebuild mutates operational trading state and needs stronger authorization, audit logging, CSRF-aware frontend behavior, and concurrency protection.
- Mock orderbook data in production is an operational risk because admins can make trading decisions from fake depth.
- No secrets were found in the audited page files.

---

## Recommended Fix Order

1. Remove production mock fallback and wire the asset selector to real UUID asset data.
2. Wire `Rebuild Orderbook` to the real POST endpoint with confirmation, CSRF, visible pending/error states, and no fake success.
3. Add backend `marketplace.view` and `marketplace.manage` checks to orderbook read/rebuild APIs.
4. Add durable audit logging and rebuild locking for Redis rebuilds.
5. Decide whether the page is aggregated depth or user-identifiable order depth, then align backend response and table labels.
6. Refactor dynamic rendering away from raw `innerHTML` patterns and add accessible loading/error states.
