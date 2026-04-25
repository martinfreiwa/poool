# Page Audit: Affiliate compliance route

Date: 2026-04-25
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/admin/affiliate-compliance`
Template: `frontend/platform/admin/affiliate-compliance.html` (missing)
JavaScript: none registered for this tracker entry
CSS: none registered for this tracker entry
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`

---

## Summary

`/admin/affiliate-compliance` is a registered admin route, but it has no matching template under `frontend/platform/admin/`. The generic admin page handler resolves the clean URL to `admin/affiliate-compliance.html`, so an authenticated admin would receive the missing-template 404 response instead of a functional page.

Unauthenticated access is protected by `AdminUser` and returned `401 Authentication required` in local runtime smoke testing. The page cannot be meaningfully UI-tested until either the route is removed/redirected or the intended affiliate compliance template is implemented.

Follow-up fix on 2026-04-25: product scope was clarified. Affiliate onboarding/compliance is user-facing through `/affiliate/onboarding` and `/affiliate/dashboard`; admins only approve applicants from `/admin/affiliate-applications`. The legacy `/admin/affiliate-compliance` and `/admin/affiliate-compliance.html` routes now redirect to `/admin/affiliate-applications` instead of attempting to render a missing admin template.

Runtime recheck on 2026-04-25 confirmed an authenticated admin request to `/admin/affiliate-compliance` returns `303 See Other` to `/admin/affiliate-applications`. A pending affiliate session can load `/affiliate/dashboard`, and `/api/affiliate/dashboard` returns `is_affiliate=true` with `status=pending_approval`.

---

## Tested Scope

- Reviewed the page tracker entry for `admin.affiliate-compliance-route`.
- Inspected admin route registration for `/admin/affiliate-compliance` and `/admin/affiliate-compliance.html`.
- Inspected `page_admin_generic` route-to-template resolution.
- Verified no matching `frontend/platform/admin/affiliate-compliance.html` template exists.
- Searched for related affiliate compliance frontend files and navigation references.
- Started the local backend and performed unauthenticated route smoke testing.
- Ran JavaScript syntax check for the closest existing compliance script, `mp-compliance.js`, to confirm it is unrelated and syntactically valid.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/affiliate-compliance` | Registered admin clean URL. |
| URL alias | `/admin/affiliate-compliance.html` | Registered admin `.html` URL. |
| Template | `frontend/platform/admin/affiliate-compliance.html` | Missing. |
| Similar template | `frontend/platform/admin/marketplace/compliance.html` | Exists, but belongs to marketplace compliance, not affiliate compliance. |
| Backend page route | `GET /admin/affiliate-compliance` | Registered in `backend/src/admin/mod.rs`. |
| Backend handler | `page_admin_generic` | Converts clean admin URL to `admin/affiliate-compliance.html`. |
| JavaScript | none | No page-specific JS is registered for this page. |
| CSS | none | No page-specific CSS is registered for this page. |
| Database table | not applicable | No page backend/API behavior could be reached because the page template is missing. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Page shell | `frontend/platform/admin/affiliate-compliance.html` | Render affiliate compliance/admin review UI. | Broken: template missing. | Partially: route exists. | Authenticated users would hit missing-template 404. |
| Clean URL route | `/admin/affiliate-compliance` | Serve the page or redirect intentionally. | Not applicable. | Yes, registered. | Unauthenticated request returned 401. Authenticated render target is missing. |
| `.html` URL route | `/admin/affiliate-compliance.html` | Serve the page or redirect intentionally. | Not applicable. | Yes, registered. | Static/code review shows same missing-template target. |
| Affiliate compliance navigation | admin sidebar / frontend references | Link admins to this page if implemented. | No active frontend navigation found. | Route exists. | Dead/orphan route. |

---

## Frontend Findings

### P1 - Registered affiliate compliance page has no template

Location:

- Template: `frontend/platform/admin/affiliate-compliance.html` missing
- Backend: `backend/src/admin/mod.rs:211`
- Backend: `backend/src/admin/pages.rs:84`

Problem:

The admin router registers `/admin/affiliate-compliance` and `/admin/affiliate-compliance.html`, but the generic admin page handler resolves those routes to `admin/affiliate-compliance.html`. No matching template exists in `frontend/platform/admin/`.

Expected:

Either implement `frontend/platform/admin/affiliate-compliance.html` with its JS/CSS/API wiring, or remove/redirect the route if affiliate compliance has been folded into another page.

Evidence:

- `find frontend/platform/admin -path '*affiliate-compliance*'` returned no files.
- `backend/src/admin/mod.rs` registers both affiliate compliance routes.
- `page_admin_generic` appends `.html` for clean admin URLs.

Recommended fix:

Decide whether affiliate compliance should be a dedicated page. If yes, add the template, navigation, JS/API route map, RBAC expectations, empty/error states, and tests. If no, delete the route and tracker entry or redirect it to the canonical page.

---

## Backend Findings

### P3 - Missing admin templates return debug details in the HTML body

Location:

- Backend: `backend/src/admin/pages.rs:120`

Problem:

The generic admin template error branch returns a 404 body containing the attempted template path and MiniJinja error details. This is not exposed to unauthenticated users because `AdminUser` runs first, but it is still unnecessary implementation detail in an admin-facing production response.

Expected:

Authenticated admins should see a generic not-found page or redirect. Detailed template errors should remain in server logs only.

Evidence:

`render_admin_template` formats: `Debug info: Tried file ..., minijinja error: ...` in the user-facing 404 HTML response.

Recommended fix:

Return a generic admin 404 body and keep the full file/error detail in `tracing::error!`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Template existence | Checked for `frontend/platform/admin/affiliate-compliance.html`. | Template exists for registered page route. | No file found. | Fail |
| Backend route mapping | Inspected `backend/src/admin/mod.rs` and `backend/src/admin/pages.rs`. | Route maps to an existing template or intentional redirect. | Route maps to missing `admin/affiliate-compliance.html`. | Fail |
| Unauthenticated route smoke | `curl -i http://localhost:8888/admin/affiliate-compliance` | Admin page protected from anonymous users. | `401 Unauthorized` with `{"error":"Authentication required"}`. | Pass |
| JS syntax smoke | `node --check frontend/platform/static/js/mp-compliance.js` | Syntax valid. | Passed; script belongs to marketplace compliance, not this page. | Pass |
| Authenticated redirect smoke | Request `/admin/affiliate-compliance` with a local admin session. | Legacy route redirects to `/admin/affiliate-applications`. | `303 See Other` with `location: /admin/affiliate-applications`. | Pass |
| Pending affiliate dashboard state | Request `/affiliate/dashboard` and `/api/affiliate/dashboard` with a local pending affiliate session. | Page loads and API reports pending approval instead of redirecting away. | Page returned `200 OK`; API returned `is_affiliate=true`, `status=pending_approval`. | Pass |

---

## Security Findings

- Authentication gate is present: unauthenticated local request returned `401 Unauthorized`.
- No page-specific authorization can be verified beyond the generic `AdminUser` gate because the page does not render.
- Missing-template debug output should be removed from user-facing admin responses.
- No state-changing controls, forms, uploads, financial operations, or database mutations are present for this missing page.

---

## Database Findings

No database dependency could be verified for this route. Because the page template and page-specific JS/API flow are absent, there are no visible database-backed controls to audit.

---

## Missing Tests

- Add a route inventory test that every registered `page_admin_generic` HTML route has a corresponding frontend template, unless explicitly allowlisted as a redirect/deprecated route.
- Add an authenticated admin route smoke test for `/admin/affiliate-compliance` after the route is either implemented or redirected.
- Add a regression test that missing admin templates do not expose debug details in user-facing HTML.

---

## Recommended Fix Order

1. Decide whether `/admin/affiliate-compliance` is a real product surface or a stale route.
2. If real, implement the missing template and connect it to concrete affiliate compliance APIs and navigation.
3. If stale, remove the route or redirect to the canonical affiliate/admin compliance destination.
4. Replace user-facing missing-template debug output with a generic admin 404 response.
5. Add route-to-template coverage so this class of dead admin route is caught automatically.

---

## Final Status

`fixed`

Reason: The stale admin route now redirects to the canonical admin affiliate applications page, and the pending affiliate dashboard state was verified with an authenticated local session.
