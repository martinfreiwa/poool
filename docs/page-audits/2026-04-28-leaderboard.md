# Page Audit: Leaderboard

Date: 2026-04-28
Status: fixed_needs_runtime_recheck
Auditor: ChatGPT/Codex
Page URL: `/leaderboard`
Template: `frontend/platform/leaderboard.html`
JavaScript: `frontend/platform/static/js/leaderboard.js`, `frontend/platform/static/js/legal-enhancements.js`, `frontend/platform/static/js/mobile-navigation.js`
CSS: `frontend/platform/static/css/leaderboard.css`
Backend Routes: `backend/src/leaderboard/mod.rs`, `backend/src/leaderboard/routes.rs`, `backend/src/leaderboard/service.rs`, `backend/src/main.rs`

---

## Summary

The authenticated leaderboard page is route-registered and backed by real leaderboard, preferences, and featured-asset APIs. Static review shows the core table, metric tabs, timeframe tabs, rows-per-page select, pagination, loading state, initial error state, and empty state are wired.

The audit findings have been fixed in the local working tree: the template and controller are aligned, refetch failures now surface a visible inline status, pagination totals use the same filtered result set as the rows, and mobile/pagination accessibility has been repaired. Runtime browser/API testing still needs recheck because local backend verification can be blocked by unrelated repository issues outside the leaderboard surface.

---

## Tested Scope

- Static template review of `frontend/platform/leaderboard.html`.
- Static shared component review of `frontend/platform/components/investor-topbar.html` and `frontend/platform/components/mobile-menu.html`.
- Static JavaScript review of `frontend/platform/static/js/leaderboard.js`, `legal-enhancements.js`, and `mobile-navigation.js`.
- Backend route and service review for `GET /leaderboard`, `GET /api/leaderboard`, `GET /api/leaderboard/me`, `GET/PUT /api/leaderboard/preferences`, `GET /api/leaderboard/refresh`, and `GET /api/assets/featured`.
- Database review for `leaderboard_scores`, `leaderboard_preferences`, `investments`, `referral_tracking`, `assets`, `asset_images`, `user_profiles`, `user_tiers`, and `tiers`.
- Existing test discovery for leaderboard HTTP/API and settings privacy coverage.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/leaderboard` | Authenticated page via `serve_protected`. |
| URL alias | `/leaderboard.html` | Same handler. |
| Template | `frontend/platform/leaderboard.html` | Page shell, loading/error/empty/content states, rankings table. |
| Component | `frontend/platform/components/investor-topbar.html` | Metric and timeframe controls for leaderboard variant. |
| Component | `frontend/platform/components/mobile-menu.html` | Shared mobile nav included on the page. |
| JS | `frontend/platform/static/js/leaderboard.js` | Fetches/render rankings, preferences, pagination, featured assets. |
| JS | `frontend/platform/static/js/legal-enhancements.js` | Cookie consent and legal-page helpers. |
| JS | `frontend/platform/static/js/mobile-navigation.js` | Mobile menu open/close, badges, profile dropdown. |
| CSS | `frontend/platform/static/css/leaderboard.css` | Main page layout and responsive styling. |
| Backend page route | `GET /leaderboard` | `backend/src/leaderboard/routes.rs::page_leaderboard`. |
| Backend API route | `GET /api/leaderboard` | Rankings for selected metric/timeframe/page. |
| Backend API route | `GET /api/leaderboard/me` | Current user's rank. |
| Backend API route | `GET/PUT /api/leaderboard/preferences` | Visibility/avatar/display-name preferences. |
| Backend API route | `GET /api/assets/featured` | Featured asset JSON for a missing spotlight widget. |
| Database table | `leaderboard_scores` | Precomputed all-time metrics and ranks. |
| Database table | `leaderboard_preferences` | Visibility, avatar, display name, and bio. |
| Database table | `investments`, `referral_tracking` | Timeframe ranking source data. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Metric tabs | `.lb-topbar-tab[data-metric]` | Switch ranking metric and refresh table/cards. | Yes, `switchMetricTab`. | Yes, `metric` query maps to server columns. | Static verified; runtime blocked. |
| Timeframe tabs | `.lb-tf-btn[data-timeframe]` | Switch all-time/week/month ranking data. | Yes, `switchTimeframe`. | Yes, timeframe SQL paths exist. | Static verified; pagination count issue noted. |
| Loading state | `#lb-loading-layer` | Show skeleton before initial data load. | Yes, initial page state and `showLayer`. | Not backend-dependent. | Static verified. |
| Error state | `#lb-error-layer`, Retry button | Show failed initial load and reload page. | Yes for initial load only. | Depends on API failures. | Refetch failures do not show this state. |
| Empty state | `#lb-empty-layer`, Explore Marketplace link | Show no leaderboard data and link to marketplace. | Template exists. | API can return empty rankings. | Not actually selected by JS; empty table row is used instead. |
| Bento cards | `#lb-bento-grid` | Render top 3 ranked users. | Yes. | Yes, rankings API. | Static verified. |
| Minor cards | `#lb-minor-grid` | Render ranks 4-9. | Yes. | Yes, rankings API. | Static verified. |
| Rankings table | `#lb-rankings-table`, `#lb-rankings-body` | Render paginated ranks and metrics. | Yes. | Yes, rankings API. | Static verified. |
| Rows per page | `#lb-per-page-select` | Change `per_page` and reload first page. | Yes, `changePerPage`. | Yes, backend clamps 1-100. | Static verified. |
| Pagination | `#lb-pagination-controls` | Navigate pages. | Yes; buttons now have labels and current-page semantics. | Yes; count now uses the filtered all-time/timeframe dataset. | Static fixed; runtime recheck pending. |
| Current user rank card | `#lb-my-rank-card`, `#lb-my-rank` | Show `my_rank` even when user is not on current page. | Yes. | API returns `my_rank`. | Static fixed; runtime recheck pending. |
| Visibility toggle | `#lb-visibility-toggle` | Toggle leaderboard visibility. | Yes; failure rolls back visible state. | API exists with CSRF middleware. | Static fixed; runtime recheck pending. |
| Search filter | `#lb-search-input`, `debounceSearch` | Filter rankings by public display name. | Yes. | Backend accepts `search`. | Static fixed; runtime recheck pending. |
| Featured spotlight | Removed JS branch | Rotate featured assets. | Removed from page controller because no widget exists in the template. | API remains available for other consumers. | Static fixed. |
| Mobile menu leaderboard item | `mobile-menu.html` | Navigate to current page. | Yes; real link and active state branch. | Page exists. | Static fixed; runtime recheck pending. |
| Cookie consent buttons | `#cookie-accept`, `#cookie-reject` | Store cookie preferences. | Yes. | Not backend-dependent. | Static verified. |

---

## Frontend Findings

### P2 - Template and controller are out of sync - Fixed Locally

Location:

- Template: `frontend/platform/leaderboard.html:55`
- JS: `frontend/platform/static/js/leaderboard.js:190`, `frontend/platform/static/js/leaderboard.js:533`, `frontend/platform/static/js/leaderboard.js:699`, `frontend/platform/static/js/leaderboard.js:738`

Problem:

`leaderboard.js` fetches featured assets and renders current-user rank, visibility, metric select, tier tabs, search, summary grid, and last-updated controls, but the current template only contains bento cards, minor cards, a table, rows-per-page, and pagination. The API supports `my_rank`, preferences, search, and tier filtering, but users cannot interact with those capabilities on this page.

Expected:

Either restore the missing UI controls or remove the dead controller branches and fetches. The page should visibly show the user's own rank if the product promise is "See how you rank."

Evidence:

The template has no `lb-my-rank-card`, `lb-visibility-toggle`, `lb-tier-tabs`, `lb-spotlight-card`, `lb-last-updated`, or search input. The JS still queries those IDs and calls `GET /api/assets/featured`.

Fix:

Added the search/current-rank/visibility/last-updated summary controls to `frontend/platform/leaderboard.html`, wired supporting styles, removed the dead featured-asset spotlight fetch/controller branch, removed dead tier/metric-select references, and centralized live/demo/empty rendering in `renderLeaderboardData`.

### P2 - Refetch failures keep stale leaderboard data visible - Fixed Locally

Location:

- JS: `frontend/platform/static/js/leaderboard.js:756`

Problem:

Initial load failures show `#lb-error-layer`, but later metric, timeframe, pagination, search, and per-page failures only log `Refetch failed` and restore stale content opacity. Users can believe old rankings reflect the newly selected filter.

Expected:

Refetch failures should show a visible inline error, keep the previous control state honest, and avoid presenting stale rows as fresh results.

Evidence:

`refetchAndRender()` catches errors at lines 799-800 and does not call `showLayer('error')`, update a status region, or roll back active tabs/page controls.

Fix:

Added `#lb-inline-status`, disabled leaderboard controls while refetching, surfaced failed refreshes visibly, kept previous rows explicitly labeled as stale, and made preference updates throw/rollback when the save fails.

### P2 - Mobile navigation still marks Leaderboard as unavailable - Fixed Locally

Location:

- Template: `frontend/platform/components/mobile-menu.html:114`

Problem:

The shared mobile menu renders Leaderboard as a disabled item with a "Soon" badge even though `/leaderboard` is a real authenticated page and this template includes the mobile menu. It also hardcodes Cart as active in the fallback markup.

Expected:

Mobile users should be able to navigate to Leaderboard and the active state should reflect the current route.

Evidence:

`mobile-menu.html` renders a non-anchor `.mobile-burger-menu__nav-item--disabled` for Leaderboard.

Fix:

Replaced both disabled mobile Leaderboard entries with real `/leaderboard` links and added `/leaderboard` to the mobile active-route script.

---

## Backend Findings

### P2 - Pagination totals do not match filtered/timeframe result sets - Fixed Locally

Location:

- Backend: `backend/src/leaderboard/service.rs:282`, `backend/src/leaderboard/service.rs:436`
- JS: `frontend/platform/static/js/leaderboard.js:314`

Problem:

All-time `total_participants` counts all rows in `leaderboard_scores`, ignoring `search` and `tier_id`. Timeframe `total_participants` counts only distinct investment users in the timeframe, ignoring referral-only leaderboard rows and also ignoring filters. The frontend uses this number to decide whether to render pagination.

Expected:

The API should return the count for the same filtered/ranked dataset returned in `rankings`.

Evidence:

All-time count uses `SELECT COUNT(*)::BIGINT FROM leaderboard_scores`. Timeframe count uses only `investments WHERE status = 'active' AND purchased_at >= cutoff`, while the actual timeframe rankings use a full outer join of investment and referral aggregates.

Fix:

Updated all-time and timeframe count queries in `backend/src/leaderboard/service.rs` to count from the same filtered/anonymized row source used by the rankings query. Timeframe counts now include referral-only rows and count query failures propagate instead of silently returning zero.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/leaderboard.js` | Syntax passes. | Passed with no output. | Pass |
| Rust format gate | `cd backend && cargo fmt --check` | No formatting diffs. | Passed with no output. | Pass |
| Scoped whitespace check | `git diff --check -- <touched files>` | No whitespace errors. | Passed with no output. | Pass |
| Backend type check | `cd backend && CARGO_TARGET_DIR=/tmp/poool-leaderboard-target cargo check -q` | Type check passes. | Passed with no output. | Pass |
| Authenticated page/API smoke | Start backend, request `/leaderboard` and `/api/leaderboard`. | Page/API load for authenticated user. | Not run in this pass; depends on backend check/startup. | Pending |
| Browser/mobile/console pass | Open `/leaderboard`, exercise tabs, pagination, mobile menu. | No console/network errors; controls work. | Not run in this pass; authenticated browser fixture still needed. | Pending |

---

## Security Findings

No open security finding was identified in static review. The page and APIs require an authenticated session, mutating preference updates are protected by the platform CSRF middleware/global fetch wrapper, and hidden users are anonymized before display unless they are the current user or have opted into visibility.

Residual security risk: runtime auth/CSRF behavior still needs browser/API verification with an authenticated fixture.

---

## Database Findings

- `leaderboard_scores` has integer/BIGINT metric columns and rank indexes via migrations `023`, `025`, and `046`.
- `leaderboard_preferences.display_name` exists via migration `024`; `bio` exists via migration `078`.
- Timeframe rankings use integer cents and basis points. No float money handling was found in the leaderboard path.
- Pagination count logic was corrected in `backend/src/leaderboard/service.rs` so all-time and timeframe totals honor the same filters as the rendered rows.

---

## Missing Tests

- Browser E2E for `/leaderboard` initial load, metric tabs, timeframe tabs, per-page select, pagination, empty state, initial API failure, and refetch API failure.
- Mobile E2E for the burger menu's Leaderboard navigation and active state.
- API tests asserting `total_participants` honors `search`, `tier_id`, and referral-only timeframe participants.
- Accessibility regression for active tab semantics and loading/error announcements.

---

## Recommended Fix Order

1. Add browser/API E2E coverage for the fixed leaderboard interactions.
2. Add API tests for filtered all-time counts and referral-only timeframe count coverage.
3. Re-run authenticated mobile/browser verification with a seeded user once the backend can be started in the target environment.

---

## Final Status

`fixed_needs_runtime_recheck`

Reason: All four audit findings were fixed locally, and static syntax/format/whitespace checks passed. Remaining work is runtime browser/API/E2E verification with authenticated fixtures and durable tests for the corrected pagination counts.
