//! Public performance KPIs for a villa (Villa-Returns P3 first slice).
//!
//! Investor + public-facing read. Returns the KPIs from PDF §6 that can be
//! computed without a daily NAV snapshot job:
//!   - Latest NAV per token (PDF §7 formula, computed live from latest valuation)
//!   - Latest published period + distributable
//!   - Last 12 months distributable (IDR + USD)
//!   - Annual yield % (12m distributable / pool value, bps)
//!   - Months with published data
//!
//! Chart series (NAV history, monthly yield series) and 5-Year Total Return are
//! P3.1+: they need either a daily snapshot job (P4) or a heavier on-the-fly
//! computation across the valuation history.

use crate::admin::extractors::ApiError;
use crate::auth::routes::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
pub struct PerformanceQuery {
    /// Time-travel cutoff. If set, returns state as published at this moment;
    /// otherwise defaults to "now" (latest published rows). Drives PDF §7
    /// NAV and §6 KPIs from `villa_operations_log` / `villa_valuations`
    /// filtered by `recorded_at <= as_of`.
    pub as_of: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct VillaPerformance {
    pub asset_id: Uuid,
    pub valuation_idr_cents: i64,
    pub valuation_usd_cents: i64,
    pub valuation_date: Option<chrono::NaiveDate>,
    pub tokenized_pct_bps: i32,
    pub tokens_in_pool: i64,
    pub nav_token_idr_cents: i64,
    pub nav_token_usd_cents: i64,
    pub latest_period_year: Option<i32>,
    pub latest_period_month: Option<i32>,
    pub latest_distributable_idr_cents: i64,
    pub latest_distributable_usd_cents: i64,
    pub last_12m_distributable_idr_cents: i64,
    pub last_12m_distributable_usd_cents: i64,
    pub annual_yield_bps: i32,
    pub months_with_data: i64,
    pub payout_currency: String,
    /// Projected annualised return = forecast yield + forecast appreciation.
    /// `None` if no forecast data + no historical yield to extrapolate.
    pub projected_annual_net_return_bps: Option<i32>,
    /// 5-year compound return using `projected_annual_net_return_bps`.
    /// Computed as ((1 + r)^5 − 1) in bps, integer arithmetic.
    pub five_year_total_return_bps: Option<i32>,
    /// Forecast year that was used to compute the projections, if any.
    pub forecast_source_year: Option<i32>,
    /// PDF §6 — Share Price Performance over +3M / +6M / +12M.
    /// Computed as `(nav_today − nav_then) × 10000 / nav_then` from
    /// `villa_market_prices_daily`. Uses closest snapshot ≤ target date.
    /// `None` if no snapshot available in the lookback window.
    pub share_price_3m_bps: Option<i32>,
    pub share_price_6m_bps: Option<i32>,
    pub share_price_12m_bps: Option<i32>,
}

/// GET /api/villas/:asset_id/performance?as_of=YYYY-MM-DDTHH:MM:SSZ — public investor KPI bundle.
///
/// When `as_of` is provided, returns the data as published at that moment.
/// Implementation note: villa_operations_current is mutable per-period, so for
/// `as_of` reads we go to villa_operations_log directly with the standard
/// (period_year DESC, period_month DESC, recorded_at DESC, id DESC) ordering.
pub async fn api_villa_performance(
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Query(q): Query<PerformanceQuery>,
) -> Result<Json<VillaPerformance>, ApiError> {
    // Villa-Returns B4 — feature-flag gate.
    // 'off'    → legacy: return empty/zero values (no Villa-Returns data).
    // 'shadow' → read both new + legacy, log divergence, return new.
    // 'on'     → read new (default behavior).
    let flag: String = sqlx::query_scalar(
        "SELECT COALESCE(value, 'on') FROM platform_settings WHERE key='villa_returns.enabled'",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .unwrap_or_else(|| "on".to_string());

    // C2 per-asset pilot gate. When platform flag is 'on', an asset must also
    // be flagged `villa_returns_pilot=TRUE` to be served from the new layer.
    let asset_pilot: bool =
        sqlx::query_scalar("SELECT COALESCE(villa_returns_pilot, FALSE) FROM assets WHERE id = $1")
            .bind(asset_id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?
            .unwrap_or(false);

    let effective_off = flag == "off" || (flag == "on" && !asset_pilot);

    if effective_off {
        return Ok(Json(VillaPerformance {
            asset_id,
            valuation_idr_cents: 0,
            valuation_usd_cents: 0,
            valuation_date: None,
            tokenized_pct_bps: 0,
            tokens_in_pool: 0,
            nav_token_idr_cents: 0,
            nav_token_usd_cents: 0,
            latest_period_year: None,
            latest_period_month: None,
            latest_distributable_idr_cents: 0,
            latest_distributable_usd_cents: 0,
            last_12m_distributable_idr_cents: 0,
            last_12m_distributable_usd_cents: 0,
            annual_yield_bps: 0,
            months_with_data: 0,
            payout_currency: "USD".to_string(),
            projected_annual_net_return_bps: None,
            five_year_total_return_bps: None,
            forecast_source_year: None,
            share_price_3m_bps: None,
            share_price_6m_bps: None,
            share_price_12m_bps: None,
        }));
    }

    if flag == "shadow" {
        tracing::info!(
            "villa_performance shadow mode for asset {}: reading new layer, divergence not yet computed",
            asset_id
        );
        // Continue to the normal read path; divergence logging hooks would go here.
    }
    let cfg: (Option<i32>, Option<i64>, Option<i32>, Option<String>) = sqlx::query_as(
        r#"
        SELECT
            tokenized_pct_bps,
            tokens_total::BIGINT,
            tokens_owner_retained,
            COALESCE(payout_currency, 'USD')
        FROM assets WHERE id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    let tokenized_pct_bps = cfg.0.unwrap_or(0);
    let tokens_total = cfg.1.unwrap_or(0);
    let tokens_owner_retained = cfg.2.unwrap_or(0);
    let payout_currency = cfg.3.unwrap_or_else(|| "USD".to_string());
    let tokens_in_pool = tokens_total - i64::from(tokens_owner_retained);

    // Latest published valuation, time-travel-aware.
    // When as_of is set we include 'superseded' rows because a row that is
    // superseded today may have been the latest 'published' at as_of.
    let val: Option<(i64, i64, chrono::NaiveDate)> = sqlx::query_as(
        r#"
        SELECT valuation_idr_cents, valuation_usd_cents, valuation_date
        FROM villa_valuations
        WHERE asset_id = $1
          AND status IN ('published','superseded')
          AND ($2::TIMESTAMPTZ IS NULL OR recorded_at <= $2)
        ORDER BY valuation_date DESC, recorded_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(asset_id)
    .bind(q.as_of)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let (valuation_idr_cents, valuation_usd_cents, valuation_date) =
        val.map(|v| (v.0, v.1, Some(v.2))).unwrap_or((0, 0, None));

    let pool_value_idr: i128 = (valuation_idr_cents as i128) * tokenized_pct_bps as i128 / 10_000;
    let nav_token_idr_cents: i64 = if tokens_in_pool > 0 && pool_value_idr > 0 {
        (pool_value_idr / tokens_in_pool as i128) as i64
    } else {
        0
    };
    let pool_value_usd: i128 = (valuation_usd_cents as i128) * tokenized_pct_bps as i128 / 10_000;
    let nav_token_usd_cents: i64 = if tokens_in_pool > 0 && pool_value_usd > 0 {
        (pool_value_usd / tokens_in_pool as i128) as i64
    } else {
        0
    };

    // Latest period + 12-month rollup, time-travel-aware. For as_of reads we
    // query villa_operations_log (with status IN published/superseded) using
    // DISTINCT ON to pick the latest recorded row per (year, month) that was
    // visible at as_of. For NOW reads we use villa_operations_current (fast path).
    let (
        latest_year,
        latest_month,
        latest_idr,
        latest_usd,
        last_12m_idr,
        last_12m_usd,
        months_with_data,
    ) = if let Some(as_of) = q.as_of {
        let rows: Vec<(i32, i32, i64, i64)> = sqlx::query_as(
            r#"
            SELECT DISTINCT ON (period_year, period_month)
                   period_year, period_month,
                   distributable_idr_cents, distributable_usd_cents
            FROM villa_operations_log
            WHERE asset_id = $1
              AND status IN ('published','superseded')
              AND recorded_at <= $2
            ORDER BY period_year DESC, period_month DESC, recorded_at DESC, id DESC
            LIMIT 12
            "#,
        )
        .bind(asset_id)
        .bind(as_of)
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::Database)?;

        let latest = rows.first();
        let (ly, lm, li, lu) = latest
            .map(|r| (Some(r.0), Some(r.1), r.2, r.3))
            .unwrap_or((None, None, 0, 0));
        let mut sum_idr: i64 = 0;
        let mut sum_usd: i64 = 0;
        for r in &rows {
            sum_idr = sum_idr.saturating_add(r.2);
            sum_usd = sum_usd.saturating_add(r.3);
        }
        (ly, lm, li, lu, sum_idr, sum_usd, rows.len() as i64)
    } else {
        let latest: Option<(i32, i32, i64, i64)> = sqlx::query_as(
            r#"
            SELECT period_year, period_month, distributable_idr_cents, distributable_usd_cents
            FROM villa_operations_current
            WHERE asset_id = $1
            ORDER BY period_year DESC, period_month DESC
            LIMIT 1
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?;
        let (ly, lm, li, lu) = latest
            .map(|r| (Some(r.0), Some(r.1), r.2, r.3))
            .unwrap_or((None, None, 0, 0));
        let totals: (Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(distributable_idr_cents), 0)::BIGINT,
                COALESCE(SUM(distributable_usd_cents), 0)::BIGINT,
                COUNT(*)::BIGINT
            FROM (
                SELECT distributable_idr_cents, distributable_usd_cents
                FROM villa_operations_current
                WHERE asset_id = $1
                ORDER BY period_year DESC, period_month DESC
                LIMIT 12
            ) sub
            "#,
        )
        .bind(asset_id)
        .fetch_one(&state.db)
        .await
        .map_err(ApiError::Database)?;
        (
            ly,
            lm,
            li,
            lu,
            totals.0.unwrap_or(0),
            totals.1.unwrap_or(0),
            totals.2.unwrap_or(0),
        )
    };

    let annual_yield_bps: i32 = if pool_value_idr > 0 && last_12m_idr > 0 {
        ((last_12m_idr as i128 * 10_000) / pool_value_idr) as i32
    } else {
        0
    };

    // ── Forecast-derived projections (PDF §6) ────────────────────
    // Latest forecast row drives both projected_annual_net_return_bps and 5Y.
    // If forecast yield is missing, fall back to current annual_yield_bps so
    // the user still sees a meaningful projection.
    let forecast: Option<(i32, Option<i32>, Option<i32>)> = sqlx::query_as(
        r#"
        SELECT forecast_year,
               projected_annual_net_yield_bps,
               projected_appreciation_bps
        FROM villa_forecast_assumptions
        WHERE asset_id = $1
        ORDER BY forecast_year DESC
        LIMIT 1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let (projected_annual_net_return_bps, five_year_total_return_bps, forecast_source_year) =
        match forecast {
            None => (None, None, None),
            Some((year, fcst_yield, fcst_appreciation)) => {
                let yield_bps = fcst_yield.unwrap_or(annual_yield_bps);
                let appreciation_bps = fcst_appreciation.unwrap_or(0);
                let per_year_bps = yield_bps + appreciation_bps;
                // Compound 5 years: ((1 + r)^5 − 1) in bps. r = per_year_bps / 10000.
                // Use i128 so the compounding doesn't overflow.
                let factor: i128 = 10_000 + per_year_bps as i128;
                let f2 = factor * factor / 10_000;
                let f3 = f2 * factor / 10_000;
                let f4 = f3 * factor / 10_000;
                let f5 = f4 * factor / 10_000;
                let five_y_bps: i32 = (f5 - 10_000) as i32;
                (Some(per_year_bps), Some(five_y_bps), Some(year))
            }
        };

    // Share-price-performance deltas from villa_market_prices_daily (PDF §6).
    let share_price_3m_bps = compute_share_price_delta(&state.db, asset_id, 90).await;
    let share_price_6m_bps = compute_share_price_delta(&state.db, asset_id, 180).await;
    let share_price_12m_bps = compute_share_price_delta(&state.db, asset_id, 365).await;

    Ok(Json(VillaPerformance {
        asset_id,
        valuation_idr_cents,
        valuation_usd_cents,
        valuation_date,
        tokenized_pct_bps,
        tokens_in_pool,
        nav_token_idr_cents,
        nav_token_usd_cents,
        latest_period_year: latest_year,
        latest_period_month: latest_month,
        latest_distributable_idr_cents: latest_idr,
        latest_distributable_usd_cents: latest_usd,
        last_12m_distributable_idr_cents: last_12m_idr,
        last_12m_distributable_usd_cents: last_12m_usd,
        annual_yield_bps,
        months_with_data,
        payout_currency,
        projected_annual_net_return_bps,
        five_year_total_return_bps,
        forecast_source_year,
        share_price_3m_bps,
        share_price_6m_bps,
        share_price_12m_bps,
    }))
}

/// Returns `(nav_today − nav_then) × 10000 / nav_then` in bps, using the closest
/// snapshot on or before `today − days`. `None` if either snapshot is missing.
async fn compute_share_price_delta(pool: &PgPool, asset_id: Uuid, days: i64) -> Option<i32> {
    let target_date = chrono::Utc::now().date_naive() - chrono::Duration::days(days);

    let (today_nav,): (i64,) = sqlx::query_as(
        r#"
        SELECT nav_token_idr_cents
        FROM villa_market_prices_daily
        WHERE asset_id = $1
        ORDER BY snapshot_date DESC
        LIMIT 1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    let (then_nav,): (i64,) = sqlx::query_as(
        r#"
        SELECT nav_token_idr_cents
        FROM villa_market_prices_daily
        WHERE asset_id = $1
          AND snapshot_date <= $2
        ORDER BY snapshot_date DESC
        LIMIT 1
        "#,
    )
    .bind(asset_id)
    .bind(target_date)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    if then_nav <= 0 {
        return None;
    }
    Some((((today_nav - then_nav) as i128 * 10_000) / then_nav as i128) as i32)
}

// ─── NAV history series ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub metric: Option<String>,
    pub from: Option<chrono::NaiveDate>,
    pub to: Option<chrono::NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct HistoryPoint {
    pub date: chrono::NaiveDate,
    pub value_idr_cents: i64,
    pub value_usd_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct HistorySeries {
    pub metric: String,
    pub points: Vec<HistoryPoint>,
}

/// GET /api/villas/:asset_id/history?metric=nav&from=&to=
///
/// Returns the time series for a metric. Currently supported: `metric=nav`.
/// PDF §8 — NAV and Market series stay separate; this endpoint serves NAV only.
/// Implementation: each published valuation yields one data point computed via
/// the PDF §7 formula. Between valuations NAV is constant (step function on
/// the frontend chart).
pub async fn api_villa_history(
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<HistoryQuery>,
) -> Result<Json<HistorySeries>, ApiError> {
    let metric = q.metric.unwrap_or_else(|| "nav".to_string());
    if !matches!(metric.as_str(), "nav" | "market") {
        return Err(ApiError::BadRequest(format!(
            "Unsupported metric '{}' — supported: nav, market",
            metric
        )));
    }

    // 1. Daily snapshots — densest source. Pull whatever's there for the range.
    let column = if metric == "market" {
        ("market_token_idr_cents", "market_token_usd_cents")
    } else {
        ("nav_token_idr_cents", "nav_token_usd_cents")
    };
    let snapshot_query = format!(
        r#"
        SELECT snapshot_date,
               {idr_col} AS value_idr_cents,
               {usd_col} AS value_usd_cents
        FROM villa_market_prices_daily
        WHERE asset_id = $1
          AND {idr_col} IS NOT NULL
          AND ($2::DATE IS NULL OR snapshot_date >= $2)
          AND ($3::DATE IS NULL OR snapshot_date <= $3)
        ORDER BY snapshot_date ASC
        "#,
        idr_col = column.0,
        usd_col = column.1,
    );
    let snapshots: Vec<(chrono::NaiveDate, Option<i64>, Option<i64>)> =
        sqlx::query_as(&snapshot_query)
            .bind(asset_id)
            .bind(q.from)
            .bind(q.to)
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?;

    // 2. NAV fallback: if no snapshots for NAV, derive from valuations history.
    //    For Market: no fallback — empty list means no observed trades.
    let mut points: Vec<HistoryPoint> = snapshots
        .into_iter()
        .filter_map(|(d, i, u)| match i {
            Some(idr) => Some(HistoryPoint {
                date: d,
                value_idr_cents: idr,
                value_usd_cents: u.unwrap_or(0),
            }),
            None => None,
        })
        .collect();

    if metric == "nav" && points.is_empty() {
        let cfg: (Option<i32>, Option<i64>, Option<i32>) = sqlx::query_as(
            r#"
            SELECT tokenized_pct_bps, tokens_total::BIGINT, tokens_owner_retained
            FROM assets WHERE id = $1
            "#,
        )
        .bind(asset_id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;
        let tokenized_pct_bps = cfg.0.unwrap_or(0);
        let tokens_in_pool: i64 = cfg.1.unwrap_or(0) - i64::from(cfg.2.unwrap_or(0));

        let valuations: Vec<(chrono::NaiveDate, i64, i64)> = sqlx::query_as(
            r#"
            SELECT valuation_date, valuation_idr_cents, valuation_usd_cents
            FROM villa_valuations
            WHERE asset_id = $1
              AND status IN ('published','superseded')
              AND ($2::DATE IS NULL OR valuation_date >= $2)
              AND ($3::DATE IS NULL OR valuation_date <= $3)
            ORDER BY valuation_date ASC, recorded_at ASC, id ASC
            "#,
        )
        .bind(asset_id)
        .bind(q.from)
        .bind(q.to)
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::Database)?;

        points = valuations
            .into_iter()
            .map(|(date, idr_total, usd_total)| {
                let nav_idr = if tokens_in_pool > 0 && tokenized_pct_bps > 0 {
                    let pool = (idr_total as i128) * tokenized_pct_bps as i128 / 10_000;
                    (pool / tokens_in_pool as i128) as i64
                } else {
                    0
                };
                let nav_usd = if tokens_in_pool > 0 && tokenized_pct_bps > 0 {
                    let pool = (usd_total as i128) * tokenized_pct_bps as i128 / 10_000;
                    (pool / tokens_in_pool as i128) as i64
                } else {
                    0
                };
                HistoryPoint {
                    date,
                    value_idr_cents: nav_idr,
                    value_usd_cents: nav_usd,
                }
            })
            .collect();
    }

    Ok(Json(HistorySeries { metric, points }))
}

// `pool` parameter type re-exposed so callers can pre-warm KPIs without going through
// the HTTP layer (P3.1 — used by the property page handler if we move to SSR injection).
#[allow(dead_code)]
async fn _typecheck_signature(_p: &PgPool) {}
