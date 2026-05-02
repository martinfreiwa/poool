/// Marketplace domain — secondary market trading engine for tokenized assets.
///
/// Architecture:
/// - `models.rs`      — Data structures (DB models, API DTOs, internal engine types)
/// - `validation.rs`  — Order validation (balance, KYC, limits, wash-trade prevention)
/// - `orderbook.rs`   — Redis Sorted Set orderbook (ZADD/ZREM/snapshot/rebuild)
/// - `routes.rs`      — Axum HTTP handlers (thin — delegate to service)
/// - `service.rs`     — Core business logic (order creation, fee calc)
/// - `matching.rs`    — Matching engine (Tokio task, price-time priority)
/// - `settlement.rs`  — 8-step ACID settlement transaction
/// - `background.rs`  — Background workers (order expiry, Redis sync, price snapshots)
/// - `websocket.rs`   — Real-time WebSocket server (orderbook, trades, ticker)
///
/// - `p2p.rs`         — P2P/OTC offer system
/// - `charts.rs`      — Candlestick aggregation
pub mod background;
pub mod charts;
pub mod invariants;
pub mod matching;
pub mod models;
pub mod orderbook;
pub mod p2p;
pub mod routes;
pub mod service;
pub mod settlement;
pub mod validation;
pub mod websocket;

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
            "/api/marketplace/secondary/assets",
            get(api_secondary_assets),
        )
        .route("/api/marketplace/:asset_id/orderbook", get(api_orderbook))
        .route("/api/marketplace/:asset_id/trades", get(api_recent_trades))
        .route("/api/marketplace/:asset_id/ticker", get(api_ticker))
        .route("/api/marketplace/:asset_id/fee-rate", get(api_fee_rate))
        // ── Candlestick Chart API ────────────────────────────
        .route("/api/marketplace/:asset_id/candles", get(api_candles))
        .route(
            "/api/marketplace/:asset_id/chart-summary",
            get(api_chart_summary),
        )
        // ── WebSocket (real-time market data) ────────────────
        .route("/ws/market/:asset_id", get(websocket::ws_market_handler))
        // ── Authenticated Trading APIs ──────────────────────
        .route("/api/marketplace/orders", post(api_submit_order))
        .route("/api/marketplace/orders/mine", get(api_my_orders))
        .route("/api/marketplace/trades/mine", get(api_my_trades))
        .route("/api/marketplace/tax-export", get(api_export_tax_report))
        .route(
            "/api/marketplace/orders/:order_id",
            delete(api_cancel_order),
        )
        // ── P2P/OTC Offer APIs ──────────────────────────────
        .route("/api/marketplace/p2p/offers", post(api_create_p2p_offer))
        .route(
            "/api/marketplace/p2p/offers/incoming",
            get(api_incoming_offers),
        )
        .route(
            "/api/marketplace/p2p/offers/outgoing",
            get(api_outgoing_offers),
        )
        .route(
            "/api/marketplace/p2p/offers/:offer_id/respond",
            post(api_respond_p2p_offer),
        )
        .route(
            "/api/marketplace/p2p/offers/:offer_id",
            delete(api_cancel_p2p_offer),
        )
        .route("/api/marketplace/:asset_id/p2p", get(api_asset_p2p_offers))
}
