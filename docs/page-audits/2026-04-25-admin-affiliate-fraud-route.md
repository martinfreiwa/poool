# Page Audit: Admin Affiliate Fraud Route

Date: 2026-04-25
Status: fixed_needs_browser_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/affiliate-fraud`
Route Alias: `/admin/affiliate-fraud.html`
Expected Template: `frontend/platform/admin/affiliate-fraud.html`
Existing Related Template: `frontend/platform/admin/admin-affiliate-fraud.html`
JavaScript: inline script in existing related template
CSS: `frontend/platform/static/css/fonts.css`, `frontend/platform/static/css/admin.css`, inline page CSS
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/rewards.rs`, `backend/src/rewards/service.rs`

---

## Summary

2026-04-28 fix pass: `/admin/affiliate-fraud` now has a clean template entrypoint, and the admin sidebar points to `/admin/affiliate-fraud.html` while keeping the legacy `/admin/admin-affiliate-fraud.html` route available.

The fraud visualizer contract is now aligned: the backend requires `affiliates.manage`, validates the scan type, returns both `flags` and Cytoscape `elements`, implements IP-overlap scanning from referral click IP clusters, writes an audit row for scan access, and the frontend renders loading/error/empty/results states without browser alerts. The unsupported "Freeze Node" danger control was removed.

---

## Tested Scope

- Static review of the selected tracker entry, route registration, generic admin page renderer, existing related template, sidebar link, affiliate fraud API, fraud scan service, and affiliate migrations.
- Runtime unauthenticated smoke checks against the already-running local server on `localhost:8888`.
- Inline JavaScript syntax check after extracting the script body from the existing related template.
- 2026-04-28 fix verification added static regression coverage, inline JavaScript parsing, and isolated Rust compile checking. Authenticated seeded browser graph coverage remains recommended.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| Selected URL | `/admin/affiliate-fraud` | Registered in `backend/src/admin/mod.rs`, protected by `AdminUser`. |
| Selected alias | `/admin/affiliate-fraud.html` | Registered and resolves to `frontend/platform/admin/affiliate-fraud.html`. |
| Existing alternate URL | `/admin/admin-affiliate-fraud` | Registered and resolves to the checked-in template. |
| Existing alternate template | `frontend/platform/admin/admin-affiliate-fraud.html` | Fraud visualizer UI and inline script. |
| Sidebar link | `/admin/affiliate-fraud.html` | Admin navigation points to the selected clean route. |
| Page renderer | `backend/src/admin/pages.rs` | Appends `.html` to clean admin URLs and loads that exact MiniJinja template path. |
| Backend API | `GET /api/admin/rewards/affiliates/fraud-scan` | Requires `affiliates.manage`, supports `type=circular` and `type=ip_overlap`, returns `flags` and `elements`. |
| Backend service | `scan_affiliate_fraud_rings()`, `scan_affiliate_ip_overlaps()` | Queries circular active-affiliate referral pairs and shared referral-click IP clusters. |
| Database tables | `affiliates`, `affiliate_referrals`, `users` | Source data for circular referral scan. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Result |
|--------|---------------------|-------------------|-----------------|----------------|--------|
| Page heading | `.fraud-header h1` | Identify fraud visualizer page. | Static HTML | Page renders through clean template. | Fixed. |
| Description text | `.fraud-header p` | Explain circular rings and IP overlap scan. | Static HTML | Backend implements both modes. | Fixed. |
| Scan Circular Rings button | `[data-scan-type="circular"]` | Fetch circular referral graph and render nodes/edges. | Event listener. | API returns `elements`. | Fixed. |
| Scan IP Overlaps button | `[data-scan-type="ip_overlap"]` | Fetch IP-overlap graph and render nodes/edges. | Event listener. | API returns IP-overlap flags/elements. | Fixed. |
| Freeze Node button | N/A | No unsupported danger action shown. | Removed. | N/A | Fixed. |
| Graph canvas | `#cy` | Render Cytoscape graph when library is available. | Initialized on DOMContentLoaded. | API response includes graph elements. | Fixed. |
| Empty/error/status state | `#fraud-status`, `#fraud-results` | Show loading, errors, empty state, and textual findings. | Safe DOM rendering. | API status/errors reflected. | Fixed. |

---

## Frontend Findings

### P1 - Selected clean route resolves to a missing template

Location:

- Tracker: `docs/page-review-tracker.yml`
- Backend: `backend/src/admin/mod.rs:207-210`
- Renderer: `backend/src/admin/pages.rs:93-102`
- Existing template: `frontend/platform/admin/admin-affiliate-fraud.html`

Problem:

`page_admin_generic` turns `/admin/affiliate-fraud` into `admin/affiliate-fraud.html`, but that file is absent. The sidebar links to `/admin/admin-affiliate-fraud.html`, so admins can reach the alternate page while the cleaner registered route remains a post-auth 404.

Expected:

Use one canonical URL and template name. Either add the expected template/redirect for `/admin/affiliate-fraud`, or remove the duplicate route and tracker entry if `/admin/admin-affiliate-fraud` is intentionally canonical.

2026-04-28 status: fixed. `frontend/platform/admin/affiliate-fraud.html` now exists and includes the maintained fraud visualizer template, and the sidebar points to `/admin/affiliate-fraud.html`.

### P1 - Fraud graph API contract does not match the UI

Location:

- Template: `frontend/platform/admin/admin-affiliate-fraud.html:132-144`
- API: `backend/src/admin/rewards.rs:1329-1348`
- Service: `backend/src/rewards/service.rs:1554-1588`

Problem:

The UI expects `data.elements` suitable for `window.cy.add(data.elements)`. The API returns `{ success, flags, count }`, and each flag is a plain object with affiliate IDs/emails and description. If real rings exist, the UI still falls into the "No fraud patterns detected" branch because `data.elements` is absent.

Expected:

Either convert API flags into Cytoscape node/edge elements in the frontend, or return a documented `elements` array from the backend. The empty state should distinguish "zero findings" from "unexpected response shape".

2026-04-28 status: fixed. The API returns `elements`, the page renders textual findings, and empty/error states are no longer browser alerts.

### P2 - IP-overlap scan is visible but not implemented

Location:

- Template: `frontend/platform/admin/admin-affiliate-fraud.html:68`
- API: `backend/src/admin/rewards.rs:1329-1348`
- Service: `backend/src/rewards/service.rs:1552-1554`

Problem:

The page advertises IP-overlap scanning and calls `buildGraph('ip_overlap')`, but the backend handler has no query extractor and the service only queries circular referral rings. The service comment mentions same-IP clusters, but no IP-overlap SQL is implemented.

Expected:

Either implement an IP-overlap scan using explicit, privacy-reviewed data sources and least-sensitive response fields, or remove/disable the button until supported.

2026-04-28 status: fixed. `type=ip_overlap` now uses `referral_clicks` joined to active affiliates by referral code and reports shared-IP clusters.

### P2 - Freeze Node button is dead UI

Location:

- Template: `frontend/platform/admin/admin-affiliate-fraud.html:69`
- Backend routes: `backend/src/admin/mod.rs:477-517`

Problem:

"Freeze Node" is a visible danger action with no click handler, no selected-node state, no confirmation, and no matching freeze endpoint. It suggests an enforcement action that cannot be performed.

Expected:

Disable or remove the button until there is a real reviewed workflow. If implemented, it should require `affiliates.manage`, CSRF, confirmation, audit logging, and an explicit state transition such as suspending an affiliate or freezing specific commissions.

2026-04-28 status: fixed. The unsupported danger control was removed.

---

## Backend Findings

### P1 - Fraud scan API lacks fine-grained affiliate permission

Location:

- API: `backend/src/admin/rewards.rs:1329-1332`
- Permission migration: `database/079_affiliate_manage_permission.sql`

Problem:

Neighboring affiliate admin endpoints require `admin.require_permission(&state.db, "affiliates.manage")`, but the fraud scan endpoint only extracts `AdminUser`. This exposes affiliate fraud graph data to any active admin/super_admin role, regardless of whether the role has affiliate-management permission.

Expected:

Require `affiliates.manage` or a narrower fraud/compliance permission before returning affiliate fraud findings.

2026-04-28 status: fixed. The endpoint now requires `affiliates.manage`.

### P2 - Fraud scan returns personal emails without an explicit minimization layer

Location:

- Service: `backend/src/rewards/service.rs:1557-1583`

Problem:

The scan response includes full affiliate emails. That may be operationally useful, but the endpoint has no fine-grained permission and no response minimization or audit event for viewing fraud-sensitive affiliate relationships.

Expected:

Return only the fields needed for graph rendering by default, gate expanded identity details behind a deliberate admin action, and audit access to fraud-sensitive affiliate identity data.

2026-04-28 status: partially addressed. Fine-grained permission and audit logging were added. The graph still uses affiliate emails as labels for operator usefulness; a future privacy-minimization pass can replace those labels with masked identifiers if desired.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated selected page | `curl -i http://localhost:8888/admin/affiliate-fraud` | Request rejected without admin session. | `401 Unauthorized` with safe security headers. | Pass |
| Unauthenticated alternate page | `curl -i http://localhost:8888/admin/admin-affiliate-fraud` | Request rejected without admin session. | `401 Unauthorized` with safe security headers. | Pass |
| Unauthenticated fraud API | `curl -i 'http://localhost:8888/api/admin/rewards/affiliates/fraud-scan?type=circular'` | Request rejected without admin session. | `401 Unauthorized` with safe security headers. | Pass |
| Inline JS syntax | `node --check <(sed -n '78,149p' frontend/platform/admin/admin-affiliate-fraud.html)` | No syntax errors. | Exit 0. | Pass |
| Authenticated selected route render | Login as admin and open `/admin/affiliate-fraud`. | Page renders or canonical redirect occurs. | Not run; static route/template review shows likely post-auth 404. | Not run |
| Authenticated graph render | Trigger circular and IP scans with controlled findings. | Graph nodes/edges render and empty states are accurate. | Not run; static API contract mismatch found. | Not run |
| Static regression | `python3 -m pytest tests/admin/test_affiliate_route_contract_static.py -q` | Affiliate fraud route/API contract assertions pass. | 16 passed. | Pass |
| Inline fraud script syntax | Extract script from `frontend/platform/admin/admin-affiliate-fraud.html` and run `node --check`. | No syntax errors. | Exit 0. | Pass |
| Rust compile check | `cd backend && CARGO_TARGET_DIR=/tmp/poool-affiliate-fraud-check cargo check -q` | Compile succeeds. | Exit 0. | Pass |

---

## Security Findings

- P1: Fraud scan API should require `affiliates.manage` or a narrower compliance/fraud permission.
- P2: Fraud scan response includes affiliate emails without response minimization or audit logging.
- Page/API are at least protected by session-based `AdminUser`; unauthenticated smoke checks returned `401`.
- No state-changing fraud action is currently wired, so CSRF was not applicable to this page’s existing scan fetches.

---

## Accessibility and UX Findings

- P2: Graph-only results have no accessible textual list/table fallback for screen readers or keyboard-only review.
- P2: `alert()` is used for empty/error states, which is disruptive and not integrated with the admin design system.
- P2: No loading state while a scan is running.
- P2: No selected-node affordance or disabled state for the "Freeze Node" action.

---

## Database Findings

- Circular scan uses `affiliates`, `affiliate_referrals`, and `users`, all backed by migrations.
- `affiliate_referrals` has indexes for `referred_user_id` and `(affiliate_id, status)`, but the circular-ring self-join may still need EXPLAIN verification against production-sized referral graphs.
- No financial mutation occurs in the existing scan flow.

---

## Recommended Fix Order

1. Canonicalize the route/template: make `/admin/affiliate-fraud` render a real template or redirect to `/admin/admin-affiliate-fraud`.
2. Align the fraud scan API contract with Cytoscape rendering, including separate circular/IP scan modes or disabled unsupported controls.
3. Add `affiliates.manage` or a narrower permission to the fraud scan endpoint.
4. Remove or implement "Freeze Node" with confirmation, audit logging, CSRF, and a clear backend state machine.
5. Add an authenticated E2E fixture for route render, empty scan, circular-ring scan, and authorization failure.

## 2026-04-28 Fix Pass Final State

- Fixed: canonical route/template.
- Fixed: sidebar canonical URL.
- Fixed: API permission gate.
- Fixed: graph response contract.
- Fixed: IP-overlap scan.
- Fixed: dead Freeze Node control.
- Fixed: accessible status/results regions replacing alerts.
- Remaining recommended coverage: authenticated seeded browser graph E2E.
