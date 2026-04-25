# Production Readiness Automation Coverage

This file tracks coverage across recurring production-readiness automations.

Source trackers and reports:

- Page review tracker: `docs/page-review-tracker.yml`
- Human-readable page tracker: `docs/PAGE_REVIEW_TRACKER.md`
- Daily page audit reports: `docs/page-audits/`
- Build/test reports: `docs/build-test-reports/`
- Route/API contract audits: `docs/route-contract-audits/`
- Financial audits: `docs/financial-audits/`
- Security audits: `docs/security-audits/`
- E2E coverage tracker: `docs/E2E_COVERAGE_TRACKER.md`

## Automation Status

| Automation | Schedule | Last Run | Last Report | Status | Notes |
|------------|----------|----------|-------------|--------|-------|
| Daily Build And Test Gate | Daily | 2026-04-25 | `docs/build-test-reports/2026-04-25-build-test.md` | partially fixed | Rust format/test/clippy, support endpoint, and public property smoke pass; broad Python `tests/` still blocked by legacy test collection and an E2E timeout. |
| Daily POOOL Page Audit | Daily | 2026-04-25 | `docs/page-audits/2026-04-25-admin-marketplace-analytics.md` | fixed, E2E verified | Fixed `/admin/marketplace/analytics` stats/trades contracts, server-side marketplace permissions, Metabase fallback, filters, and error states. |
| Daily POOOL Safe Page Fix | Daily | 2026-04-25 | `docs/page-audits/2026-04-25-admin-community-badges.md` | fixed, E2E verified | Fixed `/admin/community/badges` permission, audit, validation, revoke UI, and modal gaps; authenticated browser recheck verified create/update/grant/revoke and audit-log rows. |
| Daily Route/API Contract Audit | Daily | 2026-04-25 | `docs/route-contract-audits/2026-04-25-route-contract-admin-approvals.md` | active | Audited `/admin/approvals`; no missing routes, but approval execution, CSRF, permissions, validation, and executor contracts need fixes. |
| Daily E2E Coverage Gap Tracker | Daily | 2026-04-25 | `docs/automation-reports/2026-04-25-e2e-coverage-admin-approvals.md` | active | Audited `/admin/approvals`; page/API smoke exists, but maker/checker mutation, CSRF, authz, state, audit, concurrency, and UI E2E coverage is missing. |
| Financial Logic Audit | Mon/Wed/Fri | - | - | active | Audits money, orders, wallet, fees, commissions, and settlement flows. |
| Security Review | Tue/Thu | - | - | active | Audits auth, authorization, CSRF, IDOR, uploads, leaks, admin exposure. |

## Page Coverage Summary

Update this section after page, route-contract, E2E, security, or performance audits.

| Page / Area | URL / Route | Template | Last Page Audit | Last Route Contract Audit | Last E2E Coverage Check | Last Security Check | Status | Missing Coverage |
|-------------|-------------|----------|-----------------|---------------------------|-------------------------|--------------------|--------|------------------|
| Admin Approvals | `/admin/approvals` | `frontend/platform/admin/approvals.html` | 2026-04-25 | 2026-04-25 | 2026-04-25 gap audit: `docs/automation-reports/2026-04-25-e2e-coverage-admin-approvals.md` | 2026-04-25 | needs E2E coverage | Existing broad page/API smoke only; add maker/checker create/reject/approve, CSRF, permissions, four-eyes self-block, exact cents/state/audit verification, concurrency/idempotency, error-state, reject modal, filters, and mobile/keyboard tests. |
| Admin Affiliate Applications | `/admin/affiliate-applications` | `frontend/platform/admin/affiliate-applications.html` | 2026-04-25 | - | authenticated browser keyboard smoke recommended | 2026-04-25 | partially fixed | Inline rendering cleanup and admin HTML auth-response decision remain. |
| Admin Affiliate Finance | `/admin/affiliate-finance` | `frontend/platform/admin/affiliate-finance.html` | 2026-04-25 | - | stale payout tests; authenticated payout fixture needed | 2026-04-25 | needs recheck | Inline payout handler cleanup, locked-ID payout update, tax-gate UI, modal a11y. |
| Admin Affiliate Fraud Route | `/admin/affiliate-fraud` | expected `frontend/platform/admin/affiliate-fraud.html`; existing alternate `frontend/platform/admin/admin-affiliate-fraud.html` | 2026-04-25 | - | authenticated route/render + graph fixture needed | 2026-04-25 | needs recheck | Canonical route/template, graph API contract, affiliate permission gate, IP scan, Freeze Node. |
| Admin Dashboard | `/admin/` | `frontend/platform/admin/index.html` | 2026-04-25 | - | 2026-04-25 targeted Playwright dashboard regression passed | 2026-04-25 | fixed, E2E verified | Code fixes and targeted E2E cover safe row rendering, stats errors, health contract/fallback semantics, minimal search, notification badge, deposits label, visible errors, and CDN cleanup; optional mobile smoke remains. |
| Admin KYC | `/admin/kyc` | `frontend/platform/admin/kyc.html` | 2026-04-25 | - | authenticated KYC fixture needed | 2026-04-25 | needs recheck | Enforce KYC/PII permissions, make approve/reject transactional and audit logged, fail closed on document signing/audit errors, validate rejection reasons, add visible load errors, and fix modal/keyboard accessibility. |
| Admin Audit Logs | `/admin/audit-logs` | `frontend/platform/admin/audit-logs.html` | 2026-04-25 | - | authenticated browser click/keyboard pass needed | 2026-04-25 | fixed, needs browser recheck | Verify filters, CSV export, retry state, and modal keyboard behavior in browser. |
| Admin Community AMAs | `/admin/community/amas` | `frontend/platform/admin/community/amas.html` | 2026-04-25 | - | 2026-04-25 targeted Playwright CRUD/moderation E2E passed | 2026-04-25 | fixed, E2E verified | Create, status, detail, answer, feature, and audit-log coverage passed; mobile layout remains a separate smoke target. |
| Admin Community Announcements | `/admin/community/announcements` | `frontend/platform/admin/community/announcements.html` | 2026-04-25 | 2026-04-25 targeted Playwright admin publish/listing smoke | Persist targeted E2E as committed regression test | 2026-04-25 | fixed, E2E verified | Local browser/API recheck passed for admin page load, list API, no-CSRF rejection, and UI publish persistence. |
| Admin Community Badges | `/admin/community/badges` | `frontend/platform/admin/community/badges.html` | 2026-04-25 | - | 2026-04-25 22:42 authenticated browser create/update/grant/revoke + audit-log recheck passed | 2026-04-25 | fixed, E2E verified | Targeted browser E2E and SQL verification passed for badge create/update/grant/revoke, post-revoke zero active award state, and `badge.create`/`badge.update`/`badge.grant`/`badge.revoke` audit rows. |
| Admin Community Challenges | `/admin/community/challenges` | `frontend/platform/admin/community/challenges.html` | 2026-04-25 | - | 2026-04-25 targeted HTTP+DB and browser keyboard/mobile E2E passed | 2026-04-25 | completed | Create/toggle/audit-log/stale-toggle plus modal keyboard/mobile E2E passed. |
| Admin Community Circle Detail | `/admin/community/circle-detail` | `frontend/platform/admin/community/circle-detail.html` | 2026-04-25 | - | authenticated circle fixture needed | 2026-04-25 | fixed, needs browser recheck | CSRF headers, community permission gates, force-transfer target validation, update validation, and accessible status regions added locally. |
| Admin Community Circles | `/admin/community/circles` | `frontend/platform/admin/community/circles.html` | 2026-04-25 | - | authenticated list/delete fixture needed | 2026-04-25 | fixed, needs browser recheck | Verify community.manage gates, transactional delete audit log, retryable errors, and pagination with safe fixture. |
| Admin Community Comments | `/admin/community/comments` | `frontend/platform/admin/community/comments.html` | 2026-04-25 | - | authenticated comment fixture needed | 2026-04-25 | fixed, needs browser recheck | Verify patched permissions, transactional audit rows, visible load errors, 404 handling, and responsive/keyboard behavior with real admin session. |
| Admin Community Index | `/admin/community/` | `frontend/platform/admin/community/index.html` | 2026-04-25 | - | authenticated overview fixture needed | 2026-04-25 | needs recheck | Fix raw feed HTML rendering, wrong announcements data source, stats schema/error handling, stats permission gate, and visible load errors. |
| Admin Community Leaderboard | `/admin/community/leaderboard` | `frontend/platform/admin/community/leaderboard.html` | 2026-04-25 | - | authenticated XP fixture needed | 2026-04-25 | fixed, needs browser recheck | Static/build fixes passed for API permissions, transactional XP/audit writes, payload validation, negative-XP guard, visible errors, modal a11y, and local JS; verify grant/revoke/audit rows with safe fixture. |
| Admin Community Post Detail | `/admin/community/post-detail` | `frontend/platform/admin/community/post-detail.html` | 2026-04-25 | - | authenticated post/comment/report fixture needed | 2026-04-25 | needs recheck | Enforce community permissions for detail APIs, make post moderation transactional and audited, render community fields safely, validate tags, replace prompt/alert flows, and remove unused CDN scripts. |
| Admins | `/admin/admins` | `frontend/platform/admin/admins.html` | 2026-04-25 | - | needs authenticated mutation recheck | 2026-04-25 | partially fixed | Invite acceptance lifecycle, admin security-action auth/transactions, mobile/keyboard E2E. |
| Admin Asset Details | `/admin/asset-details` | `frontend/platform/admin/asset-details.html` | 2026-04-25 | - | authenticated asset fixture + mutation recheck needed | 2026-04-25 | fixed, needs recheck | Verify secured document links, publish/funding mutations, audit logs, and mobile/keyboard behavior with real admin session. |
| Admin Asset Tokenize | `/admin/asset-tokenize` | `frontend/platform/admin/asset-tokenize.html` | 2026-04-25 | - | safe chain fixture + authenticated browser recheck needed | 2026-04-25 | needs recheck | CSRF token source, tokenization permission, idempotency/race guard, durable audit/reconciliation, clone address parsing, generic nav. |
| Admin Assets | `/admin/assets` | `frontend/platform/admin/assets.html` | 2026-04-25 | - | authenticated list/toggle fixture needed | 2026-04-25 | fixed, needs recheck | Verify patched DB error path, visible API/toggle errors, sortable table keyboard behavior, and featured audit log. |
| Admin Marketplace Alerts | `/admin/marketplace/alerts` | `frontend/platform/admin/marketplace/alerts.html` | 2026-04-25 | - | authenticated alert fixture needed | 2026-04-25 | needs recheck | Remove mock fallback, render alert rows safely, enforce marketplace.compliance/manage gates, audit action updates, check missing rows, and define alert status transitions. |
| Admin Marketplace Analytics | `/admin/marketplace/analytics` | `frontend/platform/admin/marketplace/analytics.html` | 2026-04-25 | - | 2026-04-25 authenticated Playwright analytics E2E passed | 2026-04-25 | fixed, E2E verified | Stats/cards, paginated trades, marketplace.view gates, Metabase disabled fallback, API errors, trade filters, iframe title, and noopener links verified. |
| Affiliate compliance route | `/admin/affiliate-compliance` | legacy redirect to `/admin/affiliate-applications` | 2026-04-25 | - | needs authenticated redirect + pending-user dashboard recheck | 2026-04-25 | fixed, needs recheck | Verify admin redirect and pending affiliate dashboard state with real sessions. |
| _To be populated by automations_ | - | - | - | - | - | - | pending | Inventory needed. |

## Backend Domain Coverage Summary

Update this section after route, financial, security, and build/test audits.

| Domain | Files / Routes | Last Build/Test Check | Last Route Contract Audit | Last Financial Audit | Last Security Audit | Status | Missing Coverage |
|--------|----------------|-----------------------|---------------------------|----------------------|--------------------|--------|------------------|
| auth | `backend/src/auth/` | - | - | not applicable | - | pending | Inventory needed. |
| wallet | `backend/src/wallet/` | - | - | - | - | pending | Inventory needed. |
| cart/checkout | `backend/src/cart/` | - | - | - | - | pending | Inventory needed. |
| payments | `backend/src/payments/` | - | - | - | - | pending | Inventory needed. |
| marketplace | `backend/src/marketplace/` | - | - | - | - | pending | Inventory needed. |
| payment methods | `backend/src/payment_methods/` | - | - | - | - | pending | Inventory needed. |
| rewards/affiliate | `backend/src/rewards/` | 2026-04-25 pass | - | - | - | passing | Formatting blocker fixed; `cargo fmt --check` passes. |
| kyc | `backend/src/kyc/` | - | - | - | - | pending | Inventory needed. |
| admin | `backend/src/admin/` | - | 2026-04-25 | mixed | - | needs recheck | `/api/admin/approvals` contract audited; remaining admin route groups still need inventory. |
| assets | `backend/src/assets/` | - | - | possible | - | pending | Inventory needed. |
| developer | `backend/src/developer/` | - | - | possible | - | pending | Inventory needed. |
| dividends | `backend/src/dividends/` | - | - | - | - | pending | Inventory needed. |
| blockchain | `backend/src/blockchain/` | - | - | - | - | pending | Inventory needed. |
| support | `backend/src/support/` | 2026-04-25 pass | - | not applicable | - | passing | Support ticket integration test passes; attachments degrade when storage is unavailable. |
| community | `backend/src/community/` | - | - | not applicable | - | pending | Inventory needed. |

## Open Coverage Gaps

| Gap | Area | Severity | First Seen | Owner / Decision Needed | Status | Notes |
|-----|------|----------|------------|--------------------------|--------|-------|
| `/admin/approvals` lacks production-grade maker/checker E2E coverage. | admin approvals | high | 2026-04-25 | automation / admin test owner | open | Add CSRF, permissions, four-eyes, create/reject/approve, exact cents/state/audit, concurrency, error-state, modal, filters, and mobile/keyboard tests. |
| Full page inventory not yet normalized into this tracker. | pages | medium | - | automation | open | Automations should populate page rows as they run. |
