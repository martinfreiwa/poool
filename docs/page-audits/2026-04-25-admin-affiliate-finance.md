# Page Audit: Affiliate Finance

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/affiliate-finance`
Template: `frontend/platform/admin/affiliate-finance.html`
JavaScript: `frontend/platform/admin/js/admin-affiliate-finance.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/poool-dropdown.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/rewards.rs`

---

## Summary

The Affiliate Finance page is wired to real admin APIs and the payout backend uses integer cents, server-side authorization, row locks, a database transaction, wallet ledger entries, and audit logging. It is not ready to mark completed because the payout action renders user profile data into an inline JavaScript handler, and the payout transaction can update a broader set of payable commissions than the rows it locked and summed.

---

## Tested Scope

- Static review of the template, page JavaScript, shared admin permission/CSRF wrapper, admin page route, payout APIs, affiliate migrations, wallet schema, and existing affiliate E2E tests.
- Runtime unauthenticated smoke checks against the local server on `localhost:8888` for page/API auth and CSRF behavior.
- JavaScript syntax check for `frontend/platform/admin/js/admin-affiliate-finance.js`.
- Mutating payout execution was not run because this audit is documentation-only and no authenticated safe payout fixture was available.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/affiliate-finance` | Registered as clean and `.html` admin page route. |
| Template | `frontend/platform/admin/affiliate-finance.html` | Table, loading row, topbar search, notification button, payout modal. |
| JS | `frontend/platform/admin/js/admin-affiliate-finance.js` | Loads pending payouts, renders rows, opens modal, POSTs payout. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF header to mutating fetch calls. |
| Backend page route | `GET /admin/affiliate-finance` | `page_admin_generic`, gated by `AdminUser`. |
| Backend API route | `GET /api/admin/rewards/affiliates/payouts/pending` | Requires `affiliates.manage`; returns payout batches grouped by affiliate. |
| Backend API route | `POST /api/admin/rewards/affiliates/:id/payout` | Requires `affiliates.manage`; executes batch payout. |
| Database table | `affiliate_commissions` | Source payable commission rows. |
| Database table | `affiliates` | Referral code and tax document gate. |
| Database table | `payout_batches` | Batch payout record. |
| Database table | `wallets`, `wallet_transactions` | Treasury debit and affiliate cash-wallet credit. |
| Database table | `audit_logs` | Admin payout audit event. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin link | `a[href="/admin/"]` | Navigate to admin dashboard. | Link | `GET /admin/` | Not clicked; route exists. |
| Global admin search | `#admin-global-search` | Search admin users/assets/orders/deposits. | `admin-global-search.js` | Multiple admin APIs | Not authenticated at runtime; static wiring present. |
| Notifications button | `.admin-notification-btn` | Open notifications or notification panel. | No page-specific handler found. | Unclear | Unverified/dead on this page. |
| Pending payout count | `#pending-payouts-count` | Show number of payout batches ready. | `loadPendingPayouts()` | `GET /api/admin/rewards/affiliates/payouts/pending` | Static wiring present; unauth API returns 401. |
| Payout table body | `#payouts-body` | Render loading, empty, error, and payout rows. | `loadPendingPayouts()` | Pending payouts API | Static wiring present. |
| Release Payout button | inline `onclick="openPayoutModal(...)"` | Open confirmation modal for selected affiliate. | Yes, unsafe inline handler | Uses selected affiliate ID for POST | Needs recheck due inline handler issue. |
| Payout modal | `#payout-modal` | Confirm irreversible treasury debit/affiliate credit. | `openPayoutModal()`, `closePayoutModal()` | POST payout API | Static wiring present; missing dialog a11y/focus behavior. |
| Cancel button | `onclick="closePayoutModal()"` | Close modal without mutation. | Yes | Not needed | Static wiring present. |
| Execute Transfer button | `#payout-confirm-btn` | Disable, POST payout, show success/error, reload table. | `confirmPayout()` | `POST /api/admin/rewards/affiliates/:id/payout` | Static wiring present; unauth POST without CSRF returns 403 before auth. |

---

## Frontend Findings

### P1 - Payout row action embeds profile data into inline JavaScript

Location:

- Template: `frontend/platform/admin/affiliate-finance.html`
- JS: `frontend/platform/admin/js/admin-affiliate-finance.js:32-45`

Problem:

`loadPendingPayouts()` builds table rows with `innerHTML` and places `escapeHtml(p.name)` inside an inline `onclick="openPayoutModal('...', '...')"` JavaScript string. HTML escaping is not JavaScript-string escaping. A name containing an apostrophe can break the handler after entity decoding, and attacker-controlled profile data can expand the inline-event injection surface on an admin-only financial page.

Expected:

Build rows with DOM APIs, or render inert `data-*` attributes and attach delegated click handlers. Never place user profile fields inside inline event-handler JavaScript.

Evidence:

The row template interpolates `onclick="openPayoutModal('${p.affiliate_id}', '${escapeHtml(p.name)}', ...)"`.

Recommended fix:

Replace inline handlers with delegated listeners on `.release-payout-btn`, store ID/amount/count in `data-*`, and set the modal affiliate name with `textContent`.

### P2 - Tax-document gate is invisible in the table

Location:

- Template: `frontend/platform/admin/affiliate-finance.html:69-96`
- JS: `frontend/platform/admin/js/admin-affiliate-finance.js:14-50`
- Backend: `backend/src/admin/rewards.rs:1125-1140`

Problem:

The page title and table say “Payable Now” and “Release Payout” for every row returned by the pending payouts API, but the POST endpoint blocks payout if `affiliates.tax_document_gcs_path IS NULL`. The pending API does not return tax readiness, and the UI cannot disable or explain rows that will fail the compliance gate.

Expected:

Pending payout rows should include tax document readiness and show a clear blocked/ready state before the admin opens the irreversible payout modal.

Evidence:

GET groups all `ac.status = 'payable'` rows, while POST separately checks `tax_document_gcs_path IS NOT NULL`.

Recommended fix:

Join the `affiliates` tax fields in the pending API, return `tax_document_uploaded`/`payout_blocked_reason`, and disable the release action with guidance until tax documents are present.

### P3 - Payout modal lacks dialog accessibility behavior

Location:

- Template: `frontend/platform/admin/affiliate-finance.html:101-131`
- JS: `frontend/platform/admin/js/admin-affiliate-finance.js:65-80`

Problem:

The modal is a fixed `<div>` without `role="dialog"`, `aria-modal`, focus trapping, initial focus, Escape handling, or backdrop click behavior. Keyboard users can lose context or tab into the page behind the modal.

Expected:

Use dialog semantics, move focus into the modal when opened, restore focus on close, support Escape, and keep tab focus inside the modal while open.

Evidence:

Open/close only toggles `style.display`.

Recommended fix:

Add dialog attributes and a small focus-management helper, or reuse an existing admin modal primitive if available.

---

## Backend Findings

### P1 - Payout transaction can mark unsummed payable commissions as paid

Location:

- Backend: `backend/src/admin/rewards.rs:1096-1207`

Problem:

The payout handler locks rows with `SELECT id, provisional_amount_cents FROM affiliate_commissions WHERE affiliate_id = $1 AND status = 'payable' FOR UPDATE SKIP LOCKED`, computes `total_payable_cents` from those locked rows, but then updates commissions with `WHERE affiliate_id = $2 AND status = 'payable'`. A new payable commission inserted after the lock query, or any row outside the locked ID set, can be marked `paid` and attached to the batch without being included in the payout amount.

Expected:

Only the exact locked commission IDs should be transitioned to `paid` and attached to the payout batch. The update count should match `commissions.len()`.

Evidence:

The update predicate does not use the locked IDs selected at lines 1097-1103.

Recommended fix:

Collect locked IDs and update with `WHERE id = ANY($2)` or an equivalent SQLx-supported pattern, then verify affected rows equal the locked row count before debiting/crediting wallets.

### P2 - Existing payout E2E coverage is stale against the current implementation

Location:

- Tests: `tests/test_e2e_affiliate_full_funnel.py:253-279`
- Tests: `tests/test_e2e_affiliate.py:147-171`

Problem:

The full-funnel payout test seeds/queries wallet shapes that do not match the current payout handler: it looks for `wallet_type = 'default'`, uses a `label` field in treasury lookup, and does not set `tax_document_gcs_path`, while the backend creates/credits `wallet_type = 'cash'` and blocks payout without a tax document.

Expected:

E2E payout coverage should exercise the current tax gate, treasury wallet, `cash` destination wallet, `payout_batches`, `wallet_transactions`, and audit log behavior.

Evidence:

The inspected test code checks `wallet_type = 'default'` after calling the current payout endpoint.

Recommended fix:

Update or replace the affiliate payout E2E tests before relying on them for release confidence.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page access | `curl -I http://localhost:8888/admin/affiliate-finance` | Request rejected without admin session. | `401 Unauthorized` JSON from `AdminUser`. | Pass |
| Unauthenticated pending API | `curl -i http://localhost:8888/api/admin/rewards/affiliates/payouts/pending` | Request rejected without admin session. | `401 Unauthorized`, safe headers present. | Pass |
| Mutating API without CSRF | `curl -i -X POST /api/admin/rewards/affiliates/000.../payout` | Request rejected before mutation. | `403 Forbidden`, CSRF error. | Pass |
| JS syntax | `node --check frontend/platform/admin/js/admin-affiliate-finance.js` | No syntax errors. | No output; exit 0. | Pass |
| Authenticated happy-path payout | Create safe payable affiliate fixture and click Release/Execute. | One payout batch, exact locked commissions paid, treasury debit, cash-wallet credit, audit log. | Not run; no safe authenticated fixture in this documentation-only run. | Not run |

---

## Security Findings

- P1: Inline JavaScript handler renders profile data into executable context.
- CSRF protection exists globally through `backend/src/auth/csrf.rs`; `admin-permission-guard.js` wraps mutating fetches with `X-CSRF-Token`.
- Page and API routes are admin/session protected. API routes additionally require `affiliates.manage`.
- Reports avoid printing secrets, tokens, or personal data beyond field names and code paths.

---

## Database Findings

- Monetary values are stored and moved as `BIGINT` cents.
- Payout mutation is wrapped in one database transaction.
- Treasury wallet and destination wallet rows are locked or created in the transaction.
- Audit log and wallet transaction rows are written in the same transaction.
- The commission update predicate should be narrowed to locked IDs to preserve payout amount integrity.

---

## Missing Tests

- Unit/integration test for payout update scoping: insert a new payable commission after the lock step or simulate concurrent payout and assert only locked IDs transition to `paid`.
- Integration test for tax-document blocked rows returned by the finance board API.
- Browser/E2E test for the payout modal using a profile name containing an apostrophe and HTML-like characters.
- Accessibility test for modal focus, Escape close, and keyboard-only confirmation/cancel.

---

## Recommended Fix Order

1. Replace inline payout row handlers with delegated event listeners and non-executable data binding.
2. Change payout commission update to target only locked commission IDs and assert affected row count.
3. Add tax readiness to the pending payouts API and disable blocked rows in the UI.
4. Update affiliate payout E2E tests to match the current schema and tax gate.
5. Add dialog semantics and focus management to the payout modal.

---

## Final Status

`needs_recheck`

Reason: The page is wired to real backend functionality, but high-risk frontend injection surface and payout transaction scoping issues must be fixed and verified before completion.
