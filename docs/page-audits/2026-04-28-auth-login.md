# Page Audit: Login

Date: 2026-04-28
Status: fixed; browser E2E added, runtime recheck pending
Auditor: ChatGPT/Codex
Page URL: `/auth/login`
Template: `frontend/platform/login.html`
JavaScript: inline scripts in `frontend/platform/login.html`; shared HTMX/CSRF setup in `frontend/platform/components/auth-head.html`
CSS: `frontend/platform/static/css/login.css`, `frontend/platform/static/css/dashboard-tokens.css`
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`, `backend/src/auth/csrf.rs`

---

## Summary

The login page renders, has real backend support, enforces CSRF, rate-limits by IP and normalized email, verifies Argon2id password hashes, blocks unverified email accounts, creates HTTP-only session cookies, redirects 2FA-enabled users to `/auth/2fa`, rotates the CSRF cookie after login, and records a successful login audit event.

Follow-up fix on 2026-04-28 resolved the three documented findings: failed-login telemetry no longer sends submitted email addresses, icon-only controls now expose accessible names/state, and the testimonial carousel no longer writes quote content with `innerHTML`.

Remaining issues are verification gaps only: committed browser E2E coverage now exists for successful login, remember-me expiry, TOTP redirect, OAuth-disabled UI, keyboard/mobile controls, CSRF feedback, and console health, but the suite still needs to run against a live backend once the shared worktree can start cleanly.

---

## Tested Scope

- Reviewed `frontend/platform/login.html`, including form controls, links, inline carousel code, password toggle, HTMX lifecycle handlers, and CSRF hidden-input hydration.
- Reviewed shared auth head `frontend/platform/components/auth-head.html` for HTMX, CSRF, Sentry, and CSS dependencies.
- Reviewed backend `GET/POST /auth/login` handlers in `backend/src/auth/routes.rs`.
- Reviewed authentication/session logic in `backend/src/auth/service.rs`.
- Reviewed CSRF middleware in `backend/src/auth/csrf.rs`.
- Reviewed database dependencies in `database/001_initial_schema.sql`, `database/026_2fa_session_column.sql`, and `docs/DATABASE_SCHEMA.md`.
- Reviewed existing auth smoke tests in `tests/test_auth_login_register.py`, `tests/test_platform.py`, and `tests/e2e/pages/login_page.py`.
- Ran local HTTP smoke checks against a temporary backend on `http://127.0.0.1:8899`.
- Follow-up fix changed `backend/src/auth/service.rs`, `backend/src/auth/routes.rs`, and `frontend/platform/login.html`.
- Added static regression coverage in `tests/test_auth_login_hardening_static.py`.
- Added browser regression coverage in `tests/e2e/test_auth_login.py`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/login` | Public login page; redirects authenticated users to `/marketplace` |
| Template | `frontend/platform/login.html` | Login form, Google login, password toggle, carousel |
| Component | `frontend/platform/components/auth-head.html` | HTMX, CSRF request header hook, Sentry browser client, auth CSS |
| CSS | `frontend/platform/static/css/login.css` | Auth layout, form controls, testimonial panel, responsive styles |
| Backend page route | `GET /auth/login` | Renders login template with CSRF token and optional error |
| Backend form route | `POST /auth/login` | Rate-limits, authenticates, creates session, redirects |
| OAuth route | `GET /auth/google` | Linked from Google button |
| Forgot password route | `GET /auth/forgot-password` | Linked from form |
| Signup route | `GET /auth/signup` | Linked from form |
| Database table | `users` | Email, password hash, email verification, active status |
| Database table | `user_sessions` | Session token, remember flag, 2FA verification flag, expiry |
| Database table | `user_settings` | TOTP enabled flag used to choose `/auth/2fa` redirect |
| Database table | `audit_logs` | Successful login audit event |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Result |
|--------|---------------------|-------------------|-----------------|----------------|--------|
| Back link | `#back-link[href="/"]` | Navigate to platform root | Yes | Yes | Present |
| Logo | `#logo-pool img` | Show POOOL logo | Yes | Static asset | Present |
| Error area | `#auth-error[aria-live=polite]` | Display login errors | Yes | Yes | Invalid login returned safe HTML alert |
| Google button | `#google-button` | Navigate to `/auth/google` | Inline click | Yes | Route exists; not submitted in audit |
| Login form | `#login-form` | HTMX/form POST to `/auth/login` | Yes | Yes | CSRF enforced; invalid login handled |
| CSRF hidden input | `#csrf-token-input[name=csrf_token]` | Submit double-submit token | Yes | Yes | GET minted `csrf_token` cookie |
| Email input | `#email-input[name=email]` | Required email, autocomplete | Yes | Yes | Present |
| Password input | `#password-input[name=password]` | Required password, autocomplete | Yes | Yes | Present |
| Password visibility button | `#toggle-password` | Toggle password field type | Inline click | No backend needed | Fixed: exposes `aria-label` and `aria-pressed` |
| Remember checkbox | `#remember-checkbox[name=remember]` | Extend session lifetime to 30 days | Yes | Yes | Backend maps checked value to `remember_me` |
| Forgot password link | `#forgot-password-link[href="/auth/forgot-password"]` | Navigate to reset request page | Yes | Yes | Route exists |
| Submit button | `#login-button` | Disable and show `Logging in...` during HTMX request | Yes | Yes | Fixed: exposes `aria-busy` |
| Signup link | `#signup-link[href="/auth/signup"]` | Navigate to signup page | Yes | Yes | Route exists |
| Testimonial image | `#testimonial-image` | Change with carousel | Inline JS | Static assets | Present |
| Carousel arrows | `#prev-arrow`, `#next-arrow` | Change testimonial | Inline JS | No backend needed | Fixed: accessible names added |

---

## Findings

### P1 - Failed Login Monitoring Leaks Email Addresses

Location:

- `backend/src/auth/service.rs:168`
- `backend/src/auth/service.rs:189`
- `backend/src/auth/service.rs:209`
- `backend/src/auth/service.rs:227`
- `backend/src/auth/service.rs:234`

Problem:

Failed-login paths capture the submitted email address in Sentry message text and, for wrong passwords, set it on the Sentry user context. Login attempts are high-volume and can include personal data for real users, mistyped addresses, or attacker-supplied third-party addresses. This conflicts with the production-readiness privacy standard to avoid leaking personal data in external monitoring.

Expected:

Failed-login telemetry should preserve operational signal without raw email addresses. Use user id only when a verified account exists, hash or redact the email bucket, and keep the reason tag plus request metadata that is safe for incident response.

Evidence:

Static review shows messages such as `Failed login: unknown email {email}` and `Failed login: wrong password for {email}`. The invalid-login runtime smoke returned a generic user-facing message, so the user-facing path is safe; the leak is in backend observability.

Recommended fix:

Replace raw email strings in failed-login Sentry messages with a stable non-reversible hash or `redacted`; avoid `scope.set_user.email` for failed authentication. Keep reason tags and user id only after a known active user row is found.

Fix applied:

`backend/src/auth/service.rs` now sends static failed-login Sentry messages without raw submitted email addresses. Wrong-password and OAuth-only account paths keep the known user id only. `backend/src/auth/routes.rs` also removed the raw normalized email from the login rate-limit warning.

### P3 - Icon-Only Login Controls Lack Accessible Names And State

Location:

- `frontend/platform/login.html:64`
- `frontend/platform/login.html:126`
- `frontend/platform/login.html:129`
- `frontend/platform/login.html:209`

Problem:

The password visibility toggle and carousel arrow buttons are icon-only buttons without `aria-label`. The password toggle also does not expose `aria-pressed` or update an accessible label when the password is visible. During submission, the button is disabled and text changes, but no `aria-busy` state is exposed on the form or button.

Expected:

Icon-only buttons should have accessible names. Stateful toggles should expose their state. Submit lifecycle should expose busy state for assistive technology.

Evidence:

Static template review. Browser/a11y automation was not run in this audit.

Recommended fix:

Add `aria-label="Show password"` / `aria-label="Hide password"` and `aria-pressed` to the password toggle. Add `aria-label="Previous testimonial"` and `aria-label="Next testimonial"` to carousel arrows. Set and clear `aria-busy` during HTMX requests.

Fix applied:

`frontend/platform/login.html` now labels the password toggle and carousel arrows, updates password toggle `aria-pressed` / `aria-label`, and sets `aria-busy` on the form and submit button during HTMX login requests.

### P3 - Testimonial Carousel Uses `innerHTML` For Quote Updates

Location:

- `frontend/platform/login.html:154`
- `frontend/platform/login.html:186`

Problem:

The carousel quote list is currently developer-controlled, but the render path writes quote content with `innerHTML` to support highlighted brand spans. If these quotes later become CMS/admin-configurable, this turns into a direct XSS sink on a public authentication page.

Expected:

Public auth pages should avoid HTML sinks unless content is sanitized or constructed with DOM APIs.

Evidence:

Static review of the `team` array and `show()` function.

Recommended fix:

Render the quote with `textContent`, or build the highlighted `POOOL` spans with DOM nodes from trusted tokenized data. If HTML must remain, keep it explicitly sanitized and document the trusted source.

Fix applied:

`frontend/platform/login.html` now stores testimonial quotes as plain strings and updates the quote element with `textContent`.

---

## Backend And Data Integrity Notes

- `POST /auth/login` checks IP and per-email rate limits before Argon2 work.
- Unknown email and wrong password return the same user-facing `Invalid email or password.` message.
- Unknown-email authentication runs a dummy Argon2 verification to reduce timing enumeration.
- Email verification is enforced before session creation.
- Session cookie uses `HttpOnly`, `SameSite=Lax`, environment-aware `Secure`, and 24-hour or 30-day expiry based on `remember`.
- Sessions created before TOTP completion are marked `is_2fa_verified = false` and redirect to `/auth/2fa`.
- Login rotates the CSRF cookie after session creation.
- The login audit log is best-effort via `.ok()`. This is acceptable for login telemetry, but higher-risk financial/admin mutations should not follow this pattern.

---

## Runtime Checks

| Check | Command | Result |
|-------|---------|--------|
| Existing server check | `curl -I --max-time 5 http://127.0.0.1:8888/auth/login` | Failed to connect; no server was listening on `:8888` |
| Temporary backend | `SERVER_PORT=8899 PORT=8899 POOOL_ENV=local cargo run` | Started; startup printed existing local migration/seed errors but served requests |
| Login page GET | `curl -i http://127.0.0.1:8899/auth/login` | `200 OK`, rendered login HTML, set `csrf_token` cookie |
| CSRF-negative POST | `curl -i -X POST http://127.0.0.1:8899/auth/login ...` without token | `403 Forbidden`, empty body |
| Invalid login with CSRF | GET cookie then HTMX `POST /auth/login` with invalid credentials | `401 Unauthorized`, returned `<div class="auth-error-message" role="alert">Invalid email or password.</div>` |
| Inline JS syntax | `node --check` against both inline scripts extracted from `login.html` | Passed |
| Targeted Rust tests | `cd backend && cargo test auth::rate_limit --quiet` | Passed 3 tests; filter also produced one crate target with 0 matching tests |
| Follow-up static regression | `python3 -m pytest tests/test_auth_login_hardening_static.py -q` | Passed, 3 tests |
| Auth login E2E syntax | `python3 -m py_compile tests/e2e/test_auth_login.py` | Passed |
| Auth login browser E2E | `python3 -m pytest tests/e2e/test_auth_login.py -q` | Blocked before test execution because no backend was reachable at `http://localhost:8888/health` |
| Follow-up inline JS syntax | Extracted both inline scripts from `login.html` and ran `node --check` | Passed |
| Follow-up unsafe-pattern scan | `rg` for raw failed-login email patterns and script `.innerHTML` | Passed; no matches |
| Follow-up targeted rustfmt | `cd backend && rustfmt --edition 2021 --check src/auth/routes.rs src/auth/service.rs` | Passed after formatting the touched auth files |
| Broad Rust test compile | `cd backend && cargo test auth::rate_limit --quiet` after fixes | Blocked by existing shared-worktree compile errors unrelated to the login patch: missing `aes_gcm` dependency in auth 2FA work, asset status filter type mismatch, rewards `query_scalar` type inference, and missing `PropertyDisplayData.area` |
| Broad cargo check | `cd backend && cargo check --quiet` | Could not complete reliably in this run; it produced no diagnostics before the process exited after a long quiet wait |

---

## Final Status

`fixed; browser E2E added, runtime recheck pending`

All documented login implementation findings are fixed, and the browser coverage gap now has a committed Playwright regression file. Remaining work is runtime verification: run `tests/e2e/test_auth_login.py` after the backend is reachable on `localhost:8888`.
