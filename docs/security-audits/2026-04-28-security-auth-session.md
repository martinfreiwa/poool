# Security Audit: Auth And Session Domain

Date: 2026-04-28

Selected scope: `auth/session` backend domain, including password login, signup, email verification, password reset, logout, Google OAuth, 2FA enrollment/verification, CSRF middleware, auth templates, and session lookup helpers.

Production readiness status: **issues found**. Do not ship auth/session as production-ready until the high-severity items below are fixed and covered by browser-backed and HTTP+DB regression tests.

## Files And Routes Reviewed

- `backend/src/auth/routes.rs`
  - `GET/POST /auth/login`
  - `GET/POST /auth/signup`
  - `GET/POST /auth/2fa`
  - `GET/POST /auth/2fa/setup`
  - `POST /auth/2fa/step-up`
  - `GET /auth/logout`
  - `GET /auth/google`
  - `GET /auth/google/callback`
  - `GET/POST /auth/forgot-password`
  - `GET/POST /auth/reset-password`
  - `GET /auth/verify-email`
  - `POST /auth/resend-verification`
- `backend/src/auth/service.rs`
- `backend/src/auth/csrf.rs`
- `backend/src/auth/middleware.rs`
- `backend/src/auth/rate_limit.rs`
- `backend/src/main.rs`
- `backend/src/common/email.rs`
- `backend/src/config.rs`
- `frontend/platform/login.html`
- `frontend/platform/signup.html`
- `frontend/platform/reset-password.html`
- `frontend/platform/auth-2fa-setup.html`
- `frontend/platform/components/auth-head.html`
- `frontend/platform/components/head.html`
- `database/001_initial_schema.sql`
- `database/026_2fa_session_column.sql`
- `database/041_email_verification_tokens.sql`
- `database/045_hash_tokens_migration.sql`

## Critical Findings

None identified in this scoped static review.

## High Findings

### HIGH-1: Signup grants authenticated sessions before email verification can complete

Evidence:

- `service::register_user` creates users with `email_verified = FALSE`, then dispatches a verification token best-effort after the transaction (`backend/src/auth/service.rs:61-112`).
- `signup_submit` immediately creates a `poool_session` and redirects to `/marketplace` (`backend/src/auth/routes.rs:603-643`).
- `get_user_by_session` accepts active sessions without checking `u.email_verified` (`backend/src/auth/service.rs:329-340`).
- `verify_email_page` only renders `verify-email.html`; it does not read `?token=` or call `service::verify_email` (`backend/src/auth/routes.rs:134-142`). The actual `verify_email` service is marked dead code (`backend/src/auth/service.rs:757-795`).

Impact:

Unverified accounts can enter authenticated areas immediately after signup. This weakens email ownership assurance and can bypass workflows that assume `email_verified` means the user has proven mailbox control. Because the verification link is not consumed, legitimate users also cannot complete the intended control.

Recommended fix:

- Decide the product policy explicitly: either block normal authenticated app access until email verification, or restrict unverified sessions to a small verification/resend surface.
- Wire `GET /auth/verify-email?token=...` to hash and consume the token via `service::verify_email`.
- Make verification email dispatch durable or fail closed in production.
- Add regression tests for signup, unverified access denial, token consumption, token replay, expired token handling, and verified login.

### HIGH-2: TOTP enrollment can replace an existing secret without step-up and trusts a client-supplied pending secret

Evidence:

- `GET /auth/2fa/setup` generates a fresh secret and renders it into the page (`backend/src/auth/routes.rs:351-381`).
- The template posts that secret back in a hidden field (`frontend/platform/auth-2fa-setup.html:52-56`).
- `POST /auth/2fa/setup` verifies the posted secret and writes it directly with `enable_totp` (`backend/src/auth/routes.rs:384-417`, `backend/src/auth/service.rs:914-925`).
- The handler does not check whether TOTP is already enabled, does not require current TOTP/step-up for replacement, and does not bind the pending setup secret server-side.

Impact:

Any authenticated session that reaches the setup route can overwrite the account's TOTP secret by completing verification for a client-controlled secret. For admin or finance users, this is a privilege-control weakness because 2FA replacement should require step-up or a recovery flow.

Recommended fix:

- Store pending TOTP enrollment server-side with a short TTL and bind it to the session/user.
- If TOTP is already enabled, require step-up with the current factor or an audited recovery workflow before replacement.
- Encrypt or otherwise protect stored TOTP secrets at rest.
- Add tests for first enrollment, replacement blocked without step-up, replacement allowed after step-up, pending-secret replay, and expired pending setup.

### HIGH-3: Password reset submit path lacks rate limiting

Evidence:

- `forgot_password_submit` applies IP and per-email rate limiting before generating reset tokens (`backend/src/auth/routes.rs:648-684`).
- `reset_password_submit` accepts token, password, and confirmation, then calls `service::reset_password` with no IP, token-hash, or account-level rate limiter (`backend/src/auth/routes.rs:700-718`).

Impact:

The reset token remains high entropy, but the submit endpoint can still be abused for repeated token probes, password-validation workload, and noisy operational events. Auth endpoints should be uniformly throttled, especially when they gate account recovery.

Recommended fix:

- Apply auth rate limiting to `POST /auth/reset-password` using client IP plus token hash prefix or another non-secret stable bucket.
- Keep responses generic and avoid revealing whether the token exists beyond the final form state.
- Add tests for throttled invalid-token attempts and valid reset after normal use.

### HIGH-4: Auth telemetry and logs include user emails and provider responses

Evidence:

- Failed login paths include raw submitted email addresses in Sentry messages and wrong-password Sentry user context (`backend/src/auth/service.rs:162-239`).
- Successful session validation logs user email on every valid session lookup (`backend/src/auth/service.rs:361`).
- Password reset and email verification delivery paths log recipient emails (`backend/src/auth/service.rs:646`, `backend/src/auth/service.rs:752`, `backend/src/common/email.rs:13`).
- Google OAuth malformed token handling logs the parsed token response when `access_token` is missing (`backend/src/auth/routes.rs:934-936`).

Impact:

Auth events are high-volume and security-sensitive. Raw emails and provider response bodies in monitoring/logs increase privacy exposure and can leak tokens or provider error details if upstream responses change shape.

Recommended fix:

- Replace raw emails in auth logs/Sentry with user IDs, hashed normalized emails, or coarse event tags.
- Never log OAuth token response bodies; log only status, error code, and a request correlation ID.
- Keep frontend URL token redaction, but add backend redaction tests or static assertions for auth telemetry.

## Medium Findings

### MEDIUM-1: Logout is a CSRF-less GET state change and cookie expiry may not match the original path attributes

Evidence:

- `/auth/logout` and `/logout` are registered as GET handlers (`backend/src/auth/routes.rs:68`, `backend/src/main.rs:790-791`).
- The handler deletes the DB session on any GET carrying the cookie (`backend/src/auth/routes.rs:748-762`).
- Cookie removal uses `Cookie::from(SESSION_COOKIE)` instead of mirroring all attributes from the original cookie (`backend/src/auth/routes.rs:755-759`).

Impact:

Third-party pages can trigger logout by loading a URL, creating a session-disruption vector. The session row is deleted server-side, so the main security impact is forced logout rather than account takeover.

Recommended fix:

- Move logout mutation to CSRF-protected POST and keep GET as a confirmation or redirect-only route.
- Expire `poool_session` with explicit `Path=/`, `HttpOnly`, `Secure`, and `SameSite=Lax`.
- Add authenticated browser/API tests for logout, stale-cookie cleanup, and CSRF rejection.

### MEDIUM-2: CSRF error UX is inconsistent for auth HTMX forms

Evidence:

- `csrf_middleware` returns structured JSON for `/api/*`, but non-API failures return a bare `403` (`backend/src/auth/csrf.rs:123-135`).
- Auth forms expect HTML fragments in `#auth-error`, and login has custom error-fragment handling for application-level errors (`frontend/platform/login.html:48`, `backend/src/auth/routes.rs:1059-1119`).

Impact:

The underlying CSRF control is present and broad, but users get a blank/bare failure path for expired or missing CSRF on auth forms. This increases support burden and makes regression detection harder.

Recommended fix:

- Return an auth-compatible HTML fragment for `/auth/*` HTMX requests when CSRF fails.
- Add tests for CSRF failure render behavior on login, signup, reset password, and 2FA setup.

## Positive Controls Observed

- Session cookies are `HttpOnly`, `SameSite=Lax`, `Path=/`, and secure-by-default outside explicit local/dev environments (`backend/src/auth/routes.rs:23-33`, `backend/src/auth/routes.rs:252-257`).
- Login creates fresh cryptographically random session tokens and rotates CSRF on login (`backend/src/auth/routes.rs:216-266`, `backend/src/auth/service.rs:251-284`).
- 2FA verification rotates the session token after privilege elevation (`backend/src/auth/routes.rs:334-348`).
- Password reset stores token hashes, marks outstanding reset tokens as used, and revokes active sessions in one DB transaction (`backend/src/auth/service.rs:651-711`).
- CSRF middleware covers mutating platform routes and only skips static assets and webhook paths (`backend/src/auth/csrf.rs:28-41`, `backend/src/main.rs:936`).
- Login rate limiting has IP and normalized-email buckets (`backend/src/auth/routes.rs:154-186`), and Redis rate limiting fails closed (`backend/src/auth/rate_limit.rs:101-193`).
- Google OAuth uses state and PKCE cookies for callback validation (`backend/src/auth/routes.rs:766-886`).

## Ambiguities Or Decisions Needed

- Product/security owner must decide whether unverified signup sessions should exist at all, or whether they should be restricted to a verification-only state.
- TOTP replacement needs a recovery policy for users who lost their authenticator. That policy should include audit logging and support/compliance requirements for admin and finance roles.
- Production email behavior needs an explicit policy: missing `RESEND_API_KEY` should likely fail closed for auth-critical delivery in production while remaining no-op in local development.

## Missing Tests

- Browser-backed auth flow coverage for signup, verification, login, logout, reset password, OAuth configured/unconfigured, and 2FA.
- HTTP+DB tests that assert unverified users cannot access protected routes, verification tokens are consumed once, and session rows rotate/delete correctly.
- Rate-limit tests for reset-password submission.
- TOTP setup tests for server-side pending secret binding and replacement step-up.
- Telemetry redaction tests or static checks preventing raw emails and token-provider bodies in auth logs.

## Coverage Tracker Notes

Updated `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`:

- Security Review automation last run/report/status.
- Backend `auth` domain Last Security Audit, status, and missing coverage summary.

No production application code was modified. No page tracker YAML update was made because this run selected the backend `auth/session` domain rather than a single page, and the related auth page rows already have same-day security statuses from page audits.
