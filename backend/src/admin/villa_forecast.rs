//! Admin endpoints to accept/discard developer forecast suggestions.
//! Accept upserts the suggestion's values into `villa_forecast_assumptions`
//! and marks the suggestion as `accepted` (audit trail). Discard simply marks
//! the suggestion as `discarded` with optional outcome notes.

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    .ok_or_else(|| ApiError::Conflict(
        "Suggestion not found or not in 'submitted' state".to_string(),
    ))?;

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
    .ok_or_else(|| ApiError::Conflict(
        "Suggestion not found or not in 'submitted' state".to_string(),
    ))?;

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
        .bind(format!("Forecast suggestion discarded — {}", row.forecast_year))
        .bind(input.outcome_notes.clone().unwrap_or_else(|| "Admin discarded the suggestion. See dashboard for details.".to_string()))
        .bind(format!("/developer/villas/{}/annual/{}", row.asset_id, row.forecast_year))
        .execute(&state.db)
        .await;
    }

    Ok(Json(row))
}
