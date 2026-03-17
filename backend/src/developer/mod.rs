pub mod change_requests;
pub mod fragments;
pub mod models;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Compose all developer-flow page routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use fragments::*;
    use routes::*;
    Router::new()
        .route("/developer/dashboard", get(page_developer_dashboard))
        .route("/developer/assets", get(page_developer_assets))
        .route("/developer/add-asset", get(page_developer_add_asset))
        .route(
            "/developer/property-content",
            get(page_developer_property_content),
        )
        .route(
            "/developer/document-upload-step3",
            get(page_developer_document_upload),
        )
        .route(
            "/developer/application-form",
            get(page_developer_application_form),
        )
        .route(
            "/developer/submission-success",
            get(page_developer_submission_success),
        )
        // HTMX Fragment endpoints for dynamic UI switches (Phase 11.1)
        .route("/developer/dashboard/fragments/chart", get(fragment_chart))
        .route(
            "/developer/dashboard/fragments/assets",
            get(fragment_assets),
        )
        .route("/developer/asset-detail", get(page_developer_asset_detail))
        .route("/developer/settings", get(page_developer_settings))
        .route("/developer/submissions", get(page_developer_submissions))
        // API endpoints
        .route(
            "/api/developer/dashboard/stats",
            get(api_developer_dashboard_stats),
        )
        .route("/api/developer/draft", post(api_developer_create_draft))
        .route(
            "/api/developer/draft/:id",
            get(api_developer_get_draft)
                .put(api_developer_update_draft)
                .delete(api_developer_delete_draft),
        )
        .route("/api/developer/drafts", get(api_developer_list_drafts))
        .route(
            "/api/developer/draft/:id/submit",
            post(api_developer_submit_draft),
        )
        .route(
            "/api/developer/draft/:id/duplicate",
            post(api_developer_duplicate_draft),
        )
        .route(
            "/api/developer/assets/:id",
            get(api_developer_asset_detail).put(change_requests::submit_edit),
        )
        .route(
            "/api/developer/assets/:id/pending-changes",
            get(change_requests::get_pending),
        )
}
