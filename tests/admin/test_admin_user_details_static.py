from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def body_between(source: str, start: str, end: str) -> str:
    start_index = source.index(start)
    end_index = source.index(end, start_index)
    return source[start_index:end_index]


def test_user_detail_reads_require_user_and_pii_permissions():
    users = read("backend/src/admin/users.rs")

    list_body = body_between(
        users,
        "pub async fn api_admin_users",
        "/// GET /api/admin/users/:user_id",
    )
    detail_body = body_between(
        users,
        "pub async fn api_admin_user_detail",
        "/// Payload for updating a user's profile from the admin dashboard.",
    )

    for body in (list_body, detail_body):
        assert 'require_permission(&state.db, "users.view")' in body
        assert 'require_permission(&state.db, "pii.view")' in body
        assert "admin.pii_access" in body


def test_user_detail_mutations_require_granular_permissions():
    users = read("backend/src/admin/users.rs")

    profile_body = body_between(
        users,
        "pub async fn api_admin_user_update_profile",
        "/// Payload for adjusting a user's wallet balance",
    )
    balance_body = body_between(
        users,
        "pub async fn api_admin_user_update_balance",
        "/// Payload for updating a user's account status",
    )
    status_body = body_between(
        users,
        "pub async fn api_admin_user_update_status",
        "/// Payload for updating a user's roles.",
    )
    roles_body = body_between(
        users,
        "pub async fn api_admin_user_update_roles",
        "/// DELETE /api/admin/users/:user_id/sessions",
    )
    sessions_body = body_between(
        users,
        "pub async fn api_admin_user_revoke_sessions",
        "/// POST /api/admin/users/:id/force-password-reset",
    )
    reset_body = users[users.index("pub async fn api_admin_user_force_password_reset") :]

    assert 'require_permission(&state.db, "users.edit")' in profile_body
    assert 'require_permission(&state.db, "pii.view")' in profile_body
    assert 'require_permission(&state.db, "treasury.write")' in balance_body
    assert 'require_permission(&state.db, "users.edit")' in status_body
    assert 'require_permission(&state.db, "roles.edit")' in roles_body
    assert 'require_permission(&state.db, "users.edit")' in sessions_body
    assert 'require_permission(&state.db, "users.edit")' in reset_body


def test_user_profile_and_sensitive_mutations_are_transactional_and_audited():
    users = read("backend/src/admin/users.rs")

    profile_body = body_between(
        users,
        "pub async fn api_admin_user_update_profile",
        "/// Payload for adjusting a user's wallet balance",
    )
    status_body = body_between(
        users,
        "pub async fn api_admin_user_update_status",
        "/// Payload for updating a user's roles.",
    )
    sessions_body = body_between(
        users,
        "pub async fn api_admin_user_revoke_sessions",
        "/// POST /api/admin/users/:id/force-password-reset",
    )

    assert "state.db.begin()" in profile_body
    assert "admin.profile_update" in profile_body
    assert "user.tier_override" in profile_body
    assert "tx.commit().await.map_err(ApiError::from)" in profile_body

    assert "state.db.begin()" in status_body
    assert "admin.user_status_update" in status_body
    assert "tx.commit().await.map_err(ApiError::from)" in status_body

    assert "state.db.begin()" in sessions_body
    assert "admin.revoke_sessions" in sessions_body
    assert "tx.commit().await.map_err(ApiError::from)" in sessions_body
