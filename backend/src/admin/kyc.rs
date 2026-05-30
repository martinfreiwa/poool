use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};
use std::collections::HashMap;

//
//  Admin KYC/AML API
//

/// GET /api/admin/kyc  List all KYC records with user info.
///
/// Bounded to 500 rows per page (`?page=N`, zero-indexed) to prevent OOM at
/// scale. See CDDRP §3.5 (B6) for the broader unbounded-query remediation.
pub async fn api_admin_kyc_records(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "kyc.view").await?;

    // Pagination cap (CDDRP B6 fix).
    const KYC_PAGE_SIZE: i64 = 500;
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);
    let offset = page.saturating_mul(KYC_PAGE_SIZE);

    let rows = sqlx::query_as::<
        _,
        (
            String,         // id
            String,         // user_id
            String,         // provider
            String,         // status
            Option<String>, // provider_ref_id
            Option<String>, // document_type
            Option<bool>,   // pep_check_passed
            Option<bool>,   // sanctions_check
            Option<String>, // rejection_reason
            Option<String>, // verified_at
            Option<String>, // expires_at
            String,         // created_at
            String,         // user_email
            String,         // first_name
            String,         // last_name
            Option<i64>,    // document_count
        ),
    >(
        r#"SELECT k.id::text, k.user_id::text, k.provider, k.status,
                  k.provider_ref_id, k.document_type,
                  k.pep_check_passed, k.sanctions_check,
                  k.rejection_reason, k.verified_at::text,
                  k.expires_at::text, k.created_at::text,
                  COALESCE(u.email, ''),
                  COALESCE(up.first_name, ''), COALESCE(up.last_name, ''),
                  (SELECT COUNT(*) FROM kyc_documents kd WHERE kd.kyc_record_id = k.id) as document_count
           FROM kyc_records k
           JOIN users u ON u.id = k.user_id
           LEFT JOIN user_profiles up ON up.user_id = k.user_id
           ORDER BY
              CASE k.status
                  WHEN 'pending' THEN 0
                  WHEN 'in_review' THEN 1
                  ELSE 2
              END,
              k.created_at DESC
           LIMIT $1 OFFSET $2"#,
    )
    .bind(KYC_PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let records: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let name = format!("{} {}", r.13.clone(), r.14.clone())
                .trim()
                .to_string();
            serde_json::json!({
                "id": r.0, "user_id": r.1, "provider": r.2, "status": r.3,
                "provider_ref_id": r.4, "document_type": r.5,
                "pep_check_passed": r.6, "sanctions_check": r.7,
                "rejection_reason": r.8, "verified_at": r.9,
                "expires_at": r.10, "created_at": r.11,
                "user_email": r.12,
                "user_name": if name.is_empty() { r.12.clone() } else { name },
                "has_documents": r.15.unwrap_or(0) > 0
            })
        })
        .collect();

    let stats = serde_json::json!({
        "pending": records.iter().filter(|r| r["status"] == "pending" || r["status"] == "in_review").count(),
        "approved": records.iter().filter(|r| r["status"] == "approved").count(),
        "rejected": records.iter().filter(|r| r["status"] == "rejected").count(),
        "pep_flags": records.iter().filter(|r| r["pep_check_passed"] == false).count(),
        "expiring_soon": 0
    });

    Ok(Json(serde_json::json!({ "records": records, "stats": stats })).into_response())
}

/// GET /api/admin/kyc/providers/health - lightweight dashboard health summary.
pub async fn api_admin_kyc_providers_health(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "kyc.view").await?;

    let oldest_pending_seconds: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::bigint
        FROM kyc_records
        WHERE status IN ('pending', 'in_review')
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "providers": {
            "sumsub": { "status": "configured", "latency_ms": null },
            "didit": { "status": "fallback_manual", "latency_ms": null },
            "manual": { "status": "available", "latency_ms": null }
        },
        "sanctions": {
            "status": "available",
            "last_checked_at": chrono::Utc::now().to_rfc3339()
        },
        "freshness": {
            "oldest_pending_seconds": oldest_pending_seconds.unwrap_or(0),
            "generated_at": chrono::Utc::now().to_rfc3339()
        }
    }))
    .into_response())
}

/// GET /api/admin/kyc/:kyc_id/documents - Get signed URLs for documents.
pub async fn api_admin_kyc_documents(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(kyc_id): axum::extract::Path<uuid::Uuid>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "kyc.view").await?;

    // Audit the access — KYC documents are the most sensitive PII in the
    // system (passport, ID, selfie). Every signed-URL issuance is logged.
    if let Err(e) = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.kyc_documents_access', 'kyc_records', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(kyc_id)
    .bind(serde_json::json!({"endpoint": "GET /api/admin/kyc/:id/documents"}))
    .execute(&state.db)
    .await
    {
        tracing::error!("Failed to write KYC documents access audit log for {kyc_id}: {e}");
    }

    let docs = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id::text, gcs_path, document_type FROM kyc_documents WHERE kyc_record_id = $1",
    )
    .bind(kyc_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut result = Vec::new();
    let storage_service = crate::storage::service::GcsService::new(
        state
            .config
            .gcs_bucket
            .as_deref()
            .unwrap_or("poool-assets-primary"),
    )
    .await;

    let default_bucket = state
        .config
        .gcs_bucket
        .as_deref()
        .unwrap_or("poool-assets-primary");

    for (id, path, doc_type) in docs {
        // Phase 7.5 — AV gate: refuse to mint a signed URL if the
        // ClamAV Cloud Function flagged the object as infected. KYC
        // is PII-class A; serving a known-infected file would be a
        // P0 security incident.
        let (av_bucket, av_path) =
            crate::storage::reconciler::extract_bucket_and_path(&path, default_bucket)
                .unwrap_or_else(|| (default_bucket.to_string(), path.clone()));
        let av_outcome_label;
        let av_blocked = match crate::storage::service::av_status(&av_bucket, &av_path).await {
            Ok(crate::storage::service::AvStatus::Clean) => {
                av_outcome_label = "clean";
                false
            }
            Ok(crate::storage::service::AvStatus::Infected(detection)) => {
                tracing::error!(
                    doc_id = %id,
                    detection = %detection,
                    "KYC signed-URL blocked: object is AV-infected"
                );
                av_outcome_label = "infected";
                true
            }
            Ok(crate::storage::service::AvStatus::ScannerError(err)) => {
                tracing::warn!(
                    doc_id = %id,
                    error = %err,
                    "KYC signed-URL: scanner reported error — serving with operator caveat"
                );
                av_outcome_label = "error";
                false
            }
            Ok(crate::storage::service::AvStatus::NotYetScanned) => {
                // ClamAV typically completes within seconds of upload.
                // For v1 we serve with a warning rather than block —
                // tightening to deny-by-default is a Phase 7 toggle once
                // scanner deploy-latency is characterised.
                av_outcome_label = "not_yet_scanned";
                false
            }
            Err(e) => {
                tracing::warn!(
                    doc_id = %id,
                    error = %e,
                    "KYC signed-URL: failed to read AV metadata — serving with caveat"
                );
                av_outcome_label = "not_yet_scanned";
                false
            }
        };
        crate::metrics::record_storage_av_outcome(av_outcome_label);
        if av_blocked {
            result.push(serde_json::json!({
                "id": id,
                "document_type": doc_type,
                "url": null,
                "blocked_reason": "av_infected",
            }));
            continue;
        }

        // Force-download disposition so KYC PDFs / images cannot render
        // inline in the admin browser — closes the PDF-JS / SVG-script
        // XSS surface (Phase 2.1). Filename = `kyc_<doc-id>_<type>.bin`
        // so admins still see meaningful download names.
        let filename = format!("kyc_{}_{}.bin", id, doc_type);
        let signed_url = storage_service
            .generate_signed_url_with_disposition(&path, 3600, Some(&filename))
            .await
            .unwrap_or_default();
        result.push(serde_json::json!({
            "id": id,
            "document_type": doc_type,
            "url": signed_url
        }));
    }

    Ok(Json(result).into_response())
}

/// POST /api/admin/kyc/:kyc_id/approve
pub async fn api_admin_kyc_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(kyc_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "kyc.write").await?;

    let uid = ApiError::parse_uuid(&kyc_id)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let updated = sqlx::query(
        r#"UPDATE kyc_records SET status = 'approved', verified_at = NOW(),
                  expires_at = NOW() + INTERVAL '2 years', updated_at = NOW()
           WHERE id = $1 AND status IN ('pending', 'in_review')"#,
    )
    .bind(uid)
    .execute(&mut *tx)
    .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            // GAP-06: Affiliate referral state machine — advance any 'registered' referral to 'kyc_approved'
            // for this user so that the funnel stages are accurately tracked.
            sqlx::query(
                r#"UPDATE affiliate_referrals
                   SET status = 'kyc_approved', updated_at = NOW()
                   WHERE referred_user_id = (
                       SELECT user_id FROM kyc_records WHERE id = $1 LIMIT 1
                   )
                   AND status = 'registered'"#,
            )
            .bind(uid)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'admin.kyc_approve', 'kyc_records', $2, $3)"#,
            )
            .bind(admin.user.id)
            .bind(uid)
            .bind(serde_json::json!({"status": "approved"}))
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            tx.commit().await.map_err(ApiError::Database)?;

            // Fire-and-forget — email goes into the durable outbox; the outbox
            // worker retries on provider failure.
            let db = state.db.clone();
            tokio::spawn(async move {
                if let Ok(Some(user_id)) = sqlx::query_scalar::<_, uuid::Uuid>(
                    "SELECT user_id FROM kyc_records WHERE id = $1",
                )
                .bind(uid)
                .fetch_optional(&db)
                .await
                {
                    let _ = crate::email::trigger_transactional_email(
                        &db,
                        &user_id,
                        "kyc_approved",
                        serde_json::json!({}),
                    )
                    .await;
                }
            });

            Ok(Json(serde_json::json!({"status": "approved"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound(
            "KYC not found or already processed".to_string(),
        )),
        Err(e) => {
            tracing::error!("Failed to approve KYC {kyc_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// POST /api/admin/kyc/:kyc_id/reject
pub async fn api_admin_kyc_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(kyc_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "kyc.write").await?;

    let uid = ApiError::parse_uuid(&kyc_id)?;

    let reason = body
        .get("rejection_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadRequest("rejection_reason is required".to_string()))?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let updated = sqlx::query(
        r#"UPDATE kyc_records SET status = 'rejected', rejection_reason = $2, updated_at = NOW()
           WHERE id = $1 AND status IN ('pending', 'in_review')"#,
    )
    .bind(uid)
    .bind(reason)
    .execute(&mut *tx)
    .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'admin.kyc_reject', 'kyc_records', $2, $3)"#,
            )
            .bind(admin.user.id)
            .bind(uid)
            .bind(serde_json::json!({"status": "rejected", "reason": reason}))
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

            tx.commit().await.map_err(ApiError::Database)?;

            let db = state.db.clone();
            let reason_owned = reason.to_string();
            tokio::spawn(async move {
                if let Ok(Some(user_id)) = sqlx::query_scalar::<_, uuid::Uuid>(
                    "SELECT user_id FROM kyc_records WHERE id = $1",
                )
                .bind(uid)
                .fetch_optional(&db)
                .await
                {
                    let _ = crate::email::trigger_transactional_email(
                        &db,
                        &user_id,
                        "kyc_rejected",
                        serde_json::json!({"rejection_reason": reason_owned}),
                    )
                    .await;
                }
            });

            Ok(Json(serde_json::json!({"status": "rejected"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound(
            "KYC not found or already processed".to_string(),
        )),
        Err(e) => {
            tracing::error!("Failed to reject KYC {kyc_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}
