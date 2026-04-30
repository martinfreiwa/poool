use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

//
//  Admin Treasury API
//

/// GET /api/admin/treasury  Aggregated financial overview
pub async fn api_admin_treasury(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // 1. Wallet aggregates
    let wallet_row = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(balance_cents), 0)::bigint, COUNT(*)::bigint FROM wallets",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0));

    // 2. Transaction type breakdown
    let type_rows = sqlx::query_as::<_, (String, i64, i64)>(
        r#"SELECT type, COALESCE(SUM(ABS(amount_cents)), 0)::bigint, COUNT(*)::bigint
           FROM wallet_transactions WHERE status = 'completed'
           GROUP BY type ORDER BY SUM(ABS(amount_cents)) DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let type_breakdown: Vec<serde_json::Value> = type_rows
        .iter()
        .map(|r| serde_json::json!({ "type": r.0, "total_cents": r.1, "count": r.2 }))
        .collect();

    // Compute deposit / withdrawal totals
    let dep_total = type_rows
        .iter()
        .find(|r| r.0 == "deposit")
        .map(|r| r.1)
        .unwrap_or(0);
    let dep_count = type_rows
        .iter()
        .find(|r| r.0 == "deposit")
        .map(|r| r.2)
        .unwrap_or(0);
    let wd_total = type_rows
        .iter()
        .find(|r| r.0 == "withdrawal")
        .map(|r| r.1)
        .unwrap_or(0);
    let wd_count = type_rows
        .iter()
        .find(|r| r.0 == "withdrawal")
        .map(|r| r.2)
        .unwrap_or(0);
    let _purchase_total = type_rows
        .iter()
        .find(|r| r.0 == "purchase")
        .map(|r| r.1)
        .unwrap_or(0);
    let fee_total = type_rows
        .iter()
        .find(|r| r.0 == "fee")
        .map(|r| r.1)
        .unwrap_or(0);

    // 3. Dividend stats
    let div_paid = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint, COUNT(*)::bigint FROM dividend_payouts WHERE status = 'paid'"
    ).fetch_one(&state.db).await.unwrap_or((0, 0));

    let div_scheduled = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint, COUNT(*)::bigint FROM dividend_payouts WHERE status = 'scheduled'"
    ).fetch_one(&state.db).await.unwrap_or((0, 0));

    let div_processing = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint, COUNT(*)::bigint FROM dividend_payouts WHERE status = 'processing'"
    ).fetch_one(&state.db).await.unwrap_or((0, 0));

    let div_failed = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint, COUNT(*)::bigint FROM dividend_payouts WHERE status = 'failed'"
    ).fetch_one(&state.db).await.unwrap_or((0, 0));

    // 4. Recent transactions (last 100)
    let tx_rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            i64,
            Option<String>,
            String,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"SELECT wt.id::text, wt.type, wt.status, wt.amount_cents, wt.description,
                  wt.created_at::text, COALESCE(u.email, ''),
                  COALESCE(up.first_name, ''), COALESCE(up.last_name, '')
           FROM wallet_transactions wt
           JOIN wallets w ON w.id = wt.wallet_id
           JOIN users u ON u.id = w.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           ORDER BY wt.created_at DESC
           LIMIT 500"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let recent: Vec<serde_json::Value> = tx_rows
        .iter()
        .map(|r| {
            let name = format!(
                "{} {}",
                r.7.clone().unwrap_or_default(),
                r.8.clone().unwrap_or_default()
            )
            .trim()
            .to_string();
            serde_json::json!({
                "id": r.0, "type": r.1, "status": r.2, "amount_cents": r.3,
                "description": r.4, "created_at": r.5,
                "user_email": r.6,
                "user_name": if name.is_empty() { r.6.clone() } else { name }
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "stats": {
            "total_balance_cents": wallet_row.0,
            "wallet_count": wallet_row.1,
            "total_deposits_cents": dep_total,
            "deposit_count": dep_count,
            "total_withdrawals_cents": wd_total,
            "withdrawal_count": wd_count,
            "net_revenue_cents": fee_total // Revenue is strictly from fees
        },
        "type_breakdown": type_breakdown,
        "dividend_stats": {
            "total_paid_cents": div_paid.0, "paid_count": div_paid.1,
            "scheduled_cents": div_scheduled.0, "scheduled_count": div_scheduled.1,
            "processing_cents": div_processing.0, "processing_count": div_processing.1,
            "failed_cents": div_failed.0, "failed_count": div_failed.1
        },
        "recent_transactions": recent
    }))
    .into_response())
}

/// POST /api/admin/dividends/calculate
pub async fn api_admin_dividends_calculate(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.draft")
        .await?;

    let asset_id = body.get("asset_id").and_then(|v| v.as_str()).unwrap_or("");
    let total_amount_cents = body
        .get("total_amount_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    if asset_id.is_empty() || total_amount_cents <= 0 {
        return Err(ApiError::BadRequest("Invalid parameters".to_string()));
    }

    let aid = ApiError::parse_uuid(asset_id)?;

    // Get total tokens owned for this asset
    let total_tokens_owned_res: Result<Option<i32>, _> = sqlx::query_scalar(
        "SELECT SUM(tokens_owned)::int4 FROM investments WHERE asset_id = $1 AND status = 'active'",
    )
    .bind(aid)
    .fetch_one(&state.db)
    .await;

    let total_tokens_owned = total_tokens_owned_res.map_err(ApiError::from)?.unwrap_or(0);

    if total_tokens_owned == 0 {
        return Ok(Json(serde_json::json!({"splits":[], "total_tokens":0})).into_response());
    }

    // Get all investors
    let rows: Vec<(String, String, i32)> = sqlx::query_as(
        "SELECT u.email, u.id::text, i.tokens_owned FROM investments i JOIN users u ON u.id = i.user_id WHERE i.asset_id = $1 AND i.status = 'active' AND i.tokens_owned > 0"
    )
    .bind(aid)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let mut cumulative_allocated: i64 = 0;
    let mut cumulative_exact: u128 = 0;

    let splits: Vec<serde_json::Value> = rows
        .iter()
        .map(|(email, user_id, tokens)| {
            cumulative_exact += total_amount_cents as u128 * (*tokens as u128);
            let current_target = ((cumulative_exact + (total_tokens_owned as u128 / 2))
                / total_tokens_owned as u128) as i64;
            let amount = current_target - cumulative_allocated;
            cumulative_allocated = current_target;

            serde_json::json!({
                "email": email, "user_id": user_id, "tokens": tokens, "amount_cents": amount
            })
        })
        .collect();

    Ok(
        Json(serde_json::json!({"splits": splits, "total_tokens": total_tokens_owned}))
            .into_response(),
    )
}

/// POST /api/admin/dividends/process
pub async fn api_admin_dividends_process(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.draft")
        .await?;

    let asset_id = body.get("asset_id").and_then(|v| v.as_str()).unwrap_or("");
    let total_amount_cents = body
        .get("total_amount_cents")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    if asset_id.is_empty() || total_amount_cents <= 0 {
        return Err(ApiError::BadRequest("Invalid parameters".to_string()));
    }

    let aid = ApiError::parse_uuid(asset_id)?;

    let user = admin.user.clone();

    let payload = serde_json::json!({
        "total_amount_cents": total_amount_cents,
        "asset_id": asset_id
    });

    let result = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO admin_approval_requests (requester_id, action_type, entity_type, entity_id, payload) VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
        .bind(user.id)
        .bind("dividend.process")
        .bind("asset")
        .bind(aid)
        .bind(&payload)
        .fetch_one(&state.db)
        .await;

    match result {
        Ok(id) => {
            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
                .bind(user.id)
                .bind("approval_request.created")
                .bind("admin_approval_requests")
                .bind(id)
                .bind(serde_json::json!({"action_type": "dividend.process", "entity_id": aid}))
                .execute(&state.db).await;

            Ok(Json(serde_json::json!({
                "status": "success",
                "message": "Dividend distribution queued for approval.",
                "payout_id": id.to_string()
            }))
            .into_response())
        }
        Err(e) => {
            tracing::error!("Failed to create approval request: {}", e);
            Err(ApiError::Internal(
                "Failed to queue dividend process".to_string(),
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// ── NEW: Dividend Distribution System (Phase 9) ───────────────
// ═══════════════════════════════════════════════════════════════

/// GET /api/admin/dividends/distributions
/// List all dividend distributions, optionally filtered by asset_id.
pub async fn api_admin_dividends_list(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.draft")
        .await?;

    let asset_id = params
        .get("asset_id")
        .and_then(|v| uuid::Uuid::parse_str(v).ok());

    match crate::dividends::service::list_distributions(&state.db, asset_id).await {
        Ok(distributions) => {
            Ok(Json(serde_json::json!({ "distributions": distributions })).into_response())
        }
        Err(e) => Err(ApiError::Internal(e)),
    }
}

/// POST /api/admin/dividends/distributions
/// Create and calculate a new dividend distribution with anti-sniping.
///
/// Body: { asset_id, period_start, period_end, total_amount_cents, min_holding_days? }
pub async fn api_admin_dividends_create_distribution(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.draft")
        .await?;

    let asset_id = body
        .get("asset_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("asset_id required".to_string()))?;

    let total_amount_cents = body
        .get("total_amount_cents")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            ApiError::BadRequest("total_amount_cents required (positive integer)".to_string())
        })?;

    let period_start = body
        .get("period_start")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .ok_or_else(|| ApiError::BadRequest("period_start required (YYYY-MM-DD)".to_string()))?;

    let period_end = body
        .get("period_end")
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .ok_or_else(|| ApiError::BadRequest("period_end required (YYYY-MM-DD)".to_string()))?;

    let min_holding_days = body
        .get("min_holding_days")
        .and_then(|v| v.as_i64())
        .unwrap_or(7) as i32;

    if !(0..=365).contains(&min_holding_days) {
        return Err(ApiError::BadRequest(
            "min_holding_days must be between 0 and 365".to_string(),
        ));
    }

    let aid = ApiError::parse_uuid(asset_id)?;

    match crate::dividends::service::calculate_dividends(
        &state.db,
        aid,
        period_start,
        period_end,
        total_amount_cents,
        min_holding_days,
        admin.user.id,
    )
    .await
    {
        Ok(result) => Ok(Json(serde_json::json!({
            "status": "calculated",
            "result": result
        }))
        .into_response()),
        Err(e) => Err(ApiError::BadRequest(e)),
    }
}

/// GET /api/admin/dividends/distributions/:id
/// Get distribution details with all payouts.
pub async fn api_admin_dividends_distribution_detail(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(dist_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.draft")
        .await?;

    let did = ApiError::parse_uuid(&dist_id)?;

    match crate::dividends::service::get_distribution_detail(&state.db, did).await {
        Ok(detail) => Ok(Json(detail).into_response()),
        Err(e) => Err(ApiError::NotFound(e)),
    }
}

/// POST /api/admin/dividends/distributions/:id/approve
/// Approve a calculated distribution.
pub async fn api_admin_dividends_approve_distribution(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(dist_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.approve")
        .await?;

    let did = ApiError::parse_uuid(&dist_id)?;

    match crate::dividends::service::approve_distribution(&state.db, did, admin.user.id).await {
        Ok(()) => Ok(Json(serde_json::json!({"status": "approved"})).into_response()),
        Err(e) => Err(ApiError::BadRequest(e)),
    }
}

/// POST /api/admin/dividends/distributions/:id/execute
/// Execute an approved distribution — credit all eligible wallets.
/// 🔴 CRITICAL FINANCIAL OPERATION — This credits real money to user wallets.
pub async fn api_admin_dividends_execute_distribution(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(dist_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.approve")
        .await?;

    let did = ApiError::parse_uuid(&dist_id)?;

    match crate::dividends::service::execute_distribution(&state.db, did, admin.user.id).await {
        Ok(summary) => Ok(Json(serde_json::json!({
            "status": "distributed",
            "summary": summary
        }))
        .into_response()),
        Err(e) => {
            tracing::error!(
                "[P0-FINANCIAL] Dividend execution failed for {}: {}",
                dist_id,
                e
            );
            if e.starts_with("DB ")
                || e.starts_with("TX ")
                || e.starts_with("Wallet credit failed")
                || e.starts_with("[P0-FINANCIAL]")
            {
                Err(ApiError::Internal(format!("Distribution failed: {}", e)))
            } else {
                Err(ApiError::BadRequest(format!("Distribution failed: {}", e)))
            }
        }
    }
}

/// POST /api/admin/dividends/distributions/:id/cancel
/// Cancel a distribution (only if not yet distributed).
pub async fn api_admin_dividends_cancel_distribution(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(dist_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "financials.payout.draft")
        .await?;

    let did = ApiError::parse_uuid(&dist_id)?;
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("Cancelled by admin");

    match crate::dividends::service::cancel_distribution(&state.db, did, reason, admin.user.id)
        .await
    {
        Ok(()) => Ok(Json(serde_json::json!({"status": "cancelled"})).into_response()),
        Err(e) => Err(ApiError::BadRequest(e)),
    }
}
