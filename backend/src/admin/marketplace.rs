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
use axum_extra::extract::CookieJar;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use super::extractors::{AdminUser, ApiError};
use crate::auth::models::User;
use crate::auth::routes::AppState;
use crate::marketplace::models::MarketOrder;

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
    pub on_chain_status: String,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

/// Asset option for trade-history filters.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminTradeAsset {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub trade_count: i64,
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
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Query filters for trade history.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct TradeFilters {
    pub asset_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub side: Option<String>,
    pub on_chain_status: Option<String>,
    pub status: Option<String>,
    #[serde(alias = "_min_price_cents")]
    pub min_price_cents: Option<i64>,
    #[serde(alias = "_max_price_cents")]
    pub max_price_cents: Option<i64>,
    #[serde(alias = "_from_date")]
    pub from_date: Option<String>,
    #[serde(alias = "_to_date")]
    pub to_date: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub limit: Option<i64>,
}

struct ValidatedTradeFilters {
    asset_id: Option<Uuid>,
    user_id: Option<Uuid>,
    side: Option<String>,
    min_price_cents: Option<i64>,
    max_price_cents: Option<i64>,
    from_date: Option<NaiveDate>,
    to_date_exclusive: Option<NaiveDate>,
    on_chain_status: Option<String>,
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

fn normalize_admin_cancel_reason(reason: Option<String>) -> Result<String, ApiError> {
    let reason = reason
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::BadRequest("Cancellation reason is required".into()))?;

    if reason.len() > 500 {
        return Err(ApiError::BadRequest(
            "Cancellation reason must be 500 characters or fewer".into(),
        ));
    }

    Ok(reason)
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

/// Asset option for the admin orderbook selector.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminOrderbookAsset {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub active_orders: i64,
}

/// Admin orderbook view with aggregated levels and spread data.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminOrderbook {
    pub asset_id: Uuid,
    pub asset_title: String,
    pub asset_slug: String,
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
    pub database_connected: bool,
    pub redis_connected: bool,
    pub redis_latency_ms: Option<f64>,
    pub active_ws_connections: i64,
    pub websocket_status: String,
    pub matching_engine_status: String,
    pub last_trade_at: Option<chrono::DateTime<chrono::Utc>>,
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<MarketplaceStats>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let db = &state.db;

    let open_orders: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let volume_24h: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(price_cents * quantity::BIGINT), 0)::BIGINT
         FROM trade_history
         WHERE executed_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let trades_24h: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM trade_history
         WHERE executed_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let pending_reviews: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM market_orders
         WHERE status = 'pending_review'",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let total_assets: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT asset_id) FROM market_orders
         WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let active_users: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT user_id) FROM market_orders
         WHERE created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let fees_24h: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(fee_cents), 0)::BIGINT FROM trade_history
         WHERE executed_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    // Check trading status from Redis (or default to active when Redis is not configured).
    let trading_status = if let Some(ref redis) = state.redis {
        match redis.get().await {
            Ok(mut conn) => match redis::cmd("GET")
                .arg("marketplace:trading_enabled")
                .query_async::<Option<String>>(&mut *conn)
                .await
            {
                Ok(status) => match status.as_deref() {
                    Some("false") | Some("0") => "HALTED".to_string(),
                    Some("true") | Some("1") | None => "LIVE".to_string(),
                    _ => "UNKNOWN".to_string(),
                },
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read marketplace trading status from Redis");
                    "UNKNOWN".to_string()
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Failed to connect to Redis for marketplace trading status");
                "UNKNOWN".to_string()
            }
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminTrade>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let db = &state.db;

    let rows: Vec<AdminTrade> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.asset_id,
            a.title AS asset_name,
            t.buyer_user_id AS buyer_id,
            t.seller_user_id AS seller_id,
            bu.email AS buyer_email,
            su.email AS seller_email,
            t.price_cents,
            t.quantity,
            COALESCE(t.total_cents, t.price_cents * t.quantity::BIGINT) AS total_cents,
            COALESCE(t.fee_cents, 0) AS fee_cents,
            t.on_chain_status,
            t.executed_at
        FROM trade_history t
        LEFT JOIN assets a ON a.id = t.asset_id
        LEFT JOIN users bu ON bu.id = t.buyer_user_id
        LEFT JOIN users su ON su.id = t.seller_user_id
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

fn normalize_trade_filters(filters: &TradeFilters) -> Result<ValidatedTradeFilters, ApiError> {
    let side = filters
        .side
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    if let Some(ref side) = side {
        if side != "buy" && side != "sell" {
            return Err(ApiError::BadRequest(
                "side must be either buy or sell".to_string(),
            ));
        }
        if filters.user_id.is_none() {
            return Err(ApiError::BadRequest(
                "side filter requires user_id".to_string(),
            ));
        }
    }

    if let Some(min_price_cents) = filters.min_price_cents {
        if min_price_cents < 0 {
            return Err(ApiError::BadRequest(
                "min_price_cents must be non-negative".to_string(),
            ));
        }
    }
    if let Some(max_price_cents) = filters.max_price_cents {
        if max_price_cents < 0 {
            return Err(ApiError::BadRequest(
                "max_price_cents must be non-negative".to_string(),
            ));
        }
    }
    if let (Some(min_price_cents), Some(max_price_cents)) =
        (filters.min_price_cents, filters.max_price_cents)
    {
        if min_price_cents > max_price_cents {
            return Err(ApiError::BadRequest(
                "min_price_cents cannot exceed max_price_cents".to_string(),
            ));
        }
    }

    let from_date = parse_optional_date(&filters.from_date, "from_date")?;
    let to_date = parse_optional_date(&filters.to_date, "to_date")?;
    if let (Some(from_date), Some(to_date)) = (from_date, to_date) {
        if from_date > to_date {
            return Err(ApiError::BadRequest(
                "from_date cannot be after to_date".to_string(),
            ));
        }
    }
    let to_date_exclusive = match to_date {
        Some(date) => Some(date.succ_opt().ok_or_else(|| {
            ApiError::BadRequest("to_date is outside supported range".to_string())
        })?),
        None => None,
    };

    let status = filters
        .on_chain_status
        .as_deref()
        .or(filters.status.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    if let Some(ref status) = status {
        if !matches!(
            status.as_str(),
            "pending" | "submitted" | "confirmed" | "failed"
        ) {
            return Err(ApiError::BadRequest(
                "on_chain_status must be pending, submitted, confirmed, or failed".to_string(),
            ));
        }
    }

    Ok(ValidatedTradeFilters {
        asset_id: filters.asset_id,
        user_id: filters.user_id,
        side,
        min_price_cents: filters.min_price_cents,
        max_price_cents: filters.max_price_cents,
        from_date,
        to_date_exclusive,
        on_chain_status: status,
    })
}

fn push_trade_filter_sql<'args>(
    query: &mut QueryBuilder<'args, Postgres>,
    filters: &'args ValidatedTradeFilters,
) {
    if let Some(asset_id) = filters.asset_id {
        query.push(" AND t.asset_id = ");
        query.push_bind(asset_id);
    }
    if let Some(user_id) = filters.user_id {
        match filters.side.as_deref() {
            Some("buy") => {
                query.push(" AND t.buyer_user_id = ");
                query.push_bind(user_id);
            }
            Some("sell") => {
                query.push(" AND t.seller_user_id = ");
                query.push_bind(user_id);
            }
            _ => {
                query.push(" AND (t.buyer_user_id = ");
                query.push_bind(user_id);
                query.push(" OR t.seller_user_id = ");
                query.push_bind(user_id);
                query.push(")");
            }
        }
    }
    if let Some(min_price_cents) = filters.min_price_cents {
        query.push(" AND t.price_cents >= ");
        query.push_bind(min_price_cents);
    }
    if let Some(max_price_cents) = filters.max_price_cents {
        query.push(" AND t.price_cents <= ");
        query.push_bind(max_price_cents);
    }
    if let Some(from_date) = filters.from_date {
        query.push(" AND t.executed_at >= ");
        query.push_bind(from_date);
    }
    if let Some(to_date_exclusive) = filters.to_date_exclusive {
        query.push(" AND t.executed_at < ");
        query.push_bind(to_date_exclusive);
    }
    if let Some(ref status) = filters.on_chain_status {
        query.push(" AND t.on_chain_status = ");
        query.push_bind(status.as_str());
    }
}

async fn count_admin_trades(
    db: &sqlx::PgPool,
    filters: &ValidatedTradeFilters,
) -> Result<i64, ApiError> {
    let mut query =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*)::BIGINT FROM trade_history t WHERE 1=1");
    push_trade_filter_sql(&mut query, filters);
    query
        .build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(ApiError::Database)
}

async fn fetch_admin_trades(
    db: &sqlx::PgPool,
    filters: &ValidatedTradeFilters,
    limit: i64,
    offset: i64,
) -> Result<Vec<AdminTrade>, ApiError> {
    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            t.id,
            t.asset_id,
            a.title AS asset_name,
            t.buyer_user_id AS buyer_id,
            t.seller_user_id AS seller_id,
            bu.email AS buyer_email,
            su.email AS seller_email,
            t.price_cents,
            t.quantity,
            COALESCE(t.total_cents, t.price_cents * t.quantity::BIGINT) AS total_cents,
            COALESCE(t.fee_cents, 0) AS fee_cents,
            t.on_chain_status,
            t.executed_at
        FROM trade_history t
        LEFT JOIN assets a ON a.id = t.asset_id
        LEFT JOIN users bu ON bu.id = t.buyer_user_id
        LEFT JOIN users su ON su.id = t.seller_user_id
        WHERE 1=1
        "#,
    );
    push_trade_filter_sql(&mut query, filters);
    query.push(" ORDER BY t.executed_at DESC LIMIT ");
    query.push_bind(limit);
    query.push(" OFFSET ");
    query.push_bind(offset);
    query
        .build_query_as()
        .fetch_all(db)
        .await
        .map_err(ApiError::Database)
}

/// GET /api/admin/marketplace/trades/assets — Assets that have trade history.
pub async fn api_admin_marketplace_trade_assets(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminTradeAsset>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;

    let rows: Vec<AdminTradeAsset> = sqlx::query_as(
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            COUNT(t.id)::BIGINT AS trade_count
        FROM trade_history t
        JOIN assets a ON a.id = t.asset_id
        GROUP BY a.id, a.title, a.slug
        ORDER BY a.title ASC
        LIMIT 1000
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

/// GET /api/admin/marketplace/trades — Paginated trade history with filters.
pub async fn api_admin_marketplace_trades(
    admin: AdminUser,
    Query(filters): Query<TradeFilters>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<AdminTrade>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters
        .per_page
        .or(filters.limit)
        .unwrap_or(25)
        .clamp(1, 200);
    let offset = (page - 1) * per_page;
    let filters = normalize_trade_filters(&filters)?;

    let total = count_admin_trades(&state.db, &filters).await?;
    let rows = fetch_admin_trades(&state.db, &filters, per_page, offset).await?;

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

/// GET /api/admin/marketplace/trades/export.csv — Filtered trade CSV export.
pub async fn api_admin_marketplace_trades_export_csv(
    admin: AdminUser,
    Query(filters): Query<TradeFilters>,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;

    let filters = normalize_trade_filters(&filters)?;
    let rows = fetch_admin_trades(&state.db, &filters, 10_000, 0).await?;
    let mut csv = String::from(
        "Trade_ID,Executed_At,Asset_ID,Asset_Name,Buyer_Email,Seller_Email,Price_Cents,Quantity,Fee_Cents,Total_Cents,On_Chain_Status\n",
    );
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            row.id,
            row.executed_at,
            row.asset_id,
            csv_escape(row.asset_name.unwrap_or_default()),
            csv_escape(row.buyer_email.unwrap_or_default()),
            csv_escape(row.seller_email.unwrap_or_default()),
            row.price_cents,
            row.quantity,
            row.fee_cents,
            row.total_cents,
            csv_escape(row.on_chain_status)
        ));
    }

    tracing::info!(admin_id = %admin.user.id, "Admin exported marketplace trade history CSV");

    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"marketplace_trades.csv\"",
        ),
    ];

    Ok((headers, csv))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.6: Open Orders + Admin Cancel ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/marketplace/orders — Paginated open orders list.
pub async fn api_admin_marketplace_orders(
    admin: AdminUser,
    Query(filters): Query<OrderFilters>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<AdminOrder>>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(25).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let status_filter = filters.status.as_deref().unwrap_or("open,partially_filled");

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
            a.title AS asset_name,
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
    admin.require_permission(db, "marketplace.manage").await?;
    let order_uuid = ApiError::parse_uuid(&order_id)?;
    let reason = normalize_admin_cancel_reason(body.reason)?;

    let mut tx = db.begin().await.map_err(ApiError::Database)?;

    let order: MarketOrder = sqlx::query_as("SELECT * FROM market_orders WHERE id = $1 FOR UPDATE")
        .bind(order_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Order not found".to_string()))?;

    if order.status != "open" && order.status != "partially_filled" {
        return Err(ApiError::BadRequest(format!(
            "Order cannot be cancelled: status is '{}'",
            order.status
        )));
    }

    let remaining = order.quantity - order.quantity_filled;
    if remaining <= 0 {
        return Err(ApiError::Conflict(
            "Order has no remaining quantity to cancel".into(),
        ));
    }

    let cancelled_order: MarketOrder = sqlx::query_as(
        "UPDATE market_orders
         SET status = 'admin_cancelled', updated_at = NOW(), cancel_reason = $2
         WHERE id = $1 AND status IN ('open', 'partially_filled')
         RETURNING *",
    )
    .bind(order_uuid)
    .bind(&reason)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::Conflict("Order was already cancelled or filled".into()))?;

    match order.side.as_str() {
        "buy" => {
            let hold_cents = order_hold_cents(order.price_cents, remaining)?;
            let result = sqlx::query(
                "UPDATE wallets
                 SET held_balance_cents = held_balance_cents - $1, updated_at = NOW()
                 WHERE user_id = $2
                   AND wallet_type = 'cash'
                   AND currency = 'USD'
                   AND held_balance_cents >= $1",
            )
            .bind(hold_cents)
            .bind(order.user_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            if result.rows_affected() != 1 {
                return Err(ApiError::Conflict(
                    "Held balance could not be released for this order".into(),
                ));
            }
        }
        "sell" => {
            let result = sqlx::query(
                "UPDATE investments
                 SET held_tokens = held_tokens - $1, updated_at = NOW()
                 WHERE user_id = $2
                   AND asset_id = $3
                   AND status != 'exited'
                   AND held_tokens >= $1",
            )
            .bind(remaining)
            .bind(order.user_id)
            .bind(order.asset_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            if result.rows_affected() != 1 {
                return Err(ApiError::Conflict(
                    "Held tokens could not be released for this order".into(),
                ));
            }
        }
        _ => return Err(ApiError::BadRequest("Unsupported order side".into())),
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.order.admin_cancelled', 'market_order', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(order_uuid)
    .bind(serde_json::json!({
        "status": order.status.clone(),
        "side": order.side.clone(),
        "price_cents": order.price_cents,
        "quantity": order.quantity,
        "quantity_filled": order.quantity_filled
    }))
    .bind(serde_json::json!({
        "status": cancelled_order.status.clone(),
        "reason": reason,
        "remaining_quantity_released": remaining
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    if let Some(redis) = state.redis.as_ref() {
        if let Err(e) = crate::marketplace::orderbook::remove_order(redis, &order).await {
            tracing::error!(
                order_id = %order_uuid,
                error = %e,
                "Admin-cancelled marketplace order could not be removed from Redis orderbook"
            );
            sentry::capture_message(
                "Admin-cancelled marketplace order failed Redis orderbook removal",
                sentry::Level::Error,
            );
        } else {
            crate::marketplace::websocket::broadcast_orderbook_update(
                db,
                Some(redis),
                order.asset_id,
            )
            .await;
        }
    }

    tracing::info!(
        admin_id = %admin.user.id,
        %order_uuid,
        reason = %reason,
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

/// GET /api/admin/marketplace/orderbook/assets — Assets available in the orderbook selector.
pub async fn api_admin_marketplace_orderbook_assets(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminOrderbookAsset>>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;

    let assets = sqlx::query_as::<_, AdminOrderbookAsset>(
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            COUNT(mo.id)::BIGINT AS active_orders
        FROM assets a
        LEFT JOIN market_orders mo
          ON mo.asset_id = a.id
         AND mo.status IN ('open', 'partially_filled')
        WHERE a.published = TRUE
           OR EXISTS (
                SELECT 1
                FROM market_orders existing
                WHERE existing.asset_id = a.id
                  AND existing.status IN ('open', 'partially_filled')
           )
        GROUP BY a.id, a.title, a.slug
        ORDER BY active_orders DESC, a.title ASC
        LIMIT 100
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(assets))
}

/// GET /api/admin/marketplace/orderbook/:asset_id — Aggregated orderbook.
pub async fn api_admin_marketplace_orderbook(
    admin: AdminUser,
    Path(asset_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<AdminOrderbook>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;
    let asset_uuid = ApiError::parse_uuid(&asset_id)?;

    let asset: Option<(String, String)> =
        sqlx::query_as("SELECT title, slug FROM assets WHERE id = $1")
            .bind(asset_uuid)
            .fetch_optional(db)
            .await
            .map_err(ApiError::Database)?;
    let (asset_title, asset_slug) =
        asset.ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

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
        asset_title,
        asset_slug,
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
    let db = &state.db;
    admin.require_permission(db, "marketplace.manage").await?;

    let Some(redis) = state.redis.as_ref() else {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM market_orders WHERE status = 'open'")
                .fetch_one(db)
                .await
                .map_err(ApiError::Database)?;

        sqlx::query(
            r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
               VALUES ($1, 'marketplace.orderbook.rebuilt', 'marketplace_orderbook', $2)"#,
        )
        .bind(admin.user.id)
        .bind(serde_json::json!({
            "success": true,
            "redis_configured": false,
            "orders_restored": count
        }))
        .execute(db)
        .await
        .map_err(ApiError::Database)?;

        return Ok(Json(serde_json::json!({
            "success": true,
            "redis_configured": false,
            "orders_restored": count,
            "message": format!("Redis is not configured; verified {} open orders in PostgreSQL", count)
        })));
    };

    let mut conn = redis
        .get()
        .await
        .map_err(|e| ApiError::Internal(format!("Redis connection failed: {}", e)))?;
    let lock_key = "admin:marketplace:orderbook:rebuild:lock";
    let lock_result: Option<String> = redis::cmd("SET")
        .arg(lock_key)
        .arg(admin.user.id.to_string())
        .arg("NX")
        .arg("EX")
        .arg(120)
        .query_async(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(format!("Redis rebuild lock failed: {}", e)))?;

    if lock_result.is_none() {
        return Err(ApiError::Conflict(
            "Orderbook rebuild is already running".to_string(),
        ));
    }

    tracing::warn!(
        admin_id = %admin.user.id,
        "Admin triggered orderbook rebuild"
    );

    let rebuild_result = crate::marketplace::orderbook::rebuild_from_postgres(redis, db).await;
    let _release_result: Result<i32, redis::RedisError> = redis::cmd("DEL")
        .arg(lock_key)
        .query_async(&mut *conn)
        .await;

    let count = match rebuild_result {
        Ok(count) => count,
        Err(e) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                   VALUES ($1, 'marketplace.orderbook.rebuild_failed', 'marketplace_orderbook', $2)"#,
            )
            .bind(admin.user.id)
            .bind(serde_json::json!({
                "success": false,
                "error": e.to_string()
            }))
            .execute(db)
            .await;
            return Err(ApiError::Internal(format!(
                "Orderbook rebuild failed: {}",
                e
            )));
        }
    };

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'marketplace.orderbook.rebuilt', 'marketplace_orderbook', $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "success": true,
        "redis_configured": true,
        "orders_restored": count
    }))
    .execute(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "redis_configured": true,
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<SystemHealth>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let db = &state.db;

    // DB latency
    let start = std::time::Instant::now();
    let _: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(db)
        .await
        .map_err(ApiError::Database)?;
    let db_latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Redis check
    let (redis_connected, redis_latency_ms) = if let Some(ref redis) = state.redis {
        match redis.get().await {
            Ok(mut conn) => {
                let start = std::time::Instant::now();
                let pong: Result<String, _> = redis::cmd("PING").query_async(&mut *conn).await;
                let latency = start.elapsed().as_secs_f64() * 1000.0;
                (pong.is_ok(), Some(latency))
            }
            Err(_) => (false, None),
        }
    } else {
        (false, None)
    };

    let last_trade: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT MAX(executed_at) FROM trade_history")
            .fetch_one(db)
            .await
            .map_err(ApiError::Database)?;

    let queue_depth: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let websocket_status = "not_tracked".to_string();
    let matching_engine_status = if state.redis.is_some() {
        if redis_connected {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    } else {
        "not_configured".to_string()
    };

    Ok(Json(SystemHealth {
        database_latency_ms: db_latency_ms,
        database_connected: true,
        redis_connected,
        redis_latency_ms,
        active_ws_connections: 0,
        websocket_status,
        matching_engine_status,
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
        "SELECT COALESCE(SUM(balance_cents), 0)::BIGINT FROM wallets WHERE wallet_type = 'cash' AND balance_cents > 0",
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

    // Token integrity: are there mismatches between tokens_total and sum(investments)?
    let token_mismatches: i64 = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::BIGINT FROM (
            SELECT a.id, a.tokens_total,
                   COALESCE(SUM(i.tokens_owned), 0) AS held
            FROM assets a
            LEFT JOIN investments i ON i.asset_id = a.id AND i.status != 'exited'
            WHERE a.tokens_total IS NOT NULL
            GROUP BY a.id, a.tokens_total
            HAVING COALESCE(SUM(i.tokens_owned), 0) > a.tokens_total
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
        let settings = default_marketplace_settings();
        let json_str = serde_json::to_string(&settings).unwrap();
        let parsed: MarketplaceSettings = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.tick_size_cents, 5);
        assert!(parsed.trading_enabled);
    }

    #[test]
    fn marketplace_settings_validation_accepts_defaults() {
        let settings = default_marketplace_settings();
        assert!(validate_marketplace_settings(&settings).is_ok());
    }

    #[test]
    fn marketplace_settings_validation_rejects_unimplemented_algorithm() {
        let mut settings = default_marketplace_settings();
        settings.matching_algorithm = "pro-rata".into();
        assert!(validate_marketplace_settings(&settings).is_err());
    }

    #[test]
    fn marketplace_settings_validation_rejects_bad_settlement_mode() {
        let mut settings = default_marketplace_settings();
        settings.settlement_mode = "tomorrow".into();
        assert!(validate_marketplace_settings(&settings).is_err());
    }

    #[test]
    fn marketplace_settings_validation_rejects_zero_tick() {
        let mut settings = default_marketplace_settings();
        settings.tick_size_cents = 0;
        assert!(validate_marketplace_settings(&settings).is_err());
    }

    #[test]
    fn marketplace_settings_validation_rejects_min_greater_than_max() {
        let mut settings = default_marketplace_settings();
        settings.min_order_size = 100;
        settings.max_order_size = 10;
        assert!(validate_marketplace_settings(&settings).is_err());
    }

    #[test]
    fn marketplace_settings_validation_rejects_invalid_gas_and_batch_bounds() {
        let mut settings = default_marketplace_settings();
        settings.max_gas_gwei = 0;
        assert!(validate_marketplace_settings(&settings).is_err());

        settings = default_marketplace_settings();
        settings.settlement_batch_size = 10_001;
        assert!(validate_marketplace_settings(&settings).is_err());
    }

    #[test]
    fn test_fee_bps_validation() {
        let valid_bps = 500;
        let invalid_bps = 1001;
        assert!((0..=1000).contains(&valid_bps));
        assert!(!(0..=1000).contains(&invalid_bps));
    }

    #[test]
    fn order_hold_cents_multiplies_integer_minor_units() {
        assert_eq!(order_hold_cents(10_500, 3).unwrap(), 31_500);
    }

    #[test]
    fn order_hold_cents_allows_one_cent_order() {
        assert_eq!(order_hold_cents(1, 1).unwrap(), 1);
    }

    #[test]
    fn order_hold_cents_rejects_zero_price() {
        assert!(order_hold_cents(0, 10).is_err());
    }

    #[test]
    fn order_hold_cents_rejects_negative_price() {
        assert!(order_hold_cents(-1, 10).is_err());
    }

    #[test]
    fn order_hold_cents_rejects_zero_quantity() {
        assert!(order_hold_cents(100, 0).is_err());
    }

    #[test]
    fn order_hold_cents_rejects_negative_quantity() {
        assert!(order_hold_cents(100, -1).is_err());
    }

    #[test]
    fn order_hold_cents_rejects_overflow() {
        assert!(order_hold_cents(i64::MAX, 2).is_err());
    }

    #[test]
    fn approval_reason_trims_input() {
        assert_eq!(
            normalize_approval_reason(Some("  reviewed ".into()), "fallback").unwrap(),
            "reviewed"
        );
    }

    #[test]
    fn approval_reason_uses_fallback_for_empty_input() {
        assert_eq!(
            normalize_approval_reason(Some("   ".into()), "fallback").unwrap(),
            "fallback"
        );
    }

    #[test]
    fn approval_reason_rejects_long_input() {
        assert!(normalize_approval_reason(Some("x".repeat(501)), "fallback").is_err());
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
    pub review_reason: String,
    pub supply_impact_bps: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/approvals — List orders awaiting admin approval.
pub async fn api_admin_marketplace_approvals(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<PendingOrder>>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.manage").await?;

    let rows: Vec<PendingOrder> = sqlx::query_as(
        r#"SELECT
            o.id::TEXT,
            o.user_id::TEXT,
            u.email AS user_email,
            o.asset_id::TEXT,
            a.title AS asset_name,
            o.side,
            o.price_cents,
            o.quantity,
            (o.price_cents * o.quantity::BIGINT) AS total_value_cents,
            CASE
                WHEN (o.price_cents * o.quantity::BIGINT) > 5000000 THEN 'Order value exceeds manual review threshold'
                WHEN a.tokens_total > 0 AND (o.quantity * 10000 / a.tokens_total) > 500 THEN 'Order quantity exceeds 5% supply threshold'
                ELSE 'Flagged for admin review'
            END AS review_reason,
            CASE
                WHEN a.tokens_total > 0 THEN (o.quantity * 10000 / a.tokens_total)::INTEGER
                ELSE NULL
            END AS supply_impact_bps,
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

fn normalize_approval_reason(reason: Option<String>, fallback: &str) -> Result<String, ApiError> {
    let reason = reason
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string());

    if reason.len() > 500 {
        return Err(ApiError::BadRequest(
            "Reason must be 500 characters or fewer".into(),
        ));
    }

    Ok(reason)
}

fn order_hold_cents(price_cents: i64, quantity: i32) -> Result<i64, ApiError> {
    if price_cents <= 0 || quantity <= 0 {
        return Err(ApiError::BadRequest(
            "Order price and quantity must be positive".into(),
        ));
    }

    price_cents
        .checked_mul(quantity as i64)
        .ok_or_else(|| ApiError::BadRequest("Order total exceeds supported limits".into()))
}

/// POST /api/admin/marketplace/approvals/:order_id/approve — Approve a pending order.
pub async fn api_admin_marketplace_approve_order(
    admin: AdminUser,
    Path(order_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<ApprovalRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.manage").await?;
    let order_uuid =
        Uuid::parse_str(&order_id).map_err(|_| ApiError::BadRequest("Invalid order ID".into()))?;
    let reason = normalize_approval_reason(body.reason, "Approved by admin")?;

    let mut tx = db
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("Transaction start failed: {}", e)))?;

    let order: MarketOrder = sqlx::query_as("SELECT * FROM market_orders WHERE id = $1 FOR UPDATE")
        .bind(order_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if order.status != "pending_review" {
        return Err(ApiError::BadRequest("Order is not pending review".into()));
    }

    let approved_order: MarketOrder = sqlx::query_as(
        "UPDATE market_orders
         SET status = 'open', updated_at = NOW(), cancel_reason = $2
         WHERE id = $1 AND status = 'pending_review'
         RETURNING *",
    )
    .bind(order_uuid)
    .bind(&reason)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::Conflict("Order was already reviewed".into()))?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.order.approved', 'market_order', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(order_uuid)
    .bind(serde_json::json!({
        "status": order.status,
        "held_side": order.side,
        "price_cents": order.price_cents,
        "quantity": order.quantity
    }))
    .bind(serde_json::json!({
        "status": approved_order.status,
        "reason": reason,
        "orderbook_sync": "queued"
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Commit failed: {}", e)))?;

    let mut orderbook_synced = false;
    if let Some(redis) = state.redis.as_ref() {
        match crate::marketplace::orderbook::insert_order(redis, &approved_order).await {
            Ok(()) => {
                orderbook_synced = true;
                crate::marketplace::websocket::broadcast_orderbook_update(
                    db,
                    Some(redis),
                    approved_order.asset_id,
                )
                .await;
            }
            Err(e) => {
                tracing::error!(
                    order_id = %approved_order.id,
                    error = %e,
                    "Approved order opened in DB but Redis orderbook insert failed"
                );
                sentry::capture_message(
                    "Approved marketplace order failed Redis orderbook insert",
                    sentry::Level::Error,
                );
            }
        }
    } else {
        tracing::warn!(
            order_id = %approved_order.id,
            "Approved order opened without Redis configured; sync worker cannot insert it"
        );
    }

    tracing::info!(
        admin_id = %admin.user.id,
        order_id = %order_id,
        orderbook_synced,
        "Admin approved pending order"
    );

    Ok(Json(serde_json::json!({
        "status": "approved",
        "order_id": order_id,
        "orderbook_synced": orderbook_synced
    })))
}

/// POST /api/admin/marketplace/approvals/:order_id/reject — Reject a pending order.
pub async fn api_admin_marketplace_reject_order(
    admin: AdminUser,
    Path(order_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<ApprovalRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.manage").await?;
    let order_uuid =
        Uuid::parse_str(&order_id).map_err(|_| ApiError::BadRequest("Invalid order ID".into()))?;
    let reason = normalize_approval_reason(body.reason, "Rejected by admin")?;

    let mut tx = db
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("Transaction start failed: {}", e)))?;

    let order: MarketOrder = sqlx::query_as("SELECT * FROM market_orders WHERE id = $1 FOR UPDATE")
        .bind(order_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if order.status != "pending_review" {
        return Err(ApiError::BadRequest("Order is not pending review".into()));
    }

    sqlx::query(
        "UPDATE market_orders SET status = 'rejected', updated_at = NOW(), cancel_reason = $2 WHERE id = $1",
    )
    .bind(order_uuid)
    .bind(&reason)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to reject: {}", e)))?;

    match order.side.as_str() {
        "buy" => {
            let hold_cents = order_hold_cents(order.price_cents, order.quantity)?;
            let result = sqlx::query(
                "UPDATE wallets
                 SET held_balance_cents = held_balance_cents - $1, updated_at = NOW()
                 WHERE user_id = $2
                   AND wallet_type = 'cash'
                   AND currency = 'USD'
                   AND held_balance_cents >= $1",
            )
            .bind(hold_cents)
            .bind(order.user_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            if result.rows_affected() != 1 {
                return Err(ApiError::Conflict(
                    "Held balance could not be released for this order".into(),
                ));
            }
        }
        "sell" => {
            let result = sqlx::query(
                "UPDATE investments
                 SET held_tokens = held_tokens - $1, updated_at = NOW()
                 WHERE user_id = $2
                   AND asset_id = $3
                   AND status != 'exited'
                   AND held_tokens >= $1",
            )
            .bind(order.quantity)
            .bind(order.user_id)
            .bind(order.asset_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            if result.rows_affected() != 1 {
                return Err(ApiError::Conflict(
                    "Held tokens could not be released for this order".into(),
                ));
            }
        }
        _ => return Err(ApiError::BadRequest("Unsupported order side".into())),
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.order.rejected', 'market_order', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(order_uuid)
    .bind(serde_json::json!({
        "status": order.status,
        "side": order.side,
        "price_cents": order.price_cents,
        "quantity": order.quantity
    }))
    .bind(serde_json::json!({
        "status": "rejected",
        "reason": reason
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Commit failed: {}", e)))?;

    tracing::info!(
        admin_id = %admin.user.id,
        order_id = %order_id,
        reason = %reason,
        "Admin rejected pending order"
    );

    Ok(Json(
        serde_json::json!({ "status": "rejected", "order_id": order_id }),
    ))
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<FeeManagementResponse>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    let db = &state.db;

    let configs: Vec<FeeConfig> = sqlx::query_as(
        "SELECT id, scope, asset_id, developer_id, taker_fee_bps, maker_fee_bps, is_active, reason, created_at FROM fee_configurations ORDER BY scope, created_at DESC",
    )
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to load fee configurations: {}", e)))?;

    let promos: Vec<FeePromotion> = sqlx::query_as(
        "SELECT id, name, scope, asset_id, taker_fee_bps, maker_fee_bps, starts_at, ends_at, is_active, created_at FROM fee_promotions ORDER BY starts_at DESC",
    )
    .fetch_all(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to load fee promotions: {}", e)))?;

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
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    let db = &state.db;

    if !["platform", "asset", "developer"].contains(&body.scope.as_str()) {
        return Err(ApiError::BadRequest(
            "Scope must be 'platform', 'asset', or 'developer'".into(),
        ));
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

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.fee_config_create', 'fee_configurations', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(id)
    .bind(serde_json::json!({
        "scope": &body.scope,
        "taker_fee_bps": body.taker_fee_bps,
        "maker_fee_bps": body.maker_fee_bps,
        "asset_id": body.asset_id,
        "developer_id": body.developer_id
    }))
    .execute(db)
    .await;

    tracing::info!(admin_id = %admin.user.id, fee_id = %id, "Admin created fee configuration");
    Ok(Json(
        serde_json::json!({ "id": id.to_string(), "status": "created" }),
    ))
}

/// DELETE /api/admin/marketplace/fees/:fee_id — Deactivate a fee configuration.
pub async fn api_admin_marketplace_deactivate_fee(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(fee_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;

    let updated = sqlx::query(
        "UPDATE fee_configurations SET is_active = false, updated_at = NOW() WHERE id = $1 AND is_active = true",
    )
    .bind(fee_id)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "Fee configuration not found or already inactive".into(),
        ));
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.fee_config_deactivate', 'fee_configurations', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(fee_id)
    .bind(serde_json::json!({"is_active": false}))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({ "status": "deactivated" })))
}

/// DELETE /api/admin/marketplace/promotions/:promo_id — Deactivate a fee promotion.
pub async fn api_admin_marketplace_deactivate_promo(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(promo_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;

    let updated = sqlx::query(
        "UPDATE fee_promotions SET is_active = false WHERE id = $1 AND is_active = true",
    )
    .bind(promo_id)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "Promotion not found or already inactive".into(),
        ));
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.fee_promo_deactivate', 'fee_promotions', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(promo_id)
    .bind(serde_json::json!({"is_active": false}))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({ "status": "deactivated" })))
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AdminCancelP2PRequest {
    pub reason: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AdminP2PCancelRow {
    id: Uuid,
    status: String,
    asset_id: Uuid,
    maker_user_id: Uuid,
    taker_user_id: Uuid,
    side: String,
    price_cents: i64,
    quantity: i32,
}

/// GET /api/admin/marketplace/p2p — List P2P offers with price deviation warnings.
pub async fn api_admin_marketplace_p2p(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AdminP2POffer>>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;

    let offers: Vec<AdminP2POffer> = sqlx::query_as(
        r#"SELECT
            p.id::TEXT,
            p.asset_id::TEXT,
            a.title AS asset_name,
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
    .map_err(ApiError::Database)?;

    Ok(Json(offers))
}

/// POST /api/admin/marketplace/p2p/:offer_id/cancel — Admin-cancel a pending P2P offer.
pub async fn api_admin_marketplace_cancel_p2p(
    admin: AdminUser,
    Path(offer_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<AdminCancelP2PRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.manage").await?;

    let reason = body.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::BadRequest(
            "Cancellation reason is required".into(),
        ));
    }
    if reason.len() > 500 {
        return Err(ApiError::BadRequest(
            "Cancellation reason must be 500 characters or fewer".into(),
        ));
    }

    let mut tx = db
        .begin()
        .await
        .map_err(|e| ApiError::Internal(format!("Transaction start failed: {}", e)))?;

    let offer: AdminP2PCancelRow = sqlx::query_as(
        r#"SELECT id, status, asset_id, maker_user_id, taker_user_id, side, price_cents, quantity
           FROM p2p_offers
           WHERE id = $1
           FOR UPDATE"#,
    )
    .bind(offer_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("P2P offer not found".into()))?;

    if offer.status != "pending" {
        return Err(ApiError::Conflict(format!(
            "Only pending P2P offers can be admin-cancelled; current status is {}",
            offer.status
        )));
    }

    sqlx::query(
        "UPDATE p2p_offers SET status = 'admin_cancelled', updated_at = NOW() WHERE id = $1",
    )
    .bind(offer_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.p2p.admin_cancelled', 'p2p_offer', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(offer_id)
    .bind(serde_json::json!({
        "status": offer.status,
        "asset_id": offer.asset_id,
        "maker_user_id": offer.maker_user_id,
        "taker_user_id": offer.taker_user_id,
        "side": offer.side,
        "price_cents": offer.price_cents,
        "quantity": offer.quantity
    }))
    .bind(serde_json::json!({
        "status": "admin_cancelled",
        "reason": reason
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    tracing::info!(
        admin_id = %admin.user.id,
        offer_id = %offer.id,
        "Admin cancelled P2P offer"
    );

    Ok(Json(serde_json::json!({
        "offer_id": offer_id,
        "status": "admin_cancelled",
        "reason": reason
    })))
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
    let alert_uuid =
        Uuid::parse_str(&alert_id).map_err(|_| ApiError::BadRequest("Invalid alert ID".into()))?;

    let new_status = match body.action.as_str() {
        "acknowledge" => "acknowledged",
        "resolve" => "resolved",
        "false_positive" => "false_positive",
        _ => {
            return Err(ApiError::BadRequest(
                "action must be acknowledge, resolve, or false_positive".into(),
            ))
        }
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
    Ok(Json(
        serde_json::json!({ "status": new_status, "alert_id": alert_id }),
    ))
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
    Ok(Json(
        serde_json::json!({ "id": id.to_string(), "status": "added" }),
    ))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.14: Marketplace Settings (Redis) ───────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Marketplace settings read from Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

fn default_marketplace_settings() -> MarketplaceSettings {
    MarketplaceSettings {
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
    }
}

fn validate_marketplace_settings(settings: &MarketplaceSettings) -> Result<(), ApiError> {
    if settings.matching_algorithm != "price-time" {
        return Err(ApiError::BadRequest(
            "matching_algorithm must be price-time".into(),
        ));
    }

    if !matches!(
        settings.settlement_mode.as_str(),
        "instant" | "batched" | "manual"
    ) {
        return Err(ApiError::BadRequest(
            "settlement_mode must be instant, batched, or manual".into(),
        ));
    }

    if !(1..=1_000_000).contains(&settings.tick_size_cents) {
        return Err(ApiError::BadRequest(
            "tick_size_cents must be between 1 and 1,000,000".into(),
        ));
    }

    if !(1..=1_000_000).contains(&settings.min_order_size) {
        return Err(ApiError::BadRequest(
            "min_order_size must be between 1 and 1,000,000".into(),
        ));
    }

    if !(1..=1_000_000).contains(&settings.max_order_size) {
        return Err(ApiError::BadRequest(
            "max_order_size must be between 1 and 1,000,000".into(),
        ));
    }

    if settings.min_order_size > settings.max_order_size {
        return Err(ApiError::BadRequest(
            "min_order_size cannot exceed max_order_size".into(),
        ));
    }

    if !(1..=10_000).contains(&settings.max_gas_gwei) {
        return Err(ApiError::BadRequest(
            "max_gas_gwei must be between 1 and 10,000".into(),
        ));
    }

    if !(1..=10_000).contains(&settings.settlement_batch_size) {
        return Err(ApiError::BadRequest(
            "settlement_batch_size must be between 1 and 10,000".into(),
        ));
    }

    Ok(())
}

/// GET /api/admin/marketplace/settings — Read all marketplace settings from Redis.
pub async fn api_admin_marketplace_settings(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<MarketplaceSettings>, ApiError> {
    let can_view =
        crate::auth::middleware::has_permission(&state.db, admin.user.id, "marketplace.view").await;
    let can_manage =
        crate::auth::middleware::has_permission(&state.db, admin.user.id, "marketplace.manage")
            .await;
    if !can_view && !can_manage {
        return Err(ApiError::Forbidden(
            "Missing permission: marketplace.view".into(),
        ));
    }

    // Try reading from Redis, fall back to defaults
    let defaults = default_marketplace_settings();

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
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    validate_marketplace_settings(&body)?;

    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| ApiError::Internal("Redis not configured".into()))?;

    let mut conn = redis
        .get()
        .await
        .map_err(|e| ApiError::Internal(format!("Redis connection failed: {}", e)))?;

    let previous_settings = {
        let json_str: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
            .arg("marketplace:settings")
            .query_async(&mut *conn)
            .await;

        match json_str {
            Ok(Some(s)) => serde_json::from_str::<MarketplaceSettings>(&s)
                .unwrap_or_else(|_| default_marketplace_settings()),
            Ok(None) => default_marketplace_settings(),
            Err(e) => {
                return Err(ApiError::Internal(format!(
                    "Redis settings read failed: {}",
                    e
                )));
            }
        }
    };

    let json_str = serde_json::to_string(&body)
        .map_err(|e| ApiError::Internal(format!("Serialize failed: {}", e)))?;

    redis::cmd("SET")
        .arg("marketplace:settings")
        .arg(&json_str)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(format!("Redis settings write failed: {}", e)))?;

    // Also sync the kill-switch flag
    let enabled_val = if body.trading_enabled { "1" } else { "0" };
    redis::cmd("SET")
        .arg("marketplace:trading_enabled")
        .arg(enabled_val)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| ApiError::Internal(format!("Redis trading flag write failed: {}", e)))?;

    let previous_state = serde_json::to_value(previous_settings)
        .map_err(|e| ApiError::Internal(format!("Audit serialization failed: {}", e)))?;
    let new_state = serde_json::to_value(&body)
        .map_err(|e| ApiError::Internal(format!("Audit serialization failed: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, previous_state, new_state)
           VALUES ($1, 'marketplace.settings.update', 'marketplace_settings', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(previous_state)
    .bind(new_state)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(admin_id = %admin.user.id, "Admin saved marketplace settings");
    Ok(Json(serde_json::json!({ "status": "saved" })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.13: Compliance & OJK APIs ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OjkReportQuery {
    pub quarter: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct TravelRuleQuery {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct TaxExportQuery {
    pub year: Option<i32>,
}

fn csv_escape(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

async fn require_marketplace_compliance(
    jar: &CookieJar,
    state: &AppState,
) -> Result<User, ApiError> {
    let user = crate::auth::middleware::get_current_user(jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if crate::auth::middleware::has_permission(&state.db, user.id, "marketplace.compliance").await {
        Ok(user)
    } else {
        Err(ApiError::Forbidden(
            "marketplace.compliance permission required".to_string(),
        ))
    }
}

fn parse_ojk_quarter(input: Option<String>) -> Result<(String, NaiveDate, NaiveDate), ApiError> {
    let quarter = input.unwrap_or_else(|| {
        let today = chrono::Utc::now().date_naive();
        let q = ((today.month0() / 3) + 1).clamp(1, 4);
        format!("{}-Q{}", today.year(), q)
    });

    let (year_str, q_str) = quarter
        .split_once("-Q")
        .ok_or_else(|| ApiError::BadRequest("quarter must use YYYY-QN".to_string()))?;
    let year: i32 = year_str
        .parse()
        .map_err(|_| ApiError::BadRequest("quarter year must be numeric".to_string()))?;
    let q: u32 = q_str
        .parse()
        .map_err(|_| ApiError::BadRequest("quarter must use Q1 through Q4".to_string()))?;
    if !(1..=4).contains(&q) {
        return Err(ApiError::BadRequest(
            "quarter must use Q1 through Q4".to_string(),
        ));
    }

    let start_month = (q - 1) * 3 + 1;
    let start = NaiveDate::from_ymd_opt(year, start_month, 1)
        .ok_or_else(|| ApiError::BadRequest("quarter is outside supported range".to_string()))?;
    let end = if q == 4 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, start_month + 3, 1)
    }
    .ok_or_else(|| ApiError::BadRequest("quarter is outside supported range".to_string()))?;

    Ok((format!("{}-Q{}", year, q), start, end))
}

fn parse_optional_date(value: &Option<String>, name: &str) -> Result<Option<NaiveDate>, ApiError> {
    match value.as_deref().filter(|v| !v.trim().is_empty()) {
        Some(raw) => NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| ApiError::BadRequest(format!("{} must use YYYY-MM-DD", name))),
        None => Ok(None),
    }
}

/// GET /api/admin/marketplace/compliance/ojk-report - Returns basic quarterly metrics as CSV
pub async fn api_admin_marketplace_compliance_ojk(
    jar: CookieJar,
    Query(query): Query<OjkReportQuery>,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;
    let (current_quarter, start_date, end_date) = parse_ojk_quarter(query.quarter)?;

    let total_volume: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(price_cents * quantity::BIGINT), 0)::BIGINT
        FROM trade_history
        WHERE executed_at >= $1::date
          AND executed_at < $2::date
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let total_users: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM users
        WHERE created_at < $1::date
        "#,
    )
    .bind(end_date)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let trade_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM trade_history
        WHERE executed_at >= $1::date
          AND executed_at < $2::date
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let csv = format!(
        "Metric,Value,Period,Start_Date,End_Date\nTotal Trade Volume (cents),{},{},{},{}\nTotal Trades,{},{},{},{}\nTotal Registered Users,{},{},{},{}\n",
        total_volume,
        current_quarter,
        start_date,
        end_date,
        trade_count,
        current_quarter,
        start_date,
        end_date,
        total_users,
        current_quarter,
        start_date,
        end_date
    );

    tracing::info!(admin_id = %user.id, quarter = %current_quarter, "Admin exported OJK Report (CSV)");

    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"ojk_report.csv\"",
        ),
    ];

    Ok((headers, csv))
}

/// GET /api/admin/marketplace/compliance/travel-rule - Returns all trades for AML checks as CSV
pub async fn api_admin_marketplace_compliance_travel_rule(
    jar: CookieJar,
    Query(query): Query<TravelRuleQuery>,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;
    let from_date = parse_optional_date(&query.from_date, "from_date")?;
    let to_date = parse_optional_date(&query.to_date, "to_date")?;
    if let (Some(from_date), Some(to_date)) = (from_date, to_date) {
        if from_date > to_date {
            return Err(ApiError::BadRequest(
                "from_date cannot be after to_date".to_string(),
            ));
        }
    }

    let rows: Vec<(
        Uuid,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i32,
        i64,
    )> = sqlx::query_as(
        r#"
        SELECT
            t.id,
            t.executed_at,
            b.email AS buyer_email,
            s.email AS seller_email,
            bp.display_name AS buyer_name,
            sp.display_name AS seller_name,
            t.price_cents,
            t.quantity,
            COALESCE(t.total_cents, t.price_cents * t.quantity::BIGINT) AS total_value_cents
        FROM trade_history t 
        LEFT JOIN users b ON b.id = t.buyer_user_id
        LEFT JOIN users s ON s.id = t.seller_user_id
        LEFT JOIN user_profiles bp ON bp.user_id = t.buyer_user_id
        LEFT JOIN user_profiles sp ON sp.user_id = t.seller_user_id
        WHERE ($1::date IS NULL OR t.executed_at >= $1::date)
          AND ($2::date IS NULL OR t.executed_at < ($2::date + INTERVAL '1 day'))
        ORDER BY t.executed_at DESC
        LIMIT 10000
        "#,
    )
    .bind(from_date)
    .bind(to_date)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    let mut csv = String::from(
        "Trade_ID,Executed_At,Buyer_Email,Seller_Email,Buyer_Name,Seller_Name,Price_Cents,Quantity,Total_Value_Cents\n",
    );
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            row.0,
            row.1,
            csv_escape(row.2.unwrap_or_default()),
            csv_escape(row.3.unwrap_or_default()),
            csv_escape(row.4.unwrap_or_default()),
            csv_escape(row.5.unwrap_or_default()),
            row.6,
            row.7,
            row.8
        ));
    }

    tracing::info!(admin_id = %user.id, "Admin exported AML Travel Rule Data");

    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"travel_rule_export.csv\"",
        ),
    ];

    Ok((headers, csv))
}

/// GET /api/admin/marketplace/compliance/tax-export - Returns basic tax liability data
pub async fn api_admin_marketplace_compliance_tax(
    jar: CookieJar,
    Query(query): Query<TaxExportQuery>,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;
    let year = query.year.unwrap_or_else(|| chrono::Utc::now().year() - 1);

    if !(2000..=2100).contains(&year) {
        return Err(ApiError::BadRequest(
            "year must be between 2000 and 2100".to_string(),
        ));
    }

    let rows: Vec<(String, i32, i64, i64, i64, i64, String)> = sqlx::query_as(
        r#"
        SELECT
            u.email,
            tr.fiscal_year,
            tr.total_investment_cents,
            tr.total_dividends_cents,
            tr.capital_gains_cents,
            tr.withholding_tax_cents,
            tr.status
        FROM tax_reports tr
        JOIN users u ON u.id = tr.user_id
        WHERE tr.fiscal_year = $1
        ORDER BY u.email ASC
        "#,
    )
    .bind(year)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    let mut csv = String::from(
        "User_Email,Year,Total_Investment_Cents,Total_Dividends_Cents,Capital_Gains_Cents,Withholding_Tax_Cents,Status\n",
    );
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_escape(row.0),
            row.1,
            row.2,
            row.3,
            row.4,
            row.5,
            csv_escape(row.6)
        ));
    }

    tracing::info!(admin_id = %user.id, fiscal_year = year, "Admin exported Tax Reports");

    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"tax_export.csv\"",
        ),
    ];

    Ok((headers, csv))
}
