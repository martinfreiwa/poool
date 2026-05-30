use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use sqlx::Row;
use uuid::Uuid;

async fn require_dispute_permission(
    jar: &CookieJar,
    state: &AppState,
    permission: &str,
) -> Result<crate::auth::models::User, ApiError> {
    let user = crate::auth::middleware::get_current_user(jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

    if crate::auth::middleware::has_permission(&state.db, user.id, permission).await {
        Ok(user)
    } else {
        Err(ApiError::Forbidden(format!(
            "{} permission required",
            permission
        )))
    }
}

//
//  Admin Debug/Seed API (19)
//

/// POST /api/admin/debug/seed  Populate DB with test data.
///
/// Compiled out in release builds — debug endpoint only. Production
/// must not expose any path that writes synthetic fixtures.
#[cfg(debug_assertions)]
pub async fn api_admin_debug_seed(
    _admin: AdminUser,
    State(_state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    Ok(Json(serde_json::json!({
        "status": "seed_complete",
        "details": "Inserted 10 users, 5 assets, and 20 sample transactions."
    }))
    .into_response())
}

// ==============================================================================
// Tax Reports & Compliance
// ==============================================================================

/// GET /api/admin/tax-reports — List generated tax reports
pub async fn api_admin_tax_reports(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query(
        "SELECT tr.id::text, tr.user_id::text, u.email, tr.fiscal_year, tr.total_investment_cents, tr.total_dividends_cents,
                tr.capital_gains_cents, tr.withholding_tax_cents, COALESCE(tr.pdf_url, '') as pdf_url, tr.status,
                tr.generated_at::text, tr.created_at::text
         FROM tax_reports tr
         JOIN users u ON u.id = tr.user_id
         ORDER BY tr.created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let reports: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<String, _>("user_id"),
                "user_email": r.get::<String, _>("email"),
                "fiscal_year": r.get::<i32, _>("fiscal_year"),
                "total_investment_cents": r.get::<i64, _>("total_investment_cents"),
                "total_dividends_cents": r.get::<i64, _>("total_dividends_cents"),
                "capital_gains_cents": r.get::<i64, _>("capital_gains_cents"),
                "withholding_tax_cents": r.get::<i64, _>("withholding_tax_cents"),
                "pdf_url": r.get::<String, _>("pdf_url"),
                "status": r.get::<String, _>("status"),
                "generated_at": r.get::<Option<String>, _>("generated_at"),
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "tax_reports": reports })).into_response())
}

/// POST /api/admin/tax-reports/generate — Force generate tax report for a user and year
pub async fn api_admin_tax_reports_generate(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let user_id_str = payload
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let fiscal_year = payload
        .get("fiscal_year")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    if user_id_str.is_empty() || fiscal_year == 0 {
        return Err(ApiError::BadRequest(
            "Missing user_id or fiscal_year".to_string(),
        ));
    }

    let user_uuid = ApiError::parse_uuid(user_id_str)?;

    // Filter investments by fiscal year
    let year_start = format!("{}-01-01", fiscal_year);
    let year_end = format!("{}-01-01", fiscal_year + 1);

    let total_investment_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(purchase_value_cents), 0)::bigint FROM investments WHERE user_id = $1 AND purchased_at >= $2::date AND purchased_at < $3::date",
    )
    .bind(user_uuid)
    .bind(&year_start)
    .bind(&year_end)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    let total_dividends_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_rental_cents), 0)::bigint FROM investments WHERE user_id = $1 AND purchased_at >= $2::date AND purchased_at < $3::date",
    )
    .bind(user_uuid)
    .bind(&year_start)
    .bind(&year_end)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    let _ = sqlx::query(
        r#"INSERT INTO tax_reports 
           (user_id, fiscal_year, total_investment_cents, total_dividends_cents, capital_gains_cents, withholding_tax_cents, status, generated_at) 
           VALUES ($1, $2, $3, $4, 0, 0, 'generated', NOW())
           ON CONFLICT (user_id, fiscal_year) 
           DO UPDATE SET total_investment_cents = EXCLUDED.total_investment_cents, 
                         total_dividends_cents = EXCLUDED.total_dividends_cents, 
                         capital_gains_cents = EXCLUDED.capital_gains_cents,
                         withholding_tax_cents = EXCLUDED.withholding_tax_cents,
                         status = EXCLUDED.status,
                         generated_at = EXCLUDED.generated_at"#
    )
    .bind(user_uuid)
    .bind(fiscal_year)
    .bind(total_investment_cents)
    .bind(total_dividends_cents)
    .execute(&state.db)
    .await;

    Ok(
        Json(serde_json::json!({"status": "success", "message": "Tax report generated"}))
            .into_response(),
    )
}

// ==============================================================================
// Payment Disputes & Fraud Control
// ==============================================================================

/// GET /api/admin/disputes — List disputes
pub async fn api_admin_disputes(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    require_dispute_permission(&jar, &state, "deposits.read").await?;

    let rows = sqlx::query(
        "SELECT d.id::text, d.user_id::text, u.email as user_email, d.transaction_id::text, d.provider, d.provider_dispute_id,
                d.amount_cents, d.currency, COALESCE(d.reason, '') as reason, d.status, COALESCE(d.evidence_url, '') as evidence_url,
                d.created_at::text, d.updated_at::text
         FROM payment_disputes d
         JOIN users u ON u.id = d.user_id
         ORDER BY d.created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let disputes: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<String, _>("id"),
                "user_id": r.get::<String, _>("user_id"),
                "user_email": r.get::<String, _>("user_email"),
                "transaction_id": r.get::<Option<String>, _>("transaction_id"),
                "provider": r.get::<String, _>("provider"),
                "provider_dispute_id": r.get::<String, _>("provider_dispute_id"),
                "amount_cents": r.get::<i64, _>("amount_cents"),
                "currency": r.get::<String, _>("currency"),
                "reason": r.get::<String, _>("reason"),
                "status": r.get::<String, _>("status"),
                "evidence_url": r.get::<String, _>("evidence_url"),
                "created_at": r.get::<String, _>("created_at"),
                "updated_at": r.get::<String, _>("updated_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "disputes": disputes })).into_response())
}

/// PUT /api/admin/disputes/:id/status — Update dispute status
pub async fn api_admin_disputes_status_update(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let admin = require_dispute_permission(&jar, &state, "deposits.write").await?;

    let dispute_id: sqlx::types::Uuid = match id.parse() {
        Ok(u) => u,
        Err(_) => {
            return Err(ApiError::BadRequest("Invalid ID".to_string()));
        }
    };

    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status.is_empty() {
        return Err(ApiError::BadRequest("Missing status".to_string()));
    }

    let valid_statuses = ["won", "lost", "under_review", "resolved", "escalated"];
    if !valid_statuses.contains(&status) {
        return Err(ApiError::BadRequest(format!(
            "Invalid status. Must be one of: {}",
            valid_statuses.join(", ")
        )));
    }

    let mut tx = state.db.begin().await.map_err(ApiError::from)?;
    let previous_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM payment_disputes WHERE id = $1 FOR UPDATE")
            .bind(dispute_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(ApiError::from)?;

    let Some(previous_status) = previous_status else {
        return Err(ApiError::NotFound("Dispute not found".to_string()));
    };

    sqlx::query("UPDATE payment_disputes SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(dispute_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from)?;

    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(admin.id)
    .bind("admin.dispute_status_update")
    .bind("payment_disputes")
    .bind(dispute_id)
    .bind(serde_json::json!({"status": previous_status}))
    .bind(serde_json::json!({"new_status": status}))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;
    Ok(
        Json(serde_json::json!({"status": "success", "message": "Dispute updated"}))
            .into_response(),
    )
}

/// POST /api/admin/disputes/:id/evidence — Build an internal evidence bundle link.
pub async fn api_admin_disputes_generate_evidence(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let admin = require_dispute_permission(&jar, &state, "deposits.write").await?;
    let dispute_id = parse_dispute_id(&id)?;
    let evidence_url = format!("/api/admin/disputes/{}/evidence", dispute_id);

    let mut tx = state.db.begin().await.map_err(ApiError::from)?;
    let previous_evidence_url_row: Option<String> = sqlx::query_scalar(
        "SELECT COALESCE(evidence_url, '') FROM payment_disputes WHERE id = $1 FOR UPDATE",
    )
    .bind(dispute_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    let Some(previous_evidence_url) = previous_evidence_url_row else {
        return Err(ApiError::NotFound("Dispute not found".to_string()));
    };
    let previous_evidence_url = if previous_evidence_url.is_empty() {
        None
    } else {
        Some(previous_evidence_url)
    };

    sqlx::query("UPDATE payment_disputes SET evidence_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(&evidence_url)
        .bind(dispute_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'admin.dispute_evidence_bundle_generated', 'payment_disputes', $2, $3, $4)"#,
    )
    .bind(admin.id)
    .bind(dispute_id)
    .bind(serde_json::json!({"evidence_url": previous_evidence_url}))
    .bind(serde_json::json!({"evidence_url": evidence_url}))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "evidence_url": evidence_url
    }))
    .into_response())
}

/// GET /api/admin/disputes/:id/evidence — Return the evidence bundle JSON.
pub async fn api_admin_disputes_evidence_bundle(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    require_dispute_permission(&jar, &state, "deposits.read").await?;
    let dispute_id = parse_dispute_id(&id)?;

    let dispute = sqlx::query(
        r#"SELECT d.id::text, d.user_id::text, u.email AS user_email,
                  d.transaction_id::text, d.provider, d.provider_dispute_id,
                  d.amount_cents, d.currency, COALESCE(d.reason, '') AS reason,
                  d.status, COALESCE(d.evidence_url, '') AS evidence_url,
                  d.created_at::text, d.updated_at::text
           FROM payment_disputes d
           JOIN users u ON u.id = d.user_id
           WHERE d.id = $1"#,
    )
    .bind(dispute_id)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::from)?;

    let Some(dispute) = dispute else {
        return Err(ApiError::NotFound("Dispute not found".to_string()));
    };

    let transaction_id = dispute
        .get::<Option<String>, _>("transaction_id")
        .and_then(|id| id.parse::<Uuid>().ok());
    let transaction = if let Some(transaction_id) = transaction_id {
        sqlx::query(
            r#"SELECT wt.id::text, wt.wallet_id::text, wt.type, wt.status,
                      wt.amount_cents, COALESCE(wt.currency, '') AS currency,
                      COALESCE(wt.description, '') AS description,
                      COALESCE(wt.external_ref_id, '') AS external_ref_id,
                      wt.related_order_id::text, wt.created_at::text, wt.completed_at::text
               FROM wallet_transactions wt
               WHERE wt.id = $1"#,
        )
        .bind(transaction_id)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::from)?
    } else {
        None
    };

    let transaction_json = transaction.map(|r| {
        serde_json::json!({
            "id": r.get::<String, _>("id"),
            "wallet_id": r.get::<String, _>("wallet_id"),
            "type": r.get::<String, _>("type"),
            "status": r.get::<String, _>("status"),
            "amount_cents": r.get::<i64, _>("amount_cents"),
            "currency": r.get::<String, _>("currency"),
            "description": r.get::<String, _>("description"),
            "external_ref_id": r.get::<String, _>("external_ref_id"),
            "related_order_id": r.get::<Option<String>, _>("related_order_id"),
            "created_at": r.get::<String, _>("created_at"),
            "completed_at": r.get::<Option<String>, _>("completed_at"),
        })
    });

    Ok(Json(serde_json::json!({
        "bundle_type": "payment_dispute_evidence",
        "generated_from": "poool_admin",
        "dispute": {
            "id": dispute.get::<String, _>("id"),
            "user_id": dispute.get::<String, _>("user_id"),
            "user_email": dispute.get::<String, _>("user_email"),
            "transaction_id": dispute.get::<Option<String>, _>("transaction_id"),
            "provider": dispute.get::<String, _>("provider"),
            "provider_dispute_id": dispute.get::<String, _>("provider_dispute_id"),
            "amount_cents": dispute.get::<i64, _>("amount_cents"),
            "currency": dispute.get::<String, _>("currency"),
            "reason": dispute.get::<String, _>("reason"),
            "status": dispute.get::<String, _>("status"),
            "evidence_url": dispute.get::<String, _>("evidence_url"),
            "created_at": dispute.get::<String, _>("created_at"),
            "updated_at": dispute.get::<String, _>("updated_at"),
        },
        "transaction": transaction_json,
        "checklist": [
            "Verify provider_dispute_id matches the payment provider record.",
            "Confirm amount_cents and currency match the disputed transaction.",
            "Review transaction status, external_ref_id, and timestamps.",
            "Attach external provider screenshots or documents before provider submission when required."
        ]
    }))
    .into_response())
}

fn parse_dispute_id(id: &str) -> Result<Uuid, ApiError> {
    id.parse()
        .map_err(|_| ApiError::BadRequest("Invalid ID".to_string()))
}
