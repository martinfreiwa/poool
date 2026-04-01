pub mod models;
pub mod routes;
pub mod service;
pub mod workers;

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
        // Affiliate promo page (new system — Phase 19)
        .route("/affiliate", get(page_affiliate_promo))
        .route("/affiliate/onboarding", get(page_affiliate_onboarding))
        .route("/affiliate/dashboard", get(page_affiliate_dashboard))
        .route("/affiliate/referrals", get(page_affiliate_referrals))
        .route("/affiliate/materials", get(page_affiliate_materials))
        .route("/affiliate/settings", get(page_affiliate_settings))
        // API - Affiliate Onboarding
        .route("/api/affiliate/onboarding/submit", axum::routing::post(submit_affiliate_onboarding_handler))
        // API - Affiliate Dashboard
        .route("/api/affiliate/dashboard", get(get_affiliate_dashboard_handler))
        .route("/api/affiliate/payout/request", axum::routing::post(api_affiliate_payout_request))
        .route("/api/affiliate/subid-stats", get(api_affiliate_subid_stats))
        .route("/api/affiliate/postback", axum::routing::post(api_affiliate_postback_save))
        .route("/api/affiliate/referrals", get(api_affiliate_referrals_list))
        // GAP-08: Policy re-acceptance for updated versions
        .route("/api/affiliate/policy-reaccept", axum::routing::post(api_affiliate_policy_reaccept))
        // GAP-14: Commission export with pagination and date filter
        .route("/api/affiliate/commissions/export", get(api_affiliate_commissions_export))
        // GAP-10: Tax document upload (required for payout release)
        .route("/api/affiliate/tax-document", axum::routing::post(api_affiliate_upload_tax_document))
        // GAP-11: Custom marketing materials upload
        .route("/api/affiliate/materials/upload", axum::routing::post(api_affiliate_upload_material))
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
