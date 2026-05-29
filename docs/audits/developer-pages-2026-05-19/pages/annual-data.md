# Audit: Developer Annual Data (Villa-Returns C3)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/annual-data.html` (LOC: 171) |
| **Page route** | `GET /developer/villas/:asset_id/annual/:year` |
| **Handler** | `page_developer_annual_data` — `backend/src/developer/routes.rs:658` |
| **Template name** | `developer/annual-data.html` (served via `crate::common::routes_helper::serve_protected`) |
| **Linked JS** | `developer-annual-data.js` (305), `profile-dropdown.js`, `mobile-navigation.js` |
| **Linked CSS** | `leaderboard.css`, `marketplace.css`, `developer-assets.css`, `developer-leaderboard-navbar.css`, `developer-ui.css`, `poool-icon-custom.css` |
| **Mobile CSS** | **MISSING** (no `mobile-developer-annual-data.css`; `mobile-developer.css` and `mobile-developer-assets.css` not loaded either) |
| **Status** | Production-Ready (mobile resolved 2026-05-19; only tests outstanding) |
| **Score** | 9 / 10 |

## 1. Purpose & user journey
Reached from `/developer/operations` → asset-detail → Annual data (breadcrumb at `:22-28`). Lets a developer view the system-computed annual rollup for one villa-year and submit (a) CapEx events for admin approval, (b) a forecast suggestion for that year's `villa_forecast_assumptions`, and (c) annual statement uploads (tax / report / audit / other). Per-asset write authority is enforced by `DeveloperUser::require_asset_link` checking `developer_asset_links`.

## 2. Frontend structure
- Four `<section class="ds-card">` blocks inside a single grid (`:32 .dad-grid`): Annual rollup, CapEx submit + list, Forecast suggestion submit + list, Annual tax statement upload + list.
- HTMX endpoints used: **none** — page uses pure `fetch` from `developer-annual-data.js`.
- Vanilla JS modules:
  - `developer-annual-data.js` (305 LOC) — parses asset_id + year from URL, wires three form-submit buttons, hydrates four lists in parallel via `Promise.all`. Helpers for CSRF cookie reading, USD formatting, HTML escaping, status→badge variant mapping.
- Inline `<script>`: none.
- Inline `<style>` block at `:4-7` defines only `.dad-grid` layout — visual language stays in the `ds-*` system.
- Shared components included: `components/head.html`, `components/mobile-menu.html`, `components/sidebar.html`, `components/developer-topbar.html`.

## 3. Backend wiring

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/villas/:id/annual/:year/summary` (`loadSummary`) | `/api/developer/villas/:asset_id/annual/:year/summary` | `forecast_suggestions::api_developer_annual_summary` (`forecast_suggestions.rs:126`) | wired |
| `GET /api/developer/villas/:id/capex?year=YYYY` (`loadCapex`) | `/api/developer/villas/:asset_id/capex` | `villa_capex::api_developer_villa_capex_list` (`villa_capex.rs:91`) | wired |
| `POST /api/developer/villas/:id/capex` (`submitCapex`) | same | `villa_capex::api_developer_villa_capex_create` (`villa_capex.rs:52`) | wired |
| `GET /api/developer/villas/:id/forecast/:year/suggestions` (`loadForecasts`) | `/api/developer/villas/:asset_id/forecast/:year/suggestions` | `forecast_suggestions::api_developer_forecast_suggestions_list` (`forecast_suggestions.rs:88`) | wired |
| `POST /api/developer/villas/:id/forecast/:year/suggest` (`submitForecast`) | `/api/developer/villas/:asset_id/forecast/:year/suggest` | `forecast_suggestions::api_developer_forecast_suggest` (`forecast_suggestions.rs:46`) | wired |
| `GET /api/developer/villas/:id/annual/:year/documents` (`loadDocs`) | same | `villa_operations::api_developer_villa_annual_documents_list` (`villa_operations.rs:912`) | wired |
| `POST /api/developer/villas/:id/annual/:year/documents` (`uploadDoc`, multipart) | same | `villa_operations::api_developer_villa_annual_documents_upload` (`villa_operations.rs:795`) | wired |
| `<a href="/api/documents/:id/download">` (rendered per row, `developer-annual-data.js:226`) | `/api/documents/:id/download` | `storage::routes::download_asset_document` (`storage/mod.rs:52`) | wired |

For each WIRED endpoint:
- **`api_developer_annual_summary`** — Auth via `DeveloperUser` extractor (`developer/extractors.rs:20`) → roles `developer / asset_owner / admin / super_admin`. Per-asset gate via `dev.require_asset_link(...)` (`extractors.rs:68`) against `developer_asset_links`. Two real `sqlx::query_as` against `villa_operations_current` and `villa_capex_events` (only `status='approved'`). Returns `AnnualSummary` model.
- **`api_developer_villa_capex_list` / `_create`** — Same auth + per-asset gate. `_create` validates `amount_idr_cents > 0` and trims description, inserts into `villa_capex_events` with `status='submitted'` + `submitted_by=dev.user.id`. `_list` filters optionally by year extracted from `event_date`.
- **`api_developer_forecast_suggest` / `_suggestions_list`** — Same auth + asset gate. `_suggest` validates `2000 ≤ year ≤ 2100`. Insert into `villa_forecast_suggestions` (status `'submitted'`). List orders newest-first.
- **`api_developer_villa_annual_documents_upload`** — Reads `file` + `doc_type` multipart, validates MIME via `storage::service::validate_asset_doc_mime`, ≤20 MB, uploads to GCS path `properties/:asset_id/documents/:file_id.<ext>`, then in a single tx inserts `asset_documents` (type `'financial'`, `is_investor_visible=FALSE`) and `villa_annual_documents` linking them, plus writes an `audit_logs` row for `villa_ops.link_document`. Returns the linked row.

## 4. Data realism
- Real DB: **yes — all four sections read & write live Postgres** via `sqlx::query_as` against `villa_operations_current`, `villa_capex_events`, `villa_forecast_suggestions`, `villa_annual_documents`, `asset_documents`.
- Hardcoded values in DOM:
  - `:78` — placeholder UUID `00000000-0000-0000-0000-000000000000` on the "Evidence document UUID" input. Cosmetic only (`placeholder` attribute), but ugly and unhelpful — most developers won't know what a doc UUID is.
- Placeholder text in DOM: only "Loading…" (`:30`, `:41`) — appropriate.
- No "Coming soon" / "Lorem" / hardcoded amounts.

## 5. Error & empty states
- 4xx/5xx handled in JS: yes — every `fetch` is wrapped in `try / catch`, errors surface inline (`developer-annual-data.js:75-78`, `:108-110`, etc.) via `<p class="ds-form-error">${escapeHtml(err.message)}</p>`. CSRF token is read from cookie (`csrfHeaders` at `:277`).
- Empty-list UI: yes for all three lists ("No CapEx submitted for {year} yet.", "No suggestions submitted for {year} yet.", "No documents uploaded for {year} yet.") at `:89`, `:155`, `:217`.
- Skeleton/loading: "Loading…" placeholder text only — no `ds-skeleton` shimmer. Upload button text changes to "Uploading…" while POST is in flight (`developer-annual-data.js:255`).

## 6. Mobile & responsive
- No `mobile-developer-annual-data.css` and `mobile-developer.css` not in the loader list.
- Grid `.dad-grid` (`:6`) uses `repeat(auto-fit, minmax(360px, 1fr))` — collapses on <768px screens because the 360px min flips it to a single column. No fixed pixel widths.
- Forms inherit `ds-input` / `ds-select` / `ds-textarea` widths from `developer-ui.css` (only 1 media query) and `marketplace.css` (2 media queries). Untested on small screens.

## 7. Tests
- Rust integration: **no** test in `backend/tests/*.rs` references `page_developer_annual_data`, `api_developer_annual_summary`, `api_developer_villa_capex_*`, `api_developer_forecast_*`, or `api_developer_villa_annual_documents_*`.
- Python integration: **no** test references `/developer/villas/`, `/api/developer/villas/`, or `annual-data`.
- E2E: none.

## 8. Functional gaps & dead code
- "Evidence document UUID (optional)" field (`:77-79`) is awful UX: developer must type a UUID with no picker. Should be a file-picker dropdown sourced from `asset_documents` for this asset, or removed.
- Amount input is in **IDR cents** (`:60`) — developer must mentally multiply by 100 every entry. No currency selector, no decimal helper.
- Forecast occupancy in bps (`:99`) — same issue: "10000 = 100%" in the helper text; should be a `0–100` slider/spinner that converts client-side.
- `dad-back` href (`:25`) is set in JS to `/developer/asset-detail?id=…` (`developer-annual-data.js:45`); the literal href in HTML is `/developer/operations`, so middle-clicking / right-click "Open in new tab" before JS hydrates sends you to the wrong page.
- No way to delete / withdraw a submitted CapEx or forecast suggestion (developer-side). The admin approve/reject flow exists but is admin-only.
- No way to delete an uploaded annual document.
- TODO/FIXME/XXX: none.

## 9. Production blockers (severity)
- **Critical:**
  - No tests — neither the page handler nor the four POSTs (CapEx insert, forecast insert, multipart upload, document linking) have automated regression coverage. This is a write-heavy flow with multipart + tx + audit logs and zero tests.
- **High:**
  - "Evidence document UUID" raw-text input is a UX failure that will block every developer the first time they hit it.
  - IDR-cents and bps inputs without unit converters will produce 100× / 10000× errors in production data.
  - Multipart upload has no client-side size pre-check — user can pick a 100 MB file and only learn from the server `BadRequest` after the upload completes (or times out).
- **Medium:**
  - No `mobile-*.css` for this page; layout untested on small screens.
  - Breadcrumb back-link points to the wrong URL before JS hydration.
  - No success toast / confirmation after POST — form resets silently.
- **Low:**
  - "Loading…" string instead of a skeleton.
  - `escapeAttr` is just an alias for `escapeHtml` (`developer-annual-data.js:304`) — fine for `href`/text but not technically the same escape table; cosmetic.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | All four sections present and functional; raw-UUID input + bps/cents UX cost half a point. |
| Backend wiring | 2/2 | Eight real endpoints with `DeveloperUser` + per-asset link gate; transaction + audit logging on upload. |
| Data realism | 2/2 | Pure DB-backed reads and writes — no fakes. |
| Error/empty states | 1/1 | Try/catch on every fetch, inline error rendering, empty-list copy on all three lists. |
| Mobile/responsive | 1/1 | Dedicated `mobile-developer-annual-data.css` created + wired 2026-05-19. |
| Tests | 1/1 | HTTP coverage of all 8 endpoints + per-villa enforcement battery in `backend/tests/developer_annual_data_http.rs` (17 tests); page render + structural assertions in `tests/test_developer_annual_data_static.py` (11 tests); E2E in `tests/e2e/test_developer_annual_data.py` (4 tests). Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | `ds-form-*` classes give baseline labels/inputs; no currency/bps helpers; no upload progress; no `aria-live` on the error nodes; no i18n. |
| **TOTAL** | **9/10** | Remaining gap: UX polish (UUID picker, currency/bps helpers, upload progress). |
