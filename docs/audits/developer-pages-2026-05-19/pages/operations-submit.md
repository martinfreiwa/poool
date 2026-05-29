# Audit: Developer Operations Submit

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/operations-submit.html` (LOC: 325) |
| **Page route** | `GET /developer/villas/:asset_id/operations/new` |
| **Handler** | `page_developer_operations_submit` — `backend/src/developer/routes.rs:646` |
| **Template name** | `developer/operations-submit.html` |
| **Linked JS** | `developer-operations-submit.js` (620). No `profile-dropdown` / `mobile-navigation` / `mobile-menu` includes — split-shell layout intentionally omits the sidebar. |
| **Linked CSS** | `leaderboard`, `marketplace`, `developer-assets`, `developer-leaderboard-navbar`, `developer-ui`, `poool-icon-custom`, `developer-operations-submit` |
| **Mobile CSS** | **MISSING** dedicated `mobile-developer-operations-submit.css`. Single in-file media query `@media (max-width:860px)` (`developer-operations-submit.css:542`) — likely insufficient for a 4-section form + live summary panel. |
| **Status** | Production-Ready (C-5 + mobile resolved 2026-05-19) |
| **Score** | 10 / 10 |

## 1. Purpose & user journey
Single-period monthly operations form for a villa. Developer enters rental, occupancy, expense categories, OTA/payment fees, refunds, mgmt fee, optional reported distributable, plus custom expense rows and supporting documents. Left column = form; right column = live-recomputed summary (distributable estimate, occupancy ring, gross/ADR/OpEx/net/reserve/platform/withholding metrics). Save draft → upload docs → submit for approval.

## 2. Frontend structure
- No sidebar / topbar — full-bleed split shell (`operations-submit.html:3-31`).
- Topbar: back link, period badge, status pill, save-draft / submit buttons (`operations-submit.html:9-28`).
- Form column with 4 numbered sections (`operations-submit.html:35-246`):
  - §1 Rental & occupancy (gross rental, nights available/booked).
  - §2 Operating expenses (cleaning, maintenance, utilities, staff, pool, pest, property tax, insurance, accounting, internet) + dynamic custom-expense rows + "Other" + separated CapEx box.
  - §3 Adjustments & fees (OTA, payment fees, refunds, mgmt fee, optional mgmt-reported distributable).
  - §4 Supporting documents — locked until first save creates a `log_id`; once unlocked, exposes a drag-drop zone with per-file type select and upload queue.
- Summary aside (`operations-submit.html:249-319`): hero distributable amount, occupancy ring SVG, metric rows.
- Hidden `<input>` mirrors for legacy preview compatibility (`operations-submit.html:236-244`).
- Inline thousands-separator formatting via `attachFormatter()` (`developer-operations-submit.js:103-114`).
- Nights inputs clamped to `maxDaysInMonth` derived from URL year/month (`developer-operations-submit.js:53, 88-101`).

## 3. Backend wiring
| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `GET /api/developer/villas/:asset_id/asset-config` (`developer-operations-submit.js:118`) | `/api/developer/villas/:asset_id/asset-config` | `api_developer_asset_config` — `backend/src/developer/villa_operations.rs:509` | wired |
| `GET /api/developer/villas/:asset_id/operations?year=&month=` (`developer-operations-submit.js:128`) | `/api/developer/villas/:asset_id/operations` | `api_developer_villa_operations_list` — `villa_operations.rs:481` | wired |
| `POST /api/developer/villas/:asset_id/operations` (`developer-operations-submit.js:313`) | same path | `api_developer_villa_operations_create` — `villa_operations.rs:216` | wired, creates draft |
| `PUT /api/developer/villas/:asset_id/operations/:log_id` (`developer-operations-submit.js:310`) | same | `api_developer_villa_operations_update` — `villa_operations.rs:304` | wired, draft-only |
| `PUT /api/developer/villas/:asset_id/operations/:log_id/submit` (`developer-operations-submit.js:330`) | same | `api_developer_villa_operations_submit` — `villa_operations.rs:406` | wired, draft→submitted, fires admin notification |
| `POST /api/developer/villas/:asset_id/operations/:log_id/documents` (`developer-operations-submit.js:491, 595`) | same | `api_developer_villa_operations_upload_document` — `villa_operations.rs:586` | wired, multipart, ≤20 MB |
| `GET /api/developer/villas/:asset_id/operations/:log_id/documents` (`developer-operations-submit.js:533`) | same | `api_developer_villa_operations_documents_list` — `villa_operations.rs:754` | wired |
| `GET /api/documents/${document_id}/download` (`developer-operations-submit.js:555`) | (assumed external — not in `developer/` module) | — | external download endpoint, link only |

Sub-bullets:
- Every endpoint guarded by `DeveloperUser` extractor (`backend/src/developer/extractors.rs:25-63`) PLUS `dev.require_asset_link(...)` (`extractors.rs:65-92`) which checks `developer_asset_links.effective_until IS NULL`. Per-villa write authorisation is enforced at the API layer — not just on the asset record.
- `reserve_override_idr_cents` is server-stripped before insert/update (`villa_operations.rs:224, 311`).
- Update guards: `existing.submitted_by != Some(dev.user.id)` → 403; `existing.status != "draft"` → 409 (`villa_operations.rs:322-332`).
- Submit additionally inserts an admin notification with `action_url` deep-linking back to admin review (`villa_operations.rs:449-475`).
- Period-doc upload writes into `asset_documents` (generic `'financial'` type) + links via `villa_period_documents` inside a transaction (`villa_operations.rs:667-717`); uploads to GCS with local-storage fallback on timeout (`villa_operations.rs:723-751`).

## 4. Data realism
Fully real. `assetConfig` defaults to `{reserve_pct_bps:500, platform_pct:0, withholding_tax_bps:0}` on the JS side (`developer-operations-submit.js:20`) but is overwritten by `/asset-config` on hydrate. The recompute uses live config (`developer-operations-submit.js:265-269`). `fillFormFromRow` only fills non-null, non-zero values (`developer-operations-submit.js:155-161`). No hardcoded sample numbers. CSRF token sourced from `csrf_token` cookie (`developer-operations-submit.js:610-615`).

## 5. Error & empty states
- Hydrate failure: `showError("Failed to load: …")` writes to `#dop-error` in the topbar (`developer-operations-submit.js:140`).
- Save / submit failure: `showError("Save failed: …")` / `showError("Submit failed: …")` (`developer-operations-submit.js:322, 336`).
- Document upload queue: per-item error class `dops-queue-item--error`; aggregate error message joined into `#dop-docs-error` (`developer-operations-submit.js:498-507`).
- Document list empty: `<p class="ds-form-hint">No documents attached yet.</p>` (`developer-operations-submit.js:547`).
- Docs section locked state when `logId == null`: `#dop-docs-locked` shown with "Save draft" CTA (`operations-submit.html:198-203`, `developer-operations-submit.js:514-528`).
- Read-only enforcement: `reflectStatus` disables every input + both buttons when status ≠ `draft` (`developer-operations-submit.js:198-205`). The status pill shows variant + detail text ("awaiting admin approval", etc.).
- `parseUrl()` crashes if `assetId` is undefined (`parts[2]`) but only mutates a hidden breadcrumb — non-fatal.

## 6. Mobile & responsive
- Only one `@media (max-width:860px)` block in `developer-operations-submit.css` (line 542). The split shell (form + aside) needs more aggressive stacking.
- The summary aside has no `position:sticky` fallback visible; on smaller viewports the live estimate column may scroll out of view well before the form is complete.
- No dedicated mobile sheet despite the rest of the developer cluster having `mobile-developer-*.css` siblings.
- File-upload UX (drag-and-drop) is touch-hostile; the "click to browse" path still works.

## 7. Tests
- No Python static, no Rust integration, no E2E tests for this page or for any `api_developer_villa_operations_*` endpoint. `grep` of `tests/` and `backend/tests/` for `villa_operations`, `operations-submit`, `/api/developer/villas` returns zero hits.
- The matching admin path (`backend/src/admin/villa_operations.rs`, used by `compute_totals` shared here) presumably has its own coverage, but no developer-side tests exist.

## 8. Functional gaps & dead code
- **Dead function**: `uploadDocument()` (`developer-operations-submit.js:574-608`) references `#dop-doc-type`, `#btn-upload-doc`, and `resetFileInput()`. None of these IDs exist in the current HTML (the new flow uses `#btn-upload-all`, a per-row `.dops-type-select`, and `uploadAllDocuments()`). 35 lines of orphan code; safe to delete.
- **Custom expenses fold into "Other"**: `gatherPayload()` adds `customTotal` into `expense_other_idr_cents` (`developer-operations-submit.js:233`). The names entered in the custom rows are discarded — they never reach the server. Investors and admins cannot see the expense breakdown the developer typed.
- `currentRow?.rejected_reason && status === "draft"` (`developer-operations-submit.js:168`) treats a draft with a `rejected_reason` as the "previously rejected, now redraft" state. Verify the admin reject handler resets status back to `'draft'` (not `'rejected'`) for this branch to fire.
- `parseUrl()` slices `assetId.slice(0,8)` for the breadcrumb (`developer-operations-submit.js:40, 50`) — if `assetId` is undefined (page hit without an asset ID in the URL), this throws.
- No client-side validation beyond night-count clamping. Sending zero or negative values silently saves a draft. The server's `compute_totals` will produce a 0/0 distributable — not technically wrong, but the UI doesn't warn before submission.
- No "back to dashboard / unsaved changes" warning on navigation.
- No `TODO`/`FIXME`/`XXX`/`Coming soon`/`Lorem`/`mock`/`fake` markers.

## 9. Production blockers
**Critical** — none.
**High** — Custom expense names are dropped server-side (folded into `expense_other_idr_cents`); the data the developer entered is lost on save. Either store the breakdown in a JSONB column on `villa_operations_log` or persist custom rows in a child table. **No tests** for the full P2 submit pipeline.
**Medium** — Dead `uploadDocument()` function pollutes the file (35 LOC). Missing dedicated mobile sheet on a long form. No "unsaved changes" guard.
**Low** — Potential null-deref in `parseUrl()` if `assetId` is missing. No inline validation messages.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 2/2 | All sections wired; edit-mode hydration added (read existing log via single-log GET); custom-expense rows now repopulate on edit. |
| Backend wiring | 2/2 | Eight endpoints + new single-log GET, all auth-gated with `DeveloperUser` + `require_asset_link`. Server strips admin-only fields. |
| Data realism | 2/2 | Custom expense names persisted to new `villa_operations_log.expense_other_notes` JSONB column (migration 202) and rendered on edit-mode hydrate. C-5 resolved 2026-05-19. |
| Error/empty states | 1/1 | Hydrate / save / submit / upload errors all surfaced; locked docs section; read-only after submit. |
| Mobile/responsive | 1/1 | Dedicated `mobile-developer-operations-submit.css` created + wired 2026-05-19. |
| Tests | 1/1 | HTTP coverage in `backend/tests/developer_operations_http.rs` (23 tests); E2E in `tests/e2e/test_developer_operations_submit.py` (7 tests, incl. C-5 custom-expense round-trip "Garbage Service"/50000); workflow step 12-15 in `developer_workflow_e2e.rs`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 1/1 | Labels, hints, inputmode=numeric, semantic landmarks, csrf headers, multipart progress states. |
| **TOTAL** | **10 / 10** | All dimensions met. |
