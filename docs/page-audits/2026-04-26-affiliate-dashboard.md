# Page Audit: Affiliate Dashboard

Date: 2026-04-26
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/dashboard`
Template: `frontend/platform/affiliate-dashboard.html`
JavaScript: `frontend/platform/static/js/affiliate-dashboard.js`
CSS: `frontend/platform/static/css/affiliate-dashboard.css`, `frontend/platform/static/css/leaderboard.css`, `frontend/platform/static/css/cards-template.css`, `frontend/platform/static/css/forms-template.css`
Backend Routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, `backend/src/rewards/service.rs`

---

## Summary

The authenticated affiliate dashboard is route-registered and has real backend APIs for dashboard metrics, payout requests, SubID stats, and postback URL saving. It is not production-ready: active affiliates can hit a render-stopping JavaScript selector error, SubID reporting renders user-controlled data through `innerHTML`, postback URLs can become SSRF targets and leak query data in logs, payout requests can falsely report success, and click counts are mismatched between backend and frontend.

---

## Tested Scope

- Static review of `frontend/platform/affiliate-dashboard.html`
- Static review of `frontend/platform/static/js/affiliate-dashboard.js`
- Backend route/API review in `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, and `backend/src/rewards/service.rs`
- Schema dependency review from affiliate migrations and `docs/DATABASE_SCHEMA.md`
- Shared standards review from `AGENTS.md`, `docs/AGENT_DEVELOPMENT_PROMPT.md`, `docs/DESIGN.md`, `docs/IMPLEMENTATION_ROADMAP.md`, `docs/issue-tracking/BROKEN_LOGICS.md`, `docs/design/FRONTEND_COMPONENTS.md`, `docs/TECH_STACK.md`, `docs/SECURITY.md`, and `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`
- JavaScript syntax check with `node --check`

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/affiliate/dashboard` | Authenticated affiliate dashboard page |
| Template | `frontend/platform/affiliate-dashboard.html` | KPI cards, referral link widget, postback form, funnel, tier progress, commissions, SubID table |
| JS | `frontend/platform/static/js/affiliate-dashboard.js` | Fetches dashboard data, payout request, link/QR generation, postback save, SubID stats |
| CSS | `frontend/platform/static/css/affiliate-dashboard.css` | Shared affiliate dashboard styling |
| Backend page route | `GET /affiliate/dashboard` | Registered in `backend/src/rewards/mod.rs`; served via `page_affiliate_dashboard` |
| Backend API route | `GET /api/affiliate/dashboard` | Active/pending affiliate dashboard metrics |
| Backend API route | `POST /api/affiliate/payout/request` | Manual payout notification flow |
| Backend API route | `GET /api/affiliate/subid-stats` | SubID stats table data |
| Backend API route | `POST /api/affiliate/postback` | Saves S2S postback URL |
| Database table | `affiliates` | Status, referral code, tier, commission rate, postback URL |
| Database table | `affiliate_referrals` | Referral funnel and SubID registration stats |
| Database table | `affiliate_commissions` | Earnings, payout eligibility, recent commissions |
| Database table | `referral_clicks` | Click count and SubID click stats |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Page shell | sidebar, mobile menu, investor topbar | Load authenticated dashboard shell | Yes | `GET /affiliate/dashboard` protected route | Static route wiring verified; authenticated browser not run |
| Total earnings KPI | `#kpi-total-earnings` | Show total affiliate earnings | Yes | `GET /api/affiliate/dashboard` | Blocked by missing tier selector error |
| Paid earnings text | `#kpi-paid-earnings` | Show paid-out amount | Yes | Dashboard API | Blocked by missing tier selector error |
| Payable KPI | `#kpi-payable-earnings` | Show payable balance | Yes | Dashboard API | Blocked by missing tier selector error |
| Request payout | `#request-payout-btn` | POST payout request when payable >= $50 | Yes | `POST /api/affiliate/payout/request` | Backend can falsely succeed without durable request |
| Provisional KPI | `#kpi-provisional-earnings` | Show provisional balance | Yes | Dashboard API | Blocked by missing tier selector error |
| SubID input | `#link-gen-subid` | Add `subid` query param to referral URL | Yes | Referral click route stores SubID | Works statically; downstream XSS risk in report table |
| UTM input | `#link-gen-utm` | Add `utm_source` query param | Yes | Browser-only link generation | Static wiring verified |
| Referral URL | `#referral-url-input` | Display generated referral URL | Yes | Dashboard API returns `referral_url` | Static wiring verified |
| Copy link | `#copy-ref-btn` | Copy URL and show feedback | Yes | Browser clipboard only | No fallback for denied clipboard permission |
| Postback input | `#postback-url-input` | Accept S2S postback URL | Yes | `POST /api/affiliate/postback` | Backend lacks URL safety validation |
| Save postback | `#save-postback-btn` | Persist postback URL | Yes | `POST /api/affiliate/postback` | Security gap |
| QR code | `#qrcode` | Render QR for referral URL | Yes | CDN `qrcode-generator` dependency | Unverified runtime; no local fallback |
| Funnel clicks | `#f-clicks` | Show click count | Yes | Backend returns `clicks` | Broken contract: JS reads `referral_clicks` |
| Funnel signups | `#f-registered` | Show registered/holdback/qualified sum | Yes | Dashboard API referrals object | Blocked by missing tier selector error |
| Tier progress | `#tier-progress-fill`, `#tier-markers` | Render tier ladder | Yes | Dashboard API tier thresholds | Blocked by missing `#tier-name` and `#tier-rate` writes |
| Recent commissions | `#recent-commissions-list` | Render recent commission list | Yes | Dashboard API recent commissions | Uses `innerHTML`; current fields are backend-controlled but should be hardened |
| SubID stats table | `#subid-stats-body` | Render grouped SubID stats | Yes | `GET /api/affiliate/subid-stats` | XSS risk from raw `s.sub_id` interpolation |
| Pending review state | generated `.affiliate-review-state` | Show pending approval status | Yes | Dashboard API pending status | Static wiring verified |
| Policy reacceptance banner | `#policy-reaccept-banner` | Send users to affiliate settings to reaccept policies | Yes | Dashboard API policy version fields | Uses generated `innerHTML`; version should remain backend-controlled |

---

## Frontend Findings

### P1 - Missing tier selectors stop active dashboard rendering

Location:

- Template: `frontend/platform/affiliate-dashboard.html`
- JS: `frontend/platform/static/js/affiliate-dashboard.js:41`

Problem:

The JavaScript writes to `#tier-name` and `#tier-rate`, but the template does not define those elements. For active affiliates, `renderDashboard()` throws before KPIs, payout enablement, referral URL, recent commissions, QR code, and tier progress render.

Expected:

The active dashboard should tolerate missing optional tier labels or the template should include those IDs.

Evidence:

Static selector review found no `tier-name` or `tier-rate` IDs in the template, while `document.getElementById(...).textContent` is called without a null check.

Recommended fix:

Restore the tier elements or null-check/remove those writes. Add a selector contract test for required dashboard IDs.

### P2 - Dashboard click count contract is mismatched

Location:

- JS: `frontend/platform/static/js/affiliate-dashboard.js:90`
- Backend: `backend/src/rewards/service.rs:1065`

Problem:

The backend returns `clicks`, but the frontend reads `dashboardData.referral_clicks`, so the funnel displays zero clicks even when data exists.

Expected:

Frontend and backend should use one response field, with backward-compatible support during rollout if needed.

Evidence:

Static review shows `clicks` in the JSON response and `referral_clicks` in the render logic.

Recommended fix:

Read `dashboardData.clicks ?? dashboardData.referral_clicks ?? 0`, then standardize API docs/tests on one field.

---

## Backend Findings

### P1 - Affiliate postback URL creates SSRF and sensitive log risks

Location:

- Backend save: `backend/src/rewards/routes.rs:1111`
- Backend execution/logging: `backend/src/rewards/service.rs:903`, `backend/src/rewards/service.rs:1276`

Problem:

`POST /api/affiliate/postback` stores any trimmed string, and background postback execution later sends server-side GET requests to that URL. The execution paths log full URLs, including query parameters such as SubID and payout data.

Expected:

Only safe outbound postback URLs should be accepted, and logs should redact sensitive query strings.

Evidence:

No scheme, host, private-network, link-local, metadata-IP, DNS-resolution, or length validation is performed before persistence or execution.

Recommended fix:

Require `https://`, apply length limits, block localhost/private/link-local/cloud metadata targets after DNS resolution, URL-encode macro values, consider an allowlist, and redact query strings in logs.

### P2 - Payout request masks DB/email failures and has no durable request record

Location:

- Backend: `backend/src/rewards/routes.rs:668`

Problem:

The payout request endpoint swallows payable query failures with `unwrap_or(Some(0))`, swallows user-email lookup failures, ignores email-send failures, returns success without an audit log or durable payout request row, and formats money with `f64`.

Expected:

A payout request action should either create a durable, auditable request or clearly fail. Money display should avoid floats.

Evidence:

Static review of `api_affiliate_payout_request()` shows ignored errors and no write to an audit/request table.

Recommended fix:

Propagate DB errors, format cents with integer/decimal-safe helpers, record a payout request and audit event transactionally, make duplicate requests idempotent, and report notification delivery status honestly.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax check | `node --check frontend/platform/static/js/affiliate-dashboard.js` | JavaScript parses | Passed | Pass |
| Selector contract static check | Search for `tier-name`/`tier-rate` in template and JS | Referenced selectors exist or are guarded | JS references missing template IDs | Fail |
| API field contract static check | Compare dashboard API JSON keys and frontend reads | Field names align | Backend returns `clicks`; JS reads `referral_clicks` | Fail |
| Runtime authenticated dashboard | Load page with active affiliate session | Page renders all dashboard widgets | Not run in this documentation-only audit | Not run |
| Mutating payout/postback runtime tests | Submit safe fixture requests | Durable/audited success or visible failure | Not run to avoid mutating application data | Not run |

---

## Security Findings

### P1 - SubID report renders user-controlled SubIDs as raw HTML

Location:

- JS: `frontend/platform/static/js/affiliate-dashboard.js:369`
- Backend data source: `referral_clicks.subid`

Problem:

The referral flow accepts SubID query data and stores it for reporting. The dashboard later interpolates `s.sub_id` into `tbody.innerHTML`, creating an avoidable affiliate-dashboard XSS path.

Expected:

User-controlled SubIDs should be rendered with DOM APIs and `textContent`, or escaped before interpolation.

Evidence:

`loadSubIDStats()` maps `data.stats` into a template string and writes it directly to `tbody.innerHTML`.

Recommended fix:

Build table rows with `document.createElement()` and `textContent`, validate/report SubID length and characters server-side, and add a malicious SubID regression test.

### P1 - Postback URL SSRF and log disclosure

Covered under Backend Findings. This is also a security finding because it allows active affiliates to make backend workers call arbitrary URLs and leak postback query data into logs.

---

## Database Findings

- Required affiliate tables appear to exist through migrations `072_affiliate_core_system.sql`, `073_affiliate_profile_data.sql`, `074_affiliate_indexes.sql`, `075_affiliate_postback_url.sql`, and `076_affiliate_system_gaps.sql`.
- `affiliate_commissions.provisional_amount_cents` and related earning values use integer cents.
- No durable payout request table or audited request row was identified for `POST /api/affiliate/payout/request`.
- `affiliates.postback_url` exists, but database constraints do not enforce URL safety, length, or outbound allowlist rules.

---

## Missing Tests

- Active affiliate dashboard browser/E2E test that verifies KPIs, referral URL, QR rendering, tier progress, recent commissions, and SubID table render without console errors.
- Static selector contract test for `affiliate-dashboard.js` against `affiliate-dashboard.html`.
- Dashboard API contract test covering `clicks` versus `referral_clicks`.
- Security regression test for malicious SubID values rendered in the SubID stats table.
- Backend validation tests for `POST /api/affiliate/postback` rejecting unsafe schemes, localhost/private/link-local/metadata targets, overlong URLs, and malformed URLs.
- Payout request tests for DB failure propagation, idempotency, audit logging, minimum payable threshold, and no float-based money formatting.

---

## Recommended Fix Order

1. Fix the missing `#tier-name` / `#tier-rate` selector contract so active affiliates can render the dashboard.
2. Replace SubID table `innerHTML` rendering with safe DOM rendering and add malicious SubID tests.
3. Harden postback URL validation/execution and redact postback URLs in logs.
4. Make payout requests durable, auditable, idempotent, and honest about email/DB failures.
5. Align dashboard click count API/frontend fields and add an authenticated dashboard E2E fixture.

---

## Final Status

`needs_recheck`

Reason: The page has real route and API wiring, but active dashboard rendering, SubID rendering, postback safety, payout request durability, and click-count contract issues need fixes and authenticated recheck.
