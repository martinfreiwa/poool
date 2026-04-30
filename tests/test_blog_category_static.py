from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text(encoding="utf-8")


def test_blog_category_pagination_stays_on_category_route():
    template = read("frontend/platform/blog/index.html")

    assert "/blog/category/{{ active_category }}?page={{ page - 1 }}" in template
    assert "/blog/category/{{ active_category }}?page={{ page + 1 }}" in template
    assert "&category={{ active_category }}" not in template


def test_blog_category_route_validates_slug_before_querying_content():
    routes = read("backend/src/blog/routes.rs")

    handler_start = routes.index("pub async fn page_blog_category")
    validation = routes.index("if !is_safe_public_slug(&category_slug)", handler_start)
    query = routes.index("list_articles_for_source(&state, page, 12, Some(&category_slug)", handler_start)

    assert validation < query
    assert "fn is_safe_public_slug" in routes


def test_blog_category_metadata_uses_category_context():
    template = read("frontend/platform/blog/index.html")
    routes = read("backend/src/blog/routes.rs")

    assert "title=page_title" in template
    assert "description=page_description" in template
    assert "request_path=canonical_path" in template
    assert "active_category_detail.name" in template
    assert 'canonical_path = format!("/blog/category/{}", category_slug)' in routes
    assert 'format!("{} Articles", category.name)' in routes
