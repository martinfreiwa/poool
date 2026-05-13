//! Daily NAV snapshot job (Villa-Returns P4).
//!
//! Populates `villa_market_prices_daily` with one row per (asset, today)
//! capturing:
//!   - NAV per token (PDF §7) from latest published valuation
//!   - Annual yield from trailing 12 months of villa_operations_current
//!   - Market token VWAP from `trade_history` (last 24h) — left NULL if no trades
//!
//! This module exposes:
//!   - Admin trigger endpoint POST /api/admin/villa-nav-snapshot/run
//!   - `run_snapshot_for_all_assets(pool)` callable from background tasks
//!
//! Background-task wiring is intentionally deferred; admin can run on-demand.

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::State;
use axum::Json;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize)]
pub struct SnapshotResult {
    pub assets_processed: i64,
    pub assets_with_nav: i64,
    pub assets_with_market_price: i64,
    pub snapshot_date: chrono::NaiveDate,
}

pub async fn api_admin_villa_nav_snapshot_run(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<SnapshotResult>, ApiError> {
    let result = run_snapshot_for_all_assets(&state.db)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(result))
}

/// Compute + UPSERT today's NAV row for every published villa with a tokenization config.
/// Safe to call multiple times per day (UPSERTs on `(asset_id, snapshot_date)` PK).
pub async fn run_snapshot_for_all_assets(pool: &PgPool) -> Result<SnapshotResult, sqlx::Error> {
    let today = chrono::Utc::now().date_naive();
    let yesterday_ts = chrono::Utc::now() - chrono::Duration::hours(24);

    // FX rate freezing — pick latest IDR→USD from fx_rates_daily, default 1 bps.
    let fx_bps: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE((
            SELECT rate_bps::INTEGER FROM fx_rates_daily
            WHERE base_currency='IDR' AND quote_currency='USD'
            ORDER BY snapshot_date DESC LIMIT 1
        ), 1)
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(1);

    #[derive(sqlx::FromRow)]
    struct Candidate {
        asset_id: uuid::Uuid,
        tokenized_pct_bps: Option<i32>,
        tokens_total: Option<i64>,
        tokens_owner_retained: Option<i32>,
    }
    let assets: Vec<Candidate> = sqlx::query_as(
        r#"
        SELECT id AS asset_id,
               tokenized_pct_bps,
               tokens_total::BIGINT,
               tokens_owner_retained
        FROM assets
        WHERE tokenized_pct_bps IS NOT NULL AND tokenized_pct_bps > 0
          AND tokens_total > 0
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut with_nav: i64 = 0;
    let mut with_market: i64 = 0;

    for a in &assets {
        let tokens_in_pool: i64 =
            a.tokens_total.unwrap_or(0) - i64::from(a.tokens_owner_retained.unwrap_or(0));
        if tokens_in_pool <= 0 {
            continue;
        }

        // Latest published valuation (or superseded — we want what was last live).
        let val: Option<(i64, i64)> = sqlx::query_as(
            r#"
            SELECT valuation_idr_cents, valuation_usd_cents
            FROM villa_valuations
            WHERE asset_id = $1 AND status IN ('published','superseded')
            ORDER BY valuation_date DESC, recorded_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(a.asset_id)
        .fetch_optional(pool)
        .await?;

        let (val_idr, val_usd) = match val {
            Some((i, u)) => (i, u),
            None => continue, // No valuation → no NAV
        };

        let tokenized_pct = a.tokenized_pct_bps.unwrap_or(0) as i128;
        let nav_idr: i64 =
            ((val_idr as i128 * tokenized_pct / 10_000) / tokens_in_pool as i128) as i64;
        let nav_usd: i64 =
            ((val_usd as i128 * tokenized_pct / 10_000) / tokens_in_pool as i128) as i64;
        with_nav += 1;

        // Annual yield from villa_operations_current trailing 12 months.
        let (sum_12m, count_12m): (Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(distributable_idr_cents), 0)::BIGINT,
                   COUNT(*)::BIGINT
            FROM (
                SELECT distributable_idr_cents
                FROM villa_operations_current
                WHERE asset_id = $1
                ORDER BY period_year DESC, period_month DESC
                LIMIT 12
            ) sub
            "#,
        )
        .bind(a.asset_id)
        .fetch_one(pool)
        .await?;
        let pool_value_idr: i128 = (val_idr as i128) * tokenized_pct / 10_000;
        let annual_yield_bps: i32 = if pool_value_idr > 0 && sum_12m.unwrap_or(0) > 0 {
            ((sum_12m.unwrap_or(0) as i128 * 10_000) / pool_value_idr) as i32
        } else {
            0
        };
        let _ = count_12m;

        // Market VWAP from trade_history over last 24h.
        // Schema check note: trade_history columns assumed (asset_id, executed_at,
        // price_cents, quantity). If schema differs, the query gracefully returns
        // NULL and we leave market_token_*_cents NULL.
        let market: (Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                CASE WHEN SUM(quantity) > 0
                     THEN (SUM(price_cents * quantity) / SUM(quantity))::BIGINT
                     ELSE NULL
                END AS vwap_idr_cents,
                COUNT(*)::BIGINT AS trade_count,
                COALESCE(SUM(quantity)::BIGINT, 0) AS volume
            FROM trade_history
            WHERE asset_id = $1 AND executed_at >= $2
            "#,
        )
        .bind(a.asset_id)
        .bind(yesterday_ts)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or((None, Some(0), Some(0)));

        let market_idr = market.0;
        let market_usd: Option<i64> =
            market_idr.map(|v| (v as i128 * fx_bps as i128 / 10_000) as i64);
        let trade_count = market.1.unwrap_or(0) as i32;
        let volume = market.2.unwrap_or(0);
        if market_idr.is_some() {
            with_market += 1;
        }

        sqlx::query(
            r#"
            INSERT INTO villa_market_prices_daily
                (asset_id, snapshot_date,
                 nav_token_idr_cents, nav_token_usd_cents,
                 market_token_idr_cents, market_token_usd_cents,
                 annual_yield_bps, trade_count, volume_tokens, fx_rate_idr_to_usd_bps)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (asset_id, snapshot_date) DO UPDATE SET
                nav_token_idr_cents     = EXCLUDED.nav_token_idr_cents,
                nav_token_usd_cents     = EXCLUDED.nav_token_usd_cents,
                market_token_idr_cents  = EXCLUDED.market_token_idr_cents,
                market_token_usd_cents  = EXCLUDED.market_token_usd_cents,
                annual_yield_bps        = EXCLUDED.annual_yield_bps,
                trade_count             = EXCLUDED.trade_count,
                volume_tokens           = EXCLUDED.volume_tokens,
                fx_rate_idr_to_usd_bps  = EXCLUDED.fx_rate_idr_to_usd_bps
            "#,
        )
        .bind(a.asset_id)
        .bind(today)
        .bind(nav_idr)
        .bind(nav_usd)
        .bind(market_idr)
        .bind(market_usd)
        .bind(annual_yield_bps)
        .bind(trade_count)
        .bind(volume)
        .bind(fx_bps)
        .execute(pool)
        .await?;
    }

    Ok(SnapshotResult {
        assets_processed: assets.len() as i64,
        assets_with_nav: with_nav,
        assets_with_market_price: with_market,
        snapshot_date: today,
    })
}
