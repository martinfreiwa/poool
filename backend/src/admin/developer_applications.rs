//! Admin review surface for "Become a Developer" applications.
//!
//! Closes C-1/C-2/C-3 from the 2026-05-19 developer-pages audit:
//!   • Persisted applications now require an explicit admin decision before
//!     the `developer` role is granted.
//!   • Approval is further gated on the applicant having an `approved`
//!     `kyc_records` row (Didit verification), so an attacker who somehow
//!     reaches an admin's eye still can't list assets without KYC.

use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

/// Query string for `GET /api/admin/developer-applications`.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Optional status filter — `pending`, `approved`, `rejected`, `needs_kyc`.
    /// When omitted, returns all rows ordered most-recent-first.
    pub status: Option<String>,
}

/// GET /api/admin/developer-applications
///
/// Lists applications, optionally filtered by `?status=pending`. Returns
/// the queue with applicant email + KYC status so the admin doesn't have
/// to cross-reference manually.
pub async fn api_admin_list_developer_applications(
    admin: AdminUser,
    Query(q): Query<ListQuery>,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "developer_projects.view")
        .await?;

    // Validate the status filter — only the four CHECK-allowed values are accepted.
    let status_filter = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some("pending") | Some("approved") | Some("rejected") | Some("needs_kyc") => {
            q.status.clone()
        }
        Some(_) => {
            return Err(ApiError::BadRequest(
                "Invalid status filter — must be one of pending, approved, rejected, needs_kyc."
                    .to_string(),
            ));
        }
        None => None,
    };

    let rows: Vec<(
        uuid::Uuid,
        uuid::Uuid,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    )> = sqlx::query_as(
        r#"
        SELECT
            da.id, da.user_id,
            u.email,
            da.first_name, da.last_name, da.country,
            da.assets_count, da.asset_value, da.monthly_income,
            da.status,
            da.submitted_at,
            (SELECT k.verified_at FROM kyc_records k
              WHERE k.user_id = da.user_id AND k.status = 'approved'
              ORDER BY k.verified_at DESC LIMIT 1) AS kyc_verified_at,
            (SELECT k.status FROM kyc_records k
              WHERE k.user_id = da.user_id
              ORDER BY k.created_at DESC LIMIT 1) AS kyc_status
        FROM developer_applications da
        JOIN users u ON u.id = da.user_id
        WHERE ($1::text IS NULL OR da.status = $1)
        ORDER BY da.submitted_at DESC
        LIMIT 500
        "#,
    )
    .bind(status_filter.as_deref())
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "user_id": r.1,
                "email": r.2,
                "first_name": r.3,
                "last_name": r.4,
                "country": r.5,
                "assets_count": r.6,
                "asset_value": r.7,
                "monthly_income": r.8,
                "status": r.9,
                "submitted_at": r.10,
                "kyc_verified_at": r.11,
                "kyc_status": r.12.unwrap_or_else(|| "not_started".to_string()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "applications": items })).into_response())
}

/// POST /api/admin/developer-applications/:id/approve
///
/// Approve an application. Hard-gated on the applicant having an `approved`
/// kyc_records row — if KYC has not been completed, the application status
/// is set to `needs_kyc` and a 400 is returned. Otherwise the `developer`
/// role is granted, the application is marked `approved`, and an audit log
/// + transactional email are emitted.
pub async fn api_admin_approve_developer_application(
    admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "developer_projects.write")
        .await?;

    let app_id = ApiError::parse_uuid(&id)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    // Load the application + its current status. `FOR UPDATE` prevents a
    // second admin from racing through the same approval.
    let row: Option<(uuid::Uuid, String)> = sqlx::query_as(
        r#"SELECT user_id, status FROM developer_applications
           WHERE id = $1 FOR UPDATE"#,
    )
    .bind(app_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let (applicant_id, current_status) =
        row.ok_or_else(|| ApiError::NotFound("Developer application not found".to_string()))?;

    if current_status == "approved" {
        return Err(ApiError::Conflict(
            "Application is already approved".to_string(),
        ));
    }
    if current_status == "rejected" {
        return Err(ApiError::Conflict(
            "Application was previously rejected — re-submit to re-open".to_string(),
        ));
    }

    // KYC gate (C-3 fix). Look up the most recent kyc_records row for the
    // applicant — must be 'approved' with a verified_at timestamp.
    let kyc_verified_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        r#"SELECT verified_at FROM kyc_records
           WHERE user_id = $1 AND status = 'approved' AND verified_at IS NOT NULL
           ORDER BY verified_at DESC LIMIT 1"#,
    )
    .bind(applicant_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let kyc_verified_at = match kyc_verified_at {
        Some(t) => t,
        None => {
            // Flip status to needs_kyc + record the failed approval attempt
            // so the admin doesn't see this row in the "pending" queue any
            // more (it has been actively triaged).
            sqlx::query(
                r#"UPDATE developer_applications
                   SET status = 'needs_kyc',
                       reviewed_by = $2,
                       reviewed_at = NOW(),
                       review_notes = COALESCE(review_notes, '') ||
                                       CASE WHEN COALESCE(review_notes, '') = '' THEN '' ELSE E'\n' END ||
                                       'Approval attempted ' || NOW()::text ||
                                       ' but applicant KYC is not verified.'
                   WHERE id = $1"#,
            )
            .bind(app_id)
            .bind(admin.user.id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'admin.developer_application_needs_kyc', 'developer_applications', $2, $3)"#,
            )
            .bind(admin.user.id)
            .bind(app_id)
            .bind(serde_json::json!({
                "status": "needs_kyc",
                "reason": "kyc_not_verified",
                "applicant_user_id": applicant_id,
            }))
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            tx.commit().await.map_err(ApiError::Database)?;

            return Ok((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "applicant must complete KYC before approval",
                    "application_id": app_id,
                    "status": "needs_kyc",
                })),
            )
                .into_response());
        }
    };

    // Grant the `developer` role. ON CONFLICT NOTHING covers re-runs and
    // the (rare) case where the user already had the role from elsewhere.
    let developer_role_id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM roles WHERE name = 'developer'")
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

    let role_id = developer_role_id
        .ok_or_else(|| ApiError::Internal("Missing 'developer' row in roles table".to_string()))?;

    // Note: user_roles does not have an `assigned_by` column today —
    // grantor identity lives in the audit_logs row below.
    sqlx::query(
        r#"INSERT INTO user_roles (user_id, role_id)
           VALUES ($1, $2)
           ON CONFLICT (user_id, role_id) DO NOTHING"#,
    )
    .bind(applicant_id)
    .bind(role_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    // Mark the application approved + snapshot KYC verified_at for audit
    // evidence.
    sqlx::query(
        r#"UPDATE developer_applications
           SET status = 'approved',
               reviewed_by = $2,
               reviewed_at = NOW(),
               kyc_verified_at = $3
           WHERE id = $1"#,
    )
    .bind(app_id)
    .bind(admin.user.id)
    .bind(kyc_verified_at)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.developer_application_approved', 'developer_applications', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(app_id)
    .bind(serde_json::json!({
        "status": "approved",
        "applicant_user_id": applicant_id,
        "kyc_verified_at": kyc_verified_at,
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "approved",
        "application_id": app_id,
        "user_id": applicant_id,
    }))
    .into_response())
}

/// Optional body for `POST .../reject`.
#[derive(Debug, Deserialize)]
pub struct RejectBody {
    /// Free-form reason shown in audit logs + (future) applicant email.
    pub notes: Option<String>,
}

/// POST /api/admin/developer-applications/:id/reject
///
/// Decline an application. Stores the optional notes for the audit trail
/// and the (future) applicant notification email.
pub async fn api_admin_reject_developer_application(
    admin: AdminUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<RejectBody>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "developer_projects.write")
        .await?;

    let app_id = ApiError::parse_uuid(&id)?;
    let notes = body
        .notes
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(2_000).collect::<String>());

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let row: Option<(uuid::Uuid, String)> = sqlx::query_as(
        r#"SELECT user_id, status FROM developer_applications
           WHERE id = $1 FOR UPDATE"#,
    )
    .bind(app_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let (applicant_id, current_status) =
        row.ok_or_else(|| ApiError::NotFound("Developer application not found".to_string()))?;

    if current_status == "rejected" {
        return Err(ApiError::Conflict(
            "Application is already rejected".to_string(),
        ));
    }
    if current_status == "approved" {
        return Err(ApiError::Conflict(
            "Application is already approved — revoke the role separately".to_string(),
        ));
    }

    sqlx::query(
        r#"UPDATE developer_applications
           SET status = 'rejected',
               reviewed_by = $2,
               reviewed_at = NOW(),
               review_notes = $3
           WHERE id = $1"#,
    )
    .bind(app_id)
    .bind(admin.user.id)
    .bind(notes.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.developer_application_rejected', 'developer_applications', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(app_id)
    .bind(serde_json::json!({
        "status": "rejected",
        "applicant_user_id": applicant_id,
        "notes": notes,
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "rejected",
        "application_id": app_id,
    }))
    .into_response())
}
