use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::Row;

//
//  Admin Assets API (Live/Published)
//

/// GET /api/admin/assets  List published assets with funding progress
pub async fn api_admin_assets(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.view").await?;

    let rows = sqlx::query(
        r#"SELECT a.id::text, a.title, a.slug, a.asset_type,
                  a.location_city, a.total_value_cents, a.token_price_cents,
                  a.tokens_total, a.tokens_available, a.annual_yield_bps,
                  a.funding_status, a.featured, a.published,
                  a.created_at::text
           FROM assets a
           WHERE a.published = TRUE
           ORDER BY a.featured DESC, a.created_at DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let assets: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "title": r.get::<String, _>("title"),
                "slug": r.get::<String, _>("slug"),
                "asset_type": r.get::<String, _>("asset_type"),
                "location_city": r.get::<Option<String>, _>("location_city"),
                "total_value_cents": r.get::<i64, _>("total_value_cents"),
                "token_price_cents": r.get::<i64, _>("token_price_cents"),
                "tokens_total": r.get::<i32, _>("tokens_total"),
                "tokens_available": r.get::<i32, _>("tokens_available"),
                "annual_yield_bps": r.get::<Option<i32>, _>("annual_yield_bps"),
                "funding_status": r.get::<String, _>("funding_status"),
                "featured": r.get::<bool, _>("featured"),
                "published": r.get::<bool, _>("published"),
                "created_at": r.get::<String, _>("created_at")
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "assets": assets })).into_response())
}

/// POST /api/admin/assets/:asset_id/toggle-featured
pub async fn api_admin_toggle_featured(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "assets.publish")
        .await?;

    let uid = ApiError::parse_uuid(&asset_id)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let previous: Option<bool> =
        sqlx::query_scalar("SELECT featured FROM assets WHERE id = $1 FOR UPDATE")
            .bind(uid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
    let previous = previous.ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;
    let next = !previous;

    sqlx::query("UPDATE assets SET featured = $2, updated_at = NOW() WHERE id = $1")
        .bind(uid)
        .bind(next)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'asset.featured_toggled', 'assets', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(uid)
    .bind(serde_json::json!({ "featured": previous }))
    .bind(serde_json::json!({ "featured": next }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({"status": "success", "featured": next})).into_response())
}

/// GET /api/admin/assets/:asset_id/detail
pub async fn api_admin_asset_detail(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.view").await?;

    let aid = ApiError::parse_uuid(&asset_id)?;

    let row = sqlx::query(
        "SELECT a.id::text, a.title, a.slug, a.asset_type, a.property_type,
                a.location_city as city, a.location_country as country,
                COALESCE(a.total_value_cents,0) as total_value_cents,
                COALESCE(a.token_price_cents,0) as token_price_cents,
                COALESCE(a.tokens_total,0) as tokens_total,
                COALESCE(a.tokens_available,0) as tokens_available,
                a.annual_yield_bps, a.capital_appreciation_bps, a.occupancy_rate_bps,
                a.funding_status, a.description, a.video_url,
                COALESCE(a.featured,false) as featured,
                COALESCE(a.published,false) as published,
                a.construction_status
         FROM assets a WHERE a.id = $1",
    )
    .bind(aid)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Not found".to_string()))?;

    // Cap table
    let investors: Vec<(String, String, i32, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT COALESCE(up.first_name || ' ' || up.last_name, u.email), u.id::text,
                COALESCE(i.tokens_owned,0), COALESCE(i.purchase_value_cents,0),
                COALESCE(i.current_value_cents,0), COALESCE(i.total_rental_cents,0),
                COALESCE(i.status,'active')
	         FROM investments i JOIN users u ON u.id = i.user_id LEFT JOIN user_profiles up ON up.user_id = u.id
	         WHERE i.asset_id = $1 ORDER BY i.tokens_owned DESC"
    ).bind(aid).fetch_all(&state.db).await.map_err(ApiError::Database)?;

    // Financial records
    let financials: Vec<(i32, i32, i64, i64, i64, Option<i32>)> = sqlx::query_as(
        "SELECT period_month, period_year, COALESCE(rental_income_cents,0), COALESCE(expenses_cents,0),
	                COALESCE(net_income_cents,0), occupancy_rate_bps
	         FROM asset_financials WHERE asset_id = $1 ORDER BY period_year, period_month"
    ).bind(aid).fetch_all(&state.db).await.map_err(ApiError::Database)?;

    // Documents
    let docs: Vec<(String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT id::text, document_type, title, file_size_bytes FROM asset_documents WHERE asset_id = $1 ORDER BY document_type, created_at",
    )
    .bind(aid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Images
    let images: Vec<(String, bool, i32)> = sqlx::query_as(
        "SELECT COALESCE(image_url,''), COALESCE(is_cover,false), COALESCE(sort_order,0) FROM asset_images WHERE asset_id = $1 ORDER BY sort_order"
    ).bind(aid).fetch_all(&state.db).await.map_err(ApiError::Database)?;

    // Milestones
    let milestones: Vec<(String, Option<String>, Option<i32>, bool)> = sqlx::query_as(
        "SELECT title, description, month_index, COALESCE(is_completed,false) FROM asset_milestones WHERE asset_id = $1 ORDER BY month_index"
    ).bind(aid).fetch_all(&state.db).await.map_err(ApiError::Database)?;

    // Orders referencing this asset
    let orders: Vec<(String, String, i32, i64, String, String)> = sqlx::query_as(
        "SELECT o.order_number, COALESCE(u.email,''), oi.tokens_quantity, oi.subtotal_cents,
                COALESCE(o.status,''), o.created_at::text
         FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         JOIN users u ON u.id = o.user_id
         WHERE oi.asset_id = $1
         ORDER BY o.created_at DESC LIMIT 100",
    )
    .bind(aid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "id": row.get::<String, _>("id"),
        "title": row.get::<String, _>("title"),
        "slug": row.get::<Option<String>, _>("slug"),
        "asset_type": row.get::<Option<String>, _>("asset_type"),
        "property_type": row.get::<Option<String>, _>("property_type"),
        "city": row.get::<Option<String>, _>("city"),
        "country": row.get::<Option<String>, _>("country"),
        "total_value_cents": row.get::<i64, _>("total_value_cents"),
        "token_price_cents": row.get::<i64, _>("token_price_cents"),
        "tokens_total": row.get::<i32, _>("tokens_total"),
        "tokens_available": row.get::<i32, _>("tokens_available"),
        "annual_yield_bps": row.get::<Option<i32>, _>("annual_yield_bps"),
        "capital_appreciation_bps": row.get::<Option<i32>, _>("capital_appreciation_bps"),
        "occupancy_rate_bps": row.get::<Option<i32>, _>("occupancy_rate_bps"),
        "funding_status": row.get::<Option<String>, _>("funding_status"),
        "description": row.get::<Option<String>, _>("description"),
        "video_url": row.get::<Option<String>, _>("video_url"),
        "featured": row.get::<bool, _>("featured"),
        "published": row.get::<bool, _>("published"),
        "construction_status": row.get::<Option<String>, _>("construction_status"),
        "investors": investors.iter().map(|i| serde_json::json!({
            "name": i.0, "user_id": i.1, "tokens_owned": i.2,
            "purchase_value_cents": i.3, "current_value_cents": i.4,
            "total_rental_cents": i.5, "status": i.6
        })).collect::<Vec<_>>(),
        "financials": financials.iter().map(|f| serde_json::json!({
            "period_month": f.0, "period_year": f.1,
            "rental_income_cents": f.2, "expenses_cents": f.3,
            "net_income_cents": f.4, "occupancy_rate_bps": f.5
        })).collect::<Vec<_>>(),
        "documents": docs.iter().map(|d| serde_json::json!({
            "id": d.0,
            "document_type": d.1,
            "title": d.2,
            "file_size": d.3,
            "url": format!("/api/documents/{}/download", d.0)
        })).collect::<Vec<_>>(),
        "images": images.iter().map(|i| {
            let url = crate::storage::service::rewrite_gcs_url(&i.0);
            serde_json::json!({"url": url, "is_cover": i.1, "sort_order": i.2})
        }).collect::<Vec<_>>(),
        "milestones": milestones.iter().map(|m| serde_json::json!({"title": m.0, "description": m.1, "month_index": m.2, "is_completed": m.3})).collect::<Vec<_>>(),
        "orders": orders.iter().map(|o| serde_json::json!({
            "order_number": o.0, "user_email": o.1, "tokens": o.2,
            "subtotal_cents": o.3, "status": o.4, "created_at": o.5
        })).collect::<Vec<_>>(),
    })).into_response())
}

/// Payload for publishing or unpublishing an asset.
#[derive(Debug, Deserialize)]
pub struct AssetPublicationPayload {
    /// Whether the asset should be visible on the marketplace.
    pub published: bool,
}

/// PATCH /api/admin/assets/:asset_id/publication
pub async fn api_admin_asset_publication(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
    Json(payload): Json<AssetPublicationPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "assets.publish")
        .await?;

    let aid = ApiError::parse_uuid(&asset_id)?;
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let previous: Option<bool> =
        sqlx::query_scalar("SELECT published FROM assets WHERE id = $1 FOR UPDATE")
            .bind(aid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
    let previous = previous.ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    sqlx::query("UPDATE assets SET published = $2, updated_at = NOW() WHERE id = $1")
        .bind(aid)
        .bind(payload.published)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'asset.publication_updated', 'assets', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(aid)
    .bind(serde_json::json!({ "published": previous }))
    .bind(serde_json::json!({ "published": payload.published }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "published": payload.published
    }))
    .into_response())
}

/// Payload for updating an asset funding status.
#[derive(Debug, Deserialize)]
pub struct AssetFundingStatusPayload {
    /// New funding status. Must match the database check constraint.
    pub funding_status: String,
}

/// PATCH /api/admin/assets/:asset_id/funding-status
pub async fn api_admin_asset_funding_status(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
    Json(payload): Json<AssetFundingStatusPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;

    const ALLOWED_STATUSES: &[&str] = &[
        "upcoming",
        "funding_open",
        "funding_in_progress",
        "funded",
        "rented",
        "payout_pending",
        "exited",
    ];

    if !ALLOWED_STATUSES.contains(&payload.funding_status.as_str()) {
        return Err(ApiError::BadRequest("Invalid funding status".to_string()));
    }

    let aid = ApiError::parse_uuid(&asset_id)?;
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let previous: Option<String> =
        sqlx::query_scalar("SELECT funding_status FROM assets WHERE id = $1 FOR UPDATE")
            .bind(aid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
    let previous = previous.ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    sqlx::query("UPDATE assets SET funding_status = $2, updated_at = NOW() WHERE id = $1")
        .bind(aid)
        .bind(&payload.funding_status)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'asset.funding_status_updated', 'assets', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(aid)
    .bind(serde_json::json!({ "funding_status": previous }))
    .bind(serde_json::json!({ "funding_status": payload.funding_status }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "funding_status": payload.funding_status
    }))
    .into_response())
}
