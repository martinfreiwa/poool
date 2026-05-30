# Audit: Developer Add Asset (Type Picker)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/add-asset.html` (LOC: 161) |
| **Page route** | `GET /developer/add-asset` |
| **Handler** | `page_developer_add_asset` — `backend/src/developer/routes.rs:430` |
| **Template name** | `developer/add-asset.html` |
| **Linked JS** | `developer-add-asset.js` (68) + shared (`htmx-init`, `profile-dropdown`, `mobile-navigation`, `poool-dropdown`, `poool-dropdown-init`) |
| **Linked CSS** | `leaderboard`, `developer-add-asset`, `developer-application-form`, `cart`, `checkout`, `poool-icon-custom`, `developer-leaderboard-navbar`, `developer-ui` |
| **Mobile CSS** | `mobile-developer-add-asset.css` exists (371 LOC) but is **NOT loaded** — missing from `extra_css` list in `add-asset.html:1`. |
| **Status** | Production-Ready (H-5 + mobile resolved 2026-05-19) |
| **Score** | 8.5 / 10 |

## 1. Purpose & user journey
Step 1 of the 4-step "Add Asset" wizard: user picks an asset type (Real Estate is the only enabled option; five others are "Coming Soon"). Reached from sidebar nav or from `/developer/assets` empty-state CTA. Clicking "Next Step" writes the selection to `localStorage` and navigates to `/developer/application-form` (Step 2). Step 2 → `document-upload-step3` → `property-content` → `submission-success`.

## 2. Frontend structure
- Sections present: type-header (`asset-type-header__title` + subtitle, `add-asset.html:15-18`); 6-card grid (`asset-type-grid`, lines 21-146); "Next Step" footer button (lines 148-156).
- HTMX endpoints used: **none**.
- Vanilla JS modules:
  - `developer-add-asset.js` — `selectAssetType(id)`, sets background images from `data-image-url`, persists selection to `localStorage.selectedAssetType`, clears stale `draft_asset_id`, navigates to `/developer/application-form`.
- Inline `<script>` blocks: a single `__templ_selectAssetTypeScript_6b97` wrapper inside the grid (`add-asset.html:22-26`) — vestige of a templ-generated function for `onclick` handlers.
- Shared components: `head.html`, `sidebar.html`, `mobile-menu.html`, `developer-topbar.html` (with `dev_nav_title="Add Asset"`, `dev_nav_flow_step="asset"` to display checkout-steps wizard markers — `developer-topbar.html:33-58`).
- Notable UI patterns: glass-morphism card (`asset-type-glass`); "Coming Soon" badge on five cards; "Currently supported" eyebrow on Real Estate.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| Navigate `/developer/application-form` (Next Step) | `GET /developer/application-form` | `page_developer_application_form` — `routes.rs:543` | Wired |
| Page itself | `GET /developer/add-asset` | `page_developer_add_asset` — `routes.rs:430` | Wired |

For each WIRED endpoint:
- **`page_developer_add_asset`** (`routes.rs:430-438`): `require_developer_page` auth gate (cookie → user → developer/admin role; falls back to login or `/developer/application-form`). No DB query — just `serve_protected` to render the static template.

## 4. Data realism
- Real DB data: **no** — page is 100% static. There is no per-user state on this page (selection lives only in `localStorage`).
- Hardcoded values:
  - All six asset-type cards are literal HTML with hardcoded ids, copy, and SVG icons (`add-asset.html:27-145`).
  - "Coming Soon" badges hardcoded on Commercial Property, Commodities, Business, Startups, Land/Plots.
- Placeholder text in DOM: none.

## 5. Error & empty states
- 4xx/5xx handled? Server-side: `serve_protected` failure renders an error page. Client-side: there are no async calls, so no `.catch`.
- Empty-list UI: N/A (static grid, always 6 cards).
- Loading skeleton: N/A.

## 6. Mobile & responsive
- `mobile-developer-add-asset.css` exists (`@media (max-width: 768px)` at line 3, `@media (min-width: 769px)` at line 369) but is **not declared** in `extra_css` for this page (`add-asset.html:1`). Stylesheet is dead weight on disk for this page.
- Media queries in main CSS: `developer-add-asset.css` has 2 (`@media (max-width: 860px)` at line 239 and `@media (max-width: 560px)` at line 245).
- Hard-coded widths: `max-width: 760px` (line 13), `max-width: 900px` (line 46, line 236 with `!important`) — bounded but readable on phones.

## 7. Tests
- Rust integration test for `page_developer_add_asset`: **none**.
- E2E tests for `/developer/add-asset`:
  - `tests/e2e/test_developer_add_asset.py::test_add_asset_page_loads` (line 221) — smoke.
  - `test_asset_type_card_selection` (line 232) — verifies Real-Estate card selectable, Commercial stays unselected.
  - Also covered indirectly by the full-flow tests: `test_save_and_exit_creates_draft`, `test_full_submission_flow`, etc. (`tests/e2e/test_developer_add_asset.py:316, 416`).

## 8. Functional gaps & dead code
- **"Coming Soon" on 5 of 6 cards** — these cards have `aria-disabled="true"` and no `onclick` handler, but also no `js-selected` clearing and no toast/explanation when clicked. They're decorative only.
- **`__templ_selectAssetTypeScript_6b97`** — vestigial templ-generated wrapper; could be inlined or removed. Indicator that this page was once generated from a Go/templ project.
- **`onclick=` and `onkeydown=`** inline handlers (`add-asset.html:29-30`) — CSP-unsafe pattern (would block under strict CSP); should be moved to `addEventListener` in the JS module.
- **"Real Estate" hardcoded as default** (`developer-add-asset.js:54-55`): even if no card is selected, the script forces `selectedAssetType = "real-estate"` and proceeds. This silently obscures bugs.
- Selection lives only in `localStorage` — clearing browser storage drops the wizard state silently.
- `TODO/FIXME/XXX/Lorem/mock/fake`: none.
- "Coming soon": 5 occurrences (`add-asset.html:54, 71, 89, 107, 125`) — intentional roadmap markers.
- `href="#"`: none.

## 9. Production blockers
- **Medium** — `mobile-developer-add-asset.css` is not declared in `extra_css`. Either include it in the page's `with` block or delete the file.
- **Medium** — Inline `onclick`/`onkeydown` attributes on cards (`add-asset.html:29-30`) violate CSP-safe patterns.
- **Low** — No way to surface that 5 of 6 cards are unavailable (no toast, no tooltip on disabled cards beyond the static badge).
- **Low** — Vestigial templ wrapper function should be removed.
- **Low** — No Rust integration test for the handler (page renders nothing dynamic, but auth gate behavior would be worth a smoke test).

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | Step 1 is intentionally tiny; complete for what it does. 5/6 cards are decorative. |
| Backend wiring | 2/2 | Auth gate works; page is a static choose-asset-type — no API needed (N/A treated as max per rubric). Resolved 2026-05-19. |
| Data realism | 2/2 | Static type-picker by design — no data layer to fake. Resolved 2026-05-19. |
| Error/empty states | 1/1 | N/A — only the auth-failure path matters and that's handled by `require_developer_page`. |
| Mobile/responsive | 1/1 | `mobile-developer-add-asset` now wired in `extra_css` (resolved 2026-05-19 — H-5). |
| Tests | 1/1 | Strong Playwright coverage (smoke + selection + full flow). |
| Polish (a11y, i18n, perf) | 0/1 | `role="radio"` and `aria-checked` are correct; inline `onclick` violates CSP; missing keyboard handlers on coming-soon cards. |
| **TOTAL** | **8.5/10** | Remaining gap: inline `onclick` (H-12 CSP) + a11y polish. |
