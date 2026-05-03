# Remaining Documented Issues Snapshot

Date: 2026-04-28
Source: `docs/issue-tracking/page-review-tracker.yml`

## Summary

- Non-fixed documented issues: 93
- high: 38
- medium: 45
- low: 10
- status `fixed, needs runtime recheck`: 1
- status `needs recheck`: 12
- status `open`: 79
- status `partially fixed`: 1

Critical documented issues are currently at zero. The remaining items are broad multi-page work and runtime/E2E verification items that should be handled in focused implementation passes.

## Open Issues

| Severity | Issue | Page | Route | Status | Title |
|---|---|---|---|---|---|
| high | PAGE-ISSUE-0039 | admin.affiliate-fraud-route | `/admin/affiliate-fraud` | open | Affiliate fraud clean route resolves to missing template |
| high | PAGE-ISSUE-0040 | admin.affiliate-fraud-route | `/admin/affiliate-fraud` | open | Fraud graph API response does not match UI contract |
| high | PAGE-ISSUE-0041 | admin.affiliate-fraud-route | `/admin/affiliate-fraud` | open | Fraud scan missing fine-grained affiliate permission |
| high | PAGE-ISSUE-0292 | admin.blockchain-contracts | `/admin/blockchain-contracts` | open | Blockchain contracts page and treasury API are overbroad |
| high | PAGE-ISSUE-0293 | admin.blockchain-contracts | `/admin/blockchain-contracts` | open | Contract rows render DB fields through innerHTML |
| high | PAGE-ISSUE-0148 | admin.community.post-detail | `/admin/community/post-detail` | open | Admin post detail renderer injects community fields as HTML |
| high | PAGE-ISSUE-0149 | admin.community.post-detail | `/admin/community/post-detail` | open | Post moderation mutations are non-transactional and weakly audited |
| high | PAGE-ISSUE-0427 | admin.community.post-detail | `/admin/community/post-detail` | open | Post detail APIs lack community permission checks |
| high | PAGE-ISSUE-0139 | admin.community.posts | `/admin/community/posts` | open | Community posts admin table injects unescaped user content |
| high | PAGE-ISSUE-0560 | admin.email-marketing | `/admin/email-marketing` | open | Email campaign and template changes lack audit, approval, and abuse controls |
| high | PAGE-ISSUE-0160 | admin.kyc | `/admin/kyc` | open | KYC routes lack KYC-specific permission gates |
| high | PAGE-ISSUE-0161 | admin.kyc | `/admin/kyc` | open | KYC document signed URLs are overbroad and best-effort audited |
| high | PAGE-ISSUE-0162 | admin.kyc | `/admin/kyc` | open | KYC decisions lack audit logs and atomic side effects |
| high | PAGE-ISSUE-0167 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Alert API failures render fake operational alerts |
| high | PAGE-ISSUE-0168 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Alert rows render database text with innerHTML |
| high | PAGE-ISSUE-0169 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Marketplace alert routes lack permission gates |
| high | PAGE-ISSUE-0170 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Alert actions lack audit logs and missing-row checks |
| high | PAGE-ISSUE-0205 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Fee management routes do not enforce marketplace.manage |
| high | PAGE-ISSUE-0206 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Fee controls show success without persistence |
| high | PAGE-ISSUE-0207 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Fee list API masks database failures as empty state |
| high | PAGE-ISSUE-0208 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Fee resolver ignores accepted developer fee scope |
| high | PAGE-ISSUE-0528 | admin.marketplace.orders | `/admin/marketplace/orders` | needs recheck | Marketplace orders APIs lack granular permission checks |
| high | PAGE-ISSUE-0529 | admin.marketplace.orders | `/admin/marketplace/orders` | needs recheck | Admin order cancel is not locked or audited |
| high | PAGE-ISSUE-0530 | admin.marketplace.orders | `/admin/marketplace/orders` | needs recheck | Open orders page renders mock financial orders on API failure |
| high | PAGE-ISSUE-0531 | admin.marketplace.orders | `/admin/marketplace/orders` | needs recheck | Open order rows render backend values through innerHTML |
| high | PAGE-ISSUE-0002 | admin.marketplace.reconciliation | `/admin/marketplace/reconciliation` | open | Reconciliation page displays mock mismatch data when the API fails |
| high | PAGE-ISSUE-0279 | admin.rewards | `/admin/rewards` | open | Rewards management mutations lack granular permissions |
| high | PAGE-ISSUE-0283 | admin.settings | `/admin/settings` | open | Platform settings and maintenance actions lack granular authorization |
| high | PAGE-ISSUE-0284 | admin.support | `/admin/support` | open | Support ticket list and bulk update APIs lack support permissions and audit logs |
| high | PAGE-ISSUE-0285 | admin.support-ticket | `/admin/support-ticket` | open | Support ticket detail and reply actions lack support permissions and durable audit |
| high | PAGE-ISSUE-0286 | admin.system | `/admin/system` | open | System dashboard calls unregistered jobs, webhooks, sessions, and reset routes |
| high | PAGE-ISSUE-0287 | admin.system | `/admin/system` | open | System maintenance and session operations lack granular authorization and audit |
| high | PAGE-ISSUE-0291 | admin.users | `/admin/users` | open | User directory exposes PII and status mutation without granular user permissions |
| high | PAGE-ISSUE-0387 | cart.cart | `/cart` | open | Populated cart HTML uses incomplete manual escaping for asset data |
| high | PAGE-ISSUE-0388 | cart.cart | `/cart` | open | Cart quantity update fails open when availability lock cannot be read |
| high | PAGE-ISSUE-0499 | community.partial-feed | `/community/partials/feed/list` | needs recheck | Comment creation can leave stale feed counters |
| high | PAGE-ISSUE-0555 | community.partial-feed | `/community/partials/feed/list` | needs recheck | Feed reaction button used invalid schema value |
| high | PAGE-ISSUE-0391 | developer.asset-detail | `/developer/asset-detail` | open | Asset detail destructive/publish controls are placeholders that imply success |
| medium | PAGE-ISSUE-0023 | admin.admins | `/admin/admins` | open | Admin directory staff PII and security posture reads are not audit logged |
| medium | PAGE-ISSUE-0042 | admin.affiliate-fraud-route | `/admin/affiliate-fraud` | open | IP overlap scan button is not backed by backend logic |
| medium | PAGE-ISSUE-0043 | admin.affiliate-fraud-route | `/admin/affiliate-fraud` | open | Freeze Node danger action is dead UI |
| medium | PAGE-ISSUE-0294 | admin.blockchain-contracts | `/admin/blockchain-contracts` | open | Treasury API masks database failures as empty success |
| medium | PAGE-ISSUE-0380 | admin.blog | `/admin/blog` | open | Blog CMS taxonomy form fields lack explicit labels |
| medium | PAGE-ISSUE-0381 | admin.blog | `/admin/blog` | open | Blog CMS exposes controls that require permissions the page does not require |
| medium | PAGE-ISSUE-0382 | admin.blog-editor | `/admin/blog-editor` | open | Blog cover upload lacks server-side image type validation |
| medium | PAGE-ISSUE-0383 | admin.blog-editor | `/admin/blog-editor` | open | Blog editor URL override fields lack field-specific labels |
| medium | PAGE-ISSUE-0384 | admin.blog-editor | `/admin/blog-editor` | open | Blog editor exposes publish/archive actions without checking granular permissions |
| medium | PAGE-ISSUE-0150 | admin.community.post-detail | `/admin/community/post-detail` | open | Tag update accepts unvalidated arbitrary tag arrays |
| medium | PAGE-ISSUE-0151 | admin.community.post-detail | `/admin/community/post-detail` | open | Moderation actions rely on prompt alert confirm flows |
| medium | PAGE-ISSUE-0152 | admin.community.post-detail | `/admin/community/post-detail` | open | Admin post detail loads unused external CDN scripts |
| medium | PAGE-ISSUE-0527 | admin.dividends | `/admin/dividends` | fixed, needs runtime recheck | Dividend lifecycle E2E coverage is stale |
| medium | PAGE-ISSUE-0163 | admin.kyc | `/admin/kyc` | open | KYC backend failures render as empty states |
| medium | PAGE-ISSUE-0164 | admin.kyc | `/admin/kyc` | open | Document viewer injects signed URL data with innerHTML |
| medium | PAGE-ISSUE-0165 | admin.kyc | `/admin/kyc` | open | KYC modals and sort controls lack keyboard semantics |
| medium | PAGE-ISSUE-0166 | admin.kyc | `/admin/kyc` | open | Rejection reason validation is client-side only |
| medium | PAGE-ISSUE-0171 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Alert list database failures return empty data |
| medium | PAGE-ISSUE-0172 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Alert status transition semantics are weak |
| medium | PAGE-ISSUE-0173 | admin.marketplace.alerts | `/admin/marketplace/alerts` | open | Alerts page lacks loading empty and error states |
| medium | PAGE-ISSUE-0209 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Active fee configuration validation is ambiguous |
| medium | PAGE-ISSUE-0210 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Stored fee data renders through raw HTML |
| medium | PAGE-ISSUE-0211 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Fee mutations are not audit logged |
| medium | PAGE-ISSUE-0212 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Settlement and minimum fee fields lack backend support |
| medium | PAGE-ISSUE-0532 | admin.marketplace.orders | `/admin/marketplace/orders` | needs recheck | Marketplace orders pagination is not reachable |
| medium | PAGE-ISSUE-0539 | admin.marketplace.primary-escrow | `/admin/marketplace/primary-escrow` | needs recheck | Primary escrow loading and error states lack accessible recovery |
| medium | PAGE-ISSUE-0280 | admin.rewards | `/admin/rewards` | open | Rewards affiliate approval button calls an unregistered endpoint |
| medium | PAGE-ISSUE-0281 | admin.rewards | `/admin/rewards` | open | Rewards application links do not restrict URL schemes |
| medium | PAGE-ISSUE-0282 | admin.roles | `/admin/roles` | open | Roles page falls back to demo data instead of showing authorization failure |
| medium | PAGE-ISSUE-0290 | admin.users | `/admin/users` | open | Tracked clean URL /admin/users returns 404 instead of the users page |
| medium | PAGE-ISSUE-0460 | auth.auth-signup | `/auth/signup` | open | Verification email delivery lacks outbox retry worker |
| medium | PAGE-ISSUE-0389 | cart.cart | `/cart` | open | Generated cart item controls lack robust accessible labels |
| medium | PAGE-ISSUE-0500 | community.partial-feed | `/community/partials/feed/list` | needs recheck | Feed partial masks backend failures |
| medium | PAGE-ISSUE-0501 | community.partial-feed | `/community/partials/feed/list` | needs recheck | Reaction buttons ignore current user state |
| medium | PAGE-ISSUE-0390 | developer.add-asset | `/developer/add-asset` | open | Asset type selection is mouse-only and not semantic |
| medium | PAGE-ISSUE-0392 | developer.asset-detail | `/developer/asset-detail` | open | Developer cap table renders admin user-detail links |
| medium | PAGE-ISSUE-0483 | developer.document-upload-step3 | `/developer/document-upload-step3` | open | Document upload controls lack complete accessible names |
| medium | PAGE-ISSUE-0484 | developer.document-upload-step3 | `/developer/document-upload-step3` | open | Document upload page renders hardcoded demo document rows before JS cleanup |
| medium | PAGE-ISSUE-0482 | developer.fragment-assets | `/developer/dashboard/fragments/assets` | open | Assets fragment returns HTTP 200 for unauthenticated requests |
| medium | PAGE-ISSUE-0481 | developer.fragment-chart | `/developer/dashboard/fragments/chart` | open | Chart fragment returns HTTP 200 for unauthenticated requests |
| medium | PAGE-ISSUE-0485 | developer.property-content | `/developer/property-content` | open | Property media upload copy does not match accepted formats or limits |
| medium | PAGE-ISSUE-0488 | developer.property-content | `/developer/property-content` | open | Generated property image remove buttons have no accessible names |
| medium | PAGE-ISSUE-0487 | developer.settings | `/developer/settings` | open | Developer logo upload UI advertises SVG files that the backend rejects |
| medium | PAGE-ISSUE-0543 | kyc.identity-verification | `/kyc` | partially fixed | KYC mutation audit and email side effects are swallowed |
| medium | PAGE-ISSUE-0550 | kyc.identity-verification | `/kyc` | open | KYC email delivery still lacks durable outbox |
| low | PAGE-ISSUE-0295 | admin.blockchain-contracts | `/admin/blockchain-contracts` | open | Copy address action lacks visible feedback |
| low | PAGE-ISSUE-0296 | admin.blockchain-contracts | `/admin/blockchain-contracts` | open | Unused external HTMX dependency remains on admin blockchain page |
| low | PAGE-ISSUE-0385 | admin.blog-persona | `/admin/blog-persona` | open | Blog persona output textarea lacks an explicit label |
| low | PAGE-ISSUE-0386 | admin.blog-strategy | `/admin/blog-strategy` | open | Blog strategy output textarea lacks an explicit label |
| low | PAGE-ISSUE-0213 | admin.marketplace.fees | `/admin/marketplace/fees` | open | Fee page tabs lack accessible tab semantics |
| low | PAGE-ISSUE-0502 | community.partial-feed | `/community/partials/feed/list` | needs recheck | Feed engagement controls lack state semantics |
| low | PAGE-ISSUE-0506 | community.partial-tab | `/community/partials/:tab` | needs recheck | Dynamic community tab E2E coverage is incomplete |
| low | PAGE-ISSUE-0486 | developer.submission-success | `/developer/submission-success` | open | Submission success WhatsApp contact link points to a placeholder |
| low | PAGE-ISSUE-0566 | kyc.identity-verification | `/kyc` | open | KYC upload can orphan private object after DB failure |
| low | PAGE-ISSUE-0003 | marketplace.tax-report | `/tax-report` | open | Tax report route requires format despite route comment and path contract |
