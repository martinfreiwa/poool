# Audit: Affiliate Team Analytics (stale alias)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team-analytics.html` (LOC: 388) |
| **Page route** | `GET /developer/affiliate-team/analytics` |
| **Handler** | `page_developer_affiliate_team_analytics` — `backend/src/developer/routes.rs:385` |
| **Template name** | **`developer/affiliate-team.html` (NOT this file)** — the handler explicitly serves the canonical template; this HTML is dead code |
| **Linked JS** | (file declares) `developer-affiliate-team-shell`, `poool-line-chart`, `developer-affiliate-team-analytics` + ECharts CDN — but the file is never rendered |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | **DELETED 2026-05-19** (was: Stale orphan) |
| **Score** | N/A — file removed |
| **Resolved** | 2026-05-19 — `rm frontend/platform/developer/affiliate-team-analytics.html`. The route `/developer/affiliate-team/analytics` is kept as a legacy alias; its handler at `routes.rs:385-397` was already serving the canonical `developer/affiliate-team.html`, so URL bookmarks survive. |

## 1. Purpose & user journey
Was the previous "Analytics" sub-page (legacy dashboard) with an in-page date-preset toolbar, a 6-tile KPI grid (Conversion, Qualified, Pending, Payable, Paid out, Members), a hero trend chart with day/week/month resolution toggle + dual-axis legend, satellite charts, "Top performers" + "Members at risk" insight cards, a cohort retention heatmap, and full breakdown tables. Now superseded — the route serves the redesigned `developer/affiliate-team.html` instead, per the comment at `routes.rs:392-394`.

## 2. Frontend structure
- Standard sidebar/topbar/mobile-menu chrome.
- Same shell + invite-modal includes as the other sub-pages.
- In-page date toolbar with presets (7d / 30d / This month / Last month / YTD / All time / Custom) at lines 26-42 — these presets are unique to this file; the canonical template uses the topbar date picker instead.
- 3 hero KPIs (Revenue, Commission, Next payout), 6 secondary KPIs, trend chart with `data-res="day|week|month"` toggle, funnel, top-members hbar, top-assets hbar.
- Insights grid: Top performers + Members at risk.
- Cohort retention card (`#dat-cohort-wrap`).
- 2 large breakdown tables (by member, by asset).
- Note: This template's `<thead>` lacks `scope="col"` (lines 336-343) — accessibility regression vs the canonical template.

## 3. Backend wiring
The handler at `routes.rs:385-397` calls `serve_protected(jar, &state, "developer/affiliate-team.html")` — this file is never instantiated.

If it were rendered, it would call the same set of endpoints as the canonical analytics page:
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `/api/developer/affiliate/team` | same | `get_team_info` | Wired |
| `/api/developer/affiliate/team/analytics/overview` | same | `analytics_overview` | Wired |
| `/api/developer/affiliate/team/analytics/timeseries` | same | `analytics_timeseries` | Wired |
| `/api/developer/affiliate/team/analytics/cohort?months=12` | same | `analytics_cohort` | Wired |
| `/api/developer/affiliate/team/by-member` | same | `team_by_member` | Wired |
| `/api/developer/affiliate/team/products` | same | `team_products` | Wired |

Auth: `require_developer_page` (page handler) + `DeveloperUser` extractor (API handlers).

## 4. Data realism
Real data IF rendered; but template is unused. Cohort retention + forecast endpoints are real (`rewards/team_reports.rs:488, 556`).

## 5. Error & empty states
- Skeletons in each KPI tile.
- "Loading…" empty rows in tables.
- Same per-section failure handling via `developer-affiliate-team-analytics.js` (if it ever rendered).
- Cohort wrapper has explicit error state at `analytics.js:973`.

## 6. Mobile & responsive
Same as canonical — no dedicated mobile CSS, relies on `developer-affiliate-team.css` media queries.

## 7. Tests
None — orphan template gets no coverage.

## 8. Functional gaps & dead code
- **Entire file is dead code.** Handler at `routes.rs:395` deliberately serves `developer/affiliate-team.html` instead. Comment confirms: "Serve the same canonical template ... so the two routes never drift. Previously this had a separate (stale) copy that broke when the main template was reorganized." → This file is exactly the stale copy that broke.
- Missing `scope="col"` on table headers (lines 336-343) vs the canonical template that has them (`affiliate-team.html:269-275`).
- Custom preset bar with `dat-preset` buttons (lines 27-34) duplicates topbar date-range UI — confusing if file is ever revived.
- "Next payout" hero tile (lines 83-98) was moved to settings page in canonical version.

## 9. Production blockers
- **High**: Orphan file should be deleted or renamed (e.g. `.bak`) to prevent future developers from editing it thinking it's live.
- **Low**: If file is kept for any reason, add `scope="col"` + `<caption>` for a11y parity.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | N/A | File deleted 2026-05-19. |
| Backend wiring | N/A | File deleted 2026-05-19 (route alias kept). |
| Data realism | N/A | File deleted 2026-05-19. |
| Error/empty states | N/A | File deleted 2026-05-19. |
| Mobile/responsive | N/A | File deleted 2026-05-19. |
| Tests | N/A | File deleted 2026-05-19. |
| Polish (a11y, i18n, perf) | N/A | File deleted 2026-05-19. |
| **TOTAL** | **N/A** | File removed; canonical `affiliate-team.html` (scored 9/10 separately) handles the route. |

**Adjusted to 4/10** to reflect that the file is dead code in production. Real production-readiness of the `/analytics` route is 9/10 because it serves the canonical template — see `affiliate-team.md`.
