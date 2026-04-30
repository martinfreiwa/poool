# Page Audit: Affiliate Onboarding

Date: 2026-04-26
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/onboarding`
Template: `frontend/platform/affiliate-onboarding.html`
JavaScript: `frontend/platform/static/js/affiliate-onboarding.js`
CSS: `frontend/platform/static/css/affiliate-onboarding.css`, `frontend/platform/static/css/affiliate-promo.css`, `frontend/platform/static/css/cards-template.css`, `frontend/platform/static/css/forms-template.css`
Backend Routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, `backend/src/kyc/mod.rs`, `backend/src/kyc/routes.rs`

---

## Summary

The affiliate onboarding page is a real authenticated wizard and submits to a real backend endpoint, but it is not production-ready. The client enforces the quiz and KYC gate, while the backend accepts `exam_passed: true` without validating `exam_answers` and does not enforce approved KYC at submission time. Several route-state and UX gaps also remain, including no redirect for already pending/active affiliates, silent KYC status failures, incomplete field validation, and weak legal-tab accessibility.

---

## Tested Scope

- Reviewed `frontend/platform/affiliate-onboarding.html` UI controls, wizard steps, forms, checkboxes, quiz radios, tab controls, success state, and navigation links.
- Reviewed `frontend/platform/static/js/affiliate-onboarding.js` step validation, KYC status fetch, quiz evaluation, POST payload, error handling, and legal tab behavior.
- Reviewed `backend/src/rewards/mod.rs` route registration and `backend/src/rewards/routes.rs` onboarding submit handler.
- Reviewed `backend/src/kyc/mod.rs`, `backend/src/kyc/routes.rs`, and `backend/src/kyc/service.rs` for `/api/kyc/status`.
- Reviewed affiliate schema migrations `database/072_affiliate_core_system.sql`, `database/073_affiliate_profile_data.sql`, and `database/076_affiliate_system_gaps.sql`.
- Reviewed existing affiliate E2E scripts in `tests/test_e2e_affiliate.py` and `tests/test_e2e_affiliate_full_funnel.py`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/affiliate/onboarding` | Protected HTML page; unauthenticated runtime probe returned `303 /auth/login`. |
| Template | `frontend/platform/affiliate-onboarding.html` | Five-step wizard with profile, KYC, tax, agreements, exam, and pending review state. |
| JS | `frontend/platform/static/js/affiliate-onboarding.js` | Owns navigation, validation, KYC status fetch, legal tabs, and application submit. |
| CSS | `frontend/platform/static/css/affiliate-onboarding.css` | Page-specific wizard styling. |
| Backend page route | `GET /affiliate/onboarding` | Registered in `backend/src/rewards/mod.rs`; served by `page_affiliate_onboarding`. |
| Backend API route | `POST /api/affiliate/onboarding/submit` | Writes `affiliates` and `affiliate_policy_acceptances`. |
| Backend API route | `GET /api/kyc/status` | Used by step 2 to show approved KYC state. |
| Database table | `affiliates` | Stores profile, tax fields, status, referral code, accepted policy version. |
| Database table | `affiliate_policy_acceptances` | Stores one row per accepted policy. |
| Database table | `kyc_records` | Read by `/api/kyc/status`; not enforced by onboarding submit. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Wizard stepper | `#wizard-stepper`, `.step-item` | Show active/completed step and progress. | Yes, via `showStep` and `updateProgressBar`. | No backend needed. | Static verified; browser interaction not authenticated. |
| Traffic source select | `#traffic-source` | Required profile field. | Yes, validated before step 2; converted to POOOL dropdown when available. | Persisted to `affiliates.traffic_source`. | Static verified. |
| Audience size select | `#audience-size` | Required profile field. | Yes, validated before step 2. | Persisted to `affiliates.audience_size`. | Static verified. |
| Main URL input | `#main-url` | Required URL/social profile. | Partially; client only checks non-empty at step transition. | Server requires `http://` or `https://`. | Static verified. |
| Phone input | `#phone-number` | Required phone number. | Broken validation; not checked on step 1. | Server rejects empty value. | Static verified. |
| Step 1 continue | `onclick="nextStep(2)"` | Save local profile values and advance. | Partially; misses phone validation. | No backend until final submit. | Static verified. |
| KYC status | `#kyc-verified`, `#kyc-pending` | Show approved KYC or required state. | Partially; fetches `/api/kyc/status`, but failures are silent. | API exists. | Static verified. |
| Start KYC link | `a[href="/kyc"]` | Navigate to identity verification page. | Link only. | `/kyc` route exists in KYC router. | Static verified. |
| Step 2 continue | `onclick="nextStep(3)"` | Advance only after KYC approved. | Client-only gate. | Submit API does not enforce KYC. | Static verified. |
| Tax ID input | `#tax-id` | Required tax identifier. | Yes via `checkValidity()`. | Persisted to `affiliates.tax_id`. | Static verified. |
| Company name input | `#company-name` | Optional legal company name. | Yes. | Persisted to `affiliates.company_name`. | Static verified. |
| Legal tabs | `.legal-tab[data-target]` | Switch document sections. | Yes, via `switchLegalTab`. | No backend needed. | Static verified; a11y gaps found. |
| Policy checkboxes | `#cb-terms`, `#cb-conduct`, `#cb-materials`, `#cb-payout`, `#cb-privacy` | Require acceptance of all five documents. | Yes. | Server validates all five names and writes acceptance rows. | Static verified. |
| Quiz radios | `input[name="q1"]` through `q5` | Require 100% correct quiz. | Client validates exact answers. | Backend does not validate answers. | Static verified. |
| Submit Application | `#submit-exam-btn` | Submit application, disable while loading, show pending review on success. | Yes. | Real POST route exists with transaction. | Static verified; unauthenticated POST rejected by CSRF. |
| Success dashboard link | `a[href="/rewards"]` | Navigate after pending review. | Link only. | `/rewards` route exists. | Static verified. |

---

## Frontend Findings

### P2 - Phone field can be skipped until final backend rejection

Location:

- Template: `frontend/platform/affiliate-onboarding.html:110`
- JS: `frontend/platform/static/js/affiliate-onboarding.js:163`

Problem:

Step 1 marks phone number as required in the HTML, but `validateStep(1)` only checks traffic source, audience size, and main URL. Users can advance through the wizard without a phone number and only find out at final submit.

Expected:

Step 1 should call `profile-form.checkValidity()` or explicitly validate every required field before advancing.

Evidence:

Static review of `validateStep(1)` shows no `phone-number` validation.

Recommended fix:

Use native form validation for the full profile form, then keep the custom dropdown error decoration for select controls.

### P2 - KYC status failures are silent and leave users stuck

Location:

- Template: `frontend/platform/affiliate-onboarding.html:141`
- JS: `frontend/platform/static/js/affiliate-onboarding.js:15`

Problem:

`fetchKycStatus()` returns silently on non-OK responses and catches network errors without showing a retry or error state. If the KYC API fails, users only see "Identity Verification Required" and cannot distinguish missing KYC from a platform error.

Expected:

The KYC panel should show a visible retryable error when `/api/kyc/status` fails.

Evidence:

`fetchKycStatus()` has `if (!res.ok) return;` and an empty catch block.

Recommended fix:

Add a visible inline status area for loading, failed, not started, pending, in review, approved, and rejected states.

### P3 - Legal tabs lack tab semantics and keyboard behavior

Location:

- Template: `frontend/platform/affiliate-onboarding.html:197`
- JS: `frontend/platform/static/js/affiliate-onboarding.js:285`

Problem:

The legal document controls are plain buttons with inline `onclick`; they do not expose `role="tablist"`, `role="tab"`, `aria-selected`, `aria-controls`, or arrow-key navigation.

Expected:

Legal document navigation should follow accessible tab semantics or be presented as normal buttons with explicit pressed state and keyboard handling.

Evidence:

Static review of the tab markup and `switchLegalTab()`.

Recommended fix:

Add ARIA tab attributes, update selected state in JS, and support Left/Right/Home/End navigation.

---

## Backend Findings

### P1 - Backend accepts passed quiz flag without validating quiz answers

Location:

- Backend: `backend/src/rewards/routes.rs:378`
- Model: `backend/src/rewards/models.rs:102`
- Tests: `tests/test_e2e_affiliate_full_funnel.py:97`

Problem:

The server only checks `form.exam_passed`; the answer validation block is explicitly removed. Any authenticated user can submit `exam_passed: true` with absent or wrong `exam_answers` and bypass the compliance exam. Existing E2E data in `tests/test_e2e_affiliate_full_funnel.py` sends wrong answer values while expecting success, which codifies the bypass.

Expected:

The backend should validate `exam_answers` against the same canonical answer key and reject missing or incorrect answers. The client quiz is only UX and cannot be trusted.

Evidence:

`backend/src/rewards/routes.rs:387` says answer validation was removed, while the model still accepts `exam_answers`.

Recommended fix:

Move the quiz answer key to backend-owned validation, ignore client-supplied `exam_passed`, and update tests to cover missing, wrong, and correct answers.

### P1 - KYC approval gate is client-side only at application submission

Location:

- Template: `frontend/platform/affiliate-onboarding.html:141`
- JS: `frontend/platform/static/js/affiliate-onboarding.js:202`
- Backend: `backend/src/rewards/routes.rs:350`

Problem:

Step 2 blocks unverified users only in JavaScript by checking `#kyc-verified.style.display`. The submit endpoint does not query `kyc_records` or require approved KYC before inserting/updating an affiliate application.

Expected:

The submit endpoint should enforce the same KYC rule server-side or the product should change the UI copy to say KYC is required for approval, not for submission.

Evidence:

Static review of `submit_affiliate_onboarding_handler` found no KYC status query before writing `affiliates`.

Recommended fix:

Add an approved-KYC check in the submit transaction path, or explicitly allow pending applications while documenting/admin-surfacing the KYC status.

### P2 - Duplicate-application DB errors are masked as no existing application

Location:

- Backend: `backend/src/rewards/routes.rs:456`

Problem:

The duplicate guard uses `.fetch_optional(...).await.unwrap_or(None)`. A database read failure is treated as "no affiliate record", so the code continues into the write transaction instead of returning a clear failure.

Expected:

DB read errors should be logged and returned as a safe 500 before any write attempt.

Evidence:

Static review of the existing status lookup.

Recommended fix:

Replace `unwrap_or(None)` with explicit error handling and return a JSON error.

### P2 - Field validation allows arbitrary profile enum values and weak URL data

Location:

- Backend: `backend/src/rewards/routes.rs:412`
- Schema: `database/073_affiliate_profile_data.sql`

Problem:

The frontend offers fixed values for traffic source and audience size, but the backend only checks non-empty strings. It also accepts any URL beginning with `http://` or `https://`, with no length guard or normalization.

Expected:

The backend should enforce an allowlist for `traffic_source` and `audience_size`, bound lengths before DB writes, and normalize or validate profile URLs before storing them for admin review.

Evidence:

Static review found only empty checks and prefix checks before persistence.

Recommended fix:

Validate enum values against the template options, reject overlong fields before Postgres errors, and use a URL parser for scheme/host validation.

### P2 - Already pending or active affiliates can still load the onboarding page

Location:

- Backend: `backend/src/rewards/routes.rs:117`

Problem:

The page route only requires an authenticated session and always serves the wizard. Pending users can refill the form only to receive a final 409; active affiliates can also reopen the onboarding page.

Expected:

Pending affiliates should land on a pending-review/dashboard state, and active affiliates should be redirected to `/affiliate/dashboard`.

Evidence:

`page_affiliate_onboarding` delegates directly to `serve_protected` without checking affiliate status.

Recommended fix:

Check `affiliates.status` in the page handler and redirect `pending_approval`, `active`, and `suspended` states appropriately.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/affiliate-onboarding.js` | No syntax errors. | Passed with no output. | Pass |
| Backend build | `cd backend && cargo check` | Rust build succeeds. | Finished dev profile successfully. | Pass |
| Unauthenticated page access | `curl -i http://localhost:8888/affiliate/onboarding` after short `cargo run` | Redirect to login. | `303 See Other`, `location: /auth/login`. | Pass |
| Unauthenticated submit without CSRF | `curl -i -X POST /api/affiliate/onboarding/submit` | Request rejected. | `403` CSRF JSON error. | Pass |
| Authenticated full wizard | Not run. | Form navigation, KYC states, submit, DB write, policy rows, and success UI verified in browser. | Blocked by no safe authenticated affiliate fixture in this documentation-only run. | Not run |

---

## Security Findings

- P1: Compliance exam enforcement is client-side only; backend trusts `exam_passed`.
- P1: KYC gate is client-side only at submit time; direct API callers can create pending applications without approved KYC.
- P2: Duplicate guard masks DB errors with `unwrap_or(None)`.
- P2: Profile fields are not strongly validated server-side before storage and later admin review.
- Pass: Global CSRF middleware and head fetch interceptor are present; unauthenticated POST without CSRF returned `403`.
- Pass: Unauthenticated page access redirected to `/auth/login`.

---

## Database Findings

- `affiliates` exists with `traffic_source`, `audience_size`, `main_url`, `phone_number`, `tax_id`, and `company_name` columns.
- `affiliate_policy_acceptances` exists and receives one row per accepted policy inside the same transaction.
- `accepted_policy_version` exists, but onboarding submit does not update it when policies are accepted; it relies on the default/backfill value.
- The onboarding write transaction is good for affiliate profile plus policy rows, but KYC status and duplicate status reads happen outside that transaction.

---

## Missing Tests

- Add backend tests for `/api/affiliate/onboarding/submit` rejecting missing, wrong, and tampered `exam_answers`.
- Add backend tests for KYC-required submission behavior.
- Add backend tests for duplicate states: pending, active, suspended, terminated, and DB read failure.
- Add validation tests for invalid traffic source, audience size, overlong strings, and malformed URLs.
- Add authenticated Playwright coverage for the full wizard: required fields, KYC loading/error/approved states, legal tabs, failed quiz reset, successful submit, and pending success state.
- Update stale affiliate E2E scripts so they no longer assert success with wrong exam answers.

---

## Recommended Fix Order

1. Enforce quiz answer validation on the backend and update stale E2E fixtures.
2. Decide whether approved KYC is required for submission or only approval; align backend, copy, and admin review data.
3. Harden duplicate-state and profile-field validation in `submit_affiliate_onboarding_handler`.
4. Redirect pending/active/suspended affiliates away from the blank onboarding wizard.
5. Add visible KYC status loading/error states and accessible legal tabs.

---

## Final Status

`needs_recheck`

Reason: Serious backend trust-boundary issues were found in the compliance exam and KYC gate, plus validation and UX/a11y gaps that need fixes and authenticated re-verification.
