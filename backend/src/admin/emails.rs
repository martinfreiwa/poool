use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Email Marketing API
//

/// GET /api/admin/emails  List templates, overview stats, and recent logs
pub async fn api_admin_emails(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // 1. Templates
    let t_rows = sqlx::query(
        "SELECT id::text, name, subject, html_template, version, description, updated_at::text, 'transactional' as type FROM email_templates ORDER BY name ASC" // Include html_template
    ).fetch_all(&state.db).await.unwrap_or_default();

    let templates: Vec<serde_json::Value> = t_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"),
                "subject": r.get::<String, _>("subject"), "version": r.get::<i32, _>("version"),
                "description": r.get::<Option<String>, _>("description"),
                "html_template": r.get::<String, _>("html_template"),
                "updated_at": r.get::<String, _>("updated_at"),
                "type": r.get::<String, _>("type")
            })
        })
        .collect();

    // 2. Mock Stats / Real aggregation
    let count_sent: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM email_logs WHERE status != 'queued'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let stats = serde_json::json!({
        "deliveryRate": 99.8,
        "deliveryTrend": 0.2,
        "openRate": 42.5,
        "clickRate": 18.2,
        "bounceRate": 0.2,
        "bouncesTotal": 14,
        "totalSent": count_sent
    });

    // 3. Logs
    let log_rows = sqlx::query(
        r#"SELECT e.id::text, e.subject, e.recipient_email, e.status, e.sent_at::text
           FROM email_logs e
           ORDER BY e.sent_at DESC LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let logs: Vec<serde_json::Value> = log_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"), "subject": r.get::<String, _>("subject"),
                "recipient_email": r.get::<String, _>("recipient_email"),
                "status": r.get::<String, _>("status"), "sent_at": r.get::<String, _>("sent_at"),
            })
        })
        .collect();

    Ok(
        Json(serde_json::json!({ "templates": templates, "stats": stats, "logs": logs }))
            .into_response(),
    )
}

/// POST /api/admin/emails/templates
pub async fn api_admin_emails_create(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let subject = body.get("subject").and_then(|v| v.as_str()).unwrap_or("");
    let description = body
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let html_template = body
        .get("html_template")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if name.is_empty() || subject.is_empty() || html_template.is_empty() {
        return Err(ApiError::BadRequest("Missing required fields".to_string()));
    }

    let result = sqlx::query(
        "INSERT INTO email_templates (name, subject, html_template, description, version) VALUES ($1, $2, $3, $4, 1) RETURNING id"
    )
    .bind(name).bind(subject).bind(html_template).bind(description)
    .execute(&state.db).await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"created"})).into_response()),
        Err(e) => {
            tracing::error!("Failed to create template: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

/// PUT /api/admin/emails/templates/:id
pub async fn api_admin_emails_update(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&id)?;

    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let subject = body.get("subject").and_then(|v| v.as_str()).unwrap_or("");
    let description = body
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let html_template = body
        .get("html_template")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let result = sqlx::query(
        "UPDATE email_templates SET name = $1, subject = $2, html_template = $3, description = $4, version = version + 1, updated_at = NOW() WHERE id = $5"
    )
    .bind(name).bind(subject).bind(html_template).bind(description).bind(uid)
    .execute(&state.db).await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"updated"})).into_response()),
        Err(e) => {
            tracing::error!("Failed to update template: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

/// POST /api/admin/emails/campaigns
pub async fn api_admin_emails_campaign(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let template_id = body
        .get("templateId")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let audience = body
        .get("audience")
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    if template_id.is_empty() {
        return Err(ApiError::BadRequest("Template ID required".to_string()));
    }

    let uid = ApiError::parse_uuid(&template_id)?;

    let t_row = sqlx::query("SELECT subject FROM email_templates WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
    let subject: String = match t_row {
        Some(r) => sqlx::Row::get(&r, "subject"),
        None => {
            return Err(ApiError::NotFound("Template not found".to_string()));
        }
    };

    // Find users based on audience
    let query = match audience {
        "investors" => "SELECT id, email FROM users WHERE id IN (SELECT user_id FROM investments) AND status = 'active'",
        "kyc_approved" => "SELECT id, email FROM users WHERE id IN (SELECT user_id FROM kyc_records WHERE status = 'approved') AND status = 'active'",
        _ => "SELECT id, email FROM users WHERE status = 'active'"
    };

    let users = sqlx::query(query)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let mut sent_count = 0;

    for row in users {
        let u_id: sqlx::types::Uuid = sqlx::Row::get(&row, "id");
        let u_email: String = sqlx::Row::get(&row, "email");

        let _ = sqlx::query(
            "INSERT INTO email_logs (user_id, template_id, subject, recipient_email, status, sent_at) VALUES ($1, $2, $3, $4, 'sent', NOW())"
        ).bind(u_id).bind(uid).bind(&subject).bind(&u_email).execute(&state.db).await;

        sent_count += 1;
    }

    Ok(
        Json(serde_json::json!({"status":"campaign_queued", "target_count": sent_count}))
            .into_response(),
    )
}
