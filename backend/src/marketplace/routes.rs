/// Marketplace HTTP routes — thin handlers that delegate to the service layer.
///
/// Each handler is responsible ONLY for:
/// 1. Extracting data from the HTTP request (session, path params, body)
/// 2. Calling the appropriate service/validation function
/// 3. Formatting the HTTP response
///
/// NO business logic lives here.
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use super::{models, service, validation};
use crate::auth::routes::AppState;
use crate::error::AppError;

use sqlx::PgPool;

pub async fn resolve_asset_id(pool: &PgPool, id_or_slug: &str) -> Result<Uuid, AppError> {
    if let Ok(uuid) = Uuid::parse_str(id_or_slug) {
        Ok(uuid)
    } else {
        let asset_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM assets WHERE slug = $1")
            .bind(id_or_slug)
            .fetch_optional(pool)
            .await
            .map_err(AppError::Database)?;
        asset_id.ok_or_else(|| AppError::NotFound("Asset not found".into()))
    }
}

// ═══════════════════════════════════════════════════════════════
// ── PUBLIC READ APIs ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// GET /api/marketplace/:asset_id/orderbook
///
/// Returns the aggregated orderbook snapshot for the given asset.
/// Public — no authentication required.
pub async fn api_orderbook(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let mut snapshot = match state.redis.as_ref() {
        Some(redis) => match super::orderbook::get_orderbook_snapshot(redis, asset_id, None).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Redis orderbook unavailable ({}). Falling back to PostgreSQL.", e);
                super::service::get_orderbook_snapshot_from_db(&state.db, asset_id, None).await?
            }
        },
        None => super::service::get_orderbook_snapshot_from_db(&state.db, asset_id, None).await?,
    };

    // Fill last_price from trade_history
    let last_price: Option<i64> = sqlx::query_scalar(
        "SELECT price_cents FROM trade_history WHERE asset_id = $1 ORDER BY executed_at DESC LIMIT 1",
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    snapshot.last_price_cents = last_price;

    Ok(Json(snapshot))
}

/// GET /api/marketplace/:asset_id/trades
///
/// Returns the most recent trades for an asset (trade tape).
/// Public — no authentication required.
pub async fn api_recent_trades(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let trades = service::get_recent_trades(&state.db, asset_id, 50).await?;
    Ok(Json(trades))
}

/// GET /api/marketplace/:asset_id/ticker
///
/// Returns 24-hour ticker data for an asset.
/// Public — no authentication required.
pub async fn api_ticker(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let ticker = service::get_ticker(&state.db, asset_id).await?;
    Ok(Json(ticker))
}

/// GET /api/marketplace/secondary/assets
///
/// Returns all assets currently available on the secondary market.
/// Public — no authentication required.
pub async fn api_secondary_assets(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let assets = service::get_secondary_assets(&state.db).await?;
    Ok(Json(assets))
}

// ═══════════════════════════════════════════════════════════════
// ── AUTHENTICATED TRADING APIs ────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// POST /api/marketplace/orders
///
/// Submit a new buy or sell order. Requires authentication.
///
/// Flow:
/// 1. Validate input fields
/// 2. Verify KYC status
/// 3. Check rate limits
/// 4. Begin DB transaction
/// 5. Check balance/tokens (FOR UPDATE)
/// 6. Create order in DB
/// 7. Place hold on balance/tokens
/// 8. Insert into Redis orderbook
/// 9. Return order confirmation
pub async fn api_submit_order(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<models::SubmitOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Authenticate
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    // 2. Validate request fields
    validation::validate_order_fields(&body)?;

    // 3. Execute order creation (all business logic in service layer)
    let response = service::create_order(&state.db, state.redis.as_ref(), user.id, body).await?;

    Ok(Json(response))
}

/// GET /api/marketplace/orders/mine
///
/// Get the authenticated user's orders (open and recent).
pub async fn api_my_orders(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let orders = service::get_user_orders(&state.db, user.id).await?;
    Ok(Json(orders))
}

/// GET /api/marketplace/trades/mine
///
/// Get the authenticated user's trade history.
pub async fn api_my_trades(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let trades = service::get_user_trades_history(&state.db, user.id).await?;
    Ok(Json(trades))
}

/// DELETE /api/marketplace/orders/:order_id
///
/// Cancel an open order. Requires authentication. The order must
/// belong to the authenticated user and must be in "open" or "partially_filled" status.
pub async fn api_cancel_order(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    service::cancel_order(&state.db, state.redis.as_ref(), user.id, order_id).await?;

    Ok(Json(serde_json::json!({
        "status": "cancelled",
        "order_id": order_id,
        "message": "Order cancelled successfully."
    })))
}

// ═══════════════════════════════════════════════════════════════
// ── CANDLESTICK CHART APIs ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// GET /api/marketplace/:asset_id/candles?interval=1h&from=&to=&limit=
///
/// Returns OHLCV candlestick data for charting.
pub async fn api_candles(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
    Query(query): Query<super::charts::CandleQuery>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;
    let response = super::charts::get_candles(&state.db, asset_id, query).await?;

    Ok(Json(response))
}

/// GET /api/marketplace/:asset_id/chart-summary
///
/// Returns 24h chart summary (last price, high, low, volume, change).
pub async fn api_chart_summary(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let summary = super::charts::get_chart_summary(&state.db, asset_id).await?;
    Ok(Json(summary))
}

// ═══════════════════════════════════════════════════════════════
// ── P2P/OTC OFFER APIs ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// POST /api/marketplace/p2p/offers
///
/// Create a new P2P offer. Requires authentication.
pub async fn api_create_p2p_offer(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<super::p2p::CreateP2POfferRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let response = super::p2p::create_offer(&state.db, user.id, body).await?;
    Ok(Json(response))
}

/// POST /api/marketplace/p2p/offers/:offer_id/respond
///
/// Respond to a P2P offer (accept, decline, or counter).
pub async fn api_respond_p2p_offer(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(offer_id): Path<Uuid>,
    Json(body): Json<super::p2p::RespondP2POfferRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let response = super::p2p::respond_to_offer(&state.db, user.id, offer_id, body).await?;
    Ok(Json(response))
}

/// DELETE /api/marketplace/p2p/offers/:offer_id
///
/// Cancel a pending P2P offer (maker only).
pub async fn api_cancel_p2p_offer(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(offer_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let response = super::p2p::cancel_offer(&state.db, user.id, offer_id).await?;
    Ok(Json(response))
}

/// GET /api/marketplace/p2p/offers/incoming
///
/// Get incoming offers for the authenticated user.
pub async fn api_incoming_offers(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let offers = super::p2p::get_incoming_offers(&state.db, user.id).await?;
    Ok(Json(offers))
}

/// GET /api/marketplace/p2p/offers/outgoing
///
/// Get outgoing offers for the authenticated user.
pub async fn api_outgoing_offers(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let user = crate::auth::middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Authentication required.".into()))?;

    let offers = super::p2p::get_outgoing_offers(&state.db, user.id).await?;
    Ok(Json(offers))
}

/// GET /api/marketplace/:asset_id/p2p
///
/// Get pending P2P offers for a specific asset (public).
pub async fn api_asset_p2p_offers(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let asset_id = resolve_asset_id(&state.db, &id_or_slug).await?;

    let offers = super::p2p::get_asset_offers(&state.db, asset_id).await?;
    Ok(Json(offers))
}
