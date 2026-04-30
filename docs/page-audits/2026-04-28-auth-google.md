# Page Audit: Google OAuth Redirect

Date: 2026-04-28
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/auth/google`
Template: none; linked from `frontend/platform/login.html` and `frontend/platform/signup.html`
JavaScript: inline `onclick` handlers in login/signup templates; `frontend/platform/static/js/settings.js` for account-link flow
CSS: auth page CSS only through entry pages
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`, `backend/src/settings/routes.rs`

---

## Summary

`/auth/google` is a redirect endpoint, not an HTML page. The endpoint is implemented and runtime smoke testing confirmed it redirects to Google with state and PKCE parameters while setting short-lived HttpOnly OAuth cookies. The related `/auth/google?link=1` account-link redirect also sets the link-flow cookie, and a callback without the required state cookie fails closed with a generic login redirect.

Follow-up fixes were applied after the audit: malformed token responses now log only response keys, login/signup render the Google button only when `google_enabled` is true, and mocked-provider callback tests cover the token and userinfo failure branches. Live Google consent and DB-backed success/link fixtures remain optional follow-up coverage, not open code issues from this audit.

---

## Tested Scope

- Reviewed `/auth/google` and `/auth/google/callback` in `backend/src/auth/routes.rs`.
- Reviewed OAuth user creation/linking in `backend/src/auth/service.rs`.
- Reviewed the settings account-link entry route in `backend/src/settings/routes.rs`.
- Reviewed visible Google entry buttons in `frontend/platform/login.html` and `frontend/platform/signup.html`.
- Reviewed OAuth persistence schema in `database/001_initial_schema.sql` and `docs/DATABASE_SCHEMA.md`.
- Ran local non-mutating route smoke checks against `http://127.0.0.1:8888`.
- Ran targeted Rust auth rate-limiter tests and added static regression tests for the fixed OAuth rendering/logging issues.
- Ran `CARGO_TARGET_DIR=/tmp/poool-auth-google-test cargo test google_oauth_tests --bin poool-backend`; mocked-provider callback regressions passed.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/google` | Public OAuth redirect endpoint |
| URL | `/auth/google?link=1` | Authenticated account-link redirect flow from settings |
| Callback URL | `/auth/google/callback` | Validates state cookie/query, exchanges code, creates session or link |
| Login entry | `frontend/platform/login.html` | `#google-button` navigates to `/auth/google` |
| Signup entry | `frontend/platform/signup.html` | `#google-button` navigates to `/auth/google` |
| Settings entry | `frontend/platform/static/js/settings.js` | Uses `/api/settings/oauth/google/link`, then redirects to `/auth/google?link=1` |
| Backend redirect route | `GET /auth/google` | `google_redirect` |
| Backend callback route | `GET /auth/google/callback` | `google_callback` / `google_callback_inner` |
| Backend link API | `POST /api/settings/oauth/google/link` | Returns redirect URL when configured |
| Database table | `oauth_accounts` | Provider account link, provider email, optional token columns |
| Database table | `users` | OAuth user lookup/creation |
| Database table | `user_sessions` | OAuth login session cookie |
| Database table | `user_settings` | TOTP gate after OAuth login |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Login Google button | `frontend/platform/login.html`, `#google-button` | Render only when Google OAuth is configured, then navigate to `/auth/google` | Inline `onclick`, gated by `google_enabled` | Yes | Fixed and static-tested |
| Signup Google button | `frontend/platform/signup.html`, `#google-button` | Render only when Google OAuth is configured, then navigate to `/auth/google` | Inline `onclick`, gated by `google_enabled` | Yes | Fixed and static-tested |
| Settings OAuth link action | `settings.js` provider row action | Request link URL and navigate to `/auth/google?link=1` | Yes | Yes | Static verified; route smoke set `oauth_link=1` |
| OAuth state cookie | `oauth_state` | Bind redirect to callback state | Backend only | Yes | Runtime header showed HttpOnly, SameSite=Lax, 10-minute cookie |
| OAuth PKCE cookie | `oauth_pkce` | Store verifier for token exchange | Backend only | Yes | Runtime header showed HttpOnly, SameSite=Lax, 10-minute cookie |
| OAuth link cookie | `oauth_link` | Mark account-link flow | Backend only | Yes | Runtime `?link=1` header set cookie |
| Auth redirect URL | Google OAuth URL | Include encoded callback, `state`, S256 PKCE challenge | Backend only | Yes | Runtime redirect contained required parameters |
| Callback error path | `/auth/google/callback?state=x&code=y` without cookies | Fail closed to login with generic error | Backend only | Yes | Runtime returned 303 to `/auth/login?error=Google+sign+in+failed...` |

---

## Frontend Findings

### P3 - Google buttons ignore OAuth configuration state

Location:

- Template: `frontend/platform/login.html:32`
- Template: `frontend/platform/signup.html:30`
- Backend context: `backend/src/auth/routes.rs:1147`, `backend/src/auth/routes.rs:1169`

Status: fixed 2026-04-28

Problem:

The backend passes `google_enabled => state.config.google_oauth_enabled()` into both auth templates, but the templates always render an active Google button. If `GOOGLE_CLIENT_ID` or `GOOGLE_CLIENT_SECRET` is missing, clicking the button redirects back to `/auth/login?error=oauth_not_configured`.

Expected:

When OAuth is disabled, the buttons should be hidden, disabled with an accessible explanation, or omitted from the social-login section so users do not enter a known-dead path. Login and signup should use the same behavior.

Evidence:

Static review shows the unconditional buttons in both templates. Runtime with the current local config confirmed the configured path works, but the disabled-config branch is only handled after navigation in `google_redirect`.

Fix:

`frontend/platform/login.html` and `frontend/platform/signup.html` now wrap the Google social button and divider in `{% if google_enabled %}`. `tests/test_auth_google_static.py` verifies the templates use the config gate.

---

## Backend Findings

### P2 - Malformed Google token responses can leak token payloads to logs

Location:

- Backend: `backend/src/auth/routes.rs:934`

Status: fixed 2026-04-28

Problem:

After the token exchange, the callback logs the full `token_data` JSON when `access_token` is missing. If Google or a proxy returns a malformed success-like payload containing other sensitive fields, such as `id_token` or `refresh_token`, those values would be emitted to application logs/Sentry.

Expected:

OAuth callback logs should never include raw token response payloads. They should log status, provider, safe error code, and a correlation id, while redacting token-bearing fields.

Evidence:

Static review shows `tracing::error!("No access_token in Google response: {:?}", token_data);`. The normal error branch logs Google `error` and `error_description` only; the missing-token branch is broader.

Fix:

`backend/src/auth/routes.rs` now logs only the JSON response keys when `access_token` is absent, not the full token payload. `tests/test_auth_google_static.py` verifies the old full-payload log string is absent and the redacted log path remains.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Redirect smoke | `curl -I http://127.0.0.1:8888/auth/google` | 303 to Google with OAuth state and PKCE challenge | 303 to `accounts.google.com`; `oauth_state` and `oauth_pkce` cookies set | Pass |
| Link redirect smoke | `curl -I 'http://127.0.0.1:8888/auth/google?link=1'` | 303 to Google plus link-flow cookie | 303 to Google; `oauth_link=1`, state, and PKCE cookies set | Pass |
| Callback without cookies | `curl -I 'http://127.0.0.1:8888/auth/google/callback?state=x&code=y'` | Generic failure redirect, no login session | 303 to `/auth/login?error=Google+sign+in+failed...` | Pass |
| Mocked provider callback | `CARGO_TARGET_DIR=/tmp/poool-auth-google-test cargo test google_oauth_tests --bin poool-backend` | Callback uses configured token/userinfo endpoints and fails safely on malformed/unverified provider data | 3 passed | Pass |
| Full live OAuth callback | Complete Google consent and token exchange | User session or settings link created | Not run; requires external Google consent and safe test account | Optional manual verification |
| Disabled-config UX | Static template regression check | Button hidden when `google_enabled` is false | Templates gate social button/divider with `{% if google_enabled %}` | Pass |

---

## Security Findings

- Fixed: Full token payload logging risk in the malformed token-response branch.
- Positive: redirect flow uses random state, PKCE S256, 10-minute HttpOnly cookies, SameSite=Lax, and validates callback state before token exchange.
- Positive: OAuth-created users are inserted in a transaction with profile, wallets, investor role, and settings.
- Positive: existing local email users are only auto-linked if `email_verified` is true.
- Positive: OAuth login follows the same TOTP gate as password login.

---

## Database Findings

- `oauth_accounts` exists with `UNIQUE (provider, provider_id)` and `user_id` foreign key.
- New OAuth user creation is transactional across `users`, `oauth_accounts`, `user_profiles`, `wallets`, `user_roles`, and `user_settings`.
- Existing-user auto-link and settings link are single-row writes and use the provider/provider_id uniqueness constraint.
- No monetary values are processed by this endpoint.

---

## Missing Tests

- Add an HTTP/unit test for `GET /auth/google` that verifies redirect URL parameters and OAuth cookie attributes.
- Add a test for `GET /auth/google?link=1` that verifies the link-flow cookie.
- Added callback tests for mocked missing access token, unverified Google email, and callback error cookie cleanup.
- Added static disabled-config template guard, configurable-endpoint, cookie-cleanup, and log-redaction regression checks in `tests/test_auth_google_static.py`.
- Optional future coverage: DB-backed success-path tests for TOTP-enabled user redirect and existing email/account-link behavior.

---

## Recommended Fix Order

1. Optional: verify full Google consent/account-link/TOTP redirect behavior with a safe staging test account.
2. Optional: add DB-backed success/link/TOTP callback fixtures.

---

## Final Status

`completed`

Reason: The documented redirect and callback code issues are fixed and covered by static plus mocked-provider regression tests. Live Google consent remains optional manual verification.
