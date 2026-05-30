# Audit: Affiliate Team Tier

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/affiliate-team-tier.html` (LOC: 133) |
| **Page route** | `GET /developer/affiliate-team/tier` |
| **Handler** | `page_developer_affiliate_team_tier` — `backend/src/developer/routes.rs:418` |
| **Template name** | `developer/affiliate-team-tier.html` |
| **Linked JS** | `developer-affiliate-team-shell.js` (1429), `developer-affiliate-team-tier.js` (129) |
| **Linked CSS** | `developer-dashboard`, `unified-styles`, `unified-cards`, `developer-leaderboard-navbar`, `developer-affiliate-team` |
| **Mobile CSS** | `mobile-developer-dashboard` only — **no dedicated `mobile-developer-affiliate-team.css`** |
| **Included by** | n/a |
| **Status** | Production-Ready |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Three cards:
1. **Team Tier (hero)** — current tier name, commission rate (% + bps), 12-month team volume, next tier + remaining to threshold, progress bar.
2. **Tier ladder** — full ladder table (Tier / Commission rate / Min 12-month volume / Status: Current/Reached/Locked).
3. **Personal vs Team rate** — side-by-side comparison of the developer's personal-link tier (e.g. when they refer a customer themselves) vs their team tier (when a team member refers).

A small "Updated {timestamp}" pill in the hero header. Tier-change history HTML block is removed but JS still handles it defensively (no-op `if (!tbody) return;` at tier.js:89).

## 2. Frontend structure
- Standard chrome (no topbar actions, no date range).
- Pure data display, no forms.
- Progress bar uses `width: ${pct}%` inline style on `#dat-tier-progress-fill`.
- A11y: `aria-live="polite"` on key values, `<caption class="sr-only">` on ladder table, `aria-label` on progress bar.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/affiliate/team` (shell) | same | `get_team_info` `rewards/team_routes.rs:398` | Wired |
| `GET /api/developer/affiliate/team/tier` | same | `team_tier_info` `rewards/team_routes.rs:1924` | Wired |
| `POST /api/developer/affiliate/team/invite` (modal) | same | `invite_member` | Wired |

Auth: `require_developer_page` (page) + `DeveloperUser` (API).
Data source: `developer_teams.current_team_tier / team_commission_rate_bps / team_volume_12m_cents / team_tier_updated_at`, `affiliates.current_tier / commission_rate_bps` for the developer's personal tier, `affiliate_tiers` for the ladder, `developer_team_tier_history` for last 5 promotions.
Smart caching: `team_tier_info` debounces `recompute_team_tier()` to once per 5 minutes (`team_routes.rs:1936-1948`) — avoids row-lock per browser refresh. Background worker keeps it fresh.

## 4. Data realism
Real DB. Ladder is the canonical `affiliate_tiers` table. `recompute_team_tier` is a Postgres function — invoked on stale reads. Progress calc is server-side via comparing `team_volume_12m_cents` against the next ladder threshold (`team_routes.rs:1979-1996`).

## 5. Error & empty states
- "Loading…" placeholder rows in the ladder table (line 97).
- "Updating…" pill (line 30) replaced once data arrives.
- Toast on fetch failure: "Failed to load tier information. Please refresh." (tier.js:123).
- "Max tier ✓" / "You are at the top of the ladder." for Sovereign tier (tier.js:34-36).
- Personal tier shows "—" if developer has no `affiliates` row (defensive — backend returns null cleanly).
- Tier-change history section was removed from HTML, JS still defensively no-ops (tier.js:89).

## 6. Mobile & responsive
- Hero grid (`dat-tier-hero__grid`) collapses via `@media (max-width: 720px)` rules.
- Ladder table has no fixed column min-widths in this CSS slice — should flow OK.
- Compare cards stack via responsive grid.

## 7. Tests
- No HTTP integration tests for `team_tier_info`.
- `recompute_team_tier` Postgres function not visible in audit scope.
- `affiliate_team_integration.rs:600` test sets `team_commission_rate_bps=450, current_team_tier='Sovereign'` (smoke) but doesn't exercise the API.
- No frontend E2E tests.

## 8. Functional gaps & dead code
- `renderHistory` (tier.js:87-113) is dead code — HTML for `#dat-history-tbody` was removed but the function + branches stay. Should be deleted.
- `nextTarget` variable in `renderLadder` (tier.js:61) is computed but never used.
- No "Estimate next tier reward" projector despite forecast endpoint (`/analytics/forecast`) existing.
- Updated-at pill (line 30) only flips className to `--active` after data loads — if data fails the pill stays "Updating…".
- No `TODO`/`FIXME`/`mock`/`Lorem` markers.

## 9. Production blockers
- **Medium**: ~~No HTTP integration test for `team_tier_info`~~ — **RESOLVED 2026-05-19** via `backend/tests/developer_affiliate_team_http.rs` (`tier_debounce_skips_recompute_within_five_minutes` + `tier_recompute_runs_when_stale` cover F20 debounce).
- **Low**: Dead `renderHistory` + `nextTarget` to clean up.
- **Low**: Could link "Next tier" remaining-amount tooltip to forecast data to estimate ETA.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Hero + ladder + compare cards; progress bar; max-tier handling. |
| Backend wiring | 2/2 | Endpoint implemented with debounced recompute. |
| Data realism | 2/2 | Real ladder + real per-team volume + real personal tier. |
| Error/empty states | 1/1 | "Loading…" placeholders + max-tier branch + toast. |
| Mobile/responsive | 0.5/1 | Hero collapses; depends on shared CSS media queries. |
| Tests | 1/1 | HTTP coverage + F20 debounce regression in `backend/tests/developer_affiliate_team_http.rs` (`tier_debounce_skips_recompute_within_five_minutes`, `tier_recompute_runs_when_stale`); E2E in parametrized `tests/e2e/test_developer_affiliate_team.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | aria-live, sr-only caption, EUR formatting, 5-min debounce. |
| **TOTAL** | **9.5/10** | Remaining gap: mobile + dead `renderHistory`. |
