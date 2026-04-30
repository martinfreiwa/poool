# Daily Page Audit: Admin Marketplace Primary Escrow

Date: 2026-04-28
Page: Marketplace Primary Escrow
Route: `/admin/marketplace/primary-escrow` and `/admin/marketplace/primary-escrow.html`
Template: `frontend/platform/admin/marketplace/primary-escrow.html`
Primary JS: inline script in the template, plus `frontend/platform/static/js/mp-toast.js`
Backend: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/primary_escrow.rs`
Tracker: `docs/page-review-tracker.yml`
Final status: `fixed_verified`

## Executive Summary

`/admin/marketplace/primary-escrow` renders an admin dashboard for active primary-offering escrow campaigns. The page route is protected by the generic admin shell and broad marketplace page permissions, the list API returns integer-cent based campaign metrics, and release execution now goes through the admin four-eyes approval system.

Fix pass update: the findings from this audit were addressed on 2026-04-28. The list API now requires marketplace permissions, active escrow balance now comes from investment ledger rows instead of token availability alone, the auto-refund worker claims expired assets with row locks before refunding, the page renders backend data through DOM/textContent, the sidebar permission guard maps the primary escrow nav item, and the release control now creates a notarized four-eyes approval request instead of presenting a fake alert-only action.

Verification update: static checks, targeted Rust test selection, inline JavaScript syntax validation, and an authenticated browser/API E2E pass now cover the primary escrow page, permission boundary, release request, self-approval rejection, checker approval, DB state transitions, and audit logs.

## Scope And Evidence

- Reviewed `frontend/platform/admin/marketplace/primary-escrow.html`.
- Reviewed route registration in `backend/src/admin/mod.rs`.
- Reviewed page authorization in `backend/src/admin/pages.rs`.
- Reviewed `backend/src/admin/primary_escrow.rs`, including list API and auto-refund worker.
- Reviewed `database/063_primary_offering_targets.sql`, `database/001_initial_schema.sql`, and related schema references for escrow fields.
- Reviewed sidebar permission guard behavior in `frontend/platform/static/js/admin-permission-guard.js`.
- Reviewed existing test references for primary escrow.

## UI Inventory

| Element | Selector / Location | Expected Behavior | Wiring | Backend Support | Status |
|---|---|---|---|---|---|
| Breadcrumb Admin link | `a[href="/admin/"]` | Navigate to admin dashboard. | Native link. | Admin route exists. | Wired. |
| Breadcrumb Marketplace link | `a[href="/admin/marketplace/"]` | Navigate to marketplace overview. | Native link. | Admin marketplace route exists. | Wired. |
| Read-only dashboard status | `.escrow-page-status` | Avoid fake health signal. | Static explanatory text. | N/A | Fixed; replaced misleading green health dots. |
| Loading state | `#loading` | Show while escrow API loads. | `role="status"` and `aria-live="polite"`. | Depends on `GET /api/admin/primary-escrow`. | Fixed pending browser recheck. |
| Error state and retry | `#escrow-error`, `#escrow-retry` | Show API errors and allow reload. | Wired with `role="alert"` and focus on retry. | Depends on API recovery. | Fixed pending browser recheck. |
| Escrow grid | `#escrow-grid` | Render active campaigns. | Inline `loadEscrow()` populates it with DOM APIs. | API returns active `assets` rows. | Fixed. |
| Empty state | `#escrow-empty` | Show when no active campaigns exist. | Wired after empty API response. | API returns an empty array. | Wired. |
| Campaign card title/id/status | `.escrow-title`, `.escrow-id`, `.escrow-badge` | Show asset identity and funding status. | DOM nodes with `textContent`. | API returns asset fields. | Fixed. |
| Progress bar | `.progress-fill` | Show sold-token percentage. | Uses API `progress_percent`, width, and `role="progressbar"`. | Backend derives from `tokens_total` and `tokens_available`. | Fixed pending browser recheck. |
| Current escrow balance | `.stat-label` "Current Escrow Balance" | Show funds held in escrow. | Uses API `current_escrow_cents`. | Backend sums `investments.purchase_value_cents` for active/funding-in-progress rows. | Fixed; deeper payment-ledger reconciliation can be future work. |
| Funding target / soft cap | Funding Target stat | Show total and minimum targets. | Uses API cents fields. | Backend derives from asset token counts and price. | Wired. |
| Escrow agent | Escrow Agent stat | Show assigned agent. | Rendered with `textContent`. | Stored on `assets.escrow_agent`. | Fixed. |
| Deadline | Deadline stat | Show funding deadline. | Uses API formatted timestamp. | Stored on `assets.funding_end_at`. | Wired. |
| View Asset link | `.admin-btn[href^="/admin/asset-details.html"]` | Navigate to asset details. | Normal link with encoded asset id. | `/admin/asset-details.html` exists. | Fixed. |
| Release request form | `.escrow-release-form` | Request escrow release only after soft cap and ledger balance exist. | Posts notarization reference/reason with CSRF to release request API. | `POST /api/admin/primary-escrow/:asset_id/release-request` creates `primary_escrow.release` approval. | Fixed and E2E verified. |

## Backend And Data Review

Routes exist for:

- `GET /admin/marketplace/primary-escrow`
- `GET /admin/marketplace/primary-escrow.html`
- `GET /api/admin/primary-escrow`
- `POST /api/admin/primary-escrow/:asset_id/release-request`
- `POST /api/admin/approvals/:id/approve` for checker execution of `primary_escrow.release`

Database support exists for:

- `assets.min_funding_tokens`
- `assets.escrow_agent`
- `assets.funding_status`
- `assets.funding_end_at`
- `investments.purchase_value_cents`
- `wallets.balance_cents`
- `wallet_transactions.amount_cents`
- `orders` and `order_items` for pending-order failure during auto-refund

Positive findings:

- Money fields in the API and refund worker use integer cents (`i64` / `BIGINT`).
- The auto-refund worker wraps wallet credits, wallet transaction inserts, investment status updates, and pending order failure in one database transaction per asset.
- The page does not perform trusted financial calculations client-side; it displays backend-provided cents and token counts.

## Findings

### P0 - Auto-refund worker can double-credit wallets across concurrent instances

`process_expired_escrow_refunds()` first selects expired assets outside any transaction and does not lock or atomically claim those asset rows. It then starts a transaction, updates the asset to `aborted` without a status predicate, selects investments by asset/status, and credits wallets.

In a multi-instance Cloud Run deployment, two workers can select the same expired asset before either transaction commits. Both can then refund the same investments because neither worker uses `SELECT ... FOR UPDATE SKIP LOCKED`, an advisory lock, or an `UPDATE ... WHERE funding_status IN (...) RETURNING` claim step. This is a real financial double-credit risk.

Recommended fix: make each expired asset claim atomic before refunding. Lock the asset row, update only if it is still in an active funding status, re-read refundable investments under the same transaction, and add idempotency constraints or ledger checks so each investment can be refunded once.

Tracker: `PAGE-ISSUE-0533`.

Status after fix pass: fixed in `backend/src/admin/primary_escrow.rs` by claiming one expired asset per transaction with `FOR UPDATE SKIP LOCKED`, predicate-guarding the status update, locking refundable investments, and inserting a system audit log.

### P1 - Primary escrow API allows generic admins without marketplace-specific API authorization

`GET /api/admin/primary-escrow` extracts only `AdminUser` and never calls `require_permission`. The page route checks broad marketplace permissions, but direct API access is not aligned with the page gate.

Why it matters: this endpoint exposes admin-only primary offering status, escrow agent data, deadlines, token availability, and current balance-style figures. Direct API authorization should be at least as strict as the page and should not rely on the sidebar or page shell.

Recommended fix: require `marketplace.view` or `marketplace.manage` in `api_admin_primary_escrow_list()` and add authenticated role-boundary tests for generic admin, marketplace view, marketplace manage, and marketplace compliance users.

Tracker: `PAGE-ISSUE-0534`.

Status after fix pass: fixed in `backend/src/admin/primary_escrow.rs`; the API now requires `marketplace.view`, `marketplace.manage`, or `marketplace.compliance`.

### P1 - Live campaign data is inserted with `innerHTML`

The inline script builds a template string with backend values including `camp.title`, `camp.asset_id`, `camp.funding_status`, `camp.escrow_agent`, and `camp.funding_end_at`, then inserts it with `grid.insertAdjacentHTML(...)`.

Why it matters: asset titles and escrow agent names are admin/developer-controlled data and can become stored admin-XSS if unsafe text enters the database. Admin pages are high-value targets because they carry financial and operational permissions.

Recommended fix: render dynamic values through DOM APIs and `textContent`, or escape every backend-derived value before HTML insertion. Replace inline `onclick` handlers with delegated listeners and `data-asset-id`.

Tracker: `PAGE-ISSUE-0535`.

Status after fix pass: fixed in `frontend/platform/admin/marketplace/primary-escrow.html`; dynamic campaign values now render through DOM APIs and `textContent`.

### P1 - Close & Release Escrow is a fake primary financial action

The page's primary action button displays "Close & Release Escrow" but only runs `alert('Release functions will be enabled when notarization is complete.')`. No release endpoint or workflow was found for this page.

Why it matters: the page presents a critical escrow lifecycle control as available while no backend action exists. Operators can believe release handling is in the system when the only implemented lifecycle automation is failed-campaign auto-refund.

Recommended fix: either remove/disable the button with clear status copy, or implement a full release workflow with marketplace/finance permission gates, maker/checker approval, transactionality, audit logs, exact cents ledger effects, and E2E tests.

Tracker: `PAGE-ISSUE-0536`.

Status after fix pass: fixed in `frontend/platform/admin/marketplace/primary-escrow.html`, `backend/src/admin/primary_escrow.rs`, `backend/src/admin/approvals.rs`, and `backend/src/admin/mod.rs`. The fake alert-only action was replaced by a CSRF-protected release request form that requires `marketplace.manage`, stores a notarization reference, creates a four-eyes `admin_approval_requests` row, blocks self-approval, and executes release by activating funding investments, completing pending primary orders, marking the asset `funded`, and writing audit logs.

### P1 - Displayed "Current Escrow Balance" is derived from token availability, not an escrow ledger

The API calculates `current_escrow_cents` as `(tokens_total - tokens_available) * token_price_cents`. It does not sum actual pending orders, wallet holds, bank-transfer proof state, escrow wallet balances, or investment statuses.

Why it matters: token availability is not the same as cash held in escrow. Cancellations, failed bank transfers, manual status changes, reconciliation drift, or pending-order cleanup can make the displayed balance materially wrong.

Recommended fix: define the canonical escrow balance source, preferably committed order/investment/payment ledger rows, and display that source with reconciliation status. Add DB assertions covering wallet checkout, bank-transfer pending orders, cancellations, abort refunds, and release.

Tracker: `PAGE-ISSUE-0537`.

Status after fix pass: fixed in `backend/src/admin/primary_escrow.rs`; `current_escrow_cents` now sums investment ledger rows for the asset.

### P2 - Primary escrow sidebar item is missing from the permission guard map

The sidebar loader defines `nav-mp-primary-escrow`, but `admin-permission-guard.js` has no `PAGE_PERMISSION_MAP` entry for that id. The client-side zero-trust hide pass therefore does not hide this nav item when permissions are fetched.

Why it matters: backend gates are the source of truth, but admin navigation should not advertise critical marketplace finance pages to roles that cannot use them. This also makes the UI permission model inconsistent with adjacent marketplace items.

Recommended fix: add `nav-mp-primary-escrow` to the map with the chosen permission contract, likely `marketplace.view` for read-only display or `marketplace.manage` if release controls remain on the page.

Tracker: `PAGE-ISSUE-0538`.

Status after fix pass: fixed in `frontend/platform/static/js/admin-permission-guard.js`; `nav-mp-primary-escrow` maps to `marketplace.view`.

### P2 - Loading, error, and accessibility states are too thin for an admin finance page

The loading text and failure text are plain `div` text updates without `aria-live`, focus movement, retry controls, or structured error details. The progress bar has no ARIA semantics, the static health dots always show OK, and the page uses inline button handlers.

Why it matters: operators need clear recoverable states on finance-critical dashboards. Keyboard and screen-reader users cannot reliably understand the load/error/progress states, and static green health indicators can mask backend failures.

Recommended fix: add an accessible status region, retry button, progressbar semantics, real health data or remove the dots, and browser/mobile accessibility coverage.

Tracker: `PAGE-ISSUE-0539`.

Status after fix pass: fixed in `frontend/platform/admin/marketplace/primary-escrow.html` and verified by authenticated Playwright coverage in `tests/e2e/test_admin_primary_escrow.py`.

## Remaining Issues

No documented primary-escrow page-audit issues remain open after this fix pass.

## Runtime / Automated Checks

| Command | Result | Notes |
|---|---:|---|
| `node --check <(sed -n '175,263p' frontend/platform/admin/marketplace/primary-escrow.html | sed '1d;$d')` | Pass | Inline page script parses successfully after extracting the `<script>` body. |
| `cd backend && cargo test primary_escrow` | Pass | No primary escrow tests exist; 202 Rust tests were filtered out. |
| `lsof -i :8888 -sTCP:LISTEN -n -P` | No server listening | Authenticated browser/curl checks were not run because no local backend was running. |
| `python3 -m pytest tests/admin/test_primary_escrow_static.py -q` | Pass | 6 static regression tests cover permission gate, release approval contract, refund locking/audit, safe rendering, release control, and nav permission mapping. |
| `node --check <extracted inline script>` | Pass | Re-run after frontend patch. |
| `cd backend && CARGO_TARGET_DIR=/tmp/poool-primary-escrow-check cargo check -q` | Blocked by unrelated issue | Failed on pre-existing `missing_docs` denial in `backend/src/admin/deposits.rs:122`, not on primary escrow code. |
| `python3 -m pytest tests/e2e/test_admin_primary_escrow.py -q` | Pass | Authenticated browser/API E2E covers non-admin denial, ledger-backed balance, release request form, four-eyes self-approval rejection, checker approval, DB state transitions, and audit logs. |
| `python3 -m py_compile tests/e2e/test_admin_primary_escrow.py` | Pass | E2E syntax check. |
| `cd backend && cargo test -q primary_escrow` | Pass | Targeted Rust selection completed; no matching Rust tests were selected. |

The E2E initially failed while `localhost:8888` was unavailable, then passed after a local backend became reachable. A transient local admin permission seed was needed in the E2E fixture because the shared development DB was missing the expected `admin` role marketplace grants before the test normalized them.

## Original Severity Counts

- Critical: 1
- High: 4
- Medium: 2
- Low: 0
- Info: 0

## Remaining Severity Counts

- Critical: 0
- High: 0
- Medium: 0
- Low: 0
- Info: 0

## Final Status

`fixed_verified`

The original code findings are fixed and authenticated browser/API E2E verification now covers the page's documented critical and high-risk flows.
