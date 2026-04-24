# Page Audit: Affiliate Applications (Admin)

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/affiliate-applications`
Template: `frontend/platform/admin/affiliate-applications.html`
JavaScript: `frontend/platform/admin/js/admin-affiliate-applications.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/poool-dropdown.css`, `frontend/platform/static/css/fonts.css`, inline page styles
Backend Routes: `backend/src/admin/pages.rs`, `backend/src/admin/mod.rs`, `backend/src/admin/rewards.rs`, `backend/src/admin/extractors.rs`

---

## Summary

The Affiliate Applications admin page is structurally present (table + details/approve/reject modals) and is wired to real backend endpoints, but the core “Approve” flow is not actually consistent end-to-end: the frontend collects a referral code and commission bps, but the backend approval endpoint ignores those inputs (hardcodes commission and generates a random referral code). The “Details” modal also advertises fields (tax ID, user name) that the pending-applications API does not return, and two KPI cards are never populated.

Runtime testing was limited to unauthenticated curl smoke and CSRF behavior because no local admin session/credentials were available during this run.

---

## Tested Scope

- Static review of `frontend/platform/admin/affiliate-applications.html`.
- Static review of `frontend/platform/admin/js/admin-affiliate-applications.js` including DOM wiring, payload shapes, and error handling.
- Backend route map review in `backend/src/admin/mod.rs` and page handler behavior in `backend/src/admin/pages.rs`.
- Backend handler review for `/api/admin/rewards/affiliates/*` in `backend/src/admin/rewards.rs`.
- Database schema verification via migrations for `affiliates` fields used by the page (`database/072_affiliate_core_system.sql`, `database/073_affiliate_profile_data.sql`).
- Runtime unauthenticated curl checks for auth + CSRF behavior on page and API routes.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/affiliate-applications` | Admin generic page route |
| URL alias | `/admin/affiliate-applications.html` | Same handler (`page_admin_generic`) |
| Template | `frontend/platform/admin/affiliate-applications.html` | Admin page template |
| Component | `frontend/platform/admin/components/sidebar.html` | Included via `{% include %}` |
| JS | `frontend/platform/admin/js/admin-affiliate-applications.js` | Page controller |
| Shared JS | `frontend/platform/static/js/admin-global-search.js` | Topbar global search |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Global admin fetch guards/CSRF helpers |
| CSS | `frontend/platform/static/css/admin.css` | Admin styling |
| Backend page route | `GET /admin/affiliate-applications` | `page_admin_generic` (`backend/src/admin/pages.rs`) |
| Backend API route | `GET /api/admin/rewards/affiliates/pending` | Loads pending affiliate applications |
| Backend API route | `POST /api/admin/rewards/affiliates/:id/approve` | Approves affiliate (but ignores frontend payload) |
| Backend API route | `POST /api/admin/rewards/affiliates/:id/reject` | Rejects affiliate with reason |
| Database table | `affiliates` | Source of pending applications + approval status |
| Database table | `users` | Email lookup |
| Database table | `audit_logs` | Approval/rejection audit events |
| Database table | `kyc_records` (or equivalent) | Approval gated by KYC status (backend) |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Global search input | `#admin-global-search` | Live search across admin entities | Shared JS | Backend unknown | Not tested |
| Notifications button | `.admin-notification-btn` | Opens notifications panel/list | Shared JS | `GET /api/admin/notifications` (likely) | Not tested |
| KPI: Pending Review | `#kpi-pending` | Shows number of pending apps | Yes | Indirect (from pending list) | Not tested (admin session missing) |
| KPI: Active Affiliates | `#kpi-active` | Shows active affiliates count | No (not updated) | Unknown | Not tested |
| KPI: Rejected | `#kpi-rejected` | Shows rejected count | No (not updated) | Unknown | Not tested |
| Pending table body | `#pending-body` | Replaced with rows or empty/error state | Yes | `GET /api/admin/rewards/affiliates/pending` | Unauthed call returns `401` JSON |
| Review button | `button[onclick^="openDetailsModal("]` | Opens details modal for selected app | Yes | Uses already-loaded data | Not tested |
| Details modal | `#details-modal` | Shows app profile fields and approve/reject CTAs | Yes | Depends on pending API payload completeness | Not tested |
| Details “Approve” | `#details-approve-btn` | Opens approve modal for current app | Yes | `POST .../:id/approve` | Not tested |
| Details “Reject” | `#details-reject-btn` | Opens reject modal for current app | Yes | `POST .../:id/reject` | Not tested |
| Approve modal inputs | `#approve-referral-code`, `#approve-commission-rate` | Collect approval params | Yes | Backend currently ignores | Not tested |
| Approve confirm | `#approve-confirm-btn` | Submits approval | Yes | `POST /api/admin/rewards/affiliates/:id/approve` | Unauthed+no CSRF returns `403` JSON |
| Reject textarea | `#reject-reason` | Captures rejection reason | Yes | Sent as JSON payload | Not tested |
| Reject confirm | `#reject-confirm-btn` | Submits rejection | Yes | `POST /api/admin/rewards/affiliates/:id/reject` | Unauthed+no CSRF returns `403` JSON |

---

## Frontend Findings

### P1 - Approve modal collects values the backend does not use

Location:

- Template: `frontend/platform/admin/affiliate-applications.html` (Approve modal fields)
- JS: `frontend/platform/admin/js/admin-affiliate-applications.js` (sends `referral_code` + `commission_rate_bps`)

Problem:

The page prompts the admin to enter a specific referral code and commission rate, but the backend approval handler currently hardcodes commission (50 bps) and generates a random referral code. This can lead to operators believing they’ve assigned specific negotiated terms when they have not.

Expected:

Either the backend accepts and validates the provided values, or the frontend removes these inputs and clearly explains that values are auto-assigned.

Evidence:

- Frontend `confirmApprove()` sends JSON `{ referral_code, commission_rate_bps }`.
- Backend `api_admin_affiliate_approve()` does not parse a JSON body and sets `commission_rate_bps = 50` and generates a UUID-based code.

Recommended fix:

Align frontend/backend contract: add an approval payload to the backend and validate it (and return the assigned values), or remove/disable the inputs in the modal.

---

### P2 - KPI cards are partially dead UI

Location:

- Template: `#kpi-active`, `#kpi-rejected`
- JS: `updateKPIs()` only updates `#kpi-pending`

Problem:

The page displays “Active Affiliates” and “Rejected” KPIs but never updates them, leaving `—` permanently.

Expected:

Either fetch counts from a dedicated endpoint, or remove those KPI cards until implemented.

---

### P2 - Details modal expects fields the pending API does not return

Location:

- Template: `#details-name`, `#details-tax`
- JS: `openDetailsModal()` reads `app.user_name` and `app.tax_id`

Problem:

The pending-applications API returns no `user_name` or `tax_id`, so the modal will always show `—` for these fields even when the database has `affiliates.tax_id`.

Expected:

Include the required fields in `/pending`, or remove/hide those sections until the API supports them.

---

### P2 - Inline `onclick` + HTML string rendering increases injection surface

Location:

- JS: `renderPending()` uses inline `onclick="openDetailsModal('${app.id}')"` and builds HTML strings
- JS: `openDetailsModal()` sets `#details-url` via `innerHTML`

Problem:

Even with `escapeHtml()` for URLs, the approach relies on string concatenation and inline handlers. If backend data ever includes unexpected characters (or if IDs stop being strict UUIDs), this creates avoidable XSS/injection risk in an admin-only surface.

Expected:

Prefer DOM creation + `addEventListener` + `textContent` and use an allowlist/safe URL builder for outbound links.

---

### P2 - Modal accessibility gaps (no ARIA, no focus management, no ESC close)

Location:

- Template: `#details-modal`, `#approve-modal`, `#reject-modal`

Problem:

Modals have no `role="dialog"`, `aria-modal="true"`, labelled-by/desc-by wiring, focus trapping, or keyboard handling.

Expected:

Add baseline dialog accessibility (role/ARIA + focus trap + ESC close + return focus).

---

## Backend Findings

### P1 - Approval endpoint ignores frontend payload and overrides admin intent

Location:

- Backend: `backend/src/admin/rewards.rs` (`api_admin_affiliate_approve`)

Problem:

The backend uses a fixed commission (50 bps) and generates a referral code, while the frontend submits admin-provided values. This is a contract mismatch and will be perceived as “approval succeeded” even though the requested terms were not applied.

Recommended fix:

Define an explicit `AdminApproveAffiliatePayload { referral_code, commission_rate_bps }` and enforce:

- Referral code format/length allowlist (and uniqueness via DB constraint handling).
- Commission bps min/max (match UI or derive UI from backend constants).
- Audit log includes the final applied values.

---

### P1 - Pending list API swallows DB errors and returns empty success response

Location:

- Backend: `backend/src/admin/rewards.rs` (`api_admin_affiliates_pending`) uses `.unwrap_or_default()`

Problem:

If the DB query fails, the endpoint returns `{ pending: [] }` with `200 OK`, which can silently hide real pending applications.

Expected:

Return `500` with a safe error message and log the DB error.

---

### P2 - Reject endpoint is not transactional (update, audit log, email can diverge)

Location:

- Backend: `backend/src/admin/rewards.rs` (`api_admin_affiliate_reject`)

Problem:

The status update happens first, and audit log insertion/email sending are best-effort. If audit insertion fails, the system loses a compliance record of the rejection.

Expected:

Wrap the status update + audit insertion in one DB transaction (email can remain best-effort after commit).

---

### P3 - Admin HTML pages return JSON 401 instead of redirecting to login

Location:

- Backend: `backend/src/admin/pages.rs` (`page_admin_generic`) uses `AdminUser` whose rejection is `ApiError` (JSON)

Problem:

Unauthenticated GET requests to `/admin/*` return JSON `{"error":"Authentication required"}` instead of a redirect to a login page, which is confusing UX and breaks expected browser navigation semantics.

Expected:

Use a page-specific extractor or rejection handler that redirects unauthenticated users to an admin login route.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthed page access | `curl -i http://localhost:8888/admin/affiliate-applications` | Redirect or auth-required | `401` JSON `{"error":"Authentication required"}` | ⚠️ Needs recheck (auth ok, UX questionable) |
| Unauthed pending API | `curl -i http://localhost:8888/api/admin/rewards/affiliates/pending` | `401` | `401` JSON | ✅ Pass |
| POST approve without CSRF | `curl -i -X POST .../approve` | `403` | `403` CSRF error JSON | ✅ Pass |
| POST reject without CSRF | `curl -i -X POST .../reject` | `403` | `403` CSRF error JSON | ✅ Pass |

---

## Security Findings

- Admin APIs are gated by `AdminUser` (requires `admin`/`super_admin` role) and POSTs are blocked by CSRF middleware in local smoke.
- Missing fine-grained permission checks for affiliate-management APIs (currently “any admin role” can approve/reject/suspend).
- Frontend string-rendering + inline handlers increases XSS risk (admin-only but still avoidable).
- Rejection reason is embedded into an HTML email without escaping; a malicious admin could inject arbitrary HTML into emails.

---

## Database Findings

- `affiliates` includes `traffic_source`, `audience_size`, `main_url`, `phone_number`, `tax_id`, `company_name` via `database/073_affiliate_profile_data.sql`.
- `affiliates` includes `referral_code` (unique, NOT NULL) and `commission_rate_bps` via `database/072_affiliate_core_system.sql`.
- Backend pending API currently does not select `tax_id`, so the UI cannot display it even though the column exists.

---

## Missing Tests

- Backend integration test covering:
  - `POST /api/admin/rewards/affiliates/:id/approve` applies requested `commission_rate_bps` + `referral_code` (or assert it intentionally ignores them).
  - `GET /api/admin/rewards/affiliates/pending` returns `500` on DB error (after fixing unwrap/default behavior).
- Lightweight JS test or lint rule to prevent `onclick="..."` injection patterns in admin tables (optional).

---

## Recommended Fix Order

1. Align approve contract (backend payload + validation) OR remove the approve modal inputs.
2. Fix `/pending` error handling (no silent empty list on DB failure).
3. Make details modal fields match API (add `tax_id` + `user_name`, or hide fields).
4. Add modal accessibility baseline and remove inline `onclick` rendering patterns.

---

## Final Status

Choose one:

- `completed`: Page was audited and no blocking issues remain undocumented.
- `needs_recheck`: Issues were found and should be verified after fixes.
- `blocked`: Audit could not be completed.

Reason:
Core approve/reject behavior was not verified in an authenticated admin session, and key frontend/backend mismatches exist.

