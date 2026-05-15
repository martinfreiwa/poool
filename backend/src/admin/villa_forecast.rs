//! Admin endpoints to accept/discard developer forecast suggestions, and to
//! directly read/write `villa_forecast_assumptions` without needing a suggestion.

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Forecast assumptions (admin-direct edit) ──────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ForecastAssumptionRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub forecast_year: i32,
    pub projected_occupancy_bps: Option<i32>,
    pub projected_adr_idr_cents: Option<i64>,
    pub projected_rent_growth_bps: Option<i32>,
    pub projected_expense_inflation_bps: Option<i32>,
    pub projected_appreciation_bps: Option<i32>,
    pub projected_exit_yield_bps: Option<i32>,
    pub projected_annual_net_yield_bps: Option<i32>,
    pub notes: Option<String>,
    pub finalized_by: Option<Uuid>,
    pub finalized_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ForecastAssumptionInput {
    pub forecast_year: i32,
    pub projected_occupancy_bps: Option<i32>,
    pub projected_adr_idr_cents: Option<i64>,
    pub projected_rent_growth_bps: Option<i32>,
    pub projected_expense_inflation_bps: Option<i32>,
    pub projected_appreciation_bps: Option<i32>,
    pub projected_exit_yield_bps: Option<i32>,
    pub projected_annual_net_yield_bps: Option<i32>,
    pub notes: Option<String>,
}

/// GET /api/admin/villas/:asset_id/forecast-assumptions
/// Returns all finalized assumption rows for this asset, newest year first.
pub async fn api_admin_forecast_assumptions_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Vec<ForecastAssumptionRow>>, ApiError> {
    let rows: Vec<ForecastAssumptionRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_forecast_assumptions
        WHERE asset_id = $1
        ORDER BY forecast_year ASC
        "#,
    )
    .bind(asset_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// PUT /api/admin/villas/:asset_id/forecast-assumptions
/// Upserts one year of assumptions directly (no suggestion required).
/// Overwrites every field that is non-null in the request body.
pub async fn api_admin_forecast_assumption_upsert(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<ForecastAssumptionInput>,
) -> Result<Json<ForecastAssumptionRow>, ApiError> {
    if !(2000..=2100).contains(&input.forecast_year) {
        return Err(ApiError::BadRequest("forecast_year must be 2000–2100".into()));
    }

    let row: ForecastAssumptionRow = sqlx::query_as(
        r#"
        INSERT INTO villa_forecast_assumptions (
            asset_id, forecast_year,
            projected_occupancy_bps, projected_adr_idr_cents,
            projected_rent_growth_bps, projected_expense_inflation_bps,
            projected_appreciation_bps, projected_exit_yield_bps,
            projected_annual_net_yield_bps, notes,
            finalized_by, finalized_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
        ON CONFLICT (asset_id, forecast_year) DO UPDATE
        SET projected_occupancy_bps         = COALESCE($3,  villa_forecast_assumptions.projected_occupancy_bps),
            projected_adr_idr_cents         = COALESCE($4,  villa_forecast_assumptions.projected_adr_idr_cents),
            projected_rent_growth_bps       = COALESCE($5,  villa_forecast_assumptions.projected_rent_growth_bps),
            projected_expense_inflation_bps = COALESCE($6,  villa_forecast_assumptions.projected_expense_inflation_bps),
            projected_appreciation_bps      = COALESCE($7,  villa_forecast_assumptions.projected_appreciation_bps),
            projected_exit_yield_bps        = COALESCE($8,  villa_forecast_assumptions.projected_exit_yield_bps),
            projected_annual_net_yield_bps  = COALESCE($9,  villa_forecast_assumptions.projected_annual_net_yield_bps),
            notes                           = COALESCE($10, villa_forecast_assumptions.notes),
            finalized_by                    = $11,
            finalized_at                    = NOW(),
            updated_at                      = NOW()
        RETURNING *
        "#,
    )
    .bind(asset_id)
    .bind(input.forecast_year)
    .bind(input.projected_occupancy_bps)
    .bind(input.projected_adr_idr_cents)
    .bind(input.projected_rent_growth_bps)
    .bind(input.projected_expense_inflation_bps)
    .bind(input.projected_appreciation_bps)
    .bind(input.projected_exit_yield_bps)
    .bind(input.projected_annual_net_yield_bps)
    .bind(input.notes.clone())
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_forecast.upsert_assumptions', 'villa_forecast_assumptions', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({ "asset_id": asset_id, "forecast_year": input.forecast_year }))
    .execute(&state.db)
    .await;

    Ok(Json(row))
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

#[derive(Debug, Deserialize, Default)]
pub struct ProcessInput {
    pub outcome_notes: Option<String>,
}

/// GET /api/admin/villas/:asset_id/forecast-suggestions
pub async fn api_admin_forecast_suggestions_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Vec<ForecastSuggestionRow>>, ApiError> {
    let rows: Vec<ForecastSuggestionRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_forecast_suggestions
        WHERE asset_id = $1
        ORDER BY submitted_at DESC, id DESC
        LIMIT 200
        "#,
    )
    .bind(asset_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// PUT /api/admin/villas/:asset_id/forecast-suggestions/:id/accept
/// Merges the suggestion into `villa_forecast_assumptions` (UPSERT on
/// `(asset_id, forecast_year)`) and marks suggestion as accepted.
pub async fn api_admin_forecast_suggestion_accept(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, id)): Path<(Uuid, i64)>,
    Json(input): Json<ProcessInput>,
) -> Result<Json<ForecastSuggestionRow>, ApiError> {
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let row: ForecastSuggestionRow = sqlx::query_as(
        r#"
        UPDATE villa_forecast_suggestions
           SET status = 'accepted',
               processed_by = $2,
               processed_at = NOW(),
               processed_outcome_notes = $3
         WHERE id = $1 AND asset_id = $4 AND status = 'submitted'
         RETURNING *
        "#,
    )
    .bind(id)
    .bind(admin.user.id)
    .bind(input.outcome_notes.clone())
    .bind(asset_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| {
        ApiError::Conflict("Suggestion not found or not in 'submitted' state".to_string())
    })?;

    // UPSERT into villa_forecast_assumptions, picking only fields that the
    // suggestion provided (preserve any prior non-null values otherwise).
    sqlx::query(
        r#"
        INSERT INTO villa_forecast_assumptions (
            asset_id, forecast_year,
            projected_occupancy_bps, projected_adr_idr_cents,
            projected_rent_growth_bps, projected_expense_inflation_bps,
            projected_appreciation_bps, projected_exit_yield_bps,
            notes, finalized_by, finalized_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
        ON CONFLICT (asset_id, forecast_year) DO UPDATE
        SET projected_occupancy_bps         = COALESCE(EXCLUDED.projected_occupancy_bps,         villa_forecast_assumptions.projected_occupancy_bps),
            projected_adr_idr_cents         = COALESCE(EXCLUDED.projected_adr_idr_cents,         villa_forecast_assumptions.projected_adr_idr_cents),
            projected_rent_growth_bps       = COALESCE(EXCLUDED.projected_rent_growth_bps,       villa_forecast_assumptions.projected_rent_growth_bps),
            projected_expense_inflation_bps = COALESCE(EXCLUDED.projected_expense_inflation_bps, villa_forecast_assumptions.projected_expense_inflation_bps),
            projected_appreciation_bps      = COALESCE(EXCLUDED.projected_appreciation_bps,      villa_forecast_assumptions.projected_appreciation_bps),
            projected_exit_yield_bps        = COALESCE(EXCLUDED.projected_exit_yield_bps,        villa_forecast_assumptions.projected_exit_yield_bps),
            notes                           = COALESCE(EXCLUDED.notes,                           villa_forecast_assumptions.notes),
            finalized_by                    = $10,
            finalized_at                    = NOW(),
            updated_at                      = NOW()
        "#,
    )
    .bind(asset_id)
    .bind(row.forecast_year)
    .bind(row.projected_occupancy_bps)
    .bind(row.projected_adr_idr_cents)
    .bind(row.projected_rent_growth_bps)
    .bind(row.projected_expense_inflation_bps)
    .bind(row.projected_appreciation_bps)
    .bind(row.projected_exit_yield_bps)
    .bind(row.notes.clone())
    .bind(admin.user.id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_forecast.accept', 'villa_forecast_suggestions', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null))
    .execute(&mut *tx)
    .await;

    if let Some(submitter_id) = row.submitted_by {
        let _ = sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, type, action_url)
            VALUES ($1, $2, $3, 'system', $4)
            "#,
        )
        .bind(submitter_id)
        .bind(format!("Forecast suggestion accepted — {}", row.forecast_year))
        .bind("Admin accepted your forecast suggestion. Values merged into villa_forecast_assumptions.".to_string())
        .bind(format!("/developer/villas/{}/annual/{}", row.asset_id, row.forecast_year))
        .execute(&state.db)
        .await;
    }

    tx.commit().await.map_err(ApiError::Database)?;
    Ok(Json(row))
}

/// PUT /api/admin/villas/:asset_id/forecast-suggestions/:id/discard
pub async fn api_admin_forecast_suggestion_discard(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, id)): Path<(Uuid, i64)>,
    Json(input): Json<ProcessInput>,
) -> Result<Json<ForecastSuggestionRow>, ApiError> {
    let row: ForecastSuggestionRow = sqlx::query_as(
        r#"
        UPDATE villa_forecast_suggestions
           SET status = 'discarded',
               processed_by = $2,
               processed_at = NOW(),
               processed_outcome_notes = $3
         WHERE id = $1 AND asset_id = $4 AND status = 'submitted'
         RETURNING *
        "#,
    )
    .bind(id)
    .bind(admin.user.id)
    .bind(input.outcome_notes.clone())
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| {
        ApiError::Conflict("Suggestion not found or not in 'submitted' state".to_string())
    })?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_forecast.discard', 'villa_forecast_suggestions', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await;

    if let Some(submitter_id) = row.submitted_by {
        let _ = sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, type, action_url)
            VALUES ($1, $2, $3, 'system', $4)
            "#,
        )
        .bind(submitter_id)
        .bind(format!(
            "Forecast suggestion discarded — {}",
            row.forecast_year
        ))
        .bind(input.outcome_notes.clone().unwrap_or_else(|| {
            "Admin discarded the suggestion. See dashboard for details.".to_string()
        }))
        .bind(format!(
            "/developer/villas/{}/annual/{}",
            row.asset_id, row.forecast_year
        ))
        .execute(&state.db)
        .await;
    }

    Ok(Json(row))
}
