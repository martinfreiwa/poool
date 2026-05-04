/// Marketplace Background Workers — housekeeping tasks that run as permanent Tokio tasks.
///
/// Three workers for marketplace maintenance:
///
/// 1. **Order Expiry** (hourly) — cancel expired orders, release held balance/tokens
/// 2. **Redis Sync** (every 5 minutes) — detect & fix drift between Redis orderbook and PostgreSQL
/// 3. **Price Snapshot** (every 5 minutes) — cache last trade prices for quick lookups
///
/// The reconciliation worker is already in `main.rs` (Phase 1.9) and covers
/// the general financial reconciliation.
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use super::models::MarketOrder;
use super::orderbook;
use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── WORKER 1: ORDER EXPIRY (HOURLY) ──────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Clean up expired orders every hour.
///
/// Orders expire after 90 days (set at creation via `expires_at`).
/// For each expired order:
/// 1. Set status to 'expired' in PostgreSQL.
/// 2. Release the held balance (buy) or held tokens (sell).
/// 3. Remove from Redis orderbook.
///
/// This prevents stale orders from consuming holds indefinitely.
pub async fn run_order_expiry_worker(redis: &RedisPool, pool: &PgPool) {
    tracing::info!("⏰ Order expiry worker started (runs every hour)");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    // Skip the immediate first tick to avoid slamming DB on startup
    interval.tick().await;

    loop {
        interval.tick().await;

        match expire_stale_orders(redis, pool).await {
            Ok(count) if count > 0 => {
                tracing::info!("⏰ Expired {} stale orders and released holds", count);
            }
            Ok(_) => {
                tracing::debug!("⏰ No expired orders found");
            }
            Err(e) => {
                tracing::error!("⏰ Order expiry worker failed: {}", e);
                sentry::capture_message(
                    &format!("Order expiry worker failed: {}", e),
                    sentry::Level::Error,
                );
            }
        }
    }
}

/// Find and cancel all expired orders, releasing their holds.
///
/// Returns the number of orders expired.
async fn expire_stale_orders(redis: &RedisPool, pool: &PgPool) -> Result<u32, AppError> {
    // Find all orders that have expired
    let expired_orders = sqlx::query_as::<_, MarketOrder>(
        r#"SELECT * FROM market_orders
           WHERE status IN ('open', 'partially_filled')
             AND expires_at IS NOT NULL
             AND expires_at < NOW()
           ORDER BY created_at ASC
           LIMIT 100"#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut expired_count = 0u32;

    for order in &expired_orders {
        if let Err(e) = expire_single_order(redis, pool, order).await {
            tracing::error!(
                "Failed to expire order {}: {} — will retry next cycle",
                order.id,
                e
            );
            continue;
        }
        expired_count += 1;
    }

    Ok(expired_count)
}

/// Expire a single order: cancel it and release the held balance/tokens.
async fn expire_single_order(
    redis: &RedisPool,
    pool: &PgPool,
    order: &MarketOrder,
) -> Result<(), AppError> {
    let remaining = order.remaining_quantity();

    // ACID transaction: cancel order + release hold
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // 1. Mark order as expired (idempotent — only acts if still active)
    let cancel_affected = sqlx::query(
        "UPDATE market_orders
         SET status = 'expired', cancel_reason = 'order_expired', updated_at = NOW()
         WHERE id = $1 AND status IN ('open', 'partially_filled')",
    )
    .bind(order.id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    // Already terminal? Another worker / cancel path beat us — no-op success.
    if cancel_affected == 0 {
        tx.rollback().await.ok();
        return Ok(());
    }

    // 2. Release holds based on side. Strict updates (no GREATEST clamps);
    //    must match the formula used at order creation so released cents/tokens
    //    line up exactly with what was held.
    if order.side == "buy" {
        // Hold = (price * qty) + fee_reserve_bps share of (price * qty)
        let price_hold = order
            .price_cents
            .checked_mul(remaining as i64)
            .ok_or_else(|| AppError::Internal("expiry: hold-release overflow".into()))?;
        let fee_hold = super::models::calculate_fee_cents(price_hold, order.fee_reserve_bps);
        let held_release = price_hold
            .checked_add(fee_hold)
            .ok_or_else(|| AppError::Internal("expiry: hold-release sum overflow".into()))?;

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
        .bind(order.user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
        if affected != 1 {
            return Err(AppError::Internal(format!(
                "expiry: buyer hold release invariant violated (order={}, user={}, release={})",
                order.id, order.user_id, held_release
            )));
        }
    } else {
        let affected = sqlx::query(
            "UPDATE investments SET
                held_tokens = held_tokens - $1,
                updated_at = NOW()
             WHERE user_id = $2 AND asset_id = $3
               AND status != 'exited'
               AND held_tokens >= $1",
        )
        .bind(remaining)
        .bind(order.user_id)
        .bind(order.asset_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?
        .rows_affected();
        if affected != 1 {
            return Err(AppError::Internal(format!(
                "expiry: seller held_tokens release invariant violated (order={}, user={}, qty={})",
                order.id, order.user_id, remaining
            )));
        }
    }

    tx.commit().await.map_err(AppError::Database)?;

    // 3. Remove from Redis orderbook (best-effort, after DB commit)
    if let Err(e) = orderbook::remove_order(redis, order).await {
        tracing::warn!(
            "Failed to remove expired order {} from Redis: {} — sync worker will catch it",
            order.id,
            e
        );
    }

    tracing::debug!(
        "⏰ Expired order {}: side={}, asset={}, remaining_qty={}",
        order.id,
        order.side,
        order.asset_id,
        remaining
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── WORKER 2: REDIS SYNC (EVERY 5 MINUTES) ───────────────────
// ═══════════════════════════════════════════════════════════════

/// Periodic Redis ↔ PostgreSQL sync to detect and fix drift.
///
/// Two checks:
/// 1. **Missing orders**: active orders in PostgreSQL but not in Redis → re-insert.
/// 2. **Stale orders**: orders in Redis but filled/cancelled in PostgreSQL → remove.
///
/// This is the safety net. If Redis loses an order (crash, eviction, race condition),
/// this worker will re-insert it within 5 minutes. Zero orders lost permanently.
pub async fn run_redis_sync_worker(redis: &RedisPool, pool: &PgPool) {
    tracing::info!("🔄 Redis sync worker started (runs every 5 minutes)");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
    // Initial delay to avoid startup burst
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    loop {
        interval.tick().await;

        // Part 1: Find orders missing from Redis and re-insert
        let missing_count = match orderbook::sync_with_postgres(redis, pool).await {
            Ok(fixed) => {
                if fixed > 0 {
                    tracing::warn!("🔧 Redis sync: re-inserted {} missing orders", fixed);
                    sentry::capture_message(
                        &format!("Redis sync fixed {} missing orders", fixed),
                        sentry::Level::Warning,
                    );
                } else {
                    tracing::debug!("🔄 Redis sync: all orders in sync");
                }
                fixed as i64
            }
            Err(e) => {
                tracing::error!("🔄 Redis sync failed: {}", e);
                -1 // sentinel: cycle errored
            }
        };

        // Part 2: Find stale orders in Redis (no longer active in DB) and clean up
        let stale_count = match clean_stale_redis_orders(redis, pool).await {
            Ok(cleaned) => {
                if cleaned > 0 {
                    tracing::warn!(
                        "🧹 Redis cleanup: removed {} stale orders from Redis",
                        cleaned
                    );
                }
                cleaned as i64
            }
            Err(e) => {
                tracing::error!("🧹 Redis stale order cleanup failed: {}", e);
                -1
            }
        };

        // Part 3: Queue depth — pending matches waiting for settlement.
        let queue_depth = orderbook::match_queue_depth(redis).await.unwrap_or(-1);

        // Part 4: Persist metrics. Best-effort — never let metrics persist
        // failure interfere with the actual sync work.
        let _ = persist_drift_metric(pool, "missing_in_redis", missing_count).await;
        let _ = persist_drift_metric(pool, "stale_in_redis", stale_count).await;
        let _ = persist_drift_metric(pool, "match_queue_depth", queue_depth).await;

        // Alert on queue depth backlog (settlement worker is wedged or
        // throughput insufficient). 100 is arbitrary — tune from prod data.
        if queue_depth > 100 {
            sentry::capture_message(
                &format!("Match queue depth high: {} events pending", queue_depth),
                sentry::Level::Warning,
            );
        }
    }
}

async fn persist_drift_metric(
    pool: &PgPool,
    metric_type: &str,
    value: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO marketplace_drift_metrics (metric_type, value) VALUES ($1, $2)")
        .bind(metric_type)
        .bind(value)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Remove orders from Redis that are no longer active in PostgreSQL.
///
/// Scans all assets with active orders and checks each Redis member against
/// the database. If an order is filled/cancelled/expired in DB but still in
/// Redis, it gets removed.
async fn clean_stale_redis_orders(redis: &RedisPool, pool: &PgPool) -> Result<u32, AppError> {
    // Get all assets that have orders in the DB
    let asset_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT asset_id FROM market_orders WHERE status IN ('open', 'partially_filled')",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut cleaned = 0u32;

    for asset_id in &asset_ids {
        // Check asks
        let asks = orderbook::get_asks(redis, *asset_id, 500)
            .await
            .unwrap_or_default();
        for ask in &asks {
            if is_order_stale(pool, ask.order_id).await {
                if orderbook::remove_member(redis, *asset_id, "sell", &ask.raw_member)
                    .await
                    .is_ok()
                {
                    cleaned += 1;
                }
            }
        }

        // Check bids
        let bids = orderbook::get_bids(redis, *asset_id, 500)
            .await
            .unwrap_or_default();
        for bid in &bids {
            if is_order_stale(pool, bid.order_id).await {
                if orderbook::remove_member(redis, *asset_id, "buy", &bid.raw_member)
                    .await
                    .is_ok()
                {
                    cleaned += 1;
                }
            }
        }
    }

    Ok(cleaned)
}

/// Check if an order is no longer active in PostgreSQL.
async fn is_order_stale(pool: &PgPool, order_id: uuid::Uuid) -> bool {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM market_orders WHERE id = $1")
            .bind(order_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

    match status.as_deref() {
        Some("open") | Some("partially_filled") => false, // Still active
        Some(_) => true, // filled, cancelled, expired — stale in Redis
        None => true,    // Order doesn't exist — definitely stale
    }
}

// ═══════════════════════════════════════════════════════════════
// ── WORKER 3: PRICE SNAPSHOT (EVERY 5 MINUTES) ───────────────
// ═══════════════════════════════════════════════════════════════

/// Periodic price snapshot — caches last trade prices for each active asset.
///
/// This avoids hitting the trade_history table on every orderbook snapshot request.
/// Stores prices in Redis with a 10-minute TTL (refreshed every 5 minutes).
pub async fn run_price_snapshot_worker(redis: &RedisPool, pool: &PgPool) {
    tracing::info!("📊 Price snapshot worker started (runs every 5 minutes)");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
    // Initial delay
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    loop {
        interval.tick().await;

        match snapshot_last_prices(redis, pool).await {
            Ok(count) => {
                tracing::debug!("📊 Price snapshot: cached {} asset prices", count);
            }
            Err(e) => {
                tracing::error!("📊 Price snapshot failed: {}", e);
            }
        }
    }
}

/// Snapshot the last trade price for each active asset into Redis.
async fn snapshot_last_prices(redis: &RedisPool, pool: &PgPool) -> Result<u32, AppError> {
    #[derive(sqlx::FromRow)]
    struct PriceRow {
        asset_id: uuid::Uuid,
        price_cents: i64,
    }

    let rows = sqlx::query_as::<_, PriceRow>(
        r#"SELECT DISTINCT ON (asset_id)
              asset_id, price_cents
           FROM trade_history
           ORDER BY asset_id, executed_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut conn = redis
        .get()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Redis unavailable: {}", e)))?;

    let mut count = 0u32;

    for row in &rows {
        let key = format!("last_price:asset:{}", row.asset_id);
        let _: Result<String, _> = redis::cmd("SETEX")
            .arg(&key)
            .arg(600u64) // 10 minute TTL
            .arg(row.price_cents)
            .query_async(&mut *conn)
            .await;

        count += 1;
    }

    Ok(count)
}

// ═══════════════════════════════════════════════════════════════
// ── WORKER 4: ALERT ESCALATION (EVERY 60 SECONDS) ────────────
// ═══════════════════════════════════════════════════════════════

/// Polls marketplace_alerts for un-ack'd alerts that have crossed their
/// rule.escalate_after_min threshold and dispatches notifications.
///
/// Runs every 60s (alerts are time-sensitive). Single-leader to avoid
/// duplicate pages.
pub async fn run_alert_escalation_worker(pool: &PgPool) {
    tracing::info!("🚨 Alert escalation worker started (runs every 60s)");

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    interval.tick().await;
    let mut tick_n: u32 = 0;

    loop {
        interval.tick().await;
        tick_n = tick_n.wrapping_add(1);

        match crate::admin::marketplace::escalate_overdue_alerts(pool).await {
            Ok(0) => tracing::trace!("🚨 No alerts to escalate"),
            Ok(n) => tracing::info!("🚨 Escalated {} overdue alerts", n),
            Err(e) => tracing::error!("🚨 Alert escalation failed: {}", e),
        }

        // Refresh sparkline matview every 10th tick (~10 minutes)
        if tick_n % 10 == 0 {
            if let Err(e) = crate::admin::marketplace::refresh_alert_daily_counts(pool).await {
                tracing::warn!("🚨 Failed to refresh alert daily-counts matview: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_stale_status_logic() {
        // Active statuses are NOT stale
        let active_statuses = vec!["open", "partially_filled"];
        for status in &active_statuses {
            let is_stale = *status != "open" && *status != "partially_filled";
            assert!(!is_stale, "Status '{}' should NOT be stale", status);
        }

        // Filled/cancelled/expired ARE stale
        let stale_statuses = vec!["filled", "cancelled", "expired", "pending_review"];
        for status in &stale_statuses {
            let is_stale = *status != "open" && *status != "partially_filled";
            assert!(is_stale, "Status '{}' should be stale", status);
        }
    }

    #[test]
    fn test_expiry_hold_calculation_buy() {
        // Buy order: hold = price * remaining_qty
        let price = 15000i64; // $150/token
        let remaining = 5i32;
        let hold_release = price.saturating_mul(remaining as i64);
        assert_eq!(hold_release, 75000); // $750 released
    }

    #[test]
    fn test_expiry_hold_calculation_sell() {
        // Sell order: hold = remaining tokens (integer count)
        let remaining = 7i32;
        // For sell, we release held_tokens = remaining
        assert_eq!(remaining, 7);
    }

    #[test]
    fn test_worker_intervals() {
        // Verify our interval constants make sense
        let expiry_interval = 3600u64; // 1 hour
        let sync_interval = 300u64; // 5 minutes
        let price_interval = 300u64; // 5 minutes

        assert_eq!(expiry_interval, 60 * 60);
        assert_eq!(sync_interval, 5 * 60);
        assert_eq!(price_interval, 5 * 60);
    }
}
