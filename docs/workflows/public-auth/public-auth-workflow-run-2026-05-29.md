# Public/Auth Workflow Run - 2026-05-29

Environment:
- Local backend: `http://localhost:8888`
- Database: local `poool`
- Browser/E2E harness: Playwright via pytest

Executed:
- `python3 -m pytest tests/test_auth*_static.py tests/test_auth_login_register.py -q`
- `python3 -m pytest tests/e2e/test_auth_login.py tests/e2e/test_public_property.py tests/e2e/test_user_lifecycle.py -q`

Result:
- Static/API auth checks passed: `40 passed in 0.66s`
- Browser/E2E auth, public property, and lifecycle checks passed: `26 passed in 52.88s`

Issues found and fixed during this run:
- Public preview property pages could call `/api/villas/:asset_id/performance` with a slug such as `echo-beach-loft`, which produced a failed API request because the endpoint expects a UUID. Public previews now omit the asset-id data hook used by the authenticated performance widget.
- The public property mobile flow did not expose the expected contact CTA. A mobile WhatsApp contact section was added so the public contact workflow is visible and testable.
- The authenticated property and cart pages could render with missing sidebar context during the buy/cart lifecycle, causing a `500` after redirects. Both pages now pass the same sidebar user context as the rest of the investor shell.
- Login-time 2FA coverage was tightened so TOTP-enabled accounts must land on `/auth/2fa` instead of bypassing to `/marketplace`.
- Email verification, password reset, logout, and forgot-password static contracts were updated to match the durable transactional outbox and the current route wiring.

Workflow coverage added:
- Verify public preview property pages do not emit failed performance API requests for static slugs.
- Verify public property mobile contact CTA is present.
- Verify login happy path, invalid credentials, CSRF, remember-me, OAuth button visibility, mobile layout, authenticated redirect, and TOTP challenge redirect.
- Verify disposable login/register accounts are created and cleaned up without relying on a shared fixed test email.
- Verify the cart/purchase lifecycle reaches cart and checkout pages without sidebar-template render failures.

Remaining gaps:
- OAuth provider callback behavior still depends on provider credentials and needs a configured staging run for full external-provider proof.
- Blog/legal/public-content publishing readback remains covered by separate workflow files and still needs its own full browser pass.
