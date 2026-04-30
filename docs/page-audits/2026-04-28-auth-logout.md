# Page Audit: Logout

Date: 2026-04-28
Status: fixed, needs browser recheck
Auditor: ChatGPT/Codex
Page URL: `/auth/logout`
Template: route-only redirect, no page template
JavaScript: `frontend/platform/static/js/profile-dropdown.js`, `frontend/platform/static/js/mobile-navigation.js`
CSS: shared auth/sidebar/mobile-menu CSS only
Backend Routes: `backend/src/auth/routes.rs`, `backend/src/main.rs`

---

## Summary

The logout route now invalidates the server-side session only through a CSRF-protected `POST /auth/logout`. The legacy `GET /auth/logout` and `/logout` routes are retained as non-mutating compatibility pages that render an auto-submitting POST form.

The original audit issues are fixed in code: logout no longer deletes sessions on GET, shared desktop/mobile controls submit POST with a CSRF token, DB deletion failures are logged, and `poool_session` is expired with `Path=/`, `HttpOnly`, `SameSite=Lax`, and the environment-correct `Secure` flag. The existing auth smoke test now exercises the POST logout contract and asserts the runtime `Set-Cookie` expiry when run against a server. Remaining work is browser coverage and a full Rust check once concurrent cargo work settles.

---

## Tested Scope

- Static review of auth router registration, logout GET interstitial, POST handler, session deletion service, session lookup middleware, CSRF rotation cookie, shared sidebar/mobile logout controls, 2FA logout links, and existing logout smoke test.
- Runtime curl smoke against local backend for unauthenticated and fake-cookie `/auth/logout`, `/logout`, and unsupported `POST /auth/logout`.
- Database schema review for `user_sessions` columns and token index.
- Fix verification with `tests/test_auth_logout_static.py`, updated `tests/test_auth_login_register.py`, `node --check`, and `rustfmt --edition 2021 --check` on touched Rust files.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/logout` | Nested auth route. |
| Route alias | `/logout` | Platform-level alias in `backend/src/main.rs`. |
| Backend handler | `GET /auth/logout`, `GET /logout` | `backend/src/auth/routes.rs::logout_page`; non-mutating compatibility interstitial. |
| Backend handler | `POST /auth/logout`, `POST /logout` | `backend/src/auth/routes.rs::logout`; deletes session and redirects. |
| Backend service | `backend/src/auth/service.rs::delete_session` | Deletes one `user_sessions` row by bearer token. |
| Middleware | `backend/src/auth/middleware.rs::get_current_user` | Session token becomes invalid after DB delete. |
| CSRF helper | `backend/src/auth/csrf.rs::rotation_cookie` | Expires `csrf_token` with `Path=/`. |
| Desktop UI | `frontend/platform/components/sidebar.html`, `frontend/platform/components/developer-sidebar-template.html` | Sign-out menu items use `#menu-item-sign-out`. |
| Desktop JS | `frontend/platform/static/js/profile-dropdown.js` | Submits `POST /auth/logout` with `csrf_token`; falls back to `/logout` interstitial if no token is available. |
| Mobile UI | `frontend/platform/components/mobile-menu.html` | Mobile sign-out menu items use `#mobile-menu-item-sign-out`. |
| Mobile JS | `frontend/platform/static/js/mobile-navigation.js` | Submits `POST /auth/logout` with `csrf_token`; falls back to `/logout` interstitial if no token is available. |
| 2FA links | `frontend/platform/auth-2fa.html`, `frontend/platform/auth-2fa-setup.html` | Native links to `/logout`. |
| Database table | `user_sessions` | `session_token` unique, `expires_at`, `remember_me`, `is_2fa_verified`. |
| Existing test | `tests/test_auth_login_register.py::test_logout` | Login/logout protected-route smoke, requires running backend and test account. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Desktop sign out | `#menu-item-sign-out` in shared sidebars | End current session and redirect to login | Yes, creates hidden POST form with CSRF | Yes, `POST /auth/logout` | Static regression added; browser click still needs recheck |
| Mobile sign out | `#mobile-menu-item-sign-out` in mobile menu | End current session and redirect to login | Yes, creates hidden POST form with CSRF | Yes | Static regression added; browser tap still needs recheck |
| 2FA verify logout | `auth-2fa.html` link `href="/logout"` | Abandon partially verified session | Native link to compatibility GET | Yes, GET interstitial posts to `/auth/logout` | Static route verified; browser flow still needs recheck |
| 2FA setup logout | `auth-2fa-setup.html` link `href="/logout"` | Abandon setup session | Native link to compatibility GET | Yes | Static route verified; browser flow still needs recheck |
| 403 sign out | `403.html` link `href="/logout"` | Switch account by logging out | Native link to compatibility GET | Yes | Static route verified; browser flow still needs recheck |
| Account deletion countdown | `account-deletion.html` sets `window.location.href = "/logout"` | Log out after deletion completion | Inline JS redirect to compatibility GET | Yes | Static route verified; browser flow still needs recheck |
| Canonical GET route | Direct `GET /auth/logout` | Render a non-mutating auto-submit form | Native navigation | Yes, `logout_page` only | Static regression confirms no `delete_session` in GET handler |
| Canonical POST route | `POST /auth/logout` | Delete session, expire cookies at `Path=/`, redirect | Hidden forms/shared JS | Yes | Static regression confirms route and cookie attributes |

---

## Frontend Findings

No broken frontend logout selectors were found. Shared desktop and mobile controls now submit a hidden `POST /auth/logout` form with the current `csrf_token`, falling back to `/logout` only when the page has no token and needs the compatibility interstitial.

The logout controls are implemented as clickable `div` elements in the shared sidebar/mobile menus rather than native buttons or links. That broader accessibility pattern belongs to the shared navigation components, but logout itself remains reachable through native links on 2FA and 403 pages.

---

## Backend Findings

### P2 - Logout mutates session state over GET without CSRF

Location:

- Backend: `backend/src/auth/routes.rs::logout`
- Router: `backend/src/auth/routes.rs` and `backend/src/main.rs`

Problem:

Logout deletes a `user_sessions` row on `GET /auth/logout` and `GET /logout`. Because `GET` is safe-listed and requires no CSRF token, any cross-site top-level navigation can force a user out of the platform when SameSite=Lax cookies are sent.

Expected:

Use a CSRF-protected `POST /auth/logout` for primary logout actions, keep `GET /logout` only as a compatibility redirect or confirmation page, and update shared logout controls to submit the protected action.

Evidence:

`POST /auth/logout` returned `403` with `Allow: GET,HEAD`; `GET /auth/logout` and `GET /logout` both returned `303 /auth/login` and perform the mutation.

Recommended fix:

Add a `POST /auth/logout` route protected by the existing CSRF middleware, convert shared logout controls to a form/fetch submit with the CSRF token, and decide whether legacy GET should only redirect to login or remain as a temporary compatibility path.

Fix applied:

`backend/src/auth/routes.rs` now registers `.route("/logout", get(logout_page).post(logout))`, and `backend/src/main.rs` registers the same for the `/logout` alias. `logout_page` renders a non-mutating auto-submit form, while `logout` performs the DB delete only on POST. Shared desktop/mobile JS now submits POST with the CSRF token.

### P3 - Canonical logout cookie expiry omits `Path=/`

Location:

- Backend: `backend/src/auth/routes.rs::logout`

Problem:

The session cookie is created with `Path=/`, but logout removes it via `jar.remove(Cookie::from(SESSION_COOKIE))`. The observed `Set-Cookie` header for `GET /auth/logout` is `poool_session=; Max-Age=0; Expires=...` with no `Path=/`. Browser cookie deletion requires the path to match the original cookie, so `/auth/logout` can leave a stale root-path `poool_session` cookie in the browser even after the DB row is deleted.

Expected:

Session-cookie expiry should include `Path=/`, `HttpOnly`, `SameSite=Lax`, and the same `Secure` setting as the session cookie.

Evidence:

Runtime curl with `Cookie: poool_session=fake; csrf_token=old` showed `set-cookie: poool_session=; Max-Age=0; Expires=...` and no `Path=/`; the CSRF rotation cookie did include `Path=/`.

Recommended fix:

Replace the removal cookie with an explicit expired `Cookie::build((SESSION_COOKIE, "")).path("/").http_only(true).secure(cookie_is_secure()).same_site(...).max_age(Duration::seconds(0))`.

Fix applied:

`backend/src/auth/routes.rs::expired_session_cookie` now expires `poool_session` with `Path=/`, `HttpOnly`, `SameSite=Lax`, the same `Secure` decision helper as login, and zero max-age. The logout handler adds that cookie instead of `jar.remove(Cookie::from(SESSION_COOKIE))`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| `/auth/logout` no session | `curl -i http://localhost:8888/auth/logout` | Redirect to login and rotate CSRF | `303 See Other`, `Location: /auth/login`, CSRF expired and reissued | Pass |
| `/logout` no session | `curl -i http://localhost:8888/logout` | Redirect to login and rotate CSRF | `303 See Other`, `Location: /auth/login`, CSRF expired and reissued | Pass |
| `/auth/logout` with cookie | `curl -i -H 'Cookie: poool_session=fake; csrf_token=old' http://localhost:8888/auth/logout` | Redirect, expire session cookie at `Path=/`, rotate CSRF | Redirect and DB delete attempted; `poool_session` expiry omitted `Path=/` | Needs recheck |
| `/logout` with cookie | `curl -i -H 'Cookie: poool_session=fake; csrf_token=old' http://localhost:8888/logout` | Redirect, expire session cookie at `Path=/`, rotate CSRF | Redirect; session expiry omitted explicit `Path=/` though alias default path is `/` | Needs recheck |
| Unsupported POST | `curl -i -X POST http://localhost:8888/auth/logout` | Current route contract is GET-only | `403 Forbidden`, `Allow: GET,HEAD` | Contract confirmed |
| Targeted Rust auth tests | `cd backend && cargo test auth:: --quiet` | Auth tests pass | 3 tests passed, 0 failed; no logout unit test was included by this filter | Pass with coverage gap |
| Static logout regression | `python3 -m pytest tests/test_auth_logout_static.py -q` | Route contract, cookie expiry, alias, and shared JS callers are fixed | 4 passed | Pass |
| Auth smoke update | `tests/test_auth_login_register.py::test_logout` static review | Runtime smoke should use POST and assert `Set-Cookie` expiry | Updated to check GET interstitial, POST logout, `Path=/`, `HttpOnly`, and `SameSite=Lax` | Pass |
| JS syntax | `node --check frontend/platform/static/js/profile-dropdown.js && node --check frontend/platform/static/js/mobile-navigation.js` | No syntax errors | Passed | Pass |
| Touched Rust formatting | `cd backend && rustfmt --edition 2021 --check src/auth/routes.rs src/main.rs` | Touched Rust files are formatted | Passed | Pass |
| Full Rust formatting | `cd backend && cargo fmt --check` | Whole backend formatted | Blocked by unrelated formatting drift in `backend/src/blog/routes.rs`, `backend/src/blog/service.rs`, and `backend/src/community/routes.rs` | Blocked |
| Isolated Rust typecheck | `cd backend && CARGO_TARGET_DIR=/tmp/poool-auth-logout-check cargo check --quiet` | Backend typechecks | Stopped after roughly 6 minutes with no diagnostic output while many other cargo/rustc jobs were running in parallel | Not completed |

---

## Security Findings

- Fixed: State-changing logout moved behind CSRF-protected POST.
- Fixed: Browser cookie expiry now includes root path and session cookie security attributes.
- No sensitive data is returned by the logout response.
- Server-side session invalidation uses the DB token and prevents reuse after deletion.

---

## Database Findings

`user_sessions` exists with a unique `session_token` index. `delete_session` deletes exactly one row by token and does not require a transaction because this route performs a single-table delete with no financial state.

The handler now logs DB deletion failures before expiring cookies. It still expires the browser cookie and redirects to login so users are not trapped on a failing logout path.

---

## Missing Tests

- Add browser E2E for desktop menu logout, mobile menu logout, 2FA partial-session logout, and account-deletion countdown logout.
- Add a DB-backed assertion to the auth smoke test if a stable test-user fixture is available, so it verifies the exact `user_sessions` row deletion in addition to protected-route rejection.

---

## Remaining Issues

1. Full authenticated browser E2E still needs to verify desktop, mobile, 2FA, 403, and account-deletion logout paths.
2. Whole-repo `cargo fmt --check` is currently blocked by unrelated formatting drift outside this logout fix.
3. Full `cargo check` should be rerun when the concurrent cargo workload settles.

---

## Final Status

`fixed, needs browser recheck`

Reason: both audit issues were fixed in code and covered by static/auth-smoke regression tests, but authenticated browser E2E remains to be run after the current multi-agent worktree settles.
