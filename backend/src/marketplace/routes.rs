/// Marketplace HTTP routes — thin handlers that delegate to the service layer.
///
/// Each handler is responsible ONLY for:
/// 1. Extracting data from the HTTP request (session, path params, body)
/// 2. Calling the appropriate service/validation function
/// 3. Formatting the HTTP response
///
/// NO business logic lives here.
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use super::{models, service, validation};
use crate::auth::routes::AppState;
use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── PUBLIC READ APIs ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// GET /api/marketplace/:asset_id/orderbook
///
/// Returns the aggregated orderbook snapshot for the given asset.
/// Public — no authentication required.
pub async fn api_orderbook(
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::ServiceUnavailable("Redis not available".into()))?;

    let mut snapshot =
        super::orderbook::get_orderbook_snapshot(redis, asset_id, None).await?;

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
    Path(asset_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let trades = service::get_recent_trades(&state.db, asset_id, 50).await?;
    Ok(Json(trades))
}

/// GET /api/marketplace/:asset_id/ticker
///
/// Returns 24-hour ticker data for an asset.
/// Public — no authentication required.
pub async fn api_ticker(
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let ticker = service::get_ticker(&state.db, asset_id).await?;
    Ok(Json(ticker))
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
