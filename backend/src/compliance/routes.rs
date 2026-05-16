//! Admin-facing compliance endpoints. Today: sanctions re-screening +
//! the compliance-alerts queue. Future P0-1 transaction-monitoring
//! findings will land in the same alerts table.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct AlertListQuery {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub limit: Option<i64>,
}

/// GET /api/admin/compliance/alerts
///
/// List compliance alerts. Defaults to open + sorted by severity then
/// recency. Useful for the compliance dashboard.
pub async fn api_admin_compliance_alerts(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<AlertListQuery>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "compliance.read").await?;

    let mut wheres = Vec::<&'static str>::new();
    match params.status.as_deref().unwrap_or("open") {
        "open" => wheres.push("closed_at IS NULL"),
        "closed" => wheres.push("closed_at IS NOT NULL"),
        "all" => {}
        _ => return Err(ApiError::BadRequest("Invalid status filter".into())),
    }
    let severity = params.severity.as_deref();
    if let Some(s) = severity {
        if !matches!(s, "low" | "medium" | "high" | "critical") {
            return Err(ApiError::BadRequest("Invalid severity filter".into()));
        }
    }
    if severity.is_some() {
        wheres.push("severity = $1");
    }
    let where_sql = if wheres.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", wheres.join(" AND "))
    };
    let limit = params.limit.unwrap_or(100).clamp(1, 500);

    let sql = format!(
        r#"
        SELECT a.id::text, a.user_id::text, a.kind, a.severity, a.summary,
               a.details, a.source_log_id::text, a.assigned_to::text,
               a.closed_at::text, a.created_at::text,
               u.email
          FROM compliance_alerts a
          JOIN users u ON u.id = a.user_id
          {where_sql}
         ORDER BY
            CASE a.severity
                WHEN 'critical' THEN 4 WHEN 'high' THEN 3
                WHEN 'medium' THEN 2 ELSE 1
            END DESC,
            a.created_at DESC
         LIMIT {limit}
        "#,
    );

    let rows: Vec<sqlx::postgres::PgRow> = if let Some(s) = severity {
        sqlx::query(&sql).bind(s).fetch_all(&state.db).await
    } else {
        sqlx::query(&sql).fetch_all(&state.db).await
    }
    .map_err(ApiError::from)?;

    use sqlx::Row;
    let alerts: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<String, _>("user_id"),
                "user_email": r.get::<String, _>("email"),
                "kind": r.get::<String, _>("kind"),
                "severity": r.get::<String, _>("severity"),
                "summary": r.get::<String, _>("summary"),
                "details": r.get::<Option<serde_json::Value>, _>("details"),
                "source_log_id": r.get::<Option<String>, _>("source_log_id"),
                "assigned_to": r.get::<Option<String>, _>("assigned_to"),
                "closed_at": r.get::<Option<String>, _>("closed_at"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "alerts": alerts })).into_response())
}

#[derive(Debug, Deserialize)]
pub struct CloseAlertPayload {
    pub close_reason: String,
}

/// POST /api/admin/compliance/alerts/:id/close
///
/// Mark an alert as triaged. Compliance staff must supply a brief
/// close reason — keeps an audit trail of why we ignored a hit.
pub async fn api_admin_compliance_close_alert(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CloseAlertPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "compliance.write").await?;
    let reason = payload.close_reason.trim();
    if reason.is_empty() {
        return Err(ApiError::BadRequest("Close reason is required".into()));
    }

    let updated = sqlx::query(
        "UPDATE compliance_alerts
            SET closed_at = NOW(), closed_by = $1, close_reason = $2
          WHERE id = $3 AND closed_at IS NULL",
    )
    .bind(admin.user.id)
    .bind(reason)
    .bind(id)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound("Alert not found or already closed".into()));
    }

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'compliance.alert_closed', 'compliance_alert', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(id)
    .bind(serde_json::json!({ "close_reason": reason }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({ "status": "closed" })).into_response())
}

/// POST /api/admin/compliance/users/:user_id/rescreen
///
/// Force a re-screening of one user immediately. Returns the resulting
/// log + alert info so the operator can act without refreshing.
pub async fn api_admin_compliance_rescreen(
    _jar: CookieJar,
    admin: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "compliance.write").await?;
    let result = crate::compliance::rescreening::rescreen_user(&state.db, user_id).await;
    let (status, summary): (&str, Option<String>) = match &result {
        crate::compliance::rescreening::ScreeningResult::Clear => ("clear", None),
        crate::compliance::rescreening::ScreeningResult::Hit { summary, .. } => {
            ("hit", Some(summary.clone()))
        }
        crate::compliance::rescreening::ScreeningResult::Error(e) => ("error", Some(e.clone())),
        crate::compliance::rescreening::ScreeningResult::Skipped(r) => ("skipped", Some(r.to_string())),
    };
    Ok(Json(serde_json::json!({
        "status": status,
        "summary": summary,
        "user_id": user_id.to_string(),
        "checked_at": chrono::Utc::now().to_rfc3339(),
    }))
    .into_response())
}
