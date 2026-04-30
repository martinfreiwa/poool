# Page Audit: Signup

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/auth/signup`
Template: `frontend/platform/signup.html`
JavaScript: inline HTMX handlers in `frontend/platform/signup.html`; shared auth head scripts in `frontend/platform/components/auth-head.html`
CSS: `frontend/platform/static/css/login.css`, `frontend/platform/static/css/dashboard-tokens.css`
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`, `backend/src/auth/csrf.rs`

---

## Summary

The signup page renders and the safe negative runtime checks passed for page load, missing terms, invalid email, weak password, and missing CSRF. The core registration path is only partially production-ready: signup immediately creates an authenticated session for an unverified email, while the emailed `/auth/verify-email?token=...` link is not consumed by any route. Several HTMX failure paths also render JSON or blank responses into the visible error container.

Update after fix pass: the identified implementation issues have been addressed in the working tree. Signup now persists legal consent and an email verification token in the same DB transaction as account creation, redirects new users to `/auth/verify-email?sent=1`, blocks normal authenticated access until `email_verified = TRUE`, consumes verification links, and renders HTMX signup/CSRF failures as HTML alert fragments. Remaining work is verification with browser E2E and an optional email outbox/retry worker for production-grade delivery retries.

---

## Tested Scope

- Static review of signup template, shared auth head, auth routes, auth service, CSRF middleware, validation helpers, database migrations, and auth-related tests.
- Runtime curl smoke against a local `cargo run` backend on `http://127.0.0.1:8888`.
- Safe negative form checks only. I did not submit a successful signup to avoid creating a real account and dispatching email from this audit run.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/signup` | Public signup page |
| Alias | `/signup` | Tracker alias; www router redirects to platform domain |
| Template | `frontend/platform/signup.html` | Form, Google signup button, referral notice, inline HTMX handlers |
| Component | `frontend/platform/components/auth-head.html` | Sentry, CSRF HTMX hook, HTMX vendor script, auth CSS |
| CSS | `frontend/platform/static/css/login.css` | Shared login/signup page styling |
| Backend page route | `GET /auth/signup` | `signup_page` renders template |
| Backend form route | `POST /auth/signup` | `signup_submit` creates user/session |
| Backend OAuth route | `GET /auth/google` | Google OAuth redirect |
| Backend verification route | `GET /auth/verify-email` | Renders page only; does not verify token |
| Database table | `users` | New user with `email_verified = false` |
| Database table | `user_profiles` | Created transactionally |
| Database table | `wallets` | Cash and rewards wallets created transactionally |
| Database table | `user_roles` | Investor role assigned transactionally |
| Database table | `user_settings` | Created transactionally |
| Database table | `user_consents` | Terms consent inserted after user transaction, errors swallowed |
| Database table | `email_verification_tokens` | Token insert/send attempted, errors swallowed |
| Database table | `user_sessions` | Session created immediately after signup |
| Database table | `referral_tracking`, `affiliate_referrals` | Optional referral attribution |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| POOOL logo | `#logo-pool img` | Display brand logo | Yes | Static asset | Rendered in GET HTML |
| Error area | `#auth-error` | Receive inline signup errors | Yes, HTMX target | Partially | Terms error renders HTML; AppError and CSRF failures render JSON/blank |
| Google signup | `#google-button` | Navigate to Google OAuth | Inline `onclick` | Yes, `/auth/google` | Runtime local route produced OAuth redirect |
| Divider | `#content-divider` | Visual separator | Static | N/A | Present |
| Referral notice | `{% if referral_code %}` | Show referral code from cookie | SSR only | Cookie read | Static verified; runtime not exercised |
| Signup form | `#login-form` | HTMX POST to `/auth/signup` | Yes | Yes | Safe negative checks worked with caveats |
| Email input | `#email-input` | Required email | Browser validation + backend validation | Yes | Invalid email returned 400 JSON |
| Password input | `#password-input` | Required strong password | Browser `minlength=8`; copy lists backend rules | Yes | Weak password returned 400 JSON |
| Terms checkbox | `#terms-checkbox` | Required acceptance | Browser required + backend guard | Yes | Missing terms returned inline HTML |
| Terms link | `/terms` | Navigate to terms page | Link | Yes | Static verified |
| Privacy link | `/privacy` | Navigate to privacy page | Link | Yes | Static verified |
| Submit button | `#login-button` | Disable and show loading text during HTMX request | Inline HTMX listeners | Yes | Static wired; curl cannot verify visual state |
| Login link | `#signup-link` | Navigate to `/auth/login` | Link | Yes | Static verified |
| Right image panel | `#testimonial-image` | Decorative auth image via CSS | CSS background | Static asset | Not browser-verified |

---

## Frontend Findings

### P2 - HTMX signup failures render JSON or blank error states

Location:

- Template: `frontend/platform/signup.html`
- Shared error handling: `backend/src/error.rs`, `backend/src/auth/csrf.rs`

Problem:

The form targets `#auth-error` and forces error swaps for HTTP 400+ responses, but most backend errors return JSON and CSRF failures return an empty 403 body for non-API routes. Users can see raw `{"error":"..."}` text or no visible error at all.

Expected:

All signup failures should return accessible HTML fragments for HTMX page submissions, with `role="alert"` or an equivalent live-region pattern.

Evidence:

Runtime curl confirmed invalid email and weak password return `content-type: application/json` bodies, while missing CSRF returns `403 Forbidden` with an empty body. Missing terms is the only checked path that returns an HTML fragment.

Recommended fix:

Return HTML error fragments from `/auth/signup` for HTMX requests, or add a global HTMX error renderer that parses JSON and writes safe text into `#auth-error`. Include focus management and an ARIA live region.

### P2 - Signup error and loading states are not fully accessible

Location:

- Template: `frontend/platform/signup.html`

Problem:

`#auth-error` has no `role="alert"`/`aria-live`, the submit button only changes text visually, and failed submissions do not move focus to the error. Screen-reader and keyboard users may not notice validation or server failures.

Expected:

Async errors should be announced, focus should move predictably to the first error or the error summary, and the busy state should expose `aria-busy`/`aria-disabled` semantics where practical.

Evidence:

Static template review found no live-region attributes and no focus handling in the inline HTMX listeners.

Recommended fix:

Add an auth error component with `role="alert"` and `aria-live="polite"` or `assertive`, set `aria-busy` during request lifecycle, and focus the error summary on failed submissions.

---

## Backend Findings

### P1 - Email verification links are generated but never verified

Location:

- Backend route: `backend/src/auth/routes.rs`
- Backend service: `backend/src/auth/service.rs`
- Template: `frontend/platform/verify-email.html`

Problem:

`create_email_verification_token` emails `/auth/verify-email?token=...`, but the registered `GET /auth/verify-email` handler only renders `verify-email.html`. The service function `verify_email` exists, but no route calls it with the query token.

Expected:

The verification link should hash the token, mark `users.email_verified = true`, delete the token, and show a success or invalid-token state.

Evidence:

Static search found `verify_email(pool, token)` is defined but unused. The route registration only maps `GET /auth/verify-email` to `verify_email_page`.

Recommended fix:

Update `verify_email_page` or add a dedicated verification handler that consumes the token. Add tests for valid token, expired token, reused token, and malformed token.

### P1 - Signup authenticates users before email verification is enforceable

Location:

- Backend handler: `backend/src/auth/routes.rs`
- Backend service: `backend/src/auth/service.rs`

Problem:

`signup_submit` creates a session and redirects to `/marketplace` immediately after creating a user with `email_verified = false`. Later password login blocks unverified users, but the initial signup session bypasses that control because session lookup does not require `email_verified`.

Expected:

If email verification is required before sign-in, signup should redirect to `/auth/verify-email` without granting a normal authenticated session, or protected pages should enforce `email_verified`.

Evidence:

`register_user` inserts `email_verified = FALSE`; `signup_submit` then calls `create_session` and returns `HX-Redirect: /marketplace`. `authenticate_user` blocks unverified login, but `get_user_by_session` only checks active session, 2FA state, and user status.

Recommended fix:

Choose and enforce a single policy: either allow unverified sessions with limited capabilities and explicit UI, or redirect to verification and block privileged pages until verified.

### P2 - Signup side effects after user creation are not atomic or fail-closed

Location:

- Backend handler: `backend/src/auth/routes.rs`
- Backend service: `backend/src/auth/service.rs`

Problem:

User/profile/wallet/role/settings creation is transactional, but terms consent, referral attribution, affiliate attribution, audit logging, and email verification token creation/sending happen afterward with several swallowed errors. This can leave a new account without consent evidence or without a verification token/email while still receiving a session.

Expected:

Compliance-critical consent should fail closed or be transactionally coupled to signup. Email verification delivery should either be durable/outbox-backed or block access until retryable verification is available.

Evidence:

`signup_submit` ignores `user_consents` insert failure with `let _ = ...`; referral and audit paths also continue after errors. `register_user` ignores `create_email_verification_token`.

Recommended fix:

Make legal consent persistence mandatory for account creation, move verification email into an outbox or durable retry path, and record explicit operational status for referral/audit side effects.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Page load | `curl GET /auth/signup` | 200 HTML with signup form and CSRF cookie | 200, form present, CSRF cookie set | Pass |
| Missing terms | HTMX POST email/password without `terms_accepted` | Visible inline terms error, no session | 200 HTML terms error | Pass |
| Missing CSRF | HTMX POST valid-looking form without `X-CSRF-Token` | Visible CSRF error and no mutation | 403 empty body | Fail |
| Invalid email | HTMX POST `email=not-an-email` | Visible inline email error | 400 JSON body swapped into HTML target | Fail |
| Weak password | HTMX POST `password=short` | Visible inline password error | 400 JSON body swapped into HTML target | Fail |
| Successful signup | Submit unique email, valid password, accepted terms | User/session/consent/token verified | Not run to avoid real account creation and email dispatch | Not run |
| Browser/mobile/keyboard | Open page in browser, inspect console, keyboard, responsive layout | No console errors; accessible states | Not run in this audit | Not run |

---

## Security Findings

- P1: Email verification route does not consume tokens, so verification emails cannot complete the account verification state.
- P1: Initial signup grants a normal session to `email_verified = false` users despite later login enforcing email verification.
- P2: CSRF failures on the signup page produce no visible HTMX error body.
- P2: Signup error announcements are weak for assistive technology.
- No money-handling or client-side financial logic was present on this page.

---

## Database Findings

- Required core tables exist: `users`, `user_profiles`, `wallets`, `user_roles`, `user_settings`, `user_sessions`, `user_consents`, `email_verification_tokens`.
- Core user/profile/wallet/role/settings creation is transactional in `register_user`.
- `user_consents` is inserted after the main transaction and failures are ignored, which weakens legal evidence.
- `email_verification_tokens` exists and token hashing is implemented, but the token consumption route is not wired.

---

## Missing Tests

- Browser E2E for successful signup, redirect behavior, session cookie, no console errors, and mobile layout.
- Backend/integration tests for valid email verification token, invalid token, expired token, and token replay.
- Auth policy tests proving unverified users either cannot access protected routes or are intentionally limited.
- HTMX error-rendering tests for invalid email, weak password, duplicate email, CSRF failure, rate limiting, and DB/email failures.
- Consent persistence test that fails signup if `user_consents` cannot be recorded.

---

## Recommended Fix Order

1. Fixed in working tree: `/auth/verify-email?token=...` now calls the existing `verify_email` service and redirects to a token-free result URL.
2. Fixed in working tree: normal session lookup now requires `email_verified = TRUE`; signup redirects to the verification page instead of `/marketplace`.
3. Fixed in working tree: signup validation/rate-limit errors and HTMX CSRF failures render HTML alert fragments instead of JSON/blank bodies.
4. Fixed in working tree: signup legal consent and verification-token persistence happen in the account-creation transaction.
5. Remaining: add browser E2E coverage for successful signup, verification-link consumption, resend, protected-route blocking before verification, and accessible failure states.
6. Remaining: consider a transactional email outbox/retry worker so Resend outages are retried automatically instead of relying on manual resend.

---

## Final Status

`needs_recheck`

Reason: The implementation findings from this audit are fixed in the working tree, but the page still needs browser E2E recheck for signup, verification, resend, protected-route blocking, console health, mobile layout, and keyboard/accessibility behavior. A production email outbox/retry worker remains a recommended operational hardening item.
