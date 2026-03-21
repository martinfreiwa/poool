/// Marketplace domain — secondary market trading engine for tokenized assets.
///
/// Architecture:
/// - `models.rs`     — Data structures (DB models, API DTOs, internal engine types)
/// - `validation.rs` — Order validation (balance, KYC, limits, wash-trade prevention)
/// - `orderbook.rs`  — Redis Sorted Set orderbook (ZADD/ZREM/snapshot/rebuild)
/// - `routes.rs`     — Axum HTTP handlers (thin — delegate to service)
/// - `service.rs`    — Core business logic (order creation, fee calc)
///
/// Future files (Tasks 3.5+):
/// - `matching.rs`    — Matching engine (Tokio task, price-time priority)
/// - `settlement.rs`  — 8-step ACID settlement transaction
/// - `p2p.rs`         — P2P/OTC offer system
/// - `charts.rs`      — Candlestick aggregation
/// - `background.rs`  — Background workers (expiry, reconciliation, sync)
pub mod models;
pub mod orderbook;
pub mod routes;
pub mod service;
pub mod validation;

use crate::auth::routes::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};

/// Compose all marketplace-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // ── Public Read APIs ────────────────────────────────
        .route(
            "/api/marketplace/:asset_id/orderbook",
            get(api_orderbook),
        )
        .route(
            "/api/marketplace/:asset_id/trades",
            get(api_recent_trades),
        )
        .route("/api/marketplace/:asset_id/ticker", get(api_ticker))
        // ── Authenticated Trading APIs ──────────────────────
        .route("/api/marketplace/orders", post(api_submit_order))
        .route("/api/marketplace/orders/mine", get(api_my_orders))
        .route(
            "/api/marketplace/orders/:order_id",
            delete(api_cancel_order),
        )
}
