# Page Audit: Blog article

Date: 2026-04-28
Status: fixed_runtime_rechecked
Auditor: ChatGPT/Codex
Page URL: `/blog/:slug`
Template: `frontend/platform/blog/article.html`
JavaScript: inline handlers in `frontend/platform/blog/article.html`
CSS: `frontend/platform/static/css/blog.css`
Backend Routes: `backend/src/blog/mod.rs`, `backend/src/blog/routes.rs`, `backend/src/blog/service.rs`, `backend/src/blog/sanity.rs`

---

## Summary

The public blog article page loads published database-backed articles and returns a correct 404 for missing slugs. The 2026-04-28 fix pass remediated the audited production-readiness issues: database article HTML is sanitized before public rendering, author profile URLs are allowlisted, article metadata/JSON-LD are precomputed server-side, newsletter signup now posts to a rate-limited backend persistence endpoint, and copy/menu controls expose accessible state feedback.

## Fix Verification - 2026-04-28

All five audit findings from this report were fixed and rechecked locally.

| Issue | Status | Verification |
|-------|--------|--------------|
| PAGE-ISSUE-0470 - Database article HTML sanitization | Fixed | `cargo test -q blog::service::tests` passed sanitizer and public URL allowlist tests. |
| PAGE-ISSUE-0471 - Literal `none` SEO metadata | Fixed | Runtime curl of `/blog/real-estate-101` showed a real title/canonical/OG/Twitter title and no literal `none` metadata. |
| PAGE-ISSUE-0472 - Escaped JSON-LD values | Fixed | Runtime Node parse found two valid JSON-LD scripts with `mainEntityOfPage`/breadcrumb item as `http://localhost:8888/blog/real-estate-101`. |
| PAGE-ISSUE-0473 - Fake newsletter success | Fixed | Runtime POST to `/api/blog/newsletter` with CSRF returned `201 Created`, inserted a queued `email_logs` row, and the test row was deleted after verification. |
| PAGE-ISSUE-0474 - Copy/menu accessible state | Fixed | Static regression confirms copy live-region/failure feedback and mobile nav `aria-controls`/`aria-expanded`/Escape handling. |

Commands run:

- `rustfmt --edition 2021 --check backend/src/blog/routes.rs backend/src/blog/service.rs`
- `cd backend && cargo fmt --check`
- `CARGO_TARGET_DIR=/tmp/poool-blog-audit-target cargo check -q`
- `CARGO_TARGET_DIR=/tmp/poool-blog-audit-target cargo test -q blog::service::tests`
- `python3 -m pytest tests/test_blog_article_static.py -q`
- Runtime backend smoke with `BLOG_CONTENT_SOURCE=database SERVER_PORT=8888 CARGO_TARGET_DIR=/tmp/poool-blog-audit-target cargo run -q`
- `curl http://127.0.0.1:8888/blog/real-estate-101`
- Runtime Node JSON-LD parse of `/tmp/poool-blog-article.html`
- Runtime CSRF POST to `http://127.0.0.1:8888/api/blog/newsletter`

Remaining coverage note:

No open page-audit issues remain for `/blog/:slug`. Add committed browser E2E coverage for article render, clipboard feedback, mobile nav keyboard dismissal, newsletter success/error/rate-limit states, and malicious content fixtures to prevent regression drift.

---

## Tested Scope

- Static review of `frontend/platform/blog/article.html`, `components/blog-head.html`, `components/blog-header.html`, and `components/blog-footer.html`.
- Backend review of `page_blog_article`, content-source dispatch, database article lookup, Sanity Portable Text sanitization, and database blog schema.
- Runtime curl smoke against `http://localhost:8888/blog/real-estate-101` with `BLOG_CONTENT_SOURCE=database`.
- Runtime 404 smoke against `http://localhost:8888/blog/not-a-real-slug`.
- Headless Playwright smoke for page load, console errors, newsletter submit behavior, and mobile menu toggle.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/blog/:slug` | Public SSR article route. |
| Template | `frontend/platform/blog/article.html` | Article body, share buttons, newsletter, related content, JSON-LD. |
| Component | `frontend/platform/components/blog-head.html` | Meta, canonical, OG/Twitter tags, CSS include. |
| Component | `frontend/platform/components/blog-header.html` | Public blog nav and mobile menu button. |
| Component | `frontend/platform/components/blog-footer.html` | Footer links and social links. |
| CSS | `frontend/platform/static/css/blog.css` | Blog page, article, responsive, and newsletter styles. |
| Backend page route | `GET /blog/:slug` | Registered in `backend/src/blog/mod.rs`. |
| Backend handler | `page_blog_article` | Loads article, recent articles, related articles, then renders template. |
| Public API route | `GET /api/blog/articles/:slug` | JSON article endpoint using the same content-source helper. |
| Content source | `BLOG_CONTENT_SOURCE=sanity|database` | Defaults to Sanity; database source used for local runtime smoke. |
| Database tables | `blog_articles`, `blog_authors`, `blog_categories` | Schema in `database/024_blog.sql`; social columns in `database/031_blog_authors_social_columns.sql`. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Blog link | `frontend/platform/blog/article.html:19` | Navigate to `/blog`. | Link | Yes, `GET /blog`. | Static verified. |
| Breadcrumb category link | `article.html:21` | Navigate to category page. | Link | Yes, `GET /blog/category/:slug`. | Static verified. |
| Header category nav | `components/blog-header.html:17-22` | Navigate to blog category pages. | Link | Yes. | Static verified. |
| Language links | `components/blog-header.html:26-27` | Navigate to `/id` and `/en`. | Link | Main app routes. | Static verified. |
| Sign in link | `components/blog-header.html:29` | Navigate to `/auth/login`. | Link | Yes. | Static verified. |
| Mobile menu button | `components/blog-header.html:33` | Open/close mobile blog nav. | Inline `onclick`. | No backend needed. | Playwright: nav received `open` class. |
| Copy link button | `article.html:53`, `333-340` | Copy current URL and show copied feedback. | Inline function. | No backend needed. | Static verified; no failure path. |
| Social share links | `article.html:56-69` | Open external share URLs in new tab. | Links | No backend needed. | Static/render verified. |
| Cover image | `article.html:76-79` | Render article cover if present. | Template | Content source. | Static verified. |
| Article body | `article.html:83-84` | Render sanitized article HTML. | Template `safe`. | Sanity path sanitizes; database path does not. | Needs fix. |
| Tag links | `article.html:88-92` | Filter blog index by tag. | Link | `GET /blog?tag=...`. | Static verified. |
| Author social links | `article.html:115-142` | Open author social URLs. | Links | Content source. | Static verified; DB path lacks URL allowlisting here. |
| Newsletter form | `article.html:160-171`, `343-361` | Subscribe email or show real error. | Inline function. | No endpoint called. | Playwright: success shown with zero requests. |
| More Articles cards | `article.html:188-205` | Navigate to other articles. | Links | Backend fetches recent articles. | Static verified. |
| Sidebar CTA | `article.html:213-217` | Navigate to signup. | Link | Yes, `/auth/signup`. | Static verified. |
| Related article cards | `article.html:228-257` | Navigate to related articles. | Links | Backend fetches recent articles. | Static verified. |
| Bottom CTA | `article.html:261-267` | Navigate to signup. | Link | Yes. | Static verified. |
| JSON-LD scripts | `article.html:274-327` | Emit valid structured data. | Template | Content source. | Parses, but values contain HTML entities and metadata can be `none`. |

---

## Frontend Findings

### P2 - SEO metadata renders literal `none`

Location:

- Template: `frontend/platform/components/blog-head.html:9-18`
- Runtime output: `/tmp/blog_article.html:9-18`

Problem:

MiniJinja `default(...)` is not treating nullable article fields as missing. For `real-estate-101`, the rendered page contained `<title>none</title>`, `<meta name="description" content="none" />`, OG/Twitter title values of `none`, and `<link rel="canonical" href="none" />`.

Expected:

When `meta_title`, `meta_description`, or `canonical_url` is null, the template should fall back to article title, excerpt, and `base_url ~ request_path`.

Evidence:

`curl` page smoke returned HTTP 200, and `rg` against the saved HTML showed `title`, description, OG title, Twitter title, and canonical all rendered as `none`.

Recommended fix:

Use explicit null/empty checks or a helper context that precomputes `effective_meta_title`, `effective_meta_description`, `effective_canonical_url`, and `effective_og_image_url` server-side before rendering.

### P2 - JSON-LD is HTML-escaped instead of JSON-encoded

Location:

- Template: `frontend/platform/blog/article.html:274-327`

Problem:

JSON-LD interpolates strings with normal HTML escaping rather than JSON encoding. The runtime `mainEntityOfPage` parsed as `http:&#x2f;&#x2f;localhost:8888/blog/real-estate-101`; titles and descriptions with punctuation also contain HTML entities.

Expected:

Structured data scripts should use JSON serialization, not HTML-escaped interpolation, so URLs and text values are machine-readable exactly as intended.

Evidence:

The Node JSON parse smoke found two JSON-LD scripts and parsed the BlogPosting `mainEntityOfPage` as `http:&#x2f;&#x2f;localhost:8888/blog/real-estate-101`.

Recommended fix:

Build the schema objects in Rust and pass them through a JSON-safe template value, or apply a proper `tojson` equivalent for every interpolated JSON-LD string.

### P2 - Newsletter form shows fake success without backend persistence

Location:

- Template/form: `frontend/platform/blog/article.html:160-171`
- Inline JS: `frontend/platform/blog/article.html:343-361`

Problem:

`blogNewsletterSubmit` prevents submit, disables the button, waits one second, hides the input, and displays success. It never calls a backend endpoint and ignores the email variable after reading it.

Expected:

Newsletter signup should call a real endpoint or external provider integration, handle duplicate/invalid/provider failures, and only show success when persistence or provider subscription succeeds.

Evidence:

Headless Playwright submitted `audit@example.com`; `#newsletter-success` became visible and no network request occurred after the click.

Recommended fix:

Add a real POST endpoint or disable/remove the form until newsletter infrastructure exists. Include rate limiting, server-side email validation, CSRF if same-origin POST, and visible error/success states.

### P3 - Copy link feedback has no failure or accessible status path

Location:

- Template/button: `frontend/platform/blog/article.html:53`
- Inline JS: `frontend/platform/blog/article.html:333-340`

Problem:

`navigator.clipboard.writeText` only handles success. If clipboard permission is denied or the API is unavailable, the user receives no feedback. The success state only swaps an icon and colors, with no `aria-live` text.

Expected:

Copy should provide success and failure feedback that screen readers can detect, and should fall back to selecting the URL if Clipboard API is unavailable.

Evidence:

Static review of the inline function found no `.catch`, no fallback, and no live region.

Recommended fix:

Add a nearby polite live region, set explicit copied/error text, and handle rejected clipboard promises.

### P3 - Mobile menu state is not exposed to assistive technology

Location:

- Template: `frontend/platform/components/blog-header.html:33`

Problem:

The mobile menu toggles a CSS class through inline JavaScript but does not update `aria-expanded`, does not identify the controlled nav with `aria-controls`, and does not provide Escape/outside-click close behavior.

Expected:

The button should expose expanded/collapsed state, target the nav by ID, and support predictable keyboard dismissal.

Evidence:

Static review found only `aria-label="Menu"` and inline class toggling. Playwright verified the visual `open` class toggles.

Recommended fix:

Move the menu behavior into a small self-hosted script that updates `aria-expanded`, uses `aria-controls`, and closes on Escape.

---

## Backend Findings

### P1 - Database-backed article HTML can be rendered as trusted public markup

Location:

- Template: `frontend/platform/blog/article.html:83-84`
- Database content path: `backend/src/blog/service.rs:194-200`
- Sanity content path: `backend/src/blog/sanity.rs:1723-1793`

Problem:

The template renders `article.content_html | default(article.content) | safe`. The Sanity Portable Text path converts content through `portable_text_to_safe_html` and Ammonia, but the database path uses stored `content_html` as-is or converts Markdown to HTML without a sanitization pass before the template marks it safe.

Expected:

Every content source that reaches `article.content_html` should be sanitized with the same allowlist before public rendering. Raw HTML in Markdown should be escaped or stripped unless explicitly allowed and sanitized.

Evidence:

Static review shows the Sanity path sanitizes with Ammonia while `service::get_article_by_slug` does not. Runtime used `BLOG_CONTENT_SOURCE=database`, so the public page depends on the unsanitized database path locally.

Recommended fix:

Centralize article-body sanitization in the model/service layer and apply it to database `content_html`, Markdown-rendered fallback HTML, and any future content source. Add a regression test with script, event-handler, `javascript:` link, and iframe payloads.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Published article load | `curl http://localhost:8888/blog/real-estate-101` | 200 HTML article page | 200 OK, 56,463-byte HTML response | Pass with findings |
| Missing article | `curl http://localhost:8888/blog/not-a-real-slug` | 404 not found | 404 with `<h1>Article not found</h1>` | Pass |
| Browser console smoke | Playwright load `/blog/real-estate-101` | No console errors | No console messages or request failures | Pass |
| Newsletter submit | Fill email, click Subscribe | Backend request and success only after persistence | Success visible, zero requests after submit | Fail |
| Mobile nav smoke | 390px viewport, click menu | Nav opens | `.blog-header__nav` received `open` class | Pass |
| JSON-LD parse | Parse rendered `application/ld+json` scripts | Valid JSON with unescaped URL values | JSON parses, but URL contains `&#x2f;` entities | Fail |

---

## Security Findings

- P1: Database-backed article body can render stored HTML as trusted public markup.
- P3: Author social URLs in the database-backed path are rendered directly as external links; Sanity share links have URL allowlisting, but author profile URLs should receive the same `http://`/`https://` normalization.
- No authentication is required or expected for the public article page.
- No financial operations are present on this page.

---

## Database Findings

- `blog_articles`, `blog_authors`, and `blog_categories` exist in `database/024_blog.sql`.
- Relevant published-article indexes exist: slug, status, category, author, published date, tags, and featured.
- Local DB has published article slugs including `real-estate-101`, `understanding-real-estate-tokenization`, and `top-5-bali-investment-properties-2025`.
- The database schema supports `content_html`, but there is no schema-level guarantee that stored HTML is sanitized.

---

## Missing Tests

- Add a template/render test proving article meta title, description, canonical, OG, and Twitter values never render as `none`.
- Add a JSON-LD render test using quotes, apostrophes, slashes, and ampersands.
- Add a content sanitization regression test for database `content_html` and Markdown fallback.
- Add a browser E2E for newsletter submit that expects a real request and verifies error/success states.
- Add a mobile/keyboard accessibility E2E for the blog menu and copy-link control.

---

## Recommended Fix Order

1. Sanitize the database-backed article body path before rendering public HTML.
2. Fix SEO/meta fallback values and JSON-LD serialization.
3. Replace the fake newsletter success path with a real endpoint or disable the form.
4. Add accessible failure/success states for copy link and mobile menu controls.
5. Add targeted render and browser regression tests.

---

## Final Status

`needs_recheck`

Reason: The article page loads and basic public navigation works, but public SEO output, newsletter behavior, and database-backed content safety need fixes and re-verification.
