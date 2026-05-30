# Public Visitor, Auth, And Content Workflows

Purpose: Verify public visitor discovery, auth entry, marketing/legal/blog readback, error pages, language entrypoints, and post-auth landing behavior without assuming a logged-in session.

Roles: Public Visitor, Investor, Admin for content publishing readback.

Primary pages:
- `/`, `/landing`, `/landing-v2`, `/poool_app_home`, `/poool_app_ssr`, `/en/`, `/id/`
- `/p/:slug`, `/property-public`, `/blog/`, `/blog/:slug`, `/blog/category/:slug`, `/blog/article`
- `/terms`, `/legal/terms`, `/privacy-policy`, `/privacy`, `/legal/privacy`, `/currency-policy`, `/cookies`, `/legal/cookies`, `/imprint`, `/legal/imprint`, `/gdpr-data-request`, `/aml-kyc-policy`, `/legal/aml-kyc-policy`
- `/auth/login`, `/auth/signup`, `/auth/forgot-password`, `/auth/reset-password`, `/auth/verify-email`, `/auth/2fa`, `/auth/2fa/setup`, `/auth/2fa/step-up`, `/auth/google`, `/auth/google/callback`, `/logout`, `/welcome`
- `/403`, `/404`, `/500`, `/maintenance`

Backend/API surfaces:
- `backend/src/auth/routes.rs` for login, signup, OAuth, verification, reset, logout, 2FA, step-up.
- `backend/src/blog/mod.rs` for blog index, category, article, and feed routes.
- `backend/src/legal/mod.rs` and top-level page routes in `backend/src/lib.rs` for legal/static/error pages.
- `backend/src/assets/mod.rs` and `backend/src/assets/routes.rs` for public property detail and featured asset readback.
- `backend/src/rewards/mod.rs` for referral entry routes `/r/:code` and `/rewards/:code`.

Prerequisites:
- Local or staging environment with public routing enabled.
- Disposable email addresses for signup/password reset checks.
- Admin account only for the content publish/readback branch; otherwise run public checks logged out.
- Test article/property fixtures use `Workflow Test` naming when mutated by admin.

Steps:
1. Start with no active session and load `/`, `/en/`, and `/id/`; verify header, mobile menu, language links, property cards, blog links, WhatsApp/contact links, and legal footer links.
2. Open `/p/:slug` and `/property-public`; exercise gallery/lightbox, calculator sliders, quick-add amounts, financial tabs, document disabled states, developer links, and signup/login return-path CTAs.
3. Open `/blog/`, `/blog/:slug`, and `/blog/category/:slug`; verify category filters, article links, back links, sign-in CTAs, social/footer links, and mobile blog menu.
4. Open legal pages and error pages; verify read-only content, recovery links, support/contact links, and authenticated-shell fallback where applicable.
5. Run signup with missing fields, weak password, missing terms acceptance, valid disposable data, and Google signup entry where configured.
6. Run login with valid credentials, invalid password, unknown email, Remember me on/off, password visibility toggle, Google login entry, and already-authenticated redirect.
7. Run forgot/reset/verify-email resend flows with disposable accounts; verify no account-existence leakage.
8. Run `/auth/2fa`, `/auth/2fa/setup`, and `/auth/2fa/step-up` for accounts with and without TOTP enabled; record current 500/redirect findings as `needs-product-confirmation` until product confirms intended behavior.
9. Confirm logout clears session and protected pages redirect to `/auth/login`.
10. For admin-managed content, publish a `Workflow Test` blog/notification/legal-content change through the cross-role content workflow, then reload public and investor pages to verify readback.

Expected Result:
- Public visitors can navigate marketing, property, blog, legal, and account-entry surfaces without a session.
- Auth mutations create, verify, reset, step-up, or destroy sessions with explicit user-visible states and no credential leakage.
- Public readback reflects admin-published content only after publish and reverts during cleanup.

Coverage Matrix:

| Case | Expected Result |
|------|-----------------|
| Logged-out public navigation | Public pages render and links resolve without session-only data leakage. |
| Signup validation | Invalid inputs are blocked; valid disposable account is created once. |
| Login validation | Bad credentials fail generically; valid credentials redirect to marketplace/welcome. |
| Password reset | Reset request and token submit do not disclose account existence. |
| Email verification | Resend and confirm produce clear success/error states. |
| 2FA setup/step-up | Valid account reaches setup/verification; current local 500 remains tracked as `needs-product-confirmation`. |
| Public content readback | Published admin content is visible after reload to public and authenticated users. |
| Error/legal fallback | 403/404/500/legal routes provide recovery links and do not mutate data. |

Negative Cases:
- Invalid or expired reset/verify token.
- Signup with duplicate email.
- OAuth provider missing or callback with invalid state.
- Return-path attempts to external domains.
- Public property with disabled document download.
- Local routes currently rendering 404/500: classify with source and expected decision rather than guessing.

Audit / DB / Financial Checks:
- Auth/session rows are created and revoked as expected; no password or token appears in logs.
- Referral cookies from `/r/:code` and `/rewards/:code` persist through signup without overwriting stronger attribution unexpectedly.
- Admin content publish/unpublish writes audit logs with actor, target, and timestamp.
- No money movement occurs in public/auth flows; any displayed investment amount is not persisted until cart/checkout.

Cleanup:
- Delete disposable accounts or mark them as test fixtures according to the local test policy.
- Revert or archive `Workflow Test` content.
- Clear referral/session cookies and local browser state.
