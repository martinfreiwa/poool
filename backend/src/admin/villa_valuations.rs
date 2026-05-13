//! Admin endpoints for villa annual valuations (Villa-Returns P2.5, PDF §3).
//!
//! Valuations are Admin-only per the Field Permission Matrix. Same state
//! machine as villa_operations_log: draft → submitted → approved → published,
//! with append-only enforcement (post-publish data is immutable, corrections
//! insert a new row with supersedes_id set). 4-eyes is enforced by
//! `vv_approver_differs` CHECK + a pre-flight check that returns a clean 409.
//!
//! NAV computation per PDF §7:
//!   NAV = (valuation_idr_cents * tokenized_pct_bps / 10000)
//!         / (tokens_total - tokens_owner_retained)
//! This is exposed via `compute_nav_preview` so the entry form can show a live
//! NAV preview before publish (matches the plan's B2 "live NAV preview" panel).

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ValuationInput {
    pub valuation_date: chrono::NaiveDate,
    pub valuation_idr_cents: i64,
    pub valuation_method: String,
    pub appraiser_name: Option<String>,
    pub appraiser_user_id: Option<Uuid>,
    pub comparables: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub evidence_doc_id: Option<Uuid>,
    pub correction_reason: Option<String>,
    pub supersedes_id: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ValuationRow {
    pub id: i64,
    pub asset_id: Uuid,
    pub valuation_date: chrono::NaiveDate,
    pub valuation_idr_cents: i64,
    pub valuation_usd_cents: i64,
    pub currency_code: String,
    pub fx_rate_idr_to_usd_bps: i32,
    pub valuation_method: String,
    pub appraiser_name: Option<String>,
    pub appraiser_user_id: Option<Uuid>,
    pub comparables: Option<serde_json::Value>,
    pub notes: Option<String>,
    pub evidence_doc_id: Option<Uuid>,
    pub status: String,
    pub supersedes_id: Option<i64>,
    pub correction_reason: Option<String>,
    pub submitted_by: Option<Uuid>,
    pub approved_by: Option<Uuid>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ValuationsQuery {
    pub as_of: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RejectInput {
    pub reason: String,
}

// ─── NAV preview ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NavPreview {
    pub valuation_idr_cents: i64,
    pub tokenized_pct_bps: i32,
    pub tokens_in_pool: i64,
    pub nav_token_idr_cents: i64,
}

#[derive(Debug, Deserialize)]
pub struct NavPreviewQuery {
    pub valuation_idr_cents: i64,
}

/// GET /api/admin/villas/:asset_id/valuations/nav-preview?valuation_idr_cents=...
/// Returns the NAV that would result from a given draft valuation (PDF §7).
pub async fn api_admin_villa_nav_preview(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Query(q): Query<NavPreviewQuery>,
) -> Result<Json<NavPreview>, ApiError> {
    let cfg = load_asset_token_config(&state.db, asset_id).await?;
    Ok(Json(compute_nav_preview(q.valuation_idr_cents, &cfg)))
}

pub fn compute_nav_preview(valuation_idr_cents: i64, cfg: &TokenConfig) -> NavPreview {
    let tokens_in_pool: i64 = cfg
        .tokens_total
        .saturating_sub(cfg.tokens_owner_retained.unwrap_or(0) as i64);
    let nav: i64 = if tokens_in_pool > 0 && cfg.tokenized_pct_bps > 0 {
        let pool_value: i128 =
            (valuation_idr_cents as i128) * cfg.tokenized_pct_bps as i128 / 10_000;
        (pool_value / tokens_in_pool as i128) as i64
    } else {
        0
    };
    NavPreview {
        valuation_idr_cents,
        tokenized_pct_bps: cfg.tokenized_pct_bps,
        tokens_in_pool,
        nav_token_idr_cents: nav,
    }
}

#[derive(Debug)]
pub struct TokenConfig {
    pub tokenized_pct_bps: i32,
    pub tokens_total: i64,
    pub tokens_owner_retained: Option<i32>,
}

async fn load_asset_token_config(pool: &PgPool, asset_id: Uuid) -> Result<TokenConfig, ApiError> {
    let row: (Option<i32>, Option<i64>, Option<i32>) = sqlx::query_as(
        r#"
        SELECT
            tokenized_pct_bps,
            tokens_total::BIGINT,
            tokens_owner_retained
        FROM assets WHERE id = $1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    Ok(TokenConfig {
        tokenized_pct_bps: row.0.unwrap_or(0),
        tokens_total: row.1.unwrap_or(0),
        tokens_owner_retained: row.2,
    })
}

// ─── Handlers ─────────────────────────────────────────────────

pub async fn api_admin_villa_valuations_create(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(input): Json<ValuationInput>,
) -> Result<Json<ValuationRow>, ApiError> {
    if input.valuation_idr_cents <= 0 {
        return Err(ApiError::BadRequest("Valuation must be > 0".to_string()));
    }
    if !is_known_method(&input.valuation_method) {
        return Err(ApiError::BadRequest(format!(
            "Unknown valuation_method '{}'",
            input.valuation_method
        )));
    }

    let row: ValuationRow = sqlx::query_as(
        r#"
        INSERT INTO villa_valuations (
            asset_id, valuation_date, valuation_idr_cents,
            valuation_method, appraiser_name, appraiser_user_id,
            comparables, notes, evidence_doc_id,
            status, supersedes_id, correction_reason, submitted_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, 'draft', $10, $11, $12
        )
        RETURNING *
        "#,
    )
    .bind(asset_id)
    .bind(input.valuation_date)
    .bind(input.valuation_idr_cents)
    .bind(input.valuation_method)
    .bind(input.appraiser_name)
    .bind(input.appraiser_user_id)
    .bind(input.comparables)
    .bind(input.notes)
    .bind(input.evidence_doc_id)
    .bind(input.supersedes_id)
    .bind(input.correction_reason)
    .bind(admin.user.id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    write_audit(&state.db, admin.user.id, "create", &row, None).await;
    Ok(Json(row))
}

pub async fn api_admin_villa_valuations_update(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, val_id)): Path<(Uuid, i64)>,
    Json(input): Json<ValuationInput>,
) -> Result<Json<ValuationRow>, ApiError> {
    let existing = load_row(&state.db, val_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "draft" && existing.status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot edit valuation in status='{}'",
            existing.status
        )));
    }
    if !is_known_method(&input.valuation_method) {
        return Err(ApiError::BadRequest(format!(
            "Unknown valuation_method '{}'",
            input.valuation_method
        )));
    }

    let row: ValuationRow = sqlx::query_as(
        r#"
        UPDATE villa_valuations SET
            valuation_date       = $2,
            valuation_idr_cents  = $3,
            valuation_method     = $4,
            appraiser_name       = $5,
            appraiser_user_id    = $6,
            comparables          = $7,
            notes                = $8,
            evidence_doc_id      = $9
        WHERE id = $1 AND status IN ('draft','submitted')
        RETURNING *
        "#,
    )
    .bind(val_id)
    .bind(input.valuation_date)
    .bind(input.valuation_idr_cents)
    .bind(input.valuation_method)
    .bind(input.appraiser_name)
    .bind(input.appraiser_user_id)
    .bind(input.comparables)
    .bind(input.notes)
    .bind(input.evidence_doc_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    write_audit(&state.db, admin.user.id, "update", &row, Some(&existing)).await;
    Ok(Json(row))
}

pub async fn api_admin_villa_valuations_submit(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, val_id)): Path<(Uuid, i64)>,
) -> Result<Json<ValuationRow>, ApiError> {
    transition(&state.db, asset_id, val_id, admin.user.id, "submit").await
}

pub async fn api_admin_villa_valuations_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, val_id)): Path<(Uuid, i64)>,
) -> Result<Json<ValuationRow>, ApiError> {
    let existing = load_row(&state.db, val_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot approve valuation in status='{}'",
            existing.status
        )));
    }
    if existing.submitted_by == Some(admin.user.id) {
        return Err(ApiError::Conflict(
            "You submitted this valuation — another admin must approve".to_string(),
        ));
    }
    transition(&state.db, asset_id, val_id, admin.user.id, "approve").await
}

pub async fn api_admin_villa_valuations_publish(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, val_id)): Path<(Uuid, i64)>,
) -> Result<Json<ValuationRow>, ApiError> {
    let existing = load_row(&state.db, val_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "approved" {
        return Err(ApiError::Conflict(format!(
            "Cannot publish valuation in status='{}'",
            existing.status
        )));
    }

    // Freeze FX rate and USD derived value at publish.
    let fx_bps: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE((
            SELECT rate_bps::INTEGER
            FROM fx_rates_daily
            WHERE base_currency='IDR' AND quote_currency='USD'
            ORDER BY snapshot_date DESC LIMIT 1
        ), 1)
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(1);

    let usd: i64 = (existing.valuation_idr_cents as i128 * fx_bps as i128 / 10_000) as i64;

    // Supersede the prior published valuation for this asset (if this row has supersedes_id,
    // we'll flip the chain; otherwise we still need to retire any older publication).
    let prior_published_id: Option<i64> = if existing.supersedes_id.is_some() {
        existing.supersedes_id
    } else {
        sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT MAX(id) FROM villa_valuations
            WHERE asset_id = $1 AND status = 'published'
            "#,
        )
        .bind(asset_id)
        .fetch_one(&state.db)
        .await
        .map_err(ApiError::Database)?
    };

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;

    let row: ValuationRow = sqlx::query_as(
        r#"
        UPDATE villa_valuations SET
            status                  = 'published',
            published_at            = NOW(),
            approved_by             = COALESCE(approved_by, $2),
            fx_rate_idr_to_usd_bps  = $3,
            valuation_usd_cents     = $4
        WHERE id = $1 AND status = 'approved'
        RETURNING *
        "#,
    )
    .bind(val_id)
    .bind(admin.user.id)
    .bind(fx_bps)
    .bind(usd)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    if let Some(prior_id) = prior_published_id {
        if prior_id != row.id {
            sqlx::query(
                r#"
                UPDATE villa_valuations SET status='superseded'
                WHERE id = $1 AND status = 'published'
                "#,
            )
            .bind(prior_id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Database)?;
        }
    }

    tx.commit().await.map_err(ApiError::Database)?;

    write_audit(&state.db, admin.user.id, "publish", &row, Some(&existing)).await;
    Ok(Json(row))
}

pub async fn api_admin_villa_valuations_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    Path((asset_id, val_id)): Path<(Uuid, i64)>,
    Json(input): Json<RejectInput>,
) -> Result<Json<ValuationRow>, ApiError> {
    if input.reason.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Rejection reason required".to_string(),
        ));
    }
    let existing = load_row(&state.db, val_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    if existing.status != "submitted" {
        return Err(ApiError::Conflict(format!(
            "Cannot reject valuation in status='{}'",
            existing.status
        )));
    }

    let row: ValuationRow = sqlx::query_as(
        r#"
        UPDATE villa_valuations SET
            status          = 'rejected',
            rejected_reason = $2,
            rejected_at     = NOW()
        WHERE id = $1 AND status = 'submitted'
        RETURNING *
        "#,
    )
    .bind(val_id)
    .bind(input.reason.clone())
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    write_audit(&state.db, admin.user.id, "reject", &row, Some(&existing)).await;
    Ok(Json(row))
}

pub async fn api_admin_villa_valuations_list(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Query(q): Query<ValuationsQuery>,
) -> Result<Json<Vec<ValuationRow>>, ApiError> {
    let rows: Vec<ValuationRow> = sqlx::query_as(
        r#"
        SELECT * FROM villa_valuations
        WHERE asset_id = $1
          AND ($2::TIMESTAMPTZ IS NULL OR recorded_at <= $2)
        ORDER BY valuation_date DESC, recorded_at DESC, id DESC
        "#,
    )
    .bind(asset_id)
    .bind(q.as_of)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;
    Ok(Json(rows))
}

// ─── Internal helpers ─────────────────────────────────────────

fn is_known_method(m: &str) -> bool {
    matches!(
        m,
        "sales_comparison" | "income" | "cost" | "external_appraisal" | "other"
    )
}

async fn load_row(pool: &PgPool, val_id: i64) -> Result<ValuationRow, ApiError> {
    sqlx::query_as::<_, ValuationRow>("SELECT * FROM villa_valuations WHERE id = $1")
        .bind(val_id)
        .fetch_optional(pool)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Valuation not found".to_string()))
}

async fn transition(
    pool: &PgPool,
    asset_id: Uuid,
    val_id: i64,
    actor: Uuid,
    action: &str,
) -> Result<Json<ValuationRow>, ApiError> {
    let existing = load_row(pool, val_id).await?;
    if existing.asset_id != asset_id {
        return Err(ApiError::BadRequest("asset_id mismatch".to_string()));
    }
    let (new_status, ts_col, actor_col) = match (existing.status.as_str(), action) {
        ("draft", "submit") => ("submitted", "submitted_at", "submitted_by"),
        ("submitted", "approve") => ("approved", "approved_at", "approved_by"),
        _ => {
            return Err(ApiError::Conflict(format!(
                "Cannot apply action='{}' to row in status='{}'",
                action, existing.status
            )))
        }
    };

    let sql = format!(
        "UPDATE villa_valuations
            SET status = $2, {ts_col} = NOW(), {actor_col} = $3
          WHERE id = $1 AND status = $4
          RETURNING *"
    );
    let row: ValuationRow = sqlx::query_as(&sql)
        .bind(val_id)
        .bind(new_status)
        .bind(actor)
        .bind(&existing.status)
        .fetch_one(pool)
        .await
        .map_err(ApiError::Database)?;

    write_audit(pool, actor, action, &row, Some(&existing)).await;
    Ok(Json(row))
}

async fn write_audit(
    pool: &PgPool,
    actor: Uuid,
    action: &str,
    new_state: &ValuationRow,
    previous_state: Option<&ValuationRow>,
) {
    let new_json = serde_json::to_value(new_state).unwrap_or(serde_json::Value::Null);
    let prev_json = previous_state
        .map(|p| serde_json::to_value(p).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);

    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
        VALUES ($1, $2, 'villa_valuations', NULL, $3, $4)
        "#,
    )
    .bind(actor)
    .bind(format!("villa_val.{}", action))
    .bind(prev_json)
    .bind(new_json)
    .execute(pool)
    .await;
}
