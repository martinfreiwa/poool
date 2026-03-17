use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Submissions API
//

/// GET /api/admin/submissions  List all assets (as "submissions").
pub async fn api_admin_submissions(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query(
        r#"SELECT a.id::text, a.title, a.slug, a.short_description, a.asset_type,
                  a.property_type, a.location_city,
                  a.total_value_cents, a.token_price_cents, a.tokens_total, a.tokens_available,
                  a.annual_yield_bps, a.bedrooms, a.bathrooms,
                  a.funding_status, a.featured, a.published,
                  a.created_at::text,
                  COALESCE(u.email, 'no-dev') AS dev_email,
                  COALESCE(up.first_name, '') AS dev_first,
                  COALESCE(up.last_name, '') AS dev_last
           FROM assets a
           LEFT JOIN users u ON u.id = a.developer_user_id
           LEFT JOIN user_profiles up ON up.user_id = a.developer_user_id
           ORDER BY a.created_at DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let submissions: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let first: String = r.get("dev_first");
            let last: String = r.get("dev_last");
            let dev_name = format!("{} {}", first, last).trim().to_string();
            let dev_email: String = r.get("dev_email");
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "title": r.get::<String, _>("title"),
                "slug": r.get::<String, _>("slug"),
                "short_description": r.get::<Option<String>, _>("short_description"),
                "asset_type": r.get::<String, _>("asset_type"),
                "property_type": r.get::<Option<String>, _>("property_type"),
                "location_city": r.get::<Option<String>, _>("location_city"),
                "total_value_cents": r.get::<i64, _>("total_value_cents"),
                "token_price_cents": r.get::<i64, _>("token_price_cents"),
                "tokens_total": r.get::<i32, _>("tokens_total"),
                "tokens_available": r.get::<i32, _>("tokens_available"),
                "annual_yield_bps": r.get::<Option<i32>, _>("annual_yield_bps"),
                "bedrooms": r.get::<Option<i32>, _>("bedrooms"),
                "bathrooms": r.get::<Option<i32>, _>("bathrooms"),
                "funding_status": r.get::<String, _>("funding_status"),
                "featured": r.get::<bool, _>("featured"),
                "published": r.get::<bool, _>("published"),
                "created_at": r.get::<String, _>("created_at"),
                "developer_email": &dev_email,
                "developer_name": if dev_name.is_empty() { dev_email.clone() } else { dev_name }
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "submissions": submissions })).into_response())
}

/// POST /api/admin/submissions/:asset_id/approve
pub async fn api_admin_submission_approve(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&asset_id)?;

    let result = sqlx::query(
        "UPDATE assets SET published = TRUE, funding_status = CASE WHEN funding_status IN ('funded', 'exited') THEN funding_status ELSE 'funding_open' END, updated_at = NOW() WHERE id = $1"
    )
    .bind(uid)
    .execute(&state.db)
    .await;

    if let Ok(r) = &result {
        if r.rows_affected() > 0 {
            // Sync developer_projects status
            let _ = sqlx::query(
                "UPDATE developer_projects SET status = 'live', updated_at = NOW() WHERE asset_id = $1"
            )
            .bind(uid)
            .execute(&state.db)
            .await;
        }
    }

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Ok(Json(serde_json::json!({"status":"approved"})).into_response())
        }
        Ok(_) => return Err(ApiError::NotFound("Asset not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to approve submission {asset_id}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

/// POST /api/admin/submissions/:asset_id/reject
pub async fn api_admin_submission_reject(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&asset_id)?;

    let _notes = body
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("Rejected by admin");

    let updated =
        sqlx::query("UPDATE assets SET published = FALSE, updated_at = NOW() WHERE id = $1")
            .bind(uid)
            .execute(&state.db)
            .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            Ok(Json(serde_json::json!({"status":"rejected"})).into_response())
        }
        Ok(_) => return Err(ApiError::NotFound("Asset not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to reject submission {asset_id}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

//
//  Admin Submission Detail & Asset Detail APIs
//

/// GET /api/admin/submissions/:asset_id/detail
pub async fn api_admin_submission_detail(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let aid = ApiError::parse_uuid(&asset_id)?;

    // Fetch asset using Row::get
    let row = sqlx::query(
        "SELECT a.title, COALESCE(a.asset_type,'') as asset_type, COALESCE(a.city,'') as city,
                COALESCE(a.country,'') as country, COALESCE(a.description,'') as description,
                a.slug, COALESCE(a.total_value_cents,0) as total_value_cents,
                COALESCE(a.token_price_cents,0) as token_price_cents,
                COALESCE(a.tokens_total,0) as tokens_total,
                COALESCE(a.tokens_available,0) as tokens_available,
                a.annual_yield_bps, a.bedrooms, a.bathrooms,
                a.construction_status, a.video_url, a.google_maps_url
         FROM assets a WHERE a.id = $1",
    )
    .bind(aid)
    .fetch_optional(&state.db)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            return Err(ApiError::NotFound("Not found".to_string()));
        }
    };

    // Related data
    let docs: Vec<(String, Option<i64>)> = sqlx::query_as(
        "SELECT document_type, file_size_bytes FROM asset_documents WHERE asset_id = $1",
    )
    .bind(aid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let images: Vec<(String, bool, i32)> = sqlx::query_as(
        "SELECT COALESCE(image_url,''), COALESCE(is_cover,false), COALESCE(sort_order,0) FROM asset_images WHERE asset_id = $1 ORDER BY sort_order"
    ).bind(aid).fetch_all(&state.db).await.unwrap_or_default();

    let milestones: Vec<(String, Option<String>, Option<i32>, bool)> = sqlx::query_as(
        "SELECT title, description, month_index, COALESCE(is_completed,false) FROM asset_milestones WHERE asset_id = $1 ORDER BY month_index"
    ).bind(aid).fetch_all(&state.db).await.unwrap_or_default();

    Ok(Json(serde_json::json!({
        "title": row.get::<String, _>("title"),
        "asset_type": row.get::<String, _>("asset_type"),
        "city": row.get::<String, _>("city"),
        "country": row.get::<String, _>("country"),
        "description": row.get::<String, _>("description"),
        "slug": row.get::<Option<String>, _>("slug"),
        "total_value_cents": row.get::<i64, _>("total_value_cents"),
        "token_price_cents": row.get::<i64, _>("token_price_cents"),
        "tokens_total": row.get::<i32, _>("tokens_total"),
        "tokens_available": row.get::<i32, _>("tokens_available"),
        "annual_yield_bps": row.get::<Option<i32>, _>("annual_yield_bps"),
        "bedrooms": row.get::<Option<i32>, _>("bedrooms"),
        "bathrooms": row.get::<Option<i32>, _>("bathrooms"),
        "construction_status": row.get::<Option<String>, _>("construction_status"),
        "video_url": row.get::<Option<String>, _>("video_url"),
        "google_maps_url": row.get::<Option<String>, _>("google_maps_url"),
        "documents": docs.iter().map(|d| serde_json::json!({"document_type": d.0, "file_size": d.1})).collect::<Vec<_>>(),
        "images": images.iter().map(|i| serde_json::json!({"url": i.0, "is_cover": i.1, "sort_order": i.2})).collect::<Vec<_>>(),
        "milestones": milestones.iter().map(|m| serde_json::json!({"title": m.0, "description": m.1, "month_index": m.2, "is_completed": m.3})).collect::<Vec<_>>(),
    })).into_response())
}
