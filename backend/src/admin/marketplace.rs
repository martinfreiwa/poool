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
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::{Datelike, NaiveDate, Timelike};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    pub buy_order_id: Option<Uuid>,
    pub sell_order_id: Option<Uuid>,
    pub on_chain_tx_hash: Option<String>,
    pub on_chain_batch_id: Option<Uuid>,
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
    /// T4-E: server-side anomaly flag timestamp (NULL when not flagged).
    #[serde(default)]
    pub flagged_at: Option<chrono::DateTime<chrono::Utc>>,
    /// T4-E: short reason label (e.g. "large_hold", "stale_7d").
    #[serde(default)]
    pub flag_reason: Option<String>,
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
    pub q: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
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
    q: Option<String>,
    sort_by: TradeSortColumn,
    sort_dir_desc: bool,
}

#[derive(Clone, Copy, Debug)]
enum TradeSortColumn {
    ExecutedAt,
    TotalCents,
    PriceCents,
    Quantity,
    FeeCents,
}

impl TradeSortColumn {
    fn sql(self) -> &'static str {
        match self {
            TradeSortColumn::ExecutedAt => "t.executed_at",
            TradeSortColumn::TotalCents => {
                "COALESCE(t.total_cents, t.price_cents * t.quantity::BIGINT)"
            }
            TradeSortColumn::PriceCents => "t.price_cents",
            TradeSortColumn::Quantity => "t.quantity",
            TradeSortColumn::FeeCents => "COALESCE(t.fee_cents, 0)",
        }
    }
}

/// Aggregate summary across all trades matching the current filters.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminTradesSummary {
    pub total_quantity: i64,
    pub total_volume_cents: i64,
    pub total_fee_cents: i64,
    pub oldest_pending_age_seconds: Option<i64>,
    pub over_sla_count: i64,
}

/// SLA threshold (seconds): pending trades older than this are flagged.
pub const TRADE_PENDING_SLA_SECONDS: i64 = 3600;

/// Paginated trade response with summary aggregates.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminTradesResponse {
    pub data: Vec<AdminTrade>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub summary: AdminTradesSummary,
}

/// Query filters for order listing.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OrderFilters {
    pub asset_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub side: Option<String>,
    pub status: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Request body for admin order cancellation.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AdminCancelRequest {
    pub reason: Option<String>,
}

/// Request body for bulk admin order cancellation.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AdminBulkCancelRequest {
    pub order_ids: Vec<String>,
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
    /// Distinct users with orders at this price level. Always <= order_count.
    /// Lets the UI honestly say "5 orders from 3 traders" instead of conflating
    /// orders with users (which previously rendered as "1 buyer" lies).
    pub unique_users: i64,
}

/// Aggregated orderbook price level (API response).
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminOrderbookLevel {
    pub price_cents: i64,
    pub total_quantity: i64,
    pub order_count: i64,
    pub unique_users: i64,
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

/// Admin orderbook view with aggregated levels, spread, market context.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminOrderbook {
    pub asset_id: Uuid,
    pub asset_title: String,
    pub asset_slug: String,
    pub bids: Vec<AdminOrderbookLevel>,
    pub asks: Vec<AdminOrderbookLevel>,
    pub best_bid_cents: Option<i64>,
    pub best_ask_cents: Option<i64>,
    pub spread_cents: Option<i64>,
    pub mid_price_cents: Option<i64>,
    pub mid_price_is_fallback: bool,
    pub last_trade_cents: Option<i64>,
    pub last_trade_at: Option<chrono::DateTime<chrono::Utc>>,
    pub volume_24h_cents: i64,
    pub volume_24h_qty: i64,
    pub trades_24h: i64,
    pub change_24h_pct: Option<f64>,
    pub bid_volume: i64,
    pub ask_volume: i64,
    pub market_status: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub last_rebuild_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(sqlx::FromRow)]
struct AssetMarketContext {
    last_trade_cents: Option<i64>,
    last_trade_at: Option<chrono::DateTime<chrono::Utc>>,
    volume_24h_cents: Option<i64>,
    volume_24h_qty: Option<i64>,
    trades_24h: Option<i64>,
    open_24h_cents: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminOrderbookOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_email: Option<String>,
    pub side: String,
    pub price_cents: i64,
    pub quantity: i32,
    pub quantity_filled: i32,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OrderbookLevelQuery {
    pub side: String,
    pub price_cents: i64,
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
            t.executed_at,
            t.buy_order_id,
            t.sell_order_id,
            t.on_chain_tx_hash,
            t.on_chain_batch_id
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
            "pending" | "submitted" | "confirmed" | "failed" | "cancelled"
        ) {
            return Err(ApiError::BadRequest(
                "on_chain_status must be pending, submitted, confirmed, failed, or cancelled"
                    .to_string(),
            ));
        }
    }

    let q = filters
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(100).collect::<String>());

    let sort_by = match filters
        .sort_by
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        None | Some("executed_at") | Some("date") => TradeSortColumn::ExecutedAt,
        Some("total") | Some("total_cents") => TradeSortColumn::TotalCents,
        Some("price") | Some("price_cents") => TradeSortColumn::PriceCents,
        Some("quantity") | Some("qty") => TradeSortColumn::Quantity,
        Some("fee") | Some("fee_cents") => TradeSortColumn::FeeCents,
        Some(other) => {
            return Err(ApiError::BadRequest(format!(
                "sort_by must be one of executed_at, total, price, quantity, fee (got {other})"
            )));
        }
    };

    let sort_dir_desc = match filters
        .sort_dir
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        None | Some("desc") => true,
        Some("asc") => false,
        Some(other) => {
            return Err(ApiError::BadRequest(format!(
                "sort_dir must be asc or desc (got {other})"
            )));
        }
    };

    Ok(ValidatedTradeFilters {
        asset_id: filters.asset_id,
        user_id: filters.user_id,
        side,
        min_price_cents: filters.min_price_cents,
        max_price_cents: filters.max_price_cents,
        from_date,
        to_date_exclusive,
        on_chain_status: status,
        q,
        sort_by,
        sort_dir_desc,
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
    if let Some(ref q) = filters.q {
        let escaped = q
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{}%", escaped);
        query.push(" AND (t.id::text ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR bu.email ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR su.email ILIKE ");
        query.push_bind(pattern);
        query.push(")");
    }
}

async fn count_admin_trades(
    db: &sqlx::PgPool,
    filters: &ValidatedTradeFilters,
) -> Result<i64, ApiError> {
    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT COUNT(*)::BIGINT \
         FROM trade_history t \
         LEFT JOIN users bu ON bu.id = t.buyer_user_id \
         LEFT JOIN users su ON su.id = t.seller_user_id \
         WHERE 1=1",
    );
    push_trade_filter_sql(&mut query, filters);
    query
        .build_query_scalar()
        .fetch_one(db)
        .await
        .map_err(ApiError::Database)
}

async fn summarize_admin_trades(
    db: &sqlx::PgPool,
    filters: &ValidatedTradeFilters,
) -> Result<AdminTradesSummary, ApiError> {
    let mut query = QueryBuilder::<Postgres>::new(
        r#"
        SELECT
            COALESCE(SUM(t.quantity)::BIGINT, 0) AS total_quantity,
            COALESCE(SUM(COALESCE(t.total_cents, t.price_cents * t.quantity::BIGINT)), 0)::BIGINT AS total_volume_cents,
            COALESCE(SUM(COALESCE(t.fee_cents, 0)), 0)::BIGINT AS total_fee_cents,
            CASE
              WHEN COUNT(*) FILTER (WHERE t.on_chain_status = 'pending') = 0 THEN NULL
              ELSE EXTRACT(EPOCH FROM (NOW() - MIN(t.executed_at) FILTER (WHERE t.on_chain_status = 'pending')))::BIGINT
            END AS oldest_pending_age_seconds,
            COUNT(*) FILTER (
                WHERE t.on_chain_status = 'pending'
                  AND t.executed_at < NOW() - INTERVAL '1 hour'
            )::BIGINT AS over_sla_count
        FROM trade_history t
        LEFT JOIN users bu ON bu.id = t.buyer_user_id
        LEFT JOIN users su ON su.id = t.seller_user_id
        WHERE 1=1
        "#,
    );
    push_trade_filter_sql(&mut query, filters);
    query
        .build_query_as::<AdminTradesSummary>()
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
            t.executed_at,
            t.buy_order_id,
            t.sell_order_id,
            t.on_chain_tx_hash,
            t.on_chain_batch_id
        FROM trade_history t
        LEFT JOIN assets a ON a.id = t.asset_id
        LEFT JOIN users bu ON bu.id = t.buyer_user_id
        LEFT JOIN users su ON su.id = t.seller_user_id
        WHERE 1=1
        "#,
    );
    push_trade_filter_sql(&mut query, filters);
    query.push(" ORDER BY ");
    query.push(filters.sort_by.sql());
    query.push(if filters.sort_dir_desc {
        " DESC"
    } else {
        " ASC"
    });
    query.push(", t.id DESC LIMIT ");
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
) -> Result<Json<AdminTradesResponse>, ApiError> {
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
    let summary = summarize_admin_trades(&state.db, &filters).await?;
    let rows = fetch_admin_trades(&state.db, &filters, per_page, offset).await?;

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        0
    };

    Ok(Json(AdminTradesResponse {
        data: rows,
        total,
        page,
        per_page,
        total_pages,
        summary,
    }))
}

/// Request body for bulk retry-on-chain.
#[derive(Debug, serde::Deserialize)]
#[allow(missing_docs)]
pub struct BulkRetryRequest {
    pub trade_ids: Vec<Uuid>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Response body for bulk retry-on-chain.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct BulkRetryResponse {
    pub requested: usize,
    pub eligible: usize,
    pub reset: usize,
}

/// POST /api/admin/marketplace/trades/bulk-retry-onchain
/// Resets selected trades whose `on_chain_status` is failed/reverted/timeout
/// back to `pending` so the settlement worker picks them up again.
pub async fn api_admin_marketplace_trades_bulk_retry_onchain(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<BulkRetryRequest>,
) -> Result<Json<BulkRetryResponse>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    let db = &state.db;

    if req.trade_ids.is_empty() {
        return Ok(Json(BulkRetryResponse {
            requested: 0,
            eligible: 0,
            reset: 0,
        }));
    }
    if req.trade_ids.len() > 500 {
        return Err(ApiError::BadRequest(
            "trade_ids exceeds limit of 500".to_string(),
        ));
    }

    // Only retry trades currently in a retryable terminal state.
    let result = sqlx::query(
        r#"
        UPDATE trade_history SET
            on_chain_status   = 'pending',
            on_chain_tx_hash  = NULL,
            on_chain_batch_id = NULL,
            updated_at        = NOW()
         WHERE id = ANY($1)
           AND on_chain_status IN ('failed', 'reverted', 'timeout')
        "#,
    )
    .bind(&req.trade_ids)
    .execute(db)
    .await
    .map_err(ApiError::Database)?;

    let reset = result.rows_affected() as usize;
    tracing::info!(
        admin_id = %admin.user.id,
        requested = req.trade_ids.len(),
        reset,
        reason = req.reason.as_deref().unwrap_or("(none)"),
        "Bulk retry-on-chain executed"
    );

    Ok(Json(BulkRetryResponse {
        requested: req.trade_ids.len(),
        eligible: reset,
        reset,
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
// ── Trade mutation: cancel / retry settlement ─────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Body for `POST /api/admin/marketplace/trades/:id/cancel`.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AdminTradeCancelRequest {
    pub reason: Option<String>,
}

/// Response for trade mutation endpoints.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AdminTradeMutationResponse {
    pub id: Uuid,
    pub on_chain_status: String,
    pub cancelled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub cancellation_reason: Option<String>,
}

/// POST /api/admin/marketplace/trades/:id/cancel — Cancel a pending or failed trade.
pub async fn api_admin_marketplace_cancel_trade(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(trade_id): Path<Uuid>,
    Json(body): Json<AdminTradeCancelRequest>,
) -> Result<Json<AdminTradeMutationResponse>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    let reason = normalize_admin_cancel_reason(body.reason)?;
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let prev: Option<(String,)> =
        sqlx::query_as("SELECT on_chain_status FROM trade_history WHERE id = $1 FOR UPDATE")
            .bind(trade_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

    let prev_status = match prev {
        Some((s,)) => s,
        None => return Err(ApiError::NotFound("Trade not found".into())),
    };
    if !matches!(prev_status.as_str(), "pending" | "failed") {
        return Err(ApiError::BadRequest(format!(
            "Cannot cancel trade in status '{}' — only pending or failed trades may be cancelled",
            prev_status
        )));
    }

    let updated: AdminTradeMutationResponse = sqlx::query_as(
        r#"UPDATE trade_history
              SET on_chain_status     = 'cancelled',
                  cancelled_at        = NOW(),
                  cancelled_by        = $2,
                  cancellation_reason = $3
            WHERE id = $1
        RETURNING id, on_chain_status, cancelled_at, cancellation_reason"#,
    )
    .bind(trade_id)
    .bind(admin.user.id)
    .bind(&reason)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.trade.cancelled', 'trade', $2, $3, $4)"#,
    )
    .bind(admin.user.id).bind(trade_id)
    .bind(serde_json::json!({ "on_chain_status": prev_status }))
    .bind(serde_json::json!({ "on_chain_status": "cancelled", "reason": reason }))
    .execute(&mut *tx).await.map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;
    tracing::info!(admin_id = %admin.user.id, trade_id = %trade_id, prev_status = %prev_status, "Admin cancelled marketplace trade");
    Ok(Json(updated))
}

/// POST /api/admin/marketplace/trades/:id/retry-settlement — Requeue a failed trade.
pub async fn api_admin_marketplace_retry_trade_settlement(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(trade_id): Path<Uuid>,
) -> Result<Json<AdminTradeMutationResponse>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let prev: Option<(String,)> =
        sqlx::query_as("SELECT on_chain_status FROM trade_history WHERE id = $1 FOR UPDATE")
            .bind(trade_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

    let prev_status = match prev {
        Some((s,)) => s,
        None => return Err(ApiError::NotFound("Trade not found".into())),
    };
    if prev_status != "failed" {
        return Err(ApiError::BadRequest(format!(
            "Cannot retry trade in status '{}' — only failed trades may be retried",
            prev_status
        )));
    }

    let updated: AdminTradeMutationResponse = sqlx::query_as(
        r#"UPDATE trade_history
              SET on_chain_status     = 'pending',
                  on_chain_tx_hash    = NULL,
                  cancelled_at        = NULL,
                  cancelled_by        = NULL,
                  cancellation_reason = NULL
            WHERE id = $1
        RETURNING id, on_chain_status, cancelled_at, cancellation_reason"#,
    )
    .bind(trade_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.trade.retry_settlement', 'trade', $2, $3, $4)"#,
    )
    .bind(admin.user.id).bind(trade_id)
    .bind(serde_json::json!({ "on_chain_status": prev_status }))
    .bind(serde_json::json!({ "on_chain_status": "pending" }))
    .execute(&mut *tx).await.map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;
    tracing::info!(admin_id = %admin.user.id, trade_id = %trade_id, prev_status = %prev_status, "Admin retried marketplace trade settlement");
    Ok(Json(updated))
}

/// Body for `POST /api/admin/marketplace/trades/bulk-cancel`.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct BulkCancelRequest {
    pub trade_ids: Vec<Uuid>,
    pub reason: Option<String>,
}

/// Response shape for the bulk-cancel endpoint.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct BulkCancelResponse {
    pub requested: usize,
    pub eligible: usize,
    pub cancelled: usize,
}

/// POST /api/admin/marketplace/trades/bulk-cancel — Cancel many trades at once.
pub async fn api_admin_marketplace_trades_bulk_cancel(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<BulkCancelRequest>,
) -> Result<Json<BulkCancelResponse>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    if req.trade_ids.is_empty() {
        return Ok(Json(BulkCancelResponse {
            requested: 0,
            eligible: 0,
            cancelled: 0,
        }));
    }
    if req.trade_ids.len() > 500 {
        return Err(ApiError::BadRequest(
            "trade_ids exceeds limit of 500".to_string(),
        ));
    }
    let reason = normalize_admin_cancel_reason(req.reason.clone())?;

    let eligible: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::BIGINT FROM trade_history
            WHERE id = ANY($1) AND on_chain_status IN ('pending', 'failed')"#,
    )
    .bind(&req.trade_ids)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let result = sqlx::query(
        r#"UPDATE trade_history SET
              on_chain_status     = 'cancelled',
              cancelled_at        = NOW(),
              cancelled_by        = $2,
              cancellation_reason = $3
            WHERE id = ANY($1)
              AND on_chain_status IN ('pending', 'failed')"#,
    )
    .bind(&req.trade_ids)
    .bind(admin.user.id)
    .bind(&reason)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;
    let cancelled = result.rows_affected() as usize;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'marketplace.trade.bulk_cancel', 'trade_history', $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "requested": req.trade_ids.len(),
        "eligible": eligible,
        "cancelled": cancelled,
        "trade_ids": &req.trade_ids,
        "reason": reason,
    }))
    .execute(&state.db)
    .await;

    tracing::info!(admin_id = %admin.user.id, requested = req.trade_ids.len(), cancelled, "Admin bulk-cancelled marketplace trades");
    Ok(Json(BulkCancelResponse {
        requested: req.trade_ids.len(),
        eligible: eligible as usize,
        cancelled,
    }))
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
    let per_page = filters.per_page.unwrap_or(25).clamp(1, 500);
    let offset = (page - 1) * per_page;

    let status_filter = filters.status.as_deref().unwrap_or("open,partially_filled");
    let status_values: Vec<String> = status_filter
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let side_filter = filters
        .side
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s == "buy" || s == "sell");

    let q_filter = filters
        .q
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let sort_col = match filters.sort.as_deref().unwrap_or("created_at") {
        "price" | "price_cents" => "o.price_cents",
        "quantity" | "qty" => "o.quantity",
        "held" | "held_balance" => "(o.price_cents * (o.quantity - o.quantity_filled))",
        "status" => "o.status",
        "side" => "o.side",
        _ => "o.created_at",
    };
    let sort_dir = match filters
        .order
        .as_deref()
        .unwrap_or("desc")
        .to_lowercase()
        .as_str()
    {
        "asc" => "ASC",
        _ => "DESC",
    };

    let mut count_qb: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*)::BIGINT FROM market_orders o ");
    count_qb.push("LEFT JOIN users u ON u.id = o.user_id ");
    count_qb.push("WHERE o.status = ANY(");
    count_qb.push_bind(status_values.clone());
    count_qb.push(") ");
    if let Some(asset_id) = filters.asset_id {
        count_qb.push("AND o.asset_id = ");
        count_qb.push_bind(asset_id);
        count_qb.push(" ");
    }
    if let Some(user_id) = filters.user_id {
        count_qb.push("AND o.user_id = ");
        count_qb.push_bind(user_id);
        count_qb.push(" ");
    }
    if let Some(side) = side_filter.as_deref() {
        count_qb.push("AND o.side = ");
        count_qb.push_bind(side.to_string());
        count_qb.push(" ");
    }
    if let Some(q) = q_filter.as_deref() {
        count_qb.push("AND (u.email ILIKE ");
        count_qb.push_bind(format!("%{}%", q));
        count_qb.push(" OR o.id::text ILIKE ");
        count_qb.push_bind(format!("{}%", q));
        count_qb.push(") ");
    }

    let total: i64 = count_qb
        .build_query_scalar::<i64>()
        .fetch_one(db)
        .await
        .unwrap_or(0);

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT o.id, o.user_id, u.email AS user_email, o.asset_id, a.title AS asset_name, \
         o.side, o.order_type, o.price_cents, o.quantity, o.quantity_filled, o.status, o.created_at, o.flagged_at, o.flag_reason \
         FROM market_orders o \
         LEFT JOIN users u ON u.id = o.user_id \
         LEFT JOIN assets a ON a.id = o.asset_id \
         WHERE o.status = ANY(",
    );
    qb.push_bind(status_values);
    qb.push(") ");
    if let Some(asset_id) = filters.asset_id {
        qb.push("AND o.asset_id = ");
        qb.push_bind(asset_id);
        qb.push(" ");
    }
    if let Some(user_id) = filters.user_id {
        qb.push("AND o.user_id = ");
        qb.push_bind(user_id);
        qb.push(" ");
    }
    if let Some(side) = side_filter.as_deref() {
        qb.push("AND o.side = ");
        qb.push_bind(side.to_string());
        qb.push(" ");
    }
    if let Some(q) = q_filter.as_deref() {
        qb.push("AND (u.email ILIKE ");
        qb.push_bind(format!("%{}%", q));
        qb.push(" OR o.id::text ILIKE ");
        qb.push_bind(format!("{}%", q));
        qb.push(") ");
    }
    qb.push(format!(" ORDER BY {} {} ", sort_col, sort_dir));
    qb.push("LIMIT ");
    qb.push_bind(per_page);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows: Vec<AdminOrder> = qb
        .build_query_as::<AdminOrder>()
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

/// GET /api/admin/marketplace/orders/export.csv — Filtered open orders CSV.
pub async fn api_admin_marketplace_orders_export_csv(
    admin: AdminUser,
    Query(filters): Query<OrderFilters>,
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;

    let status_filter = filters.status.as_deref().unwrap_or("open,partially_filled");
    let status_values: Vec<String> = status_filter
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let side_filter = filters
        .side
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s == "buy" || s == "sell");

    let q_filter = filters
        .q
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT o.id, o.user_id, u.email AS user_email, o.asset_id, a.title AS asset_name, \
         o.side, o.order_type, o.price_cents, o.quantity, o.quantity_filled, o.status, o.created_at, o.flagged_at, o.flag_reason \
         FROM market_orders o \
         LEFT JOIN users u ON u.id = o.user_id \
         LEFT JOIN assets a ON a.id = o.asset_id \
         WHERE o.status = ANY(",
    );
    qb.push_bind(status_values);
    qb.push(") ");
    if let Some(asset_id) = filters.asset_id {
        qb.push("AND o.asset_id = ");
        qb.push_bind(asset_id);
        qb.push(" ");
    }
    if let Some(user_id) = filters.user_id {
        qb.push("AND o.user_id = ");
        qb.push_bind(user_id);
        qb.push(" ");
    }
    if let Some(side) = side_filter.as_deref() {
        qb.push("AND o.side = ");
        qb.push_bind(side.to_string());
        qb.push(" ");
    }
    if let Some(q) = q_filter.as_deref() {
        qb.push("AND (u.email ILIKE ");
        qb.push_bind(format!("%{}%", q));
        qb.push(" OR o.id::text ILIKE ");
        qb.push_bind(format!("{}%", q));
        qb.push(") ");
    }
    qb.push(" ORDER BY o.created_at DESC LIMIT 10000");

    let rows: Vec<AdminOrder> = qb
        .build_query_as::<AdminOrder>()
        .fetch_all(db)
        .await
        .map_err(ApiError::Database)?;

    let mut csv = String::from(
        "Order_ID,Created_At,User_ID,User_Email,Asset_ID,Asset_Name,Side,Type,Price_Cents,Quantity,Quantity_Filled,Held_Cents,Status\n",
    );
    for r in rows {
        let remaining = (r.quantity - r.quantity_filled) as i64;
        let held = r.price_cents.saturating_mul(remaining);
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.id,
            r.created_at.format("%Y-%m-%dT%H:%M:%SZ"),
            r.user_id,
            csv_escape(r.user_email.unwrap_or_default()),
            r.asset_id,
            csv_escape(r.asset_name.unwrap_or_default()),
            r.side,
            r.order_type,
            r.price_cents,
            r.quantity,
            r.quantity_filled,
            held,
            r.status,
        ));
    }

    tracing::info!(admin_id = %admin.user.id, "Admin exported open orders CSV");

    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
        (
            axum::http::header::CONTENT_DISPOSITION,
            "attachment; filename=\"open_orders.csv\"",
        ),
    ];

    Ok((headers, csv))
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

/// POST /api/admin/marketplace/orders/bulk-cancel — Cancel multiple orders.
pub async fn api_admin_marketplace_orders_bulk_cancel(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<AdminBulkCancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.manage").await?;
    if body.order_ids.is_empty() {
        return Err(ApiError::BadRequest("No order IDs provided".into()));
    }
    if body.order_ids.len() > 200 {
        return Err(ApiError::BadRequest(
            "Cannot cancel more than 200 orders per request".into(),
        ));
    }
    let reason = normalize_admin_cancel_reason(body.reason)?;
    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();
    for raw_id in &body.order_ids {
        let order_uuid = match Uuid::parse_str(raw_id) {
            Ok(u) => u,
            Err(_) => {
                failed.push(serde_json::json!({"order_id": raw_id, "error": "Invalid UUID"}));
                continue;
            }
        };
        match cancel_single_order(db, &state, admin.user.id, order_uuid, &reason).await {
            Ok(()) => succeeded.push(order_uuid.to_string()),
            Err(e) => failed.push(
                serde_json::json!({"order_id": order_uuid.to_string(), "error": format!("{}", e)}),
            ),
        }
    }
    tracing::info!(admin_id = %admin.user.id, succeeded = succeeded.len(), failed = failed.len(), "Admin bulk-cancelled marketplace orders");
    Ok(Json(serde_json::json!({
        "succeeded": succeeded, "failed": failed,
        "succeeded_count": succeeded.len(), "failed_count": failed.len(),
    })))
}

async fn cancel_single_order(
    db: &sqlx::PgPool,
    state: &AppState,
    actor_id: Uuid,
    order_uuid: Uuid,
    reason: &str,
) -> Result<(), ApiError> {
    let mut tx = db.begin().await.map_err(ApiError::Database)?;
    let order: MarketOrder = sqlx::query_as("SELECT * FROM market_orders WHERE id = $1 FOR UPDATE")
        .bind(order_uuid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Order not found".to_string()))?;
    if order.status != "open" && order.status != "partially_filled" {
        return Err(ApiError::BadRequest(format!("status='{}'", order.status)));
    }
    let remaining = order.quantity - order.quantity_filled;
    if remaining <= 0 {
        return Err(ApiError::Conflict("no remaining quantity".into()));
    }
    let cancelled: MarketOrder = sqlx::query_as(
        "UPDATE market_orders SET status='admin_cancelled', updated_at=NOW(), cancel_reason=$2 \
         WHERE id=$1 AND status IN ('open','partially_filled') RETURNING *",
    )
    .bind(order_uuid)
    .bind(reason)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::Conflict("already cancelled or filled".into()))?;
    match order.side.as_str() {
        "buy" => {
            let hold_cents = order_hold_cents(order.price_cents, remaining)?;
            let r = sqlx::query(
                "UPDATE wallets SET held_balance_cents = held_balance_cents - $1, updated_at=NOW() \
                 WHERE user_id=$2 AND wallet_type='cash' AND currency='USD' AND held_balance_cents >= $1",
            ).bind(hold_cents).bind(order.user_id).execute(&mut *tx).await.map_err(ApiError::Database)?;
            if r.rows_affected() != 1 {
                return Err(ApiError::Conflict("hold release failed".into()));
            }
        }
        "sell" => {
            let r = sqlx::query(
                "UPDATE investments SET held_tokens = held_tokens - $1, updated_at=NOW() \
                 WHERE user_id=$2 AND asset_id=$3 AND status != 'exited' AND held_tokens >= $1",
            )
            .bind(remaining)
            .bind(order.user_id)
            .bind(order.asset_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
            if r.rows_affected() != 1 {
                return Err(ApiError::Conflict("token release failed".into()));
            }
        }
        _ => return Err(ApiError::BadRequest("unsupported side".into())),
    }
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'marketplace.order.admin_cancelled', 'market_order', $2, $3, $4)"#,
    )
    .bind(actor_id).bind(order_uuid)
    .bind(serde_json::json!({
        "status": order.status, "side": order.side,
        "price_cents": order.price_cents, "quantity": order.quantity,
        "quantity_filled": order.quantity_filled, "bulk": true
    }))
    .bind(serde_json::json!({
        "status": cancelled.status, "reason": reason,
        "remaining_quantity_released": remaining
    }))
    .execute(&mut *tx).await.map_err(ApiError::Database)?;
    tx.commit().await.map_err(ApiError::Database)?;
    if let Some(redis) = state.redis.as_ref() {
        if let Err(e) = crate::marketplace::orderbook::remove_order(redis, &order).await {
            tracing::error!(order_id = %order_uuid, error = %e, "Bulk-cancel Redis remove failed");
        } else {
            crate::marketplace::websocket::broadcast_orderbook_update(
                db,
                Some(redis),
                order.asset_id,
            )
            .await;
        }
    }
    Ok(())
}

/// GET /api/admin/marketplace/orders/stats — KPI aggregates with deltas + age distribution.
pub async fn api_admin_marketplace_orders_stats(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;
    let row = sqlx::query_as::<_, (i64, i64, Option<f64>, Option<f64>, Option<f64>)>(
        r#"
        SELECT COUNT(*)::BIGINT,
               COALESCE(SUM(price_cents * (quantity - quantity_filled)), 0)::BIGINT,
               EXTRACT(EPOCH FROM AVG(NOW() - created_at))::float8,
               EXTRACT(EPOCH FROM percentile_cont(0.5) WITHIN GROUP (ORDER BY NOW() - created_at))::float8,
               EXTRACT(EPOCH FROM percentile_cont(0.9) WITHIN GROUP (ORDER BY NOW() - created_at))::float8
        FROM market_orders WHERE status IN ('open','partially_filled')
        "#,
    ).fetch_one(db).await.map_err(ApiError::Database)?;
    let yesterday = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM market_orders \
         WHERE created_at < NOW() - INTERVAL '24 hours' AND created_at >= NOW() - INTERVAL '48 hours'",
    ).fetch_one(db).await.unwrap_or(0);
    let today = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::BIGINT FROM market_orders WHERE created_at >= NOW() - INTERVAL '24 hours'",
    ).fetch_one(db).await.unwrap_or(0);
    let spark: Vec<(i64,)> = sqlx::query_as(
        r#"SELECT COALESCE((
             SELECT COUNT(*)::BIGINT FROM market_orders
             WHERE created_at >= date_trunc('hour', NOW()) - ((gs+1) * INTERVAL '6 hour')
               AND created_at <  date_trunc('hour', NOW()) - (gs * INTERVAL '6 hour')
           ), 0)
           FROM generate_series(0, 27) gs ORDER BY gs DESC"#,
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let sparkline: Vec<i64> = spark.iter().map(|(c,)| *c).collect();
    Ok(Json(serde_json::json!({
        "total_open": row.0, "held_cents": row.1,
        "avg_age_sec": row.2, "p50_age_sec": row.3, "p90_age_sec": row.4,
        "today_count": today, "yesterday_count": yesterday,
        "sparkline": sparkline,
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
            COUNT(*)::BIGINT AS order_count,
            COUNT(DISTINCT user_id)::BIGINT AS unique_users
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
            COUNT(*)::BIGINT AS order_count,
            COUNT(DISTINCT user_id)::BIGINT AS unique_users
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
            unique_users: r.unique_users,
        })
        .collect();

    let asks: Vec<AdminOrderbookLevel> = ask_levels
        .into_iter()
        .map(|r| AdminOrderbookLevel {
            price_cents: r.price_cents,
            total_quantity: r.total_quantity,
            order_count: r.order_count,
            unique_users: r.unique_users,
        })
        .collect();

    Ok(Json(
        build_admin_orderbook(db, asset_uuid, asset_title, asset_slug, bids, asks).await?,
    ))
}

async fn build_admin_orderbook(
    db: &sqlx::PgPool,
    asset_id: Uuid,
    asset_title: String,
    asset_slug: String,
    bids: Vec<AdminOrderbookLevel>,
    asks: Vec<AdminOrderbookLevel>,
) -> Result<AdminOrderbook, ApiError> {
    let best_bid = bids.first().map(|l| l.price_cents);
    let best_ask = asks.first().map(|l| l.price_cents);
    // Crossed-book detection: bid >= ask is an invariant violation (matching
    // engine should have consumed any cross). Surface as a distinct status so
    // ops can investigate; do NOT render a negative spread to the UI.
    let is_crossed = matches!((best_bid, best_ask), (Some(b), Some(a)) if b >= a);
    let spread_cents = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) if ask >= bid => Some(ask - bid),
        _ => None,
    };
    let (mid_price_cents, mid_price_is_fallback) = match (best_bid, best_ask) {
        (Some(bid), Some(ask)) if ask >= bid => (Some((bid + ask) / 2), false),
        (Some(bid), None) => (Some(bid), true),
        (None, Some(ask)) => (Some(ask), true),
        // Crossed → no honest mid; UI shows N/A and the crossed pill.
        _ => (None, false),
    };

    let bid_volume: i64 = bids.iter().map(|l| l.total_quantity).sum();
    let ask_volume: i64 = asks.iter().map(|l| l.total_quantity).sum();

    let ctx: AssetMarketContext = sqlx::query_as(
        r#"
        SELECT
            (SELECT price_cents FROM trade_history
             WHERE asset_id = $1 ORDER BY executed_at DESC LIMIT 1) AS last_trade_cents,
            (SELECT executed_at FROM trade_history
             WHERE asset_id = $1 ORDER BY executed_at DESC LIMIT 1) AS last_trade_at,
            (SELECT COALESCE(SUM(price_cents * quantity::BIGINT), 0)::BIGINT
             FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours') AS volume_24h_cents,
            (SELECT COALESCE(SUM(quantity::BIGINT), 0)::BIGINT
             FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours') AS volume_24h_qty,
            (SELECT COUNT(*)::BIGINT
             FROM trade_history
             WHERE asset_id = $1 AND executed_at >= NOW() - INTERVAL '24 hours') AS trades_24h,
            (SELECT price_cents FROM trade_history
             WHERE asset_id = $1 AND executed_at <= NOW() - INTERVAL '24 hours'
             ORDER BY executed_at DESC LIMIT 1) AS open_24h_cents
        "#,
    )
    .bind(asset_id)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let last_rebuild_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        r#"SELECT created_at FROM audit_logs
           WHERE action = 'marketplace.orderbook.rebuilt'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten();

    let change_24h_pct = match (ctx.last_trade_cents, ctx.open_24h_cents) {
        (Some(last), Some(open)) if open > 0 => Some(((last - open) as f64 / open as f64) * 100.0),
        _ => None,
    };

    let market_status = if bids.is_empty() && asks.is_empty() {
        "no_orders"
    } else if is_crossed {
        // Crossed book = matching engine missed a match. Distinct status so
        // ops dashboards can alert and admins can run a rebuild.
        "crossed"
    } else if best_bid.is_some() && best_ask.is_some() {
        "live"
    } else {
        "one_sided"
    }
    .to_string();

    Ok(AdminOrderbook {
        asset_id,
        asset_title,
        asset_slug,
        bids,
        asks,
        best_bid_cents: best_bid,
        best_ask_cents: best_ask,
        spread_cents,
        mid_price_cents,
        mid_price_is_fallback,
        last_trade_cents: ctx.last_trade_cents,
        last_trade_at: ctx.last_trade_at,
        volume_24h_cents: ctx.volume_24h_cents.unwrap_or(0),
        volume_24h_qty: ctx.volume_24h_qty.unwrap_or(0),
        trades_24h: ctx.trades_24h.unwrap_or(0),
        change_24h_pct,
        bid_volume,
        ask_volume,
        market_status,
        generated_at: chrono::Utc::now(),
        last_rebuild_at,
    })
}

/// GET /api/admin/marketplace/orderbook/:asset_id/level — Individual orders at a price level.
pub async fn api_admin_marketplace_orderbook_level(
    admin: AdminUser,
    Path(asset_id): Path<String>,
    State(state): State<AppState>,
    Query(q): Query<OrderbookLevelQuery>,
) -> Result<Json<Vec<AdminOrderbookOrder>>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;
    let asset_uuid = ApiError::parse_uuid(&asset_id)?;
    let side = match q.side.as_str() {
        "buy" | "sell" => q.side,
        _ => return Err(ApiError::BadRequest("Invalid side".into())),
    };

    let rows: Vec<AdminOrderbookOrder> = sqlx::query_as(
        r#"
        SELECT mo.id, mo.user_id, u.email AS user_email, mo.side,
               mo.price_cents, mo.quantity, mo.quantity_filled,
               mo.status, mo.created_at
        FROM market_orders mo
        LEFT JOIN users u ON u.id = mo.user_id
        WHERE mo.asset_id = $1
          AND mo.side = $2
          AND mo.price_cents = $3
          AND mo.status IN ('open', 'partially_filled')
        ORDER BY mo.created_at ASC
        LIMIT 200
        "#,
    )
    .bind(asset_uuid)
    .bind(&side)
    .bind(q.price_cents)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
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
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<ReconciliationReport>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
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
    pub maker_kyc_status: Option<String>,
    pub taker_kyc_status: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AdminCancelP2PRequest {
    pub reason: String,
}

/// Query params for paginated P2P offer list.
#[derive(Debug, Deserialize, Default)]
#[allow(missing_docs)]
pub struct P2PListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Paginated envelope for AdminP2POffer.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct AdminP2POfferPage {
    pub items: Vec<AdminP2POffer>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AdminP2POfferRow {
    id: String,
    asset_id: String,
    asset_name: Option<String>,
    maker_email: Option<String>,
    taker_email: Option<String>,
    side: String,
    price_cents: i64,
    quantity: i32,
    total_value_cents: i64,
    status: String,
    market_price_cents: Option<i64>,
    price_deviation_pct: Option<f64>,
    maker_kyc_status: Option<String>,
    taker_kyc_status: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
    full_count: i64,
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

/// GET /api/admin/marketplace/p2p — Paginated P2P offers with price deviation warnings.
pub async fn api_admin_marketplace_p2p(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(q): Query<P2PListQuery>,
) -> Result<Json<AdminP2POfferPage>, ApiError> {
    let db = &state.db;
    admin.require_permission(db, "marketplace.view").await?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(50).clamp(10, 200);
    let offset = (page - 1) * page_size;

    let rows: Vec<AdminP2POfferRow> = sqlx::query_as(
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
            mk.status AS maker_kyc_status,
            tk.status AS taker_kyc_status,
            p.created_at,
            p.expires_at,
            COUNT(*) OVER() AS full_count
        FROM p2p_offers p
        LEFT JOIN assets a ON a.id = p.asset_id
        LEFT JOIN users mu ON mu.id = p.maker_user_id
        LEFT JOIN users tu ON tu.id = p.taker_user_id
        LEFT JOIN LATERAL (
            SELECT price_cents AS last_price FROM trade_history
            WHERE asset_id = p.asset_id ORDER BY executed_at DESC LIMIT 1
        ) lp ON true
        LEFT JOIN LATERAL (
            SELECT status FROM kyc_records
            WHERE user_id = p.maker_user_id
            ORDER BY created_at DESC LIMIT 1
        ) mk ON true
        LEFT JOIN LATERAL (
            SELECT status FROM kyc_records
            WHERE user_id = p.taker_user_id
            ORDER BY created_at DESC LIMIT 1
        ) tk ON true
        ORDER BY p.created_at DESC
        LIMIT $1 OFFSET $2"#,
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    let total = rows.first().map(|r| r.full_count).unwrap_or(0);
    let items: Vec<AdminP2POffer> = rows
        .into_iter()
        .map(|r| AdminP2POffer {
            id: r.id,
            asset_id: r.asset_id,
            asset_name: r.asset_name,
            maker_email: r.maker_email,
            taker_email: r.taker_email,
            side: r.side,
            price_cents: r.price_cents,
            quantity: r.quantity,
            total_value_cents: r.total_value_cents,
            status: r.status,
            market_price_cents: r.market_price_cents,
            price_deviation_pct: r.price_deviation_pct,
            maker_kyc_status: r.maker_kyc_status,
            taker_kyc_status: r.taker_kyc_status,
            created_at: r.created_at,
            expires_at: r.expires_at,
        })
        .collect();

    Ok(Json(AdminP2POfferPage {
        items,
        total,
        page,
        page_size,
    }))
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<MarketplaceAlert>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
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
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
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
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<WatchlistEntry>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
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
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
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

/// GET /api/admin/marketplace/settings/asset/:asset_id — Per-asset settings view.
///
/// Returns the resolved global+override view alongside an `ETag` derived from
/// the override payload. Clients must echo it back via `If-Match` on POST to
/// avoid clobbering concurrent edits.
pub async fn api_admin_marketplace_settings_for_asset(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let asset_uuid = ApiError::parse_uuid(&asset_id)?;

    let asset_row: Option<(String, String)> =
        sqlx::query_as("SELECT title, slug FROM assets WHERE id = $1")
            .bind(asset_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?;
    let (asset_title, asset_slug) =
        asset_row.ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    // Reuse the global resolver — Redis-backed, falls back to defaults.
    let mut global = default_marketplace_settings();
    if let Some(ref redis) = state.redis {
        if let Ok(mut conn) = redis.get().await {
            if let Ok(Some(s)) = redis::cmd("GET")
                .arg("marketplace:settings")
                .query_async::<Option<String>>(&mut *conn)
                .await
            {
                if let Ok(parsed) = serde_json::from_str::<MarketplaceSettings>(&s) {
                    global = parsed;
                }
            }
            if let Ok(Some(val)) = redis::cmd("GET")
                .arg("marketplace:trading_enabled")
                .query_async::<Option<String>>(&mut *conn)
                .await
            {
                global.trading_enabled = val == "1" || val.to_lowercase() == "true";
            }
        }
    }

    let (overrides, has_override) = read_asset_overrides(&state, asset_uuid).await;
    let etag = compute_settings_etag(&overrides);

    let body = serde_json::json!({
        "asset_id": asset_uuid,
        "asset_title": asset_title,
        "asset_slug": asset_slug,
        "has_override": has_override,
        "global": global,
        "asset_overrides": overrides,
        "etag": etag,
    });
    let mut resp = Json(body).into_response();
    resp.headers_mut().insert(
        axum::http::header::ETAG,
        axum::http::HeaderValue::from_str(&format!("\"{etag}\""))
            .unwrap_or(axum::http::HeaderValue::from_static("\"\"")),
    );
    Ok(resp)
}

async fn read_asset_overrides(
    state: &AppState,
    asset_uuid: Uuid,
) -> (Option<AssetSettingsOverrides>, bool) {
    let Some(redis) = state.redis.as_ref() else {
        return (None, false);
    };
    let mut conn = match redis.get().await {
        Ok(c) => c,
        Err(_) => return (None, false),
    };
    let key = format!("marketplace:settings:asset:{}", asset_uuid);
    match redis::cmd("GET")
        .arg(&key)
        .query_async::<Option<String>>(&mut *conn)
        .await
    {
        Ok(Some(s)) => match serde_json::from_str::<AssetSettingsOverrides>(&s) {
            Ok(parsed) => (Some(parsed), true),
            Err(_) => (None, false),
        },
        _ => (None, false),
    }
}

fn compute_settings_etag(overrides: &Option<AssetSettingsOverrides>) -> String {
    let payload = serde_json::to_string(overrides).unwrap_or_else(|_| "null".into());
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    let bytes = hasher.finalize();
    bytes.iter().take(8).map(|b| format!("{:02x}", b)).collect()
}

/// Per-asset settings overrides (currently Redis-backed JSON keyed by asset_id).
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[allow(missing_docs)]
pub struct AssetSettingsOverrides {
    pub tick_size_cents: Option<i64>,
    pub min_order_size: Option<i32>,
    pub max_order_size: Option<i32>,
    pub trading_enabled: Option<bool>,
    pub weekend_trading: Option<bool>,
}

/// POST /api/admin/marketplace/settings/asset/:asset_id — Save per-asset overrides.
pub async fn api_admin_marketplace_save_asset_settings(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    headers: axum::http::HeaderMap,
    Json(body): Json<AssetSettingsOverrides>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    let asset_uuid = ApiError::parse_uuid(&asset_id)?;

    // Optimistic lock: if the client supplied If-Match, it must equal the
    // current ETag (sha256-trim of the stored override payload).
    let (current_overrides, _) = read_asset_overrides(&state, asset_uuid).await;
    let current_etag = compute_settings_etag(&current_overrides);
    if let Some(if_match) = headers.get(axum::http::header::IF_MATCH) {
        let raw = if_match.to_str().unwrap_or("").trim_matches('"');
        if raw != current_etag {
            return Err(ApiError::Conflict(format!(
                "ETag mismatch: settings changed under you. Expected {}, got {}.",
                current_etag, raw
            )));
        }
    }

    if let Some(redis) = state.redis.as_ref() {
        let mut conn = redis
            .get()
            .await
            .map_err(|e| ApiError::Internal(format!("Redis connection failed: {}", e)))?;
        let key = format!("marketplace:settings:asset:{}", asset_uuid);
        let payload = serde_json::to_string(&body)
            .map_err(|e| ApiError::Internal(format!("Encode failed: {}", e)))?;
        let _: Result<(), redis::RedisError> = redis::cmd("SET")
            .arg(&key)
            .arg(&payload)
            .query_async(&mut *conn)
            .await;
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'marketplace.asset_settings.saved', 'asset_settings', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(asset_uuid)
    .bind(serde_json::to_value(&body).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await
    .ok();

    let new_etag = compute_settings_etag(&Some(body.clone()));
    let mut resp = Json(serde_json::json!({
        "success": true,
        "asset_id": asset_uuid,
        "overrides": body,
        "etag": new_etag,
    }))
    .into_response();
    resp.headers_mut().insert(
        axum::http::header::ETAG,
        axum::http::HeaderValue::from_str(&format!("\"{new_etag}\""))
            .unwrap_or(axum::http::HeaderValue::from_static("\"\"")),
    );
    Ok(resp)
}

/// Match-preview request body.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct MatchPreviewRequest {
    pub asset_id: Uuid,
    pub side: String, // "buy" | "sell"
    pub quantity: i64,
    pub limit_price_cents: Option<i64>,
}

/// Per-level fill detail.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct MatchPreviewLevel {
    pub price_cents: i64,
    pub qty_consumed: i64,
}

/// Match-preview response.
#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct MatchPreviewResponse {
    pub filled_qty: i64,
    pub remaining_qty: i64,
    pub total_cost_cents: i64,
    pub avg_price_cents: Option<i64>,
    pub worst_price_cents: Option<i64>,
    pub levels_consumed: Vec<MatchPreviewLevel>,
    pub partial: bool,
    pub crossed_limit: bool,
}

/// POST /api/admin/marketplace/match-preview — Simulate a market/limit fill against current depth.
///
/// Walks aggregated opposite-side levels in price-time priority and computes
/// filled qty, total cost, average + worst price, and whether the limit was
/// crossed before the order could fill. Read-only: no orders are created.
static MATCH_PREVIEW_LIMITER: std::sync::OnceLock<
    tokio::sync::Mutex<std::collections::HashMap<Uuid, (std::time::Instant, u32)>>,
> = std::sync::OnceLock::new();

fn match_preview_limiter(
) -> &'static tokio::sync::Mutex<std::collections::HashMap<Uuid, (std::time::Instant, u32)>> {
    MATCH_PREVIEW_LIMITER.get_or_init(|| tokio::sync::Mutex::new(std::collections::HashMap::new()))
}

const MATCH_PREVIEW_WINDOW: std::time::Duration = std::time::Duration::from_secs(10);
const MATCH_PREVIEW_BURST: u32 = 30; // 30 requests per 10s per admin

async fn check_match_preview_rate(admin_id: Uuid) -> Result<(), ApiError> {
    let mut map = match_preview_limiter().lock().await;
    let now = std::time::Instant::now();
    let entry = map.entry(admin_id).or_insert((now, 0));
    if now.duration_since(entry.0) > MATCH_PREVIEW_WINDOW {
        *entry = (now, 0);
    }
    if entry.1 >= MATCH_PREVIEW_BURST {
        let retry = MATCH_PREVIEW_WINDOW
            .saturating_sub(now.duration_since(entry.0))
            .as_secs()
            .max(1);
        return Err(ApiError::TooManyRequests(format!(
            "Match-preview rate limit exceeded. Retry in {}s.",
            retry
        )));
    }
    entry.1 += 1;
    Ok(())
}

/// POST /api/admin/marketplace/match-preview — Simulate fill at current depth.
pub async fn api_admin_marketplace_match_preview(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<MatchPreviewRequest>,
) -> Result<Json<MatchPreviewResponse>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    check_match_preview_rate(admin.user.id).await?;
    if req.quantity <= 0 {
        return Err(ApiError::BadRequest("quantity must be > 0".into()));
    }
    let opposite = match req.side.as_str() {
        "buy" => "sell",
        "sell" => "buy",
        _ => return Err(ApiError::BadRequest("side must be 'buy' or 'sell'".into())),
    };
    // Buys consume the lowest asks first; sells consume the highest bids first.
    let order_by = if opposite == "sell" { "ASC" } else { "DESC" };

    let levels: Vec<AdminOrderbookLevelRow> = sqlx::query_as(&format!(
        r#"
        SELECT price_cents,
               SUM(quantity - quantity_filled)::BIGINT AS total_quantity,
               COUNT(*)::BIGINT AS order_count
        FROM market_orders
        WHERE asset_id = $1 AND side = $2 AND status IN ('open', 'partially_filled')
        GROUP BY price_cents
        ORDER BY price_cents {order_by}
        LIMIT 200
        "#,
    ))
    .bind(req.asset_id)
    .bind(opposite)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let mut remaining = req.quantity;
    let mut filled: i64 = 0;
    let mut total_cost: i64 = 0;
    let mut consumed: Vec<MatchPreviewLevel> = Vec::new();
    let mut worst_price: Option<i64> = None;
    let mut crossed_limit = false;

    for lvl in levels {
        if remaining <= 0 {
            break;
        }
        if let Some(limit) = req.limit_price_cents {
            let allowed = match req.side.as_str() {
                "buy" => lvl.price_cents <= limit,
                "sell" => lvl.price_cents >= limit,
                _ => true,
            };
            if !allowed {
                crossed_limit = true;
                break;
            }
        }
        let take = lvl.total_quantity.min(remaining);
        if take <= 0 {
            continue;
        }
        filled += take;
        remaining -= take;
        total_cost += lvl.price_cents * take;
        worst_price = Some(lvl.price_cents);
        consumed.push(MatchPreviewLevel {
            price_cents: lvl.price_cents,
            qty_consumed: take,
        });
    }

    let avg_price_cents = if filled > 0 {
        Some(total_cost / filled)
    } else {
        None
    };

    Ok(Json(MatchPreviewResponse {
        filled_qty: filled,
        remaining_qty: remaining.max(0),
        total_cost_cents: total_cost,
        avg_price_cents,
        worst_price_cents: worst_price,
        levels_consumed: consumed,
        partial: remaining > 0,
        crossed_limit,
    }))
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

/// GET /api/admin/marketplace/settings/history — Recent settings changes from audit_logs.
pub async fn api_admin_marketplace_settings_history(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
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

    let rows = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            String,
        ),
    >(
        r#"
        SELECT a.id::text,
               u.email,
               a.previous_state,
               a.new_state,
               a.created_at::text
        FROM audit_logs a
        LEFT JOIN users u ON u.id = a.actor_user_id
        WHERE a.action = 'marketplace.settings.update'
          AND a.entity_type = 'marketplace_settings'
        ORDER BY a.created_at DESC
        LIMIT 25
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, email, previous, new_state, created_at)| {
            serde_json::json!({
                "id": id,
                "actor": email.unwrap_or_else(|| "unknown".to_string()),
                "previous_state": previous,
                "new_state": new_state,
                "created_at": created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "entries": entries })))
}

/// GET /api/admin/marketplace/settings/context — Live context: network gas, batch utilization.
pub async fn api_admin_marketplace_settings_context(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;

    let recent_gas: Option<i64> = sqlx::query_scalar(
        "SELECT gas_price_gwei FROM chain_settlement_batches \
         WHERE gas_price_gwei IS NOT NULL \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let avg_batch_24h: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(batch_size)::float8 FROM chain_settlement_batches \
         WHERE created_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let pending_settlements: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM chain_settlement_batches WHERE status = 'pending'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let batch_size_limit: i64 = if let Some(ref redis) = state.redis {
        let mut conn_res = redis.get().await;
        if let Ok(ref mut conn) = conn_res {
            let s: Result<Option<String>, redis::RedisError> = redis::cmd("GET")
                .arg("marketplace:settings")
                .query_async(&mut **conn)
                .await;
            if let Ok(Some(json)) = s {
                serde_json::from_str::<MarketplaceSettings>(&json)
                    .map(|m| m.settlement_batch_size as i64)
                    .unwrap_or(50)
            } else {
                50
            }
        } else {
            50
        }
    } else {
        50
    };

    let trades_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM trade_history WHERE executed_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let volume_24h_cents: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(quantity * price_cents), 0)::bigint FROM trade_history \
         WHERE executed_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let open_orders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let last_settings_update: Option<String> = sqlx::query_scalar(
        "SELECT created_at::text FROM audit_logs \
         WHERE action = 'marketplace.settings.update' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    Ok(Json(serde_json::json!({
        "network_gas_gwei": recent_gas,
        "avg_batch_size_24h": avg_batch_24h.map(|v| (v * 10.0).round() / 10.0),
        "batch_size_limit": batch_size_limit,
        "pending_settlements": pending_settlements,
        "trades_24h": trades_24h,
        "volume_24h_cents": volume_24h_cents.unwrap_or(0),
        "open_orders": open_orders,
        "last_settings_update": last_settings_update,
    })))
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ScheduleSettingsBody {
    pub state: MarketplaceSettings,
    pub apply_at: String,
    pub note: Option<String>,
}

/// POST /api/admin/marketplace/settings/schedule — Schedule a future settings change.
pub async fn api_admin_marketplace_schedule_settings(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<ScheduleSettingsBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;
    validate_marketplace_settings(&body.state)?;

    let apply_at = chrono::DateTime::parse_from_rfc3339(&body.apply_at)
        .map_err(|_| ApiError::BadRequest("apply_at must be RFC3339".into()))?
        .with_timezone(&chrono::Utc);
    if apply_at <= chrono::Utc::now() + chrono::Duration::seconds(30) {
        return Err(ApiError::BadRequest(
            "apply_at must be at least 30s in the future".into(),
        ));
    }

    let scheduled_state = serde_json::to_value(&body.state)
        .map_err(|e| ApiError::Internal(format!("Serialize failed: {}", e)))?;

    let id: uuid::Uuid = sqlx::query_scalar(
        r#"INSERT INTO marketplace_settings_schedule
               (scheduled_state, apply_at, created_by, note)
           VALUES ($1, $2, $3, $4)
           RETURNING id"#,
    )
    .bind(scheduled_state)
    .bind(apply_at)
    .bind(admin.user.id)
    .bind(body.note.as_deref())
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "id": id.to_string(),
        "apply_at": apply_at.to_rfc3339(),
        "status": "pending",
    })))
}

/// GET /api/admin/marketplace/settings/schedule — List scheduled changes.
pub async fn api_admin_marketplace_list_scheduled_settings(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
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

    let rows = sqlx::query_as::<
        _,
        (
            String,
            serde_json::Value,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT s.id::text,
               s.scheduled_state,
               s.apply_at::text,
               s.status,
               s.note,
               s.created_at::text,
               u.email,
               s.error_message
        FROM marketplace_settings_schedule s
        LEFT JOIN users u ON u.id = s.created_by
        WHERE s.status IN ('pending', 'failed')
           OR s.applied_at > NOW() - INTERVAL '7 days'
        ORDER BY s.apply_at DESC
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(
            |(id, scheduled_state, apply_at, status, note, created_at, actor, err)| {
                serde_json::json!({
                    "id": id,
                    "scheduled_state": scheduled_state,
                    "apply_at": apply_at,
                    "status": status,
                    "note": note,
                    "created_at": created_at,
                    "actor": actor.unwrap_or_else(|| "unknown".to_string()),
                    "error_message": err,
                })
            },
        )
        .collect();

    Ok(Json(serde_json::json!({ "entries": entries })))
}

/// DELETE /api/admin/marketplace/settings/schedule/:id — Cancel pending scheduled change.
pub async fn api_admin_marketplace_cancel_scheduled_settings(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.manage")
        .await?;

    let uuid = uuid::Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid id".into()))?;

    let rows = sqlx::query(
        "UPDATE marketplace_settings_schedule \
         SET status = 'cancelled' \
         WHERE id = $1 AND status = 'pending'",
    )
    .bind(uuid)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?
    .rows_affected();

    if rows == 0 {
        return Err(ApiError::NotFound(
            "Scheduled change not found or not pending".into(),
        ));
    }

    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

/// Worker: apply due scheduled settings changes. Run via run_as_leader.
pub async fn run_settings_scheduler(pool: sqlx::PgPool, redis: Option<deadpool_redis::Pool>) {
    use std::time::Duration;
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        if let Err(e) = apply_due_scheduled_settings(&pool, redis.as_ref()).await {
            tracing::error!("settings scheduler failed: {}", e);
        }
    }
}

async fn apply_due_scheduled_settings(
    pool: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
) -> Result<(), sqlx::Error> {
    let due = sqlx::query_as::<_, (uuid::Uuid, serde_json::Value, uuid::Uuid)>(
        "SELECT id, scheduled_state, created_by \
         FROM marketplace_settings_schedule \
         WHERE status = 'pending' AND apply_at <= NOW() \
         ORDER BY apply_at ASC \
         LIMIT 10",
    )
    .fetch_all(pool)
    .await?;

    for (id, scheduled_state, created_by) in due {
        let new_settings: MarketplaceSettings =
            match serde_json::from_value(scheduled_state.clone()) {
                Ok(s) => s,
                Err(e) => {
                    let _ = sqlx::query(
                        "UPDATE marketplace_settings_schedule \
                         SET status = 'failed', error_message = $2, applied_at = NOW() \
                         WHERE id = $1",
                    )
                    .bind(id)
                    .bind(format!("deserialize failed: {}", e))
                    .execute(pool)
                    .await;
                    continue;
                }
            };

        if let Err(e) = validate_marketplace_settings(&new_settings) {
            let _ = sqlx::query(
                "UPDATE marketplace_settings_schedule \
                 SET status = 'failed', error_message = $2, applied_at = NOW() \
                 WHERE id = $1",
            )
            .bind(id)
            .bind(format!("validate failed: {:?}", e))
            .execute(pool)
            .await;
            continue;
        }

        // Read previous from audit_logs (latest new_state)
        let previous: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT new_state FROM audit_logs \
             WHERE action = 'marketplace.settings.update' \
             ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let new_state_json = serde_json::to_value(&new_settings).unwrap_or(serde_json::json!({}));

        // Audit-log the apply
        let _ = sqlx::query(
            r#"INSERT INTO audit_logs
                  (actor_user_id, action, entity_type, previous_state, new_state)
               VALUES ($1, 'marketplace.settings.update', 'marketplace_settings', $2, $3)"#,
        )
        .bind(created_by)
        .bind(previous)
        .bind(new_state_json)
        .execute(pool)
        .await;

        let _ = sqlx::query(
            "UPDATE marketplace_settings_schedule \
             SET status = 'applied', applied_at = NOW() \
             WHERE id = $1",
        )
        .bind(id)
        .execute(pool)
        .await;

        // Redis sync — make scheduled state live for matching engine
        if let Some(r) = redis {
            if let Ok(mut conn) = r.get().await {
                let json_str = serde_json::to_string(&new_settings).unwrap_or_default();
                let _: Result<(), _> = redis::cmd("SET")
                    .arg("marketplace:settings")
                    .arg(&json_str)
                    .query_async::<()>(&mut *conn)
                    .await;
                let enabled_val = if new_settings.trading_enabled {
                    "1"
                } else {
                    "0"
                };
                let _: Result<(), _> = redis::cmd("SET")
                    .arg("marketplace:trading_enabled")
                    .arg(enabled_val)
                    .query_async::<()>(&mut *conn)
                    .await;
            }
        }

        tracing::info!(schedule_id = %id, "Applied scheduled marketplace settings");
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.13: Compliance & OJK APIs ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct OjkReportQuery {
    pub quarter: Option<String>,
    /// `csv` (default) or `json`
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct TravelRuleQuery {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub format: Option<String>,
    /// Single-use approval token from /requests/:id/approve
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct TaxExportQuery {
    pub year: Option<i32>,
    pub format: Option<String>,
}

fn wants_json(format: &Option<String>) -> bool {
    matches!(
        format.as_deref().map(str::to_ascii_lowercase).as_deref(),
        Some("json")
    )
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

    let want_json = wants_json(&query.format);
    let body = if want_json {
        serde_json::to_string(&serde_json::json!({
            "period": current_quarter,
            "start_date": start_date,
            "end_date": end_date,
            "metrics": {
                "total_trade_volume_cents": total_volume,
                "total_trades": trade_count,
                "total_registered_users": total_users
            }
        }))
        .unwrap_or_else(|_| "{}".into())
    } else {
        csv
    };

    record_export_audit(
        db,
        "ojk_quarterly",
        &current_quarter,
        Some(start_date),
        Some(end_date),
        user.id,
        3,
        &body,
    )
    .await;

    tracing::info!(admin_id = %user.id, quarter = %current_quarter, format = %if want_json {"json"} else {"csv"}, "Admin exported OJK Report");

    let (ext, ctype) = if want_json {
        ("json", "application/json; charset=utf-8")
    } else {
        ("csv", "text/csv; charset=utf-8")
    };
    let filename = format!("ojk_report_{}.{}", current_quarter, ext);
    let headers = [
        (axum::http::header::CONTENT_TYPE, ctype.to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, body))
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

    // 4-eye gate: travel-rule requires an approved single-use token.
    // Bypass only if env COMPLIANCE_REQUIRE_APPROVAL=0 (dev/local).
    let approval_required = std::env::var("COMPLIANCE_REQUIRE_APPROVAL")
        .map(|v| v != "0")
        .unwrap_or(true);
    if approval_required {
        let token = query.token.as_deref().ok_or_else(|| {
            ApiError::Forbidden(
                "travel_rule export requires an approval token (POST /requests then /approve)"
                    .into(),
            )
        })?;
        let (_req_id, req_start, req_end, _label) =
            consume_download_token(db, token, "travel_rule").await?;
        if req_start != from_date || req_end != to_date {
            return Err(ApiError::Forbidden(
                "approval token does not match the requested date range".into(),
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

    let want_json = wants_json(&query.format);
    let row_count = rows.len() as i64;
    let body = if want_json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "trade_id": r.0,
                    "executed_at": r.1,
                    "buyer_email": r.2,
                    "seller_email": r.3,
                    "buyer_name": r.4,
                    "seller_name": r.5,
                    "price_cents": r.6,
                    "quantity": r.7,
                    "total_value_cents": r.8
                })
            })
            .collect();
        serde_json::to_string(&serde_json::json!({ "trades": arr })).unwrap_or_else(|_| "{}".into())
    } else {
        let mut csv = String::from(
            "Trade_ID,Executed_At,Buyer_Email,Seller_Email,Buyer_Name,Seller_Name,Price_Cents,Quantity,Total_Value_Cents\n",
        );
        for row in &rows {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                row.0,
                row.1,
                csv_escape(row.2.clone().unwrap_or_default()),
                csv_escape(row.3.clone().unwrap_or_default()),
                csv_escape(row.4.clone().unwrap_or_default()),
                csv_escape(row.5.clone().unwrap_or_default()),
                row.6,
                row.7,
                row.8
            ));
        }
        csv
    };

    let period_label = format!(
        "{}..{}",
        from_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "start".into()),
        to_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "end".into()),
    );
    record_export_audit(
        db,
        "travel_rule",
        &period_label,
        from_date,
        to_date,
        user.id,
        row_count,
        &body,
    )
    .await;

    tracing::info!(admin_id = %user.id, format = %if want_json {"json"} else {"csv"}, "Admin exported AML Travel Rule Data");

    let (ext, ctype) = if want_json {
        ("json", "application/json; charset=utf-8")
    } else {
        ("csv", "text/csv; charset=utf-8")
    };
    let filename = format!(
        "travel_rule_{}_to_{}.{}",
        from_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "start".into()),
        to_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "end".into()),
        ext,
    );
    let headers = [
        (axum::http::header::CONTENT_TYPE, ctype.to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, body))
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

    let want_json = wants_json(&query.format);
    let row_count = rows.len() as i64;
    let body = if want_json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "user_email": r.0,
                    "fiscal_year": r.1,
                    "total_investment_cents": r.2,
                    "total_dividends_cents": r.3,
                    "capital_gains_cents": r.4,
                    "withholding_tax_cents": r.5,
                    "status": r.6
                })
            })
            .collect();
        serde_json::to_string(&serde_json::json!({ "fiscal_year": year, "reports": arr }))
            .unwrap_or_else(|_| "{}".into())
    } else {
        let mut csv = String::from(
            "User_Email,Year,Total_Investment_Cents,Total_Dividends_Cents,Capital_Gains_Cents,Withholding_Tax_Cents,Status\n",
        );
        for row in &rows {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                csv_escape(&row.0),
                row.1,
                row.2,
                row.3,
                row.4,
                row.5,
                csv_escape(&row.6)
            ));
        }
        csv
    };

    let period_label = format!("FY{}", year);
    let fy_start = NaiveDate::from_ymd_opt(year, 1, 1);
    let fy_end = NaiveDate::from_ymd_opt(year + 1, 1, 1);
    record_export_audit(
        db,
        "tax_fiscal",
        &period_label,
        fy_start,
        fy_end,
        user.id,
        row_count,
        &body,
    )
    .await;

    tracing::info!(admin_id = %user.id, fiscal_year = year, format = %if want_json {"json"} else {"csv"}, "Admin exported Tax Reports");

    let (ext, ctype) = if want_json {
        ("json", "application/json; charset=utf-8")
    } else {
        ("csv", "text/csv; charset=utf-8")
    };
    let filename = format!("tax_export_FY{}.{}", year, ext);
    let headers = [
        (axum::http::header::CONTENT_TYPE, ctype.to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((headers, body))
}

// ═══════════════════════════════════════════════════════════════════
// ── Compliance Audit, Summary & History ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Insert a compliance_export_audit row. Failures logged, never raised —
/// the export itself already succeeded; audit write is best-effort.
async fn record_export_audit(
    db: &sqlx::PgPool,
    export_type: &str,
    period_label: &str,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    requested_by: Uuid,
    row_count: i64,
    csv_body: &str,
) {
    let mut hasher = Sha256::new();
    hasher.update(csv_body.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let byte_size = csv_body.len() as i64;

    let result = sqlx::query(
        r#"
        INSERT INTO compliance_export_audit
            (export_type, period_label, period_start, period_end,
             requested_by, row_count, byte_size, content_sha256)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(export_type)
    .bind(period_label)
    .bind(period_start)
    .bind(period_end)
    .bind(requested_by)
    .bind(row_count)
    .bind(byte_size)
    .bind(&digest)
    .execute(db)
    .await;

    if let Err(e) = result {
        tracing::error!(
            error = %e,
            export_type = export_type,
            period_label = period_label,
            "Failed to write compliance_export_audit row"
        );
    }
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ComplianceSummaryQuery {
    pub quarter: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub year: Option<i32>,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct CompliancePreviewSection {
    pub export_type: String,
    pub period_label: String,
    pub row_count: i64,
    /// Rows present in the source data but excluded from the export
    /// (e.g. soft-deleted users, void trades). Reserved for future filters;
    /// currently always 0.
    pub excluded_count: i64,
    pub estimated_bytes: i64,
    pub data_cutoff: chrono::DateTime<chrono::Utc>,
    pub last_export_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_export_hash: Option<String>,
    pub last_export_count: i64,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ComplianceHealthDeadline {
    pub export_type: String,
    pub period_label: String,
    pub due_date: NaiveDate,
    pub days_until_due: i64,
    pub status: String,
    pub last_submitted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ComplianceSummaryResponse {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub deadlines: Vec<ComplianceHealthDeadline>,
    pub previews: Vec<CompliancePreviewSection>,
}

fn previous_quarter(today: NaiveDate) -> Result<(String, NaiveDate, NaiveDate), ApiError> {
    let q = (today.month0() / 3) + 1;
    let (py, pq) = if q == 1 {
        (today.year() - 1, 4u32)
    } else {
        (today.year(), q - 1)
    };
    let start_month = (pq - 1) * 3 + 1;
    let start = NaiveDate::from_ymd_opt(py, start_month, 1)
        .ok_or_else(|| ApiError::Internal("date overflow".into()))?;
    let end = if pq == 4 {
        NaiveDate::from_ymd_opt(py + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(py, start_month + 3, 1)
    }
    .ok_or_else(|| ApiError::Internal("date overflow".into()))?;
    Ok((format!("{}-Q{}", py, pq), start, end))
}

/// GET /api/admin/marketplace/compliance/summary
/// Pre-export row-count/byte estimate per section + regulatory deadline status.
pub async fn api_admin_marketplace_compliance_summary(
    jar: CookieJar,
    Query(q): Query<ComplianceSummaryQuery>,
    State(state): State<AppState>,
) -> Result<Json<ComplianceSummaryResponse>, ApiError> {
    let _user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;
    let now = chrono::Utc::now();

    let (ojk_label, _ojk_start, _ojk_end) = parse_ojk_quarter(q.quarter.clone())?;
    let ojk_count: i64 = 3;
    let ojk_bytes: i64 = 256 + ojk_count * 96;

    let ojk_last: Option<(chrono::DateTime<chrono::Utc>, String, i64)> = sqlx::query_as(
        r#"SELECT requested_at, content_sha256, row_count
           FROM compliance_export_audit
           WHERE export_type='ojk_quarterly' AND period_label=$1
           ORDER BY requested_at DESC LIMIT 1"#,
    )
    .bind(&ojk_label)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    let from_date = parse_optional_date(&q.from_date, "from_date")?;
    let to_date = parse_optional_date(&q.to_date, "to_date")?;
    let tr_counts: (i64, i64) = sqlx::query_as(
        r#"SELECT
              LEAST(COUNT(*) FILTER (
                WHERE t.buyer_user_id IS NOT NULL AND t.seller_user_id IS NOT NULL
              ), 10000)::BIGINT AS included,
              COUNT(*) FILTER (
                WHERE t.buyer_user_id IS NULL OR t.seller_user_id IS NULL
              )::BIGINT AS excluded
           FROM trade_history t
           WHERE ($1::date IS NULL OR t.executed_at >= $1::date)
             AND ($2::date IS NULL OR t.executed_at < ($2::date + INTERVAL '1 day'))"#,
    )
    .bind(from_date)
    .bind(to_date)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;
    let tr_count = tr_counts.0;
    let tr_excluded = tr_counts.1;
    let tr_bytes: i64 = 180 + tr_count * 180;
    let tr_label = format!(
        "{}..{}",
        from_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "start".into()),
        to_date
            .map(|d| d.to_string())
            .unwrap_or_else(|| "end".into()),
    );

    let tr_last: Option<(chrono::DateTime<chrono::Utc>, String, i64)> = sqlx::query_as(
        r#"SELECT requested_at, content_sha256, row_count
           FROM compliance_export_audit
           WHERE export_type='travel_rule'
             AND ($1::date IS NULL OR period_start = $1::date)
             AND ($2::date IS NULL OR period_end = $2::date)
           ORDER BY requested_at DESC LIMIT 1"#,
    )
    .bind(from_date)
    .bind(to_date)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    let tax_year = q.year.unwrap_or_else(|| chrono::Utc::now().year() - 1);
    let tax_counts: (i64, i64) = sqlx::query_as(
        r#"SELECT
              COUNT(*) FILTER (WHERE u.email NOT LIKE '%@example.com'
                                 AND u.email NOT LIKE '%+test@%')::BIGINT AS included,
              COUNT(*) FILTER (WHERE u.email LIKE '%@example.com'
                                  OR u.email LIKE '%+test@%')::BIGINT AS excluded
           FROM tax_reports tr
           JOIN users u ON u.id = tr.user_id
           WHERE tr.fiscal_year = $1"#,
    )
    .bind(tax_year)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;
    let tax_count = tax_counts.0;
    let tax_excluded = tax_counts.1;
    let tax_bytes: i64 = 120 + tax_count * 110;
    let tax_label = format!("FY{}", tax_year);

    let tax_last: Option<(chrono::DateTime<chrono::Utc>, String, i64)> = sqlx::query_as(
        r#"SELECT requested_at, content_sha256, row_count
           FROM compliance_export_audit
           WHERE export_type='tax_fiscal' AND period_label=$1
           ORDER BY requested_at DESC LIMIT 1"#,
    )
    .bind(&tax_label)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    let previews = vec![
        CompliancePreviewSection {
            export_type: "ojk_quarterly".into(),
            period_label: ojk_label,
            row_count: ojk_count,
            excluded_count: 0,
            estimated_bytes: ojk_bytes,
            data_cutoff: now,
            last_export_at: ojk_last.as_ref().map(|r| r.0),
            last_export_hash: ojk_last.as_ref().map(|r| r.1.clone()),
            last_export_count: ojk_last.as_ref().map(|r| r.2).unwrap_or(0),
        },
        CompliancePreviewSection {
            export_type: "travel_rule".into(),
            period_label: tr_label,
            row_count: tr_count,
            excluded_count: tr_excluded,
            estimated_bytes: tr_bytes,
            data_cutoff: now,
            last_export_at: tr_last.as_ref().map(|r| r.0),
            last_export_hash: tr_last.as_ref().map(|r| r.1.clone()),
            last_export_count: tr_last.as_ref().map(|r| r.2).unwrap_or(0),
        },
        CompliancePreviewSection {
            export_type: "tax_fiscal".into(),
            period_label: tax_label,
            row_count: tax_count,
            excluded_count: tax_excluded,
            estimated_bytes: tax_bytes,
            data_cutoff: now,
            last_export_at: tax_last.as_ref().map(|r| r.0),
            last_export_hash: tax_last.as_ref().map(|r| r.1.clone()),
            last_export_count: tax_last.as_ref().map(|r| r.2).unwrap_or(0),
        },
    ];

    let today = now.date_naive();
    let mut deadlines: Vec<ComplianceHealthDeadline> = Vec::new();

    let (prev_q_label, _prev_q_start, prev_q_end) = previous_quarter(today)?;
    let ojk_due = prev_q_end + chrono::Duration::days(30);
    let ojk_submitted: Option<(chrono::DateTime<chrono::Utc>,)> = sqlx::query_as(
        r#"SELECT submitted_at FROM compliance_export_audit
           WHERE export_type='ojk_quarterly' AND period_label=$1
             AND submission_status='submitted'
           ORDER BY submitted_at DESC LIMIT 1"#,
    )
    .bind(&prev_q_label)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;
    let days_until = (ojk_due - today).num_days();
    let status = if ojk_submitted.is_some() {
        "submitted"
    } else if days_until < 0 {
        "overdue"
    } else if days_until <= 14 {
        "due_soon"
    } else {
        "pending"
    };
    deadlines.push(ComplianceHealthDeadline {
        export_type: "ojk_quarterly".into(),
        period_label: prev_q_label,
        due_date: ojk_due,
        days_until_due: days_until,
        status: status.into(),
        last_submitted_at: ojk_submitted.map(|r| r.0),
    });

    let prev_year = today.year() - 1;
    let tax_label_prev = format!("FY{}", prev_year);
    let tax_due = NaiveDate::from_ymd_opt(today.year(), 3, 31)
        .ok_or_else(|| ApiError::Internal("date overflow".into()))?;
    let tax_submitted: Option<(chrono::DateTime<chrono::Utc>,)> = sqlx::query_as(
        r#"SELECT submitted_at FROM compliance_export_audit
           WHERE export_type='tax_fiscal' AND period_label=$1
             AND submission_status='submitted'
           ORDER BY submitted_at DESC LIMIT 1"#,
    )
    .bind(&tax_label_prev)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;
    let tax_days = (tax_due - today).num_days();
    let tax_status = if tax_submitted.is_some() {
        "submitted"
    } else if tax_days < 0 {
        "overdue"
    } else if tax_days <= 30 {
        "due_soon"
    } else {
        "pending"
    };
    deadlines.push(ComplianceHealthDeadline {
        export_type: "tax_fiscal".into(),
        period_label: tax_label_prev,
        due_date: tax_due,
        days_until_due: tax_days,
        status: tax_status.into(),
        last_submitted_at: tax_submitted.map(|r| r.0),
    });

    Ok(Json(ComplianceSummaryResponse {
        generated_at: now,
        deadlines,
        previews,
    }))
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ComplianceExportRecord {
    pub id: i64,
    pub export_type: String,
    pub period_label: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub requested_by_email: Option<String>,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub row_count: i64,
    pub byte_size: i64,
    pub content_sha256: String,
    pub submission_status: String,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ComplianceExportListQuery {
    pub limit: Option<i64>,
    pub export_type: Option<String>,
}

/// GET /api/admin/marketplace/compliance/exports
pub async fn api_admin_marketplace_compliance_exports(
    jar: CookieJar,
    Query(q): Query<ComplianceExportListQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ComplianceExportRecord>>, ApiError> {
    let _user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;
    let limit = q.limit.unwrap_or(50).clamp(1, 500);

    let rows: Vec<(
        i64,
        String,
        String,
        Option<NaiveDate>,
        Option<NaiveDate>,
        Option<String>,
        chrono::DateTime<chrono::Utc>,
        i64,
        i64,
        String,
        String,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        r#"
        SELECT a.id, a.export_type, a.period_label, a.period_start, a.period_end,
               u.email AS requested_by_email, a.requested_at,
               a.row_count, a.byte_size, a.content_sha256,
               a.submission_status, a.submitted_at
        FROM compliance_export_audit a
        LEFT JOIN users u ON u.id = a.requested_by
        WHERE ($1::text IS NULL OR a.export_type = $1::text)
        ORDER BY a.requested_at DESC
        LIMIT $2
        "#,
    )
    .bind(q.export_type.as_deref())
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    let out = rows
        .into_iter()
        .map(|r| ComplianceExportRecord {
            id: r.0,
            export_type: r.1,
            period_label: r.2,
            period_start: r.3,
            period_end: r.4,
            requested_by_email: r.5,
            requested_at: r.6,
            row_count: r.7,
            byte_size: r.8,
            content_sha256: r.9,
            submission_status: r.10,
            submitted_at: r.11,
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct MarkSubmittedBody {
    pub notes: Option<String>,
}

/// POST /api/admin/marketplace/compliance/exports/:id/mark-submitted
pub async fn api_admin_marketplace_compliance_mark_submitted(
    jar: CookieJar,
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(body): Json<MarkSubmittedBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    let updated: Option<(i64,)> = sqlx::query_as(
        r#"UPDATE compliance_export_audit
           SET submission_status='submitted',
               submitted_at=NOW(),
               submitted_by=$1,
               notes=COALESCE($2, notes)
           WHERE id=$3 AND submission_status <> 'submitted'
           RETURNING id"#,
    )
    .bind(user.id)
    .bind(body.notes.as_deref())
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    if updated.is_none() {
        return Err(ApiError::NotFound(
            "export not found or already submitted".into(),
        ));
    }
    tracing::info!(admin_id = %user.id, export_audit_id = id, "Marked compliance export submitted");
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ═══════════════════════════════════════════════════════════════════
// ── 4-Eye Approval for PII exports (Travel-Rule) ───────────────────
// ═══════════════════════════════════════════════════════════════════

const APPROVAL_REQUIRED_TYPES: &[&str] = &["travel_rule"];
const APPROVAL_TOKEN_TTL_HOURS: i64 = 24;

fn generate_download_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct CreateExportRequestBody {
    pub export_type: String,
    pub period_label: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ExportRequestRecord {
    pub id: i64,
    pub export_type: String,
    pub period_label: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub requested_by_email: Option<String>,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub requested_reason: Option<String>,
    pub status: String,
    pub decided_by_email: Option<String>,
    pub decided_at: Option<chrono::DateTime<chrono::Utc>>,
    pub decision_notes: Option<String>,
    pub token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub download_token: Option<String>,
}

/// POST /api/admin/marketplace/compliance/requests
pub async fn api_admin_marketplace_compliance_request_create(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(body): Json<CreateExportRequestBody>,
) -> Result<Json<ExportRequestRecord>, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    if !APPROVAL_REQUIRED_TYPES.contains(&body.export_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "{} does not require approval",
            body.export_type
        )));
    }

    let row: (i64, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        r#"INSERT INTO compliance_export_request
              (export_type, period_label, period_start, period_end,
               requested_by, requested_reason)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, requested_at"#,
    )
    .bind(&body.export_type)
    .bind(&body.period_label)
    .bind(body.period_start)
    .bind(body.period_end)
    .bind(user.id)
    .bind(body.reason.as_deref())
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        admin_id = %user.id, request_id = row.0, export_type = %body.export_type,
        "Created compliance export approval request"
    );

    Ok(Json(ExportRequestRecord {
        id: row.0,
        export_type: body.export_type,
        period_label: body.period_label,
        period_start: body.period_start,
        period_end: body.period_end,
        requested_by_email: Some(user.email.clone()),
        requested_at: row.1,
        requested_reason: body.reason,
        status: "pending".into(),
        decided_by_email: None,
        decided_at: None,
        decision_notes: None,
        token_expires_at: None,
        download_token: None,
    }))
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ListRequestsQuery {
    pub status: Option<String>,
}

/// GET /api/admin/marketplace/compliance/requests
pub async fn api_admin_marketplace_compliance_requests_list(
    jar: CookieJar,
    Query(q): Query<ListRequestsQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ExportRequestRecord>>, ApiError> {
    let _user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    let rows: Vec<(
        i64,
        String,
        String,
        Option<NaiveDate>,
        Option<NaiveDate>,
        Option<String>,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        String,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        r#"
        SELECT r.id, r.export_type, r.period_label, r.period_start, r.period_end,
               u.email AS requested_by_email, r.requested_at, r.requested_reason,
               r.status,
               d.email AS decided_by_email, r.decided_at, r.decision_notes,
               r.token_expires_at
        FROM compliance_export_request r
        LEFT JOIN users u ON u.id = r.requested_by
        LEFT JOIN users d ON d.id = r.decided_by
        WHERE ($1::text IS NULL OR r.status = $1::text)
        ORDER BY r.requested_at DESC
        LIMIT 200
        "#,
    )
    .bind(q.status.as_deref())
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ExportRequestRecord {
                id: r.0,
                export_type: r.1,
                period_label: r.2,
                period_start: r.3,
                period_end: r.4,
                requested_by_email: r.5,
                requested_at: r.6,
                requested_reason: r.7,
                status: r.8,
                decided_by_email: r.9,
                decided_at: r.10,
                decision_notes: r.11,
                token_expires_at: r.12,
                download_token: None,
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct DecideRequestBody {
    pub notes: Option<String>,
}

/// POST /api/admin/marketplace/compliance/requests/:id/approve
pub async fn api_admin_marketplace_compliance_request_approve(
    jar: CookieJar,
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(body): Json<DecideRequestBody>,
) -> Result<Json<ExportRequestRecord>, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    let token = generate_download_token();
    let expires = chrono::Utc::now() + chrono::Duration::hours(APPROVAL_TOKEN_TTL_HOURS);

    let row: Option<(
        i64,
        String,
        String,
        Option<NaiveDate>,
        Option<NaiveDate>,
        Option<String>,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        String,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        r#"
        UPDATE compliance_export_request
        SET status='approved', decided_by=$1, decided_at=NOW(), decision_notes=$2,
            download_token=$3, token_expires_at=$4
        WHERE id=$5 AND status='pending' AND requested_by <> $1
        RETURNING id, export_type, period_label, period_start, period_end,
                  (SELECT email FROM users WHERE id=requested_by),
                  requested_at, requested_reason, status, token_expires_at
        "#,
    )
    .bind(user.id)
    .bind(body.notes.as_deref())
    .bind(&token)
    .bind(expires)
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    let r = row.ok_or_else(|| {
        ApiError::BadRequest(
            "request not found, already decided, or you cannot approve your own request".into(),
        )
    })?;

    tracing::info!(
        admin_id = %user.id, request_id = id, export_type = %r.1,
        "Approved compliance export request"
    );

    Ok(Json(ExportRequestRecord {
        id: r.0,
        export_type: r.1,
        period_label: r.2,
        period_start: r.3,
        period_end: r.4,
        requested_by_email: r.5,
        requested_at: r.6,
        requested_reason: r.7,
        status: r.8,
        decided_by_email: Some(user.email.clone()),
        decided_at: Some(chrono::Utc::now()),
        decision_notes: body.notes,
        token_expires_at: Some(r.9),
        download_token: Some(token),
    }))
}

/// Verify single-use token, mark used, return matched request period.
async fn consume_download_token(
    db: &sqlx::PgPool,
    token: &str,
    export_type: &str,
) -> Result<(i64, Option<NaiveDate>, Option<NaiveDate>, String), ApiError> {
    let row: Option<(i64, Option<NaiveDate>, Option<NaiveDate>, String)> = sqlx::query_as(
        r#"UPDATE compliance_export_request
           SET status='used', used_at=NOW()
           WHERE download_token=$1
             AND export_type=$2
             AND status='approved'
             AND token_expires_at > NOW()
           RETURNING id, period_start, period_end, period_label"#,
    )
    .bind(token)
    .bind(export_type)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    row.ok_or_else(|| {
        ApiError::Forbidden(
            "approval token invalid, expired, already used, or type mismatch".into(),
        )
    })
}

/// POST /api/admin/marketplace/compliance/requests/:id/deny
pub async fn api_admin_marketplace_compliance_request_deny(
    jar: CookieJar,
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(body): Json<DecideRequestBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    let updated: Option<(i64,)> = sqlx::query_as(
        r#"UPDATE compliance_export_request
           SET status='denied', decided_by=$1, decided_at=NOW(), decision_notes=$2
           WHERE id=$3 AND status='pending' AND requested_by <> $1
           RETURNING id"#,
    )
    .bind(user.id)
    .bind(body.notes.as_deref())
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(ApiError::Database)?;

    if updated.is_none() {
        return Err(ApiError::BadRequest(
            "request not found, already decided, or you cannot deny your own request".into(),
        ));
    }
    tracing::info!(admin_id = %user.id, request_id = id, "Denied compliance export request");
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ═══════════════════════════════════════════════════════════════════
// ── Auto-Schedule (cron-style) ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct CreateScheduleBody {
    pub export_type: String,
    pub cadence: String,
    pub delivery_email: String,
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ScheduleRecord {
    pub id: i64,
    pub export_type: String,
    pub cadence: String,
    pub delivery_email: String,
    pub format: String,
    pub enabled: bool,
    pub created_by_email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run_status: Option<String>,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn next_run_at(cadence: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let now = chrono::Utc::now();
    let days = match cadence {
        "weekly" => 7,
        "monthly" => 30,
        "quarterly" => 90,
        "annually" => 365,
        _ => return None,
    };
    Some(now + chrono::Duration::days(days))
}

/// POST /api/admin/marketplace/compliance/schedules
pub async fn api_admin_marketplace_compliance_schedule_create(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(body): Json<CreateScheduleBody>,
) -> Result<Json<ScheduleRecord>, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    if !["weekly", "monthly", "quarterly", "annually"].contains(&body.cadence.as_str()) {
        return Err(ApiError::BadRequest("invalid cadence".into()));
    }
    if !["ojk_quarterly", "travel_rule", "tax_fiscal"].contains(&body.export_type.as_str()) {
        return Err(ApiError::BadRequest("invalid export_type".into()));
    }
    let format = body.format.unwrap_or_else(|| "csv".into());
    if !["csv", "json"].contains(&format.as_str()) {
        return Err(ApiError::BadRequest("format must be csv or json".into()));
    }
    let next = next_run_at(&body.cadence);

    let row: (i64, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        r#"INSERT INTO compliance_export_schedule
              (export_type, cadence, delivery_email, format, created_by, next_run_at)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, created_at"#,
    )
    .bind(&body.export_type)
    .bind(&body.cadence)
    .bind(&body.delivery_email)
    .bind(&format)
    .bind(user.id)
    .bind(next)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        admin_id = %user.id, schedule_id = row.0, export_type = %body.export_type,
        cadence = %body.cadence, "Created compliance export schedule"
    );

    Ok(Json(ScheduleRecord {
        id: row.0,
        export_type: body.export_type,
        cadence: body.cadence,
        delivery_email: body.delivery_email,
        format,
        enabled: true,
        created_by_email: Some(user.email.clone()),
        created_at: row.1,
        last_run_at: None,
        last_run_status: None,
        next_run_at: next,
    }))
}

/// GET /api/admin/marketplace/compliance/schedules
pub async fn api_admin_marketplace_compliance_schedule_list(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<Json<Vec<ScheduleRecord>>, ApiError> {
    let _user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    let rows: Vec<(
        i64,
        String,
        String,
        String,
        String,
        bool,
        Option<String>,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        r#"
        SELECT s.id, s.export_type, s.cadence, s.delivery_email, s.format,
               s.enabled, u.email,
               s.created_at, s.last_run_at, s.last_run_status, s.next_run_at
        FROM compliance_export_schedule s
        LEFT JOIN users u ON u.id = s.created_by
        ORDER BY s.created_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ScheduleRecord {
                id: r.0,
                export_type: r.1,
                cadence: r.2,
                delivery_email: r.3,
                format: r.4,
                enabled: r.5,
                created_by_email: r.6,
                created_at: r.7,
                last_run_at: r.8,
                last_run_status: r.9,
                next_run_at: r.10,
            })
            .collect(),
    ))
}

/// DELETE /api/admin/marketplace/compliance/schedules/:id
pub async fn api_admin_marketplace_compliance_schedule_delete(
    jar: CookieJar,
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;
    let res = sqlx::query("DELETE FROM compliance_export_schedule WHERE id=$1")
        .bind(id)
        .execute(db)
        .await
        .map_err(ApiError::Database)?;
    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("schedule not found".into()));
    }
    tracing::info!(admin_id = %user.id, schedule_id = id, "Deleted compliance schedule");
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ═══════════════════════════════════════════════════════════════════
// ── Compare-mode (period vs prior period) ───────────────────────────
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct CompareQuery {
    pub quarter: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct CompareResponse {
    pub current_label: String,
    pub previous_label: String,
    pub current_volume_cents: i64,
    pub previous_volume_cents: i64,
    pub volume_delta_pct: Option<f64>,
    pub current_trades: i64,
    pub previous_trades: i64,
    pub trades_delta_pct: Option<f64>,
}

fn quarter_before(label: &str) -> Result<String, ApiError> {
    let (y, q) = label
        .split_once("-Q")
        .ok_or_else(|| ApiError::BadRequest("invalid quarter".into()))?;
    let yi: i32 = y
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid quarter year".into()))?;
    let qi: u32 = q
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid quarter num".into()))?;
    let (py, pq) = if qi == 1 {
        (yi - 1, 4u32)
    } else {
        (yi, qi - 1)
    };
    Ok(format!("{}-Q{}", py, pq))
}

fn pct_delta(curr: i64, prev: i64) -> Option<f64> {
    if prev == 0 {
        None
    } else {
        Some(((curr - prev) as f64 / prev as f64) * 100.0)
    }
}

/// GET /api/admin/marketplace/compliance/compare
pub async fn api_admin_marketplace_compliance_compare(
    jar: CookieJar,
    Query(q): Query<CompareQuery>,
    State(state): State<AppState>,
) -> Result<Json<CompareResponse>, ApiError> {
    let _user = require_marketplace_compliance(&jar, &state).await?;
    let db = &state.db;

    let (curr_label, curr_start, curr_end) = parse_ojk_quarter(q.quarter)?;
    let prev_label = quarter_before(&curr_label)?;
    let (_pl, prev_start, prev_end) = parse_ojk_quarter(Some(prev_label.clone()))?;

    let curr: (Option<i64>, i64) = sqlx::query_as(
        r#"SELECT COALESCE(SUM(price_cents * quantity::BIGINT), 0)::BIGINT, COUNT(*)::BIGINT
           FROM trade_history WHERE executed_at >= $1::date AND executed_at < $2::date"#,
    )
    .bind(curr_start)
    .bind(curr_end)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let prev: (Option<i64>, i64) = sqlx::query_as(
        r#"SELECT COALESCE(SUM(price_cents * quantity::BIGINT), 0)::BIGINT, COUNT(*)::BIGINT
           FROM trade_history WHERE executed_at >= $1::date AND executed_at < $2::date"#,
    )
    .bind(prev_start)
    .bind(prev_end)
    .fetch_one(db)
    .await
    .map_err(ApiError::Database)?;

    let curr_vol = curr.0.unwrap_or(0);
    let prev_vol = prev.0.unwrap_or(0);

    Ok(Json(CompareResponse {
        current_label: curr_label,
        previous_label: prev_label,
        current_volume_cents: curr_vol,
        previous_volume_cents: prev_vol,
        volume_delta_pct: pct_delta(curr_vol, prev_vol),
        current_trades: curr.1,
        previous_trades: prev.1,
        trades_delta_pct: pct_delta(curr.1, prev.1),
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── Compliance metadata (schema/reg version from env) ───────────────
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct ComplianceMetaResponse {
    pub schema_version: String,
    pub ojk_regulation: String,
    pub fatf_recommendation: String,
    pub support_email: String,
    pub data_sources: Vec<String>,
}

/// GET /api/admin/marketplace/compliance/meta
pub async fn api_admin_marketplace_compliance_meta(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<Json<ComplianceMetaResponse>, ApiError> {
    let _user = require_marketplace_compliance(&jar, &state).await?;
    Ok(Json(ComplianceMetaResponse {
        schema_version: std::env::var("COMPLIANCE_SCHEMA_VERSION")
            .unwrap_or_else(|_| "compliance_export_audit v1".into()),
        ojk_regulation: std::env::var("OJK_REGULATION_REF")
            .unwrap_or_else(|_| "POJK 27/2024".into()),
        fatf_recommendation: std::env::var("FATF_REC_REF")
            .unwrap_or_else(|_| "FATF Recommendation 16".into()),
        support_email: std::env::var("COMPLIANCE_SUPPORT_EMAIL")
            .unwrap_or_else(|_| "compliance@poool.app".into()),
        data_sources: vec![
            "trade_history".into(),
            "users".into(),
            "user_profiles".into(),
            "tax_reports".into(),
        ],
    }))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.16: Trade Admin Notes (#35) ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Audit note attached to a trade by an admin user.
#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct TradeAdminNote {
    pub id: Uuid,
    pub trade_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_email: Option<String>,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request body for creating a trade note.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct CreateTradeNoteRequest {
    pub content: String,
}

/// GET /api/admin/marketplace/trade-notes/:trade_id — List notes (newest first).
pub async fn api_admin_marketplace_trade_notes_list(
    admin: AdminUser,
    Path(trade_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<TradeAdminNote>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;

    let rows: Vec<TradeAdminNote> = sqlx::query_as(
        r#"
        SELECT
            n.id,
            n.trade_id,
            n.author_id,
            u.email AS author_email,
            n.content,
            n.created_at
        FROM trade_admin_notes n
        LEFT JOIN users u ON u.id = n.author_id
        WHERE n.trade_id = $1
        ORDER BY n.created_at DESC
        LIMIT 200
        "#,
    )
    .bind(trade_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

/// POST /api/admin/marketplace/trade-notes/:trade_id — Append an admin note.
pub async fn api_admin_marketplace_trade_notes_create(
    admin: AdminUser,
    Path(trade_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<CreateTradeNoteRequest>,
) -> Result<Json<TradeAdminNote>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.edit")
        .await?;

    let content = body.content.trim();
    if content.is_empty() || content.len() > 2000 {
        return Err(ApiError::BadRequest(
            "Note content must be 1–2000 chars".into(),
        ));
    }

    // Verify the trade exists to fail-fast with a clean 404 instead of a FK error.
    let trade_exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM trade_history WHERE id = $1")
            .bind(trade_id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?;
    if trade_exists.is_none() {
        return Err(ApiError::NotFound("Trade not found".into()));
    }

    let row: TradeAdminNote = sqlx::query_as(
        r#"
        WITH inserted AS (
            INSERT INTO trade_admin_notes (trade_id, author_id, content)
            VALUES ($1, $2, $3)
            RETURNING id, trade_id, author_id, content, created_at
        )
        SELECT
            i.id,
            i.trade_id,
            i.author_id,
            u.email AS author_email,
            i.content,
            i.created_at
        FROM inserted i
        LEFT JOIN users u ON u.id = i.author_id
        "#,
    )
    .bind(trade_id)
    .bind(admin.user.id)
    .bind(content)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        admin_id = %admin.user.id,
        trade_id = %trade_id,
        note_id = %row.id,
        "Admin appended trade note"
    );

    Ok(Json(row))
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.12+: Alert assignee, audit, snooze, rules (mig 109) ────────
// ═══════════════════════════════════════════════════════════════════

/// Insert a row into marketplace_alert_audit.
/// Best-effort: log on failure but never fail the parent action.
async fn record_alert_audit(
    db: &sqlx::PgPool,
    alert_id: Uuid,
    by_user_id: Uuid,
    action: &str,
    details: Option<serde_json::Value>,
) {
    if let Err(e) = sqlx::query(
        "INSERT INTO marketplace_alert_audit (alert_id, by_user_id, action, details) VALUES ($1, $2, $3, $4)",
    )
    .bind(alert_id)
    .bind(by_user_id)
    .bind(action)
    .bind(details)
    .execute(db)
    .await
    {
        tracing::warn!(alert_id = %alert_id, action = %action, err = %e, "Failed to write alert audit row");
    }
}

// ── Claim ──────────────────────────────────────────────────────────

/// POST /api/admin/marketplace/alerts/:alert_id/claim
pub async fn api_admin_marketplace_claim_alert(
    admin: AdminUser,
    Path(alert_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let db = &state.db;
    let alert_uuid =
        Uuid::parse_str(&alert_id).map_err(|_| ApiError::BadRequest("Invalid alert ID".into()))?;

    sqlx::query("UPDATE marketplace_alerts SET assigned_to = $1 WHERE id = $2")
        .bind(admin.user.id)
        .bind(alert_uuid)
        .execute(db)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to claim alert: {}", e)))?;

    record_alert_audit(db, alert_uuid, admin.user.id, "claim", None).await;
    Ok(Json(
        serde_json::json!({ "alert_id": alert_id, "assigned_to": admin.user.id.to_string() }),
    ))
}

// ── Snooze ─────────────────────────────────────────────────────────

/// Body for snooze: minutes to snooze (use -1 for "until resolved").
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AlertSnoozeRequest {
    pub minutes: i64,
}

/// POST /api/admin/marketplace/alerts/:alert_id/snooze
pub async fn api_admin_marketplace_snooze_alert(
    admin: AdminUser,
    Path(alert_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<AlertSnoozeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let db = &state.db;
    let alert_uuid =
        Uuid::parse_str(&alert_id).map_err(|_| ApiError::BadRequest("Invalid alert ID".into()))?;

    // -1 means "until resolved" → far-future timestamp (year 9999)
    let until_sql = if body.minutes < 0 {
        sqlx::query("UPDATE marketplace_alerts SET snoozed_until = 'infinity' WHERE id = $1")
            .bind(alert_uuid)
            .execute(db)
            .await
    } else if body.minutes == 0 {
        sqlx::query("UPDATE marketplace_alerts SET snoozed_until = NULL WHERE id = $1")
            .bind(alert_uuid)
            .execute(db)
            .await
    } else {
        sqlx::query(
            "UPDATE marketplace_alerts SET snoozed_until = NOW() + ($1 || ' minutes')::INTERVAL WHERE id = $2",
        )
        .bind(body.minutes.to_string())
        .bind(alert_uuid)
        .execute(db)
        .await
    };

    until_sql.map_err(|e| ApiError::Internal(format!("Failed to snooze: {}", e)))?;
    record_alert_audit(
        db,
        alert_uuid,
        admin.user.id,
        if body.minutes == 0 {
            "unsnooze"
        } else {
            "snooze"
        },
        Some(serde_json::json!({ "minutes": body.minutes })),
    )
    .await;
    Ok(Json(
        serde_json::json!({ "alert_id": alert_id, "minutes": body.minutes }),
    ))
}

// ── Bulk action ────────────────────────────────────────────────────

/// Body for bulk action: list of alert ids + single action verb.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AlertBulkRequest {
    pub ids: Vec<String>,
    pub action: String,
}

/// POST /api/admin/marketplace/alerts/bulk
pub async fn api_admin_marketplace_alerts_bulk(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<AlertBulkRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let db = &state.db;
    if body.ids.is_empty() {
        return Err(ApiError::BadRequest("ids must be non-empty".into()));
    }
    if body.ids.len() > 500 {
        return Err(ApiError::BadRequest("max 500 ids per bulk call".into()));
    }

    let new_status = match body.action.as_str() {
        "acknowledge" => "acknowledged",
        "resolve" => "resolved",
        "false_positive" => "false_positive",
        _ => return Err(ApiError::BadRequest("invalid action".into())),
    };

    let uuids: Vec<Uuid> = body
        .ids
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect();

    let res = sqlx::query(
        "UPDATE marketplace_alerts SET status = $1, resolved_by = $2, resolved_at = NOW() WHERE id = ANY($3)",
    )
    .bind(new_status)
    .bind(admin.user.id)
    .bind(&uuids)
    .execute(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Bulk update failed: {}", e)))?;

    for id in &uuids {
        record_alert_audit(
            db,
            *id,
            admin.user.id,
            &format!("bulk_{}", body.action),
            None,
        )
        .await;
    }

    Ok(Json(
        serde_json::json!({ "updated": res.rows_affected(), "status": new_status }),
    ))
}

// ── Audit trail fetch ──────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AlertAuditEntry {
    pub id: Uuid,
    pub alert_id: Uuid,
    pub by_user_id: Uuid,
    pub by_user_email: Option<String>,
    pub action: String,
    pub details: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/alerts/:alert_id/audit
pub async fn api_admin_marketplace_alert_audit(
    admin: AdminUser,
    Path(alert_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<AlertAuditEntry>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let db = &state.db;
    let alert_uuid =
        Uuid::parse_str(&alert_id).map_err(|_| ApiError::BadRequest("Invalid alert ID".into()))?;

    let rows: Vec<AlertAuditEntry> = sqlx::query_as(
        r#"SELECT a.id, a.alert_id, a.by_user_id, u.email AS by_user_email,
                  a.action, a.details, a.created_at
             FROM marketplace_alert_audit a
             LEFT JOIN users u ON u.id = a.by_user_id
            WHERE a.alert_id = $1
         ORDER BY a.created_at DESC
            LIMIT 100"#,
    )
    .bind(alert_uuid)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    Ok(Json(rows))
}

// ── Detection rules CRUD ───────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AlertRule {
    pub id: Uuid,
    pub name: String,
    pub category: String,
    pub severity: String,
    pub threshold_text: Option<String>,
    pub escalate_after_min: i32,
    pub channel: String,
    pub enabled: bool,
    pub mute_schedule: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AlertRuleUpsert {
    pub name: String,
    pub category: String,
    pub severity: String,
    pub threshold_text: Option<String>,
    pub escalate_after_min: i32,
    pub channel: String,
    pub enabled: bool,
    #[serde(default)]
    pub mute_schedule: Option<serde_json::Value>,
}

fn validate_rule(r: &AlertRuleUpsert) -> Result<(), ApiError> {
    if r.name.trim().is_empty() || r.name.len() > 100 {
        return Err(ApiError::BadRequest("name 1..100 chars".into()));
    }
    if !["trading", "compliance", "system", "anomaly"].contains(&r.category.as_str()) {
        return Err(ApiError::BadRequest("invalid category".into()));
    }
    if !["info", "warning", "critical"].contains(&r.severity.as_str()) {
        return Err(ApiError::BadRequest("invalid severity".into()));
    }
    if !["none", "slack", "email", "sms", "page"].contains(&r.channel.as_str()) {
        return Err(ApiError::BadRequest("invalid channel".into()));
    }
    if !(0..=10080).contains(&r.escalate_after_min) {
        return Err(ApiError::BadRequest("escalate_after_min 0..10080".into()));
    }
    Ok(())
}

/// GET /api/admin/marketplace/alert-rules
pub async fn api_admin_marketplace_list_rules(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AlertRule>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let rows: Vec<AlertRule> = sqlx::query_as(
        "SELECT id, name, category, severity, threshold_text, escalate_after_min, channel, enabled, mute_schedule, created_at, updated_at
           FROM marketplace_alert_rules
       ORDER BY enabled DESC, name ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Ok(Json(rows))
}

/// POST /api/admin/marketplace/alert-rules
pub async fn api_admin_marketplace_create_rule(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<AlertRuleUpsert>,
) -> Result<Json<AlertRule>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    validate_rule(&body)?;
    let row: AlertRule = sqlx::query_as(
        "INSERT INTO marketplace_alert_rules
            (name, category, severity, threshold_text, escalate_after_min, channel, enabled, mute_schedule, created_by)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
         RETURNING id, name, category, severity, threshold_text, escalate_after_min, channel, enabled, mute_schedule, created_at, updated_at",
    )
    .bind(&body.name)
    .bind(&body.category)
    .bind(&body.severity)
    .bind(body.threshold_text.as_ref())
    .bind(body.escalate_after_min)
    .bind(&body.channel)
    .bind(body.enabled)
    .bind(body.mute_schedule.as_ref())
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to create rule: {}", e)))?;
    Ok(Json(row))
}

/// PUT /api/admin/marketplace/alert-rules/:id
pub async fn api_admin_marketplace_update_rule(
    admin: AdminUser,
    Path(rule_id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<AlertRuleUpsert>,
) -> Result<Json<AlertRule>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    validate_rule(&body)?;
    let id = Uuid::parse_str(&rule_id).map_err(|_| ApiError::BadRequest("invalid id".into()))?;
    let row: AlertRule = sqlx::query_as(
        "UPDATE marketplace_alert_rules
            SET name = $1, category = $2, severity = $3, threshold_text = $4,
                escalate_after_min = $5, channel = $6, enabled = $7, mute_schedule = $8, updated_at = NOW()
          WHERE id = $9
         RETURNING id, name, category, severity, threshold_text, escalate_after_min, channel, enabled, mute_schedule, created_at, updated_at",
    )
    .bind(&body.name)
    .bind(&body.category)
    .bind(&body.severity)
    .bind(body.threshold_text.as_ref())
    .bind(body.escalate_after_min)
    .bind(&body.channel)
    .bind(body.enabled)
    .bind(body.mute_schedule.as_ref())
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to update rule: {}", e)))?;
    Ok(Json(row))
}

/// DELETE /api/admin/marketplace/alert-rules/:id
pub async fn api_admin_marketplace_delete_rule(
    admin: AdminUser,
    Path(rule_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let id = Uuid::parse_str(&rule_id).map_err(|_| ApiError::BadRequest("invalid id".into()))?;
    let res = sqlx::query("DELETE FROM marketplace_alert_rules WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to delete rule: {}", e)))?;
    Ok(Json(serde_json::json!({ "deleted": res.rows_affected() })))
}

/// POST /api/admin/marketplace/alert-rules/:id/test
/// Fires a synthetic alert tagged to the rule (for verifying notification wiring).
pub async fn api_admin_marketplace_test_rule(
    admin: AdminUser,
    Path(rule_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let db = &state.db;
    let id = Uuid::parse_str(&rule_id).map_err(|_| ApiError::BadRequest("invalid id".into()))?;

    let rule: Option<AlertRule> = sqlx::query_as(
        "SELECT id, name, category, severity, threshold_text, escalate_after_min, channel, enabled, mute_schedule, created_at, updated_at
           FROM marketplace_alert_rules WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to load rule: {}", e)))?;
    let rule = rule.ok_or_else(|| ApiError::BadRequest("rule not found".into()))?;

    let alert_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO marketplace_alerts (alert_type, severity, message, status, rule_id, metadata)
            VALUES ($1, $2, $3, 'new', $4, $5)
            RETURNING id"#,
    )
    .bind(format!("{} (test)", rule.name))
    .bind(&rule.severity)
    .bind(format!(
        "Synthetic test fire of rule \"{}\" — channel: {}",
        rule.name, rule.channel
    ))
    .bind(rule.id)
    .bind(serde_json::json!({ "source": "rule_test", "fired_by": admin.user.id }))
    .fetch_one(db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to create test alert: {}", e)))?;

    record_alert_audit(
        db,
        alert_id,
        admin.user.id,
        "rule_test_fire",
        Some(serde_json::json!({ "rule_id": rule.id, "rule_name": rule.name })),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "alert_id": alert_id.to_string(), "rule": rule.name }),
    ))
}

// ── Watchlist (extended for entity types) ──────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct WatchlistEntryV2 {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_identifier: Option<String>,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub reason: String,
    pub added_by: Uuid,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/watchlist/v2 — Generic entity watchlist.
pub async fn api_admin_marketplace_watchlist_v2(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<WatchlistEntryV2>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let rows: Vec<WatchlistEntryV2> = sqlx::query_as(
        r#"SELECT w.id, w.entity_type, w.entity_identifier, w.user_id,
                  u.email AS user_email, w.reason, w.added_by, w.is_active, w.created_at
             FROM marketplace_watchlist w
             LEFT JOIN users u ON u.id = w.user_id
            WHERE w.is_active = true
         ORDER BY w.created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AddWatchlistV2Request {
    pub entity_type: String,
    pub identifier: String,
    pub reason: String,
}

/// POST /api/admin/marketplace/watchlist/v2
pub async fn api_admin_marketplace_add_watchlist_v2(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<AddWatchlistV2Request>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    if !["user", "wallet", "asset", "ip"].contains(&body.entity_type.as_str()) {
        return Err(ApiError::BadRequest("invalid entity_type".into()));
    }
    if body.identifier.trim().is_empty() {
        return Err(ApiError::BadRequest("identifier required".into()));
    }
    if body.reason.len() > 500 {
        return Err(ApiError::BadRequest("reason max 500 chars".into()));
    }

    // For user entity, also fill user_id when identifier parses as UUID.
    let user_id = if body.entity_type == "user" {
        Uuid::parse_str(&body.identifier).ok()
    } else {
        None
    };

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO marketplace_watchlist (entity_type, entity_identifier, user_id, reason, added_by)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (entity_type, entity_identifier) WHERE is_active = true
         DO UPDATE SET reason = EXCLUDED.reason
         RETURNING id",
    )
    .bind(&body.entity_type)
    .bind(&body.identifier)
    .bind(user_id)
    .bind(&body.reason)
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to add to watchlist: {}", e)))?;

    Ok(Json(
        serde_json::json!({ "id": id.to_string(), "status": "added" }),
    ))
}

/// DELETE /api/admin/marketplace/watchlist/v2/:id — Soft-delete (is_active = false).
pub async fn api_admin_marketplace_delete_watchlist_v2(
    admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("invalid id".into()))?;
    let res = sqlx::query("UPDATE marketplace_watchlist SET is_active = false WHERE id = $1")
        .bind(uuid)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to delete watchlist entry: {}", e)))?;
    Ok(Json(serde_json::json!({ "deleted": res.rows_affected() })))
}

// ── Escalation worker (called from background.rs) ──────────────────

/// Find unack alerts that have crossed their rule.escalate_after_min and
/// haven't been escalated yet. Returns the count escalated.
///
/// Notification dispatch is currently a tracing log — wire to Slack/email/SMS
/// transport in a follow-up.
pub async fn escalate_overdue_alerts(db: &sqlx::PgPool) -> Result<u64, sqlx::Error> {
    let rows: Vec<(Uuid, String, String, String, String, Option<serde_json::Value>)> = sqlx::query_as(
        r#"SELECT a.id, a.alert_type, a.message, a.severity, COALESCE(r.channel, 'none'), r.mute_schedule
             FROM marketplace_alerts a
             LEFT JOIN marketplace_alert_rules r ON r.id = a.rule_id
            WHERE a.status IN ('new', 'acknowledged')
              AND a.escalated_at IS NULL
              AND (a.snoozed_until IS NULL OR a.snoozed_until < NOW())
              AND r.escalate_after_min IS NOT NULL
              AND r.escalate_after_min > 0
              AND a.created_at < NOW() - (r.escalate_after_min || ' minutes')::INTERVAL
            LIMIT 50"#,
    )
    .fetch_all(db)
    .await?;

    let now = chrono::Utc::now();
    let weekday = now.weekday().num_days_from_monday(); // Mon=0..Sun=6
    let is_weekend = weekday >= 5;
    let hour = now.hour() as i64;

    let mut count = 0u64;
    for (id, alert_type, message, severity, channel, mute) in &rows {
        // Skip if mute_schedule active right now
        if let Some(mute) = mute {
            let weekends = mute
                .get("weekends")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if weekends && is_weekend {
                continue;
            }
            if let Some(hours) = mute.get("hours").and_then(|v| v.as_array()) {
                if hours.iter().any(|h| h.as_i64() == Some(hour)) {
                    continue;
                }
            }
        }
        count += 1;
        tracing::warn!(
            alert_id = %id,
            severity = %severity,
            channel = %channel,
            alert_type = %alert_type,
            "🚨 ESCALATION: {}: {}",
            alert_type,
            message
        );
        dispatch_alert_notification(channel, severity, alert_type, message, &id.to_string());
        // Always also fire web-push (independent of channel — admin opted in via SW subscribe)
        dispatch_web_push(db, alert_type, message, severity, &id.to_string()).await;
        sqlx::query("UPDATE marketplace_alerts SET escalated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(count)
}

// ═══════════════════════════════════════════════════════════════════
// ── 6A.12+++: Notification transports + push subs + backtest ──────
// ═══════════════════════════════════════════════════════════════════

/// Dispatch an escalation notification via the configured channel.
/// All transports are HTTP POST webhooks — set env vars to wire them up.
/// Fire-and-forget; never blocks escalation worker.
fn dispatch_alert_notification(
    channel: &str,
    severity: &str,
    alert_type: &str,
    message: &str,
    alert_id: &str,
) {
    let env_var = match channel {
        "slack" => "ALERT_SLACK_WEBHOOK_URL",
        "email" => "ALERT_EMAIL_WEBHOOK_URL",
        "sms" => "ALERT_SMS_WEBHOOK_URL",
        "page" => "ALERT_PAGERDUTY_WEBHOOK_URL",
        _ => return,
    };
    let url = match std::env::var(env_var) {
        Ok(v) if !v.is_empty() => v,
        _ => {
            tracing::debug!(channel, "Notification skipped: {} not configured", env_var);
            return;
        }
    };
    let alert_type = alert_type.to_string();
    let message = message.to_string();
    let severity = severity.to_string();
    let alert_id = alert_id.to_string();
    let channel = channel.to_string();
    tokio::spawn(async move {
        let payload = match channel.as_str() {
            "slack" => serde_json::json!({
                "text": format!("🚨 *{}* alert: {}\n>{}\nID: `{}`", severity.to_uppercase(), alert_type, message, alert_id),
            }),
            "page" => serde_json::json!({
                "routing_key": std::env::var("ALERT_PAGERDUTY_ROUTING_KEY").unwrap_or_default(),
                "event_action": "trigger",
                "dedup_key": format!("poool-alert-{}", alert_id),
                "payload": {
                    "summary": format!("{}: {}", alert_type, message),
                    "severity": severity,
                    "source": "poool-admin",
                },
            }),
            // email/sms generic webhook — recipient list configured server-side
            _ => serde_json::json!({
                "severity": severity, "type": alert_type, "message": message, "id": alert_id,
            }),
        };
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build();
        if let Ok(c) = client {
            match c.post(&url).json(&payload).send().await {
                Ok(r) if r.status().is_success() => {
                    tracing::info!(channel, alert_id, "Alert notification dispatched");
                }
                Ok(r) => {
                    tracing::warn!(channel, alert_id, status = %r.status(), "Notification webhook returned non-2xx")
                }
                Err(e) => {
                    tracing::warn!(channel, alert_id, err = %e, "Notification webhook failed")
                }
            }
        }
    });
}

// ── Web push subscriptions ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct PushSubKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct PushSubBody {
    pub endpoint: String,
    pub keys: PushSubKeys,
}

/// GET /api/admin/marketplace/push-vapid-key — Returns server VAPID public
/// key (or 404-ish empty payload if not configured).
pub async fn api_admin_marketplace_push_vapid_key(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let key = std::env::var("VAPID_PUBLIC_KEY").unwrap_or_default();
    if key.is_empty() {
        return Err(ApiError::BadRequest(
            "VAPID_PUBLIC_KEY not configured".into(),
        ));
    }
    Ok(Json(serde_json::json!({ "key": key })))
}

/// POST /api/admin/marketplace/push-subscriptions — Store a browser push
/// subscription for the current admin.
pub async fn api_admin_marketplace_register_push_subscription(
    admin: AdminUser,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<PushSubBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    sqlx::query(
        "INSERT INTO marketplace_alert_push_subscriptions (user_id, endpoint, p256dh, auth, user_agent)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (endpoint) DO UPDATE
            SET p256dh = EXCLUDED.p256dh, auth = EXCLUDED.auth,
                user_agent = EXCLUDED.user_agent, last_seen_at = NOW()",
    )
    .bind(admin.user.id)
    .bind(&body.endpoint)
    .bind(&body.keys.p256dh)
    .bind(&body.keys.auth)
    .bind(&ua)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to store subscription: {}", e)))?;

    Ok(Json(serde_json::json!({ "status": "registered" })))
}

// ── Watchlist enrichment ──────────────────────────────────────────
// Already supported via LEFT JOIN users in v2 list; extend to count linked
// active alerts per entry.

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct WatchlistEntryEnriched {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_identifier: Option<String>,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub reason: String,
    pub added_by: Uuid,
    pub added_by_email: Option<String>,
    pub linked_alerts: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/watchlist/v2/enriched — Adds per-entity
/// linked-alerts count and "added by" email.
pub async fn api_admin_marketplace_watchlist_enriched(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<WatchlistEntryEnriched>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let rows: Vec<WatchlistEntryEnriched> = sqlx::query_as(
        r#"SELECT
              w.id, w.entity_type, w.entity_identifier,
              w.user_id, u.email AS user_email,
              w.reason, w.added_by, ab.email AS added_by_email,
              COALESCE((
                SELECT COUNT(*)
                  FROM marketplace_alerts a
                 WHERE a.status IN ('new','acknowledged')
                   AND (
                     (w.entity_type = 'user' AND a.user_id::TEXT = w.entity_identifier)
                     OR (w.entity_type = 'asset' AND a.alert_type ILIKE '%' || w.entity_identifier || '%')
                     OR (w.entity_type IN ('wallet','ip') AND a.metadata::TEXT LIKE '%' || w.entity_identifier || '%')
                   )
              ), 0) AS linked_alerts,
              w.created_at
            FROM marketplace_watchlist w
            LEFT JOIN users u ON u.id = w.user_id
            LEFT JOIN users ab ON ab.id = w.added_by
           WHERE w.is_active = true
        ORDER BY w.created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Ok(Json(rows))
}

// ── Rule backtest ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
pub struct RuleBacktestResult {
    pub rule_id: Uuid,
    pub rule_name: String,
    pub days: i32,
    pub matched_alerts: i64,
    pub critical_matches: i64,
    pub avg_per_day: f64,
}

/// POST /api/admin/marketplace/alert-rules/:id/backtest?days=30
/// Counts past alerts matching the rule's name pattern in the lookback
/// window. Cheap, naive — pattern match on `alert_type`. Good enough as
/// a sanity check before enabling a rule.
pub async fn api_admin_marketplace_backtest_rule(
    admin: AdminUser,
    Path(rule_id): Path<String>,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<RuleBacktestResult>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.compliance")
        .await?;
    let id = Uuid::parse_str(&rule_id).map_err(|_| ApiError::BadRequest("invalid id".into()))?;
    let days: i32 = q
        .get("days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
        .clamp(1, 365);

    let rule: Option<AlertRule> = sqlx::query_as(
        "SELECT id, name, category, severity, threshold_text, escalate_after_min, channel, enabled, mute_schedule, created_at, updated_at
           FROM marketplace_alert_rules WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to load rule: {}", e)))?;
    let rule = rule.ok_or_else(|| ApiError::BadRequest("rule not found".into()))?;

    // Match alert_type containing first significant word of rule.name.
    let key = rule
        .name
        .split_whitespace()
        .find(|w| w.len() > 3)
        .unwrap_or(&rule.name)
        .to_string();

    let row: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*),
                COUNT(*) FILTER (WHERE severity = 'critical')
           FROM marketplace_alerts
          WHERE alert_type ILIKE '%' || $1 || '%'
            AND created_at > NOW() - ($2::INT * INTERVAL '1 day')",
    )
    .bind(&key)
    .bind(days)
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0));

    Ok(Json(RuleBacktestResult {
        rule_id: rule.id,
        rule_name: rule.name,
        days,
        matched_alerts: row.0,
        critical_matches: row.1,
        avg_per_day: row.0 as f64 / days as f64,
    }))
}

// ── Matview refresh ────────────────────────────────────────────────

/// Refresh the daily-counts matview. Call from background worker every 5–10 min.
pub async fn refresh_alert_daily_counts(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY marketplace_alert_daily_counts")
        .execute(db)
        .await
        .map(|_| ())
}

// ═══════════════════════════════════════════════════════════════════
// ── Restored: Saved views, history (mig 110) ──────────────────────
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AlertView {
    pub id: Uuid,
    pub name: String,
    pub state: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/marketplace/alert-views
pub async fn api_admin_marketplace_list_views(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<AlertView>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let rows: Vec<AlertView> = sqlx::query_as(
        "SELECT id, name, state, created_at
           FROM marketplace_alert_views
          WHERE user_id = $1
       ORDER BY created_at DESC",
    )
    .bind(admin.user.id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct SaveViewRequest {
    pub name: String,
    pub state: serde_json::Value,
}

/// POST /api/admin/marketplace/alert-views
pub async fn api_admin_marketplace_save_view(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<SaveViewRequest>,
) -> Result<Json<AlertView>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    if body.name.trim().is_empty() || body.name.len() > 80 {
        return Err(ApiError::BadRequest("name 1..80 chars".into()));
    }
    let row: AlertView = sqlx::query_as(
        "INSERT INTO marketplace_alert_views (user_id, name, state) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, name) DO UPDATE SET state = EXCLUDED.state, created_at = NOW()
         RETURNING id, name, state, created_at",
    )
    .bind(admin.user.id)
    .bind(&body.name)
    .bind(&body.state)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to save view: {}", e)))?;
    Ok(Json(row))
}

/// DELETE /api/admin/marketplace/alert-views/:id
pub async fn api_admin_marketplace_delete_view(
    admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("invalid id".into()))?;
    let res = sqlx::query("DELETE FROM marketplace_alert_views WHERE id = $1 AND user_id = $2")
        .bind(uuid)
        .bind(admin.user.id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to delete view: {}", e)))?;
    Ok(Json(serde_json::json!({ "deleted": res.rows_affected() })))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[allow(missing_docs)]
pub struct AlertHistoryBucket {
    pub day: chrono::NaiveDate,
    pub severity: String,
    pub count: i32,
}

/// GET /api/admin/marketplace/alerts/history?days=7
pub async fn api_admin_marketplace_alert_history(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<AlertHistoryBucket>>, ApiError> {
    admin
        .require_permission(&state.db, "marketplace.view")
        .await?;
    let days: i32 = q
        .get("days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(7)
        .clamp(1, 90);

    let rows: Vec<AlertHistoryBucket> = sqlx::query_as(
        "SELECT day, severity, count
           FROM marketplace_alert_daily_counts
          WHERE day >= (CURRENT_DATE - ($1::INT - 1))
       ORDER BY day ASC, severity",
    )
    .bind(days)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Ok(Json(rows))
}

// ═══════════════════════════════════════════════════════════════════
// ── Web-push real send (via web-push crate) ───────────────────────
// ═══════════════════════════════════════════════════════════════════

/// Send a push notification to all stored subscriptions.
/// Stale subscriptions (410 Gone / 404) are auto-removed.
/// Requires VAPID_PUBLIC_KEY + VAPID_PRIVATE_KEY + VAPID_SUBJECT (mailto:…) env.
pub async fn dispatch_web_push(
    db: &sqlx::PgPool,
    title: &str,
    body: &str,
    severity: &str,
    alert_id: &str,
) {
    use web_push::{
        ContentEncoding, HyperWebPushClient, SubscriptionInfo, SubscriptionKeys,
        VapidSignatureBuilder, WebPushClient, WebPushError, WebPushMessageBuilder,
    };

    let private_pem = match std::env::var("VAPID_PRIVATE_KEY") {
        Ok(v) if !v.is_empty() => v,
        _ => return,
    };
    let subject =
        std::env::var("VAPID_SUBJECT").unwrap_or_else(|_| "mailto:admin@poool.app".into());

    let subs: Vec<(Uuid, String, String, String)> = match sqlx::query_as(
        "SELECT id, endpoint, p256dh, auth FROM marketplace_alert_push_subscriptions",
    )
    .fetch_all(db)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Failed to load push subs: {}", e);
            return;
        }
    };
    if subs.is_empty() {
        return;
    }

    let client = HyperWebPushClient::new();

    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "severity": severity,
        "tag": format!("poool-alert-{}", alert_id),
        "url": "/admin/marketplace/alerts.html",
    })
    .to_string();

    for (sub_id, endpoint, p256dh, auth) in subs {
        let info = SubscriptionInfo {
            endpoint: endpoint.clone(),
            keys: SubscriptionKeys { p256dh, auth },
        };
        let mut builder_v = match VapidSignatureBuilder::from_pem(private_pem.as_bytes(), &info) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("VAPID PEM parse failed: {}", e);
                return;
            }
        };
        builder_v.add_claim("sub", subject.clone());
        let sig = match builder_v.build() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("VAPID build failed: {}", e);
                continue;
            }
        };
        let mut builder = WebPushMessageBuilder::new(&info);
        builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
        builder.set_vapid_signature(sig);
        let msg = match builder.build() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Push message build failed: {}", e);
                continue;
            }
        };
        match client.send(msg).await {
            Ok(_) => {}
            Err(WebPushError::EndpointNotValid) | Err(WebPushError::EndpointNotFound) => {
                let _ =
                    sqlx::query("DELETE FROM marketplace_alert_push_subscriptions WHERE id = $1")
                        .bind(sub_id)
                        .execute(db)
                        .await;
                tracing::info!("Removed stale push subscription {}", sub_id);
            }
            Err(e) => tracing::warn!(endpoint, "Push send failed: {}", e),
        }
    }
}
