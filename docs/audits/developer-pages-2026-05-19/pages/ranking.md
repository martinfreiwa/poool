# Audit: Developer Ranking (Leaderboard reuse)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/ranking.html` (LOC: 161) |
| **Page route** | `GET /developer/ranking` |
| **Handler** | `page_developer_ranking` — `backend/src/developer/routes.rs:619` |
| **Template name** | `developer/ranking.html` (via `crate::common::routes_helper::serve_protected`) |
| **Linked JS** | (via `extra_js`) `htmx-init.js`, `profile-dropdown.js`, `marketplace-search.js`, `poool-dropdown.js`; (via direct `<script src>` at end of body, `:156-158`) `leaderboard.js?v=14` (1030), `legal-enhancements.js`, `mobile-navigation.js`; lazy `leaderboard-demo.js` (70) only when `?demo` query param present |
| **Linked CSS** | `leaderboard.css` (1808) |
| **Mobile CSS** | **MISSING dedicated** (no `mobile-leaderboard.css`). `leaderboard.css` ships 12 inline `@media` queries at 480/640/768/900/1024/1280 px breakpoints. |
| **Status** | Production-Ready (M-1 resolved 2026-05-19) |
| **Score** | 9 / 10 |

## 1. Purpose & user journey
This is a thin alias — it serves the **exact same `developer/ranking.html` template** (which is identical in structure to `/leaderboard`) to developers so the sidebar's "Ranking" item lives under `/developer/...` rather than navigating away to the investor leaderboard. Lands here from the developer sidebar (`sidebar.html:50` maps `/developer/ranking → activePage = "ranking"`). User journey: view your own rank, browse top-3 bento + minor cards (ranks 4–9), browse paginated table, click "Explore Marketplace" to invest more.

## 2. Frontend structure
- Sections: loading skeleton layer (`:23-30`), error layer (`:33-42`), empty layer (`:45-52`), content layer (`:55-150`).
- Inside content: "Your Standing" card with breakdown rendered client-side, summary grid (`#lb-bento-grid`, `#lb-minor-grid`), Rankings Ledger table with per-page select + pagination, admin-only "Refresh now" button (`:108-116`, hidden by default).
- HTMX endpoints used: none directly in this file. Pure `fetch` from `leaderboard.js`.
- Vanilla JS:
  - `leaderboard.js` — `init()` calls `fetchRankings` + `fetchPreferences` in parallel, lazy-loads `leaderboard-demo.js` only when `?demo` is in the URL, renders the four state layers, wires ARIA tablist keyboard nav, admin refresh handler, pagination, debounced search/tier filter.
  - `legal-enhancements.js`, `mobile-navigation.js`, `marketplace-search.js`, `poool-dropdown.js`, `profile-dropdown.js`, `htmx-init.js` — UI primitives.
- Inline `<script>`: none — the only inline JS is the `onclick="location.reload()"` retry on the error layer (`:41`) and the `onclick="changePerPage(this.value)"` (`:138`).
- Shared components: `components/head.html`, `components/mobile-menu.html`, `components/sidebar.html`, `components/investor-topbar.html` (with `investor_topbar_variant="leaderboard"` → renders the 6 metric tabs and 3 timeframe tabs).
- Anti-pattern: the page uses `components/investor-topbar.html` (not the developer topbar), so the topbar visually matches the investor leaderboard. Acceptable since this is a leaderboard view, but means the sidebar says "Developer / Ranking" while the page header says "Leaderboard".

## 3. Backend wiring

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/leaderboard?metric=…&timeframe=…&page=…&per_page=…&search=…&tier_id=…` (`fetchRankings`) | `/api/leaderboard` | `leaderboard::routes::get_rankings` (`leaderboard/routes.rs:95`) | wired |
| `GET /api/leaderboard/preferences` (`fetchPreferences`) | `/api/leaderboard/preferences` | `leaderboard::routes::get_preferences` (`leaderboard/routes.rs:323`) | wired |
| `PUT /api/leaderboard/preferences` (`updatePreferences`) | same | `leaderboard::routes::update_preferences` (`leaderboard/routes.rs:347`) | wired |
| `POST /api/leaderboard/refresh` (`adminRefreshLeaderboard`) | `/api/leaderboard/refresh` | `leaderboard::routes::trigger_refresh` (`leaderboard/routes.rs:416`) | wired (admin-only) |
| `GET /api/me` (`revealAdminControls`) | `/api/me` | `api_me` (`backend/src/lib.rs:1642`) | wired |
| `<a href="/marketplace">` (CTA) | `/marketplace` | marketplace page | wired |

For each WIRED endpoint:
- **`page_developer_ranking`** — Gated by `require_developer_page` (`routes.rs:213`) — must hold `developer`/`asset_owner`/`admin`/`super_admin` role. Then `serve_protected` returns the static `developer/ranking.html`.
- **`get_rankings`** — Gated by `require_user_id` (any logged-in user). Hand-rolled rate limit via `check_rate_limit(state, user_id, "get")`. ETag + `Cache-Control: private, max-age=30`. Real DB via `service::get_rankings` (computes from `investments` joined to `assets`, weighted target-yield, tier metadata). Returns real `LeaderboardResponse`.
- **`get_preferences` / `update_preferences`** — Real DB read/write of `leaderboard_preferences`.
- **`trigger_refresh`** — Admin-only role check; recomputes leaderboard scores.
- **`api_me`** — Returns the current user with roles list; the client uses this to conditionally reveal the admin "Refresh now" button.

## 4. Data realism
- Real DB: **yes** — every render path goes through `/api/leaderboard` against live `investments` + `assets`.
- Hardcoded values:
  - Tier-color fallback maps (`leaderboard.js:366-372`, `:496-502`) for legacy responses without `tier_badge_color`. Cosmetic only.
  - The "You are currently in the top tier of institutional traders." copy at `leaderboard.js:586` is shown to anyone with a rank, regardless of their actual tier — overclaim.
- Placeholder text in DOM: "Start investing to get ranked." (empty layer, `:50` and `leaderboard.js:586`) — appropriate.
- Demo mode: `?demo` URL param triggers lazy load of `leaderboard-demo.js`, replacing real data with a sample fixture. Production page loads do NOT download it (verified at `leaderboard.js:147-167`).

## 5. Error & empty states
- 4xx/5xx: `init()` wraps the two parallel fetches in try/catch; failures route to `showLayer('error')` (`leaderboard.js:118-122`). Refetch failures fall back to `showInlineStatus('Could not refresh the leaderboard. Previous results are still shown.')` (`:849`).
- Empty-list UI: top-level empty layer (`ranking.html:45-52` "Be the first on the leaderboard"), per-table empty row injected by `renderTable` (`leaderboard.js:605` "No investors found matching your filters."), per-card empty/hidden states.
- Skeleton/loading: yes — `#lb-loading-layer` (`:23-30`) with `.skeleton-box` divs for bento + table.

## 6. Mobile & responsive
- No dedicated `mobile-leaderboard.css`. `leaderboard.css` has 12 inline `@media` queries (`:56`, `:241`, `:618`, `:624`, `:1010`, `:1407`, `:1420`, `:1453`, `:1590`, `:1628`) covering 480/640/768/900/1024/1280 px breakpoints.
- Some inline pixel widths in markup: `:60` `style="grid-column: 1 / -1; padding: 24px;"`, `:83` `style="margin-top: 16px;"`, `:85` `style="grid-column: 1 / -1;"` — none are absolute pixel widths that would break <768px.
- `mobile-navigation.js` is loaded.

## 7. Tests
- Rust integration: `backend/tests/leaderboard_http.rs`, `leaderboard_integration.rs`, `leaderboard_production_audit.rs`, `leaderboard_roi_precision.rs` exist and cover the underlying `/api/leaderboard` engine. They do **not** specifically exercise the `/developer/ranking` page mount, but the same backend serves both.
- Python integration: `tests/test_leaderboard.py` exercises `GET /leaderboard` (`:95`) and `GET /api/leaderboard?…` (`:103`), `GET /api/leaderboard/me` (`:116`). **Does not test `/developer/ranking` specifically.**
- E2E: none for the developer alias.

## 8. Functional gaps & dead code
- Page is labelled "Leaderboard" in the topbar (`investor_topbar_title="Leaderboard"`) but the sidebar item that linked here is "Ranking". Either rename or align.
- Filters/search comment at `:75-79` says "Search + tier filter were of limited use … removed" — but the JS still ships `switchTier`, `debounceSearch`, `lb-search-input`, `lb-tier-filter` handlers (`leaderboard.js:805-827`, `:335`). Dead handlers on a removed UI.
- `setPreferenceStatus()` writes to `#lb-preference-status` (`leaderboard.js:309`) which is no longer in this page's DOM — the visibility/avatar/display-name controls were "managed exclusively in Profile Settings" per `:78`. The `applyPrefs` calls at `:729-752` look up `#lb-visibility-toggle`, `#lb-show-avatar-toggle`, `#lb-display-name-input` — none exist here. Defensive `if (toggle)` checks make this safe but it's wasted code paths.
- "You are currently in the top tier of institutional traders." (`leaderboard.js:586`) is shown verbatim to **anyone ranked** — misleading copy.
- TODO/FIXME/XXX: none.

## 9. Production blockers (severity)
- **Critical:** none — same battle-tested engine as `/leaderboard`.
- **High:**
  - No dedicated test for the `/developer/ranking` mount specifically — easy to add (one-line analogue to `test_leaderboard.py`).
- **Medium:**
  - Misleading "top tier of institutional traders" copy regardless of actual rank/tier.
  - Naming inconsistency: sidebar says "Ranking", topbar says "Leaderboard".
  - Dead handlers for removed search/tier/preferences UI (~150 LOC of `leaderboard.js` defensively no-ops here).
- **Low:**
  - No `mobile-leaderboard.css` — all responsive rules inlined into the main file. Works fine because of the 12 media queries but is inconsistent with the platform's `mobile-<page>.css` convention.
  - `?v=14` cache-buster in `<script src="…leaderboard.js?v=14">` (`:156`) is manual; should be wired into the build asset-hash pipeline.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | Four-state layer UX is polished; dead code for removed search/tier/prefs controls. |
| Backend wiring | 2/2 | Real `/api/leaderboard` + ETag + rate limit + admin refresh; same engine as the main leaderboard. |
| Data realism | 2/2 | Live DB; misleading "top tier of institutional traders" copy replaced 2026-05-19 with actual rank (`leaderboard.js:586`) — M-1 resolved. |
| Error/empty states | 1/1 | Loading / error / empty / content layers, inline status on refetch failure. |
| Mobile/responsive | 0.5/1 | 12 media queries inline in `leaderboard.css`; no dedicated `mobile-leaderboard.css`. |
| Tests | 1/1 | Route-specific HTTP tests in `backend/tests/developer_ranking_http.rs` (4 tests, covers `/api/leaderboard/me` with developer session + anonymous); page render in `tests/test_developer_ranking_static.py` (9 tests, incl. M-1 banned-copy guard); E2E in `tests/e2e/test_developer_ranking.py` (4 tests, asserts new "ranked #N" copy). Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | Roving-tabindex ARIA pattern on tablist, debounced tab refetch, ETag cache, lazy demo module. |
| **TOTAL** | **9/10** | Remaining gap: dedicated mobile sheet. |
