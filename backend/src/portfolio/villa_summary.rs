//! Villa-Returns portfolio summary endpoint (P3 — `poool_app_home.html` cards).
//!
//! Aggregates lifetime distributions, active positions, and value snapshot for
//! the authenticated investor across all their `investments` rows. Used by
//! the multi-asset dashboard. Per-position breakdown is a separate slice.

use crate::admin::extractors::ApiError;
use crate::auth::middleware;
use crate::auth::routes::AppState;
use axum::extract::State;
use axum::Json;
use axum_extra::extract::CookieJar;
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PortfolioVillaSummary {
    pub active_position_count: i64,
    pub total_tokens_owned: i64,
    pub total_invested_cents: i64,
    pub current_value_cents: i64,
    /// Realised gain/loss = current_value − invested. Negative if positions are down.
    pub unrealised_pnl_cents: i64,
    /// Lifetime dividends received (sum of `dividend_payouts.amount_cents` where status='paid').
    /// Always in USD cents — wallet credits go to the cash USD wallet.
    pub lifetime_dividends_usd_cents: i64,
    pub lifetime_dividend_count: i64,
    /// Mirror sum from investments.total_rental_cents — should match lifetime_dividends
    /// once the dividend_payouts → wallet bridge has processed everything.
    pub lifetime_rental_cents: i64,
}

/// GET /api/investors/me/portfolio-villa-summary
pub async fn api_portfolio_villa_summary(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<Json<PortfolioVillaSummary>, ApiError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    let summary: PortfolioVillaSummary = sqlx::query_as(
        r#"
        WITH agg AS (
            SELECT
                COALESCE(COUNT(*)::BIGINT, 0)                       AS active_position_count,
                COALESCE(SUM(tokens_owned)::BIGINT, 0)              AS total_tokens_owned,
                COALESCE(SUM(purchase_value_cents)::BIGINT, 0)      AS total_invested_cents,
                COALESCE(SUM(current_value_cents)::BIGINT, 0)       AS current_value_cents,
                COALESCE(SUM(total_rental_cents)::BIGINT, 0)        AS lifetime_rental_cents
            FROM investments
            WHERE user_id = $1
              AND tokens_owned > 0
              AND status <> 'exited'
        ),
        div AS (
            SELECT
                COALESCE(SUM(amount_cents)::BIGINT, 0) AS lifetime_dividends_usd_cents,
                COALESCE(COUNT(*)::BIGINT, 0)          AS lifetime_dividend_count
            FROM dividend_payouts
            WHERE user_id = $1 AND status = 'paid'
        )
        SELECT
            agg.active_position_count,
            agg.total_tokens_owned,
            agg.total_invested_cents,
            agg.current_value_cents,
            (agg.current_value_cents - agg.total_invested_cents) AS unrealised_pnl_cents,
            div.lifetime_dividends_usd_cents,
            div.lifetime_dividend_count,
            agg.lifetime_rental_cents
        FROM agg, div
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(summary))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PositionNav {
    pub asset_id: uuid::Uuid,
    pub nav_token_usd_cents: Option<i64>,
    pub market_token_usd_cents: Option<i64>,
}

/// GET /api/investors/me/positions-nav — latest NAV/Market price per asset the user holds.
/// Joins user's `investments` rows to the most-recent `villa_market_prices_daily` snapshot
/// per asset. Returns one entry per asset; consumers merge with `/api/portfolio` client-side.
pub async fn api_positions_nav(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<Json<Vec<PositionNav>>, ApiError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    let rows: Vec<PositionNav> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (i.asset_id)
            i.asset_id,
            m.nav_token_usd_cents,
            m.market_token_usd_cents
        FROM investments i
        LEFT JOIN villa_market_prices_daily m
            ON m.asset_id = i.asset_id
        WHERE i.user_id = $1
          AND i.tokens_owned > 0
          AND i.status <> 'exited'
        ORDER BY i.asset_id, m.snapshot_date DESC NULLS LAST
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}
