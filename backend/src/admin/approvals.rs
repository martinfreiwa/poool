use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::{common::sanitize, payments};
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};
use sqlx::Row;

#[derive(Clone, Copy)]
struct ApprovalActionSpec {
    entity_type: &'static str,
    entity_id_required: bool,
    permission: &'static str,
}

fn action_spec(action_type: &str) -> Option<ApprovalActionSpec> {
    match action_type {
        "deposit.confirm" | "deposit.cancel" => Some(ApprovalActionSpec {
            entity_type: "deposit_requests",
            entity_id_required: true,
            permission: "deposits.write",
        }),
        "balance.adjust" => Some(ApprovalActionSpec {
            entity_type: "user",
            entity_id_required: true,
            permission: "treasury.write",
        }),
        "user.suspend" | "user.delete" => Some(ApprovalActionSpec {
            entity_type: "user",
            entity_id_required: true,
            permission: "admins.manage",
        }),
        "kyc.override" | "kyc.reject" => Some(ApprovalActionSpec {
            entity_type: "kyc_records",
            entity_id_required: true,
            permission: "kyc.override",
        }),
        "settings.update" => Some(ApprovalActionSpec {
            entity_type: "settings",
            entity_id_required: false,
            permission: "roles.edit",
        }),
        "submission.approve" | "submission.reject" => Some(ApprovalActionSpec {
            entity_type: "assets",
            entity_id_required: true,
            permission: "assets.publish",
        }),
        "dividend.process" => Some(ApprovalActionSpec {
            entity_type: "assets",
            entity_id_required: true,
            permission: "financials.payout.approve",
        }),
        "primary_escrow.release" => Some(ApprovalActionSpec {
            entity_type: "assets",
            entity_id_required: true,
            permission: "marketplace.manage",
        }),
        // treasury.payout has no durable executor/table contract yet; do not
        // allow new requests until the real payout flow is wired.
        "treasury.payout" => None,
        _ => None,
    }
}

fn parse_action_contract(
    action_type: &str,
    entity_type: &str,
    entity_id: Option<&str>,
) -> Result<(ApprovalActionSpec, Option<uuid::Uuid>), ApiError> {
    let spec = action_spec(action_type).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "Invalid or unsupported action_type: {}",
            action_type
        ))
    })?;

    if entity_type != spec.entity_type {
        return Err(ApiError::BadRequest(format!(
            "Invalid entity_type for {}: expected {}",
            action_type, spec.entity_type
        )));
    }

    let entity_uuid = match entity_id {
        Some(id) if !id.trim().is_empty() => Some(ApiError::parse_uuid(id.trim())?),
        _ if spec.entity_id_required => {
            return Err(ApiError::BadRequest(format!(
                "entity_id is required for {}",
                action_type
            )));
        }
        _ => None,
    };

    Ok((spec, entity_uuid))
}

// ═══════════════════════════════════════════════════════════════════
//  Four-Eyes (Maker-Checker) Approval Workflow
// ═══════════════════════════════════════════════════════════════════

/// GET /api/admin/approvals — List all approval requests (optionally filtered by status).
pub async fn api_admin_approvals_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "approvals.manage")
        .await?;

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
    .map_err(ApiError::from)?;

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
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "approvals.manage")
        .await?;
    let user = admin.user.clone();
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

    let (spec, entity_uuid) = parse_action_contract(action_type, entity_type, entity_id)?;
    admin.require_permission(&state.db, spec.permission).await?;

    let mut tx = state.db.begin().await.map_err(ApiError::from)?;
    let id = sqlx::query_scalar::<_, uuid::Uuid>(
        "INSERT INTO admin_approval_requests (requester_id, action_type, entity_type, entity_id, payload) VALUES ($1, $2, $3, $4, $5) RETURNING id"
    )
        .bind(user.id)
        .bind(action_type)
        .bind(entity_type)
        .bind(entity_uuid)
        .bind(&action_payload)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::from)?;

    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
    )
        .bind(user.id)
        .bind("approval_request.created")
        .bind("admin_approval_requests")
        .bind(id)
        .bind(serde_json::json!({"action_type": action_type, "entity_type": entity_type, "entity_id": entity_uuid.map(|id| id.to_string())}))
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;

    Ok(Json(serde_json::json!({
        "status": "pending",
        "approval_id": id.to_string(),
        "message": "Approval request created. Awaiting review from another administrator."
    }))
    .into_response())
}

/// POST /api/admin/approvals/:id/approve — Approve a pending request (Checker step).
/// The approver MUST be a different admin than the requester.
pub async fn api_admin_approvals_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(approval_id): axum::extract::Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "approvals.manage")
        .await?;
    let user = admin.user.clone();

    let uid = ApiError::parse_uuid(&approval_id)?;
    let mut tx = state.db.begin().await.map_err(ApiError::from)?;

    // Fetch the approval request
    let row = sqlx::query(
        "SELECT id, requester_id, action_type, entity_type, entity_id, payload, status, expires_at FROM admin_approval_requests WHERE id = $1 FOR UPDATE"
    )
        .bind(uid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("Approval request not found".to_string()))?;

    let status: String = row.get::<Option<String>, _>("status").unwrap_or_default();
    if status != "pending" {
        return Err(ApiError::Conflict(format!("Request is already {}", status)));
    }

    // Four-Eyes enforcement: approver must be different from requester
    let requester_id: uuid::Uuid = row.get("requester_id");
    if requester_id == user.id {
        return Err(ApiError::Forbidden("Four-Eyes violation: You cannot approve your own request. A different administrator must approve.".to_string()));
    }

    // Check expiry removed: Requests remain pending indefinitely until acted upon.
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
    let spec = action_spec(&action_type).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "Invalid or unsupported action_type: {}",
            action_type
        ))
    })?;
    if entity_type != spec.entity_type {
        return Err(ApiError::BadRequest(format!(
            "Invalid entity_type for {}: expected {}",
            action_type, spec.entity_type
        )));
    }
    admin.require_permission(&state.db, spec.permission).await?;

    let updated = sqlx::query(
        r#"UPDATE admin_approval_requests
           SET status = 'processing', approver_id = $1, updated_at = NOW()
           WHERE id = $2 AND status = 'pending'"#,
    )
    .bind(user.id)
    .bind(uid)
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    if updated.rows_affected() != 1 {
        return Err(ApiError::Conflict(
            "Approval request could not be claimed for execution".to_string(),
        ));
    }

    sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, $2, $3, $4, $5)"#,
    )
    .bind(user.id)
    .bind("approval_request.execution_started")
    .bind("admin_approval_requests")
    .bind(uid)
    .bind(serde_json::json!({
        "action_type": action_type,
        "requester_id": requester_id.to_string()
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;

    let execution_result = execute_approved_action(
        &state,
        user.id,
        &action_type,
        &entity_type,
        entity_id,
        &payload,
    )
    .await;

    match execution_result {
        Ok(result_json) => {
            let mut tx = state.db.begin().await.map_err(ApiError::from)?;
            let updated = sqlx::query(
                "UPDATE admin_approval_requests SET status = 'approved', updated_at = NOW() WHERE id = $1 AND status = 'processing'"
            )
                .bind(uid)
                .execute(&mut *tx)
                .await
                .map_err(ApiError::from)?;

            if updated.rows_affected() != 1 {
                return Err(ApiError::Conflict(
                    "Approval request could not be marked approved".to_string(),
                ));
            }

            // Audit log
            sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
                .bind(user.id)
                .bind("approval_request.approved")
                .bind("admin_approval_requests")
                .bind(uid)
                .bind(serde_json::json!({"action_type": action_type, "requester_id": requester_id.to_string(), "result": result_json}))
                .execute(&mut *tx)
                .await
                .map_err(ApiError::from)?;

            tx.commit().await.map_err(ApiError::from)?;

            Ok(Json(serde_json::json!({
                "status": "approved",
                "message": format!("Action '{}' has been approved and executed.", action_type),
                "result": result_json,
            }))
            .into_response())
        }
        Err(err_msg) => {
            tracing::error!("Four-Eyes action execution failed: {err_msg}");
            let mut tx = state.db.begin().await.map_err(ApiError::from)?;
            sqlx::query(
                r#"UPDATE admin_approval_requests
                   SET status = 'rejected', rejection_reason = $1, updated_at = NOW()
                   WHERE id = $2 AND status = 'processing'"#,
            )
            .bind(format!("Execution failed: {err_msg}"))
            .bind(uid)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::from)?;
            sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, $2, $3, $4, $5)"#,
            )
            .bind(user.id)
            .bind("approval_request.execution_failed")
            .bind("admin_approval_requests")
            .bind(uid)
            .bind(serde_json::json!({"action_type": action_type, "error": err_msg}))
            .execute(&mut *tx)
            .await
            .map_err(ApiError::from)?;
            tx.commit().await.map_err(ApiError::from)?;
            Err(ApiError::Internal(err_msg))
        }
    }
}

/// POST /api/admin/approvals/:id/reject — Reject a pending request.
pub async fn api_admin_approvals_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(approval_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "approvals.manage")
        .await?;
    let user = admin.user.clone();

    let uid = ApiError::parse_uuid(&approval_id)?;

    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::BadRequest("Rejection reason is required".to_string()))?;

    // Fetch to check owner
    let mut tx = state.db.begin().await.map_err(ApiError::from)?;
    let row = sqlx::query(
        "SELECT requester_id, action_type, entity_type, status FROM admin_approval_requests WHERE id = $1 FOR UPDATE",
    )
        .bind(uid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound("Approval request not found".to_string()))?;

    let status: String = row.get::<Option<String>, _>("status").unwrap_or_default();
    if status != "pending" {
        return Err(ApiError::Conflict(format!("Request is already {}", status)));
    }

    // Four-eyes: requester cannot also reject their own request
    let requester_id: uuid::Uuid = row.get("requester_id");
    if requester_id == user.id {
        return Err(ApiError::Forbidden(
            "You cannot reject your own request (four-eyes rule)".to_string(),
        ));
    }

    let action_type: String = row
        .get::<Option<String>, _>("action_type")
        .unwrap_or_default();
    let entity_type: String = row
        .get::<Option<String>, _>("entity_type")
        .unwrap_or_default();
    let spec = action_spec(&action_type).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "Invalid or unsupported action_type: {}",
            action_type
        ))
    })?;
    if entity_type != spec.entity_type {
        return Err(ApiError::BadRequest(format!(
            "Invalid entity_type for {}: expected {}",
            action_type, spec.entity_type
        )));
    }
    admin.require_permission(&state.db, spec.permission).await?;

    let updated = sqlx::query(
        "UPDATE admin_approval_requests SET status = 'rejected', approver_id = $1, rejection_reason = $2, updated_at = NOW() WHERE id = $3"
    )
        .bind(user.id)
        .bind(reason)
        .bind(uid)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from)?;

    if updated.rows_affected() != 1 {
        return Err(ApiError::Conflict(
            "Approval request could not be marked rejected".to_string(),
        ));
    }

    // Audit log
    sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
    )
        .bind(user.id)
        .bind("approval_request.rejected")
        .bind("admin_approval_requests")
        .bind(uid)
        .bind(serde_json::json!({"reason": reason}))
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from)?;

    tx.commit().await.map_err(ApiError::from)?;

    Ok(
        Json(serde_json::json!({"status": "rejected", "message": "Request rejected."}))
            .into_response(),
    )
}

/// Execute the actual business action when a Four-Eyes request is approved.
/// This is the "action executor" that performs the operation atomically.
async fn execute_approved_action(
    state: &AppState,
    approver_id: uuid::Uuid,
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
            payments::service::confirm_deposit_with_audit(
                &state.db,
                &provider_ref,
                Some(approver_id),
                Some("Four-eyes approval".to_string()),
            )
            .await
            .map(|_| serde_json::json!({"deposit_id": eid.to_string(), "confirmed": true}))
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
            Err("treasury.payout is not enabled until a durable treasury payout executor is implemented".to_string())
        }
        "settings.update" => {
            let settings_obj = payload
                .as_object()
                .ok_or("settings.update payload must be a JSON object of setting keys")?;
            let mut tx = state
                .db
                .begin()
                .await
                .map_err(|e| format!("TX error: {e}"))?;

            for (key, val) in settings_obj {
                if key.trim().is_empty() || key.len() > 100 {
                    return Err("settings.update keys must be 1-100 characters".to_string());
                }

                let (str_val, val_type) = match val {
                    serde_json::Value::Bool(b) => (b.to_string(), "boolean"),
                    serde_json::Value::Number(n) => (n.to_string(), "number"),
                    serde_json::Value::String(s) => (sanitize::sanitize_text(s), "string"),
                    _ => (val.to_string(), "json"),
                };

                sqlx::query(
                    r#"INSERT INTO platform_settings (key, value, value_type, updated_at, updated_by)
                       VALUES ($1, $2, $3, NOW(), $4)
                       ON CONFLICT (key)
                       DO UPDATE SET value = EXCLUDED.value,
                                     value_type = EXCLUDED.value_type,
                                     updated_at = NOW(),
                                     updated_by = EXCLUDED.updated_by"#,
                )
                .bind(key)
                .bind(&str_val)
                .bind(val_type)
                .bind(approver_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Settings update failed: {e}"))?;
            }

            tx.commit()
                .await
                .map_err(|e| format!("Commit error: {e}"))?;
            Ok(serde_json::json!({"settings_updated": true, "count": settings_obj.len()}))
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
                sqlx::query_scalar("SELECT title FROM assets WHERE id = $1 FOR UPDATE")
                    .bind(aid)
                    .fetch_one(&mut *tx)
                    .await;
            let asset_title = asset_title_res.unwrap_or_else(|_| "Unknown Asset".to_string());

            let rows: Vec<(uuid::Uuid, i32)> = sqlx::query_as(
                "SELECT user_id, tokens_owned FROM investments WHERE asset_id = $1 AND status = 'active' AND tokens_owned > 0 FOR UPDATE"
            ).bind(aid).fetch_all(&mut *tx).await.unwrap_or_default();

            let total_tokens_owned: i32 = rows.iter().map(|(_, t)| *t).sum();

            if total_tokens_owned == 0 {
                return Err("No active investments found for asset".to_string());
            }

            let payout_id = uuid::Uuid::new_v4();
            sqlx::query("INSERT INTO dividend_payouts (id, asset_id, amount_cents, status) VALUES ($1, $2, $3, 'processing')")
                .bind(payout_id).bind(aid).bind(total_amount_cents).execute(&mut *tx).await
                .map_err(|e| format!("Failed to create payout record: {e}"))?;

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
        "primary_escrow.release" => {
            let aid = entity_id.ok_or("entity_id (asset_id) required for primary_escrow.release")?;
            crate::admin::primary_escrow::execute_primary_escrow_release(
                &state.db,
                approver_id,
                aid,
                payload,
            )
            .await
        }
        // Catch-all for future action types
        _ => Err(format!(
            "Unknown action_type: {action_type}. No executor registered."
        )),
    }
}
