use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

//
//  Admin Assets API (Live/Published)
//

/// Query parameters for admin assets
#[derive(serde::Deserialize)]
pub struct AdminAssetQuery {
    /// Search term
    pub search: Option<String>,
    /// Asset type filter
    pub r#type: Option<String>,
    /// Funding status filter
    pub status: Option<String>,
    /// Featured filter
    pub featured: Option<bool>,
    /// Sort column
    pub sort: Option<String>,
    /// Sort order
    pub order: Option<String>,
    /// Page number
    pub page: Option<i64>,
    /// Items per page
    pub limit: Option<i64>,
}

/// GET /api/admin/assets  List published assets with funding progress
pub async fn api_admin_assets(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminAssetQuery>,
) -> Result<axum::response::Response, ApiError> {
    let mut qb = sqlx::QueryBuilder::new(
        r#"
        WITH filtered_assets AS (
            SELECT a.id::text, a.title, a.slug, a.asset_type,
                   a.location_city, a.total_value_cents, a.token_price_cents,
                   a.tokens_total, a.tokens_available, a.annual_yield_bps,
                   a.funding_status, a.featured, a.published,
                   a.created_at::text,
                   (CASE WHEN a.tokens_total > 0 THEN (a.tokens_total - a.tokens_available)::float / a.tokens_total::float ELSE 0 END) as funding_progress
            FROM assets a
            WHERE a.published = TRUE 
        "#,
    );

    if let Some(s) = &q.search {
        if !s.trim().is_empty() {
            let term = format!("%{}%", s.trim());
            qb.push(" AND (a.title ILIKE ");
            qb.push_bind(term.clone());
            qb.push(" OR a.slug ILIKE ");
            qb.push_bind(term.clone());
            qb.push(" OR a.location_city ILIKE ");
            qb.push_bind(term);
            qb.push(") ");
        }
    }

    if let Some(t) = &q.r#type {
        if !t.is_empty() {
            qb.push(" AND a.asset_type = ");
            qb.push_bind(t);
        }
    }

    if let Some(st) = &q.status {
        if !st.is_empty() {
            qb.push(" AND a.funding_status = ");
            qb.push_bind(st);
        }
    }

    if let Some(true) = q.featured {
        qb.push(" AND a.featured = TRUE ");
    }

    let sort_col = match q.sort.as_deref() {
        Some("title") => "title",
        Some("asset_type") => "asset_type",
        Some("total_value_cents") => "total_value_cents",
        Some("annual_yield_bps") => "annual_yield_bps",
        Some("location_city") => "location_city",
        Some("funding_status") => "funding_status",
        Some("featured") => "featured",
        Some("funding_progress") => "funding_progress",
        _ => "created_at",
    };

    let order = match q.order.as_deref() {
        Some("asc") => "ASC",
        _ => "DESC",
    };

    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(10).max(1).min(100);
    let offset = (page - 1) * limit;

    qb.push(format!(
        r#"
        ),
        stats AS (
            SELECT 
                COUNT(*)::bigint as stat_total,
                COUNT(*) FILTER (WHERE funding_status IN ('funding_open', 'funding_in_progress'))::bigint as stat_funding,
                COUNT(*) FILTER (WHERE funding_status IN ('funded', 'rented', 'exited'))::bigint as stat_funded,
                COALESCE(SUM(total_value_cents), 0)::bigint as stat_aum,
                COALESCE(SUM(tokens_total - tokens_available), 0)::bigint as stat_tokens_sold
            FROM filtered_assets
        )
        SELECT 
            (SELECT row_to_json(s) FROM stats s) as stats,
            COALESCE(
                (
                    SELECT json_agg(row_to_json(d)) 
                    FROM (
                        SELECT * FROM filtered_assets
                        ORDER BY {sort_col} {order}
                        LIMIT {limit} OFFSET {offset}
                    ) d
                ), 
            '[]'::json) as assets
        "#
    ));

    let row = qb
        .build()
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let stats: serde_json::Value = row.get("stats");
    let assets: serde_json::Value = row.get("assets");

    Ok(Json(serde_json::json!({
        "assets": assets,
        "stats": stats,
        "page": page,
        "limit": limit
    }))
    .into_response())
}

/// POST /api/admin/assets/:asset_id/toggle-featured
pub async fn api_admin_toggle_featured(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&asset_id)?;

    let updated =
        sqlx::query("UPDATE assets SET featured = NOT featured, updated_at = NOW() WHERE id = $1")
            .bind(uid)
            .execute(&state.db)
            .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            Ok(Json(serde_json::json!({"status": "toggled"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound("Asset not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to toggle featured {asset_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// GET /api/admin/assets/:asset_id/detail
pub async fn api_admin_asset_detail(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(asset_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let aid = ApiError::parse_uuid(&asset_id)?;

    let row = sqlx::query(
        "SELECT a.title, a.slug, a.asset_type, a.property_type, a.city, a.country,
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
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            return Err(ApiError::NotFound("Not found".to_string()));
        }
    };

    // Cap table
    let investors: Vec<(String, String, i32, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT COALESCE(up.first_name || ' ' || up.last_name, u.email), u.id::text,
                COALESCE(i.tokens_owned,0), COALESCE(i.purchase_value_cents,0),
                COALESCE(i.current_value_cents,0), COALESCE(i.total_rental_cents,0),
                COALESCE(i.status,'active')
         FROM investments i JOIN users u ON u.id = i.user_id LEFT JOIN user_profiles up ON up.user_id = u.id
         WHERE i.asset_id = $1 ORDER BY i.tokens_owned DESC"
    ).bind(aid).fetch_all(&state.db).await.unwrap_or_default();

    // Financial records
    let financials: Vec<(i32, i32, i64, i64, i64, Option<i32>)> = sqlx::query_as(
        "SELECT period_month, period_year, COALESCE(rental_income_cents,0), COALESCE(expenses_cents,0),
                COALESCE(net_income_cents,0), occupancy_rate_bps
         FROM asset_financials WHERE asset_id = $1 ORDER BY period_year, period_month"
    ).bind(aid).fetch_all(&state.db).await.unwrap_or_default();

    // Documents
    let docs: Vec<(String, Option<i64>)> = sqlx::query_as(
        "SELECT document_type, file_size_bytes FROM asset_documents WHERE asset_id = $1",
    )
    .bind(aid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Images
    let images: Vec<(String, bool, i32)> = sqlx::query_as(
        "SELECT COALESCE(image_url,''), COALESCE(is_cover,false), COALESCE(sort_order,0) FROM asset_images WHERE asset_id = $1 ORDER BY sort_order"
    ).bind(aid).fetch_all(&state.db).await.unwrap_or_default();

    // Milestones
    let milestones: Vec<(String, Option<String>, Option<i32>, bool)> = sqlx::query_as(
        "SELECT title, description, month_index, COALESCE(is_completed,false) FROM asset_milestones WHERE asset_id = $1 ORDER BY month_index"
    ).bind(aid).fetch_all(&state.db).await.unwrap_or_default();

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
    .unwrap_or_default();

    Ok(Json(serde_json::json!({
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
        "documents": docs.iter().map(|d| serde_json::json!({"document_type": d.0, "file_size": d.1})).collect::<Vec<_>>(),
        "images": images.iter().map(|i| serde_json::json!({"url": i.0, "is_cover": i.1, "sort_order": i.2})).collect::<Vec<_>>(),
        "milestones": milestones.iter().map(|m| serde_json::json!({"title": m.0, "description": m.1, "month_index": m.2, "is_completed": m.3})).collect::<Vec<_>>(),
        "orders": orders.iter().map(|o| serde_json::json!({
            "order_number": o.0, "user_email": o.1, "tokens": o.2,
            "subtotal_cents": o.3, "status": o.4, "created_at": o.5
        })).collect::<Vec<_>>(),
    })).into_response())
}
