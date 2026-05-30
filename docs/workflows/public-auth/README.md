# Public And Auth Workflows

Purpose: Cover public visitor and authentication workflows that are not owned by Community or a cross-role business process.

Roles: Public Visitor, Investor, Developer, Admin.

Primary pages:
- `/`, `/id/`, `/landing`, `/landing-v2`, `/p/:slug`, `/blog`, `/blog/:slug`, `/blog/category/:slug`
- `/terms`, `/legal/terms`, `/privacy-policy`, `/privacy`, `/legal/privacy`, `/currency-policy`, `/cookies`, `/legal/cookies`, `/imprint`, `/legal/imprint`, `/gdpr-data-request`, `/aml-kyc-policy`, `/legal/aml-kyc-policy`
- `/auth/login`, `/auth/signup`, `/auth/google`, `/auth/google/callback`, `/auth/forgot-password`, `/auth/reset-password`, `/auth/verify-email`, `/auth/2fa`, `/auth/2fa/setup`, `/auth/2fa/step-up`, `/logout`, `/welcome`
- Error/offline pages: `/403`, `/404`, `/500`, `/maintenance`, `/offline`

Backend/API surfaces:
- Auth submit, signup, logout, verify/resend email, password reset, Google OAuth, TOTP setup/verify/step-up.
- Blog newsletter and public blog feeds.
- Public legal/static routes and protected-route redirects.

Prerequisites:
- Local backend is running.
- One public browser context with no session.
- Test investor/developer/admin accounts exist.
- OAuth, email, and 2FA flows use local/staging providers or documented disabled states.

Steps:
1. Open `/`, `/id/`, `/landing`, and `/landing-v2`; verify primary CTAs, language controls, mobile menu, public property cards, blog links, WhatsApp/contact links, and legal footer links.
2. Open public property pages through `/p/:slug`; verify gallery/lightbox, calculator, amount controls, financial tabs, documents, signup/login return path, and disabled document behavior where applicable.
3. Open blog index/article/category pages; verify category filters, article links, social/footer links, newsletter controls, mobile menu, and sign-in CTAs.
4. Open every legal/static page; verify `200` or documented recovery behavior for known 404-style pages.
5. Submit login happy path, invalid password, unknown email, empty fields, Remember me on/off, logout, and protected-route redirect.
6. Submit signup happy path, duplicate email, invalid password, missing terms, referral signup, and OAuth signup/login where configured.
7. Run forgot-password, reset-password, verify-email resend/confirm, and expired/invalid token cases.
8. Run 2FA setup, invalid code, valid code, login challenge, and step-up before sensitive action.
9. Verify `/welcome` decisions route correctly to marketplace or KYC.
10. Verify `/403`, `/404`, `/500`, `/maintenance`, and offline state have recovery actions and no broken layout.

Expected Result:
- Public and auth journeys are reachable, role-gated, and return users to the correct destination.
- Disabled or unavailable external providers show clear fallback states.
- No auth flow reveals account existence beyond allowed generic messages.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Public browsing | Navigation, CTAs, legal links, blog, and public property flows work without session. |
| Login/logout | Session is created, reused, remembered when requested, and removed on logout. |
| Signup/OAuth | New account is created or provider-disabled state is explicit. |
| Password reset/email verify | Tokens and resend flows handle valid, invalid, expired, and repeated requests. |
| 2FA/step-up | Sensitive flows require valid fresh verification. |
| Error/offline pages | Recovery links work and no sensitive data leaks. |

Negative Cases:
- Invalid credentials, rate limit, malformed OAuth callback, invalid reset token, missing terms, expired verification, disabled OAuth credentials, and protected URL without session.

Audit / DB / Financial Checks:
- Auth/session writes are audited where implemented.
- No money mutation occurs in this workflow.
- User/session rows are created, rotated, or removed according to auth state.

Cleanup:
- Delete disposable signup accounts or mark them inactive.
- Remove local email/reset tokens generated for the run.

Run History:
- 2026-05-29: [Public/Auth workflow run](./public-auth-workflow-run-2026-05-29.md) verified auth static/API contracts plus browser/E2E login, public property, and lifecycle coverage. The run fixed public preview performance fetches, mobile property contact CTA visibility, sidebar context on authenticated property/cart pages, and tightened 2FA/login coverage.
