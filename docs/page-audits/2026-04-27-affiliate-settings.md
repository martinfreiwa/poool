# Page Audit: Affiliate Settings

Date: 2026-04-27
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/settings`
Template: `frontend/platform/affiliate-settings.html`
JavaScript: `frontend/platform/static/js/affiliate-settings.js`
CSS: `frontend/platform/static/css/affiliate-dashboard.css`, `frontend/platform/static/css/forms-template.css`, `frontend/platform/static/css/leaderboard.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`, `backend/src/rewards/service.rs`, `backend/src/rewards/models.rs`

---

## Summary

The Affiliate Settings page now has a dedicated settings API, CSRF-protected save flow, real status loading, a working topbar Save Changes button, single script load, responsive settings layout, masked Tax ID responses/audit logs, and encrypted-at-rest Tax ID writes for both onboarding and settings updates.

Final status remains `needs_recheck` because authenticated browser verification was not completed in this pass. No documented implementation issue remains for this page after the 2026-04-28 Tax ID encryption fix.


---

## Fix Pass: 2026-04-28

Implemented fixes:

- Added `GET/POST /api/affiliate/settings` as a dedicated affiliate settings contract.
- Updated settings save to run in a database transaction, preserve existing encrypted Tax ID when the field is left blank, set `is_tax_ready = false` after tax classification/name/ID changes, upsert payout method/name/VAT fields, and write an `AFFILIATE_SETTINGS_UPDATED` audit row without raw tax data.
- Added `database/092_affiliate_tax_id_encryption.sql` and encrypted Tax ID writes via `TAX_ID_ENCRYPTION_KEY`, storing ciphertext in `affiliates.tax_id_encrypted`, display suffix in `affiliates.tax_id_last4`, and clearing legacy `affiliates.tax_id` on onboarding/settings writes.
- Blocked settings saves from silently preserving legacy plaintext `affiliates.tax_id`; affected users must re-enter Tax ID once so the new encrypted storage can replace the legacy value.
- Rewired `affiliate-settings.js` to load current settings, show real tax/payout status badges, send `X-CSRF-Token`, submit to `/api/affiliate/settings`, and show inline loading/success/error messages.
- Associated the topbar Save Changes button with `#affiliate-settings-form` using `type="submit"` and `form="affiliate-settings-form"`.
- Removed the duplicate page-level script include so `affiliate-settings.js` loads once via `extra_js`.
- Moved the page layout into CSS classes and added mobile fallbacks for the main grid and split form row.
- Added static regression tests in `tests/test_affiliate_settings_static.py`.

Remaining implementation issues: none documented. Remaining verification: authenticated browser save/mobile recheck.

---

## Tested Scope

- Reviewed `frontend/platform/affiliate-settings.html`.
- Reviewed `frontend/platform/components/investor-topbar.html` for the page-level Save Changes control.
- Reviewed `frontend/platform/static/js/affiliate-settings.js`.
- Reviewed route registration in `backend/src/rewards/mod.rs`.
- Reviewed page handler and API handlers in `backend/src/rewards/routes.rs`.
- Reviewed payout settings model/service code in `backend/src/rewards/models.rs` and `backend/src/rewards/service.rs`.
- Reviewed affiliate and payout schema in `database/072_affiliate_core_system.sql`, `database/073_affiliate_profile_data.sql`, and `database/025_commissions.sql`.
- Checked local DB columns and constraints for `affiliates` and `payout_settings`.
- Ran unauthenticated runtime smoke for `GET /affiliate/settings` and `POST /api/affiliate/onboarding/submit`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/affiliate/settings` | Protected page route; unauthenticated request redirects to `/auth/login`. |
| Template | `frontend/platform/affiliate-settings.html` | Tax classification, tax ID, VAT, tax name, payout preference, certification checkbox, status sidebar. |
| Component | `frontend/platform/components/investor-topbar.html` | Renders `#save-settings-btn` when `investor_topbar_variant == 'affiliate-settings'`. |
| JS | `frontend/platform/static/js/affiliate-settings.js` | Loads settings, submits to dedicated settings API with CSRF, renders status messages and badges. |
| CSS | `frontend/platform/static/css/affiliate-dashboard.css` | Provides settings grid, cards, messages, badges, and mobile fallbacks. |
| Backend page route | `GET /affiliate/settings` | Registered in `backend/src/rewards/mod.rs`; served by `page_affiliate_settings`. |
| Backend API route used by JS | `GET /api/affiliate/settings` | Returns masked tax ID and current tax/payout status. |
| Backend API route used by JS | `POST /api/affiliate/settings` | Saves tax/payout settings transactionally and writes masked audit state. |
| Existing relevant API route | `GET/POST /api/rewards/payout-settings` | Existing payout settings endpoint, but this page does not use it. |
| Database table | `affiliates` | Contains legacy `tax_id`, encrypted `tax_id_encrypted`, `tax_id_last4`, `tax_recipient_class`, `is_tax_ready`, `status`, `tax_document_gcs_path`. |
| Database table | `payout_settings` | Contains `payment_method`, account/address fields, and `vat_number`; no tax classification/status fields. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Save Changes button | `#save-settings-btn` in `investor-topbar.html` | Submit the settings form after certification is checked. | Fixed: button has `type="submit"` and `form="affiliate-settings-form"`. | Fixed: `POST /api/affiliate/settings`. | Static regression covered; authenticated browser recheck still needed. |
| Tax classification select | `#tax_class[name="tax_class"]` | Save affiliate tax classification and trigger payout hold/reverification. | Fixed: JS sends existing DB-backed values. | Fixed: updates `affiliates.tax_recipient_class` and sets `is_tax_ready=false` when changed. | Static regression covered. |
| Tax ID / SSN input | `#tax_id[name="tax_id"]` | Save replacement Tax ID or preserve existing encrypted value when blank. | Fixed: JS sends nullable `tax_id`; UI shows only masked current value. | Fixed: API preserves existing encrypted value when omitted, encrypts replacements, clears legacy plaintext, and masks response/audit logs. | Static regression covered; authenticated browser recheck still needed. |
| VAT Number input | `#vat_number[name="vat_number"]` | Save optional VAT number. | Fixed. | Fixed: upserts `payout_settings.vat_number`. | Static regression covered. |
| Business / Full Name input | `#tax_name[name="tax_name"]` | Save legal/tax recipient name. | Fixed. | Fixed: saves `affiliates.company_name` and `payout_settings.full_name`. | Static regression covered. |
| Payout preference select | `#payout_method[name="payout_method"]` | Save payout preference; only wallet currently selectable. | Fixed. | Fixed: validates and stores `poool_wallet`; rejects disabled methods server-side. | Static regression covered. |
| Certification checkbox | `#tax_certify` | Gate Save Changes and ensure user certification. | Fixed: required before save. | Fixed: `tax_certified` is required and audit metadata records certification. | Static regression covered. |
| Tax Status badge | `#tax_status_badge` | Show real tax verification state. | Fixed: populated from settings API. | Fixed: derived from affiliate tax state. | Static regression covered. |
| Payouts badge | `#payout_status_badge` | Show real payout eligibility/hold state. | Fixed: populated from settings API. | Fixed: derived from affiliate status, tax readiness, and tax document state. | Static regression covered. |
| Threshold note | Sidebar text `$50.00` | Explain payout threshold. | Static content. | Threshold is enforced elsewhere for payout requests, not loaded here. | Static only; acceptable as informational if kept in sync. |
| 1099 note | Sidebar text | Explain tax document timeline. | Static content. | No backend support on this page. | Static only. |

---

## Frontend Findings

### P1 - Save Changes Button Does Not Submit The Form

Location:

- Template: `frontend/platform/components/investor-topbar.html`
- Template: `frontend/platform/affiliate-settings.html`
- JS: `frontend/platform/static/js/affiliate-settings.js`

Problem:

`#save-settings-btn` is rendered in the topbar outside `#affiliate-settings-form`. It has no `type="submit"`, no `form="affiliate-settings-form"` attribute, and no click listener. The JS only listens for the form `submit` event, so clicking the primary visible button will not save anything.

Expected:

The visible Save Changes button should submit the form exactly once, show loading/error/success state, and remain disabled until required fields and certification are valid.

Evidence:

Static review confirmed the button exists only in the topbar component and lacks a form association. `affiliate-settings.js` only binds `form.addEventListener("submit", ...)`.

Recommended fix:

Associate the button with the form using `form="affiliate-settings-form" type="submit"` or bind a click handler that calls `form.requestSubmit()`. Keep one owner for the submit lifecycle.

### P1 - Settings Form Posts The Wrong Payload To The Onboarding Endpoint

Location:

- JS: `frontend/platform/static/js/affiliate-settings.js`
- Backend: `backend/src/rewards/routes.rs`
- Backend model: `backend/src/rewards/models.rs`

Problem:

The settings form posts `{ tax_class, tax_id, vat_number, tax_name, payout_method }` to `/api/affiliate/onboarding/submit`. That endpoint deserializes `SubmitOnboardingForm`, which requires `exam_passed`, `traffic_source`, `audience_size`, `main_url`, `phone_number`, `tax_id`, and `accepted_policies`. It also blocks users with existing `active` or `pending_approval` affiliate status, which are the users most likely to use settings.

Expected:

Affiliate settings should use a dedicated settings endpoint or the existing `/api/rewards/payout-settings` route with a compatible payload. Tax classification changes should update the affiliate tax state and trigger a clearly modeled compliance hold.

Evidence:

Static route/model review found no `/api/affiliate/settings` endpoint. Existing `/api/rewards/payout-settings` accepts `payment_method`, `account_email`, `full_name`, address fields, and `vat_number`, but the page does not use it. The local unauthenticated POST smoke to the onboarding endpoint was rejected by CSRF middleware before auth/validation.

Recommended fix:

Create a dedicated affiliate settings API or rework this page to call `/api/rewards/payout-settings` for payout fields plus a separate tax settings API for `tax_recipient_class`, tax ID, certification, verification state, and audit logging.

### P1 - State-Changing Fetch Omits Required CSRF Header

Location:

- JS: `frontend/platform/static/js/affiliate-settings.js`
- Runtime route: `POST /api/affiliate/onboarding/submit`

Problem:

The JS sends only `Content-Type: application/json`. The backend rejects state-changing requests without a valid CSRF token header.

Expected:

The save request should include the same CSRF token pattern used by other protected mutating flows and should show a visible inline error if the token is missing or expired.

Evidence:

Runtime command `curl -sS -H 'Content-Type: application/json' -d ... http://localhost:8888/api/affiliate/onboarding/submit` returned `403` with `CSRF token missing or invalid`.

Recommended fix:

Read the CSRF token from the platform’s established cookie/meta helper and send the expected `X-CSRF-Token` header on save.

### P2 - Page Loads The Same JavaScript Twice

Location:

- Template: `frontend/platform/affiliate-settings.html`
- Shared head: `frontend/platform/components/head.html`

Problem:

The template passes `extra_js=['affiliate-settings']` to the shared head, then also includes `<script src="/static/js/affiliate-settings.js"></script>` at the bottom of the page. Both copies register a `DOMContentLoaded` callback and can attach duplicate submit handlers.

Expected:

The page-specific script should be loaded once.

Evidence:

Static review confirmed both the `extra_js` path and explicit script tag. `node --check frontend/platform/static/js/affiliate-settings.js` passed, but duplicate loading remains a behavioral risk.

Recommended fix:

Remove the explicit body script or remove the `extra_js` entry, following the local page convention.

### P2 - Inline Two-Column Layout Has No Mobile Fallback

Location:

- Template: `frontend/platform/affiliate-settings.html`
- CSS: `frontend/platform/static/css/affiliate-dashboard.css`

Problem:

The main page grid is inline `grid-template-columns: 2fr 1fr`, and the tax ID/VAT row is inline `grid-template-columns: 1fr 1fr`. The loaded `affiliate-dashboard.css` does not define a responsive override for `.dash-grid`, `.side-col`, or this page’s form rows.

Expected:

The settings form should collapse to one column on mobile, with the sticky status card becoming normal document flow and all input text fitting without horizontal overflow.

Evidence:

Static CSS search found no relevant responsive rule for this page. Browser mobile verification was not performed because the audit is documentation-only and no authenticated fixture was available.

Recommended fix:

Move layout rules into CSS and add mobile breakpoints for the page grid and form rows.

## Backend Findings

### P1 - No Settings-Specific Backend Contract Or Verification State Machine

Location:

- Backend: `backend/src/rewards/mod.rs`
- Backend: `backend/src/rewards/routes.rs`
- Database: `affiliates`, `payout_settings`

Problem:

The page promises that changing tax classification or tax ID will temporarily freeze payout capability while compliance verifies the update. No settings-specific API implements that transition. The existing onboarding endpoint creates or resets an application, and the payout settings endpoint updates generic payout settings without tax verification semantics.

Expected:

A backend settings save should be authenticated, CSRF-protected, server-validated, auditable, and should atomically persist tax/payout changes plus the payout-hold state.

Evidence:

Route search found only `/api/affiliate/onboarding/submit`, `/api/rewards/payout-settings`, upload/material/policy routes, and no `/api/affiliate/settings` equivalent.

Recommended fix:

Add a dedicated route such as `GET/POST /api/affiliate/settings` that returns current affiliate tax/payout status and updates settings inside a transaction. Include audit logs and explicit status transitions for payout hold/reverification.

### P2 - Sensitive Tax Fields Are Not Modeled For Secure Settings Updates

Location:

- Database: `affiliates.tax_id`
- Database: `payout_settings`
- Docs: `docs/SECURITY.md`

Problem:

The page collects Tax ID / SSN, but the current affiliate schema stores `tax_id` as `VARCHAR(50)`. The security model expects tax ID handling to be sensitive and encrypted/minimized. The settings page also does not mask existing values because it does not fetch current settings at all.

Expected:

Tax IDs should be stored and displayed according to a sensitive-data policy: minimized, encrypted or tokenized where applicable, masked in UI, and never logged.

Evidence:

Local DB schema check showed `affiliates.tax_id character varying`. No settings GET path populates masked tax values.

Recommended fix:

Define the tax-data storage policy before wiring the page. At minimum, do not echo full tax IDs to the browser and avoid raw logs/errors containing tax fields.

Resolution 2026-04-28:

Implemented encrypted-at-rest writes with `TAX_ID_ENCRYPTION_KEY`, `tax_id_encrypted`, and `tax_id_last4`; settings and onboarding now clear the legacy plaintext column on write and do not preserve legacy plaintext without Tax ID re-entry.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax check | `node --check frontend/platform/static/js/affiliate-settings.js` | Script parses. | Passed with no output. | Pass |
| Backend compile check | `cd backend && cargo check` | Backend compiles. | Passed. | Pass |
| Protected page unauthenticated smoke | `curl -I http://localhost:8888/affiliate/settings` after starting local server. | Redirect to login. | `303 See Other`, `location: /auth/login`. | Pass |
| Save endpoint CSRF smoke | POST representative settings JSON to `/api/affiliate/onboarding/submit` without CSRF/session. | State-changing route rejects unsafe request. | `403` CSRF error. | Pass for middleware; page JS currently lacks required header. |
| Static JS syntax | `node --check frontend/platform/static/js/affiliate-settings.js` after fixes. | Script parses. | Passed. | Pass |
| Static regression tests | `python3 -m pytest tests/test_affiliate_settings_static.py -q` | Contract tests pass. | 4 passed. | Pass |
| Backend compile | `CARGO_TARGET_DIR=/tmp/poool-affiliate-settings-target cargo check -j1` | Backend compiles. | Passed after isolated dependency build. | Pass |
| Scoped diff check | `git diff --check -- ...affiliate settings files...` | No whitespace errors. | Passed. | Pass |
| Authenticated save click | Click Save Changes with active affiliate session. | Settings persist, status updates, success shown. | Not run; no authenticated affiliate fixture/session available. | Blocked |
| Mobile viewport smoke | Open `/affiliate/settings` on mobile. | Single-column form without overflow. | Not run in browser; responsive CSS was added and static-checked. | Needs browser recheck |

---

## Security Findings

- P1: The page’s mutating fetch omits CSRF token handling, so the current save path cannot pass backend CSRF validation.
- P1: The frontend posts tax/settings data to the onboarding endpoint rather than a settings endpoint with explicit authorization, validation, audit logging, and compliance state transitions.
- Fixed P2: Tax ID / SSN handling now uses `TAX_ID_ENCRYPTION_KEY`, encrypted storage, last-four display suffixes, masked responses/audit logs, and fail-closed legacy plaintext preservation.
- P2: The certification checkbox is client-only and is not persisted as an auditable attestation.

---

## Database Findings

- `affiliates` now has encrypted Tax ID storage columns (`tax_id_encrypted`, `tax_id_last4`) added by `database/092_affiliate_tax_id_encryption.sql`; settings and onboarding writes clear the legacy plaintext `tax_id` column.
- `payout_settings` has `payment_method`, `full_name`, and `vat_number`, but the page sends `payout_method` and `tax_name`, which do not match `SavePayoutSettingsForm`.
- No database-backed audit trail was found for affiliate settings tax certification or tax classification changes.

---

## Missing Tests

- Add an authenticated browser test for `/affiliate/settings` covering initial load, no console errors, Save Changes disabled/enabled state, successful save, and visible error state.
- Add API tests for the future affiliate settings endpoint covering CSRF rejection, unauthenticated rejection, active-affiliate authorization, validation failures, tax/payout persistence, and audit log rows.
- Add regression tests proving Save Changes submits exactly once.
- Add mobile/keyboard accessibility smoke for the settings form and topbar Save Changes button.
- Add authenticated data-sensitivity tests ensuring Tax ID is never returned unmasked or logged and that database writes clear legacy `tax_id`.

---

## Recommended Fix Order

1. Wire `#save-settings-btn` to submit `#affiliate-settings-form` exactly once and remove duplicate script loading.
2. Replace the onboarding POST with a dedicated affiliate settings API, or split payout fields to `/api/rewards/payout-settings` and tax fields to a new audited tax settings route.
3. Add CSRF token handling and inline success/error/loading states.
4. Load real tax/payout status into the sidebar and remove hardcoded `Verified` / `Active` badges.
5. Move inline layout styles into CSS and add mobile breakpoints.
6. Add authenticated browser/API regression coverage.

---

## Final Status

`needs_recheck`

Reason: documented implementation issues are fixed in code, including encrypted Tax ID storage, but authenticated browser/API verification is still needed.
