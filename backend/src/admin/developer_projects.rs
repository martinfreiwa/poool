use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

// ─────────────────────────────────────────────────────────────────────────────
//  Developer Projects (Canonical Submission Pipeline)
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/admin/developer-projects  Full list of all developer_projects with linked asset + developer info.
pub async fn api_admin_developer_projects(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // Use UNION ALL: first assets WITH developer_projects rows, then orphaned assets WITHOUT.
    // This ensures all assets are visible in the submissions list.
    let rows = sqlx::query(
        r#"SELECT * FROM (
            -- Assets WITH developer_projects rows
            SELECT
               dp.id::text              AS project_id,
               dp.project_name,
               dp.status               AS project_status,
               dp.total_raised_cents,
               dp.investors_count,
               dp.funding_progress_bps,
               dp.created_at::text     AS project_created_at,
               dp.updated_at::text     AS project_updated_at,
               a.id::text              AS asset_id,
               COALESCE(a.title,'')    AS asset_title,
               COALESCE(a.asset_type,'') AS asset_type,
               COALESCE(a.location_city,'') AS location_city,
               COALESCE(a.location_country,'') AS location_country,
               COALESCE(a.total_value_cents,0)    AS total_value_cents,
               COALESCE(a.token_price_cents,0)    AS token_price_cents,
               COALESCE(a.tokens_total,0)         AS tokens_total,
               COALESCE(a.published,false)        AS published,
               COALESCE(a.funding_status,'upcoming') AS funding_status,
               dp.developer_id::text   AS developer_user_id,
               COALESCE(u.email,'')    AS developer_email,
               COALESCE(up.first_name,'') AS dev_first,
               COALESCE(up.last_name,'')  AS dev_last,
               (SELECT kr.status FROM kyc_records kr
                WHERE kr.user_id = dp.developer_id
                ORDER BY kr.created_at DESC LIMIT 1) AS kyc_status,
               (SELECT COUNT(*) FROM developer_projects dp2
                WHERE dp2.developer_id = dp.developer_id) AS other_projects_count
           FROM developer_projects dp
           LEFT JOIN assets a ON a.id = dp.asset_id
           LEFT JOIN users u ON u.id = dp.developer_id
           LEFT JOIN user_profiles up ON up.user_id = dp.developer_id

            UNION ALL

            -- Orphaned assets WITHOUT developer_projects rows
            SELECT
               a.id::text              AS project_id,
               a.title                 AS project_name,
               CASE
                 WHEN a.published = true AND a.funding_status IN ('funding_open','funding_in_progress') THEN 'live'
                 WHEN a.published = true AND a.funding_status IN ('funded','exited') THEN 'approved'
                 ELSE 'draft'
               END                     AS project_status,
               0::bigint               AS total_raised_cents,
               0                       AS investors_count,
               CASE WHEN a.tokens_total > 0
                 THEN ((a.tokens_total - a.tokens_available)::bigint * 10000 / a.tokens_total)::int
                 ELSE 0
               END                     AS funding_progress_bps,
               a.created_at::text      AS project_created_at,
               a.updated_at::text      AS project_updated_at,
               a.id::text              AS asset_id,
               COALESCE(a.title,'')    AS asset_title,
               COALESCE(a.asset_type,'') AS asset_type,
               COALESCE(a.location_city,'') AS location_city,
               COALESCE(a.location_country,'') AS location_country,
               COALESCE(a.total_value_cents,0)    AS total_value_cents,
               COALESCE(a.token_price_cents,0)    AS token_price_cents,
               COALESCE(a.tokens_total,0)         AS tokens_total,
               COALESCE(a.published,false)        AS published,
               COALESCE(a.funding_status,'upcoming') AS funding_status,
               a.developer_user_id::text AS developer_user_id,
               COALESCE(u.email,'')    AS developer_email,
               COALESCE(up.first_name,'') AS dev_first,
               COALESCE(up.last_name,'')  AS dev_last,
               (SELECT kr.status FROM kyc_records kr
                WHERE kr.user_id = a.developer_user_id
                ORDER BY kr.created_at DESC LIMIT 1) AS kyc_status,
               0::bigint AS other_projects_count
           FROM assets a
           LEFT JOIN users u ON u.id = a.developer_user_id
           LEFT JOIN user_profiles up ON up.user_id = a.developer_user_id
           WHERE a.deleted_at IS NULL
             AND NOT EXISTS (SELECT 1 FROM developer_projects dp WHERE dp.asset_id = a.id)
        ) combined
        ORDER BY project_created_at DESC
        LIMIT 500"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let projects: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let first: String = r.try_get("dev_first").unwrap_or_default();
            let last: String = r.try_get("dev_last").unwrap_or_default();
            let dev_email: String = r.try_get("developer_email").unwrap_or_default();
            let dev_name = format!("{} {}", first, last).trim().to_string();

            serde_json::json!({
                // Project
                "id": r.get::<String, _>("project_id"),
                "project_name": r.try_get::<String, _>("project_name").unwrap_or_default(),
                "status": r.get::<String, _>("project_status"),
                "total_raised_cents": r.get::<i64, _>("total_raised_cents"),
                "investors_count": r.get::<i32, _>("investors_count"),
                "funding_progress_bps": r.get::<i32, _>("funding_progress_bps"),
                "created_at": r.get::<String, _>("project_created_at"),
                "updated_at": r.get::<String, _>("project_updated_at"),
                // Asset
                "asset_id": r.get::<Option<String>, _>("asset_id"),
                "title": r.try_get::<String, _>("asset_title").unwrap_or_default(),
                "asset_type": r.try_get::<String, _>("asset_type").unwrap_or_default(),
                "location_city": r.try_get::<String, _>("location_city").unwrap_or_default(),
                "location_country": r.try_get::<String, _>("location_country").unwrap_or_default(),
                "total_value_cents": r.get::<i64, _>("total_value_cents"),
                "token_price_cents": r.get::<i64, _>("token_price_cents"),
                "tokens_total": r.get::<i32, _>("tokens_total"),
                "published": r.get::<bool, _>("published"),
                "funding_status": r.get::<String, _>("funding_status"),
                // Developer
                "developer_user_id": r.get::<Option<String>, _>("developer_user_id"),
                "developer_email": &dev_email,
                "developer_name": if dev_name.is_empty() { dev_email.clone() } else { dev_name },
                "kyc_status": r.get::<Option<String>, _>("kyc_status"),
                "other_projects_count": r.get::<i64, _>("other_projects_count"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "projects": projects })).into_response())
}

/// GET /api/admin/developer-projects/:project_id  Full detail: project + asset + developer + docs + images + milestones.
/// Side-effect: if status = 'submitted', auto-transitions to 'in_review' and logs to audit_logs.
pub async fn api_admin_developer_project_detail(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = Some(_admin.user.clone());

    let pid = ApiError::parse_uuid(&project_id)?;

    // Fetch the project + linked asset + developer info in one query
    let row = sqlx::query(
        r#"SELECT
               dp.id::text              AS project_id,
               dp.project_name,
               dp.status               AS project_status,
               dp.total_raised_cents,
               dp.investors_count,
               dp.funding_progress_bps,
               dp.developer_id::text   AS developer_user_id,
               dp.created_at::text     AS project_created_at,
               dp.updated_at::text     AS project_updated_at,
               -- Asset
               a.id::text              AS asset_id,
               COALESCE(a.title,'')    AS title,
               COALESCE(a.slug,'')     AS slug,
               a.description,
               a.short_description,
               COALESCE(a.asset_type,'') AS asset_type,
               a.property_type,
               a.area,
               a.lease_type,
               a.lease_term_years,
               a.land_size_sqm,
               a.building_size_sqm,
               a.bedrooms,
               a.bathrooms,
               a.construction_status,
               a.year_built,
               COALESCE(a.location_city,'')    AS location_city,
               COALESCE(a.location_country,'') AS location_country,
               a.location_address,
               a.location_lat,
               a.location_lng,
               a.location_description,
               a.google_maps_url,
               a.video_url,
               COALESCE(a.total_value_cents,0)  AS total_value_cents,
               COALESCE(a.token_price_cents,0)  AS token_price_cents,
               COALESCE(a.tokens_total,0)       AS tokens_total,
               COALESCE(a.tokens_available,0)   AS tokens_available,
               a.annual_yield_bps,
               a.capital_appreciation_bps,
               a.occupancy_rate_bps,
               a.operator_name,
               a.term_months,
               a.fixed_roi_bps,
               a.revenue_min_cents,
               a.revenue_max_cents,
               a.expenses_cents,
               a.net_profit_min_cents,
               a.net_profit_max_cents,
               a.investor_payout_cents,
               a.operator_split_pct,
               a.poool_split_pct,
               COALESCE(a.funding_status,'upcoming') AS funding_status,
               COALESCE(a.published,false)           AS published,
               a.funding_start_at::text,
               a.funding_end_at::text,
               -- Developer
               COALESCE(u.email,'')    AS developer_email,
               COALESCE(up.first_name,'') AS dev_first,
               COALESCE(up.last_name,'')  AS dev_last,
               (SELECT kr.status FROM kyc_records kr
                WHERE kr.user_id = dp.developer_id
                ORDER BY kr.created_at DESC LIMIT 1) AS kyc_status,
               (SELECT COUNT(*) FROM developer_projects dp2
                WHERE dp2.developer_id = dp.developer_id) AS other_projects_count
           FROM developer_projects dp
           LEFT JOIN assets a ON a.id = dp.asset_id
           LEFT JOIN users u ON u.id = dp.developer_id
           LEFT JOIN user_profiles up ON up.user_id = dp.developer_id
           WHERE dp.id = $1"#,
    )
    .bind(pid)
    .fetch_optional(&state.db)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err(ApiError::NotFound("Project not found".to_string()));
        }
        Err(e) => {
            tracing::error!("Failed to fetch developer project {project_id}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    };

    let project_status: String = row.get("project_status");
    let project_id_str: String = row.get("project_id");
    let developer_user_id: String = row.get("developer_user_id");
    let was_transitioned = project_status == "submitted";

    // Auto-transition submitted → in_review when admin opens the detail
    if was_transitioned {
        let _ = sqlx::query(
            "UPDATE developer_projects SET status = 'in_review', updated_at = NOW() WHERE id = $1",
        )
        .bind(pid)
        .execute(&state.db)
        .await;

        // Audit log the auto-transition
        let _ = sqlx::query(
            r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
               VALUES ($1, 'developer_project.review_started', 'developer_projects', $2,
                       '{"status":"submitted"}', '{"status":"in_review"}')"#
        )
        .bind(admin_user.as_ref().map(|u| u.id))
        .bind(pid)
        .execute(&state.db)
        .await;
    }

    // Fetch the asset_id for related data queries
    let asset_id_opt: Option<uuid::Uuid> = {
        let s: Option<String> = row.get("asset_id");
        s.and_then(|s| s.parse().ok())
    };

    // Fetch documents (with file_url for viewing)
    let docs: Vec<(String, String, String, String, Option<i64>, String)> =
        if let Some(aid) = asset_id_opt {
            sqlx::query_as(
                r#"SELECT id::text, document_type, COALESCE(title,''), file_url,
                      file_size_bytes, created_at::text
               FROM asset_documents WHERE asset_id = $1 ORDER BY document_type, created_at"#,
            )
            .bind(aid)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
        } else {
            vec![]
        };

    // Fetch images (using correct column: image_url)
    let images: Vec<(String, String, Option<String>, bool, i32)> = if let Some(aid) = asset_id_opt {
        sqlx::query_as(
            r#"SELECT id::text, image_url, alt_text, is_cover, sort_order
               FROM asset_images WHERE asset_id = $1 ORDER BY is_cover DESC, sort_order ASC"#,
        )
        .bind(aid)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    // Fetch milestones (including milestone_date)
    #[allow(clippy::type_complexity)]
    let milestones: Vec<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<i32>,
        bool,
    )> = if let Some(aid) = asset_id_opt {
        sqlx::query_as(
            r#"SELECT id::text, title, description, milestone_date::text, month_index, is_completed
               FROM asset_milestones WHERE asset_id = $1 ORDER BY COALESCE(month_index, 9999), milestone_date"#
        )
        .bind(aid)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let first: String = row.get("dev_first");
    let last: String = row.get("dev_last");
    let dev_email: String = row.get("developer_email");
    let dev_name = format!("{} {}", first, last).trim().to_string();

    Ok(Json(serde_json::json!({
        "project": {
            "id": project_id_str,
            "project_name": row.get::<String, _>("project_name"),
            "status": if was_transitioned { "in_review".to_string() } else { project_status },
            "total_raised_cents": row.get::<i64, _>("total_raised_cents"),
            "investors_count": row.get::<i32, _>("investors_count"),
            "funding_progress_bps": row.get::<i32, _>("funding_progress_bps"),
            "created_at": row.get::<String, _>("project_created_at"),
            "updated_at": row.get::<String, _>("project_updated_at"),
            "was_transitioned_to_in_review": was_transitioned,
        },
        "developer": {
            "user_id": developer_user_id,
            "email": &dev_email,
            "first_name": &first,
            "last_name": &last,
            "name": if dev_name.is_empty() { dev_email.clone() } else { dev_name },
            "kyc_status": row.get::<Option<String>, _>("kyc_status"),
            "other_projects_count": row.get::<i64, _>("other_projects_count"),
        },
        "asset": {
            "id": row.get::<Option<String>, _>("asset_id"),
            "title": row.get::<String, _>("title"),
            "slug": row.get::<String, _>("slug"),
            "description": row.get::<Option<String>, _>("description"),
            "short_description": row.get::<Option<String>, _>("short_description"),
            "asset_type": row.get::<String, _>("asset_type"),
            "property_type": row.get::<Option<String>, _>("property_type"),
            "area": row.get::<Option<String>, _>("area"),
            "lease_type": row.get::<Option<String>, _>("lease_type"),
            "lease_term_years": row.get::<Option<i32>, _>("lease_term_years"),
            "land_size_sqm": row.get::<Option<rust_decimal::Decimal>, _>("land_size_sqm").map(|d| d.to_string()),
            "building_size_sqm": row.get::<Option<rust_decimal::Decimal>, _>("building_size_sqm").map(|d| d.to_string()),
            "bedrooms": row.get::<Option<i32>, _>("bedrooms"),
            "bathrooms": row.get::<Option<i32>, _>("bathrooms"),
            "construction_status": row.get::<Option<String>, _>("construction_status"),
            "year_built": row.get::<Option<i32>, _>("year_built"),
            "location_city": row.get::<String, _>("location_city"),
            "location_country": row.get::<String, _>("location_country"),
            "location_address": row.get::<Option<String>, _>("location_address"),
            "location_description": row.get::<Option<String>, _>("location_description"),
            "google_maps_url": row.get::<Option<String>, _>("google_maps_url"),
            "video_url": row.get::<Option<String>, _>("video_url"),
            "total_value_cents": row.get::<i64, _>("total_value_cents"),
            "token_price_cents": row.get::<i64, _>("token_price_cents"),
            "tokens_total": row.get::<i32, _>("tokens_total"),
            "tokens_available": row.get::<i32, _>("tokens_available"),
            "annual_yield_bps": row.get::<Option<i32>, _>("annual_yield_bps"),
            "capital_appreciation_bps": row.get::<Option<i32>, _>("capital_appreciation_bps"),
            "occupancy_rate_bps": row.get::<Option<i32>, _>("occupancy_rate_bps"),
            // Commodity-specific
            "operator_name": row.get::<Option<String>, _>("operator_name"),
            "term_months": row.get::<Option<i32>, _>("term_months"),
            "fixed_roi_bps": row.get::<Option<i32>, _>("fixed_roi_bps"),
            "revenue_min_cents": row.get::<Option<i64>, _>("revenue_min_cents"),
            "revenue_max_cents": row.get::<Option<i64>, _>("revenue_max_cents"),
            "expenses_cents": row.get::<Option<i64>, _>("expenses_cents"),
            "net_profit_min_cents": row.get::<Option<i64>, _>("net_profit_min_cents"),
            "net_profit_max_cents": row.get::<Option<i64>, _>("net_profit_max_cents"),
            "investor_payout_cents": row.get::<Option<i64>, _>("investor_payout_cents"),
            "operator_split_pct": row.get::<Option<i32>, _>("operator_split_pct"),
            "poool_split_pct": row.get::<Option<i32>, _>("poool_split_pct"),
            "funding_status": row.get::<String, _>("funding_status"),
            "published": row.get::<bool, _>("published"),
            "funding_start_at": row.get::<Option<String>, _>("funding_start_at"),
            "funding_end_at": row.get::<Option<String>, _>("funding_end_at"),
        },
        "documents": docs.iter().map(|d| serde_json::json!({
            "id": d.0,
            "document_type": d.1,
            "title": d.2,
            "file_url": d.3,
            "file_size_bytes": d.4,
            "created_at": d.5,
        })).collect::<Vec<_>>(),
        "images": images.iter().map(|i| serde_json::json!({
            "id": i.0,
            "image_url": i.1,
            "alt_text": i.2,
            "is_cover": i.3,
            "sort_order": i.4,
        })).collect::<Vec<_>>(),
        "milestones": milestones.iter().map(|m| serde_json::json!({
            "id": m.0,
            "title": m.1,
            "description": m.2,
            "milestone_date": m.3,
            "month_index": m.4,
            "is_completed": m.5,
        })).collect::<Vec<_>>(),
    })).into_response())
}

/// POST /api/admin/developer-projects/:project_id/review
/// Payload: { "action": "approve"|"reject"|"in_review"|"request_revision", "notes": "..." }
/// This is an alias for the notes/review endpoint — delegates all logic there.
pub async fn api_admin_developer_project_review(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    // Delegate to the full review handler (api_admin_project_notes_create)
    api_admin_project_notes_create(
        admin,
        State(state),
        axum::extract::Path(id),
        Json(payload),
    )
    .await
}

// ==============================================================================
// Developer Project Notes
// ==============================================================================

/// GET /api/admin/developer-projects/:id/notes — list all notes for a project
pub async fn api_admin_project_notes_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let pid = ApiError::parse_uuid(&project_id)?;

    let rows = sqlx::query(
        r#"SELECT
               n.id::text,
               n.content,
               n.created_at::text,
               n.author_id::text,
               COALESCE(up.first_name || ' ' || up.last_name, u.email) AS author_name,
               u.email AS author_email
           FROM developer_project_notes n
           JOIN users u ON u.id = n.author_id
           LEFT JOIN user_profiles up ON up.user_id = n.author_id
           WHERE n.project_id = $1
           ORDER BY n.created_at DESC"#,
    )
    .bind(pid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let notes: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "content": r.get::<String, _>("content"),
                "created_at": r.get::<String, _>("created_at"),
                "author_id": r.get::<String, _>("author_id"),
                "author_name": r.get::<String, _>("author_name"),
                "author_email": r.get::<String, _>("author_email"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({"notes": notes})).into_response())
}

/// POST /api/admin/developer-projects/:id/notes — add a note to a project
pub async fn api_admin_project_notes_create(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = Some(_admin.user.clone());

    let pid = ApiError::parse_uuid(&project_id)?;

    let action = match body.get("action").and_then(|v| v.as_str()) {
        Some(a) if matches!(a, "approve" | "reject" | "request_revision" | "in_review") => {
            a.to_string()
        }
        Some(other) => return Err(ApiError::BadRequest(format!("Unknown action: {other}"))),
        None => {
            return Err(ApiError::BadRequest(
                "Action is required (approve | reject | request_revision | in_review)".to_string(),
            ))
        }
    };

    let notes = body
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    // Notes required for reject and request_revision (not for approve or in_review)
    if (action == "reject" || action == "request_revision") && notes.is_empty() {
        return Err(ApiError::BadRequest(
            "Notes are required for rejection and revision requests".to_string(),
        ));
    }

    // Fetch the project to get developer_id, asset_id, project_name, and current status
    let project_row = sqlx::query(
        "SELECT developer_id, asset_id, project_name, status FROM developer_projects WHERE id = $1",
    )
    .bind(pid)
    .fetch_optional(&state.db)
    .await;

    let project_row = match project_row {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err(ApiError::NotFound("Project not found".to_string()));
        }
        Err(e) => {
            tracing::error!("Failed to fetch project {project_id}: {e}");
            return Err(ApiError::BadRequest(format!(
                "Database error fetching project: {e}"
            )));
        }
    };

    let developer_id: uuid::Uuid = project_row.get("developer_id");
    let asset_id: Option<uuid::Uuid> = project_row.get("asset_id");
    let project_name: String = project_row.get("project_name");
    let previous_status: String = project_row.get("status");

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to start transaction: {e}");
            return Err(ApiError::BadRequest(format!(
                "Database transaction error: {e}"
            )));
        }
    };

    match action.as_str() {
        "approve" => {
            // 1. Update developer_projects status
            let result = sqlx::query(
                "UPDATE developer_projects SET status = 'approved', updated_at = NOW() WHERE id = $1"
            )
            .bind(pid)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to update project status to approved: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to update project status: {e}"
                )));
            }

            // 2. Publish the asset
            if let Some(aid) = asset_id {
                let result = sqlx::query(
                    "UPDATE assets SET published = TRUE, funding_status = CASE WHEN funding_status IN ('funded', 'exited') THEN funding_status ELSE 'funding_open' END, updated_at = NOW() WHERE id = $1"
                )
                .bind(aid)
                .execute(&mut *tx)
                .await;

                if let Err(e) = result {
                    tracing::error!("Failed to publish asset {aid}: {e}");
                    return Err(ApiError::BadRequest(format!(
                        "Failed to publish asset: {e}"
                    )));
                }
            }

            // 3. Notify developer
            let msg = format!(
                "Congratulations! Your project \"{}\" has been approved and is now live on the marketplace.",
                project_name
            );
            let result = sqlx::query(
                r#"INSERT INTO notifications (user_id, title, message, type, action_url)
                   VALUES ($1, 'Project Approved! 🎉', $2, 'investment', '/developer/assets')"#,
            )
            .bind(developer_id)
            .bind(&msg)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!(
                    "Failed to create approval notification for developer {developer_id}: {e}"
                );
                return Err(ApiError::BadRequest(format!(
                    "Failed to create notification: {e}"
                )));
            }

            // 4. Audit log
            let result = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
                   VALUES ($1, 'developer_project.approved', 'developer_projects', $2, $3, $4)"#
            )
            .bind(admin_user.as_ref().map(|u| u.id))
            .bind(pid)
            .bind(serde_json::json!({"status": previous_status}))
            .bind(serde_json::json!({"status": "approved", "asset_published": true, "funding_status": "funding_open"}))
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create audit log for project approval {pid}: {e}");
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                tracing::error!("Failed to commit project approval transaction: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to commit approval transaction: {e}"
                )));
            }

            Ok(Json(serde_json::json!({
                "status": "approved",
                "message": "Project approved and published to marketplace. Developer has been notified."
            })).into_response())
        }

        "reject" => {
            // 1. Update developer_projects status
            let result = sqlx::query(
                "UPDATE developer_projects SET status = 'rejected', updated_at = NOW() WHERE id = $1"
            )
            .bind(pid)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to update project status to rejected: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to update project status: {e}"
                )));
            }

            // 2. Ensure asset is NOT published
            if let Some(aid) = asset_id {
                let result = sqlx::query(
                    "UPDATE assets SET published = FALSE, updated_at = NOW() WHERE id = $1",
                )
                .bind(aid)
                .execute(&mut *tx)
                .await;

                if let Err(e) = result {
                    tracing::error!("Failed to unpublish asset {aid}: {e}");
                }
            }

            // 3. Notify developer with rejection reason
            let msg = format!(
                "Your project \"{}\" has been rejected. Reason: {}. Please contact support if you have questions.",
                project_name, notes
            );
            let result = sqlx::query(
                r#"INSERT INTO notifications (user_id, title, message, type, action_url)
                   VALUES ($1, 'Project Submission Rejected', $2, 'system', '/developer/assets')"#,
            )
            .bind(developer_id)
            .bind(&msg)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create rejection notification: {e}");
            }

            // 4. Record as an admin note for history
            if let Some(ref admin) = admin_user {
                let result = sqlx::query(
                    r#"INSERT INTO developer_project_notes (project_id, author_id, content) VALUES ($1, $2, $3)"#,
                )
                .bind(pid)
                .bind(admin.id)
                .bind(format!("[Decision: Rejected] {}", notes))
                .execute(&mut *tx)
                .await;

                if let Err(e) = result {
                    tracing::error!("Failed to create rejection note: {e}");
                }
            }

            // 5. Audit log (notes stored in new_state)
            let result = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
                   VALUES ($1, 'developer_project.rejected', 'developer_projects', $2, $3, $4)"#
            )
            .bind(admin_user.as_ref().map(|u| u.id))
            .bind(pid)
            .bind(serde_json::json!({"status": previous_status}))
            .bind(serde_json::json!({"status": "rejected", "rejection_reason": &notes}))
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create rejection audit log: {e}");
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                tracing::error!("Failed to commit project rejection: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to commit rejection: {e}"
                )));
            }

            Ok(Json(serde_json::json!({
                "status": "rejected",
                "message": "Project rejected. Developer has been notified with the reason provided."
            }))
            .into_response())
        }

        "request_revision" => {
            // 1. Return to 'submitted' so it goes back to queue
            let result = sqlx::query(
                "UPDATE developer_projects SET status = 'submitted', updated_at = NOW() WHERE id = $1"
            )
            .bind(pid)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to update project status for revision: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to update project status: {e}"
                )));
            }

            // 2. Notify developer with revision notes
            let msg = format!(
                "Your project \"{}\" requires revisions before it can be approved. Admin notes: {}",
                project_name, notes
            );
            let result = sqlx::query(
                r#"INSERT INTO notifications (user_id, title, message, type, action_url)
                   VALUES ($1, 'Revision Required for Your Project', $2, 'system', '/developer/assets')"#
            )
            .bind(developer_id)
            .bind(&msg)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create revision notification: {e}");
            }

            // 3. Record as an admin note for history
            if let Some(ref admin) = admin_user {
                let result = sqlx::query(
                    r#"INSERT INTO developer_project_notes (project_id, author_id, content) VALUES ($1, $2, $3)"#,
                )
                .bind(pid)
                .bind(admin.id)
                .bind(format!("[Decision: Revision Requested] {}", notes))
                .execute(&mut *tx)
                .await;

                if let Err(e) = result {
                    tracing::error!("Failed to create revision note: {e}");
                }
            }

            // 4. Audit log
            let result = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
                   VALUES ($1, 'developer_project.revision_requested', 'developer_projects', $2, $3, $4)"#
            )
            .bind(admin_user.as_ref().map(|u| u.id))
            .bind(pid)
            .bind(serde_json::json!({"status": previous_status}))
            .bind(serde_json::json!({"status": "submitted", "revision_notes": &notes}))
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create revision audit log: {e}");
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                tracing::error!("Failed to commit revision request: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to commit revision: {e}"
                )));
            }

            Ok(Json(serde_json::json!({
                "status": "revision_requested",
                "message": "Revision request sent to developer. Project returned to submissions queue."
            })).into_response())
        }

        "in_review" => {
            // 1. Update developer_projects status
            let result = sqlx::query(
                "UPDATE developer_projects SET status = 'in_review', updated_at = NOW() WHERE id = $1"
            )
            .bind(pid)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to update project status to in_review: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to update project status: {e}"
                )));
            }

            // 2. Notify developer
            let msg = format!(
                "Your project \"{}\" is now under review by our team.",
                project_name
            );
            let result = sqlx::query(
                r#"INSERT INTO notifications (user_id, title, message, type, action_url)
                   VALUES ($1, 'Project Review Started', $2, 'system', '/developer/assets')"#,
            )
            .bind(developer_id)
            .bind(&msg)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create in_review notification: {e}");
            }

            // 3. Audit log
            let result = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
                   VALUES ($1, 'developer_project.review_started_manual', 'developer_projects', $2, $3, $4)"#
            )
            .bind(admin_user.as_ref().map(|u| u.id))
            .bind(pid)
            .bind(serde_json::json!({"status": previous_status}))
            .bind(serde_json::json!({"status": "in_review", "notes": &notes}))
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create in_review audit log: {e}");
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                tracing::error!("Failed to commit in_review update: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to commit in_review: {e}"
                )));
            }

            Ok(Json(serde_json::json!({
                "status": "in_review",
                "message": "Project status changed to In Review. Developer has been notified."
            }))
            .into_response())
        }

        _ => unreachable!(),
    }
}
