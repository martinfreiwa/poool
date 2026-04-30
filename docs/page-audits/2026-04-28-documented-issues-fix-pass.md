# Documented Issues Fix Pass

Date: 2026-04-28
Scope: page-review tracker documented issues

## Fixed In This Pass

- `admin.approvals`: fixed `PAGE-ISSUE-0044` through `PAGE-ISSUE-0048` in tracker documentation after verifying the current working-tree code and static tests for approval locking, granular permissions, executor contracts, queue error propagation, and reject/busy UI states.
- `admin.user-details`: fixed `PAGE-ISSUE-0288` and `PAGE-ISSUE-0289` in tracker documentation after verifying the current working-tree code and static tests for granular authorization, PII audit logging, and transactional profile/tier updates.

## Verification

- `python3 -m pytest tests/admin/test_admin_approvals_static.py -q`
- `node --check frontend/platform/static/js/admin-approvals.js`
- `rustfmt --edition 2021 --check backend/src/admin/approvals.rs`
- `python3 -m pytest tests/admin/test_admin_user_details_static.py -q`
- `node --check frontend/platform/static/js/admin-user-details.js`
- `rustfmt --edition 2021 --check backend/src/admin/users.rs`
- `python3 scripts/audit_page_review_tracker.py --write-md`
- scoped `git diff --check`

## Remaining Open Documented Issues

After this pass, the tracker has no open critical issues. The remaining open queue is 85 issues: 34 high, 41 medium, and 10 low. One additional medium issue, `PAGE-ISSUE-0527`, is fixed but still needs runtime recheck.

| Severity | Page | Issue | Title |
|---|---|---|---|
| high | `admin.affiliate-finance` | `PAGE-ISSUE-0035` | Payout can mark unsummed payable commissions as paid |
| high | `admin.affiliate-finance` | `PAGE-ISSUE-0368` | Affiliate finance page lacks page-level permission gate |
| high | `admin.affiliate-fraud-route` | `PAGE-ISSUE-0039` | Affiliate fraud clean route resolves to missing template |
| high | `admin.affiliate-fraud-route` | `PAGE-ISSUE-0040` | Fraud graph API response does not match UI contract |
| high | `admin.affiliate-fraud-route` | `PAGE-ISSUE-0041` | Fraud scan missing fine-grained affiliate permission |
| high | `admin.blockchain-contracts` | `PAGE-ISSUE-0292` | Blockchain contracts page and treasury API are overbroad |
| high | `admin.blockchain-contracts` | `PAGE-ISSUE-0293` | Contract rows render DB fields through innerHTML |
| high | `admin.community.post-detail` | `PAGE-ISSUE-0148` | Admin post detail renderer injects community fields as HTML |
| high | `admin.community.post-detail` | `PAGE-ISSUE-0149` | Post moderation mutations are non-transactional and weakly audited |
| high | `admin.community.post-detail` | `PAGE-ISSUE-0427` | Post detail APIs lack community permission checks |
| high | `admin.community.posts` | `PAGE-ISSUE-0139` | Community posts admin table injects unescaped user content |
| high | `admin.email-marketing` | `PAGE-ISSUE-0560` | Email campaign and template changes lack audit, approval, and abuse controls |
| high | `admin.kyc` | `PAGE-ISSUE-0160` | KYC routes lack KYC-specific permission gates |
| high | `admin.kyc` | `PAGE-ISSUE-0161` | KYC document signed URLs are overbroad and best-effort audited |
| high | `admin.kyc` | `PAGE-ISSUE-0162` | KYC decisions lack audit logs and atomic side effects |
| high | `admin.marketplace.alerts` | `PAGE-ISSUE-0167` | Alert API failures render fake operational alerts |
| high | `admin.marketplace.alerts` | `PAGE-ISSUE-0168` | Alert rows render database text with innerHTML |
| high | `admin.marketplace.alerts` | `PAGE-ISSUE-0169` | Marketplace alert routes lack permission gates |
| high | `admin.marketplace.alerts` | `PAGE-ISSUE-0170` | Alert actions lack audit logs and missing-row checks |
| high | `admin.marketplace.fees` | `PAGE-ISSUE-0205` | Fee management routes do not enforce marketplace.manage |
| high | `admin.marketplace.fees` | `PAGE-ISSUE-0206` | Fee controls show success without persistence |
| high | `admin.marketplace.fees` | `PAGE-ISSUE-0207` | Fee list API masks database failures as empty state |
| high | `admin.marketplace.fees` | `PAGE-ISSUE-0208` | Fee resolver ignores accepted developer fee scope |
| high | `admin.marketplace.reconciliation` | `PAGE-ISSUE-0002` | Reconciliation page displays mock mismatch data when the API fails |
| high | `admin.rewards` | `PAGE-ISSUE-0279` | Rewards management mutations lack granular permissions |
| high | `admin.settings` | `PAGE-ISSUE-0283` | Platform settings and maintenance actions lack granular authorization |
| high | `admin.support` | `PAGE-ISSUE-0284` | Support ticket list and bulk update APIs lack support permissions and audit logs |
| high | `admin.support-ticket` | `PAGE-ISSUE-0285` | Support ticket detail and reply actions lack support permissions and durable audit |
| high | `admin.system` | `PAGE-ISSUE-0286` | System dashboard calls unregistered jobs, webhooks, sessions, and reset routes |
| high | `admin.system` | `PAGE-ISSUE-0287` | System maintenance and session operations lack granular authorization and audit |
| high | `admin.users` | `PAGE-ISSUE-0291` | User directory exposes PII and status mutation without granular user permissions |
| high | `cart.cart` | `PAGE-ISSUE-0387` | Populated cart HTML uses incomplete manual escaping for asset data |
| high | `cart.cart` | `PAGE-ISSUE-0388` | Cart quantity update fails open when availability lock cannot be read |
| high | `developer.asset-detail` | `PAGE-ISSUE-0391` | Asset detail destructive/publish controls are placeholders that imply success |
| medium | `admin.admins` | `PAGE-ISSUE-0023` | Admin directory staff PII and security posture reads are not audit logged |
| medium | `admin.affiliate-finance` | `PAGE-ISSUE-0036` | Finance board hides tax-document payout gate |
| medium | `admin.affiliate-finance` | `PAGE-ISSUE-0038` | Affiliate payout E2E tests are stale |
| medium | `admin.affiliate-fraud-route` | `PAGE-ISSUE-0042` | IP overlap scan button is not backed by backend logic |
| medium | `admin.affiliate-fraud-route` | `PAGE-ISSUE-0043` | Freeze Node danger action is dead UI |
| medium | `admin.blockchain-contracts` | `PAGE-ISSUE-0294` | Treasury API masks database failures as empty success |
| medium | `admin.blog` | `PAGE-ISSUE-0380` | Blog CMS taxonomy form fields lack explicit labels |
| medium | `admin.blog` | `PAGE-ISSUE-0381` | Blog CMS exposes controls that require permissions the page does not require |
| medium | `admin.blog-editor` | `PAGE-ISSUE-0382` | Blog cover upload lacks server-side image type validation |
| medium | `admin.blog-editor` | `PAGE-ISSUE-0383` | Blog editor URL override fields lack field-specific labels |
| medium | `admin.blog-editor` | `PAGE-ISSUE-0384` | Blog editor exposes publish/archive actions without checking granular permissions |
| medium | `admin.community.post-detail` | `PAGE-ISSUE-0150` | Tag update accepts unvalidated arbitrary tag arrays |
| medium | `admin.community.post-detail` | `PAGE-ISSUE-0151` | Moderation actions rely on prompt alert confirm flows |
| medium | `admin.community.post-detail` | `PAGE-ISSUE-0152` | Admin post detail loads unused external CDN scripts |
| medium | `admin.kyc` | `PAGE-ISSUE-0163` | KYC backend failures render as empty states |
| medium | `admin.kyc` | `PAGE-ISSUE-0164` | Document viewer injects signed URL data with innerHTML |
| medium | `admin.kyc` | `PAGE-ISSUE-0165` | KYC modals and sort controls lack keyboard semantics |
| medium | `admin.kyc` | `PAGE-ISSUE-0166` | Rejection reason validation is client-side only |
| medium | `admin.marketplace.alerts` | `PAGE-ISSUE-0171` | Alert list database failures return empty data |
| medium | `admin.marketplace.alerts` | `PAGE-ISSUE-0172` | Alert status transition semantics are weak |
| medium | `admin.marketplace.alerts` | `PAGE-ISSUE-0173` | Alerts page lacks loading empty and error states |
| medium | `admin.marketplace.fees` | `PAGE-ISSUE-0209` | Active fee configuration validation is ambiguous |
| medium | `admin.marketplace.fees` | `PAGE-ISSUE-0210` | Stored fee data renders through raw HTML |
| medium | `admin.marketplace.fees` | `PAGE-ISSUE-0211` | Fee mutations are not audit logged |
| medium | `admin.marketplace.fees` | `PAGE-ISSUE-0212` | Settlement and minimum fee fields lack backend support |
| medium | `admin.rewards` | `PAGE-ISSUE-0280` | Rewards affiliate approval button calls an unregistered endpoint |
| medium | `admin.rewards` | `PAGE-ISSUE-0281` | Rewards application links do not restrict URL schemes |
| medium | `admin.roles` | `PAGE-ISSUE-0282` | Roles page falls back to demo data instead of showing authorization failure |
| medium | `admin.users` | `PAGE-ISSUE-0290` | Tracked clean URL /admin/users returns 404 instead of the users page |
| medium | `auth.auth-signup` | `PAGE-ISSUE-0460` | Verification email delivery lacks outbox retry worker |
| medium | `cart.cart` | `PAGE-ISSUE-0389` | Generated cart item controls lack robust accessible labels |
| medium | `developer.add-asset` | `PAGE-ISSUE-0390` | Asset type selection is mouse-only and not semantic |
| medium | `developer.asset-detail` | `PAGE-ISSUE-0392` | Developer cap table renders admin user-detail links |
| medium | `developer.document-upload-step3` | `PAGE-ISSUE-0483` | Document upload controls lack complete accessible names |
| medium | `developer.document-upload-step3` | `PAGE-ISSUE-0484` | Document upload page renders hardcoded demo document rows before JS cleanup |
| medium | `developer.fragment-assets` | `PAGE-ISSUE-0482` | Assets fragment returns HTTP 200 for unauthenticated requests |
| medium | `developer.fragment-chart` | `PAGE-ISSUE-0481` | Chart fragment returns HTTP 200 for unauthenticated requests |
| medium | `developer.property-content` | `PAGE-ISSUE-0485` | Property media upload copy does not match accepted formats or limits |
| medium | `developer.property-content` | `PAGE-ISSUE-0488` | Generated property image remove buttons have no accessible names |
| medium | `developer.settings` | `PAGE-ISSUE-0487` | Developer logo upload UI advertises SVG files that the backend rejects |
| medium | `kyc.identity-verification` | `PAGE-ISSUE-0550` | KYC email delivery still lacks durable outbox |
| low | `admin.affiliate-finance` | `PAGE-ISSUE-0037` | Payout modal lacks dialog keyboard semantics |
| low | `admin.affiliate-finance` | `PAGE-ISSUE-0369` | Affiliate finance notification action is unwired |
| low | `admin.blockchain-contracts` | `PAGE-ISSUE-0295` | Copy address action lacks visible feedback |
| low | `admin.blockchain-contracts` | `PAGE-ISSUE-0296` | Unused external HTMX dependency remains on admin blockchain page |
| low | `admin.blog-persona` | `PAGE-ISSUE-0385` | Blog persona output textarea lacks an explicit label |
| low | `admin.blog-strategy` | `PAGE-ISSUE-0386` | Blog strategy output textarea lacks an explicit label |
| low | `admin.marketplace.fees` | `PAGE-ISSUE-0213` | Fee page tabs lack accessible tab semantics |
| low | `developer.submission-success` | `PAGE-ISSUE-0486` | Submission success WhatsApp contact link points to a placeholder |
| low | `kyc.identity-verification` | `PAGE-ISSUE-0566` | KYC upload can orphan private object after DB failure |
| low | `marketplace.tax-report` | `PAGE-ISSUE-0003` | Tax report route requires format despite route comment and path contract |
