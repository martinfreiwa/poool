# Page Audit: Admin Marketplace P2P

Date: 2026-04-26
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/p2p`
Template: `frontend/platform/admin/marketplace/p2p.html`
JavaScript: `frontend/platform/static/js/mp-p2p.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

Follow-up fixes were applied after the audit. The P2P oversight page now uses real API-backed loading/error/empty states, safe DOM rendering for dynamic API data, explicit side/maker/taker/status/expiry columns, marketplace permission checks on the admin APIs, and an audited `admin_cancelled` backend transition for pending P2P offers.

Final status is `completed`.

---

## Fix Verification

Fixed issues:

- `PAGE-ISSUE-0228`: Added `POST /api/admin/marketplace/p2p/:offer_id/cancel`, `marketplace.manage` enforcement, pending-only transition, and `audit_logs` entry.
- `PAGE-ISSUE-0229`: Removed production mock fallback and added visible loading, empty, error, and retry table states.
- `PAGE-ISSUE-0230`: Replaced P2P row/modal dynamic rendering with DOM APIs and `textContent`; shared modal now escapes title/subtitle/confirm labels by construction.
- `PAGE-ISSUE-0231`: `GET /api/admin/marketplace/p2p` now requires `marketplace.view`; cancellation requires `marketplace.manage`.
- `PAGE-ISSUE-0232`: Replaced P2P list `.unwrap_or_default()` with `ApiError::Database` propagation.
- `PAGE-ISSUE-0233`: Shared marketplace modal now has `role="dialog"`, `aria-modal`, Escape handling, initial focus, async confirm handling, and inline validation support.
- `PAGE-ISSUE-0234`: Table now shows side, maker, taker, status, expiry, and created columns instead of mislabeling maker as seller.
- `PAGE-ISSUE-0235`: Added explicit loading, empty, error, and retry states.

Verification commands:

- `node --check frontend/platform/static/js/mp-p2p.js && node --check frontend/platform/static/js/mp-toast.js`
- `python3 -m py_compile tests/e2e/test_admin_marketplace_p2p.py`
- `cargo fmt --check`
- `cargo check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `curl -i --max-time 5 http://localhost:8888/admin/marketplace/p2p`
- `curl -i --max-time 5 http://localhost:8888/api/admin/marketplace/p2p`
- `BASE_URL=http://localhost:8888 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_marketplace_p2p.py -q`
- `python3 -m pytest tests/e2e/test_admin_marketplace_*.py -q`
- `python3 -m pytest tests/ -q`
- `git diff --check -- backend/src/admin/marketplace.rs backend/src/admin/mod.rs frontend/platform/admin/marketplace/p2p.html frontend/platform/static/js/mp-p2p.js frontend/platform/static/js/mp-toast.js tests/e2e/test_admin_marketplace_p2p.py`

Latest verification result:

- Passed: JS syntax checks, Python compile, `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` with 202 Rust tests, unauthenticated page/API curl smoke returning 401, targeted authenticated P2P Playwright E2E, and scoped `git diff --check`.
- Blocked outside this P2P fix: broad `python3 -m pytest tests/ -q` stops at `tests/e2e/test_admin_blockchain_contract_detail.py` because the mocked blockchain pause response reports `"mocked": false`; marketplace E2E subset stops at `tests/e2e/test_admin_marketplace_orderbook.py` because orderbook rebuild returns 500 when Redis is unavailable/configuration-dependent. The targeted P2P E2E passed independently.

---

## Tested Scope

- Reviewed the page template, table controls, breadcrumbs, and script includes.
- Reviewed `mp-p2p.js` loading, rendering, mock fallback, cancellation modal, and API assumptions.
- Reviewed shared `mp-toast.js` modal/toast behavior used by the page.
- Reviewed admin route registration and page permission handling.
- Reviewed `GET /api/admin/marketplace/p2p` query, response shape, and database dependencies.
- Reviewed `p2p_offers`, `assets`, `users`, and `trade_history` schema support.
- Started the local backend with `cargo run`; startup reported pre-existing duplicate migration warnings and Redis-not-configured warning, then served requests on `localhost:8888`.
- Performed unauthenticated route/API smoke checks against the running local server.
- Ran JavaScript syntax checks for page scripts.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/p2p` | Registered through `page_admin_generic`; unauthenticated curl returned 401 from the existing local server. |
| Alias | `/admin/marketplace/p2p.html` | Registered. |
| Template | `frontend/platform/admin/marketplace/p2p.html` | Table shell and breadcrumbs only. |
| JS | `frontend/platform/static/js/mp-p2p.js` | Owns list fetch, rendering, mock fallback, and cancel modal. |
| JS | `frontend/platform/static/js/mp-toast.js` | Shared toast and modal utility. |
| CSS | `frontend/platform/static/css/admin-marketplace.css` | Marketplace admin visual system. |
| Backend page route | `GET /admin/marketplace/p2p` | Authenticated admin page route; marketplace permission gate in generic page handler. |
| Backend API route | `GET /api/admin/marketplace/p2p` | Lists offers; lacks marketplace-specific permission check; no admin mutation route exists for cancel. |
| Database table | `p2p_offers` | Source of P2P offers. |
| Database table | `trade_history` | Last price source for deviation calculation. |
| Database table | `assets` | Asset title source. |
| Database table | `users` | Maker/taker email source. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `p2p.html:31-36` | Navigate to Admin and Marketplace overview. | Native links. | Page routes exist. | Static route map verified. |
| P2P table | `#p2p-table`, `#p2p-body` | Load real P2P offers and show empty/error/loading states. | Yes, via `loadP2P()`. | Partially, `GET /api/admin/marketplace/p2p` exists. | Unauthenticated API smoke returns 401; authenticated data path unverified. |
| Offer ID column | Rendered in `mp-p2p.js:83-86` | Show stable offer identifier. | Yes. | Yes. | Uses raw `innerHTML`. |
| Seller column | Rendered in `mp-p2p.js:86` | Show seller identity safely. | Partially. | Partially. | Uses unescaped `maker_email`; buy-side maker offers would be mislabeled as sellers. |
| Asset column | Rendered in `mp-p2p.js:87` | Show asset title safely. | Yes. | Yes. | Uses unescaped asset title. |
| Quantity/price columns | Rendered in `mp-p2p.js:88-90` | Show integer-token quantity and cents-derived prices. | Yes. | Yes. | Uses cents from backend; no validation for malformed payload. |
| Deviation badge | Rendered in `mp-p2p.js:68-77`, `91` | Flag price deviations from last market trade. | Yes. | Yes. | Works by static review; no empty market-price explanation beyond N/A. |
| Admin Cancel button | `.btn-cancel-p2p` | Cancel a suspicious P2P offer durably with reason, authz, audit log, and visible result. | Local-only click handler. | No admin cancel API route found. | Broken; removes row and shows success without persistence. |
| Cancellation reason field | `#p2p-cancel-reason` | Require reason and keep modal open on validation error. | Partially. | No backend. | Shared modal closes even when reason is empty. |
| Toasts | `mpToast()` | Announce success/error messages. | Yes. | N/A. | Uses safe `textContent`, but no `aria-live`. |
| Modal | `mpModal()` | Accessible confirmation dialog. | Partially. | N/A. | No dialog role, focus trap, Escape close, or validation-aware close. |

---

## Frontend Findings

### P1 - Admin Cancel is a fake local-only mutation

Location:

- Template: `frontend/platform/admin/marketplace/p2p.html:61-65`
- JS: `frontend/platform/static/js/mp-p2p.js:102-135`

Problem:

The Admin Cancel button opens a modal, accepts a reason, fades out the row, and shows a success toast. It never calls a backend endpoint, never changes `p2p_offers.status`, never records the admin actor or reason, and does not audit the action.

Expected:

Admin cancellation should call a real `POST`/`PATCH`/`DELETE` admin API, enforce `marketplace.manage`, validate the reason server-side, update `p2p_offers` transactionally, write an audit log, and only remove the row after a successful response.

Evidence:

No route for admin P2P cancellation exists in `backend/src/admin/mod.rs`; only `GET /api/admin/marketplace/p2p` is registered. The JS only manipulates DOM rows and calls `mpToast()`.

Recommended fix:

Add an admin cancellation endpoint such as `POST /api/admin/marketplace/p2p/:offer_id/cancel`, require `marketplace.manage`, lock the offer row, validate cancellable status, set `status = 'admin_cancelled'`, store reason if the schema is extended, and write `audit_logs`.

### P1 - API failures render mock offers as real oversight data

Location:

- JS: `frontend/platform/static/js/mp-p2p.js:13-22`, `141-153`

Problem:

If the list API returns 401/403/500 or the network fails, the page silently renders eight hardcoded mock offers. Because those rows still expose the same Admin Cancel controls and success toast, admins can mistake fake data and fake mutations for real marketplace oversight.

Expected:

Production admin pages must fail closed with a visible retryable error state. Demo data must not be used on authenticated admin surfaces unless behind an explicit local-dev flag.

Evidence:

`loadP2P()` catches all errors, logs a console warning, assigns `MOCK_OFFERS`, sets `usingMockData = true`, and calls `render()`.

Recommended fix:

Remove the mock fallback from production. Render a visible error row with a retry button and preserve HTTP status context for unauthorized, forbidden, and server-error cases.

### P1 - API fields are rendered through innerHTML

Location:

- JS: `frontend/platform/static/js/mp-p2p.js:44-98`
- Shared modal: `frontend/platform/static/js/mp-toast.js:79-92`

Problem:

The table row template interpolates `asset_name`, maker email local-part, offer IDs, status, and timestamps into `tbody.innerHTML`. Asset titles and user fields are database-backed and can include user/developer-controlled content depending on upstream validation. The shared modal also interpolates subtitles built from asset labels.

Expected:

Database-backed values should be rendered with DOM APIs and `textContent`, or escaped before use in HTML templates.

Evidence:

`tbody.innerHTML = offers.map(...).join('')` includes `${asset}`, `${seller}`, `${offerId}`, and `${created}`. `mpModal()` writes `opts.subtitle` and `opts.bodyHTML` through `overlay.innerHTML`.

Recommended fix:

Build table rows with `document.createElement()` and assign all dynamic text via `textContent`. For the shared modal, either build static structure with DOM APIs or restrict `bodyHTML` to developer-controlled markup and pass title/subtitle as text nodes.

### P2 - Empty, loading, and error states are missing

Location:

- Template: `frontend/platform/admin/marketplace/p2p.html:50-66`
- JS: `frontend/platform/static/js/mp-p2p.js:33-153`

Problem:

The page starts with an empty `<tbody>`, has no loading skeleton, shows no empty state when the API returns `[]`, and replaces errors with mock data instead of a visible failure state.

Expected:

The table should expose loading, empty, error, and retry states that are visible to keyboard and screen-reader users.

Evidence:

The template only contains `<!-- Filled by JS -->`. `render()` maps `offers` to a string; an empty array produces an empty table body.

Recommended fix:

Render an initial loading row, a real empty state for zero offers, and an error row with retry action. Add `aria-live` status text for async transitions.

### P3 - Seller column ignores offer side

Location:

- Template: `frontend/platform/admin/marketplace/p2p.html:53-61`
- JS: `frontend/platform/static/js/mp-p2p.js:53-56`, `86`

Problem:

The table column is labelled `Seller`, but the renderer always displays `maker_email` and ignores `side` and `taker_email`. For buy-side P2P offers, the maker is the buyer, so the page can label the wrong party as the seller.

Expected:

The page should show `Side`, `Maker`, and `Taker`, or derive the seller from `side` so compliance reviewers know which party is offering tokens.

Evidence:

The API returns `maker_email`, `taker_email`, and `side`, but the table only displays `seller = o.maker_email ? ...`.

Recommended fix:

Add a side/status column and render maker/taker roles explicitly with safe text nodes.

### P2 - Cancel modal validation and accessibility are incomplete

Location:

- JS: `frontend/platform/static/js/mp-p2p.js:110-135`
- Shared modal: `frontend/platform/static/js/mp-toast.js:79-111`

Problem:

The modal closes after confirm even when the cancellation reason is empty because `mpModal()` always calls `close()` after `opts.onConfirm()`. It also lacks `role="dialog"`, `aria-modal`, labelled title wiring, Escape close, focus trap, and focus restoration.

Expected:

Validation errors should keep the modal open and focus the failing field. Admin modals should be keyboard-operable and announced as dialogs.

Evidence:

The confirm listener calls `opts.onConfirm(overlay)` and then `close()` unconditionally.

Recommended fix:

Let `onConfirm` return `false` or throw a validation error to keep the modal open. Add dialog semantics, initial focus, Escape handling, focus trapping, and restoration to the triggering button.

---

## Backend Findings

### P1 - P2P list query hides database failures as an empty successful response

Location:

- Backend: `backend/src/admin/marketplace.rs:1566-1601`

Problem:

`GET /api/admin/marketplace/p2p` uses `.unwrap_or_default()` after `fetch_all()`. Any query/schema/database failure becomes `200 OK` with `[]`, making operators believe there are no P2P offers to review.

Expected:

Database errors should be logged and returned through `ApiError::Database` so the frontend can show an error state and operators can distinguish no offers from broken oversight.

Evidence:

The query ends with `.fetch_all(db).await.unwrap_or_default()`.

Recommended fix:

Replace with `.fetch_all(db).await.map_err(ApiError::Database)?`, and ensure the frontend displays non-OK responses instead of mock data.

### P1 - No backend support for admin P2P cancellation

Location:

- Backend route map: `backend/src/admin/mod.rs:757-758`
- User P2P routes: `backend/src/marketplace/routes.rs:540-552`

Problem:

The admin page exposes a cancellation action but the admin router only registers the read endpoint. The user-facing cancel endpoint exists for offer makers, not for admin oversight.

Expected:

Admin cancellation should have a dedicated admin route with marketplace management permission, server-side reason validation, state-machine checks, and audit logging.

Evidence:

`rg` found only `GET /api/admin/marketplace/p2p` under admin APIs.

Recommended fix:

Implement the admin mutation route or remove/disable the UI action until the backend contract exists.

### P1 - P2P API does not enforce marketplace-specific permission

Location:

- Backend: `backend/src/admin/marketplace.rs:1560-1563`

Problem:

The page-level generic route checks marketplace permissions, but `GET /api/admin/marketplace/p2p` only extracts `AdminUser`. Direct API calls therefore do not enforce the same fine-grained `marketplace.view`, `marketplace.manage`, or documented compliance permission gate used by the page/navigation model.

Expected:

Admin marketplace APIs should enforce the same domain permission that makes the sidebar/page visible.

Evidence:

The handler takes `_admin: AdminUser` and never calls `admin.require_permission(...)`.

Recommended fix:

Require `marketplace.view` for read-only P2P listing, or a documented `marketplace.compliance`/`marketplace.manage` permission if P2P oversight is intended to be narrower.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/mp-p2p.js && node --check frontend/platform/static/js/mp-toast.js` | Both scripts parse. | Passed with no output. | Pass |
| Backend startup | `cd backend && cargo run` | Server starts for smoke checks. | Started successfully after pre-existing duplicate migration warnings; Redis was not configured. | Pass |
| Unauthenticated page smoke | `curl -i http://localhost:8888/admin/marketplace/p2p` | 401 or login redirect. | `401 Unauthorized` JSON with security headers. | Pass |
| Unauthenticated API smoke | `curl -i http://localhost:8888/api/admin/marketplace/p2p` | 401. | `401 Unauthorized` JSON with security headers. | Pass |
| Schema support | `psql` checks for `p2p_offers`, `trade_history`, `assets`, `users`, `audit_logs`, and admin list SQL `EXPLAIN` | Required tables/columns exist and query parses. | Required tables/columns exist; SQL explains successfully. | Pass |
| Authenticated P2P list | Seed pending offer and load `/admin/marketplace/p2p` with admin session. | Real row renders without executing malicious asset title HTML. | Targeted Playwright E2E passed; no `img[src=x]` was created. | Pass |
| Admin cancel validation | Click Admin Cancel and submit empty reason. | Dialog remains open and reason error is visible. | Targeted Playwright E2E passed. | Pass |
| Admin cancel mutation | Submit cancellation with reason. | Backend status update and audit row. | Targeted Playwright E2E passed; `p2p_offers.status = admin_cancelled` and audit reason persisted. | Pass |
| Marketplace E2E subset | `python3 -m pytest tests/e2e/test_admin_marketplace_*.py -q` | Marketplace admin subset passes. | P2P and earlier marketplace tests passed until unrelated orderbook rebuild test returned 500 due Redis/config-dependent rebuild. | Blocked outside P2P |
| Broad pytest | `python3 -m pytest tests/ -q` | Full Python suite passes. | Stops at unrelated blockchain contract detail mocked-pause assertion before reaching most tests. | Blocked outside P2P |

---

## Security Findings

- P1: Admin Cancel is a client-side-only sensitive mutation with fake success, creating an operational integrity risk.
- P1: Mock fallback can expose fake admin-only marketplace data during real auth/API failures.
- P1: Database-backed fields are interpolated through `innerHTML`, creating an XSS risk if upstream validation permits HTML-like content.
- P1: Backend read failures are hidden as empty success, weakening monitoring and incident response.
- P1: The API lacks a marketplace-specific permission check even though the page-level route has one.
- No secrets or tokens were observed in the audited template or JS.
- The page and API correctly return 401 without an authenticated session in local smoke testing.

---

## Database Findings

- `p2p_offers` exists with integer `price_cents`, positive quantity checks, status constraints, expiry, parent offer, and trade references.
- `idx_p2p_asset`, `idx_p2p_taker`, and `idx_p2p_expiry` support the primary list/notification/expiry patterns.
- No schema field currently stores an admin cancellation reason; either add a reason/audit-log detail path or rely on `audit_logs` metadata.
- The list API joins `trade_history` for last price and computes deviation with SQL `NUMERIC`, which avoids JavaScript-side financial calculation for the core deviation.

---

## Missing Tests

- Added `tests/e2e/test_admin_marketplace_p2p.py`, covering authenticated page load, seeded P2P offer rendering, malicious asset-title safe rendering, required cancellation reason validation, backend admin cancellation, `admin_cancelled` persistence, and audit-log reason capture.
- Remaining optional coverage: direct API-only negative tests for non-marketplace admins, non-pending cancellation conflict, and forced database-error response behavior.

---

## Recommended Fix Order

1. Completed: real audited admin cancellation endpoint.
2. Completed: no mock fallback, visible states, and DB error propagation.
3. Completed: safe DOM rendering and aligned side/maker/taker/status columns.
4. Completed: marketplace.view/manage API permission gates.
5. Completed: authenticated P2P admin E2E coverage with safe seeded offers.

---

## Final Status

`completed`

Reason: The documented P2P issues were fixed and covered by an authenticated E2E that verifies safe rendering, reason validation, server-side admin cancellation, `admin_cancelled` persistence, and audit-log reason capture.
