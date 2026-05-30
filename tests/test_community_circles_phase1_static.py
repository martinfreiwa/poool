from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_circle_feed_route_is_default_destination():
    lib = read("backend/src/lib.rs")

    assert '"/community/circles"' in lib
    assert "page_community_circles" in lib
    assert '"/community/circle/:slug", get(page_community_circle_feed)' in lib
    assert "Redirect::to(&format!(\"/community/circle/{}/settings\"" not in lib
    assert '"/community/circle/:slug/settings"' in lib
    assert "page_community_circle_settings" in lib


def test_circle_posts_schema_and_global_feed_isolation_exist():
    migration = read("database/community/049_circle_posts.sql")
    service = read("backend/src/community/service.rs")
    models = read("backend/src/community/models.rs")

    assert "ADD COLUMN IF NOT EXISTS circle_id UUID REFERENCES circles(id)" in migration
    assert "idx_posts_circle_visible_created" in migration
    assert "idx_posts_global_visible_created" in migration
    assert "p.circle_id IS NULL" in service
    assert "pub async fn get_circle_feed" in service
    assert "AND p.circle_id = $1" in service
    assert "circle_id: Option<Uuid>" in models


def test_circle_post_api_contract_is_registered_and_authorized():
    routes = read("backend/src/community/routes.rs")

    assert '"/api/community/circles/:id/posts"' in routes
    assert "get(get_circle_posts).post(create_circle_post)" in routes
    assert "ensure_circle_read_access" in routes
    assert "ensure_circle_write_access" in routes
    assert "Join this Circle before posting." in routes
    assert "Use the Circle post endpoint for Circle-scoped posts." in routes


def test_circle_detail_template_is_content_first_and_role_gated():
    html = read("frontend/platform/community-circle.html")
    settings = read("frontend/platform/community-circle-settings.html")

    assert "window.POOOL_CIRCLE_CONTEXT" in html
    assert "Post to: <strong>{{ circle_name }}</strong>" in html
    assert 'hx-get="/community/partials/feed/list"' in html
    assert 'hx-include="#feed-filters"' in html
    assert '<input type="hidden" name="circle_id" value="{{ circle_id }}">' in html
    assert 'href="/community/circle/{{ circle_slug }}/settings"' in html
    assert "{% if can_manage %}" in html
    assert "Circle Settings" in settings
    assert 'href="/community/circles"' in settings


def test_circle_frontend_posts_to_circle_endpoint_when_context_exists():
    js = read("frontend/platform/static/js/community-feed.js")
    partial = read("frontend/platform/partials/community_post_list.html")

    assert "window.POOOL_CIRCLE_CONTEXT" in js
    assert "getPostCreateEndpoint" in js
    assert "`/api/community/circles/${encodeURIComponent(circleContext.id)}/posts`" in js
    assert "circle_id: circleContext ? circleContext.id : null" in js
    assert "New Circle · Be the first to post" in partial
    assert "{% if current_circle_id %}&circle_id={{ current_circle_id }}{% endif %}" in partial
    assert 'hx-include="this"' in partial


def test_feed_owner_post_delete_uses_automatable_modal():
    page = read("frontend/platform/community.html")
    js = read("frontend/platform/static/js/community-feed.js")

    for token in [
        "delete-post-modal",
        "delete-post-title",
        "delete-post-desc",
        "delete-post-id",
        "delete-post-error",
        "delete-post-confirm-btn",
        "This permanently deletes the post",
    ]:
        assert token in page

    for token in [
        "window.deleteOwnPost = function",
        "window.submitDeletePost = async function",
        "delete-post-confirm-btn",
        "window.openCommunityModal('delete-post-modal')",
        "window.closeCommunityModal('delete-post-modal')",
    ]:
        assert token in js

    assert "confirm('Delete this post? This cannot be undone.')" not in js
