# Route/API Contract Audit: Admin Affiliate Finance

Date: 2026-04-27
Auditor: ChatGPT/Codex
Selected scope: Admin Affiliate Finance (`/admin/affiliate-finance`)
Status: issues found

## Summary

`/admin/affiliate-finance` has the expected page and API routes registered, and the frontend points at real backend endpoints for pending payout batches and payout execution. No missing backend routes were found.

The route contract is not release-ready because the page route is only admin-role gated while its navigation and APIs require `affiliates.manage`, and the pending-payout response presents rows as executable even though the POST payout route can reject the same affiliate for missing tax documents. The payout mutation also still transitions commissions using a broader predicate than the locked/summed rows, so the success response can report an amount/count that does not match every commission marked paid.

## Route And File Map

| Surface | Path / File | Contract Notes |
|---------|-------------|----------------|
| Page URL | `GET /admin/affiliate-finance` | Registered in `backend/src/admin/mod.rs` and rendered by `page_admin_generic`. |
| Page URL alias | `GET /admin/affiliate-finance.html` | Registered in `backend/src/admin/mod.rs`. |
| Template | `frontend/platform/admin/affiliate-finance.html` | Loads shared admin scripts and `/admin/js/admin-affiliate-finance.js`. |
| Page JS | `frontend/platform/admin/js/admin-affiliate-finance.js` | Calls pending-payout GET and affiliate payout POST. |
| Shared CSRF JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds `X-CSRF-Token` for mutating `fetch()` calls when the cookie exists. |
| Backend page handler | `backend/src/admin/pages.rs::page_admin_generic` | Authenticates admin role; no page-specific `affiliates.manage` gate for this path. |
| Pending API | `GET /api/admin/rewards/affiliates/payouts/pending` | Requires `affiliates.manage`; returns a top-level JSON array. |
| Execute API | `POST /api/admin/rewards/affiliates/:id/payout` | Requires `affiliates.manage`; path param is `uuid::Uuid`; no request body consumed. |
| Backend API handlers | `backend/src/admin/rewards.rs` | Implements pending payout grouping and payout execution. |

## Frontend Action Inventory

| UI Action | Frontend Source | Method / URL | Request Contract | Response Contract Expected |
|-----------|-----------------|--------------|------------------|----------------------------|
| Load page | Browser navigation | `GET /admin/affiliate-finance` | Admin session cookie. | HTML template with table and modal. |
| Admin breadcrumb | `href="/admin/"` | `GET /admin/` | Admin session cookie. | Admin dashboard HTML. |
| Global search | `#admin-global-search` | Shared admin search APIs | Shared script-owned contract. | Search results/dropdown. |
| Notification icon | `.admin-notification-btn` | None found in selected page | N/A | Dead or shared behavior not evident in selected files. |
| Load pending payouts | `loadPendingPayouts()` | `GET /api/admin/rewards/affiliates/payouts/pending` | Admin session with `affiliates.manage`; no query params. | Top-level array; each row has `affiliate_id`, `email`, `name`, `referral_code`, `total_payable_cents`, `commission_count`. |
| Open payout modal | Inline row `onclick` | Client-only | Uses selected row fields. | Modal text updates with affiliate name, cents, and count. |
| Execute payout | `confirmPayout()` | `POST /api/admin/rewards/affiliates/${id}/payout` | `Content-Type: application/json`; no JSON body; CSRF expected via global fetch wrapper. | JSON object with `batch_id`, `amount_cents`; frontend also tolerates `error` or `message` on non-2xx. |

## Backend Route Inventory

| Backend Route | Handler | Auth / Authorization | Request Contract | Response Contract |
|---------------|---------|----------------------|------------------|-------------------|
| `GET /admin/affiliate-finance` | `page_admin_generic` | Admin role only. | Path maps to `admin/affiliate-finance.html`. | HTML or redirect to login/admin. |
| `GET /admin/affiliate-finance.html` | `page_admin_generic` | Admin role only. | Static admin template path. | HTML or redirect to login/admin. |
| `GET /api/admin/rewards/affiliates/payouts/pending` | `api_admin_affiliate_payouts_pending` | `AdminUser` plus `affiliates.manage`. | No params/body. | JSON array of pending payout batch candidates. |
| `POST /api/admin/rewards/affiliates/:id/payout` | `api_admin_affiliate_batch_payout` | CSRF middleware, `AdminUser`, `affiliates.manage`. | UUID path param; no body. | Success JSON `{ success, batch_id, amount_cents, commission_count }` or JSON error. |

## Mismatches And Issues

### HIGH - Page route authorization is weaker than API and navigation contract

Evidence:

- `admin-permission-guard.js` maps `nav-affiliate-finance` to `affiliates.manage`.
- Both selected APIs require `affiliates.manage`.
- `page_admin_generic` has a special `affiliates.manage` page guard for `/admin/affiliate-applications`, but not for `/admin/affiliate-finance`.

Impact:

An admin without `affiliates.manage` can render the finance board page shell while the data APIs fail. That is an authorization contract mismatch for an admin-only financial surface and may expose page copy, action affordances, and operational workflow context to broader admins than intended.

Recommended fix:

Add `/admin/affiliate-finance` and `/admin/affiliate-finance.html` to the same page-level `affiliates.manage` guard used by affiliate applications.

### HIGH - Payout success response can describe fewer commissions than the mutation marks paid

Evidence:

- The payout handler locks and sums rows selected by `affiliate_id` and `status = 'payable'`.
- The later `UPDATE affiliate_commissions` uses `WHERE affiliate_id = $2 AND status = 'payable'` instead of the locked IDs.
- The response returns `amount_cents` and `commission_count` from the initially locked row set.

Impact:

The API contract says a payout response describes the executed batch. Under concurrent or stale-row conditions, rows outside the locked/summed set can be attached to the batch without being included in `amount_cents` or `commission_count`. This is a financial integrity and response-contract mismatch.

Recommended fix:

Update only the selected locked commission IDs and verify the affected row count equals the locked row count before wallet movements and commit.

### MEDIUM - Pending-payout API omits payout-blocking tax readiness that POST enforces

Evidence:

- The page labels the table “Payable Now” and renders `Release Payout` for rows from `GET /payouts/pending`.
- The pending API returns payable commission totals but no tax-document readiness or blocked reason.
- The POST payout route separately rejects affiliates without `tax_document_gcs_path`.

Impact:

The list endpoint and UI present a row as executable that the mutation endpoint can reject for a known compliance precondition. Admins only discover this after opening the modal and attempting the payout.

Recommended fix:

Return `tax_document_uploaded` and/or `payout_blocked_reason` from the pending API and disable or label blocked rows before the POST attempt.

### LOW - Notification button has no selected-scope route or handler

Evidence:

- The template renders `.admin-notification-btn`.
- No selected page script wires click behavior for that button.

Impact:

This appears to be a dead UI action within the selected page shell unless a broader shared handler is expected elsewhere. It is lower severity than payout actions but should be made consistent with the admin notification contract.

Recommended fix:

Wire the button to the admin notifications route or remove/disable it where no notification panel is available.

## Missing Routes

None found for the selected scope.

## Dead UI Actions

- `.admin-notification-btn` has no selected-scope handler.

## Unused Backend Routes Noticed In Selected Scope

No unused backend routes were identified for this selected scope. Adjacent affiliate admin routes such as pending applications, fraud scan, suspension, clawback, and materials were not audited beyond confirming they are out of scope for this run.

## CSRF And Error Handling Notes

- The selected POST endpoint is covered by global CSRF middleware.
- The frontend relies on `admin-permission-guard.js` to add `X-CSRF-Token`; the page includes that script before the page-specific JS.
- The frontend parses JSON before checking `res.ok` on payout execution. This is compatible with current API and CSRF JSON errors, but would show a generic catch/alert if an unexpected non-JSON response is returned.

## Issue Counts

| Severity | Count |
|----------|-------|
| Critical | 0 |
| High | 2 |
| Medium | 1 |
| Low | 1 |
| Info | 0 |

## Recommended Fix Order

1. Add the missing page-level `affiliates.manage` guard for `/admin/affiliate-finance`.
2. Restrict payout commission updates to the locked/summed commission IDs and assert affected row count.
3. Extend the pending-payout response with tax-document readiness/blocking reason and render it in the table.
4. Wire or remove the notification icon action in this page shell.

## Verification Performed

- Static route registration review in `backend/src/admin/mod.rs`.
- Static page/auth handler review in `backend/src/admin/pages.rs`.
- Static API handler review in `backend/src/admin/rewards.rs`.
- Static frontend/template review in `frontend/platform/admin/affiliate-finance.html`.
- Static page JS review in `frontend/platform/admin/js/admin-affiliate-finance.js`.
- No production application code was modified.
