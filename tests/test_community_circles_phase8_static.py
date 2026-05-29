from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase8_schema_adds_asset_circle_and_resource_controls():
    migration = read("database/community/054_asset_circle_resources.sql")

    for token in [
        "ADD COLUMN IF NOT EXISTS related_asset_id UUID NULL",
        "ADD COLUMN IF NOT EXISTS is_primary_asset_circle BOOLEAN NOT NULL DEFAULT FALSE",
        "ADD COLUMN IF NOT EXISTS holder_only_documents BOOLEAN NOT NULL DEFAULT TRUE",
        "ADD COLUMN IF NOT EXISTS asset_circle_tabs TEXT[]",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_circles_primary_asset_circle",
        "CREATE TABLE IF NOT EXISTS circle_resources",
        "access_scope VARCHAR(32) NOT NULL DEFAULT 'member'",
        "'holder_only'",
        "'admin_only'",
        "URLs are returned only after Circle and holder access checks",
        "private_investor_club = TRUE",
        "allow_cross_post = FALSE",
    ]:
        assert token in migration


def test_phase8_resource_delivery_schema_adds_versioned_private_metadata():
    migration = read("database/community/056_circle_resource_delivery.sql")

    for token in [
        "ADD COLUMN IF NOT EXISTS file_name VARCHAR(240)",
        "ADD COLUMN IF NOT EXISTS mime_type VARCHAR(120)",
        "ADD COLUMN IF NOT EXISTS file_size_bytes BIGINT",
        "ADD COLUMN IF NOT EXISTS sha256_hex CHAR(64)",
        "ADD COLUMN IF NOT EXISTS version_label VARCHAR(80) NOT NULL DEFAULT 'v1'",
        "ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "ADD COLUMN IF NOT EXISTS requires_download BOOLEAN NOT NULL DEFAULT TRUE",
        "CREATE INDEX IF NOT EXISTS idx_circle_resources_delivery_active",
        "Never returned by list APIs; only resolved by the authenticated delivery endpoint.",
    ]:
        assert token in migration


def test_phase8_resource_version_history_schema_adds_explicit_versions():
    migration = read("database/community/059_circle_resource_versions.sql")

    for token in [
        "CREATE TABLE IF NOT EXISTS circle_resource_versions",
        "resource_id UUID NOT NULL REFERENCES circle_resources(id) ON DELETE CASCADE",
        "circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE",
        "version_label VARCHAR(80) NOT NULL DEFAULT 'v1'",
        "storage_object_path TEXT",
        "change_note TEXT",
        "is_current BOOLEAN NOT NULL DEFAULT FALSE",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_resource_versions_current",
        "WHERE is_current = TRUE",
        "INSERT INTO circle_resource_versions",
        "CASE WHEN r.storage_object_path IS NULL THEN r.url ELSE NULL END",
        "Backfilled current version from circle_resources",
        "Storage object paths are for backend delivery only and must not be exposed",
    ]:
        assert token in migration


def test_phase8_resource_version_review_schema_adds_auditable_review_state():
    migration = read("database/community/066_circle_resource_version_review.sql")

    for token in [
        "review_status VARCHAR(32) NOT NULL DEFAULT 'pending'",
        "reviewed_at TIMESTAMPTZ",
        "reviewed_by UUID",
        "review_note TEXT",
        "Backfilled approved current version",
        "review_status = 'superseded'",
        "circle_resource_versions_review_status_allowed",
        "CHECK (review_status IN ('pending', 'approved', 'rejected', 'superseded'))",
        "idx_circle_resource_versions_review_queue",
        "WHERE review_status = 'pending'",
        "idx_circle_resource_versions_reviewed_at",
    ]:
        assert token in migration


def test_phase8_backend_models_and_apis_expose_asset_circle_contracts():
    circles = read("backend/src/community/circles.rs")
    routes = read("backend/src/community/routes.rs")

    for field in [
        "pub related_asset_id: Option<Uuid>",
        "pub is_primary_asset_circle: bool",
        "pub holder_only_documents: bool",
        "pub asset_circle_tabs: Vec<String>",
    ]:
        assert field in circles

    for token in [
        "async fn user_has_asset_holding",
        "async fn ensure_asset_circle_access",
        "ensure_circle_read_access(state",
        "related_asset_id",
        "join_policy == \"holder_only\"",
        "crate::community::circles::check_token_gate(pool, &state.db, user_id, circle_id)",
        "user_has_asset_holding(&state.db, user_id, asset_id)",
        "You must hold the related asset to view this Circle.",
        "tokens_owned > 0",
        "async fn get_asset_circle",
        "async fn get_circle_resources",
        "async fn get_circle_resource_access",
        "async fn get_circle_resource_manage",
        "async fn create_circle_resource_manage",
        "async fn update_circle_resource_manage",
        "async fn get_circle_resource_versions",
        "async fn create_circle_resource_version",
        "async fn upload_circle_resource_version_file",
        "async fn get_circle_resource_version_access",
        "async fn restore_circle_resource_version",
        "CircleResourceVersionReviewReq",
        "normalize_circle_resource_version_review_action",
        "async fn review_circle_resource_version",
        "circle_resource_version_comparison",
        '"/api/community/assets/:id/circle"',
        '"/api/community/circles/:id/resources"',
        '"/api/community/circles/:id/resources/manage"',
        '"/api/community/circles/:id/resources/:resource_id/manage"',
        '"/api/community/circles/:id/resources/:resource_id/versions"',
        '"/api/community/circles/:id/resources/:resource_id/versions/upload"',
        '"/api/community/circles/:id/resources/:resource_id/versions/:version_id/access"',
        '"/api/community/circles/:id/resources/:resource_id/versions/:version_id/restore"',
        '"/api/community/circles/:id/resources/:resource_id/versions/:version_id/review"',
        '"/api/community/circles/:id/resources/:resource_id/access"',
        "circle_resources",
        "circle_resource_versions",
        "ensure_circle_resource_admin_access",
        "Circle resource management requires owner, admin, or platform admin access.",
        "normalize_circle_resource_source",
        "validate_circle_resource_storage_path_input",
        "storage_paths_hidden",
        "circle.resource.create",
        "circle.resource.update",
        "circle.resource.version.create",
        "circle.resource.version.upload",
        "circle.resource.version.restore",
        "circle.resource.version.review",
        "log_community_admin_action_tx",
        "circle_resource_version_delivery_url",
        "access_scope = 'holder_only' AND $3",
        "WHERE (related_asset_id = $1 OR token_gate_asset_id = $1)",
        "access_state",
        '"comparison": circle_resource_version_comparison(&versions)',
        '"review_status": row.try_get::<String, _>("review_status")',
        '"review_note": row.try_get::<Option<String>, _>("review_note")',
        "Review note is required when rejecting a resource version.",
    ]:
        assert token in routes


def test_phase8_resource_list_uses_delivery_endpoint_not_raw_storage_paths():
    routes = read("backend/src/community/routes.rs")
    js = read("frontend/platform/static/js/community-feed.js")
    storage = read("backend/src/storage/service.rs")

    for token in [
        "storage_object_path IS NOT NULL AS has_private_file",
        '"delivery_mode": if has_private_file { "api_stream" } else { "api_redirect" }',
        '"delivery_url": resource_id.map(|id| circle_resource_delivery_url(circle_id, id))',
        "fn circle_resource_delivery_url",
        "fn parse_circle_resource_storage_path",
        "fn safe_circle_resource_filename",
        "crate::storage::service::download_object",
        "header::CACHE_CONTROL",
        "header::CONTENT_DISPOSITION",
        "x-content-type-options",
        "Redirect::temporary(&url)",
    ]:
        assert token in routes

    for raw_leak in [
        '"url": row.try_get::<Option<String>, _>("url")',
        '"storage_object_path": row.try_get::<Option<String>, _>("storage_object_path")',
    ]:
        assert raw_leak not in routes

    for token in [
        "resource.delivery_url",
        "Version ${resource.version_label}",
        "rel = 'noopener noreferrer'",
        "target = '_blank'",
    ]:
        assert token in js

    assert "resource.url" not in js

    for token in [
        "POOOL_GCS_DOWNLOAD_FAKE_ROOT",
        "download_fake_gcs_object",
        "is_local_fallback_allowed()",
        "GCS object path is invalid.",
    ]:
        assert token in storage


def test_phase8_resource_management_ui_exposes_library_and_versions():
    page = read("frontend/platform/community-circle-settings.html")
    js = read("frontend/platform/static/js/community-circle-settings.js")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        "Resource Library",
        "ccs-resources-card",
        "ccs-resource-form",
        "ccs-resource-title",
        "ccs-resource-url",
        "ccs-resource-file",
        "ccs-resource-type",
        "ccs-resource-access",
        "ccs-resources-manage-list",
        "Storage paths stay server-side",
    ]:
        assert token in page

    for token in [
        "loadResourceLibrary",
        "renderResourceLibrary",
        "createResourceFromForm",
        "toggleResourceActive",
        "addResourceVersion",
        "uploadResourceVersionFile",
        "pickResourceVersionFile",
        "restoreResourceVersion",
        "reviewResourceVersion",
        "versionComparisonSummary",
        "toggleResourceVersions",
        "/resources/manage",
        "/manage",
        "/versions",
        "/versions/upload",
        "/restore",
        "/review",
        "data-ccs-resource-action=\"upload_version\"",
        "data-ccs-resource-version-action=\"restore\"",
        "data-ccs-resource-version-action=\"approve\"",
        "data-ccs-resource-version-action=\"reject\"",
        "Replace file",
        "Replacement uploaded",
        "Version restored",
        "Version reviewed",
        "data-ccs-resource-action",
        "canManageResources",
        "has_private_file",
    ]:
        assert token in js

    for token in [
        ".ccs-resource-form",
        ".ccs-resource-list",
        ".ccs-resource-row",
        ".ccs-resource-row__versions",
        ".ccs-resource-version-row",
        ".ccs-resource-version-compare",
        ".ccs-resource-version-row__review",
        ".ccs-resource-version-row__actions",
    ]:
        assert token in css


def test_phase8_resource_lifecycle_foundation_tracks_upload_retention_and_review():
    migration = read("database/community/062_circle_resource_lifecycle.sql")
    routes = read("backend/src/community/routes.rs")
    page = read("frontend/platform/community-circle-settings.html")
    js = read("frontend/platform/static/js/community-circle-settings.js")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        "upload_status VARCHAR(32) NOT NULL DEFAULT 'external'",
        "retention_policy VARCHAR(32) NOT NULL DEFAULT 'standard'",
        "review_required_at TIMESTAMPTZ",
        "reviewed_at TIMESTAMPTZ",
        "legal_hold BOOLEAN NOT NULL DEFAULT FALSE",
        "deleted_at TIMESTAMPTZ",
        "document_lifecycle_notes TEXT",
        "circle_resources_upload_status_allowed",
        "idx_circle_resources_lifecycle_review",
        "idx_circle_resources_retention_due",
    ]:
        assert token in migration

    for token in [
        "CircleResourceLifecycleReq",
        "CircleResourceUploadFields",
        "MAX_CIRCLE_RESOURCE_UPLOAD_BYTES",
        "normalize_circle_resource_upload_status",
        "normalize_circle_resource_retention_policy",
        "normalize_circle_resource_lifecycle_action",
        "parse_circle_resource_upload_datetime",
        '"/api/community/circles/:id/resources/:resource_id/lifecycle"',
        '"/api/community/circles/:id/resources/upload"',
        "async fn update_circle_resource_lifecycle",
        "async fn upload_circle_resource_file",
        "circle_resource_upload",
        "read_field_capped",
        "upload_private_with_markers",
        "PiiClass::B",
        "validate_asset_doc_mime",
        "sha256_hex",
        "circle.resource.upload",
        "circle.resource.lifecycle",
        '"mark_reviewed"',
        '"soft_delete"',
        '"legal_hold"',
        '"schedule_review"',
        '"upload_status": row.try_get::<String, _>("upload_status")',
        '"retention_policy": row.try_get::<String, _>("retention_policy")',
        '"document_lifecycle_notes": row.try_get::<Option<String>, _>("document_lifecycle_notes")',
    ]:
        assert token in routes

    for token in [
        "ccs-resource-storage-path",
        "ccs-resource-upload-status",
        "ccs-resource-retention-policy",
        "ccs-resource-retention-until",
        "ccs-resource-review-required-at",
        "ccs-resource-lifecycle-notes",
    ]:
        assert token in page

    for token in [
        "dateTimeLocalToIso",
        "runResourceLifecycleAction",
        "FormData",
        "/resources/upload",
        "/lifecycle",
        "data-ccs-resource-action=\"mark_reviewed\"",
        "data-ccs-resource-action=\"legal_hold\"",
        "data-ccs-resource-action=\"soft_delete\"",
        "document_lifecycle_notes",
        "retention_until",
        "review_required_at",
        "storage_object_path",
    ]:
        assert token in js

    for token in [
        ".ccs-resource-row__lifecycle",
        ".ccs-resource-row__note",
        "grid-template-columns: minmax(80px, 0.8fr) minmax(0, 1fr) minmax(90px, 0.6fr) minmax(110px, 0.7fr) auto auto;",
    ]:
        assert token in css


def test_phase8_resource_retention_worker_soft_deletes_due_documents():
    background = read("backend/src/community/background.rs")
    lib = read("backend/src/lib.rs")

    for token in [
        "CircleResourceRetentionSummary",
        "circle_resource_retention_worker",
        "POOOL_CIRCLE_RESOURCE_RETENTION_SECS",
        "run_circle_resource_retention_once",
        "retention_policy = 'delete_after_expiry'",
        "retention_until <= NOW()",
        "legal_hold = FALSE",
        "upload_status = 'deleted'",
        "deleted_at = NOW()",
        "retention_policy_due",
        "circle.resource.retention_soft_delete",
        "community_audit_logs",
        "resources_soft_deleted",
    ]:
        assert token in background

    assert "circle_resource_retention_worker" in lib


def test_phase8_resource_object_cleanup_deletes_soft_deleted_private_objects():
    migration = read("database/community/063_circle_resource_object_cleanup.sql")
    background = read("backend/src/community/background.rs")
    lib = read("backend/src/lib.rs")

    for token in [
        "storage_deleted_at TIMESTAMPTZ",
        "storage_delete_attempts INTEGER NOT NULL DEFAULT 0",
        "storage_delete_last_error TEXT",
        "storage_delete_next_attempt_at TIMESTAMPTZ",
        "idx_circle_resources_object_cleanup_due",
        "idx_circle_resource_versions_object_cleanup_due",
    ]:
        assert token in migration

    for token in [
        "CircleResourceObjectCleanupSummary",
        "circle_resource_object_cleanup_worker",
        "POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_SECS",
        "POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_GRACE_DAYS",
        "POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_LIMIT",
        "run_circle_resource_object_cleanup_once",
        "delete_circle_resource_storage_object",
        "extract_bucket_and_path",
        "delete_object",
        "mark_circle_resource_object_deleted",
        "mark_circle_resource_object_delete_failed",
        "storage_deleted_at = NOW()",
        "circle.resource.object_delete",
        "circle.resource.version.object_delete",
        "storage_delete_next_attempt_at",
        "record_storage_gcs_error",
        "resource.delete",
    ]:
        assert token in background

    for token in [
        "circle_resource_object_cleanup_worker",
        "GCS_BUCKET_NAME is not configured",
    ]:
        assert token in lib


def test_phase8_frontend_renders_permissioned_resources_widget():
    page = read("frontend/platform/community-circle.html")
    js = read("frontend/platform/static/js/community-feed.js")
    css = read("frontend/platform/static/css/community.css")

    for token in [
        'id="resources"',
        "circle-resources-list",
        "Permissioned",
    ]:
        assert token in page

    for token in [
        "renderCircleResources",
        "/api/community/circles/${id}/resources",
        "resource.access_scope",
        "resource.resource_type",
        "resource.delivery_url",
        "rel = 'noopener noreferrer'",
        "target = '_blank'",
    ]:
        assert token in js

    for token in [
        ".circle-resource-item__title",
        "a.circle-resource-item__title:hover",
    ]:
        assert token in css
