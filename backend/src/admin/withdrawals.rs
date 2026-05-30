use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

use super::extractors::AdminUser;
use crate::{auth::routes::AppState, error::AppError};

/// API view model for a withdrawal request in the admin interface.
#[derive(Serialize)]
pub struct WithdrawalRequestView {
    /// Unique withdrawal request ID.
    pub id: Uuid,
    /// ID of the user who made the request.
    pub user_id: Uuid,
    /// Email of the user.
    pub user_email: String,
    /// Amount requested in cents.
    pub amount_cents: i64,
    /// Currency code (e.g. "USD").
    pub currency: String,
    /// Lifecycle status: pending, approved, rejected, cancelled.
    pub status: String,
    /// Optional payment method the user wants funds sent to.
    pub payment_method_id: Option<Uuid>,
    /// Optional notes left by admin when rejecting.
    pub admin_notes: Option<String>,
    /// When the request was created.
    pub created_at: chrono::DateTime<Utc>,
}

/// Payload for rejecting a withdrawal request.
#[derive(Deserialize)]
pub struct RejectWithdrawalPayload {
    /// Admin-provided rejection reason shown in audit log.
    pub reason: String,
}

/// Permission gate shared by every withdrawal admin endpoint. Returns
/// `AppError::Forbidden` when the admin role doesn't carry the required
/// permission so the bulk + per-row paths fail with the same shape.
async fn require_withdraw_permission(
    admin: &AdminUser,
    pool: &sqlx::PgPool,
    permission: &str,
) -> Result<(), AppError> {
    if crate::auth::middleware::has_permission(pool, admin.user.id, permission).await {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "Missing permission: {}",
            permission
        )))
    }
}

/// GET /api/admin/withdrawals
///
/// Bounded to 500 rows per page (`?page=N`, zero-indexed) to prevent OOM at
/// scale. See CDDRP §3.5 (B6) for the broader unbounded-query remediation.
pub async fn api_admin_withdrawals(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<WithdrawalRequestView>>, AppError> {
    require_withdraw_permission(&admin, &state.db, "withdrawals.read").await?;

    // Pagination cap (CDDRP B6 fix).
    const WITHDRAWALS_PAGE_SIZE: i64 = 500;
    let page = params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);
    let offset = page.saturating_mul(WITHDRAWALS_PAGE_SIZE);

    let rows = sqlx::query(
        r#"
        SELECT wr.id, wr.user_id, u.email as user_email, wr.amount_cents, wr.currency, wr.status,
               wr.payment_method_id, wr.admin_notes, wr.created_at
        FROM withdrawal_requests wr
        JOIN users u ON wr.user_id = u.id
        ORDER BY wr.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(WITHDRAWALS_PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let requests = rows
        .into_iter()
        .map(|row| WithdrawalRequestView {
            id: row.get("id"),
            user_id: row.get("user_id"),
            user_email: row.get("user_email"),
            amount_cents: row.get("amount_cents"),
            currency: row.get("currency"),
            status: row.get("status"),
            payment_method_id: row.try_get("payment_method_id").unwrap_or(None),
            admin_notes: row.try_get("admin_notes").unwrap_or(None),
            created_at: row.get("created_at"),
        })
        .collect();

    Ok(Json(requests))
}

/// POST /api/admin/withdrawals/:req_id/approve
/// Atomically: verify balance → deduct → mark approved → update ledger tx
pub async fn api_admin_withdrawal_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_withdraw_permission(&admin, &state.db, "withdrawals.write").await?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("TX begin failed: {}", e)))?;

    // 1. Fetch withdrawal with row lock
    let req: Option<(Uuid, i64, String, String)> = sqlx::query_as(
        "SELECT user_id, amount_cents, currency, status FROM withdrawal_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(req_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Withdrawal fetch failed: {}", e)))?;

    let (user_id, amount_cents, _currency, status) = match req {
        Some(r) => r,
        None => return Err(AppError::NotFound("Withdrawal request not found".into())),
    };

    if status != "pending" {
        return Err(AppError::BadRequest(format!(
            "Cannot approve: request is '{}'",
            status
        )));
    }

    // Funds are already frozen/deducted during the withdrawal request.
    // We just need to mark as approved.

    // 2. Mark withdrawal request as approved
    sqlx::query(
        "UPDATE withdrawal_requests SET status = 'approved', approved_at = NOW() WHERE id = $1",
    )
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Withdrawal status update failed: {}", e)))?;

    // 5. Mark the pending ledger transaction as completed
    sqlx::query(
        "UPDATE wallet_transactions SET status = 'completed' WHERE external_ref_id = $1 AND type = 'withdrawal'",
    )
    .bind(req_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Ledger tx update failed: {}", e)))?;

    // 6. Audit log
    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id) VALUES ($1, 'withdrawal.approved', 'withdrawal_request', $2)",
    )
    .bind(admin.user.id)
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Audit log failed: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("TX commit failed: {}", e)))?;

    tracing::info!(
        "Withdrawal {} approved for user {} (amount {} cents)",
        req_id,
        user_id,
        amount_cents
    );

    // Best-effort confirmation email
    spawn_withdraw_email(
        state.db.clone(),
        user_id,
        req_id,
        amount_cents,
        "withdraw_approved",
        None,
    );

    Ok(Json(serde_json::json!({ "status": "success" })))
}

/// Send a withdrawal status email out-of-band. Looks up the destination label
/// from `payment_methods` so the email reads "Sent to Wise USD" rather than
/// "your bank account". Failures are logged but never block the admin action.
fn spawn_withdraw_email(
    db: sqlx::PgPool,
    user_id: Uuid,
    req_id: Uuid,
    amount_cents: i64,
    event_type: &'static str,
    reason: Option<String>,
) {
    tokio::spawn(async move {
        let pm_id: Option<Uuid> =
            sqlx::query_scalar("SELECT payment_method_id FROM withdrawal_requests WHERE id = $1")
                .bind(req_id)
                .fetch_optional(&db)
                .await
                .ok()
                .flatten();

        let destination = if let Some(pmid) = pm_id {
            sqlx::query_scalar::<_, Option<String>>(
                "SELECT COALESCE(brand, 'Bank Account') || COALESCE(' ending in ' || NULLIF(last4, ''), '') FROM payment_methods WHERE id = $1",
            )
            .bind(pmid)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten()
            .flatten()
            .unwrap_or_else(|| "your bank account".to_string())
        } else {
            "your bank account".to_string()
        };

        let abs_cents = amount_cents.unsigned_abs();
        let dollars = abs_cents / 100;
        let rem = abs_cents % 100;
        let dollars_str = dollars
            .to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .map(|chunks| chunks.join(","))
            .unwrap_or_else(|_| dollars.to_string());
        let amount_display = format!("USD {}.{:02}", dollars_str, rem);

        let mut payload = serde_json::json!({
            "amount_display": amount_display,
            "destination": destination,
        });
        if let Some(r) = reason {
            payload["admin_notes"] = serde_json::Value::String(r);
        }

        let _ = crate::email::trigger_transactional_email(&db, &user_id, event_type, payload).await;
    });
}

/// Payload for the bulk withdrawal action endpoint.
#[derive(serde::Deserialize)]
pub struct AdminWithdrawBulkPayload {
    /// Withdrawal request UUIDs to act on.
    pub ids: Vec<String>,
    /// "approve" | "reject"
    pub action: String,
    /// Required when action="reject"; ignored for approve.
    pub reason: Option<String>,
}

/// POST /api/admin/withdrawals/bulk
///
/// Apply the same action to many withdrawal_requests in one call.
/// Mirrors the deposits bulk endpoint — per-id results, partial-failure
/// tolerant.
pub async fn api_admin_withdrawals_bulk(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<AdminWithdrawBulkPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_withdraw_permission(&admin, &state.db, "withdrawals.write").await?;
    if payload.ids.is_empty() {
        return Err(AppError::BadRequest("No withdrawal ids supplied".into()));
    }
    if payload.ids.len() > 200 {
        return Err(AppError::BadRequest(
            "Bulk size capped at 200 — split your selection".into(),
        ));
    }

    let mut results: Vec<serde_json::Value> = Vec::with_capacity(payload.ids.len());
    let mut succeeded = 0;
    let mut failed = 0;

    for raw_id in &payload.ids {
        let uid = match Uuid::parse_str(raw_id) {
            Ok(u) => u,
            Err(_) => {
                failed += 1;
                results.push(serde_json::json!({"id": raw_id, "ok": false, "error": "invalid_id"}));
                continue;
            }
        };

        let outcome: Result<&'static str, String> = match payload.action.as_str() {
            "approve" => approve_one(&state.db, uid, admin.user.id)
                .await
                .map(|_| "approved"),
            "reject" => {
                let reason = payload
                    .reason
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "Bulk admin rejection".to_string());
                reject_one(&state.db, uid, admin.user.id, &reason)
                    .await
                    .map(|_| "rejected")
            }
            other => Err(format!("unknown_action_{}", other)),
        };

        match outcome {
            Ok(s) => {
                succeeded += 1;
                results.push(serde_json::json!({"id": raw_id, "ok": true, "status": s}));
            }
            Err(e) => {
                failed += 1;
                results.push(serde_json::json!({"id": raw_id, "ok": false, "error": e}));
            }
        }
    }

    Ok(Json(serde_json::json!({
        "succeeded": succeeded,
        "failed": failed,
        "results": results,
    })))
}

/// Approve a single withdrawal. Extracted so the bulk handler shares
/// the same transition logic as the per-row endpoint.
async fn approve_one(db: &sqlx::PgPool, req_id: Uuid, admin_user_id: Uuid) -> Result<(), String> {
    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    let req: Option<(Uuid, i64, String, String)> = sqlx::query_as(
        "SELECT user_id, amount_cents, currency, status FROM withdrawal_requests
           WHERE id = $1 FOR UPDATE",
    )
    .bind(req_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let (user_id, amount_cents, _currency, status) = req.ok_or_else(|| "not_found".to_string())?;
    if status != "pending" {
        return Err(format!("not_pending_{}", status));
    }
    sqlx::query(
        "UPDATE withdrawal_requests SET status = 'approved', approved_at = NOW() WHERE id = $1",
    )
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE wallet_transactions SET status = 'completed'
           WHERE external_ref_id = $1 AND type = 'withdrawal'",
    )
    .bind(req_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id)
         VALUES ($1, 'withdrawal.approved_bulk', 'withdrawal_request', $2)",
    )
    .bind(admin_user_id)
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    spawn_withdraw_email(
        db.clone(),
        user_id,
        req_id,
        amount_cents,
        "withdraw_approved",
        None,
    );
    Ok(())
}

/// Reject a single withdrawal + refund amount + fee. Mirrors the
/// per-row endpoint's logic.
async fn reject_one(
    db: &sqlx::PgPool,
    req_id: Uuid,
    admin_user_id: Uuid,
    reason: &str,
) -> Result<(), String> {
    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    let req: Option<(String, i64, i64, String, Uuid)> = sqlx::query_as(
        "SELECT status, amount_cents, fee_cents, currency, user_id
           FROM withdrawal_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(req_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    let (status, amount_cents, fee_cents, currency, user_id) =
        req.ok_or_else(|| "not_found".to_string())?;
    if status != "pending" {
        return Err(format!("not_pending_{}", status));
    }
    sqlx::query(
        "UPDATE withdrawal_requests SET status = 'rejected', admin_notes = $1 WHERE id = $2",
    )
    .bind(reason)
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1
           WHERE user_id = $2 AND wallet_type = 'cash' AND currency = $3",
    )
    .bind(amount_cents.saturating_add(fee_cents))
    .bind(user_id)
    .bind(currency)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "UPDATE wallet_transactions SET status = 'failed'
           WHERE external_ref_id = $1 AND type = 'withdrawal'",
    )
    .bind(req_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id)
         VALUES ($1, 'withdrawal.rejected_bulk', 'withdrawal_request', $2)",
    )
    .bind(admin_user_id)
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    spawn_withdraw_email(
        db.clone(),
        user_id,
        req_id,
        amount_cents,
        "withdraw_rejected",
        Some(reason.to_string()),
    );
    Ok(())
}

/// POST /api/admin/withdrawals/:req_id/reject
pub async fn api_admin_withdrawal_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
    Json(payload): Json<RejectWithdrawalPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_withdraw_permission(&admin, &state.db, "withdrawals.write").await?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("TX begin failed: {}", e)))?;

    // Fetch status, amount, currency, fee, and user_id atomically with
    // row lock. fee_cents must be refunded too — we deducted amount+fee
    // on submission.
    let req: Option<(String, i64, i64, String, Uuid)> = sqlx::query_as(
        "SELECT status, amount_cents, fee_cents, currency, user_id
           FROM withdrawal_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(req_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Withdrawal fetch failed: {}", e)))?;

    let (status, amount_cents, fee_cents, currency, user_id) = match req {
        Some(r) => r,
        None => return Err(AppError::NotFound("Withdrawal request not found".into())),
    };

    if status != "pending" {
        return Err(AppError::BadRequest(format!(
            "Cannot reject: request is '{}'",
            status
        )));
    }

    // Mark as rejected
    sqlx::query(
        "UPDATE withdrawal_requests SET status = 'rejected', admin_notes = $1 WHERE id = $2",
    )
    .bind(&payload.reason)
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Withdrawal rejection failed: {}", e)))?;

    // Refund amount AND fee — both were debited at submission time.
    sqlx::query("UPDATE wallets SET balance_cents = balance_cents + $1 WHERE user_id = $2 AND wallet_type = 'cash' AND currency = $3")
        .bind(amount_cents.saturating_add(fee_cents))
        .bind(user_id)
        .bind(currency)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Internal(format!("Wallet refund failed: {}", e)))?;

    // Mark the pending ledger tx as failed
    sqlx::query(
        "UPDATE wallet_transactions SET status = 'failed' WHERE external_ref_id = $1 AND type = 'withdrawal'",
    )
    .bind(req_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Ledger tx update failed: {}", e)))?;

    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id)
         VALUES ($1, 'withdrawal.rejected', 'withdrawal_request', $2)",
    )
    .bind(admin.user.id)
    .bind(req_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Audit log failed: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("TX commit failed: {}", e)))?;

    // Reason stored in admin_notes (DB); not logged to avoid PII/free-text leak.
    tracing::info!(
        request_id = %req_id,
        user_id = %user_id,
        "Withdrawal rejected"
    );

    spawn_withdraw_email(
        state.db.clone(),
        user_id,
        req_id,
        amount_cents,
        "withdraw_rejected",
        Some(payload.reason.clone()),
    );

    Ok(Json(serde_json::json!({ "status": "success" })))
}
