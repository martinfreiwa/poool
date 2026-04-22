---
description: "Enterprise-grade pre-launch SOP for auditing individual web pages to 100% production readiness — covers backend, frontend, security, performance, UX, and SEO."
---

# 🛡️ Page Audit SOP — Production Readiness Checklist

> **Purpose:** A repeatable, step-by-step Standard Operating Procedure for auditing any individual web page in the POOOL platform before it goes live. Every page MUST pass all applicable items before it is cleared for production.
>
> **Audience:** Dev · QA · Product
>
> **How to use:** Copy this checklist into a per-page audit ticket. Replace `[PAGE]` with the page under review (e.g., `/wallet`, `/admin/users`). Mark each item `[x]` when verified. If an item is intentionally not applicable, mark it `[N/A]` with a brief justification.

---

## Quick Reference — Audit Phases

| # | Phase | Owner | Est. Time |
|---|-------|-------|-----------|
| 0 | Claiming & State Management | Agent | 5 min |
| 1 | Code Quality & Architecture | Dev / Agent | 30–45 min |
| 2 | Backend & Database Integrity | Dev / Agent | 45–60 min |
| 3 | Security & Authorization | Dev / Agent | 30–45 min |
| 4 | Feature & Bug Testing | QA / Agent | 60–90 min |
| 5 | Frontend Performance & Assets | Dev / Agent | 30 min |
| 6 | UX, Polishing & Accessibility | QA / Agent | 45–60 min |
| 7 | SEO, Social & Analytics | PM / Agent | 20–30 min |

---

## Phase 0 — Claiming & State Management (MULTI-AGENT ONLY)

> **Goal:** Before starting any work, you MUST claim the page to prevent duplicate efforts.
>
> **Instructions:**
> 1. Read `agent/workflows/ACTIVE_AUDITS.md`.
> 2. Select a page that is marked as "Pending" or add a new row for your page.
> 3. If a page is already "In Progress" by another agent, YOU MUST STOP and pick a different page.
> 4. Modify `ACTIVE_AUDITS.md` by putting your Agent Name/ID in the "Assigned Agent ID" column, updating the "Status", "Current Phase", and "Last Updated" timestamp.
> 5. Make sure you continuously update `ACTIVE_AUDITS.md` as you progress through the phases below.

---

## Phase 1 — Code Quality & Architecture

> **Goal:** The codebase behind `[PAGE]` is clean, maintainable, and contains zero hard-coded secrets or environment-specific values.

### 1.1 No Hard-Coded Values or Credentials
- [ ] **No API keys, tokens, passwords, or secrets** exist anywhere in the page's templates, JS, or CSS files.
- [ ] **No hard-coded URLs or domains** (e.g., `http://localhost:8888`, `https://api.poool.co`). All URLs are constructed from environment variables or relative paths.
- [ ] **No hard-coded user IDs, email addresses, or test data** remain in the source (search for `TODO`, `FIXME`, `HACK`, `XXX`, `test@`, `admin@`).
- [ ] **No hard-coded currency values, prices, or financial figures.** All monetary data comes from the database or API response.
- [ ] **No commented-out dead code blocks** larger than 3 lines. Remove or convert to a tracked issue.

### 1.2 Environment Variables & Configuration
- [ ] All environment-dependent values (DB connection strings, API base URLs, feature flags, third-party keys) are read from `.env` / runtime config and **never** baked into source.
- [ ] `.env.example` or `.env.production.template` is updated if new variables were introduced for this page.
- [ ] **Feature flags** for incomplete or beta features on this page are properly gated and default to `off` in production.

### 1.3 Clean Code Standards
- [ ] **Naming conventions** are consistent (snake_case for Rust/Python, camelCase for JS, kebab-case for CSS classes).
- [ ] **No duplicated logic** — shared utilities are extracted into common modules (e.g. `format_usd` from `wallet.js` / `utils.rs`).
- [ ] **Design System adherence** — Page MUST utilize `dashboard-tokens.css` (for layout spacings, z-index, radii) and `fonts.css` (for centralized typography/colors) rather than creating new unstructured CSS variables.
- [ ] **Tech stack adherence** — Frontend logic must use **Vanilla JS** (no React/Vue) and **Vanilla CSS** (no Tailwind unless explicitly approved). Backend must use **Rust with Axum**.
- [ ] **File organization** follows POOOL project conventions:
  - HTML templates → `frontend/platform/` (or `admin/`, `developer/`, `www/`) using `minijinja` `.html` templates.
  - Page-specific CSS → `frontend/platform/static/css/`
  - Page-specific JS → `frontend/platform/static/js/`
  - Backend route handler → `backend/src/<module>/routes.rs` registering Axum router.
- [ ] **Template inheritance** is used correctly — page extends the correct base layout and does not redefine the sidebar/header inline.
- [ ] **No `console.log` statements** remain in production JS (use Sentry or a controlled logger).
- [ ] Code has been **linted and formatted** (`cargo fmt`, `prettier`, or project linter) with zero warnings.

### 1.4 Documentation
- [ ] Complex business logic on this page has inline comments explaining the *why*, not just the *what*.
- [ ] Any new API endpoints introduced are documented (at minimum: method, path, request body, response shape, error codes).

---

## Phase 2 — Backend & Database Integrity

> **Goal:** The server-side logic for `[PAGE]` is correct, resilient, and handles all edge cases without data corruption.

### 2.1 Business Logic Verification
- [ ] **Happy path** — The primary user flow works end-to-end with valid data (e.g., submit form → DB write → success response → UI update).
- [ ] **Edge cases** — Boundary values are handled:
  - Empty strings / null values
  - Maximum length inputs
  - Zero and negative numbers (especially for financial fields)
  - Unicode / special characters (emoji, RTL text)
  - Concurrent requests (double-click submit, race conditions)
- [ ] **Calculations are correct** — Verify any math (tax, fees, percentages, portfolio values) against a manual calculation with at least 3 different data sets.
- [ ] **State transitions** are valid — If the page changes entity state (e.g., order status, KYC status), verify that only legal transitions are allowed and illegal transitions return proper errors.

### 2.2 Database CRUD Operations
- [ ] **CREATE** — New records are inserted with all required fields populated, correct foreign keys, and proper defaults. Verify with a direct DB query after the operation.
- [ ] **READ** — Data displayed on the page matches the database exactly. Cross-check at least 3 records manually.
- [ ] **UPDATE** — Only the intended fields are modified. Unchanged fields remain intact. `updated_at` timestamp is refreshed.
- [ ] **DELETE** — Soft-delete is used where required (financial records, user data). Hard-delete cascades do not orphan related records.
- [ ] **No data corruption** — After performing all CRUD operations, run a data integrity check:
  - Foreign key references are valid (no dangling IDs)
  - Unique constraints are not violated
  - Numeric precision is maintained (especially `DECIMAL` / `NUMERIC` for money — never use `FLOAT`)
- [ ] **Transactions** — Multi-step operations (e.g., place order + debit wallet + create transaction record) are wrapped in a database transaction. Verify rollback on partial failure.

- [ ] **All possible error states** return appropriate HTTP status codes (using POOOL's standard `ApiError` format when applicable):
  - `400` — Bad request / validation failure
  - `401` — Unauthenticated (caught by `auth_middleware`)
  - `403` — Forbidden (authenticated but unauthorized)
  - `404` — Resource not found
  - `409` — Conflict (duplicate submission)
  - `422` — Unprocessable entity
  - `500` — Internal server error
- [ ] **Error responses** are structured JSON. The frontend JS generic fetch handlers properly parse and display these `ApiError` messages.
- [ ] **No stack traces or internal paths** are exposed in production error responses.
- [ ] **Structured logging** — Ensure the backend utilizes the `tracing` crate (`info!`, `warn!`, `error!`) for operations (CREATE, UPDATE, DELETE).
- [ ] **Sentry integration** — Unhandled exceptions are captured by the `NewSentryLayer` and frontend `Sentry.captureConsoleIntegration` is correctly configured if custom logging is present.

### 2.4 API Contract
- [ ] **Request validation** — The backend rejects requests with missing required fields, wrong types, or out-of-range values *before* touching the database.
- [ ] **Response shape** — The API response matches the documented contract. No extra fields leak (e.g., `password_hash`, internal IDs that should be opaque).
- [ ] **Pagination** — List endpoints with potentially large result sets are paginated (verify `page`, `per_page`, `total_count` in response).
- [ ] **Idempotency** — POST/PUT operations that create or modify resources handle duplicate submissions gracefully (idempotency key or upsert logic).

---

## Phase 3 — Security & Authorization

> **Goal:** `[PAGE]` is hardened against common web vulnerabilities and enforces proper access control.

### 3.1 Input Validation & Sanitization
- [ ] **All user inputs** (form fields, query params, URL path segments, file uploads) are validated on the **server side**, regardless of client-side validation. Use exact struct field types in Rust for strict deserialize validation.
- [ ] **XSS prevention** — User-supplied data rendered in HTML is auto-escaped by the template engine (verify `minijinja` auto-escape is active). Raw/unescaped output (`|safe`) is used ONLY when absolutely necessary and the source is 100% trusted.
- [ ] **SQL injection prevention** — All database queries MUST use parameterized statements via `sqlx::query!` or `sqlx::query_as!`. Do NOT manually construct SQL strings using `format!`.
- [ ] **CSRF protection** — State-changing requests (POST, PUT, DELETE) are protected by CSRF tokens or SameSite cookie policy.
- [ ] **File upload validation** (if applicable):
  - File type is validated by magic bytes, not just extension
  - File size has a server-enforced maximum
  - Uploaded files are stored outside the web root and served via a separate handler
  - Filenames are sanitized (no path traversal via `../`)

### 3.2 Authentication & Session Security
- [ ] **Unauthenticated access** — Protected pages are correctly guarded by `axum::middleware::from_fn(auth_middleware)`. Unauthenticated users hit `401` or redirect to `/auth/login`.
- [ ] **User ID Extraction** — The backend MUST extract identity using the `AuthUser` extractor struct inside Axum route handlers. Do NOT trust or accept `user_id` inside request JSON payloads for operations acting on the current user.
- [ ] **Session cookies** are correctly configured by the auth module:
  - `HttpOnly` — ✅
  - `Secure` — ✅ (in production)
  - `SameSite=Strict` or `SameSite=Lax` — ✅
  - Reasonable expiration — ✅
- [ ] **Session fixation** — A new session token is issued after login. Old tokens are invalidated.

### 3.3 Role-Based Access Control (RBAC)
- [ ] **Authorization check** — The backend verifies the user's role/permissions *before* processing the request, not just in the UI.
- [ ] **Horizontal privilege escalation** — User A cannot access/modify User B's data by manipulating IDs in the URL or request body. Verify by:
  1. Log in as User A, capture a request
  2. Replay the request with User B's resource ID
  3. Confirm the server returns `403` or `404`
- [ ] **Vertical privilege escalation** — A regular user cannot access admin-only pages or endpoints. Verify by navigating to `[PAGE]` as a non-admin user if the page is admin-only.
- [ ] **UI reflects permissions** — Buttons, links, and actions that the user is not authorized to perform are **hidden** in the UI *and* enforced on the backend.

### 3.4 Secure Headers & Transport
- [ ] **Content-Security-Policy (CSP)** header is present and restrictive (no `unsafe-eval` unless absolutely required; `unsafe-inline` for styles only if necessary).
- [ ] **X-Content-Type-Options: nosniff** — ✅
- [ ] **X-Frame-Options: DENY** or **SAMEORIGIN** — ✅
- [ ] **Referrer-Policy: strict-origin-when-cross-origin** — ✅
- [ ] **Strict-Transport-Security (HSTS)** — ✅ (with `max-age` ≥ 31536000)
- [ ] **No sensitive data in URL query parameters** (tokens, passwords, PII). Use POST body or headers instead.

---

## Phase 4 — Feature & Bug Testing

> **Goal:** Every interactive element on `[PAGE]` works correctly, all design requirements are met, and there are zero errors in the browser console.

### 4.1 Functional Testing — Interactive Elements
- [ ] **All buttons** perform their intended action (submit, cancel, delete, navigate, toggle).
- [ ] **All forms** submit successfully with valid data and display appropriate success feedback.
- [ ] **All form fields** enforce validation rules:
  - Required fields show an error when left empty
  - Email fields reject malformed addresses
  - Numeric fields reject non-numeric input
  - Password fields enforce minimum complexity requirements
  - Date fields accept only valid dates
- [ ] **All modals/dialogs** open, display correct content, and close properly (via close button, overlay click, and Escape key).
- [ ] **All dropdowns/selects** populate with correct options and submit the selected value.
- [ ] **All toggles/checkboxes** persist their state after save/refresh.
- [ ] **All links** navigate to the correct destination (no broken links, no links to `#` or `javascript:void(0)` in production).
- [ ] **All tables** display data correctly:
  - Column headers match the data
  - Sorting works (if implemented)
  - Pagination works (if implemented)
  - Empty state is handled ("No results found" message, not a blank table)
- [ ] **All search/filter functionality** returns correct results and handles empty results gracefully.
- [ ] **All file upload inputs** accept the correct file types and show upload progress/confirmation.

### 4.2 Edge Cases & Error States
- [ ] **Empty state** — What does the page look like with zero data? (No orders, no assets, no transactions). It should show a friendly empty state, not a broken layout.
- [ ] **Large data set** — Does the page handle 100+ records without performance degradation or layout breaking?
- [ ] **Long text** — Do long names, descriptions, or values truncate gracefully with ellipsis or wrap without breaking the layout?
- [ ] **Rapid actions** — Double-clicking submit buttons does not create duplicate records (button should disable after first click).
- [ ] **Network failure** — If the API call fails mid-operation, the user sees a clear error message and can retry.
- [ ] **Back button** — Pressing the browser back button does not cause unexpected behavior (duplicate submissions, stale data).
- [ ] **Refresh** — Refreshing the page after a form submission does not re-submit the form (POST-Redirect-GET pattern).

### 4.3 Console & Network Verification
- [ ] **Zero JavaScript errors** in the browser console during normal page use.
- [ ] **Zero unhandled promise rejections** in the console.
- [ ] **No failed network requests** (4xx or 5xx) in the Network tab during normal page use.
- [ ] **No mixed content warnings** (HTTP resources loaded on an HTTPS page).
- [ ] **No deprecation warnings** that indicate imminent breakage.

### 4.4 Design & Feature Completeness
- [ ] **All features** specified in the design/PRD for this page are implemented and working.
- [ ] **Visual fidelity** — The page matches the approved design mockup (spacing, colors, typography, iconography).
- [ ] **Copy/text accuracy** — All labels, headings, descriptions, and error messages are spelled correctly and use consistent terminology.
- [ ] **Currency formatting** — All monetary values display with correct currency symbol, thousand separators, and decimal precision (e.g., `$1,234.56`).
- [ ] **Date/time formatting** — All dates display in the user's expected format and timezone.
- [ ] **Number formatting** — Large numbers use appropriate separators and units.

---

## Phase 5 — Frontend Performance & Assets

> **Goal:** `[PAGE]` loads fast, all assets resolve, and Core Web Vitals are in the green.

### 5.1 Asset Integrity
- [ ] **Zero 404 errors** — All CSS, JS, images, fonts, and other assets load successfully. Check the Network tab filtered by status `4xx`.
- [ ] **All images render** — No broken image icons. Every `<img>` tag has a valid `src` and a meaningful `alt` attribute.
- [ ] **All icons render** — No missing icon glyphs (broken squares or `□` characters). Verify icon font or SVG sprite is loaded.
- [ ] **All fonts load** — Text renders in the intended font family, not a system fallback. Check the Network tab for font file requests.
- [ ] **Favicon is present** — The browser tab shows the POOOL favicon, not the default browser icon.

### 5.2 Image Optimization
- [ ] **Image formats** — Use WebP or AVIF for photographic images; SVG for icons and logos; PNG only when transparency is needed and WebP is not available.
- [ ] **Image dimensions** — Images are served at the display size (not a 4000px image scaled to 200px in CSS).
- [ ] **Lazy loading** — Off-screen images use `loading="lazy"` to defer loading until they are near the viewport.
- [ ] **Image compression** — File sizes are reasonable (hero images < 200KB, thumbnails < 50KB, icons < 10KB).

### 5.3 Core Web Vitals
- [ ] **Largest Contentful Paint (LCP)** — < 2.5 seconds. Verify with Lighthouse or Chrome DevTools Performance tab.
- [ ] **Cumulative Layout Shift (CLS)** — < 0.1. No elements jump or shift after the page appears to be loaded. Common culprits:
  - Images without `width`/`height` attributes
  - Fonts loading late and causing text reflow (FOUT)
  - Dynamic content injected above existing content
- [ ] **Interaction to Next Paint (INP)** — < 200ms. All user interactions (clicks, key presses) respond within 200ms.
- [ ] **First Contentful Paint (FCP)** — < 1.8 seconds.
- [ ] **Total page weight** — Reasonable for the page type (target: < 1MB for a standard dashboard page, < 2MB for media-heavy pages).

### 5.4 Caching & Compression
- [ ] **Static assets** are served with appropriate `Cache-Control` headers (long cache for fingerprinted assets, short for HTML).
- [ ] **Gzip or Brotli compression** is enabled for text-based assets (HTML, CSS, JS, JSON, SVG).
- [ ] **No render-blocking resources** — Critical CSS is inlined or loaded with high priority; non-critical JS is deferred or loaded async.

---

## Phase 6 — UX, Polishing & Accessibility

> **Goal:** `[PAGE]` delivers a polished, inclusive experience across all devices, browsers, and input methods.

### 6.1 Responsive Design & Cross-Device
- [ ] **Desktop (1920px, 1440px, 1280px)** — Layout is correct, no horizontal scroll, content fills the viewport appropriately.
- [ ] **Tablet (1024px, 768px)** — Layout adapts (sidebar may collapse, grid adjusts). All content is reachable.
- [ ] **Mobile (375px, 390px, 414px)** — Page is fully usable. Text is readable without zooming. Buttons are large enough to tap (minimum 44×44px touch target).
- [ ] **No horizontal overflow** — At no viewport width does content overflow horizontally or cause a horizontal scrollbar.

### 6.2 Cross-Browser Compatibility
- [ ] **Chrome (latest)** — Fully functional ✅
- [ ] **Safari (latest)** — Fully functional, especially: date inputs, flex/grid rendering, backdrop-filter ✅
- [ ] **Firefox (latest)** — Fully functional ✅
- [ ] **Edge (latest)** — Fully functional ✅
- [ ] **Mobile Safari (iOS)** — Fully functional, including: form inputs, fixed positioning, viewport units ✅
- [ ] **Chrome Android** — Fully functional ✅

### 6.3 Loading States, Flickers & Transitions
- [ ] **Initial page load** — A loading skeleton, spinner, or placeholder is shown until data is available. The page never shows "undefined", raw template syntax (`{{ variable }}`).
- [ ] **Pre-Injected Data (No Flickers)** — Core page data (balances, profile info) should be injected via `minijinja` context from the server render instead of relying solely on client-side JS fetching on load. This prevents the "sidebar rendering flicker" and layout jumping.
- [ ] **API call loading states** — When an action triggers an API call (submit, save, delete), the UI shows a loading indicator and disables the trigger element to prevent double-submission.
- [ ] **Transition between states** — State changes (e.g., tab switch, accordion expand, modal open) use smooth CSS transitions (200–300ms) aligned with POOOL's premium aesthetic requirements.
- [ ] **Error states** — When an operation fails, the error message is visible, clearly explains what went wrong, and provides a path to recovery (retry button, corrective instructions).
- [ ] **Success states** — When an operation succeeds, the user receives clear confirmation (toast notification, success message, redirect with status).
- [ ] **No flickering** — Ensure components do not momentarily appear in a default state before snapping to their data-bound state on page refresh.

### 6.4 Keyboard Navigation & Focus Management
- [ ] **Tab order** — All interactive elements (links, buttons, inputs, selects) are reachable via Tab key in a logical, visual order.
- [ ] **Focus visibility** — The currently focused element has a clearly visible focus indicator (outline or ring).
- [ ] **Escape key** — Modals and dropdowns close when Escape is pressed.
- [ ] **Enter key** — Forms submit when Enter is pressed on the last field. Buttons activate on Enter.
- [ ] **Arrow keys** — Dropdowns, tabs, and radio groups support arrow key navigation (if applicable).
- [ ] **Focus trapping** — When a modal is open, Tab cycles through the modal content only and does not escape to background elements.
- [ ] **Focus restoration** — When a modal closes, focus returns to the element that triggered it.

### 6.5 Accessibility (WCAG 2.1 AA)
- [ ] **Color contrast** — All text meets WCAG AA contrast ratios: 4.5:1 for normal text, 3:1 for large text. Verify with axe DevTools or Chrome's built-in contrast checker.
- [ ] **Alt text** — All informational images have descriptive `alt` text. Decorative images have `alt=""`.
- [ ] **ARIA labels** — Interactive elements without visible text labels have `aria-label` or `aria-labelledby` (e.g., icon-only buttons).
- [ ] **ARIA roles** — Custom widgets (tabs, accordions, dialogs) use appropriate ARIA roles (`role="tablist"`, `role="dialog"`, etc.).
- [ ] **ARIA live regions** — Dynamic content updates (toast notifications, form validation errors, live data) use `aria-live="polite"` or `aria-live="assertive"` to announce changes to screen readers.
- [ ] **Form labels** — Every form input has an associated `<label>` element (via `for`/`id` pairing) or `aria-label`.
- [ ] **Heading hierarchy** — One `<h1>` per page. Heading levels do not skip (e.g., no jump from `<h2>` to `<h4>`).
- [ ] **Semantic HTML** — Correct use of `<nav>`, `<main>`, `<header>`, `<footer>`, `<section>`, `<article>`, `<aside>` instead of generic `<div>` soup.
- [ ] **Screen reader test** — Navigate the page with VoiceOver (macOS) or NVDA (Windows). Verify all content is announced logically and interactive elements are operable.

---

## Phase 7 — SEO, Social & Analytics

> **Goal:** `[PAGE]` is discoverable by search engines, shareable on social platforms, and instrumented for analytics.

### 7.1 Meta Tags & SEO Fundamentals
- [ ] **`<title>` tag** — Present, unique, descriptive, and 50–60 characters. Format: `Page Name – POOOL`.
- [ ] **`<meta name="description">`** — Present, unique, compelling, and 120–160 characters. Accurately summarizes the page content.
- [ ] **Canonical URL** — `<link rel="canonical" href="...">` points to the authoritative URL for this page (prevents duplicate content issues).
- [ ] **Heading structure** — Single `<h1>` that includes the primary keyword for the page. Proper hierarchy with `<h2>`–`<h6>`.
- [ ] **Semantic HTML** — Content uses appropriate HTML5 elements for SEO signals (`<article>`, `<nav>`, `<main>`, etc.).
- [ ] **`robots` meta tag** — Public pages: `index, follow`. Private/authenticated pages: `noindex, nofollow`. Verify with `<meta name="robots" content="...">`.
- [ ] **Structured data** (if applicable) — JSON-LD for relevant schema types (Organization, Product, FAQ, BreadcrumbList). Validate with Google's Rich Results Test.

### 7.2 Open Graph & Social Sharing
- [ ] **`og:title`** — Present and matches (or is a variation of) the `<title>`.
- [ ] **`og:description`** — Present and matches (or is a variation of) the meta description.
- [ ] **`og:image`** — Present, points to a valid image URL (absolute), and image is at least 1200×630px for optimal display on Facebook/LinkedIn.
- [ ] **`og:url`** — Present and set to the canonical URL.
- [ ] **`og:type`** — Set to `website` (or `article` for blog posts).
- [ ] **`og:site_name`** — Set to `POOOL`.
- [ ] **Twitter card tags** — `twitter:card` set to `summary_large_image`, `twitter:title`, `twitter:description`, `twitter:image` present.
- [ ] **Social share test** — Validate the page URL with:
  - [Facebook Sharing Debugger](https://developers.facebook.com/tools/debug/)
  - [Twitter Card Validator](https://cards-dev.twitter.com/validator)
  - [LinkedIn Post Inspector](https://www.linkedin.com/post-inspector/)

### 7.3 Analytics & Event Tracking
- [ ] **Analytics script** — Google Analytics (GA4), Mixpanel, or equivalent is loaded on the page. Verify by checking `Network` tab for the analytics endpoint.
- [ ] **Page view event** — A page view event fires on page load (verify in analytics debugger / real-time view).
- [ ] **Key interaction events** — Critical user actions on this page fire tracked events:
  - Form submissions (event name, success/failure)
  - Button clicks on primary CTAs
  - Tab switches, filter selections
  - External link clicks
- [ ] **User identity** — Logged-in users are identified in the analytics platform (user ID or anonymized ID).
- [ ] **Error tracking** — Sentry (or equivalent) is initialized and captures unhandled exceptions with user context.
- [ ] **No duplicate tracking** — The analytics script loads only once per page (not duplicated in both the base template and the page template).

### 7.4 Sitemap & Robots
- [ ] **Sitemap inclusion** — If this is a public page, it is listed in `sitemap.xml` with the correct `<loc>`, `<lastmod>`, and `<changefreq>`.
- [ ] **`robots.txt`** — The page is not unintentionally blocked by a `Disallow` rule in `robots.txt`.
- [ ] **Hreflang** (if multilingual) — `<link rel="alternate" hreflang="...">` tags are present for all language variants.

---

## Sign-Off Template

Once all phases have been completed for `[PAGE]`, record the sign-off below:

```
Page:        [PAGE URL]
Audited by:  [Name / Agent ID]
Date:        [YYYY-MM-DD]
Environment: [localhost / staging / production]

Phase 1 — Code Quality & Architecture:       ✅ PASS / ❌ FAIL (notes: ...)
Phase 2 — Backend & Database Integrity:       ✅ PASS / ❌ FAIL (notes: ...)
Phase 3 — Security & Authorization:           ✅ PASS / ❌ FAIL (notes: ...)
Phase 4 — Feature & Bug Testing:              ✅ PASS / ❌ FAIL (notes: ...)
Phase 5 — Frontend Performance & Assets:      ✅ PASS / ❌ FAIL (notes: ...)
Phase 6 — UX, Polishing & Accessibility:      ✅ PASS / ❌ FAIL (notes: ...)
Phase 7 — SEO, Social & Analytics:            ✅ PASS / ❌ FAIL (notes: ...)

Overall Verdict: ✅ CLEARED FOR PRODUCTION / 🚫 BLOCKED (see notes above)
```

---

## Appendix A — Tooling Quick Reference

| Task | Tool | Command / Link |
|------|------|----------------|
| Lint HTML | W3C Validator | https://validator.w3.org/ |
| Lint CSS | Stylelint | `npx stylelint "**/*.css"` |
| Lint JS | ESLint | `npx eslint .` |
| Performance audit | Lighthouse | Chrome DevTools → Lighthouse tab |
| Accessibility audit | axe DevTools | Chrome extension |
| Contrast check | WebAIM Contrast Checker | https://webaim.org/resources/contrastchecker/ |
| Screen reader | VoiceOver (macOS) | `Cmd + F5` to toggle |
| OG tag validation | Facebook Debugger | https://developers.facebook.com/tools/debug/ |
| Security headers | securityheaders.com | https://securityheaders.com/ |
| SQL param check | grep for raw SQL | `rg 'format!.*SELECT\|format!.*INSERT\|format!.*UPDATE\|format!.*DELETE' backend/src/` |
| Dead code search | grep for markers | `rg 'TODO\|FIXME\|HACK\|XXX\|console\.log' --type-add 'web:*.{html,js,css}' -t web frontend/` |

## Appendix B — Severity Classification

| Severity | Definition | Action |
|----------|-----------|--------|
| 🔴 **P0 — Blocker** | Data loss, security vulnerability, complete feature failure | Must fix before launch. No exceptions. |
| 🟠 **P1 — Critical** | Major feature broken, significant UX issue, accessibility failure | Must fix before launch. |
| 🟡 **P2 — Major** | Minor feature broken, cosmetic issue visible to most users | Should fix before launch, may defer with PM approval. |
| 🟢 **P3 — Minor** | Edge case, minor cosmetic polish, nice-to-have improvement | May defer to post-launch sprint. |
