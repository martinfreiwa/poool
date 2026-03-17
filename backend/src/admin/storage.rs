use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

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
