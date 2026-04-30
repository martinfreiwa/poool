# Page Audit: Affiliate Materials

Date: 2026-04-26
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/affiliate/materials`
Template: `frontend/platform/affiliate-materials.html`
JavaScript: `frontend/platform/static/js/affiliate-materials.js` (referenced, missing)
CSS: `frontend/platform/static/css/affiliate-dashboard.css`, `frontend/platform/static/css/leaderboard.css`, `frontend/platform/static/css/cards-template.css`, `frontend/platform/static/css/forms-template.css`
Backend Routes: `backend/src/rewards/mod.rs`, `backend/src/rewards/routes.rs`

---

## Summary

The affiliate materials page is protected by login, but it is not production-ready. The page references a missing JavaScript file, so every visible download button is dead UI. The legal guideline download points at a URL that returns the platform HTML fallback with `content-type: text/html`, not a PDF. The backend has a custom material upload API and database table, but the page exposes no upload/status workflow, and the upload endpoint lacks strong file-type validation. The page should be rechecked after fixes.

---

## Tested Scope

- Reviewed `frontend/platform/affiliate-materials.html`.
- Reviewed shared head/topbar/sidebar behavior relevant to this page.
- Reviewed `backend/src/rewards/mod.rs` route registration.
- Reviewed `backend/src/rewards/routes.rs` page and material upload handlers.
- Reviewed `database/076_affiliate_system_gaps.sql` for `affiliate_materials`.
- Started the backend with `cd backend && cargo run`.
- Probed unauthenticated page access and static asset responses with `curl`.
- Checked for page-specific tests and references with `rg`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/affiliate/materials` | Protected HTML page route |
| Template | `frontend/platform/affiliate-materials.html` | Static asset library and download controls |
| Component | `frontend/platform/components/head.html` | Loads `extra_js=['affiliate-materials']` |
| Component | `frontend/platform/components/investor-topbar.html` | Adds `Payout Settings` link |
| JS | `frontend/platform/static/js/affiliate-materials.js` | Referenced by template, missing on disk and 404s |
| CSS | `frontend/platform/static/css/affiliate-dashboard.css` | Existing shared affiliate dashboard CSS |
| Backend page route | `GET /affiliate/materials` | Registered in `backend/src/rewards/mod.rs`, handled by `page_affiliate_materials` |
| Backend API route | `POST /api/affiliate/materials/upload` | Registered, but no UI calls it |
| Database table | `affiliate_materials` | Created by `database/076_affiliate_system_gaps.sql` |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Payout Settings link | `a[href="/affiliate/settings"]` in topbar | Navigate to affiliate settings | Yes, normal link | Yes, protected page route exists | Static route map verified |
| Brand guidelines PDF link | `a[href="/docs/POOOL-Affiliate-Brand-Guidelines.pdf"]` | Download compliance PDF | Link only | No dedicated static docs mount | Runtime returned `200 text/html`, not PDF |
| Download All button | `button` near Social Media Assets header | Download approved assets ZIP | No handler, no link | No matching endpoint found | Dead UI |
| Banner Download button | First asset card button | Download 1200x628 PNG | No handler, no link | No matching endpoint found | Dead UI |
| Instagram Download button | Second asset card button | Download 1080x1080 PNG | No handler, no link | No matching endpoint found | Dead UI |
| Story Download Video button | Third asset card button | Download MP4 | No handler, no link | No matching endpoint found | Dead UI |
| Logo Download SVG button | Light logo card button | Download SVG logo | No handler, no link | Static logo asset exists | Dead UI despite source logo existing |
| Dark Logo Download SVG button | Dark logo card button | Download dark SVG logo | No handler, no link | Static logo asset exists | Dead UI despite source logo existing |
| Logo preview images | `/static/images/icons/logo-pool.svg`, `/static/images/logos/Logo%20Pool.svg` | Render previews | Yes, static images | Static assets exist | Runtime static probes returned 200 |
| FOUC guard script | Inline DOMContentLoaded handler | Reveal body | Yes | Not applicable | Static review only |

---

## Frontend Findings

### P1 - Material Download Buttons Are Dead UI

Location:

- Template: `frontend/platform/affiliate-materials.html`
- JS: `frontend/platform/static/js/affiliate-materials.js`

Problem:

The page includes `extra_js=['affiliate-materials']`, which makes `head.html` load `/static/js/affiliate-materials.js`, but that file does not exist. The visible download controls are plain `<button>` elements with no `type`, `data-*` attributes, click handlers, form action, or wrapped links.

Expected:

Each download control should either be an `<a download>` pointing to a real approved asset, or a button handled by an existing JS controller that calls a real download endpoint and shows loading/error states.

Evidence:

- `ls frontend/platform/static/js/affiliate-materials.js` failed.
- Runtime probe returned `affiliate-materials.js 404`.
- `rg` found no handlers for `Download All`, `Download Video`, `Download SVG`, or page-specific material downloads.

Recommended fix:

Create the missing page controller or replace the buttons with real download links. Add visible error/loading states and a regression test that fails on missing page-specific script references.

### P1 - Brand Guidelines Download Returns HTML, Not A PDF

Location:

- Template: `frontend/platform/affiliate-materials.html`

Problem:

The legal/compliance link points to `/docs/POOOL-Affiliate-Brand-Guidelines.pdf`, but no matching file exists in the repo. Runtime returned `200 OK` with `content-type: text/html` and an HTML body, which means the user downloads the platform fallback document as a `.pdf`.

Expected:

The guideline URL should serve the real current PDF with a PDF content type, or the link should be removed until the approved document is available.

Evidence:

- `find . -name 'POOOL-Affiliate-Brand-Guidelines.pdf'` found no file.
- `curl -I http://localhost:8888/docs/POOOL-Affiliate-Brand-Guidelines.pdf` returned `content-type: text/html`.

Recommended fix:

Place the approved PDF under a served static path, update the URL, and add an automated check for the expected content type.

### P2 - Custom Material Upload Workflow Is Not Exposed

Location:

- Template: `frontend/platform/affiliate-materials.html`
- Backend: `backend/src/rewards/routes.rs`

Problem:

The backend registers `POST /api/affiliate/materials/upload` and the database has `affiliate_materials`, but the page has no file input, upload button, review status list, empty state, or admin-review status feedback.

Expected:

If custom material review is part of this page, active affiliates should be able to upload a file, see pending/approved/rejected submissions, and see validation or storage errors. If custom uploads are intentionally out of scope, the orphan API should be documented or moved to the page that owns it.

Evidence:

- `backend/src/rewards/mod.rs` registers `/api/affiliate/materials/upload`.
- `database/076_affiliate_system_gaps.sql` creates `affiliate_materials`.
- `frontend/platform/affiliate-materials.html` contains no upload control.

Recommended fix:

Add a small upload/review-status section for active affiliates or remove the misleading orphaned page/API coupling from this page.

### P3 - Logo Images Lack Alternative Text

Location:

- Template: `frontend/platform/affiliate-materials.html`

Problem:

The two logo preview images under "Logos & Brand Elements" omit `alt` attributes.

Expected:

Decorative previews should use `alt=""`; meaningful previews should use concise text such as `POOOL logo preview`.

Evidence:

- Static template review of the two `<img src="/static/images/logos/Logo%20Pool.svg">` elements.

Recommended fix:

Add appropriate `alt` attributes when wiring the logo downloads.

---

## Backend Findings

### P1 - Affiliate Materials Page Is Not Gated To Active Affiliates

Location:

- Backend: `backend/src/rewards/routes.rs`
- Helper: `backend/src/common/routes_helper.rs`

Problem:

`page_affiliate_materials` calls `serve_protected`, which only requires a valid session. It fetches `affiliate_status` for rendering but does not deny `unregistered`, `pending_approval`, `suspended`, or `terminated` users. The upload API correctly requires `status = 'active'`, so the page and API have mismatched authorization.

Expected:

Approved marketing materials and affiliate-only controls should be visible only to active affiliates, or the page should render a locked state matching `/affiliate/dashboard`.

Evidence:

- `serve_protected` redirects unauthenticated users but returns the template for any authenticated user.
- `api_affiliate_upload_material` explicitly checks `affiliates.status = 'active'`.
- The onboarding copy says affiliates must use current approved materials from the affiliate dashboard.

Recommended fix:

Use an affiliate-specific page guard or render a locked state for non-active affiliates. Add an authenticated non-active affiliate test.

### P1 - Material Upload Accepts Any File Type

Location:

- Backend: `backend/src/rewards/routes.rs`

Problem:

`api_affiliate_upload_material` only checks a 20 MB size limit and stores all files as `application/octet-stream`. It does not validate extension, detected MIME type, file signature, or allowed creative formats before placing private objects into storage for admin review.

Expected:

Restrict uploads to approved formats such as PNG, JPG, SVG, PDF, MP4, or ZIP as product/legal decide; validate both extension and detected content; store the correct content type; and return clear validation errors.

Evidence:

- Static review of `api_affiliate_upload_material`.
- Runtime unauthenticated POST was blocked by CSRF before auth, confirming route exists behind middleware.

Recommended fix:

Add server-side file validation and tests for rejected executable/HTML/polyglot files, oversize files, and accepted creative formats.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page request | `curl -I http://localhost:8888/affiliate/materials` | Redirect to login | `303 See Other`, `location: /auth/login` | Pass |
| Missing page script | `curl /static/js/affiliate-materials.js` | 200 JS or no reference | `404` | Fail |
| Guideline PDF download | `curl -I /docs/POOOL-Affiliate-Brand-Guidelines.pdf` | PDF content type | `200 OK`, `content-type: text/html` | Fail |
| Static preview assets | Probe icon and logo SVGs | 200 | Both returned 200 | Pass |
| Upload API unauthenticated/CSRF guard | `POST /api/affiliate/materials/upload` without CSRF | Safe rejection | `403` CSRF error JSON | Pass |

---

## Security Findings

- P1: Authenticated but non-active users can reach the marketing materials page because `serve_protected` does not enforce affiliate status.
- P1: The custom material upload route lacks content-type and file-signature validation.
- P2: If the upload UI is added later, it must not trust client-side filename or MIME claims, and admin previews/downloads must remain private until approved.

---

## Database Findings

- `affiliate_materials` exists with status constraints and indexes in `database/076_affiliate_system_gaps.sql`.
- No page-visible workflow reads this table for the affiliate who uploaded materials.
- No audit-log row is written when an affiliate uploads a material; only admin review has surrounding admin code paths.

---

## Missing Tests

- Add a route/render test proving `/affiliate/materials` denies or locks non-active affiliates.
- Add a static/template test that fails when `extra_js` points to a missing file.
- Add an HTTP/content-type test for the brand guidelines PDF URL.
- Add upload validation tests for accepted formats, rejected formats, oversize files, and GCS-disabled behavior.
- Add a browser test for each material download button once wired.

---

## Recommended Fix Order

1. Replace dead download buttons with real approved asset links or implement `affiliate-materials.js`.
2. Serve a real brand guidelines PDF from a valid static path with correct content type.
3. Gate the page to active affiliates or render a locked non-active affiliate state.
4. Add upload/status UI only if custom material review belongs on this page.
5. Harden material upload validation and add regression tests.

---

## Final Status

`needs_recheck`

Reason: The page was audited and documented, but core download controls are unwired, the compliance PDF URL is invalid, affiliate-status authorization is too broad, and the related upload endpoint needs validation before this page can be considered production-ready.
