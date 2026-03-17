use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::{auth, payments};
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use sqlx::Row;

// ═══════════════════════════════════════════════════════════════════
//  Four-Eyes (Maker-Checker) Approval Workflow
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/approvals — List all approval requests (optionally filtered by status).
pub async fn api_admin_approvals_list(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let rows = sqlx::query(
        r#"SELECT ar.id::text, ar.action_type, ar.entity_type, ar.entity_id::text,
                  ar.payload, ar.status, ar.rejection_reason,
                  ar.expires_at::text, ar.created_at::text, ar.updated_at::text,
                  req.email AS requester_email,
                  COALESCE(rp.first_name || ' ' || rp.last_name, req.email) AS requester_name,
                  app.email AS approver_email,
                  COALESCE(ap.first_name || ' ' || ap.last_name, app.email) AS approver_name
           FROM admin_approval_requests ar
           JOIN users req ON ar.requester_id = req.id
           LEFT JOIN user_profiles rp ON rp.user_id = req.id
           LEFT JOIN users app ON ar.approver_id = app.id
           LEFT JOIN user_profiles ap ON ap.user_id = app.id
           ORDER BY ar.created_at DESC
           LIMIT 200"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let approvals: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<Option<String>, _>("id"),
                "action_type": r.get::<Option<String>, _>("action_type"),
                "entity_type": r.get::<Option<String>, _>("entity_type"),
                "entity_id": r.get::<Option<String>, _>("entity_id"),
                "payload": r.get::<Option<serde_json::Value>, _>("payload"),
                "status": r.get::<Option<String>, _>("status"),
                "rejection_reason": r.get::<Option<String>, _>("rejection_reason"),
                "requester_email": r.get::<Option<String>, _>("requester_email"),
                "requester_name": r.get::<Option<String>, _>("requester_name"),
                "approver_email": r.get::<Option<String>, _>("approver_email"),
                "approver_name": r.get::<Option<String>, _>("approver_name"),
                "expires_at": r.get::<Option<String>, _>("expires_at"),
                "created_at": r.get::<Option<String>, _>("created_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "approvals": approvals,
        "pending_count": approvals.iter().filter(|a| a["status"] == "pending").count(),
    }))
    .into_response())
}

/// POST /api/admin/approvals — Create a new approval request (Maker step).
pub async fn api_admin_approvals_create(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let user = _admin.user.clone();
    let action_type = payload
        .get("action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let entity_type = payload
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let entity_id = payload.get("entity_id").and_then(|v| v.as_str());
    let action_payload = payload
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    if action_type.is_empty() || entity_type.is_empty() {
        return Err(ApiError::BadRequest(
            "action_type and entity_type are required".to_string(),
        ));
    }

    // Validate action_type is a known four-eyes action
    let valid_actions = [
        "deposit.confirm",
        "deposit.cancel",
        "balance.adjust",
        "user.suspend",
        "user.delete",
        "kyc.override",
        "kyc.reject",
        "treasury.payout",
        "settings.update",
        "submission.approve",
        "submission.reject",
    ];
    if !valid_actions.contains(&action_type) {
        return Err(ApiError::BadRequest(format!(
            "Invalid action_type: {}. Must be one of: {:?}",
            action_type, valid_actions
        )));
    }

    let entity_uuid: Option<uuid::Uuid> = entity_id.and_then(|s| s.parse().ok());

    let result = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO admin_approval_requests (requester_id, action_type, entity_type, entity_id, payload) VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
        .bind(user.id)
        .bind(action_type)
        .bind(entity_type)
        .bind(entity_uuid)
        .bind(&action_payload)
        .fetch_one(&state.db)
        .await;

    match result {
        Ok(id) => {
            // Audit log the request creation
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
                .bind(user.id)
                .bind("approval_request.created")
                .bind("admin_approval_requests")
                .bind(id)
                .bind(serde_json::json!({"action_type": action_type, "entity_type": entity_type, "entity_id": entity_id}))
                .execute(&state.db)
                .await;

            Ok(Json(serde_json::json!({
                "status": "pending",
                "approval_id": id.to_string(),
                "message": "Approval request created. Awaiting review from another administrator."
            }))
            .into_response())
        }
        Err(e) => {
            tracing::error!("Failed to create approval request: {e}");
            return Err(ApiError::Internal(
                "Failed to create approval request".to_string(),
            ));
        }
    }
}

/// POST /api/admin/approvals/:id/approve — Approve a pending request (Checker step).
/// The approver MUST be a different admin than the requester.
pub async fn api_admin_approvals_approve(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(approval_id): axum::extract::Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let user = _admin.user.clone();

    let uid = ApiError::parse_uuid(&approval_id)?;

    // Fetch the approval request
    let request = sqlx::query(
        "SELECT id, requester_id, action_type, entity_type, entity_id, payload, status, expires_at FROM admin_approval_requests WHERE id = $1"
    )
        .bind(uid)
        .fetch_optional(&state.db)
        .await;

    let row = match request {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err(ApiError::NotFound("Approval request not found".to_string()));
        }
        Err(e) => {
            tracing::error!("Failed to fetch approval: {e}");
            return Err(ApiError::Internal("Server error".to_string()));
        }
    };

    let status: String = row.get::<Option<String>, _>("status").unwrap_or_default();
    if status != "pending" {
        return Err(ApiError::Conflict(format!("Request is already {}", status)));
    }

    // Four-Eyes enforcement: approver must be different from requester
    let requester_id: uuid::Uuid = row.get("requester_id");
    if requester_id == user.id {
        return Err(ApiError::Forbidden("Four-Eyes violation: You cannot approve your own request. A different administrator must approve.".to_string()));
    }

    // Check expiry
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = row.get("expires_at");
    if let Some(exp) = expires_at {
        if exp < chrono::Utc::now() {
            let _ =
                sqlx::query("UPDATE admin_approval_requests SET status = 'expired' WHERE id = $1")
                    .bind(uid)
                    .execute(&state.db)
                    .await;
            return Err(ApiError::NotFound(
                "This approval request has expired.".to_string(),
            ));
        }
    }

    let action_type: String = row
        .get::<Option<String>, _>("action_type")
        .unwrap_or_default();
    let entity_type: String = row
        .get::<Option<String>, _>("entity_type")
        .unwrap_or_default();
    let entity_id: Option<uuid::Uuid> = row.get("entity_id");
    let payload: serde_json::Value = row
        .get::<Option<serde_json::Value>, _>("payload")
        .unwrap_or(serde_json::json!({}));

    // Execute the action
    let execution_result =
        execute_approved_action(&state, &action_type, &entity_type, entity_id, &payload).await;

    match execution_result {
        Ok(result_json) => {
            // Mark as approved
            let _ = sqlx::query(
                "UPDATE admin_approval_requests SET status = 'approved', approver_id = $1, updated_at = NOW() WHERE id = $2"
            )
                .bind(user.id)
                .bind(uid)
                .execute(&state.db)
                .await;

            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
                .bind(user.id)
                .bind("approval_request.approved")
                .bind("admin_approval_requests")
                .bind(uid)
                .bind(serde_json::json!({"action_type": action_type, "requester_id": requester_id.to_string(), "result": result_json}))
                .execute(&state.db)
                .await;

            Ok(Json(serde_json::json!({
                "status": "approved",
                "message": format!("Action '{}' has been approved and executed.", action_type),
                "result": result_json,
            }))
            .into_response())
        }
        Err(err_msg) => {
            tracing::error!("Four-Eyes action execution failed: {err_msg}");
            Err(ApiError::Internal(err_msg))
        }
    }
}

/// POST /api/admin/approvals/:id/reject — Reject a pending request.
pub async fn api_admin_approvals_reject(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(approval_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    let user = match auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"Unauthorized"})),
            )
                .into_response()
        }
    };
    if !auth::middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"Admin access required"})),
        )
            .into_response();
    }

    let uid: uuid::Uuid = match approval_id.parse() {
        Ok(u) => u,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"Invalid ID"})),
            )
                .into_response()
        }
    };

    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("No reason provided");

    // Fetch to check owner
    let req = sqlx::query("SELECT requester_id, status FROM admin_approval_requests WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await;

    match req {
        Ok(Some(row)) => {
            let status: String = row.get::<Option<String>, _>("status").unwrap_or_default();
            if status != "pending" {
                return (
                    axum::http::StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": format!("Request is already {}", status)})),
                )
                    .into_response();
            }

            let _ = sqlx::query(
                "UPDATE admin_approval_requests SET status = 'rejected', approver_id = $1, rejection_reason = $2, updated_at = NOW() WHERE id = $3"
            )
                .bind(user.id)
                .bind(reason)
                .bind(uid)
                .execute(&state.db)
                .await;

            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
                .bind(user.id)
                .bind("approval_request.rejected")
                .bind("admin_approval_requests")
                .bind(uid)
                .bind(serde_json::json!({"reason": reason}))
                .execute(&state.db)
                .await;

            Json(serde_json::json!({"status": "rejected", "message": "Request rejected."}))
                .into_response()
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"Not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Reject approval error: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"Server error"})),
            )
                .into_response()
        }
    }
}

/// Execute the actual business action when a Four-Eyes request is approved.
/// This is the "action executor" that performs the operation atomically.
async fn execute_approved_action(
    state: &AppState,
    action_type: &str,
    _entity_type: &str,
    entity_id: Option<uuid::Uuid>,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match action_type {
        "deposit.confirm" => {
            let eid = entity_id.ok_or("entity_id required for deposit.confirm")?;
            let provider_ref: Option<String> =
                sqlx::query_scalar("SELECT provider_reference FROM deposit_requests WHERE id = $1")
                    .bind(eid)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);
            let provider_ref = provider_ref.ok_or("Deposit request not found")?;
            payments::service::confirm_deposit(&state.db, &provider_ref)
                .await
                .map(|_| serde_json::json!({"deposit_id": eid.to_string(), "confirmed": true}))
                .map_err(|e| e)
        }
        "deposit.cancel" => {
            let eid = entity_id.ok_or("entity_id required")?;
            sqlx::query("UPDATE deposit_requests SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND status = 'pending'")
                .bind(eid).execute(&state.db).await
                .map(|_| serde_json::json!({"cancelled": true}))
                .map_err(|e| format!("DB error: {e}"))
        }
        "balance.adjust" => {
            let eid = entity_id.ok_or("entity_id (user_id) required")?;
            let amount = payload
                .get("amount_cents")
                .and_then(|v| v.as_i64())
                .ok_or("amount_cents required")?;
            let reason = payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Admin balance adjustment (four-eyes approved)");
            let wallet_type = payload
                .get("wallet_type")
                .and_then(|v| v.as_str())
                .unwrap_or("cash");

            let mut tx = state
                .db
                .begin()
                .await
                .map_err(|e| format!("TX error: {e}"))?;

            // 1. Ensure wallet exists and get ID
            let wallet_id: uuid::Uuid = match sqlx::query_scalar(
                "INSERT INTO wallets (user_id, wallet_type, currency) VALUES ($1, $2, 'USD') 
                 ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET updated_at = NOW() RETURNING id"
            )
            .bind(eid).bind(wallet_type).fetch_one(&mut *tx).await {
                Ok(id) => id,
                Err(e) => return Err(format!("Failed to ensure wallet: {e}")),
            };

            // 2. Update balance
            sqlx::query("UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE id = $2")
                .bind(amount).bind(wallet_id)
                .execute(&mut *tx).await
                .map_err(|e| format!("DB error updating wallet: {e}"))?;

            // 3. Create transaction record for audit trail
            let tx_type = if amount >= 0 {
                "admin_credit"
            } else {
                "admin_debit"
            };
            sqlx::query(
                "INSERT INTO wallet_transactions (wallet_id, type, amount_cents, status, description, currency) VALUES ($1, $2, $3, 'completed', $4, 'USD')"
            )
                .bind(wallet_id).bind(tx_type).bind(amount).bind(reason)
                .execute(&mut *tx).await
                .map_err(|e| format!("DB error creating transaction: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Commit error: {e}"))?;
            Ok(
                serde_json::json!({"user_id": eid.to_string(), "wallet_id": wallet_id.to_string(), "adjusted_by": amount, "reason": reason}),
            )
        }
        "user.suspend" => {
            let eid = entity_id.ok_or("entity_id (user_id) required")?;
            sqlx::query("UPDATE users SET status = 'suspended', updated_at = NOW() WHERE id = $1")
                .bind(eid)
                .execute(&state.db)
                .await
                .map(|_| serde_json::json!({"user_id": eid.to_string(), "suspended": true}))
                .map_err(|e| format!("DB error: {e}"))
        }
        "kyc.override" => {
            let eid = entity_id.ok_or("entity_id (kyc_id) required")?;
            let new_status = payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("approved");
            sqlx::query("UPDATE kyc_records SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(new_status)
                .bind(eid)
                .execute(&state.db)
                .await
                .map(|_| serde_json::json!({"kyc_id": eid.to_string(), "new_status": new_status}))
                .map_err(|e| format!("DB error: {e}"))
        }
        "submission.approve" => {
            let eid = entity_id.ok_or("entity_id (asset_id) required")?;
            let mut tx = state
                .db
                .begin()
                .await
                .map_err(|e| format!("TX error: {e}"))?;

            sqlx::query("UPDATE assets SET published = true, funding_status = CASE WHEN funding_status IN ('funded', 'exited') THEN funding_status ELSE 'funding_open' END, updated_at = NOW() WHERE id = $1")
                .bind(eid).execute(&mut *tx).await
                .map_err(|e| format!("DB error updating asset: {e}"))?;

            sqlx::query("UPDATE developer_projects SET status = 'live', updated_at = NOW() WHERE asset_id = $1")
                .bind(eid).execute(&mut *tx).await
                .map_err(|e| format!("DB error updating developer project: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Commit error: {e}"))?;
            Ok(
                serde_json::json!({"asset_id": eid.to_string(), "published": true, "status": "live"}),
            )
        }
        "submission.reject" => {
            let eid = entity_id.ok_or("entity_id (asset_id) required")?;
            let reason = payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Rejected");
            let _ = sqlx::query("UPDATE developer_projects SET status = 'rejected', updated_at = NOW() WHERE asset_id = $1")
                .bind(eid).execute(&state.db).await;
            Ok(serde_json::json!({"asset_id": eid.to_string(), "rejected": true, "reason": reason}))
        }
        "user.delete" => {
            let eid = entity_id.ok_or("entity_id (user_id) required")?;
            sqlx::query("UPDATE users SET status = 'deleted', updated_at = NOW() WHERE id = $1")
                .bind(eid)
                .execute(&state.db)
                .await
                .map(|_| serde_json::json!({"user_id": eid.to_string(), "deleted": true}))
                .map_err(|e| format!("DB error: {e}"))
        }
        "kyc.reject" => {
            let eid = entity_id.ok_or("entity_id (kyc_id) required")?;
            sqlx::query(
                "UPDATE kyc_records SET status = 'rejected', updated_at = NOW() WHERE id = $1",
            )
            .bind(eid)
            .execute(&state.db)
            .await
            .map(|_| serde_json::json!({"kyc_id": eid.to_string(), "rejected": true}))
            .map_err(|e| format!("DB error: {e}"))
        }
        "treasury.payout" => {
            // Simplified treasury payout execution
            let amount = payload
                .get("amount_cents")
                .and_then(|v| v.as_i64())
                .ok_or("amount_cents required")?;
            let mut tx = state
                .db
                .begin()
                .await
                .map_err(|e| format!("TX error: {e}"))?;
            sqlx::query("INSERT INTO treasury_transactions (type, amount_cents, status, created_at) VALUES ('payout', $1, 'completed', NOW())")
                .bind(amount).execute(&mut *tx).await.ok(); // ignore if table doesn't exist
            tx.commit()
                .await
                .map_err(|e| format!("Commit error: {e}"))?;
            Ok(serde_json::json!({"payout_processed": true, "amount_cents": amount}))
        }
        "settings.update" => {
            let mut tx = state
                .db
                .begin()
                .await
                .map_err(|e| format!("TX error: {e}"))?;
            sqlx::query(
                "UPDATE platform_settings SET settings = $1, updated_at = NOW() WHERE id = 1",
            )
            .bind(payload)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Settings update failed: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Commit error: {e}"))?;
            Ok(serde_json::json!({"settings_updated": true}))
        }
        "dividend.process" => {
            let aid = entity_id.ok_or("entity_id (asset_id) required for dividend.process")?;
            let total_amount_cents = payload
                .get("total_amount_cents")
                .and_then(|v| v.as_i64())
                .ok_or("total_amount_cents missing in payload")?;
            let mut tx = state
                .db
                .begin()
                .await
                .map_err(|e| format!("TX error: {e}"))?;

            let asset_title_res: Result<String, _> =
                sqlx::query_scalar("SELECT title FROM assets WHERE id = $1")
                    .bind(aid)
                    .fetch_one(&mut *tx)
                    .await;
            let asset_title = asset_title_res.unwrap_or_else(|_| "Unknown Asset".to_string());

            let total_tokens_owned_res: Result<Option<i32>, _> = sqlx::query_scalar("SELECT SUM(tokens_owned)::int4 FROM investments WHERE asset_id = $1 AND status = 'active'")
                .bind(aid).fetch_one(&mut *tx).await;
            let total_tokens_owned = total_tokens_owned_res.unwrap_or(Some(0)).unwrap_or(0);

            if total_tokens_owned == 0 {
                return Err("No active investments found for asset".to_string());
            }

            let payout_id = uuid::Uuid::new_v4();
            sqlx::query("INSERT INTO dividend_payouts (id, asset_id, amount_cents, status) VALUES ($1, $2, $3, 'processing')")
                .bind(payout_id).bind(aid).bind(total_amount_cents).execute(&mut *tx).await
                .map_err(|e| format!("Failed to create payout record: {e}"))?;

            let rows: Vec<(uuid::Uuid, i32)> = sqlx::query_as(
                "SELECT user_id, tokens_owned FROM investments WHERE asset_id = $1 AND status = 'active' AND tokens_owned > 0"
            ).bind(aid).fetch_all(&mut *tx).await.unwrap_or_default();

            let mut cumulative_allocated: i64 = 0;
            let mut cumulative_exact: u128 = 0;

            for (user_id, tokens) in rows {
                cumulative_exact += total_amount_cents as u128 * (tokens as u128);
                let current_target = ((cumulative_exact + (total_tokens_owned as u128 / 2))
                    / total_tokens_owned as u128) as i64;
                let amount = current_target - cumulative_allocated;
                cumulative_allocated = current_target;

                if amount <= 0 {
                    continue;
                }

                let description = format!("Dividend: {} ({} tokens)", asset_title, tokens);

                // 1. Ensure USD wallet exists
                let wallet_id: uuid::Uuid = match sqlx::query_scalar(
                    "INSERT INTO wallets (user_id, wallet_type, currency) VALUES ($1, 'cash', 'USD') 
                     ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET updated_at = NOW() RETURNING id"
                )
                .bind(user_id).fetch_one(&mut *tx).await {
                    Ok(id) => id,
                    Err(e) => return Err(format!("Failed to ensure wallet for user {user_id}: {e}")),
                };

                // 2. Update balance
                sqlx::query("UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE id = $2")
                    .bind(amount).bind(wallet_id).execute(&mut *tx).await
                    .map_err(|e| format!("Failed to update wallet for {user_id}: {e}"))?;

                // 3. Log transaction
                sqlx::query("INSERT INTO wallet_transactions (wallet_id, amount_cents, type, status, description, currency) VALUES ($1, $2, 'dividend', 'completed', $3, 'USD')")
                    .bind(wallet_id).bind(amount).bind(&description).execute(&mut *tx).await
                    .map_err(|e| format!("Failed to create transaction record for {user_id}: {e}"))?;

                // 4. Update investment rental stats
                sqlx::query("UPDATE investments SET total_rental_cents = total_rental_cents + $1 WHERE user_id = $2 AND asset_id = $3")
                    .bind(amount).bind(user_id).bind(aid).execute(&mut *tx).await
                    .map_err(|e| format!("Failed to update investment stats for {user_id}: {e}"))?;
            }

            sqlx::query("UPDATE dividend_payouts SET status = 'paid' WHERE id = $1")
                .bind(payout_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Failed to finalize payout record: {e}"))?;

            tx.commit()
                .await
                .map_err(|e| format!("Commit error: {e}"))?;
            Ok(
                serde_json::json!({"dividend_processed": true, "payout_id": payout_id.to_string(), "asset_id": aid.to_string()}),
            )
        }
        // Catch-all for future action types
        _ => Err(format!(
            "Unknown action_type: {action_type}. No executor registered."
        )),
    }
}
