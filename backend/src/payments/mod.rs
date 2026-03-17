/// Payments domain – handles deposits, withdrawals, and payment provider integration.
pub mod models;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};

/// Compose all payments-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // HTML pages — checkout needs a larger body limit for proof-of-transfer uploads
        .route(
            "/checkout",
            get(checkout_page)
                .post(handle_checkout)
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024)), // 10 MB
        )
        // User-facing API
        .route("/api/payments/deposit", post(initiate_deposit))
        .route("/api/webhooks/payments", post(payment_webhook))
        .route("/api/invoices", get(list_invoices))
        .route("/api/deposits", get(list_deposits))
        .route("/api/wallets", get(list_wallets))
        .route("/api/payments/bank-details", get(get_bank_details))
        .route("/api/orders/latest", get(api_latest_order))
        // Admin order management
        .route(
            "/api/admin/orders/:id/approve",
            post(api_admin_approve_order),
        )
        .route("/api/admin/orders/:id/reject", post(api_admin_reject_order))
}
