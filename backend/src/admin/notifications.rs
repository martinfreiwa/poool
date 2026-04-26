use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

const MAX_TITLE_LEN: usize = 255;
const MAX_MESSAGE_LEN: usize = 2_000;

//
//  Admin Notifications API
//

/// Request body for broadcasting an admin notification to platform users.
#[derive(Debug, Deserialize)]
pub struct BroadcastNotificationRequest {
    #[serde(rename = "type")]
    notification_type: String,
    title: String,
    message: String,
}

impl BroadcastNotificationRequest {
    fn validate(self) -> Result<ValidatedBroadcastNotification, ApiError> {
        let notification_type = self.notification_type.trim().to_string();
        let title = self.title.trim().to_string();
        let message = self.message.trim().to_string();

        if !matches!(
            notification_type.as_str(),
            "kyc" | "investment" | "payout" | "system" | "promo"
        ) {
            return Err(ApiError::BadRequest(
                "Notification type is invalid.".to_string(),
            ));
        }

        if title.is_empty() {
            return Err(ApiError::BadRequest("Title is required.".to_string()));
        }

        if title.chars().count() > MAX_TITLE_LEN {
            return Err(ApiError::BadRequest(format!(
                "Title must be {} characters or fewer.",
                MAX_TITLE_LEN
            )));
        }

        if message.is_empty() {
            return Err(ApiError::BadRequest("Message is required.".to_string()));
        }

        if message.chars().count() > MAX_MESSAGE_LEN {
            return Err(ApiError::BadRequest(format!(
                "Message must be {} characters or fewer.",
                MAX_MESSAGE_LEN
            )));
        }

        Ok(ValidatedBroadcastNotification {
            notification_type,
            title,
            message,
        })
    }
}

struct ValidatedBroadcastNotification {
    notification_type: String,
    title: String,
    message: String,
}

/// GET /api/admin/notifications  Recent notifications
pub async fn api_admin_notifications(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "notifications.view")
        .await?;

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
    .await?;

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
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<BroadcastNotificationRequest>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "notifications.send")
        .await?;
    let body = body.validate()?;

    let mut tx = state.db.begin().await?;
    let result = sqlx::query(
        r#"INSERT INTO notifications (user_id, title, message, type)
           SELECT id, $1, $2, $3 FROM users"#,
    )
    .bind(&body.title)
    .bind(&body.message)
    .bind(&body.notification_type)
    .execute(&mut *tx)
    .await?;

    let recipient_count = result.rows_affected();
    let audit_payload = json!({
        "type": body.notification_type,
        "title": body.title,
        "recipient_count": recipient_count
    });

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state, metadata)
           VALUES ($1, 'notification.broadcast', 'notifications', $2, $2)"#,
    )
    .bind(admin.user.id)
    .bind(audit_payload)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({"status":"broadcast_sent","count": recipient_count})).into_response())
}
