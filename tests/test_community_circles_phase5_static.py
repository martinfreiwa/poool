from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase5_schema_extends_post_taxonomy_tags_and_circle_policies():
    migration = read("database/community/051_post_types_tags_compliance.sql")

    for post_type in [
        "discussion",
        "question",
        "market_insight",
        "property_update",
        "due_diligence",
        "poll",
        "announcement",
        "ama_question",
        "resource",
        "risk_discussion",
        "official_update",
    ]:
        assert f"'{post_type}'" in migration

    assert "ADD COLUMN IF NOT EXISTS content_tags TEXT[] NOT NULL DEFAULT" in migration
    assert "ADD COLUMN IF NOT EXISTS required_post_tags TEXT[]" in migration
    assert "ADD COLUMN IF NOT EXISTS allowed_post_types TEXT[]" in migration
    assert "idx_posts_content_tags_gin" in migration
    assert "idx_posts_circle_type_created" in migration
    assert "server-side" in migration


def test_phase5_backend_validates_post_types_tags_and_official_permissions():
    routes = read("backend/src/community/routes.rs")

    for token in [
        "const COMMUNITY_POST_TYPES",
        "const COMMUNITY_POST_TAGS",
        "const OFFICIAL_ONLY_POST_TYPES",
        "const PRIVILEGED_POST_TAGS",
        "fn normalize_post_type",
        "fn normalize_post_tags",
        "async fn ensure_post_taxonomy_allowed",
        "Invalid post type for community content",
        "Invalid post tag",
        "Only Circle moderators or platform admins can publish this post type",
        "This Circle requires the",
        "This Circle does not allow that post type",
    ]:
        assert token in routes

    create_scope = routes[
        routes.index("async fn create_user_post_for_scope")
        : routes.index("let post_id = service::create_user_post")
    ]
    assert "normalize_post_type(&payload.post_type)" in create_scope
    assert "normalize_post_tags(payload.content_tags.take())" in create_scope
    assert "ensure_post_taxonomy_allowed(" in create_scope
    assert "payload.content_tags = Some(content_tags)" in create_scope


def test_phase5_backend_persists_tags_filters_feeds_and_disclaimers():
    models = read("backend/src/community/models.rs")
    service = read("backend/src/community/service.rs")
    moderation = read("backend/src/community/moderation.rs")
    routes = read("backend/src/community/routes.rs")

    assert "pub content_tags: Option<Vec<String>>" in models
    assert "pub content_tags: Vec<String>" in models
    assert "pub circle_name: Option<String>" in models

    assert "post_requires_compliance_disclaimer" in moderation
    assert "COMPLIANCE_POST_TYPES" in moderation
    assert "COMPLIANCE_TAGS" in moderation
    assert "User opinion, not financial advice" in read(
        "frontend/platform/partials/community_post_card.html"
    )

    assert "INSERT INTO posts (user_id, post_type, content, content_sanitized, asset_id, circle_id, image_urls, content_tags" in service
    assert ".bind(&content_tags)" in service
    assert "disclaimer_shown" in service
    assert "p.post_type = $5" in service
    assert "ANY(COALESCE(p.content_tags" in service

    assert "pub post_type: Option<String>" in routes
    assert "pub tag: Option<String>" in routes
    assert "circle_name_map" in routes
    assert "display.circle_name" in routes


def test_phase5_frontend_composer_exposes_types_tags_filters_and_payload():
    global_feed = read("frontend/platform/partials/community_feed.html")
    circle_page = read("frontend/platform/community-circle.html")
    lib = read("backend/src/lib.rs")
    js = read("frontend/platform/static/js/community-feed.js")

    for html in [global_feed, circle_page]:
        assert 'id="form-post-type" name="post_type"' in html
        assert 'id="form-post-tag" name="tag"' in html

    assert 'id="post-type-input"' not in global_feed
    assert 'id="post-tags-input"' not in global_feed
    assert "community-composer__metadata" not in global_feed
    assert 'id="post-type-input"' not in circle_page
    assert 'id="post-tags-input"' not in circle_page
    assert "community-composer__metadata" not in circle_page
    assert "circle-space-hero__icon" not in circle_page
    assert 'aria-label="Notification settings"' not in circle_page
    assert "document.getElementById('post-content-input')?.focus()" not in circle_page
    assert "{{ member_count }} / {{ max_members }} members" not in circle_page
    assert "{{ member_count }} member" in circle_page
    assert "max_members: circle.max_members" not in lib

    assert 'id="feed-post-type-filter"' not in global_feed
    assert 'id="feed-post-type-filter"' not in circle_page
    assert 'id="feed-post-tag-filter"' not in global_feed
    assert 'id="feed-post-tag-filter"' not in circle_page
    assert "community-feed__filter-group--selects" not in global_feed
    assert "community-feed__filter-group--selects" not in circle_page
    assert "community-feed__filters" not in circle_page
    assert "circle-space-tabs-row" in circle_page
    assert 'aria-label="Circle sections"' not in circle_page
    assert 'role="tablist"' in circle_page
    assert 'role="tab"' in circle_page
    assert 'role="tabpanel"' in circle_page
    assert 'id="members"' not in circle_page
    assert "{{ member_count }} members - {{ role_label }}" not in circle_page
    assert "circle-space-sort ds-segmented" in circle_page
    assert circle_page.index("circle-space-sort") < circle_page.index("circle-space-layout")

    for token in [
        "window.setPostTypeFilter",
        "window.setPostTagFilter",
        "function canonicalCommunityCode",
        "function parsePostTags",
        "function updateDisclaimerWarning",
        "const COMPLIANCE_POST_TYPES",
        "const COMPLIANCE_TAGS",
        "content_tags: contentTags",
        "requestBody.post_type = 'poll'",
    ]:
        assert token in js


def test_phase5_circle_section_tabs_are_removed_but_sort_remains():
    css = read("frontend/platform/static/css/community.css")

    assert ".circle-space-tabs-row" in css
    assert ".circle-space-sort" in css
    assert ".circle-space-hero__icon" not in css
    assert "grid-template-columns: minmax(0, 1fr) auto;" in css


def test_phase5_post_cards_render_type_badges_tags_circle_and_compliance_text():
    post_card = read("frontend/platform/partials/community_post_card.html")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        "p.circle_name",
        "feed-post-circle-name",
        "p.content_tags",
        "feed-post-tags",
        "feed-post-tag",
        "Official Update",
        "Market Insight",
        "Question",
        "Property Update",
        "Due Diligence",
        "Risk Discussion",
        "setPostTagFilter",
        "User opinion, not financial advice",
    ]:
        assert token in post_card

    for token in [
        ".feed-post-badge--insight",
        ".feed-post-badge--diligence",
        ".feed-post-badge--risk",
        ".feed-post-tags",
        ".feed-post-tag",
    ]:
        assert token in css
