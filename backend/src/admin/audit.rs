use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::Row;

//
//  Admin Audit Logs API
//

/// Query filters for the audit-log listing.
#[derive(Debug, Deserialize, Default)]
#[allow(missing_docs)]
pub struct AuditLogFilters {
    pub action: Option<String>,
    pub entity_type: Option<String>,
    /// Exact-match filter on entity_id (e.g. an asset UUID).
    pub entity_id: Option<String>,
    /// Action LIKE prefix (e.g. `marketplace.`).
    pub action_prefix: Option<String>,
    pub per_page: Option<i64>,
}

/// GET /api/admin/audit-logs — recent audit log entries (optionally filtered).
///
/// Query params:
/// - `action`: exact-match filter on the audit action (e.g. `marketplace.orderbook.rebuilt`).
/// - `entity_type`: exact-match filter on entity_type.
/// - `per_page`: cap on rows returned (1..=500, default 500).
pub async fn api_admin_audit_logs(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(filters): Query<AuditLogFilters>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "audit.read").await?;

    let limit = filters.per_page.unwrap_or(500).clamp(1, 500);

    // `action_prefix` (e.g. `marketplace.`) enables fetching multi-action sets
    // for inline viewers without round-tripping per action.
    let action_prefix = filters.action_prefix.as_deref();
    let rows = sqlx::query(
        r#"SELECT al.id, al.action, al.entity_type, al.entity_id::text,
                  al.previous_state, al.new_state,
                  al.ip_address::text, al.user_agent, al.created_at::text,
                  COALESCE(u.email, 'system') AS actor_email
           FROM audit_logs al
           LEFT JOIN users u ON u.id = al.actor_user_id
           WHERE ($1::text IS NULL OR al.action = $1)
             AND ($2::text IS NULL OR al.entity_type = $2)
             AND ($3::text IS NULL OR al.action LIKE $3 || '%')
             AND ($4::text IS NULL OR al.entity_id::text = $4)
           ORDER BY al.created_at DESC
           LIMIT $5"#,
    )
    .bind(filters.action.as_deref())
    .bind(filters.entity_type.as_deref())
    .bind(action_prefix)
    .bind(filters.entity_id.as_deref())
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let logs: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "action": r.get::<String, _>("action"),
                "entity_type": r.get::<String, _>("entity_type"),
                "entity_id": r.get::<Option<String>, _>("entity_id"),
                "previous_state": r.get::<Option<serde_json::Value>, _>("previous_state"),
                "new_state": r.get::<Option<serde_json::Value>, _>("new_state"),
                "ip_address": r.get::<Option<String>, _>("ip_address"),
                "user_agent": r.get::<Option<String>, _>("user_agent"),
                "created_at": r.get::<String, _>("created_at"),
                "actor_email": r.get::<String, _>("actor_email")
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "logs": logs })).into_response())
}
