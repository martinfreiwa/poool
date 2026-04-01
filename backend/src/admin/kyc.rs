use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

//
//  Admin KYC/AML API
//

/// GET /api/admin/kyc  List all KYC records with user info.
pub async fn api_admin_kyc_records(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            String,         // id
            String,         // user_id
            String,         // provider
            String,         // status
            Option<String>, // provider_ref_id
            String,         // document_type
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
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

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

/// GET /api/admin/kyc/:kyc_id/documents - Get signed URLs for documents.
pub async fn api_admin_kyc_documents(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(kyc_id): axum::extract::Path<uuid::Uuid>,
) -> Result<axum::response::Response, ApiError> {
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

    for (id, path, doc_type) in docs {
        let signed_url = storage_service
            .generate_signed_url(&path, 3600)
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
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(kyc_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&kyc_id)?;

    let updated = sqlx::query(
        r#"UPDATE kyc_records SET status = 'approved', verified_at = NOW(),
                  expires_at = NOW() + INTERVAL '2 years', updated_at = NOW()
           WHERE id = $1 AND status IN ('pending', 'in_review')"#,
    )
    .bind(uid)
    .execute(&state.db)
    .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            // GAP-06: Affiliate referral state machine — advance any 'registered' referral to 'kyc_approved'
            // for this user so that the funnel stages are accurately tracked.
            let _ = sqlx::query(
                r#"UPDATE affiliate_referrals
                   SET status = 'kyc_approved', updated_at = NOW()
                   WHERE referred_user_id = (
                       SELECT user_id FROM kyc_records WHERE id = $1 LIMIT 1
                   )
                   AND status = 'registered'"#
            )
            .bind(uid)
            .execute(&state.db)
            .await;

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
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(kyc_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&kyc_id)?;

    let reason = body
        .get("rejection_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("No reason provided");

    let updated = sqlx::query(
        r#"UPDATE kyc_records SET status = 'rejected', rejection_reason = $2, updated_at = NOW()
           WHERE id = $1 AND status IN ('pending', 'in_review')"#,
    )
    .bind(uid)
    .bind(reason)
    .execute(&state.db)
    .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
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
