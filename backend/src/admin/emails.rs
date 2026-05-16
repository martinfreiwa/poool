use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Email Marketing API
//

/// Maximum marketing campaigns that can be triggered in a rolling hour
/// across the whole system. A typo or runaway script multiplied by the
/// `all` audience would otherwise mail every user multiple times.
const CAMPAIGN_RATE_LIMIT_PER_HOUR: i64 = 5;

/// Event-type used for outbox rows produced by the admin campaign tool.
/// Classified as optional in `is_optional_email_event` so List-Unsubscribe
/// and `email_notifications=false` are honoured per recipient.
pub const MARKETING_CAMPAIGN_EVENT_TYPE: &str = "marketing_campaign";

/// GET /api/admin/emails — list templates (without bulky HTML), KPI stats, recent logs.
pub async fn api_admin_emails(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    // 1. Templates — exclude the heavy HTML column so the list endpoint
    // stays small. The detail endpoint (`GET /:id`) returns the full body.
    let t_rows = sqlx::query(
        "SELECT id::text, name, subject, version, description, updated_at::text, 'transactional' as type
         FROM email_templates ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let templates: Vec<serde_json::Value> = t_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"),
                "subject": r.get::<String, _>("subject"), "version": r.get::<i32, _>("version"),
                "description": r.get::<Option<String>, _>("description"),
                "updated_at": r.get::<String, _>("updated_at"),
                "type": r.get::<String, _>("type")
            })
        })
        .collect();

    // 2. KPI aggregates — current 30d window + previous 30d window so the
    // dashboard delivery trend is real instead of a hard-coded zero.
    let stats_row = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days')::bigint
                AS total_sent,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status IN ('delivered', 'opened', 'clicked'))::bigint
                AS total_delivered,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status IN ('opened', 'clicked'))::bigint
                AS total_opened,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status = 'clicked')::bigint
                AS total_clicked,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '30 days'
                             AND status = 'bounced')::bigint
                AS total_bounced,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '60 days'
                             AND sent_at <  NOW() - INTERVAL '30 days')::bigint
                AS prev_total_sent,
            COUNT(*) FILTER (WHERE sent_at >= NOW() - INTERVAL '60 days'
                             AND sent_at <  NOW() - INTERVAL '30 days'
                             AND status IN ('delivered', 'opened', 'clicked'))::bigint
                AS prev_total_delivered
        FROM email_logs
        WHERE status != 'queued'
        "#
    )
    .fetch_one(&state.db)
    .await;

    let (
        total_sent,
        total_delivered,
        total_opened,
        total_clicked,
        total_bounced,
        prev_total_sent,
        prev_total_delivered,
    ) = match stats_row {
        Ok(r) => (
            r.total_sent.unwrap_or(0),
            r.total_delivered.unwrap_or(0),
            r.total_opened.unwrap_or(0),
            r.total_clicked.unwrap_or(0),
            r.total_bounced.unwrap_or(0),
            r.prev_total_sent.unwrap_or(0),
            r.prev_total_delivered.unwrap_or(0),
        ),
        Err(_) => (0i64, 0i64, 0i64, 0i64, 0i64, 0i64, 0i64),
    };

    let delivery_rate = if total_sent > 0 {
        (total_delivered as f64 / total_sent as f64) * 100.0
    } else {
        0.0
    };
    let prev_delivery_rate = if prev_total_sent > 0 {
        (prev_total_delivered as f64 / prev_total_sent as f64) * 100.0
    } else {
        0.0
    };
    let delivery_trend = delivery_rate - prev_delivery_rate;

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
        "deliveryTrend": (delivery_trend * 10.0).round() / 10.0,
        "openRate": (open_rate * 10.0).round() / 10.0,
        "clickRate": (click_rate * 10.0).round() / 10.0,
        "bounceRate": (bounce_rate * 10.0).round() / 10.0,
        "bouncesTotal": total_bounced,
        "totalSent": total_sent
    });

    // 3. Recent delivery logs
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

/// GET /api/admin/emails/templates/:id — full template incl. HTML body.
pub async fn api_admin_emails_get(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;
    let uid = ApiError::parse_uuid(&id)?;

    let row = sqlx::query(
        "SELECT id::text, name, subject, html_template, version, description, updated_at::text
         FROM email_templates WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::from)?;

    let Some(r) = row else {
        return Err(ApiError::NotFound("Template not found".to_string()));
    };

    Ok(Json(serde_json::json!({
        "id": r.get::<String, _>("id"),
        "name": r.get::<String, _>("name"),
        "subject": r.get::<String, _>("subject"),
        "html_template": r.get::<String, _>("html_template"),
        "version": r.get::<i32, _>("version"),
        "description": r.get::<Option<String>, _>("description"),
        "updated_at": r.get::<String, _>("updated_at"),
    }))
    .into_response())
}

/// POST /api/admin/emails/templates
pub async fn api_admin_emails_create(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;

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
        "INSERT INTO email_templates (name, subject, html_template, description, version)
         VALUES ($1, $2, $3, $4, 1) RETURNING id",
    )
    .bind(name)
    .bind(subject)
    .bind(html_template)
    .bind(description)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"created"})).into_response()),
        Err(e) => {
            // Unique-name collision is a user-fixable BadRequest, not a 500.
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Err(ApiError::Conflict(format!(
                        "Template name '{name}' already exists"
                    )));
                }
            }
            tracing::error!("Failed to create template: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// PUT /api/admin/emails/templates/:id
pub async fn api_admin_emails_update(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;
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

    if name.is_empty() || subject.is_empty() || html_template.is_empty() {
        return Err(ApiError::BadRequest("Missing required fields".to_string()));
    }

    let result = sqlx::query(
        "UPDATE email_templates SET name = $1, subject = $2, html_template = $3,
         description = $4, version = version + 1, updated_at = NOW()
         WHERE id = $5 RETURNING id",
    )
    .bind(name)
    .bind(subject)
    .bind(html_template)
    .bind(description)
    .bind(uid)
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(_)) => Ok(Json(serde_json::json!({"status":"updated"})).into_response()),
        Ok(None) => Err(ApiError::NotFound("Template not found".to_string())),
        Err(e) => {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return Err(ApiError::Conflict(format!(
                        "Template name '{name}' already exists"
                    )));
                }
            }
            tracing::error!("Failed to update template: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// DELETE /api/admin/emails/templates/:id — hard delete. `email_logs.template_id`
/// has `ON DELETE SET NULL` so historical logs are preserved.
pub async fn api_admin_emails_delete(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.edit").await?;
    let uid = ApiError::parse_uuid(&id)?;

    let res = sqlx::query("DELETE FROM email_templates WHERE id = $1")
        .bind(uid)
        .execute(&state.db)
        .await
        .map_err(ApiError::from)?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("Template not found".to_string()));
    }

    Ok(Json(serde_json::json!({"status":"deleted"})).into_response())
}

/// SQL fragment selecting `(id, email, first_name)` triples for a given
/// audience segment. `first_name` is left-joined from `user_profiles` and
/// returned as empty string when missing, so the campaign render context
/// can always interpolate `{{first_name}}` without a NULL panic.
///
/// Centralised so the campaign endpoint and the recipient-count preview
/// stay in sync — and so the queries can be regression-tested in one place.
///
/// Returns `None` for an unknown segment (caller maps that to BadRequest).
fn audience_query(segment: &str) -> Option<&'static str> {
    match segment {
        "all" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE",
        ),
        "investors" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND EXISTS (SELECT 1 FROM investments i WHERE i.user_id = u.id)",
        ),
        "kyc_approved" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND EXISTS (
                  SELECT 1 FROM kyc_records k
                  WHERE k.user_id = u.id AND k.status = 'approved'
                )",
        ),
        // Tier 'Plus' has sort_order = 2 (Intro=1, Plus=2, Pro=3, Elite=4, Premium=5).
        // "Plus and above" = sort_order >= 2.
        "tier_plus" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
               JOIN user_tiers ut ON ut.user_id = u.id
               JOIN tiers t ON t.id = ut.tier_id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND t.sort_order >= 2",
        ),
        // Dormant = no successful login in the past 30 days. `user_sessions`
        // is the source of truth (one row per login). Users with no session
        // at all are also dormant.
        "dormant" => Some(
            "SELECT u.id, u.email, COALESCE(p.first_name, '') AS first_name
               FROM users u
               LEFT JOIN user_profiles p ON p.user_id = u.id
              WHERE u.status = 'active' AND u.email_verified = TRUE
                AND NOT EXISTS (
                  SELECT 1 FROM user_sessions s
                  WHERE s.user_id = u.id
                    AND s.created_at >= NOW() - INTERVAL '30 days'
                )",
        ),
        _ => None,
    }
}

/// POST /api/admin/emails/campaigns
///
/// Resolves the template + audience, then enqueues one row per recipient
/// into `transactional_email_outbox`. The existing outbox worker delivers
/// via Resend, retries with backoff, and honours per-user opt-out via
/// `is_optional_email_event` — campaigns are classified optional, so users
/// who toggled off `email_notifications` (or used the inbox unsubscribe
/// button) are skipped at send time.
///
/// Rate-limited globally to `CAMPAIGN_RATE_LIMIT_PER_HOUR` campaigns in a
/// rolling hour so a runaway script cannot mail the entire user base
/// repeatedly.
pub async fn api_admin_emails_campaign(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.send").await?;

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

    let Some(audience_sql) = audience_query(audience) else {
        return Err(ApiError::BadRequest(format!(
            "Unknown audience segment: {audience}"
        )));
    };

    let uid = ApiError::parse_uuid(template_id)?;

    // Rate limit: count distinct campaign batches in the last hour. Each
    // batch shares a `created_at` second within ~1ms, so we approximate by
    // bucketing outbox rows on `event_type` + subject within the window.
    let recent_batches = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT date_trunc('second', created_at))
           FROM transactional_email_outbox
          WHERE event_type = $1
            AND created_at > NOW() - INTERVAL '1 hour'",
    )
    .bind(MARKETING_CAMPAIGN_EVENT_TYPE)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    if recent_batches >= CAMPAIGN_RATE_LIMIT_PER_HOUR {
        return Err(ApiError::TooManyRequests(format!(
            "Campaign rate limit reached ({CAMPAIGN_RATE_LIMIT_PER_HOUR}/hour). \
             Wait before sending the next campaign."
        )));
    }

    let t_row =
        sqlx::query("SELECT subject, html_template FROM email_templates WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?;
    let Some(r) = t_row else {
        return Err(ApiError::NotFound("Template not found".to_string()));
    };
    let subject: String = r.get("subject");
    let html_body: String = r.get("html_template");

    let users = sqlx::query(audience_sql)
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::from)?;
    let mut queued_count: i64 = 0;

    for row in users {
        let u_id: sqlx::types::Uuid = row.get("id");
        let u_email: String = row.get("email");
        let first_name: String = row.try_get("first_name").unwrap_or_default();

        // Per-recipient render: {{first_name}} / {{email}} interpolated via
        // MiniJinja. Subject is rendered too so admins can personalise the
        // email subject line (`Hi {{first_name}}, your asset shipped`).
        let ctx = serde_json::json!({
            "first_name": first_name,
            "email": u_email,
            "user_id": u_id.to_string(),
        });
        let rendered_subject = crate::common::email::render_template(&subject, &ctx);
        let rendered_body = crate::common::email::render_template(&html_body, &ctx);

        // Durable enqueue. Worker picks it up via process_transactional_email_outbox.
        let outbox_result = sqlx::query(
            "INSERT INTO transactional_email_outbox
                (user_id, event_type, recipient_email, subject, html_body)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(u_id)
        .bind(MARKETING_CAMPAIGN_EVENT_TYPE)
        .bind(&u_email)
        .bind(&rendered_subject)
        .bind(&rendered_body)
        .execute(&state.db)
        .await;

        if outbox_result.is_err() {
            continue;
        }

        // Mirror to email_logs so the delivery-logs tab in the admin shows
        // the queue immediately (worker will update status on send).
        let _ = sqlx::query(
            "INSERT INTO email_logs
                (user_id, template_id, subject, recipient_email, status, sent_at)
             VALUES ($1, $2, $3, $4, 'queued', NOW())",
        )
        .bind(u_id)
        .bind(uid)
        .bind(&rendered_subject)
        .bind(&u_email)
        .execute(&state.db)
        .await;

        queued_count += 1;
    }

    // Best-effort audit trail. Lets admins answer "who sent campaign X?".
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'email_campaign_queued', 'email_template', $2, $3)",
    )
    .bind(admin.user.id)
    .bind(uid)
    .bind(serde_json::json!({
        "audience": audience,
        "queued_count": queued_count,
        "subject": subject,
    }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({
        "status": "campaign_queued",
        "target_count": queued_count,
    }))
    .into_response())
}

/// GET /api/admin/emails/audiences/:segment/count — recipient count for a
/// segment without sending. Powers the campaign preview "this will mail N
/// users" UI introduced in Commit 5.
pub async fn api_admin_emails_audience_count(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(segment): Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "emails.view").await?;

    let Some(sql) = audience_query(&segment) else {
        return Err(ApiError::BadRequest(format!(
            "Unknown audience segment: {segment}"
        )));
    };

    let count_sql = format!("SELECT COUNT(*) FROM ({sql}) AS _seg");
    let count: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(serde_json::json!({ "segment": segment, "count": count })).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audience_query_known_segments_return_sql() {
        for seg in ["all", "investors", "kyc_approved", "tier_plus", "dormant"] {
            assert!(
                audience_query(seg).is_some(),
                "segment '{seg}' should be supported"
            );
        }
    }

    #[test]
    fn audience_query_unknown_segment_returns_none() {
        assert!(audience_query("totally_unknown").is_none());
        assert!(audience_query("").is_none());
        assert!(audience_query("ALL").is_none(), "case-sensitive on purpose");
    }

    #[test]
    fn audience_queries_all_select_id_email_and_first_name() {
        // Every segment query must expose `id`, `email`, and `first_name`
        // (left-joined from user_profiles, COALESCED to empty string) so
        // the campaign render loop can construct a per-user context.
        for seg in ["all", "investors", "kyc_approved", "tier_plus", "dormant"] {
            let sql = audience_query(seg).unwrap();
            assert!(sql.contains("u.id"), "segment '{seg}' missing u.id column");
            assert!(
                sql.contains("u.email"),
                "segment '{seg}' missing u.email column"
            );
            assert!(
                sql.contains("first_name"),
                "segment '{seg}' missing first_name column — render loop \
                 would crash trying to read it"
            );
            assert!(
                sql.contains("LEFT JOIN user_profiles"),
                "segment '{seg}' must LEFT JOIN user_profiles so users \
                 with no profile row still receive campaign mail"
            );
        }
    }

    #[test]
    fn audience_queries_filter_inactive_and_unverified() {
        // All audiences must exclude unverified or suspended users —
        // otherwise we'd mail people who never confirmed their address.
        for seg in ["all", "investors", "kyc_approved", "tier_plus", "dormant"] {
            let sql = audience_query(seg).unwrap();
            assert!(
                sql.contains("status = 'active'"),
                "segment '{seg}' must filter active users"
            );
            assert!(
                sql.contains("email_verified = TRUE"),
                "segment '{seg}' must filter verified emails"
            );
        }
    }

    #[test]
    fn marketing_campaign_event_type_is_optional_class() {
        // The outbox worker pref-gates optional events. Campaigns MUST be
        // optional so a user who unsubscribed via List-Unsubscribe is
        // skipped at send time.
        assert!(crate::common::email::is_optional_email_event(
            MARKETING_CAMPAIGN_EVENT_TYPE
        ));
    }
}
