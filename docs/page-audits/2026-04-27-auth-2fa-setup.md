# Page Audit: 2FA setup

Date: 2026-04-27
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/auth/2fa/setup`
Template: `frontend/platform/auth-2fa-setup.html`
JavaScript: none page-specific; shared HTMX/CSRF logic in `frontend/platform/components/head.html`
CSS: `frontend/platform/static/css/login.css`, shared bundle
Backend Routes: `backend/src/auth/routes.rs`

---

## Summary

The 2FA setup page has a real route, real TOTP generation, CSRF coverage through the shared HTMX hook, and persistence through `user_settings`.

2026-04-28 fix pass: the original seven findings were addressed in code. Existing enrolled users are blocked from replacing TOTP through `/auth/2fa/setup`, newly stored secrets are AES-GCM encrypted, setup uses an encrypted user-bound expiring token instead of a hidden raw secret, setup verification is rate-limited and replay-guarded, HTMX errors/loading/accessibility states were improved, and QR generation failures now propagate.

Remaining issues are operational/test coverage items: production must set `TOTP_SECRET_ENCRYPTION_KEY`, legacy plaintext `user_settings.totp_secret` rows need a migration or re-enrollment plan, and the full setup flow still needs authenticated browser/API recheck.

---

## Tested Scope

- Reviewed `frontend/platform/auth-2fa-setup.html`.
- Reviewed shared head/HTMX/CSRF behavior in `frontend/platform/components/head.html`.
- Reviewed route registration and handlers in `backend/src/auth/routes.rs`.
- Reviewed TOTP helpers and persistence in `backend/src/auth/service.rs`.
- Reviewed `user_settings` and `user_sessions.is_2fa_verified` schema references in `docs/DATABASE_SCHEMA.md`, `database/001_initial_schema.sql`, and `database/026_2fa_session_column.sql`.
- Searched tests for 2FA setup coverage.

Runtime browser testing was not performed because this audit was limited to documentation and no authenticated TOTP enrollment fixture was available in the run.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/2fa/setup` | Authenticated page for TOTP enrollment |
| Template | `frontend/platform/auth-2fa-setup.html` | QR code, manual secret, verification form |
| Shared component | `frontend/platform/components/head.html` | Loads HTMX, injects CSRF header for mutating HTMX requests |
| Backend page route | `GET /auth/2fa/setup` | `totp_setup_page` generates secret, QR, and renders template |
| Backend form route | `POST /auth/2fa/setup` | `totp_setup_submit` verifies submitted code, enables TOTP, rotates session |
| Backend service | `backend/src/auth/service.rs` | `generate_totp_secret`, `verify_totp_code`, `enable_totp` |
| Database table | `user_settings` | Stores `totp_secret`, `totp_enabled` |
| Database table | `user_sessions` | Stores `is_2fa_verified`; session token is rotated after enrollment |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| POOOL logo | `#logo-pool img`, lines 10-11 | Display brand mark | Static asset reference | No backend needed | Static review only |
| QR code | `.qr-code`, line 30 | Show generated TOTP provisioning QR | Server-rendered data URI | `generate_totp_secret()` provides QR base64 | Unverified in browser |
| Manual secret | `.secret-box`, lines 38-40 | Let user enter secret manually in authenticator app | Server-rendered text | `generate_totp_secret()` provides base32 secret | Unverified in browser |
| Error target | `#auth-error`, line 51 | Display invalid code/session/setup errors | HTMX target with live-region semantics | Setup handler now returns escaped HTML for expected form errors | Needs browser recheck |
| Setup form | `form[hx-post="/auth/2fa/setup"]`, lines 52-54 | Submit TOTP code and enable 2FA | HTMX submit, shared CSRF hook | `POST /auth/2fa/setup` exists | Static route verified |
| Setup token | `input[name="setup_token"]`, line 57 | Submit encrypted user-bound expiring enrollment token | Browser-submitted encrypted value | Backend decrypts and validates token user id | Static fixed |
| Code input | `input[name="code"]`, lines 61-63 | Accept a 6-digit authenticator code | HTML pattern/required, label, autocomplete | Replay-guarded TOTP verification | Needs browser recheck |
| Submit button | `#login-button`, lines 68-73 | Enable 2FA and redirect to marketplace | HTMX submit with indicator/disabled state | Backend sends `HX-Redirect: /marketplace` | Needs browser recheck |
| Logout link | `#signup-link[href="/logout"]`, line 75 | Let user abandon setup and end session | Normal link navigation | `GET /auth/logout` exists in router | Static route verified |

---

## Frontend Findings

### P2 - Invalid setup errors render as JSON or global HTMX errors

Fix status: fixed in code on 2026-04-28; authenticated browser recheck still required.

Location:

- Template: `frontend/platform/auth-2fa-setup.html:51-54`
- Shared head: `frontend/platform/components/head.html:190`
- Backend: `backend/src/error.rs:180-186`

Problem:

The form uses `hx-ext="response-targets"` and `hx-target-400="#auth-error"`, but the shared head loads only core HTMX. `AppError` responses are JSON, so invalid TOTP setup codes are not shaped as the auth error HTML expected by `#auth-error`.

Expected:

Invalid codes, expired sessions, CSRF failures, and server errors should render a visible, accessible page-level error without requiring console inspection.

Evidence:

The setup page declares `hx-ext="response-targets"` and `hx-target-400`, while `components/head.html` only loads `https://unpkg.com/htmx.org@1.9.10`. `AppError::into_response()` emits `Json({"error": ...})`.

Recommended fix:

Either load the response-targets extension where used and return auth-safe HTML fragments for HTMX auth forms, or remove the extension dependency and add a local `htmx:responseError` handler that renders escaped error text into `#auth-error`.

### P2 - Code input has weak accessibility metadata

Fix status: fixed in template/CSS on 2026-04-28; mobile/screen-reader recheck still required.

Location:

- Template: `frontend/platform/auth-2fa-setup.html:51-60`

Problem:

The code input has no visible or programmatic `<label>`, no `autocomplete="one-time-code"`, and the error container has no `role="alert"` or `aria-live` semantics.

Expected:

The verification input should be labeled, mobile/TOTP managers should be hinted with one-time-code autocomplete, and errors should be announced.

Evidence:

The input is wrapped in generic `div` elements and only has a placeholder.

Recommended fix:

Add a label tied to the input, `autocomplete="one-time-code"`, `maxlength="6"`, `aria-describedby`, and `role="alert" aria-live="polite"` on the error container.

### P2 - Submit has no real loading or duplicate-submit state

Fix status: fixed in template/CSS on 2026-04-28; browser recheck still required.

Location:

- Template: `frontend/platform/auth-2fa-setup.html:52-69`

Problem:

The form references `hx-indicator="#loading-indicator"`, but no `#loading-indicator` exists. The submit button is not disabled while a request is in flight.

Expected:

Submitting should show progress, prevent duplicate clicks, and restore state on error.

Evidence:

No matching loading indicator appears in the template, and there is no page JS for submit lifecycle handling.

Recommended fix:

Add a real indicator and use HTMX events or `hx-disabled-elt` to disable the submit button during setup verification.

---

## Backend Findings

### P1 - Existing TOTP can be replaced without current-password or current-TOTP step-up

Fix status: fixed in code on 2026-04-28 by blocking setup when `totp_enabled=true`; authenticated enabled-2FA fixture recheck still required.

Location:

- Backend: `backend/src/auth/routes.rs:351-407`
- Backend: `backend/src/auth/service.rs:915-922`

Problem:

Any authenticated session that can reach `/auth/2fa/setup` can generate a new secret and overwrite `user_settings.totp_secret`. The handler does not check whether TOTP is already enabled, does not require current-password confirmation, and does not require current TOTP step-up before replacing an existing second factor.

Expected:

Initial enrollment may use the logged-in session, but replacing an existing factor should require a fresh step-up using current password/current TOTP or an explicit recovery flow.

Evidence:

`totp_setup_page` uses `get_user_by_session_unverified()` and always generates a fresh secret. `totp_setup_submit` calls `enable_totp(&state.db, user.id, &form.secret)` unconditionally.

Recommended fix:

Before rendering or enabling setup, read `user_settings.totp_enabled`. If enabled, redirect to settings or require a dedicated re-enrollment flow with password + current TOTP verification, audit logging, and session revocation/rotation.

### P1 - TOTP secret is stored directly in `user_settings`

Fix status: fixed for new enrollments on 2026-04-28 with AES-GCM encrypted values. Remaining issue: existing legacy plaintext rows need migration or forced re-enrollment, and production must configure `TOTP_SECRET_ENCRYPTION_KEY`.

Location:

- Backend: `backend/src/auth/service.rs:915-922`
- Schema docs: `docs/DATABASE_SCHEMA.md:502`
- Migration: `database/001_initial_schema.sql:433`

Problem:

`enable_totp()` stores the base32 TOTP secret directly in `user_settings.totp_secret`. The schema docs mark the field as encrypted, but the implementation does not show encryption before persistence.

Expected:

TOTP secrets should be encrypted at rest with a server-side key/KMS envelope before DB storage, with safe key rotation and no secret leakage in logs or reports.

Evidence:

The SQL update binds `secret_b32` directly into `totp_secret`.

Recommended fix:

Introduce an encryption/decryption helper for TOTP secrets, migrate existing plaintext values carefully, and keep verification code decrypting only in memory.

### P1 - Setup verification lacks rate limiting and replay protection

Fix status: fixed in code on 2026-04-28 with IP/user setup rate limits and the existing Redis-backed replay guard; Redis-backed replay behavior still needs runtime recheck.

Location:

- Backend: `backend/src/auth/routes.rs:384-407`
- Backend: `backend/src/auth/service.rs:833-852`
- Backend: `backend/src/auth/service.rs:864-912`

Problem:

`POST /auth/2fa/setup` verifies with `verify_totp_code()` rather than `verify_totp_code_with_replay_guard()` and does not call `auth_rate_limiter`. Repeated invalid attempts against a live setup session are not throttled, and a valid code can be resubmitted within the TOTP window.

Expected:

Setup verification should be rate-limited per user/session/IP and should use replay protection consistent with login and step-up verification.

Evidence:

The setup submit handler has no `auth_rate_limiter.check()` call, while login/forgot/signup handlers do. The replay guard helper exists but is not used here.

Recommended fix:

Add a setup-specific rate-limit key such as `totp_setup:{user_id}` plus optional IP key, and use `verify_totp_code_with_replay_guard(state.redis.as_ref(), user.id, &form.secret, &form.code).await`.

### P2 - Enrollment trusts a client-submitted secret instead of a server-side pending secret

Fix status: fixed in code on 2026-04-28 by replacing the hidden raw secret with an encrypted, user-bound, 10-minute `setup_token`.

Location:

- Template: `frontend/platform/auth-2fa-setup.html:55`
- Backend: `backend/src/auth/routes.rs:365-407`

Problem:

The generated secret is sent back to the server from a hidden form field and then stored if the submitted code matches that submitted secret. There is no server-side pending enrollment record binding the generated secret to the session.

Expected:

The server should store a pending enrollment secret server-side, or sign/encrypt the pending secret with tamper detection before sending it to the browser.

Evidence:

`totp_setup_page` generates `secret`; the template renders it into both visible text and a hidden input; `totp_setup_submit` verifies and stores `form.secret`.

Recommended fix:

Persist pending setup state server-side with expiry, or use an authenticated encrypted token for the secret. On submit, verify the code against the server-held pending secret.

### P3 - Empty QR generation can fail silently

Fix status: fixed in code on 2026-04-28 by propagating QR generation errors.

Location:

- Backend: `backend/src/auth/service.rs:829`
- Template: `frontend/platform/auth-2fa-setup.html:30`

Problem:

`totp.get_qr_base64().unwrap_or_default()` can produce an empty image data URI without surfacing a setup error.

Expected:

QR generation failure should return a safe backend error or the UI should explicitly fall back to manual setup.

Evidence:

The QR value defaults to `""`, and the template always renders `data:image/png;base64,{{ qr_code }}`.

Recommended fix:

Propagate QR generation failure or conditionally render a clear manual-entry-only state.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route mapping | Inspected `auth::router()` | GET and POST `/auth/2fa/setup` registered | Both route methods registered | Pass |
| Static form contract | Compared form action/fields to handler | Form posts `secret` and `code` to backend | Handler expects `TotpSetupForm` and verifies code | Pass with security caveat |
| CSRF static check | Inspected shared HTMX header injection and CSRF middleware | Mutating HTMX request sends `X-CSRF-Token` | Shared `htmx:configRequest` adds token from cookie | Pass static |
| Invalid code UX | Compared HTMX target with backend error response | Error appears in `#auth-error` as HTML | Backend emits JSON and extension is not loaded | Fail |
| Browser enrollment | Authenticated browser setup with TOTP fixture | QR renders, invalid/valid code behavior verified | Not run; no authenticated TOTP fixture in this run | Not run |

---

## Security Findings

- P1: Existing TOTP enrollment can be overwritten from an authenticated session without current password/current TOTP step-up.
- P1: TOTP secrets appear to be stored plaintext despite schema documentation saying encrypted.
- P1: Setup verification lacks rate limiting and does not use the existing replay guard helper.
- P2: Hidden/client-submitted enrollment secret is trusted as the source of truth.
- P2: Error response shape can hide setup failures from users.

---

## Database Findings

- `user_settings.totp_secret` and `totp_enabled` exist in schema and migrations.
- `user_sessions.is_2fa_verified` exists and the setup handler rotates the session token after enrollment.
- No audit log or event table write was identified for TOTP enrollment or replacement.
- No pending TOTP enrollment table/column was identified for server-side setup state.

---

## Missing Tests

- Authenticated GET `/auth/2fa/setup` renders QR, manual secret, encrypted setup token form, and CSRF-compatible setup state.
- POST `/auth/2fa/setup` rejects invalid codes with visible HTMX error HTML.
- POST `/auth/2fa/setup` rate-limits repeated invalid attempts.
- POST `/auth/2fa/setup` prevents replay of a used code where Redis is available.
- Existing `totp_enabled=true` users cannot replace their TOTP secret through the setup route.
- Production startup/deployment validation ensures `TOTP_SECRET_ENCRYPTION_KEY` is configured before 2FA setup is used.
- Legacy plaintext TOTP secrets are migrated or users are forced through a safe re-enrollment flow.
- Browser E2E for setup from settings, valid code redirect, invalid code feedback, keyboard navigation, and mobile layout.

---

## Recommended Fix Order

1. Configure `TOTP_SECRET_ENCRYPTION_KEY` in every environment before enabling new 2FA setup.
2. Decide and execute the legacy plaintext `totp_secret` migration or forced re-enrollment plan.
3. Add authenticated setup E2E and API tests for valid setup, invalid code, replay, rate limit, and enabled-2FA replacement block.
4. Recheck mobile and keyboard behavior for the updated form states.

---

## Final Status

`needs_recheck`

Reason: Original code issues have been fixed, but the page still needs authenticated runtime recheck, production encryption-key configuration, and a legacy plaintext-secret migration/re-enrollment plan.
