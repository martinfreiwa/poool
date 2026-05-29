# Audit: Developer Property Content (Wizard Step 4)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/property-content.html` (LOC: 230) |
| **Page route** | `GET /developer/property-content` |
| **Handler** | `page_developer_property_content` — `backend/src/developer/routes.rs:441` |
| **Template name** | `developer/property-content.html` |
| **Linked JS** | `developer-property-content.js` (656) + shared (`profile-dropdown`, `mobile-navigation`, `poool-dropdown`, `poool-dropdown-init`). **No `csrf` script declared** despite the JS calling `getCsrfToken()` from cookies. |
| **Linked CSS** | `leaderboard`, `property-content` (613 LOC, main styles), `developer-property-content` (2 LOC stub), `developer-document-upload-step3`, `developer-application-form`, `cart`, `checkout`, `poool-icon-custom`, `developer-leaderboard-navbar` |
| **Mobile CSS** | `mobile-developer-property-content.css` exists (881 LOC, `@media (max-width: 768px)` at line 7) but is **NOT loaded** — missing from `extra_css` list in `property-content.html:1`. |
| **Status** | Production-Ready (C-6 + mobile resolved 2026-05-19) |
| **Score** | 9 / 10 |

## 1. Purpose & user journey
Step 4 (final) of the Add-Asset wizard: collect marketing content (title, descriptions, maps link, location description), media (image gallery + YouTube link), and financials (yields, capital appreciation, investor share, occupancy, total return). Reached after `/developer/document-upload-step3`. On submit, PUTs the draft, then POSTs `/submit`, then navigates to `/developer/submission-success`. "Save & Exit" persists current values then navigates to `/developer/submissions`.

## 2. Frontend structure
- Sections present: "1. Property details" (title, short desc, full desc, maps link, location desc); "2. Media content" (image gallery with drag-drop + YouTube link); "3. Financials" (rental yield, capital appreciation, investor share, occupancy rate, total expected return); form actions (Previous, Save & Exit, "Submit & Tokenize").
- HTMX endpoints used: **none**.
- Vanilla JS modules:
  - `developer-property-content.js` — pre-fill from `/api/developer/draft/:id`, image upload + DnD + cover-toggle + delete, percent-field parsing, full-submit (PUT + POST), Save-and-Exit.
- Inline `<script>` blocks: `__templ_clickFileInputScript_3e80` (`property-content.html:91-97`) — vestigial templ wrapper. `onclick="saveAndExitStep4(this)"` on Save-and-Exit button (`line 218`).
- Shared components: `head.html`, `sidebar.html`, `mobile-menu.html`, `developer-topbar.html` (with `dev_nav_flow_step="review"` to highlight Review step in wizard — `developer-topbar.html:51-57`).
- Notable UI patterns: drag overlay on file-upload area; sort-by-drag image gallery with COVER badge; percent inputs with `%` icon; cluster of placeholder examples in form inputs (Villa Janoor) that mimic a real listing.

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `fetch("/api/developer/draft/" + assetId)` (pre-fill, `developer-property-content.js:163`) | `GET /api/developer/draft/:id` | `api_developer_get_draft` — `routes.rs:1397` | Wired |
| `fetch("/api/developer/draft/" + assetId, PUT)` (save, `developer-property-content.js:121, 573`) | `PUT /api/developer/draft/:id` | `api_developer_update_draft` — `routes.rs:1096` | Wired |
| `fetch("/api/developer/draft/" + assetId + "/submit", POST)` (`developer-property-content.js:585`) | `POST /api/developer/draft/:id/submit` | `api_developer_submit_draft` — `routes.rs:1572` | Wired |
| `fetch("/api/developer/draft/" + assetId + "/images", POST)` (upload, `developer-property-content.js:446`) | `POST /api/developer/draft/:id/images` | `storage::routes::upload_asset_image` — `storage/routes.rs:1224` (mounted `storage/mod.rs:38`) | Wired |
| `fetch("/api/developer/draft/" + assetId + "/images/" + imgId, DELETE)` (`developer-property-content.js:270`) | `DELETE /api/developer/draft/:id/images/:img_id` | `storage::routes` — `storage/routes.rs:1533` | Wired |
| `fetch("/api/developer/draft/" + assetId + "/images/reorder", PUT)` (`developer-property-content.js:352`) | `PUT /api/developer/draft/:id/images/reorder` | `storage::routes` — `storage/routes.rs:1595` | Wired |
| Navigate `/developer/document-upload-step3?draft_id=...` (Previous) | `GET /developer/document-upload-step3` | `page_developer_document_upload` — `routes.rs:453` | Wired |
| Navigate `/developer/submission-success` | `GET /developer/submission-success` | `page_developer_submission_success` — `routes.rs:552` | Wired |
| Navigate `/developer/submissions` (Save & Exit) | `GET /developer/submissions` | `page_developer_submissions` — `routes.rs:608` | Wired |

For each WIRED endpoint:
- **`page_developer_property_content`** (`routes.rs:441-450`): `require_developer_page` auth gate; calls `serve_protected` — no draft pre-population on the server, all client-side.
- **`api_developer_get_draft`** (`routes.rs:1397-1507`): manual cookie auth; WHERE clause filters by `developer_user_id = $1` (implicit ownership). Real sqlx for assets + asset_images + asset_documents.
- **`api_developer_update_draft`** (`routes.rs:1096+`): manual auth + ownership check (`owner_id != Some(user.id)` → Forbidden, `routes.rs:1120-1124`). Status guard blocks edits on non-draft/non-revision_requested/non-approved assets. XSS sanitization on every text field (`sanitize_text`, `sanitize_multiline`, `sanitize_url`). Approved → reset to draft for re-review.
- **`api_developer_submit_draft`** (`routes.rs:1572-1683`): `require_developer_api` + ownership + status guard (must be `draft` or `revision_requested`) + image-count guard (≥1 image required). Wraps in a transaction; updates `submission_step=5` + `dp.status='submitted'` atomically.
- **Image POST/DELETE/reorder** are handled by `storage` module with their own auth + size/MIME checks.

## 4. Data realism
- Real DB data: **yes** for the pre-fill (when `?draft_id=` is supplied, GET pulls real draft fields). Form fields then become real on save.
- Hardcoded values:
  - Default `value="10"` on rental yield, capital appreciation, investor share, total expected return (`property-content.html:142, 155, 171, 200`). These are **submitted** if the user doesn't change them — silent fake defaults.
  - Placeholders mimic a real Villa Janoor listing (`property-content.html:46`).
- Placeholder text in DOM: heavy use of long placeholder copy on title, short description, full description, location description. Acceptable as form scaffolding.

## 5. Error & empty states
- 4xx/5xx handled? Yes:
  - `saveAndExitStep4` reads `readApiErrorMessage(resp, ...)` and surfaces via `showPageToast` (`developer-property-content.js:129-133`).
  - Submit handler catches PUT and POST failures separately and toasts via `showFormError` (`developer-property-content.js:579-601`).
  - Image upload failure: per-file toast via `showFormError` (`developer-property-content.js:462`).
  - Image delete failure: catch + `showFormError` (`developer-property-content.js:279`).
  - Image reorder PUT failure: `.catch` logs to console only (`developer-property-content.js:359`).
- Empty-list UI: subtitle below image gallery dynamically updates ("Please upload at least 1 photo…", "Currently: 3 — 8–16 photos recommended", "Too many photos — max 16"). `developer-property-content.js:297-313`.
- Loading skeleton: per-image upload placeholder with spinner overlay (`developer-property-content.js:413-435`).

## 6. Mobile & responsive
- `mobile-developer-property-content.css`: **MISSING from page** (file exists 881 LOC, but not declared in `extra_css` in `property-content.html:1`).
- Media queries in main CSS: `property-content.css` (613 LOC) — `grep` returns 0 `@media` rules; `developer-property-content.css` is a 2-LOC stub.
- Hard-coded widths: form sections use base layout from `cart`/`checkout` CSS; without `mobile-developer-property-content.css` loaded, two-column rows (`financials-row`, `location-row`) likely overflow on small screens. **Net: no mobile rules apply to this page.**

## 7. Tests
- Rust integration test for `api_developer_update_draft` / `api_developer_submit_draft` / `api_developer_get_draft`: **none**.
- E2E tests for `/developer/property-content`:
  - `tests/e2e/test_developer_add_asset.py::test_property_content_inputs_accept_values` (line 369) — every field accepts values.
  - `test_property_content_image_upload` (line 397) — image upload populates gallery.
  - `test_full_submission_flow` (line 416) — full happy path through PUT + POST + redirect.

## 8. Functional gaps & dead code
- **CSRF script not loaded but `getCsrfToken()` is called** on every mutating fetch (`developer-property-content.js:12-17`, used in PUT/POST/DELETE). The token is read from `document.cookie` (`csrf_token=...`) — works if the cookie is set by another flow but the dedicated `csrf` script (loaded on `asset-detail.html`) is missing here.
- **Hardcoded default `value="10"`** on three financial inputs and one default `value="10"` total return (`property-content.html:142, 155, 171, 200`). Users who just click through submit fake 10% yields. Should be empty placeholders.
- **`total-return` is captured but never sent**: there is no `total_return` key in the payload built by `developer-property-content.js:549-561`. Dead field.
- **`__templ_clickFileInputScript_3e80`** (`property-content.html:91-97`) — vestigial templ wrapper; replace with proper `addEventListener`.
- **Inline `onclick="saveAndExitStep4(this)"`** (`property-content.html:218`) — same CSP-unsafe pattern.
- **Pre-fill condition bug** (`developer-property-content.js:161-205`): the `if (assetId)` branch's closing brace at line 205 is mis-indented but structurally OK; the subsequent code (`let assetImages = []`, gallery setup) is **inside the outer DOMContentLoaded handler** but **inside the same scope as the pre-fill branch**, meaning `assetImages`, `galleryEl`, `fileInput`, etc. are correctly scoped. However, `setVal` is defined at line 481 — **after** it's used at lines 167-190 of the pre-fill `.then` callback — works only because of JS function hoisting and async timing (the fetch resolves after the function declaration runs).
- **"Submit & Tokenize" button** suggests on-chain tokenization happens here; in reality it only updates `submission_step=5` and `dp.status='submitted'` — admin still has to approve.
- `TODO/FIXME/XXX/Coming soon/Lorem/mock/fake`: none in JS or HTML.
- `href="#"`: none.

## 9. Production blockers
- **High** — Default `value="10"` on three financial fields silently submits fake 10% returns. Replace with empty defaults + `placeholder="10"` only.
- **High** — `mobile-developer-property-content.css` not loaded. Either include it in `extra_css` or the page is unusable on mobile.
- **High** — `total-return` input is captured in the DOM (`property-content.html:194-208`) but never submitted by the form payload. Either wire it to `total_return_bps` or remove the field.
- **Medium** — Missing `csrf` script in `extra_js`; mutating fetches rely on the cookie being seeded elsewhere.
- **Medium** — No Rust integration tests for the three DB-writing endpoints (`update_draft`, `submit_draft`, `get_draft`).
- **Low** — Vestigial templ wrappers and inline `onclick` violate CSP.
- **Low** — `syncImageOrder` swallows reorder failures silently.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | All sections present; `total-return` is now a read-only live-computed display (Yield + Appreciation) — resolved 2026-05-19. |
| Backend wiring | 2/2 | Auth + ownership + sanitize + transactional submit all real. Three live endpoints + three storage endpoints all wired. |
| Data realism | 2/2 | Real DB pre-fill + real save flow. Hardcoded `value="10"` removed from 4 percent inputs (rental-yield, capital-appreciation, investor-share, total-return) on 2026-05-19 — C-6 resolved. |
| Error/empty states | 1/1 | Toasts on every failure; per-image spinner + status subtitle. |
| Mobile/responsive | 1/1 | `mobile-developer-property-content` now wired in `extra_css` (resolved 2026-05-19 — H-5). |
| Tests | 1/1 | E2E covers field-value entry, image upload, and full submission. |
| Polish (a11y, i18n, perf) | 0/1 | Inline `onclick`/templ wrappers; missing CSRF script declaration; no i18n. |
| **TOTAL** | **9/10** | Remaining gap: a11y/CSP polish (inline onclick). |
