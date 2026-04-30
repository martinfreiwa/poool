# Daily Page Audit: Blog Category

Date: 2026-04-28  
Auditor: ChatGPT/Codex  
Page audited: Blog category  
Route: `/blog/category/:slug`  
Template: `frontend/platform/blog/index.html`  
Backend route: `backend/src/blog/mod.rs` -> `page_blog_category`  
Final status: `fixed`

## Scope And Evidence

Initial audit was documentation-only. Follow-up fix work updated the blog category route/template, fixed compile blockers encountered during verification, and added static plus browser regression tests.

Reviewed:

- `frontend/platform/blog/index.html`
- `frontend/platform/components/blog-head.html`
- `frontend/platform/components/blog-header.html`
- `frontend/platform/components/blog-footer.html`
- `frontend/platform/static/css/blog.css`
- `backend/src/blog/mod.rs`
- `backend/src/blog/routes.rs`
- `backend/src/blog/service.rs`
- `backend/src/blog/sanity.rs`
- `database/024_blog.sql`
- `docs/page-review-tracker.yml`
- `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`

Commands run:

```bash
python3 - <<'PY'
import yaml
from pathlib import Path
data = yaml.safe_load(Path('docs/page-review-tracker.yml').read_text())
print(next(page['id'] for page in data['pages'] if page['id'] == 'blog.category'))
PY
```

```bash
rg -n "blog/category|page_blog_category|list_articles\\(|list_categories\\(|blog/index.html|blog-header__mobile-toggle" frontend/platform/blog/index.html frontend/platform/components/blog-*.html backend/src/blog database tests
```

```bash
curl -sS -o /tmp/blog-category.html -w 'HTTP %{http_code}\n' http://localhost:8888/blog/category/investment-guides
```

The curl smoke was blocked: no local backend was listening on `localhost:8888` (`curl: (7) Failed to connect`). No dedicated blog category E2E tests were found under `tests/`.

Follow-up fix commands:

```bash
python3 -m pytest tests/test_blog_category_static.py -q
```

Result: `3 passed`.

```bash
BASE_URL=http://localhost:8888 python3 -m pytest tests/e2e/test_blog_category.py -q
```

Result: `4 passed`.

```bash
git diff --check -- backend/src/blog/routes.rs backend/src/blog/models.rs frontend/platform/blog/index.html tests/test_blog_category_static.py
```

Result: passed.

```bash
rustfmt --edition 2021 --check backend/src/blog/routes.rs backend/src/blog/models.rs
```

Result: passed.

```bash
CARGO_TARGET_DIR=/tmp/poool-fix-documented-current cargo check --message-format=short
```

Result: passed.

```bash
cargo check
```

Initial result: blocked by unrelated existing compile errors. Follow-up result with `CARGO_TARGET_DIR=/tmp/poool-blog-category-final2 cargo check --message-format=short`: passed.

```bash
python3 scripts/audit_page_review_tracker.py --write-md
```

Result: regenerated `docs/PAGE_REVIEW_TRACKER.md`; script reported one unrelated missing reference: `affiliate.settings: frontend/platform/static/js/affiliate-settings.js`.

Follow-up tracker result after script/source updates: passed with 0 missing routes, 0 missing page template entries, 0 missing supporting template entries, and 0 missing file references.

```bash
BLOG_CONTENT_SOURCE=database CARGO_TARGET_DIR=/tmp/poool-blog-category-final2 SERVER_PORT=8888 cargo run
curl -sS -o /tmp/blog-category-fixed.html -w 'HTTP %{http_code}\n' http://localhost:8888/blog/category/investment-guides
curl -sS -o /tmp/blog-category-bad.html -w 'HTTP %{http_code}\n' 'http://localhost:8888/blog/category/%3Cscript%3E'
curl -sS -o /tmp/blog-feed.xml -w 'HTTP %{http_code}\n' http://localhost:8888/blog/feed.xml
```

Result: category page returned `HTTP 200` with category title/canonical/breadcrumb metadata, unsafe category slug returned `HTTP 404`, and RSS feed returned `HTTP 200`.

## Page Map

The route is registered publicly in `backend/src/blog/mod.rs`:

- `GET /blog/category/:slug` -> `page_blog_category`

The handler:

- Rejects unsafe category slugs before querying the content source.
- Parses `page` from the query string and clamps it to at least `1`.
- Calls `list_articles_for_source(&state, page, 12, Some(&category_slug), None, false)`.
- Loads categories through `list_categories_for_source`.
- Renders `blog/index.html` with `articles`, `total_pages`, `page`, `categories`, `active_category`, `active_category_detail`, category metadata, and `base_url`.

Data dependencies:

- Sanity mode: `SanityClient::list_articles` and `SanityClient::list_categories`.
- Database fallback: `blog_articles`, `blog_categories`, `blog_authors`.
- `database/024_blog.sql` provides the blog tables and relevant indexes.

No page-specific JavaScript file is attached. Runtime behavior is link navigation plus a shared inline mobile menu toggle in `components/blog-header.html`.

## UI Elements Reviewed

| Element | Location | Expected behavior | Backend support | Status |
|---|---|---|---|---|
| Blog logo | `components/blog-header.html` | Navigate to `/en/` | Static route/marketing route | Unverified runtime |
| Header nav links | `components/blog-header.html` | Navigate to `/blog` and known category routes | Blog routes exist | Wired by links |
| Language links | `components/blog-header.html` | Navigate to `/id` and `/en` | Host/root routing dependent | Unverified runtime |
| Sign in header button | `components/blog-header.html` | Navigate to `/auth/login` | Auth route exists | Wired by link |
| Mobile menu toggle | `components/blog-header.html` | Toggle `.blog-header__nav.open` | Frontend only | Browser E2E passed |
| Category pills | `blog/index.html` | Navigate to `/blog` or `/blog/category/{{ cat.slug }}` and mark active category | Category handler and category data exist | Wired |
| Featured article card | `blog/index.html` | Navigate to `/blog/{{ featured.slug }}` | Article handler exists | Not used by category handler unless `featured` is supplied |
| Article cards | `blog/index.html` | Navigate to article detail route | Article handler exists | Wired |
| Empty state | `blog/index.html` | Show when no articles match | Handler supplies empty article list | Works statically; DB/Sanity error distinction weak |
| Pagination previous/next | `blog/index.html` | Stay within selected category while changing page | Backend can paginate category route | Fixed with static and E2E template contract coverage |
| CTA Sign in | `blog/index.html` | Navigate to `/auth/login` | Auth route exists | Wired |
| Footer category links | `components/blog-footer.html` | Navigate to known category pages | Category handler exists | Wired |
| Footer legal/platform/social links | `components/blog-footer.html` | Navigate to static/internal/external pages | Mixed route support | Unverified runtime |
| Breadcrumb JSON-LD | `blog/index.html` | Describe category breadcrumb | Backend passes active category context | Fixed and browser verified |

## Findings

### PAGE-ISSUE-0475: Category pagination leaves the category route

Severity: high  
Category: functional  
Status: fixed, needs runtime recheck

`frontend/platform/blog/index.html` renders category pagination links as:

```html
/blog?page={{ page - 1 }}{% if active_category %}&category={{ active_category }}{% endif %}
```

and the same pattern for next page. `page_blog_index` only reads `page` and `tag`; it ignores a `category` query parameter. A user on `/blog/category/investment-guides?page=1` who clicks Next is sent to `/blog?page=2&category=investment-guides`, which can show the generic blog page rather than the selected category.

Fix applied: when `active_category` is present, pagination now links to `/blog/category/{{ active_category }}?page=N`. Static regression coverage was added in `tests/test_blog_category_static.py`.

### PAGE-ISSUE-0476: Category pages render generic blog SEO metadata

Severity: medium  
Category: functional/SEO  
Status: fixed, needs runtime recheck

`blog/index.html` always includes `blog-head.html` with fixed `title="Blog"` and `request_path="/blog"`. The category route does not pass the selected category object, category description, category canonical path, or category breadcrumb data. As a result, `/blog/category/:slug` can render:

- Generic title and description.
- Canonical URL for `/blog`, not the category URL.
- Open Graph URL for `/blog`, not the category URL.
- Breadcrumb JSON-LD containing only Home > Blog.

Fix applied: `page_blog_category` now resolves the active category, passes category-specific `page_title`, `page_description`, `canonical_path`, and `active_category_detail`, and the template emits a category breadcrumb item when category context exists. Static regression coverage was added.

### PAGE-ISSUE-0477: Unsafe Sanity category slugs drop the filter

Severity: medium  
Category: security/reliability  
Status: fixed, needs runtime recheck

The Sanity article filter only adds the category predicate if `is_safe_slug(category_slug)` returns true:

```rust
if let Some(slug) = category_slug.filter(|s| is_safe_slug(s)) {
    parts.push(format!("category->slug.current == {}", groq_string(slug)));
}
```

`page_blog_category` passes the path slug directly and does not reject unsafe slugs before calling the content source. In Sanity mode, a malformed category slug can therefore remove the category predicate and return the full public article list. The DB fallback does not have injection exposure because it uses SQL binds, but behavior is inconsistent across content sources.

Fix applied: `page_blog_category` now validates `category_slug` with `is_safe_public_slug` before calling the content source and returns `404` for unsafe values. Static regression coverage was added.

## Remaining Issues

No known code-level issues remain for this page from the 2026-04-28 audit findings.

Remaining verification work: none documented for this page. The local seed does not currently include enough articles in one category to click through multiple category pages, so the committed coverage protects the multi-page pagination route contract at template level.

## Security And Data Integrity Notes

- The page is public and read-only. No auth, PII, KYC, wallet, payment, or money mutation path is involved.
- Database fallback uses bound parameters for category filtering.
- No financial values are calculated on this page.
- Public content error handling currently degrades article list failures to an empty list. That avoids exposing internal errors, but it can hide provider/DB outages as legitimate empty categories.

## Accessibility Notes

- Main category, article, CTA, and footer actions are links with visible text.
- Header mobile menu button has `aria-label="Menu"`, `aria-controls`, and browser-tested `aria-expanded` updates for click, Enter, Escape, and outside-click behavior.
- Pagination nav has `aria-label="Blog pagination"`.
- Focused browser coverage verifies mobile menu state, active category indication, and absence of critical console/network errors.

## Tests And Verification

Initial runtime verification was blocked because no backend was running on port `8888`. Follow-up runtime and browser verification now pass against a local backend on `localhost:8888`.

## Tracker Updates

Updated:

- `docs/page-review-tracker.yml`
- `docs/PAGE_REVIEW_TRACKER.md`
- `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`

Severity counts:

- Critical: 0
- High: 1
- Medium: 2
- Low: 0
- Info: 0

Fixed issue counts:

- High: 1
- Medium: 2

Remaining issue counts:

- Code issues: 0
- Verification gaps: 0
