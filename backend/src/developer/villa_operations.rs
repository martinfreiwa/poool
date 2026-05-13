//! Developer endpoints for villa monthly operations (Villa-Returns P2).
//!
//! Dev-owned fields only (per Field Permission Matrix in
//! drafts/villa-returns-pages-outline.md §2.5):
//!   rental, occupancy, expense categories, OTA/payment/refunds, mgmt_fee.
//! Admin-owned fields (reserve override, platform fee, etc.) are server-controlled
//! and rejected if present in the request — enforced by ignoring them in the
//! shared `VillaOperationsInput` and forcing `reserve_override = None` here.
//!
//! State transitions allowed for Developer: draft → submitted only.
//! Everything else (approve / publish / override / reject) is Admin.

use crate::admin::extractors::ApiError;
use crate::admin::villa_operations::{compute_totals, VillaOperationsInput, VillaOperationsRow};
use crate::auth::routes::AppState;
use crate::developer::extractors::DeveloperUser;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct OperationsQuery {
    pub year: Option<i32>,
    pub month: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DashboardEntry {
    pub asset_id: Uuid,
    pub asset_title: String,
    pub latest_period_year: Option<i32>,
    pub latest_period_month: Option<i32>,
    pub latest_status: Option<String>,
    pub latest_rejected_reason: Option<String>,
}

/// GET /api/developer/operations/dashboard — list assigned villas + per-month status.
pub async fn api_developer_operations_dashboard(
    dev: DeveloperUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<DashboardEntry>>, ApiError> {
    let rows: Vec<DashboardEntry> = sqlx::query_as(
        r#"
        WITH latest AS (
            SELECT DISTINCT ON (l.asset_id)
                   l.asset_id, l.period_year, l.period_month, l.status, l.rejected_reason
            FROM villa_operations_log l
            JOIN developer_asset_links dal ON dal.asset_id = l.asset_id
            WHERE dal.developer_user_id = $1 AND dal.effective_until IS NULL
            ORDER BY l.asset_id, l.recorded_at DESC
        )
        SELECT
            a.id   AS asset_id,
            a.title AS asset_title,
            latest.period_year       AS latest_period_year,
            latest.period_month      AS latest_period_month,
            latest.status            AS latest_status,
            latest.rejected_reason   AS latest_rejected_reason
        FROM developer_asset_links dal
        JOIN assets a ON a.id = dal.asset_id
        LEFT JOIN latest ON latest.asset_id = a.id
        WHERE dal.developer_user_id = $1 AND dal.effective_until IS NULL
        ORDER BY a.title
        "#,
    )
    .bind(dev.user.id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

/// POST /api/developer/villas/:asset_id/operations — create draft (dev-owned fields only).
pub async fn api_developer_villa_operations_create(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(mut input): Json<VillaOperationsInput>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    // Developer cannot set the reserve override — strip it server-side.
    input.reserve_override_idr_cents = None;

    if !(2000..=2100).contains(&input.period_year) || !(1..=12).contains(&input.period_month) {
        return Err(ApiError::BadRequest("Invalid period".to_string()));
    }

    let (reserve_pct, platform_pct, withholding_bps) =
        load_asset_config(&state.db, asset_id).await?;
    let totals = compute_totals(&input, reserve_pct, platform_pct, withholding_bps, None);

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        INSERT INTO villa_operations_log (
            asset_id, period_year, period_month,
            gross_rental_idr_cents, currency_code,
            nights_available, nights_booked,
            expense_cleaning_idr_cents, expense_maintenance_idr_cents,
            expense_utilities_idr_cents, expense_staff_idr_cents,
            expense_pool_garden_idr_cents, expense_pest_idr_cents,
            expense_other_idr_cents, ota_fees_idr_cents,
            payment_fees_idr_cents, refunds_idr_cents, mgmt_fee_idr_cents,
            total_opex_idr_cents, net_rental_income_idr_cents,
            reserve_applied_idr_cents, platform_fee_idr_cents,
            withholding_idr_cents, distributable_idr_cents,
            status, supersedes_id, correction_reason, submitted_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
            $16, $17, $18, $19, $20, $21, $22, $23, $24, 'draft', $25, $26, $27
        )
        RETURNING *
        "#,
    )
    .bind(asset_id)
    .bind(input.period_year)
    .bind(input.period_month)
    .bind(input.gross_rental_idr_cents)
    .bind(input.currency_code.unwrap_or_else(|| "IDR".to_string()))
    .bind(input.nights_available)
    .bind(input.nights_booked)
    .bind(input.expense_cleaning_idr_cents)
    .bind(input.expense_maintenance_idr_cents)
    .bind(input.expense_utilities_idr_cents)
    .bind(input.expense_staff_idr_cents)
    .bind(input.expense_pool_garden_idr_cents)
    .bind(input.expense_pest_idr_cents)
    .bind(input.expense_other_idr_cents)
    .bind(input.ota_fees_idr_cents)
    .bind(input.payment_fees_idr_cents)
    .bind(input.refunds_idr_cents)
    .bind(input.mgmt_fee_idr_cents)
    .bind(totals.total_opex_idr_cents)
    .bind(totals.net_rental_income_idr_cents)
    .bind(totals.reserve_applied_idr_cents)
    .bind(totals.platform_fee_idr_cents)
    .bind(totals.withholding_idr_cents)
    .bind(totals.distributable_idr_cents)
    .bind(input.supersedes_id)
    .bind(input.correction_reason)
    .bind(dev.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(row))
}

/// PUT /api/developer/villas/:asset_id/operations/:log_id — edit own draft.
pub async fn api_developer_villa_operations_update(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
    Json(mut input): Json<VillaOperationsInput>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    input.reserve_override_idr_cents = None;

    let existing: VillaOperationsRow = sqlx::query_as(
        "SELECT * FROM villa_operations_log WHERE id = $1 AND asset_id = $2",
    )
    .bind(log_id)
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Operations row not found".to_string()))?;

    if existing.submitted_by != Some(dev.user.id) {
        return Err(ApiError::Forbidden(
            "You can only edit rows you submitted".to_string(),
        ));
    }
    if existing.status != "draft" {
        return Err(ApiError::Conflict(format!(
            "Cannot edit row in status='{}'",
            existing.status
        )));
    }

    let (reserve_pct, platform_pct, withholding_bps) =
        load_asset_config(&state.db, asset_id).await?;
    let totals = compute_totals(&input, reserve_pct, platform_pct, withholding_bps, None);

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        UPDATE villa_operations_log SET
            gross_rental_idr_cents       = $2,
            nights_available             = $3,
            nights_booked                = $4,
            expense_cleaning_idr_cents   = $5,
            expense_maintenance_idr_cents= $6,
            expense_utilities_idr_cents  = $7,
            expense_staff_idr_cents      = $8,
            expense_pool_garden_idr_cents= $9,
            expense_pest_idr_cents       = $10,
            expense_other_idr_cents      = $11,
            ota_fees_idr_cents           = $12,
            payment_fees_idr_cents       = $13,
            refunds_idr_cents            = $14,
            mgmt_fee_idr_cents           = $15,
            total_opex_idr_cents         = $16,
            net_rental_income_idr_cents  = $17,
            reserve_applied_idr_cents    = $18,
            platform_fee_idr_cents       = $19,
            withholding_idr_cents        = $20,
            distributable_idr_cents      = $21
        WHERE id = $1 AND status = 'draft'
        RETURNING *
        "#,
    )
    .bind(log_id)
    .bind(input.gross_rental_idr_cents)
    .bind(input.nights_available)
    .bind(input.nights_booked)
    .bind(input.expense_cleaning_idr_cents)
    .bind(input.expense_maintenance_idr_cents)
    .bind(input.expense_utilities_idr_cents)
    .bind(input.expense_staff_idr_cents)
    .bind(input.expense_pool_garden_idr_cents)
    .bind(input.expense_pest_idr_cents)
    .bind(input.expense_other_idr_cents)
    .bind(input.ota_fees_idr_cents)
    .bind(input.payment_fees_idr_cents)
    .bind(input.refunds_idr_cents)
    .bind(input.mgmt_fee_idr_cents)
    .bind(totals.total_opex_idr_cents)
    .bind(totals.net_rental_income_idr_cents)
    .bind(totals.reserve_applied_idr_cents)
    .bind(totals.platform_fee_idr_cents)
    .bind(totals.withholding_idr_cents)
    .bind(totals.distributable_idr_cents)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(row))
}

/// PUT /api/developer/villas/:asset_id/operations/:log_id/submit — submit own draft.
pub async fn api_developer_villa_operations_submit(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let existing: VillaOperationsRow = sqlx::query_as(
        "SELECT * FROM villa_operations_log WHERE id = $1 AND asset_id = $2",
    )
    .bind(log_id)
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Operations row not found".to_string()))?;

    if existing.submitted_by != Some(dev.user.id) {
        return Err(ApiError::Forbidden(
            "You can only submit rows you created".to_string(),
        ));
    }
    if existing.status != "draft" {
        return Err(ApiError::Conflict(format!(
            "Cannot submit row in status='{}'",
            existing.status
        )));
    }

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        UPDATE villa_operations_log SET
            status = 'submitted',
            submitted_at = NOW()
        WHERE id = $1 AND status = 'draft'
        RETURNING *
        "#,
    )
    .bind(log_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Notify all admins (W15). Fire-and-forget; failure does not block the submit.
    let _ = sqlx::query(
        r#"
        INSERT INTO notifications (user_id, title, message, type, action_url)
        SELECT DISTINCT ur.user_id,
               $1,
               $2,
               'system',
               $3
        FROM user_roles ur
        JOIN roles r ON r.id = ur.role_id
        WHERE r.name IN ('admin','super_admin') AND ur.is_active = TRUE
        "#,
    )
    .bind(format!(
        "Villa operations submitted — {}-{:02}",
        row.period_year, row.period_month
    ))
    .bind(format!(
        "Developer {} submitted monthly operations for villa {} (distributable {:.2} IDR cents).",
        dev.user.email,
        asset_id,
        row.distributable_idr_cents as f64
    ))
    .bind(format!(
        "/admin/villas/{}/operations/{}/{}?log_id={}&mode=review",
        asset_id, row.period_year, row.period_month, row.id
    ))
    .execute(&state.db)
    .await;

    Ok(Json(row))
}

/// GET /api/developer/villas/:asset_id/operations?year=&month=
pub async fn api_developer_villa_operations_list(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Query(q): Query<OperationsQuery>,
) -> Result<Json<Vec<VillaOperationsRow>>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let rows: Vec<VillaOperationsRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_operations_log
        WHERE asset_id = $1
          AND ($2::INT IS NULL OR period_year  = $2)
          AND ($3::INT IS NULL OR period_month = $3)
        ORDER BY period_year DESC, period_month DESC, recorded_at DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .bind(q.year)
    .bind(q.month)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// GET /api/developer/villas/:asset_id/asset-config
/// Read-only config the developer needs to render the computed-preview pane.
pub async fn api_developer_asset_config(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<AssetConfig>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let cfg: AssetConfig = sqlx::query_as(
        r#"
        SELECT
            COALESCE(reserve_pct_bps, 500)     AS reserve_pct_bps,
            COALESCE(poool_split_pct, 0)       AS platform_pct,
            COALESCE(withholding_tax_bps, 0)   AS withholding_tax_bps,
            COALESCE(mgmt_fee_bps, 0)          AS mgmt_fee_bps,
            COALESCE(native_currency_code, 'IDR') AS native_currency_code,
            COALESCE(payout_currency, 'USD')   AS payout_currency
        FROM assets WHERE id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    Ok(Json(cfg))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AssetConfig {
    pub reserve_pct_bps: i32,
    pub platform_pct: i32,
    pub withholding_tax_bps: i32,
    pub mgmt_fee_bps: i32,
    pub native_currency_code: String,
    pub payout_currency: String,
}

async fn load_asset_config(
    pool: &sqlx::PgPool,
    asset_id: Uuid,
) -> Result<(i32, i32, i32), ApiError> {
    let row: (Option<i32>, Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT reserve_pct_bps, poool_split_pct, withholding_tax_bps FROM assets WHERE id = $1",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;
    Ok((row.0.unwrap_or(500), row.1.unwrap_or(0), row.2.unwrap_or(0)))
}
