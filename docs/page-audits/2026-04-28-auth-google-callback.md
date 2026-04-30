# Page Audit: Google OAuth Callback

Date: 2026-04-28
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/auth/google/callback`
Template: none
JavaScript: none
CSS: none
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`

---

## Summary

`/auth/google/callback` is a redirect endpoint, not an HTML page. The callback validates OAuth state before token exchange, requires the PKCE verifier cookie, enforces verified Google email, creates or links users through backend code, and applies the same TOTP gate as password login.

The code issues found in this audit have been fixed in the working tree: token-response logging is redacted, OAuth registration logs no longer include raw email addresses, failed callbacks clear transient OAuth cookies, and token/userinfo calls are configurable for mocked-provider testing. Focused mocked-provider Rust tests now exercise the callback token and userinfo branches without calling Google's live endpoints.

---

## Tested Scope

- Reviewed callback routing in `backend/src/auth/routes.rs`.
- Reviewed OAuth user creation/linking in `backend/src/auth/service.rs`.
- Reviewed account-link entry point in `backend/src/settings/routes.rs`.
- Reviewed OAuth/session schema in `database/001_initial_schema.sql`, `database/026_2fa_session_column.sql`, and `docs/DATABASE_SCHEMA.md`.
- Ran local backend and non-mutating `curl -I` callback failure-path smoke checks.
- After fixes, ran `python3 -m pytest tests/test_auth_google_static.py -q`.
- After mockability changes, ran `CARGO_TARGET_DIR=/tmp/poool-auth-google-test cargo test google_oauth_tests --bin poool-backend`.
- Searched existing tests for OAuth callback coverage.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/google/callback` | Public Google OAuth callback redirect |
| Backend route | `GET /auth/google/callback` | Registered by `auth::routes::routes()` |
| Handler | `backend/src/auth/routes.rs` | `google_callback` and `google_callback_inner` |
| User creation/linking | `backend/src/auth/service.rs` | `oauth_find_or_create_user` |
| Account-link entry | `POST /api/settings/oauth/google/link` | Returns `/auth/google?link=1` |
| Database table | `oauth_accounts` | Provider identity mapping |
| Database table | `users` | OAuth-created or linked user account |
| Database table | `user_profiles` | First/last name backfill for new users |
| Database table | `wallets` | New OAuth user wallets, integer cents |
| Database table | `user_roles` | New OAuth user investor role |
| Database table | `user_settings` | TOTP decision after OAuth login |
| Database table | `user_sessions` | Login session with `is_2fa_verified` |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Callback route | `GET /auth/google/callback` | Validate `code`, state cookie, state query, and PKCE cookie | Backend only | Yes | Missing cookies returned generic login redirect |
| Provider denied callback | `?error=access_denied&state=x` | Fail closed without creating a session | Backend only | Yes | Returned generic login redirect |
| State mismatch path | `oauth_state=abc`, query `state=xyz` | Reject as possible CSRF before token exchange | Backend only | Yes | Returned generic login redirect |
| Token exchange | Configured token endpoint | Exchange code using client credentials and PKCE verifier | Backend only | Yes | Mocked-provider Rust test passed |
| User info fetch | Configured userinfo endpoint | Fetch verified Google profile | Backend only | Yes | Mocked-provider Rust test passed |
| Account-link callback | `oauth_link=1` cookie | Link Google account to current authenticated user | Backend only | Yes | Static review only |
| Normal login callback | no `oauth_link` cookie | Find/create user, create session, redirect `/marketplace` or `/auth/2fa` | Backend only | Yes | Static review only |

---

## Frontend Findings

No callback-specific frontend issues. The callback is a backend redirect route with no template, no page JavaScript, and no visible controls.

---

## Backend Findings

### P2 - Malformed token responses can be logged with sensitive fields

Status: fixed in working tree.

Location:

- Backend: `backend/src/auth/routes.rs:935`

Problem:

When the Google token response does not contain `access_token`, the callback logs the full `token_data` JSON with `{:?}`. A malformed provider or proxy response could include `id_token`, `refresh_token`, scope, or other token-adjacent fields, and those values would be emitted to application logs or Sentry.

Expected:

OAuth callback logs should redact token payloads and record only safe context such as provider, status, safe error code, and a correlation id.

Evidence:

Static review found `tracing::error!("No access_token in Google response: {:?}", token_data);`. Runtime negative-path checks confirmed callback errors are routed through logging before a generic login redirect.

Recommended fix:

Implemented fix:

`backend/src/auth/routes.rs` now logs only token response keys when `access_token` is absent. `tests/test_auth_google_static.py` asserts the old raw-payload log pattern is absent and the redacted message is present.

### P2 - New OAuth registration logs the user email address

Status: fixed in working tree.

Location:

- Backend: `backend/src/auth/service.rs:545`

Problem:

New OAuth user creation logs `New OAuth user registered: {user.id} ({email}) via {provider}`. Email is personal data, and the shared production-readiness standard says logs should avoid leaking personal data.

Expected:

Registration logs should use user id/provider and omit or hash the email address.

Evidence:

Static review of `oauth_find_or_create_user` shows the email interpolated into an info-level log after the transaction commits.

Recommended fix:

Implemented fix:

`backend/src/auth/service.rs` now logs structured `user_id` and `provider` fields without the raw email address. `tests/test_auth_google_static.py` covers the regression.

### P3 - Failed callbacks do not clear transient OAuth cookies

Status: fixed in working tree.

Location:

- Backend: `backend/src/auth/routes.rs:850`
- Backend: `backend/src/auth/routes.rs:1027`

Problem:

The success paths remove `oauth_state`, `oauth_pkce`, and, for link flow, `oauth_link`. The error wrapper redirects to login without removing those cookies. A failed or denied callback leaves transient OAuth cookies in the browser until their 10-minute expiry.

Expected:

The callback should clear transient OAuth cookies on both success and failure so failed login/link attempts do not leave stale state markers.

Evidence:

Runtime `curl -I` checks for missing and mismatched callback state returned only a new `csrf_token` cookie and no expiry/removal cookies for `oauth_state` or `oauth_pkce`.

Recommended fix:

Implemented fix:

`backend/src/auth/routes.rs` now uses a shared `clear_oauth_cookies` helper on callback success and error paths. `tests/test_auth_google_static.py` asserts the helper removes `oauth_state`, `oauth_pkce`, and `oauth_link`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Missing cookies with code/state | `curl -I 'http://127.0.0.1:8888/auth/google/callback?state=x&code=y'` | Generic login failure redirect, no session | `303` to `/auth/login?error=Google+sign+in+failed...` | Pass |
| Provider denied callback | `curl -I 'http://127.0.0.1:8888/auth/google/callback?error=access_denied&state=x'` | Generic login failure redirect, no session | `303` to `/auth/login?error=Google+sign+in+failed...` | Pass |
| Missing query params | `curl -I 'http://127.0.0.1:8888/auth/google/callback'` | Generic login failure redirect, no session | `303` to `/auth/login?error=Google+sign+in+failed...` | Pass |
| State mismatch | `curl -I -H 'Cookie: oauth_state=abc; oauth_pkce=def' '/auth/google/callback?state=xyz&code=fake'` | Reject before token exchange | `303` to generic login failure | Pass |
| Static regression tests | `python3 -m pytest tests/test_auth_google_static.py -q` | Google OAuth static regressions pass | 6 passed | Pass |
| Mocked token callback branch | `CARGO_TARGET_DIR=/tmp/poool-auth-google-test cargo test google_oauth_tests --bin poool-backend` | Callback uses mock token endpoint and fails safely on missing access token | Passed | Pass |
| Mocked userinfo callback branch | `CARGO_TARGET_DIR=/tmp/poool-auth-google-test cargo test google_oauth_tests --bin poool-backend` | Callback uses mock userinfo endpoint and rejects unverified email | Passed | Pass |
| Callback error cleanup branch | `CARGO_TARGET_DIR=/tmp/poool-auth-google-test cargo test google_oauth_tests --bin poool-backend` | Error path clears `oauth_state`, `oauth_pkce`, and `oauth_link` cookies | Passed | Pass |
| Full live Google login | Complete real Google consent/token/userinfo flow | Session created and redirect to `/marketplace` or `/auth/2fa` | Not run | Optional live-provider manual verification |
| Account-link callback | Authenticated link flow with `oauth_link=1` | OAuth account linked, redirect to settings security tab | Not run | Optional live-provider/manual verification |

---

## Security Findings

- Fixed P2: Malformed token response logging now records response keys only.
- Fixed P2: New OAuth registration logs no longer include raw email addresses.
- Fixed P3: Failed callbacks now clear transient OAuth cookies.
- Positive: Callback checks state cookie/query before token exchange.
- Positive: Callback requires the PKCE verifier cookie.
- Positive: Unverified Google emails are rejected server-side.
- Positive: OAuth login honors the TOTP gate before full app entry.
- Positive: New OAuth user bootstrap runs in a database transaction and initializes wallets with integer-cent balances.

---

## Database Findings

- `oauth_accounts` has `UNIQUE (provider, provider_id)` and a `user_id` foreign key.
- New OAuth users are created transactionally with `users`, `oauth_accounts`, `user_profiles`, `wallets`, `user_roles`, and `user_settings`.
- Session creation writes `user_sessions` with `is_2fa_verified`, added by `database/026_2fa_session_column.sql`.
- No financial mutation is performed by the callback. Wallet bootstrap uses `0` integer cents.

---

## Missing Tests

- Added mocked-provider callback tests for missing `access_token` and unverified email.
- Added static log-redaction, configurable-endpoint, and cookie-cleanup assertions in `tests/test_auth_google_static.py`.
- Optional future coverage: DB-backed success-path callback tests for account creation, existing-user login, TOTP redirect, and settings account linking.

---

## Recommended Fix Order

1. Optional: run a staging/manual live Google consent flow with a safe test account.
2. Optional: add DB-backed success-path callback tests for account creation, existing-user login, TOTP redirect, and settings account linking.

---

## Final Status

`completed`

Reason: The documented callback code issues are fixed and covered by static plus mocked-provider callback regression tests. Live Google consent remains optional manual verification.
