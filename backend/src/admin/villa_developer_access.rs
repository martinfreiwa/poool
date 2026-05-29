//! Admin endpoints to manage developer access to villas (Villa-Returns P2 / W1).
//!
//! Backs the "Developer access" section on admin/asset-details.html.
//! Append-only: revocations set `effective_until`, never DELETE.

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DeveloperAccessRow {
    pub id: i64,
    pub developer_user_id: Uuid,
    pub developer_email: Option<String>,
    pub asset_id: Uuid,
    pub effective_from: chrono::DateTime<chrono::Utc>,
    pub effective_until: Option<chrono::DateTime<chrono::Utc>>,
    pub granted_by: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GrantInput {
    pub developer_user_id: Uuid,
    pub notes: Option<String>,
}

/// GET /api/admin/villas/:asset_id/developer-access
pub async fn api_admin_developer_access_list(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<Vec<DeveloperAccessRow>>, ApiError> {
    admin
        .require_permission(&state.db, "villa.developer_access.view")
        .await?;
    let rows: Vec<DeveloperAccessRow> = sqlx::query_as(
        r#"
        SELECT
            dal.id,
            dal.developer_user_id,
            u.email AS developer_email,
            dal.asset_id,
            dal.effective_from,
            dal.effective_until,
            dal.granted_by,
            dal.notes
        FROM developer_asset_links dal
        LEFT JOIN users u ON u.id = dal.developer_user_id
        WHERE dal.asset_id = $1
        ORDER BY dal.effective_from DESC
        "#,
    )
    .bind(asset_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

/// POST /api/admin/villas/:asset_id/developer-access
pub async fn api_admin_developer_access_grant(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<GrantInput>,
) -> Result<Json<DeveloperAccessRow>, ApiError> {
    admin
        .require_permission(&state.db, "villa.developer_access.manage")
        .await?;
    let row: DeveloperAccessRow = sqlx::query_as(
        r#"
        INSERT INTO developer_asset_links (developer_user_id, asset_id, granted_by, notes)
        VALUES ($1, $2, $3, $4)
        RETURNING
            id, developer_user_id,
            (SELECT email FROM users WHERE id = $1) AS developer_email,
            asset_id, effective_from, effective_until, granted_by, notes
        "#,
    )
    .bind(input.developer_user_id)
    .bind(asset_id)
    .bind(admin.user.id)
    .bind(input.notes)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.constraint() == Some("uq_dal_active_link") => {
            ApiError::Conflict("This developer is already linked to this asset.".to_string())
        }
        _ => ApiError::Database(e),
    })?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'developer_access.grant', 'developer_asset_links', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await;

    Ok(Json(row))
}

/// DELETE /api/admin/villas/:asset_id/developer-access/:link_id  — revokes by setting effective_until.
pub async fn api_admin_developer_access_revoke(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, link_id)): Path<(Uuid, i64)>,
) -> Result<Json<DeveloperAccessRow>, ApiError> {
    admin
        .require_permission(&state.db, "villa.developer_access.manage")
        .await?;
    let row: DeveloperAccessRow = sqlx::query_as(
        r#"
        UPDATE developer_asset_links SET
            effective_until = NOW(),
            revoked_by      = $3,
            revoked_at      = NOW()
        WHERE id = $1 AND asset_id = $2 AND effective_until IS NULL
        RETURNING
            id, developer_user_id,
            (SELECT email FROM users WHERE id = developer_asset_links.developer_user_id) AS developer_email,
            asset_id, effective_from, effective_until, granted_by, notes
        "#,
    )
    .bind(link_id)
    .bind(asset_id)
    .bind(admin.user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Active link not found".to_string()))?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'developer_access.revoke', 'developer_asset_links', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await;

    Ok(Json(row))
}
