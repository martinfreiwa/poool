# Audit: Developer Dashboard

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/dashboard.html` (LOC: 334) |
| **Page route** | `GET /developer/dashboard` |
| **Handler** | `page_developer_dashboard` — `backend/src/developer/routes.rs:244` |
| **Template name** | `developer/dashboard.html` |
| **Linked JS** | `developer-dashboard.js` (170), `profile-dropdown.js`, `mobile-navigation.js`, `metric-card-animations.js`, `poool-line-chart.js`, `developer-sales-chart.js` (195) — plus external `gsap.min.js` and `echarts@5.5.1` |
| **Linked CSS** | `developer-dashboard.css` (1888), `unified-styles.css`, `unified-cards.css`, `mobile-developer-dashboard.css` (480), `developer-leaderboard-navbar.css`, `developer-onboarding.css`, `cards-template.css` |
| **Mobile CSS** | `mobile-developer-dashboard.css` (480) |
| **Status** | Production-Ready |
| **Score** | 9 / 10 |

## 1. Purpose & user journey
Lands here after `/developer` redirect (or sign-in for users with the `developer` / `asset_owner` / `admin` / `super_admin` role). Displays priority funding metrics, a sales-over-time ECharts line chart, an "Activity snapshot" tile grid (customizable via localStorage), and the developer's top-performing assets table. Primary jumping-off point to `/developer/assets`, `/developer/asset-detail?id=…`, and `/marketplace/property/:id`.

## 2. Frontend structure
- Sections: review banner (localStorage-gated, `dashboard.html:47`); priority-metrics grid (`:65`); sales chart card (included via `components/developer-chart.html`, `:104`); Activity snapshot tile grid with customize popover (`:108`); Top Performing Assets card (`:249`) using `components/developer-assets.html`.
- HTMX endpoints used directly: `GET /developer/dashboard/fragments/chart?period=…` and `GET /developer/dashboard/fragments/assets?period=…` — both fired programmatically by `window.setDashboardPeriod()` (`dashboard.html:283-319`) via `window.htmx.ajax`.
- Vanilla JS modules:
  - `developer-dashboard.js` — table sort by `data-dev-sort`, metric-card counter easing animations.
  - `developer-sales-chart.js` — DOM-recovers data from the server-rendered SVG and replaces it with an ECharts smooth-area line; re-mounts on `htmx:afterSwap` + MutationObserver.
  - `metric-card-animations.js`, `poool-line-chart.js`, `gsap.min.js` (CDN), `echarts@5.5.1` (CDN), `profile-dropdown.js`, `mobile-navigation.js` — UI primitives.
- Inline `<script>` blocks: 2 (the Activity-snapshot customize/persist IIFE at `:171-247` and the period-tab dispatcher + review-banner gating IIFE at `:278-331`).
- Shared components: `components/head.html`, `components/mobile-menu.html`, `components/sidebar.html`, `components/developer-topbar.html`, `components/developer-chart.html`, `components/developer-assets.html`.
- Anti-pattern: `dashboard.html:283-319` defines `setDashboardPeriod()` and the topbar partial supports `dev_nav_show_period_tabs`, but the dashboard's `{% with %}` (`:40-42`) never passes that flag. The unified period selector therefore **never renders** and the function is dead. The trigger panel comment at `:264-266` claims they "moved to topbar" — they're missing from prod.

## 3. Backend wiring

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /developer/dashboard/fragments/chart?period=…` (`setDashboardPeriod`) | `/developer/dashboard/fragments/chart` | `fragments::fragment_chart` (`fragments.rs:18`) | wired |
| `GET /developer/dashboard/fragments/assets?period=…` (`setDashboardPeriod`) | `/developer/dashboard/fragments/assets` | `fragments::fragment_assets` (`fragments.rs:73`) | wired |
| `GET /api/developer/dashboard/stats` (NOT called from this page) | `/api/developer/dashboard/stats` | `api_developer_dashboard_stats` (`routes.rs:671`) | wired but page is server-rendered, so JSON endpoint is unused here |
| `<a href="/developer/asset-detail?id=…">` (table) | `/developer/asset-detail` | `page_developer_asset_detail` | wired |
| `<a href="/marketplace/property/{{ asset.id }}">` (table) | `/marketplace/property/:id` | marketplace module | wired |

For each WIRED endpoint:
- **`fragment_chart` / `fragment_assets`** — Auth gate is hand-rolled: `middleware::get_current_user` + role-check query on `user_roles JOIN roles` (`fragments.rs:33-45` and `:88-100`). Unauthenticated returns 401 HTML; non-developer returns `Redirect::to("/marketplace")`. Both call `service::fetch_dashboard_stats_for_period` / `fetch_assets_for_period` — real `sqlx` against `assets/investments/asset_views/cart_items/order_items`.
- **`page_developer_dashboard`** — Gated by `require_developer_page` (`routes.rs:213`) which calls `user_has_developer_access`. Renders `developer/dashboard.html` with `stats`, `developer_assets`, `active_period`. Stats come from `service::fetch_dashboard_stats` → `fetch_dashboard_stats_for_period(pool, id, "all")` — real DB queries (`service.rs:55-411`).
- **`api_developer_dashboard_stats`** — Gated by `require_developer_api`; returns `Json(stats)`. Real DB. Currently unused by the page (page is server-rendered) but kept as a public API.

## 4. Data realism
- Real DB: **yes**. All 13 metric values, chart line, top-assets table are real `sqlx` queries (`service.rs:55-410`, `:751-770`, `:773-923`). Auth banner toggles only via localStorage; no backend signal.
- Resolved 2026-05-19 (H-10): the fake "Saved Properties" metric tile and the fake "Saved" column in the assets table were removed from backend (`service.rs`, `models.rs`) and frontend (`developer-assets.html`, `developer-dashboard.css`). No saves table exists in the schema and there is no investor-side "save" flow in the marketplace, so removing the dead UI was the honest fix rather than inventing a table.
- Resolved 2026-05-19 (H-11): `fetch_attention_assets` was an orphan query whose result was never rendered. Removed the function definition, the call from `fetch_dashboard_stats_for_period`, and the `attention_assets` field from `DeveloperDashboardStats`. Saves one DB round-trip per dashboard page load.
- Hardcoded values (remaining):
  - `service.rs:891` — Fallback cover image `/static/images/seed/villa1.webp` for assets with no images.
  - `service.rs:334-368` — Several metric `change_pct` arguments are hardcoded `0.0` ("Total Assets", "Funding Target", "Amount Remaining", "Total Views", "Checkout Starts", "Add to Cart", "Avg. Conversion Rate", "Sold Out Ratio", "Avg. Funding Progress"). Only `sales_pct`, `inv_pct`, `avg_pct` are computed for real → most tiles render "No change yet".
- Placeholder text in DOM: `annual-data.html:30` etc. is on a different page; this page has no "Coming soon" / "Lorem".

## 5. Error & empty states
- 4xx/5xx: Page is server-rendered; the only client-side fetches are inside `setDashboardPeriod` (`:296-318`) — `window.htmx.ajax` calls are fired without `.then(…)` error handlers (chart has a `.then` for ECharts mount only). HTMX swap failures will leave the user with stale UI and no inline feedback.
- Empty-list UI: `components/developer-assets.html:96-103` has a proper `table__row--empty` block with "No live asset performance yet." copy.
- Skeleton/loading: Metric-card counter animation provides visual movement on first load via `developer-dashboard.js:115-168`. No HTMX `hx-indicator` or skeleton during fragment swaps.

## 6. Mobile & responsive
- Dedicated `mobile-developer-dashboard.css` is loaded (`:2`) and contains 3 media queries.
- Main `developer-dashboard.css` has 4 inline media queries.
- Inline `style="grid-column: 1 / -1;"` etc. in section markup is fine. No hard-coded `width: <px>` that would break <768px were noticed in the HTML itself (the heavy widths live in the `.bak` file, not this page).

## 7. Tests
- Rust integration: no test in `backend/tests/*.rs` references `page_developer_dashboard`, `fetch_dashboard_stats`, `fragment_chart`, or `fragment_assets`.
- Python integration (`tests/test_developer_dashboard.py`): yes — covers `GET /developer/dashboard` (auth + content, `:111`, `:119`), `GET /api/developer/dashboard/stats` (`:171`), `GET /developer/dashboard/fragments/{chart,assets}` (`:183`, `:190`).
- `tests/test_platform.py:1283` — smoke-tests `("/developer/dashboard", "Developer Dashboard")`.
- No E2E test in `backend/tests/e2e/` (the directory contains only a `reports/` subdir).

## 8. Functional gaps & dead code
- **Dead code:** `window.setDashboardPeriod` (`:283-319`) is unreachable — its target buttons (`#dev-period-tabs .dev-period-tab`) only render when `dev_nav_show_period_tabs` is set, and the dashboard never sets it. Comment at `:264-266` references the tabs as if they exist.
- **Resolved 2026-05-19 (H-11):** `service::fetch_attention_assets` was orphan-queried every page load; the function and the orphan call have been removed.
- **Resolved 2026-05-19 (H-10):** "Saved Properties" metric tile and "Saved" table column (hardcoded `0`) have been removed from backend and frontend.
- `href="#"` placeholders: none in this file.
- Commented-out blocks: only descriptive Jinja `{# … #}` comments.
- TODO/FIXME/XXX: none in HTML or linked JS.

## 9. Production blockers (severity)
- **Critical:** none. The page renders and shows real money + real assets.
- **High:**
  - Period tabs are missing from the topbar despite `setDashboardPeriod()` existing — users cannot change the period. Either set `dev_nav_show_period_tabs=True` in the `{% with %}` block (`dashboard.html:40-42`) or delete the dead JS.
  - HTMX swaps in `setDashboardPeriod()` have no error handler / no inline status — a failed swap silently leaves stale UI.
- **Medium:**
  - ~~"Saved Properties" metric and "Saved" column always 0~~ — **Resolved 2026-05-19 (H-10):** removed from backend + frontend since no saves table exists in schema.
  - 9 of 13 metric tiles never compute a real `change_pct` ("No change yet" forever).
  - ~~`fetch_attention_assets` is queried but never rendered~~ — **Resolved 2026-05-19 (H-11):** function and orphan call removed.
- **Low:**
  - Inline `<script>` IIFEs at `:171-247` and `:278-331` would be cleaner extracted to `developer-dashboard.js`.
  - Auth banner is purely client-state (`localStorage`) — refreshing while signed-out on another tab won't dismiss; a stale signal can linger across sessions.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | All sections present; dead `setDashboardPeriod` controls drop half a point. |
| Backend wiring | 2/2 | Page handler + fragments are real and gated; orphan `fetch_attention_assets` query removed 2026-05-19. |
| Data realism | 2/2 | Real DB for sales/funding/investors; hardcoded saved-count fields removed 2026-05-19. Remaining `change_pct=0` defaults are honest (no fake values, just "No change yet" labels). |
| Error/empty states | 0.5/1 | Good empty state in assets table; HTMX failures are silent. |
| Mobile/responsive | 1/1 | Dedicated `mobile-developer-dashboard.css` + media queries inside the main file. |
| Tests | 1/1 | Python `test_developer_dashboard.py` covers page + API + fragments. |
| Polish (a11y, i18n, perf) | 0.5/1 | ARIA on tablist/popover is solid; no i18n; loads GSAP + ECharts from CDN (no SRI / no preconnect). |
| **TOTAL** | **9/10** |  |
