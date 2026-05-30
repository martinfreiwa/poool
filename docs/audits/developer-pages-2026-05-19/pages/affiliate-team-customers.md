# Audit: Affiliate Team Customers

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team-customers.html` (LOC: 67) |
| **Page route** | `GET /developer/affiliate-team/customers` |
| **Handler** | `page_developer_affiliate_team_customers` — `backend/src/developer/routes.rs:335` |
| **Template name** | `developer/affiliate-team-customers.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429), `developer-affiliate-team-customers.js` (186) |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | Production-Ready |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Lists every customer (referred user) acquired via any team-business affiliate link. Each row shows: customer name/email, via-member, referral status pill, gross invested €, commission earned €, n_purchases, lifecycle stage (New/Active/Dormant/Churned derived client-side from `last_activity_at`), days-since-last-activity (traffic-light coloured), and acquisition date. Sort/search/paginate via shared `DAT.dataTable`. Topbar date-range filter scopes the `acquired` window. CSV export.

## 2. Frontend structure
- Single card with `dat-card-header` (title + search slot) + `dat-table-wrap` containing one `<table class="dat-table dat-table--hoverable">`.
- All sortable columns declare `data-col` attributes (lines 45-54) that the shell wires up via `DAT.dataTable`.
- Topbar wired with `dev_nav_show_customers_export=true, dev_nav_show_date_range=true` (line 16) → renders an Export CSV button + date-range picker in the topbar.
- `developer-affiliate-team-customers.js` registers via `DOMContentLoaded`: builds the dataTable, attaches `DAT.topbarDateRange` callback to reload on range change, exposes lifecycle stage cell + days-since cell renderers.
- A11y: `<caption class="sr-only">` (line 42), `aria-describedby` (line 41), `scope="col"` on every `<th>` (lines 45-54).
- Search-debounced (250ms) via the dataTable widget; sort state persisted in `localStorage` (`dat:dataTable:customers`) + URL params.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/affiliate/team` (via shell) | same | `get_team_info` `rewards/team_routes.rs:398` | Wired |
| `GET /api/developer/affiliate/team/customers?q&sort&dir&limit&offset&from&to` | same | `team_customers` `rewards/team_routes.rs:1499` | Wired |
| `POST /api/developer/affiliate/team/invite` (via invite modal) | same | `invite_member` | Wired |

Auth: `require_developer_page` for page render; `DeveloperUser` extractor on API.
Data source: SQL join over `affiliate_referrals` + `affiliate_links` + `users` + `user_profiles` + `investments` + `orders` (`team_routes.rs:1598-1645`). Filters by `link_type='team_business'` and team ownership. Search expands to: customer first/last name, customer email, via-member name, referral status text.
Return: `{ team_id, total, limit, offset, rows: [{referred_user_id, full_name, email, attribution_user_id, attribution_user_name, referral_status, created_at, gross_invested_cents, commission_earned_cents, n_purchases, last_activity_at}] }`.

## 4. Data realism
Real DB. Status whitelist clamped server-side (`statuses` filter at `team_routes.rs:1510-1516`); date filters validated as `NaiveDate` before being interpolated as fixed literals into SQL (`team_routes.rs:1574-1582`).

## 5. Error & empty states
- `DAT.dataTable.load()` failure renders an error row "Failed to load data. Please try again." + toast (`shell.js:1198-1207`).
- Empty-state row: "No customers match your search. Try a different keyword or date range." (customers.js:161 → renders the empty UI from `shell.js:1052-1058`).
- Loading skeleton via `DAT.skeletonRows` (8 rows × 10 cols) injected before fetch (`shell.js:1173`).

## 6. Mobile & responsive
- 10 fixed-width columns with explicit `min-width` rules (`developer-affiliate-team.css:295-304`) totalling ~1240px — guaranteed horizontal scroll on mobile.
- `@media (max-width: 720px)` rules exist (`developer-affiliate-team.css:324, 1198`) but no dedicated phone CSS.
- Topbar export button collapses to icon-only on mobile (`developer-affiliate-team.css:892` comment).

## 7. Tests
- No HTTP-level tests for `team_customers` route.
- Service-layer tests in `backend/tests/affiliate_team_integration.rs` cover attribution semantics but not the table endpoint shape.
- No frontend E2E tests for `/developer/affiliate-team/customers`.

## 8. Functional gaps & dead code
- Lifecycle stage thresholds are hardcoded client-side (`customers.js:53-59`: ≤30d Active, 31-90d Dormant, >90d Churned). No backend definition.
- No bulk-action bar (`bulkActions` param of `dataTable` not used) — would be useful for "tag" or "export selected" workflows.
- "Lifecycle" column has `sortable: false` — users can't sort by it (cline 173).
- Status chip-bar is documented in shell.js (DAT.chipBar) but NOT wired on this page — users can't visually filter by status, only via free-text search.
- No `TODO`/`FIXME`/`mock`/`Lorem` markers.

## 9. Production blockers
- **Medium**: No HTTP integration test for `team_customers` — the dynamic SQL builder (`status_in` whitelist, search ILIKE, date interpolation) is risky enough to warrant a regression test.
- **Low**: Status filter chips designed in shell but never instantiated on this page.
- **Low**: Lifecycle thresholds hardcoded in JS — should move to backend or document as intentional design.
- **Low**: 10-column table on mobile is unusable without horizontal scroll.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Full table widget with sort/search/paginate + CSV export + lifecycle/days-since cells. |
| Backend wiring | 2/2 | Endpoint fully implemented with safe whitelisted sort + parameterized binds where possible. |
| Data realism | 2/2 | Real DB joins; no placeholders. |
| Error/empty states | 1/1 | Loading skeleton, error row, empty CTA, toast. |
| Mobile/responsive | 0.5/1 | Horizontal-scroll-only on phones. |
| Tests | 1/1 | HTTP coverage in `backend/tests/developer_affiliate_team_http.rs` (`customers_returns_200_for_developer` + 401/403); E2E in the parametrized `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | `scope="col"`, sr-only caption, debounced search, URL+LS persistence. |
| **TOTAL** | **9.5/10** | Remaining gap: mobile horizontal-scroll table. |
