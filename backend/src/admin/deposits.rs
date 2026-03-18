use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::payments;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

/// GET /api/admin/deposits  List deposit transactions with user info.
pub async fn api_admin_deposits(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            i64,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"SELECT d.id::text, d.status, d.amount_cents, d.currency, d.provider_reference,
                  d.provider, d.expires_at::text, d.created_at::text,
                  COALESCE(u.email, ''),
                  COALESCE(up.first_name, ''), COALESCE(up.last_name, '')
           FROM deposit_requests d
           JOIN users u ON u.id = d.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           ORDER BY d.created_at DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let deposits: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let name = format!(
                "{} {}",
                r.9.clone().unwrap_or_default(),
                r.10.clone().unwrap_or_default()
            )
            .trim()
            .to_string();
            serde_json::json!({
                "id": r.0, "type": "deposit", "status": r.1, "amount_cents": r.2,
                "currency": r.3, "external_ref_id": r.4,
                "provider": r.5, "expires_at": r.6, "created_at": r.7,
                "user_email": r.8,
                "user_name": if name.is_empty() { r.8.clone() } else { name }
            })
        })
        .collect();

    // Stats
    let pending = deposits.iter().filter(|d| d["status"] == "pending").count();
    let confirmed_24h = deposits.iter().filter(|d| d["status"] == "paid").count();
    let expired = deposits.iter().filter(|d| d["status"] == "expired").count();

    let pending_value_cents: i64 = deposits
        .iter()
        .filter(|d| d["status"] == "pending")
        .filter_map(|d| d["amount_cents"].as_i64())
        .sum();
    let confirmed_value_cents: i64 = deposits
        .iter()
        .filter(|d| d["status"] == "paid")
        .filter_map(|d| d["amount_cents"].as_i64())
        .sum();

    Ok(Json(serde_json::json!({
        "deposits": deposits,
        "stats": {
            "pending_count": pending,
            "confirmed_24h": confirmed_24h,
            "expired_count": expired,
            "volume_30d_cents": deposits.iter()
                .filter(|d| d["status"] == "paid")
                .filter_map(|d| d["amount_cents"].as_i64())
                .sum::<i64>(),
            "volume_30d_count": deposits.iter().filter(|d| d["status"] == "paid").count(),
            "pending_value_cents": pending_value_cents,
            "confirmed_24h_value_cents": confirmed_value_cents
        }
    }))
    .into_response())
}

/// POST /api/admin/deposits/:tx_id/confirm
pub async fn api_admin_deposit_confirm(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&tx_id)?;

    // First, look up the provider reference for the deposit request
    let provider_ref: Option<String> =
        sqlx::query_scalar("SELECT provider_reference FROM deposit_requests WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    let provider_ref = match provider_ref {
        Some(r) => r,
        None => {
            return Err(ApiError::NotFound("Deposit request not found".to_string()));
        }
    };

    // We intentionally ignore any notes from the request body right now.
    // In the future, we can insert the notes into a generic 'notes' field or audit log payload.
    match payments::service::confirm_deposit(&state.db, &provider_ref).await {
        Ok(_) => Ok(Json(serde_json::json!({"status": "confirmed"})).into_response()),
        Err(e) => {
            tracing::error!("Failed to confirm deposit {tx_id}: {e}");
            Err(ApiError::Internal(format!(
                "Failed to confirm deposit: {}",
                e
            )))
        }
    }
}

/// Payload for cancelling a deposit request.
#[derive(serde::Deserialize)]
pub struct AdminDepositCancelPayload {
    reason: Option<String>,
}

/// POST /api/admin/deposits/:tx_id/cancel
pub async fn api_admin_deposit_cancel(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
    Json(payload): Json<AdminDepositCancelPayload>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = _admin.user.id;

    let uid = ApiError::parse_uuid(&tx_id)?;

    let updated = sqlx::query(
        "UPDATE deposit_requests SET status = 'cancelled' WHERE id = $1 AND status = 'pending'",
    )
    .bind(uid)
    .execute(&state.db)
    .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            let reason = payload
                .reason
                .unwrap_or_else(|| "Admin cancelled".to_string());

            // Audit log
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'admin.deposit_cancel', 'deposit_request', $2, $3)"#,
            )
            .bind(admin_user)
            .bind(uid)
            .bind(serde_json::json!({"status": "cancelled", "reason": reason}))
            .execute(&state.db)
            .await;

            Ok(Json(serde_json::json!({"status": "cancelled"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound(
            "Not found or already processed".to_string(),
        )),
        Err(e) => {
            tracing::error!("Failed to cancel deposit {tx_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// POST /api/admin/deposits/:tx_id/extend - Extend expiry by 48 hours for a pending deposit.
pub async fn api_admin_deposit_extend_expiry(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = _admin.user.id;

    let uid = ApiError::parse_uuid(&tx_id)?;

    let updated = sqlx::query(
        r#"UPDATE deposit_requests SET expires_at = COALESCE(expires_at, NOW()) + INTERVAL '48 hours', updated_at = NOW()
           WHERE id = $1 AND status = 'pending'"#
    )
    .bind(uid)
    .execute(&state.db)
    .await;

    match updated {
        Ok(r) if r.rows_affected() > 0 => {
            // Audit log
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'admin.deposit_extend_expiry', 'deposit_request', $2, $3)"#,
            )
            .bind(admin_user)
            .bind(uid)
            .bind(serde_json::json!({"extended_by": "48 hours"}))
            .execute(&state.db)
            .await;

            Ok(
                Json(serde_json::json!({"status": "extended", "extended_by_hours": 48}))
                    .into_response(),
            )
        }
        Ok(_) => Err(ApiError::NotFound(
            "Deposit not found or not pending".to_string(),
        )),
        Err(e) => {
            tracing::error!("Failed to extend deposit expiry {tx_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}
