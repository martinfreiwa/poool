pub mod didit;
pub mod models;
pub mod provider;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post},
    Router,
};

/// Compose all KYC-domain routes into a single mountable [`Router`].
///
/// HTML page routes are absolute (mounted at `/`);
/// API routes live under `/api/kyc` and `/api/webhooks/kyc`.
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // HTML page
        .route("/kyc", get(page_kyc))
        // JSON API
        .route("/api/kyc/status", get(get_status))
        .route("/api/kyc/submit", post(submit))
        .route("/api/kyc/initiate", post(initiate))
        .route("/api/kyc/provider", get(get_provider_info))
        // SIWE wallet binding — sovereign-wallet onboarding flow.
        .route("/api/kyc/wallet/challenge", post(wallet_challenge))
        .route("/api/kyc/wallet/bind", post(wallet_bind))
        // Webhook (unauthenticated – signature-verified internally)
        .route("/api/webhooks/kyc/didit", post(didit_webhook))
}
