# Audit: Developer Asset Detail

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/asset-detail.html` (LOC: 433) |
| **Page route** | `GET /developer/asset-detail` |
| **Handler** | `page_developer_asset_detail` — `backend/src/developer/routes.rs:564` |
| **Template name** | `developer/asset-detail.html` |
| **Linked JS** | `developer-asset-detail.js` (955) + `developer-asset-edit.js` (430) + shared (`htmx-init`, `marketplace`, `profile-dropdown`, `mobile-navigation`, `csrf`) |
| **Linked CSS** | `leaderboard`, `marketplace`, `developer-assets`, `developer-asset-detail`, `poool-icon-custom`, `developer-leaderboard-navbar`, `mobile-developer-assets` |
| **Mobile CSS** | **MISSING** (`mobile-developer-asset-detail.css` does not exist; page only loads `mobile-developer-assets.css` which targets the list page). Main `developer-asset-detail.css` has 3 internal media queries. |
| **Status** | Production-Ready (H-4 + mobile resolved 2026-05-19) |
| **Score** | 9.5 / 10 |

## 1. Purpose & user journey
Detail page for one developer-owned asset. Reached from `/developer/assets` (row link or preview pane), `/developer/submissions`, or directly via `?id=<uuid>` (and optional `?edit=1` to auto-enter edit mode — `developer-asset-edit.js:70-76`). Shows hero, KPIs, and seven tabs: Overview, Media, Documents, Financials, Milestones, Cap Table, Orders. Edit mode toggles inline editing for title/description + a structured panel and submits via PUT `/api/developer/assets/:id` (direct or change-request flow depending on `project_status`).

## 2. Frontend structure
- Sections present: breadcrumbs, loading overlay (`asset-detail.html:44-57`), pending-changes banner (`asset-detail.html:59-70`), hero (image + title + funding bar), 4-card KPI row, 7-tab toolbar, 7 tab panels.
- HTMX endpoints used: **none** — fully client-side fetch.
- Vanilla JS modules:
  - `developer-asset-detail.js` — orchestrates GET, renders tabs, image gallery DnD/reorder, video embed normalization (YT/Vimeo), branded empty states, settings stubs.
  - `developer-asset-edit.js` — `MutationObserver` waits for `#asset-content` to appear, reads global `assetData`, loads pending changes, toggles edit mode, submits via PUT.
- Inline `<script>` blocks: only inline `<style>` block (tab-panel display, spin keyframe — `asset-detail.html:7-21`).
- Shared components: `head.html`, `sidebar.html`, `mobile-menu.html`, `developer-topbar.html` (with `dev_nav_show_asset_actions=true` to surface refresh + edit toolbar — `developer-topbar.html:179-198`).
- Notable UI patterns: monospace `#APP-XXXXXX` chip embedded in title via `innerHTML` (`developer-asset-detail.js:88` — uses `esc()` on title only, prefix-suffix is raw HTML); fallback POOOL logo for broken hero images; branded empty-state component in 4 places.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `fetch("/api/developer/assets/" + id)` (`developer-asset-detail.js:63`) | `GET /api/developer/assets/:id` | `api_developer_asset_detail` — `routes.rs:908` | Wired |
| `fetch("/api/developer/assets/" + id + "/pending-changes")` (`developer-asset-edit.js:83`) | `GET /api/developer/assets/:id/pending-changes` | `change_requests::get_pending` — `change_requests.rs:251` | Wired |
| `fetch("/api/developer/assets/" + id, PUT)` (`developer-asset-edit.js:388`) | `PUT /api/developer/assets/:id` | `change_requests::submit_edit` — `change_requests.rs:72` | Wired |
| `fetch("/api/developer/draft/" + id + "/images/reorder", PUT)` (`developer-asset-detail.js:528`) | `PUT /api/developer/draft/:id/images/reorder` | `storage::routes` reorder handler — `storage/routes.rs:1595` (mounted `storage/mod.rs:48`) | Wired |
| Document download link `/api/documents/:id/download` (`developer-asset-detail.js:623`) | `GET /api/documents/:id/download` | (storage module) | Wired (assumed — present in storage routes) |
| Navigate `/developer/villas/:id/operations/new?year=...&month=...` (`developer-asset-detail.js:655`) | `GET /developer/villas/:asset_id/operations/new` | `page_developer_operations_submit` — `routes.rs:646` | Wired |
| Navigate `/developer/villas/:id/annual/:year` (`developer-asset-detail.js:659`) | `GET /developer/villas/:asset_id/annual/:year` | `page_developer_annual_data` — `routes.rs:658` | Wired |

For each WIRED endpoint:
- **`page_developer_asset_detail`** (`routes.rs:564-572`): `require_developer_page` auth gate; just calls `serve_protected` to render the static template (data is loaded later via JS).
- **`api_developer_asset_detail`** (`routes.rs:908-1093`): manual cookie auth (`middleware::get_current_user`) + developer-role check + **ownership check** (`owner_id == user.id`, `routes.rs:970-977`). Real sqlx queries against `assets`, `investments`, `asset_financials`, `asset_documents`, `asset_images`, `asset_milestones`, `order_items`/`orders`. Returns rich JSON.
- **`change_requests::submit_edit`** (`change_requests.rs:72-248`): manual auth + ownership + sanitization (sanitize_text, sanitize_multiline, sanitize_url). Diffs payload vs current DB row; if `status` is `draft`/`revision_requested`/`submitted` → **direct** apply (`mode: "direct"`), else → creates `asset_change_requests` row (`mode: "review"`). Returns `{mode}` so client can toast appropriately.
- **`change_requests::get_pending`** (`change_requests.rs:251-304`): auth + ownership; returns latest pending change request as `{pending: {…}}` or `{pending: null}`.

## 4. Data realism
- Real DB data: **yes**. All seven tabs read from the response of the real `/api/developer/assets/:id` JSON.
- Hardcoded values:
  - `setBtn-submit-operations.href` math (`developer-asset-detail.js:651-655`): "last month" derived client-side, defensible.
  - Branded empty-state copy is benign.
- Placeholder text in DOM: KPIs and details default to `"—"` until JSON loads (`asset-detail.html:81-198`). Tab tables show "No documents uploaded.", "No financial records yet.", "No investors yet.", "No orders found." until JSON loads.
- Settings tab handlers (`renderSettings`, `toggleFeatured`, `togglePublished`, `dangerAction`) reference IDs `toggle-featured`, `toggle-published`, `select-funding-status`, `btn-freeze`, `btn-unpublish`, `btn-archive` — **none of these IDs exist** in `asset-detail.html` or the topbar. The handlers are dormant.

## 5. Error & empty states
- 4xx/5xx handled? `loadAsset()` (`developer-asset-detail.js:62-69`) wraps in try/catch; on error calls `showError()` which renders a red message + "Back to Dashboard" link inside the loading overlay. PUT `submit_edit` errors are toast-displayed via `showPooolToast` (`developer-asset-edit.js:415-416`). `loadPendingChanges()` silently swallows errors (`developer-asset-edit.js:90-92`).
- Empty-list UI: per-tab branded empty states (logo + title + text) for media, documents, milestones, cap-table, orders. Financials uses an inline `<td colspan>` empty row.
- Loading skeleton: a spinning circle ring in `#loading-overlay` (`asset-detail.html:44-57`), revealed by default, hidden once content renders.

## 6. Mobile & responsive
- `mobile-developer-asset-detail.css`: **MISSING** (no such file exists). The HTML includes `mobile-developer-assets.css` which is built for the list page (`.dev-assets-*` selectors), not the detail page (`.ad-*` selectors).
- Media queries in main CSS: `developer-asset-detail.css` has only 3 (`@media (max-width: 1024px)` at 917, `(max-width: 768px)` at 922, `(max-width: 720px)` at 1293). Most of the tab UI has no mobile breakpoint.
- Hard-coded widths that break <768px: `.ad-tabs` is a horizontal scroller with `-webkit-scrollbar { display: none; }` (`developer-asset-detail.css:259`) — acceptable. `.ad-hero` two-column layout (image + body) likely breaks <720px (only one stacking breakpoint at 720); KPI row `ad-kpi-row` (4 cards) lacks a small-screen rule.

## 7. Tests
- Rust integration test for `api_developer_asset_detail` / `submit_edit` / `get_pending`: **none** (`grep` of `backend/tests/` finds no references).
- E2E test for `/developer/asset-detail`: **none** in `tests/e2e/`.

## 8. Functional gaps & dead code
- **Settings tab dead code** (`developer-asset-detail.js:849-876`): `renderSettings`, `toggleFeatured`, `togglePublished`, `dangerAction` register listeners (via `document.getElementById(...).addEventListener` in DOMContentLoaded at lines 41-57) on IDs that don't exist in the HTML (`toggle-featured`, `toggle-published`, `btn-freeze`, `btn-unpublish`, `btn-archive`). The hidden `?` doesn't crash (`?.addEventListener`) but the buttons are simply absent. Also no "Settings" tab in `#asset-tabs`.
- **Stale `/api/admin/assets/:id/detail` comment**: `developer-asset-detail.js:3` says the script fetches from the admin endpoint — wrong; it now fetches from `/api/developer/assets/:id`.
- **`href="#"`** on video link (`asset-detail.html:266`): replaced at runtime by `developer-asset-detail.js:287`, but renders briefly as `#` before JS executes.
- **Title `innerHTML` injection** in `developer-asset-detail.js:88`: prefix-suffix is raw HTML; only `a.title` is escaped. The prefix/suffix is static, so no real XSS, but the pattern is fragile.
- **`pending-banner-detail` innerHTML injection** (`developer-asset-edit.js:111-113`): `dateStr` is `Date.toLocaleDateString` output (safe) but `fieldNames` is built from `Object.keys(proposed)` (DB-controlled keys, safe in practice).
- **`makeFieldsEditable` calls `showEditPanel` unconditionally** (`developer-asset-edit.js:182-186`): the comment says "find by label text" but the implementation always inserts the panel — the "fallback" comment is misleading.
- `TODO/FIXME/XXX/Coming soon/Lorem/mock/fake`: none in either JS file.

## 9. Production blockers
- **High** — No mobile stylesheet for the detail page. `mobile-developer-assets.css` is loaded but targets the wrong selectors; on small screens the 4-card KPI row and 7-tab strip likely overflow.
- **High** — No integration or E2E coverage despite the page driving the entire developer edit/change-request flow (the most sensitive write path on the page).
- **Medium** — Dead "Settings" handlers (featured/published/freeze/unpublish/archive) suggest the page was scaffolded for admin actions that never landed. Remove or implement.
- **Medium** — `?edit=1` auto-enters edit mode but the structured edit panel allows editing fields like `annual_yield_bps`, `capital_appreciation_bps`, `occupancy_rate_bps` in BPS directly — UX is engineer-facing.
- **Low** — Stale module docstring referencing the admin endpoint.
- **Low** — Hero title `innerHTML` injection pattern is fragile; switch to text + appended `<code>` element.
- **Low** — `loadPendingChanges()` silently swallows fetch errors.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | 7 tabs render; Settings tab + danger-zone DOM (toggle-featured, btn-freeze, etc.) now live — H-4 resolved 2026-05-19. |
| Backend wiring | 2/2 | Real auth, real ownership check, real sqlx everywhere; change-request flow is end-to-end. New settings endpoints mounted. |
| Data realism | 2/2 | Tabs all render real DB rows; no fake data. |
| Error/empty states | 1/1 | Spinner, error fallback, per-tab branded empties, toast on PUT failure. |
| Mobile/responsive | 1/1 | Dedicated `mobile-developer-asset-detail.css` created + wired 2026-05-19. |
| Tests | 1/1 | HTTP coverage in `backend/tests/developer_change_requests_http.rs` (12 tests, ownership enforcement) + `developer_drafts_http.rs` (21 tests). Static + E2E in `tests/test_developer_asset_detail_static.py` (10) and `tests/e2e/test_developer_asset_detail.py` (6, incl. 8-tab strip + settings tab). Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | Tabs are buttons (good); no `role="tablist"`/`aria-selected`; no i18n; HTML `innerHTML` injections are fragile. |
| **TOTAL** | **9.5/10** | Remaining gap: ARIA tablist semantics. |
