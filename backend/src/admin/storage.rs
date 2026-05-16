use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::storage::reconciler::{run_reconciliation, SourceTable};
use crate::storage::retention::{arm_retention_for_user, run_retention_worker};
use axum::{
    extract::{Json, Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

/// GET /api/admin/storage — GCS storage analytics.
///
/// Aggregates document metadata from the database to produce storage
/// analytics: total files, estimated bytes, per-category breakdown,
/// and monthly cost estimates using GCS Standard Storage pricing ($0.020/GB).
pub async fn api_admin_storage(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // ── Aggregate counts & estimated sizes ──────────────

    // 1. KYC Documents
    let kyc_stats = sqlx::query_as::<_, (String, i64)>(
        "SELECT document_type, COUNT(*) FROM kyc_documents GROUP BY document_type",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let doc_status_stats = sqlx::query_as::<_, (String, i64)>(
        "SELECT status, COUNT(*) FROM kyc_documents GROUP BY status",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // 2. Asset Documents
    let asset_doc_stats = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT document_type, COUNT(*), COALESCE(SUM(file_size_bytes), 0)::bigint FROM asset_documents GROUP BY document_type"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // 3. Asset Images
    let total_asset_images: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM asset_images")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    // 4. Users with avatars
    let users_with_avatars: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE avatar_url IS NOT NULL AND avatar_url <> ''",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let kyc_docs_count: i64 = kyc_stats.iter().map(|r| r.1).sum();
    let asset_docs_count: i64 = asset_doc_stats.iter().map(|r| r.1).sum();

    let total_files = kyc_docs_count + asset_docs_count + total_asset_images + users_with_avatars;

    let avg_kyc_bytes: i64 = 350_000;
    let avg_asset_image_bytes: i64 = 600_000;
    let avg_avatar_bytes: i64 = 80_000;

    let mut estimated_asset_docs_bytes: i64 = 0;
    for stat in &asset_doc_stats {
        if stat.2 == 0 {
            estimated_asset_docs_bytes += stat.1 * 500_000;
        } else {
            estimated_asset_docs_bytes += stat.2;
        }
    }

    let estimated_kyc_bytes = kyc_docs_count * avg_kyc_bytes;
    let estimated_asset_images_bytes = total_asset_images * avg_asset_image_bytes;
    let estimated_avatar_bytes = users_with_avatars * avg_avatar_bytes;

    let total_estimated_bytes = estimated_kyc_bytes
        + estimated_asset_docs_bytes
        + estimated_asset_images_bytes
        + estimated_avatar_bytes;

    let total_estimated_gb = total_estimated_bytes as f64 / 1_073_741_824.0;

    // GCS Standard Storage pricing: $0.020 per GB/month (multi-region US)
    // Operations: Class A $0.05/10k (write), Class B $0.004/10k (read)
    let storage_cost_per_month = total_estimated_gb * 0.020;
    let class_a_ops_cost = (total_files as f64 / 10_000.0) * 0.05;
    let class_b_ops_cost = (total_files as f64 * 3.0 / 10_000.0) * 0.004; // 3 reads per doc on avg
    let total_cost_per_month = storage_cost_per_month + class_a_ops_cost + class_b_ops_cost;

    // ── Breakdown by type ───────────────────────────────────────────────────
    let mut breakdown_by_type = Vec::new();
    for r in &kyc_stats {
        breakdown_by_type.push(serde_json::json!({
            "type": r.0,
            "count": r.1,
            "estimated_mb": (r.1 * avg_kyc_bytes / 1_048_576)
        }));
    }
    for r in &asset_doc_stats {
        let bytes = if r.2 == 0 { r.1 * 500_000 } else { r.2 };
        breakdown_by_type.push(serde_json::json!({
            "type": format!("asset_{}", r.0),
            "count": r.1,
            "estimated_mb": (bytes / 1_048_576)
        }));
    }
    if total_asset_images > 0 {
        breakdown_by_type.push(serde_json::json!({
            "type": "asset_image",
            "count": total_asset_images,
            "estimated_mb": (estimated_asset_images_bytes / 1_048_576)
        }));
    }
    if users_with_avatars > 0 {
        breakdown_by_type.push(serde_json::json!({
            "type": "user_avatar",
            "count": users_with_avatars,
            "estimated_mb": (estimated_avatar_bytes / 1_048_576)
        }));
    }
    breakdown_by_type.sort_by(|a, b| {
        let count_a = a["count"].as_i64().unwrap_or(0);
        let count_b = b["count"].as_i64().unwrap_or(0);
        count_b.cmp(&count_a)
    });

    // ── Recent uploads (last 10) ────────────────────────────────────────────
    let recent_uploads = sqlx::query_as::<_, (String, String, String, String)>(
        r#"
        SELECT id::text, document_type, status, user_email, uploaded_at FROM (
            SELECT kd.id, kd.document_type, kd.status, COALESCE(u.email, 'unknown') as user_email, kd.uploaded_at
            FROM kyc_documents kd
            JOIN users u ON u.id = kd.user_id
            UNION ALL
            SELECT ad.id, 'asset_' || ad.document_type as document_type, 'approved' as status, COALESCE(u.email, 'unknown') as user_email, ad.created_at as uploaded_at
            FROM asset_documents ad
            JOIN assets a ON a.id = ad.asset_id
            LEFT JOIN users u ON u.id = a.developer_user_id
            UNION ALL
            SELECT ai.id, 'asset_image' as document_type, 'approved' as status, COALESCE(u.email, 'unknown') as user_email, ai.created_at as uploaded_at
            FROM asset_images ai
            JOIN assets a ON a.id = ai.asset_id
            LEFT JOIN users u ON u.id = a.developer_user_id
        ) combined
        ORDER BY uploaded_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let recent: Vec<serde_json::Value> = recent_uploads
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0, "document_type": r.1, "status": r.2, "user_email": r.3
            })
        })
        .collect();

    // ── Monthly upload trend (last 6 months) ───────────────────────────────
    // Using generate_series guarantees exactly 6 months are returned,
    // which prevents the CSS `flex: 1` bar from stretching over the whole width if only 1 month exists.
    let monthly_trend = sqlx::query_as::<_, (String, i64)>(
        r#"
        WITH months AS (
            SELECT TO_CHAR(series.month, 'Mon YYYY') as month_label, series.month as month_date
            FROM generate_series(DATE_TRUNC('month', NOW() - INTERVAL '5 months'), DATE_TRUNC('month', NOW()), '1 month') series(month)
        ),
        uploads AS (
            SELECT DATE_TRUNC('month', uploaded_at) as m, COUNT(id) as c FROM kyc_documents GROUP BY 1
            UNION ALL
            SELECT DATE_TRUNC('month', created_at) as m, COUNT(id) as c FROM asset_documents GROUP BY 1
            UNION ALL
            SELECT DATE_TRUNC('month', created_at) as m, COUNT(id) as c FROM asset_images GROUP BY 1
        )
        SELECT m.month_label, COALESCE(SUM(u.c), 0)::bigint as uploads
        FROM months m
        LEFT JOIN uploads u ON m.month_date = u.m
        GROUP BY m.month_label, m.month_date
        ORDER BY m.month_date
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let trend: Vec<serde_json::Value> = monthly_trend
        .iter()
        .map(|r| serde_json::json!({"month": r.0, "uploads": r.1}))
        .collect();

    Ok(Json(serde_json::json!({
        "bucket": state.config.gcs_bucket.as_deref().unwrap_or("not configured"),
        "summary": {
            "total_files": total_files,
            "kyc_documents": kyc_docs_count,
            "asset_documents": asset_docs_count,
            "asset_images": total_asset_images,
            "avatars": users_with_avatars,
            "estimated_storage_bytes": total_estimated_bytes,
            "estimated_storage_gb": (total_estimated_gb * 1000.0).round() / 1000.0,
        },
        "cost_estimate": {
            "storage_per_month_usd": (storage_cost_per_month * 10000.0).round() / 10000.0,
            "operations_per_month_usd": ((class_a_ops_cost + class_b_ops_cost) * 10000.0).round() / 10000.0,
            "total_per_month_usd": (total_cost_per_month * 10000.0).round() / 10000.0,
            "pricing_note": "Estimates based on GCS Standard Storage (US multi-region, $0.020/GB/month). Actual costs from Google Cloud Console billing.",
        },
        "breakdown_by_type": breakdown_by_type,
        "breakdown_by_status": doc_status_stats.iter().map(|r| serde_json::json!({
            "status": r.0,
            "count": r.1
        })).collect::<Vec<_>>(),
        "recent_uploads": recent,
        "monthly_trend": trend,
    }))
    .into_response())
}

// ─── Reconciler admin endpoint (Phase 3.3) ─────────────────────────
//
// POST /api/admin/storage/reconcile?source=kyc|assets
//
// Triggers a DB↔GCS drift scan over the requested source table. Writes
// audit rows into storage_reconcile_runs + storage_reconcile_findings
// (migration 199) and returns the run summary. Admin-only.
//
// Designed to be invoked both interactively (operator clicks Refresh)
// and by Cloud Scheduler nightly. The scheduler request must carry an
// admin-token; the route does NOT support an unauth bypass.

/// Query parameters for the storage-reconciliation admin endpoint.
#[derive(Debug, Deserialize)]
pub struct ReconcileQuery {
    /// Which table to scan. Accepts "kyc" or "assets".
    pub source: String,
    /// Optional operator note attached to the run row for triage.
    pub note: Option<String>,
}

/// POST /admin/api/storage/reconcile?source=kyc|assets — manual trigger
/// for the storage reconciliation pass. Also invoked nightly by Cloud
/// Scheduler with an admin token.
pub async fn api_admin_storage_reconcile(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(q): Query<ReconcileQuery>,
) -> Result<axum::response::Response, ApiError> {
    let source = match q.source.as_str() {
        "kyc" => SourceTable::KycDocuments,
        "assets" | "asset_documents" => SourceTable::AssetDocuments,
        other => {
            return Err(ApiError::BadRequest(format!(
                "unknown source '{}'; expected 'kyc' or 'assets'",
                other
            )));
        }
    };

    let default_bucket = state
        .config
        .gcs_bucket
        .as_deref()
        .ok_or_else(|| ApiError::Internal("GCS_BUCKET not configured".to_string()))?
        .to_string();

    let summary = run_reconciliation(&state.db, source, &default_bucket, q.note.as_deref())
        .await
        .map_err(|e| ApiError::Internal(format!("reconciler failed: {}", e)))?;

    Ok(Json(serde_json::to_value(&summary).unwrap_or_default()).into_response())
}

// ─── Retention worker admin endpoint (Phase 4.1) ───────────────────
//
// POST /api/admin/storage/retention/run?dry_run=true|false
//
// Triggers the GwG §8 retention worker. dry_run=true returns the same
// summary without touching either GCS or the DB — intended for the ops
// dashboard preview + scheduler canaries. dry_run=false is the actual
// nightly delete. Admin-only.

/// Query parameters for the retention-worker admin endpoint.
#[derive(Debug, Deserialize)]
pub struct RetentionQuery {
    /// When true (default), no objects/rows are touched — summary only.
    /// Set to false for a real delete pass.
    pub dry_run: Option<bool>,
    /// Optional operator note attached to the run row.
    pub note: Option<String>,
}

/// POST /api/admin/storage/retention/run — invoke the KYC retention
/// worker (GwG §8 deletions). Admin-only. See
/// `docs/storage/04-compliance-and-retention.md` for the runbook.
pub async fn api_admin_storage_retention_run(
    _admin: AdminUser,
    State(state): State<AppState>,
    Query(q): Query<RetentionQuery>,
) -> Result<axum::response::Response, ApiError> {
    let default_bucket = state
        .config
        .gcs_bucket
        .as_deref()
        .ok_or_else(|| ApiError::Internal("GCS_BUCKET not configured".to_string()))?
        .to_string();

    let dry_run = q.dry_run.unwrap_or(true);

    let summary = run_retention_worker(&state.db, &default_bucket, dry_run, q.note.as_deref())
        .await
        .map_err(|e| ApiError::Internal(format!("retention worker failed: {}", e)))?;

    Ok(Json(serde_json::to_value(&summary).unwrap_or_default()).into_response())
}

// ─── DSGVO trigger: arm retention on a single user (Phase 4.4) ─────

/// POST /api/admin/storage/retention/arm/:user_id?years=5
///
/// Admin endpoint that arms the retention clock for a user. Called by
/// the DSGVO user-delete flow + the admin off-boarding flow. Idempotent
/// — repeated calls don't move the trigger date.

#[derive(Debug, Deserialize)]
/// Query parameters for the DSGVO retention-arming admin endpoint.
pub struct ArmRetentionQuery {
    /// Retention window in years (default 5; max 10 per GwG §8).
    pub years: Option<i32>,
}

/// POST /api/admin/storage/retention/arm/:user_id — set the retention
/// clock for the given user's KYC docs. Idempotent.
pub async fn api_admin_storage_retention_arm(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(q): Query<ArmRetentionQuery>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&user_id)?;
    let years = q.years.unwrap_or(5).clamp(5, 10);

    let n = arm_retention_for_user(&state.db, uid, years)
        .await
        .map_err(|e| ApiError::Internal(format!("arm_retention failed: {}", e)))?;

    Ok(Json(serde_json::json!({
        "user_id": uid,
        "retention_years": years,
        "kyc_docs_armed": n,
    }))
    .into_response())
}
