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
use axum::extract::{Multipart, Path, Query, State};
use axum::Json;
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// ─── C-5: custom-expense breakdown ───────────────────────────────────────────
//
// Migration 202 added `villa_operations_log.expense_other_notes` (JSONB NULL)
// to capture the named "other" expense rows the developer types into the
// submit form. Prior to this fix, those user-typed names were dropped on the
// client before the request left the browser and only the summed amount was
// stored in `expense_other_idr_cents`. The new column is additive — the
// existing `expense_other_idr_cents` keeps its semantics (sum of catch-all +
// custom rows) and continues to feed `compute_totals()`.
//
// Wrappers below let us reuse the shared admin DTOs without touching the
// admin module: `DeveloperOpsInput` extends `VillaOperationsInput` with the
// new write-side field, `DeveloperOpsRow` extends `VillaOperationsRow` with
// the read-side field. Both use `#[serde(flatten)]` so the JSON shape on the
// wire is identical to the prior contract plus the one new key.

#[derive(Debug, Deserialize, Default)]
pub struct DeveloperOpsInput {
    #[serde(flatten)]
    pub base: VillaOperationsInput,
    /// `[{"name": "Garbage collection", "amount_idr_cents": 250000}, …]`.
    /// Null/empty array = no custom rows for this period.
    pub expense_other_notes: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DeveloperOpsRow {
    #[serde(flatten)]
    pub base: VillaOperationsRow,
    pub expense_other_notes: Option<serde_json::Value>,
}

/// Read the JSONB breakdown for a given log id. Used to assemble
/// `DeveloperOpsRow` responses after INSERT/UPDATE and on the GET paths.
async fn load_other_notes(
    pool: &sqlx::PgPool,
    log_id: i64,
) -> Result<Option<serde_json::Value>, ApiError> {
    let (notes,): (Option<serde_json::Value>,) =
        sqlx::query_as("SELECT expense_other_notes FROM villa_operations_log WHERE id = $1")
            .bind(log_id)
            .fetch_one(pool)
            .await
            .map_err(ApiError::Database)?;
    Ok(notes)
}

/// Period documents (receipts / invoices / statements) cap at 20 MB,
/// matching the admin asset-document limit.
const MAX_PERIOD_DOC_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct OperationsQuery {
    pub year: Option<i32>,
    pub month: Option<i32>,
}

// ─── Matrix dashboard response types ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MatrixPeriodCell {
    pub month: i32,
    pub log_id: i64,
    pub status: String,
    pub rejected_reason: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub has_period_docs: bool,
}

#[derive(Debug, Serialize)]
pub struct MatrixAssetEntry {
    pub asset_id: Uuid,
    pub asset_title: String,
    pub listed_year: i32,
    pub listed_month: i32,
    pub prior_published_count: i64,
    pub prior_expected_count: i64,
    pub annual_doc_year: i32,
    pub annual_doc_uploaded: bool,
    pub periods: Vec<MatrixPeriodCell>,
}

#[derive(Debug, Serialize)]
pub struct MatrixDashboardResponse {
    pub year: i32,
    pub assets: Vec<MatrixAssetEntry>,
}

// Flat row returned by the SQL query — assembled into MatrixAssetEntry in Rust.
#[derive(Debug, sqlx::FromRow)]
struct MatrixFlatRow {
    asset_id: Uuid,
    asset_title: String,
    listed_year: i32,
    listed_month: i32,
    prior_published_count: i64,
    prior_expected_count: i64,
    annual_doc_year: i32,
    annual_doc_uploaded: bool,
    period_month: Option<i32>,
    log_id: Option<i64>,
    period_status: Option<String>,
    rejected_reason: Option<String>,
    recorded_at: Option<chrono::DateTime<chrono::Utc>>,
    has_period_docs: bool,
}

/// GET /api/developer/operations/dashboard?year=YYYY
/// Returns all assigned villas with per-month status for the requested year.
pub async fn api_developer_operations_dashboard(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<OperationsQuery>,
) -> Result<Json<MatrixDashboardResponse>, ApiError> {
    let year = q.year.unwrap_or_else(|| chrono::Utc::now().year());

    let rows: Vec<MatrixFlatRow> = sqlx::query_as(
        r#"
        WITH
        dev_assets AS (
            SELECT a.id AS asset_id, a.title AS asset_title, dal.effective_from
            FROM developer_asset_links dal
            JOIN assets a ON a.id = dal.asset_id
            WHERE dal.developer_user_id = $1 AND dal.effective_until IS NULL
        ),
        latest_year_ops AS (
            SELECT DISTINCT ON (l.asset_id, l.period_month)
                   l.asset_id, l.id AS log_id, l.period_month,
                   l.status, l.rejected_reason, l.recorded_at
            FROM villa_operations_log l
            WHERE l.period_year = $2
              AND l.asset_id IN (SELECT asset_id FROM dev_assets)
            ORDER BY l.asset_id, l.period_month, l.recorded_at DESC
        ),
        period_docs_agg AS (
            SELECT asset_id, period_month
            FROM villa_period_documents
            WHERE period_year = $2
              AND asset_id IN (SELECT asset_id FROM dev_assets)
            GROUP BY asset_id, period_month
        ),
        prior_ops_latest AS (
            SELECT DISTINCT ON (l.asset_id, l.period_year, l.period_month)
                   l.asset_id, l.status
            FROM villa_operations_log l
            WHERE l.period_year < $2
              AND l.asset_id IN (SELECT asset_id FROM dev_assets)
            ORDER BY l.asset_id, l.period_year, l.period_month, l.recorded_at DESC
        ),
        prior_agg AS (
            SELECT asset_id,
                   COUNT(*) FILTER (WHERE status IN ('published','approved')) AS published_count,
                   COUNT(*)                                                    AS expected_count
            FROM prior_ops_latest
            GROUP BY asset_id
        ),
        annual_docs_agg AS (
            SELECT asset_id
            FROM villa_annual_documents
            WHERE period_year = $2 - 1
              AND asset_id IN (SELECT asset_id FROM dev_assets)
            GROUP BY asset_id
        )
        SELECT
            da.asset_id,
            da.asset_title,
            EXTRACT(YEAR  FROM da.effective_from)::int  AS listed_year,
            EXTRACT(MONTH FROM da.effective_from)::int  AS listed_month,
            COALESCE(pa.published_count, 0)              AS prior_published_count,
            COALESCE(pa.expected_count,  0)              AS prior_expected_count,
            ($2 - 1)                                     AS annual_doc_year,
            (ad.asset_id IS NOT NULL)                    AS annual_doc_uploaded,
            lyo.period_month,
            lyo.log_id,
            lyo.status                                   AS period_status,
            lyo.rejected_reason,
            lyo.recorded_at,
            (pda.asset_id IS NOT NULL)                   AS has_period_docs
        FROM dev_assets da
        LEFT JOIN latest_year_ops lyo ON lyo.asset_id = da.asset_id
        LEFT JOIN period_docs_agg pda  ON pda.asset_id = da.asset_id
                                      AND pda.period_month = lyo.period_month
        LEFT JOIN prior_agg pa         ON pa.asset_id  = da.asset_id
        LEFT JOIN annual_docs_agg ad   ON ad.asset_id  = da.asset_id
        ORDER BY da.asset_title, lyo.period_month
        "#,
    )
    .bind(dev.user.id)
    .bind(year)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Group flat rows by asset_id (BTreeMap preserves insertion order by key,
    // but assets arrive in title order from SQL so we keep a Vec for ordering).
    let mut seen: BTreeMap<Uuid, usize> = BTreeMap::new();
    let mut assets: Vec<MatrixAssetEntry> = Vec::new();

    for row in rows {
        let idx = if let Some(&i) = seen.get(&row.asset_id) {
            i
        } else {
            let i = assets.len();
            assets.push(MatrixAssetEntry {
                asset_id: row.asset_id,
                asset_title: row.asset_title.clone(),
                listed_year: row.listed_year,
                listed_month: row.listed_month,
                prior_published_count: row.prior_published_count,
                prior_expected_count: row.prior_expected_count,
                annual_doc_year: row.annual_doc_year,
                annual_doc_uploaded: row.annual_doc_uploaded,
                periods: Vec::new(),
            });
            seen.insert(row.asset_id, i);
            i
        };

        if let (Some(month), Some(log_id), Some(status), Some(recorded_at)) = (
            row.period_month,
            row.log_id,
            row.period_status,
            row.recorded_at,
        ) {
            assets[idx].periods.push(MatrixPeriodCell {
                month,
                log_id,
                status,
                rejected_reason: row.rejected_reason,
                recorded_at,
                has_period_docs: row.has_period_docs,
            });
        }
    }

    Ok(Json(MatrixDashboardResponse { year, assets }))
}

/// POST /api/developer/villas/:asset_id/operations — create draft (dev-owned fields only).
pub async fn api_developer_villa_operations_create(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(payload): Json<DeveloperOpsInput>,
) -> Result<Json<DeveloperOpsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    let DeveloperOpsInput {
        mut base,
        expense_other_notes,
    } = payload;
    let input = &mut base;
    // Developer cannot set the reserve override — strip it server-side.
    input.reserve_override_idr_cents = None;

    if !(2000..=2100).contains(&input.period_year) || !(1..=12).contains(&input.period_month) {
        return Err(ApiError::BadRequest("Invalid period".to_string()));
    }

    let (reserve_pct, platform_pct, withholding_bps) =
        load_asset_config(&state.db, asset_id).await?;
    let totals = compute_totals(input, reserve_pct, platform_pct, withholding_bps, None);

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
            reserve_applied_idr_cents, platform_fee_idr_cents,
            withholding_idr_cents, distributable_idr_cents,
            mgmt_reported_distributable_idr_cents,
            expense_other_notes,
            status, supersedes_id, correction_reason, submitted_by
        ) VALUES (
            $1,  $2,  $3,  $4,  $5,  $6,  $7,  $8,  $9,  $10, $11, $12, $13, $14,
            $15, $16, $17, $18, $19,
            $20, $21, $22, $23,
            $24, $25, $26, $27, $28, $29, $30,
            $31,
            'draft', $32, $33, $34
        )
        RETURNING *
        "#,
    )
    .bind(asset_id) // $1
    .bind(input.period_year) // $2
    .bind(input.period_month) // $3
    .bind(input.gross_rental_idr_cents) // $4
    .bind(
        input
            .currency_code
            .clone()
            .unwrap_or_else(|| "IDR".to_string()),
    ) // $5
    .bind(input.nights_available) // $6
    .bind(input.nights_booked) // $7
    .bind(input.expense_cleaning_idr_cents) // $8
    .bind(input.expense_maintenance_idr_cents) // $9
    .bind(input.expense_utilities_idr_cents) // $10
    .bind(input.expense_staff_idr_cents) // $11
    .bind(input.expense_pool_garden_idr_cents) // $12
    .bind(input.expense_pest_idr_cents) // $13
    .bind(input.expense_other_idr_cents) // $14
    .bind(input.expense_property_tax_idr_cents) // $15
    .bind(input.expense_insurance_idr_cents) // $16
    .bind(input.expense_accounting_idr_cents) // $17
    .bind(input.expense_internet_idr_cents) // $18
    .bind(input.expense_capex_idr_cents) // $19
    .bind(input.ota_fees_idr_cents) // $20
    .bind(input.payment_fees_idr_cents) // $21
    .bind(input.refunds_idr_cents) // $22
    .bind(input.mgmt_fee_idr_cents) // $23
    .bind(totals.total_opex_idr_cents) // $24
    .bind(totals.net_rental_income_idr_cents) // $25
    .bind(totals.reserve_applied_idr_cents) // $26
    .bind(totals.platform_fee_idr_cents) // $27
    .bind(totals.withholding_idr_cents) // $28
    .bind(totals.distributable_idr_cents) // $29
    .bind(input.mgmt_reported_distributable_idr_cents) // $30
    .bind(expense_other_notes.clone()) // $31
    .bind(input.supersedes_id) // $32
    .bind(input.correction_reason.clone()) // $33
    .bind(dev.user.id) // $34
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(DeveloperOpsRow {
        base: row,
        expense_other_notes,
    }))
}

/// PUT /api/developer/villas/:asset_id/operations/:log_id — edit own draft.
pub async fn api_developer_villa_operations_update(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
    Json(payload): Json<DeveloperOpsInput>,
) -> Result<Json<DeveloperOpsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    let DeveloperOpsInput {
        mut base,
        expense_other_notes,
    } = payload;
    let input = &mut base;
    input.reserve_override_idr_cents = None;

    let existing: VillaOperationsRow =
        sqlx::query_as("SELECT * FROM villa_operations_log WHERE id = $1 AND asset_id = $2")
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
    let totals = compute_totals(input, reserve_pct, platform_pct, withholding_bps, None);

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
            reserve_applied_idr_cents                = $23,
            platform_fee_idr_cents                   = $24,
            withholding_idr_cents                    = $25,
            distributable_idr_cents                  = $26,
            mgmt_reported_distributable_idr_cents    = $27,
            expense_other_notes                      = $28
        WHERE id = $1 AND status = 'draft'
        RETURNING *
        "#,
    )
    .bind(log_id) // $1
    .bind(input.gross_rental_idr_cents) // $2
    .bind(input.nights_available) // $3
    .bind(input.nights_booked) // $4
    .bind(input.expense_cleaning_idr_cents) // $5
    .bind(input.expense_maintenance_idr_cents) // $6
    .bind(input.expense_utilities_idr_cents) // $7
    .bind(input.expense_staff_idr_cents) // $8
    .bind(input.expense_pool_garden_idr_cents) // $9
    .bind(input.expense_pest_idr_cents) // $10
    .bind(input.expense_other_idr_cents) // $11
    .bind(input.expense_property_tax_idr_cents) // $12
    .bind(input.expense_insurance_idr_cents) // $13
    .bind(input.expense_accounting_idr_cents) // $14
    .bind(input.expense_internet_idr_cents) // $15
    .bind(input.expense_capex_idr_cents) // $16
    .bind(input.ota_fees_idr_cents) // $17
    .bind(input.payment_fees_idr_cents) // $18
    .bind(input.refunds_idr_cents) // $19
    .bind(input.mgmt_fee_idr_cents) // $20
    .bind(totals.total_opex_idr_cents) // $21
    .bind(totals.net_rental_income_idr_cents) // $22
    .bind(totals.reserve_applied_idr_cents) // $23
    .bind(totals.platform_fee_idr_cents) // $24
    .bind(totals.withholding_idr_cents) // $25
    .bind(totals.distributable_idr_cents) // $26
    .bind(input.mgmt_reported_distributable_idr_cents) // $27
    .bind(expense_other_notes.clone()) // $28
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(DeveloperOpsRow {
        base: row,
        expense_other_notes,
    }))
}

/// PUT /api/developer/villas/:asset_id/operations/:log_id/submit — submit own draft.
pub async fn api_developer_villa_operations_submit(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<VillaOperationsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let existing: VillaOperationsRow =
        sqlx::query_as("SELECT * FROM villa_operations_log WHERE id = $1 AND asset_id = $2")
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
        dev.user.email, asset_id, row.distributable_idr_cents as f64
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
) -> Result<Json<Vec<DeveloperOpsRow>>, ApiError> {
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

    // Side-fetch `expense_other_notes` for each row. `VillaOperationsRow`
    // lives in the admin module and we cannot widen it from here; the
    // wrapper merges the JSONB column into the wire shape.
    let mut out: Vec<DeveloperOpsRow> = Vec::with_capacity(rows.len());
    for base in rows {
        let log_id = base.id;
        let notes = load_other_notes(&state.db, log_id).await?;
        out.push(DeveloperOpsRow {
            base,
            expense_other_notes: notes,
        });
    }
    Ok(Json(out))
}

/// GET /api/developer/villas/:asset_id/operations/:log_id — single-log read.
///
/// C-4 fix: the operations dashboard links draft/in-review/rejected matrix
/// cells to `/developer/villas/:asset_id/operations/:log_id`. The edit page
/// uses this endpoint to populate the form. Same auth model and per-villa
/// gate as the list endpoint.
pub async fn api_developer_villa_operations_get(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<DeveloperOpsRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    let base: VillaOperationsRow = sqlx::query_as(
        r#"
        SELECT * FROM villa_operations_log
        WHERE id = $1 AND asset_id = $2
        "#,
    )
    .bind(log_id)
    .bind(asset_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Operations row not found".to_string()))?;

    let expense_other_notes = load_other_notes(&state.db, base.id).await?;
    Ok(Json(DeveloperOpsRow {
        base,
        expense_other_notes,
    }))
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

// ─── Period documents (PDF §2 items 16-17 — developer submission path) ───
//
// The admin path is 2-step: upload to a generic asset_documents endpoint,
// then link. There is no developer-accessible generic upload (the existing
// `/api/developer/draft/:id/documents` authorises via `assets.developer_user_id`,
// not the Villa-Returns `developer_asset_links` model), so the developer path
// is a single combined upload-and-link endpoint guarded by `require_asset_link`.

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

/// POST /api/developer/villas/:asset_id/operations/:log_id/documents
/// Upload a receipt / invoice / statement and link it to the period in one step.
/// multipart/form-data: `file` + `doc_type`.
pub async fn api_developer_villa_operations_upload_document(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
    mut multipart: Multipart,
) -> Result<Json<PeriodDocumentRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;

    // Confirm the log row belongs to this asset and grab its period.
    let row: (Uuid, i32, i32) = sqlx::query_as(
        "SELECT asset_id, period_year, period_month FROM villa_operations_log WHERE id = $1",
    )
    .bind(log_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Operations row not found".to_string()))?;
    if row.0 != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    let (period_year, period_month) = (row.1, row.2);

    // Read multipart fields: `file` + `doc_type`.
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type = String::from("application/octet-stream");
    let mut doc_type = String::new();
    let mut file_name = String::from("document");
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("Failed to read multipart data".to_string()))?
    {
        match field.name().unwrap_or("") {
            "doc_type" => {
                doc_type = field
                    .text()
                    .await
                    .map_err(|_| ApiError::BadRequest("Invalid doc_type".to_string()))?;
            }
            "file" => {
                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }
                file_name = field.file_name().unwrap_or("document").to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| ApiError::BadRequest("Failed to read uploaded file".to_string()))?
                    .to_vec();
                if bytes.len() > MAX_PERIOD_DOC_BYTES {
                    return Err(ApiError::BadRequest("File must be <= 20 MB".to_string()));
                }
                file_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    let doc_type = doc_type.trim().to_string();
    if doc_type.is_empty() {
        return Err(ApiError::BadRequest("doc_type required".to_string()));
    }
    let file_bytes =
        file_bytes.ok_or_else(|| ApiError::BadRequest("file field required".to_string()))?;
    crate::storage::service::validate_asset_doc_mime(&mime_type)
        .map_err(|_| ApiError::BadRequest("Unsupported file type".to_string()))?;

    // Upload to GCS (private bucket); fall back to local storage on failure.
    let file_id = Uuid::new_v4();
    let object_path = format!(
        "properties/{}/documents/{}.{}",
        asset_id,
        file_id,
        crate::storage::service::extension_for_doc_mime(&mime_type)
    );
    let file_url = upload_villa_document(&state, &object_path, &file_bytes, &mime_type).await?;

    // asset_documents row + villa_period_documents link, in one transaction.
    // document_type is the generic 'financial' — the only operational type the
    // asset_documents CHECK constraint permits; the real subtype lives in
    // villa_period_documents.doc_type.
    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;
    let title = format!("{doc_type} {period_year}-{period_month:02} — {file_name}");
    let document_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO asset_documents
               (asset_id, document_type, title, file_url, file_size_bytes, is_investor_visible)
           VALUES ($1, 'financial', $2, $3, $4, FALSE)
           RETURNING id"#,
    )
    .bind(asset_id)
    .bind(&title)
    .bind(&file_url)
    .bind(file_bytes.len() as i64)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let linked: PeriodDocumentRow = sqlx::query_as(
        r#"INSERT INTO villa_period_documents
               (asset_id, period_year, period_month, log_id, document_id, doc_type, uploaded_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, asset_id, period_year, period_month, log_id, document_id, doc_type, uploaded_by, created_at"#,
    )
    .bind(asset_id)
    .bind(period_year)
    .bind(period_month)
    .bind(log_id)
    .bind(document_id)
    .bind(&doc_type)
    .bind(dev.user.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db)
            if db.constraint()
                == Some("villa_period_documents_period_year_period_month_document_id_key") =>
        {
            ApiError::Conflict("This document is already linked to this period".to_string())
        }
        _ => ApiError::Database(e),
    })?;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'villa_ops.link_document', 'villa_period_documents', NULL, $2)"#,
    )
    .bind(dev.user.id)
    .bind(serde_json::to_value(&linked).unwrap_or(serde_json::Value::Null))
    .execute(&mut *tx)
    .await;

    tx.commit().await.map_err(ApiError::Database)?;
    Ok(Json(linked))
}

// Shared GCS upload helper for both period and annual villa documents.
// Uploads to the private bucket; falls back to local storage on failure.
async fn upload_villa_document(
    state: &AppState,
    object_path: &str,
    file_bytes: &[u8],
    mime_type: &str,
) -> Result<String, ApiError> {
    if let Some(bucket) = &state.config.gcs_bucket {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            crate::storage::service::upload_private(
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
                tracing::warn!("Villa doc GCS upload failed: {e}; falling back to local")
            }
            Err(_) => tracing::warn!("Villa doc GCS upload timed out; falling back to local"),
        }
    }
    crate::storage::service::upload_local(object_path, file_bytes.to_vec())
        .await
        .map_err(|e| ApiError::Internal(format!("Document upload failed: {e}")))
}

/// GET /api/developer/villas/:asset_id/operations/:log_id/documents — list linked docs.
pub async fn api_developer_villa_operations_documents_list(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, log_id)): Path<(Uuid, i64)>,
) -> Result<Json<Vec<PeriodDocumentRow>>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    let rows: Vec<PeriodDocumentRow> = sqlx::query_as(
        r#"SELECT id, asset_id, period_year, period_month, log_id, document_id, doc_type, uploaded_by, created_at
           FROM villa_period_documents
           WHERE asset_id = $1 AND log_id = $2
           ORDER BY created_at DESC, id DESC"#,
    )
    .bind(asset_id)
    .bind(log_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

// ─── Annual documents (PDF §3 — tax statements, annual reports) ──────────
//
// Year-keyed sibling of the period-document endpoints above. Annual documents
// have no month, so they link through `villa_annual_documents` (migration 149)
// instead of `villa_period_documents`. Same combined upload-and-link shape,
// same `require_asset_link` guard.

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AnnualDocumentRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub period_year: i32,
    pub document_id: Uuid,
    pub doc_type: String,
    pub uploaded_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// POST /api/developer/villas/:asset_id/annual/:year/documents
/// Upload an annual tax statement / report and link it to the villa year.
/// multipart/form-data: `file` + `doc_type`.
pub async fn api_developer_villa_annual_documents_upload(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, year)): Path<(Uuid, i32)>,
    mut multipart: Multipart,
) -> Result<Json<AnnualDocumentRow>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::BadRequest("Invalid year".to_string()));
    }

    // Read multipart fields: `file` + `doc_type`.
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type = String::from("application/octet-stream");
    let mut doc_type = String::new();
    let mut file_name = String::from("document");
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("Failed to read multipart data".to_string()))?
    {
        match field.name().unwrap_or("") {
            "doc_type" => {
                doc_type = field
                    .text()
                    .await
                    .map_err(|_| ApiError::BadRequest("Invalid doc_type".to_string()))?;
            }
            "file" => {
                if let Some(ct) = field.content_type() {
                    mime_type = ct.to_string();
                }
                file_name = field.file_name().unwrap_or("document").to_string();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| ApiError::BadRequest("Failed to read uploaded file".to_string()))?
                    .to_vec();
                if bytes.len() > MAX_PERIOD_DOC_BYTES {
                    return Err(ApiError::BadRequest("File must be <= 20 MB".to_string()));
                }
                file_bytes = Some(bytes);
            }
            _ => {}
        }
    }

    let doc_type = doc_type.trim().to_string();
    if doc_type.is_empty() {
        return Err(ApiError::BadRequest("doc_type required".to_string()));
    }
    let file_bytes =
        file_bytes.ok_or_else(|| ApiError::BadRequest("file field required".to_string()))?;
    crate::storage::service::validate_asset_doc_mime(&mime_type)
        .map_err(|_| ApiError::BadRequest("Unsupported file type".to_string()))?;

    let file_id = Uuid::new_v4();
    let object_path = format!(
        "properties/{}/documents/{}.{}",
        asset_id,
        file_id,
        crate::storage::service::extension_for_doc_mime(&mime_type)
    );
    let file_url = upload_villa_document(&state, &object_path, &file_bytes, &mime_type).await?;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;
    let title = format!("{doc_type} {year} — {file_name}");
    let document_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO asset_documents
               (asset_id, document_type, title, file_url, file_size_bytes, is_investor_visible)
           VALUES ($1, 'financial', $2, $3, $4, FALSE)
           RETURNING id"#,
    )
    .bind(asset_id)
    .bind(&title)
    .bind(&file_url)
    .bind(file_bytes.len() as i64)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    let linked: AnnualDocumentRow = sqlx::query_as(
        r#"INSERT INTO villa_annual_documents
               (asset_id, period_year, document_id, doc_type, uploaded_by)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, asset_id, period_year, document_id, doc_type, uploaded_by, created_at"#,
    )
    .bind(asset_id)
    .bind(year)
    .bind(document_id)
    .bind(&doc_type)
    .bind(dev.user.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db)
            if db.constraint() == Some("villa_annual_documents_period_year_document_id_key") =>
        {
            ApiError::Conflict("This document is already linked to this year".to_string())
        }
        _ => ApiError::Database(e),
    })?;

    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'villa_ops.link_document', 'villa_annual_documents', NULL, $2)"#,
    )
    .bind(dev.user.id)
    .bind(serde_json::to_value(&linked).unwrap_or(serde_json::Value::Null))
    .execute(&mut *tx)
    .await;

    tx.commit().await.map_err(ApiError::Database)?;
    Ok(Json(linked))
}

/// GET /api/developer/villas/:asset_id/annual/:year/documents — list linked annual docs.
pub async fn api_developer_villa_annual_documents_list(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path((asset_id, year)): Path<(Uuid, i32)>,
) -> Result<Json<Vec<AnnualDocumentRow>>, ApiError> {
    dev.require_asset_link(&state.db, asset_id).await?;
    let rows: Vec<AnnualDocumentRow> = sqlx::query_as(
        r#"SELECT id, asset_id, period_year, document_id, doc_type, uploaded_by, created_at
           FROM villa_annual_documents
           WHERE asset_id = $1 AND period_year = $2
           ORDER BY created_at DESC, id DESC"#,
    )
    .bind(asset_id)
    .bind(year)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}
