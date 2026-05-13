pub mod models;
pub mod public_assets;
pub mod routes;
/// Villa-Returns P3 — public investor performance KPIs.
#[allow(missing_docs)]
pub mod villa_performance;

use crate::auth::routes::AppState;
use axum::{routing::get, Router};

/// Compose all asset-domain routes into a single mountable [`Router`].
///
/// Mounted at `/` (inline, not nested) so paths are absolute:
/// `/marketplace`, `/portfolio`, `/commodity`, etc.
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        .route("/marketplace", get(page_marketplace))
        .route("/marketplace/tab", get(api_marketplace_tab))
        .route(
            "/commodities-marketplace",
            get(page_commodities_marketplace),
        )
        .route("/commodities-marketplace/tab", get(api_commodities_tab))
        .route("/property", get(page_property))
        .route("/property/:slug", get(page_property))
        // Public property detail page (no auth) — linked from landing-v2.html
        .route("/p/:slug", get(page_property_public))
        .route("/commodity", get(page_commodity))
        .route("/commodity/:slug", get(page_commodity))
        .route("/api/assets/search", get(api_asset_search))
        // Villa-Returns P3 — public investor KPIs.
        .route(
            "/api/villas/:asset_id/performance",
            get(villa_performance::api_villa_performance),
        )
        .route(
            "/api/villas/:asset_id/history",
            get(villa_performance::api_villa_history),
        )
}
