pub mod models;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::{routing::get, Router};

/// Compose all rewards-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // HTML pages
        .route("/rewards", get(page_rewards))
        .route("/rewards.html", get(page_rewards))
        .route("/rewards/:code", get(page_referral_landing))
        .route("/r/:code", get(page_referral_landing))
        .route("/tier", get(page_tier))
        // JSON API
        .route("/api/rewards", get(get_rewards_handler))
        .route("/api/rewards/tiers", get(get_tiers_handler))
        .route("/api/rewards/campaigns", get(get_campaigns_handler))
        .route(
            "/api/rewards/payout-settings",
            get(get_payout_settings_handler).post(save_payout_settings_handler),
        )
        .route("/api/rewards/commissions", get(list_commissions_handler))
}
