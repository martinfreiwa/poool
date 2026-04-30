# Page Audit: Blockchain Contracts

Date: 2026-04-26
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/admin/blockchain-contracts`
Template: `frontend/platform/admin/blockchain-contracts.html`
JavaScript: `frontend/platform/static/js/admin-blockchain-contracts.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page styles
Backend Routes: `GET /admin/blockchain-contracts`, `GET /admin/blockchain-contracts.html`, `GET /api/admin/blockchain/treasury`

---

## Summary

The original audit findings have been fixed and verified with targeted authenticated E2E. `/admin/blockchain-contracts` and `/api/admin/blockchain/treasury` now require `treasury.read`, treasury database read failures propagate as API errors, contract rows are rendered with DOM/textContent APIs, copy actions provide visible accessible feedback, and the unused external HTMX dependency was removed.

---

## Tested Scope

- Static template review of `frontend/platform/admin/blockchain-contracts.html`.
- Static JavaScript review of `frontend/platform/static/js/admin-blockchain-contracts.js`.
- Backend route and handler review in `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, and `backend/src/admin/blockchain.rs`.
- Schema review of `database/058_blockchain_integration.sql`, `database/085_admin_blockchain_contract_controls.sql`, and related schema docs.
- Runtime unauthenticated curl smoke against the already-running local backend on `localhost:8888`.
- Fix verification on fresh local backend `http://localhost:8894`.
- Syntax check with `node --check frontend/platform/static/js/admin-blockchain-contracts.js`.
- Targeted authenticated E2E: `BASE_URL=http://localhost:8894 python3 -m pytest tests/e2e/test_admin_blockchain_contracts.py -q`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/blockchain-contracts` | Clean admin page route. |
| URL alias | `/admin/blockchain-contracts.html` | Registered alias. |
| Template | `frontend/platform/admin/blockchain-contracts.html` | KPI cards, contracts table, accessible copy status region. |
| JS | `frontend/platform/static/js/admin-blockchain-contracts.js` | Fetches treasury API and renders KPI/table rows with safe DOM construction. |
| Shared JS | `admin-permission-guard.js`, `admin-theme.js`, `admin-global-search.js`, dropdown scripts, `user-data.js` | Loaded by template. |
| Backend page route | `GET /admin/blockchain-contracts` | `page_admin_generic`. |
| Backend API route | `GET /api/admin/blockchain/treasury` | `api_admin_blockchain_treasury`. |
| Database table | `assets` | `chain_token_id`, `chain_contract_address`, `chain_network`, `chain_tx_hash`, token supply fields. |
| Database table | `chain_settlement_batches` | Batch KPI data included in shared treasury response. |
| Database table | `users` | Counts chain wallet addresses. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `nav.admin-breadcrumbs a[href="/admin/"]` | Navigate back to admin dashboard. | Link only. | Yes, admin dashboard route. | Not clicked; route exists. |
| Tokenize New Asset | `a[href="/admin/asset-tokenize.html"]` | Navigate to tokenization workflow. | Link only. | Yes, page is gated by `blockchain.tokenize`. | Not clicked; route exists. |
| Active Deployments KPI | `#kpi-active-clones` | Show tokenized assets with contract addresses. | Yes. | Yes via treasury API. | Verified by authenticated E2E. |
| Total Circulating Supply KPI | `#kpi-total-supply` | Sum `tokens_total`. | Yes. | Yes via treasury API. | Verified by authenticated E2E smoke. |
| Distributed Tokens KPI | `#kpi-distributed`, `#kpi-distributed-sub` | Sum sold tokens and percentage. | Yes. | Yes via treasury API. | Verified by authenticated E2E smoke. |
| Total count badge | `#total-count-badge` | Show number of contract rows. | Yes. | Yes via treasury API. | Verified by authenticated E2E. |
| Contracts table | `#contracts-tbody` | Render loading, empty, error, and rows. | Yes. | Yes via treasury API. | Safe DOM rendering verified by E2E. |
| Empty-state Tokenize Asset | Injected link in `renderContractsTable` | Navigate to tokenization when no contracts exist. | Yes. | Yes, page route exists. | Static verified only. |
| Contract explorer link | `.basescan-link` | Open contract in configured explorer. | Yes. | Explorer URL supplied by API/env. | Verified with `rel="noopener noreferrer"` and validated URL construction. |
| Copy Address button | `.copy-btn` | Copy contract address with Clipboard API/fallback. | Yes. | No backend needed. | Verified visible `#contracts-status` success feedback. |
| View Clone link | `href="/admin/blockchain-contract-detail.html?address=..."` | Navigate to contract detail page. | Yes. | Detail page/API exist. | Verified safe encoded address link. |
| Tx History link | `Tx History` | Open transaction in explorer. | Yes. | `chain_tx_hash` supplied by API. | Verified with `rel="noopener noreferrer"` and validated hash construction. |

---

## Frontend Findings

### P1 - Contract rows render database fields through `innerHTML`

Location:

- Template: `frontend/platform/admin/blockchain-contracts.html`
- JS: `frontend/platform/static/js/admin-blockchain-contracts.js:81`

Problem:

`renderContractsTable` builds row markup with template strings and assigns it to `tbody.innerHTML`. Asset titles, IDs, network values, contract addresses, transaction hashes, and explorer URLs are interpolated directly into HTML, attributes, and an inline `onclick`.

Expected:

Rows should be built with DOM APIs and `textContent`, or every value should be escaped for the exact HTML/attribute/JavaScript context. Explorer URLs should be constructed from validated chain addresses and hashes.

Evidence:

`asset.title`, `asset.id`, `asset.chain_network`, `asset.chain_contract_address`, and `asset.chain_tx_hash` are inserted into template markup without escaping.

Recommended fix:

Replace table rendering with `document.createElement`/`textContent`, `addEventListener` for copy buttons, and strict client-side URL construction from validated address/hash patterns. Keep backend validation as the authority.

Status: fixed 2026-04-26. `admin-blockchain-contracts.js` now builds rows with DOM APIs, uses `textContent`, validates contract addresses and transaction hashes before constructing links, and uses `addEventListener` instead of inline handlers. Targeted E2E seeded a malicious title and verified it rendered as text with no injected image.

### P2 - Copy action has no visible success or error state

Location:

- Template: `frontend/platform/admin/blockchain-contracts.html:131`

Problem:

The Clipboard API helper logs failures to the console and has a comment placeholder for success feedback, but admins receive no visible confirmation or failure message.

Expected:

Show a toast or inline status using the existing admin notification pattern, and make the button announce copy success/failure to assistive technology.

Evidence:

`navigator.clipboard.writeText(...).then(() => { /* UI feedback could go here */ })`.

Recommended fix:

Use the shared toast/status helper, remove inline event handlers, and add an accessible status region or button label update.

Status: fixed 2026-04-26. The template now includes `#contracts-status` with `role="status"` and `aria-live="polite"`, and the copy handler shows visible success/failure feedback. Targeted E2E stubs `navigator.clipboard`, clicks the copy button, and verifies the copied address plus visible success message.

### P3 - Page loads external HTMX although no HTMX behavior is used

Location:

- Template: `frontend/platform/admin/blockchain-contracts.html:11`

Problem:

The page includes `https://unpkg.com/htmx.org@1.9.10`, but no `hx-*` attributes appear in the template. This adds an avoidable CDN dependency on an admin blockchain page.

Expected:

Remove the script or self-host it only if the page uses HTMX.

Evidence:

Static template review found no HTMX attributes.

Recommended fix:

Delete the unused external script from the page.

Status: fixed 2026-04-26. The external HTMX script was removed from `frontend/platform/admin/blockchain-contracts.html`.

---

## Backend Findings

### P1 - Blockchain contracts page and treasury API are overbroadly available to generic admins

Location:

- Page route: `backend/src/admin/pages.rs:161`
- API route: `backend/src/admin/blockchain.rs:512`

Problem:

`page_admin_generic` only verifies an active `admin` or `super_admin` role for `/admin/blockchain-contracts`, and `api_admin_blockchain_treasury` only extracts `AdminUser`. The detail endpoint already requires `treasury.read`, and tokenization/control endpoints have dedicated permissions, but this page exposes settlement wallet config, contract addresses, batch/trade counts, and tokenized asset inventory to any generic admin.

Expected:

The page and `GET /api/admin/blockchain/treasury` should require a granular permission such as `treasury.read`, `blockchain.manage`, or a dedicated `blockchain.view`, aligned with sidebar visibility.

Evidence:

`page_admin_generic` has no blockchain-contracts permission branch, while `api_admin_blockchain_treasury` accepts `_admin: AdminUser` and never calls `require_permission`.

Recommended fix:

Add a blockchain page gate in `page_admin_generic` and require the same permission in the treasury API. Add denial tests for admins without that permission.

Status: fixed 2026-04-26. The page route and `GET /api/admin/blockchain/treasury` now both require `treasury.read`. Targeted E2E verifies unauthenticated API 401, admin-without-permission denial, and admin-with-`treasury.read` access.

### P2 - Treasury API silently converts database failures into empty/zero data

Location:

- Backend: `backend/src/admin/blockchain.rs:543`
- Backend: `backend/src/admin/blockchain.rs:603`
- Backend: `backend/src/admin/blockchain.rs:1240`

Problem:

The treasury handler uses `unwrap_or(0)` for KPI queries and helper functions use `unwrap_or_default()` for tokenized assets and settlement batches. A database/schema/query failure can look like a healthy zero-contract state.

Expected:

Read failures should propagate as an API error and render a visible page error. Zero/empty states should only appear after successful queries.

Evidence:

Static review found silent fallback on every core query used by this page.

Recommended fix:

Return `Result<Vec<_>, ApiError>` from helpers, use `?` for all database reads, and let the frontend display its existing error state for non-2xx responses.

Status: fixed 2026-04-26. Core treasury count queries and tokenized asset/batch helpers now propagate `ApiError` instead of using `unwrap_or(0)`/`unwrap_or_default()`. `cargo check` verifies the new result contract; fault-injection of a live DB failure was not performed.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page access | `curl -i http://localhost:8888/admin/blockchain-contracts` | Redirect to login. | `303 See Other` with `location: /auth/login`. | Pass |
| Unauthenticated API access | `curl -i http://localhost:8888/api/admin/blockchain/treasury` | JSON auth error. | `401 Unauthorized` with `{"error":"Authentication required"}`. | Pass |
| JS syntax | `node --check frontend/platform/static/js/admin-blockchain-contracts.js` | No syntax errors. | Passed with no output. | Pass |
| Authenticated contracts render | Open page with admin session and seeded tokenized asset. | KPI/table rows render from DB. | `tests/e2e/test_admin_blockchain_contracts.py` passed on `BASE_URL=http://localhost:8894`. | Pass |
| Permission denial | Admin without `treasury.read` opens page/API. | Page redirects to `/admin/`; API returns 403. | Targeted E2E passed. | Pass |
| Safe row rendering | Seed title with `<img src=x onerror=alert(1)>`. | Title renders as text; no injected image exists. | Targeted E2E passed. | Pass |
| Copy feedback | Click contract copy button with clipboard stub. | Address is copied and visible status appears. | Targeted E2E passed. | Pass |
| DB failure state | Force treasury query failure. | API returns error and UI shows error state. | Not fault-injected; code now propagates `ApiError` and `cargo check` passes. | Static verified |

---

## Security Findings

- Fixed: Generic admin access is replaced by aligned `treasury.read` page/API gates.
- Fixed: Contract rows no longer use `innerHTML` for database-sourced values.
- Fixed: Unused external HTMX script removed.
- Pass: unauthenticated page/API requests fail closed.

---

## Database Findings

- Required blockchain columns exist on `assets`: `chain_token_id`, `chain_contract_address`, `chain_network`, `chain_tx_hash`, and `chain_metadata_uri`.
- `chain_settlement_batches` exists for shared treasury batch data.
- `chain_contract_controls` exists for the contract-detail pause/unpause page, but this list page does not use it and always labels any asset with `chain_contract_address` as `Live Clone`; paused state is only visible after clicking through to detail.
- Fixed: core treasury query failures now propagate through `ApiError`; empty states only come from successful empty result sets.

---

## Missing Tests

- Added: authenticated E2E for `/admin/blockchain-contracts` with a seeded tokenized asset.
- Added: permission denial tests for admins without `treasury.read`.
- Added: UI test proving malicious asset titles render as text and cannot inject HTML.
- Partially covered: API database failure propagation is statically verified and compiled; live DB fault injection was not added.
- Keyboard/mobile smoke for copy buttons, explorer links, and detail navigation.

---

## Recommended Fix Order

1. Follow-up: add mobile viewport smoke for the contracts table.
2. Follow-up: add a fault-injection test for treasury DB errors if a safe test harness is introduced.
3. Follow-up: optionally surface paused clone state on the list by joining `chain_contract_controls`.

---

## Final Status

`completed`

Reason: PAGE-ISSUE-0292 through PAGE-ISSUE-0296 were fixed and targeted authenticated E2E passed. Remaining work is non-blocking broader mobile/fault-injection coverage.
