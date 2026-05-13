//! Developer endpoints for forecast suggestions (Villa-Returns C3 / W10).
//!
//! Developer suggests forecast values; admin reviews + accepts/discards into
//! `villa_forecast_assumptions` via a separate admin endpoint (not in this slice).

use crate::admin::extractors::ApiError;
use crate::auth::routes::AppState;
use crate::developer::extractors::DeveloperUser;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ForecastSuggestionInput {
    pub projected_occupancy_bps: Option<i32>,
    pub projected_adr_idr_cents: Option<i64>,
    pub projected_rent_growth_bps: Option<i32>,
    pub projected_expense_inflation_bps: Option<i32>,
    pub projected_appreciation_bps: Option<i32>,
    pub projected_exit_yield_bps: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ForecastSuggestionRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub forecast_year: i32,
    pub projected_occupancy_bps: Option<i32>,
    pub projected_adr_idr_cents: Option<i64>,
    pub projected_rent_growth_bps: Option<i32>,
    pub projected_expense_inflation_bps: Option<i32>,
    pub projected_appreciation_bps: Option<i32>,
    pub projected_exit_yield_bps: Option<i32>,
    pub notes: Option<String>,
    pub status: String,
    pub submitted_by: Option<Uuid>,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub processed_by: Option<Uuid>,
    pub processed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub processed_outcome_notes: Option<String>,
}

/// POST /api/developer/villas/:asset_id/forecast/:year/suggest
pub async fn api_developer_forecast_suggest(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, forecast_year)): Path<(Uuid, i32)>,
    Json(input): Json<ForecastSuggestionInput>,
) -> Result<Json<ForecastSuggestionRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    if !(2000..=2100).contains(&forecast_year) {
        return Err(ApiError::BadRequest("Invalid forecast_year".to_string()));
    }

    let row: ForecastSuggestionRow = sqlx::query_as(
        r#"
        INSERT INTO villa_forecast_suggestions (
            asset_id, forecast_year,
            projected_occupancy_bps, projected_adr_idr_cents,
            projected_rent_growth_bps, projected_expense_inflation_bps,
            projected_appreciation_bps, projected_exit_yield_bps,
            notes, status, submitted_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'submitted', $10)
        RETURNING *
        "#,
    )
    .bind(asset_id)
    .bind(forecast_year)
    .bind(input.projected_occupancy_bps)
    .bind(input.projected_adr_idr_cents)
    .bind(input.projected_rent_growth_bps)
    .bind(input.projected_expense_inflation_bps)
    .bind(input.projected_appreciation_bps)
    .bind(input.projected_exit_yield_bps)
    .bind(input.notes)
    .bind(dev.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(row))
}

/// GET /api/developer/villas/:asset_id/forecast/:year/suggestions
pub async fn api_developer_forecast_suggestions_list(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, forecast_year)): Path<(Uuid, i32)>,
) -> Result<Json<Vec<ForecastSuggestionRow>>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let rows: Vec<ForecastSuggestionRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_forecast_suggestions
        WHERE asset_id = $1 AND forecast_year = $2
        ORDER BY submitted_at DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .bind(forecast_year)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// GET /api/developer/villas/:asset_id/annual/:year/summary
///
/// Annual rollup the developer sees on the C3 page: total distributable for
/// the year from villa_operations_current (latest published only) + count of
/// approved CapEx events.
#[derive(Debug, Serialize)]
pub struct AnnualSummary {
    pub forecast_year: i32,
    pub months_published: i64,
    pub total_distributable_idr_cents: i64,
    pub total_distributable_usd_cents: i64,
    pub total_net_rental_idr_cents: i64,
    pub approved_capex_count: i64,
    pub approved_capex_idr_cents: i64,
}

pub async fn api_developer_annual_summary(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, forecast_year)): Path<(Uuid, i32)>,
) -> Result<Json<AnnualSummary>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let ops: (Option<i64>, Option<i64>, Option<i64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*)::BIGINT,
            COALESCE(SUM(distributable_idr_cents), 0)::BIGINT,
            COALESCE(SUM(distributable_usd_cents), 0)::BIGINT,
            COALESCE(SUM(net_rental_income_idr_cents), 0)::BIGINT
        FROM villa_operations_current
        WHERE asset_id = $1 AND period_year = $2
        "#,
    )
    .bind(asset_id)
    .bind(forecast_year)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let capex: (Option<i64>, Option<i64>) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::BIGINT, COALESCE(SUM(amount_idr_cents), 0)::BIGINT
        FROM villa_capex_events
        WHERE asset_id = $1
          AND EXTRACT(YEAR FROM event_date)::INT = $2
          AND status = 'approved'
        "#,
    )
    .bind(asset_id)
    .bind(forecast_year)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(AnnualSummary {
        forecast_year,
        months_published: ops.0.unwrap_or(0),
        total_distributable_idr_cents: ops.1.unwrap_or(0),
        total_distributable_usd_cents: ops.2.unwrap_or(0),
        total_net_rental_idr_cents: ops.3.unwrap_or(0),
        approved_capex_count: capex.0.unwrap_or(0),
        approved_capex_idr_cents: capex.1.unwrap_or(0),
    }))
}
