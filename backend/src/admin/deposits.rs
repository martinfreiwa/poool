use super::extractors::ApiError;
use crate::auth::routes::AppState;
use crate::common::idempotency::{self, Reservation};
use crate::payments;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

/// Atomic deposit-cancel + audit entry. Wraps both in one transaction
/// so a cancel without its audit_log row is impossible (H-4 fix).
async fn cancel_one_deposit_with_audit(
    db: &sqlx::PgPool,
    deposit_id: uuid::Uuid,
    admin_id: uuid::Uuid,
    reason: &str,
) -> Result<&'static str, String> {
    let mut tx = db.begin().await.map_err(|e| e.to_string())?;
    let res = sqlx::query(
        "UPDATE deposit_requests
            SET status = 'cancelled', updated_at = NOW()
          WHERE id = $1 AND status IN ('pending', 'requested')",
    )
    .bind(deposit_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    if res.rows_affected() != 1 {
        return Err("not_pending".to_string());
    }
    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.deposit_cancel_bulk', 'deposit_request', $2, $3)"#,
    )
    .bind(admin_id)
    .bind(deposit_id)
    .bind(serde_json::json!({ "reason": reason }))
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok("cancelled")
}

async fn require_deposit_permission(
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

/// GET /api/admin/deposits  List deposit transactions with user info.
pub async fn api_admin_deposits(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    require_deposit_permission(&jar, &state, "deposits.read").await?;

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
            String,
            String,
            String,
            bool,
            Option<String>,
        ),
    >(
        r#"SELECT d.id::text, d.status, d.amount_cents, d.currency, d.provider_reference,
                  d.provider, d.expires_at::text, d.created_at::text, d.updated_at::text,
                  d.user_id::text,
                  COALESCE(u.email, ''),
                  COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, ''),
                  (d.proof_gcs_path IS NOT NULL) AS has_proof,
                  d.user_notes
           FROM deposit_requests d
           JOIN users u ON u.id = d.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           ORDER BY d.created_at DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let deposits: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let email = r.10.clone();
            let display_name = {
                let n = r.11.trim();
                if n.is_empty() {
                    email.clone()
                } else {
                    n.to_string()
                }
            };
            serde_json::json!({
                "id": r.0, "type": "deposit", "status": r.1, "amount_cents": r.2,
                "currency": r.3, "external_ref_id": r.4,
                "provider": r.5, "expires_at": r.6, "created_at": r.7,
                "updated_at": r.8,
                "user_id": r.9,
                "user_email": email,
                "user_name": display_name,
                "has_proof": r.12,
                "user_notes": r.13,
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

    let oldest_pending_age_seconds: Option<f64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (NOW() - MIN(created_at)))::float8 FROM deposit_requests WHERE status = 'pending'",
    )
    .fetch_one(&state.db)
    .await
    .ok()
    .flatten();

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
            "confirmed_24h_value_cents": confirmed_value_cents,
            "oldest_pending_age_seconds": oldest_pending_age_seconds
        }
    }))
    .into_response())
}

/// Optional admin note payload for manually confirming a deposit.
#[derive(serde::Deserialize)]
pub struct AdminDepositConfirmPayload {
    /// Internal admin note recorded with the deposit confirmation.
    notes: Option<String>,
}

/// POST /api/admin/deposits/:tx_id/confirm
///
/// Confirms a pending deposit and credits the user wallet.
///
/// Hardened against double-confirm via `Idempotency-Key` (audit H#1):
/// repeating the same header within 24h replays the original JSON
/// response without firing a second `deposit_confirmed` email.
///
/// The post-confirm transactional email is enqueued *synchronously*
/// into `transactional_email_outbox` (audit M#5) — a pod restart
/// between the DB commit and the email enqueue no longer drops the
/// user's notification, because `trigger_transactional_email`'s first
/// step is the durable outbox INSERT.
pub async fn api_admin_deposit_confirm(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
    payload: Option<Json<AdminDepositConfirmPayload>>,
) -> Result<axum::response::Response, ApiError> {
    let admin = require_deposit_permission(&jar, &state, "deposits.write").await?;
    let uid = ApiError::parse_uuid(&tx_id)?;

    // Reserve the Idempotency-Key for (header, admin.id). Replays return
    // the cached JSON; in-flight duplicates get 409.
    let reservation = idempotency::try_reserve(
        &state.db,
        &headers,
        admin.id,
        &format!("/api/admin/deposits/{tx_id}/confirm"),
        "POST",
    )
    .await;
    let idem_key = match reservation {
        Reservation::NoKey => None,
        Reservation::Reserved(k) => Some(k),
        Reservation::CachedJson { status, body } => {
            return Ok((status, Json(body)).into_response());
        }
        Reservation::CachedRedirect { .. } => {
            return Err(ApiError::Internal(
                "Unexpected cached redirect on confirm".into(),
            ));
        }
        Reservation::InProgress => {
            return Err(ApiError::Conflict(
                "Confirmation already in progress for this Idempotency-Key".into(),
            ));
        }
    };

    let provider_ref: Option<String> =
        sqlx::query_scalar("SELECT provider_reference FROM deposit_requests WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::from)?;

    let provider_ref = match provider_ref {
        Some(r) => r,
        None => {
            if let Some(k) = &idem_key {
                idempotency::release(&state.db, k).await;
            }
            return Err(ApiError::NotFound("Deposit request not found".to_string()));
        }
    };

    let notes = payload.and_then(|Json(payload)| {
        payload
            .notes
            .map(|notes| notes.trim().to_string())
            .filter(|notes| !notes.is_empty())
    });

    match payments::service::confirm_deposit_with_audit(
        &state.db,
        &provider_ref,
        Some(admin.id),
        notes,
    )
    .await
    {
        Ok(_) => {
            // Enqueue confirmation email *synchronously* — the outbox
            // INSERT inside `trigger_transactional_email` is the durability
            // anchor. Keep it before the response so a pod restart between
            // commit and enqueue cannot drop the user's email. Immediate
            // send is then attempted by `trigger_transactional_email`
            // itself; the periodic outbox worker retries on failure.
            if let Ok(Some((user_id, amount_cents))) = sqlx::query_as::<_, (uuid::Uuid, i64)>(
                "SELECT user_id, amount_cents FROM deposit_requests WHERE id = $1",
            )
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            {
                let amount_display = crate::common::currency::format_usd(amount_cents);
                let _ = crate::email::trigger_transactional_email(
                    &state.db,
                    &user_id,
                    "deposit_confirmed",
                    serde_json::json!({
                        "deposit_id": uid.to_string(),
                        "confirmed_by": admin.id.to_string(),
                        "amount_display": amount_display,
                    }),
                )
                .await;
            }

            let body = serde_json::json!({"status": "confirmed"});
            if let Some(k) = &idem_key {
                idempotency::commit_json(&state.db, k, StatusCode::OK, &body).await;
            }
            Ok(Json(body).into_response())
        }
        Err(e) => {
            if let Some(k) = &idem_key {
                idempotency::release(&state.db, k).await;
            }
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
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
    Json(payload): Json<AdminDepositCancelPayload>,
) -> Result<axum::response::Response, ApiError> {
    let admin = require_deposit_permission(&jar, &state, "deposits.write").await?;

    let uid = ApiError::parse_uuid(&tx_id)?;
    let reason = payload
        .reason
        .map(|reason| reason.trim().to_string())
        .filter(|reason| !reason.is_empty())
        .unwrap_or_else(|| "Admin cancelled".to_string());

    let mut tx = state.db.begin().await.map_err(ApiError::from)?;
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "SELECT id, status, expires_at::text FROM deposit_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(uid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    let Some((deposit_id, previous_status, previous_expires_at)) = row else {
        return Err(ApiError::NotFound("Deposit request not found".to_string()));
    };

    if previous_status != "pending" {
        return Err(ApiError::Conflict(
            "Deposit is already processed".to_string(),
        ));
    }

    sqlx::query(
        "UPDATE deposit_requests SET status = 'cancelled', updated_at = NOW() WHERE id = $1",
    )
    .bind(deposit_id)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'admin.deposit_cancel', 'deposit_request', $2, $3, $4)"#,
    )
    .bind(admin.id)
    .bind(deposit_id)
    .bind(serde_json::json!({"status": previous_status, "expires_at": previous_expires_at}))
    .bind(serde_json::json!({"status": "cancelled", "reason": reason}))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"status": "cancelled"})).into_response())
}

/// POST /api/admin/deposits/:tx_id/extend - Extend expiry by 48 hours for a pending deposit.
pub async fn api_admin_deposit_extend_expiry(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let admin = require_deposit_permission(&jar, &state, "deposits.write").await?;

    let uid = ApiError::parse_uuid(&tx_id)?;

    let mut tx = state.db.begin().await.map_err(ApiError::from)?;
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
        "SELECT id, status, expires_at::text FROM deposit_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(uid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    let Some((deposit_id, previous_status, previous_expires_at)) = row else {
        return Err(ApiError::NotFound("Deposit request not found".to_string()));
    };

    if previous_status != "pending" {
        return Err(ApiError::Conflict(
            "Deposit is already processed".to_string(),
        ));
    }

    let new_expires_at: Option<String> = sqlx::query_scalar(
        r#"UPDATE deposit_requests
           SET expires_at = COALESCE(expires_at, NOW()) + INTERVAL '48 hours',
               updated_at = NOW()
           WHERE id = $1
           RETURNING expires_at::text"#,
    )
    .bind(deposit_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
           VALUES ($1, 'admin.deposit_extend_expiry', 'deposit_request', $2, $3, $4)"#,
    )
    .bind(admin.id)
    .bind(deposit_id)
    .bind(serde_json::json!({"status": previous_status, "expires_at": previous_expires_at}))
    .bind(serde_json::json!({"status": "pending", "expires_at": new_expires_at, "extended_by_hours": 48}))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({
        "status": "extended",
        "extended_by_hours": 48,
        "expires_at": new_expires_at
    }))
    .into_response())
}

/// Payload for the bulk deposit action endpoint.
#[derive(serde::Deserialize)]
pub struct AdminDepositBulkPayload {
    /// Deposit request UUIDs to act on.
    pub ids: Vec<String>,
    /// "confirm" | "cancel"
    pub action: String,
    /// Optional admin notes attached to the confirm audit entry.
    pub notes: Option<String>,
    /// Optional rejection reason — overrides the default for action="cancel".
    pub reason: Option<String>,
}

/// POST /api/admin/deposits/bulk
///
/// Apply the same action (confirm / cancel) to many deposit_requests in
/// one request. Returns per-id results so the UI can show partial-
/// failure detail instead of "X of Y succeeded". Each id is processed
/// independently — a failure on one doesn't roll back the others.
pub async fn api_admin_deposits_bulk(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<AdminDepositBulkPayload>,
) -> Result<axum::response::Response, ApiError> {
    let admin = require_deposit_permission(&jar, &state, "deposits.write").await?;
    if payload.ids.is_empty() {
        return Err(ApiError::BadRequest("No deposit ids supplied".into()));
    }
    if payload.ids.len() > 200 {
        return Err(ApiError::BadRequest(
            "Bulk size capped at 200 — split your selection".into(),
        ));
    }

    let mut results: Vec<serde_json::Value> = Vec::with_capacity(payload.ids.len());
    let mut succeeded = 0;
    let mut failed = 0;

    for raw_id in &payload.ids {
        let uid = match ApiError::parse_uuid(raw_id) {
            Ok(u) => u,
            Err(_) => {
                failed += 1;
                results.push(serde_json::json!({"id": raw_id, "ok": false, "error": "invalid_id"}));
                continue;
            }
        };

        let outcome: Result<&'static str, String> = match payload.action.as_str() {
            "confirm" => {
                let provider_ref: Option<String> = sqlx::query_scalar(
                    "SELECT provider_reference FROM deposit_requests WHERE id = $1",
                )
                .bind(uid)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();
                match provider_ref {
                    Some(r) => crate::payments::service::confirm_deposit_with_audit(
                        &state.db,
                        &r,
                        Some(admin.id),
                        payload.notes.clone().filter(|s| !s.trim().is_empty()),
                    )
                    .await
                    .map(|_| "confirmed"),
                    None => Err("not_found".to_string()),
                }
            }
            "cancel" => {
                let reason = payload
                    .reason
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| "Bulk admin cancellation".to_string());
                // H-4 fix: do the cancel UPDATE + audit INSERT in one
                // transaction so we never have a state change without
                // its corresponding audit row. Compliance reviewers can
                // reconstruct every transition.
                cancel_one_deposit_with_audit(&state.db, uid, admin.id, &reason).await
            }
            other => Err(format!("unknown_action_{}", other)),
        };

        match outcome {
            Ok(status) => {
                succeeded += 1;
                results.push(serde_json::json!({"id": raw_id, "ok": true, "status": status}));
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
    }))
    .into_response())
}

/// GET /api/admin/deposits/reconciliation
///
/// On-demand reconciliation report. Runs the same checks the background
/// worker runs hourly: expires stale deposits, counts stuck pending
/// deposits split by proof-uploaded vs not, counts withdrawals beyond
/// the review SLA. Returns the resulting counts as JSON.
pub async fn api_admin_deposits_reconciliation(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    require_deposit_permission(&jar, &state, "deposits.read").await?;

    let report = crate::wallet::reconciliation::run_once(&state.db)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "deposits_expired": report.deposits_expired,
        "deposits_stuck_no_proof": report.deposits_stuck_no_proof,
        "deposits_stuck_with_proof": report.deposits_stuck_with_proof,
        "withdrawals_stuck": report.withdrawals_stuck,
        "generated_at": chrono::Utc::now().to_rfc3339(),
    }))
    .into_response())
}

/// GET /api/admin/deposits/:tx_id/proof-url
///
/// Mints a short-lived (15-minute) signed URL for the deposit's
/// proof-of-transfer file. Returns 404 if no proof is attached. The raw
/// `gs://` path is never returned — only the signed URL the admin can open
/// in a new tab.
pub async fn api_admin_deposit_proof_url(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(tx_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    require_deposit_permission(&jar, &state, "deposits.read").await?;
    let uid = ApiError::parse_uuid(&tx_id)?;

    let row: Option<(
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT proof_gcs_path, proof_uploaded_at, user_notes
               FROM deposit_requests WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::from)?;

    let (proof_gcs_path, proof_uploaded_at, user_notes) = match row {
        Some(r) => r,
        None => return Err(ApiError::NotFound("Deposit request not found".to_string())),
    };

    let gcs_path = match proof_gcs_path {
        Some(p) => p,
        None => {
            return Err(ApiError::NotFound(
                "No proof attached to this deposit".to_string(),
            ))
        }
    };

    // Strip the gs://bucket/ prefix to get the object path
    let (bucket, object_path) = if let Some(rest) = gcs_path.strip_prefix("gs://") {
        let mut split = rest.splitn(2, '/');
        let b = split.next().unwrap_or("").to_string();
        let p = split.next().unwrap_or("").to_string();
        (b, p)
    } else {
        // Local fallback path (dev only) — just return the raw path
        return Ok(Json(serde_json::json!({
            "signed_url": gcs_path,
            "uploaded_at": proof_uploaded_at,
            "user_notes": user_notes,
            "expires_in_minutes": null,
            "local": true,
        }))
        .into_response());
    };

    let signed = match crate::storage::service::generate_signed_url(&bucket, &object_path, 15).await
    {
        Ok(url) => url,
        Err(e) => {
            tracing::error!("Failed to sign deposit proof URL for {}: {}", tx_id, e);
            return Err(ApiError::Internal(
                "Could not generate signed URL".to_string(),
            ));
        }
    };

    Ok(Json(serde_json::json!({
        "signed_url": signed,
        "uploaded_at": proof_uploaded_at,
        "user_notes": user_notes,
        "expires_in_minutes": 15,
        "local": false,
    }))
    .into_response())
}
