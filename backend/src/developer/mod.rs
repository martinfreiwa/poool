pub mod change_requests;
/// Villa-Returns P2 — DeveloperUser extractor + asset-link enforcement.
#[allow(missing_docs)]
pub mod extractors;
/// Villa-Returns C3 — developer forecast suggestions + annual summary.
#[allow(missing_docs)]
pub mod forecast_suggestions;
pub mod fragments;
pub mod models;
pub mod routes;
pub mod service;
/// Villa-Returns C3 — developer CapEx submission.
#[allow(missing_docs)]
pub mod villa_capex;
/// Villa-Returns P2 — developer endpoints for monthly operations submission.
#[allow(missing_docs)]
pub mod villa_operations;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

/// Compose all developer-flow page routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use fragments::*;
    use routes::*;
    Router::new()
        .route(
            "/developer",
            get(|| async { axum::response::Redirect::to("/developer/dashboard") }),
        )
        .route("/developer/dashboard", get(page_developer_dashboard))
        .route("/developer/assets", get(page_developer_assets))
        .route(
            "/developer/affiliate-team",
            get(page_developer_affiliate_team),
        )
        .route(
            "/developer/affiliate-team/customers",
            get(page_developer_affiliate_team_customers),
        )
        .route(
            "/developer/affiliate-team/products",
            get(page_developer_affiliate_team_products),
        )
        .route(
            "/developer/affiliate-team/settings",
            get(page_developer_affiliate_team_settings),
        )
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
            "/developer/onboarding",
            get(routes::page_developer_onboarding),
        )
        .route(
            "/api/developer/apply",
            post(routes::api_developer_apply),
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
        .route("/developer/support", get(page_developer_support))
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
        .route(
            "/developer/ranking",
            get(routes::page_developer_ranking),
        )
        // ── Villa-Returns P2 — developer operations workflow ─────────
        .route(
            "/developer/operations",
            get(routes::page_developer_operations_dashboard),
        )
        .route(
            "/developer/villas/:asset_id/operations/new",
            get(routes::page_developer_operations_submit),
        )
        .route(
            "/api/developer/operations/dashboard",
            get(villa_operations::api_developer_operations_dashboard),
        )
        .route(
            "/api/developer/villas/:asset_id/operations",
            post(villa_operations::api_developer_villa_operations_create)
                .get(villa_operations::api_developer_villa_operations_list),
        )
        .route(
            "/api/developer/villas/:asset_id/operations/:log_id",
            put(villa_operations::api_developer_villa_operations_update),
        )
        .route(
            "/api/developer/villas/:asset_id/operations/:log_id/submit",
            put(villa_operations::api_developer_villa_operations_submit),
        )
        .route(
            "/api/developer/villas/:asset_id/operations/:log_id/documents",
            post(villa_operations::api_developer_villa_operations_upload_document)
                .get(villa_operations::api_developer_villa_operations_documents_list),
        )
        .route(
            "/api/developer/villas/:asset_id/annual/:year/documents",
            post(villa_operations::api_developer_villa_annual_documents_upload)
                .get(villa_operations::api_developer_villa_annual_documents_list),
        )
        .route(
            "/api/developer/villas/:asset_id/asset-config",
            get(villa_operations::api_developer_asset_config),
        )
        // ── Villa-Returns C3 — developer annual data ─────────────────
        .route(
            "/developer/villas/:asset_id/annual/:year",
            get(routes::page_developer_annual_data),
        )
        .route(
            "/api/developer/villas/:asset_id/capex",
            post(villa_capex::api_developer_villa_capex_create)
                .get(villa_capex::api_developer_villa_capex_list),
        )
        .route(
            "/api/developer/villas/:asset_id/forecast/:year/suggest",
            post(forecast_suggestions::api_developer_forecast_suggest),
        )
        .route(
            "/api/developer/villas/:asset_id/forecast/:year/suggestions",
            get(forecast_suggestions::api_developer_forecast_suggestions_list),
        )
        .route(
            "/api/developer/villas/:asset_id/annual/:year/summary",
            get(forecast_suggestions::api_developer_annual_summary),
        )
}
