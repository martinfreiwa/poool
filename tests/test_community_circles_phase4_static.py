from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase4_backend_uses_structured_circle_mention_tokens():
    routes = read("backend/src/community/routes.rs")

    assert "struct CircleMentionRef" in routes
    assert "enum InlineMentionToken" in routes
    assert "InlineMentionToken::Circle" in routes
    assert "extract_circle_mention_terms" in routes
    assert "resolve_circle_mentions_for_content" in routes
    assert "tokenize_inline_mentions" in routes
    assert "render_inline_content_with_circle_mentions" in routes
    assert "@circle/" in routes
    assert "hydrate_circle_mentions(c_pool" in routes


def test_phase4_circle_mentions_are_privacy_safe_and_xss_escaped():
    routes = read("backend/src/community/routes.rs")

    assert "circle.viewer_role.is_some()" in routes
    assert 'circle.is_public && circle.visibility == "public"' in routes
    assert 'circle.visibility == "hidden"' in routes
    assert "Private Circle" in routes
    assert "Circle mention unavailable" in routes
    assert "circle-mention-tag--private" in routes
    assert "circle-mention-tag--redacted" in routes

    assert "fn html_escape" in routes
    assert "fn attr_escape" in routes
    assert "attr_escape(&circle.slug)" in routes
    assert "html_escape(&label)" in routes
    assert "html_escape(&user)" in routes
    assert "html_escape(&tag)" in routes


def test_phase4_circle_mentions_do_not_trigger_circle_member_notifications():
    routes = read("backend/src/community/routes.rs")

    mention_notifier = routes[
        routes.index("async fn parse_and_notify_mentions") : routes.index("/// Helper to parse the first URL")
    ]
    assert 'word.starts_with("@circle/")' in mention_notifier
    assert "continue;" in mention_notifier
    assert "Circle Mentions are not user mentions" in mention_notifier


def test_phase4_autocomplete_returns_only_visible_circles():
    routes = read("backend/src/community/routes.rs")
    js = read("frontend/platform/static/js/community-autocomplete.js")

    suggest_mentions = routes[
        routes.index("async fn suggest_mentions") : routes.index("// ─── UX.8: Trending posts")
    ]
    assert "middleware::get_current_user" in suggest_mentions
    assert "LEFT JOIN circle_members cm" in suggest_mentions
    assert "cm.user_id = $2" in suggest_mentions
    assert "c.visibility = 'public' AND c.is_public = TRUE" in suggest_mentions
    assert "OR cm.user_id IS NOT NULL" in suggest_mentions
    assert '"circles": circles' in suggest_mentions
    assert '"mention_token": format!("@circle/{}", slug)' in suggest_mentions

    assert "(data.circles || [])" in js
    assert 'kind: "circle"' in js
    assert 'value: "circle/" + c.slug' in js
    assert 'info.query.indexOf("circle/") === 0' in js
    assert "return circles.concat(users)" in js


def test_phase4_frontend_has_circle_mention_styles():
    css = read("frontend/platform/static/css/community.css")

    assert ".circle-mention-tag" in css
    assert ".circle-mention-tag--private" in css
    assert ".circle-mention-tag--redacted" in css
    assert "cursor: default" in css
