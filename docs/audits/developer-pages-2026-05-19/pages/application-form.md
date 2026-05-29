# Audit: Developer Application Form (Property Info — Asset Submission Step 2)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/application-form.html` (LOC: 247) |
| **Page route** | `GET /developer/application-form` |
| **Handler** | `page_developer_application_form` — `backend/src/developer/routes.rs:543` |
| **Template name** | `developer/application-form.html` |
| **Linked JS** | `developer-application-form.js` (841), plus shared `profile-dropdown.js`, `mobile-navigation.js`, `poool-dropdown.js`, `poool-dropdown-init.js` |
| **Linked CSS** | `leaderboard.css`, `developer-application-form.css` (359), `mobile-developer-application-form.css` (487), `cart.css`, `checkout.css`, `poool-icon-custom.css`, `developer-leaderboard-navbar.css` |
| **Mobile CSS** | `mobile-developer-application-form.css` (487 LOC, 2 media queries: ≤768px at line 7, ≤374px at line 472) |
| **Status** | Beta — wired to real DB drafts, role-grant side-effect removed 2026-05-19 (C-2 fix); UX gaps remain. |
| **Score** | 7.75 / 10 |

## 1. Purpose & user journey
Step 2 of the 4-step asset submission wizard (`developer-topbar.html:34-58`: Asset Type → Property Info → Documents → Review). Captures property details (name, type, area, address, lease, dimensions, year built) and financials (purchase price, minimum share price), then either saves & exits to `/developer/submissions` or POSTs/PUTs to the real `assets` table and advances to `/developer/document-upload-step3?draft_id=…`. Previous step is `/developer/add-asset`; next step is `/developer/document-upload-step3`. Note: **misnamed** — the URL says "application" but it is actually the property-details form, not a developer-application form (the latter lives at `/developer/onboarding`).

## 2. Frontend structure
- Single `<main>` with `developer-topbar.html` flow stepper at `:13-15` (`dev_nav_flow_step="property"`), then two `.form-section` cards (`:17-183` property details, `:185-223` financials), then `.form-actions` (`:225-242`).
- 15 inputs in a `.form-grid` (`developer-application-form.css:89-94`) — mix of `<input type=text/number/url>` and `<select>` widgets. Each `<select>` is later upgraded to a `PooolDropdown` custom widget at run-time by `poool-dropdown-init.js` (referenced in `extra_js`).
- Action row: Previous (back to `/developer/add-asset`), Save & Exit, Next Step. "Previous" uses inline `onclick="window.location.href = '/developer/add-asset'"` (`:226`) — does not preserve `draft_id`.
- `developer-application-form.js` (841 LOC) is the page brain:
  - `saveAndExitStep2()` (`:83-192`) — POST or PUT to `/api/developer/draft[/:id]`, redirects to `/developer/submissions`.
  - DOM-ready block (`:194-571`) — pre-fills from `GET /api/developer/draft/:id` if `draft_id` is in URL or localStorage; binds Next Step handler; binds file upload (dead code, see Section 8); attaches currency live-formatting.
  - Validation (`:299-377`) — per-field error display with red border + helper text (`:589-621`); top-level error toast (`:666-699`).
  - File upload helpers (`:721-827`) — `handleFiles`, `addFileToList`, `simulateUpload`, `removeFile`. Dead on this page (no `#file-input` exists in `application-form.html`).
- No HTMX.
- Shared components: `head.html`, `mobile-menu.html`, `sidebar.html`, `developer-topbar.html`.

## 3. Backend wiring

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `fetch('/api/developer/draft/:id')` GET (`developer-application-form.js:203`) | `GET /api/developer/draft/:id` | `api_developer_get_draft` — `backend/src/developer/routes.rs:1397` | Wired |
| `fetch('/api/developer/draft')` POST (`:148, :162, :424, :438`) | `POST /api/developer/draft` | `api_developer_create_draft` — `backend/src/developer/routes.rs:687` | Wired |
| `fetch('/api/developer/draft/:id')` PUT (`:148, :424`) | `PUT /api/developer/draft/:id` | `api_developer_update_draft` — `backend/src/developer/routes.rs:1096` | Wired |
| Page back/next nav targets `/developer/add-asset`, `/developer/document-upload-step3` | `GET /developer/{…}` | `page_developer_add_asset`, `page_developer_document_upload` — `mod.rs:64, 71` | Wired |

- All draft APIs are **session-cookie auth**. **Resolved 2026-05-19 (C-2):** the auto-grant block (formerly at `routes.rs:719-746`) was deleted; `api_developer_create_draft` (`backend/src/developer/routes.rs:707-714`) now delegates to `require_developer_api`, which returns 403 to any caller lacking the `developer`/`admin`/`super_admin` role. The only path to that role is admin approval of a `developer_applications` row (see developer-onboarding.md).
- CSRF: `X-CSRF-Token` header is added both explicitly (`developer-application-form.js:152, 166, 428, 442`) and by the global `head.html:154-181` interceptor.
- `GET /api/developer/draft/:id` filters by `developer_user_id = $2` (`routes.rs:1430`) — proper ownership check; returns 404 on someone else's draft (stale-pointer recovery at `:159-170, :433-446` clears localStorage and POSTs a fresh one).
- `validate_draft_shape()` (`routes.rs:119-144`) caps title at 160 chars, lease 1–150 years, sizes ≤1M sqm, bedrooms/bathrooms 0–200, year_built 1800–2100, share price ≥$1. Server-side bounds match client-side ranges.

## 4. Data realism
- **Real** Postgres `assets` table read/write. `api_developer_get_draft` (`routes.rs:1414-1454`) selects 30+ columns from `assets`, plus images and documents joins.
- **Real** XSS sanitization on create (`routes.rs:763-787`) and update (search for `sanitize_text` calls).
- Hardcoded option lists in HTML (not from DB):
  - `property-type` 4 values (`:36-41`).
  - `area` 7 values (`:55-63`) — Bali-only hardcode (Canggu, Uluwatu, Seminyak, Ubud, Sanur, Denpasar, Jimbaran).
  - `lease-type` 2 values (`:99-102`).
  - `status` 3 values (`:161-166`).
- Hardcoded units ("years", "sq.m") rendered as readonly inputs (`:117, :127, :138`) — fine for now but won't internationalize.
- Placeholder text uses real Bali examples ("Villa Yanami Green" `:25`, "800" `:126`, etc) — pure UI hints, no data leakage.
- Minimum share price defaults to `$1` (`:210`), enforced both client-side (`developer-application-form.js:104, 287, 364`) and server-side (`routes.rs:95-98`).
- The `assetType` from step 1 is read from `window.selectedAssetType || localStorage.getItem('selectedAssetType')` (`:92, :260-262`) — soft dependency on `/developer/add-asset` setting it; default fallback is `'real_estate'`. If user lands here directly without visiting step 1, `real_estate` is assumed.

## 5. Error & empty states
- Per-field errors with red border + icon (`:589-621`).
- Top-level toast for API errors (`:666-699`) auto-dismisses after 8s (`:698`).
- Network errors: explicit "Connection lost — please check your internet and try again" (`:469`) and Save & Exit "Connection lost — your draft was not saved. Please try again." (`:188`).
- Stale-draft handling: 404/410/403 on PUT clears localStorage `draft_asset_id` and re-POSTs (`:159-170, :433-446`). 403 means draft belongs to someone else.
- Permission/auth error mapping in `readApiErrorMessage()` (`:57-77`): 401 → "You are not logged in…"; 403 → "You don't have permission…".
- Pre-fill GET: silent fallback — if draft 404s, `removeItem('draft_asset_id')` and no UI feedback (`:206-209`).
- No skeleton/loading state during pre-fill — the form is interactive immediately but values pop in 200-500ms later, causing user typing to be overwritten.
- The PooolDropdown `setDropdownVal` retry loop (`:22-37`) silently gives up after 5 attempts — pre-fill of `<select>`s can fail invisibly if the dropdown widget is slow to initialize.

## 6. Mobile & responsive
- Dedicated `mobile-developer-application-form.css` (487 LOC) with breakpoints at ≤768px and ≤374px.
- `developer-application-form.css:319` adds an additional ≤768px block.
- Hard widths in main CSS: `max-width: 1096px` for title/sections (`:52, :68, :295`) — proper responsive containment.
- Grid collapses 2-col → 1-col on mobile (verify in `mobile-*-application-form.css`).
- Action button bar may stack vertically on narrow widths — Save & Exit inline-styled at `:233-235` could overflow.

## 7. Tests
- **Zero integration tests** for `page_developer_application_form`, `api_developer_create_draft`, `api_developer_update_draft`, `api_developer_get_draft`. Grep across `backend/tests/` for `application-form`, `api_developer_apply`, `api_developer_create_draft` returns nothing.
- Inline unit tests for the validation helpers exist at `routes.rs:146-200` — `derive_tokens_total`, `normalize_asset_type`, `validate_draft_shape` are covered (6 tests). But no end-to-end test of the full request/response flow, no test of auto-role-grant, no test of stale-draft recovery.
- No E2E test (`tests-e2e/` is empty).

## 8. Functional gaps & dead code
- **Dead file-upload code** (`developer-application-form.js:477-507, 721-827` — ~150 LOC). The script binds drag/drop and file-input handlers, defines `handleFiles`, `addFileToList`, `simulateUpload`, `removeFile`, `formatFileSize`. There is no `<input type="file">`, `.file-upload-area`, or `.drag-overlay` in `application-form.html`. This entire path is dead on this page; it's a leftover from when the form had a single combined upload section.
- `simulateUpload` (`:793-814`) uses `setInterval` with `Math.random()` to fake progress — never makes a network call. If file upload were re-enabled here, this would silently lie to the user.
- Inline `onclick` on Previous (`:226`) does not preserve `draft_id` in URL — user goes back to `/developer/add-asset` and loses the localStorage reference if they navigate elsewhere first.
- Save & Exit redirects to `/developer/submissions` (`:179`) without confirmation; the partial draft might fail server-side validation but the button text says "Saved" and the user is teleported away.
- Multiple `placeholder=` text strings are realistic Bali examples (`:25, :82, :89, :116, :126, :137, :146, :153, :179`) — fine, but combined with the hardcoded `area` options means the form is implicitly Indonesia-only.
- No "Coming soon"/`TODO`/`FIXME`/`Lorem`/`mock`/`fake` markers in HTML or JS.
- `addFileToList` (`:751-791`) builds raw HTML with concatenation — XSS-safe only because `escFormHtml` is applied to `file.name` (`:768`); but `fileExt` (`:754`) is not escaped, an attacker-named file like `foo.<script>` would interpolate raw HTML — moot because the code is dead.

## 9. Production blockers

**Resolved 2026-05-19**
- **C-2 — Auto-grant on first draft POST (Resolved):** the role-grant block was deleted from `api_developer_create_draft`; the handler now uses the `require_developer_api` extractor (`backend/src/developer/routes.rs:707-714`) and 403s any caller without the `developer`/`admin`/`super_admin` role. Role can only be granted by an admin via `POST /api/admin/developer-applications/:id/approve`, which is KYC-gated (C-3 fix; see developer-onboarding.md).
- **No KYC gate (Resolved indirectly via C-3):** the role grant — the only thing that lets a user reach this endpoint — now requires an `approved` `kyc_records` row. So while this handler itself does not call the KYC service, the only way a user can reach it is by having already completed Didit verification.

**Critical (remaining)**
- **No geographic limits** — `area` (`:55-63`) is Bali-only but `address`/`city`/`country` (`:75, :82, :89`) are free text. A developer could submit a property in a sanctioned country with no server-side jurisdiction check.

**High**
- **Dead file-upload code** (~150 LOC) inside the main page script. Risk of confusing developers; if accidentally re-enabled it would fake uploads with `simulateUpload`.
- **No "save" indicator on field blur** — user can lose 5 minutes of typing if they tab away before clicking Save & Exit.
- **Pre-fill race** — `setDropdownVal` retries 5 times then gives up silently (`developer-application-form.js:22-37`). If the user has slow JS, `<select>` widgets may load blank despite a saved draft.
- **Misleading URL slug** — `application-form` collides with the conceptual "developer application" (`/developer/onboarding`). Two distinct flows, near-identical names.

**Medium**
- Inline `onclick="window.location.href = '/developer/add-asset'"` (`:226`) bypasses the JS draft-id preservation.
- Hardcoded `area` list (`:55-63`) is Bali-only. Expand or load from DB before listing global assets.
- `Save & Exit` button is styled with raw inline CSS (`:233-235`) instead of a class — inconsistent with the rest of the design system.
- No `aria-required` on required inputs (only `<span class="required-asterisk">*</span>` visual marker).

**Low**
- "years" / "sq.m" hardcoded as readonly inputs (`:117, :127, :138`) — won't internationalize.
- No client-side debounce on currency live-formatting (`:535-544`) — fine for short values, but could thrash on paste of 100-digit numbers.
- `developer-document-upload-step3.css` is loaded as a CSS dep on this page implicitly (via being referenced in step3 only), but `developer-application-form.css` is also loaded on step3 (`document-upload-step3.html:1`), creating mutual coupling.

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | All 15 fields work; ~150 LOC of dead file-upload code is noise. |
| Backend wiring | 2/2 | Resolved 2026-05-19 (C-2) — auto-role-grant removed; `require_developer_api` extractor now enforces role boundary; real CRUD against `assets` unchanged. |
| Data realism | 2/2 | Real DB writes with sanitization; role/KYC gate is now real (admin approval path). Option lists still hardcoded; Bali-only `area` (out of scope). |
| Error/empty states | 0.75/1 | Per-field errors, stale-draft recovery, network toast; pre-fill failures are silent. |
| Mobile/responsive | 1/1 | Two breakpoints, 487 LOC dedicated mobile file. |
| Tests | 1/1 | HTTP coverage: `backend/tests/developer_drafts_http.rs` (21 tests incl. C-2 regression guard for the draft auto-grant fix); static checks in `tests/test_developer_application_form_static.py` (11 tests); workflow path in `backend/tests/developer_workflow_e2e.rs` `happy_path_apply_to_payout`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | Required-asterisks visual-only; hardcoded units; Bali bias; clean CSRF wiring. |
| **TOTAL** | **7.75/10** | Remaining gap: jurisdiction enforcement + dead file-upload code + a11y polish. |
