# Page Audit: Affiliate Onboarding

Date: 2026-04-27
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/onboarding`
Template: `frontend/platform/affiliate-onboarding.html`
JavaScript: `frontend/platform/static/js/affiliate-onboarding.js`
CSS: `frontend/platform/static/css/affiliate-onboarding.css`, `frontend/platform/static/css/affiliate-promo.css`, `frontend/platform/static/css/cards-template.css`, `frontend/platform/static/css/forms-template.css`
Backend routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, `backend/src/kyc/mod.rs`, `backend/src/kyc/routes.rs`

---

## Summary

`/affiliate/onboarding` is a real authenticated five-step wizard and submits to a transactional backend endpoint, but it remains not production-ready. The page has a credible UI path, CSRF protection is active, and unauthenticated page access redirects to login. The main blockers are trust-boundary and workflow gaps: the backend still trusts `exam_passed` instead of validating `exam_answers`, KYC approval is only enforced in JavaScript, duplicate-state DB read errors are masked, and field validation/page-state handling are incomplete.

Final status is `needs_recheck` because fixes are required before this page can be considered completed.

---

## Scope Reviewed

- `frontend/platform/affiliate-onboarding.html`: wizard steps, forms, links, tab controls, policy acknowledgements, quiz radios, and success state.
- `frontend/platform/static/js/affiliate-onboarding.js`: step navigation, validation, KYC fetch, legal tab switching, quiz scoring, submit payload, and error handling.
- `backend/src/rewards/mod.rs`: page and submit route registration.
- `backend/src/rewards/routes.rs`: protected page handler and `submit_affiliate_onboarding_handler`.
- `backend/src/rewards/models.rs`: `SubmitOnboardingForm`, including optional `exam_answers`.
- `backend/src/kyc/mod.rs`, `backend/src/kyc/routes.rs`, `backend/src/kyc/service.rs`: `/api/kyc/status` route and data source.
- Affiliate schema migrations: `database/072_affiliate_core_system.sql`, `database/073_affiliate_profile_data.sql`, `database/076_affiliate_system_gaps.sql`.
- Existing affiliate E2E tests: `tests/test_e2e_affiliate.py`, `tests/test_e2e_affiliate_full_funnel.py`.

---

## Route and File Map

| Type | Path / Route | Status |
|------|--------------|--------|
| Page URL | `/affiliate/onboarding` | Protected; unauthenticated smoke returned `303 /auth/login`. |
| Template | `frontend/platform/affiliate-onboarding.html` | Exists; renders five-step wizard. |
| JS | `frontend/platform/static/js/affiliate-onboarding.js` | Exists; `node --check` passed. |
| CSS | `frontend/platform/static/css/affiliate-onboarding.css` | Exists via page head include. |
| Page route | `GET /affiliate/onboarding` | Registered in `backend/src/rewards/mod.rs`; served by `page_affiliate_onboarding`. |
| Submit API | `POST /api/affiliate/onboarding/submit` | Registered; CSRF-protected; writes affiliate application and policy rows. |
| KYC API | `GET /api/kyc/status` | Used by step 2; submit API does not enforce its result. |
| DB table | `affiliates` | Stores profile, tax, status, referral code, policy version. |
| DB table | `affiliate_policy_acceptances` | Stores policy acceptance rows inside the submit transaction. |
| DB table | `kyc_records` | Read by KYC status API; not checked during submit. |

---

## UI Element Inventory

| Element | Selector / Location | Expected behavior | Frontend | Backend | Audit result |
|--------|---------------------|-------------------|----------|---------|--------------|
| Wizard stepper | `#wizard-stepper`, `.step-item` | Show active/completed steps and progress. | Wired by `showStep()` and `updateProgressBar()`. | None needed. | Works by static review. |
| Traffic source select | `#traffic-source` | Required profile field. | Validated for non-empty; converted to POOOL dropdown if available. | Stored in `affiliates.traffic_source`. | Partial: backend accepts arbitrary values. |
| Audience size select | `#audience-size` | Required profile field. | Validated for non-empty. | Stored in `affiliates.audience_size`. | Partial: backend accepts arbitrary values. |
| Main URL input | `#main-url` | Required URL/social profile. | Only checked for non-empty before advancing. | Requires `http://` or `https://`. | Partial: weak backend URL validation. |
| Phone input | `#phone-number` | Required phone. | Marked `required`, but step 1 JS does not call full form validity. | Rejects empty value at final submit. | Broken UX: can advance until final API rejection. |
| Step 1 continue | inline `onclick="nextStep(2)"` | Validate profile and advance. | Partially wired. | No write until final submit. | Partial. |
| KYC status panel | `#kyc-verified`, `#kyc-pending` | Show verified or required KYC state. | Fetches `/api/kyc/status`; failures silent. | KYC API exists. | Partial: no visible loading/error/retry state. |
| Start KYC link | `a[href="/kyc"]` | Navigate to KYC page. | Link only. | `/kyc` exists through KYC module. | Wired. |
| Step 2 continue | inline `onclick="nextStep(3)"` | Advance only after verified KYC. | Client-only gate. | Submit API does not enforce KYC. | Security/workflow gap. |
| Tax ID input | `#tax-id` | Required tax identifier. | Uses native `checkValidity()`. | Stored in `affiliates.tax_id`. | Wired, but no strong length/format validation. |
| Company input | `#company-name` | Optional company name. | Included in payload. | Stored in `affiliates.company_name`. | Wired, but no length validation before DB. |
| Legal tabs | `.legal-tab[data-target]` | Switch policy document panels. | Wired by `switchLegalTab()`. | None needed. | Works, but missing ARIA tab semantics and arrow-key handling. |
| Policy checkboxes | `#cb-terms`, `#cb-conduct`, `#cb-materials`, `#cb-payout`, `#cb-privacy` | Require all five acknowledgements. | Checked before step 5. | Server validates exact required policy names and count. | Wired. |
| Quiz radios | `input[name="q1"]` through `q5` | Require all questions and 100% pass score. | Client validates exact answers. | Backend ignores `exam_answers`. | High-risk gap. |
| Submit button | `#submit-exam-btn` | Submit application, disable during request, show pending success state. | Wired to real `fetch()`. | Real transactional POST route exists. | Partial: backend accepts forged pass flag. |
| Success dashboard link | `a[href="/rewards"]` | Navigate after submission. | Link only. | `/rewards` exists. | Wired, but label says dashboard while target is rewards. |

---

## Findings

### High - Backend still accepts passed quiz flag without validating answers

Location: `backend/src/rewards/routes.rs:378`, `backend/src/rewards/models.rs:91`, `frontend/platform/static/js/affiliate-onboarding.js:44`

The frontend sends `exam_answers`, but the server only checks `form.exam_passed`. The handler still contains a comment that answer validation was removed. Any authenticated caller with a valid CSRF token can submit `exam_passed: true` with missing or incorrect answers and bypass the compliance exam. Existing E2E fixtures also send answer values that do not match the current client answer key.

Expected: backend-owned quiz validation should ignore the client trust flag or derive it from canonical server-side answers, rejecting missing, malformed, or incorrect `exam_answers`.

### High - KYC approval gate is client-side only

Location: `frontend/platform/static/js/affiliate-onboarding.js:15`, `frontend/platform/static/js/affiliate-onboarding.js:202`, `backend/src/rewards/routes.rs:348`

Step 2 blocks users by checking whether `#kyc-verified` is visible, but `POST /api/affiliate/onboarding/submit` never queries `kyc_records`. A direct API caller can create a pending affiliate application without approved KYC, despite the page copy saying active KYC is required.

Expected: either enforce approved KYC server-side at submission, or change product copy/admin workflow so KYC is explicitly required for approval rather than submission.

### Medium - Duplicate application read errors are treated as no existing application

Location: `backend/src/rewards/routes.rs:456`

The duplicate-state guard uses `.fetch_optional(...).await.unwrap_or(None)`. A database read failure becomes `None`, allowing the handler to continue into the write transaction instead of returning a safe 500. This can mask operational failures and make state transitions harder to reason about.

Expected: explicitly log and return a safe error on duplicate-status lookup failure.

### Medium - Backend accepts arbitrary profile enum values and weak profile data

Location: `backend/src/rewards/routes.rs:412`, `database/073_affiliate_profile_data.sql:4`

The UI offers constrained values for traffic source and audience size, but the backend only checks non-empty strings. The URL check is prefix-based, and key profile fields have no explicit length or format validation before storage.

Expected: enforce allowlists matching the template options, parse URLs with a URL parser, reject overlong strings before DB errors, and validate phone/tax fields to the degree required for admin review.

### Medium - Already pending or active affiliates can still load the onboarding wizard

Location: `backend/src/rewards/routes.rs:117`

The page route only requires authentication and always serves the wizard. Pending affiliates can fill the full wizard before receiving a final 409, and active affiliates can reopen onboarding instead of being redirected to the affiliate dashboard.

Expected: route pending/suspended/active states to the correct status or dashboard page before rendering the wizard.

### Medium - KYC status failures are silent

Location: `frontend/platform/static/js/affiliate-onboarding.js:15`

`fetchKycStatus()` returns on non-OK responses and catches network errors without showing a visible error or retry state. Users see "Identity Verification Required" even when the platform failed to load their actual KYC status.

Expected: show loading, verified, pending, rejected, and retryable error states.

### Low - Step 1 allows missing phone until final submit

Location: `frontend/platform/affiliate-onboarding.html:110`, `frontend/platform/static/js/affiliate-onboarding.js:163`

The phone input is marked `required`, but `validateStep(1)` only checks dropdowns and `main-url`. Users can advance and only receive the phone error at final submit.

Expected: call `profile-form.checkValidity()` or explicitly validate all required fields before step 2.

### Low - Legal tabs lack full tab accessibility semantics

Location: `frontend/platform/affiliate-onboarding.html:197`, `frontend/platform/static/js/affiliate-onboarding.js:285`

The legal document switcher uses plain buttons and inline handlers without `role="tablist"`, `role="tab"`, `aria-selected`, `aria-controls`, or arrow-key navigation.

Expected: implement accessible tabs or style them as normal buttons with explicit pressed state and documented keyboard behavior.

---

## Security, Data, and UX Assessment

- Authentication: protected page route works for unauthenticated users, returning `303 /auth/login`.
- CSRF: unauthenticated POST without CSRF returned `403`; global CSRF protection is active.
- Authorization: submit endpoint requires an authenticated session, but KYC and quiz authorization-like gates are not enforced server-side.
- Data integrity: affiliate profile and policy acceptance writes are wrapped in one transaction; the duplicate status and KYC state checks are outside that transaction path.
- Money rules: no direct monetary mutation on this page.
- XSS: this page mostly uses static text and form controls; no user-rendered HTML path was found in the onboarding wizard itself.
- Accessibility: form labels mostly exist, but legal tabs and KYC states need keyboard/status improvements.
- UX: users can hit late backend rejections for phone, existing affiliate state, and silent KYC API errors.

---

## Commands Run

| Command | Result |
|---------|--------|
| `node --check frontend/platform/static/js/affiliate-onboarding.js` | Passed. |
| `cd backend && cargo check` | Passed. |
| `curl -i http://localhost:8888/affiliate/onboarding` | Blocked: no server listening on `:8888`. |
| `curl -i -X POST http://localhost:8888/api/affiliate/onboarding/submit ...` | Blocked: no server listening on `:8888`. |
| `SERVER_PORT=8898 PORT=8898 cargo run` | Started temporary local backend; existing idempotency issues in local migration runner logged but server came up. |
| `curl -i http://localhost:8898/affiliate/onboarding` | Passed expected protection: `303 See Other`, `location: /auth/login`. |
| `curl -i -X POST http://localhost:8898/api/affiliate/onboarding/submit ...` | Passed expected CSRF protection: `403 Forbidden` with CSRF JSON error. |

Authenticated full-browser wizard testing was not run because this documentation-only audit did not create a safe authenticated affiliate/KYC fixture.

---

## Recommended Fix Order

1. Add backend quiz-answer validation and update stale affiliate E2E fixtures.
2. Decide and enforce the KYC rule server-side, or rewrite copy/admin workflow if KYC is only required before approval.
3. Replace duplicate-status `unwrap_or(None)` with explicit DB error handling.
4. Validate profile enum values, URL, length bounds, phone, and tax fields server-side.
5. Redirect pending/active/suspended affiliates away from the blank onboarding wizard.
6. Add visible KYC loading/error states and legal-tab keyboard/ARIA behavior.
7. Add authenticated Playwright or HTTP+DB E2E coverage for wrong quiz answers, missing KYC, duplicate states, and successful submit.

---

## Final Status

`needs_recheck`

Severity counts: High 2, Medium 4, Low 2.
