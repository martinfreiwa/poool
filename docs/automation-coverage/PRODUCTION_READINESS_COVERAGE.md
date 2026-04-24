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
| Daily Build And Test Gate | Daily | - | - | active | Verifies build/test baseline. |
| Daily POOOL Page Audit | Daily | - | - | active | Audits one page per day. |
| Daily POOOL Safe Page Fix | Daily | - | - | active | Fixes one safe documented issue batch. |
| Daily Route/API Contract Audit | Daily | - | - | active | Compares frontend calls/forms/links with Axum routes. |
| Daily E2E Coverage Gap Tracker | Daily | - | - | active | Maps pages/routes/features to E2E coverage. |
| Financial Logic Audit | Mon/Wed/Fri | - | - | active | Audits money, orders, wallet, fees, commissions, and settlement flows. |
| Security Review | Tue/Thu | - | - | active | Audits auth, authorization, CSRF, IDOR, uploads, leaks, admin exposure. |

## Page Coverage Summary

Update this section after page, route-contract, E2E, security, or performance audits.

| Page / Area | URL / Route | Template | Last Page Audit | Last Route Contract Audit | Last E2E Coverage Check | Last Security Check | Status | Missing Coverage |
|-------------|-------------|----------|-----------------|---------------------------|-------------------------|--------------------|--------|------------------|
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
| rewards/affiliate | `backend/src/rewards/` | - | - | - | - | pending | Inventory needed. |
| kyc | `backend/src/kyc/` | - | - | - | - | pending | Inventory needed. |
| admin | `backend/src/admin/` | - | - | mixed | - | pending | Inventory needed. |
| assets | `backend/src/assets/` | - | - | possible | - | pending | Inventory needed. |
| developer | `backend/src/developer/` | - | - | possible | - | pending | Inventory needed. |
| dividends | `backend/src/dividends/` | - | - | - | - | pending | Inventory needed. |
| blockchain | `backend/src/blockchain/` | - | - | - | - | pending | Inventory needed. |
| support | `backend/src/support/` | - | - | not applicable | - | pending | Inventory needed. |
| community | `backend/src/community/` | - | - | not applicable | - | pending | Inventory needed. |

## Open Coverage Gaps

| Gap | Area | Severity | First Seen | Owner / Decision Needed | Status | Notes |
|-----|------|----------|------------|--------------------------|--------|-------|
| Full page inventory not yet normalized into this tracker. | pages | medium | - | automation | open | Automations should populate page rows as they run. |
