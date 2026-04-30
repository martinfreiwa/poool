# Page Audit: Affiliate Promo

Date: 2026-04-26
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/affiliate`
Template: `frontend/platform/affiliate-promo.html`
JavaScript: inline calculator/reveal script in `frontend/platform/affiliate-promo.html`
CSS: `frontend/platform/static/css/affiliate-promo.css`, `frontend/platform/static/css/cards-template.css`
Backend Routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, `backend/src/rewards/service.rs`

---

## 2026-04-28 Fix Update

All code-level findings from this audit were fixed in `frontend/platform/affiliate-promo.html`, static regression coverage was added in `tests/test_affiliate_promo_static.py`, and authenticated desktop/mobile browser coverage was added in `tests/e2e/test_affiliate_promo.py`.

Fixed:

- `PAGE-ISSUE-0318`: promo tier economics now match `backend/src/rewards/service.rs::AFFILIATE_TIERS`.
- `PAGE-ISSUE-0319`: the calculator now uses integer basis points and cents instead of hardcoded float commission rates.
- `PAGE-ISSUE-0320`: the `#who-we-want` qualification section now exists.
- `PAGE-ISSUE-0321`: calculator range inputs have programmatic labels/descriptions, and tier lock icons include accessible status text.
- `PAGE-ISSUE-0322`: application-review copy is aligned to `1-3 business days`.

Remaining issue:

- None. Authenticated desktop/mobile browser verification passed on 2026-04-28.

Verification:

- `python3 -m pytest tests/test_affiliate_promo_static.py -q` passed.
- `python3 -m pytest tests/e2e/test_affiliate_promo.py -q` passed.
- Inline script syntax parse with `node` passed.
- Targeted stale-copy scan passed for the production template.

---

## Summary

`/affiliate` is a protected affiliate-program promo page. The page route is registered and unauthenticated requests correctly redirect to `/auth/login`; the static CSS asset is served. The page is mostly static, but it has a serious contract mismatch: it advertises affiliate tier names, qualification rules, and a 4.50% maximum commission that do not match the backend tier progression logic, which is based on qualified-referral counts and maxes at 175 bps.

Final status is `completed` because the documented code findings are fixed and the authenticated desktop/mobile browser recheck passed.

---

## Tested Scope

- Reviewed `frontend/platform/affiliate-promo.html` hero CTAs, stats, tier table, calculator, FAQ, and final CTA.
- Reviewed inline JavaScript for the calculator and scroll-reveal behavior.
- Reviewed `backend/src/rewards/mod.rs` and `backend/src/rewards/routes.rs` for page route registration and auth protection.
- Reviewed `backend/src/rewards/service.rs` affiliate tier constants used by dashboard responses and tier progression.
- Reviewed affiliate schema migrations supporting affiliate records, commissions, policy acceptances, and materials.
- Ran unauthenticated HTTP smoke checks against a local backend on port `8898`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/affiliate` | Protected page route; unauthenticated request redirects to `/auth/login`. |
| Template | `frontend/platform/affiliate-promo.html` | Static promo page with inline calculator/reveal script. |
| CSS | `frontend/platform/static/css/affiliate-promo.css` | Page-specific promo styling; asset returned `200 OK` in runtime smoke. |
| Shared component | `frontend/platform/components/head.html` | Loads shared CSS, Sentry, CSRF/fetch interception, and requested page CSS. |
| Shared component | `frontend/platform/components/sidebar.html` | Marks `/affiliate` as active sidebar area. |
| Backend page route | `GET /affiliate` | Registered in `backend/src/rewards/mod.rs`; handled by `page_affiliate_promo`. |
| Backend handler | `page_affiliate_promo` | Calls `serve_protected(..., "affiliate-promo.html")`. |
| Backend tier source | `backend/src/rewards/service.rs` | `AFFILIATE_TIERS` uses qualified-referral thresholds and bps rates. |
| Database table | `affiliates` | Stores affiliate status, current tier, and commission rate bps. |
| Database table | `affiliate_referrals` | Stores referred-user state machine and qualification status. |
| Database table | `affiliate_commissions` | Stores provisional/payable commission records in cents. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Page shell | `body.app-body`, sidebar/mobile menu includes | Render authenticated app shell. | Static SSR include | `GET /affiliate` protected route | Unauthenticated request returned `303 /auth/login`; authenticated render not run. |
| Hero apply CTA | `#hero-apply-btn` | Navigate to `/affiliate/onboarding`. | Link | `GET /affiliate/onboarding` exists and is protected | Unauthenticated smoke returned `303 /auth/login`. |
| Qualification CTA | `#hero-view-tiers-btn[href="#who-we-want"]` | Scroll to qualification/audience section. | Link only | No backend needed | Broken by static review: no `id="who-we-want"` exists. |
| Stats bar | `.promo-stats-bar` | Present AUM, live property count, tier count, max commission. | Static text | No direct backend support | Values are hardcoded; commission/tier claims mismatch backend tier logic. |
| Process steps | `.promo-process-grid` | Explain application, review, earnings. | Static text | Onboarding/admin review routes exist | Copy is static; final CTA review SLA conflicts with FAQ. |
| Tier table | `#tier-table` | Explain current affiliate tier ladder. | Static HTML and calculator highlight | Backend tier source exists but is not used | Mismatch with backend tier names, thresholds, and rates. |
| Calculator investment slider | `#calc-investment` | Update estimate and tier highlight. | Inline `input` listener | No backend support | Static JS wired, but label is not programmatically associated. |
| Calculator referral slider | `#calc-referrals` | Update estimate and tier highlight. | Inline `input` listener | No backend support | Static JS wired, but label is not programmatically associated. |
| Calculator result | `#calc-monthly-earnings`, `#calc-tier-badge`, `#calc-tier-rate` | Show estimated commission. | Inline JS | No backend support | Uses hardcoded client-side tier constants that conflict with backend. |
| FAQ accordion | `.faq-item details > summary` | Native expand/collapse for FAQs. | Native `<details>` | No backend needed | Static wiring is valid; tier FAQ repeats backend-mismatched claims. |
| Final CTA | `#cta-apply-btn` | Navigate to `/affiliate/onboarding`. | Link | `GET /affiliate/onboarding` exists and is protected | Unauthenticated smoke returned `303 /auth/login`. |
| Reveal animation | `.reveal` | Reveal sections when in viewport, fallback to visible. | Inline `IntersectionObserver` | No backend needed | Static JS has fallback for unsupported observers. |

---

## Frontend Findings

### P1 - Promo tier economics do not match backend tier progression

Location:

- Template: `frontend/platform/affiliate-promo.html:67`, `frontend/platform/affiliate-promo.html:130`, `frontend/platform/affiliate-promo.html:263`, `frontend/platform/affiliate-promo.html:361`
- Backend: `backend/src/rewards/service.rs:1395`

Problem:

The page advertises 8 tiers named Access, Plus, Pro, Elite, Premium, Platinum, Signature, and Sovereign, qualified by trailing 12-month referral volume, with a max commission of 4.50%. The backend tier worker uses 8 different thresholds based on qualified referral count: Access, Bronze, Silver, Gold, Platinum, Diamond, Elite, Ambassador, maxing at 175 bps (1.75%).

Expected:

The promo page, calculator, FAQ, dashboard tier response, admin approval constraints, and actual commission engine should share one authoritative affiliate tier contract.

Evidence:

`AFFILIATE_TIERS` in `backend/src/rewards/service.rs` is `(qualified_referrals_required, tier_name, commission_rate_bps)` and ends at `("Ambassador", 175)`, while the page shows volume-based `Sovereign` at `4.50%`.

Recommended fix:

Move affiliate tier metadata into one backend/source-of-truth contract and render this page from it, or update the page copy/calculator to exactly match backend tier progression. Do not leave public-facing commission promises hardcoded in the template.

2026-04-28 fix:

The page tier table, stats, FAQ, and calculator now match the backend `AFFILIATE_TIERS` names, thresholds, and bps rates. `tests/test_affiliate_promo_static.py` compares the template and calculator constants against the backend constant to catch future drift.

### P2 - Calculator performs client-side commission logic with hardcoded float rates

Location:

- Template/inline JS: `frontend/platform/affiliate-promo.html:263`

Problem:

The calculator uses hardcoded JavaScript floats (`0.0050`, `0.0450`) and client-side tier rules to calculate estimated affiliate earnings. This is not a mutating financial path, but it is still a user-facing money estimate and can drift from the backend, as it already has.

Expected:

Use integer basis points and backend-provided tier metadata for any money-related estimate, with clear “estimate only” language if the value is not binding.

Evidence:

The inline `tiers` array is independent from `backend/src/rewards/service.rs::AFFILIATE_TIERS`.

Recommended fix:

Expose read-only affiliate tier metadata from the backend or SSR context, calculate with bps/integer cents, and render a visible caveat that actual payable commissions depend on qualification, holdback, tax, fraud, and admin approval checks.

2026-04-28 fix:

The calculator now mirrors backend tier names and rates using integer basis points, computes estimated earnings in cents, and displays a caveat about KYC, eligible investment, holdback, tax readiness, fraud checks, and admin review.

### P2 - Qualification CTA points to a missing section

Location:

- Template: `frontend/platform/affiliate-promo.html:32`

Problem:

The “Are You Qualified?” CTA links to `#who-we-want`, but no element on the page has that id. Clicking the button does not take the user to qualification criteria.

Expected:

The link should target an existing section, likely `#tiers`, `#how-it-works`, or a newly added qualification section.

Evidence:

Static review found `href="#who-we-want"` and no matching `id="who-we-want"` in `frontend/platform/affiliate-promo.html`.

Recommended fix:

Add the intended qualification section with `id="who-we-want"` or change the link target to an existing section.

2026-04-28 fix:

Added a `#who-we-want` qualification section describing relevant audience, compliant distribution, and quality referrals.

### P2 - Calculator range inputs lack programmatic labels

Location:

- Template: `frontend/platform/affiliate-promo.html:210`
- Template: `frontend/platform/affiliate-promo.html:219`

Problem:

The slider labels are sibling `<label>` elements without `for` attributes and the inputs are not wrapped by the labels. Screen readers may announce the range controls without useful names.

Expected:

Each label should have `for="calc-investment"` or `for="calc-referrals"`, or the inputs should have equivalent `aria-label` attributes.

Evidence:

The range inputs use ids, but the visible labels do not reference those ids.

Recommended fix:

Add `for` attributes to both slider labels and ensure the displayed values remain associated via `aria-describedby`.

2026-04-28 fix:

Added `for` attributes to both calculator labels and `aria-describedby` on both range inputs.

### P2 - Locked tier status icons have no accessible text

Location:

- Template: `frontend/platform/affiliate-promo.html:146`

Problem:

Most tier status cells contain only decorative lock SVGs. Assistive technology users do not get a meaningful status such as “Locked” or “Not yet qualified”.

Expected:

Each icon-only status cell should include visually hidden text or an `aria-label`, and decorative SVGs should be `aria-hidden="true"`.

Evidence:

The status cells for Plus through Signature contain only inline SVG markup.

Recommended fix:

Add hidden status text to each lock cell and mark the SVG as decorative.

2026-04-28 fix:

Added hidden status text to locked tier cells and marked lock SVGs as decorative with `aria-hidden="true"` and `focusable="false"`.

### P3 - Review-time copy is inconsistent

Location:

- Template: `frontend/platform/affiliate-promo.html:378`
- Template: `frontend/platform/affiliate-promo.html:394`

Problem:

The FAQ says applications are reviewed within 1-3 business days, while the final CTA says applications are reviewed within 48 hours.

Expected:

Use one SLA phrasing across the page.

Evidence:

Static copy review found both values in the same template.

Recommended fix:

Choose the product-approved review window and update both locations to match.

2026-04-28 fix:

Aligned final CTA copy with the FAQ and process copy: applications are typically reviewed within `1-3 business days`.

---

## Backend Findings

### P1 - Backend tier source conflicts with page claims

Location:

- Backend: `backend/src/rewards/service.rs:1395`
- Template: `frontend/platform/affiliate-promo.html:140`

Problem:

The backend tier progression worker uses qualified-referral counts and commission bps that do not match the page’s trailing-volume ladder. This can mislead applicants and creates a compliance/support risk if affiliates expect the advertised rates.

Expected:

Backend tier calculation, admin approval constraints, dashboard responses, and marketing/promo display should use the same source of truth.

Evidence:

`get_affiliate_tier_thresholds()` returns the backend `AFFILIATE_TIERS` values to the dashboard, while `/affiliate` renders unrelated static values.

Recommended fix:

Canonicalize the affiliate tier model before further affiliate promo/dashboard work.

2026-04-28 fix:

The promo page now matches the current backend tier model. Static regression coverage protects the template/calculator/backend constant contract, and authenticated desktop/mobile browser coverage passed.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Route registration | Inspected `backend/src/rewards/mod.rs` and `routes.rs`. | `GET /affiliate` exists and renders `affiliate-promo.html`. | Route exists and calls `serve_protected`. | Pass |
| Unauthenticated page access | `curl -I http://localhost:8898/affiliate` | Redirect to login. | `303 See Other`, `location: /auth/login`. | Pass |
| Onboarding link target auth | `curl -I http://localhost:8898/affiliate/onboarding` | Protected onboarding route exists. | `303 See Other`, `location: /auth/login`. | Pass |
| CSS asset | `curl -I http://localhost:8898/static/css/affiliate-promo.css` | CSS returns `200`. | `200 OK`, `content-type: text/css`. | Pass |
| Shared JS syntax | `node --check` on shared JS loaded by page shell. | No syntax errors. | Passed for `csrf.js`, `user-data.js`, `poool-dropdown.js`, and `poool-dropdown-init.js`. | Pass |
| Inline calculator syntax | Extracted inline `<script>` from `affiliate-promo.html` and parsed with `node`. | Script parses. | `script 1 ok`. | Pass |
| Affiliate tier contract regression | `python3 -m pytest tests/test_affiliate_promo_static.py -q` | Template/calculator match backend tier contract. | `2 passed`. | Pass |
| Authenticated browser render | `python3 -m pytest tests/e2e/test_affiliate_promo.py -q` | Page renders without console errors and CTA/slider/FAQ work. | Desktop and mobile checks passed with authenticated sessions. | Pass |

---

## Security Findings

- No direct state-changing action exists on `/affiliate`; primary actions navigate to protected onboarding.
- Unauthenticated access to `/affiliate` and `/affiliate/onboarding` redirects to login with security headers.
- The main security/compliance risk is economic misrepresentation: affiliate commission claims are not backed by the backend tier source of truth.
- The page uses no user-provided data in its own inline script, so no page-local XSS issue was identified.

---

## Database Findings

- `affiliates.current_tier` and `affiliates.commission_rate_bps` exist and support backend-managed affiliate rates.
- `affiliate_referrals` and `affiliate_commissions` exist for referral qualification and commission tracking.
- The promo page does not read these tables or a backend tier contract; all tier/rate display data is static.

---

## Missing Tests

- Fixed: static contract test compares affiliate tier names/rates/threshold semantics rendered on `/affiliate` with the backend tier source of truth.
- Fixed: authenticated browser test covers `/affiliate` page render, hero CTA, qualification CTA scroll target, sliders, FAQ toggles, and mobile viewport.
- Fixed: accessibility checks cover the calculator range labels and tier status cells.

---

## Recommended Fix Order

1. Fixed: canonicalized `/affiliate` copy/calculator against the backend tier contract.
2. Fixed: added the missing `#who-we-want` qualification section.
3. Fixed: added accessible labels/descriptions for calculator sliders and tier status icons.
4. Fixed: aligned review-time copy across the FAQ and CTA.
5. Fixed: authenticated browser/mobile recheck passed and results are documented.

---

## Final Status

`completed`

Reason: The page route is protected and available, affiliate economics now match backend tier progression, static contract coverage passed, and authenticated desktop/mobile browser verification passed.

2026-04-28 reason: Code-level findings are fixed and covered by static regression tests. Final status is `completed` after authenticated browser/mobile verification passed.
