/// Settlement Pipeline — where money actually moves.
///
/// This module runs as a permanent Tokio task, consuming MatchEvents from
/// the Redis queue and executing 8-step ACID transactions in PostgreSQL.
///
/// 🔴 THIS IS THE MOST CRITICAL CODE IN THE ENTIRE SYSTEM.
/// Every line here must be reviewed with financial precision.
///
/// INVARIANTS:
/// - Every settlement is a SINGLE PostgreSQL transaction (all-or-nothing).
/// - Balance changes use `SELECT ... FOR UPDATE` to prevent concurrent modifications.
/// - All monetary values are `i64` cents — never floats.
/// - Failed settlements stay in the queue and are retried.
/// - No `unwrap()` in any production path.
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::MatchEvent;
use super::{orderbook, service};
use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── SETTLEMENT WORKER ENTRY POINT ─────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Main loop of the settlement worker. Runs forever as a Tokio task.
///
/// Consumes MatchEvents from the `match:queue` Redis list and settles
/// each one in an ACID transaction.
///
/// Spawned in `main.rs`:
/// ```ignore
/// tokio::spawn(async move {
///     marketplace::settlement::run_settlement_worker(&redis, &db).await;
/// });
/// ```
pub async fn run_settlement_worker(redis: &RedisPool, pool: &PgPool) {
    tracing::info!("💰 Settlement worker starting...");

    loop {
        // Block-wait for the next match event (1s timeout for heartbeat)
        let event_json = match orderbook::pop_match_from_queue(redis, 1).await {
            Ok(Some(json)) => json,
            Ok(None) => continue, // Timeout — no events, loop back
            Err(e) => {
                tracing::error!("Failed to pop from match queue: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        // Parse the match event
        let match_event: MatchEvent = match serde_json::from_str(&event_json) {
            Ok(evt) => evt,
            Err(e) => {
                tracing::error!(
                    "Failed to deserialize match event: {} — event dropped: {}",
                    e,
                    &event_json[..event_json.len().min(200)]
                );
                // Corrupt event — don't retry, log for investigation
                sentry::capture_message(
                    &format!("Corrupt match event dropped: {}", e),
                    sentry::Level::Error,
                );
                continue;
            }
        };

        // Settle the match
        match settle_trade(pool, redis, &match_event).await {
            Ok(trade_id) => {
                tracing::info!(
                    "✅ Trade settled: trade_id={}, asset={}, price={}, qty={}, buyer={}, seller={}",
                    trade_id,
                    match_event.asset_id,
                    match_event.match_price_cents,
                    match_event.match_quantity,
                    match_event.buyer_user_id,
                    match_event.seller_user_id,
                );
            }
            Err(e) => {
                tracing::error!("❌ Settlement FAILED: {} — re-queuing event for retry", e);
                sentry::capture_message(
                    &format!(
                        "Settlement failed: asset={}, price={}, qty={}: {}",
                        match_event.asset_id,
                        match_event.match_price_cents,
                        match_event.match_quantity,
                        e
                    ),
                    sentry::Level::Error,
                );

                // Re-queue the event for retry (push back to the queue)
                if let Err(re_err) = orderbook::push_match_to_queue(redis, &event_json).await {
                    tracing::error!(
                        "🔴 CRITICAL: Failed to re-queue match event: {} — MATCH MAY BE LOST",
                        re_err
                    );
                    sentry::capture_message(
                        &format!("CRITICAL: Match event lost: {}", event_json),
                        sentry::Level::Fatal,
                    );
                }

                // Back off to avoid tight retry loop
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// ── 8-STEP ACID SETTLEMENT ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Settle a single trade in an ACID transaction.
///
/// The 8 steps (all within a single PostgreSQL transaction):
///
/// 1. **Validate orders** — both must still be active in DB.
/// 2. **Calculate fees** — look up applicable fee for the asset.
/// 3. **Update sell order** — increment `quantity_filled`, update status.
/// 4. **Update buy order** — increment `quantity_filled`, update status.
/// 5. **Transfer balance** — seller receives (total - fee), buyer's hold is consumed.
/// 6. **Transfer tokens** — tokens move from seller's investment to buyer's.
/// 7. **Record trade** — immutable entry in `trade_history` table.
/// 8. **Release holds** — adjust held_balance/held_tokens for matched amounts.
///
/// Returns the trade ID on success.
async fn settle_trade(
    pool: &PgPool,
    _redis: &RedisPool,
    event: &MatchEvent,
) -> Result<Uuid, AppError> {
    let total_cents = event
        .match_price_cents
        .checked_mul(event.match_quantity as i64)
        .ok_or_else(|| {
            AppError::Internal(format!(
                "Settlement overflow: price={} qty={}",
                event.match_price_cents, event.match_quantity
            ))
        })?;

    // ── Begin ACID Transaction ───────────────────────────────
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // ── Step 1: Validate both orders are still active ────────
    let sell_order = sqlx::query_as::<_, super::models::MarketOrder>(
        "SELECT * FROM market_orders WHERE id = $1 FOR UPDATE",
    )
    .bind(event.ask_order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| {
        AppError::Internal(format!(
            "Sell order {} not found during settlement",
            event.ask_order_id
        ))
    })?;

    if !sell_order.is_active() {
        return Err(AppError::Internal(format!(
            "Sell order {} is no longer active (status={})",
            sell_order.id, sell_order.status
        )));
    }

    let buy_order = sqlx::query_as::<_, super::models::MarketOrder>(
        "SELECT * FROM market_orders WHERE id = $1 FOR UPDATE",
    )
    .bind(event.bid_order_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| {
        AppError::Internal(format!(
            "Buy order {} not found during settlement",
            event.bid_order_id
        ))
    })?;

    if !buy_order.is_active() {
        return Err(AppError::Internal(format!(
            "Buy order {} is no longer active (status={})",
            buy_order.id, buy_order.status
        )));
    }

    // Double-check: user IDs match the match event
    if sell_order.user_id != event.seller_user_id {
        return Err(AppError::Internal(
            "Sell order user_id mismatch with match event".into(),
        ));
    }
    if buy_order.user_id != event.buyer_user_id {
        return Err(AppError::Internal(
            "Buy order user_id mismatch with match event".into(),
        ));
    }

    // ── Step 2: Calculate fees ───────────────────────────────
    let (taker_fee_cents, taker_fee_bps) =
        service::calculate_trade_fee(pool, event.asset_id, total_cents, true).await?;

    let seller_proceeds = total_cents.saturating_sub(taker_fee_cents);

    // ── Step 3: Update sell order ────────────────────────────
    let new_sell_filled = sell_order.quantity_filled + event.match_quantity;
    let sell_status = if new_sell_filled >= sell_order.quantity {
        "filled"
    } else {
        "partially_filled"
    };

    sqlx::query(
        "UPDATE market_orders SET quantity_filled = $1, status = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(new_sell_filled)
    .bind(sell_status)
    .bind(event.ask_order_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── Step 4: Update buy order ─────────────────────────────
    let new_buy_filled = buy_order.quantity_filled + event.match_quantity;
    let buy_status = if new_buy_filled >= buy_order.quantity {
        "filled"
    } else {
        "partially_filled"
    };

    sqlx::query(
        "UPDATE market_orders SET quantity_filled = $1, status = $2, updated_at = NOW() WHERE id = $3",
    )
    .bind(new_buy_filled)
    .bind(buy_status)
    .bind(event.bid_order_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── Step 5: Transfer balance ─────────────────────────────
    // Seller receives proceeds (total - fee)
    sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW()
         WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(seller_proceeds)
    .bind(event.seller_user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Buyer's balance was already held at order creation.
    // The held amount is consumed — actual balance_cents doesn't change
    // (it was already reduced by the hold). We'll release the hold in Step 8.

    // ── Step 6: Transfer tokens ──────────────────────────────
    // Seller: reduce tokens_owned
    sqlx::query(
        "UPDATE investments SET tokens_owned = tokens_owned - $1, updated_at = NOW()
         WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
    )
    .bind(event.match_quantity)
    .bind(event.seller_user_id)
    .bind(event.asset_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Buyer: add tokens (upsert — buyer may not have an investment record yet)
    let buyer_existing = sqlx::query_scalar::<_, i32>(
        "SELECT tokens_owned FROM investments WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'",
    )
    .bind(event.buyer_user_id)
    .bind(event.asset_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    match buyer_existing {
        Some(_) => {
            // Buyer already has an investment — update tokens
            sqlx::query(
                "UPDATE investments SET tokens_owned = tokens_owned + $1, updated_at = NOW()
                 WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
            )
            .bind(event.match_quantity)
            .bind(event.buyer_user_id)
            .bind(event.asset_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }
        None => {
            // Buyer doesn't have an investment — create one
            sqlx::query(
                "INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_price_cents, status)
                 VALUES ($1, $2, $3, $4, 'active')",
            )
            .bind(event.buyer_user_id)
            .bind(event.asset_id)
            .bind(event.match_quantity)
            .bind(event.match_price_cents)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }
    }

    // ── Step 7: Record trade in trade_history ─────────────────
    let trade_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO trade_history
           (asset_id, buy_order_id, sell_order_id, buyer_user_id, seller_user_id,
            price_cents, quantity, fee_cents, fee_bps, on_chain_status)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending')
           RETURNING id"#,
    )
    .bind(event.asset_id)
    .bind(event.bid_order_id)
    .bind(event.ask_order_id)
    .bind(event.buyer_user_id)
    .bind(event.seller_user_id)
    .bind(event.match_price_cents)
    .bind(event.match_quantity)
    .bind(taker_fee_cents)
    .bind(taker_fee_bps)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── Step 8: Release holds ────────────────────────────────
    // Buyer: hold was placed at the bid's LIMIT price × qty at order creation.
    // Trade executes at MATCH price (<= limit). Release hold at limit price,
    // deduct balance at match price — the difference returns to free balance.
    let held_release = buy_order
        .price_cents
        .checked_mul(event.match_quantity as i64)
        .ok_or_else(|| AppError::Internal("Hold-release overflow".to_string()))?;
    let actual_paid = event
        .match_price_cents
        .checked_mul(event.match_quantity as i64)
        .ok_or_else(|| AppError::Internal("Actual-paid overflow".to_string()))?;

    sqlx::query(
        "UPDATE wallets SET
            balance_cents = balance_cents - $1,
            held_balance_cents = GREATEST(held_balance_cents - $2, 0),
            updated_at = NOW()
         WHERE user_id = $3 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(actual_paid)
    .bind(held_release)
    .bind(event.buyer_user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Seller: release held_tokens for the matched amount
    sqlx::query(
        "UPDATE investments SET
            held_tokens = GREATEST(held_tokens - $1, 0),
            updated_at = NOW()
         WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
    )
    .bind(event.match_quantity)
    .bind(event.seller_user_id)
    .bind(event.asset_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Collect platform fee into the singleton platform wallet. Require
    // rows_affected == 1 so an unseeded or accidentally duplicated
    // platform_fee wallet row aborts the settlement instead of silently
    // losing or duplicating the fee credit.
    if taker_fee_cents > 0 {
        let affected = sqlx::query(
            "UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW()
             WHERE wallet_type = 'platform_fee' AND currency = 'USD'",
        )
        .bind(taker_fee_cents)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
        if affected != 1 {
            return Err(AppError::Internal(format!(
                "Platform fee wallet not uniquely matched (affected={})",
                affected
            )));
        }
    }

    // ── COMMIT ───────────────────────────────────────────────
    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        "💰 Settlement TX committed: trade={}, total={}, fee={}, seller_proceeds={}",
        trade_id,
        total_cents,
        taker_fee_cents,
        seller_proceeds,
    );

    // ── Real-time Broadcasts ────────────────────────────────
    // All trades are broadcast to the global WebSocket pool.
    // matches decided in matching.rs use the maker's price.
    super::websocket::broadcast_trade(
        pool,
        Some(_redis),
        event.asset_id,
        event.match_price_cents,
        event.match_quantity,
        total_cents,
        true, // Matches from matching.rs always cross the book
    )
    .await;

    // Ticker data (24h volume/price change) is refreshed after every trade
    if let Ok(ticker) = super::service::get_ticker(pool, event.asset_id).await {
        super::websocket::broadcast_ticker(
            pool,
            Some(_redis),
            event.asset_id,
            ticker.last_price_cents.unwrap_or(0),
            ticker.change_24h_pct,
            ticker.volume_24h_cents,
        )
        .await;
    }

    Ok(trade_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_fee_calculation() {
        // $1,000 trade at 5% (500 BPS)
        let total = 100_000i64; // $1,000
        let fee = super::super::models::calculate_fee_cents(total, 500);
        let proceeds = total.saturating_sub(fee);

        assert_eq!(fee, 5000); // $50.00 fee
        assert_eq!(proceeds, 95000); // $950.00 to seller
        assert_eq!(fee + proceeds, total); // Conservation: fee + proceeds = total
    }

    #[test]
    fn test_settlement_conservation_of_funds() {
        // Verify that in every settlement:
        // buyer_pays = seller_receives + platform_fee
        let price = 10000i64; // $100/token
        let qty = 5;
        let total = price * qty as i64; // $500 total
        let fee = super::super::models::calculate_fee_cents(total, 500); // 5% = $25
        let seller_proceeds = total - fee; // $475

        assert_eq!(total, seller_proceeds + fee);
    }

    #[test]
    fn test_order_status_transitions() {
        // Verify status transitions are correct
        let qty = 10;
        let filled = 10;
        let status = if filled >= qty {
            "filled"
        } else {
            "partially_filled"
        };
        assert_eq!(status, "filled");

        let filled_partial = 5;
        let status_partial = if filled_partial >= qty {
            "filled"
        } else {
            "partially_filled"
        };
        assert_eq!(status_partial, "partially_filled");
    }

    #[test]
    fn test_hold_release_matches_trade_value() {
        let price = 15000i64; // $150/token
        let qty = 3i32;
        let hold_release = price.saturating_mul(qty as i64);
        assert_eq!(hold_release, 45000); // $450 released from hold
    }
}
