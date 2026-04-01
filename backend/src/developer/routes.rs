use super::models::{CreateDraftAsset, UpdateDraftAsset};
use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;
use crate::error::AppError;
/// Developer page & API route handlers.
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

// ─── Page Handlers ──────────────────────────────────────────

/// GET /developer/dashboard — Render the developer dashboard with real DB data.
pub async fn page_developer_dashboard(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };
    let is_developer = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name IN ('developer', 'admin', 'super_admin'))",
        user.id
    ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);

    if !is_developer {
        return Redirect::to("/developer/application-form").into_response();
    }

    // Fetch all dashboard statistics for this developer
    let stats = service::fetch_dashboard_stats(&state.db, user.id).await;

    // Render using MiniJinja with full data context
    let template = match state.templates.get_template("developer/dashboard.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load developer/dashboard.html template: {}", e);
            return Html("<h1>Page not found</h1>".to_string()).into_response();
        }
    };

    let html = match template.render(minijinja::context! {
        user => user,
        stats => stats,
        is_developer => true,
    }) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to render developer/dashboard.html: {}", e);
            return Html(format!("<h1>Internal Server Error: {}</h1>", e)).into_response();
        }
    };

    Html(html).into_response()
}

/// GET /developer/assets — Render the developer assets list.
pub async fn page_developer_assets(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };
    let is_developer = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name IN ('developer', 'admin', 'super_admin'))",
        user.id
    ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);

    if !is_developer {
        return Redirect::to("/developer/application-form").into_response();
    }

    let assets = service::fetch_all_assets(&state.db, user.id).await;
    let stats = service::fetch_dashboard_stats(&state.db, user.id).await;

    let template = match state.templates.get_template("developer/assets.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load developer/assets.html template: {}", e);
            return Html("<h1>Page not found</h1>".to_string()).into_response();
        }
    };

    let html = match template.render(minijinja::context! {
        user => user,
        developer_assets => assets,
        stats => stats,
        active_period => "all",
        is_developer => true,
    }) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to render developer/assets.html: {}", e);
            return Html(format!("<h1>Internal Server Error: {}</h1>", e)).into_response();
        }
    };

    Html(html).into_response()
}

/// GET /developer/add-asset — Render the add-new-asset form.
pub async fn page_developer_add_asset(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "developer/add-asset.html").await
}

/// GET /developer/property-content — Render the property content management page.
pub async fn page_developer_property_content(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "developer/property-content.html")
        .await
}

/// GET /developer/document-upload-step3 — Render the document upload step.
pub async fn page_developer_document_upload(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(
        jar,
        &state,
        "developer/document-upload-step3.html",
    )
    .await
}

/// GET /developer/application-form — Render the developer application form.
pub async fn page_developer_application_form(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "developer/application-form.html")
        .await
}

/// GET /developer/submission-success — Render the submission success page.
pub async fn page_developer_submission_success(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "developer/submission-success.html")
        .await
}

/// GET /developer/asset-detail — Render the asset detail page.
pub async fn page_developer_asset_detail(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "developer/asset-detail.html").await
}

pub async fn page_developer_settings(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected_with_context(jar, &state, "settings-2.html", serde_json::json!({ "is_developer": true })).await
}

/// GET /developer/submissions — Render the submissions management page.
pub async fn page_developer_submissions(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "developer/submissions.html").await
}

// ─── API Endpoints ──────────────────────────────────────────

/// GET /api/developer/dashboard/stats — JSON endpoint for dashboard stats.
pub async fn api_developer_dashboard_stats(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let stats = service::fetch_dashboard_stats(&state.db, user.id).await;
    Json(stats).into_response()
}

/// POST /api/developer/draft — Create a new asset draft securely for a developer.
/// Wrapped in a transaction (B7 fix). Creates developer_projects row (B5 fix).
/// Uses full UUID for slug uniqueness (B8 fix).
pub async fn api_developer_create_draft(
    jar: CookieJar,
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, AppError> {
    // Log raw body for debugging, then deserialize manually so we get
    // proper error messages instead of Axum's silent 422.
    let payload: CreateDraftAsset = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            let body_str = String::from_utf8_lossy(&body);
            tracing::error!(
                "Failed to parse CreateDraftAsset: {}  — body: {}",
                e,
                &body_str[..body_str.len().min(500)]
            );
            return Err(AppError::BadRequest(format!("Invalid request body: {}", e)));
        }
    };

    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Please log in".to_string())),
    };

    // The application form is the entry point for any user to become a developer.
    // If the user doesn't have the developer role yet, auto-assign it on first draft creation.
    let is_developer = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name IN ('developer', 'admin', 'super_admin'))",
        user.id
    ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);

    if !is_developer {
        // Auto-assign the developer role
        let developer_role_id: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = 'developer'")
                .fetch_optional(&state.db)
                .await
                .unwrap_or(None);

        if let Some(role_id) = developer_role_id {
            let _ = sqlx::query(
                "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(user.id)
            .bind(role_id)
            .execute(&state.db)
            .await;

            tracing::info!(
                "Auto-assigned developer role to user {} via application form",
                user.id
            );
        } else {
            tracing::error!("Developer role not found in roles table — cannot auto-assign");
            return Err(AppError::Internal(
                "System configuration error: developer role missing".to_string(),
            ));
        }
    }

    // ── Enforce 100-draft limit ──
    let draft_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM assets a LEFT JOIN developer_projects dp ON dp.asset_id = a.id WHERE a.developer_user_id = $1 AND a.deleted_at IS NULL AND COALESCE(dp.status, 'draft') = 'draft'"
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    if draft_count >= 100 {
        return Err(AppError::BadRequest(
            "You can have a maximum of 100 drafts. Please delete unused drafts before creating a new one.".to_string(),
        ));
    }

    // ── XSS Sanitization on create ──
    use crate::common::sanitize::sanitize_text;
    let mut payload = payload;
    payload.title = sanitize_text(&payload.title);
    payload.asset_type = sanitize_text(&payload.asset_type);
    if let Some(ref v) = payload.property_type {
        payload.property_type = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.area {
        payload.area = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.address {
        payload.address = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.city {
        payload.city = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.country {
        payload.country = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.lease_type {
        payload.lease_type = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.construction_status {
        payload.construction_status = Some(sanitize_text(v));
    }

    // Use full UUID to avoid slug collisions (B8 fix)
    let slug_base = payload
        .title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let slug = format!("{}-{}", slug_base, uuid::Uuid::new_v4());

    // Wrap in a DB transaction for ACID compliance (B7 fix)
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {e}");
        AppError::Internal("Database error".to_string())
    })?;

    let asset_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO assets (
            developer_user_id, title, slug, asset_type, total_value_cents,
            token_price_cents, tokens_total, tokens_available, funding_status,
            featured, published, updated_at, submission_step,
            property_type, area, location_address, location_city, location_country,
            lease_type, lease_term_years,
            land_size_sqm, building_size_sqm, bedrooms, bathrooms,
            construction_status, year_built
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, 'upcoming', false, false, NOW(), 2,
            $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21
        ) RETURNING id",
    )
    .bind(user.id)
    .bind(&payload.title)
    .bind(&slug)
    .bind(&payload.asset_type)
    .bind(payload.total_value_cents)
    .bind(payload.token_price_cents)
    .bind(payload.tokens_total as i32)
    .bind(payload.tokens_total as i32)
    .bind(&payload.property_type)
    .bind(&payload.area)
    .bind(&payload.address)
    .bind(&payload.city)
    .bind(&payload.country)
    .bind(&payload.lease_type)
    .bind(payload.lease_term_years)
    .bind(
        payload
            .land_size_sqm
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default()),
    )
    .bind(
        payload
            .building_size_sqm
            .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default()),
    )
    .bind(payload.bedrooms)
    .bind(payload.bathrooms)
    .bind(&payload.construction_status)
    .bind(payload.year_built)
    .fetch_one(&mut *tx)
    .await?;

    // Create developer_projects row for status tracking (B5 fix)
    sqlx::query(
        "INSERT INTO developer_projects (developer_id, asset_id, project_name, status)
         VALUES ($1, $2, $3, 'draft')",
    )
    .bind(user.id)
    .bind(asset_id)
    .bind(&payload.title)
    .execute(&mut *tx)
    .await?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit draft creation: {e}");
        AppError::Internal("Failed to save draft".to_string())
    })?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Draft asset created",
        "asset_id": asset_id
    })))
}

/// GET /api/developer/assets/:id — Fetch developer draft by ID securely. Prevents cross-modification.
pub async fn api_developer_asset_detail(
    jar: CookieJar,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Please log in"})),
            )
                .into_response()
        }
    };

    let is_developer = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id WHERE ur.user_id = $1 AND r.name IN ('developer', 'admin', 'super_admin'))",
        user.id
    ).fetch_one(&state.db).await.unwrap_or(Some(false)).unwrap_or(false);

    if !is_developer {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Only developers can view this tool"})),
        )
            .into_response();
    }

    let row = sqlx::query(
        "SELECT a.title, a.slug, a.asset_type, a.property_type, a.location_city as city, a.location_country as country,
                COALESCE(a.total_value_cents,0) as total_value_cents,
                COALESCE(a.token_price_cents,0) as token_price_cents,
                COALESCE(a.tokens_total,0) as tokens_total,
                COALESCE(a.tokens_available,0) as tokens_available,
                a.annual_yield_bps, a.capital_appreciation_bps, a.occupancy_rate_bps,
                a.funding_status, a.description, a.short_description, a.video_url,
                a.location_address, a.location_description, a.google_maps_url,
                a.area, a.lease_type, a.lease_term_years,
                a.land_size_sqm, a.building_size_sqm,
                a.bedrooms, a.bathrooms, a.year_built,
                COALESCE(a.featured,false) as featured,
                COALESCE(a.published,false) as published,
                a.construction_status, a.developer_user_id
         FROM assets a WHERE a.id = $1 AND a.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response()
        }
    };

    use sqlx::Row;
    let owner_id: uuid::Uuid = row.get("developer_user_id");
    if owner_id != user.id {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "You are not authorized to access this asset draft"})),
        )
            .into_response();
    }

    // Project status (for edit mode detection)
    let project_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM developer_projects WHERE asset_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    // Cap table
    let investors: Vec<(String, String, i32, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT COALESCE(up.first_name || ' ' || up.last_name, u.email), u.id::text,
                COALESCE(i.tokens_owned,0), COALESCE(i.purchase_value_cents,0),
                COALESCE(i.current_value_cents,0), COALESCE(i.total_rental_cents,0),
                COALESCE(i.status,'active')
         FROM investments i JOIN users u ON u.id = i.user_id LEFT JOIN user_profiles up ON up.user_id = u.id
         WHERE i.asset_id = $1 ORDER BY i.tokens_owned DESC"
    ).bind(id).fetch_all(&state.db).await.unwrap_or_default();

    // Financial records
    let financials: Vec<(i32, i32, i64, i64, i64, Option<i32>)> = sqlx::query_as(
        "SELECT period_month, period_year, COALESCE(rental_income_cents,0), COALESCE(expenses_cents,0),
                COALESCE(net_income_cents,0), occupancy_rate_bps
         FROM asset_financials WHERE asset_id = $1 ORDER BY period_year, period_month"
    ).bind(id).fetch_all(&state.db).await.unwrap_or_default();

    // Documents
    let docs: Vec<(String, String, Option<i64>, uuid::Uuid)> = sqlx::query_as(
        "SELECT document_type, COALESCE(title, document_type), file_size_bytes, id FROM asset_documents WHERE asset_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Images
    let images: Vec<(String, bool, i32)> = sqlx::query_as(
        "SELECT COALESCE(image_url,''), COALESCE(is_cover,false), COALESCE(sort_order,0) FROM asset_images WHERE asset_id = $1 ORDER BY sort_order"
    ).bind(id).fetch_all(&state.db).await.unwrap_or_default();

    // Milestones
    let milestones: Vec<(String, Option<String>, Option<i32>, bool)> = sqlx::query_as(
        "SELECT title, description, month_index, COALESCE(is_completed,false) FROM asset_milestones WHERE asset_id = $1 ORDER BY month_index"
    ).bind(id).fetch_all(&state.db).await.unwrap_or_default();

    // Orders
    let orders: Vec<(String, String, i32, i64, String, String)> = sqlx::query_as(
        "SELECT o.order_number, COALESCE(u.email,''), oi.tokens_quantity, oi.subtotal_cents,
                COALESCE(o.status,''), o.created_at::text
         FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         JOIN users u ON u.id = o.user_id
         WHERE oi.asset_id = $1
         ORDER BY o.created_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({
        "id": id.to_string(),
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
        "short_description": row.get::<Option<String>, _>("short_description"),
        "video_url": row.get::<Option<String>, _>("video_url"),
        "location_address": row.get::<Option<String>, _>("location_address"),
        "location_description": row.get::<Option<String>, _>("location_description"),
        "google_maps_url": row.get::<Option<String>, _>("google_maps_url"),
        "area": row.get::<Option<String>, _>("area"),
        "lease_type": row.get::<Option<String>, _>("lease_type"),
        "lease_term_years": row.get::<Option<i32>, _>("lease_term_years"),
        "land_size_sqm": row.get::<Option<rust_decimal::Decimal>, _>("land_size_sqm").map(|d| d.to_string()),
        "building_size_sqm": row.get::<Option<rust_decimal::Decimal>, _>("building_size_sqm").map(|d| d.to_string()),
        "bedrooms": row.get::<Option<i32>, _>("bedrooms"),
        "bathrooms": row.get::<Option<i32>, _>("bathrooms"),
        "year_built": row.get::<Option<i32>, _>("year_built"),
        "featured": row.get::<bool, _>("featured"),
        "published": row.get::<bool, _>("published"),
        "construction_status": row.get::<Option<String>, _>("construction_status"),
        "project_status": project_status.unwrap_or_else(|| "draft".to_string()),
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
        "documents": docs.iter().map(|d| serde_json::json!({"document_type": d.0, "title": d.1, "file_size": d.2, "id": d.3})).collect::<Vec<_>>(),
        "images": images.iter().map(|i| {
            let url = crate::storage::service::rewrite_gcs_url(&i.0);
            serde_json::json!({"url": url, "is_cover": i.1, "sort_order": i.2})
        }).collect::<Vec<_>>(),
        "milestones": milestones.iter().map(|m| serde_json::json!({"title": m.0, "description": m.1, "month_index": m.2, "is_completed": m.3})).collect::<Vec<_>>(),
        "orders": orders.iter().map(|o| serde_json::json!({
            "order_number": o.0, "user_email": o.1, "tokens": o.2,
            "subtotal_cents": o.3, "status": o.4, "created_at": o.5
        })).collect::<Vec<_>>(),
    })).into_response()
}

/// PUT /api/developer/draft/:id — Update a draft asset with content from Steps 3/4.
pub async fn api_developer_update_draft(
    jar: CookieJar,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<UpdateDraftAsset>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Please log in".to_string())),
    };

    // Verify ownership and ensure asset is not deleted
    let owner_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if owner_id != Some(user.id) {
        return Err(AppError::Forbidden(
            "Not authorized or asset deleted".to_string(),
        ));
    }

    // Only allow edits on draft assets; approved/live assets must use change request flow
    let project_status: Option<String> =
        sqlx::query_scalar("SELECT dp.status FROM developer_projects dp JOIN assets a ON a.id = dp.asset_id WHERE a.id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    if let Some(ref status) = project_status {
        if status != "draft" && status != "revision_requested" {
            return Err(AppError::BadRequest(format!(
                "Cannot edit asset in '{}' status. Only draft or revision-requested assets can be edited.",
                status
            )));
        }
    }

    // ── XSS Sanitization: sanitize all user-supplied text fields ──
    use crate::common::sanitize::{sanitize_multiline, sanitize_text, sanitize_url};
    let mut payload = payload;
    if let Some(ref v) = payload.title {
        payload.title = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.short_description {
        payload.short_description = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.description {
        payload.description = Some(sanitize_multiline(v));
    }
    if let Some(ref v) = payload.location_description {
        payload.location_description = Some(sanitize_multiline(v));
    }
    if let Some(ref v) = payload.google_maps_url {
        payload.google_maps_url = sanitize_url(v);
    }
    if let Some(ref v) = payload.video_url {
        payload.video_url = sanitize_url(v);
    }
    if let Some(ref v) = payload.property_type {
        payload.property_type = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.area {
        payload.area = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.address {
        payload.address = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.city {
        payload.city = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.country {
        payload.country = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.lease_type {
        payload.lease_type = Some(sanitize_text(v));
    }
    if let Some(ref v) = payload.construction_status {
        payload.construction_status = Some(sanitize_text(v));
    }

    // Build dynamic UPDATE query for only provided fields
    let mut set_clauses: Vec<String> = vec!["updated_at = NOW()".to_string()];
    let mut param_idx = 2u32; // $1 = asset_id

    macro_rules! push_field {
        ($field:expr, $col:expr) => {
            if $field.is_some() {
                set_clauses.push(format!("{}  = ${}", $col, param_idx));
                param_idx += 1;
            }
        };
    }

    push_field!(payload.title, "title");
    push_field!(payload.short_description, "short_description");
    push_field!(payload.description, "description");
    push_field!(payload.location_description, "location_description");
    push_field!(payload.google_maps_url, "google_maps_url");
    push_field!(payload.video_url, "video_url");
    push_field!(payload.annual_yield_bps, "annual_yield_bps");
    push_field!(payload.capital_appreciation_bps, "capital_appreciation_bps");
    push_field!(payload.occupancy_rate_bps, "occupancy_rate_bps");
    push_field!(payload.investor_share_bps, "investor_share_bps");
    push_field!(payload.amenities, "amenities");
    push_field!(payload.submission_step, "submission_step");
    push_field!(payload.property_type, "property_type");
    push_field!(payload.area, "area");
    push_field!(payload.address, "location_address");
    push_field!(payload.city, "location_city");
    push_field!(payload.country, "location_country");
    push_field!(payload.lease_type, "lease_type");
    push_field!(payload.lease_term_years, "lease_term_years");
    push_field!(payload.land_size_sqm, "land_size_sqm");
    push_field!(payload.building_size_sqm, "building_size_sqm");
    push_field!(payload.bedrooms, "bedrooms");
    push_field!(payload.bathrooms, "bathrooms");
    push_field!(payload.construction_status, "construction_status");
    push_field!(payload.year_built, "year_built");
    push_field!(payload.total_value_cents, "total_value_cents");
    push_field!(payload.token_price_cents, "token_price_cents");
    push_field!(payload.tokens_total, "tokens_total");
    if payload.tokens_total.is_some() {
        set_clauses.push(format!("tokens_available = ${}", param_idx - 1));
    }

    let sql = format!("UPDATE assets SET {} WHERE id = $1", set_clauses.join(", "));

    let mut q = sqlx::query(&sql).bind(id);

    // Bind values in the same order as set_clauses
    if let Some(ref v) = payload.title {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.short_description {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.description {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.location_description {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.google_maps_url {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.video_url {
        q = q.bind(v);
    }
    if let Some(v) = payload.annual_yield_bps {
        q = q.bind(v);
    }
    if let Some(v) = payload.capital_appreciation_bps {
        q = q.bind(v);
    }
    if let Some(v) = payload.occupancy_rate_bps {
        q = q.bind(v);
    }
    if let Some(v) = payload.investor_share_bps {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.amenities {
        q = q.bind(v);
    }
    if let Some(v) = payload.submission_step {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.property_type {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.area {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.address {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.city {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.country {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.lease_type {
        q = q.bind(v);
    }
    if let Some(v) = payload.lease_term_years {
        q = q.bind(v);
    }
    if let Some(v) = payload.land_size_sqm {
        q = q.bind(rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
    }
    if let Some(v) = payload.building_size_sqm {
        q = q.bind(rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
    }
    if let Some(v) = payload.bedrooms {
        q = q.bind(v);
    }
    if let Some(v) = payload.bathrooms {
        q = q.bind(v);
    }
    if let Some(ref v) = payload.construction_status {
        q = q.bind(v);
    }
    if let Some(v) = payload.year_built {
        q = q.bind(v);
    }
    if let Some(v) = payload.total_value_cents {
        q = q.bind(v);
    }
    if let Some(v) = payload.token_price_cents {
        q = q.bind(v);
    }
    if let Some(v) = payload.tokens_total {
        q = q.bind(v);
    }

    match q.execute(&state.db).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Failed to update draft {}: {} — SQL: {}", id, e, sql);
            return Err(AppError::Internal(format!("Failed to update draft: {}", e)));
        }
    }

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Draft updated"
    })))
}

/// GET /api/developer/draft/:id — Fetch a single draft for form pre-filling.
pub async fn api_developer_get_draft(
    jar: CookieJar,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Please log in"})),
            )
                .into_response()
        }
    };

    use sqlx::Row;
    let row = match sqlx::query(
        r#"
        SELECT a.id, a.title, COALESCE(a.asset_type, 'real_estate') as asset_type,
               COALESCE(a.submission_step, 1) as submission_step,
               a.short_description, a.description, a.location_description,
               a.google_maps_url, a.video_url,
               a.annual_yield_bps, a.capital_appreciation_bps, a.occupancy_rate_bps,
               a.investor_share_bps,
               a.property_type, a.area, a.location_address,
               a.location_city, a.location_country,
               a.lease_type, a.lease_term_years,
               a.land_size_sqm, a.building_size_sqm, a.bedrooms, a.bathrooms,
               a.construction_status, a.year_built,
               a.total_value_cents, a.token_price_cents, a.tokens_total,
               a.amenities
        FROM assets a
        WHERE a.id = $1 AND a.developer_user_id = $2 AND a.deleted_at IS NULL
        "#,
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Draft not found"})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch draft: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Database error"})),
            )
                .into_response();
        }
    };

    let images: Vec<(uuid::Uuid, String, bool, i32)> = sqlx::query_as(
        "SELECT id, image_url, COALESCE(is_cover, false), COALESCE(sort_order, 0) FROM asset_images WHERE asset_id = $1 ORDER BY sort_order"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let documents: Vec<(uuid::Uuid, String, Option<String>, Option<i64>)> = sqlx::query_as(
        "SELECT id, document_type, title, file_size_bytes FROM asset_documents WHERE asset_id = $1 ORDER BY created_at"
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({
        "id": row.get::<uuid::Uuid, _>("id").to_string(),
        "title": row.get::<Option<String>, _>("title"),
        "asset_type": row.get::<String, _>("asset_type"),
        "submission_step": row.get::<i32, _>("submission_step"),
        "short_description": row.get::<Option<String>, _>("short_description"),
        "description": row.get::<Option<String>, _>("description"),
        "location_description": row.get::<Option<String>, _>("location_description"),
        "google_maps_url": row.get::<Option<String>, _>("google_maps_url"),
        "video_url": row.get::<Option<String>, _>("video_url"),
        "annual_yield_bps": row.get::<Option<i32>, _>("annual_yield_bps"),
        "capital_appreciation_bps": row.get::<Option<i32>, _>("capital_appreciation_bps"),
        "occupancy_rate_bps": row.get::<Option<i32>, _>("occupancy_rate_bps"),
        "investor_share_bps": row.get::<Option<i32>, _>("investor_share_bps"),
        "property_type": row.get::<Option<String>, _>("property_type"),
        "area": row.get::<Option<String>, _>("area"),
        "location_address": row.get::<Option<String>, _>("location_address"),
        "city": row.get::<Option<String>, _>("location_city"),
        "country": row.get::<Option<String>, _>("location_country"),
        "lease_type": row.get::<Option<String>, _>("lease_type"),
        "lease_term_years": row.get::<Option<i32>, _>("lease_term_years"),
        "land_size_sqm": row.get::<Option<rust_decimal::Decimal>, _>("land_size_sqm"),
        "building_size_sqm": row.get::<Option<rust_decimal::Decimal>, _>("building_size_sqm"),
        "bedrooms": row.get::<Option<i32>, _>("bedrooms"),
        "bathrooms": row.get::<Option<i32>, _>("bathrooms"),
        "construction_status": row.get::<Option<String>, _>("construction_status"),
        "year_built": row.get::<Option<i32>, _>("year_built"),
        "total_value_cents": row.get::<Option<i64>, _>("total_value_cents"),
        "token_price_cents": row.get::<Option<i64>, _>("token_price_cents"),
        "tokens_total": row.get::<Option<i32>, _>("tokens_total"),
        "amenities": row.get::<Option<serde_json::Value>, _>("amenities"),
        "images": images.iter().map(|i| serde_json::json!({"id": i.0, "url": crate::storage::service::rewrite_gcs_url(&i.1), "is_cover": i.2, "sort_order": i.3})).collect::<Vec<_>>(),
        "documents": documents.iter().map(|d| serde_json::json!({"id": d.0, "document_type": d.1, "title": d.2, "file_size": d.3})).collect::<Vec<_>>(),
    }))
    .into_response()
}

/// GET /api/developer/drafts — List all drafts/assets for the logged-in developer.
pub async fn api_developer_list_drafts(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Please log in"})),
            )
                .into_response()
        }
    };

    let rows = sqlx::query(
        r#"
        SELECT a.id, a.title, COALESCE(a.asset_type, 'real_estate') as asset_type,
               COALESCE(a.submission_step, 1) as submission_step,
               COALESCE(dp.status, 'draft') as project_status,
               dp.revision_notes,
               a.updated_at::text,
               a.created_at::text,
               (SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY is_cover DESC, sort_order ASC LIMIT 1) as cover_image_url
        FROM assets a
        LEFT JOIN developer_projects dp ON dp.asset_id = a.id
        WHERE a.developer_user_id = $1
          AND a.deleted_at IS NULL
        ORDER BY a.updated_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    use sqlx::Row;
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<uuid::Uuid, _>("id").to_string(),
                "title": row.get::<String, _>("title"),
                "asset_type": row.get::<String, _>("asset_type"),
                "submission_step": row.get::<i32, _>("submission_step"),
                "project_status": row.get::<String, _>("project_status"),
                "revision_notes": row.get::<Option<String>, _>("revision_notes"),
                "updated_at": row.get::<String, _>("updated_at"),
                "created_at": row.get::<String, _>("created_at"),
                "cover_image_url": row.get::<Option<String>, _>("cover_image_url").map(|u| crate::storage::service::rewrite_gcs_url(&u)),
            })
        })
        .collect();

    Json(serde_json::json!({
        "items": items,
        "total": items.len()
    }))
    .into_response()
}

/// POST /api/developer/draft/:id/submit — Mark a draft as submitted for review.
pub async fn api_developer_submit_draft(
    jar: CookieJar,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Please log in".to_string())),
    };

    // Verify ownership and ensure asset is not deleted
    let owner_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if owner_id != Some(user.id) {
        return Err(AppError::Forbidden(
            "Not authorized or asset deleted".to_string(),
        ));
    }

    // Only allow submit from draft or revision_requested
    let current_status: Option<String> = sqlx::query_scalar(
        "SELECT dp.status FROM developer_projects dp WHERE dp.asset_id = $1 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if let Some(ref status) = current_status {
        if status != "draft" && status != "revision_requested" {
            return Err(AppError::BadRequest(format!(
                "Cannot submit from '{}' status. Only draft or revision-requested assets can be submitted.",
                status
            )));
        }
    }

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {e}");
        AppError::Internal("Database error".to_string())
    })?;

    // Ensure the asset has at least one image uploaded before allowing submission
    let image_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM asset_images WHERE asset_id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(0);

    if image_count == 0 {
        let _ = tx.rollback().await;
        return Err(AppError::BadRequest(
            "You must upload at least one image before submitting the asset for review."
                .to_string(),
        ));
    }

    // Update asset submission_step to 5 (submitted)
    sqlx::query("UPDATE assets SET submission_step = 5, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Update developer_projects status to 'submitted' and clear revision_notes
    sqlx::query(
        "UPDATE developer_projects SET status = 'submitted', revision_notes = NULL, updated_at = NOW() WHERE asset_id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit submission: {e}");
        AppError::Internal("Failed to submit".to_string())
    })?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Asset submitted for review"
    })))
}

/// POST /api/developer/draft/:id/duplicate — Clone a draft asset.
pub async fn api_developer_duplicate_draft(
    jar: CookieJar,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Please log in".to_string())),
    };

    // Verify ownership and ensure asset is not deleted
    let owner_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT developer_user_id FROM assets WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if owner_id != Some(user.id) {
        return Err(AppError::Forbidden(
            "Not authorized or asset deleted".to_string(),
        ));
    }

    // ── Enforce 100-draft limit ──
    let draft_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM assets a LEFT JOIN developer_projects dp ON dp.asset_id = a.id WHERE a.developer_user_id = $1 AND a.deleted_at IS NULL AND COALESCE(dp.status, 'draft') = 'draft'"
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    if draft_count >= 100 {
        return Err(AppError::BadRequest(
            "You can have a maximum of 100 drafts. Please delete unused drafts before creating a new one.".to_string(),
        ));
    }

    let new_slug = format!("copy-{}", uuid::Uuid::new_v4());

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {e}");
        AppError::Internal("Database error".to_string())
    })?;

    // Clone the asset row (text fields only, no images/documents)
    let new_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO assets (
            developer_user_id, title, slug, asset_type, total_value_cents,
            token_price_cents, tokens_total, tokens_available, funding_status,
            featured, published, updated_at, submission_step,
            property_type, area, location_address, lease_type, lease_term_years,
            land_size_sqm, building_size_sqm, bedrooms, bathrooms,
            construction_status, year_built,
            short_description, description, location_description,
            google_maps_url, video_url,
            annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
            investor_share_bps, amenities
        )
        SELECT
            developer_user_id, title || ' (Copy)', $2, asset_type, total_value_cents,
            token_price_cents, tokens_total, tokens_total, 'upcoming',
            false, false, NOW(), 1,
            property_type, area, location_address, lease_type, lease_term_years,
            land_size_sqm, building_size_sqm, bedrooms, bathrooms,
            construction_status, year_built,
            short_description, description, location_description,
            google_maps_url, video_url,
            annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
            investor_share_bps, amenities
        FROM assets WHERE id = $1
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(&new_slug)
    .fetch_one(&mut *tx)
    .await?;

    // Create developer_projects row for the clone
    sqlx::query(
        "INSERT INTO developer_projects (developer_id, asset_id, project_name, status)
         SELECT developer_id, $1, project_name || ' (Copy)', 'draft'
         FROM developer_projects WHERE asset_id = $2",
    )
    .bind(new_id)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit duplicate: {e}");
        AppError::Internal("Failed to duplicate".to_string())
    })?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Draft duplicated",
        "new_asset_id": new_id
    })))
}

/// DELETE /api/developer/draft/:id — Soft-delete a draft asset.
pub async fn api_developer_delete_draft(
    jar: CookieJar,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Please log in".to_string())),
    };

    // Verify ownership
    let owner_id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    if owner_id != Some(user.id) {
        return Err(AppError::Forbidden("Not authorized".to_string()));
    }

    // Block deletion of approved/live assets
    let project_status: Option<String> = sqlx::query_scalar(
        "SELECT dp.status FROM developer_projects dp WHERE dp.asset_id = $1 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    if let Some(ref status) = project_status {
        if status == "approved" || status == "live" {
            return Err(AppError::BadRequest(
                "Cannot delete an approved or live asset. Please contact support.".to_string(),
            ));
        }
    }

    // Block deletion of assets with existing investments
    let has_investors: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM investments WHERE asset_id = $1 AND status != 'exited')",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(Some(false))
    .unwrap_or(false);

    if has_investors {
        return Err(AppError::BadRequest(
            "Cannot delete an asset with active investors.".to_string(),
        ));
    }

    // Soft-delete: set deleted_at timestamp
    sqlx::query("UPDATE assets SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Draft deleted"
    })))
}
