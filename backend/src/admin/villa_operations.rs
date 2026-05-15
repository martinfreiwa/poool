//! Admin endpoints for villa monthly operations (Villa-Returns P2).
//!
//! Full state-machine over `villa_operations_log`:
//!   draft → submitted → approved → published
//!                  ↘  rejected (back to draft via re-edit)
//! Corrections (`supersedes_id`) and admin overrides create new rows;
//! the prior published row is flipped to `superseded` by the AFTER trigger
//! installed in migration 133.
//!
//! 4-eyes: approver_user_id must differ from submitter_user_id
//! (enforced by both the `vol_approver_differs` CHECK constraint and pre-flight
//! checks in this module, so the API returns a clean 409 rather than a raw
//! DB error).

use crate::admin::extractors::{AdminUser, ApiError};
use crate::admin::villa_nav_snapshot;
use crate::auth::routes::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ─── DTOs ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct VillaOperationsInput {
    pub period_year: i32,
    pub period_month: i32,
    pub currency_code: Option<String>,
    pub gross_rental_idr_cents: i64,
    pub nights_available: i32,
    pub nights_booked: i32,
    pub expense_cleaning_idr_cents: i64,
    pub expense_maintenance_idr_cents: i64,
    pub expense_utilities_idr_cents: i64,
    pub expense_staff_idr_cents: i64,
    pub expense_pool_garden_idr_cents: i64,
    pub expense_pest_idr_cents: i64,
    pub expense_other_idr_cents: i64,
    pub expense_property_tax_idr_cents: i64,
    pub expense_insurance_idr_cents: i64,
    pub expense_accounting_idr_cents: i64,
    pub expense_internet_idr_cents: i64,
    pub expense_capex_idr_cents: i64,
    pub ota_fees_idr_cents: i64,
    pub payment_fees_idr_cents: i64,
    pub refunds_idr_cents: i64,
    pub mgmt_fee_idr_cents: i64,
    pub reserve_override_idr_cents: Option<i64>,
    pub mgmt_reported_distributable_idr_cents: Option<i64>,
    pub correction_reason: Option<String>,
    pub supersedes_id: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VillaOperationsRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub period_year: i32,
    pub period_month: i32,
    pub gross_rental_idr_cents: i64,
    pub gross_rental_usd_cents: i64,
    pub currency_code: String,
    pub fx_rate_idr_to_usd_bps: i32,
    pub nights_available: i32,
    pub nights_booked: i32,
    pub occupancy_bps: i32,
    pub adr_idr_cents: i64,
    pub adr_usd_cents: i64,
    pub expense_cleaning_idr_cents: i64,
    pub expense_maintenance_idr_cents: i64,
    pub expense_utilities_idr_cents: i64,
    pub expense_staff_idr_cents: i64,
    pub expense_pool_garden_idr_cents: i64,
    pub expense_pest_idr_cents: i64,
    pub expense_other_idr_cents: i64,
    pub expense_property_tax_idr_cents: i64,
    pub expense_insurance_idr_cents: i64,
    pub expense_accounting_idr_cents: i64,
    pub expense_internet_idr_cents: i64,
    pub expense_capex_idr_cents: i64,
    pub ota_fees_idr_cents: i64,
    pub payment_fees_idr_cents: i64,
    pub refunds_idr_cents: i64,
    pub mgmt_fee_idr_cents: i64,
    pub total_opex_idr_cents: i64,
    pub total_opex_usd_cents: i64,
    pub net_rental_income_idr_cents: i64,
    pub net_rental_income_usd_cents: i64,
    pub reserve_override_idr_cents: Option<i64>,
    pub reserve_applied_idr_cents: i64,
    pub platform_fee_idr_cents: i64,
    pub withholding_idr_cents: i64,
    pub distributable_idr_cents: i64,
    pub distributable_usd_cents: i64,
    pub mgmt_reported_distributable_idr_cents: Option<i64>,
    pub status: String,
    pub supersedes_id: Option<i64>,
    pub correction_reason: Option<String>,
    pub submitted_by: Option<Uuid>,
    pub approved_by: Option<Uuid>,
    pub rejected_reason: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct OperationsQuery {
    pub year: Option<i32>,
    pub month: Option<i32>,
    /// "as_of" filter: only return rows recorded_at <= this timestamp.
    pub as_of: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RejectInput {
    pub reason: String,
}

// ─── Compute helpers ──────────────────────────────────────────

/// Server-side derivation of all monetary totals from raw inputs + per-asset config.
/// Pure integer arithmetic; intermediates promoted to `i128` to avoid overflow.
pub fn compute_totals(
    input: &VillaOperationsInput,
    reserve_pct_bps: i32,
    platform_pct: i32,
    withholding_bps: i32,
    reserve_override: Option<i64>,
) -> ComputedTotals {
    // CapEx (expense_capex_idr_cents) is intentionally excluded — it is tracked
    // for transparency but is not an operating expense and must not reduce distributable.
    let total_opex = (input.expense_cleaning_idr_cents as i128)
        + (input.expense_maintenance_idr_cents as i128)
        + (input.expense_utilities_idr_cents as i128)
        + (input.expense_staff_idr_cents as i128)
        + (input.expense_pool_garden_idr_cents as i128)
        + (input.expense_pest_idr_cents as i128)
        + (input.expense_other_idr_cents as i128)
        + (input.expense_property_tax_idr_cents as i128)
        + (input.expense_insurance_idr_cents as i128)
        + (input.expense_accounting_idr_cents as i128)
        + (input.expense_internet_idr_cents as i128)
        + (input.ota_fees_idr_cents as i128)
        + (input.payment_fees_idr_cents as i128)
        + (input.mgmt_fee_idr_cents as i128)
        - (input.refunds_idr_cents as i128);

    let net_rental = (input.gross_rental_idr_cents as i128) - total_opex;

    let reserve_applied = match reserve_override {
        Some(v) => v as i128,
        None => (net_rental * reserve_pct_bps as i128) / 10_000,
    };

    let platform_fee = (net_rental * platform_pct as i128) / 100;
    let after_fee_and_reserve = net_rental - platform_fee - reserve_applied;
    let withholding = (after_fee_and_reserve.max(0) * withholding_bps as i128) / 10_000;
    let distributable = after_fee_and_reserve - withholding;

    ComputedTotals {
        total_opex_idr_cents: total_opex as i64,
        net_rental_income_idr_cents: net_rental as i64,
        reserve_applied_idr_cents: reserve_applied.max(0) as i64,
        platform_fee_idr_cents: platform_fee.max(0) as i64,
        withholding_idr_cents: withholding.max(0) as i64,
        distributable_idr_cents: distributable as i64,
    }
}

#[derive(Debug)]
pub struct ComputedTotals {
    pub total_opex_idr_cents: i64,
    pub net_rental_income_idr_cents: i64,
    pub reserve_applied_idr_cents: i64,
    pub platform_fee_idr_cents: i64,
    pub withholding_idr_cents: i64,
    pub distributable_idr_cents: i64,
}

/// Convert an IDR cent value to USD cents using a basis-point FX rate.
/// `rate_bps` is interpreted as USD-per-IDR scaled by 10000 — i.e. 1 USD = 15_500 IDR
/// is stored as `rate_bps = 645` (≈ 0.0000645 USD per IDR, scaled by 10000 = 0.645,
/// then bumped per migration 132 default of 1). The publish step is responsible
/// for choosing the right scale and persisting it back on the row.
fn idr_to_usd_cents(idr_cents: i64, rate_bps: i32) -> i64 {
    ((idr_cents as i128) * rate_bps as i128 / 10_000) as i64
}

async fn load_asset_config(pool: &PgPool, asset_id: Uuid) -> Result<(i32, i32, i32), ApiError> {
    let row: (Option<i32>, Option<i32>, Option<i32>) = sqlx::query_as(
        r#"
        SELECT reserve_pct_bps, poool_split_pct, withholding_tax_bps
        FROM assets WHERE id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    Ok((row.0.unwrap_or(500), row.1.unwrap_or(0), row.2.unwrap_or(0)))
}

// ─── Handlers ─────────────────────────────────────────────────

/// POST /api/admin/villas/:asset_id/operations — create draft.
pub async fn api_admin_villa_operations_create(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<VillaOperationsInput>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    if !(2000..=2100).contains(&input.period_year) || !(1..=12).contains(&input.period_month) {
        return Err(ApiError::BadRequest("Invalid period".to_string()));
    }

    let (reserve_pct, platform_pct, withholding_bps) =
        load_asset_config(&state.db, asset_id).await?;
    let totals = compute_totals(
        &input,
        reserve_pct,
        platform_pct,
        withholding_bps,
        input.reserve_override_idr_cents,
    );

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        INSERT INTO villa_operations_log (
            asset_id, period_year, period_month,
            gross_rental_idr_cents, currency_code,
            nights_available, nights_booked,
            expense_cleaning_idr_cents, expense_maintenance_idr_cents,
            expense_utilities_idr_cents, expense_staff_idr_cents,
            expense_pool_garden_idr_cents, expense_pest_idr_cents,
            expense_other_idr_cents,
            expense_property_tax_idr_cents, expense_insurance_idr_cents,
            expense_accounting_idr_cents, expense_internet_idr_cents,
            expense_capex_idr_cents,
            ota_fees_idr_cents, payment_fees_idr_cents, refunds_idr_cents, mgmt_fee_idr_cents,
            total_opex_idr_cents, net_rental_income_idr_cents,
            reserve_override_idr_cents, reserve_applied_idr_cents,
            platform_fee_idr_cents, withholding_idr_cents, distributable_idr_cents,
            mgmt_reported_distributable_idr_cents,
            status, supersedes_id, correction_reason, submitted_by
        ) VALUES (
            $1,  $2,  $3,  $4,  $5,  $6,  $7,  $8,  $9,  $10, $11, $12, $13, $14,
            $15, $16, $17, $18, $19,
            $20, $21, $22, $23,
            $24, $25, $26, $27, $28, $29, $30, $31,
            'draft', $32, $33, $34
        )
        RETURNING *
        "#,
    )
    .bind(asset_id)                                         // $1
    .bind(input.period_year)                                // $2
    .bind(input.period_month)                               // $3
    .bind(input.gross_rental_idr_cents)                     // $4
    .bind(input.currency_code.unwrap_or_else(|| "IDR".to_string())) // $5
    .bind(input.nights_available)                           // $6
    .bind(input.nights_booked)                              // $7
    .bind(input.expense_cleaning_idr_cents)                 // $8
    .bind(input.expense_maintenance_idr_cents)              // $9
    .bind(input.expense_utilities_idr_cents)                // $10
    .bind(input.expense_staff_idr_cents)                    // $11
    .bind(input.expense_pool_garden_idr_cents)              // $12
    .bind(input.expense_pest_idr_cents)                     // $13
    .bind(input.expense_other_idr_cents)                    // $14
    .bind(input.expense_property_tax_idr_cents)             // $15
    .bind(input.expense_insurance_idr_cents)                // $16
    .bind(input.expense_accounting_idr_cents)               // $17
    .bind(input.expense_internet_idr_cents)                 // $18
    .bind(input.expense_capex_idr_cents)                    // $19
    .bind(input.ota_fees_idr_cents)                         // $20
    .bind(input.payment_fees_idr_cents)                     // $21
    .bind(input.refunds_idr_cents)                          // $22
    .bind(input.mgmt_fee_idr_cents)                         // $23
    .bind(totals.total_opex_idr_cents)                      // $24
    .bind(totals.net_rental_income_idr_cents)               // $25
    .bind(input.reserve_override_idr_cents)                 // $26
    .bind(totals.reserve_applied_idr_cents)                 // $27
    .bind(totals.platform_fee_idr_cents)                    // $28
    .bind(totals.withholding_idr_cents)                     // $29
    .bind(totals.distributable_idr_cents)                   // $30
    .bind(input.mgmt_reported_distributable_idr_cents)      // $31
    .bind(input.supersedes_id)                              // $32
    .bind(input.correction_reason.clone())                  // $33
    .bind(admin.user.id)                                    // $34
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    write_audit(&state.db, admin.user.id, "create", row.id, &row, None).await;
    Ok(Json(row))
}

/// PUT /api/admin/villas/:asset_id/operations/:log_id — update draft.
pub async fn api_admin_villa_operations_update(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
    Json(input): Json<VillaOperationsInput>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    let existing = load_row(&state.db, log_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "draft" && existing.status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot edit row in status='{}'",
            existing.status
        )));
    }

    let (reserve_pct, platform_pct, withholding_bps) =
        load_asset_config(&state.db, asset_id).await?;
    let totals = compute_totals(
        &input,
        reserve_pct,
        platform_pct,
        withholding_bps,
        input.reserve_override_idr_cents,
    );

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        UPDATE villa_operations_log SET
            gross_rental_idr_cents                   = $2,
            nights_available                         = $3,
            nights_booked                            = $4,
            expense_cleaning_idr_cents               = $5,
            expense_maintenance_idr_cents            = $6,
            expense_utilities_idr_cents              = $7,
            expense_staff_idr_cents                  = $8,
            expense_pool_garden_idr_cents            = $9,
            expense_pest_idr_cents                   = $10,
            expense_other_idr_cents                  = $11,
            expense_property_tax_idr_cents           = $12,
            expense_insurance_idr_cents              = $13,
            expense_accounting_idr_cents             = $14,
            expense_internet_idr_cents               = $15,
            expense_capex_idr_cents                  = $16,
            ota_fees_idr_cents                       = $17,
            payment_fees_idr_cents                   = $18,
            refunds_idr_cents                        = $19,
            mgmt_fee_idr_cents                       = $20,
            total_opex_idr_cents                     = $21,
            net_rental_income_idr_cents              = $22,
            reserve_override_idr_cents               = $23,
            reserve_applied_idr_cents                = $24,
            platform_fee_idr_cents                   = $25,
            withholding_idr_cents                    = $26,
            distributable_idr_cents                  = $27,
            mgmt_reported_distributable_idr_cents    = $28
        WHERE id = $1 AND status IN ('draft','submitted')
        RETURNING *
        "#,
    )
    .bind(log_id)                                           // $1
    .bind(input.gross_rental_idr_cents)                     // $2
    .bind(input.nights_available)                           // $3
    .bind(input.nights_booked)                              // $4
    .bind(input.expense_cleaning_idr_cents)                 // $5
    .bind(input.expense_maintenance_idr_cents)              // $6
    .bind(input.expense_utilities_idr_cents)                // $7
    .bind(input.expense_staff_idr_cents)                    // $8
    .bind(input.expense_pool_garden_idr_cents)              // $9
    .bind(input.expense_pest_idr_cents)                     // $10
    .bind(input.expense_other_idr_cents)                    // $11
    .bind(input.expense_property_tax_idr_cents)             // $12
    .bind(input.expense_insurance_idr_cents)                // $13
    .bind(input.expense_accounting_idr_cents)               // $14
    .bind(input.expense_internet_idr_cents)                 // $15
    .bind(input.expense_capex_idr_cents)                    // $16
    .bind(input.ota_fees_idr_cents)                         // $17
    .bind(input.payment_fees_idr_cents)                     // $18
    .bind(input.refunds_idr_cents)                          // $19
    .bind(input.mgmt_fee_idr_cents)                         // $20
    .bind(totals.total_opex_idr_cents)                      // $21
    .bind(totals.net_rental_income_idr_cents)               // $22
    .bind(input.reserve_override_idr_cents)                 // $23
    .bind(totals.reserve_applied_idr_cents)                 // $24
    .bind(totals.platform_fee_idr_cents)                    // $25
    .bind(totals.withholding_idr_cents)                     // $26
    .bind(totals.distributable_idr_cents)                   // $27
    .bind(input.mgmt_reported_distributable_idr_cents)      // $28
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    write_audit(
        &state.db,
        admin.user.id,
        "update",
        row.id,
        &row,
        Some(&existing),
    )
    .await;
    Ok(Json(row))
}

/// PUT /api/admin/villas/:asset_id/operations/:log_id/submit
pub async fn api_admin_villa_operations_submit(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    transition(&state.db, asset_id, log_id, admin.user.id, "submit").await
}

/// PUT /api/admin/villas/:asset_id/operations/:log_id/approve  (4-eyes: approver != submitter)
pub async fn api_admin_villa_operations_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    let existing = load_row(&state.db, log_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot approve row in status='{}'",
            existing.status
        )));
    }
    if existing.submitted_by == Some(admin.user.id) {
        return Err(ApiError::Conflict(
            "You submitted this row — another admin must approve".to_string(),
        ));
    }
    transition(&state.db, asset_id, log_id, admin.user.id, "approve").await
}

/// PUT /api/admin/villas/:asset_id/operations/:log_id/publish
pub async fn api_admin_villa_operations_publish(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    let existing = load_row(&state.db, log_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "approved" {
        return Err(ApiError::Conflict(format!(
            "Cannot publish row in status='{}'",
            existing.status
        )));
    }

    // Freeze FX rate at publish: pull the latest IDR→USD rate; default to 1 if absent.
    let fx_bps: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE((
            SELECT rate_bps::INTEGER
            FROM fx_rates_daily
            WHERE base_currency = 'IDR' AND quote_currency = 'USD'
            ORDER BY snapshot_date DESC
            LIMIT 1
        ), 1)
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(1);

    let gross_rental_usd = idr_to_usd_cents(existing.gross_rental_idr_cents, fx_bps);
    let total_opex_usd = idr_to_usd_cents(existing.total_opex_idr_cents, fx_bps);
    let net_rental_usd = idr_to_usd_cents(existing.net_rental_income_idr_cents, fx_bps);
    let distributable_usd = idr_to_usd_cents(existing.distributable_idr_cents, fx_bps);

    // Villa-Returns C1 shadow-write — gated on feature flag.
    // 'off' or 'shadow' → also upsert legacy `asset_financials` row.
    // 'on'              → skip shadow-write (legacy stays frozen at cut-over moment).
    let flag: String = sqlx::query_scalar(
        "SELECT COALESCE(value, 'on') FROM platform_settings WHERE key='villa_returns.enabled'",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .unwrap_or_else(|| "on".to_string());

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        UPDATE villa_operations_log SET
            status                       = 'published',
            published_at                 = NOW(),
            approved_by                  = COALESCE(approved_by, $2),
            fx_rate_idr_to_usd_bps       = $3,
            gross_rental_usd_cents       = $4,
            total_opex_usd_cents         = $5,
            net_rental_income_usd_cents  = $6,
            distributable_usd_cents      = $7
        WHERE id = $1 AND status = 'approved'
        RETURNING *
        "#,
    )
    .bind(log_id)
    .bind(admin.user.id)
    .bind(fx_bps)
    .bind(gross_rental_usd)
    .bind(total_opex_usd)
    .bind(net_rental_usd)
    .bind(distributable_usd)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    if flag != "on" {
        sqlx::query(
            r#"
            INSERT INTO asset_financials
                (asset_id, period_year, period_month,
                 rental_income_cents, expenses_cents, net_income_cents,
                 occupancy_rate_bps, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (asset_id, period_month, period_year) DO UPDATE SET
                rental_income_cents = EXCLUDED.rental_income_cents,
                expenses_cents      = EXCLUDED.expenses_cents,
                net_income_cents    = EXCLUDED.net_income_cents,
                occupancy_rate_bps  = EXCLUDED.occupancy_rate_bps
            "#,
        )
        .bind(row.asset_id)
        .bind(row.period_year)
        .bind(row.period_month)
        .bind(row.gross_rental_idr_cents)
        .bind(row.total_opex_idr_cents)
        .bind(row.net_rental_income_idr_cents)
        .bind(row.occupancy_bps)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
        tracing::info!(
            "C1 shadow-write: legacy asset_financials upserted for asset={} period={}-{:02}",
            row.asset_id,
            row.period_year,
            row.period_month
        );
    }

    tx.commit().await.map_err(ApiError::Database)?;

    // Kick off a NAV snapshot in the background — fire-and-forget, does not block response.
    // run_snapshot_for_all_assets is idempotent (UPSERTs on asset_id + snapshot_date).
    {
        let pool = state.db.clone();
        tokio::spawn(async move {
            if let Err(e) = villa_nav_snapshot::run_snapshot_for_all_assets(&pool).await {
                tracing::warn!("post-publish NAV snapshot failed: {e}");
            }
        });
    }

    // Auto-distribute immediately after publish: generate dividend_payouts rows and
    // credit investor wallets in one background task. Both steps are idempotent so
    // the manual "Distribute" button on the UI remains a safe no-op retry.
    {
        let pool      = state.db.clone();
        let actor_id  = admin.user.id;
        let asset_id_ = row.asset_id;
        let log_id_   = row.id;
        tokio::spawn(async move {
            match distribute_core(&pool, asset_id_, log_id_, actor_id).await {
                Err(e) => {
                    tracing::warn!("post-publish auto-distribute failed: {e:?}");
                }
                Ok(dist) => {
                    tracing::info!(
                        "post-publish auto-distribute: created={} skipped={} total={}",
                        dist.created, dist.skipped, dist.total_paid_cents
                    );
                    match process_payouts_core(&pool, asset_id_, log_id_, actor_id).await {
                        Err(e) => tracing::warn!("post-publish auto-process-payouts failed: {e:?}"),
                        Ok(pp) => tracing::info!(
                            "post-publish auto-process-payouts: paid={} total={}",
                            pp.paid_count, pp.paid_total_cents
                        ),
                    }
                }
            }
        });
    }

    // Notify the submitter that their row is live (W15).
    if let Some(submitter_id) = row.submitted_by {
        let _ = sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, type, action_url)
            VALUES ($1, $2, $3, 'system', $4)
            "#,
        )
        .bind(submitter_id)
        .bind(format!(
            "Operations published — {}-{:02}",
            row.period_year, row.period_month
        ))
        .bind(format!(
            "Your submission for {}-{:02} is now live. Distributable: {} IDR cents.",
            row.period_year, row.period_month, row.distributable_idr_cents
        ))
        .bind(format!(
            "/admin/villas/{}/operations/{}/{}?log_id={}",
            row.asset_id, row.period_year, row.period_month, row.id
        ))
        .execute(&state.db)
        .await;

        let _ = crate::email::trigger_transactional_email(
            &state.db,
            &submitter_id,
            "operations_published",
            serde_json::json!({
                "period_year":          row.period_year,
                "period_month":         row.period_month,
                "asset_id":             row.asset_id,
                "distributable_idr":    row.distributable_idr_cents,
            }),
        )
        .await;
    }

    write_audit(
        &state.db,
        admin.user.id,
        "publish",
        row.id,
        &row,
        Some(&existing),
    )
    .await;
    Ok(Json(row))
}

/// POST /api/admin/villas/:asset_id/operations/:log_id/top-up — Q11.
///
/// For a published row that supersedes a prior published row, compute per-investor
/// deltas between what they ALREADY received (rental payouts for this period) and
/// what their share would be under the corrected `distributable_*_cents`. Positive
/// deltas → insert a `dividend_payouts.payout_type='bonus'` row, credit the wallet,
/// log a wallet_transactions row, notify the investor. Negative deltas are absorbed
/// by POOOL (Q11 lock-in: never claw back).
#[derive(Debug, Serialize)]
pub struct TopUpResult {
    pub topped_up_count: i64,
    pub topped_up_total_cents: i64,
    pub skipped_no_delta: i64,
    pub currency: String,
}

pub async fn api_admin_villa_operations_top_up(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<TopUpResult>, ApiError> {
    let row = load_row(&state.db, log_id).await?;
    if row.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if row.status != "published" {
        return Err(ApiError::Conflict(format!(
            "Top-up requires a published row (got status='{}')",
            row.status
        )));
    }
    if row.supersedes_id.is_none() {
        return Err(ApiError::Conflict(
            "Top-up only meaningful on a corrected (supersedes_id != NULL) published row"
                .to_string(),
        ));
    }

    let cfg: (Option<String>, Option<i64>, Option<i32>, Option<i32>) = sqlx::query_as(
        r#"
        SELECT payout_currency, tokens_total::BIGINT, tokens_payout_eligible, tokens_owner_retained
        FROM assets WHERE id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    let payout_currency = cfg.0.unwrap_or_else(|| "USD".to_string());
    let tokens_total = cfg.1.unwrap_or(0);
    let denominator: i64 = cfg
        .2
        .map(i64::from)
        .unwrap_or(tokens_total - i64::from(cfg.3.unwrap_or(0)));
    if denominator <= 0 {
        return Err(ApiError::Conflict(
            "Asset has no payout-eligible tokens".to_string(),
        ));
    }

    let new_distributable = match payout_currency.as_str() {
        "USD" => row.distributable_usd_cents,
        "IDR" => row.distributable_idr_cents,
        other => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported payout currency '{}'",
                other
            )));
        }
    };

    #[derive(sqlx::FromRow)]
    struct Investor {
        investment_id: Uuid,
        user_id: Uuid,
        tokens_owned: i32,
        already_paid: i64,
    }
    let investors: Vec<Investor> = sqlx::query_as(
        r#"
        SELECT
            i.id AS investment_id,
            i.user_id,
            i.tokens_owned,
            COALESCE((
                SELECT SUM(dp.amount_cents)::BIGINT
                FROM dividend_payouts dp
                WHERE dp.user_id = i.user_id
                  AND dp.asset_id = i.asset_id
                  AND dp.period_year = $2
                  AND dp.period_month = $3
                  AND dp.status = 'paid'
            ), 0) AS already_paid
        FROM investments i
        WHERE i.asset_id = $1
          AND i.tokens_owned > 0
          AND i.status IN ('active','funded','rented','payout_pending')
        "#,
    )
    .bind(asset_id)
    .bind(row.period_year)
    .bind(row.period_month)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let mut topped_count: i64 = 0;
    let mut topped_total: i64 = 0;
    let mut skipped: i64 = 0;
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    for inv in &investors {
        let new_share: i64 =
            ((new_distributable as i128) * inv.tokens_owned as i128 / denominator as i128) as i64;
        let delta = new_share - inv.already_paid;
        if delta <= 0 {
            skipped += 1;
            continue;
        }

        // Wallet upsert + credit + tx insert + investment stat update + payout row.
        // C4: use asset's payout_currency (already loaded as `payout_currency` above).
        let wallet_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO wallets (user_id, wallet_type, currency)
            VALUES ($1, 'cash', $2)
            ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(inv.user_id)
        .bind(&payout_currency)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        sqlx::query("UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE id = $2")
            .bind(delta)
            .bind(wallet_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

        let description = format!(
            "Villa rental top-up — correction {}-{:02}",
            row.period_year, row.period_month
        );
        let new_tx_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO wallet_transactions
                (wallet_id, amount_cents, type, status, description, currency)
            VALUES ($1, $2, 'dividend', 'completed', $3, $4)
            RETURNING id
            "#,
        )
        .bind(wallet_id)
        .bind(delta)
        .bind(&description)
        .bind(&payout_currency)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        sqlx::query("UPDATE investments SET total_rental_cents = total_rental_cents + $1, updated_at = NOW() WHERE id = $2")
            .bind(delta)
            .bind(inv.investment_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;

        sqlx::query(
            r#"
            INSERT INTO dividend_payouts
                (investment_id, user_id, asset_id, amount_cents, payout_type, status,
                 scheduled_at, paid_at, wallet_tx_id,
                 source_villa_operations_log_id, period_year, period_month)
            VALUES ($1, $2, $3, $4, 'bonus', 'paid', NOW(), NOW(), $5, $6, $7, $8)
            "#,
        )
        .bind(inv.investment_id)
        .bind(inv.user_id)
        .bind(asset_id)
        .bind(delta)
        .bind(new_tx_id)
        .bind(log_id)
        .bind(row.period_year)
        .bind(row.period_month)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        let _ = sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, type, action_url)
            VALUES ($1, $2, $3, 'payout', '/transactions')
            "#,
        )
        .bind(inv.user_id)
        .bind(format!(
            "Correction top-up — {}-{:02}",
            row.period_year, row.period_month
        ))
        .bind(format!(
            "USD {:.2} additional dividend credited for {}-{:02} correction.",
            (delta as f64) / 100.0,
            row.period_year,
            row.period_month
        ))
        .execute(&mut *tx)
        .await;

        topped_count += 1;
        topped_total += delta;
    }

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_ops.top_up', 'villa_operations_log', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "log_id": log_id,
        "asset_id": asset_id,
        "period_year": row.period_year,
        "period_month": row.period_month,
        "topped_up_count": topped_count,
        "topped_up_total_cents": topped_total,
        "skipped_no_delta": skipped,
        "currency": payout_currency,
    }))
    .execute(&mut *tx)
    .await;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(TopUpResult {
        topped_up_count: topped_count,
        topped_up_total_cents: topped_total,
        skipped_no_delta: skipped,
        currency: payout_currency,
    }))
}

/// PUT /api/admin/villas/:asset_id/operations/:log_id/reject
pub async fn api_admin_villa_operations_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
    Json(input): Json<RejectInput>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    if input.reason.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Rejection reason required".to_string(),
        ));
    }
    let existing = load_row(&state.db, log_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot reject row in status='{}'",
            existing.status
        )));
    }

    let row: VillaOperationsRow = sqlx::query_as(
        r#"
        UPDATE villa_operations_log SET
            status          = 'draft',
            rejected_reason = $2,
            rejected_at     = NOW()
        WHERE id = $1 AND status = 'submitted'
        RETURNING *
        "#,
    )
    .bind(log_id)
    .bind(input.reason.clone())
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Notify the submitter (W15). Fire-and-forget.
    if let Some(submitter_id) = row.submitted_by {
        let _ = sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, type, action_url)
            VALUES ($1, $2, $3, 'system', $4)
            "#,
        )
        .bind(submitter_id)
        .bind(format!(
            "Operations submission rejected — {}-{:02}",
            row.period_year, row.period_month
        ))
        .bind(format!(
            "Admin rejected your submission. Reason: {}",
            input.reason
        ))
        .bind(format!(
            "/developer/villas/{}/operations/new?year={}&month={}",
            asset_id, row.period_year, row.period_month
        ))
        .execute(&state.db)
        .await;

        let _ = crate::email::trigger_transactional_email(
            &state.db,
            &submitter_id,
            "operations_rejected",
            serde_json::json!({
                "period_year":  row.period_year,
                "period_month": row.period_month,
                "asset_id":     row.asset_id,
                "reason":       input.reason.clone(),
            }),
        )
        .await;
    }

    write_audit(
        &state.db,
        admin.user.id,
        "reject",
        row.id,
        &row,
        Some(&existing),
    )
    .await;
    Ok(Json(row))
}

/// POST /api/admin/villas/:asset_id/operations/:log_id/process-payouts
/// Processes all `scheduled` dividend_payouts rows tied to this villa_operations_log
/// row: credits the user's cash wallet, inserts a wallet_transactions row,
/// updates investment rental stats, and flips dividend_payouts to `paid`.
/// Idempotent: only `status='scheduled'` rows are touched.
#[derive(Debug, Serialize)]
pub struct ProcessPayoutsResult {
    pub paid_count: i64,
    pub paid_total_cents: i64,
    pub skipped_already_paid: i64,
}

/// Core process-payouts logic — extracted so it can be called from both the
/// HTTP handler and the post-publish background task.
pub(crate) async fn process_payouts_core(
    pool: &PgPool,
    asset_id: Uuid,
    log_id: i64,
    actor_id: Uuid,
) -> Result<ProcessPayoutsResult, ApiError> {
    let log_row = load_row(pool, log_id).await?;
    if log_row.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if log_row.status != "published" {
        return Err(ApiError::Conflict(format!(
            "Cannot process payouts for row in status='{}'",
            log_row.status
        )));
    }

    #[derive(sqlx::FromRow)]
    struct Pending {
        id: Uuid,
        user_id: Uuid,
        amount_cents: i64,
    }
    let pending: Vec<Pending> = sqlx::query_as(
        r#"
        SELECT id, user_id, amount_cents
        FROM dividend_payouts
        WHERE source_villa_operations_log_id = $1
          AND status = 'scheduled'
        "#,
    )
    .bind(log_id)
    .fetch_all(pool)
    .await
    .map_err(ApiError::Database)?;

    let already_paid: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT FROM dividend_payouts
        WHERE source_villa_operations_log_id = $1 AND status = 'paid'
        "#,
    )
    .bind(log_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let mut paid_count: i64 = 0;
    let mut paid_total: i64 = 0;

    // C4 multi-currency: discover the asset's payout currency once; route the
    // wallet credit and the wallet_transactions row through it. USD/IDR are
    // fully supported; other currencies require an fx_rates_daily entry from
    // IDR to that currency (deferred — no EUR/USDT asset in dev to test against).
    let payout_currency: String =
        sqlx::query_scalar("SELECT COALESCE(payout_currency, 'USD') FROM assets WHERE id = $1")
            .bind(asset_id)
            .fetch_one(pool)
            .await
            .map_err(ApiError::Database)?;

    let mut tx = pool.begin().await.map_err(ApiError::Database)?;
    for p in &pending {
        // 1. Ensure user's cash wallet in the asset's payout currency.
        let wallet_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO wallets (user_id, wallet_type, currency)
            VALUES ($1, 'cash', $2)
            ON CONFLICT (user_id, wallet_type, currency)
                DO UPDATE SET updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(p.user_id)
        .bind(&payout_currency)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        // 2. Credit balance.
        sqlx::query(
            r#"
            UPDATE wallets
               SET balance_cents = balance_cents + $1,
                   updated_at = NOW()
             WHERE id = $2
            "#,
        )
        .bind(p.amount_cents)
        .bind(wallet_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        // 3. wallet_transactions row in the asset's payout currency.
        let description = format!(
            "Villa rental dividend — {}-{:02}",
            log_row.period_year, log_row.period_month
        );
        let new_tx_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO wallet_transactions
                (wallet_id, amount_cents, type, status, description, currency)
            VALUES ($1, $2, 'dividend', 'completed', $3, $4)
            RETURNING id
            "#,
        )
        .bind(wallet_id)
        .bind(p.amount_cents)
        .bind(&description)
        .bind(&payout_currency)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        // 4. Investment lifetime rental stat.
        sqlx::query(
            r#"
            UPDATE investments
               SET total_rental_cents = total_rental_cents + $1,
                   updated_at = NOW()
             WHERE user_id = $2 AND asset_id = $3
            "#,
        )
        .bind(p.amount_cents)
        .bind(p.user_id)
        .bind(asset_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        // 5. Link + flip payout to paid.
        sqlx::query(
            r#"
            UPDATE dividend_payouts
               SET status = 'paid',
                   wallet_tx_id = $1,
                   paid_at = NOW()
             WHERE id = $2 AND status = 'scheduled'
            "#,
        )
        .bind(new_tx_id)
        .bind(p.id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        // 6. In-app notification for the investor (W15).
        let _ = sqlx::query(
            r#"
            INSERT INTO notifications (user_id, title, message, type, action_url)
            VALUES ($1, $2, $3, 'payout', '/transactions')
            "#,
        )
        .bind(p.user_id)
        .bind(format!(
            "Dividend paid — {}-{:02}",
            log_row.period_year, log_row.period_month
        ))
        .bind(format!(
            "{} {:.2} credited to your cash wallet for {}-{:02} villa rental dividend.",
            payout_currency,
            (p.amount_cents as f64) / 100.0,
            log_row.period_year,
            log_row.period_month
        ))
        .execute(&mut *tx)
        .await;

        // 7. Transactional email — fire-and-forget outside the tx.
        let email_pool = pool.clone();
        let email_user = p.user_id;
        let email_meta = serde_json::json!({
            "period_year":    log_row.period_year,
            "period_month":   log_row.period_month,
            "amount_cents":   p.amount_cents,
            "currency":       payout_currency.clone(),
            "asset_id":       asset_id,
        });
        tokio::spawn(async move {
            let _ = crate::email::trigger_transactional_email(
                &email_pool, &email_user, "dividend_payout", email_meta,
            ).await;
        });

        paid_count += 1;
        paid_total += p.amount_cents;
    }

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_ops.process_payouts', 'villa_operations_log', NULL, $2)
        "#,
    )
    .bind(actor_id)
    .bind(serde_json::json!({
        "log_id": log_id,
        "asset_id": asset_id,
        "paid_count": paid_count,
        "paid_total_cents": paid_total,
        "skipped_already_paid": already_paid,
    }))
    .execute(&mut *tx)
    .await;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(ProcessPayoutsResult {
        paid_count,
        paid_total_cents: paid_total,
        skipped_already_paid: already_paid,
    })
}

pub async fn api_admin_villa_operations_process_payouts(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<ProcessPayoutsResult>, ApiError> {
    let result = process_payouts_core(&state.db, asset_id, log_id, admin.user.id).await?;
    Ok(Json(result))
}

/// GET /api/admin/villa-operations-queue — cross-asset queue of submitted rows.
pub async fn api_admin_villa_operations_queue(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<QueueRow>>, ApiError> {
    let rows: Vec<QueueRow> = sqlx::query_as(
        r#"
        SELECT
            l.id,
            l.asset_id,
            a.title       AS asset_title,
            l.period_year,
            l.period_month,
            l.distributable_idr_cents,
            l.status,
            l.supersedes_id,
            l.correction_reason,
            l.submitted_by,
            u.email       AS submitter_email,
            l.submitted_at,
            l.recorded_at
        FROM villa_operations_log l
        JOIN assets a    ON a.id = l.asset_id
        LEFT JOIN users u ON u.id = l.submitted_by
        WHERE l.status = 'submitted'
        ORDER BY l.submitted_at ASC NULLS LAST, l.recorded_at ASC
        LIMIT 200
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct QueueRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub asset_title: String,
    pub period_year: i32,
    pub period_month: i32,
    pub distributable_idr_cents: i64,
    pub status: String,
    pub supersedes_id: Option<i64>,
    pub correction_reason: Option<String>,
    pub submitted_by: Option<Uuid>,
    pub submitter_email: Option<String>,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/admin/villas/:asset_id/operations/:log_id/documents
/// Link an existing `asset_documents` row to a specific monthly operations period
/// (Villa-Returns A1.a). Doc must already exist in asset_documents — this endpoint
/// only writes the link row in `villa_period_documents`.
#[derive(Debug, Deserialize)]
pub struct LinkDocumentInput {
    pub document_id: Uuid,
    pub doc_type: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PeriodDocumentRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub period_year: i32,
    pub period_month: i32,
    pub log_id: Option<i64>,
    pub document_id: Uuid,
    pub doc_type: String,
    pub uploaded_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn api_admin_villa_operations_link_document(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
    Json(input): Json<LinkDocumentInput>,
) -> Result<Json<PeriodDocumentRow>, ApiError> {
    let row = load_row(&state.db, log_id).await?;
    if row.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if input.doc_type.trim().is_empty() {
        return Err(ApiError::BadRequest("doc_type required".to_string()));
    }

    let inserted: PeriodDocumentRow = sqlx::query_as(
        r#"
        INSERT INTO villa_period_documents
            (asset_id, period_year, period_month, log_id, document_id, doc_type, uploaded_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, asset_id, period_year, period_month, log_id, document_id, doc_type, uploaded_by, created_at
        "#,
    )
    .bind(asset_id)
    .bind(row.period_year)
    .bind(row.period_month)
    .bind(log_id)
    .bind(input.document_id)
    .bind(&input.doc_type)
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.constraint() == Some("villa_period_documents_period_year_period_month_document_id_key") => {
            ApiError::Conflict("This document is already linked to this period".to_string())
        }
        _ => ApiError::Database(e),
    })?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_ops.link_document', 'villa_period_documents', NULL, $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::to_value(&inserted).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await;

    Ok(Json(inserted))
}

/// GET /api/admin/villas/:asset_id/operations/:log_id/documents — list linked docs.
pub async fn api_admin_villa_operations_documents_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<Vec<PeriodDocumentRow>>, ApiError> {
    let rows: Vec<PeriodDocumentRow> = sqlx::query_as(
        r#"
        SELECT id, asset_id, period_year, period_month, log_id, document_id, doc_type, uploaded_by, created_at
        FROM villa_period_documents
        WHERE asset_id = $1 AND log_id = $2
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .bind(log_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

/// POST /api/admin/villas/:asset_id/operations/:log_id/distribute
/// Generate `dividend_payouts` rows pro-rata across investors holding tokens
/// at the period record date. Idempotent: uq_dividend_payouts_villa_period
/// prevents double-paying the same (asset, user, period).
#[derive(Debug, Serialize)]
pub struct DistributeResult {
    pub created: i64,
    pub skipped: i64,
    pub total_paid_cents: i64,
    pub currency: String,
}

/// Core distribute logic — extracted so it can be called from both the HTTP
/// handler and the post-publish background task.
pub(crate) async fn distribute_core(
    pool: &PgPool,
    asset_id: Uuid,
    log_id: i64,
    actor_id: Uuid,
) -> Result<DistributeResult, ApiError> {
    let row = load_row(pool, log_id).await?;
    if row.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if row.status != "published" {
        return Err(ApiError::Conflict(format!(
            "Cannot distribute from row in status='{}'",
            row.status
        )));
    }

    // Asset-level config: distribution_record_day + payout_currency + token denominators.
    let cfg: (
        Option<i32>,
        Option<String>,
        Option<i64>,
        Option<i32>,
        Option<i32>,
    ) = sqlx::query_as(
        r#"
        SELECT
            distribution_record_day,
            payout_currency,
            tokens_total::BIGINT,
            tokens_payout_eligible,
            tokens_owner_retained
        FROM assets WHERE id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    let record_day = cfg.0.unwrap_or(1).clamp(1, 28);
    let payout_currency = cfg.1.unwrap_or_else(|| "USD".to_string());
    let tokens_total = cfg.2.unwrap_or(0);
    let tokens_owner_retained = cfg.4.unwrap_or(0);
    let denominator: i64 = cfg
        .3
        .map(i64::from)
        .unwrap_or(tokens_total - i64::from(tokens_owner_retained));

    if denominator <= 0 {
        return Err(ApiError::Conflict(
            "Asset has no payout-eligible tokens configured".to_string(),
        ));
    }

    let total_distributable: i64 = match payout_currency.as_str() {
        "USD" => row.distributable_usd_cents,
        "IDR" => row.distributable_idr_cents,
        other => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported payout currency '{}' — only USD and IDR supported in P2.7",
                other
            )));
        }
    };

    if total_distributable <= 0 {
        return Ok(DistributeResult {
            created: 0,
            skipped: 0,
            total_paid_cents: 0,
            currency: payout_currency,
        });
    }

    // Record date = end of period_month, day=record_day at 23:59:59 UTC.
    // Simpler: last second of period_year/period_month for MVP. Record-day override deferred.
    let record_date_sql = format!(
        "make_timestamptz({}, {}, 1, 0, 0, 0) + INTERVAL '1 month' - INTERVAL '1 second'",
        row.period_year, row.period_month
    );
    let _ = record_day; // record_day override is planned but not used in P2.7 (records-end-of-month).

    // Eligible investors at record date.
    #[derive(sqlx::FromRow)]
    struct Eligible {
        id: Uuid,
        user_id: Uuid,
        tokens_owned: i32,
    }
    let eligible_sql = format!(
        r#"
        SELECT id, user_id, tokens_owned
        FROM investments
        WHERE asset_id = $1
          AND tokens_owned > 0
          AND purchased_at <= {record_date_sql}
          AND status IN ('active','funded','rented','payout_pending')
        "#,
    );
    let eligible: Vec<Eligible> = sqlx::query_as(&eligible_sql)
        .bind(asset_id)
        .fetch_all(pool)
        .await
        .map_err(ApiError::Database)?;

    let mut created: i64 = 0;
    let mut skipped: i64 = 0;
    let mut total_paid: i64 = 0;

    // One tx for the whole batch.
    let mut tx = pool.begin().await.map_err(ApiError::Database)?;
    for inv in &eligible {
        // Pro-rata payout, integer arithmetic. `i128` intermediate.
        let amount: i64 =
            ((total_distributable as i128) * inv.tokens_owned as i128 / denominator as i128) as i64;
        if amount <= 0 {
            skipped += 1;
            continue;
        }

        let inserted: Option<(i64,)> = sqlx::query_as(
            r#"
            INSERT INTO dividend_payouts
                (investment_id, user_id, asset_id, amount_cents, payout_type, status,
                 scheduled_at, source_villa_operations_log_id, period_year, period_month)
            VALUES ($1, $2, $3, $4, 'rental', 'scheduled',
                    NOW(), $5, $6, $7)
            ON CONFLICT (asset_id, user_id, period_year, period_month)
              WHERE period_year IS NOT NULL AND payout_type = 'rental'
              DO NOTHING
            RETURNING amount_cents
            "#,
        )
        .bind(inv.id)
        .bind(inv.user_id)
        .bind(asset_id)
        .bind(amount)
        .bind(log_id)
        .bind(row.period_year)
        .bind(row.period_month)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Database)?;

        match inserted {
            Some((amt,)) => {
                created += 1;
                total_paid += amt;
            }
            None => skipped += 1,
        }
    }

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_ops.distribute', 'villa_operations_log', NULL, $2)
        "#,
    )
    .bind(actor_id)
    .bind(serde_json::json!({
        "log_id": log_id,
        "asset_id": asset_id,
        "period_year": row.period_year,
        "period_month": row.period_month,
        "created": created,
        "skipped": skipped,
        "total_paid_cents": total_paid,
        "currency": payout_currency,
    }))
    .execute(&mut *tx)
    .await;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(DistributeResult {
        created,
        skipped,
        total_paid_cents: total_paid,
        currency: payout_currency,
    })
}

pub async fn api_admin_villa_operations_distribute(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<DistributeResult>, ApiError> {
    let result = distribute_core(&state.db, asset_id, log_id, admin.user.id).await?;
    Ok(Json(result))
}

/// GET /api/admin/villas/:asset_id/config-summary — full Villa-Returns config bundle.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VillaConfigSummary {
    pub tokenized_pct_bps: Option<i32>,
    pub tokens_total: i32,
    pub tokens_owner_retained: Option<i32>,
    pub tokens_payout_eligible: Option<i32>,
    pub reserve_pct_bps: i32,
    pub mgmt_fee_bps: Option<i32>,
    pub poool_split_pct: i32,
    pub withholding_tax_bps: i32,
    pub payout_frequency: String,
    pub payout_currency: String,
    pub distribution_record_day: i32,
    pub native_currency_code: String,
    pub allow_developer_submission: bool,
    pub villa_returns_pilot: bool,
}

pub async fn api_admin_villa_config_summary(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<Json<VillaConfigSummary>, ApiError> {
    let cfg: VillaConfigSummary = sqlx::query_as(
        r#"
        SELECT
            tokenized_pct_bps,
            tokens_total,
            tokens_owner_retained,
            tokens_payout_eligible,
            reserve_pct_bps,
            mgmt_fee_bps,
            COALESCE(poool_split_pct, 0) AS poool_split_pct,
            withholding_tax_bps,
            payout_frequency,
            payout_currency,
            distribution_record_day,
            native_currency_code,
            allow_developer_submission,
            villa_returns_pilot
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

/// PUT /api/admin/villas/:asset_id/config — partial update of Villa-Returns config.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct VillaConfigInput {
    pub tokenized_pct_bps: Option<i32>,
    pub tokens_owner_retained: Option<i32>,
    pub tokens_payout_eligible: Option<i32>,
    pub reserve_pct_bps: Option<i32>,
    pub mgmt_fee_bps: Option<i32>,
    pub withholding_tax_bps: Option<i32>,
    pub payout_frequency: Option<String>,
    pub payout_currency: Option<String>,
    pub distribution_record_day: Option<i32>,
    pub allow_developer_submission: Option<bool>,
    pub villa_returns_pilot: Option<bool>,
}

pub async fn api_admin_villa_config_update(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<VillaConfigInput>,
) -> Result<Json<VillaConfigSummary>, ApiError> {
    // Range validation up-front so partial inputs fail with clear messages.
    if let Some(v) = input.tokenized_pct_bps {
        if !(0..=10_000).contains(&v) {
            return Err(ApiError::BadRequest(
                "tokenized_pct_bps must be 0–10000".to_string(),
            ));
        }
    }
    if let Some(v) = input.reserve_pct_bps {
        if !(0..=10_000).contains(&v) {
            return Err(ApiError::BadRequest(
                "reserve_pct_bps must be 0–10000".to_string(),
            ));
        }
    }
    if let Some(v) = input.withholding_tax_bps {
        if !(0..=10_000).contains(&v) {
            return Err(ApiError::BadRequest(
                "withholding_tax_bps must be 0–10000".to_string(),
            ));
        }
    }
    if let Some(v) = input.mgmt_fee_bps {
        if !(0..=10_000).contains(&v) {
            return Err(ApiError::BadRequest(
                "mgmt_fee_bps must be 0–10000".to_string(),
            ));
        }
    }
    if let Some(v) = input.distribution_record_day {
        if !(1..=28).contains(&v) {
            return Err(ApiError::BadRequest(
                "distribution_record_day must be 1–28".to_string(),
            ));
        }
    }
    if let Some(ref f) = input.payout_frequency {
        if !matches!(f.as_str(), "monthly" | "quarterly" | "annual") {
            return Err(ApiError::BadRequest(
                "payout_frequency must be monthly|quarterly|annual".to_string(),
            ));
        }
    }
    if let Some(ref c) = input.payout_currency {
        if c.len() != 3 {
            return Err(ApiError::BadRequest(
                "payout_currency must be 3-char ISO code".to_string(),
            ));
        }
    }

    let cfg: VillaConfigSummary = sqlx::query_as(
        r#"
        UPDATE assets SET
            tokenized_pct_bps          = COALESCE($2,  tokenized_pct_bps),
            tokens_owner_retained      = COALESCE($3,  tokens_owner_retained),
            tokens_payout_eligible     = COALESCE($4,  tokens_payout_eligible),
            reserve_pct_bps            = COALESCE($5,  reserve_pct_bps),
            mgmt_fee_bps               = COALESCE($6,  mgmt_fee_bps),
            withholding_tax_bps        = COALESCE($7,  withholding_tax_bps),
            payout_frequency           = COALESCE($8,  payout_frequency),
            payout_currency            = COALESCE($9,  payout_currency),
            distribution_record_day    = COALESCE($10, distribution_record_day),
            allow_developer_submission = COALESCE($11, allow_developer_submission),
            villa_returns_pilot        = COALESCE($12, villa_returns_pilot)
        WHERE id = $1
        RETURNING
            tokenized_pct_bps, tokens_total, tokens_owner_retained, tokens_payout_eligible,
            reserve_pct_bps, mgmt_fee_bps, COALESCE(poool_split_pct, 0) AS poool_split_pct,
            withholding_tax_bps, payout_frequency, payout_currency, distribution_record_day,
            native_currency_code, allow_developer_submission, villa_returns_pilot
        "#,
    )
    .bind(asset_id)
    .bind(input.tokenized_pct_bps)
    .bind(input.tokens_owner_retained)
    .bind(input.tokens_payout_eligible)
    .bind(input.reserve_pct_bps)
    .bind(input.mgmt_fee_bps)
    .bind(input.withholding_tax_bps)
    .bind(input.payout_frequency)
    .bind(input.payout_currency)
    .bind(input.distribution_record_day)
    .bind(input.allow_developer_submission)
    .bind(input.villa_returns_pilot)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'villa_config.update', 'assets', $2, $3)
        "#,
    )
    .bind(admin.user.id)
    .bind(asset_id)
    .bind(serde_json::to_value(&cfg).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await;

    Ok(Json(cfg))
}

/// GET /api/admin/villas/:asset_id/operations?year=&month=&as_of=
pub async fn api_admin_villa_operations_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Query(q): Query<OperationsQuery>,
) -> Result<Json<Vec<VillaOperationsRow>>, ApiError> {
    let rows: Vec<VillaOperationsRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_operations_log
        WHERE asset_id = $1
          AND ($2::INT IS NULL OR period_year  = $2)
          AND ($3::INT IS NULL OR period_month = $3)
          AND ($4::TIMESTAMPTZ IS NULL OR recorded_at <= $4)
        ORDER BY period_year DESC, period_month DESC, recorded_at DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .bind(q.year)
    .bind(q.month)
    .bind(q.as_of)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

// ─── Internal helpers ─────────────────────────────────────────

async fn load_row(pool: &PgPool, log_id: i64) -> Result<VillaOperationsRow, ApiError> {
    sqlx::query_as::<_, VillaOperationsRow>("SELECT * FROM villa_operations_log WHERE id = $1")
        .bind(log_id)
        .fetch_optional(pool)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Operations row not found".to_string()))
}

async fn transition(
    pool: &PgPool,
    asset_id: Uuid,
    log_id: i64,
    actor: Uuid,
    action: &str,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    let existing = load_row(pool, log_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }

    let (new_status, set_ts_col) = match (existing.status.as_str(), action) {
        ("draft", "submit") => ("submitted", "submitted_at"),
        ("submitted", "approve") => ("approved", "approved_at"),
        _ => {
            return Err(ApiError::Conflict(format!(
                "Cannot apply action='{}' to row in status='{}'",
                action, existing.status
            )))
        }
    };

    let sql = format!(
        "UPDATE villa_operations_log
            SET status = $2, {set_ts_col} = NOW(),
                {actor_col} = $3
          WHERE id = $1 AND status = $4
          RETURNING *",
        set_ts_col = set_ts_col,
        actor_col = if action == "approve" {
            "approved_by"
        } else {
            "submitted_by"
        },
    );

    let row: VillaOperationsRow = sqlx::query_as(&sql)
        .bind(log_id)
        .bind(new_status)
        .bind(actor)
        .bind(&existing.status)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;

    // Notify the submitter on key transitions (W15).
    if matches!(action, "approve") {
        if let Some(submitter_id) = row.submitted_by {
            let _ = sqlx::query(
                r#"
                INSERT INTO notifications (user_id, title, message, type, action_url)
                VALUES ($1, $2, $3, 'system', $4)
                "#,
            )
            .bind(submitter_id)
            .bind(format!(
                "Operations approved — {}-{:02}",
                row.period_year, row.period_month
            ))
            .bind("Admin approved your monthly operations submission. Pending publish.".to_string())
            .bind(format!(
                "/admin/villas/{}/operations/{}/{}?log_id={}",
                row.asset_id, row.period_year, row.period_month, row.id
            ))
            .execute(pool)
            .await;

            let _ = crate::email::trigger_transactional_email(
                pool,
                &submitter_id,
                "operations_approved",
                serde_json::json!({
                    "period_year":  row.period_year,
                    "period_month": row.period_month,
                    "asset_id":     row.asset_id,
                }),
            )
            .await;
        }
    }

    write_audit(pool, actor, action, row.id, &row, Some(&existing)).await;
    Ok(Json(row))
}

async fn write_audit(
    pool: &PgPool,
    actor: Uuid,
    action: &str,
    entity_id: i64,
    new_state: &VillaOperationsRow,
    previous_state: Option<&VillaOperationsRow>,
) {
    let new_json = serde_json::to_value(new_state).unwrap_or(serde_json::Value::Null);
    let prev_json = previous_state
        .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
        VALUES ($1, $2, 'villa_operations_log', NULL, $3, $4)
        "#,
    )
    .bind(actor)
    .bind(format!("villa_ops.{}", action))
    .bind(prev_json)
    .bind(new_json)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!(
            "Failed to write audit_logs for villa_operations_log id={} action={}: {}",
            entity_id, action, e
        );
        e
    });
}
