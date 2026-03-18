use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
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

/// GET /api/admin/withdrawals
pub async fn api_admin_withdrawals(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<WithdrawalRequestView>>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT wr.id, wr.user_id, u.email as user_email, wr.amount_cents, wr.currency, wr.status,
               wr.payment_method_id, wr.admin_notes, wr.created_at
        FROM withdrawal_requests wr
        JOIN users u ON wr.user_id = u.id
        ORDER BY wr.created_at DESC
        "#,
    )
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
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
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
    .bind(user_id)
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

    Ok(Json(serde_json::json!({ "status": "success" })))
}

/// POST /api/admin/withdrawals/:req_id/reject
pub async fn api_admin_withdrawal_reject(
    _admin: AdminUser,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
    Json(payload): Json<RejectWithdrawalPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("TX begin failed: {}", e)))?;

    // Fetch status, amount, currency, and user_id atomically with row lock
    let req: Option<(String, i64, String, Uuid)> = sqlx::query_as(
        "SELECT status, amount_cents, currency, user_id FROM withdrawal_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(req_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Withdrawal fetch failed: {}", e)))?;

    let (status, amount_cents, currency, user_id) = match req {
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

    // Refund the wallet specifically matching the currency
    sqlx::query("UPDATE wallets SET balance_cents = balance_cents + $1 WHERE user_id = $2 AND wallet_type = 'cash' AND currency = $3")
        .bind(amount_cents)
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

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("TX commit failed: {}", e)))?;

    tracing::info!("Withdrawal {} rejected: {}", req_id, payload.reason);

    Ok(Json(serde_json::json!({ "status": "success" })))
}
