use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::common::sanitize;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

const SUBMISSIONS_REVIEW_PERMISSION: &str = "submissions.review";
const SUBMISSIONS_APPROVE_PERMISSION: &str = "submissions.approve";
const REQUIRED_APPROVAL_CHECKS: &[&str] = &[
    "chk-kyc",
    "chk-legal",
    "chk-tax",
    "chk-fin",
    "chk-spv",
    "chk-math",
    "chk-loc",
    "chk-fields",
];

// ─────────────────────────────────────────────────────────────────────────────
//  Developer Projects (Canonical Submission Pipeline)
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/admin/developer-projects  Full list of all developer_projects with linked asset + developer info.
///
/// Query params:
///   include_test=1  → include rows flagged is_test (excluded by default)
pub async fn api_admin_developer_projects(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(qs): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;

    let include_test = matches!(
        qs.get("include_test").map(|s| s.as_str()),
        Some("1") | Some("true")
    );

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
               COALESCE(dp.is_test, false)         AS is_test,
               dp.assigned_admin_id::text          AS assigned_admin_id,
               (SELECT COALESCE(up2.first_name || ' ' || up2.last_name, u2.email)
                FROM users u2 LEFT JOIN user_profiles up2 ON up2.user_id = u2.id
                WHERE u2.id = dp.assigned_admin_id)  AS assigned_admin_name,
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
           WHERE ($1::boolean OR COALESCE(dp.is_test, false) = false)

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
               false                   AS is_test,
               NULL::text              AS assigned_admin_id,
               NULL::text              AS assigned_admin_name,
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
    .bind(include_test)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch developer projects list: {e}");
        ApiError::Database(e)
    })?;

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
                // Admin workflow fields (migration 102)
                "is_test": r.try_get::<bool, _>("is_test").unwrap_or(false),
                "assigned_admin_id": r.try_get::<Option<String>, _>("assigned_admin_id").unwrap_or(None),
                "assigned_admin_name": r.try_get::<Option<String>, _>("assigned_admin_name").unwrap_or(None),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "projects": projects })).into_response())
}

/// GET /api/admin/developer-projects/:project_id  Full detail: project + asset + developer + docs + images + milestones.
/// Side-effect: if status = 'submitted', auto-transitions to 'in_review' and logs to audit_logs.
pub async fn api_admin_developer_project_detail(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;
    let admin_user = Some(admin.user.clone());

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
        let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

        let updated = sqlx::query(
            "UPDATE developer_projects SET status = 'in_review', updated_at = NOW() WHERE id = $1 AND status = 'submitted'",
        )
        .bind(pid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        if updated.rows_affected() == 0 {
            return Err(ApiError::Conflict(
                "Project status changed while starting review".to_string(),
            ));
        }

        sqlx::query(
            r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
               VALUES ($1, 'developer_project.review_started', 'developer_projects', $2,
                       '{"status":"submitted"}', '{"status":"in_review"}')"#,
        )
        .bind(admin_user.as_ref().map(|u| u.id))
        .bind(pid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        tx.commit().await.map_err(ApiError::Database)?;
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
    api_admin_project_notes_create(admin, State(state), axum::extract::Path(id), Json(payload))
        .await
}

// ==============================================================================
// Developer Project Notes
// ==============================================================================

/// GET /api/admin/developer-projects/:id/notes — list all notes for a project
pub async fn api_admin_project_notes_list(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;

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
    .map_err(|e| {
        tracing::error!("Failed to fetch project notes for {project_id}: {e}");
        ApiError::Database(e)
    })?;

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

/// POST /api/admin/developer-projects/:id/notes — add a note or review decision to a project
pub async fn api_admin_project_notes_create(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;
    let admin_user = Some(admin.user.clone());

    let pid = ApiError::parse_uuid(&project_id)?;

    let action = match body.get("action").and_then(|v| v.as_str()) {
        Some(a) if matches!(a, "approve" | "reject" | "request_revision" | "in_review") => {
            Some(a.to_string())
        }
        Some(other) => return Err(ApiError::BadRequest(format!("Unknown action: {other}"))),
        None => None,
    };

    let raw_notes = body
        .get("notes")
        .or_else(|| body.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let notes = sanitize::sanitize_multiline(raw_notes);

    if notes.chars().count() > 5000 {
        return Err(ApiError::BadRequest(
            "Note must be 5000 characters or less".to_string(),
        ));
    }

    // Notes required for reject and request_revision (not for approve or in_review)
    if matches!(action.as_deref(), Some("reject" | "request_revision")) && notes.is_empty() {
        return Err(ApiError::BadRequest(
            "Notes are required for rejection and revision requests".to_string(),
        ));
    }

    if action.is_none() && notes.is_empty() {
        return Err(ApiError::BadRequest("Note content is required".to_string()));
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

    if action.is_none() {
        let note_row = sqlx::query(
            r#"INSERT INTO developer_project_notes (project_id, author_id, content)
               VALUES ($1, $2, $3)
               RETURNING id::text, content, created_at::text"#,
        )
        .bind(pid)
        .bind(admin_user.as_ref().map(|u| u.id))
        .bind(&notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create admin note for project {pid}: {e}");
            ApiError::BadRequest(format!("Failed to create note: {e}"))
        })?;

        let note_id: String = note_row.get("id");
        let note_content: String = note_row.get("content");
        let created_at: String = note_row.get("created_at");

        if let Err(e) = sqlx::query(
            r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
               VALUES ($1, 'developer_project.note_created', 'developer_projects', $2, $3, $4)"#,
        )
        .bind(admin_user.as_ref().map(|u| u.id))
        .bind(pid)
        .bind(serde_json::json!({"status": previous_status}))
        .bind(serde_json::json!({"note_id": note_id}))
        .execute(&mut *tx)
        .await
        {
            tracing::error!("Failed to create admin note audit log for project {pid}: {e}");
            return Err(ApiError::Database(e));
        }

        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit admin note transaction for project {pid}: {e}");
            ApiError::BadRequest(format!("Failed to commit note: {e}"))
        })?;

        return Ok(Json(serde_json::json!({
            "id": note_id,
            "content": note_content,
            "created_at": created_at,
            "status": "created"
        }))
        .into_response());
    }

    let Some(action) = action else {
        return Err(ApiError::BadRequest(
            "Action is required for project review decisions".to_string(),
        ));
    };

    match action.as_str() {
        "approve" => {
            admin
                .require_permission(&state.db, SUBMISSIONS_APPROVE_PERMISSION)
                .await?;

            validate_project_ready_for_approval(&state, pid).await?;

            // 1. Update developer_projects status
            let result = sqlx::query(
                "UPDATE developer_projects SET status = 'approved', updated_at = NOW() WHERE id = $1 AND status IN ('submitted', 'in_review')"
            )
            .bind(pid)
            .execute(&mut *tx)
            .await;

            match result {
                Ok(r) if r.rows_affected() == 0 => {
                    return Err(ApiError::Conflict(
                        "Project must be submitted or in review before approval".to_string(),
                    ));
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to update project status to approved: {e}");
                    return Err(ApiError::BadRequest(format!(
                        "Failed to update project status: {e}"
                    )));
                }
            }

            // 2. Publish the asset
            let aid = asset_id.ok_or_else(|| {
                ApiError::BadRequest("Cannot approve project without a linked asset".to_string())
            })?;
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
                return Err(ApiError::Database(e));
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
                    return Err(ApiError::Database(e));
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
                return Err(ApiError::Database(e));
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
                    return Err(ApiError::Database(e));
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
                return Err(ApiError::Database(e));
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
            // 1. Set status to 'revision_requested' and store the revision notes
            let result = sqlx::query(
                "UPDATE developer_projects SET status = 'revision_requested', revision_notes = $2, updated_at = NOW() WHERE id = $1"
            )
            .bind(pid)
            .bind(&notes)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to update project status for revision: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to update project status: {e}"
                )));
            }

            // 2. Notify developer with revision notes (in-app)
            let msg = format!(
                "Your project \"{}\" requires revisions before it can be approved. Reason: {}",
                project_name, notes
            );
            let result = sqlx::query(
                r#"INSERT INTO notifications (user_id, title, message, type, action_url)
                   VALUES ($1, 'Revision Required for Your Project', $2, 'system', '/developer/submissions')"#
            )
            .bind(developer_id)
            .bind(&msg)
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create revision notification: {e}");
                return Err(ApiError::Database(e));
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
                    return Err(ApiError::Database(e));
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
            .bind(serde_json::json!({"status": "revision_requested", "revision_notes": &notes}))
            .execute(&mut *tx)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to create revision audit log: {e}");
                return Err(ApiError::Database(e));
            }

            // Commit the transaction
            if let Err(e) = tx.commit().await {
                tracing::error!("Failed to commit revision request: {e}");
                return Err(ApiError::BadRequest(format!(
                    "Failed to commit revision: {e}"
                )));
            }

            // 5. Send email notification (after commit, non-blocking)
            let developer_email: String =
                sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                    .bind(developer_id)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None)
                    .unwrap_or_default();

            if !developer_email.is_empty() {
                let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| {
                    "https://poool-backend-c54klbv5ka-ew.a.run.app".to_string()
                });
                let email_body = format!(
                    r#"<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 40px 20px;">
                        <div style="text-align: center; margin-bottom: 32px;">
                            <h1 style="color: #0000FF; font-size: 28px; margin: 0;">POOOL</h1>
                        </div>
                        <div style="background: #fff; border: 1px solid #e5e7eb; border-radius: 12px; padding: 32px;">
                            <div style="background: #FFF7ED; border: 1px solid #FDBA74; border-radius: 8px; padding: 12px 16px; margin-bottom: 24px;">
                                <span style="color: #9A3412; font-weight: 600; font-size: 14px;">⚠️ Revision Required</span>
                            </div>
                            <h2 style="font-size: 20px; color: #111; margin: 0 0 8px;">Your submission needs changes</h2>
                            <p style="color: #6b7280; font-size: 15px; line-height: 1.6; margin: 0 0 20px;">
                                Your project <strong>"{pname}"</strong> has been reviewed and requires revisions before it can be approved.
                            </p>
                            <div style="background: #F9FAFB; border: 1px solid #E5E7EB; border-radius: 8px; padding: 16px; margin-bottom: 24px;">
                                <p style="color: #374151; font-size: 13px; font-weight: 600; margin: 0 0 8px; text-transform: uppercase; letter-spacing: 0.05em;">Admin Feedback</p>
                                <p style="color: #1f2937; font-size: 14px; line-height: 1.6; margin: 0;">{revision_notes}</p>
                            </div>
                            <p style="color: #6b7280; font-size: 14px; line-height: 1.5; margin: 0 0 24px;">
                                Please review the feedback above, make the necessary changes, and resubmit your project.
                            </p>
                            <a href="{base}/developer/submissions" style="display: inline-block; background: #0000FF; color: #fff; padding: 12px 28px; border-radius: 8px; text-decoration: none; font-weight: 600; font-size: 14px;">
                                View My Submissions
                            </a>
                        </div>
                        <p style="color: #9ca3af; font-size: 12px; text-align: center; margin-top: 24px;">
                            &copy; 2026 POOOL. All rights reserved.
                        </p>
                    </div>"#,
                    pname = project_name,
                    revision_notes = notes,
                    base = base_url,
                );

                let _ = crate::common::email::send_email(
                    &developer_email,
                    &format!("Revision Required: {}", project_name),
                    &email_body,
                )
                .await;
            }

            Ok(Json(serde_json::json!({
                "status": "revision_requested",
                "message": "Revision request sent to developer. Developer has been notified via email and in-app notification."
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
                return Err(ApiError::Database(e));
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
                return Err(ApiError::Database(e));
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

// ==============================================================================
// Compliance Checklist Persistence
// ==============================================================================

/// GET /api/admin/developer-projects/:id/checklist — retrieve saved checklist state
pub async fn api_admin_project_checklist_get(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;

    let pid = ApiError::parse_uuid(&project_id)?;

    let checklist: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT COALESCE(compliance_checklist, '{}'::jsonb) FROM developer_projects WHERE id = $1",
    )
    .bind(pid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch checklist for project {project_id}: {e}");
        ApiError::Internal("Database error".to_string())
    })?;

    match checklist {
        Some(cl) => Ok(Json(serde_json::json!({ "checklist": cl })).into_response()),
        None => Err(ApiError::NotFound("Project not found".to_string())),
    }
}

/// PUT /api/admin/developer-projects/:id/checklist — persist checklist state
/// Payload: { "checklist": { "chk-kyc": true, "chk-legal": false, ... } }
pub async fn api_admin_project_checklist_save(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;

    let pid = ApiError::parse_uuid(&project_id)?;

    let checklist = body
        .get("checklist")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    // Validate it's an object
    if !checklist.is_object() {
        return Err(ApiError::BadRequest(
            "checklist must be a JSON object".to_string(),
        ));
    }

    let result = sqlx::query(
        "UPDATE developer_projects SET compliance_checklist = $2, updated_at = NOW() WHERE id = $1",
    )
    .bind(pid)
    .bind(&checklist)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() == 0 => Err(ApiError::NotFound("Project not found".to_string())),
        Ok(_) => Ok(Json(serde_json::json!({
            "status": "saved",
            "checklist": checklist
        }))
        .into_response()),
        Err(e) => {
            tracing::error!("Failed to save checklist for project {project_id}: {e}");
            Err(ApiError::Internal("Failed to save checklist".to_string()))
        }
    }
}

async fn validate_project_ready_for_approval(
    state: &AppState,
    project_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
            dp.asset_id,
            COALESCE(dp.compliance_checklist, '{}'::jsonb) AS compliance_checklist,
            (
                SELECT kr.status
                FROM kyc_records kr
                WHERE kr.user_id = dp.developer_id
                ORDER BY kr.created_at DESC
                LIMIT 1
            ) AS kyc_status
        FROM developer_projects dp
        WHERE dp.id = $1
        "#,
    )
    .bind(project_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;

    let asset_id: uuid::Uuid = row
        .get::<Option<uuid::Uuid>, _>("asset_id")
        .ok_or_else(|| ApiError::BadRequest("Project has no linked asset".to_string()))?;

    let kyc_status = row.get::<Option<String>, _>("kyc_status");
    if kyc_status.as_deref() != Some("approved") {
        return Err(ApiError::BadRequest(
            "Developer KYC must be approved before publishing".to_string(),
        ));
    }

    let checklist = row.get::<serde_json::Value, _>("compliance_checklist");
    for key in REQUIRED_APPROVAL_CHECKS {
        if checklist.get(*key).and_then(|v| v.as_bool()) != Some(true) {
            return Err(ApiError::BadRequest(format!(
                "Approval checklist item '{}' must be completed before publishing",
                key
            )));
        }
    }

    let document_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM asset_documents WHERE asset_id = $1")
            .bind(asset_id)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?;
    if document_count == 0 {
        return Err(ApiError::BadRequest(
            "At least one asset document is required before publishing".to_string(),
        ));
    }

    let image_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM asset_images WHERE asset_id = $1")
            .bind(asset_id)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?;
    if image_count == 0 {
        return Err(ApiError::BadRequest(
            "At least one asset image is required before publishing".to_string(),
        ));
    }

    Ok(())
}

// ==============================================================================
// Admin workflow: assignment + test-flag (migration 102)
// ==============================================================================

/// POST /api/admin/developer-projects/:id/assign
/// Body: { "admin_id": "<uuid>" | null }  (null = unassign; omit admin_id = self-assign)
pub async fn api_admin_developer_project_assign(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;

    let pid = ApiError::parse_uuid(&project_id)?;

    // Three modes:
    //   { "admin_id": "<uuid>" }  → assign to that admin
    //   { "admin_id": null }      → unassign
    //   {}                         → self-assign (current admin)
    let target: Option<uuid::Uuid> = match body.get("admin_id") {
        Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(ApiError::parse_uuid(s)?),
        Some(_) => {
            return Err(ApiError::BadRequest(
                "admin_id must be a UUID string or null".to_string(),
            ));
        }
        None => Some(admin.user.id),
    };

    let prev_assignee: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT assigned_admin_id FROM developer_projects WHERE id = $1")
            .bind(pid)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;

    sqlx::query(
        r#"UPDATE developer_projects
              SET assigned_admin_id = $1,
                  assigned_at       = CASE WHEN $1 IS NULL THEN NULL ELSE NOW() END,
                  updated_at        = NOW()
            WHERE id = $2"#,
    )
    .bind(target)
    .bind(pid)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'developer_project.assign', 'developer_projects', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(pid)
    .bind(serde_json::json!({ "assigned_admin_id": prev_assignee }))
    .bind(serde_json::json!({ "assigned_admin_id": target }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "assigned_admin_id": target,
    }))
    .into_response())
}

/// GET /api/admin/developer-projects/:id/history
/// Returns audit_logs entries for this project, newest first.
/// Captures: status transitions, notes, assignment changes, test-flag toggles.
pub async fn api_admin_developer_project_history(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, SUBMISSIONS_REVIEW_PERMISSION)
        .await?;

    let pid = ApiError::parse_uuid(&project_id)?;

    let rows = sqlx::query(
        r#"SELECT
              al.id,
              al.action,
              al.previous_state,
              al.new_state,
              al.created_at::text AS created_at,
              al.actor_user_id::text AS actor_user_id,
              COALESCE(up.first_name || ' ' || up.last_name, u.email) AS actor_name,
              u.email AS actor_email
           FROM audit_logs al
           LEFT JOIN users u ON u.id = al.actor_user_id
           LEFT JOIN user_profiles up ON up.user_id = al.actor_user_id
           WHERE al.entity_type = 'developer_projects'
             AND al.entity_id = $1
           ORDER BY al.created_at DESC
           LIMIT 200"#,
    )
    .bind(pid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch project history for {project_id}: {e}");
        ApiError::Database(e)
    })?;

    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "action": r.get::<String, _>("action"),
                "previous_state": r.try_get::<serde_json::Value, _>("previous_state").unwrap_or(serde_json::Value::Null),
                "new_state": r.try_get::<serde_json::Value, _>("new_state").unwrap_or(serde_json::Value::Null),
                "created_at": r.get::<String, _>("created_at"),
                "actor_user_id": r.try_get::<Option<String>, _>("actor_user_id").unwrap_or(None),
                "actor_name": r.try_get::<Option<String>, _>("actor_name").unwrap_or(None),
                "actor_email": r.try_get::<Option<String>, _>("actor_email").unwrap_or(None),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "history": entries })).into_response())
}

/// PATCH /api/admin/developer-projects/:id/test-flag
/// Body: { "is_test": true|false }
pub async fn api_admin_developer_project_test_flag(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(project_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    // Only users who can approve should be able to mark something as test
    // (it hides items from the default review queue).
    admin
        .require_permission(&state.db, SUBMISSIONS_APPROVE_PERMISSION)
        .await?;

    let pid = ApiError::parse_uuid(&project_id)?;
    let is_test = body
        .get("is_test")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| ApiError::BadRequest("is_test (boolean) is required".to_string()))?;

    let prev: Option<bool> =
        sqlx::query_scalar("SELECT is_test FROM developer_projects WHERE id = $1")
            .bind(pid)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?
            .ok_or_else(|| ApiError::NotFound("Project not found".to_string()))?;

    sqlx::query("UPDATE developer_projects SET is_test = $1, updated_at = NOW() WHERE id = $2")
        .bind(is_test)
        .bind(pid)
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'developer_project.test_flag', 'developer_projects', $2, $3, $4)"#,
    )
    .bind(admin.user.id)
    .bind(pid)
    .bind(serde_json::json!({ "is_test": prev }))
    .bind(serde_json::json!({ "is_test": is_test }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({ "ok": true, "is_test": is_test })).into_response())
}
