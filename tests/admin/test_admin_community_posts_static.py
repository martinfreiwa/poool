from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text(encoding="utf-8")


def test_admin_community_posts_page_escapes_table_content_and_script_markers():
    html = read("frontend/platform/admin/community/posts.html")

    assert "escapeHtml(p.author_name)" in html
    assert "escapeHtml(p.post_type)" in html
    assert "escapeHtml(truncContent)" in html
    assert "escapeHtml(t)" in html
    assert "escapeAttr(p.id)" in html
    assert "defeat </script> breakout" not in html
    assert "defeat closing script tag breakout" in html


def test_admin_community_posts_api_uses_bounded_latest_posts_query():
    routes = read("backend/src/community/routes.rs")
    handler = routes[
        routes.index("async fn admin_get_posts") :
        routes.index("#[derive(serde::Deserialize)]\npub struct HidePostPayload")
    ]

    assert "ORDER BY created_at DESC LIMIT 200" in handler
    assert "SELECT * FROM posts ORDER BY created_at DESC\")" not in handler


def test_user_bridge_accepts_nullable_email_without_nested_option():
    bridge = read("backend/src/community/user_bridge.rs")
    batch_handler = bridge[
        bridge.index("pub async fn get_users_info_batch") :
        bridge.index("/// Evicts the Redis cache entry")
    ]

    assert "Some(r.email" not in batch_handler
    assert "r.email.clone()" in batch_handler
    assert "r.email," in batch_handler
