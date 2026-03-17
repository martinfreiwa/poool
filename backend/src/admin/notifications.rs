use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Notifications API
//

/// GET /api/admin/notifications  Recent notifications
pub async fn api_admin_notifications(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query(
        r#"SELECT n.id::text, n.title, n.message, n.type, n.is_read, n.created_at::text,
                  COALESCE(u.email, '') AS user_email,
                  COALESCE(up.first_name, '') AS first_name,
                  COALESCE(up.last_name, '') AS last_name
           FROM notifications n
           JOIN users u ON u.id = n.user_id
           LEFT JOIN user_profiles up ON up.user_id = n.user_id
           ORDER BY n.created_at DESC LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let notifs: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let first: String = r.get("first_name");
            let last: String = r.get("last_name");
            let name = format!("{} {}", first, last).trim().to_string();
            let email: String = r.get("user_email");
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "title": r.get::<String, _>("title"),
                "message": r.get::<Option<String>, _>("message"),
                "type": r.get::<String, _>("type"),
                "is_read": r.get::<bool, _>("is_read"),
                "created_at": r.get::<String, _>("created_at"),
                "user_email": &email,
                "user_name": if name.is_empty() { email.clone() } else { name }
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "notifications": notifs })).into_response())
}

/// POST /api/admin/notifications/broadcast  Send to all users
pub async fn api_admin_notification_broadcast(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let title = body.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let message = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let ntype = body
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("system");

    if title.is_empty() {
        return Err(ApiError::BadRequest("Title is required".to_string()));
    }

    let result = sqlx::query(
        r#"INSERT INTO notifications (user_id, title, message, type)
           SELECT id, $1, $2, $3 FROM users"#,
    )
    .bind(title)
    .bind(message)
    .bind(ntype)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) => Ok(Json(
            serde_json::json!({"status":"broadcast_sent","count": r.rows_affected()}),
        )
        .into_response()),
        Err(e) => {
            tracing::error!("Broadcast failed: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}
