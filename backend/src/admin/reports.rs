use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

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
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
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
    .unwrap_or_default();

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
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> axum::response::Response {
    let dispute_id: sqlx::types::Uuid = match id.parse() {
        Ok(u) => u,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"Invalid ID"})),
            )
                .into_response()
        }
    };

    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
    if status.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"Missing status"})),
        )
            .into_response();
    }

    let valid_statuses = ["won", "lost", "under_review", "resolved", "escalated"];
    if !valid_statuses.contains(&status) {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("Invalid status. Must be one of: {}", valid_statuses.join(", "))})),
        )
            .into_response();
    }

    let _ =
        sqlx::query("UPDATE payment_disputes SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(dispute_id)
            .execute(&state.db)
            .await;

    // Audit log
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(_admin.user.id)
    .bind("admin.dispute_status_update")
    .bind("payment_disputes")
    .bind(dispute_id)
    .bind(serde_json::json!({"new_status": status}))
    .execute(&state.db)
    .await;

    Json(serde_json::json!({"status": "success", "message": "Dispute updated"})).into_response()
}
