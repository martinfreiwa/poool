# Page Audit: Blockchain Contract Detail

Date: 2026-04-26
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/admin/blockchain-contract-detail`
Template: `frontend/platform/admin/blockchain-contract-detail.html`
JavaScript: `frontend/platform/static/js/admin-blockchain-contract-detail.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page styles
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/blockchain.rs`

---

## Summary

The admin page shell, route registration, read API, and pause/unpause API routes exist, and the database has the on-chain metadata/cache tables needed by the page. A follow-up fix on 2026-04-26 resolved the production-readiness findings for this page: clone controls now require dedicated blockchain control authorization, pause state is persisted and rendered from control state, holder reads fail visibly instead of silently, dynamic holder rows are DOM-rendered, manual netting is explicitly unavailable, and authenticated E2E coverage verifies read and mocked mutation flows.

Final status is `fixed`.

## Follow-up Fix Verification

Fixed issues: 9 total (`4 high`, `3 medium`, `2 low`).

Verification covered:

- `blockchain.manage` control permission and normal-admin denial.
- CSRF denial for pause mutations.
- Strict `0x` address validation and mapped-asset enforcement.
- Persisted paused/live state through `chain_contract_controls`.
- Mandatory transactional audit logging for pause/unpause.
- Safe holder DOM rendering with validated explorer links.
- Disabled manual netting state with explanatory copy.
- Authenticated browser flow with modal confirmation and mocked chain mutation.

---

## Tested Scope

- Static review of `frontend/platform/admin/blockchain-contract-detail.html`
- Static review of `frontend/platform/static/js/admin-blockchain-contract-detail.js`
- Backend route registration in `backend/src/admin/mod.rs`
- Backend clone detail and pause/unpause handlers in `backend/src/admin/blockchain.rs`
- Admin auth/permission extractor behavior in `backend/src/admin/extractors.rs`
- CSRF middleware and admin fetch-interceptor behavior in `backend/src/auth/csrf.rs` and `frontend/platform/static/js/admin-permission-guard.js`
- Schema support in `database/058_blockchain_integration.sql` and `database/059_onchain_balances.sql`
- Test and source inventory search for blockchain route coverage

This report was originally produced as an audit-only pass. The follow-up fix modified application, test, migration, and documentation files listed in the tracker.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/blockchain-contract-detail?address=<0x...>` | Generic admin page route; requires `address` query parameter for useful rendering. |
| Alias | `/admin/blockchain-contract-detail.html?address=<0x...>` | Also registered and used by the contracts list. |
| Template | `frontend/platform/admin/blockchain-contract-detail.html` | Admin shell with breadcrumb, contract header, KPI cards, holders table, and danger-zone controls. |
| JS | `frontend/platform/static/js/admin-blockchain-contract-detail.js` | Loads clone detail, renders holders, wires pause/unpause. |
| Backend page route | `GET /admin/blockchain-contract-detail` | `page_admin_generic` in `backend/src/admin/mod.rs`. |
| Backend API route | `GET /api/admin/blockchain/contracts/:address/detail` | Reads mapped asset and holder cache. |
| Backend API route | `POST /api/admin/blockchain/contracts/:address/pause` | Calls `cast send pause()` against the supplied address. |
| Backend API route | `POST /api/admin/blockchain/contracts/:address/unpause` | Calls `cast send unpause()` against the supplied address. |
| Database table | `assets` | Supplies chain contract address, title, and token supply. |
| Database table | `onchain_balances` | Supplies cached holder balances. |
| Database table | `users` | Supplies holder emails and chain wallet addresses. |
| Database table | `audit_logs` | Intended audit record for pause/unpause, currently best-effort only. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `frontend/platform/admin/blockchain-contract-detail.html:53` | Navigate Admin > Live Contracts > current detail. | Yes | Page routes exist | Works statically. |
| Page title | `#page-asset-title` | Show contract asset title or error state. | Yes, `textContent` | GET detail API | Wired. |
| Clone address display | `#clone-address` | Show query address and allow copy. | Yes | Query param only | Copy has no success/error feedback. |
| Copy address button | `.copy-btn` inline handler | Copy address to clipboard. | Inline `onclick` | Not required | Works only when clipboard API is available; no fallback or announcement. |
| Contract explorer link | `#contract-link` | Open contract on explorer. | Yes | Query param only | Hardcoded to Amoy PolygonScan and lacks `rel="noopener"`. |
| Live status badge | `#kpi-live-status` | Show paused/live state. | Partially | GET detail API | Backend always returns `is_paused: false`, so paused contracts display as live. |
| Total supply KPI | `#kpi-supply` | Show total token supply. | Yes | GET detail API | Wired. |
| Tokens distributed KPI | `#kpi-sold`, `#kpi-sold-bar` | Show sold tokens and percentage. | Partially | GET detail API | Uses `innerHTML` for the value and can overrun width if DB values are inconsistent. |
| Holders table | `#holders-tbody` | Render top 100 on-chain holders. | Yes | GET detail API | Uses backend data but renders rows with raw HTML interpolation. |
| Refresh Sync button | inline `window.location.reload()` | Refresh current page. | Yes | GET detail API after reload | Reload-only; does not trigger indexer sync and has no loading state. |
| Freeze/Unfreeze button | `#btn-freeze-transfers` | Confirm and pause/unpause this clone contract. | Yes | POST pause/unpause APIs | Backend accepts broad admin access and the button state is wrong when the contract is already paused. |
| Manual On-Chain Netting button | second danger-zone button | Trigger manual settlement/netting for this contract. | No | No page-specific backend call | Dead UI: visible button has no handler, no disabled state, and no explanatory copy. |
| Missing-address error state | no `address` query param | Tell operator the URL is invalid. | Yes | Not required | Works, but leaves danger-zone button visible and unwired. |
| API failure state | catch block | Show load failure. | Partially | GET API | Error message is injected through `innerHTML`. |

---

## Findings

### P1 - Clone pause/unpause lacks fine-grained permission checks

Locations:

- Backend: `backend/src/admin/blockchain.rs:1176`, `backend/src/admin/blockchain.rs:1251`
- Admin extractor: `backend/src/admin/extractors.rs:120`

The pause and unpause handlers only require `AdminUser`, which accepts any active `admin` or `super_admin` role. These endpoints can halt or resume transfers for an isolated asset contract and should require a dedicated high-risk permission such as `blockchain.manage`, `blockchain.pause`, or a two-person approval flow.

Expected: enforce explicit blockchain-control permission before calling `cast send`, and add an authenticated permission-denial test.

### P1 - Pause state is hardcoded to live

Location:

- Backend: `backend/src/admin/blockchain.rs:308`

The detail API always returns `is_paused: false` instead of reading chain or persisted state. A paused clone is shown as live, and the primary control remains a freeze action instead of unfreeze. Operators can send the wrong transaction or fail to notice a frozen asset.

Expected: read paused state from RPC or a trusted indexed/persisted state, and clearly mark unknown state as unavailable rather than live.

### P1 - Pause/unpause audit logging is best-effort only

Locations:

- Backend: `backend/src/admin/blockchain.rs:1220`
- Backend: `backend/src/admin/blockchain.rs:1295`

Both mutation handlers execute the chain transaction first, then insert an audit log with `.ok()`. Audit insert failure is silently ignored, leaving no durable operator record for a sensitive blockchain control action.

Expected: persist a mandatory audit/control record, surface audit persistence failures, and record chain-success/database-failure reconciliation state if the external transaction succeeds but local persistence fails.

### P1 - Holder rows render backend data through raw HTML

Location:

- JS: `frontend/platform/static/js/admin-blockchain-contract-detail.js:92`

The holders table builds one large `innerHTML` string with `holder.wallet_address`, `holder.email`, `holder.balance`, and `holder.last_synced_at`. Those values come from the database and should not be interpolated into HTML attributes or text. A malformed wallet/email value can break attributes or create admin-side XSS.

Expected: build rows with DOM APIs and `textContent`, validate `0x` addresses before using them in URLs, and avoid inline `onclick` strings for copy actions.

### P2 - Holder database failures are hidden as an empty holder list

Location:

- Backend: `backend/src/admin/blockchain.rs:274`

The holder query uses `.unwrap_or_default()`. If `onchain_balances`, `users.chain_wallet_address`, or the DB connection fails, the API still returns success with no holders. This hides sync/schema/runtime failures and makes the page look healthy while the cache is broken.

Expected: return a safe error status or explicit degraded state when holder-cache reads fail.

### P2 - Contract address is not validated before backend chain execution

Locations:

- JS: `frontend/platform/static/js/admin-blockchain-contract-detail.js:5`
- Backend: `backend/src/admin/blockchain.rs:1179`, `backend/src/admin/blockchain.rs:1254`

The page accepts any `address` string from the URL, and pause/unpause pass the route value directly to `cast send`. The detail GET confirms whether an asset is mapped to that address, but the mutation handlers do not re-check that the address belongs to a tokenized asset before sending the chain transaction.

Expected: validate `0x` + 40 hex format and require an existing `assets.chain_contract_address` row before pause/unpause.

### P2 - Manual On-Chain Netting is visible dead UI

Location:

- Template: `frontend/platform/admin/blockchain-contract-detail.html:156`

The danger zone includes a `Trigger Manual On-Chain Netting` button with no id, handler, disabled attribute, explanatory state, or backend route mapping from this page. A critical admin control appears available but does nothing.

Expected: either wire it to a real contract-scoped settlement endpoint with confirmation and audit logging, or render it disabled/hidden until implemented.

### P3 - External explorer links lack `rel="noopener"`

Locations:

- Template: `frontend/platform/admin/blockchain-contract-detail.html:82`
- JS: `frontend/platform/static/js/admin-blockchain-contract-detail.js:105`

The contract and holder explorer links open a new tab with `target="_blank"` but no `rel="noopener noreferrer"`.

Expected: add `rel="noopener noreferrer"` to all external new-tab links.

### P3 - Confirmation, alerts, and copy controls do not meet dashboard accessibility patterns

Locations:

- Template: `frontend/platform/admin/blockchain-contract-detail.html:74`
- JS: `frontend/platform/static/js/admin-blockchain-contract-detail.js:26`
- JS: `frontend/platform/static/js/admin-blockchain-contract-detail.js:42`

The page uses native `confirm()` and `alert()` for a critical mutation, inline copy buttons have icon-only names via title only, and clipboard success/failure is not announced. This is functional baseline behavior but below the modal, focus, and status-region standards expected for admin tools.

Expected: use the platform confirmation/toast pattern, provide accessible labels, manage focus, and expose success/error states without blocking browser dialogs.

---

## Commands Run

```bash
node --check frontend/platform/static/js/admin-blockchain-contract-detail.js
psql -d poool -Atc "SELECT to_regclass('public.onchain_balances'), to_regclass('public.chain_indexer_cursor'), EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='assets' AND column_name='chain_contract_address'), EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='users' AND column_name='chain_wallet_address');"
curl -I --max-time 3 'http://localhost:8888/admin/blockchain-contract-detail?address=0x0000000000000000000000000000000000000000'
```

Results:

- JS syntax check passed.
- Database schema check returned `onchain_balances|chain_indexer_cursor|t|t`.
- Curl smoke could not connect because no backend server was running on `localhost:8888`.

---

## Follow-up Commands Run

```bash
psql -d poool -f database/085_admin_blockchain_contract_controls.sql
node --check frontend/platform/static/js/admin-blockchain-contract-detail.js
python3 -m py_compile tests/e2e/test_admin_blockchain_contract_detail.py
cd backend && cargo fmt --check
cd backend && cargo check
BASE_URL=http://localhost:8899 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_blockchain_contract_detail.py -q
rg -n "innerHTML|onclick=|confirm\\(|alert\\(" frontend/platform/static/js/admin-blockchain-contract-detail.js frontend/platform/admin/blockchain-contract-detail.html || true
```

Results: all commands passed. The targeted authenticated E2E passed with `CHAIN_CONTROL_MOCK=true` on a local backend at `localhost:8899`.
