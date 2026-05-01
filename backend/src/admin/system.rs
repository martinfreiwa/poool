use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin System Health API
//

/// GET /api/admin/system  DB size, table stats, environment
pub async fn api_admin_system(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    // DB size
    let db_size_result: Result<(String, i64), sqlx::Error> = sqlx::query_as(
        "SELECT pg_size_pretty(pg_database_size(current_database())), pg_database_size(current_database())"
    )
    .fetch_one(&state.db)
    .await;
    let db_healthy = db_size_result.is_ok();
    let (db_size, db_bytes) = db_size_result.unwrap_or_else(|_| ("unknown".to_string(), 0));

    // Storage estimation for costs
    let storage_bytes_result: Result<i64, sqlx::Error> = sqlx::query_scalar(
        r#"
        SELECT
            (SELECT COUNT(*) FROM kyc_documents) * 350000 +
            (SELECT COALESCE(SUM(file_size_bytes), 0) FROM asset_documents) +
            (SELECT COUNT(*) FROM asset_images) * 600000 +
            (SELECT COUNT(*) FROM users WHERE avatar_url IS NOT NULL AND avatar_url <> '') * 80000
        "#,
    )
    .fetch_one(&state.db)
    .await;
    let storage_available = storage_bytes_result.is_ok();
    let storage_bytes = storage_bytes_result.unwrap_or(0);

    // Table stats
    let table_rows_result = sqlx::query_as::<_, (String, i64, String)>(
        r#"SELECT s.relname, s.n_live_tup, pg_size_pretty(pg_total_relation_size(c.oid))
           FROM pg_stat_user_tables s
           JOIN pg_class c ON c.relname = s.relname AND c.relnamespace = s.schemaname::regnamespace
           ORDER BY s.n_live_tup DESC"#,
    )
    .fetch_all(&state.db)
    .await;
    let table_stats_available = table_rows_result.is_ok();
    let table_rows = table_rows_result.unwrap_or_default();

    let total_records: i64 = table_rows.iter().map(|r| r.1).sum();
    let max_rows = table_rows.iter().map(|r| r.1).max().unwrap_or(1);

    let tables: Vec<serde_json::Value> = table_rows
        .iter()
        .map(|r| serde_json::json!({ "name": r.0, "row_count": r.1, "size": r.2 }))
        .collect();

    // Cost Estimates (Monthly USD)
    let storage_gb = storage_bytes as f64 / 1_073_741_824.0;
    let storage_cost = (storage_gb * 0.020).max(0.01); // Min 1 cent if files exist

    let db_gb = db_bytes as f64 / 1_073_741_824.0;
    let db_cost = db_gb * 0.15 + 9.90; // $9.90 base for small instance + storage

    let compute_cost = 5.00; // Base Cloud Run estimate

    let total_monthly_cost = storage_cost + db_cost + compute_cost;

    let psp_connected = std::env::var("STRIPE_SECRET_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
        || std::env::var("STRIPE_PUBLISHABLE_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_some();
    let kyc_provider = if state.config.didit_api_key.is_some() {
        Some("didit")
    } else {
        None
    };
    let email_configured = std::env::var("RESEND_API_KEY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .is_some()
        || std::env::var("SMTP_HOST")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .is_some();
    let api_healthy = db_healthy && storage_available && table_stats_available;

    Ok(Json(serde_json::json!({
        "api_healthy": api_healthy,
        "db_healthy": db_healthy,
        "db_connected": db_healthy,
        "psp_connected": psp_connected,
        "kyc_provider": kyc_provider,
        "email_configured": email_configured,
        "email_connected": email_configured,
        "database": {
            "size": db_size,
            "tables": tables,
            "total_records": total_records,
            "max_rows": max_rows,
            "storage_available": storage_available,
            "table_stats_available": table_stats_available,
        },
        "costs": {
            "storage_monthly_usd": (storage_cost * 100.0).round() / 100.0,
            "database_monthly_usd": (db_cost * 100.0).round() / 100.0,
            "compute_monthly_usd": (compute_cost * 100.0).round() / 100.0,
            "total_monthly_usd": (total_monthly_cost * 100.0).round() / 100.0,
        },
        "environment": {
            "Runtime": "Rust + Axum",
            "Database": "PostgreSQL",
            "API Version": "v1.0.0"
        },
        "recent_errors": []
    }))
    .into_response())
}

/// GET /api/admin/system/sessions — List all active user sessions.
pub async fn api_admin_system_sessions(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let rows = sqlx::query(
        r#"SELECT s.id::text, s.user_id::text, COALESCE(u.email,'') AS email,
                  COALESCE(s.ip_address::text,'') AS ip_address,
                  COALESCE(s.user_agent,'') AS user_agent,
                  s.created_at::text, s.expires_at::text,
                  COALESCE(s.remember_me, false) AS remember_me
           FROM user_sessions s
           JOIN users u ON u.id = s.user_id
           WHERE s.expires_at > NOW()
           ORDER BY s.created_at DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let sessions: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<String, _>("user_id"),
                "email": r.get::<String, _>("email"),
                "ip_address": r.get::<String, _>("ip_address"),
                "user_agent": r.get::<String, _>("user_agent"),
                "created_at": r.get::<String, _>("created_at"),
                "expires_at": r.get::<String, _>("expires_at"),
                "remember_me": r.get::<bool, _>("remember_me"),
            })
        })
        .collect();

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions WHERE expires_at > NOW()")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "sessions": sessions,
        "total": total
    }))
    .into_response())
}

/// DELETE /api/admin/system/sessions/:id — Revoke a specific session.
pub async fn api_admin_system_session_revoke(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let result = sqlx::query("DELETE FROM user_sessions WHERE id::text = $1")
        .bind(&session_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                let _ = sqlx::query(
                    r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                       VALUES ($1, 'admin.session_revoke', 'user_sessions', $2)"#,
                )
                .bind(admin.user.id)
                .bind(serde_json::json!({"session_id": &session_id}))
                .execute(&state.db)
                .await;

                Ok(
                    Json(serde_json::json!({"status":"success","message":"Session revoked"}))
                        .into_response(),
                )
            } else {
                Err(ApiError::NotFound("Session not found".to_string()))
            }
        }
        Err(e) => {
            tracing::error!("Failed to revoke session: {e}");
            Err(ApiError::Internal("Failed to revoke session".to_string()))
        }
    }
}

/// POST /api/admin/system/sessions/bulk-revoke
pub async fn api_admin_system_sessions_bulk_revoke(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let ip_pattern = body
        .get("ip_pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if ip_pattern.is_empty() {
        return Err(ApiError::BadRequest("IP pattern is required".to_string()));
    }

    let like_pattern = format!("{}%", ip_pattern);
    let result = sqlx::query("DELETE FROM user_sessions WHERE ip_address::text LIKE $1")
        .bind(&like_pattern)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                   VALUES ($1, 'admin.sessions_bulk_revoke', 'user_sessions', $2)"#,
            )
            .bind(admin.user.id)
            .bind(serde_json::json!({"ip_pattern": ip_pattern, "revoked": r.rows_affected()}))
            .execute(&state.db)
            .await;

            Ok(
                Json(serde_json::json!({"status":"success", "revoked": r.rows_affected()}))
                    .into_response(),
            )
        }
        Err(e) => {
            tracing::error!("Failed to bulk revoke sessions: {}", e);
            Err(ApiError::Internal(
                "Failed to bulk revoke sessions".to_string(),
            ))
        }
    }
}

/// GET /api/admin/system/jobs — List background jobs.
pub async fn api_admin_system_jobs(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let rows = sqlx::query(
        r#"SELECT id::text, job_name as name, status, attempts,
                  COALESCE(payload::text, '') as payload,
                  run_at::text, created_at::text
           FROM background_jobs
           ORDER BY created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let jobs: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "name": r.get::<String, _>("name"),
                "status": r.get::<String, _>("status"),
                "attempts": r.get::<i32, _>("attempts"),
                "payload": r.get::<String, _>("payload"),
                "run_at": r.get::<String, _>("run_at"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(jobs).into_response())
}

/// DELETE /api/admin/system/jobs/:id — Cancel a background job.
pub async fn api_admin_system_job_cancel(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let result = sqlx::query("UPDATE background_jobs SET status = 'cancelled' WHERE id::text = $1")
        .bind(&job_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                   VALUES ($1, 'admin.job_cancel', 'background_jobs', $2)"#,
            )
            .bind(admin.user.id)
            .bind(serde_json::json!({"job_id": &job_id}))
            .execute(&state.db)
            .await;

            Ok(Json(serde_json::json!({"status":"success"})).into_response())
        }
        Err(_) => Err(ApiError::Internal("Failed to cancel job".to_string())),
    }
}

/// POST /api/admin/system/jobs/:id/retry — Retry a background job.
pub async fn api_admin_system_job_retry(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let result = sqlx::query("UPDATE background_jobs SET status = 'pending', attempts = 0, run_at = NOW() WHERE id::text = $1")
        .bind(&job_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                   VALUES ($1, 'admin.job_retry', 'background_jobs', $2)"#,
            )
            .bind(admin.user.id)
            .bind(serde_json::json!({"job_id": &job_id}))
            .execute(&state.db)
            .await;

            Ok(Json(serde_json::json!({"status":"success"})).into_response())
        }
        Err(_) => Err(ApiError::Internal("Failed to retry job".to_string())),
    }
}

/// GET /api/admin/system/webhooks — List webhook logs.
pub async fn api_admin_system_webhooks(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let rows = sqlx::query(
        r#"SELECT id::text, provider, endpoint, http_status,
                  COALESCE(payload::text, '') as payload,
                  processed, created_at::text
           FROM webhook_logs
           ORDER BY created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let webhooks: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "provider": r.get::<String, _>("provider"),
                "endpoint": r.get::<String, _>("endpoint"),
                "http_status": r.get::<Option<i32>, _>("http_status"),
                "payload": r.get::<String, _>("payload"),
                "processed": r.get::<bool, _>("processed"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(webhooks).into_response())
}

/// POST /api/admin/system/webhooks/:id/replay — Replay a webhook log.
pub async fn api_admin_system_webhook_replay(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(webhook_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    let result = sqlx::query("UPDATE webhook_logs SET processed = false WHERE id::text = $1")
        .bind(&webhook_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
                   VALUES ($1, 'admin.webhook_replay', 'webhook_logs', $2)"#,
            )
            .bind(admin.user.id)
            .bind(serde_json::json!({"webhook_id": &webhook_id}))
            .execute(&state.db)
            .await;

            Ok(Json(serde_json::json!({"status":"success"})).into_response())
        }
        Err(_) => Err(ApiError::Internal("Failed to replay webhook".to_string())),
    }
}

/// GET /api/admin/system/password-resets — List recent password reset requests.
pub async fn api_admin_system_password_resets(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "platform.manage")
        .await?;

    // Fetch password reset audit entries from audit_logs
    let rows = sqlx::query(
        r#"SELECT al.id::text, al.actor_user_id::text AS user_id,
                  COALESCE(u.email,'') AS email,
                  al.action, COALESCE(al.ip_address::text,'') AS ip_address,
                  al.created_at::text
           FROM audit_logs al
           LEFT JOIN users u ON u.id = al.actor_user_id
           WHERE al.action IN ('password.reset_request', 'password.reset_complete', 'password.force_reset')
           ORDER BY al.created_at DESC
           LIMIT 100"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let resets: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<Option<String>, _>("user_id"),
                "email": r.get::<String, _>("email"),
                "action": r.get::<String, _>("action"),
                "ip_address": r.get::<String, _>("ip_address"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "resets": resets,
        "total": resets.len()
    }))
    .into_response())
}
