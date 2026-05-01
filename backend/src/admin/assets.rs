use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Multipart, Path, State},
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::Row;
use std::collections::HashSet;
use uuid::Uuid;

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
    let investors: Vec<(String, String, String, i32, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT COALESCE(up.first_name || ' ' || up.last_name, u.email), u.email, u.id::text,
                COALESCE(i.tokens_owned,0), COALESCE(i.purchase_value_cents,0),
                COALESCE(i.current_value_cents,0), COALESCE(i.total_rental_cents,0),
                COALESCE(i.status,'active')
	         FROM investments i JOIN users u ON u.id = i.user_id LEFT JOIN user_profiles up ON up.user_id = u.id
	         WHERE i.asset_id = $1 ORDER BY i.tokens_owned DESC"
    ).bind(aid).fetch_all(&state.db).await.map_err(ApiError::Database)?;

    // Resale liquidity: open sell orders on secondary market
    let resale_available: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(quantity - quantity_filled), 0) FROM market_orders
         WHERE asset_id = $1 AND side = 'sell' AND status IN ('open', 'partially_filled')",
    )
    .bind(aid)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

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
            "name": i.0, "email": i.1, "user_id": i.2, "tokens_owned": i.3,
            "purchase_value_cents": i.4, "current_value_cents": i.5,
            "total_rental_cents": i.6, "status": i.7
        })).collect::<Vec<_>>(),
        "resale_tokens_available": resale_available,
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

/// Payload item for reordering asset images.
#[derive(Debug, Deserialize)]
pub struct AdminImageOrderUpdate {
    /// Image row id.
    pub id: Uuid,
    /// New zero-based sort position.
    pub sort_order: i32,
    /// Whether this image should be the cover image.
    pub is_cover: bool,
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

/// POST /api/admin/assets/:asset_id/images
pub async fn api_admin_asset_image_upload(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    multipart: Multipart,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;

    let aid = ApiError::parse_uuid(&asset_id)?;
    ensure_asset_exists(&state, aid).await?;

    let upload = read_admin_asset_image_multipart(multipart).await?;
    let object_path = format!(
        "properties/{}/images/{}.{}",
        aid,
        Uuid::new_v4(),
        crate::storage::service::extension_for_mime(&upload.mime_type)
    );

    let image_url =
        upload_admin_asset_image(&state, &object_path, &upload.bytes, &upload.mime_type)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload admin asset image for {aid}: {e}");
                ApiError::Internal("Failed to upload image".to_string())
            })?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    if upload.is_cover {
        sqlx::query("UPDATE asset_images SET is_cover = FALSE WHERE asset_id = $1")
            .bind(aid)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
    }

    let image_id: Uuid = sqlx::query_scalar(
        "INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(aid)
    .bind(&image_url)
    .bind(upload.alt_text.as_deref())
    .bind(upload.sort_order)
    .bind(upload.is_cover)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'asset.image_uploaded', 'assets', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(aid)
    .bind(serde_json::json!({
        "image_id": image_id,
        "image_url": image_url,
        "sort_order": upload.sort_order,
        "is_cover": upload.is_cover,
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "image_id": image_id,
        "image_url": crate::storage::service::rewrite_gcs_url(&image_url),
        "is_cover": upload.is_cover,
        "sort_order": upload.sort_order,
    }))
    .into_response())
}

/// DELETE /api/admin/assets/:asset_id/images/:image_id
pub async fn api_admin_asset_image_delete(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, image_id)): Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;

    let aid = ApiError::parse_uuid(&asset_id)?;
    let iid = ApiError::parse_uuid(&image_id)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;
    let previous = sqlx::query(
        "SELECT image_url, alt_text, sort_order, is_cover
         FROM asset_images
         WHERE id = $1 AND asset_id = $2
         FOR UPDATE",
    )
    .bind(iid)
    .bind(aid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Image not found".to_string()))?;

    let deleted = sqlx::query("DELETE FROM asset_images WHERE id = $1 AND asset_id = $2")
        .bind(iid)
        .bind(aid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::NotFound("Image not found".to_string()));
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state)
           VALUES ($1, 'asset.image_deleted', 'assets', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(aid)
    .bind(serde_json::json!({
        "image_id": iid,
        "image_url": previous.get::<String, _>("image_url"),
        "alt_text": previous.get::<Option<String>, _>("alt_text"),
        "sort_order": previous.get::<i32, _>("sort_order"),
        "is_cover": previous.get::<bool, _>("is_cover"),
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({"status": "success"})).into_response())
}

/// PUT /api/admin/assets/:asset_id/images/reorder
pub async fn api_admin_asset_images_reorder(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    Json(payload): Json<Vec<AdminImageOrderUpdate>>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;

    if payload.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one image order update is required".to_string(),
        ));
    }

    let aid = ApiError::parse_uuid(&asset_id)?;
    let cover_count = payload.iter().filter(|img| img.is_cover).count();
    if cover_count != 1 {
        return Err(ApiError::BadRequest(
            "Exactly one image must be marked as cover".to_string(),
        ));
    }

    let mut seen = HashSet::with_capacity(payload.len());
    for item in &payload {
        if item.sort_order < 0 {
            return Err(ApiError::BadRequest(
                "sort_order must not be negative".to_string(),
            ));
        }
        if !seen.insert(item.id) {
            return Err(ApiError::BadRequest(
                "Duplicate image id in reorder payload".to_string(),
            ));
        }
    }

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;
    sqlx::query("UPDATE asset_images SET is_cover = FALSE WHERE asset_id = $1")
        .bind(aid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

    for item in &payload {
        let updated = sqlx::query(
            "UPDATE asset_images
             SET sort_order = $3, is_cover = $4
             WHERE id = $1 AND asset_id = $2",
        )
        .bind(item.id)
        .bind(aid)
        .bind(item.sort_order)
        .bind(item.is_cover)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        if updated.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Image not found for asset: {}",
                item.id
            )));
        }
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'asset.images_reordered', 'assets', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(aid)
    .bind(
        serde_json::json!({ "images": payload.iter().map(|item| serde_json::json!({
        "id": item.id,
        "sort_order": item.sort_order,
        "is_cover": item.is_cover,
    })).collect::<Vec<_>>() }),
    )
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({"status": "success"})).into_response())
}

struct AdminAssetImageUpload {
    bytes: Vec<u8>,
    mime_type: String,
    sort_order: i32,
    is_cover: bool,
    alt_text: Option<String>,
}

const MAX_ADMIN_ASSET_IMAGE_BYTES: usize = 20 * 1024 * 1024;

async fn ensure_asset_exists(state: &AppState, asset_id: Uuid) -> Result<(), ApiError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assets WHERE id = $1 AND deleted_at IS NULL)",
    )
    .bind(asset_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    if exists {
        Ok(())
    } else {
        Err(ApiError::NotFound("Asset not found".to_string()))
    }
}

async fn read_admin_asset_image_multipart(
    mut multipart: Multipart,
) -> Result<AdminAssetImageUpload, ApiError> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut client_mime = "application/octet-stream".to_string();
    let mut sort_order = 0;
    let mut is_cover = false;
    let mut alt_text: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("Failed to read multipart data".to_string()))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "file" => {
                client_mime = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| ApiError::BadRequest("Failed to read uploaded image".to_string()))?
                    .to_vec();
                if bytes.len() > MAX_ADMIN_ASSET_IMAGE_BYTES {
                    return Err(ApiError::BadRequest("Image must be <= 20 MB".to_string()));
                }
                file_bytes = Some(bytes);
            }
            "sort_order" => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| ApiError::BadRequest("Invalid sort_order".to_string()))?;
                sort_order = text
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| ApiError::BadRequest("Invalid sort_order".to_string()))?;
                if sort_order < 0 {
                    return Err(ApiError::BadRequest(
                        "sort_order must not be negative".to_string(),
                    ));
                }
            }
            "is_cover" => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| ApiError::BadRequest("Invalid is_cover".to_string()))?;
                is_cover = matches!(text.trim(), "true" | "1" | "on");
            }
            "alt_text" => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| ApiError::BadRequest("Invalid alt_text".to_string()))?;
                let sanitized = crate::common::sanitize::sanitize_text(&text);
                if !sanitized.is_empty() {
                    alt_text = Some(sanitized.chars().take(255).collect());
                }
            }
            _ => {}
        }
    }

    let bytes =
        file_bytes.ok_or_else(|| ApiError::BadRequest("No file field in request".to_string()))?;
    let sniffed = sniff_admin_image_mime(&bytes).ok_or_else(|| {
        ApiError::BadRequest("Unsupported or unrecognized image format".to_string())
    })?;

    if !admin_mime_matches(&client_mime, sniffed) {
        return Err(ApiError::BadRequest(
            "File content does not match declared type".to_string(),
        ));
    }

    crate::storage::service::validate_asset_image_mime(sniffed).map_err(|_| {
        ApiError::BadRequest("Only JPEG, PNG, WebP, and GIF images are accepted".to_string())
    })?;

    Ok(AdminAssetImageUpload {
        bytes,
        mime_type: sniffed.to_string(),
        sort_order,
        is_cover,
        alt_text,
    })
}

async fn upload_admin_asset_image(
    state: &AppState,
    object_path: &str,
    file_bytes: &[u8],
    mime_type: &str,
) -> Result<String, crate::error::AppError> {
    if let Some(bucket) = &state.config.gcs_bucket {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            crate::storage::service::upload_public(
                bucket,
                object_path,
                file_bytes.to_vec(),
                mime_type,
            ),
        )
        .await
        {
            Ok(Ok(url)) => return Ok(url),
            Ok(Err(e)) => {
                tracing::warn!("Admin asset image GCS upload failed: {e}; falling back to local");
            }
            Err(_) => {
                tracing::warn!("Admin asset image GCS upload timed out; falling back to local");
            }
        }
    }

    crate::storage::service::upload_local(object_path, file_bytes.to_vec()).await
}

fn sniff_admin_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else {
        None
    }
}

fn admin_mime_matches(client_mime: &str, sniffed: &str) -> bool {
    let declared = client_mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    declared == sniffed
        || matches!(
            (declared.as_str(), sniffed),
            ("image/jpg", "image/jpeg")
                | ("image/pjpeg", "image/jpeg")
                | ("application/octet-stream", _)
        )
}
