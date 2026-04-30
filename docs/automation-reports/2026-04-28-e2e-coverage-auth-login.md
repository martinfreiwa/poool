# E2E Coverage Gap Audit: Auth Login

Date: 2026-04-28

Selected scope: `/auth/login` email/password login flow, including the `frontend/platform/login.html` form, `backend/src/auth/routes.rs::login_page`, `backend/src/auth/routes.rs::login_submit`, and the immediate 2FA redirect decision.

## Existing Coverage Found

- `tests/test_auth_login_register.py` has `requests` coverage for `GET /auth/login`, HTMX-style valid login, invalid login, session cookie creation, `/api/me` access, and a direct `user_sessions` database row check.
- `tests/test_platform.py` and `tests/test_e2e.py` include broader smoke checks for login page availability and login POST behavior.
- Several domain tests call `/auth/login` as a setup helper for authenticated flows.
- `tests/e2e/pages/login_page.py` defines a Playwright page object for `/auth/login`, but no committed Playwright spec currently uses it for production-grade login coverage.
- Existing route/page/security audits on 2026-04-28 already found related non-test issues: active Google OAuth button when OAuth is not configured, bare 403 HTMX CSRF failure response, failed-login email telemetry, missing accessible names/state for some controls, and testimonial `innerHTML`.

## Follow-Up Coverage Added

Follow-up on 2026-04-28 added `tests/e2e/test_auth_login.py`. The committed Playwright suite seeds disposable login-capable users with real Argon2id password hashes and covers:

- successful browser login, `/marketplace` redirect, session cookie, default expiry, and `user_sessions` state;
- invalid credentials with visible alert, no redirect, restored submit button, and no session row;
- HTMX CSRF failure returning the auth error fragment;
- remember-me session duration and `remember_me = true`;
- TOTP-enabled login redirecting to `/auth/2fa` with an unverified session and protected-route blocking;
- disabled Google OAuth UI when credentials are absent;
- password toggle and carousel accessible names/state;
- mobile viewport overflow;
- authenticated users visiting `/auth/login` redirecting to `/marketplace`.

The test file passed `python3 -m py_compile tests/e2e/test_auth_login.py`. Runtime execution is still pending because `python3 -m pytest tests/e2e/test_auth_login.py -q` exited at the session health gate: no backend was reachable at `http://localhost:8888/health`.

## Coverage Adequacy

Coverage is now adequate as committed regression intent, but not yet runtime-verified in this worktree.

The previous HTTP tests were useful smoke coverage but did not exercise browser-visible behavior. The new Playwright file fills those cases; the remaining release gate is to run it against a healthy local or CI backend.

## Missing Coverage

- Runtime execution of the committed Playwright suite against a live backend.
- Full configured-Google OAuth redirect/callback flow with a mocked provider or safe test account.
- Rate-limit/lockout behavior for repeated failed attempts, including safe visible error state and no raw email leakage in test-visible logs where feasible.
- Deeper keyboard tab-order assertions beyond the current accessible-name/state and mobile-fit checks.

## Suggested Test Files And Names

- `tests/e2e/test_auth_login.py`
- `test_login_happy_path_sets_session_and_redirects`
- `test_login_invalid_credentials_render_error_without_session`
- `test_login_requires_csrf_and_preserves_visible_error_state`
- `test_login_remember_me_extends_session_duration`
- `test_login_totp_enabled_redirects_to_2fa_and_requires_verification`
- `test_login_google_button_respects_oauth_configuration`
- `test_login_page_accessibility_mobile_and_console_health`

## Test Data Required

- A seeded active investor user with known Argon2id password and `totp_enabled = false`.
- A seeded active user with `totp_enabled = true`, a deterministic TOTP secret, and helper code for current TOTP generation.
- Optional OAuth-configured test mode or route mocking for `/auth/google` without contacting Google.
- Database helpers to read `user_sessions.remember_me`, `is_2fa_verified`, `expires_at`, and the latest login `audit_logs` row.
- Browser fixtures for desktop and mobile viewport coverage.

## Priority Order

1. Browser happy path plus backend session verification.
2. Invalid credentials and CSRF failure visible error behavior.
3. Remember-me session duration assertions.
4. 2FA redirect, unverified session blocking, verification, and session rotation.
5. OAuth button configuration behavior.
6. Keyboard/mobile/accessibility and console/network health checks.
7. Rate-limit and lockout checks.

## Minimum Recommended Regression Suite Before Release

- Desktop and mobile browser login happy path with seeded non-2FA user.
- Invalid credentials browser path with visible error and no session.
- Missing CSRF rejection path.
- Remember-me expiry/DB assertion.
- 2FA-enabled login redirect and protected-route block.
- OAuth disabled-state check when Google credentials are absent.
- Console/network health check for `/auth/login`.

## Production Readiness Decision

Status: E2E coverage added; runtime execution pending.

Do not treat `/auth/login` as runtime-verified until `python3 -m pytest tests/e2e/test_auth_login.py -q` passes against a live backend. The committed suite now covers the documented browser gap; the remaining issue is execution, not missing test code.
