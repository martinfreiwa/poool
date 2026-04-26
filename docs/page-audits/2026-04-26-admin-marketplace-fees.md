# Page Audit: Marketplace Fees

Date: 2026-04-26
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/fees`
Template: `frontend/platform/admin/marketplace/fees.html`
JavaScript: `frontend/platform/static/js/mp-fees.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`, `backend/src/marketplace/validation.rs`

---

## Summary

`/admin/marketplace/fees` is only partially implemented. The page route exists, the tables exist, and `GET /api/admin/marketplace/fees` can list fee configurations/promotions, but the visible fee-management workflow is not end-to-end reliable. Several controls show success or remove UI locally without persisting anything, the fee APIs are not restricted to `marketplace.manage`, database failures are hidden as empty fee state, and parts of the stored fee model are not actually consumed by the marketplace fee resolver.

Final status is `needs_recheck`.

---

## Tested Scope

- Reviewed `frontend/platform/admin/marketplace/fees.html`.
- Reviewed `frontend/platform/static/js/mp-fees.js` and shared `mp-toast.js`.
- Reviewed admin route registration in `backend/src/admin/mod.rs`.
- Reviewed page authorization in `backend/src/admin/pages.rs`.
- Reviewed fee API handlers in `backend/src/admin/marketplace.rs`.
- Reviewed marketplace fee consumption in `backend/src/marketplace/validation.rs`.
- Verified schema/table support in `database/053_fee_configuration.sql` and local PostgreSQL.
- Checked for existing automated tests referencing this page/API.

Runtime browser testing was blocked because no backend was running on `localhost:8888`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/fees` | Registered through `page_admin_generic`. |
| URL alias | `/admin/marketplace/fees.html` | Registered. |
| Template | `frontend/platform/admin/marketplace/fees.html` | Static shell plus JS-populated tables/cards. |
| JS | `frontend/platform/static/js/mp-fees.js` | Owns tabs, API loads, mock fallback, local-only actions. |
| JS | `frontend/platform/static/js/mp-toast.js` | Toast/button utility; simulates delayed success. |
| Backend page route | `GET /admin/marketplace/fees` | Registered in `backend/src/admin/mod.rs`; page guard allows any marketplace view/manage/compliance permission. |
| Backend API route | `GET /api/admin/marketplace/fees` | Lists configurations/promotions; no fine-grained permission check; DB errors hidden. |
| Backend API route | `POST /api/admin/marketplace/fees` | Creates fee configuration; no `marketplace.manage` check; no audit log. |
| Database table | `fee_configurations` | Supports platform, asset, and developer scopes. |
| Database table | `fee_promotions` | Supports global and asset promotions. |
| Database table | `audit_logs` | Exists, but fee changes are not written to it. |
| Fee resolver | `backend/src/marketplace/validation.rs` | Uses active promotion, asset fee, platform fee, then hardcoded fallback; ignores developer scope. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `fees.html:31-35` | Navigate to admin and marketplace index. | Link navigation only. | Page routes exist. | Static review only. |
| Platform Defaults tab | `.admin-tab[data-tab="defaults"]` | Show default fee form. | Yes, local tab switch. | No backend dependency. | Static pass. |
| Tier Discounts tab | `.admin-tab[data-tab="tier-discounts"]` | Show tier fee table. | Yes, local tab switch. | Loads `/api/admin/rewards`, not fee engine. | Static pass; data meaning questionable. |
| Asset-Specific tab | `.admin-tab[data-tab="asset-specific"]` | Show asset overrides. | Yes, local tab switch. | `GET /api/admin/marketplace/fees` list only. | Static pass; no persistent action path. |
| Promotions tab | `.admin-tab[data-tab="promotions"]` | Show active promotions. | Yes, local tab switch. | `GET /api/admin/marketplace/fees` list only. | Static pass; no persistent action path. |
| Taker Fee input | `#fee-taker` | Edit platform default taker fee. | Value read by no code. | POST supports bps config, but this form does not call it. | Dead input for persistence. |
| Maker Fee input | `#fee-maker` | Edit platform default maker fee. | Value read by no code. | POST supports bps config, but this form does not call it. | Dead input for persistence. |
| Settlement Fee input | `#fee-settlement` | Edit settlement fee. | Value read by no code. | No fee API field or resolver support found. | Dead input. |
| Min Fee input | `#fee-min` | Edit minimum fee. | Value read by no code. | No database/API support found. | Dead input. |
| Save Default Fees | `#btn-save-defaults` | Validate and persist default fees. | Shows delayed success toast only. | Not called. | Broken; false success. |
| Tier fee rows | `#tier-fees-body` | Edit tier discounts. | Renders inputs with `innerHTML`. | Uses rewards tiers, not marketplace fees. | Partially rendered; not persisted. |
| Save Tier Configuration | `#btn-save-tiers` | Persist tier discount config. | No listener found. | No fee API endpoint. | Dead button. |
| Add Override | `#btn-add-override` | Open/create asset fee override form. | Shows info toast: "connect to backend". | POST endpoint exists but not used. | Dead workflow. |
| Asset fee rows | `#asset-fees-body` | Show active configs and allow removal/editing. | Renders configs with `innerHTML`; remove only appears in mock mode. | No DELETE/PATCH route. | Read-only for real data; fake remove in mock mode. |
| Remove fee override | `.btn-remove-fee` | Deactivate/remove override. | DOM-only removal in mock mode. | No API call. | False success when mock data is active. |
| Promotion cards | `#promos-grid` | Show active/expired promotions. | Renders promos with `innerHTML`. | GET only. | Static list only. |
| Deactivate promotion | `.btn-deactivate-promo` | Persist promotion deactivation. | DOM-only removal and success toast. | No API call or route. | Broken; false success. |
| Toasts | `window.mpToast` | User feedback. | Uses text nodes for messages. | N/A. | Static pass. |

---

## Frontend Findings

### P1 - Fee controls show success without persistence

Location:

- Template: `frontend/platform/admin/marketplace/fees.html:63-87`, `frontend/platform/admin/marketplace/fees.html:116-126`
- JS: `frontend/platform/static/js/mp-fees.js:237-245`, `frontend/platform/static/js/mp-fees.js:90-102`, `frontend/platform/static/js/mp-fees.js:148-163`

Problem:

`Save Default Fees` shows "Default fees saved successfully" but never reads input values or calls the backend. `Save Tier Configuration` has no listener. `Add Override` only displays "Override form — connect to backend". Mock-mode remove/deactivate buttons remove DOM nodes and show success without persistence.

Expected:

Fee-changing controls should validate input, convert percentages to basis points, call authenticated CSRF-protected backend routes, wait for success, and reload the authoritative fee state.

Evidence:

Static JS review found no `fetch()` call for save-defaults, tier save, override creation, removal, or promotion deactivation. Only initial `GET` requests are made.

Recommended fix:

Wire default-fee save to `POST` or `PATCH /api/admin/marketplace/fees`, add explicit update/deactivate endpoints where needed, and replace simulated `mpButtonAction` success with response-aware loading/error/success states.

### P2 - Fee and promotion rendering interpolates backend data into raw HTML

Location:

- JS: `frontend/platform/static/js/mp-fees.js:53-84`, `frontend/platform/static/js/mp-fees.js:110-145`, `frontend/platform/static/js/mp-fees.js:178-198`

Problem:

Fee reasons, promotion names, dates, tier names, and badge colors are interpolated into `innerHTML`. These values come from database-backed admin APIs, so compromised or malformed stored values can execute or break markup in an admin session.

Expected:

Render rows/cards with DOM construction and `textContent`; validate color values before using them in style attributes.

Evidence:

`renderAssetFees`, `renderPromotions`, and `renderTiers` all build HTML strings with unescaped values.

Recommended fix:

Replace dynamic `innerHTML` rendering with `document.createElement()` helpers and a strict color sanitizer for badge swatches.

### P2 - Tier-discount UI is not tied to the marketplace fee engine

Location:

- Template: `frontend/platform/admin/marketplace/fees.html:92-118`
- JS: `frontend/platform/static/js/mp-fees.js:176-208`

Problem:

The page labels rewards tier cashback as "Fee Discount" and calculates an "Effective Taker Fee" locally from a hardcoded 5.00% default. The marketplace resolver does not consume tier discounts, and the save button is not wired.

Expected:

Either remove this tab until a tier-based marketplace-fee contract exists, or add a real fee-tier model/API and update the resolver to apply it deterministically.

Evidence:

`loadTiers()` fetches `/api/admin/rewards`; `resolve_fees()` only checks promotions, asset configs, platform defaults, then hardcoded fallback.

Recommended fix:

Define whether reward cashback is separate from marketplace fee discounts. If it is a fee discount, model it in fee-specific tables and add resolver/tests.

### P3 - Tab UI lacks selected-state semantics

Location:

- Template: `frontend/platform/admin/marketplace/fees.html:48-53`
- JS: `frontend/platform/static/js/mp-fees.js:34-45`

Problem:

The tab buttons visually toggle `.active`, but they do not expose `role="tab"`, `aria-selected`, `aria-controls`, or keyboard arrow behavior.

Expected:

Tabs should be accessible with clear selected state and keyboard navigation, or be treated as ordinary buttons with `aria-expanded`/`aria-controls`.

Evidence:

Static template/JS review found no ARIA state updates.

Recommended fix:

Add tablist semantics and update ARIA attributes during `initTabs()`.

---

## Backend Findings

### P1 - Fee management routes do not enforce `marketplace.manage`

Location:

- Page guard: `backend/src/admin/pages.rs:171-190`
- API handlers: `backend/src/admin/marketplace.rs:1455-1499`
- Route registration: `backend/src/admin/mod.rs:271-272`, `backend/src/admin/mod.rs:751-753`

Problem:

The fee page is a fee-management surface, but `page_admin_generic` allows any admin with `marketplace.view`, `marketplace.manage`, or `marketplace.compliance` to render generic marketplace pages except approvals. `GET /api/admin/marketplace/fees` accepts `AdminUser` and performs no permission check. `POST /api/admin/marketplace/fees` also accepts any `AdminUser` and does not require `marketplace.manage`.

Expected:

The page and all mutating fee APIs should require `marketplace.manage`; read-only fee listing should at minimum require `marketplace.view` and should not expose mutation affordances to viewers.

Evidence:

`admin.require_permission(...)` is absent from both fee API handlers. The sidebar expects `nav-mp-fees` to require `marketplace.manage`, but backend enforcement is broader.

Recommended fix:

Add a dedicated `page_admin_marketplace_fees` or page-specific branch requiring `marketplace.manage`; add `admin.require_permission(&state.db, "marketplace.manage").await?` to `POST` and either `marketplace.view` or `marketplace.manage` to `GET` based on product intent.

### P1 - Fee list API hides database failures as empty fee state

Location:

- Backend: `backend/src/admin/marketplace.rs:1462-1474`

Problem:

Both `fee_configurations` and `fee_promotions` queries use `.unwrap_or_default()`. If either table/query fails, admins receive an empty successful response and may think all fee configs/promotions are absent.

Expected:

Fee configuration load failures should return an error and the UI should show a retryable failure state.

Evidence:

Static backend review shows database errors are swallowed instead of propagated.

Recommended fix:

Replace `.unwrap_or_default()` with `.map_err(ApiError::Database)?` and add frontend retry/error rendering.

### P1 - Fee configuration model can create ambiguous active platform configs

Location:

- Backend: `backend/src/admin/marketplace.rs:1502-1527`
- Database: `database/053_fee_configuration.sql:6-23`
- Resolver: `backend/src/marketplace/validation.rs:467-485`

Problem:

`POST /api/admin/marketplace/fees` validates the scope string and basis-point range, but it does not validate required/forbidden IDs by scope. Because `asset_id` and `developer_id` are nullable, the unique constraint can allow multiple active platform rows in PostgreSQL. `resolve_fees()` then selects the first active platform fee with `LIMIT 1` and no deterministic `ORDER BY`.

Expected:

Platform configs should require both IDs null and allow only one active row. Asset configs should require `asset_id`; developer configs should require `developer_id`. The resolver should have deterministic ordering and tests for duplicate/active-state behavior.

Evidence:

The table uses nullable columns in the unique key, and the create handler does not enforce scope/reference compatibility before insert.

Recommended fix:

Add server-side validation plus partial unique indexes such as one active platform default, one active asset override per asset, and one active developer deal per developer.

### P1 - Developer fee scope is accepted but ignored by fee resolution

Location:

- Backend create handler: `backend/src/admin/marketplace.rs:1502-1527`
- Resolver: `backend/src/marketplace/validation.rs:410-493`

Problem:

The API and table support `scope = 'developer'`, but `resolve_fees()` checks active promotions, asset-specific fees, platform default, and fallback only. Developer deals can be created but will not affect trade fees.

Expected:

Either remove developer scope from the fee-management API until implemented, or resolve developer-specific fees using the asset's developer/issuer relationship before asset/platform fallback.

Evidence:

The resolver comment says "Developer Deal" is tier 2, but no developer query exists.

Recommended fix:

Join or query the asset developer and apply active developer config in the documented hierarchy; add regression tests for promotion > developer > asset > platform precedence.

### P2 - Fee mutations are not audit logged

Location:

- Backend: `backend/src/admin/marketplace.rs:1514-1532`
- Audit table: `database/001_initial_schema.sql` / `docs/DATABASE_SCHEMA.md`

Problem:

Creating a fee configuration changes financial behavior, but the handler only logs through `tracing::info!`; it does not insert an immutable `audit_logs` row.

Expected:

Fee create/update/deactivate operations should write audit log rows with actor, entity, previous/new state, and reason.

Evidence:

No `audit_logs` insert appears in the fee create handler, while other sensitive marketplace decisions insert audit rows.

Recommended fix:

Wrap fee mutations and audit writes in a transaction, then commit atomically.

### P2 - Backend/API does not cover visible settlement fee and minimum fee fields

Location:

- Template: `frontend/platform/admin/marketplace/fees.html:76-83`
- Database/API: `database/053_fee_configuration.sql:6-23`, `backend/src/admin/marketplace.rs:1482-1491`

Problem:

The UI exposes `Settlement Fee (%)` and `Min Fee (USD)`, but `fee_configurations` and `CreateFeeConfigRequest` only model taker/maker basis points. The resolver also returns only taker/maker rates.

Expected:

Remove these fields or add explicit database columns, API request/response fields, validation, fee calculation logic, and tests.

Evidence:

Static schema and handler review found no `settlement_fee_bps` or `min_fee_cents` support in fee management.

Recommended fix:

Decide whether settlement/minimum fees are product requirements. If yes, model them in cents/bps and apply them in `calculate_trade_fee`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/mp-fees.js && node --check frontend/platform/static/js/mp-toast.js` | Both scripts parse. | Passed with no output. | Pass |
| Local schema support | `psql -d poool -Atc "SELECT to_regclass(...)"` | `fee_configurations`, `fee_promotions`, `audit_logs`, `tiers` exist. | All four tables resolved. | Pass |
| Page load smoke | `curl -I --max-time 3 http://localhost:8888/admin/marketplace/fees` | Auth redirect or page response. | Failed to connect; backend not running. | Blocked |
| API smoke | `curl --max-time 3 http://localhost:8888/api/admin/marketplace/fees` | Auth error or JSON response. | Failed to connect; backend not running. | Blocked |
| Existing test coverage search | `rg` across tests/backend/frontend for fee page/API | Targeted tests exist. | No targeted page/API E2E found. | Missing |

---

## Security and Data Integrity Notes

- Fee settings directly affect financial outcomes; they must be protected by `marketplace.manage`, audited, and tested.
- Fee percentages are represented as integer basis points in the marketplace model, which is correct. The UI uses percentages and must convert to BPS server-side or with server validation.
- Stored fee/promotion/tier rendering should avoid raw `innerHTML` because it runs in privileged admin sessions.
- Silent DB fallback to empty lists is unsafe for a financial configuration surface.
- The resolver should match the documented fee hierarchy and be deterministic when multiple active rows exist.

---

## Recommended Fix Order

1. Enforce `marketplace.manage` on the fee page and mutating fee APIs; define whether GET is `marketplace.view` or `marketplace.manage`.
2. Stop swallowing fee/promotion DB errors and add frontend error/retry states.
3. Replace fake save/deactivate/remove UI with real response-aware API calls, or disable/remove controls until implemented.
4. Add scope/reference validation and partial unique indexes for active fee configurations.
5. Implement or remove developer, settlement, minimum-fee, and tier-discount concepts.
6. Add audit logs and targeted tests for fee creation, update/deactivation, authz, and resolver precedence.

---

## Severity Counts

| Severity | Count |
|----------|-------|
| P0 / Critical | 0 |
| P1 / High | 4 |
| P2 / Medium | 4 |
| P3 / Low | 1 |

---

## Final Status

`needs_recheck` after fixes. The page is not safe to treat as production-ready fee management because several controls do not persist, permission boundaries are too broad, and the backend resolver does not fully honor the documented fee hierarchy.
