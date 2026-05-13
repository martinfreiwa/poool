pub mod models;
pub mod routes;
pub mod service;
/// Villa-Returns P3 — aggregated portfolio KPIs for the dashboard.
#[allow(missing_docs)]
pub mod villa_summary;

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
        .route(
            "/api/portfolio/cancel",
            axum::routing::post(cancel_investment_handler),
        )
        .route(
            "/api/investors/me/portfolio-villa-summary",
            get(villa_summary::api_portfolio_villa_summary),
        )
        .route(
            "/api/investors/me/positions-nav",
            get(villa_summary::api_positions_nav),
        )
}
