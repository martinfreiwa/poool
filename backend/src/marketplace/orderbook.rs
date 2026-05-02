/// Redis Orderbook — the speed layer for the marketplace.
///
/// This module encapsulates ALL Redis operations for the orderbook.
/// The rest of the codebase NEVER calls Redis directly for order-related data.
///
/// Architecture:
/// - **Redis = cache / speed layer.** PostgreSQL is always the source of truth.
/// - If Redis crashes, `rebuild_from_postgres()` restores the orderbook in seconds.
/// - All Redis failures are non-fatal (graceful degradation, logged and alerted).
///
/// Redis Key Schema:
/// ```text
/// asks:asset:{asset_id}       → Sorted Set (Score = price_cents, Member = order:...)
/// bids:asset:{asset_id}       → Sorted Set (Score = price_cents, Member = order:...)
/// lock:order:{order_id}       → String (TTL 5s, cancel/match race condition lock)
/// idempotency:{key}           → String (TTL 24h, first request result)
/// rl:orders:user:{user_id}    → Counter (TTL 60s, rate limiting)
/// ```
use deadpool_redis::Pool as RedisPool;
use uuid::Uuid;

use super::models::{MarketOrder, OrderbookSnapshot, ParsedOrderMember, PriceLevel};
use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── KEY PREFIXES ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

const ASKS_PREFIX: &str = "asks:asset:";
const BIDS_PREFIX: &str = "bids:asset:";
const LOCK_PREFIX: &str = "lock:order:";
const IDEMPOTENCY_PREFIX: &str = "idempotency:";
const RATE_LIMIT_PREFIX: &str = "rl:orders:user:";
const CANCEL_RATE_LIMIT_PREFIX: &str = "rl:cancels:user:";
const MATCH_QUEUE_KEY: &str = "match:queue";
/// Holding queue for events the settlement worker has popped but not yet
/// committed. On worker crash, events stuck here get drained back to the
/// main queue at next startup. See [`recover_match_processing_queue`].
const MATCH_PROCESSING_KEY: &str = "match:processing";

/// Default orderbook depth (number of price levels).
const DEFAULT_DEPTH: usize = 20;

// ═══════════════════════════════════════════════════════════════
// ── ORDER INSERTION / REMOVAL ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Insert an order into the Redis orderbook.
///
/// Uses ZADD with score = price_cents and member = `order:{id}:{user_id}:{qty}:{ts}`.
/// Fails gracefully if Redis is unavailable (logged, not fatal).
pub async fn insert_order(redis: &RedisPool, order: &MarketOrder) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = order_set_key(order.asset_id, &order.side);
    let member = order.redis_member();

    let _: i32 = redis::cmd("ZADD")
        .arg(&key)
        .arg(order.price_cents)
        .arg(&member)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis ZADD failed: {}", e)))?;

    tracing::debug!(
        "Orderbook: inserted {} order {} at {} cents (asset={})",
        order.side,
        order.id,
        order.price_cents,
        order.asset_id
    );

    Ok(())
}

/// Remove an order from the Redis orderbook.
///
/// Uses ZREM. Fails gracefully if Redis is unavailable.
pub async fn remove_order(redis: &RedisPool, order: &MarketOrder) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = order_set_key(order.asset_id, &order.side);
    let member = order.redis_member();

    let _: i32 = redis::cmd("ZREM")
        .arg(&key)
        .arg(&member)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis ZREM failed: {}", e)))?;

    tracing::debug!(
        "Orderbook: removed {} order {} from asset={}",
        order.side,
        order.id,
        order.asset_id
    );

    Ok(())
}

/// Remove a specific order member by its raw member string.
///
/// Used by the matching engine when it knows the exact member string.
pub async fn remove_member(
    redis: &RedisPool,
    asset_id: Uuid,
    side: &str,
    raw_member: &str,
) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = order_set_key(asset_id, side);

    let _: i32 = redis::cmd("ZREM")
        .arg(&key)
        .arg(raw_member)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis ZREM member failed: {}", e)))?;

    Ok(())
}

/// Insert a raw member string into the orderbook at a given price.
///
/// Used by the matching engine to re-insert partially-filled orders with
/// an updated quantity in the member string.
pub async fn insert_member(
    redis: &RedisPool,
    asset_id: Uuid,
    side: &str,
    price_cents: i64,
    raw_member: &str,
) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = order_set_key(asset_id, side);

    let _: i32 = redis::cmd("ZADD")
        .arg(&key)
        .arg(price_cents)
        .arg(raw_member)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis ZADD member failed: {}", e)))?;

    Ok(())
}

/// Push a match event JSON string onto the settlement queue.
pub async fn push_match_to_queue(redis: &RedisPool, event_json: &str) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let _: i64 = redis::cmd("RPUSH")
        .arg(MATCH_QUEUE_KEY)
        .arg(event_json)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis RPUSH failed: {}", e)))?;

    Ok(())
}

/// Atomically move the next match event from the main queue to the
/// processing queue and return its JSON. Blocks up to `timeout_seconds`.
///
/// Using `BRPOPLPUSH` instead of `BLPOP` is the durability fix: if the
/// settlement worker crashes mid-settlement, the event is still in
/// `match:processing` and gets recovered on next startup. With `BLPOP`
/// the event would simply be lost.
///
/// Caller MUST eventually call either:
/// - [`ack_match_processed`] on success (removes from processing queue), or
/// - [`requeue_match_failed`] on failure (puts it back on main queue).
pub async fn pop_match_from_queue(
    redis: &RedisPool,
    timeout_seconds: u64,
) -> Result<Option<String>, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    // BRPOPLPUSH src dst timeout — atomic move, returns the value or nil.
    // BLMOVE is the modern replacement (Redis 6.2+) but BRPOPLPUSH still
    // works and matches the deadpool-redis API surface used elsewhere.
    let result: Option<String> = redis::cmd("BRPOPLPUSH")
        .arg(MATCH_QUEUE_KEY)
        .arg(MATCH_PROCESSING_KEY)
        .arg(timeout_seconds)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis BRPOPLPUSH failed: {}", e)))?;

    Ok(result)
}

/// Remove a successfully-settled match event from the processing queue.
/// Idempotent — if the event isn't there, LREM returns 0 and we still Ok.
pub async fn ack_match_processed(redis: &RedisPool, event_json: &str) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    // LREM count=1 — remove the first matching occurrence from the head.
    let _: i64 = redis::cmd("LREM")
        .arg(MATCH_PROCESSING_KEY)
        .arg(1)
        .arg(event_json)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis LREM failed: {}", e)))?;
    Ok(())
}

/// Atomically move a failed event from processing back to the main queue
/// for retry. If the LREM fails (event already removed), we still RPUSH
/// because the previous code path also did so on settle failure — keeping
/// behaviour identical.
pub async fn requeue_match_failed(
    redis: &RedisPool,
    event_json: &str,
) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let mut pipe = redis::pipe();
    pipe.cmd("LREM")
        .arg(MATCH_PROCESSING_KEY)
        .arg(1)
        .arg(event_json)
        .ignore();
    pipe.cmd("RPUSH")
        .arg(MATCH_QUEUE_KEY)
        .arg(event_json)
        .ignore();
    let _: () = pipe
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis requeue failed: {}", e)))?;
    Ok(())
}

/// On settlement-worker startup: drain anything stuck in the processing
/// queue (left over from a previous crash) back to the main queue. Safe to
/// call exactly once at boot before the worker starts consuming.
pub async fn recover_match_processing_queue(redis: &RedisPool) -> Result<usize, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let mut recovered = 0usize;
    loop {
        // Atomic move: tail of processing → tail of queue. Non-blocking.
        // Returns nil when processing queue is empty.
        let moved: Option<String> = redis::cmd("RPOPLPUSH")
            .arg(MATCH_PROCESSING_KEY)
            .arg(MATCH_QUEUE_KEY)
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::ServiceUnavailable(format!("Redis RPOPLPUSH failed: {}", e)))?;

        if moved.is_none() {
            break;
        }
        recovered += 1;
        if recovered > 10_000 {
            // Sanity bound — something's wrong if we have this many stuck.
            tracing::error!(
                "🔴 Recovered {} stuck match events; bailing out of recovery loop",
                recovered
            );
            break;
        }
    }
    Ok(recovered)
}

/// Return the current depth of the match settlement queue.
/// Returns -1 on Redis error so callers can persist a sentinel value.
pub async fn match_queue_depth(redis: &RedisPool) -> Result<i64, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;
    let depth: i64 = redis::cmd("LLEN")
        .arg(MATCH_QUEUE_KEY)
        .query_async(&mut *conn)
        .await
        .unwrap_or(0);
    Ok(depth)
}

// ═══════════════════════════════════════════════════════════════
// ── BEST BID / ASK QUERIES ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Get the best ask (lowest sell price) for an asset.
///
/// Returns `None` if there are no sell orders.
pub async fn best_ask(
    redis: &RedisPool,
    asset_id: Uuid,
) -> Result<Option<ParsedOrderMember>, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", ASKS_PREFIX, asset_id);

    // ZRANGEBYSCORE key -inf +inf WITHSCORES LIMIT 0 1 → lowest price
    let result: Vec<(String, f64)> = redis::cmd("ZRANGEBYSCORE")
        .arg(&key)
        .arg("-inf")
        .arg("+inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(1)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis query failed: {}", e)))?;

    Ok(result
        .first()
        .and_then(|(member, score)| ParsedOrderMember::parse(member, *score as i64)))
}

/// Get the best bid (highest buy price) for an asset.
///
/// Returns `None` if there are no buy orders.
pub async fn best_bid(
    redis: &RedisPool,
    asset_id: Uuid,
) -> Result<Option<ParsedOrderMember>, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", BIDS_PREFIX, asset_id);

    // ZREVRANGEBYSCORE key +inf -inf WITHSCORES LIMIT 0 1 → highest price
    let result: Vec<(String, f64)> = redis::cmd("ZREVRANGEBYSCORE")
        .arg(&key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(1)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis query failed: {}", e)))?;

    Ok(result
        .first()
        .and_then(|(member, score)| ParsedOrderMember::parse(member, *score as i64)))
}

/// Get all asks for an asset up to a depth limit.
///
/// Returns orders sorted by price ascending (lowest first).
pub async fn get_asks(
    redis: &RedisPool,
    asset_id: Uuid,
    limit: usize,
) -> Result<Vec<ParsedOrderMember>, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", ASKS_PREFIX, asset_id);

    let raw: Vec<(String, f64)> = redis::cmd("ZRANGEBYSCORE")
        .arg(&key)
        .arg("-inf")
        .arg("+inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(limit)
        .query_async(&mut *conn)
        .await
        .unwrap_or_default();

    Ok(raw
        .iter()
        .filter_map(|(member, score)| ParsedOrderMember::parse(member, *score as i64))
        .collect())
}

/// Get all bids for an asset up to a depth limit.
///
/// Returns orders sorted by price descending (highest first).
pub async fn get_bids(
    redis: &RedisPool,
    asset_id: Uuid,
    limit: usize,
) -> Result<Vec<ParsedOrderMember>, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", BIDS_PREFIX, asset_id);

    let raw: Vec<(String, f64)> = redis::cmd("ZREVRANGEBYSCORE")
        .arg(&key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(limit)
        .query_async(&mut *conn)
        .await
        .unwrap_or_default();

    Ok(raw
        .iter()
        .filter_map(|(member, score)| ParsedOrderMember::parse(member, *score as i64))
        .collect())
}

// ═══════════════════════════════════════════════════════════════
// ── ORDERBOOK SNAPSHOT ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Build a full orderbook snapshot for frontend display.
///
/// Aggregates individual orders into price levels (same price → combined).
/// Returns at most `depth` price levels on each side.
pub async fn get_orderbook_snapshot(
    redis: &RedisPool,
    asset_id: Uuid,
    depth: Option<usize>,
) -> Result<OrderbookSnapshot, AppError> {
    let depth = depth.unwrap_or(DEFAULT_DEPTH);

    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let asks_key = format!("{}{}", ASKS_PREFIX, asset_id);
    let bids_key = format!("{}{}", BIDS_PREFIX, asset_id);

    // Fetch more than needed to allow aggregation
    let fetch_limit = depth * 5;

    // All asks: lowest price first
    let raw_asks: Vec<(String, f64)> = redis::cmd("ZRANGEBYSCORE")
        .arg(&asks_key)
        .arg("-inf")
        .arg("+inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(fetch_limit)
        .query_async(&mut *conn)
        .await
        .unwrap_or_default();

    // All bids: highest price first
    let raw_bids: Vec<(String, f64)> = redis::cmd("ZREVRANGEBYSCORE")
        .arg(&bids_key)
        .arg("+inf")
        .arg("-inf")
        .arg("WITHSCORES")
        .arg("LIMIT")
        .arg(0)
        .arg(fetch_limit)
        .query_async(&mut *conn)
        .await
        .unwrap_or_default();

    // Aggregate into price levels
    let asks = aggregate_price_levels(&raw_asks, depth);
    let bids = aggregate_price_levels(&raw_bids, depth);

    // Calculate spread
    let spread = match (asks.first(), bids.first()) {
        (Some(best_ask), Some(best_bid)) => Some(best_ask.price_cents - best_bid.price_cents),
        _ => None,
    };

    Ok(OrderbookSnapshot {
        asset_id,
        bids,
        asks,
        spread_cents: spread,
        last_price_cents: None, // Caller fills this from trade_history
        timestamp: chrono::Utc::now(),
    })
}

// ═══════════════════════════════════════════════════════════════
// ── ORDER LOCKING (CANCEL/MATCH RACE CONDITION) ───────────────
// ═══════════════════════════════════════════════════════════════

/// Try to acquire a lock on an order (for cancel/match race protection).
///
/// Uses `SET NX EX` — only succeeds if the key doesn't exist.
/// The lock automatically expires after `ttl_seconds`.
///
/// Returns `true` if the lock was acquired, `false` if already locked.
pub async fn try_lock_order(
    redis: &RedisPool,
    order_id: Uuid,
    ttl_seconds: u64,
) -> Result<bool, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", LOCK_PREFIX, order_id);

    let result: Option<String> = redis::cmd("SET")
        .arg(&key)
        .arg("locked")
        .arg("NX")
        .arg("EX")
        .arg(ttl_seconds)
        .query_async(&mut *conn)
        .await
        .unwrap_or(None);

    Ok(result.is_some())
}

/// Release a lock on an order.
pub async fn release_lock(redis: &RedisPool, order_id: Uuid) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", LOCK_PREFIX, order_id);

    let _: i32 = redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis DEL failed: {}", e)))?;

    Ok(())
}

/// Check if an order is currently locked.
pub async fn is_order_locked(redis: &RedisPool, order_id: Uuid) -> Result<bool, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let key = format!("{}{}", LOCK_PREFIX, order_id);

    let exists: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .unwrap_or(None);

    Ok(exists.is_some())
}

// ═══════════════════════════════════════════════════════════════
// ── IDEMPOTENCY ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Check if an idempotency key has already been processed.
///
/// Returns `Some(cached_response)` if the key was found, `None` otherwise.
pub async fn check_idempotency(redis: &RedisPool, key: &str) -> Result<Option<String>, AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let full_key = format!("{}{}", IDEMPOTENCY_PREFIX, key);

    let result: Option<String> = redis::cmd("GET")
        .arg(&full_key)
        .query_async(&mut *conn)
        .await
        .unwrap_or(None);

    Ok(result)
}

/// Store an idempotency result with 24-hour TTL.
pub async fn store_idempotency(redis: &RedisPool, key: &str, result: &str) -> Result<(), AppError> {
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let full_key = format!("{}{}", IDEMPOTENCY_PREFIX, key);

    let _: String = redis::cmd("SETEX")
        .arg(&full_key)
        .arg(86400_u64) // 24 hours
        .arg(result)
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis SETEX failed: {}", e)))?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── RATE LIMITING ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Check order rate limit for a user.
///
/// Allows up to `max_per_window` orders per `window_seconds`.
/// Returns `Ok(())` if under limit, `Err(retry_after_secs)` if exceeded.
pub async fn check_order_rate_limit(
    redis: &RedisPool,
    user_id: Uuid,
    max_per_window: u32,
    window_seconds: u32,
) -> Result<(), u64> {
    let mut conn = match redis.get().await {
        Ok(c) => c,
        Err(_) => {
            // Redis down → allow the request (fail-open for availability)
            tracing::warn!(
                "Rate limiter unavailable, allowing request for user {}",
                user_id
            );
            return Ok(());
        }
    };

    let key = format!("{}{}", RATE_LIMIT_PREFIX, user_id);

    // INCR atomically
    let count: u32 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .unwrap_or(1);

    // Set TTL on first increment
    if count == 1 {
        let _: Result<i32, _> = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(window_seconds)
            .query_async(&mut *conn)
            .await;
    }

    if count > max_per_window {
        // Get remaining TTL for Retry-After header
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut *conn)
            .await
            .unwrap_or(window_seconds as i64);

        return Err(ttl.max(1) as u64);
    }

    Ok(())
}

/// Per-user cancel rate limit. Mirrors `check_order_rate_limit` but uses
/// a separate Redis bucket so a cancel storm can't starve order placement
/// (or vice versa). Default 20 cancels per 60s per masterplan §2.12.
pub async fn check_cancel_rate_limit(
    redis: &RedisPool,
    user_id: Uuid,
    max_per_window: u32,
    window_seconds: u32,
) -> Result<(), u64> {
    let mut conn = match redis.get().await {
        Ok(c) => c,
        Err(_) => {
            tracing::warn!(
                "Cancel rate limiter unavailable, allowing request for user {}",
                user_id
            );
            return Ok(());
        }
    };

    let key = format!("{}{}", CANCEL_RATE_LIMIT_PREFIX, user_id);
    let count: u32 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .unwrap_or(1);
    if count == 1 {
        let _: Result<i32, _> = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(window_seconds)
            .query_async(&mut *conn)
            .await;
    }
    if count > max_per_window {
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut *conn)
            .await
            .unwrap_or(window_seconds as i64);
        return Err(ttl.max(1) as u64);
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── REBUILD FROM POSTGRESQL ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Rebuild the Redis orderbook from PostgreSQL.
///
/// This is the self-healing mechanism: if Redis is empty (after a crash or restart),
/// all active orders are loaded from the `market_orders` table and re-inserted.
///
/// Called:
/// 1. On server startup (if Redis DBSIZE == 0)
/// 2. By the 5-minute sync worker (to fix drift)
/// 3. By admin via `POST /api/admin/marketplace/orderbook/rebuild`
pub async fn rebuild_from_postgres(
    redis: &RedisPool,
    pool: &sqlx::PgPool,
) -> Result<u32, AppError> {
    tracing::warn!("🔄 Rebuilding Redis orderbook from PostgreSQL...");

    // 1. Clear existing orderbook keys
    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    // Find and delete all orderbook keys
    let ask_keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("{}*", ASKS_PREFIX))
        .query_async(&mut *conn)
        .await
        .unwrap_or_default();

    let bid_keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("{}*", BIDS_PREFIX))
        .query_async(&mut *conn)
        .await
        .unwrap_or_default();

    for key in ask_keys.iter().chain(bid_keys.iter()) {
        let _: Result<i32, _> = redis::cmd("DEL").arg(key).query_async(&mut *conn).await;
    }

    // 2. Load all active orders from PostgreSQL
    let open_orders = sqlx::query_as::<_, MarketOrder>(
        "SELECT * FROM market_orders WHERE status IN ('open', 'partially_filled') ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let count = open_orders.len() as u32;

    // 3. Re-insert into Redis
    for order in &open_orders {
        if let Err(e) = insert_order(redis, order).await {
            tracing::error!(
                "Failed to re-insert order {} during rebuild: {}",
                order.id,
                e
            );
            // Continue with other orders — don't fail the entire rebuild
        }
    }

    tracing::info!("✅ Redis orderbook rebuilt: {} orders restored", count);
    Ok(count)
}

/// Sync Redis with PostgreSQL — detect and fix drift.
///
/// Finds orders that exist in PostgreSQL (status = open/partially_filled)
/// but are missing from Redis, and re-inserts them.
///
/// Returns the number of orders that were re-inserted.
pub async fn sync_with_postgres(redis: &RedisPool, pool: &sqlx::PgPool) -> Result<u32, AppError> {
    // Load all active orders from DB
    let db_orders = sqlx::query_as::<_, MarketOrder>(
        "SELECT * FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut fixed = 0u32;

    for order in &db_orders {
        // Check if the order's member exists in the correct Redis ZSET
        let mut conn = match redis.get().await {
            Ok(c) => c,
            Err(_) => continue,
        };

        let key = order_set_key(order.asset_id, &order.side);
        let member = order.redis_member();

        // ZSCORE returns None if member doesn't exist
        let score: Option<f64> = redis::cmd("ZSCORE")
            .arg(&key)
            .arg(&member)
            .query_async(&mut *conn)
            .await
            .unwrap_or(None);

        if score.is_none() {
            // Order is missing from Redis — re-insert
            if insert_order(redis, order).await.is_ok() {
                fixed += 1;
                tracing::warn!(
                    "🔧 Redis sync: re-inserted missing order {} (side={}, asset={})",
                    order.id,
                    order.side,
                    order.asset_id
                );
            }
        }
    }

    Ok(fixed)
}

// ═══════════════════════════════════════════════════════════════
// ── INTERNAL HELPERS ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Generate the Redis key for an order set based on asset_id and side.
fn order_set_key(asset_id: Uuid, side: &str) -> String {
    if side == "sell" {
        format!("{}{}", ASKS_PREFIX, asset_id)
    } else {
        format!("{}{}", BIDS_PREFIX, asset_id)
    }
}

/// Aggregate individual orders into price levels for frontend display.
///
/// Groups orders by price (score), sums their quantities, and counts orders per level.
/// Returns at most `max_levels` aggregated levels.
fn aggregate_price_levels(raw: &[(String, f64)], max_levels: usize) -> Vec<PriceLevel> {
    use std::collections::BTreeMap;

    let mut levels: BTreeMap<i64, (i32, i32)> = BTreeMap::new();

    for (member, score) in raw {
        let price = *score as i64;
        if let Some(parsed) = ParsedOrderMember::parse(member, price) {
            let entry = levels.entry(price).or_insert((0, 0));
            entry.0 += parsed.quantity; // total_quantity
            entry.1 += 1; // order_count
        }
    }

    levels
        .into_iter()
        .take(max_levels)
        .map(|(price, (qty, count))| PriceLevel {
            price_cents: price,
            total_quantity: qty,
            order_count: count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_set_key_buy() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(
            order_set_key(id, "buy"),
            "bids:asset:550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_order_set_key_sell() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(
            order_set_key(id, "sell"),
            "asks:asset:550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate_price_levels(&[], 20);
        assert!(result.is_empty());
    }

    #[test]
    fn test_aggregate_single_order() {
        let id = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let member = format!("order:{}:{}:10:1700000000", id, uid);
        let raw = vec![(member, 10500.0)];

        let levels = aggregate_price_levels(&raw, 20);
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].price_cents, 10500);
        assert_eq!(levels[0].total_quantity, 10);
        assert_eq!(levels[0].order_count, 1);
    }

    #[test]
    fn test_aggregate_same_price_multiple_orders() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let uid1 = Uuid::new_v4();
        let uid2 = Uuid::new_v4();

        let raw = vec![
            (format!("order:{}:{}:10:1700000000", id1, uid1), 10500.0),
            (format!("order:{}:{}:20:1700000001", id2, uid2), 10500.0),
        ];

        let levels = aggregate_price_levels(&raw, 20);
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0].price_cents, 10500);
        assert_eq!(levels[0].total_quantity, 30); // 10 + 20
        assert_eq!(levels[0].order_count, 2);
    }

    #[test]
    fn test_aggregate_different_prices() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let uids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();

        let raw = vec![
            (
                format!("order:{}:{}:10:1700000000", ids[0], uids[0]),
                10000.0,
            ),
            (
                format!("order:{}:{}:5:1700000001", ids[1], uids[1]),
                10500.0,
            ),
            (
                format!("order:{}:{}:15:1700000002", ids[2], uids[2]),
                11000.0,
            ),
        ];

        let levels = aggregate_price_levels(&raw, 20);
        assert_eq!(levels.len(), 3);
        // BTreeMap sorts by key (price) ascending
        assert_eq!(levels[0].price_cents, 10000);
        assert_eq!(levels[1].price_cents, 10500);
        assert_eq!(levels[2].price_cents, 11000);
    }

    #[test]
    fn test_aggregate_respects_max_levels() {
        let mut raw = Vec::new();
        for i in 0..10 {
            let id = Uuid::new_v4();
            let uid = Uuid::new_v4();
            raw.push((
                format!("order:{}:{}:5:1700000000", id, uid),
                (10000 + i * 100) as f64,
            ));
        }

        let levels = aggregate_price_levels(&raw, 3);
        assert_eq!(levels.len(), 3);
    }

    #[test]
    fn test_aggregate_skips_invalid_members() {
        let raw = vec![
            ("invalid_format".to_string(), 10000.0),
            ("also:invalid".to_string(), 10500.0),
        ];

        let levels = aggregate_price_levels(&raw, 20);
        assert!(levels.is_empty());
    }
}
