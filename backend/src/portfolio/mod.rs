pub mod models;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::{routing::get, Router};

/// Compose all portfolio-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        .route("/portfolio", get(page_portfolio))
        .route("/portfolio.html", get(page_portfolio))
        .route("/transactions", get(page_transactions))
        .route("/api/portfolio", get(get_portfolio_handler))
        .route("/api/portfolio/cancel", axum::routing::post(cancel_investment_handler))
}

