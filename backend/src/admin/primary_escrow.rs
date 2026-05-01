use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;

const RELEASE_REASON_MAX_CHARS: usize = 500;

/// Response type for the Primary Escrow tracker
#[derive(Serialize)]
pub struct EscrowCampagin {
    /// Asset ID
    pub asset_id: Uuid,
    /// Human readable title
    pub title: String,
    /// System status e.g., 'funding_in_progress'
    pub funding_status: String,
    /// Expiration deadline for funding
    pub funding_end_at: Option<String>,
    /// How many tokens exist
    pub tokens_total: i32,
    /// How many are left
    pub tokens_available: i32,
    /// Soft cap in tokens
    pub min_funding_tokens: i32,
    /// Price per token
    pub token_price_cents: i64,
    /// Calculation: Sold tokens * Price
    pub current_escrow_cents: i64,
    /// Calculation: Total tokens * Price
    pub target_total_cents: i64,
    /// Calculation: Min tokens * Price
    pub target_min_cents: i64,
    /// Assigned escrow agent name
    pub escrow_agent: String,
    /// Percentage sold vs Total tokens
    pub progress_percent: f64,
    /// Whether the soft-cap is reached and admins may request release.
    pub release_ready: bool,
}

/// Request body for creating a primary escrow release approval request.
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct ReleaseRequestPayload {
    pub notarization_reference: String,
    pub reason: Option<String>,
}

// Removed redundant page_admin_primary_escrow as page_admin_generic handles it.

/// JSON API to list all open and pending primary offering campaigns
pub async fn api_admin_primary_escrow_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<EscrowCampagin>>, ApiError> {
    let pool = &state.db;
    if !crate::auth::middleware::has_permission(pool, admin.user.id, "marketplace.view").await
        && !crate::auth::middleware::has_permission(pool, admin.user.id, "marketplace.manage").await
        && !crate::auth::middleware::has_permission(pool, admin.user.id, "marketplace.compliance")
            .await
    {
        return Err(ApiError::Forbidden(
            "Missing marketplace permission".to_string(),
        ));
    }

    let rows = sqlx::query!(
        r#"
        SELECT 
            id, title, funding_status, funding_end_at,
            tokens_total, tokens_available, min_funding_tokens, 
            token_price_cents, COALESCE(escrow_agent, 'unassigned') as escrow_agent,
            COALESCE((
                SELECT SUM(i.purchase_value_cents)::bigint
                FROM investments i
                WHERE i.asset_id = assets.id
                  AND i.status IN ('funding_in_progress', 'active')
            ), 0)::bigint as "current_escrow_cents!"
        FROM assets
        WHERE funding_status IN ('funding_open', 'funding_in_progress')
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let mut response = Vec::new();
    for row in rows {
        let sold = row.tokens_total - row.tokens_available;
        let target_total_cents = row.tokens_total as i64 * row.token_price_cents;
        let target_min_cents = row.min_funding_tokens as i64 * row.token_price_cents;

        let progress = if row.tokens_total > 0 {
            (sold as f64 / row.tokens_total as f64) * 100.0
        } else {
            0.0
        };

        response.push(EscrowCampagin {
            asset_id: row.id,
            title: row.title,
            funding_status: row.funding_status,
            funding_end_at: row
                .funding_end_at
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string()),
            tokens_total: row.tokens_total,
            tokens_available: row.tokens_available,
            min_funding_tokens: row.min_funding_tokens,

            token_price_cents: row.token_price_cents,
            current_escrow_cents: row.current_escrow_cents,
            target_total_cents,
            target_min_cents,
            escrow_agent: row.escrow_agent.unwrap_or_else(|| "unassigned".to_string()),
            progress_percent: progress,
            release_ready: sold >= row.min_funding_tokens && row.current_escrow_cents > 0,
        });
    }

    Ok(Json(response))
}

fn normalize_release_text(value: &str, field: &str) -> Result<String, ApiError> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }
    if normalized.len() > RELEASE_REASON_MAX_CHARS {
        return Err(ApiError::BadRequest(format!(
            "{field} must be {RELEASE_REASON_MAX_CHARS} characters or fewer"
        )));
    }
    Ok(normalized)
}

/// POST /api/admin/primary-escrow/:asset_id/release-request
///
/// Creates a maker/checker approval request for releasing a funded primary escrow.
pub async fn api_admin_primary_escrow_release_request(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    Json(payload): Json<ReleaseRequestPayload>,
) -> Result<axum::response::Response, ApiError> {
    let pool = &state.db;
    admin.require_permission(pool, "marketplace.manage").await?;
    let asset_uuid = asset_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest(format!("Invalid ID format: {}", asset_id)))?;
    let notarization_reference =
        normalize_release_text(&payload.notarization_reference, "Notarization reference")?;
    let reason = match payload.reason {
        Some(value) => Some(normalize_release_text(&value, "Release reason")?),
        None => None,
    };

    let mut tx = pool.begin().await.map_err(ApiError::Database)?;

    let asset = sqlx::query!(
        r#"
        SELECT title, funding_status, tokens_total, tokens_available, min_funding_tokens
        FROM assets
        WHERE id = $1
        FOR UPDATE
        "#,
        asset_uuid
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    if !matches!(
        asset.funding_status.as_str(),
        "funding_open" | "funding_in_progress"
    ) {
        return Err(ApiError::BadRequest(
            "Asset is not in a releasable funding state".to_string(),
        ));
    }

    let sold_tokens = asset.tokens_total - asset.tokens_available;
    if sold_tokens < asset.min_funding_tokens {
        return Err(ApiError::BadRequest(
            "Minimum funding target has not been reached".to_string(),
        ));
    }

    let active_pending_request: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM admin_approval_requests
        WHERE action_type = 'primary_escrow.release'
          AND entity_type = 'assets'
          AND entity_id = $1
          AND status = 'pending'
        LIMIT 1
        "#,
    )
    .bind(asset_uuid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Database)?;
    if let Some(existing_id) = active_pending_request {
        return Err(ApiError::Conflict(format!(
            "Release approval request already pending: {existing_id}"
        )));
    }

    let approval_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO admin_approval_requests (requester_id, action_type, entity_type, entity_id, payload)
        VALUES ($1, 'primary_escrow.release', 'assets', $2, $3)
        RETURNING id
        "#,
    )
    .bind(admin.user.id)
    .bind(asset_uuid)
    .bind(serde_json::json!({
        "notarization_reference": notarization_reference,
        "reason": reason,
        "asset_title": asset.title,
        "sold_tokens": sold_tokens,
        "min_funding_tokens": asset.min_funding_tokens,
    }))
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'primary_escrow.release_requested', 'assets', $2, $3)
        "#,
    )
    .bind(admin.user.id)
    .bind(asset_uuid)
    .bind(serde_json::json!({
        "approval_id": approval_id,
        "notarization_reference": notarization_reference,
        "reason": reason,
    }))
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Database)?;

    tx.commit().await.map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "status": "pending_approval",
        "approval_id": approval_id,
        "message": "Release request created. A different administrator must approve it."
    }))
    .into_response())
}

/// Execute a four-eyes-approved primary escrow release.
pub async fn execute_primary_escrow_release(
    pool: &PgPool,
    approver_id: Uuid,
    asset_id: Uuid,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let notarization_reference = payload
        .get("notarization_reference")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "notarization_reference missing from release payload".to_string())?
        .to_string();
    let reason = payload
        .get("reason")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("Four-eyes approved primary escrow release")
        .to_string();

    let mut tx = pool.begin().await.map_err(|e| format!("TX error: {e}"))?;

    let asset = sqlx::query!(
        r#"
        SELECT title, funding_status, tokens_total, tokens_available, min_funding_tokens
        FROM assets
        WHERE id = $1
        FOR UPDATE
        "#,
        asset_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Failed to lock asset: {e}"))?
    .ok_or_else(|| "Asset not found".to_string())?;

    if !matches!(
        asset.funding_status.as_str(),
        "funding_open" | "funding_in_progress"
    ) {
        return Err(format!(
            "Asset is not releasable from status {}",
            asset.funding_status
        ));
    }

    let sold_tokens = asset.tokens_total - asset.tokens_available;
    if sold_tokens < asset.min_funding_tokens {
        return Err("Minimum funding target has not been reached".to_string());
    }

    let active_investments = sqlx::query!(
        r#"
        UPDATE investments
        SET status = 'active', updated_at = NOW()
        WHERE asset_id = $1
          AND status = 'funding_in_progress'
        "#,
        asset_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to activate investments: {e}"))?
    .rows_affected();

    let completed_orders = sqlx::query!(
        r#"
        UPDATE orders
        SET status = 'completed', completed_at = COALESCE(completed_at, NOW())
        WHERE status = 'pending'
          AND id IN (
              SELECT order_id FROM order_items WHERE asset_id = $1
          )
        "#,
        asset_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to complete pending orders: {e}"))?
    .rows_affected();

    sqlx::query!(
        r#"
        UPDATE assets
        SET funding_status = 'funded', updated_at = NOW()
        WHERE id = $1
          AND funding_status IN ('funding_open', 'funding_in_progress')
        "#,
        asset_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to mark asset funded: {e}"))?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state)
        VALUES ($1, 'primary_escrow.released', 'assets', $2, $3, $4)
        "#,
    )
    .bind(approver_id)
    .bind(asset_id)
    .bind(serde_json::json!({
        "funding_status": asset.funding_status,
        "sold_tokens": sold_tokens,
    }))
    .bind(serde_json::json!({
        "funding_status": "funded",
        "notarization_reference": notarization_reference,
        "reason": reason,
        "activated_investments": active_investments,
        "completed_orders": completed_orders,
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to write release audit log: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("Commit error: {e}"))?;

    Ok(serde_json::json!({
        "asset_id": asset_id,
        "asset_title": asset.title,
        "funding_status": "funded",
        "notarization_reference": notarization_reference,
        "activated_investments": active_investments,
        "completed_orders": completed_orders,
    }))
}

/// Core Abort & Auto-Refund Worker (Phase 16.4)
/// Continuously checks for `funding_open` or `funding_in_progress` assets
/// whose `funding_end_at` has passed without reaching `min_funding_tokens`.
pub async fn run_auto_refund_worker(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(600)); // Every 10 mins

    loop {
        interval.tick().await;

        if let Err(e) = process_expired_escrow_refunds(&pool).await {
            tracing::error!("Auto-refund worker encountered an error: {}", e);
            sentry::capture_message(
                &format!("Auto-refund worker error: {}", e),
                sentry::Level::Error,
            );
        }
    }
}

async fn process_expired_escrow_refunds(pool: &PgPool) -> Result<(), ApiError> {
    loop {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let Some(asset) = sqlx::query!(
            r#"
            SELECT id, title, tokens_total, tokens_available, min_funding_tokens
            FROM assets
            WHERE funding_status IN ('funding_open', 'funding_in_progress')
              AND funding_end_at < NOW()
              AND (tokens_total - tokens_available) < min_funding_tokens
            ORDER BY funding_end_at ASC NULLS LAST, created_at ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
            "#
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        else {
            tx.commit()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            break;
        };

        let sold = asset.tokens_total - asset.tokens_available;
        tracing::warn!(
            "Asset '{}' ({}) expired. Target {} tokens, sold {}. Initiating auto-refund abort sequence.",
            asset.title, asset.id, asset.min_funding_tokens, sold
        );

        let updated = sqlx::query!(
            r#"
            UPDATE assets
            SET funding_status = 'aborted', updated_at = NOW()
            WHERE id = $1
              AND funding_status IN ('funding_open', 'funding_in_progress')
            "#,
            asset.id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .rows_affected();
        if updated != 1 {
            tx.rollback()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
            continue;
        }

        // 2. Find and refund all investments (funding_in_progress)
        let investments = sqlx::query!(
            r#"
            SELECT id, user_id, purchase_value_cents
            FROM investments
            WHERE asset_id = $1 AND status IN ('funding_in_progress', 'active')
            FOR UPDATE
            "#,
            asset.id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        for inv in &investments {
            // Credit wallet
            let wallet_id: Uuid = sqlx::query_scalar!(
                r#"
                INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
                VALUES ($1, 'cash', 'USD', $2)
                ON CONFLICT (user_id, wallet_type, currency) DO UPDATE
                SET balance_cents = wallets.balance_cents + $2, updated_at = NOW()
                RETURNING id
                "#,
                inv.user_id,
                inv.purchase_value_cents
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            // Log tx
            sqlx::query!(
                r#"
                INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description)
                VALUES ($1, 'refund', 'completed', $2, 'USD', $3)
                "#,
                wallet_id, inv.purchase_value_cents, format!("Auto-refund: Escrow target not met for {}", asset.title)
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            // Mark investment refunded
            sqlx::query!(
                "UPDATE investments SET status = 'refunded', updated_at = NOW() WHERE id = $1",
                inv.id
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            // Fail associated orders if pending (best-effort link via order items, or fail all pending for this asset)
            sqlx::query(
                r#"
                UPDATE orders SET status = 'failed'
                WHERE id IN (
                    SELECT o.id FROM orders o 
                    JOIN order_items oi ON oi.order_id = o.id 
                    WHERE oi.asset_id = $1 AND o.status = 'pending'
                )
                "#,
            )
            .bind(asset.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        }

        sqlx::query!(
            r#"
            INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state, metadata)
            VALUES (NULL, 'primary_escrow.auto_refund', 'assets', $1, $2, $3)
            "#,
            asset.id,
            serde_json::json!({
                "funding_status": "aborted",
                "refunded_investments": investments.len(),
            }),
            serde_json::json!({
                "sold_tokens": sold,
                "min_funding_tokens": asset.min_funding_tokens,
            })
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        sentry::capture_message(
            &format!(
                "Auto-refund completed for '{}'. {} investors refunded.",
                asset.title,
                investments.len()
            ),
            sentry::Level::Info,
        );
    }

    Ok(())
}
