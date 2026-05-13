//! Developer endpoints for villa CapEx events (Villa-Returns C3 / PDF §3, §9).
//!
//! Q8 lock-in: CapEx never reduces monthly distributable — captured here for
//! admin review and consumed in the next valuation. Developer submits;
//! admin approves via the existing approvals/CapEx flow (not in this slice).

use crate::admin::extractors::ApiError;
use crate::auth::routes::AppState;
use crate::developer::extractors::DeveloperUser;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CapexInput {
    pub event_date: chrono::NaiveDate,
    pub amount_idr_cents: i64,
    pub category: String,
    pub description: String,
    pub evidence_doc_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CapexRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub event_date: chrono::NaiveDate,
    pub amount_idr_cents: i64,
    pub amount_usd_cents: i64,
    pub currency_code: String,
    pub category: String,
    pub description: String,
    pub evidence_doc_id: Option<Uuid>,
    pub status: String,
    pub submitted_by: Option<Uuid>,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rejected_reason: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CapexQuery {
    pub year: Option<i32>,
}

/// POST /api/developer/villas/:asset_id/capex — submit a new CapEx event.
/// Created with status='submitted' directly (no draft state for developer-submitted
/// CapEx — keep the dev workflow short; admin reviews and approves/rejects).
pub async fn api_developer_villa_capex_create(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<CapexInput>,
) -> Result<Json<CapexRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    if input.amount_idr_cents <= 0 {
        return Err(ApiError::BadRequest("Amount must be > 0".to_string()));
    }
    if input.description.trim().is_empty() {
        return Err(ApiError::BadRequest("Description required".to_string()));
    }

    let row: CapexRow = sqlx::query_as(
        r#"
        INSERT INTO villa_capex_events
            (asset_id, event_date, amount_idr_cents, category,
             description, evidence_doc_id, status, submitted_by, submitted_at)
        VALUES ($1, $2, $3, $4, $5, $6, 'submitted', $7, NOW())
        RETURNING *
        "#,
    )
    .bind(asset_id)
    .bind(input.event_date)
    .bind(input.amount_idr_cents)
    .bind(input.category)
    .bind(input.description)
    .bind(input.evidence_doc_id)
    .bind(dev.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(row))
}

/// GET /api/developer/villas/:asset_id/capex?year=YYYY — list CapEx events.
pub async fn api_developer_villa_capex_list(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Query(q): Query<CapexQuery>,
) -> Result<Json<Vec<CapexRow>>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let rows: Vec<CapexRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_capex_events
        WHERE asset_id = $1
          AND ($2::INT IS NULL OR EXTRACT(YEAR FROM event_date)::INT = $2)
        ORDER BY event_date DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .bind(q.year)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}
