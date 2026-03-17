use crate::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

/// Database access layer.
pub mod db;
/// Handlers for support requests.
pub mod handlers;
/// Domain models and DTOs.
pub mod models;
/// Service layer for business logic.
pub mod service;
/// SLA Tracker
pub mod sla;

/// Returns the support domain router.
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        // HTML Page
        .route("/support", get(handlers::page_support))
        // API Endpoints
        .nest(
            "/api/support",
            Router::new()
                .route(
                    "/tickets",
                    get(handlers::api_support_tickets_list)
                        .post(handlers::api_support_tickets_submit),
                )
                .route(
                    "/tickets/:ticket_id/reply",
                    post(handlers::api_support_ticket_reply),
                )
                .route(
                    "/tickets/:ticket_id/reopen",
                    put(handlers::api_support_ticket_reopen),
                ),
        )
        .with_state(state)
}
