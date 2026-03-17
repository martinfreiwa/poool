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
        .route("/privacy-policy", get(page_privacy_policy))
        .route("/currency-policy", get(page_currency_policy))
        .route("/cookies", get(page_cookies))
}
