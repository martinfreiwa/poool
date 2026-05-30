//! Legal page routes – terms, privacy policy, currency policy, and cookies.
#[allow(missing_docs)]
pub mod routes;

use crate::auth::routes::AppState;
use axum::{routing::get, Router};

/// Compose all legal-page routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        .route("/terms", get(page_terms))
        .route("/legal/terms", get(page_terms))
        .route("/privacy-policy", get(page_privacy_policy))
        .route("/privacy", get(page_privacy_policy))
        .route("/legal/privacy", get(page_privacy_policy))
        .route("/legal/privacy-policy", get(page_privacy_policy))
        .route("/currency-policy", get(page_currency_policy))
        .route("/legal/currency", get(page_currency_policy))
        .route("/legal/currency-policy", get(page_currency_policy))
        .route("/cookies", get(page_cookies))
        .route("/legal/cookies", get(page_cookies))
        .route("/imprint", get(page_imprint))
        .route("/legal/imprint", get(page_imprint))
        .route("/gdpr-data-request", get(page_gdpr_data_request))
        .route("/legal/gdpr-data-request", get(page_gdpr_data_request))
        .route("/aml-kyc-policy", get(page_aml_kyc_policy))
        .route("/legal/aml-kyc-policy", get(page_aml_kyc_policy))
}
