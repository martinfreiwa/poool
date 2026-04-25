# Page Audit: Asset Tokenize

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/asset-tokenize`
Template: `frontend/platform/admin/asset-tokenize.html`
JavaScript: `frontend/platform/static/js/admin-asset-tokenize.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page styles
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/blockchain.rs`

---

## Summary

The page shell and eligibility fetch are implemented, but the core tokenization flow is not production-ready. The frontend POST sends an empty CSRF token on this admin template, the backend allows any `admin`/`super_admin` role to tokenize without fine-grained blockchain or asset-publish permission checks, and the irreversible on-chain deployment path is not protected against double-submit/race conditions or durable audit failures.

Final status is `needs_recheck`.

---

## Tested Scope

- Static review of `frontend/platform/admin/asset-tokenize.html`
- Static review of `frontend/platform/static/js/admin-asset-tokenize.js`
- Backend route registration in `backend/src/admin/mod.rs`
- Backend tokenization handlers in `backend/src/admin/blockchain.rs`
- Admin auth/permission extractor behavior in `backend/src/admin/extractors.rs`
- CSRF middleware behavior in `backend/src/auth/csrf.rs`
- Blockchain schema support in `database/058_blockchain_integration.sql` and `database/059_onchain_balances.sql`
- Test inventory search under `tests/`, `backend/src/`, and `frontend/platform/`

No production application code was modified.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/asset-tokenize` | Generic admin page route; page requires `?id=<asset_uuid>` or `?asset_id=<asset_uuid>`. |
| Alias | `/admin/asset-tokenize.html` | Also registered. |
| Template | `frontend/platform/admin/asset-tokenize.html` | SSR admin shell with sidebar, topbar, summary cards, checklist, deploy area. |
| JS | `frontend/platform/static/js/admin-asset-tokenize.js` | Loads pre-flight JSON and submits tokenization POST. |
| CSS | `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline `<style>` | Uses admin card/button classes plus page-specific inline styles. |
| Backend page route | `GET /admin/asset-tokenize` | `page_admin_generic` in `backend/src/admin/mod.rs`. |
| Backend API route | `GET /api/admin/blockchain/tokenize/:asset_id` | Eligibility and existing tokenization status. |
| Backend API route | `POST /api/admin/blockchain/tokenize/:asset_id` | Calls `cast send deployAsset(...)` then updates asset chain metadata. |
| Database table | `assets` | Reads valuation/supply/published/chain fields; writes chain metadata. |
| Database table | `audit_logs` | Intended audit record for tokenization, currently best-effort only. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `frontend/platform/admin/asset-tokenize.html:91` | Navigate Admin > Assets > asset details > current page. | Partially | Yes | Asset crumb is populated after API load; without query id remains `#`. |
| Back to Asset button | `frontend/platform/admin/asset-tokenize.html:102` | Return to previous page. | Inline `history.back()` | Not required | Works as browser history only; no deterministic asset detail fallback. |
| Page title | `#page-title` | Show `Tokenize: {asset title}`. | Yes, `textContent` | GET API | Statically wired. |
| Status timeline | `#timeline-tokenize`, `#timeline-live` | Show submitted/approved/tokenized/live progress. | Partially | GET API | Static assumptions; live state only changes for already-tokenized assets. |
| Asset summary cards | `#summary-valuation`, `#summary-price`, `#summary-supply`, `#summary-network` | Show valuation, token price, supply, network. | Yes | GET API | Statically wired; network is inferred from hostname, not backend config. |
| Pre-flight checklist | `#checklist` | Show pass/fail checks. | Yes | GET API | Only checks published, token supply, price, not already tokenized. |
| Deploy area | `#deploy-area` | Render deploy CTA or already-tokenized result. | Yes | GET/POST APIs | CTA renders when all current checks pass. |
| Tokenize button | `#btn-tokenize` | Confirm, disable in-flight, POST tokenization, reload on success. | Partially | POST API | CSRF token source is wrong on this template, so POST is expected to 403. |
| Native confirm dialog | `window.confirm(...)` | Require operator confirmation before chain deployment. | Yes | Not required | Works but is not a focus-managed platform confirmation. |
| Success/error alerts | `window.alert(...)` | Communicate POST result. | Yes | POST API | Basic alerts only; JSON parse failures are not handled before `res.json()`. |
| Treasury link | `/admin/blockchain-treasury` | Navigate to treasury for tokenized asset overview. | Yes | Page route exists | Statically wired. |
| Sidebar Asset Tokenize link | `frontend/platform/static/js/admin-sidebar-loader.js` | Navigate to tokenization page. | Yes | Page route exists | Links to `/admin/asset-tokenize.html` without an asset id, producing the page error state. |

---

## Frontend Findings

### P1 - Tokenize POST cannot satisfy CSRF protection

Location:

- Template: `frontend/platform/admin/asset-tokenize.html:11-21`
- JS: `frontend/platform/static/js/admin-asset-tokenize.js:198-205`
- Backend: `backend/src/auth/csrf.rs:16-20`, `backend/src/auth/csrf.rs:59-108`

Problem:

The frontend reads `document.querySelector('meta[name="csrf-token"]')?.content || ''`, but this admin template does not render a CSRF meta tag and does not include `csrf.js`. The global CSRF middleware requires `X-CSRF-Token` to match the `csrf_token` cookie for POST requests, so the tokenization POST sends an empty token and should receive 403.

Expected:

The page should use the shared CSRF helper or read the `csrf_token` cookie before making the irreversible `POST /api/admin/blockchain/tokenize/:asset_id` request.

Evidence:

`node --check frontend/platform/static/js/admin-asset-tokenize.js` passed, but static route and middleware review shows the header cannot match the cookie from this template.

Recommended fix:

Load the shared CSRF helper on the page or use the existing global `getCsrfToken()` convention, and add an admin tokenization browser test that proves POST includes a valid CSRF header.

### P2 - Sidebar and contracts-page entrypoints open a dead error state

Location:

- Template/JS: `frontend/platform/static/js/admin-sidebar-loader.js:144`
- Template: `frontend/platform/admin/blockchain-contracts.html:72`
- JS: `frontend/platform/static/js/admin-asset-tokenize.js:14-19`

Problem:

The page requires `?id=<uuid>` or `?asset_id=<uuid>`, but shared navigation links point to `/admin/asset-tokenize.html` without an id. Opening the page from those entrypoints produces `No asset ID provided` instead of offering an asset selector or route-safe workflow.

Expected:

Generic navigation should either route to a list/selector of tokenizable assets or be hidden in contexts without an asset id. Asset-specific tokenization links should include the id.

Evidence:

`assetId` is read only from query params, and the code returns an error before any API call when missing.

Recommended fix:

Create a tokenizable asset picker for the generic route, or remove generic links and keep only asset-specific links from review/detail pages.

### P2 - Dynamic HTML rendering is not consistently escaped

Location:

- JS: `frontend/platform/static/js/admin-asset-tokenize.js:76-92`, `frontend/platform/static/js/admin-asset-tokenize.js:107-137`, `frontend/platform/static/js/admin-asset-tokenize.js:149-175`, `frontend/platform/static/js/admin-asset-tokenize.js:230-234`

Problem:

The page uses `innerHTML` for API-derived and error-derived content. Most current interpolations are numeric or developer-controlled, but `chain_token_id`, `chain_contract_address`, and error strings are injected into HTML/attributes without escaping. This violates the frontend standard for user or database-originated data.

Expected:

Use DOM construction plus `textContent`/safe attribute setters for dynamic values, or escape all interpolated text and validate URL/address fields before using them in `href`.

Evidence:

The existing code directly interpolates `data.chain_contract_address` into an explorer URL and link text.

Recommended fix:

Replace dynamic `innerHTML` blocks with DOM builders or a small `escapeHtml`/validated-address helper, especially for already-tokenized result and error rendering.

---

## Backend Findings

### P1 - Tokenization lacks fine-grained permission checks

Location:

- Backend: `backend/src/admin/blockchain.rs:325-329`, `backend/src/admin/blockchain.rs:394-398`
- Admin extractor: `backend/src/admin/extractors.rs:120-163`

Problem:

Both GET and POST tokenization handlers only require `AdminUser`, which accepts any active `admin` or `super_admin` role. The endpoint does not require a specific `blockchain.tokenize`, `assets.publish`, or equivalent high-risk permission even though it can deploy a clone contract and mint the full token supply.

Expected:

Eligibility reads should require at least asset/blockchain read permission, and POST tokenization should require an explicit high-risk permission such as `blockchain.tokenize` plus `assets.publish` or a dedicated maker/checker approval.

Evidence:

Other admin areas now use `admin.require_permission(...)`; this handler does not.

Recommended fix:

Add explicit permissions to the RBAC migration/permission catalog and enforce them in both handlers. Consider two-person approval for production mainnet tokenization.

### P1 - Tokenization can double-deploy under concurrent requests

Location:

- Backend: `backend/src/admin/blockchain.rs:401-409`, `backend/src/admin/blockchain.rs:458-543`

Problem:

The handler checks `chain_token_id` before calling `cast`, but it does not lock the asset row, does not use idempotency, and updates with `WHERE id = $6` rather than `WHERE id = $6 AND chain_token_id IS NULL`. Two concurrent POSTs can both pass the pre-flight read and submit separate on-chain deployments before either update lands.

Expected:

An irreversible on-chain action must be idempotent and race-safe. At minimum, acquire a DB guard state or tokenization job row before the external call, and make the final update conditional with conflict handling.

Evidence:

The backend performs a plain `SELECT ... WHERE id = $1`, then a blocking external `cast send`, then an unconditional update.

Recommended fix:

Introduce a tokenization state/job table or row lock pattern that marks tokenization `in_progress` before calling the chain, with a unique/idempotency key per asset. Treat duplicate requests as conflict or return the existing job/result.

### P1 - Chain metadata update and audit log are not atomic or durable

Location:

- Backend: `backend/src/admin/blockchain.rs:524-561`

Problem:

After a successful on-chain deployment, the asset update and audit insert run as separate statements. The audit insert is followed by `.ok()`, so audit failure is silently ignored. If the asset update succeeds but audit logging fails, there is no durable operator record for the deployment. If the asset update fails after the chain transaction succeeds, the UI returns 500 but the chain may already contain a deployed asset that the database does not know about.

Expected:

Database writes after a successful chain transaction should be wrapped in a transaction where possible, and audit logging should be mandatory for this sensitive action. Chain-success/DB-failure reconciliation should be explicitly persisted.

Evidence:

The code calls `.execute(pool)` for the asset update, then a separate audit insert with `.await.ok()`.

Recommended fix:

Use a database transaction for metadata and audit writes, remove silent audit failure, and add a reconciliation record/status for chain success when DB persistence fails.

### P1 - Clone address parsing can persist the factory address as the asset contract

Location:

- Backend: `backend/src/admin/blockchain.rs:485-522`

Problem:

If receipt parsing or event-topic matching fails, the handler stores `contract_address` as the asset's `chain_contract_address`. In the EIP-1167 architecture, that value is the factory address, not the deployed clone. This can make the asset appear tokenized while pointing settlement/indexer/explorer workflows at the wrong contract.

Expected:

Missing clone address should be a hard failure or a recoverable reconciliation state, not a success fallback.

Evidence:

`unwrap_or_else(|| contract_address.clone())` is used as the clone address fallback after event parsing.

Recommended fix:

Require a valid deployed clone address from the expected event, validate `0x` address format, and store no success metadata until the clone address is known.

### P2 - Pre-flight checks are too thin for production tokenization

Location:

- Backend: `backend/src/admin/blockchain.rs:347-364`
- Frontend: `frontend/platform/static/js/admin-asset-tokenize.js:100-103`

Problem:

The pre-flight checklist only checks `published`, positive token supply, positive token price, and not already tokenized. It does not verify legal/KYC/compliance approvals, required documents, metadata availability/pinning, chain environment readiness, marketplace/funding status, or whether the current operator has permission to tokenize.

Expected:

The checklist should reflect all production blockers required before minting a real-world asset token.

Evidence:

The GET query only reads the `assets` row and returns four booleans.

Recommended fix:

Extend the API response with explicit checks for compliance status, required documents, metadata URI/IPFS readiness, chain config, permission, and funding/publication state.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route registration | Checked `backend/src/admin/mod.rs` for page/API routes. | Page and GET/POST API registered. | Routes exist for page aliases and `/api/admin/blockchain/tokenize/:asset_id`. | Pass |
| JS syntax | Ran `node --check frontend/platform/static/js/admin-asset-tokenize.js`. | No syntax errors. | Command passed. | Pass |
| Rust compile baseline | Ran `cargo check` in `backend/`. | Build completes. | Build passed with existing warning baseline. | Pass |
| CSRF contract | Compared frontend POST header source against CSRF middleware. | POST sends token matching `csrf_token` cookie. | Frontend sends empty header because no meta token exists. | Fail |
| Auth/authorization static check | Reviewed `AdminUser` extractor and tokenization handlers. | Explicit high-risk permission required. | Generic admin role is enough. | Fail |
| Race/idempotency static check | Reviewed POST sequence. | Duplicate POSTs cannot double-deploy. | Plain pre-flight read before external chain call; no idempotency or conditional update. | Fail |
| Browser authenticated mutation | Would require safe admin session, chain test config, and reversible test asset. | Confirm UI and network behavior. | Not run; destructive/irreversible chain action is not safe from this audit. | Not run |

---

## Security Findings

- P1: Tokenization POST lacks fine-grained authorization beyond generic admin role.
- P1: The frontend cannot submit valid CSRF for the mutation from this template.
- P1: Double-submit/race conditions can cause duplicate on-chain deployments.
- P1: Durable audit logging is not guaranteed for a sensitive blockchain action.
- P2: Dynamic rendering uses `innerHTML` for database/API-originated values and error strings.
- P2: Mainnet/testnet network display is inferred from hostname, not from backend chain configuration, which can mislead operators in staging or custom domains.

---

## Database Findings

- `assets.chain_token_id`, `assets.chain_contract_address`, `assets.chain_network`, `assets.chain_tx_hash`, and `assets.chain_metadata_uri` exist via `database/058_blockchain_integration.sql`.
- No unique partial constraint prevents more than one asset from being associated with the same clone address, and no tokenization job/idempotency table exists.
- The POST path updates `assets` and writes `audit_logs` outside a transaction.
- Audit log insertion is best-effort and can fail silently.
- No persisted reconciliation state exists for "chain transaction succeeded but database write failed".

---

## Missing Tests

- Backend test: non-permitted admin cannot GET or POST tokenization.
- Backend test: POST rejects duplicate/in-progress tokenization for the same asset.
- Backend test: successful tokenization writes asset chain metadata and audit log atomically.
- Backend test: missing clone-address event fails safely and does not persist the factory address as clone address.
- Frontend/admin E2E: tokenization POST includes a valid CSRF header.
- Frontend/admin E2E: generic `/admin/asset-tokenize` route shows an asset picker or redirects, rather than a dead error.
- Frontend unit/smoke: dynamic result rendering escapes chain metadata and error strings.

---

## Recommended Fix Order

1. Fix CSRF token retrieval so the button can call the backend in a real admin browser.
2. Add explicit tokenization permissions and gate GET/POST with those permissions.
3. Make tokenization idempotent/race-safe before the external chain call.
4. Persist chain metadata and audit log durably, with reconciliation for chain-success/DB-failure.
5. Remove the factory-address fallback when clone parsing fails.
6. Replace generic dead navigation with a tokenizable asset selector or remove generic links.
7. Replace dynamic `innerHTML` for API data with safe DOM rendering.

---

## Final Status

`needs_recheck`

Reason: The page is implemented enough to load and run static checks, but core tokenization is blocked by CSRF wiring and has high-risk backend authorization, idempotency, and audit durability gaps that must be fixed and verified before production use.
