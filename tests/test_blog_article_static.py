from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def test_blog_article_sanitizes_database_html_and_public_author_urls():
    service = read("backend/src/blog/service.rs")

    assert "fn sanitize_article_html" in service
    assert ".map(|html| sanitize_article_html(&html))" in service
    assert ".map(|md| sanitize_article_html(&render_markdown(&md)))" in service
    assert "fn clean_public_url" in service
    assert 'lower.starts_with("https://") || lower.starts_with("http://")' in service
    assert "website_url: clean_public_url" in service
    assert "linkedin_url: clean_public_url" in service
    assert "facebook_url: clean_public_url" in service
    assert "instagram_url: clean_public_url" in service


def test_blog_article_metadata_and_json_ld_are_precomputed_server_side():
    routes = read("backend/src/blog/routes.rs")
    head = read("frontend/platform/components/blog-head.html")
    article = read("frontend/platform/blog/article.html")

    assert "let canonical_url = effective_canonical_url" in routes
    assert "let meta_title = non_empty(article.meta_title.as_deref())" in routes
    assert "let blog_schema_json = blog_posting_schema(&article, &canonical_url)" in routes
    assert "serde_json::to_string(&schema)" in routes
    assert "{{ blog_schema_json | safe }}" in article
    assert "{{ breadcrumb_schema_json | safe }}" in article
    assert "{{ faq_schema_json | safe }}" in article
    assert "effective_meta_title = meta_title if meta_title else" in head
    assert "effective_canonical_url = canonical_url if canonical_url else" in head


def test_blog_article_newsletter_uses_real_endpoint_and_error_state():
    mod = read("backend/src/blog/mod.rs")
    routes = read("backend/src/blog/routes.rs")
    article = read("frontend/platform/blog/article.html")

    assert '.route("/api/blog/newsletter", post(subscribe_newsletter))' in mod
    assert "pub async fn subscribe_newsletter" in routes
    assert "validate_email(&email)" in routes
    assert "state.auth_rate_limiter.check" in routes
    assert "INSERT INTO email_logs" in routes
    assert "fetch('/api/blog/newsletter'" in article
    assert "'X-CSRF-Token': blogCsrfToken()" in article
    assert 'class="blog-newsletter__error"' in article


def test_blog_article_copy_and_mobile_menu_have_accessible_state():
    article = read("frontend/platform/blog/article.html")
    header = read("frontend/platform/components/blog-header.html")

    assert 'id="blog-copy-status" aria-live="polite"' in article
    assert "blogSetCopyStatus('Link copied.')" in article
    assert "blogSetCopyStatus('Could not copy link.')" in article
    assert 'aria-controls="blog-primary-nav"' in header
    assert 'aria-expanded="false"' in header
    assert "toggle.setAttribute('aria-expanded', isOpen ? 'true' : 'false')" in header
    assert "event.key === 'Escape'" in header
