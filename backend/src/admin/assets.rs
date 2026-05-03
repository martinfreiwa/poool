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
                  a.created_at::text, a.updated_at::text,
                  a.chain_contract_address, a.chain_token_id, a.chain_tx_hash,
                  COALESCE(t.holders_count, 0)::bigint AS holders_count,
                  COALESCE(t.pending_settlements, 0)::bigint AS pending_settlements
           FROM assets a
           LEFT JOIN (
             SELECT asset_id,
                    COUNT(DISTINCT buyer_user_id) AS holders_count,
                    COUNT(*) FILTER (WHERE on_chain_status IN ('pending', 'submitted')) AS pending_settlements
             FROM trade_history
             GROUP BY asset_id
           ) t ON t.asset_id = a.id
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
                "created_at": r.get::<String, _>("created_at"),
                "updated_at": r.get::<String, _>("updated_at"),
                "chain_contract_address": r.get::<Option<String>, _>("chain_contract_address"),
                "chain_token_id": r.get::<Option<String>, _>("chain_token_id"),
                "chain_tx_hash": r.get::<Option<String>, _>("chain_tx_hash"),
                "holders_count": r.get::<i64, _>("holders_count"),
                "pending_settlements": r.get::<i64, _>("pending_settlements")
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

// ─────────────────────────────────────────────────────────────────────────────
//  Editable Property-Page Content  (migration 116)
//
//  Admin endpoints that let staff populate the CMS-style fields rendered on
//  the property detail page (location description, investment type, leasing
//  strategy, calculator defaults, developer card, risk notification) plus
//  full CRUD for the roadmap / funding-timeline milestones.
// ─────────────────────────────────────────────────────────────────────────────

/// Partial update payload for `PATCH /api/admin/assets/:asset_id/page-content`.
///
/// All fields are optional; only those present in the JSON body are written.
/// Use `null` to clear a field, omit it to leave it untouched.
#[allow(missing_docs)]
#[derive(Debug, Deserialize)]
pub struct AssetPageContentPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_description: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub investment_type: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub investment_type_description: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leasing_strategy_type: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leasing_strategy_description: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_notification: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_investment_amount_cents: Option<Option<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value_growth_bps: Option<Option<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_rental_yield_bps: Option<Option<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_logo_url: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_name: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_description: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_website: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_facebook: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_instagram: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer_youtube: Option<Option<String>>,
}

const TEXT_MAX_SHORT: usize = 255;
const TEXT_MAX_LONG: usize = 8_000;
const URL_MAX: usize = 512;

fn clean_opt_text(v: Option<Option<String>>, max: usize) -> Result<Option<Option<String>>, ApiError> {
    match v {
        None => Ok(None),
        Some(None) => Ok(Some(None)),
        Some(Some(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(Some(None));
            }
            if trimmed.chars().count() > max {
                return Err(ApiError::BadRequest(format!(
                    "Field exceeds {} character limit",
                    max
                )));
            }
            Ok(Some(Some(trimmed.to_string())))
        }
    }
}

fn validate_bps(v: Option<Option<i32>>) -> Result<Option<Option<i32>>, ApiError> {
    if let Some(Some(n)) = v {
        if !(0..=20_000).contains(&n) {
            return Err(ApiError::BadRequest(
                "Basis-points value must be between 0 and 20000".to_string(),
            ));
        }
    }
    Ok(v)
}

fn validate_cents(v: Option<Option<i64>>) -> Result<Option<Option<i64>>, ApiError> {
    if let Some(Some(n)) = v {
        if n < 0 {
            return Err(ApiError::BadRequest(
                "Amount in cents cannot be negative".to_string(),
            ));
        }
    }
    Ok(v)
}

/// PATCH /api/admin/assets/:asset_id/page-content
pub async fn api_admin_asset_page_content(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
    Json(mut payload): Json<AssetPageContentPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;
    let aid = ApiError::parse_uuid(&asset_id)?;

    payload.location_description = clean_opt_text(payload.location_description, TEXT_MAX_LONG)?;
    payload.investment_type = clean_opt_text(payload.investment_type, TEXT_MAX_SHORT)?;
    payload.investment_type_description =
        clean_opt_text(payload.investment_type_description, TEXT_MAX_LONG)?;
    payload.leasing_strategy_type = clean_opt_text(payload.leasing_strategy_type, TEXT_MAX_SHORT)?;
    payload.leasing_strategy_description =
        clean_opt_text(payload.leasing_strategy_description, TEXT_MAX_LONG)?;
    payload.risk_notification = clean_opt_text(payload.risk_notification, TEXT_MAX_LONG)?;
    payload.default_investment_amount_cents =
        validate_cents(payload.default_investment_amount_cents)?;
    payload.default_value_growth_bps = validate_bps(payload.default_value_growth_bps)?;
    payload.default_rental_yield_bps = validate_bps(payload.default_rental_yield_bps)?;
    payload.developer_logo_url = clean_opt_text(payload.developer_logo_url, URL_MAX)?;
    payload.developer_name = clean_opt_text(payload.developer_name, TEXT_MAX_SHORT)?;
    payload.developer_description = clean_opt_text(payload.developer_description, TEXT_MAX_LONG)?;
    payload.developer_website = clean_opt_text(payload.developer_website, URL_MAX)?;
    payload.developer_facebook = clean_opt_text(payload.developer_facebook, URL_MAX)?;
    payload.developer_instagram = clean_opt_text(payload.developer_instagram, URL_MAX)?;
    payload.developer_youtube = clean_opt_text(payload.developer_youtube, URL_MAX)?;

    // Build dynamic UPDATE — only touch columns the client sent.
    let mut sets: Vec<String> = Vec::new();
    let mut idx = 1usize;
    let mut q = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE assets SET ");

    macro_rules! push_text {
        ($field:expr, $col:literal) => {
            if let Some(v) = $field {
                if !sets.is_empty() {
                    q.push(", ");
                }
                idx += 1;
                q.push(concat!($col, " = "));
                q.push_bind(v);
                sets.push($col.to_string());
            }
        };
    }
    macro_rules! push_int {
        ($field:expr, $col:literal, $ty:ty) => {
            if let Some(v) = $field {
                if !sets.is_empty() {
                    q.push(", ");
                }
                idx += 1;
                q.push(concat!($col, " = "));
                q.push_bind::<Option<$ty>>(v);
                sets.push($col.to_string());
            }
        };
    }

    push_text!(payload.location_description, "location_description");
    push_text!(payload.investment_type, "investment_type");
    push_text!(
        payload.investment_type_description,
        "investment_type_description"
    );
    push_text!(payload.leasing_strategy_type, "leasing_strategy_type");
    push_text!(
        payload.leasing_strategy_description,
        "leasing_strategy_description"
    );
    push_text!(payload.risk_notification, "risk_notification");
    push_int!(
        payload.default_investment_amount_cents,
        "default_investment_amount_cents",
        i64
    );
    push_int!(
        payload.default_value_growth_bps,
        "default_value_growth_bps",
        i32
    );
    push_int!(
        payload.default_rental_yield_bps,
        "default_rental_yield_bps",
        i32
    );
    push_text!(payload.developer_logo_url, "developer_logo_url");
    push_text!(payload.developer_name, "developer_name");
    push_text!(payload.developer_description, "developer_description");
    push_text!(payload.developer_website, "developer_website");
    push_text!(payload.developer_facebook, "developer_facebook");
    push_text!(payload.developer_instagram, "developer_instagram");
    push_text!(payload.developer_youtube, "developer_youtube");

    if sets.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }
    let _ = idx;

    q.push(", updated_at = NOW() WHERE id = ");
    q.push_bind(aid);

    let result = q
        .build()
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Asset not found".to_string()));
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'asset.page_content_updated', 'assets', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(aid)
    .bind(serde_json::json!({ "fields": sets }))
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "fields_updated": sets,
    }))
    .into_response())
}

// ── Milestones (Roadmap / Funding Timeline) CRUD ─────────────────────────────

/// Payload for creating a milestone via `POST /api/admin/assets/:asset_id/milestones`.
#[allow(missing_docs)]
#[derive(Debug, Deserialize)]
pub struct MilestoneCreatePayload {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub month_index: Option<i32>,
    #[serde(default)]
    pub milestone_date: Option<String>,
    #[serde(default)]
    pub is_completed: Option<bool>,
}

/// Payload for updating a milestone via `PATCH /api/admin/assets/:asset_id/milestones/:milestone_id`.
#[allow(missing_docs)]
#[derive(Debug, Deserialize)]
pub struct MilestoneUpdatePayload {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month_index: Option<Option<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone_date: Option<Option<String>>,
    #[serde(default)]
    pub is_completed: Option<bool>,
}

/// POST /api/admin/assets/:asset_id/milestones
pub async fn api_admin_asset_milestone_create(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
    Json(payload): Json<MilestoneCreatePayload>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;
    let aid = ApiError::parse_uuid(&asset_id)?;

    let title = payload.title.trim();
    if title.is_empty() {
        return Err(ApiError::BadRequest("Title is required".to_string()));
    }
    if title.chars().count() > TEXT_MAX_SHORT {
        return Err(ApiError::BadRequest(format!(
            "Title exceeds {} character limit",
            TEXT_MAX_SHORT
        )));
    }

    let description = payload
        .description
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let date_ts = match payload.milestone_date.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(s) => Some(
            chrono::DateTime::parse_from_rfc3339(s)
                .map_err(|_| ApiError::BadRequest("Invalid milestone_date".to_string()))?
                .with_timezone(&chrono::Utc),
        ),
    };

    let row = sqlx::query(
        r#"INSERT INTO asset_milestones
              (asset_id, title, description, milestone_date, month_index, is_completed)
           VALUES ($1, $2, $3, $4, $5, COALESCE($6, false))
           RETURNING id::text, title, description, milestone_date::text, month_index, is_completed"#,
    )
    .bind(aid)
    .bind(title)
    .bind(description.as_deref())
    .bind(date_ts)
    .bind(payload.month_index)
    .bind(payload.is_completed)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "id": row.get::<String, _>("id"),
        "title": row.get::<String, _>("title"),
        "description": row.get::<Option<String>, _>("description"),
        "milestone_date": row.get::<Option<String>, _>("milestone_date"),
        "month_index": row.get::<Option<i32>, _>("month_index"),
        "is_completed": row.get::<bool, _>("is_completed"),
    }))
    .into_response())
}

/// PATCH /api/admin/assets/:asset_id/milestones/:milestone_id
pub async fn api_admin_asset_milestone_update(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path((asset_id, milestone_id)): axum::extract::Path<(String, String)>,
    Json(payload): Json<MilestoneUpdatePayload>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;
    let aid = ApiError::parse_uuid(&asset_id)?;
    let mid = ApiError::parse_uuid(&milestone_id)?;

    let mut sets: Vec<String> = Vec::new();
    let mut q = sqlx::QueryBuilder::<sqlx::Postgres>::new("UPDATE asset_milestones SET ");

    if let Some(t) = payload.title.as_ref() {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            return Err(ApiError::BadRequest("Title cannot be empty".to_string()));
        }
        if trimmed.chars().count() > TEXT_MAX_SHORT {
            return Err(ApiError::BadRequest("Title too long".to_string()));
        }
        q.push("title = ");
        q.push_bind(trimmed.to_string());
        sets.push("title".to_string());
    }

    if let Some(d) = payload.description {
        let cleaned = d
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        if !sets.is_empty() {
            q.push(", ");
        }
        q.push("description = ");
        q.push_bind(cleaned);
        sets.push("description".to_string());
    }

    if let Some(m) = payload.month_index {
        if !sets.is_empty() {
            q.push(", ");
        }
        q.push("month_index = ");
        q.push_bind::<Option<i32>>(m);
        sets.push("month_index".to_string());
    }

    if let Some(d) = payload.milestone_date {
        let parsed = match d.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(s) => Some(
                chrono::DateTime::parse_from_rfc3339(s)
                    .map_err(|_| ApiError::BadRequest("Invalid milestone_date".to_string()))?
                    .with_timezone(&chrono::Utc),
            ),
        };
        if !sets.is_empty() {
            q.push(", ");
        }
        q.push("milestone_date = ");
        q.push_bind(parsed);
        sets.push("milestone_date".to_string());
    }

    if let Some(c) = payload.is_completed {
        if !sets.is_empty() {
            q.push(", ");
        }
        q.push("is_completed = ");
        q.push_bind(c);
        sets.push("is_completed".to_string());
    }

    if sets.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    q.push(" WHERE id = ");
    q.push_bind(mid);
    q.push(" AND asset_id = ");
    q.push_bind(aid);

    let result = q
        .build()
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Milestone not found".to_string()));
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "fields_updated": sets,
    }))
    .into_response())
}

/// DELETE /api/admin/assets/:asset_id/milestones/:milestone_id
pub async fn api_admin_asset_milestone_delete(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path((asset_id, milestone_id)): axum::extract::Path<(String, String)>,
) -> Result<axum::response::Response, ApiError> {
    admin.require_permission(&state.db, "assets.edit").await?;
    let aid = ApiError::parse_uuid(&asset_id)?;
    let mid = ApiError::parse_uuid(&milestone_id)?;

    let result =
        sqlx::query("DELETE FROM asset_milestones WHERE id = $1 AND asset_id = $2")
            .bind(mid)
            .bind(aid)
            .execute(&state.db)
            .await
            .map_err(ApiError::Database)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Milestone not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "status": "success" })).into_response())
}
