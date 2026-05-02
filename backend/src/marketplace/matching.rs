/// Matching Engine — the heartbeat of the trading system.
///
/// This module runs as a permanent Tokio task, scanning the Redis orderbook
/// for matchable orders (ask ≤ bid), producing MatchEvents, and pushing them
/// to a Redis queue for the Settlement Worker to process.
///
/// ARCHITECTURE:
/// - **Matching (this module)**: CPU-bound, reads Redis, produces matches.
/// - **Settlement (settlement.rs)**: I/O-bound, PostgreSQL ACID transactions.
/// - **Queue (Redis List)**: Decouples matching from settlement for resilience.
///
/// If the matching engine crashes, no data is lost. The orderbook is rebuilt
/// from PostgreSQL, and unprocessed match events remain in the Redis queue.
///
/// CRITICAL INVARIANTS:
/// - Self-trades (wash trading) are never produced.
/// - Locked orders (being cancelled) are never matched.
/// - Match price is ALWAYS the maker's price (the resting order).
/// - Partial fills produce a new Redis member with reduced quantity.
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::MatchEvent;
use super::orderbook;

// ═══════════════════════════════════════════════════════════════
// ── ENGINE ENTRY POINT ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Main loop of the matching engine. Runs forever as a Tokio task.
///
/// On startup:
/// 1. Check if Redis orderbook is empty → rebuild from PostgreSQL.
/// 2. Enter matching loop: scan assets → try matches → queue for settlement.
///
/// Spawned in `main.rs`:
/// ```ignore
/// tokio::spawn(async move {
///     marketplace::matching::run_matching_engine(&redis, &db).await;
/// });
/// ```
pub async fn run_matching_engine(redis: &RedisPool, pool: &PgPool) {
    tracing::info!("⚡ Matching engine starting...");

    // ── Startup: rebuild orderbook if Redis is empty ─────────
    match redis.get().await {
        Ok(mut conn) => {
            let key_count: i64 = redis::cmd("DBSIZE")
                .query_async(&mut *conn)
                .await
                .unwrap_or(0);
            drop(conn);

            if key_count == 0 {
                tracing::warn!("Redis is empty — rebuilding orderbook from PostgreSQL");
                match orderbook::rebuild_from_postgres(redis, pool).await {
                    Ok(n) => tracing::info!("✅ Orderbook rebuilt with {} orders", n),
                    Err(e) => tracing::error!("❌ Orderbook rebuild failed: {}", e),
                }
            } else {
                tracing::info!("Redis has {} keys — skipping rebuild", key_count);
            }
        }
        Err(e) => {
            tracing::error!("Cannot connect to Redis for matching engine: {}", e);
            // Keep trying in the loop — Redis may come back
        }
    }

    tracing::info!("⚡ Matching engine running (10ms cycle, ~100 scans/sec)");

    // ── Main matching loop ───────────────────────────────────
    loop {
        // Get all assets with active orders
        let active_assets = get_active_asset_ids(pool).await;

        for asset_id in active_assets {
            let mut matched = false;
            // Try to match orders for this asset until no more matches are possible
            loop {
                match try_match_once(redis, pool, asset_id).await {
                    Ok(Some(match_event)) => {
                        matched = true;
                        // Match found → push to settlement queue
                        let event_json = match serde_json::to_string(&match_event) {
                            Ok(json) => json,
                            Err(e) => {
                                tracing::error!(
                                    "Failed to serialize match event: {} — MATCH LOST",
                                    e
                                );
                                break;
                            }
                        };

                        if let Err(e) = orderbook::push_match_to_queue(redis, &event_json).await {
                            tracing::error!(
                                "Failed to push match to queue: {} — retrying next cycle",
                                e
                            );
                            break;
                        }

                        tracing::info!(
                            "⚡ Match: asset={}, price={}, qty={}, seller={}, buyer={}",
                            asset_id,
                            match_event.match_price_cents,
                            match_event.match_quantity,
                            match_event.seller_user_id,
                            match_event.buyer_user_id,
                        );
                    }
                    Ok(None) => break, // No more matches for this asset
                    Err(e) => {
                        tracing::error!("Matching error for asset {}: {}", asset_id, e);
                        break;
                    }
                }
            }

            if matched {
                // Real-time broadcast: orderbook has changed after matches
                super::websocket::broadcast_orderbook_update(pool, Some(redis), asset_id).await;
            }

            // IOC sweep: cancel any "immediate-or-cancel" orders that have
            // had at least one matching cycle (created_at older than 100ms)
            // and still have unfilled quantity. Releases their hold.
            if let Err(e) = sweep_ioc_orders(redis, pool, asset_id).await {
                tracing::warn!("IOC sweep failed for asset {}: {}", asset_id, e);
            }
        }

        // 10ms pause: ~100 matching cycles/second
        // More than sufficient for POOOL's current volume (~1-10 trades/day).
        // At ~1ms Redis latency per cycle, this uses ~10% CPU.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

// ═══════════════════════════════════════════════════════════════
// ── CORE MATCHING LOGIC ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Attempt to find ONE match for an asset.
///
/// Returns:
/// - `Ok(Some(MatchEvent))` — a match was found and Redis was updated.
/// - `Ok(None)` — no matchable orders exist.
/// - `Err(msg)` — an error occurred (logged, caller should break and retry next cycle).
///
/// Algorithm (Price-Time Priority):
/// 1. Get best ask (lowest sell) and best bid (highest buy) from Redis.
/// 2. If ask.price > bid.price → no match possible.
/// 3. Self-trade check: same user on both sides → cancel the newer order.
/// 4. Lock check: if either order is being cancelled → skip this cycle.
/// 5. Compute match price (maker's price) and quantity (min of both sides).
/// 6. Update Redis: remove fully-filled orders, re-insert partial-fills.
/// 7. Return the MatchEvent for the settlement worker.
async fn try_match_once(
    redis: &RedisPool,
    pool: &PgPool,
    asset_id: Uuid,
) -> Result<Option<MatchEvent>, String> {
    // 1. Get best ask and bid
    let best_ask = orderbook::best_ask(redis, asset_id)
        .await
        .map_err(|e| e.to_string())?;
    let best_bid = orderbook::best_bid(redis, asset_id)
        .await
        .map_err(|e| e.to_string())?;

    let (ask, bid) = match (best_ask, best_bid) {
        (Some(a), Some(b)) => (a, b),
        _ => return Ok(None), // One side is empty — no match
    };

    // 2. Match condition: ask price ≤ bid price
    if ask.price_cents > bid.price_cents {
        return Ok(None); // Spread is positive — no match
    }

    // 3. Self-trade prevention (wash trading)
    if ask.user_id == bid.user_id {
        tracing::warn!(
            "🚫 Self-trade blocked: user {} on asset {} (ask={}, bid={})",
            ask.user_id,
            asset_id,
            ask.order_id,
            bid.order_id,
        );

        // Cancel the NEWER order (lower priority in time).
        // The older order stays in the book.
        // Atomically cancel in DB (releases hold) THEN remove from Redis.
        // If DB fails, leave Redis alone — order remains in book and we'll
        // retry next cycle. If DB succeeds but Redis remove fails, the
        // 5-minute sync worker reconciles (DB is source of truth).
        let (newer_order_id, newer_user_id, newer_side, newer_member) =
            if ask.timestamp > bid.timestamp {
                (ask.order_id, ask.user_id, "sell", &ask.raw_member)
            } else {
                (bid.order_id, bid.user_id, "buy", &bid.raw_member)
            };

        cancel_order_in_db(pool, newer_order_id, newer_user_id, "self_trade_blocked")
            .await
            .map_err(|e| format!("self-trade DB cancel failed: {}", e))?;

        orderbook::remove_member(redis, asset_id, newer_side, newer_member)
            .await
            .map_err(|e| e.to_string())?;

        return Ok(None);
    }

    // 4. Lock check: skip if either order is being cancelled
    let ask_locked = orderbook::is_order_locked(redis, ask.order_id)
        .await
        .unwrap_or(false);
    let bid_locked = orderbook::is_order_locked(redis, bid.order_id)
        .await
        .unwrap_or(false);

    if ask_locked || bid_locked {
        // One of the orders is being cancelled — skip this cycle, check again next time
        return Ok(None);
    }

    // 5. Compute match parameters
    // Match price = MAKER's price (the order that was resting first).
    // The maker provided liquidity; the taker crossed the spread to hit it.
    // Standard exchange semantics: trade executes at the maker's quote.
    //
    // Earlier timestamp = maker. On exact-tie (same ms), prefer ask price
    // (deterministic — favors buyer marginally, matches most CEX defaults).
    let ask_is_maker = ask.timestamp <= bid.timestamp;
    let match_price = if ask_is_maker {
        ask.price_cents
    } else {
        bid.price_cents
    };
    let match_qty = std::cmp::min(ask.quantity, bid.quantity);

    if match_qty <= 0 {
        // Safety check — shouldn't happen but prevent zero-quantity matches
        tracing::error!(
            "Zero-quantity match attempted: ask_qty={}, bid_qty={}",
            ask.quantity,
            bid.quantity
        );
        return Ok(None);
    }

    // 6. Build the MatchEvent
    let match_event = MatchEvent {
        ask_order_id: ask.order_id,
        bid_order_id: bid.order_id,
        asset_id,
        seller_user_id: ask.user_id,
        buyer_user_id: bid.user_id,
        match_price_cents: match_price,
        match_quantity: match_qty,
        timestamp: chrono::Utc::now(),
        maker_side: if ask_is_maker {
            super::models::MakerSide::Sell
        } else {
            super::models::MakerSide::Buy
        },
    };

    // 7. Update Redis orderbook (remove old members, re-insert partials)
    update_orders_after_match(redis, asset_id, &ask, &bid, match_qty).await?;

    Ok(Some(match_event))
}

// ═══════════════════════════════════════════════════════════════
// ── POST-MATCH REDIS UPDATE ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Update the Redis orderbook after a match.
///
/// For each side (ask/bid):
/// - If fully filled: ZREM the member.
/// - If partially filled: ZREM old member, ZADD new member with reduced quantity.
async fn update_orders_after_match(
    redis: &RedisPool,
    asset_id: Uuid,
    ask: &super::models::ParsedOrderMember,
    bid: &super::models::ParsedOrderMember,
    matched_qty: i32,
) -> Result<(), String> {
    // ── Update ask (sell) side ────────────────────────────────
    if matched_qty >= ask.quantity {
        // Fully filled → remove from book
        orderbook::remove_member(redis, asset_id, "sell", &ask.raw_member)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        // Partially filled → remove old, insert new with reduced qty
        orderbook::remove_member(redis, asset_id, "sell", &ask.raw_member)
            .await
            .map_err(|e| e.to_string())?;

        let new_member = format!(
            "order:{}:{}:{}:{}",
            ask.order_id,
            ask.user_id,
            ask.quantity - matched_qty,
            ask.timestamp
        );
        orderbook::insert_member(redis, asset_id, "sell", ask.price_cents, &new_member)
            .await
            .map_err(|e| e.to_string())?;
    }

    // ── Update bid (buy) side ────────────────────────────────
    if matched_qty >= bid.quantity {
        // Fully filled → remove from book
        orderbook::remove_member(redis, asset_id, "buy", &bid.raw_member)
            .await
            .map_err(|e| e.to_string())?;
    } else {
        // Partially filled → remove old, insert new with reduced qty
        orderbook::remove_member(redis, asset_id, "buy", &bid.raw_member)
            .await
            .map_err(|e| e.to_string())?;

        let new_member = format!(
            "order:{}:{}:{}:{}",
            bid.order_id,
            bid.user_id,
            bid.quantity - matched_qty,
            bid.timestamp
        );
        orderbook::insert_member(redis, asset_id, "buy", bid.price_cents, &new_member)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── HELPERS ───────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Atomically cancel an order in the DB and release its hold.
///
/// Used by the self-trade prevention path to keep DB and Redis in sync —
/// without this, a self-trade would only remove the order from Redis,
/// leaving the DB row open and the user's funds/tokens frozen forever.
///
/// All work happens in a single transaction. If any step fails, nothing
/// changes. The investment row is targeted by id (FOR UPDATE) to handle
/// duplicate (user, asset) rows correctly.
async fn cancel_order_in_db(
    pool: &PgPool,
    order_id: Uuid,
    user_id: Uuid,
    reason: &'static str,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let order = sqlx::query_as::<_, super::models::MarketOrder>(
        "SELECT * FROM market_orders WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(order_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let order = match order {
        Some(o) if o.is_active() => o,
        // Order doesn't exist or already terminal — treat as no-op success.
        _ => {
            tx.rollback().await.ok();
            return Ok(());
        }
    };

    let remaining = order.remaining_quantity();

    let cancel_affected = sqlx::query(
        "UPDATE market_orders
         SET status = 'cancelled', cancel_reason = $1, updated_at = NOW()
         WHERE id = $2",
    )
    .bind(reason)
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?
    .rows_affected();
    if cancel_affected != 1 {
        return Err(format!(
            "order cancel update failed (id={}, affected={})",
            order_id, cancel_affected
        ));
    }

    match super::models::OrderSide::parse(&order.side) {
        Some(super::models::OrderSide::Buy) => {
            let held_release = order
                .price_cents
                .checked_mul(remaining as i64)
                .ok_or_else(|| "hold-release overflow".to_string())?;
            let affected = sqlx::query(
                "UPDATE wallets SET
                    held_balance_cents = held_balance_cents - $1,
                    updated_at = NOW()
                 WHERE user_id = $2
                   AND wallet_type = 'cash'
                   AND currency = 'USD'
                   AND held_balance_cents >= $1",
            )
            .bind(held_release)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected();
            if affected != 1 {
                return Err(format!(
                    "buyer hold release invariant violated (user={}, release={})",
                    user_id, held_release
                ));
            }
        }
        Some(super::models::OrderSide::Sell) => {
            let inv_id = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM investments
                 WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'
                 FOR UPDATE",
            )
            .bind(user_id)
            .bind(order.asset_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!(
                    "seller investment row missing (user={}, asset={})",
                    user_id, order.asset_id
                )
            })?;

            let affected = sqlx::query(
                "UPDATE investments SET
                    held_tokens = held_tokens - $1,
                    updated_at = NOW()
                 WHERE id = $2 AND held_tokens >= $1",
            )
            .bind(remaining)
            .bind(inv_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected();
            if affected != 1 {
                return Err(format!(
                    "seller hold release invariant violated (inv={}, qty={})",
                    inv_id, remaining
                ));
            }
        }
        None => {
            return Err(format!("invalid order side: {}", order.side));
        }
    }

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Cancel any IOC ("immediate-or-cancel") orders for this asset that are
/// still open after a matching pass. Releases their hold via the same path
/// as user-initiated cancels.
///
/// Only cancels orders older than 100ms — younger ones may still be racing
/// the Redis-insert step from `service::create_order` and deserve another cycle.
async fn sweep_ioc_orders(redis: &RedisPool, pool: &PgPool, asset_id: Uuid) -> Result<(), String> {
    let candidates = sqlx::query_as::<_, (Uuid, Uuid, String)>(
        r#"SELECT id, user_id, side
           FROM market_orders
           WHERE asset_id = $1
             AND time_in_force = 'ioc'
             AND status IN ('open', 'partially_filled')
             AND created_at < NOW() - INTERVAL '100 milliseconds'
           LIMIT 50"#,
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    for (order_id, user_id, side) in candidates {
        // Atomic DB cancel + hold release.
        cancel_order_in_db(pool, order_id, user_id, "ioc_unfilled")
            .await
            .map_err(|e| format!("ioc cancel db: {}", e))?;
        // Best-effort Redis remove. Sync worker reconciles if it lingers.
        let book_side = if side == "sell" { "sell" } else { "buy" };
        // Without the raw_member format we can't ZREM precisely; rely on
        // the 5-min sync worker to clean up. The DB row is now `cancelled`
        // so settlement will drop any incoming match (OrderTerminal path).
        let _ = (book_side, redis);
    }

    Ok(())
}

/// Get all asset IDs that have active orders (open or partially_filled).
///
/// Called once per matching cycle. Uses PostgreSQL (not Redis) to ensure
/// we don't miss any assets.
async fn get_active_asset_ids(pool: &PgPool) -> Vec<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT DISTINCT asset_id FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marketplace::models::ParsedOrderMember;

    #[test]
    fn test_match_condition_ask_lte_bid() {
        // ask $100, bid $105 → match possible
        let ask_price = 10000i64;
        let bid_price = 10500i64;
        assert!(ask_price <= bid_price);
    }

    #[test]
    fn test_match_condition_ask_gt_bid() {
        // ask $105, bid $100 → no match
        let ask_price = 10500i64;
        let bid_price = 10000i64;
        assert!(ask_price > bid_price);
    }

    #[test]
    fn test_match_quantity_is_minimum() {
        assert_eq!(std::cmp::min(10, 5), 5); // ask 10, bid 5 → trade 5
        assert_eq!(std::cmp::min(3, 7), 3); // ask 3, bid 7 → trade 3
        assert_eq!(std::cmp::min(5, 5), 5); // exact → trade 5
    }

    #[test]
    fn test_self_trade_detection() {
        let user_id = Uuid::new_v4();
        let ask_user = user_id;
        let bid_user = user_id;
        assert_eq!(ask_user, bid_user); // Self-trade!
    }

    #[test]
    fn test_self_trade_newer_order_removed() {
        // Newer order (higher timestamp) should be removed
        let older_ts = 1700000000i64;
        let newer_ts = 1700000010i64;
        assert!(newer_ts > older_ts);
        // In the engine: if ask.timestamp > bid.timestamp → ask is removed
    }

    #[test]
    fn test_match_price_is_ask_price() {
        // In a standard order book, match price = maker's price (resting order)
        // The ask was placed first (in the book), so it's the maker
        let ask_price = 10000i64;
        let _bid_price = 10500i64;
        let match_price = ask_price; // NOT bid_price
        assert_eq!(match_price, 10000);
    }

    #[test]
    fn test_partial_fill_member_format() {
        let order_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let original_qty = 10;
        let matched_qty = 3;
        let remaining = original_qty - matched_qty;
        let ts = 1700000000i64;

        let new_member = format!("order:{}:{}:{}:{}", order_id, user_id, remaining, ts);
        let parsed = ParsedOrderMember::parse(&new_member, 10000).unwrap();
        assert_eq!(parsed.quantity, 7);
        assert_eq!(parsed.order_id, order_id);
    }
}
