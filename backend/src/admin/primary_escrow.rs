use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::extractors::AdminUser;
use crate::auth::routes::AppState;
use crate::error::AppError as ApiError;

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
}

// Removed redundant page_admin_primary_escrow as page_admin_generic handles it.

/// JSON API to list all open and pending primary offering campaigns
pub async fn api_admin_primary_escrow_list(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<EscrowCampagin>>, ApiError> {
    let pool = &state.db;

    let rows = sqlx::query!(
        r#"
        SELECT 
            id, title, funding_status, funding_end_at,
            tokens_total, tokens_available, min_funding_tokens, 
            token_price_cents, COALESCE(escrow_agent, 'unassigned') as escrow_agent
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
        let current_escrow_cents = sold as i64 * row.token_price_cents;
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
            current_escrow_cents,
            target_total_cents,
            target_min_cents,
            escrow_agent: row.escrow_agent.unwrap_or_else(|| "unassigned".to_string()),
            progress_percent: progress,
        });
    }

    Ok(Json(response))
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
    let expired_assets = sqlx::query!(
        r#"
        SELECT id, title, tokens_total, tokens_available, min_funding_tokens
        FROM assets
        WHERE funding_status IN ('funding_open', 'funding_in_progress')
          AND funding_end_at < NOW()
          AND (tokens_total - tokens_available) < min_funding_tokens
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    for asset in expired_assets {
        let sold = asset.tokens_total - asset.tokens_available;
        tracing::warn!(
            "Asset '{}' ({}) expired. Target {} tokens, sold {}. Initiating auto-refund abort sequence.",
            asset.title, asset.id, asset.min_funding_tokens, sold
        );

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // 1. Mark asset as aborted
        sqlx::query!(
            "UPDATE assets SET funding_status = 'aborted', updated_at = NOW() WHERE id = $1",
            asset.id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        // 2. Find and refund all investments (funding_in_progress)
        let investments = sqlx::query!(
            r#"
            SELECT id, user_id, purchase_value_cents
            FROM investments
            WHERE asset_id = $1 AND status IN ('funding_in_progress', 'active')
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

        sentry::capture_message(
            &format!(
                "Auto-refund completed for '{}'. {} investors refunded.",
                asset.title,
                investments.len()
            ),
            sentry::Level::Info,
        );

        tx.commit()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
    }

    Ok(())
}
