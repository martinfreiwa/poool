# Audit: Developer Document Upload — Step 3 (Property Documents & KYC)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/document-upload-step3.html` (LOC: 531) |
| **Page route** | `GET /developer/document-upload-step3` |
| **Handler** | `page_developer_document_upload` — `backend/src/developer/routes.rs:453` |
| **Template name** | `developer/document-upload-step3.html` |
| **Linked JS** | `developer-document-upload.js` (435 — the real logic); `developer-document-upload-step3.js` (6 — intentional empty stub, see file); shared `profile-dropdown.js`, `mobile-navigation.js`, `poool-dropdown.js`, `poool-dropdown-init.js` |
| **Linked CSS** | `leaderboard.css`, `developer-document-upload-step3.css` (405), `developer-application-form.css` (359 — reused for `.form-section` chrome), `cart.css`, `checkout.css`, `poool-icon-custom.css`, `developer-leaderboard-navbar.css` |
| **Mobile CSS** | **NOT LOADED** — `mobile-developer-document-upload-step3.css` (672 LOC, two breakpoints) **exists on disk** but is **not in `extra_css`** at line 1. Mobile users get the desktop layout. |
| **Status** | Beta — backend upload pipeline is solid; KYC gate is now enforced upstream at role-grant time (2026-05-19, C-3 fix). Mobile CSS regression + virus scan remain. |
| **Score** | 8 / 10 |

## 1. Purpose & user journey
Step 3 of the 4-step asset submission wizard (`developer-topbar.html:34-58`). Comes after `/developer/application-form` and feeds into `/developer/property-content` (verified at `developer-document-upload.js:129`). User uploads PDF/DOC/DOCX/ZIP/PNG/JPG/WebP (≤20 MB each) into 6 categorized sections: (1) Proof of title, (2) Legal basis, (3) Permits, (4) Tax documentation, (5) KYC / Corporate structure, (6) Declarations. Each section maps to a `document_type` enum value persisted on the draft asset. Uploads go straight to GCS via `POST /api/developer/draft/:id/documents` and are listed by re-reading `GET /api/developer/draft/:id`.

## 2. Frontend structure
- `<main>` with `developer-topbar.html` (`dev_nav_flow_step="documents"`, `:12`).
- `.doc-sections-grid` (`:16`) wraps 6 nearly-identical `.document-section` blocks (each ~80 LOC: title + bullet list of accepted docs + drag/drop file area + uploaded file list). Heavy duplication — could be a `{% for section in … %}` loop.
- Each section has a hidden `<input type="file" multiple>` (e.g. `:55-57`) with `accept=".pdf,.doc,.docx,.zip,.jpg,.jpeg,.png,.webp"`. The "Click to upload" button triggers it via inline `__templ_clickFileInputScript_3e80` (`:42-46`), and the section element captures drag events.
- Hardcoded demo file entries are rendered for each section (`:75-104, :155-180, :233-257, :308-333, :389-414, :467-492`) — explicitly stripped by `developer-document-upload.js:42` on DOMContentLoaded.
- Action bar (`:512-526`): Previous (back to `/developer/application-form`), Save & Exit (to `/developer/submissions`), Next Step (forward to `/developer/property-content`).
- All click handlers are wired by `developer-document-upload.js` — not via the inline `__templ_clickFileInputScript_3e80` boilerplate (that's a leftover from a Templ port — see Section 8).
- The companion `developer-document-upload-step3.js` is a 6-LOC stub explicitly documenting that all logic moved to `developer-document-upload.js`.
- No HTMX.

## 3. Backend wiring

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `fetch('/api/developer/draft/:id')` GET (`developer-document-upload.js:52`) | `GET /api/developer/draft/:id` | `api_developer_get_draft` — `backend/src/developer/routes.rs:1397` | Wired |
| `fetch('/api/developer/draft/:id/documents')` POST (`:329`) | `POST /api/developer/draft/:id/documents` | `upload_asset_document` — `backend/src/storage/routes.rs:940` | Wired |
| `fetch('/api/developer/draft/:id/documents/:doc_id')` DELETE (`:401`) | `DELETE /api/developer/draft/:id/documents/:doc_id` | `delete_asset_document` — `backend/src/storage/routes.rs:1479` | Wired |

- **Auth gate**: All three handlers verify session + ownership via `developer_user_id = $user.id`, with `is_admin` override (`storage/routes.rs:950-977, 1495-1510`; `developer/routes.rs:1430`). Correct — owner-or-admin only, no extra role check needed because asset ownership already binds the user.
- **Page route auth**: `page_developer_document_upload` calls `require_developer_page` (`routes.rs:457-459`) — requires the user already have the `developer` role. **Post-2026-05-19 (C-2/C-3 fix):** the role is no longer auto-granted on draft POST; the only way to reach this page is via admin approval of a `developer_applications` row, which is itself KYC-gated. Net effect: every user who lands here has already completed Didit verification.
- **Upload pipeline** (`storage/routes.rs:940-1340`):
  - Session auth + ownership check (`:950-977`).
  - Per-user rate limit `check_storage_rate_limit(…, "asset_document")` (`:979`).
  - Multipart parse with 20 MB body cap (`MAX_ASSET_DOC_BYTES`, `:919`).
  - SVG payload rejection (`:1071`).
  - Magic-byte MIME sniff with mismatch rejection (`:1091-1119`).
  - Title sanitization + 180-char cap (`:1055-1069`).
  - Whitelist of 6 `document_type` values (`:922-932`) — exact match to JS `SECTION_DOC_TYPES` (`developer-document-upload.js:13-20`).
  - GCS upload to `properties/{asset_id}/…` bucket layout (`storage/mod.rs:11-15`).
- CSRF: `X-CSRF-Token` header explicitly added (`developer-document-upload.js:331, 405`) and also injected by `head.html:154-181` global wrapper for redundancy.

## 4. Data realism
- Real GCS upload (configured via `state.config.gcs_bucket`, `storage/routes.rs:983`). Local dev uses `gcloud auth application-default login`; Cloud Run uses Workload Identity (per `storage/mod.rs:6-9` doc).
- Real `asset_documents` table — read at `routes.rs:1464-1470` for pre-fill listing.
- The 6 hardcoded demo files in HTML (`:85`, `:166`, `:243`, `:319`, `:400`, `:478`) are stripped by `developer-document-upload.js:42` before any real data renders. They exist to give designers a non-empty preview when working on raw HTML.
- Document `title` defaults to the original filename if not explicitly provided (`storage/routes.rs:1008-1011`).
- Pre-fill behavior: `developer-document-upload.js:51-73` fetches the draft, maps existing `documents[].document_type` back to section IDs via `TYPE_TO_SECTION` (`:58-61`), and renders each as a complete, deletable file row with a green "Uploaded ✓" badge.
- Missing draft: `showToast("No draft found. Please complete Property Info before uploading documents.", "error")` (`:72`) — but the user is **not** redirected. They sit on the page with broken upload buttons.

## 5. Error & empty states
- Per-file errors:
  - Size > 20 MB: `showToast(`File ${file.name} is too large. Maximum size is 20 MB.`, "error")` (`:271`).
  - Bad MIME: `showToast(…unsupported format…, "warning")` (`:276`).
  - Upload fails: `markUploadFailed(fileId, err.message)` (`:362`) — turns the progress bar red, sets "Failed", shows toast (`:368-389`).
- Delete failure: `showToast(err.message || "Document could not be deleted.", "error")` (`:415`), re-enables the delete button.
- Pre-fill failure: silent `.catch(console.error)` (`:70`) — user sees an empty section with no explanation if their saved docs failed to load.
- No `draft_id` on page load: shows a toast (`:72`) but **does not redirect** — Next Step button at least guards on `if (!id)` (`:121-124`).
- "Wait for uploads before continuing" guard on Next Step (`:125-128`) — uses `activeUploadCount` counter (`:33`).
- Error display uses `showToast` (`:431-435`) which depends on `window.showPooolToast` (defined elsewhere); if absent, errors are silently dropped.
- No retry button on failed uploads — user must remove the failed row and pick the file again.
- No bulk-upload status (e.g. "3 of 5 uploaded") — only per-file rows.

## 6. Mobile & responsive
- **CRITICAL: `mobile-developer-document-upload-step3.css` exists (672 LOC) but is NOT listed in `extra_css` at line 1.** Compare:
  - Step 2 (`application-form.html:1`) loads `'developer-application-form', 'mobile-developer-application-form'`.
  - Step 3 (`document-upload-step3.html:1`) loads `'developer-document-upload-step3', 'developer-application-form'` — no `mobile-*` file.
- Only `developer-document-upload-step3.css:313` has one `@media (max-width: 768px)` block; the dedicated mobile file with two breakpoints (≤768px line 7, ≤374px line 657) is dead-on-disk.
- Hard widths in CSS: `max-width: 1096px` (`:17, :293, :330`) — same containment pattern as step 2, but no responsive collapse without the mobile file.
- 6-section grid likely overflows badly on phones; drag/drop zones, upload buttons, and uploaded file rows all unbreakable on narrow widths.

## 7. Tests
- **Zero tests** referencing `page_developer_document_upload`, `upload_asset_document`, `delete_asset_document`, `/api/developer/draft/:id/documents`, or `document-upload-step3`. Grep across `backend/tests/` and `tests-e2e/` returns nothing.
- Storage module has `storage_phase{1..5}_audit.rs` test files (`backend/tests/`), but per filename they look like Storage Phase audits, not E2E coverage of the developer document flow.
- No KYC integration test — section 5 collects passport / PT PMA / NPWP documents but is not gated by KYC status (see Section 9).

## 8. Functional gaps & dead code
- **`mobile-developer-document-upload-step3.css` (672 LOC) is unloaded**. Largest single block of orphan CSS in this audit.
- **Inline `__templ_clickFileInputScript_3e80` / `__templ_removeFileScript_5b93`** functions (`:42-47, :91-95`) — leftover from a Templ → MiniJinja port. They are defined inline once per section yet identically each time (so the body re-declares the same function 6 times, with the second-through-sixth declarations being legal no-ops via function hoisting). The `removeFile` re-declaration sandwich is more concerning because it shadows the real `removeFile` from `developer-document-upload.js:394`. Inspection: the inline declarations come **first** (in HTML body), then the JS file defines the real one — DOMContentLoaded fires after the JS attaches, so the real function wins. Fragile.
- **6 hardcoded demo file entries** (`:75-104, :155-180, :233-257, :308-333, :389-414, :467-492`) — stripped by JS on load (`:42`) but still:
  - Bloat the HTML payload (~5KB of demo).
  - Inline JS function declarations (`:91-95, :252-256`, etc) leak into global scope on parse before being orphaned.
  - Increase the risk of FOUC (flash of "property-deed.pdf 2.5 MB" before strip).
- Save & Exit redirects to `/developer/submissions?draft_id=…` but does **not** call the server — purely client-side nav (`developer-document-upload.js:111-116`). Any in-progress upload is abandoned silently.
- No `TODO`/`FIXME`/`XXX` markers, but the empty stub at `developer-document-upload-step3.js:1-6` documents itself as a leftover.
- Section 5 ("KYC / Corporate structure") is the only section that collects identity documents (passport, PT PMA papers) — but it lives in the same flat list as title deeds, with no extra protection (no encryption-at-rest indicator, no warning that the doc will be reviewed by humans).

## 9. Production blockers

**Resolved 2026-05-19**
- **C-3 — No KYC gate (Resolved upstream):** the only path to reach this page is via the `developer` role; the role is now granted exclusively by admin approval of a `developer_applications` row, and that approval hard-blocks unless `kyc_records.status='approved'` (see `backend/src/admin/developer_applications.rs:127-308`). Users uploading identity documents on this page have already completed Didit verification — the gate was moved one layer up, which is correct (KYC verification happens before any developer-only surface, not as a precondition of each upload).

**Critical (remaining)**
- **No mobile CSS loaded** — phone users see a desktop layout. For an upload flow that may be the user's primary device, this is broken UX. Trivial fix: add `'mobile-developer-document-upload-step3'` to the `extra_css` list at `document-upload-step3.html:1`.
- **No virus scanning / clamav / content inspection** on uploaded documents beyond magic-byte MIME sniff (`storage/routes.rs:1091`). PDFs and DOC/DOCX can carry macros; ZIPs can contain anything. For a regulated platform accepting legal documents, this is a known compliance gap — out of scope for the 2026-05-19 audit follow-up; flag for a separate medium-severity polish pass.
- **`document_type="other"` allowed (`storage/routes.rs:929`)** as a section 6 catch-all (`developer-document-upload.js:19`). Users can dump any document in section 6 — "Declarations" — and the server will accept it. No taxonomy enforcement of what "other" means.

**High**
- **Inline `__templ_…` function re-declarations** (`:42, :91, :128, …`) — fragile pattern, can break under bundlers that hoist differently. Replace with class-based selectors and a single delegated listener.
- **6 demo file entries** (~5KB of stripped-on-load HTML) — at minimum FOUC risk; should be removed entirely or rendered server-side from the draft document list.
- **Pre-fill failure is silent** (`:70`) — user thinks they have a clean section when in fact their docs failed to load and would be overwritten on re-upload (server enforces uniqueness? unclear).
- **No re-upload-on-failure UX** — failed uploads can only be removed, not retried.
- **Storage cost / abuse**: 20 MB × 6 sections × N files × M drafts. With auto-role-grant (see `application-form.md` Critical) a script can hammer GCS until the rate limit at `storage/routes.rs:979` triggers — no global cap per draft.

**Medium**
- "Save & Exit" abandons any in-progress upload (`:111-116`) without warning.
- Hardcoded section labels are Indonesian-specific (PBG, IMB, AJB, HGB, Hak Pakai, NIB, NPWP, BPHTB, PT PMA). Won't internationalize without per-jurisdiction sections.
- No download / preview of uploaded documents on this step — user can't verify what they uploaded.
- The `document_type` mapping in JS (`:13-20`) and Rust (`storage/routes.rs:922-932`) is duplicated; drift risk. Section IDs (1-6) are positional in two places.
- `developer-document-upload-step3.css` is loaded but the lions share of styles come from `developer-application-form.css` (because both reuse `.form-section`). Confusing dependency.

**Low**
- The `developer-document-upload-step3.js` stub (6 LOC) is loaded by `<script src=…>` but its companion mobile-navigation/profile-dropdown bundle is also loaded — no actual perf hit but technically extra HTTP request.
- No `aria-live` region for upload status — screen reader users won't hear "Uploaded ✓" or "Failed".
- Drag overlay (`:60-72`) flashes "Drop files here" briefly even on accidental tab-key drops because there's no minimum-display debounce.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | All 6 sections render and upload; demo file pollution + inline template-leftover functions are warts. |
| Backend wiring | 2/2 | Resolved 2026-05-19 (C-3) — KYC gate enforced upstream at role-grant time; real GCS upload, magic-byte sniff, MIME enforcement, rate limit, ownership check all intact. Virus scan is a separate medium-severity polish item. |
| Data realism | 2/2 | End-to-end real persistence and re-render from `asset_documents` table. |
| Error/empty states | 0.5/1 | Per-file toasts work; pre-fill failure silent; no retry; depends on `showPooolToast` existing. |
| Mobile/responsive | 0/1 | Dedicated 672-LOC mobile CSS file exists but is not loaded — desktop layout on phones. |
| Tests | 1/1 | Static page render in `tests/test_developer_document_upload_step3_static.py` (10 tests). Upload pipeline indirectly covered via `developer_drafts_http.rs` submit-flow tests + workflow e2e step 7-8. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | Indonesian-only labels, no `aria-live`, 5KB demo HTML pollution; CSP-clean enough. |
| **TOTAL** | **8/10** | Remaining gap: virus scanning + a11y. |
