# Audit: Affiliate Team (Overview / Analytics shell entry)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team.html` (LOC: 323) |
| **Page route** | `GET /developer/affiliate-team` |
| **Handler** | `page_developer_affiliate_team` — `backend/src/developer/routes.rs:323` |
| **Template name** | `developer/affiliate-team.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429), `developer-affiliate-team-analytics.js` (976), `poool-line-chart`, ECharts CDN |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` (3521) |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | Production-Ready |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Default landing for the `/developer/affiliate-team/*` cluster. Shows a hero KPI strip (Revenue, Commission), a 3-stage payout pipeline (Pending → Payable → Paid out), secondary KPIs (Conversion, Qualified, Members), a hero ECharts commission/revenue trend chart with metric toggle, 3-column satellite charts (funnel, top members, top assets), a "Members at risk" list, and two full breakdown tables (by member, by asset). Page is the analytics dashboard for the affiliate-team owner.

## 2. Frontend structure
- Standard developer chrome (sidebar + topbar) using `dev_nav_show_team_actions=true` + `dev_nav_show_date_range=true` (`affiliate-team.html:17`).
- Includes `developer/_affiliate_team_shell.html` (line 22) and `developer/_affiliate_team_invite_modal.html` (line 320).
- All JS is the IIFE in `developer-affiliate-team-analytics.js`; SVG/ECharts charts are rendered into `#dat-chart-trend`, `#dat-chart-funnel`, `#dat-chart-members`, `#dat-chart-assets`.
- Loading skeletons (`<span class="dat-skeleton">`) in every KPI tile.
- A11y: explicit `scope="col"`, `sr-only` `<caption>`, `aria-sort="descending"`, `aria-live="polite"` on KPI values.
- No HTMX — every interactive section is fetched via `DAT.apiGet` (with 15s timeout + retry on idempotent GETs).

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/affiliate/team` | `/api/developer/affiliate/team` | `get_team_info` `rewards/team_routes.rs:398` | Wired |
| `GET /api/developer/affiliate/team/analytics/overview` | same | `analytics_overview` `rewards/team_routes.rs:1898` | Wired |
| `GET /api/developer/affiliate/team/analytics/timeseries` | same | `analytics_timeseries` `rewards/team_routes.rs:2056` | Wired |
| `GET /api/developer/affiliate/team/by-member` | same | `team_by_member` `rewards/team_routes.rs:1434` | Wired |
| `GET /api/developer/affiliate/team/products` | same | `team_products` `rewards/team_routes.rs:1738` | Wired |
| `GET /api/developer/affiliate/team/analytics/cohort?months=12` | same | `analytics_cohort` `rewards/team_routes.rs:2118` | Wired (but the cohort UI was removed from this template; cohort still renders only on `affiliate-team-analytics.html`) |
| `POST /api/developer/affiliate/team/invite` | same | `invite_member` `rewards/team_routes.rs:1251` | Wired (rate-limited per-dev + per-email) |

Auth gate: `require_developer_page` (cookies → `User` extractor → role check `developer/admin/super_admin`) at `routes.rs:213-227`. API handlers use the `DeveloperUser` extractor (`backend/src/developer/extractors.rs`).

Data sources: all SQL-backed against `developer_teams`, `developer_team_memberships`, `affiliate_referrals`, `affiliate_commissions`, `affiliate_links`, `affiliate_live_counters`, `orders`, `investments`, `affiliate_tiers`, `developer_team_tier_history`. No hardcoded values.

## 4. Data realism
Real DB. `analytics_overview` performs full period + previous-period calc; `team_by_member` and `team_products` join across referrals/commissions/orders. Bank IBAN dual-path read (encrypted column preferred, legacy plaintext fallback — `team_routes.rs:421-432`).

## 5. Error & empty states
- `Promise.allSettled` in `loadAll` (`developer-affiliate-team-analytics.js:630`) — each section gets an explicit failure message when its API call rejects (FC3 fix at lines 670-682, 692-705).
- Empty-state hero CTA "Try a wider date range →" for trend chart (`analytics.js:299-307`).
- KPI tiles initialize with `dat-skeleton` placeholder spans (HTML lines 43, 59, 74, etc.).
- Toast notification + sentinel on team-info load failure (`shell.js:1262-1267`).

## 6. Mobile & responsive
No dedicated `mobile-developer-affiliate-team.css`. `developer-affiliate-team.css` has 31 `@media` queries (e.g. `developer-affiliate-team.css:324, 674, 747, 865, 869, 1022, 1198, 1219, 1222, 1249`). Hard table column min-widths force horizontal scroll on small screens (`developer-affiliate-team.css:314-321` — members table reserves up to 220px per column).

## 7. Tests
- `backend/tests/affiliate_team_integration.rs` covers schema invariants + service-layer flows (`invite_by_email`, `approve_pending`, `remove_member`, attribution split, tier-change semantics) — **no HTTP-level tests** for `/api/developer/affiliate/team/*` routes.
- No Playwright/E2E tests covering `/developer/affiliate-team` pages (`tests/e2e/` has nothing matching `affiliate-team`).

## 8. Functional gaps & dead code
- Page is dual-published: `/developer/affiliate-team` and `/developer/affiliate-team/analytics` BOTH serve `developer/affiliate-team.html` (`routes.rs:395`). Comment at routes.rs:392 says "Kept as an alias of the base ... so existing bookmarks survive". OK but worth noting.
- The hero KPI row only has 2 tiles (Revenue, Commission) — analytics.js still tries to populate `dat-k-next-amount` etc., which simply no-op since those IDs don't exist on this template (intentional, see analytics.js:174-193).
- No `TODO`/`FIXME`/`mock`/`Lorem` markers in this file.

## 9. Production blockers
- **Medium**: No HTTP integration tests for the `/api/developer/affiliate/team/analytics/*` family. Test coverage is service-layer only.
- **Medium**: ECharts 5.5.1 loaded from `cdn.jsdelivr.net` (`affiliate-team.html:5`) — external CDN dependency for a core dashboard render. Self-host or pin SRI hash recommended for production.
- **Low**: No dedicated mobile stylesheet — column min-widths cause horizontal scroll on phones (acceptable for an analytics dashboard).
- **Low**: Bank-IBAN legacy plaintext path still active (`team_routes.rs:421-432`); migration to encrypted column is in-progress.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Full KPI grid, 4 charts, 2 tables, deficit list, skeletons. |
| Backend wiring | 2/2 | All 7 endpoints implemented with proper auth + SQL. |
| Data realism | 2/2 | 100% real DB queries, no placeholders. |
| Error/empty states | 1/1 | Per-section `Promise.allSettled` + CTAs + skeletons. |
| Mobile/responsive | 0.5/1 | Has media queries but no dedicated mobile CSS, fixed table widths. |
| Tests | 1/1 | HTTP suite at `backend/tests/developer_affiliate_team_http.rs` covers all 7 analytics endpoints + auth triad. E2E in parametrized `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | Excellent a11y (scope, captions, aria-live, focus traps); EUR formatting; external CDN concern. |
| **TOTAL** | **9.5/10** | Remaining gap: ECharts CDN dependency + mobile table widths. |
