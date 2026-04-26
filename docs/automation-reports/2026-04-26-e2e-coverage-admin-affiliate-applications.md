# E2E Coverage Gap Audit: Admin Affiliate Applications

Date: 2026-04-26  
Automation: Daily POOOL E2E Coverage Gap Tracker  
Selected scope: `/admin/affiliate-applications` and `/api/admin/rewards/affiliates/{pending,approve,reject}`  
Report path: `docs/automation-reports/2026-04-26-e2e-coverage-admin-affiliate-applications.md`

## Production Readiness Verdict

Coverage is not adequate for production. The flow controls who can become an active affiliate and receive commission attribution, so it needs committed admin-review E2E coverage with database assertions and security negative cases before release.

## Existing Coverage Found

- `tests/test_e2e_affiliate.py` exercises affiliate onboarding, referral cookie attribution, dashboard access, and payout request. It does not open `/admin/affiliate-applications`; it approves the applicant by directly updating `affiliates.status` and `referral_code` in the database.
- `tests/test_e2e_affiliate_full_funnel.py` attempts a full lifecycle including admin approval API and payout, but `tests/conftest.py` excludes it from normal pytest collection. It is also stale against the current approval API because it omits `referral_code` and uses `commission_rate_bps: 500`, while the current backend accepts `1..=450`.
- No test was found for `frontend/platform/admin/affiliate-applications.html` or `frontend/platform/admin/js/admin-affiliate-applications.js`.
- No committed test was found for the pending-list response schema, the admin detail modal, rejection path, CSRF denial, missing `affiliates.manage`, KYC gating, duplicate referral-code conflict, or audit-log persistence.

## Missing Coverage

- Happy path: admin with `affiliates.manage` loads the page, sees seeded pending applications, opens details, approves one applicant with a valid referral code/rate, and verifies it disappears from pending.
- Reject path: admin rejects a pending application with a reason and verifies `affiliates.status = 'terminated'` plus an `affiliate.rejected` audit row.
- Auth redirect: unauthenticated visits to `/admin/affiliate-applications` and `/admin/affiliate-applications.html` should redirect to `/auth/login`.
- Authorization: authenticated admin without `affiliates.manage` should be blocked from page/data/mutation access, including sidebar visibility if that remains the intended contract.
- CSRF: approve/reject POSTs without a valid token must fail and must not change `affiliates` or `audit_logs`.
- Backend state: approve must persist `status = 'active'`, uppercase referral code, exact `commission_rate_bps`, `approved_at`, and `affiliate.approved` audit data.
- Validation failures: short/long referral code, disallowed code characters, duplicate code, commission rates outside `1..=450`, missing rejection reason, malformed applicant ID, non-pending applicant, and KYC-not-approved applicant.
- UI validation parity: referral-code client validation should match backend length/character rules so invalid submissions fail locally or surface the backend error clearly.
- Error states: pending-list API failure and approve/reject API failure should show deterministic visible errors and leave buttons usable after retry.
- Accessibility/responsive: details/approve/reject modals need keyboard open/close/focus-trap checks and a mobile viewport smoke for clipped actions.
- Security rendering: seeded user-controlled fields in email, name, traffic source, URL, company, tax ID, phone, audience, and rejection reason should not execute HTML/JS.

## Suggested Test Files And Names

- `tests/e2e/test_admin_affiliate_applications.py`
- `test_admin_affiliate_applications_requires_login`
- `test_admin_affiliate_applications_requires_affiliates_manage`
- `test_admin_affiliate_applications_lists_pending_and_opens_details`
- `test_admin_affiliate_applications_approve_persists_state_and_audit`
- `test_admin_affiliate_applications_reject_persists_state_and_audit`
- `test_admin_affiliate_applications_blocks_missing_csrf`
- `test_admin_affiliate_applications_validation_errors_do_not_mutate_state`
- `test_admin_affiliate_applications_modal_keyboard_and_mobile`

## Test Data Required

- Admin session with `affiliates.manage`.
- Admin session without `affiliates.manage`.
- Pending affiliate user with approved KYC, profile fields, and user-controlled URL/text fixtures.
- Pending affiliate user without approved KYC.
- Existing active affiliate owning a referral code for duplicate-code testing.
- Audit-log cleanup keyed by seeded applicant IDs.

## Priority Order

1. HTTP+DB regression for pending list, approve, reject, CSRF denial, permission denial, KYC gate, duplicate code, and audit rows.
2. Playwright UI regression for page load, detail modal, approve/reject modal flows, validation messages, and retryable API failures.
3. Mobile and keyboard modal checks.
4. XSS fixture checks for table/detail rendering and URL allowlisting.

## Minimum Regression Suite Before Release

- One committed test that creates a pending affiliate with approved KYC, approves through the real admin API/UI, and verifies `affiliates`, `audit_logs`, and pending-list removal.
- One committed test that rejects a pending affiliate and verifies durable status/audit state.
- Security negative tests for no session, missing permission, missing CSRF, KYC-not-approved, duplicate referral code, and invalid applicant ID.
- UI smoke for pending list, details modal, approve modal, reject modal, API failure state, and keyboard/mobile behavior.
