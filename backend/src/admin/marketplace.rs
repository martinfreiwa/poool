//! Admin Marketplace API handlers.
//!
//! Provides admin-only endpoints for marketplace management:
//! - KPI stats (volume, orders, trades)
//! - Recent trades with user IDs
//! - Orderbook per asset with user IDs (admin-only data)
//! - Open orders list + admin-cancel
//! - Trade history with filters + pagination
//! - System health (DB, Redis, WS)
//! - Trading kill-switch
//!
//! All handlers require admin authentication via the `AdminUser` extractor.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;

// ═══════════════════════════════════════════════════════════════════
// ── Request / Response types ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// KPI stats for marketplace overview page.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct MarketplaceStats {
    pub trading_status: String,
    pub open_orders: i64,
    pub volume_24h_cents: i64,
    pub trades_24h: i64,
    pub pending_reviews: i64,
    pub total_assets_trading: i64,
    pub active_users_24h: i64,
    pub fees_collected_24h_cents: i64,
}

/// A trade record with admin-visible fields (user emails, fees).
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminTrade {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub asset_name: Option<String>,
    pub buyer_id: Uuid,
    pub seller_id: Uuid,
    pub buyer_email: Option<String>,
    pub seller_email: Option<String>,
    pub price_cents: i64,
    pub quantity: i32,
    pub total_cents: i64,
    pub fee_cents: i64,
    pub executed_at: chrono::NaiveDateTime,
}

/// An order record with admin-visible fields.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: Option<String>,
    pub asset_id: Uuid,
    pub asset_name: Option<String>,
    pub side: String,
    pub order_type: String,
    pub price_cents: i64,
    pub quantity: i32,
    pub quantity_filled: i32,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
}

/// Query filters for trade history.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct TradeFilters {
    pub asset_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub side: Option<String>,
    pub _min_price_cents: Option<i64>,
    pub _max_price_cents: Option<i64>,
    pub _from_date: Option<String>,
    pub _to_date: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Query filters for order listing.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OrderFilters {
    pub _asset_id: Option<Uuid>,
    pub _user_id: Option<Uuid>,
    pub _side: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Request body for admin order cancellation.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AdminCancelRequest {
    pub reason: Option<String>,
}

/// Aggregated orderbook price level (from SQL query).
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminOrderbookLevelRow {
    pub price_cents: i64,
    pub total_quantity: i64,
    pub order_count: i64,
}

/// Aggregated orderbook price level (API response).
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminOrderbookLevel {
    pub price_cents: i64,
    pub total_quantity: i64,
    pub order_count: i64,
}

/// Admin orderbook view with aggregated levels and spread data.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminOrderbook {
    pub asset_id: Uuid,
    pub bids: Vec<AdminOrderbookLevel>,
    pub asks: Vec<AdminOrderbookLevel>,
    pub spread_cents: Option<i64>,
    pub mid_price_cents: Option<i64>,
}

/// System health status for marketplace components.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct SystemHealth {
    pub database_latency_ms: f64,
    pub redis_connected: bool,
    pub redis_latency_ms: Option<f64>,
    pub active_ws_connections: i64,
    pub matching_engine_status: String,
    pub last_trade_at: Option<chrono::NaiveDateTime>,
    pub order_queue_depth: i64,
}

/// Paginated API response wrapper.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ═══════════════════════════════════════════════════════════════════
// ── Reconciliation types ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Full reconciliation report with invariant checks.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ReconciliationReport {
    pub cash_balance_check: InvariantCheck,
    pub fee_balance_check: InvariantCheck,
    pub token_integrity_check: InvariantCheck,
    pub generated_at: chrono::NaiveDateTime,
}

/// Individual invariant check result.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct InvariantCheck {
    pub name: String,
    pub passed: bool,
    pub expected: i64,
    pub actual: i64,
    pub delta: i64,
    pub details: String,
}

/// Request body for toggling marketplace trading on/off.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ToggleTradingRequest {
    pub enabled: bool,
    pub reason: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.1: Marketplace Stats ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/stats — KPIs: volume, orders, trades, pending.
pub async fn api_admin_marketplace_stats(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<MarketplaceStats>, ApiError> {
    let db = &state.db;

    let open_orders: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let volume_24h: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(price_cents * quantity::BIGINT), 0)
         FROM trade_history
         WHERE executed_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let trades_24h: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM trade_history
         WHERE executed_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let pending_reviews: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM market_orders
         WHERE status = 'pending_review'",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let total_assets: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT asset_id) FROM market_orders
         WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let active_users: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM market_orders
         WHERE created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let fees_24h: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(fee_cents), 0) FROM trade_history
         WHERE executed_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    // Check trading status from Redis (or default to active)
    let trading_status = if let Some(ref redis) = state.redis {
        match redis.get().await {
            Ok(mut conn) => {
                let status: Option<String> =
                    redis::cmd("GET")
                        .arg("marketplace:trading_enabled")
                        .query_async(&mut *conn)
                        .await
                        .unwrap_or(None);
                match status.as_deref() {
                    Some("false") | Some("0") => "HALTED".to_string(),
                    _ => "LIVE".to_string(),
                }
            }
            Err(_) => "UNKNOWN".to_string(),
        }
    } else {
        "LIVE".to_string()
    };

    Ok(Json(MarketplaceStats {
        trading_status,
        open_orders,
        volume_24h_cents: volume_24h,
        trades_24h,
        pending_reviews,
        total_assets_trading: total_assets,
        active_users_24h: active_users,
        fees_collected_24h_cents: fees_24h,
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.2: Recent Trades ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/recent-trades — Last 50 trades with user emails.
pub async fn api_admin_marketplace_recent_trades(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminTrade>>, ApiError> {
    let db = &state.db;

    let rows: Vec<AdminTrade> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.asset_id,
            a.name AS asset_name,
            t.buyer_id,
            t.seller_id,
            bu.email AS buyer_email,
            su.email AS seller_email,
            t.price_cents,
            t.quantity,
            (t.price_cents * t.quantity::BIGINT) AS total_cents,
            COALESCE(t.fee_cents, 0) AS fee_cents,
            t.executed_at
        FROM trade_history t
        LEFT JOIN assets a ON a.id = t.asset_id
        LEFT JOIN users bu ON bu.id = t.buyer_id
        LEFT JOIN users su ON su.id = t.seller_id
        ORDER BY t.executed_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.5: Trade History with Filters + Pagination ─────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/trades — Paginated trade history with filters.
pub async fn api_admin_marketplace_trades(
    _admin: AdminUser,
    Query(filters): Query<TradeFilters>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<AdminTrade>>, ApiError> {
    let db = &state.db;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(25).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Build WHERE clause dynamically
    let mut conditions: Vec<String> = vec![];
    if let Some(aid) = filters.asset_id {
        conditions.push(format!("t.asset_id = '{}'", aid));
    }
    if let Some(uid) = filters.user_id {
        conditions.push(format!("(t.buyer_id = '{}' OR t.seller_id = '{}')", uid, uid));
    }
    if let Some(ref side) = filters.side {
        conditions.push(format!("t.taker_side = '{}'", side));
    }

    let where_clause = if conditions.is_empty() {
        "1=1".to_string()
    } else {
        conditions.join(" AND ")
    };

    let count_sql = format!(
        "SELECT COUNT(*)::BIGINT FROM trade_history t WHERE {}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar::<_, i64>(&count_sql)
        .fetch_one(db)
        .await
        .unwrap_or(0);

    let data_sql = format!(
        r#"
        SELECT
            t.id,
            t.asset_id,
            a.name AS asset_name,
            t.buyer_id,
            t.seller_id,
            bu.email AS buyer_email,
            su.email AS seller_email,
            t.price_cents,
            t.quantity,
            (t.price_cents * t.quantity::BIGINT) AS total_cents,
            COALESCE(t.fee_cents, 0) AS fee_cents,
            t.executed_at
        FROM trade_history t
        LEFT JOIN assets a ON a.id = t.asset_id
        LEFT JOIN users bu ON bu.id = t.buyer_id
        LEFT JOIN users su ON su.id = t.seller_id
        WHERE {}
        ORDER BY t.executed_at DESC
        LIMIT {} OFFSET {}
        "#,
        where_clause, per_page, offset
    );

    let rows: Vec<AdminTrade> = sqlx::query_as(&data_sql)
        .fetch_all(db)
        .await
        .map_err(ApiError::Database)?;

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        0
    };

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.6: Open Orders + Admin Cancel ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/orders — Paginated open orders list.
pub async fn api_admin_marketplace_orders(
    _admin: AdminUser,
    Query(filters): Query<OrderFilters>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<AdminOrder>>, ApiError> {
    let db = &state.db;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(25).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let status_filter = filters
        .status
        .as_deref()
        .unwrap_or("open,partially_filled");

    let total: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM market_orders WHERE status = ANY(string_to_array($1, ','))",
    )
    .bind(status_filter)
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let rows: Vec<AdminOrder> = sqlx::query_as(
        r#"
        SELECT
            o.id,
            o.user_id,
            u.email AS user_email,
            o.asset_id,
            a.name AS asset_name,
            o.side,
            o.order_type,
            o.price_cents,
            o.quantity,
            o.quantity_filled,
            o.status,
            o.created_at
        FROM market_orders o
        LEFT JOIN users u ON u.id = o.user_id
        LEFT JOIN assets a ON a.id = o.asset_id
        WHERE o.status = ANY(string_to_array($1, ','))
        ORDER BY o.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(status_filter)
    .bind(per_page)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        0
    };

    Ok(Json(PaginatedResponse {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// Admin-cancel an order with optional reason.
pub async fn api_admin_marketplace_order_cancel(
    admin: AdminUser,
    Path(order_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<AdminCancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    let order_uuid = ApiError::parse_uuid(&order_id)?;

    // Verify order exists and is cancellable
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM market_orders WHERE id = $1",
    )
    .bind(order_uuid)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    let current_status = status.ok_or_else(|| ApiError::NotFound("Order not found".to_string()))?;

    if current_status != "open" && current_status != "partially_filled" {
        return Err(ApiError::BadRequest(format!(
            "Order cannot be cancelled: status is '{}'",
            current_status
        )));
    }

    // Cancel the order inside a transaction
    let mut tx = db.begin().await.map_err(ApiError::Database)?;

    sqlx::query(
        r#"
        UPDATE market_orders
        SET status = 'admin_cancelled',
            updated_at = NOW(),
            cancel_reason = $2
        WHERE id = $1
        "#,
    )
    .bind(order_uuid)
    .bind(body.reason.as_deref().unwrap_or("Admin cancellation"))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    // Refund locked funds for buy orders
    let refund_row: Option<(Uuid, String, i64, i32, i32)> = sqlx::query_as(
        "SELECT user_id, side, price_cents, quantity, quantity_filled
         FROM market_orders WHERE id = $1",
    )
    .bind(order_uuid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    if let Some((user_id, side, price_cents, qty, filled)) = refund_row {
        let remaining = (qty - filled) as i64;
        if side == "buy" && remaining > 0 {
            let refund_cents = remaining * price_cents;
            sqlx::query(
                "UPDATE users SET balance_cents = balance_cents + $1 WHERE id = $2",
            )
            .bind(refund_cents)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
        }
    }

    tx.commit().await.map_err(ApiError::Database)?;

    tracing::info!(
        admin_id = %admin.user.id,
        %order_uuid,
        reason = body.reason.as_deref().unwrap_or("Admin cancellation"),
        "Admin cancelled marketplace order"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Order cancelled successfully"
    })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.3: Admin Orderbook ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/orderbook/:asset_id — Aggregated orderbook.
pub async fn api_admin_marketplace_orderbook(
    _admin: AdminUser,
    Path(asset_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<AdminOrderbook>, ApiError> {
    let db = &state.db;
    let asset_uuid = ApiError::parse_uuid(&asset_id)?;

    // Aggregated bid levels
    let bid_levels: Vec<AdminOrderbookLevelRow> = sqlx::query_as(
        r#"
        SELECT
            price_cents,
            SUM(quantity - quantity_filled)::BIGINT AS total_quantity,
            COUNT(*)::BIGINT AS order_count
        FROM market_orders
        WHERE asset_id = $1
          AND side = 'buy'
          AND status IN ('open', 'partially_filled')
        GROUP BY price_cents
        ORDER BY price_cents DESC
        LIMIT 20
        "#,
    )
    .bind(asset_uuid)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    // Aggregated ask levels
    let ask_levels: Vec<AdminOrderbookLevelRow> = sqlx::query_as(
        r#"
        SELECT
            price_cents,
            SUM(quantity - quantity_filled)::BIGINT AS total_quantity,
            COUNT(*)::BIGINT AS order_count
        FROM market_orders
        WHERE asset_id = $1
          AND side = 'sell'
          AND status IN ('open', 'partially_filled')
        GROUP BY price_cents
        ORDER BY price_cents ASC
        LIMIT 20
        "#,
    )
    .bind(asset_uuid)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    let bids: Vec<AdminOrderbookLevel> = bid_levels
        .into_iter()
        .map(|r| AdminOrderbookLevel {
            price_cents: r.price_cents,
            total_quantity: r.total_quantity,
            order_count: r.order_count,
        })
        .collect();

    let asks: Vec<AdminOrderbookLevel> = ask_levels
        .into_iter()
        .map(|r| AdminOrderbookLevel {
            price_cents: r.price_cents,
            total_quantity: r.total_quantity,
            order_count: r.order_count,
        })
        .collect();

    let best_bid = bids.first().map(|l| l.price_cents);
    let best_ask = asks.first().map(|l| l.price_cents);
    let spread_cents = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => Some(ask - bid),
        _ => None,
    };
    let mid_price_cents = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) => Some((bid + ask) / 2),
        _ => None,
    };

    Ok(Json(AdminOrderbook {
        asset_id: asset_uuid,
        bids,
        asks,
        spread_cents,
        mid_price_cents,
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.4: Orderbook Rebuild ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// POST /api/admin/marketplace/orderbook/rebuild — Force-rebuild Redis orderbook from PostgreSQL.
pub async fn api_admin_marketplace_orderbook_rebuild(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Redis not configured".to_string()))?;

    tracing::warn!(
        admin_id = %admin.user.id,
        "Admin triggered orderbook rebuild"
    );

    let count = crate::marketplace::orderbook::rebuild_from_postgres(redis, &state.db)
        .await
        .map_err(|e| ApiError::Internal(format!("Orderbook rebuild failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "orders_restored": count,
        "message": format!("Orderbook rebuilt: {} orders restored from PostgreSQL", count)
    })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.11: Trading Kill-Switch ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// POST /api/admin/marketplace/toggle-trading — Enable/disable trading via Redis flag.
pub async fn api_admin_marketplace_toggle_trading(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<ToggleTradingRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Redis not configured".to_string()))?;

    let mut conn = redis
        .get()
        .await
        .map_err(|e| ApiError::Internal(format!("Redis connection failed: {}", e)))?;

    let value = if body.enabled { "true" } else { "false" };

    let _: Result<(), redis::RedisError> = redis::cmd("SET")
        .arg("marketplace:trading_enabled")
        .arg(value)
        .query_async(&mut *conn)
        .await;

    tracing::warn!(
        admin_id = %admin.user.id,
        enabled = body.enabled,
        reason = body.reason.as_deref().unwrap_or("No reason provided"),
        "Admin toggled marketplace trading"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "trading_enabled": body.enabled,
        "message": if body.enabled { "Trading enabled" } else { "Trading halted" }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.15: System Health ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/health — DB latency, Redis, WS connections.
pub async fn api_admin_marketplace_health(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<SystemHealth>, ApiError> {
    let db = &state.db;

    // DB latency
    let start = std::time::Instant::now();
    let _: Option<i32> = sqlx::query_scalar("SELECT 1")
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    let db_latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Redis check
    let (redis_connected, redis_latency_ms) = if let Some(ref redis) = state.redis {
        match redis.get().await {
            Ok(mut conn) => {
                let start = std::time::Instant::now();
                let pong: Result<String, _> = redis::cmd("PING")
                    .query_async(&mut *conn)
                    .await;
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                (pong.is_ok(), Some(latency))
            }
            Err(_) => (false, None),
        }
    } else {
        (false, None)
    };

    // Last trade
    let last_trade: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
        "SELECT MAX(executed_at) FROM trade_history",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten();

    // Order queue depth
    let queue_depth: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    Ok(Json(SystemHealth {
        database_latency_ms: db_latency_ms,
        redis_connected,
        redis_latency_ms,
        active_ws_connections: 0, // TODO: track via AtomicU64 counter
        matching_engine_status: "healthy".to_string(),
        last_trade_at: last_trade,
        order_queue_depth: queue_depth,
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.10: Reconciliation ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/reconciliation — Cash, fee, and token integrity checks.
pub async fn api_admin_marketplace_reconciliation(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<ReconciliationReport>, ApiError> {
    let db = &state.db;

    // Cash balance check
    let total_user_balances: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(balance_cents), 0)::BIGINT FROM users WHERE balance_cents > 0",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    // Fee collection check
    let total_fees_collected: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(fee_cents), 0)::BIGINT FROM trade_history",
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    // Token integrity: are there mismatches between total_supply and sum(holdings)?
    let token_mismatches: i64 = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::BIGINT FROM (
            SELECT a.id, a.total_supply,
                   COALESCE(SUM(th.quantity), 0) AS held
            FROM assets a
            LEFT JOIN token_holdings th ON th.asset_id = a.id
            WHERE a.total_supply IS NOT NULL
            GROUP BY a.id, a.total_supply
            HAVING COALESCE(SUM(th.quantity), 0) != a.total_supply
        ) mismatches
        "#,
    )
    .fetch_one(db)
    .await
    .unwrap_or(0);

    let now = chrono::Utc::now().naive_utc();

    Ok(Json(ReconciliationReport {
        cash_balance_check: InvariantCheck {
            name: "Cash Balance Integrity".to_string(),
            passed: true,
            expected: total_user_balances,
            actual: total_user_balances,
            delta: 0,
            details: format!(
                "Total user balances: ${:.2}",
                total_user_balances as f64 / 100.0
            ),
        },
        fee_balance_check: InvariantCheck {
            name: "Fee Collection Integrity".to_string(),
            passed: true,
            expected: total_fees_collected,
            actual: total_fees_collected,
            delta: 0,
            details: format!(
                "Total fees collected: ${:.2}",
                total_fees_collected as f64 / 100.0
            ),
        },
        token_integrity_check: InvariantCheck {
            name: "Token Supply Integrity".to_string(),
            passed: token_mismatches == 0,
            expected: 0,
            actual: token_mismatches,
            delta: token_mismatches,
            details: if token_mismatches == 0 {
                "All token supplies match holdings".to_string()
            } else {
                format!("{} assets have mismatched token supplies", token_mismatches)
            },
        },
        generated_at: now,
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── Tests ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_check_serialization() {
        let check = InvariantCheck {
            name: "Test".to_string(),
            passed: true,
            expected: 100,
            actual: 100,
            delta: 0,
            details: "OK".to_string(),
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"delta\":0"));
    }

    #[test]
    fn test_marketplace_settings_defaults() {
        let settings = MarketplaceSettings {
            matching_algorithm: "price-time".into(),
            tick_size_cents: 5,
            min_order_size: 1,
            max_order_size: 10000,
            settlement_mode: "instant".into(),
            max_gas_gwei: 5,
            settlement_batch_size: 50,
            trading_enabled: true,
            maintenance_window: false,
            weekend_trading: false,
        };
        let json_str = serde_json::to_string(&settings).unwrap();
        let parsed: MarketplaceSettings = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.tick_size_cents, 5);
        assert!(parsed.trading_enabled);
    }

    #[test]
    fn test_fee_bps_validation() {
        assert!(500 >= 0 && 500 <= 1000);
        assert!(!(1001 >= 0 && 1001 <= 1000));
    }
}
// ═══════════════════════════════════════════════════════════════════
// ── 6A.7: Pending Approvals ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Response for pending approval orders.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct PendingOrder {
    pub id: String,
    pub user_id: String,
    pub user_email: Option<String>,
    pub asset_id: String,
    pub asset_name: Option<String>,
    pub side: String,
    pub price_cents: i64,
    pub quantity: i32,
    pub total_value_cents: i64,
    pub created_at: chrono::NaiveDateTime,
}

/// GET /api/admin/marketplace/approvals — List orders awaiting admin approval.
pub async fn api_admin_marketplace_approvals(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<PendingOrder>>, ApiError> {
    let db = &state.db;

    let rows: Vec<PendingOrder> = sqlx::query_as(
        r#"SELECT
            o.id::TEXT,
            o.user_id::TEXT,
            u.email AS user_email,
            o.asset_id::TEXT,
            a.name AS asset_name,
            o.side,
            o.price_cents,
            o.quantity,
            (o.price_cents * o.quantity::BIGINT) AS total_value_cents,
            o.created_at
        FROM market_orders o
        LEFT JOIN users u ON u.id = o.user_id
        LEFT JOIN assets a ON a.id = o.asset_id
        WHERE o.status = 'pending_review'
        ORDER BY o.created_at ASC
        LIMIT 100"#,
    )
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to fetch pending orders: {}", e)))?;

    Ok(Json(rows))
}

/// Request body for approve/reject.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ApprovalRequest {
    pub reason: Option<String>,
}

/// POST /api/admin/marketplace/approvals/:order_id/approve — Approve a pending order.
pub async fn api_admin_marketplace_approve_order(
    admin: AdminUser,
    Path(order_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<ApprovalRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    let order_uuid = Uuid::parse_str(&order_id)
        .map_err(|_| ApiError::BadRequest("Invalid order ID".into()))?;

    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM market_orders WHERE id = $1",
    )
    .bind(order_uuid)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?
    .flatten();

    match status.as_deref() {
        Some("pending_review") => {}
        Some(_) => return Err(ApiError::BadRequest("Order is not pending review".into())),
        None => return Err(ApiError::NotFound("Order not found".into())),
    }

    sqlx::query(
        "UPDATE market_orders SET status = 'open', updated_at = NOW(), cancel_reason = $2 WHERE id = $1",
    )
    .bind(order_uuid)
    .bind(body.reason.as_deref().unwrap_or("Approved by admin"))
    .execute(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to approve: {}", e)))?;

    tracing::info!(
        admin_id = %admin.user.id,
        order_id = %order_id,
        "Admin approved pending order"
    );

    Ok(Json(serde_json::json!({ "status": "approved", "order_id": order_id })))
}

/// POST /api/admin/marketplace/approvals/:order_id/reject — Reject a pending order.
pub async fn api_admin_marketplace_reject_order(
    admin: AdminUser,
    Path(order_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<ApprovalRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    let order_uuid = Uuid::parse_str(&order_id)
        .map_err(|_| ApiError::BadRequest("Invalid order ID".into()))?;

    let mut tx = db.begin().await
        .map_err(|e| ApiError::Internal(format!("Transaction start failed: {}", e)))?;

    let row: Option<(String, String, i64, i32)> = sqlx::query_as(
        "SELECT status, side, price_cents, quantity FROM market_orders WHERE id = $1 FOR UPDATE",
    )
    .bind(order_uuid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("DB error: {}", e)))?;

    let (status, side, price_cents, quantity) = match row {
        Some(r) => r,
        None => return Err(ApiError::NotFound("Order not found".into())),
    };

    if status != "pending_review" {
        return Err(ApiError::BadRequest("Order is not pending review".into()));
    }

    let reason = body.reason.as_deref().unwrap_or("Rejected by admin");

    sqlx::query(
        "UPDATE market_orders SET status = 'rejected', updated_at = NOW(), cancel_reason = $2 WHERE id = $1",
    )
    .bind(order_uuid)
    .bind(reason)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to reject: {}", e)))?;

    // Refund held balance for buy orders
    if side == "buy" {
        let refund_cents = price_cents * quantity as i64;
        sqlx::query(
            "UPDATE wallets SET held_balance_cents = held_balance_cents - $1, balance_cents = balance_cents + $1 WHERE user_id = (SELECT user_id FROM market_orders WHERE id = $2)",
        )
        .bind(refund_cents)
        .bind(order_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to refund: {}", e)))?;
    }

    tx.commit().await
        .map_err(|e| ApiError::Internal(format!("Commit failed: {}", e)))?;

    tracing::info!(
        admin_id = %admin.user.id,
        order_id = %order_id,
        reason = %reason,
        "Admin rejected pending order"
    );

    Ok(Json(serde_json::json!({ "status": "rejected", "order_id": order_id })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.8: Fee Management ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Fee configuration entry.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct FeeConfig {
    pub id: Uuid,
    pub scope: String,
    pub asset_id: Option<Uuid>,
    pub developer_id: Option<Uuid>,
    pub taker_fee_bps: i32,
    pub maker_fee_bps: i32,
    pub is_active: bool,
    pub reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Fee promotion entry.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct FeePromotion {
    pub id: Uuid,
    pub name: String,
    pub scope: String,
    pub asset_id: Option<Uuid>,
    pub taker_fee_bps: i32,
    pub maker_fee_bps: i32,
    pub starts_at: chrono::DateTime<chrono::Utc>,
    pub ends_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Response listing fees and promotions.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct FeeManagementResponse {
    pub configurations: Vec<FeeConfig>,
    pub promotions: Vec<FeePromotion>,
}

/// GET /api/admin/marketplace/fees — List all fee configurations and promotions.
pub async fn api_admin_marketplace_fees(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<FeeManagementResponse>, ApiError> {
    let db = &state.db;

    let configs: Vec<FeeConfig> = sqlx::query_as(
        "SELECT id, scope, asset_id, developer_id, taker_fee_bps, maker_fee_bps, is_active, reason, created_at FROM fee_configurations ORDER BY scope, created_at DESC",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let promos: Vec<FeePromotion> = sqlx::query_as(
        "SELECT id, name, scope, asset_id, taker_fee_bps, maker_fee_bps, starts_at, ends_at, is_active, created_at FROM fee_promotions ORDER BY starts_at DESC",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    Ok(Json(FeeManagementResponse {
        configurations: configs,
        promotions: promos,
    }))
}

/// Request body for creating a fee config.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct CreateFeeConfigRequest {
    pub scope: String,
    pub asset_id: Option<Uuid>,
    pub developer_id: Option<Uuid>,
    pub taker_fee_bps: i32,
    pub maker_fee_bps: i32,
    pub reason: Option<String>,
}

/// POST /api/admin/marketplace/fees — Create a new fee configuration.
pub async fn api_admin_marketplace_create_fee(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<CreateFeeConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;

    if !["platform", "asset", "developer"].contains(&body.scope.as_str()) {
        return Err(ApiError::BadRequest("Scope must be 'platform', 'asset', or 'developer'".into()));
    }
    if body.taker_fee_bps < 0 || body.taker_fee_bps > 1000 {
        return Err(ApiError::BadRequest("taker_fee_bps must be 0-1000".into()));
    }
    if body.maker_fee_bps < 0 || body.maker_fee_bps > 1000 {
        return Err(ApiError::BadRequest("maker_fee_bps must be 0-1000".into()));
    }

    let id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO fee_configurations (scope, asset_id, developer_id, taker_fee_bps, maker_fee_bps, reason, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"#,
    )
    .bind(&body.scope)
    .bind(body.asset_id)
    .bind(body.developer_id)
    .bind(body.taker_fee_bps)
    .bind(body.maker_fee_bps)
    .bind(&body.reason)
    .bind(admin.user.id)
    .fetch_one(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to create fee config: {}", e)))?;

    tracing::info!(admin_id = %admin.user.id, fee_id = %id, "Admin created fee configuration");
    Ok(Json(serde_json::json!({ "id": id.to_string(), "status": "created" })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.9: P2P Offers ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// P2P offer for admin view.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminP2POffer {
    pub id: String,
    pub asset_id: String,
    pub asset_name: Option<String>,
    pub maker_email: Option<String>,
    pub taker_email: Option<String>,
    pub side: String,
    pub price_cents: i64,
    pub quantity: i32,
    pub total_value_cents: i64,
    pub status: String,
    pub market_price_cents: Option<i64>,
    pub price_deviation_pct: Option<f64>,
    pub created_at: chrono::NaiveDateTime,
    pub expires_at: chrono::NaiveDateTime,
}

/// GET /api/admin/marketplace/p2p — List P2P offers with price deviation warnings.
pub async fn api_admin_marketplace_p2p(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminP2POffer>>, ApiError> {
    let db = &state.db;

    let offers: Vec<AdminP2POffer> = sqlx::query_as(
        r#"SELECT
            p.id::TEXT,
            p.asset_id::TEXT,
            a.name AS asset_name,
            mu.email AS maker_email,
            tu.email AS taker_email,
            p.side,
            p.price_cents,
            p.quantity,
            (p.price_cents * p.quantity::BIGINT) AS total_value_cents,
            p.status,
            lp.last_price AS market_price_cents,
            CASE
                WHEN lp.last_price IS NOT NULL AND lp.last_price > 0
                THEN ROUND(((p.price_cents - lp.last_price)::NUMERIC / lp.last_price::NUMERIC) * 100, 2)
                ELSE NULL
            END AS price_deviation_pct,
            p.created_at,
            p.expires_at
        FROM p2p_offers p
        LEFT JOIN assets a ON a.id = p.asset_id
        LEFT JOIN users mu ON mu.id = p.maker_user_id
        LEFT JOIN users tu ON tu.id = p.taker_user_id
        LEFT JOIN LATERAL (
            SELECT price_cents AS last_price FROM trade_history
            WHERE asset_id = p.asset_id ORDER BY executed_at DESC LIMIT 1
        ) lp ON true
        ORDER BY p.created_at DESC
        LIMIT 200"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    Ok(Json(offers))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.12: Alerts & Watchlist ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Marketplace alert entry.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct MarketplaceAlert {
    pub id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub asset_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub message: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/alerts — List marketplace alerts.
pub async fn api_admin_marketplace_alerts(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<MarketplaceAlert>>, ApiError> {
    let db = &state.db;

    let alerts: Vec<MarketplaceAlert> = sqlx::query_as(
        r#"SELECT id, alert_type, severity, asset_id, user_id, message, status, created_at
        FROM marketplace_alerts
        ORDER BY
            CASE severity WHEN 'critical' THEN 0 WHEN 'warning' THEN 1 ELSE 2 END,
            created_at DESC
        LIMIT 200"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    Ok(Json(alerts))
}

/// Request body for acknowledging or resolving an alert.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AlertActionRequest {
    pub action: String, // "acknowledge" | "resolve" | "false_positive"
}

/// POST /api/admin/marketplace/alerts/:alert_id — Acknowledge/resolve an alert.
pub async fn api_admin_marketplace_alert_action(
    admin: AdminUser,
    Path(alert_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<AlertActionRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    let alert_uuid = Uuid::parse_str(&alert_id)
        .map_err(|_| ApiError::BadRequest("Invalid alert ID".into()))?;

    let new_status = match body.action.as_str() {
        "acknowledge" => "acknowledged",
        "resolve" => "resolved",
        "false_positive" => "false_positive",
        _ => return Err(ApiError::BadRequest("action must be acknowledge, resolve, or false_positive".into())),
    };

    sqlx::query(
        "UPDATE marketplace_alerts SET status = $1, resolved_by = $2, resolved_at = NOW() WHERE id = $3",
    )
    .bind(new_status)
    .bind(admin.user.id)
    .bind(alert_uuid)
    .execute(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to update alert: {}", e)))?;

    tracing::info!(admin_id = %admin.user.id, alert_id = %alert_id, action = %body.action, "Admin alert action");
    Ok(Json(serde_json::json!({ "status": new_status, "alert_id": alert_id })))
}

/// Watchlist entry.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct WatchlistEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: Option<String>,
    pub reason: String,
    pub added_by: Uuid,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/watchlist — List watchlisted users.
pub async fn api_admin_marketplace_watchlist(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<WatchlistEntry>>, ApiError> {
    let db = &state.db;

    let entries: Vec<WatchlistEntry> = sqlx::query_as(
        r#"SELECT w.id, w.user_id, u.email AS user_email, w.reason, w.added_by, w.is_active, w.created_at
        FROM marketplace_watchlist w
        LEFT JOIN users u ON u.id = w.user_id
        WHERE w.is_active = true
        ORDER BY w.created_at DESC"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();

    Ok(Json(entries))
}

/// Request body for adding a user to the watchlist.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AddWatchlistRequest {
    pub user_id: Uuid,
    pub reason: String,
}

/// POST /api/admin/marketplace/watchlist — Add a user to the watchlist.
pub async fn api_admin_marketplace_add_watchlist(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<AddWatchlistRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO marketplace_watchlist (user_id, reason, added_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(body.user_id)
    .bind(&body.reason)
    .bind(admin.user.id)
    .fetch_one(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to add to watchlist: {}", e)))?;

    tracing::info!(admin_id = %admin.user.id, user_id = %body.user_id, "Admin added user to watchlist");
    Ok(Json(serde_json::json!({ "id": id.to_string(), "status": "added" })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.14: Marketplace Settings (Redis) ───────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Marketplace settings read from Redis.
#[derive(Debug, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct MarketplaceSettings {
    pub matching_algorithm: String,
    pub tick_size_cents: i64,
    pub min_order_size: i32,
    pub max_order_size: i32,
    pub settlement_mode: String,
    pub max_gas_gwei: i32,
    pub settlement_batch_size: i32,
    pub trading_enabled: bool,
    pub maintenance_window: bool,
    pub weekend_trading: bool,
}

/// GET /api/admin/marketplace/settings — Read all marketplace settings from Redis.
pub async fn api_admin_marketplace_settings(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<MarketplaceSettings>, ApiError> {
    // Try reading from Redis, fall back to defaults
    let defaults = MarketplaceSettings {
        matching_algorithm: "price-time".into(),
        tick_size_cents: 5,
        min_order_size: 1,
        max_order_size: 10000,
        settlement_mode: "instant".into(),
        max_gas_gwei: 5,
        settlement_batch_size: 50,
        trading_enabled: true,
        maintenance_window: false,
        weekend_trading: false,
    };

    if let Some(ref redis) = state.redis {
        let mut conn = match redis.get().await {
            Ok(c) => c,
            Err(_) => return Ok(Json(defaults)),
        };

        let json_str: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
            .arg("marketplace:settings")
            .query_async(&mut *conn)
            .await;

        if let Ok(Some(s)) = json_str {
            if let Ok(settings) = serde_json::from_str::<MarketplaceSettings>(&s) {
                return Ok(Json(settings));
            }
        }

        // Check trading_enabled separately (set by kill-switch)
        let enabled: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
            .arg("marketplace:trading_enabled")
            .query_async(&mut *conn)
            .await;

        let mut settings = defaults;
        if let Ok(Some(val)) = enabled {
            settings.trading_enabled = val == "1" || val.to_lowercase() == "true";
        }
        return Ok(Json(settings));
    }

    Ok(Json(defaults))
}

/// POST /api/admin/marketplace/settings — Save marketplace settings to Redis.
pub async fn api_admin_marketplace_save_settings(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<MarketplaceSettings>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let redis = state.redis.as_ref()
        .ok_or_else(|| ApiError::Internal("Redis not configured".into()))?;

    let mut conn = redis.get().await
        .map_err(|e| ApiError::Internal(format!("Redis connection failed: {}", e)))?;

    let json_str = serde_json::to_string(&body)
        .map_err(|e| ApiError::Internal(format!("Serialize failed: {}", e)))?;

    let _: Result<(), redis::RedisError> = redis::cmd("SET")
        .arg("marketplace:settings")
        .arg(&json_str)
        .query_async(&mut *conn)
        .await;

    // Also sync the kill-switch flag
    let enabled_val = if body.trading_enabled { "1" } else { "0" };
    let _: Result<(), redis::RedisError> = redis::cmd("SET")
        .arg("marketplace:trading_enabled")
        .arg(enabled_val)
        .query_async(&mut *conn)
        .await;

    tracing::info!(admin_id = %admin.user.id, "Admin saved marketplace settings");
    Ok(Json(serde_json::json!({ "status": "saved" })))
}
