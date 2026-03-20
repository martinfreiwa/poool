use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};
use std::collections::HashMap;

//
//  Admin KYC/AML API — Server-Side Pagination
//

/// GET /api/admin/kyc?page=1&page_size=20&tab=queue&search=&status=&sort=created_at&order=desc
///
/// Query parameters:
///   - `tab`:       queue | approved | rejected | pep | expiring | all (default: queue)
///   - `page`:      Page number, 1-indexed (default: 1)
///   - `page_size`:  Records per page, max 100 (default: 20)
///   - `search`:    Filter by user name or email (partial match)
///   - `status`:    Additional status filter (only when tab allows it)
///   - `sort`:      Sort field: created_at | user_name | provider | status | verified_at | expires_at
///   - `order`:     asc | desc (default: desc)
pub async fn api_admin_kyc_records(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    // ── Parse query parameters ────────────────────────────────────
    let tab = params.get("tab").map(|s| s.as_str()).unwrap_or("queue");
    let page: i64 = params
        .get("page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    let page_size: i64 = params
        .get("page_size")
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let search = params
        .get("search")
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();
    let status_filter = params.get("status").cloned().unwrap_or_default();
    let sort_field = params.get("sort").map(|s| s.as_str()).unwrap_or("created_at");
    let sort_order = params.get("order").map(|s| s.as_str()).unwrap_or("desc");

    let offset = (page - 1) * page_size;

    // ── Build WHERE clauses based on tab ─────────────────────────
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    match tab {
        "queue" => {
            where_clauses.push("k.status IN ('pending', 'in_review')".to_string());
        }
        "approved" => {
            where_clauses.push("k.status = 'approved'".to_string());
        }
        "rejected" => {
            where_clauses.push("k.status = 'rejected'".to_string());
        }
        "pep" => {
            where_clauses.push("k.pep_check_passed = false".to_string());
        }
        "expiring" => {
            where_clauses.push(
                "k.expires_at IS NOT NULL AND k.expires_at > NOW() AND k.expires_at < NOW() + INTERVAL '30 days'"
                    .to_string(),
            );
        }
        _ => {
            // "all" — no tab-level filter
        }
    }

    // Additional status dropdown filter (only for tabs that allow it)
    if !status_filter.is_empty()
        && matches!(tab, "all" | "pep" | "expiring")
        && matches!(
            status_filter.as_str(),
            "pending" | "in_review" | "approved" | "rejected" | "expired"
        )
    {
        bind_values.push(status_filter.clone());
        where_clauses.push(format!("k.status = ${}", bind_values.len()));
    }

    // Search filter
    if !search.is_empty() {
        bind_values.push(format!("%{search}%"));
        let idx = bind_values.len();
        where_clauses.push(format!(
            "(LOWER(COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, '')) LIKE ${idx} OR LOWER(u.email) LIKE ${idx})"
        ));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // ── Build ORDER BY ──────────────────────────────────────────
    let direction = if sort_order == "asc" { "ASC" } else { "DESC" };
    let order_sql = match sort_field {
        "user_name" => format!(
            "COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, '') {direction}, k.created_at DESC"
        ),
        "provider" => format!("k.provider {direction}, k.created_at DESC"),
        "status" => format!("k.status {direction}, k.created_at DESC"),
        "verified_at" => format!("k.verified_at {direction} NULLS LAST, k.created_at DESC"),
        "expires_at" => format!("k.expires_at {direction} NULLS LAST, k.created_at DESC"),
        "submitted_at" | "created_at" | _ => {
            if tab == "queue" {
                // Queue always sorts pending first
                format!(
                    "CASE k.status WHEN 'pending' THEN 0 WHEN 'in_review' THEN 1 ELSE 2 END, k.created_at {direction}"
                )
            } else {
                format!("k.created_at {direction}")
            }
        }
    };

    // ── Count query (for pagination metadata) ───────────────────
    let count_sql = format!(
        "SELECT COUNT(*) as cnt FROM kyc_records k \
         JOIN users u ON u.id = k.user_id \
         LEFT JOIN user_profiles up ON up.user_id = k.user_id \
         {where_sql}"
    );

    // Build the count query dynamically
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }

    let total_count: i64 = count_query
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count KYC records: {e}");
            ApiError::Internal("Failed to count KYC records".to_string())
        })?;

    let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i64;

    // ── Data query (paginated) ──────────────────────────────────
    let data_sql = format!(
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
           {where_sql}
           ORDER BY {order_sql}
           LIMIT {page_size} OFFSET {offset}"#,
    );

    let mut data_query = sqlx::query_as::<
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
    >(&data_sql);

    for val in &bind_values {
        data_query = data_query.bind(val);
    }

    let rows = data_query
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch KYC records: {e}");
            ApiError::Internal("Failed to load KYC records".to_string())
        })?;

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

    // ── Stats (efficient COUNT queries — always over full dataset) ──
    let stats = fetch_kyc_stats(&state.db).await;

    Ok(Json(serde_json::json!({
        "records": records,
        "stats": stats,
        "pagination": {
            "page": page,
            "page_size": page_size,
            "total_count": total_count,
            "total_pages": total_pages.max(1),
        }
    }))
    .into_response())
}

/// Fetch KYC stats using efficient COUNT queries against the full dataset.
async fn fetch_kyc_stats(db: &sqlx::PgPool) -> serde_json::Value {
    // Single query with conditional aggregation — one round-trip
    let stats_row = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        r#"SELECT
            COUNT(*) FILTER (WHERE status IN ('pending', 'in_review')) as pending,
            COUNT(*) FILTER (WHERE status = 'approved') as approved,
            COUNT(*) FILTER (WHERE status = 'rejected') as rejected,
            COUNT(*) FILTER (WHERE pep_check_passed = false) as pep_flags,
            COUNT(*) FILTER (WHERE expires_at IS NOT NULL
                              AND expires_at > NOW()
                              AND expires_at < NOW() + INTERVAL '30 days') as expiring_soon
         FROM kyc_records"#,
    )
    .fetch_one(db)
    .await;

    match stats_row {
        Ok((pending, approved, rejected, pep_flags, expiring_soon)) => {
            serde_json::json!({
                "pending": pending,
                "approved": approved,
                "rejected": rejected,
                "pep_flags": pep_flags,
                "expiring_soon": expiring_soon,
            })
        }
        Err(e) => {
            tracing::error!("Failed to fetch KYC stats: {e}");
            serde_json::json!({
                "pending": 0, "approved": 0, "rejected": 0,
                "pep_flags": 0, "expiring_soon": 0
            })
        }
    }
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
    .map_err(|e| {
        tracing::error!("Failed to fetch KYC documents for {kyc_id}: {e}");
        ApiError::Internal("Failed to load KYC documents".to_string())
    })?;

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
