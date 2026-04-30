from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def function_segment(source: str, name: str, length: int = 5000) -> str:
    start = source.index(f"pub async fn {name}")
    return source[start : start + length]


def test_submission_review_backend_uses_granular_permissions_and_transactional_review_start():
    source = read("backend/src/admin/developer_projects.rs")
    migration = read("database/089_admin_submission_permissions.sql")

    assert 'const SUBMISSIONS_REVIEW_PERMISSION: &str = "submissions.review"' in source
    assert 'const SUBMISSIONS_APPROVE_PERMISSION: &str = "submissions.approve"' in source

    for handler in [
        "api_admin_developer_projects",
        "api_admin_developer_project_detail",
        "api_admin_project_notes_list",
        "api_admin_project_notes_create",
        "api_admin_project_checklist_get",
        "api_admin_project_checklist_save",
    ]:
        segment = function_segment(source, handler)
        assert "require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)" in segment

    notes_segment = function_segment(source, "api_admin_project_notes_create", 18000)
    assert "require_permission(&state.db, SUBMISSIONS_APPROVE_PERMISSION)" in notes_segment
    assert "validate_project_ready_for_approval(&state, pid).await?" in notes_segment
    assert "'developer_project.approved'" in notes_segment
    assert "return Err(ApiError::Database(e));" in notes_segment

    detail_segment = function_segment(source, "api_admin_developer_project_detail", 9000)
    assert "let mut tx = state.db.begin().await.map_err(ApiError::Database)?" in detail_segment
    assert "AND status = 'submitted'" in detail_segment
    assert "updated.rows_affected() == 0" in detail_segment
    assert "'developer_project.review_started'" in detail_segment
    assert "tx.commit().await.map_err(ApiError::Database)?" in detail_segment

    notes_list_segment = function_segment(source, "api_admin_project_notes_list")
    assert ".unwrap_or_default()" not in notes_list_segment
    assert "ApiError::Database(e)" in notes_list_segment

    assert "('submissions.review')" in migration
    assert "('submissions.approve')" in migration


def test_submission_review_admin_image_routes_are_dedicated_audited_and_frontend_wired():
    routes = read("backend/src/admin/mod.rs")
    assets = read("backend/src/admin/assets.rs")
    js = read("frontend/platform/static/js/admin-submission-review.js")

    assert '"/api/admin/assets/:asset_id/images"' in routes
    assert "post(api_admin_asset_image_upload)" in routes
    assert '"/api/admin/assets/:asset_id/images/:image_id"' in routes
    assert "delete(api_admin_asset_image_delete)" in routes
    assert '"/api/admin/assets/:asset_id/images/reorder"' in routes
    assert "put(api_admin_asset_images_reorder)" in routes

    for handler in [
        "api_admin_asset_image_upload",
        "api_admin_asset_image_delete",
        "api_admin_asset_images_reorder",
    ]:
        segment = function_segment(assets, handler, 9000)
        assert 'require_permission(&state.db, "assets.edit")' in segment
        assert "state.db.begin().await.map_err(ApiError::Database)?" in segment
        assert "audit_logs" in segment

    assert "validate_asset_image_mime" in assets
    assert "sniff_admin_image_mime" in assets
    assert "File content does not match declared type" in assets
    assert "Exactly one image must be marked as cover" in assets

    assert "/api/admin/assets/${assetId}/images/${imgId}" in js
    assert "/api/admin/assets/${assetId}/images/reorder" in js
    assert "/api/admin/assets/${assetIdForUpload}/images" in js
    assert "/api/developer/draft/${assetId}" not in js
    assert "/api/developer/draft/${assetIdForUpload}" not in js
    assert "_adminImages = previousImages" in js


def test_submission_review_frontend_escapes_errors_and_has_modal_a11y_feedback():
    html = read("frontend/platform/admin/developer-submission-review.html")
    js = read("frontend/platform/static/js/admin-submission-review.js")

    assert "Failed to load project: ${esc(error.message || \"Unknown error\")}" in js
    assert '<script src="/static/js/toast.js"></script>' in html
    assert 'role="dialog"' in html
    assert 'aria-modal="true"' in html
    assert 'aria-labelledby="reason-modal-title"' in html
    assert 'aria-describedby="reason-modal-subtitle"' in html
    assert "function trapReasonModalFocus" in js
    assert 'event.key === "Escape"' in js
    assert "event.target === reasonModal" in js
