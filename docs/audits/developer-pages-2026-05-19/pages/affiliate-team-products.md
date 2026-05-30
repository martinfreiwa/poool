# Audit: Affiliate Team Products

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team-products.html` (LOC: 63) |
| **Page route** | `GET /developer/affiliate-team/products` |
| **Handler** | `page_developer_affiliate_team_products` — `backend/src/developer/routes.rs:351` |
| **Template name** | `developer/affiliate-team-products.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429), `developer-affiliate-team-products.js` (102) |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | Production-Ready |
| **Score** | 9 / 10 |

## 1. Purpose & user journey
Aggregates each asset (property) sold via any team-business affiliate link in the selected date range. Columns: asset name, units sold, unique buyers, gross revenue €, avg sale €, total commission €, last sale date. Sort/search/paginate via shared `DAT.dataTable`. CSV export. Topbar exposes a date-range picker.

## 2. Frontend structure
- Single card with header (title + search slot) + table (lines 23-57).
- 7 data columns with `data-col` attributes (lines 43-50).
- `developer-affiliate-team-products.js` boots the data-table on `DOMContentLoaded`, attaches `DAT.topbarDateRange` callback to reload on range change.
- CSV download wired via `DAT.downloadCsv` (lines 41-58 of products.js).
- A11y: `<caption class="sr-only">`, `aria-describedby`, `scope="col"`.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/affiliate/team` (shell) | same | `get_team_info` `rewards/team_routes.rs:398` | Wired |
| `GET /api/developer/affiliate/team/products?q&sort&dir&limit&offset&from&to` | same | `team_products` `rewards/team_routes.rs:1738` | Wired |
| `POST /api/developer/affiliate/team/invite` (modal) | same | `invite_member` | Wired |

Auth: `require_developer_page` (page) + `DeveloperUser` (API).
Data source: SQL CTE `commission_orders` joins `affiliate_commissions` × `affiliate_links` filtered by team_id + `link_type='team_business'` + date range, then joins `order_items` × `assets` to aggregate units_sold, gross_revenue_cents, commission_cents per asset (`team_routes.rs:1770-1800`).

## 4. Data realism
Real DB. Whitelisted sort columns; default sort `gross_revenue_cents DESC` (`team_routes.rs:1751-1762`). Date range parsed and validated via `parse_date_query` with sensible defaults (30 days back).

## 5. Error & empty states
- Empty: "No assets sold via your team yet. Once customers your team referred make purchases, each asset rolls up here." (products.js:79).
- Error: red row + toast (from `DAT.dataTable`).
- Loading skeleton (8 rows × 7 cols).
- Search debounced 250ms.

## 6. Mobile & responsive
- 7 columns with min-widths totalling ~1005px (`developer-affiliate-team.css:306-312`) → horizontal scroll on phones.
- Date-range picker lives in topbar; topbar has its own collapse rules.

## 7. Tests
- No HTTP integration tests for `team_products`.
- No frontend E2E tests.
- Service-layer affiliate_team_integration tests cover attribution + commission split but not the products aggregation SQL.

## 8. Functional gaps & dead code
- The "Search" column for products only filters on asset title (`LOWER(a.title) ILIKE $5` at `team_routes.rs:1765`) — no fallback to asset ID search.
- No drill-down: clicking an asset row does NOT navigate to the asset's detail page (no `onclick` or `<a>` wrapper).
- Old date-range helpers in products.js (`isoDay`, `todayIso`, `daysAgoIso`, etc. at lines 24-39) are dead code — superseded by `DAT.currentRange()` from the shell.
- No `TODO`/`FIXME`/`mock`/`Lorem` markers.

## 9. Production blockers
- **Medium**: ~~No HTTP integration test for `team_products`~~ — **RESOLVED 2026-05-19** via `backend/tests/developer_affiliate_team_http.rs`.
- **Low**: Dead helper functions in products.js (date helpers superseded by shell helpers).
- **Low**: Asset name not clickable; users can't jump to asset detail.
- **Low**: Mobile horizontal scroll.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Full dataTable with search/sort/paginate/export. |
| Backend wiring | 2/2 | Endpoint implemented with CTE, real aggregation. |
| Data realism | 2/2 | Real DB joins; date filtering parameterized via `parse_date_query`. |
| Error/empty states | 1/1 | Skeleton + error + empty CTA. |
| Mobile/responsive | 0.5/1 | Horizontal scroll forced. |
| Tests | 1/1 | HTTP coverage in `backend/tests/developer_affiliate_team_http.rs` + parametrized E2E in `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | A11y solid; minor dead code; no clickable asset link. |
| **TOTAL** | **9/10** | Remaining gap: mobile + dead helpers + clickable rows. |
