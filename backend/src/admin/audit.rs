use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Audit Logs API
//

/// GET /api/admin/audit-logs  Recent audit log entries
pub async fn api_admin_audit_logs(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "audit.read").await?;

    let rows = sqlx::query(
        r#"SELECT al.id, al.action, al.entity_type, al.entity_id::text,
                  al.previous_state, al.new_state,
                  al.ip_address::text, al.user_agent, al.created_at::text,
                  COALESCE(u.email, 'system') AS actor_email
           FROM audit_logs al
           LEFT JOIN users u ON u.id = al.actor_user_id
           ORDER BY al.created_at DESC
           LIMIT 500"#,
    )
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
