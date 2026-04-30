from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_community_users_page_uses_dedicated_safe_controller():
    html = read("frontend/platform/admin/community/users.html")
    js = read("frontend/platform/static/js/admin-community-users.js")

    assert "https://unpkg.com/htmx" not in html
    assert "cdn.jsdelivr.net/npm/alpinejs" not in html
    assert 'onclick="loadUsers()"' not in html
    assert "<script>\n    document.addEventListener" not in html
    assert '<script src="/static/js/admin-community-users.js"></script>' in html
    assert "tbody.innerHTML" not in js
    assert "prompt(" not in js
    assert "confirm(" not in js
    assert "alert(" not in js
    assert "textContent" in js
    assert "createElement" in js
    assert "showModal" in js
    assert "X-CSRF-Token" in js


def test_community_users_backend_enforces_permissions_csrf_and_audit_transaction():
    routes = read("backend/src/community/routes.rs")

    list_handler = routes[routes.index("async fn admin_get_users") : routes.index("#[derive(serde::Deserialize)]\npub struct BanUserPayload")]
    ban_handler = routes[routes.index("async fn admin_toggle_ban_user") : routes.index("#[derive(serde::Deserialize)]\npub struct MuteUserPayload")]

    assert "require_community_view_or_manage(&state, &admin).await?" in list_handler
    assert "require_community_manage(&state, &admin).await?" in ban_handler
    assert "require_csrf_header(&headers, &jar)?" in ban_handler
    assert "SELECT is_community_banned, ban_reason FROM community_profiles WHERE user_id = $1 FOR UPDATE" in ban_handler
    assert "let mut tx = c_pool.begin().await?" in ban_handler
    assert "updated_at = NOW()" in ban_handler
    assert "log_community_admin_action_tx(" in ban_handler
    assert "previous_profile" in ban_handler
    assert "new_profile" in ban_handler
    assert "tx.commit().await?" in ban_handler
