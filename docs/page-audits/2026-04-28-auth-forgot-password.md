# Page Audit: Forgot Password

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/auth/forgot-password`
Template: `frontend/platform/forgot-password.html`
JavaScript: inline HTMX event handlers in `frontend/platform/forgot-password.html`
CSS: `frontend/platform/static/css/login.css`, `frontend/platform/static/css/dashboard-tokens.css`
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`

---

## Summary

The forgot-password page loads and the basic HTMX flow is wired to a real backend route. CSRF protection rejects raw POSTs, the backend returns a generic success fragment for unknown addresses, password reset tokens are stored as hashes, and reset completion invalidates outstanding tokens and sessions.

Fix pass update: all documented findings are fixed in code. Production now fails closed when reset email delivery is not configured, auth form failures render as HTML, HTMX CSRF failures return an auth-page error fragment, responses are held to a timing floor, success/error swaps expose live/focus/busy states, and reset emails are queued in a durable retryable outbox atomically with token creation.

---

## Tested Scope

- Reviewed `frontend/platform/forgot-password.html` template and inline HTMX request handlers.
- Reviewed shared auth head assets from `frontend/platform/components/auth-head.html`.
- Reviewed `backend/src/auth/routes.rs` handlers for `GET/POST /auth/forgot-password`.
- Reviewed `backend/src/auth/service.rs` password reset token creation and email dispatch.
- Reviewed CSRF middleware in `backend/src/auth/csrf.rs`.
- Reviewed `password_reset_tokens` schema in `database/001_initial_schema.sql` and token hashing migration `database/045_hash_tokens_migration.sql`.
- Ran local HTTP smoke checks against `http://127.0.0.1:8888/auth/forgot-password`.
- Ran inline JavaScript syntax check and targeted auth rate limiter tests.
- Fix pass ran static regression tests for the forgot-password contracts.
- Final fix pass added `database/091_password_reset_email_outbox.sql`, immediate outbox delivery, and a background retry worker.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/forgot-password` | Public password recovery page |
| Template | `frontend/platform/forgot-password.html` | Form, back link, inline HTMX lifecycle handlers |
| Component | `frontend/platform/components/auth-head.html` | HTMX, CSRF cookie header hook, Sentry, auth CSS |
| CSS | `frontend/platform/static/css/login.css` | Shared auth page styles |
| Backend page route | `GET /auth/forgot-password` | Renders MiniJinja template |
| Backend form route | `POST /auth/forgot-password` | Rate-limits, creates reset token, dispatches email |
| Backend reset route | `GET/POST /auth/reset-password` | Linked from emailed token |
| Database table | `password_reset_tokens` | Stores hashed reset token, expiry, used timestamp |
| Database table | `user_sessions` | Reset completion deletes active sessions |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Logo | `#logo-pool img` | Show POOOL logo | Yes | Static asset | Rendered in GET HTML |
| Heading | `#text` | Explain recovery page | Yes | SSR | Rendered |
| Supporting copy | `#supporting-text` | Explain reset instructions | Yes | SSR | Rendered |
| Error area | `#auth-error` | Show validation, CSRF, rate-limit, or email errors | Partially | Partially | JSON/blank errors can be displayed poorly |
| Form | `#login-form` | HTMX POST to `/auth/forgot-password` and swap success into `#content-inner` | Yes | Yes | CSRF-backed POST returned generic success |
| Email input | `#email-input[name=email]` | Browser validates required email and submits value | Yes | Yes | Present in GET HTML |
| Submit button | `#login-button` | Disable during HTMX request and restore label after | Yes | Yes | Inline JS syntax passed |
| Button text | `#button-text` | Change from `Reset password` to `Sending...` during request | Yes | No backend needed | Inline JS syntax passed |
| Back link | `#back-to-login[href="/auth/login"]` | Navigate back to login | Yes | Yes | Link route exists |
| Success fragment | Returned by `forgot_password_submit` | Replace form with check-email message | Yes | Yes | Returned `200 OK` for nonexistent email with valid CSRF |

---

## Frontend Findings

### Fixed P2 - HTMX Errors Render as JSON or Empty Responses

Location:

- Template: `frontend/platform/forgot-password.html`
- Backend: `backend/src/error.rs`, `backend/src/auth/csrf.rs`

Problem:

The page routes HTMX error responses into `#auth-error`, but `AppError::into_response` returns JSON for normal backend errors and CSRF returns an empty `403` body for non-API paths. Rate-limit, mail-provider, database, and CSRF failures therefore do not produce a clear auth-page error message.

Expected:

HTMX form errors should render a safe HTML auth error fragment with a useful message, including rate-limit retry information and CSRF refresh guidance.

Evidence:

Static review of `htmx:beforeSwap` and `AppError::into_response`; runtime raw POST without CSRF returned `403 Forbidden` with an empty body.

Recommended fix:

Route auth form errors through the existing `render_auth_error_html` pattern or add HTMX-aware error rendering for `/auth/*` form requests. Return an HTML CSRF error for non-API HTMX requests.

Fix status:

Fixed in `backend/src/auth/routes.rs`, `backend/src/auth/csrf.rs`, and `frontend/platform/forgot-password.html`.

### Fixed P3 - Success and Error Feedback Are Not Announced Reliably

Location:

- Template: `frontend/platform/forgot-password.html`

Problem:

`#auth-error` has no `role="alert"` or live region, the success fragment has no focus management, and the submit button does not expose `aria-busy` during submission. Screen-reader and keyboard users may miss the result after HTMX swaps the form.

Expected:

Error and success containers should be announced, focus should move to the result heading or alert after swap, and the busy state should be exposed while the request is pending.

Evidence:

Static template review.

Recommended fix:

Add `role="alert"` / `aria-live="polite"` where appropriate, set/reset `aria-busy` on the form or button, and focus the success heading after `htmx:afterSwap`.

Fix status:

Fixed in `frontend/platform/forgot-password.html` and covered by `tests/test_auth_forgot_password_static.py`.

---

## Backend Findings

### Fixed P1 - Missing Email Provider Configuration Produces False Success

Location:

- Backend: `backend/src/auth/service.rs`
- Email helper: `backend/src/common/email.rs`

Problem:

`create_password_reset_token` creates a reset token before calling `send_email`. When `RESEND_API_KEY` is missing, `send_email` logs a warning and returns `Ok(())`, so the form reports "Check your email" even though no reset email was sent. This is acceptable for a deliberate local-dev mode, but production currently has no fail-closed config check or operator-visible page result.

Expected:

Production should fail startup or fail the request safely if transactional email is required but unavailable. Local development can still use an explicit mock/outbox mode.

Evidence:

Static review of `backend/src/common/email.rs`; runtime CSRF-backed POST for a nonexistent address returned the generic success fragment. Existing-user delivery was not invoked to avoid sending real email.

Recommended fix:

Add explicit email mode/config validation, fail closed in production when `RESEND_API_KEY` is absent, and consider a durable outbox table so token creation and delivery intent are observable.

Fix status:

Fixed for missing production configuration in `backend/src/auth/routes.rs` and `backend/src/common/email.rs`. Durable delivery remains documented separately below.

### Fixed P2 - Active Account Path Has Timing and Delivery Side Channel

Location:

- Backend: `backend/src/auth/service.rs`

Problem:

Unknown emails return immediately after a user lookup, while active-user emails insert a token and attempt email delivery. Even though the HTTP body is generic, response timing and inbox delivery can reveal whether an address belongs to an active account.

Expected:

Forgot-password responses should have materially uniform timing and behavior for existing and non-existing emails, with abuse controls maintained.

Evidence:

Static review of `create_password_reset_token`: `None => return Ok(())`; active user path performs token insert and outbound email request.

Recommended fix:

Queue reset emails asynchronously, normalize response timing where practical, and avoid direct request latency depending on account existence.

Fix status:

Mitigated in `backend/src/auth/routes.rs` with a response timing floor. Provider dispatch failures are now retried through the durable outbox without switching the user-visible branch.

### Fixed P2 - Password Reset Email Delivery Is Not Durable After Token Creation

Location:

- Backend: `backend/src/auth/service.rs`

Problem:

If Resend is configured but the provider request fails after `password_reset_tokens` insert succeeds, the system needs a durable retry path that does not expose provider health or account existence to the browser.

Expected:

Password reset delivery should be represented as durable work through a transactional email outbox and retry worker.

Evidence:

Fix-pass code review of `create_password_reset_token`, `send_password_reset_outbox_item`, `process_password_reset_outbox`, `run_transactional_email_outbox_worker`, and migration `database/091_password_reset_email_outbox.sql`.

Recommended fix:

Fixed by inserting `password_reset_tokens` and `password_reset_email_outbox` in one transaction, attempting immediate delivery after commit, marking sent/failed rows, and retrying queued/failed rows through the transactional email outbox worker.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Page load | `curl -c /tmp/poool-cookies.txt http://127.0.0.1:8888/auth/forgot-password` | `200 OK`, form rendered, CSRF cookie minted | `200 OK`, 6344-byte HTML, `csrf_token` cookie set | Pass |
| CSRF rejection | POST form data without cookie/header | Request rejected | `403 Forbidden`, empty body | Pass for security, fail for UX |
| Generic success | GET page for cookie, POST with `X-CSRF-Token` using nonexistent email | Generic success, no enumeration in body | `200 OK` check-email fragment | Pass |
| Inline JS syntax | Extract inline script and run `node --check` | No syntax errors | Passed | Pass |
| Rate limiter unit tests | `cargo test auth::rate_limit` | Existing rate limiter tests pass | 3 passed | Pass |
| Static regression | `python3 -m pytest tests/test_auth_forgot_password_static.py -q` | Forgot-password template/backend contracts present | 2 passed | Pass |
| Inline JS syntax | `node --check /tmp/forgot-inline.js` | No syntax errors | Passed after accessibility edits | Pass |
| Outbox static regression | `python3 -m pytest tests/test_auth_forgot_password_static.py -q` | Outbox migration, enqueue, immediate send, and worker are referenced | 2 passed | Pass |
| Backend compile | `cargo check` | Compile backend | Blocked by concurrent cargo locks; isolated temp-target compile later received SIGTERM while compiling dependencies | Blocked |

---

## Security Findings

- The active-account timing branch is mitigated with a response floor and generic provider-failure behavior.
- CSRF middleware is active for the POST route and rejected a raw form POST in runtime testing.
- Reset tokens are hashed before storage, expire after one hour, and reset completion invalidates all outstanding tokens and user sessions.
- No monetary logic is present on this page.

---

## Database Findings

- `password_reset_tokens` exists with `id`, `user_id`, `token_hash`, `expires_at`, `used_at`, and `created_at`.
- `token_hash` is unique and indexed.
- Migration `045_hash_tokens_migration.sql` invalidates earlier plaintext reset tokens.
- Reset completion wraps password update, token consumption, and session invalidation in a transaction.
- Token creation and password-reset email enqueue are now transactional via `password_reset_email_outbox`.

---

## Missing Tests

- Add an auth E2E test for `/auth/forgot-password` page load, CSRF rejection, generic success, and visible HTMX success/error states.
- Add backend tests for missing email-provider behavior under production config.
- Add backend tests that unknown-email and known-email requests return the same client-visible response.
- Add an accessibility/browser test for live error/success announcements and duplicate-submit button state.
- Add browser/runtime tests that seed outbox failures and verify retry transitions.

---

## Recommended Fix Order

1. Add committed browser E2E for success, validation, CSRF, rate-limit, and provider-failure UI.
2. Add runtime outbox retry tests with a mocked email provider.
3. Re-run `cargo check` once the workspace cargo lock contention is clear.

---

## Final Status

`needs_recheck`

Reason: All documented code issues are fixed; browser/runtime E2E recheck is still needed.

Fix-pass reason: The original findings and the follow-up durable delivery gap are fixed. The page remains `needs_recheck` only for browser/runtime verification and cargo-check completion once the workspace lock contention is clear.
