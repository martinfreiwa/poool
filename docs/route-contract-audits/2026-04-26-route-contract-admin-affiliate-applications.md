# Route/API Contract Audit: Admin Affiliate Applications

Date: 2026-04-26

Selected scope: Admin Affiliate Applications page and affiliate application approval/rejection API contract.

Page route: `/admin/affiliate-applications`, `/admin/affiliate-applications.html`

Frontend:

- `frontend/platform/admin/affiliate-applications.html`
- `frontend/platform/admin/js/admin-affiliate-applications.js`
- `frontend/platform/static/js/admin-permission-guard.js`
- `frontend/platform/static/js/admin-sidebar-loader.js`
- `frontend/platform/static/js/user-data.js`

Backend:

- `backend/src/admin/mod.rs`
- `backend/src/admin/pages.rs`
- `backend/src/admin/rewards.rs`
- `backend/src/admin/extractors.rs`
- `database/079_affiliate_manage_permission.sql`

## Route/File Map

| UI / API surface | Frontend caller | Backend route / handler | Contract status |
| --- | --- | --- | --- |
| Page render | `/admin/affiliate-applications`, `/admin/affiliate-applications.html` | `backend/src/admin/mod.rs` -> `page_admin_generic` | Exists and admin-protected, but not gated by `affiliates.manage`. |
| Sidebar navigation | `/admin/affiliate-applications.html` link in `admin-sidebar-loader.js` | Same page route | Exists, but sidebar permission map does not hide affiliate nav links for admins without affiliate permissions. |
| Sidebar pending badge | `fetch("/api/admin/rewards/affiliates/pending")` in `admin-sidebar-loader.js` | `GET /api/admin/rewards/affiliates/pending` -> `api_admin_affiliates_pending` | Exists; badge call requires `affiliates.manage`. |
| Load pending applications | `fetch("/api/admin/rewards/affiliates/pending")` | `api_admin_affiliates_pending` | Exists; response shape matches current page fields. |
| Approve application | `POST /api/admin/rewards/affiliates/:id/approve` | `api_admin_affiliate_approve` | Exists; body shape matches, backend validation is stricter than frontend validation. |
| Reject application | `POST /api/admin/rewards/affiliates/:id/reject` | `api_admin_affiliate_reject` | Exists; body shape matches, but reason length/structure contract is not bounded. |

## Frontend Action Inventory

Links/navigation:

- Breadcrumb link to `/admin/`.
- Sidebar link from `admin-sidebar-loader.js` to `/admin/affiliate-applications.html`.
- External application URL links from `main_url`, restricted client-side to `http:` and `https:` and opened with `rel="noopener"`.

Forms/modals:

- No traditional HTML form submit.
- Details modal is populated from the pending-list response.
- Approve modal collects:
  - `referral_code`
  - `commission_rate_bps`
- Reject modal collects:
  - `reason`

Fetch/API calls:

- `GET /api/admin/rewards/affiliates/pending`
  - Expected response: `{ pending: AffiliateApplication[], counts: { pending, active, rejected } }`.
  - UI consumes `pending[].id`, `email`, `user_name`, `traffic_source`, `audience_size`, `main_url`, `phone_number`, `company_name`, `tax_id`, `created_at`.
  - UI consumes `counts.pending`, `counts.active`, `counts.rejected`.
- `POST /api/admin/rewards/affiliates/:id/approve`
  - Body: `{ referral_code: string, commission_rate_bps: number }`.
  - Expected success: any 2xx JSON; backend returns `{ status: "approved", referral_code }`.
  - Expected error: `{ error }`.
- `POST /api/admin/rewards/affiliates/:id/reject`
  - Body: `{ reason: string }`.
  - Expected success: any 2xx JSON; backend returns `{ status: "rejected" }`.
  - Expected error: `{ error }`.

CSRF/auth expectations:

- `admin-permission-guard.js` installs a global `fetch` interceptor that adds `X-CSRF-Token` from the `csrf_token` cookie for `POST`, `PUT`, `PATCH`, and `DELETE`.
- The page script itself sends only `Content-Type: application/json`; it relies on the global interceptor for CSRF.
- `backend/src/auth/csrf.rs` is mounted globally in `backend/src/main.rs`, so state-changing affiliate POSTs are covered by middleware.
- Backend page/API authentication uses `AdminUser`.
- Backend affiliate APIs call `admin.require_permission(&state.db, "affiliates.manage")`.

## Backend Route Inventory

Registered in `backend/src/admin/mod.rs`:

- `GET /admin/affiliate-applications`
- `GET /admin/affiliate-applications.html`
- `GET /api/admin/rewards/affiliates/pending`
- `POST /api/admin/rewards/affiliates/:id/approve`
- `POST /api/admin/rewards/affiliates/:id/reject`

Related affiliate routes registered in the same backend route group but not used by this page:

- `GET /api/admin/rewards/affiliates/fraud-scan`
- `POST /api/admin/rewards/affiliates/:id/suspend`
- `GET /api/admin/rewards/affiliates/payouts/pending`
- `POST /api/admin/rewards/affiliates/:id/payout`
- `POST /api/admin/rewards/affiliates/:id/clawback`
- `GET /api/admin/rewards/affiliates/materials`
- `POST /api/admin/rewards/affiliates/materials/:id/review`

Handler response shapes:

- Pending returns `{ pending, counts }`, where each pending item includes the fields consumed by the page.
- Approve validates UUID path, referral-code length/charset, commission-rate range, KYC status, pending status, and referral-code uniqueness; on success it returns `{ status: "approved", referral_code }`.
- Reject validates UUID path, non-empty sanitized reason, and pending status; on success it returns `{ status: "rejected" }`.
- Errors use `ApiError`, returning JSON `{ error }` with the appropriate status.

## Mismatches And Issues

### High: Page route is less restrictive than the affiliate APIs

Evidence:

- `GET /admin/affiliate-applications(.html)` is served by `page_admin_generic`.
- `page_admin_generic` has dedicated route-level permission checks for community and marketplace pages, but not affiliate pages.
- The selected APIs require `affiliates.manage`.

Impact:

- An admin with generic admin role but without `affiliates.manage` can load the affiliate applications page shell.
- The API calls then fail with 403, leaving a partially usable admin surface whose page-level authorization contract does not match the backend API contract.
- Because this is an admin-only workflow involving personal/tax application data, the page should fail closed before rendering the workflow shell.

Recommended fix:

1. Add an affiliate-specific page gate for `/admin/affiliate-applications(.html)` requiring `affiliates.manage`.
2. Prefer a dedicated page handler, or extend `page_admin_generic` with a small affiliate route permission map.
3. Keep API permission checks in place; the page gate is not a substitute for API authorization.

### Medium: Affiliate navigation is not represented in the client permission map

Evidence:

- `admin-sidebar-loader.js` renders `nav-affiliate-apps`, `nav-affiliate-finance`, and `nav-affiliate-fraud`.
- `admin-permission-guard.js` has no permission map entries for those nav IDs.
- The sidebar badge fetches `/api/admin/rewards/affiliates/pending`, which requires `affiliates.manage`.

Impact:

- Admins without affiliate permissions may still see and navigate to affiliate admin links.
- The pending badge and page fetch can produce avoidable 403s.
- The client guard is advisory only, but it should match backend route/API contracts to avoid dead-end UI.

Recommended fix:

1. Add affiliate nav IDs to `PAGE_PERMISSION_MAP`.
2. Use `affiliates.manage` for applications and management actions.
3. Consider a narrower `affiliates.view` permission if read-only affiliate admin pages are intended.

### Medium: Referral-code frontend validation is weaker than the backend contract

Evidence:

- Frontend approval validation only requires a non-empty code and a 1-450 bps commission rate.
- Backend requires referral codes to be 3-20 characters and only ASCII uppercase letters, digits, underscores, or hyphens.

Impact:

- Operators can submit codes that the backend predictably rejects.
- The modal does surface the backend error, but the client-side contract advertises a looser input than the API accepts.

Recommended fix:

1. Mirror the backend referral-code length and character rules in the approve modal.
2. Keep backend validation authoritative.
3. Show a field-level error instead of `alert()`.

### Medium: Rejection reason contract has no maximum length

Evidence:

- Frontend only checks that `reason.trim()` is non-empty.
- Backend sanitizes text and checks non-empty, then writes the reason into `audit_logs.new_state` and an outbound email body.
- No maximum length or structured reason taxonomy is visible in the selected handler.

Impact:

- Very large rejection reasons can be stored and sent.
- The backend accepts an unbounded admin-supplied string for an audit/email path.
- Operationally, this makes audit review and email delivery less predictable.

Recommended fix:

1. Add a server-side maximum length for rejection reasons.
2. Mirror the same limit in the textarea UI.
3. Consider adding optional predefined rejection categories if compliance needs structured reporting.

### Low: Pending-list response schema is not validated by the frontend

Evidence:

- The page assigns `pendingApps = data.pending || []`.
- It passes `data.counts || {}` to KPI rendering.
- A malformed 2xx response without `pending` is treated as a successful empty list rather than a contract failure.

Impact:

- Backend or proxy regressions can look like "0 pending" or placeholder counts instead of a visible contract error.
- This weakens operator confidence in a review queue.

Recommended fix:

1. Validate that `pending` is an array and `counts` is an object with numeric fields.
2. Show a retryable contract error when the schema is malformed.

## Missing Routes

No missing backend routes were found for the selected page's visible fetch calls, page navigation, or approval/rejection actions.

## Dead UI Actions

No completely dead UI action was found. The visible Review, Approve, and Reject actions all map to backend routes.

## Unused Backend Routes Noticed In Scope

The affiliate admin route group contains related routes that this page does not call:

- Fraud scan.
- Suspend affiliate.
- Pending payouts.
- Batch payout.
- Clawback.
- Marketing material review.

These appear intended for neighboring affiliate admin pages rather than dead routes in this selected scope.

## Severity Summary

| Severity | Count |
| --- | ---: |
| Critical | 0 |
| High | 1 |
| Medium | 3 |
| Low | 1 |
| Info | 0 |

## Recommended Fix Order

1. Align page-shell and sidebar authorization with `affiliates.manage`.
2. Add matching frontend referral-code validation and field-level API error display.
3. Add a backend maximum length for rejection reasons and mirror it in the UI.
4. Add pending-list response schema checks so malformed 2xx responses are not treated as valid empty state.

## Verification

This was a static route/API contract audit only. No production application code was changed and no mutating runtime calls were executed.

## Fix Follow-Up

Updated on 2026-04-26:

- Fixed page-shell authorization by requiring `affiliates.manage` for `/admin/affiliate-applications` and `/admin/affiliate-applications.html`.
- Fixed affiliate sidebar permission mapping for applications, finance, and fraud links; the affiliate badge fetch now waits for permission state.
- Fixed referral-code client validation to match the backend `3-20` uppercase letters, digits, underscores, or hyphens contract.
- Fixed rejection reason bounds in both frontend and backend with a 1000-character maximum.
- Fixed pending-list schema handling so malformed 2xx responses show a load error instead of rendering as an empty queue.
- Added static regression coverage in `tests/admin/test_affiliate_route_contract_static.py`.
