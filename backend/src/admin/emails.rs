use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State, Query},
    response::IntoResponse,
};
use sqlx::Row;

/// Standard pagination and search parameters for list endpoints
#[derive(serde::Deserialize)]
pub struct ListParams {
    /// Optional page index
    page: Option<i64>,
    /// Optional limit per page
    limit: Option<i64>,
    /// Optional search term
    search: Option<String>,
}

//
//  Admin Email Marketing API
//

/// GET /api/admin/emails  List overview stats
pub async fn api_admin_emails(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // 2. Real Aggregation
    let stats_row = sqlx::query!(
        r#"
        SELECT 
            COUNT(*)::bigint as total_sent,
            COUNT(*) FILTER (WHERE status IN ('delivered', 'opened', 'clicked'))::bigint as total_delivered,
            COUNT(*) FILTER (WHERE status IN ('opened', 'clicked'))::bigint as total_opened,
            COUNT(*) FILTER (WHERE status = 'clicked')::bigint as total_clicked,
            COUNT(*) FILTER (WHERE status = 'bounced')::bigint as total_bounced
        FROM email_logs
        WHERE status != 'queued'
        "#
    )
    .fetch_one(&state.db)
    .await;

    let (total_sent, total_delivered, total_opened, total_clicked, total_bounced) = match stats_row
    {
        Ok(r) => (
            r.total_sent.unwrap_or(0),
            r.total_delivered.unwrap_or(0),
            r.total_opened.unwrap_or(0),
            r.total_clicked.unwrap_or(0),
            r.total_bounced.unwrap_or(0),
        ),
        Err(_) => (0i64, 0i64, 0i64, 0i64, 0i64),
    };

    let delivery_rate = if total_sent > 0 {
        (total_delivered as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    let open_rate = if total_delivered > 0 {
        (total_opened as f64 / total_delivered as f64) * 100.0
    } else {
        0.0
    };

    let click_rate = if total_opened > 0 {
        (total_clicked as f64 / total_opened as f64) * 100.0
    } else {
        0.0
    };

    let bounce_rate = if total_sent > 0 {
        (total_bounced as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };

    let stats = serde_json::json!({
        "deliveryRate": (delivery_rate * 10.0).round() / 10.0,
        "deliveryTrend": 0.0,
        "openRate": (open_rate * 10.0).round() / 10.0,
        "clickRate": (click_rate * 10.0).round() / 10.0,
        "bounceRate": (bounce_rate * 10.0).round() / 10.0,
        "bouncesTotal": total_bounced,
        "totalSent": total_sent
    });

    Ok(
        Json(serde_json::json!({ "stats": stats })).into_response(),
    )
}

/// GET /api/admin/emails/logs  List paginated email logs
pub async fn api_admin_emails_logs(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<axum::response::Response, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(15).clamp(1, 100);
    let offset = (page - 1) * limit;
    let search = params.search.unwrap_or_default();
    
    let logs_query = if search.is_empty() {
        sqlx::query(
            "SELECT e.id::text, e.subject, e.recipient_email, e.status, e.sent_at::text, t.name as template_name
             FROM email_logs e
             LEFT JOIN email_templates t ON e.template_id = t.id
             ORDER BY e.sent_at DESC LIMIT $1 OFFSET $2"
        ).bind(limit).bind(offset)
    } else {
        let s = format!("%{}%", search);
        sqlx::query(
            "SELECT e.id::text, e.subject, e.recipient_email, e.status, e.sent_at::text, t.name as template_name
             FROM email_logs e
             LEFT JOIN email_templates t ON e.template_id = t.id
             WHERE e.recipient_email ILIKE $1 OR e.subject ILIKE $1
             ORDER BY e.sent_at DESC LIMIT $2 OFFSET $3"
        ).bind(s).bind(limit).bind(offset)
    };

    let log_rows = logs_query.fetch_all(&state.db).await.unwrap_or_default();
    
    let total_query = if search.is_empty() {
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM email_logs").fetch_one(&state.db).await.unwrap_or(0i64)
    } else {
        let s = format!("%{}%", search);
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM email_logs WHERE recipient_email ILIKE $1 OR subject ILIKE $1")
            .bind(s).fetch_one(&state.db).await.unwrap_or(0i64)
    };

    let logs: Vec<serde_json::Value> = log_rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<String, _>("id"), "subject": r.get::<String, _>("subject"),
            "recipient_email": r.get::<String, _>("recipient_email"),
            "status": r.get::<String, _>("status"), "sent_at": r.get::<String, _>("sent_at"),
            "template_name": r.get::<Option<String>, _>("template_name")
        })
    }).collect();

    Ok(Json(serde_json::json!({ "items": logs, "total": total_query })).into_response())
}

/// GET /api/admin/emails/templates  List paginated email templates
pub async fn api_admin_emails_templates(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<axum::response::Response, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * limit;
    let search = params.search.unwrap_or_default();
    
    let t_query = if search.is_empty() {
        sqlx::query(
            "SELECT id::text, name, subject, html_template, version, description, updated_at::text, 'transactional' as type FROM email_templates ORDER BY name ASC LIMIT $1 OFFSET $2"
        ).bind(limit).bind(offset)
    } else {
        let s = format!("%{}%", search);
        sqlx::query(
            "SELECT id::text, name, subject, html_template, version, description, updated_at::text, 'transactional' as type FROM email_templates WHERE name ILIKE $1 OR subject ILIKE $1 ORDER BY name ASC LIMIT $2 OFFSET $3"
        ).bind(s).bind(limit).bind(offset)
    };

    let t_rows = t_query.fetch_all(&state.db).await.unwrap_or_default();
    
    let total_query = if search.is_empty() {
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM email_templates").fetch_one(&state.db).await.unwrap_or(0i64)
    } else {
        let s = format!("%{}%", search);
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM email_templates WHERE name ILIKE $1 OR subject ILIKE $1")
            .bind(s).fetch_one(&state.db).await.unwrap_or(0i64)
    };

    let templates: Vec<serde_json::Value> = t_rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"),
            "subject": r.get::<String, _>("subject"), "version": r.get::<i32, _>("version"),
            "description": r.get::<Option<String>, _>("description"),
            "html_template": r.get::<String, _>("html_template"),
            "updated_at": r.get::<String, _>("updated_at"),
            "type": r.get::<String, _>("type")
        })
    }).collect();

    Ok(Json(serde_json::json!({ "items": templates, "total": total_query })).into_response())
}

/// GET /api/admin/emails/templates_all  Fetch all templates without pagination
pub async fn api_admin_emails_templates_all(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let t_query = sqlx::query("SELECT id::text, name, subject FROM email_templates ORDER BY name ASC");
    let t_rows = t_query.fetch_all(&state.db).await.unwrap_or_default();
    
    let templates: Vec<serde_json::Value> = t_rows.iter().map(|r| {
        serde_json::json!({
            "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"),
            "subject": r.get::<String, _>("subject")
        })
    }).collect();

    Ok(Json(serde_json::json!({ "items": templates })).into_response())
}

/// POST /api/admin/emails/test  Test send an email template
pub async fn api_admin_emails_test(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let template_id = body.get("templateId").and_then(|v| v.as_str()).unwrap_or("");
    if template_id.is_empty() {
        return Err(ApiError::BadRequest("Template ID required".to_string()));
    }
    
    let uid = ApiError::parse_uuid(template_id)?;
    
    let t_row = sqlx::query("SELECT subject FROM email_templates WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
    let subject: String = match t_row {
        Some(r) => sqlx::Row::get(&r, "subject"),
        None => return Err(ApiError::NotFound("Template not found".to_string())),
    };

    let admin_email = admin.user.email.clone();
    
    let _ = sqlx::query(
        "INSERT INTO email_logs (user_id, template_id, subject, recipient_email, status, sent_at) VALUES ($1, $2, $3, $4, 'delivered', NOW())"
    ).bind(admin.user.id).bind(uid).bind(&subject).bind(&admin_email).execute(&state.db).await;

    Ok(Json(serde_json::json!({"status":"test_queued", "recipient": admin_email})).into_response())
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
            Err(ApiError::Internal("Database error".to_string()))
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
            Err(ApiError::Internal("Database error".to_string()))
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

    let uid = ApiError::parse_uuid(template_id)?;

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
