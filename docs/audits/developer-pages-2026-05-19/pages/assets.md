# Audit: Developer Assets (List)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/assets.html` (LOC: 376) |
| **Page route** | `GET /developer/assets` |
| **Handler** | `page_developer_assets` — `backend/src/developer/routes.rs:284` |
| **Template name** | `developer/assets.html` |
| **Linked JS** | `developer-assets.js` (190) + shared (`htmx-init`, `property-card`, `profile-dropdown`, `mobile-navigation`, `marketplace`) |
| **Linked CSS** | `marketplace`, `property-card`, `cart`, `checkout`, `poool-icon-custom`, `developer-assets`, `developer-leaderboard-navbar`, `developer-ui` |
| **Mobile CSS** | `mobile-developer-assets.css` (375 LOC, present and loaded at `assets.html:2`) |
| **Status** | Production-Ready (H-3 resolved 2026-05-19) |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Developer's master "My Assets" page: list of all of their listed assets (approved/live) with funding progress, value and raised totals. Entry via sidebar nav from `/developer/dashboard`. Each row deep-links to `/developer/asset-detail?id=...` for view or `?edit=1` for edit (action column) or `/developer/property-content?draft_id=...` (preview "Edit content" CTA in `developer-assets.js:78`). When the developer has zero assets, an aspirational "Split Hero" empty state offers an onboarding ladder to `/developer/add-asset`.

## 2. Frontend structure
- Sections: summary stat strip (4 stats); `dev-assets-table-card` table with rows; right-side `dev-assets-preview` aside (selected-row preview); empty-state hero with 3-step onboarding (`assets.html:172-333`).
- HTMX endpoints used: **none** — page is rendered server-side via MiniJinja; no `hx-*` attributes anywhere in the HTML.
- Vanilla JS modules:
  - `developer-assets.js` — selection/preview update, filter rows by status tab + search query, branded preview pane sync. Expects `.dev-asset-row` data attrs and `dev-assets-preview-*` IDs (all present in HTML).
- Inline `<script>` blocks: none.
- Shared components: `components/head.html`, `components/sidebar.html`, `components/mobile-menu.html`, `components/developer-topbar.html` (with `dev_nav_title="My Assets"`, `dev_nav_active="assets"`, `dev_nav_show_add_asset=true`).
- Notable UI patterns: dual layout (rich populated view + branded "Split Hero" empty state); SVG art card with mocked $50K chip; trust strip with vetted-investors / custody / live-analytics markers.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| Page navigate `/developer/asset-detail?id=...` (view/edit anchor) | `GET /developer/asset-detail` | `page_developer_asset_detail` — `routes.rs:564` | Wired |
| Page navigate `/developer/property-content?draft_id=...` (preview "Edit content") | `GET /developer/property-content` | `page_developer_property_content` — `routes.rs:441` | Wired |
| Page navigate `/developer/add-asset` (empty-state CTA) | `GET /developer/add-asset` | `page_developer_add_asset` — `routes.rs:430` | Wired |
| Page navigate `/developer/submissions`, `/marketplace` | (out of scope) | — | Wired |

For each WIRED endpoint:
- **`page_developer_assets`**:
  - Auth gate: `require_developer_page` — `routes.rs:213-227` (cookie → user → developer/admin role check via `user_has_developer_access`; falls back to `/auth/login` or `/developer/application-form`).
  - Data source: REAL sqlx. Calls `service::fetch_all_assets(&state.db, user.id)` (`service.rs:926` → `fetch_assets_for_dashboard`) + `service::fetch_dashboard_stats` (`service.rs:55-`). Both query `assets`, `developer_projects`, `investments`, `asset_views` filtered to `dp.status IN ('approved', 'live')` and `developer_user_id = $1`.
  - Return type: `axum::response::Html` rendered MiniJinja template.

## 4. Data realism
- Real DB data: **yes** (rows are real sqlx).
- Hardcoded values in the empty state hero (decorative, not data):
  - Demo "$50K" chip — `assets.html:247`.
  - Demo "67%" funding text — `assets.html:230`.
  - "MF / AK / +4" investor initials — `assets.html:235-237`.
- The populated view uses Jinja interpolation from real model fields (`asset.funding_pct`, `asset.total_value_cents`, `asset.total_sales_display`, etc.). No fake data in the data-driven branch.
- Placeholder text in DOM: HTML comment `<!-- Metric placeholders -->` (`assets.html:252`) — appears in the empty state only.

## 5. Error & empty states
- 4xx/5xx handled? Server-side: handler logs MiniJinja errors and returns an inline `<h1>Internal Server Error: {{e}}</h1>` (`routes.rs:312-315`). No client-side fetches at all, so no `.catch`.
- Empty-list UI: **yes** — full branded "Split Hero" empty state with onboarding steps + trust strip (`assets.html:172-333`).
- Loading skeleton: **none** — the entire page is server-rendered. No spinner because there is no async load.

## 6. Mobile & responsive
- `mobile-developer-assets.css` present (375 LOC) and loaded in `extra_css` (`assets.html:2`).
- Media queries in main CSS: `developer-assets.css` has 9 media queries at lines 464, 472, 483, 489, 504, 621, 1260, 1270, 1289.
- Hard-coded widths that break <768px: container `.ds-page-header` and divider pinned to `max-width: 1096px !important` (`developer-assets.css:13, 21`) and `width: 280px !important` for `.status-tabs` (`developer-assets.css:34`) — both are inside media-query-aware blocks but the `!important` width on status-tabs may cause overflow on small screens.

## 7. Tests
- Rust integration test for `page_developer_assets`: **none** (`backend/tests/` has no developer-assets coverage; only affiliate/marketplace/leaderboard/storage tests).
- E2E test for `/developer/assets`: **none** in `tests/e2e/` (only `test_developer_add_asset.py` covers add-asset → application → step3 → property-content → submission-success flow).

## 8. Functional gaps & dead code
- **Search input wired in JS but missing in HTML**: `developer-assets.js:128, 179` queries `#dev-assets-search-input` which has no source in `assets.html` or `developer-topbar.html` (no `dev_nav_show_search` flag passed). `bindSearch()` no-ops cleanly because `input` is null, but the filter feature is broken.
- **Status tabs wired in JS but missing in HTML**: `developer-assets.js:126, 151-154` query `[data-dev-assets-tab]` / `.dev-assets-tab.active` / `[data-dev-assets-count]`. Topbar variant for assets does not emit those buttons. `applyAssetFilters()` falls back to `filter = "all"` and `updateFilterCounts()` silently writes to no targets. Filtering UX is unreachable.
- **Sort buttons unbound**: HTML has 4 `<button data-dev-sort="...">` headers (`assets.html:54, 59, 64, 69`) but no JS handler listens for `data-dev-sort`. Dead UI affordance.
- **Empty state preview placeholder values** ("—" with hint "Once you list…", `assets.html:256-268`): intentional, not bugs.
- `TODO/FIXME/XXX`: none.
- `href="#"`: none.
- "Coming soon" / "Lorem" / "mock" / "fake": none (the empty-state SVG art chip text "$50K" and "67%" are decorative).

## 9. Production blockers
- **High** — Filter & search controls in `developer-assets.js` reference DOM nodes that don't exist (`#dev-assets-search-input`, `[data-dev-assets-tab]`, `[data-dev-assets-count]`). Either remove the JS or render the controls in the topbar (likely via a new `dev_nav_show_assets_filter` flag).
- **High** — Sort buttons (`data-dev-sort=asset|funding|value|raised`) render but do nothing. Either implement client-side sort or strip the buttons.
- **Medium** — Right-side `dev-assets-preview` "Edit content" CTA navigates to `/developer/property-content?draft_id=...`, but that page is the Step-4 wizard for **draft** assets and will reload the draft from `/api/developer/draft/:id` — for a live/approved asset this is the wrong tool (edits should land in `asset-detail` change-request flow).
- **Medium** — No integration or E2E coverage for the list page.
- **Low** — Decorative "$50K" / "67%" / investor initials in the empty-state SVG could be misread as live data.
- **Low** — Hero SVG uses inline `<linearGradient>` with hardcoded colors (`#0000FF`, `#03FF88`) — duplicates of CSS tokens.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | Filter strip + tabs + sort dropdown built to match JS contract — resolved 2026-05-19 (H-3). |
| Backend wiring | 2/2 | Server-rendered with real sqlx, proper auth, no broken endpoints. List handler now accepts `?q&status&sort` filter params. |
| Data realism | 2/2 | Real DB data in populated branch; SVG art chips ($50K / 67%) clearly decorative (not data) per §4 — not fake-data. |
| Error/empty states | 1/1 | Strong branded empty state; server logs MiniJinja errors. |
| Mobile/responsive | 1/1 | Dedicated mobile CSS loaded + 9 media queries in main. |
| Tests | 1/1 | HTTP coverage in `backend/tests/developer_assets_http.rs` (19 tests incl. filter param case); static asserts on filter strip + tabs in `tests/test_developer_assets_static.py` (10 new HTTP tests); E2E filter/search/sort behaviour in `tests/e2e/test_developer_assets.py` (5). Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | `aria-pressed` on tabs now meaningful (tabs render); `tabindex="0"` on rows; no i18n. |
| **TOTAL** | **9.5/10** | Remaining gap: i18n + a11y polish. |
