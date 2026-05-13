//! Admin endpoints for per-asset deduction policy (Villa-Returns B3, PDF §4).
//!
//! `villa_deduction_policy` is append-only (migration 138 guard). Policy changes
//! are new rows with later `effective_from`; the active policy at any moment T
//! is the row with the largest `effective_from <= T`. Past published rows are
//! unaffected by later policy edits.

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeductionPolicyRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub effective_from: chrono::NaiveDate,
    pub allowed_codes: Vec<String>,
    pub per_category_cap_bps: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub set_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct DeductionPolicyInput {
    pub effective_from: chrono::NaiveDate,
    pub allowed_codes: Vec<String>,
    pub per_category_cap_bps: Option<serde_json::Value>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExpenseCategoryRow {
    pub code: String,
    pub label: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub sort_order: i32,
}

/// GET /api/villa-expense-categories — public catalog (no auth required; used by both admin + dev forms).
pub async fn api_villa_expense_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<ExpenseCategoryRow>>, ApiError> {
    let rows: Vec<ExpenseCategoryRow> = sqlx::query_as(
        r#"
        SELECT code, label, description, is_default, sort_order
        FROM villa_expense_categories
        ORDER BY sort_order, code
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// GET /api/admin/villas/:asset_id/deduction-policies — full history, newest first.
pub async fn api_admin_deduction_policies_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Vec<DeductionPolicyRow>>, ApiError> {
    let rows: Vec<DeductionPolicyRow> = sqlx::query_as(
        r#"
        SELECT id, asset_id, effective_from, allowed_codes,
               per_category_cap_bps, notes, set_by, created_at
        FROM villa_deduction_policy
        WHERE asset_id = $1
        ORDER BY effective_from DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// POST /api/admin/villas/:asset_id/deduction-policies — append a new policy row.
pub async fn api_admin_deduction_policy_create(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<DeductionPolicyInput>,
) -> Result<Json<DeductionPolicyRow>, ApiError> {
    if input.allowed_codes.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one allowed category required".to_string(),
        ));
    }

    // Validate every code is known (avoids typos that would silently invalidate
    // future operations submissions against this policy).
    let known: Vec<String> = sqlx::query_scalar("SELECT code FROM villa_expense_categories")
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::Database)?;
    let known_set: std::collections::HashSet<&str> = known.iter().map(|s| s.as_str()).collect();
    for code in &input.allowed_codes {
        if !known_set.contains(code.as_str()) {
            return Err(ApiError::BadRequest(format!(
                "Unknown expense category '{}'",
                code
            )));
        }
    }

    let row: DeductionPolicyRow = sqlx::query_as(
        r#"
        INSERT INTO villa_deduction_policy
            (asset_id, effective_from, allowed_codes, per_category_cap_bps, notes, set_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, asset_id, effective_from, allowed_codes,
                  per_category_cap_bps, notes, set_by, created_at
        "#,
    )
    .bind(asset_id)
    .bind(input.effective_from)
    .bind(&input.allowed_codes)
    .bind(input.per_category_cap_bps)
    .bind(input.notes)
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db)
            if db.constraint() == Some("villa_deduction_policy_asset_id_effective_from_key") =>
        {
            ApiError::Conflict(
                "A policy already exists for this asset on this effective_from date".to_string(),
            )
        }
        _ => ApiError::Database(e),
    })?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_deduction_policy.create', 'villa_deduction_policy', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await;

    Ok(Json(row))
}
