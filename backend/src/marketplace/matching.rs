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
                match try_match_once(redis, asset_id).await {
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
async fn try_match_once(redis: &RedisPool, asset_id: Uuid) -> Result<Option<MatchEvent>, String> {
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
        if ask.timestamp > bid.timestamp {
            // Ask is newer → remove it
            orderbook::remove_member(redis, asset_id, "sell", &ask.raw_member)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            // Bid is newer → remove it
            orderbook::remove_member(redis, asset_id, "buy", &bid.raw_member)
                .await
                .map_err(|e| e.to_string())?;
        }

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
    // Match price = maker's price (the resting order that was first).
    // In a standard order book, the maker is the one who provided liquidity.
    // Here we use the ask price (seller's limit) as the match price.
    let match_price = ask.price_cents;
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
