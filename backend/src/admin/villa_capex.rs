//! Admin endpoints to approve/reject developer-submitted CapEx events.
//! Closes the loop on developer/villa_capex.rs (Villa-Returns C3).
//!
//! Approval flips status submitted → approved. 4-eyes enforced
//! (approver_user_id must differ from submitter_user_id, per
//! `vce_approver_differs` CHECK on villa_capex_events).

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CapexRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub event_date: chrono::NaiveDate,
    pub amount_idr_cents: i64,
    pub amount_usd_cents: i64,
    pub category: String,
    pub description: String,
    pub status: String,
    pub submitted_by: Option<Uuid>,
    pub approved_by: Option<Uuid>,
    pub rejected_reason: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RejectInput {
    pub reason: String,
}

/// GET /api/admin/villas/:asset_id/capex?status=submitted — admin-side list.
pub async fn api_admin_villa_capex_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Vec<CapexRow>>, ApiError> {
    let rows: Vec<CapexRow> = sqlx::query_as(
        r#"
        SELECT id, asset_id, event_date, amount_idr_cents, amount_usd_cents,
               category, description, status, submitted_by, approved_by,
               rejected_reason, recorded_at
        FROM villa_capex_events
        WHERE asset_id = $1
        ORDER BY recorded_at DESC, id DESC
        LIMIT 200
        "#,
    )
    .bind(asset_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// PUT /api/admin/villas/:asset_id/capex/:capex_id/approve
pub async fn api_admin_villa_capex_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, capex_id)): Path<(Uuid, i64)>,
) -> Result<Json<CapexRow>, ApiError> {
    let existing: Option<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT asset_id, status, submitted_by FROM villa_capex_events WHERE id = $1",
    )
    .bind(capex_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let (existing_asset, existing_status, submitted_by) =
        existing.ok_or_else(|| ApiError::NotFound("CapEx not found".to_string()))?;
    if existing_asset != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing_status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot approve CapEx in status='{}'",
            existing_status
        )));
    }
    if submitted_by == Some(admin.user.id) {
        return Err(ApiError::Conflict(
            "You submitted this CapEx — another admin must approve".to_string(),
        ));
    }

    let row: CapexRow = sqlx::query_as(
        r#"
        UPDATE villa_capex_events
           SET status = 'approved', approved_by = $2, approved_at = NOW()
         WHERE id = $1 AND status = 'submitted'
         RETURNING id, asset_id, event_date, amount_idr_cents, amount_usd_cents,
                   category, description, status, submitted_by, approved_by,
                   rejected_reason, recorded_at
        "#,
    )
    .bind(capex_id)
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_capex.approve', 'villa_capex_events', NULL, $2)
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
        .bind("CapEx event approved")
        .bind(format!(
            "Admin approved your CapEx event: {} ({} IDR cents).",
            row.description, row.amount_idr_cents
        ))
        .bind(format!(
            "/developer/villas/{}/annual/{}",
            row.asset_id,
            row.event_date.format("%Y")
        ))
        .execute(&state.db)
        .await;
    }

    Ok(Json(row))
}

/// PUT /api/admin/villas/:asset_id/capex/:capex_id/reject
pub async fn api_admin_villa_capex_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, capex_id)): Path<(Uuid, i64)>,
    Json(input): Json<RejectInput>,
) -> Result<Json<CapexRow>, ApiError> {
    if input.reason.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Rejection reason required".to_string(),
        ));
    }

    let row: CapexRow = sqlx::query_as(
        r#"
        UPDATE villa_capex_events
           SET status = 'rejected', rejected_reason = $3, rejected_at = NOW()
         WHERE id = $1 AND asset_id = $2 AND status = 'submitted'
         RETURNING id, asset_id, event_date, amount_idr_cents, amount_usd_cents,
                   category, description, status, submitted_by, approved_by,
                   rejected_reason, recorded_at
        "#,
    )
    .bind(capex_id)
    .bind(asset_id)
    .bind(input.reason.clone())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::Conflict("CapEx not found or not in 'submitted' state".to_string()))?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_capex.reject', 'villa_capex_events', NULL, $2)
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
        .bind("CapEx event rejected")
        .bind(format!(
            "Admin rejected your CapEx event ({}). Reason: {}",
            row.description, input.reason
        ))
        .bind(format!(
            "/developer/villas/{}/annual/{}",
            row.asset_id,
            row.event_date.format("%Y")
        ))
        .execute(&state.db)
        .await;
    }

    Ok(Json(row))
}
