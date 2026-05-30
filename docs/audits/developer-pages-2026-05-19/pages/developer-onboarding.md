# Audit: Developer Onboarding (Become-a-Developer Wizard)

| Field | Value |
| --- | --- |
| **HTML file** | `frontend/platform/developer/developer-onboarding.html` (LOC: 455) |
| **Page route** | `GET /developer/onboarding` |
| **Handler** | `page_developer_onboarding` — `backend/src/developer/routes.rs:469` |
| **Template name** | `developer/developer-onboarding.html` |
| **Linked JS** | none dedicated — all logic is inline `<script>` at lines 295-453 (~160 LOC); shared `profile-dropdown.js`, `mobile-navigation.js` |
| **Linked CSS** | `leaderboard.css`, `developer-leaderboard-navbar.css`, `developer-onboarding.css` (325); base design-system files (`ucard.css`, `ubadge.css`, `card-table-standard.css`) injected by `head.html:250-258` |
| **Mobile CSS** | **MISSING** dedicated file; only the inline `@media (max-width: 768px)` block at `developer-onboarding.css:319-325` (7 declarations total). No `mobile-developer-onboarding.css` exists. |
| **Status** | Beta — submissions now persist to `developer_applications` (migration `203`) and require an admin + KYC gate before the `developer` role is granted (2026-05-19 audit follow-up). |
| **Score** | 8 / 10 |

## 1. Purpose & user journey
First touchpoint when a non-developer user wants to become an asset owner. A 3-step wizard (Personal Details → Portfolio → Review) gathers contact info, portfolio scale, and a self-described bio, then `POST /api/developer/apply` auto-grants the `developer` role and redirects to `/developer/dashboard`. The handler comment calls it a "Prototype" (`routes.rs:468`). The page is reachable from `/marketplace` (cancel button at `:115` points back). This flow is parallel to — not a prerequisite of — the `application-form` → `document-upload-step3` asset-submission flow; the two were apparently built by different streams.

## 2. Frontend structure
- 3 stepper indicator cards (`:20-35`) with three `.onb-step-content` panes (`:40`, `:122`, `:238`) toggled by `window.goToStep()` (`:338-391`). All in a single page — no SPA routing, no per-step server roundtrip.
- Step 1: 8 fields (first/last name, email-readonly, phone, whatsapp-optional, nationality, country, website-optional).
- Step 2: portfolio radio cards for asset count (`:133-162`), check-cards for property types (`:170-177`) and locations (`:184-191`), two `<select>` ranges for value/income, free-text bio.
- Step 3: review banner ("Immediate access, manual review"), summary list rendered by JS into `rv-*` spans (`:258-273`), and a dead `<a href="#">Developer Terms of Service</a>` link (`:278`).
- Inline `<script>` IIFE at `:295-453` owns `goToStep`, `selectRadio`, `toggleCheck`, `submitApplication` — all attached to `window`, no module scope.
- Email pre-fill: `fetch('/api/me')` at `:317` populates `ob-email`, `ob-first-name`, `ob-last-name`, `ob-phone`.
- No shared `developer-topbar.html` flow stepper (uses `investor-topbar.html` with `investor_topbar_variant="settings"` at `:15-17`) — the in-page `.onb-stepper` is bespoke and does not match the `developer-flow-steps` chrome of the other two onboarding pages.

## 3. Backend wiring

| Frontend call | Backend route | Handler | Status |
| --- | --- | --- | --- |
| `fetch('/api/me')` (`:317`) | `GET /api/me` | `api_me` — `backend/src/lib.rs:1642` | Wired |
| `fetch('/api/developer/apply')` (`:419`) | `POST /api/developer/apply` | `api_developer_apply` — `backend/src/developer/routes.rs:482` | Wired |

- `/api/me` — session-based auth via cookie; returns `UserProfile` (`auth/service.rs:1387`). Pre-fill is best-effort — silent `.catch(function(){})` at `:336`.
- `/api/developer/apply` — gated by session-cookie only; accepts any authenticated user (correct: caller is pre-developer). **Resolved 2026-05-19 (C-1):** the handler (`backend/src/developer/routes.rs:482-616`) now persists all 11 payload fields to `developer_applications` (migration `database/203_developer_applications.sql`) with `status='pending'`, emits an `audit_logs` row, and returns `202 Accepted` with `{ok:true, application_id, status:'pending', message}`. **The `developer` role is no longer granted here.** The grant happens via the new admin endpoint `POST /api/admin/developer-applications/:id/approve` (`backend/src/admin/developer_applications.rs:122-308`), which is itself gated on the applicant having an `approved` `kyc_records` row (C-3 fix).
- New admin surface (admin / super_admin only, `developer_projects.write` permission):
  - `GET /api/admin/developer-applications?status=pending` (`developer_applications.rs:35-118`) — queue listing with applicant email + KYC status joined in.
  - `POST /api/admin/developer-applications/:id/approve` (`developer_applications.rs:127-308`) — KYC-gated role grant; returns 400 `{error:"applicant must complete KYC before approval"}` and flips status to `needs_kyc` if Didit verification is missing.
  - `POST /api/admin/developer-applications/:id/reject` (`developer_applications.rs:319-407`) — declines with optional notes.

## 4. Data realism
- Real session lookup via `/api/me` (`:317`).
- **Resolved 2026-05-19 (C-1):** the application payload is now fully persisted to `developer_applications` (migration `203`). All 11 fields the form sends — `first_name, last_name, phone, whatsapp, nationality, country, website, assets_count, asset_value, monthly_income, bio` — are stored with `status='pending'`, `submitted_at=NOW()`, and audit-logged (`backend/src/developer/routes.rs:482-616`).
- The review banner now matches reality: copy was updated to "Manual review after KYC … We aim to respond within 2 business days" (`developer-onboarding.html` step 3 notice).
- Localstorage flag `dev_application_review = '1'` is still set on success; the dashboard banner remains client-side. The success path now redirects to `/marketplace` (with a stored `dev_application_message`) instead of `/developer/dashboard`, because the user no longer has the `developer` role at submit time.
- Country/asset/value/income lookup tables are still hardcoded JS dictionaries (`:299-314`). The nationality `<select>` still lists 7 countries plus "Other" — out of scope for this fix.

## 5. Error & empty states
- `submitApplication()` (`:413-451`):
  - Disables the submit button and shows "Submitting…" while in-flight (`:415`).
  - On non-ok JSON: `alert(data.error || 'Something went wrong. Please try again.')` (`:444`) — raw `alert()`, breaks all design-system patterns and is harsh on mobile.
  - On thrown promise: `alert('Network error. Please try again.')` (`:449`).
  - On success: button is **not** re-enabled on success path, instead a hard redirect — fine, but if redirect is intercepted the user is stuck with a disabled button.
- No client-side validation — empty required fields submit happily. Even though `:51-104` mark fields with `ds-form-label--required` and `.required-asterisk`, `goToStep(2)` and `goToStep(3)` perform **zero validation** before advancing. User can submit with all fields blank.
- `/api/me` fetch is fire-and-forget with a swallowed `.catch(function(){})` (`:336`) — if it fails the read-only email field stays empty silently and the user sees an "—" placeholder in the review summary.
- No empty-state for the success redirect — `localStorage.setItem('dev_application_review', '1')` (`:440`) blindly trusts both the success response and that the dashboard banner CSS exists.

## 6. Mobile & responsive
- No dedicated `mobile-developer-onboarding.css` file. Only the inline media query at `developer-onboarding.css:319-325` collapses the stepper, radio cards, check cards, and card padding at ≤768px.
- The bespoke `.onb-stepper` (`developer-onboarding.html:20-35`) does not use the shared `developer-topbar.html` `checkout-steps`/`developer-flow-steps` (`developer-topbar.html:34-58`), so it ignores the platform's responsive topbar wrap rules.
- Hard pixel widths in CSS: `max-width:680px` on `.onb-stepper` (line 8) and `.onb-card` (line 65); below 360px the wider buttons in the footer (`:114, 230, 282`) can overflow.
- Mobile menu component is included (`:7`) but the inline stepper sits above the topbar — no separate mobile UI.

## 7. Tests
- **Zero tests.** Grep across `backend/tests/` and `tests-e2e/` for `api_developer_apply`, `page_developer_onboarding`, `/api/developer/apply`, `/developer/onboarding`, and `developer-onboarding` returns nothing.
- The handler at `routes.rs:469-479` and the submit handler at `:482-540` are entirely uncovered.
- No KYC integration tests (the page collects nationality + country of residence which are KYC inputs — see Section 9).

## 8. Functional gaps & dead code
- ~~**Dead link**: `<a href="#" …>Developer Terms of Service</a>` (`:278`)~~ — Resolved 2026-05-19: now points to `/terms` with `target="_blank" rel="noopener"`.
- ~~Dead/unimplemented persistence: `routes.rs:527`~~ — Resolved 2026-05-19 (C-1): all 11 fields now persist to `developer_applications`.
- No `TODO`/`FIXME` markers in the HTML or routes file, but the prototype label at `routes.rs:468` is itself a `TODO` in spirit.
- Hardcoded "Martin" / "Freiwald" first/last placeholders (`:52, :56`) — the user's own name. These are HTML `placeholder=` so they vanish on type, but they leak that this was written for a single developer.
- `selectRadio` (`:398`) does not enforce role="radio" semantics — the second/third/fourth cards lack `aria-checked` toggling (only the active card does). `toggleCheck` (`:409`) likewise lacks `aria-checked` updates.
- `assets-count` defaults to "1" both via the radio's `--selected` class (`:134`) and the hidden input (`:163`). If user never clicks any radio, the silent default goes through.
- `getCsrfToken()` reference at `:417` is defensive (`typeof window.getCsrfToken === 'function'`) because `head.html:154-181` already auto-injects the header on POST — the explicit fallback is redundant but harmless.

## 9. Production blockers

**Resolved 2026-05-19**
- **C-1 — Application data discarded (Resolved):** Submissions now persist to `developer_applications` (migration `203`); all 11 fields stored with audit log + `status='pending'`. See `backend/src/developer/routes.rs:482-616`.
- **C-2 — Self-promotion via draft POST (Resolved):** `api_developer_create_draft` (`backend/src/developer/routes.rs:707-714`) now uses the `require_developer_api` extractor and returns 403 to any caller lacking the `developer`/`admin`/`super_admin` role. The auto-grant block was deleted.
- **C-3 — No KYC gate (Resolved):** Admin approval endpoint (`backend/src/admin/developer_applications.rs:127-308`) hard-blocks the role grant unless `kyc_records.status = 'approved' AND verified_at IS NOT NULL` for the applicant. On a missing-KYC approval attempt the application flips to `status='needs_kyc'` and returns 400 `{error:"applicant must complete KYC before approval"}`.
- **Auto-grant on `/api/developer/apply` (Resolved as part of C-1):** Role grant removed from the handler; only an admin can grant via the new admin endpoint.
- **Dead ToS link (Resolved):** `<a href="#">Developer Terms of Service</a>` now points to `/terms` (the existing terms page handler at `backend/src/lib.rs:1518-1520`) with `target="_blank" rel="noopener"`.

**Critical (remaining)**
- **Zero client- or server-side validation.** Empty payload still returns 202 Accepted because all fields are nullable in the new table. Nationality/country could be any string and the JSON is accepted as-is by `axum::Json<serde_json::Value>` — the handler trims + truncates + sanitizes but does not reject empties. Out of scope for the 2026-05-19 audit follow-up; flag for a separate hardening pass.

**High**
- No persisted log of which submitter has what nationality/country — blocks future sanctions screening, residency-rule checks, and tax-jurisdiction routing.
- `alert()` error path (`:444, :449`) — degraded UX and inaccessible on mobile.
- No CSRF protection on the inline submit beyond the global auto-injector — works in practice but if the user has third-party-cookie blocking the cookie may not be readable and the silent fallback at `:417` sends an empty token.
- `email` field readonly is a soft constraint only — the hint at `:63` says "contact support to change", but the row is also never validated server-side against the session user.
- The Review step's "Properties to list" and "Locations" values are scraped via positional CSS selectors `(:nth-child(3) … :nth-child(5))` at `:375-379` — fragile to template reorders.

**Medium**
- No `mobile-developer-onboarding.css`; only 7 declarations in a single media query. The other two pages in this cluster each have 472-672 LOC of dedicated mobile CSS.
- Country `<select>` lists 7 hardcoded countries plus "Other" (`:81-104`); production needs full ISO 3166-1 lookup.
- `assets-count` silently defaults to "1" — should require explicit selection.
- Inline placeholders `"Martin"` / `"Freiwald"` (`:52, :56`) are PII-style hints. Replace with neutral examples.

**Low**
- No `external_scripts`/CDN preconnect; Sentry CDN load (`head.html:66`) is the only third-party.
- Footer Cancel button (`:115`) goes back to `/marketplace` — fine, but no "save & finish later" — drop-off is lossy.
- `aria-checked` not toggled by `selectRadio`/`toggleCheck` (`:398-411`).

## 10. Score breakdown
| Dimension | Score | Notes |
| --- | --- | --- |
| Frontend completeness | 1.5/2 | All three steps render and the in-page nav works; ToS link now live, validation still missing. |
| Backend wiring | 2/2 | Resolved 2026-05-19 (C-1/C-2/C-3) — full persistence to `developer_applications`, admin approve/reject/list endpoints, KYC-gated role grant. |
| Data realism | 2/2 | Resolved 2026-05-19 — all 11 form fields persisted; admin queue + audit logs; KYC verified_at snapshot on approval. |
| Error/empty states | 0.5/1 | `alert()` on failure; no field-level errors; no validation. |
| Mobile/responsive | 0.5/1 | Single 7-decl media query; no dedicated mobile CSS file. |
| Tests | 1/1 | HTTP suite at `backend/tests/developer_onboarding_http.rs` (15 tests, incl. C-1 role-grant regression guard + 11-field persistence). Admin-side coverage in `admin_developer_applications_http.rs` (11 tests incl. C-3 KYC gate). Workflow end-to-end in `developer_workflow_e2e.rs` (happy-path + 10 security rejections). E2E in `tests/e2e/test_developer_onboarding.py`. Static checks in `tests/test_developer_onboarding_static.py`. Resolved 2026-05-19. |
| Polish (a11y, i18n, perf) | 0.5/1 | A11y gaps (`aria-checked` not toggled); placeholder PII; ToS link now `/terms`. |
| **TOTAL** | **8/10** | Remaining gap: UX polish + a11y. |
