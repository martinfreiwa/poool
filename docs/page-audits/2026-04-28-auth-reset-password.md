# Page Audit: Reset Password

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/auth/reset-password`
Template: `frontend/platform/reset-password.html`
JavaScript: inline HTMX lifecycle handlers in `frontend/platform/reset-password.html`
CSS: `frontend/platform/static/css/login.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`

---

## Summary

The reset-password page is backed by a real HTMX form route and the critical backend reset operation is implemented safely: reset tokens are hashed, the token lookup requires unused and unexpired rows, password hashing uses Argon2id, token consumption and session invalidation run inside one database transaction, and CSRF rejects raw POSTs.

Follow-up fixes were applied after the audit: the reset token is scrubbed from the browser URL, reset submit is rate-limited by IP and token bucket, HTMX errors render escaped HTML fragments, and missing-token/password-requirement UX is explicit. The reset-password code issues are fixed. `cargo check`, the targeted reset-password test filter, file-level formatting, and frontend script syntax now pass. A full browser/server reset-success E2E remains a runtime verification follow-up because `cargo run` is currently blocked by a local Cargo artifact lock in this workspace.

---

## Fix Update - 2026-04-28

Fixed locally:

- `PAGE-ISSUE-0451`: `frontend/platform/reset-password.html` now copies the query token into the hidden input and immediately removes it from `window.location` with `history.replaceState`.
- `PAGE-ISSUE-0452`: `backend/src/auth/routes.rs::reset_password_submit` now applies the shared auth rate limiter by trusted client IP and submitted token hash bucket before token lookup.
- `PAGE-ISSUE-0453`: reset-password failures now return escaped HTML auth-error fragments for HTMX instead of raw JSON.
- `PAGE-ISSUE-0454`: the visible password requirement copy now matches backend validation, `#auth-error` is a live region, and missing-token links disable submit with a visible alert.

Remaining verification issue:

- Full browser/server reset-success/reuse/session-invalidation E2E still needs to run with a safe fixture user. Code-level verification now passes, but starting the local server with `cargo run` is blocked by a local Cargo artifact lock in this workspace.

---

## Tested Scope

- Reviewed `frontend/platform/reset-password.html` template, form controls, inline token copy, and HTMX lifecycle handlers.
- Reviewed shared head behavior from `frontend/platform/components/head.html`, including HTMX and CSRF injection.
- Reviewed `backend/src/auth/routes.rs` for `GET/POST /auth/reset-password`.
- Reviewed `backend/src/auth/service.rs` password reset service logic.
- Reviewed `backend/src/common/validation.rs` password requirements.
- Reviewed `password_reset_tokens` schema in `database/001_initial_schema.sql` and `docs/DATABASE_SCHEMA.md`.
- Ran local HTTP smoke checks against `http://127.0.0.1:8888/auth/reset-password`.
- Checked for committed reset-password tests.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/reset-password` | Public page reached from emailed reset links |
| Template | `frontend/platform/reset-password.html` | Password form and inline HTMX handlers |
| Shared head | `frontend/platform/components/head.html` | Loads HTMX, injects CSRF headers, initializes Sentry |
| CSS | `frontend/platform/static/css/login.css` | Shared auth-page layout and button styles |
| Backend page route | `GET /auth/reset-password` | Renders the MiniJinja template |
| Backend form route | `POST /auth/reset-password` | Validates token/password and redirects to login on success |
| Service | `backend/src/auth/service.rs::reset_password` | Validates password, updates password, consumes tokens, deletes sessions in one transaction |
| Database table | `password_reset_tokens` | Stores `token_hash`, `expires_at`, `used_at`, and user FK |
| Database table | `user_sessions` | Existing sessions are deleted after reset |
| Database table | `users` | `password_hash` is updated |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Logo | `#logo-pool img` | Render POOOL logo | Yes | Static asset | Not exhaustively checked; page HTML rendered |
| Heading | `#text` | Tell user they are setting a new password | Yes | No backend needed | Rendered |
| Supporting copy | `#supporting-text` | Explain new password must differ from previous passwords | Static only | Partially; backend enforces strength but not previous-password difference explicitly in reviewed code | Copy overpromises previous-password enforcement |
| Error container | `#auth-error` | Show validation/reset failures | Yes | Backend returns escaped HTML fragments after fix | Fixed locally; runtime browser/server recheck still pending |
| Reset form | `#reset-password-form[hx-post="/auth/reset-password"]` | Submit token, password, and confirmation via HTMX | Yes | Yes | Safe invalid-token and mismatch submissions returned 400 |
| Hidden token input | `#reset-token[name="token"]` | Copy `token` query parameter into POST body and scrub URL | Yes, inline URLSearchParams script | Yes | Fixed locally with `history.replaceState`; runtime browser/server recheck still pending |
| Password input | `#password-input[name="password"]` | Enter new password | Browser `required`; helper copy now matches backend rules | Backend validates length, uppercase, lowercase, digit, max length | Fixed locally; backend remains authoritative |
| Password hint | inline helper text | Explain requirements | Static copy updated | Backend requires 8-128 chars, uppercase, lowercase, and digit | Fixed locally |
| Confirm password input | `#confirm-password-input[name="confirm_password"]` | Re-enter password | Browser `required`; backend checks equality | Yes | Mismatch returns 400 JSON |
| Submit button | `#login-button` / `#button-text` | Disable during request and show `Resetting...` | Yes | No backend needed | Inline handlers present |
| Back link | `#back-to-login[href="/auth/login"]` | Navigate to login | Yes | `GET /auth/login` exists | Route exists |

---

## Frontend Findings

### P1 - Reset token remains in browser URL after page load

Location:

- Template: `frontend/platform/reset-password.html`
- Shared head: `frontend/platform/components/head.html`

Problem:

The inline script reads `?token=...` and copies it into the hidden form field, but it never removes the token from `window.location`. The shared Sentry hook redacts `token=` before sending error events, but the raw token still remains in browser history, the address bar, screenshots, and any same-origin script access to `window.location`.

Expected:

After copying the token into `#reset-token`, the page should call `history.replaceState` to remove the query token from the visible URL, while preserving the hidden form value for the current submit. The app should also set a conservative `Referrer-Policy` for auth pages if it is not already global.

Evidence:

Static review of `frontend/platform/reset-password.html` shows `params.get("token")` assignment only. Runtime GET used `/auth/reset-password?token=test-token` and the page rendered the form without scrubbing the URL.

Recommended fix:

After assigning the hidden input, replace the current URL with `/auth/reset-password` or a safe tokenless path. Keep Sentry redaction as defense in depth.

### P2 - HTMX reset errors render as raw JSON

Location:

- Template: `frontend/platform/reset-password.html`
- Backend: `backend/src/auth/routes.rs::reset_password_submit`
- Backend: `backend/src/error.rs::IntoResponse`

Problem:

The form targets `#auth-error` and forces swaps for 400 responses, but backend errors are returned by the global `AppError` JSON response. Users get raw JSON such as `{"error":"Passwords do not match."}` inside an auth-page error region instead of a styled, accessible error message.

Expected:

HTMX reset-password errors should return a small HTML fragment with `role="alert"` or the frontend should parse JSON and render safe text. CSRF/rate-limit/provider failures should also produce visible user-safe messages.

Evidence:

Runtime smoke with valid CSRF and mismatched passwords returned `400 Bad Request`, `content-type: application/json`, body `{"error":"Passwords do not match."}`. Invalid token returned `{"error":"Invalid or expired password reset link."}`.

Recommended fix:

Use a reset-password-specific error response helper, similar to the login-page error fragment pattern, or add a global HTMX JSON error renderer that writes `error` via `textContent`.

### P2 - Visible password requirements do not match backend validation

Location:

- Template: `frontend/platform/reset-password.html`
- Backend: `backend/src/common/validation.rs::validate_password`

Problem:

The UI only tells users `Min. 8 characters`. Backend validation also requires at least one uppercase letter, one lowercase letter, one number, and a 128-character maximum. The page does not explain those requirements before submit.

Expected:

The visible password hint should match backend validation, and client-side validation may provide early feedback while leaving backend validation authoritative.

Evidence:

Static review of the template and validation function.

Recommended fix:

Update the hint to list the real requirements and add accessible inline validation feedback for mismatch and strength errors.

### P3 - Missing-token page renders a normal reset form

Location:

- Template: `frontend/platform/reset-password.html`

Problem:

`GET /auth/reset-password` without a query token renders the same form with an empty hidden token. A user can fill both password fields and only then receive a backend `Missing reset token` error, currently as raw JSON.

Expected:

When no token is present, the page should show a safe expired/missing-link state with a link to `/auth/forgot-password`, or disable the submit button until a token is available.

Evidence:

Static review shows `#reset-token` defaults to empty and the script only assigns a value when the query param exists.

Recommended fix:

Render or initialize a token-missing state that directs the user to request a new reset email.

---

## Backend Findings

### P1 - Reset-password submit is not rate-limited

Location:

- Backend: `backend/src/auth/routes.rs::reset_password_submit`

Problem:

`POST /auth/forgot-password` applies IP and email rate limits, but `POST /auth/reset-password` does not apply an IP or token-keyed throttle before hitting the reset-token query path. Reset tokens are high entropy, so brute force is unlikely to succeed, but the endpoint can still be used for token-spraying and repeated DB work.

Expected:

Apply the shared auth rate limiter to reset submissions, keyed by trusted client IP and possibly a hash/prefix of the submitted token. Keep responses generic.

Evidence:

Static route review found rate limiter checks in `forgot_password_submit` and none in `reset_password_submit`.

Recommended fix:

Add a reset-specific rate-limit key before calling `service::reset_password`.

### Positive backend notes

- `service::reset_password` validates password strength before mutation.
- Tokens are compared by hash, not plaintext.
- Token lookup requires `expires_at > NOW()` and `used_at IS NULL`.
- Password update, reset-token consumption, and session deletion run in a single SQL transaction.
- Existing sessions are invalidated after reset.
- No monetary logic is involved.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Page load | `curl -c /tmp/poool-reset-cookies.txt http://127.0.0.1:8888/auth/reset-password?token=test-token` | `200 OK`, form rendered, CSRF cookie set | `200 OK`, form rendered, `csrf_token` cookie set | Pass |
| CSRF rejection | Raw POST without CSRF header to `/auth/reset-password` | Request rejected | `403 Forbidden`, empty body | Pass |
| Password mismatch | POST via HTMX headers with valid CSRF, mismatched password fields | Visible validation error | Pre-fix runtime returned JSON; code now returns escaped HTML fragment | Needs runtime recheck |
| Invalid token | POST via HTMX headers with valid CSRF and invalid token | Visible invalid/expired-link error | Pre-fix runtime returned JSON; code now returns escaped HTML fragment | Needs runtime recheck |
| Success reset | Submit a real unused reset token with test user | Password changed, sessions invalidated, redirect to login | Not run; would mutate account state | Not run |
| Mobile/browser console | Browser viewport and console inspection | No console errors and responsive layout OK | Not run; HTTP-only audit | Not run |

---

## Security Findings

- Fixed: reset tokens are removed from the browser URL after being copied to the hidden input.
- Fixed: reset submissions are rate-limited by IP and submitted-token hash bucket.
- CSRF protection is present and rejects raw POSTs.
- Token storage is hashed and expiry/used checks are enforced.
- Password reset invalidates existing sessions.
- Sentry URL redaction exists for `token=`, but it is defense in depth rather than a substitute for URL scrubbing.

---

## Database Findings

- `password_reset_tokens` exists with `id`, `user_id`, `token_hash`, `expires_at`, `used_at`, and `created_at`.
- `idx_prt_token` exists for token-hash lookup.
- `token_hash` is unique.
- Password reset mutates `users`, `password_reset_tokens`, and `user_sessions` inside one transaction.
- No schema blocker was found for the reset flow.

---

## Missing Tests

- Add an auth E2E test that creates a safe test user and reset token, loads `/auth/reset-password?token=...`, verifies URL scrubbing, submits a new valid password, verifies redirect, verifies old sessions are deleted, and verifies the token cannot be reused.
- Add HTTP tests for CSRF rejection, missing token, invalid/expired token, password mismatch, weak password, and rate limiting.
- Add browser accessibility coverage for focus order, live error announcement, disabled/loading state, mobile layout, and keyboard-only submission.
- Add a regression test that backend HTMX failures return visible HTML error fragments or are rendered safely by frontend code.

---

## Remaining Work

1. Run full reset-password browser/server E2E with safe fixture data: URL scrubbing, missing token, mismatch, weak password, invalid/expired token, valid reset, token reuse rejection, and session invalidation.
2. Add committed regression coverage for reset-submit rate limiting and HTMX HTML error fragments.

---

## Final Status

`needs_recheck`

Reason: The documented code issues are fixed and code-level checks pass; full browser/server reset-success/reuse/session-invalidation E2E remains as a runtime follow-up because `cargo run` is blocked locally by a Cargo artifact lock.
