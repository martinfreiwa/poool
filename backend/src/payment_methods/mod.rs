pub mod models;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::routing::delete;
use axum::{
    routing::{get, post},
    Router,
};

/// Compose all payment-methods routes into a single mountable [`Router`].
///
/// All routes are under `/api/payment-methods`.
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        .route("/api/payment-methods", get(list_payment_methods))
        .route("/api/payment-methods/card", post(handle_add_card))
        .route("/api/payment-methods/bank", post(handle_add_bank))
        .route("/api/payment-methods/:id", delete(delete_payment_method))
        .route(
            "/api/payment-methods/:id/default",
            post(set_default_payment_method),
        )
}
