# Page Audit: Identity Verification

Date: 2026-04-28
Status: fixed_needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/kyc`
Template: `frontend/platform/kyc.html`
JavaScript: `frontend/platform/static/js/kyc-page.js`
CSS: `frontend/platform/static/css/kyc.css`, `frontend/platform/static/css/leaderboard.css`
Backend Routes: `backend/src/kyc/mod.rs`, `backend/src/kyc/routes.rs`, `backend/src/kyc/service.rs`, `backend/src/storage/routes.rs`

---

## Summary

The `/kyc` page is route-protected and its core APIs are registered. A follow-up fix pass on 2026-04-28 addressed the audited high-risk manual-review gaps: the XHR upload now sends the CSRF token, manual submission requires and transactionally links a user-owned pending document, manual identity/address fields are persisted to `user_profiles`, KYC mutation audit rows are written in the same transaction, endpoint-specific rate limits were added, provider-return state now fetches authoritative backend status, and frontend document/file constants match backend normalization.

Final status is `fixed_needs_recheck` because authenticated browser/API E2E still needs a seeded KYC session, and two residual issues remain documented below.

## Remediation Update - 2026-04-28

Fixed:

- `PAGE-ISSUE-0540`: `frontend/platform/static/js/kyc-page.js` now reads `csrf_token` and sets `X-CSRF-Token` on the upload XHR before `send()`.
- `PAGE-ISSUE-0541`: `backend/src/kyc/service.rs` now rejects missing/invalid/already-linked documents and rolls back the manual KYC record unless exactly one pending document for the same user is linked.
- `PAGE-ISSUE-0542`: manual KYC now validates and persists first name, last name, DOB, nationality, address, city, and country through `user_profiles`.
- `PAGE-ISSUE-0543`: required audit rows for initiation, manual submission, webhook status updates, and document upload now participate in the same DB transaction as the sensitive mutation. Email enqueue failures are logged for operator visibility.
- `PAGE-ISSUE-0544`: user-scoped rate-limit keys were added for KYC initiate, submit, and document upload.
- `PAGE-ISSUE-0545`: the `poool_kyc_pending` flag is now only cleared as a return marker; it no longer bypasses `/api/kyc/status`.
- `PAGE-ISSUE-0546`: `driving_licence` is used consistently in the template, the backend normalizes the legacy `driving_license` spelling, and the upload accept list matches JPEG/PNG/WebP/PDF.

Remaining issues:

- `PAGE-ISSUE-0550` (medium): KYC email delivery is still best-effort after commit. The code logs enqueue failures, but there is no durable outbox/retry table yet.
- `PAGE-ISSUE-0551` (low): document bytes are uploaded to private storage before the DB transaction starts. A later DB/audit failure can leave an orphaned private object until cleanup tooling removes it.
- Authenticated browser/API E2E remains required for upload success with CSRF, no-CSRF rejection, manual submit DB state, provider-return status, and mobile/keyboard behavior.

---

## Tested Scope

- Static review of `frontend/platform/kyc.html`.
- Static review of `frontend/platform/static/js/kyc-page.js`.
- Backend review of KYC status/initiate/submit/webhook routes and storage upload route.
- Schema review of `kyc_records` and `kyc_documents`.
- Existing test discovery for KYC API/page coverage.
- `node --check frontend/platform/static/js/kyc-page.js`.
- `cd backend && cargo check` attempted during remediation; blocked by concurrent Cargo build activity in the shared workspace.
- `python3 -m pytest tests/test_kyc_identity_static.py -q`.
- Local curl smoke attempted against `http://localhost:8888`, but no server was listening.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/kyc` | Protected page via `serve_protected`. |
| Template | `frontend/platform/kyc.html` | Alpine-powered manual and redirect KYC states. |
| JS | `frontend/platform/static/js/kyc-page.js` | Owns status loading, provider detection, wizard validation, upload, submit, redirect. |
| CSS | `frontend/platform/static/css/kyc.css` | Page-specific layout and state styling. |
| Backend page route | `GET /kyc` | `kyc::routes::page_kyc`. |
| Backend API route | `GET /api/kyc/status` | Current user's latest KYC status. |
| Backend API route | `GET /api/kyc/provider` | Active provider and redirect support. |
| Backend API route | `POST /api/kyc/initiate` | Starts provider session. |
| Backend API route | `POST /api/kyc/submit` | Legacy manual submission. |
| Backend API route | `POST /api/upload/kyc` | Private document upload. |
| Webhook route | `POST /api/webhooks/kyc/didit` | HMAC-verified Didit status updates. |
| Database table | `kyc_records` | Main KYC status/session table. |
| Database table | `kyc_documents` | Private document metadata table. |
| Database table | `user_profiles` | Partially updated by manual and webhook flows. |
| Tests | `tests/test_e2e_kyc_registration.py`, `tests/test_rewards.py`, `tests/test_platform.py` | Mostly API/page smoke; no browser upload/manual submit coverage. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Mobile logo | `.mobile-header__logo` | Navigate to marketplace. | Link | `GET /marketplace` | Not runtime-tested. |
| Mobile menu button | `#mobile-burger-btn` | Open mobile menu. | Inline `toggleMobileMenu()` from shared JS | N/A | Not runtime-tested. |
| Sidebar | `components/sidebar.html` | Investor navigation. | Shared JS/CSS | Protected page routes | Not runtime-tested. |
| Status subtitle | `getStatusMessage()` in topbar | Reflect KYC status. | Alpine computed text | `GET /api/kyc/status` | Static verified. |
| Loading state | `status === 'loading'` | Show status-check spinner. | Alpine | `GET /api/kyc/status` | Static verified. |
| Error refresh | `#kyc-refresh-btn` | Reload page after status failure. | Inline `window.location.reload()` | N/A | Static verified. |
| Redirect document type | `#kyc-doc-type-redirect` | Choose document type before provider redirect. | Alpine model | `POST /api/kyc/initiate` stores hint | Partially wired; value uses `driving_license`, while upload/manual allowlist uses `driving_licence`. |
| Start Verification | `#kyc-start-verification-btn` | POST initiate and redirect to provider URL. | `submitKyc()` fetch | `POST /api/kyc/initiate` | Static verified; no authenticated runtime. |
| First Name | `#kyc-first-name` | Required manual field. | Alpine validation | Backend updates `user_profiles.first_name` best-effort | Backend does not store in `kyc_records`. |
| Last Name | `#kyc-last-name` | Required manual field. | Alpine validation | Backend updates `user_profiles.last_name` best-effort | Backend does not fail if update fails. |
| Step 1 Continue | `#kyc-step1-next` | Validate name fields and advance. | `validateAndNext(1)` | N/A | Static verified. |
| Date of Birth | `#kyc-dob` | Required manual field. | Alpine validation | Payload sent to backend | Backend ignores value. |
| Nationality | `#kyc-nationality` | Required manual field. | Alpine validation | Payload sent to backend | Backend ignores value except webhook approval path. |
| Step 2 Back/Continue | `#kyc-step2-back`, `#kyc-step2-next` | Navigate wizard. | Alpine | N/A | Static verified. |
| Address | `#kyc-address` | Required manual field. | Alpine validation | Payload sent to backend | Backend ignores value. |
| City | `#kyc-city` | Required manual field. | Alpine validation | Payload sent to backend | Backend ignores value. |
| Country | `#kyc-country` | Required manual field. | Alpine validation | Payload sent to backend | Backend ignores value. |
| Step 3 Back/Continue | `#kyc-step3-back`, `#kyc-step3-next` | Navigate wizard. | Alpine | N/A | Static verified. |
| Manual document type | `#kyc-doc-type` | Choose document type for upload and record. | Alpine model | `POST /api/upload/kyc`, `POST /api/kyc/submit` | Static verified. |
| Upload input | `input[type=file]` in `.kyc-upload-link` | Upload PDF/image, show progress and document ID. | `uploadDocument()` XHR | `POST /api/upload/kyc` | Expected broken due missing XHR CSRF header. |
| Drag/drop upload | `.kyc-upload-container` | Upload dropped file. | `handleDocDrop()` | `POST /api/upload/kyc` | Expected broken due missing XHR CSRF header. |
| Remove document | `#kyc-remove-doc` | Clear selected uploaded document from client state. | Alpine only | No delete route | Client-only; previously uploaded server document remains pending/orphaned. |
| Upload progress | `.kyc-upload-progress` | Display upload percent. | XHR progress | Upload route | Static verified. |
| Step 4 Continue | `#kyc-step4-next` | Require uploaded document ID. | Alpine validation | N/A | Static verified. |
| Review values | `.kyc-review-value`, `.kyc-review-doc-name` | Show entered KYC data safely. | `x-text` | `POST /api/kyc/submit` | Frontend safe; backend drops several fields. |
| Submit for Review | `#kyc-submit-btn` | Submit manual KYC. | `submitKyc()` fetch | `POST /api/kyc/submit` | Fetch CSRF is globally injected; backend validation incomplete. |
| Pending marketplace link | `#kyc-back-to-marketplace` | Navigate to marketplace. | Inline location change | `GET /marketplace` | Static verified. |
| Approved marketplace link | `#kyc-go-to-marketplace` | Navigate to marketplace. | Inline location change | `GET /marketplace` | Static verified. |
| Try Again | `#kyc-try-again` | Reset local form after rejected/expired status. | `resetForm()` | No backend reset | Client-only; previous records remain. |
| Fallback refresh | `#kyc-fallback-refresh` | Reload page. | Inline reload | N/A | Static verified. |

---

## Frontend Findings

### P1 - KYC document upload omits CSRF header

Location:

- Template: `frontend/platform/kyc.html` lines 295-311
- JS: `frontend/platform/static/js/kyc-page.js` lines 238-266
- Backend: `backend/src/auth/csrf.rs` lines 52-132

Problem:

The upload path uses raw `XMLHttpRequest` and never sets `X-CSRF-Token`. The shared head template only injects CSRF for `fetch`, HTMX requests, and normal HTML form submissions. `POST /api/upload/kyc` is not exempt from CSRF, so authenticated browser uploads are expected to return 403 before reaching `upload_kyc_document`.

Expected:

KYC document upload should either use `fetch` so the global interceptor applies, or explicitly read the `csrf_token` cookie and set `X-CSRF-Token` on the XHR before `send()`.

Evidence:

`kyc-page.js` calls `xhr.open("POST", "/api/upload/kyc"); xhr.send(formData);` with no CSRF header. `auth/csrf.rs` validates all mutating `/api/*` requests except webhook/static routes.

Recommended fix:

Set `X-CSRF-Token` on the XHR, or refactor upload to `fetch` plus progress-compatible upload handling if progress can be dropped or implemented another way. Add browser/API regression coverage that proves missing CSRF is rejected and the page upload succeeds with the header.

### P2 - Provider return state is client-trusted

Location:

- JS: `frontend/platform/static/js/kyc-page.js` lines 30-34 and 308-313

Problem:

After initiating a redirect flow, the page stores `poool_kyc_pending` in `localStorage`. On the next `/kyc` load it immediately sets `status = "in_review"` and returns before fetching `/api/kyc/status`. A user who cancels or fails the provider flow can see "Under Review" until a later refresh.

Expected:

The page should always fetch authoritative status on return. Local storage can show a transitional loading message, but it should not bypass backend status verification.

Evidence:

`initKyc()` returns immediately when the localStorage flag exists.

Recommended fix:

Remove the early return and use the flag only to adjust copy while `/api/kyc/status` and provider status are checked.

### P3 - Accepted upload/document-type options are inconsistent

Location:

- Template: `frontend/platform/kyc.html` lines 147-151 and 288-311
- Backend: `backend/src/storage/routes.rs` lines 430-444

Problem:

The redirect document selector sends `driving_license`, while the manual upload route allowlist accepts `driving_licence`. The file input accepts `image/*`, which allows GIF in the browser picker, while the backend only accepts JPEG, PNG, WebP, and PDF and the UI hint says PNG/JPG/PDF.

Expected:

Frontend values and accept hints should match backend allowlists exactly.

Evidence:

The template contains both `driving_license` and `driving_licence` spellings; storage only allows `driving_licence`.

Recommended fix:

Normalize document type constants across redirect/manual/backend paths and use `accept="image/jpeg,image/png,image/webp,application/pdf"` or update copy and backend intentionally.

---

## Backend Findings

### P1 - Manual KYC can be submitted without a linked identity document

Location:

- Backend: `backend/src/kyc/routes.rs` lines 48-69
- Backend: `backend/src/kyc/service.rs` lines 338-379
- JS: `frontend/platform/static/js/kyc-page.js` lines 330-348

Problem:

The frontend requires `documentId`, but the backend accepts `document_id: Option<Uuid>`. If a client posts directly to `/api/kyc/submit` with no document ID, the backend still inserts a pending manual `kyc_records` row. If a document ID is supplied but the `UPDATE kyc_documents ... WHERE id = $2 AND user_id = $3` matches zero rows, that result is also swallowed.

Expected:

Manual KYC submission must require an authenticated user's uploaded pending KYC document and fail if it cannot atomically link that document to the created KYC record.

Evidence:

`submit_kyc` inserts into `kyc_records` unconditionally, then runs `let _ = sqlx::query!(...).execute(...)` only when an optional document ID exists.

Recommended fix:

Wrap manual submit in a transaction. Validate required fields server-side, require `document_id`, lock or update exactly one `kyc_documents` row for the same user and pending status, and rollback the KYC record if linking fails.

### P1 - Manual KYC discards most identity fields entered by the user

Location:

- Template: `frontend/platform/kyc.html` lines 183-262
- JS: `frontend/platform/static/js/kyc-page.js` lines 331-341
- Backend: `backend/src/kyc/service.rs` lines 344-365
- Schema: `docs/DATABASE_SCHEMA.md` lines 123-144

Problem:

The page collects DOB, nationality, address, city, and country and sends them to `/api/kyc/submit`, but `submit_kyc` only best-effort updates first and last name in `user_profiles`. The pending `kyc_records` row stores only `document_type`; the remaining identity/address fields are not persisted for review.

Expected:

All required manual-review fields should be persisted in a defined schema/table, or the page should not collect them. Persistence errors should fail the submission.

Evidence:

`KycSubmitRequest` contains these fields, but the manual service ignores DOB, nationality, address, city, and country.

Recommended fix:

Persist manual KYC payload into a dedicated reviewed-data structure or extend the KYC/profile schema intentionally. Make the write transactional with the `kyc_records` and document-link writes.

### P2 - KYC initiate and manual submission side effects are not durable

Location:

- Backend: `backend/src/kyc/service.rs` lines 142-172 and 381-386
- Backend: `backend/src/storage/routes.rs` lines 486-498

Problem:

KYC initiation commits the `kyc_records` row, then best-effort writes the audit log and sends email. Manual submit and document upload also swallow audit/email side effects. For sensitive identity workflows, missing audit rows make compliance investigation harder.

Expected:

At minimum, audit logging for KYC creation/submission/document upload should be durable or the response should expose a safe degraded state. Email can be async, but failed enqueue/delivery should be observable.

Evidence:

The code uses `let _ =` for audit/email calls after KYC writes.

Recommended fix:

Move required audit rows into the same transaction as the KYC/document mutation. Route email through a durable outbox or record enqueue failure in operator-visible logs/metrics.

### P2 - KYC submit/initiate/upload have no endpoint-specific rate limit

Location:

- Backend: `backend/src/kyc/routes.rs` lines 48-110
- Backend: `backend/src/storage/routes.rs` lines 315-514

Problem:

Authenticated users can repeatedly initiate KYC sessions, submit manual applications, and upload up to 10 MB identity documents without a route-level rate limit. CSRF and authentication protect cross-site requests, but do not prevent account-level spam or storage-cost abuse.

Expected:

KYC initiation, manual submission, and document upload should have per-user/session/IP limits and clear 429 JSON responses.

Evidence:

No KYC route checks a rate limiter; upload only enforces per-file size and MIME/content validation.

Recommended fix:

Add user-scoped throttles for KYC initiation/submission/upload and test repeated requests.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/kyc-page.js` | Valid JS | Exit 0 | Pass |
| Backend format | `cd backend && cargo fmt --check` | Rust formatting is clean | Exit 0 | Pass |
| Targeted static regression | `python3 -m pytest tests/test_kyc_identity_static.py -q` | KYC static contract checks pass | 7 passed | Pass |
| Backend build | `cd backend && cargo check` | Build succeeds | Blocked by concurrent Cargo build activity in shared workspace | Blocked |
| `/kyc` curl smoke | `curl -I --max-time 3 http://localhost:8888/kyc` | Redirect to login or 200 with auth | Curl exit 7, no server listening | Blocked |
| `/api/kyc/status` unauth curl | `curl -i --max-time 3 http://localhost:8888/api/kyc/status` | 401 JSON | Curl exit 7, no server listening | Blocked |
| `/api/kyc/initiate` unauth POST | `curl -i --max-time 3 -X POST ...` | 401/403 | Curl exit 7, no server listening | Blocked |
| Browser manual upload | Not run | File upload succeeds with CSRF and creates `kyc_documents` | Static review predicts 403 due missing XHR CSRF | Needs recheck |
| Authenticated provider redirect | Not run | Start button creates session and redirects | Not verified | Needs recheck |

---

## Security Findings

- P1: Manual KYC accepts direct backend submissions without a linked identity document.
- P1: Manual document upload is wired in a way that omits CSRF, blocking legitimate users and leaving the page unverified in the real browser flow.
- P2: Sensitive KYC audit/email side effects are best-effort instead of durable.
- P2: KYC initiation/submission/upload endpoints lack endpoint-specific abuse throttling.
- Positive: Page and APIs require authenticated sessions, webhook is excluded from CSRF only under `/api/webhooks/` and validates provider signatures internally, uploads use magic-byte sniffing and private GCS paths.

---

## Database Findings

- `kyc_records` exists with status constraints and indexes.
- `kyc_documents` exists with document type/status constraints and user/status indexes.
- Manual KYC has no durable storage for DOB, nationality, address, city, or country despite collecting and posting them.
- Manual submit does not transactionally link exactly one `kyc_documents` row to the new `kyc_records` row.
- KYC document upload persists `gcs_path` only after successful private upload, but audit logging is swallowed.

---

## Missing Tests

- Browser E2E for `/kyc` manual flow: page load, status/provider fetch, validation steps, file upload with CSRF, submit, pending state, and console/network health.
- API test for `POST /api/upload/kyc` proving no-CSRF is 403 and valid CSRF creates a private document row.
- API test for `POST /api/kyc/submit` rejecting missing/foreign/already-linked `document_id`.
- API test proving manual KYC persists all required personal/address fields or rejects unsupported fields.
- API test for provider return flow so localStorage cannot falsely display `in_review`.
- Rate-limit tests for initiate, submit, and upload.

---

## Recommended Fix Order

1. Fix KYC document upload CSRF and add targeted browser/API coverage.
2. Make manual KYC submit transactional and require a linked user-owned document.
3. Persist or intentionally remove manual identity/address fields.
4. Make audit logging durable for KYC mutations and document upload.
5. Add KYC rate limits and provider-return status verification.
6. Normalize document type constants and file accept hints.

---

## Final Status

`fixed_needs_recheck`

Reason: The audited KYC code issues have targeted fixes and static regression coverage, but authenticated browser/API E2E is still required. Remaining documented issues are durable email outbox/retry support and cleanup for private storage objects uploaded before a later DB/audit failure.
