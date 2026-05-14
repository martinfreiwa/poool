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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Period documents (receipts / invoices / statements) cap at 20 MB,
/// matching the admin asset-document limit.
const MAX_PERIOD_DOC_BYTES: usize = 20 * 1024 * 1024;

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
    let file_url = upload_period_document(&state, &object_path, &file_bytes, &mime_type).await?;

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

async fn upload_period_document(
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
                tracing::warn!("Period doc GCS upload failed: {e}; falling back to local")
            }
            Err(_) => tracing::warn!("Period doc GCS upload timed out; falling back to local"),
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
