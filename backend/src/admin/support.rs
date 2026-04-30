use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Support Tickets API
//

/// Query parameters for filtering support tickets
#[derive(serde::Deserialize)]
pub struct AdminSupportFilters {
    /// The page number
    pub page: Option<i64>,
    /// Number of items per page
    pub limit: Option<i64>,
    /// Status filter
    pub status: Option<String>,
    /// Priority filter
    pub priority: Option<String>,
    /// Search term
    pub search: Option<String>,
    /// Date filter
    pub date_filter: Option<String>,
    /// Field to sort by
    pub sort_field: Option<String>,
    /// Sort order (asc/desc)
    pub sort_order: Option<String>,
}

/// Payload for bulk updating support tickets
#[derive(serde::Deserialize)]
pub struct AdminSupportBulkPayload {
    /// List of ticket IDs to update
    pub ticket_ids: Vec<String>,
    /// New status string
    pub status: Option<String>,
    /// New priority string
    pub priority: Option<String>,
    /// New assignee user ID
    pub assigned_to: Option<String>,
}

/// GET /api/admin/support  List all support tickets with pagination and filtering
pub async fn api_admin_support_tickets(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(filters): axum::extract::Query<AdminSupportFilters>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "support.manage").await?;
    let mut query = String::from(
        r#"SELECT st.id::text, st.subject, st.message, st.status, st.priority,
                  st.category, st.metadata, st.sla_breach_at::text,
                  st.created_at::text, st.updated_at::text,
                  COALESCE(u.email, '') AS user_email,
                  COALESCE(up.first_name, '') AS first_name,
                  COALESCE(up.last_name, '') AS last_name,
                  COUNT(*) OVER() as total_count
           FROM support_tickets st
           JOIN users u ON u.id = st.user_id
           LEFT JOIN user_profiles up ON up.user_id = st.user_id
           WHERE 1=1"#,
    );

    let mut bind_idx = 1;
    let mut args = sqlx::postgres::PgArguments::default();

    if let Some(status) = &filters.status {
        if !status.is_empty() {
            query.push_str(&format!(" AND st.status = ${}", bind_idx));
            use sqlx::Arguments;
            let _ = args.add(status.clone());
            bind_idx += 1;
        }
    }

    if let Some(priority) = &filters.priority {
        if !priority.is_empty() {
            query.push_str(&format!(" AND st.priority = ${}", bind_idx));
            use sqlx::Arguments;
            let _ = args.add(priority.clone());
            bind_idx += 1;
        }
    }

    if let Some(search) = &filters.search {
        if !search.is_empty() {
            query.push_str(&format!(" AND (st.subject ILIKE ${} OR u.email ILIKE ${} OR up.first_name ILIKE ${} OR up.last_name ILIKE ${})", bind_idx, bind_idx, bind_idx, bind_idx));
            use sqlx::Arguments;
            let _ = args.add(format!("%{}%", search));
            bind_idx += 1;
        }
    }

    if let Some(df) = &filters.date_filter {
        if df == "7d" {
            query.push_str(" AND st.created_at >= NOW() - INTERVAL '7 days'");
        } else if df == "30d" {
            query.push_str(" AND st.created_at >= NOW() - INTERVAL '30 days'");
        }
    }

    let sort_field = filters.sort_field.as_deref().unwrap_or("created_at");
    let sort_order = filters.sort_order.as_deref().unwrap_or("desc");

    let sort_col = match sort_field {
        "subject" => "st.subject",
        "user_name" => "first_name",
        "priority" => "CASE st.priority WHEN 'urgent' THEN 4 WHEN 'high' THEN 3 WHEN 'normal' THEN 2 ELSE 1 END",
        "status" => "st.status",
        "updated_at" => "st.updated_at",
        _ => "st.created_at",
    };

    let order = if sort_order.eq_ignore_ascii_case("asc") {
        "ASC"
    } else {
        "DESC"
    };
    query.push_str(&format!(" ORDER BY {} {}", sort_col, order));

    let limit = filters.limit.unwrap_or(20).clamp(1, 100);
    let page = filters.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    query.push_str(&format!(" LIMIT ${} OFFSET ${}", bind_idx, bind_idx + 1));
    use sqlx::Arguments;
    let _ = args.add(limit);
    let _ = args.add(offset);

    let rows = sqlx::query_with(&query, args)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let total_count: i64 = rows.first().map(|r| r.get("total_count")).unwrap_or(0);

    let tickets: Vec<serde_json::Value> = rows.iter().map(|r| {
        let first: String = r.get("first_name");
        let last: String = r.get("last_name");
        let name = format!("{} {}", first, last).trim().to_string();
        let email: String = r.get("user_email");
        serde_json::json!({
            "id": r.get::<String, _>("id"), "subject": r.get::<String, _>("subject"),
            "message": r.get::<String, _>("message"), "status": r.get::<String, _>("status"),
            "priority": r.get::<String, _>("priority"),
            "category": r.get::<Option<String>, _>("category"),
            "sla_breach_at": r.get::<Option<String>, _>("sla_breach_at"),
            "metadata": r.get::<Option<serde_json::Value>, _>("metadata"),
            "created_at": r.get::<String, _>("created_at"), "updated_at": r.get::<Option<String>, _>("updated_at"),
            "user_email": &email,
            "user_name": if name.is_empty() { email.clone() } else { name }
        })
    }).collect();

    let stats = sqlx::query!(
        r#"SELECT 
            (SELECT COUNT(*) FROM support_tickets WHERE status = 'open') as open_count,
            (SELECT COUNT(*) FROM support_tickets WHERE status = 'in_progress') as progress_count,
            (SELECT COUNT(*) FROM support_tickets WHERE status IN ('resolved', 'closed')) as resolved_count,
            (SELECT COUNT(*) FROM support_tickets WHERE priority = 'urgent') as urgent_count"#
    ).fetch_optional(&state.db).await.unwrap_or(None);

    let mut st_json = serde_json::json!({
        "open": 0, "in_progress": 0, "resolved": 0, "urgent": 0
    });

    if let Some(s) = stats {
        st_json = serde_json::json!({
            "open": s.open_count.unwrap_or(0),
            "in_progress": s.progress_count.unwrap_or(0),
            "resolved": s.resolved_count.unwrap_or(0),
            "urgent": s.urgent_count.unwrap_or(0)
        });
    }

    Ok(Json(
        serde_json::json!({ "tickets": tickets, "total_count": total_count, "stats": st_json }),
    )
    .into_response())
}

/// PATCH /api/admin/support/bulk - Bulk update multiple tickets
pub async fn api_admin_support_bulk(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<AdminSupportBulkPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "support.manage").await?;
    if payload.ticket_ids.is_empty() {
        return Err(ApiError::BadRequest("No tickets selected".to_string()));
    }

    let mut query = String::from("UPDATE support_tickets SET updated_at = NOW()");
    let mut args = sqlx::postgres::PgArguments::default();
    let mut bind_idx = 1;

    use sqlx::Arguments;
    if let Some(status) = &payload.status {
        query.push_str(&format!(", status = ${}", bind_idx));
        let _ = args.add(status.clone());
        bind_idx += 1;
    }

    if let Some(priority) = &payload.priority {
        query.push_str(&format!(", priority = ${}", bind_idx));
        let _ = args.add(priority.clone());
        bind_idx += 1;
    }

    if let Some(assigned_to) = &payload.assigned_to {
        if assigned_to.is_empty() {
            query.push_str(", assigned_to = NULL");
        } else {
            if let Ok(uid) = assigned_to.parse::<sqlx::types::Uuid>() {
                query.push_str(&format!(", assigned_to = ${}", bind_idx));
                let _ = args.add(uid);
                bind_idx += 1;
            }
        }
    }

    if bind_idx == 1 && payload.assigned_to.is_none() {
        return Err(ApiError::BadRequest("No updates specified".to_string()));
    }

    let mut in_clause = Vec::new();
    for id in &payload.ticket_ids {
        if let Ok(uid) = id.parse::<sqlx::types::Uuid>() {
            in_clause.push(format!("${}", bind_idx));
            let _ = args.add(uid);
            bind_idx += 1;
        }
    }

    if in_clause.is_empty() {
        return Err(ApiError::BadRequest("Invalid ticket IDs".to_string()));
    }

    query.push_str(&format!(" WHERE id IN ({})", in_clause.join(", ")));

    let result = sqlx::query_with(&query, args).execute(&state.db).await;

    match result {
        Ok(r) => {
            let count = r.rows_affected();
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                   VALUES ($1, 'admin.support_bulk_update', 'support_tickets', $2)"#,
            )
            .bind(admin.user.id)
            .bind(serde_json::json!({
                "count": count,
                "changes": {
                    "status": payload.status,
                    "priority": payload.priority,
                    "assigned_to": payload.assigned_to
                }
            }))
            .execute(&state.db)
            .await;

            Ok(
                Json(serde_json::json!({"status":"updated", "count": count}))
                    .into_response(),
            )
        }
        Err(e) => {
            tracing::error!("Failed bulk update for support tickets: {:?}", e);
            Err(ApiError::Internal(
                "Failed to execute bulk update".to_string(),
            ))
        }
    }
}

/// PATCH /api/admin/support/:ticket_id  Update ticket status/priority
pub async fn api_admin_support_update(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(ticket_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "support.manage").await?;
    let uid = ApiError::parse_uuid(&ticket_id)?;

    let existing_res = sqlx::query_as::<_, (String, String)>(
        "SELECT status, priority FROM support_tickets WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await;

    let (curr_status, curr_priority) = match existing_res {
        Ok(Some(row)) => row,
        _ => {
            return Err(ApiError::NotFound("Ticket not found".to_string()));
        }
    };

    let new_status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or(&curr_status);
    let new_priority = body
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or(&curr_priority);

    // Check if assignee_id is provided
    let new_assignee = body.get("assigned_to").and_then(|v| v.as_str());

    let updated = if let Some(assignee_str) = new_assignee {
        if assignee_str.is_empty() {
            sqlx::query(
                "UPDATE support_tickets SET status = $1, priority = $2, assigned_to = NULL, updated_at = NOW() WHERE id = $3"
            )
            .bind(new_status).bind(new_priority).bind(uid)
            .execute(&state.db).await
        } else {
            let assignee_uid: Result<sqlx::types::Uuid, _> = assignee_str.parse();
            if let Ok(a_uid) = assignee_uid {
                sqlx::query(
                    "UPDATE support_tickets SET status = $1, priority = $2, assigned_to = $3, updated_at = NOW() WHERE id = $4"
                )
                .bind(new_status).bind(new_priority).bind(a_uid).bind(uid)
                .execute(&state.db).await
            } else {
                return Err(ApiError::BadRequest("Invalid Assignee ID".to_string()));
            }
        }
    } else {
        sqlx::query(
            "UPDATE support_tickets SET status = $1, priority = $2, updated_at = NOW() WHERE id = $3"
        )
        .bind(new_status).bind(new_priority).bind(uid)
        .execute(&state.db).await
    };

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'admin.support_ticket_update', 'support_tickets', $2, $3)"#,
            )
            .bind(admin.user.id)
            .bind(uid)
            .bind(serde_json::json!({
                "status": new_status,
                "priority": new_priority,
                "assigned_to": new_assignee
            }))
            .execute(&state.db)
            .await;

            Ok(Json(serde_json::json!({"status":"updated"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound("Ticket not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to update ticket {ticket_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// GET /api/admin/support/:ticket_id  Get ticket details & replies
pub async fn api_admin_support_ticket_detail(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(ticket_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "support.manage").await?;
    let uid = ApiError::parse_uuid(&ticket_id)?;

    let ticket_result = sqlx::query(
        r#"SELECT st.id, st.subject, st.message, st.status, st.priority,
                  st.category, st.metadata, st.sla_breach_at,
                  st.created_at, st.updated_at, st.user_id,
                  st.assigned_to,
                  COALESCE(u.email, 'unknown@poool.finance') AS user_email,
                  COALESCE(u.created_at, NOW()) AS user_created_at,
                  COALESCE(up.first_name, '') AS first_name,
                  COALESCE(up.last_name, '') AS last_name,
                  (SELECT COUNT(*) FROM support_tickets WHERE user_id = st.user_id AND status != 'closed')::bigint as user_open_tickets,
                  COALESCE((SELECT SUM(purchase_value_cents) FROM investments WHERE user_id = st.user_id), 0)::bigint as user_total_invested_cents
           FROM support_tickets st
           LEFT JOIN users u ON u.id = st.user_id
           LEFT JOIN user_profiles up ON up.user_id = st.user_id
           WHERE st.id = $1"#
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await;

    let ticket_row = match ticket_result {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Error fetching ticket {uid}: {e}");
            return Err(ApiError::Internal(e.to_string()));
        }
    };

    let row = match ticket_row {
        Some(r) => r,
        None => {
            return Err(ApiError::NotFound("Ticket not found".to_string()));
        }
    };

    let first: String = row.get("first_name");
    let last: String = row.get("last_name");
    let name = format!("{} {}", first, last).trim().to_string();
    let email: String = row.get("user_email");
    let user_name = if name.is_empty() {
        email.clone()
    } else {
        name.clone()
    };

    let mut ticket_json = serde_json::json!({
        "id": row.get::<uuid::Uuid, _>("id").to_string(),
        "subject": row.get::<String, _>("subject"),
        "status": row.get::<String, _>("status"),
        "priority": row.get::<String, _>("priority"),
        "category": row.get::<Option<String>, _>("category"),
        "metadata": row.get::<Option<serde_json::Value>, _>("metadata"),
        "sla_breach_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("sla_breach_at").map(|d| d.to_rfc3339()),
        "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        "updated_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("updated_at").map(|d| d.to_rfc3339()),
        "user_id": row.get::<uuid::Uuid, _>("user_id").to_string(),
        "assigned_to": row.get::<Option<uuid::Uuid>, _>("assigned_to").map(|u| u.to_string()),
        "user_email": &email,
        "user_name": user_name.clone(),
        "user_created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("user_created_at").to_rfc3339(),
        "user_open_tickets": row.get::<i64, _>("user_open_tickets"),
        "user_total_invested_cents": row.get::<i64, _>("user_total_invested_cents"),
        "messages": Vec::<serde_json::Value>::new(),
    });

    let replies = sqlx::query(
        r#"SELECT r.id, r.author_id, r.author_name, r.author_role, r.type, r.content, r.created_at,
                  COALESCE(
                      (SELECT json_agg(json_build_object('file_url', a.file_url, 'file_type', a.file_type)) 
                       FROM support_ticket_attachments a WHERE a.reply_id = r.id), 
                      '[]'::json
                  ) as attachments_json
           FROM support_ticket_replies r
           WHERE r.ticket_id = $1
           ORDER BY r.created_at ASC"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut messages: Vec<serde_json::Value> = Vec::new();

    // Avoid duplication: if the replies already contain an 'initial' type message, don't add the ticket message again
    let has_initial_reply = replies.iter().any(|r| {
        let t: String = r.get("type");
        t.trim() == "initial"
    });

    if !has_initial_reply {
        messages.push(serde_json::json!({
            "id": format!("msg-initial-{}", uid),
            "content": row.get::<String, _>("message"),
            "type": "initial",
            "author_role": "customer",
            "author_name": user_name,
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339()
        }));
    }

    // 2. Add replies
    for rep in replies {
        messages.push(serde_json::json!({
            "id": rep.get::<uuid::Uuid, _>("id").to_string(),
            "content": rep.get::<String, _>("content"),
            "type": rep.get::<String, _>("type"),
            "author_role": rep.get::<String, _>("author_role"),
            "author_name": rep.get::<String, _>("author_name"),
            "attachments_json": rep.get::<serde_json::Value, _>("attachments_json"),
            "created_at": rep.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339()
        }));
    }

    // 3. Add audit logs
    let logs = sqlx::query(
        r#"SELECT l.id, l.action, l.created_at, u.email as actor_email, up.first_name, up.last_name
           FROM audit_logs l 
           LEFT JOIN users u ON u.id = l.actor_user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           WHERE l.entity_id = $1 AND l.entity_type = 'support_tickets'
        "#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    for lg in logs {
        let action: String = lg.get("action");
        let first: Option<String> = lg.get("first_name");
        let last: Option<String> = lg.get("last_name");
        let email: Option<String> = lg.get("actor_email");
        let name = if let (Some(f), Some(l)) = (first, last) {
            let n = format!("{} {}", f, l).trim().to_string();
            if n.is_empty() {
                email.unwrap_or_else(|| "System".to_string())
            } else {
                n
            }
        } else {
            email.unwrap_or_else(|| "System".to_string())
        };

        let content = match action.as_str() {
            "ticket.status_changed" => "changed the ticket status",
            "ticket.priority_changed" => "changed the ticket priority",
            "ticket.assigned" => "changed ticket assignment",
            _ => "updated the ticket",
        };

        messages.push(serde_json::json!({
            "id": lg.get::<uuid::Uuid, _>("id").to_string(),
            "content": format!("<i>* {}</i>", content),
            "type": "internal_note",
            "author_role": "system",
            "author_name": name,
            "created_at": lg.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339()
        }));
    }

    // Sort messages chronologically
    messages.sort_by_key(|m| {
        m.get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    });

    ticket_json["messages"] = serde_json::json!(messages);
    Ok(Json(ticket_json).into_response())
}

/// POST /api/admin/support/:ticket_id/messages  Add a reply or internal note
pub async fn api_admin_support_ticket_reply(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(ticket_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "support.manage").await?;
    let current_user = admin.user.clone();

    let uid = ApiError::parse_uuid(&ticket_id)?;

    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let mtype = body.get("type").and_then(|v| v.as_str()).unwrap_or("reply");

    if content.trim().is_empty() {
        return Err(ApiError::BadRequest("Content is required".to_string()));
    }

    let name = current_user
        .email
        .split('@')
        .next()
        .unwrap_or("Admin")
        .to_string();

    let sanitized_content = crate::common::sanitize::sanitize_html(content);

    let inserted = sqlx::query(
        "INSERT INTO support_ticket_replies (ticket_id, author_id, author_name, author_role, type, content) VALUES ($1, $2, $3, 'admin', $4, $5)"
    )
    .bind(uid).bind(current_user.id).bind(&name).bind(mtype).bind(sanitized_content)
    .execute(&state.db).await;

    match inserted {
        Ok(_) => {
            let _ = sqlx::query("UPDATE support_tickets SET updated_at = NOW() WHERE id = $1")
                .bind(uid)
                .execute(&state.db)
                .await;
            Ok(Json(serde_json::json!({"status":"reply_added"})).into_response())
        }
        Err(e) => {
            tracing::error!("Failed to append reply {ticket_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}
