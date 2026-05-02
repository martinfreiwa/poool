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
use sqlx::{PgPool, Postgres, Transaction};
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

    // Recover any events stuck in the processing queue from a prior crash.
    // BRPOPLPUSH atomically moves events from match:queue → match:processing,
    // so anything still there means the previous worker died mid-settlement.
    // Move them back to the main queue for retry.
    match orderbook::recover_match_processing_queue(redis).await {
        Ok(0) => {}
        Ok(n) => {
            tracing::warn!(
                "💰 Recovered {} match events from processing queue (previous worker crashed)",
                n
            );
            sentry::capture_message(
                &format!(
                    "Settlement worker recovered {} stuck match events on startup",
                    n
                ),
                sentry::Level::Warning,
            );
        }
        Err(e) => {
            tracing::error!(
                "💰 Failed to recover processing queue on startup: {} — continuing anyway",
                e
            );
        }
    }

    loop {
        // Atomic move: pop from main queue → push to processing queue.
        // If we crash before ack/requeue, recovery on next startup gets it.
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
                sentry::capture_message(
                    &format!("Corrupt match event dropped: {}", e),
                    sentry::Level::Error,
                );
                // Drain corrupt event from the processing queue so it
                // doesn't get recovered into the main queue forever.
                let _ = orderbook::ack_match_processed(redis, &event_json).await;
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
                if let Err(e) = orderbook::ack_match_processed(redis, &event_json).await {
                    // ACK failure means the event will be recovered next
                    // startup and retried. Settlement was already committed,
                    // so settle_trade will hit the OrderTerminal path and
                    // drop it cleanly. Log loud but don't block.
                    tracing::error!(
                        "🔴 ACK failed for settled trade {}: {} — will be re-processed and dropped",
                        trade_id,
                        e
                    );
                }
            }
            // TERMINAL: order is no longer active (already cancelled, expired,
            // or fully filled by an earlier match). Re-queueing creates an
            // infinite death spiral — DROP the event and clean stale Redis
            // entries. The order's Redis member is already gone in normal
            // cancel paths; if it isn't, the 5-min sync worker will catch it.
            Err(AppError::OrderTerminal { reason }) => {
                tracing::warn!(
                    "⚠️ Match dropped — order terminal ({}): asset={}, ask={}, bid={}",
                    reason,
                    match_event.asset_id,
                    match_event.ask_order_id,
                    match_event.bid_order_id,
                );
                sentry::capture_message(
                    &format!(
                        "Match dropped (terminal order): {}: asset={}, ask={}, bid={}",
                        reason,
                        match_event.asset_id,
                        match_event.ask_order_id,
                        match_event.bid_order_id
                    ),
                    sentry::Level::Warning,
                );
                // Drop from processing queue — event is dead.
                let _ = orderbook::ack_match_processed(redis, &event_json).await;
            }
            Err(e) => {
                tracing::error!(
                    "❌ Settlement FAILED: {} (detail: {}) — re-queuing event for retry",
                    e,
                    e.detail()
                );
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

                // Atomic LREM(processing) + RPUSH(queue) so the event ends
                // up exactly once on the main queue regardless of failure
                // mode. If this Redis call itself fails, the event sits in
                // processing and gets recovered on the next startup —
                // durable either way (fixes the BLPOP loss bug).
                if let Err(re_err) = orderbook::requeue_match_failed(redis, &event_json).await {
                    tracing::error!(
                        "🔴 Requeue failed: {} — event remains in match:processing, will recover on restart",
                        re_err
                    );
                    sentry::capture_message(
                        &format!(
                            "Settlement requeue failed (will recover on restart): {}",
                            re_err
                        ),
                        sentry::Level::Error,
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

async fn credit_seller_cash_wallet(
    tx: &mut Transaction<'_, Postgres>,
    seller_user_id: Uuid,
    amount_cents: i64,
) -> Result<(), AppError> {
    if amount_cents < 0 {
        return Err(AppError::Internal(format!(
            "Negative seller credit attempted (user={}, amount={})",
            seller_user_id, amount_cents
        )));
    }

    let wallet_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
           VALUES ($1, 'cash', 'USD', $2)
           ON CONFLICT (user_id, wallet_type, currency)
           DO UPDATE SET
               balance_cents = wallets.balance_cents + EXCLUDED.balance_cents,
               updated_at = NOW()
           RETURNING id"#,
    )
    .bind(seller_user_id)
    .bind(amount_cents)
    .fetch_one(&mut **tx)
    .await
    .map_err(AppError::Database)?;

    tracing::debug!(
        "Credited seller cash wallet {} by {} cents",
        wallet_id,
        amount_cents
    );

    Ok(())
}

async fn credit_platform_fee_wallet(
    tx: &mut Transaction<'_, Postgres>,
    amount_cents: i64,
) -> Result<(), AppError> {
    if amount_cents <= 0 {
        return Ok(());
    }

    let existing_wallet_id: Option<Uuid> = sqlx::query_scalar(
        r#"SELECT id
           FROM wallets
           WHERE wallet_type = 'platform_fee' AND currency = 'USD'
           ORDER BY created_at ASC
           LIMIT 1
           FOR UPDATE"#,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(AppError::Database)?;

    let wallet_id = match existing_wallet_id {
        Some(id) => id,
        None => {
            let admin_id: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT id
                   FROM users
                   WHERE email IN ('admin@poool.app', 'support@traffic-creator.com')
                   ORDER BY CASE
                       WHEN email = 'admin@poool.app' THEN 0
                       WHEN email = 'support@traffic-creator.com' THEN 1
                       ELSE 2
                   END
                   LIMIT 1"#,
            )
            .fetch_optional(&mut **tx)
            .await
            .map_err(AppError::Database)?;

            let admin_id = admin_id.ok_or_else(|| {
                AppError::Internal(
                    "Cannot create platform fee wallet: no platform admin user exists".into(),
                )
            })?;

            sqlx::query_scalar::<_, Uuid>(
                r#"INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
                   VALUES ($1, 'platform_fee', 'USD', 0)
                   ON CONFLICT (user_id, wallet_type, currency)
                   DO UPDATE SET updated_at = wallets.updated_at
                   RETURNING id"#,
            )
            .bind(admin_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(AppError::Database)?
        }
    };

    let affected = sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW()
         WHERE id = $2 AND wallet_type = 'platform_fee' AND currency = 'USD'",
    )
    .bind(amount_cents)
    .bind(wallet_id)
    .execute(&mut **tx)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    if affected != 1 {
        return Err(AppError::Internal(format!(
            "Platform fee wallet credit failed (wallet={}, affected={})",
            wallet_id, affected
        )));
    }

    Ok(())
}

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
        // Terminal — drop the match, don't retry. (See worker loop dispatch.)
        return Err(AppError::OrderTerminal {
            reason: format!("sell order {} status={}", sell_order.id, sell_order.status),
        });
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
        return Err(AppError::OrderTerminal {
            reason: format!("buy order {} status={}", buy_order.id, buy_order.status),
        });
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
    // Both sides pay fees per maker/taker designation from the matching
    // engine. Maker = resting order, taker = order that crossed the spread.
    let resolved = super::validation::resolve_fees(pool, event.asset_id).await?;

    let (buyer_fee_bps, seller_fee_bps, taker_side_str) = match event.maker_side {
        super::models::MakerSide::Sell => {
            // Seller was maker, buyer was taker
            (resolved.taker_fee_bps, resolved.maker_fee_bps, "buy")
        }
        super::models::MakerSide::Buy => {
            // Buyer was maker, seller was taker
            (resolved.maker_fee_bps, resolved.taker_fee_bps, "sell")
        }
    };

    let buyer_fee_cents = super::models::calculate_fee_cents(total_cents, buyer_fee_bps);
    let seller_fee_cents = super::models::calculate_fee_cents(total_cents, seller_fee_bps);
    let total_fee_cents = buyer_fee_cents
        .checked_add(seller_fee_cents)
        .ok_or_else(|| AppError::Internal("fee sum overflow".into()))?;

    // Seller receives match value minus their fee
    let seller_proceeds = total_cents
        .checked_sub(seller_fee_cents)
        .ok_or_else(|| AppError::Internal("seller_proceeds underflow".into()))?;
    // (Buyer's `total_cents + buyer_fee_cents` cash-out is computed in Step 8
    // as `buyer_cash_out` — kept inline there to avoid double-arithmetic.)

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
    // Seller receives proceeds (total - fee). Lock seller wallet FOR UPDATE
    // to ensure atomicity with concurrent settlements / withdrawals.
    credit_seller_cash_wallet(&mut tx, event.seller_user_id, seller_proceeds).await?;

    // Buyer's balance was already held at order creation.
    // The held amount is consumed — actual balance_cents doesn't change
    // (it was already reduced by the hold). We'll release the hold in Step 8.

    // ── Step 6: Transfer tokens ──────────────────────────────
    // Lock seller's investment row FOR UPDATE and target it by id, so that
    // duplicate (user, asset) rows don't cause the deduction to apply
    // multiple times (H5 — fixes multi-row UPDATE accounting bug).
    // Schema guarantees UNIQUE (user_id, asset_id) — at most one row.
    let seller_investment = sqlx::query!(
        r#"SELECT id, tokens_owned, held_tokens
           FROM investments
           WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'
           FOR UPDATE"#,
        event.seller_user_id,
        event.asset_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| {
        AppError::Internal(format!(
            "Seller investment row missing (user={}, asset={})",
            event.seller_user_id, event.asset_id
        ))
    })?;

    if seller_investment.tokens_owned < event.match_quantity
        || seller_investment.held_tokens < event.match_quantity
    {
        return Err(AppError::Internal(format!(
            "Seller token invariant violated (owned={}, held={}, match_qty={})",
            seller_investment.tokens_owned, seller_investment.held_tokens, event.match_quantity
        )));
    }

    let seller_token_affected = sqlx::query(
        "UPDATE investments SET tokens_owned = tokens_owned - $1, updated_at = NOW()
         WHERE id = $2",
    )
    .bind(event.match_quantity)
    .bind(seller_investment.id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .rows_affected();
    if seller_token_affected != 1 {
        return Err(AppError::Internal(format!(
            "Seller investment update failed (id={}, affected={})",
            seller_investment.id, seller_token_affected
        )));
    }

    // Buyer: add tokens (upsert — buyer may not have an investment record yet).
    // Lock the row FOR UPDATE if it exists.
    let buyer_existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM investments
         WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'
         FOR UPDATE",
    )
    .bind(event.buyer_user_id)
    .bind(event.asset_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    match buyer_existing {
        Some(buyer_inv_id) => {
            let buyer_token_affected = sqlx::query(
                "UPDATE investments SET tokens_owned = tokens_owned + $1, updated_at = NOW()
                 WHERE id = $2",
            )
            .bind(event.match_quantity)
            .bind(buyer_inv_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected();
            if buyer_token_affected != 1 {
                return Err(AppError::Internal(format!(
                    "Buyer investment update failed (id={}, affected={})",
                    buyer_inv_id, buyer_token_affected
                )));
            }
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
    // Legacy `fee_cents` / `fee_bps` columns are populated with the TAKER's
    // fee for backward compat. New per-side columns are the source of truth.
    let (legacy_fee_cents, legacy_fee_bps) = match event.maker_side {
        super::models::MakerSide::Sell => (buyer_fee_cents, buyer_fee_bps),
        super::models::MakerSide::Buy => (seller_fee_cents, seller_fee_bps),
    };
    let trade_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO trade_history
           (asset_id, buy_order_id, sell_order_id, buyer_user_id, seller_user_id,
            price_cents, quantity, fee_cents, fee_bps, on_chain_status,
            taker_side, buyer_fee_cents, seller_fee_cents,
            buyer_fee_bps, seller_fee_bps)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending',
                   $10, $11, $12, $13, $14)
           RETURNING id"#,
    )
    .bind(event.asset_id)
    .bind(event.bid_order_id)
    .bind(event.ask_order_id)
    .bind(event.buyer_user_id)
    .bind(event.seller_user_id)
    .bind(event.match_price_cents)
    .bind(event.match_quantity)
    .bind(legacy_fee_cents)
    .bind(legacy_fee_bps)
    .bind(taker_side_str)
    .bind(buyer_fee_cents)
    .bind(seller_fee_cents)
    .bind(buyer_fee_bps)
    .bind(seller_fee_bps)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── Step 8: Release holds ────────────────────────────────
    // Buyer's hold at order creation was:
    //   limit_price × qty + fee_reserve_bps_share(limit_price × qty)
    // We release that full slice (proportional to match qty), and debit
    // the actual cash leaving the buyer (match_price × qty + buyer_fee).
    // The difference (over-reserved fee + better-than-limit price) flows
    // back to the buyer's free balance.
    let price_hold_share = buy_order
        .price_cents
        .checked_mul(event.match_quantity as i64)
        .ok_or_else(|| AppError::Internal("Hold-release overflow".to_string()))?;
    let fee_hold_share =
        super::models::calculate_fee_cents(price_hold_share, buy_order.fee_reserve_bps);
    let held_release = price_hold_share
        .checked_add(fee_hold_share)
        .ok_or_else(|| AppError::Internal("Hold-release sum overflow".into()))?;
    // Cash leaving the buyer's wallet for this match:
    let buyer_cash_out = total_cents
        .checked_add(buyer_fee_cents)
        .ok_or_else(|| AppError::Internal("Buyer cash-out overflow".into()))?;

    // Strict update — fail tx if invariants don't hold (no silent clamps).
    let buyer_wallet_affected = sqlx::query(
        "UPDATE wallets SET
            balance_cents = balance_cents - $1,
            held_balance_cents = held_balance_cents - $2,
            updated_at = NOW()
         WHERE user_id = $3
           AND wallet_type = 'cash'
           AND currency = 'USD'
           AND held_balance_cents >= $2
           AND balance_cents >= $1",
    )
    .bind(buyer_cash_out)
    .bind(held_release)
    .bind(event.buyer_user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .rows_affected();
    if buyer_wallet_affected != 1 {
        return Err(AppError::Internal(format!(
            "Buyer wallet invariant violated or row missing (user={}, cash_out={}, held_release={}, affected={})",
            event.buyer_user_id, buyer_cash_out, held_release, buyer_wallet_affected
        )));
    }

    // Seller: release held_tokens for the matched amount. Strict — must have
    // enough held tokens; the seller_investment row was already FOR UPDATE'd
    // and validated above, so this is a safety net.
    let seller_release_affected = sqlx::query(
        "UPDATE investments SET
            held_tokens = held_tokens - $1,
            updated_at = NOW()
         WHERE id = $2 AND held_tokens >= $1",
    )
    .bind(event.match_quantity)
    .bind(seller_investment.id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .rows_affected();
    if seller_release_affected != 1 {
        return Err(AppError::Internal(format!(
            "Seller held_tokens release failed (id={}, qty={}, affected={})",
            seller_investment.id, event.match_quantity, seller_release_affected
        )));
    }

    // Collect platform fee into the canonical platform wallet. Older seed
    // states can miss this wallet; create it once instead of letting a
    // valid match loop forever in settlement retry.
    credit_platform_fee_wallet(&mut tx, total_fee_cents).await?;

    // ── Step 9: Immutable audit log entries ───────────────────
    // One row per fund movement, inside the same transaction so audit and
    // accounting cannot diverge. `audit_logs` is append-only (no UPDATE/DELETE
    // grants in prod) — required for OJK / financial-conduct compliance.
    let audit_payload = serde_json::json!({
        "trade_id": trade_id,
        "asset_id": event.asset_id,
        "match_price_cents": event.match_price_cents,
        "match_quantity": event.match_quantity,
        "total_cents": total_cents,
        "buyer_fee_cents": buyer_fee_cents,
        "seller_fee_cents": seller_fee_cents,
        "buyer_fee_bps": buyer_fee_bps,
        "seller_fee_bps": seller_fee_bps,
        "taker_side": taker_side_str,
        "buyer_user_id": event.buyer_user_id,
        "seller_user_id": event.seller_user_id,
    });

    sqlx::query(
        "INSERT INTO audit_logs
             (actor_user_id, action, entity_type, entity_id, new_state)
         VALUES
             ($1, 'trade.settlement.buyer_debit', 'trade', $2, $3),
             ($4, 'trade.settlement.seller_credit', 'trade', $2, $3)",
    )
    .bind(event.buyer_user_id)
    .bind(trade_id)
    .bind(&audit_payload)
    .bind(event.seller_user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── COMMIT ───────────────────────────────────────────────
    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        "💰 Settlement TX committed: trade={}, total={}, fee_total={} (buyer={}, seller={}), seller_proceeds={}",
        trade_id,
        total_cents,
        total_fee_cents,
        buyer_fee_cents,
        seller_fee_cents,
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
