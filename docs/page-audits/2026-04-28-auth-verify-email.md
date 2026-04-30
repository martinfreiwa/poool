# Page Audit: Verify email

Date: 2026-04-28
Status: fixed, needs_e2e_recheck
Auditor: ChatGPT/Codex
Page URL: `/auth/verify-email`
Template: `frontend/platform/verify-email.html`
JavaScript: shared HTMX from `frontend/platform/components/head.html`
CSS: `frontend/platform/static/css/login.css` via `extra_css=['login']`
Backend Routes: `GET /auth/verify-email`, `POST /auth/resend-verification`

---

## Summary

The original audit found that `/auth/verify-email?token=...` did not consume verification tokens, resend could falsely report success, and the resend form lacked accessible feedback/loading states. These implementation issues are now fixed in `backend/src/auth/routes.rs`, `backend/src/auth/service.rs`, `frontend/platform/verify-email.html`, and `frontend/platform/static/css/login.css`.

Remaining issue documented:

- Full runtime/browser verification is still needed with an email outbox or captured test email. Static regression coverage now protects the documented implementation fixes.

---

## Tested Scope

- Static review of `frontend/platform/verify-email.html`.
- Static review of `backend/src/auth/routes.rs` and `backend/src/auth/service.rs`.
- Schema review of `database/041_email_verification_tokens.sql`, `database/045_hash_tokens_migration.sql`, and `docs/DATABASE_SCHEMA.md`.
- Original runtime HTTP smoke against local `cargo run` server on `127.0.0.1:8888`.
- Post-fix static verification of the touched route/template/CSS code.
- Post-fix JavaScript syntax check for the verify-email inline script.
- Post-fix static regression test: `python3 -m pytest tests/test_auth_verify_email_static.py -q`.
- Post-fix backend compile: `CARGO_TARGET_DIR=/tmp/poool-verify-email-check cargo check -q`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/auth/verify-email` | Public page render. |
| URL with token | `/auth/verify-email?token=...` | Now consumed by `verify_email_page`; valid tokens are verified, invalid tokens render an expired-link state. |
| Template | `frontend/platform/verify-email.html` | Static check-email UI with resend form and login link. |
| Component | `frontend/platform/components/head.html` | Loads HTMX, Sentry, shared CSRF injection, shared JS/CSS. |
| CSS | `frontend/platform/static/css/login.css` | Auth layout styling. |
| Backend page route | `GET /auth/verify-email` | Renders pending/success/error states and consumes verification tokens. |
| Backend HTMX route | `POST /auth/resend-verification` | Requires a valid session, rate-limits by IP/user, and surfaces failures. |
| Backend service | `backend/src/auth/service.rs::create_email_verification_token` | Replaces older tokens, inserts hashed token, emails `/auth/verify-email?token=...`, and deletes undelivered tokens on send failure. |
| Backend service | `backend/src/auth/service.rs::verify_email` | Transactionally marks `users.email_verified = TRUE` and deletes token. |
| Database table | `users` | `email_verified BOOLEAN NOT NULL DEFAULT FALSE`. |
| Database table | `email_verification_tokens` | Hashed token, user FK, expiry, unique token hash. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Logo | `#logo-pool img`, template lines 10-11 | Decorative brand image with alt text. | Yes | Not needed | Rendered on GET. |
| Mail icon | `.mail-icon`, template lines 13-19 | Visual email state indicator. | Static | Not needed | Rendered on GET. |
| Heading/support copy | `#text`, `#supporting-text`, template lines 20-25 | Tell user pending/success/expired state. | Yes | Yes | Fixed; driven by route status context. |
| Error/success target | `#auth-error`, template line 30 | Receive HTMX success/error fragments. | Yes | Yes | Fixed for resend route errors; global CSRF middleware still returns blank 403 before handler. |
| Resend form | `#verify-email-form`, template lines 47-57 | POST `/auth/resend-verification` and show result. | Yes, HTMX | Yes | Fixed; requires valid session and does not falsely report no-session success. |
| Resend button | `#get-started-button`, template lines 49-53 | Submit once with loading/disabled feedback. | Yes | Not needed | Fixed with `aria-busy`, disabled state, and loading text. |
| Back to log in | `#back-to-login`, template lines 42-48 | Navigate to login. | Yes | Yes | Normal link. |

---

## Frontend Findings

### P1 - Verification token links are rendered as a generic check-email page

Status: fixed in implementation; needs runtime/browser recheck.

Location:

- Template: `frontend/platform/verify-email.html:21`
- Backend: `backend/src/auth/routes.rs:134`

Problem:

The email body links to `/auth/verify-email?token=...`, but the route handler renders the same static "Check your email" page for token and no-token requests. Runtime GET `/auth/verify-email?token=invalid-token` returned HTTP 200 with the same resend page and no invalid-token message.

Expected:

When `token` is present, the page route should attempt verification, consume the token, delete it, mark the account verified, and render/redirect to an explicit success or invalid/expired state.

Evidence:

`verify_email_page` accepts only `State(state)` and never reads query params; `service::verify_email` is marked `#[allow(dead_code)]`. Curl with `?token=invalid-token` returned the same "Check your email" page.

Fix:

`verify_email_page` now reads query parameters, calls `service::verify_email` when `token` is present, redirects valid tokens to a success state, and redirects invalid/expired tokens to an explicit expired-link state rendered by the template.

### P2 - Resend feedback can falsely report delivery

Status: fixed in implementation; needs runtime/browser recheck.

Location:

- Template: `frontend/platform/verify-email.html:33`
- Backend: `backend/src/auth/routes.rs:721`

Problem:

`POST /auth/resend-verification` returns "Verification email resent successfully!" even when no session exists. For authenticated users, the handler also discards token/email-send errors with `let _ = ...`.

Expected:

Unauthenticated users should see a neutral sign-in-required or expired-session message. Authenticated resend should surface safe delivery failure states or queue durable retry work.

Evidence:

Runtime GET to collect CSRF then POST with no session returned HTTP 200 and the success fragment. Static code shows `if let Some(cookie)` is optional and the success fragment is always returned.

Fix:

`POST /auth/resend-verification` now requires a valid unverified-session lookup, returns a visible expired-session error for missing sessions, rate-limits by IP and user, propagates token/email failures into safe HTMX errors, and no longer reports success when no email was queued.

### P2 - HTMX error and loading states are incomplete

Status: fixed in implementation; needs browser recheck.

Location:

- Template: `frontend/platform/verify-email.html:30`
- Template: `frontend/platform/verify-email.html:35`

Problem:

The form has no `aria-live` on the feedback region, no `hx-disabled-elt`/indicator/loading text, and no HTMX error-target behavior. A CSRF failure returns a blank 403, leaving users without visible guidance.

Expected:

The feedback target should be announced to assistive tech, submit should show busy/disabled state, and CSRF/rate-limit/server failures should render a useful message.

Evidence:

Runtime POST without CSRF returned HTTP 403 with empty body. Static template has `#auth-error` without ARIA role/live attributes and the button has no loading state.

Fix:

The template now has a focusable live feedback region, success/error styling, `aria-busy` state, disabled duplicate-submit behavior, and visible loading text for resend.

---

## Backend Findings

### P1 - Token verification service is implemented but unreachable

Status: fixed in implementation; needs runtime/browser recheck.

`backend/src/auth/service.rs::verify_email` correctly hashes the token, checks expiry, updates `users.email_verified`, deletes the token, and commits in a transaction. `GET /auth/verify-email?token=...` now calls it.

### P2 - Resend path lacks session-specific failure behavior and rate limiting

Status: fixed in implementation; needs runtime/browser recheck.

The resend endpoint is CSRF-protected by global middleware, requires a valid session before success, and applies endpoint-specific IP/user throttles. Token creation now deletes previous verification tokens for the user before inserting the new token, and removes the new token if email delivery fails.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Page load | `curl http://127.0.0.1:8888/auth/verify-email` | 200 with verify-email page | 200, page rendered with CSRF cookie | Pass |
| Invalid token URL | `curl /auth/verify-email?token=invalid-token` | Invalid/expired token state or redirect | Pre-fix: 200 generic "Check your email" page. Post-fix static regression verifies token query dispatch and invalid-token redirect. | Fixed; needs browser recheck |
| Resend without CSRF | `curl -X POST /auth/resend-verification` | CSRF rejection | 403 empty body | Pass security, fail UX |
| Resend with CSRF but no session | GET page for CSRF cookie, POST with `X-CSRF-Token` | Neutral unauthenticated/expired-session response | Pre-fix: 200 "Verification email resent successfully!" Static regression now verifies session requirement and no false token creation success. | Fixed; needs browser recheck |
| Backend compile | `cd backend && cargo check -q` | Successful compile | Passed | Pass |
| Post-fix JS syntax | Extract inline verify-email script and run `node --check` | Script parses | Passed | Pass |
| Post-fix static regression | `python3 -m pytest tests/test_auth_verify_email_static.py -q` | Verify documented auth verify-email protections | 4 passed | Pass |
| Post-fix backend compile | `CARGO_TARGET_DIR=/tmp/poool-verify-email-check cargo check -q` | Successful compile | Passed | Pass |
| Tracker regeneration | `python3 scripts/audit_page_review_tracker.py --write-md` | Regenerate Markdown tracker cleanly | Wrote `docs/PAGE_REVIEW_TRACKER.md`; no missing routes/templates reported | Pass |

---

## Security Findings

- Fixed: Email verification tokens are consumed by the public route.
- Fixed: Resend verification has endpoint-specific IP/user throttling.
- Fixed: No-session resend attempts now return a visible error instead of success.
- Remaining verification risk: full browser/outbox E2E is still needed.

---

## Database Findings

- `email_verification_tokens` has the expected `user_id`, `token_hash`, `expires_at`, and unique hash constraint.
- `users.email_verified` exists and defaults to false.
- The implemented verification mutation uses a transaction and is now reachable from the HTTP route.
- Resend now invalidates prior verification tokens for the same user before inserting a new token.
- If email delivery fails after token insertion, the undelivered token is deleted.

---

## Remaining Tests

- Full browser/outbox E2E signup-to-email-verification flow with a captured test email or test email outbox.
- Runtime route tests for valid, expired, reused, and invalid verification tokens against a test database.
- Runtime resend tests for authenticated success, CSRF failure, email-provider failure, and rate limiting.
- Browser accessibility recheck for feedback live region, loading state, keyboard navigation, and mobile layout.

---

## Recommended Fix Order

1. Add browser/outbox E2E coverage for signup, verify, login-after-verify, resend, and invalid-token paths.
2. Add runtime route tests for valid, invalid, expired, and reused verification tokens.
3. Add runtime resend tests for authenticated success, CSRF failure, email-provider failure, and rate limiting.

---

## Final Status

`fixed, needs_e2e_recheck`

Reason: The documented implementation issues are fixed and covered by static regression tests. Full browser/outbox E2E remains as the only documented follow-up for this page.
