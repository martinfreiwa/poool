# Route/API Contract Audit: Auth Login

Date: 2026-04-28
Auditor: ChatGPT/Codex
Selected scope: Auth Login (`/auth/login`)
Status: fixed; browser E2E still needed

## Summary

`/auth/login` has the expected page route, form POST route, CSRF token flow, HTMX error response, session cookie creation, remember-me mapping, and 2FA-aware redirect contract. No missing backend route was found for the login form or linked auth destinations.

Follow-up fixes on 2026-04-28 resolved the route-contract mismatches: the login template now gates the Google OAuth button with the backend `google_enabled` context, and HTMX auth CSRF failures now return an auth-styled HTML error fragment instead of a bare 403 body.

## Route And File Map

| Surface | Path / File | Contract Notes |
|---------|-------------|----------------|
| Page URL | `GET /auth/login` | Mounted by `backend/src/main.rs` through `auth::routes::router`; redirects authenticated users to `/marketplace`. |
| Login form | `POST /auth/login` | Registered in `backend/src/auth/routes.rs`; accepts URL-encoded form data. |
| Template | `frontend/platform/login.html` | Renders the form, social login button, links, CSRF field, and inline HTMX lifecycle handlers. |
| Shared auth head | `frontend/platform/components/auth-head.html` | Loads HTMX and injects `X-CSRF-Token` for mutating HTMX requests when the cookie exists. |
| CSRF middleware | `backend/src/auth/csrf.rs` | Double-submit cookie; validates header, form body, or query token for mutating requests. |
| Auth service | `backend/src/auth/service.rs` | Authenticates active, verified users; creates session records through route handler. |

## Frontend Action Inventory

| UI Action | Frontend Source | Method / URL | Request Contract | Response Contract Expected |
|-----------|-----------------|--------------|------------------|----------------------------|
| Load login page | Browser navigation | `GET /auth/login` | Optional `?error=` query; no session required. | HTML login page with CSRF token and optional escaped error. Authenticated users redirect to `/marketplace`. |
| Submit login form | `#login-form` | `POST /auth/login` plus `hx-post="/auth/login"` | `application/x-www-form-urlencoded` body with `email`, `password`, optional `remember=on`, and `csrf_token`; auth head also sends `X-CSRF-Token` when available. | On success, `HX-Redirect` or HTTP redirect to `/marketplace` or `/auth/2fa`; on validation failure, HTML fragment for `#auth-error`. |
| Google login | `#google-button` inline click | `GET /auth/google` | No body; optional existing session affects backend redirect. Button is rendered only when `google_enabled` is true. | Google OAuth redirect when configured. |
| Forgot password link | `#forgot-password-link` | `GET /auth/forgot-password` | No body. | Forgot-password HTML page. |
| Signup link | `#signup-link` | `GET /auth/signup` | No body. | Signup HTML page. |
| Back link | `#back-link` | `GET /` | No body. | Public/platform root behavior. |
| Password visibility toggle | inline click | Client-only | No backend contract. | Toggles password input type. |
| Testimonial carousel | inline click handlers | Client-only | No backend contract. | Rotates static testimonial data. |

## Backend Route Inventory

| Backend Route | Handler | Auth / Authorization | Request Contract | Response Contract |
|---------------|---------|----------------------|------------------|-------------------|
| `GET /auth/login` | `login_page` | Public; redirects if existing valid session. | Optional `error` query string. | HTML `login.html`; includes `csrf_token` and `google_enabled` context. |
| `POST /auth/login` | `login_submit` | Public, CSRF-protected, IP and email rate-limited. | URL-encoded `LoginForm`: `email`, `password`, optional `remember`. | HTMX success: `HX-Redirect`; non-HTMX success: redirect. HTMX auth errors: HTML alert fragment with matching status. |
| `GET /auth/google` | `google_redirect` | Public; authenticated users redirect to `/marketplace` unless link flow. | Optional `link=1/true`. | Redirect to Google OAuth with state/PKCE cookies, or back to login with `oauth_not_configured`. |
| `GET /auth/forgot-password` | `forgot_password_page` | Public. | No params required. | HTML or redirect to login on template failure. |
| `GET /auth/signup` | `signup_page` | Public. | Optional `error` query string. | HTML signup page. |

## Mismatches And Issues

### MEDIUM - Login template ignores the backend OAuth-enabled contract

Status: fixed 2026-04-28

Original evidence:

- `render_login` passes `google_enabled => state.config.google_oauth_enabled()` to `login.html`.
- `frontend/platform/login.html` always renders an active Google button that navigates to `/auth/google`.
- `GET /auth/google` redirects back to `/auth/login?error=oauth_not_configured` when Google OAuth is not configured.

Impact:

The page exposes a working-looking login method that the backend already knows is unavailable. Users can enter a dead OAuth round trip, and automated browser coverage must special-case an avoidable false action. This also duplicates the Google OAuth gap already tracked for auth pages.

Recommended fix:

Use the existing `google_enabled` template context to hide or disable the Google button when OAuth is not configured, with a non-actionable explanatory state if product wants the affordance visible.

Fix applied:

`frontend/platform/login.html` now wraps the Google OAuth button and divider in `{% if google_enabled %}`.

### LOW - CSRF failure response does not match the HTMX error-fragment contract

Status: fixed 2026-04-28

Original evidence:

- Normal HTMX login failures return `Html(render_auth_error_html(...))` from `login_error_response`.
- Global CSRF middleware rejects invalid non-API mutating requests with bare `StatusCode::FORBIDDEN`.
- The login page swaps any `>=400` HTMX response into `#auth-error`, so a stale/invalid token path can render a bare framework response instead of the alert fragment used by login errors.

Impact:

The login request is protected, but the failure contract is inconsistent. A stale login page or expired CSRF cookie can produce a confusing or unstyled error in the login error region instead of the expected accessible auth error fragment.

Recommended fix:

For HTMX non-API CSRF failures, return the same small HTML alert fragment style used by auth form errors, or add an auth-form-specific CSRF fallback that renders a clear refresh-and-retry message.

Fix applied:

`backend/src/auth/csrf.rs` now returns a `403` auth error HTML fragment for HTMX non-API CSRF failures.

## Missing Routes

None found for the selected scope.

## Dead UI Actions

None missing a backend route. The Google OAuth action is now hidden when backend configuration says the provider is disabled.

## Unused Backend Routes Noticed In Selected Scope

No unused backend routes were identified in this selected scope.

## CSRF, Auth, And Error Handling Notes

- `POST /auth/login` is protected by the global CSRF middleware.
- The form sends a hidden `csrf_token`, and the shared auth head adds `X-CSRF-Token` for HTMX when the cookie exists.
- Successful password login sets an HTTP-only `poool_session` cookie, rotates the CSRF cookie, and redirects to `/marketplace` or `/auth/2fa` depending on TOTP settings.
- Invalid credentials, unverified email, OAuth-only password attempts, and rate limits return HTMX-compatible HTML error fragments from the route handler.
- Existing page audit issue `PAGE-ISSUE-0448` was fixed in the same follow-up pass: failed-login observability paths no longer include raw submitted email addresses.

## Issue Counts

| Severity | Count |
|----------|-------|
| Critical | 0 |
| High | 0 |
| Medium | 0 open, 1 fixed |
| Low | 0 open, 1 fixed |
| Info | 0 |

## Recommended Fix Order

1. Add browser E2E for configured and unconfigured Google OAuth login states.
2. Add browser E2E for stale/invalid CSRF token feedback on the login form.
3. Keep the already-tracked broader login browser E2E work in the auth page backlog.

## Verification Performed

- Static route registration review in `backend/src/main.rs`.
- Static auth route review in `backend/src/auth/routes.rs`.
- Static CSRF middleware review in `backend/src/auth/csrf.rs`.
- Static login template and shared auth head review in `frontend/platform/login.html` and `frontend/platform/components/auth-head.html`.
- Static auth model/service contract review in `backend/src/auth/models.rs` and `backend/src/auth/service.rs`.
- No production application code was modified.
- Follow-up fix modified production code and reran static verification:
  - `python3 -m pytest tests/test_auth_login_hardening_static.py -q`
  - extracted inline login scripts and ran `node --check`
  - unsafe-pattern scan for raw failed-login email strings and script `.innerHTML`
