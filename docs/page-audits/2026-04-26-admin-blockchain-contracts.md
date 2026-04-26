# Page Audit: Blockchain Contracts

Date: 2026-04-26
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/blockchain-contracts`
Template: `frontend/platform/admin/blockchain-contracts.html`
JavaScript: `frontend/platform/static/js/admin-blockchain-contracts.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page styles
Backend Routes: `GET /admin/blockchain-contracts`, `GET /admin/blockchain-contracts.html`, `GET /api/admin/blockchain/treasury`

---

## Summary

The page is registered and unauthenticated access fails closed, but the authenticated read path is not production-ready. The page and treasury API are available to any generic admin instead of a granular blockchain/treasury permission, the API masks database failures as zero/empty data, and the table renders asset data with template-string `innerHTML`.

---

## Tested Scope

- Static template review of `frontend/platform/admin/blockchain-contracts.html`.
- Static JavaScript review of `frontend/platform/static/js/admin-blockchain-contracts.js`.
- Backend route and handler review in `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, and `backend/src/admin/blockchain.rs`.
- Schema review of `database/058_blockchain_integration.sql`, `database/085_admin_blockchain_contract_controls.sql`, and related schema docs.
- Runtime unauthenticated curl smoke against the already-running local backend on `localhost:8888`.
- Syntax check with `node --check frontend/platform/static/js/admin-blockchain-contracts.js`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/blockchain-contracts` | Clean admin page route. |
| URL alias | `/admin/blockchain-contracts.html` | Registered alias. |
| Template | `frontend/platform/admin/blockchain-contracts.html` | KPI cards, contracts table, inline copy helper. |
| JS | `frontend/platform/static/js/admin-blockchain-contracts.js` | Fetches treasury API and renders KPI/table rows. |
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
| Active Deployments KPI | `#kpi-active-clones` | Show tokenized assets with contract addresses. | Yes. | Yes via treasury API. | Unauthenticated API returns 401; authenticated not run. |
| Total Circulating Supply KPI | `#kpi-total-supply` | Sum `tokens_total`. | Yes. | Yes via treasury API. | Static verified only. |
| Distributed Tokens KPI | `#kpi-distributed`, `#kpi-distributed-sub` | Sum sold tokens and percentage. | Yes. | Yes via treasury API. | Static verified only. |
| Total count badge | `#total-count-badge` | Show number of contract rows. | Yes. | Yes via treasury API. | Static verified only. |
| Contracts table | `#contracts-tbody` | Render loading, empty, error, and rows. | Yes. | Yes via treasury API. | Error/loading paths exist; row rendering is unsafe. |
| Empty-state Tokenize Asset | Injected link in `renderContractsTable` | Navigate to tokenization when no contracts exist. | Yes. | Yes, page route exists. | Static verified only. |
| Contract explorer link | `.basescan-link` | Open contract in configured explorer. | Yes. | Explorer URL supplied by API/env. | Static verified only; URL is interpolated into HTML. |
| Copy Address button | `.copy-btn`, inline `onclick` | Copy contract address with Clipboard API. | Yes. | No backend needed. | Static verified only; no visible success/failure feedback. |
| View Clone link | `href="/admin/blockchain-contract-detail.html?address=..."` | Navigate to contract detail page. | Yes. | Detail page/API exist. | Static verified only. |
| Tx History link | `txLink` | Open transaction in explorer. | Yes. | `chain_tx_hash` supplied by API. | Static verified only; URL is interpolated into HTML. |

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

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page access | `curl -i http://localhost:8888/admin/blockchain-contracts` | Redirect to login. | `303 See Other` with `location: /auth/login`. | Pass |
| Unauthenticated API access | `curl -i http://localhost:8888/api/admin/blockchain/treasury` | JSON auth error. | `401 Unauthorized` with `{"error":"Authentication required"}`. | Pass |
| JS syntax | `node --check frontend/platform/static/js/admin-blockchain-contracts.js` | No syntax errors. | Passed with no output. | Pass |
| Authenticated contracts render | Open page with admin session and seeded tokenized asset. | KPI/table rows render from DB. | Not run in this documentation-only audit. | Not verified |
| Permission denial | Admin without blockchain/treasury permission opens page/API. | Denied or redirected. | Not run; static review indicates overbroad access. | Fail by static evidence |
| DB failure state | Force treasury query failure. | API returns error and UI shows error state. | Not run; static review indicates failures are masked. | Fail by static evidence |

---

## Security Findings

- P1: Generic admin access is too broad for a critical blockchain/contracts page and API.
- P1: Contract rows use `innerHTML` with database-sourced values, creating XSS risk in an admin-only surface.
- P3: Unused external HTMX script adds avoidable third-party dependency exposure.
- Pass: unauthenticated page/API requests fail closed.

---

## Database Findings

- Required blockchain columns exist on `assets`: `chain_token_id`, `chain_contract_address`, `chain_network`, `chain_tx_hash`, and `chain_metadata_uri`.
- `chain_settlement_batches` exists for shared treasury batch data.
- `chain_contract_controls` exists for the contract-detail pause/unpause page, but this list page does not use it and always labels any asset with `chain_contract_address` as `Live Clone`; paused state is only visible after clicking through to detail.
- Query failures are swallowed in the treasury handler and helpers, which can hide schema/data issues from operators.

---

## Missing Tests

- Authenticated E2E for `/admin/blockchain-contracts` with a seeded tokenized asset.
- Permission denial tests for admins without `treasury.read`/blockchain view permission.
- UI test proving asset titles/network/address/tx values render safely and cannot inject HTML.
- API test proving database read failures return errors instead of zero/empty success payloads.
- Keyboard/mobile smoke for copy buttons, explorer links, and detail navigation.

---

## Recommended Fix Order

1. Gate `/admin/blockchain-contracts` and `/api/admin/blockchain/treasury` behind a granular blockchain/treasury read permission.
2. Replace table `innerHTML` rendering and inline copy handlers with safe DOM construction and event listeners.
3. Propagate treasury database errors instead of returning empty/zero data.
4. Remove the unused external HTMX dependency and add accessible copy feedback.
5. Add authenticated permission, rendering, XSS, and error-state tests.

---

## Final Status

`needs_recheck`

Reason: the page was audited and unauthenticated access was verified, but security, XSS, DB error-masking, accessibility, and authenticated E2E gaps require fixes and recheck.
