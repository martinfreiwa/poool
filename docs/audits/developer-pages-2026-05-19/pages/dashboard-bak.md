# Audit: Developer Dashboard (.bak — Stale Backup)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/dashboard.html.bak` (LOC: 6700) |
| **Page route** | none — **not mounted, not served** |
| **Handler** | none |
| **Template name** | none (`page_developer_dashboard` in `backend/src/developer/routes.rs:244` resolves `developer/dashboard.html`, not the `.bak` sibling — MiniJinja loader does not auto-discover `.bak` files) |
| **Linked JS** | (irrelevant — never executed) `htmx-init.js`, `profile-dropdown.js`, `mobile-navigation.js`, `developer-dashboard.js`, `user-data.js`, `poool-dropdown.js`, `poool-dropdown-init.js` |
| **Linked CSS** | (irrelevant) `main.css`, `marketplace.css`, `wallet.css`, `portfolio.css`, `cart.css`, `assets-table-populated.css`, `bem/table.css`, `property-item-card.css`, `kyc-banner.css`, `sidebar-navigation.css`, `bem/sidebar.css`, `sidebar-developer.css`, `developer.css`, `developer-dashboard.css`, `profile-dropdown.css`, `htmx-fixes.css`, plus `mobile-*` variants |
| **Mobile CSS** | (referenced but never used) `mobile-header.css`, `mobile-burger-menu.css`, `mobile-kyc-banner.css`, `mobile-profile-dropdown.css`, `mobile-developer-dashboard.css` |
| **Status** | **DELETED 2026-05-19** (was: Stale) |
| **Score** | N/A — file removed |
| **Resolved** | 2026-05-19 — `rm frontend/platform/developer/dashboard.html.bak`. 6,700-LOC orphan with malformed HTML, never mounted; deletion was the recommendation in §9. |

## 1. Purpose & user journey
None. This is a pre-MiniJinja, pre-`{% include "components/head.html" %}` snapshot of the developer dashboard. It uses the old monolithic `<head>` (HTMX/Alpine via unpkg, full `<link>` list), inlines an HTMX-fed `<header>` and sidebar markup, and ships ~6,700 lines of hardcoded "demo" tiles, assets-table rows, and chart SVG (Stylo / sandbox 1990s-era output).

## 2. Frontend structure
- Hardcoded markup for: priority metrics, full sales-chart SVG, assets table with 6 fully-baked rows (`Tropical Villa Bali` / `Beachfront Resort Plaza` / `Modern Apartment NY` etc.), "Show All" button.
- HTMX endpoints used: none meaningful — the file references `developer-dashboard.js` and global UI helpers but contains no `hx-get` / `hx-post` (it predates the HTMX fragment endpoints).
- Vanilla JS: same scripts the live page used to load; none of them mount because the file is never served.
- Inline `<script>`: minimal.
- Anti-patterns: hardcoded asset names + dollar amounts + funding percentages (e.g. `83% funded`, `7.7%` conversion, `11,599` views — all baked into the HTML). Also has duplicate `else` text inside `style="..."` attributes (e.g. `dashboard.html.bak:6608-6620`) suggesting it's the output of a broken template-extraction tool.

## 3. Backend wiring
None. Verified via:
- `grep -rn "dashboard\\.html\\.bak\\|dashboard\\.bak" .` — **0 references** anywhere in `backend/src/`, `tests/`, or `frontend/` templates.
- `backend/src/developer/routes.rs:258` loads `"developer/dashboard.html"` (not `.bak`). MiniJinja's default file loader (`crate::common::templates`) does not enumerate `.bak` extensions.
- No mount in `backend/src/developer/mod.rs:23-188`.

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| (none in production) | — | — | n/a — file is orphaned |

## 4. Data realism
- **Zero real data.** Every number, name, image src, and percentage is hardcoded copy-from-a-design.
- Sample: `dashboard.html.bak:6613` `<span class="table__cell-text-value">11,599</span>`; `:6632` `<span class="table__progress-text">83% funded</span>`; `:6622` `<span class="table__cell-text-value">7.7%</span>`.

## 5. Error & empty states
- N/A — file is never rendered.

## 6. Mobile & responsive
- N/A — file is never rendered.

## 7. Tests
- None. No test loads or references `.bak`.

## 8. Functional gaps & dead code
- **The entire file is dead code (6,700 lines).** Recommend: `git rm frontend/platform/developer/dashboard.html.bak`.
- Contains malformed inline attributes (`else` keywords leaked into HTML), e.g. `:6609-6611`, `:6617-6620` — evidence this was produced by a buggy formatter and was never valid HTML even at the time.

## 9. Production blockers (severity)
- **Critical:** none — the file is not served.
- **High:** none.
- **Medium:** none.
- **Low:** clutter — 6,700 lines of stale code in the developer page tree increases grep noise and risks a junior dev copying the hardcoded asset rows into a live template. Delete the file.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | N/A | File deleted 2026-05-19. |
| Backend wiring | N/A | File deleted 2026-05-19. |
| Data realism | N/A | File deleted 2026-05-19. |
| Error/empty states | N/A | File deleted 2026-05-19. |
| Mobile/responsive | N/A | File deleted 2026-05-19. |
| Tests | N/A | File deleted 2026-05-19. |
| Polish (a11y, i18n, perf) | N/A | File deleted 2026-05-19. |
| **TOTAL** | **N/A** | File removed from working tree. Audit kept as historical record. |
