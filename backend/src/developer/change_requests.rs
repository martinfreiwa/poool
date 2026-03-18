/// Asset Change Requests — Developer edit-review workflow.
///
/// When a developer edits an approved/live asset, changes are stored as
/// a "change request" (pending) and must be approved by an admin before
/// they are applied to the `assets` table.
use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::{self, middleware, routes::AppState};
use crate::common::sanitize;

// ── Models ────────────────────────────────────────────────────────────────────

/// Payload for editing an asset (only changed fields are sent).
#[derive(Debug, Deserialize)]
pub struct AssetEditPayload {
    pub title: Option<String>,
    pub description: Option<String>,
    pub short_description: Option<String>,
    pub annual_yield_bps: Option<i32>,
    pub capital_appreciation_bps: Option<i32>,
    pub occupancy_rate_bps: Option<i32>,
    pub video_url: Option<String>,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub location_address: Option<String>,
    pub location_description: Option<String>,
    pub google_maps_url: Option<String>,
    pub property_type: Option<String>,
    pub area: Option<String>,
    pub lease_type: Option<String>,
    pub lease_term_years: Option<i32>,
    pub land_size_sqm: Option<f64>,
    pub building_size_sqm: Option<f64>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub construction_status: Option<String>,
    pub year_built: Option<i32>,
}

/// Body for admin rejection.
#[derive(Debug, Deserialize)]
pub struct RejectPayload {
    pub notes: Option<String>,
}

/// Body for admin approval.
#[derive(Debug, Deserialize)]
pub struct ApprovePayload {
    pub notes: Option<String>,
}

// ── Developer Routes ──────────────────────────────────────────────────────────

/// PUT /api/developer/assets/:id — Submit an edit.
///
/// If the asset is still draft/submitted, changes are applied directly.
/// If it's approved/live, a change request is created for admin review.
pub async fn submit_edit(
    jar: CookieJar,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<AssetEditPayload>,
) -> axum::response::Response {
    // 1. Authenticate
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

    // 2. Load asset and verify ownership
    let asset_row = match sqlx::query(
        "SELECT developer_user_id, title, description, short_description, \
                annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps, \
                video_url, location_city, location_country, location_address, \
                location_description, google_maps_url, property_type, area, \
                lease_type, lease_term_years, land_size_sqm, building_size_sqm, \
                bedrooms, bathrooms, construction_status, year_built \
         FROM assets WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        _ => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Asset not found"})),
            )
                .into_response()
        }
    };

    let owner_id: Uuid = asset_row.get("developer_user_id");
    if owner_id != user.id {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized"})),
        )
            .into_response();
    }

    // 3. Determine project status
    let project_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM developer_projects WHERE asset_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    let status = project_status.unwrap_or_else(|| "draft".to_string());

    // 4. Sanitize and build changed fields
    let mut payload = payload;
    if let Some(ref v) = payload.title { payload.title = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.short_description { payload.short_description = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.description { payload.description = Some(sanitize::sanitize_multiline(v)); }
    if let Some(ref v) = payload.location_city { payload.location_city = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.location_country { payload.location_country = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.location_address { payload.location_address = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.location_description { payload.location_description = Some(sanitize::sanitize_multiline(v)); }
    if let Some(ref v) = payload.google_maps_url { payload.google_maps_url = sanitize::sanitize_url(v); }
    if let Some(ref v) = payload.video_url { payload.video_url = sanitize::sanitize_url(v); }
    if let Some(ref v) = payload.property_type { payload.property_type = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.area { payload.area = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.lease_type { payload.lease_type = Some(sanitize::sanitize_text(v)); }
    if let Some(ref v) = payload.construction_status { payload.construction_status = Some(sanitize::sanitize_text(v)); }

    let (original_values, proposed_values) = build_diff(&asset_row, &payload);

    if proposed_values.as_object().is_none_or(|o| o.is_empty()) {
        return Json(serde_json::json!({
            "mode": "none",
            "message": "No changes detected"
        }))
        .into_response();
    }

    // 5. Branch on status
    match status.as_str() {
        "draft" | "submitted" => {
            // Direct edit — apply immediately
            apply_changes_to_asset(&state.db, id, &proposed_values).await;

            Json(serde_json::json!({
                "mode": "direct",
                "message": "Changes saved successfully"
            }))
            .into_response()
        }
        _ => {
            // Approved/live/in_review — create change request
            let cr_id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO asset_change_requests \
                    (asset_id, developer_id, original_values, proposed_values) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (asset_id) WHERE status = 'pending' \
                 DO UPDATE SET \
                    original_values = EXCLUDED.original_values, \
                    proposed_values = EXCLUDED.proposed_values, \
                    updated_at = NOW() \
                 RETURNING id",
            )
            .bind(id)
            .bind(user.id)
            .bind(&original_values)
            .bind(&proposed_values)
            .fetch_one(&state.db)
            .await;

            match cr_id {
                Ok(cr_id) => {
                    // Audit log
                    let _ = sqlx::query(
                        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) \
                         VALUES ($1, 'asset.change_request.created', 'asset_change_request', $2, $3)",
                    )
                    .bind(user.id)
                    .bind(cr_id)
                    .bind(&proposed_values)
                    .execute(&state.db)
                    .await;

                    Json(serde_json::json!({
                        "mode": "review",
                        "message": "Changes submitted for admin review",
                        "change_request_id": cr_id.to_string()
                    }))
                    .into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to create change request: {e}");
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": "Failed to submit changes"})),
                    )
                        .into_response()
                }
            }
        }
    }
}

/// GET /api/developer/assets/:id/pending-changes
pub async fn get_pending(
    jar: CookieJar,
    Path(id): Path<Uuid>,
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

    // Verify ownership
    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT developer_user_id FROM assets WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    if owner_id != Some(user.id) {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Not authorized"})),
        )
            .into_response();
    }

    let pending = sqlx::query(
        "SELECT id, original_values, proposed_values, status, admin_notes, created_at, updated_at \
         FROM asset_change_requests WHERE asset_id = $1 AND status = 'pending' LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await;

    match pending {
        Ok(Some(row)) => Json(serde_json::json!({
            "pending": {
                "id": row.get::<Uuid, _>("id").to_string(),
                "original_values": row.get::<serde_json::Value, _>("original_values"),
                "proposed_values": row.get::<serde_json::Value, _>("proposed_values"),
                "status": row.get::<String, _>("status"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            }
        }))
        .into_response(),
        _ => Json(serde_json::json!({ "pending": null })).into_response(),
    }
}

// ── Admin Routes ──────────────────────────────────────────────────────────────

/// GET /api/admin/change-requests — List all change requests.
pub async fn admin_list(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    if !auth::middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    let rows = sqlx::query(
        "SELECT cr.id, cr.asset_id, cr.developer_id, cr.original_values, cr.proposed_values, \
                cr.status, cr.admin_notes, cr.reviewed_at, cr.created_at, \
                a.title AS asset_title, \
                COALESCE(up.first_name || ' ' || up.last_name, u.email) AS developer_name \
         FROM asset_change_requests cr \
         JOIN assets a ON cr.asset_id = a.id \
         JOIN users u ON cr.developer_id = u.id \
         LEFT JOIN user_profiles up ON cr.developer_id = up.user_id \
         ORDER BY cr.created_at DESC \
         LIMIT 100",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let proposed: serde_json::Value = row.get("proposed_values");
            let fields_changed = proposed
                .as_object()
                .map(|o| o.len())
                .unwrap_or(0);

            serde_json::json!({
                "id": row.get::<Uuid, _>("id").to_string(),
                "asset_id": row.get::<Uuid, _>("asset_id").to_string(),
                "asset_title": row.get::<String, _>("asset_title"),
                "developer_name": row.get::<String, _>("developer_name"),
                "status": row.get::<String, _>("status"),
                "fields_changed": fields_changed,
                "admin_notes": row.get::<Option<String>, _>("admin_notes"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            })
        })
        .collect();

    // Counts
    let pending_count = items.iter().filter(|i| i["status"] == "pending").count();
    let approved_count = items.iter().filter(|i| i["status"] == "approved").count();
    let rejected_count = items.iter().filter(|i| i["status"] == "rejected").count();

    Json(serde_json::json!({
        "items": items,
        "pending_count": pending_count,
        "approved_count": approved_count,
        "rejected_count": rejected_count,
    }))
    .into_response()
}

/// GET /api/admin/change-requests/:id — Get detail with diff.
pub async fn admin_detail(
    jar: CookieJar,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> axum::response::Response {
    if !auth::middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    let row = sqlx::query(
        "SELECT cr.*, a.title AS asset_title, a.slug, a.asset_type, \
                COALESCE(up.first_name || ' ' || up.last_name, u.email) AS developer_name, \
                u.email AS developer_email \
         FROM asset_change_requests cr \
         JOIN assets a ON cr.asset_id = a.id \
         JOIN users u ON cr.developer_id = u.id \
         LEFT JOIN user_profiles up ON cr.developer_id = up.user_id \
         WHERE cr.id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(row)) => Json(serde_json::json!({
            "id": row.get::<Uuid, _>("id").to_string(),
            "asset_id": row.get::<Uuid, _>("asset_id").to_string(),
            "asset_title": row.get::<String, _>("asset_title"),
            "asset_type": row.get::<Option<String>, _>("asset_type"),
            "developer_name": row.get::<String, _>("developer_name"),
            "developer_email": row.get::<String, _>("developer_email"),
            "original_values": row.get::<serde_json::Value, _>("original_values"),
            "proposed_values": row.get::<serde_json::Value, _>("proposed_values"),
            "status": row.get::<String, _>("status"),
            "admin_notes": row.get::<Option<String>, _>("admin_notes"),
            "reviewed_at": row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("reviewed_at")
                .map(|d| d.to_rfc3339()),
            "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        }))
        .into_response(),
        _ => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Change request not found"})),
        )
            .into_response(),
    }
}

/// POST /api/admin/change-requests/:id/approve — Approve and apply changes.
pub async fn admin_approve(
    jar: CookieJar,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<ApprovePayload>,
) -> axum::response::Response {
    let admin = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Please log in"})),
            )
                .into_response()
        }
    };

    if !auth::middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    // Load the change request
    let cr_row = match sqlx::query(
        "SELECT asset_id, original_values, proposed_values, status \
         FROM asset_change_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        _ => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Change request not found"})),
            )
                .into_response()
        }
    };

    let cr_status: String = cr_row.get("status");
    if cr_status != "pending" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Change request is not pending"})),
        )
            .into_response();
    }

    let asset_id: Uuid = cr_row.get("asset_id");
    let original_values: serde_json::Value = cr_row.get("original_values");
    let proposed_values: serde_json::Value = cr_row.get("proposed_values");

    // Begin transaction
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to begin transaction: {e}");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response();
        }
    };

    // Apply changes to the assets table
    apply_changes_to_asset_tx(&mut tx, asset_id, &proposed_values).await;

    // Mark request as approved
    let _ = sqlx::query(
        "UPDATE asset_change_requests \
         SET status = 'approved', reviewed_by = $1, reviewed_at = NOW(), \
             admin_notes = $2, updated_at = NOW() \
         WHERE id = $3",
    )
    .bind(admin.id)
    .bind(body.notes.as_deref())
    .bind(id)
    .execute(&mut *tx)
    .await;

    // Audit log
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state) \
         VALUES ($1, 'asset.change_request.approved', 'asset', $2, $3, $4)",
    )
    .bind(admin.id)
    .bind(asset_id)
    .bind(&original_values)
    .bind(&proposed_values)
    .execute(&mut *tx)
    .await;

    match tx.commit().await {
        Ok(_) => Json(serde_json::json!({
            "status": "success",
            "message": "Changes approved and applied"
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to commit approval: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to apply changes"})),
            )
                .into_response()
        }
    }
}

/// POST /api/admin/change-requests/:id/reject — Reject with reason.
pub async fn admin_reject(
    jar: CookieJar,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<RejectPayload>,
) -> axum::response::Response {
    let admin = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Please log in"})),
            )
                .into_response()
        }
    };

    if !auth::middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    // Check status
    let cr_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM asset_change_requests WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    if cr_status.as_deref() != Some("pending") {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Change request is not pending"})),
        )
            .into_response();
    }

    let result = sqlx::query(
        "UPDATE asset_change_requests \
         SET status = 'rejected', admin_notes = $1, reviewed_by = $2, \
             reviewed_at = NOW(), updated_at = NOW() \
         WHERE id = $3",
    )
    .bind(body.notes.as_deref())
    .bind(admin.id)
    .bind(id)
    .execute(&state.db)
    .await;

    if let Err(e) = result {
        tracing::error!("Failed to reject change request: {e}");
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to reject"})),
        )
            .into_response();
    }

    // Audit log
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata) \
         VALUES ($1, 'asset.change_request.rejected', 'asset_change_request', $2, $3)",
    )
    .bind(admin.id)
    .bind(id)
    .bind(serde_json::json!({"reason": body.notes}))
    .execute(&state.db)
    .await;

    Json(serde_json::json!({
        "status": "success",
        "message": "Change request rejected"
    }))
    .into_response()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a diff between the current asset row and the proposed changes.
/// Returns (original_values, proposed_values) with only changed fields.
fn build_diff(
    row: &sqlx::postgres::PgRow,
    payload: &AssetEditPayload,
) -> (serde_json::Value, serde_json::Value) {
    let mut original = serde_json::Map::new();
    let mut proposed = serde_json::Map::new();

    macro_rules! diff_field {
        ($field:ident, $col:expr, $ty:ty) => {
            if let Some(ref new_val) = payload.$field {
                let old_val: Option<$ty> = row.get($col);
                let old_json = serde_json::to_value(&old_val).unwrap_or_default();
                let new_json = serde_json::to_value(new_val).unwrap_or_default();
                if old_json != new_json {
                    original.insert(stringify!($field).to_string(), old_json);
                    proposed.insert(stringify!($field).to_string(), new_json);
                }
            }
        };
    }

    macro_rules! diff_field_i32 {
        ($field:ident, $col:expr) => {
            if let Some(new_val) = payload.$field {
                let old_val: Option<i32> = row.get($col);
                if old_val != Some(new_val) {
                    original.insert(stringify!($field).to_string(), serde_json::json!(old_val));
                    proposed.insert(stringify!($field).to_string(), serde_json::json!(new_val));
                }
            }
        };
    }

    diff_field!(title, "title", String);
    diff_field!(description, "description", Option<String>);
    diff_field!(short_description, "short_description", Option<String>);
    diff_field_i32!(annual_yield_bps, "annual_yield_bps");
    diff_field_i32!(capital_appreciation_bps, "capital_appreciation_bps");
    diff_field_i32!(occupancy_rate_bps, "occupancy_rate_bps");
    diff_field!(video_url, "video_url", Option<String>);
    diff_field!(location_city, "location_city", Option<String>);
    diff_field!(location_country, "location_country", Option<String>);
    diff_field!(location_address, "location_address", Option<String>);
    diff_field!(location_description, "location_description", Option<String>);
    diff_field!(google_maps_url, "google_maps_url", Option<String>);
    diff_field!(property_type, "property_type", Option<String>);
    diff_field!(area, "area", Option<String>);
    diff_field!(lease_type, "lease_type", Option<String>);
    diff_field_i32!(lease_term_years, "lease_term_years");
    diff_field_i32!(bedrooms, "bedrooms");
    diff_field_i32!(bathrooms, "bathrooms");
    diff_field!(construction_status, "construction_status", Option<String>);
    diff_field_i32!(year_built, "year_built");

    // Handle Decimal fields (land_size_sqm, building_size_sqm)
    if let Some(new_val) = payload.land_size_sqm {
        let old_val: Option<rust_decimal::Decimal> = row.get("land_size_sqm");
        let old_f = old_val.map(|d| {
            use rust_decimal::prelude::ToPrimitive;
            d.to_f64().unwrap_or(0.0)
        });
        if old_f != Some(new_val) {
            original.insert("land_size_sqm".to_string(), serde_json::json!(old_f));
            proposed.insert("land_size_sqm".to_string(), serde_json::json!(new_val));
        }
    }

    if let Some(new_val) = payload.building_size_sqm {
        let old_val: Option<rust_decimal::Decimal> = row.get("building_size_sqm");
        let old_f = old_val.map(|d| {
            use rust_decimal::prelude::ToPrimitive;
            d.to_f64().unwrap_or(0.0)
        });
        if old_f != Some(new_val) {
            original.insert("building_size_sqm".to_string(), serde_json::json!(old_f));
            proposed.insert("building_size_sqm".to_string(), serde_json::json!(new_val));
        }
    }

    (
        serde_json::Value::Object(original),
        serde_json::Value::Object(proposed),
    )
}

/// Apply changes directly to the assets table (for draft/submitted assets).
async fn apply_changes_to_asset(db: &sqlx::PgPool, asset_id: Uuid, proposed: &serde_json::Value) {
    let obj = match proposed.as_object() {
        Some(o) => o,
        None => return,
    };

    // Apply each field individually to avoid SQL injection risks
    for (key, val) in obj {
        match key.as_str() {
            // String fields
            "title"
            | "description"
            | "short_description"
            | "video_url"
            | "location_city"
            | "location_country"
            | "location_address"
            | "location_description"
            | "google_maps_url"
            | "property_type"
            | "area"
            | "lease_type"
            | "construction_status" => {
                let str_val = val.as_str().map(|s| s.to_string());
                let q = format!(
                    "UPDATE assets SET {} = $1, updated_at = NOW() WHERE id = $2",
                    key
                );
                sqlx::query(&q)
                    .bind(str_val)
                    .bind(asset_id)
                    .execute(db)
                    .await
                    .ok();
            }
            // Integer fields
            "annual_yield_bps"
            | "capital_appreciation_bps"
            | "occupancy_rate_bps"
            | "lease_term_years"
            | "bedrooms"
            | "bathrooms"
            | "year_built" => {
                let int_val = val.as_i64().map(|v| v as i32);
                let q = format!(
                    "UPDATE assets SET {} = $1, updated_at = NOW() WHERE id = $2",
                    key
                );
                sqlx::query(&q)
                    .bind(int_val)
                    .bind(asset_id)
                    .execute(db)
                    .await
                    .ok();
            }
            // Decimal fields
            "land_size_sqm" | "building_size_sqm" => {
                let f_val = val
                    .as_f64()
                    .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
                let q = format!(
                    "UPDATE assets SET {} = $1, updated_at = NOW() WHERE id = $2",
                    key
                );
                sqlx::query(&q)
                    .bind(f_val)
                    .bind(asset_id)
                    .execute(db)
                    .await
                    .ok();
            }
            _ => {}
        }
    }
}

/// Apply changes within a transaction (for admin approval).
async fn apply_changes_to_asset_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    asset_id: Uuid,
    proposed: &serde_json::Value,
) {
    let obj = match proposed.as_object() {
        Some(o) => o,
        None => return,
    };

    // Apply each field individually to avoid SQL injection risks
    for (key, val) in obj {
        match key.as_str() {
            // String fields
            "title"
            | "description"
            | "short_description"
            | "video_url"
            | "location_city"
            | "location_country"
            | "location_address"
            | "location_description"
            | "google_maps_url"
            | "property_type"
            | "area"
            | "lease_type"
            | "construction_status" => {
                let str_val = val.as_str().map(|s| s.to_string());
                let q = format!(
                    "UPDATE assets SET {} = $1, updated_at = NOW() WHERE id = $2",
                    key
                );
                sqlx::query(&q)
                    .bind(str_val)
                    .bind(asset_id)
                    .execute(&mut **tx)
                    .await
                    .ok();
            }
            // Integer fields
            "annual_yield_bps"
            | "capital_appreciation_bps"
            | "occupancy_rate_bps"
            | "lease_term_years"
            | "bedrooms"
            | "bathrooms"
            | "year_built" => {
                let int_val = val.as_i64().map(|v| v as i32);
                let q = format!(
                    "UPDATE assets SET {} = $1, updated_at = NOW() WHERE id = $2",
                    key
                );
                sqlx::query(&q)
                    .bind(int_val)
                    .bind(asset_id)
                    .execute(&mut **tx)
                    .await
                    .ok();
            }
            // Decimal fields
            "land_size_sqm" | "building_size_sqm" => {
                let f_val = val
                    .as_f64()
                    .map(|v| rust_decimal::Decimal::from_f64_retain(v).unwrap_or_default());
                let q = format!(
                    "UPDATE assets SET {} = $1, updated_at = NOW() WHERE id = $2",
                    key
                );
                sqlx::query(&q)
                    .bind(f_val)
                    .bind(asset_id)
                    .execute(&mut **tx)
                    .await
                    .ok();
            }
            _ => {}
        }
    }
}
