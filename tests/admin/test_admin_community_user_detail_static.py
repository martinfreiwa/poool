from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def function_body(source: str, name: str) -> str:
    start = source.index(f"async fn {name}")
    next_fn = source.find("\nasync fn ", start + 1)
    if next_fn == -1:
        next_fn = source.find("\n#[", start + 1)
    return source[start:next_fn]


def test_community_user_detail_template_renders_untrusted_data_safely():
    html = read("frontend/platform/admin/community/user-detail.html")

    assert "innerHTML" not in html
    assert "outerHTML" not in html
    assert "onclick=" not in html
    assert "prompt(" not in html
    assert "alert(" not in html
    assert "confirm(" not in html
    assert "https://unpkg.com" not in html
    assert "cdn.jsdelivr" not in html
    assert "textContent" in html
    assert "document.createElement" in html
    assert "getSafeHttpUrl" in html
    assert '/static/js/csrf.js' in html
    assert "csrfHeaders({ 'Content-Type': 'application/json' })" in html
    assert "moderation-dialog" in html
    assert 'aria-live="polite"' in html


def test_community_user_detail_backend_uses_fine_grained_permissions():
    routes = read("backend/src/community/routes.rs")

    assert "require_community_view_or_manage(&state, &admin).await?;" in function_body(
        routes, "admin_get_users"
    )
    assert "require_community_view_or_manage(&state, &admin).await?;" in function_body(
        routes, "admin_get_user_detail"
    )

    for name in [
        "admin_toggle_ban_user",
        "admin_mute_user",
        "admin_toggle_shadowban",
        "admin_warn_user",
        "admin_update_mod_notes",
    ]:
        body = function_body(routes, name)
        assert "require_community_manage(&state, &admin).await?;" in body


def test_community_user_detail_mutations_are_transactional_and_audited():
    routes = read("backend/src/community/routes.rs")

    for name in [
        "admin_toggle_ban_user",
        "admin_mute_user",
        "admin_toggle_shadowban",
        "admin_warn_user",
        "admin_update_mod_notes",
    ]:
        body = function_body(routes, name)
        assert "let mut tx = c_pool.begin().await?;" in body
        assert ".execute(&mut *tx)" in body
        assert "rows_affected() == 0" in body
        assert 'AppError::NotFound("Community user not found."' in body
        assert "log_community_admin_action_tx(" in body
        assert ".await?;" in body
        assert "tx.commit().await?;" in body

    ban_body = function_body(routes, "admin_toggle_ban_user")
    assert "Ban reason is required." in ban_body
    assert "Ban reason must be 1000 characters or fewer." in ban_body


def test_community_user_detail_api_returns_moderation_state_needed_by_ui():
    routes = read("backend/src/community/routes.rs")
    body = function_body(routes, "admin_get_user_detail")

    assert "'mod_notes', mod_notes" in body
    assert "'muted_until', muted_until" in body
    assert "'is_shadowbanned', is_shadowbanned" in body
    assert 'AppError::NotFound("Community user not found."' in body
