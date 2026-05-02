/// Marketplace service — core business logic for the trading engine.
///
/// Contains all state-mutating operations. Every function here can be tested
/// without HTTP by passing a `PgPool` and optional `RedisPool`.
///
/// RULES:
/// - All multi-table writes use `sqlx::Transaction`.
/// - All balance reads before writes use `SELECT ... FOR UPDATE`.
/// - All monetary values are `i64` cents.
/// - No `unwrap()` in production paths.
use chrono::Utc;
use deadpool_redis::Pool as RedisPool;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    MarketOrder, OrderResponse, OrderSide, OrderbookSnapshot, PriceLevel, RecentTrade,
    SubmitOrderRequest, TickerResponse,
};
use super::{orderbook, validation};
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize)]
struct MarketplaceRuntimeSettings {
    tick_size_cents: i64,
    min_order_size: i32,
    max_order_size: i32,
    trading_enabled: bool,
    maintenance_window: bool,
}

impl Default for MarketplaceRuntimeSettings {
    fn default() -> Self {
        Self {
            tick_size_cents: 5,
            min_order_size: 1,
            max_order_size: 10000,
            trading_enabled: true,
            maintenance_window: false,
        }
    }
}

async fn load_marketplace_runtime_settings(
    redis: Option<&RedisPool>,
) -> Result<MarketplaceRuntimeSettings, AppError> {
    let Some(redis) = redis else {
        return Ok(MarketplaceRuntimeSettings::default());
    };

    let mut conn = redis.get().await.map_err(|e| {
        AppError::ServiceUnavailable(format!("Marketplace settings unavailable: {}", e))
    })?;

    let json_str: Option<String> = redis::cmd("GET")
        .arg("marketplace:settings")
        .query_async(&mut *conn)
        .await
        .map_err(|e| {
            AppError::ServiceUnavailable(format!("Marketplace settings unavailable: {}", e))
        })?;

    let mut settings = match json_str {
        Some(raw) => serde_json::from_str::<MarketplaceRuntimeSettings>(&raw).map_err(|e| {
            AppError::ServiceUnavailable(format!("Marketplace settings are invalid: {}", e))
        })?,
        None => MarketplaceRuntimeSettings::default(),
    };

    let enabled: Option<String> = redis::cmd("GET")
        .arg("marketplace:trading_enabled")
        .query_async(&mut *conn)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Trading flag unavailable: {}", e)))?;
    if let Some(value) = enabled {
        settings.trading_enabled = value == "1" || value.eq_ignore_ascii_case("true");
    }

    Ok(settings)
}

fn validate_runtime_settings_for_order(
    settings: &MarketplaceRuntimeSettings,
    req: &SubmitOrderRequest,
    price_cents: i64,
    asset_tick_size_override: Option<i64>,
) -> Result<(), AppError> {
    if !settings.trading_enabled || settings.maintenance_window {
        return Err(AppError::TradingDisabled);
    }

    if req.quantity < settings.min_order_size {
        return Err(AppError::BadRequest(format!(
            "Order quantity must be at least {} tokens.",
            settings.min_order_size
        )));
    }

    if req.quantity > settings.max_order_size {
        return Err(AppError::BadRequest(format!(
            "Order quantity cannot exceed {} tokens.",
            settings.max_order_size
        )));
    }

    // Tick size only enforced on LIMIT orders. Market orders use whatever
    // best price exists in the orderbook — tick alignment isn't meaningful.
    if req.order_type == "limit" {
        let effective_tick = asset_tick_size_override.unwrap_or(settings.tick_size_cents);
        if effective_tick > 0 && price_cents % effective_tick != 0 {
            return Err(AppError::BadRequest(format!(
                "Limit price must be a multiple of {} cents.",
                effective_tick
            )));
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── ORDER CREATION ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Resolves an asset ID from either a UUID string or a URL slug.
pub async fn resolve_asset_id(pool: &PgPool, id_or_slug: &str) -> Result<Uuid, AppError> {
    if let Ok(uuid) = Uuid::parse_str(id_or_slug) {
        Ok(uuid)
    } else {
        let asset_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM assets WHERE slug = $1")
            .bind(id_or_slug)
            .fetch_optional(pool)
            .await
            .map_err(AppError::Database)?;
        asset_id.ok_or_else(|| AppError::NotFound("Asset not found".into()))
    }
}

/// Create a new market order.
///
/// This is the main entry point for order submission. It performs the full
/// validation pipeline and creates the order within an ACID transaction.
///
/// Steps:
/// 1. Check KYC verification
/// 2. Check asset is tradable
/// 3. Check open order count
/// 4. Check idempotency key
/// 5. Check for opposing orders (wash trade prevention)
/// 6. Begin DB transaction:
///    a. FOR UPDATE lock on wallet/investments
///    b. Balance/token check
///    c. Check admin review threshold
///    d. Insert order into market_orders
///    e. Place hold on balance (buy) or tokens (sell)
/// 7. Commit transaction
/// 8. Insert into Redis orderbook (best-effort)
pub async fn create_order(
    pool: &PgPool,
    redis: Option<&RedisPool>,
    user_id: Uuid,
    req: SubmitOrderRequest,
) -> Result<OrderResponse, AppError> {
    let side = OrderSide::parse(&req.side)
        .ok_or_else(|| AppError::BadRequest("Invalid order side.".into()))?;

    let asset_uuid = resolve_asset_id(pool, &req.asset_id).await?;

    // ── Pre-transaction validation (reads only) ─────────────
    validation::check_kyc_verified(pool, user_id)
        .await
        .map_err(|r| r.into_app_error())?;

    let total_tokens = validation::check_asset_tradable(pool, asset_uuid, user_id)
        .await
        .map_err(|r| r.into_app_error())?;

    validation::check_open_order_count(pool, user_id, asset_uuid)
        .await
        .map_err(|r| r.into_app_error())?;

    validation::check_idempotency_key(pool, user_id, &req.idempotency_key)
        .await
        .map_err(|r| r.into_app_error())?;

    // Self-trade-cross check happens AFTER price is resolved (below) so we
    // know the actual price for market orders.

    // ── Rate limiting (Redis, non-fatal) ─────────────────────
    if let Some(redis) = redis {
        if let Err(retry_after) = orderbook::check_order_rate_limit(redis, user_id, 10, 60).await {
            return Err(AppError::RateLimited(retry_after));
        }
    }

    let runtime_settings = load_marketplace_runtime_settings(redis).await?;

    // ── Determine price ──────────────────────────────────────
    let price_cents = match req.order_type.as_str() {
        "limit" => req
            .price_cents
            .ok_or_else(|| AppError::BadRequest("Price is required for limit orders.".into()))?,
        "market" => {
            // For market orders, use the best available price from the orderbook.
            // Priority: Redis (if available) -> Database (fallback)
            if let Some(redis) = redis {
                let best = match side {
                    OrderSide::Buy => orderbook::best_ask(redis, asset_uuid)
                        .await
                        .map(|o| o.map(|o| o.price_cents)),
                    OrderSide::Sell => orderbook::best_bid(redis, asset_uuid)
                        .await
                        .map(|o| o.map(|o| o.price_cents)),
                };

                match best {
                    Ok(Some(price)) => price,
                    Ok(None) => {
                        tracing::warn!(
                            "Redis best-price lookup returned no {} orders for asset {}. Falling back to Database.",
                            if side == OrderSide::Buy {
                                "sell"
                            } else {
                                "buy"
                            },
                            asset_uuid
                        );
                        get_best_price_from_db(pool, asset_uuid, side).await?
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Redis best-price lookup failed ({}). Falling back to Database.",
                            e
                        );
                        // Fall through to DB fallback
                        get_best_price_from_db(pool, asset_uuid, side).await?
                    }
                }
            } else {
                // No Redis configured — use database fallback
                get_best_price_from_db(pool, asset_uuid, side).await?
            }
        }
        _ => {
            return Err(AppError::BadRequest("Invalid order type.".into()));
        }
    };

    // Per-asset tick size override (NULL = fall back to platform default).
    let asset_tick_override: Option<i64> =
        sqlx::query_scalar("SELECT tick_size_cents::bigint FROM assets WHERE id = $1")
            .bind(asset_uuid)
            .fetch_optional(pool)
            .await
            .map_err(AppError::Database)?
            .flatten();

    validate_runtime_settings_for_order(&runtime_settings, &req, price_cents, asset_tick_override)?;

    validation::check_no_opposing_orders(pool, user_id, asset_uuid, &req.side, price_cents)
        .await
        .map_err(|r| r.into_app_error())?;

    // Use checked_mul — saturating_mul would silently clamp to i64::MAX on
    // overflow, giving an attacker a way to place an order whose stored
    // total is wildly smaller than the real amount owed.
    let order_total_cents = price_cents
        .checked_mul(req.quantity as i64)
        .ok_or_else(|| {
            AppError::BadRequest("Order total exceeds maximum supported value".into())
        })?;

    // Resolve fees once for this asset so we can reserve the buyer's fee
    // alongside the price hold. The buyer might end up as taker or maker —
    // we conservatively reserve the TAKER fee (the larger of the two).
    // Any unused portion is refunded at settlement.
    let resolved_fees = validation::resolve_fees(pool, asset_uuid).await?;
    let fee_reserve_bps: i32 = if side == OrderSide::Buy {
        resolved_fees
            .taker_fee_bps
            .max(resolved_fees.maker_fee_bps)
            .max(0)
    } else {
        0
    };
    let buyer_fee_reserve_cents: i64 =
        super::models::calculate_fee_cents(order_total_cents, fee_reserve_bps);
    let buyer_hold_total_cents = order_total_cents
        .checked_add(buyer_fee_reserve_cents)
        .ok_or_else(|| AppError::BadRequest("Order total + fee exceeds maximum".into()))?;

    // ── Check if admin review is needed ──────────────────────
    let requires_review =
        validation::check_admin_review_required(order_total_cents, req.quantity, total_tokens);

    let initial_status = if requires_review.is_some() {
        "pending_review"
    } else {
        "open"
    };

    // ── ACID Transaction: create order + place hold ──────────
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // Tx-scoped advisory lock keyed on (user_id, asset_id). Serialises
    // concurrent order placements by the same user on the same asset, so
    // the open-orders cap below can't be checked-then-fillable. The two
    // i32 args produce a 64-bit lock key; collisions across (user,asset)
    // pairs are rare and only cause brief serialisation, not correctness
    // issues. The lock auto-releases at tx commit/rollback.
    let user_lo = (user_id.as_u128() as u32) as i32;
    let asset_lo = (asset_uuid.as_u128() as u32) as i32;
    sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(user_lo)
        .bind(asset_lo)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

    // Re-check the open-orders cap inside the lock. The earlier
    // `check_open_order_count` was best-effort; this one is the real gate.
    let open_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM market_orders
         WHERE user_id = $1 AND asset_id = $2
           AND status IN ('open', 'partially_filled')",
    )
    .bind(user_id)
    .bind(asset_uuid)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;
    if open_count >= validation::max_open_orders_per_asset() as i64 {
        return Err(super::models::OrderRejection::TooManyOpenOrders {
            max: validation::max_open_orders_per_asset(),
            current: open_count as i32,
        }
        .into_app_error());
    }

    // Balance/token check inside transaction with FOR UPDATE.
    // Concentration check (buy side only) is also done inside the tx so the
    // (current_owned + held) snapshot can't change between the check and
    // the hold placement (H7 — closes concurrent-buy race window).
    match side {
        OrderSide::Buy => {
            validation::check_buyer_balance(&mut tx, user_id, buyer_hold_total_cents).await?;
            validation::check_concentration_limit_tx(
                &mut tx,
                user_id,
                asset_uuid,
                req.quantity,
                total_tokens,
            )
            .await?;
        }
        OrderSide::Sell => {
            validation::check_seller_tokens(&mut tx, user_id, asset_uuid, req.quantity).await?;
        }
    }

    // Parse idempotency key
    let idempotency_uuid = Uuid::parse_str(&req.idempotency_key)
        .map_err(|_| AppError::BadRequest("Invalid idempotency_key.".into()))?;

    // Default expiry: 90 days from now
    let expires_at = Utc::now() + chrono::Duration::days(90);

    // Validate + normalise time-in-force
    let tif = req
        .time_in_force
        .as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "gtc".to_string());
    if !matches!(tif.as_str(), "gtc" | "ioc") {
        return Err(AppError::BadRequest(format!(
            "Unsupported time_in_force: {} (allowed: gtc, ioc)",
            tif
        )));
    }

    // Insert order
    let order = sqlx::query_as::<_, MarketOrder>(
        r#"INSERT INTO market_orders
           (user_id, asset_id, side, order_type, price_cents, quantity, status,
            idempotency_key, expires_at, fee_reserve_bps, time_in_force)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING *"#,
    )
    .bind(user_id)
    .bind(asset_uuid)
    .bind(side.as_str())
    .bind(&req.order_type)
    .bind(price_cents)
    .bind(req.quantity)
    .bind(initial_status)
    .bind(idempotency_uuid)
    .bind(expires_at)
    .bind(fee_reserve_bps)
    .bind(&tif)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        // Check for unique constraint violation on idempotency_key
        if e.to_string().contains("idempotency_key") {
            return AppError::OrderRejected("This order has already been submitted.".into());
        }
        AppError::Database(e)
    })?;

    // Place hold on balance (buy) or tokens (sell).
    // Buyer hold includes the price total + reserved taker fee — settlement
    // refunds any unused portion of the fee reserve back to free balance.
    match side {
        OrderSide::Buy => {
            sqlx::query(
                "UPDATE wallets SET held_balance_cents = held_balance_cents + $1, updated_at = NOW()
                 WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'",
            )
            .bind(buyer_hold_total_cents)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }
        OrderSide::Sell => {
            sqlx::query(
                "UPDATE investments SET held_tokens = held_tokens + $1, updated_at = NOW()
                 WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
            )
            .bind(req.quantity)
            .bind(user_id)
            .bind(asset_uuid)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }
    }

    // Commit transaction
    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        "📝 Order created: id={}, user={}, side={}, asset={}, price={}, qty={}, status={}",
        order.id,
        user_id,
        side,
        asset_uuid,
        price_cents,
        req.quantity,
        initial_status
    );

    // ── Insert into Redis orderbook (best-effort, after DB commit) ─
    if initial_status == "open" {
        if let Some(redis) = redis {
            if let Err(e) = orderbook::insert_order(redis, &order).await {
                // Non-fatal: the 5-minute sync worker will catch this
                tracing::error!(
                    "Failed to insert order {} into Redis orderbook: {} — will be caught by sync worker",
                    order.id,
                    e
                );
            } else {
                // Real-time broadcast: orderbook has changed with new order
                super::websocket::broadcast_orderbook_update(pool, Some(redis), order.asset_id)
                    .await;
            }
        }
    }

    // Build response
    let message = if initial_status == "pending_review" {
        "Your order has been submitted for admin review.".to_string()
    } else {
        "Order placed successfully.".to_string()
    };

    Ok(OrderResponse {
        order_id: order.id,
        status: initial_status.to_string(),
        message,
        immediate_fill: None, // Matching engine handles this (Task 3.6)
    })
}

// ═══════════════════════════════════════════════════════════════
// ── ORDER CANCELLATION ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Cancel an open order and release the held balance/tokens.
///
/// Uses a Redis lock to prevent cancel-during-match race conditions.
pub async fn cancel_order(
    pool: &PgPool,
    redis: Option<&RedisPool>,
    user_id: Uuid,
    order_id: Uuid,
) -> Result<(), AppError> {
    // 1. Try to acquire a lock (5s TTL)
    if let Some(redis) = redis {
        let locked = orderbook::try_lock_order(redis, order_id, 5).await?;
        if !locked {
            return Err(AppError::Conflict(
                "Order is being processed. Please try again.".into(),
            ));
        }
    }

    // 2. Open the cancellation tx FIRST, then re-fetch the order with
    //    `FOR UPDATE` so we read the latest filled-quantity even if a
    //    settlement is racing us. The previous version fetched outside
    //    the tx (autocommit, no row lock) and computed `remaining` from
    //    a stale snapshot — when settlement filled some quantity in the
    //    race window, the hold-release amount drifted from reality and
    //    the entire cancel would error out.
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let order = sqlx::query_as::<_, MarketOrder>(
        "SELECT * FROM market_orders WHERE id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(order_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Order not found.".into()))?;

    // 3. Re-check cancellability inside the lock. Tx auto-rolls back when
    //    `tx` drops below.
    if !order.is_active() {
        if let Some(redis) = redis {
            let _ = orderbook::release_lock(redis, order_id).await;
        }
        return Err(AppError::BadRequest(format!(
            "Cannot cancel order in '{}' status.",
            order.status
        )));
    }

    let remaining = order.remaining_quantity();

    // Cancel the order. Strict status guard: rules out flipping a row
    // that's already been finalised by another path (defence in depth —
    // FOR UPDATE above should make this impossible).
    let cancel_affected = sqlx::query(
        "UPDATE market_orders SET status = 'cancelled', cancel_reason = 'user_cancelled', updated_at = NOW()
         WHERE id = $1 AND status IN ('open', 'partially_filled')",
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .rows_affected();
    if cancel_affected != 1 {
        return Err(AppError::Conflict(
            "Order was modified during cancellation; please retry.".into(),
        ));
    }

    // Release the hold
    let side = OrderSide::parse(&order.side);
    match side {
        Some(OrderSide::Buy) => {
            // Release price-hold + the unused fee reserve for the
            // remaining quantity. Mirrors the formula used at creation:
            //   hold = price*qty + fee_bps_share(price*qty)
            let price_hold = order
                .price_cents
                .checked_mul(remaining as i64)
                .ok_or_else(|| AppError::Internal("Hold-release overflow on cancel".into()))?;
            let fee_hold = super::models::calculate_fee_cents(price_hold, order.fee_reserve_bps);
            let held_release = price_hold
                .checked_add(fee_hold)
                .ok_or_else(|| AppError::Internal("Hold-release sum overflow on cancel".into()))?;
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
            .map_err(AppError::Database)?
            .rows_affected();
            if affected != 1 {
                return Err(AppError::Internal(format!(
                    "Buyer hold release invariant violated on cancel (user={}, release={})",
                    user_id, held_release
                )));
            }
        }
        Some(OrderSide::Sell) => {
            let affected = sqlx::query(
                "UPDATE investments SET
                    held_tokens = held_tokens - $1,
                    updated_at = NOW()
                 WHERE user_id = $2 AND asset_id = $3
                   AND status != 'exited'
                   AND held_tokens >= $1",
            )
            .bind(remaining)
            .bind(user_id)
            .bind(order.asset_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?
            .rows_affected();
            if affected != 1 {
                return Err(AppError::Internal(format!(
                    "Seller held_tokens release invariant violated on cancel (user={}, asset={}, qty={})",
                    user_id, order.asset_id, remaining
                )));
            }
        }
        None => {
            tracing::error!("Invalid side '{}' on order {}", order.side, order_id);
        }
    }

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        "🚫 Order cancelled: id={}, user={}, side={}, released_qty={}",
        order_id,
        user_id,
        order.side,
        remaining
    );

    // 5. Remove from Redis orderbook (best-effort, after DB commit)
    if let Some(redis) = redis {
        if let Err(e) = orderbook::remove_order(redis, &order).await {
            tracing::error!(
                "Failed to remove cancelled order {} from Redis: {} — will be caught by sync worker",
                order_id,
                e
            );
        } else {
            // Real-time broadcast: orderbook has changed after removal
            super::websocket::broadcast_orderbook_update(pool, Some(redis), order.asset_id).await;
        }
        // Release the lock
        let _ = orderbook::release_lock(redis, order_id).await;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── READ OPERATIONS ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Maximum page size for paginated user-facing lists. Higher values
/// require more memory in the DB driver; lower values mean more API calls.
pub const MAX_PAGE_SIZE: i64 = 200;
pub const DEFAULT_PAGE_SIZE: i64 = 50;

/// Clamp a user-supplied limit to a safe range.
pub fn clamp_page_size(req: Option<i64>) -> i64 {
    req.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE)
}

/// Get the user's orders, paginated by `(created_at, id)` cursor.
///
/// Pass `before` to fetch the next page (orders strictly older than the
/// cursor). `limit` is clamped to [1, MAX_PAGE_SIZE]. Pass `None`/`None` for
/// the first page (most recent).
pub async fn get_user_orders(
    pool: &PgPool,
    user_id: Uuid,
    before: Option<chrono::DateTime<chrono::Utc>>,
    limit: Option<i64>,
) -> Result<Vec<super::models::MyOrderResponse>, AppError> {
    let limit = clamp_page_size(limit);
    let orders = sqlx::query!(
        r#"SELECT
            m.id, m.asset_id, a.title as asset_name, m.side, m.price_cents,
            m.quantity, m.quantity_filled, m.status, m.created_at
           FROM market_orders m
           JOIN assets a ON m.asset_id = a.id
           WHERE m.user_id = $1
             AND ($2::timestamptz IS NULL OR m.created_at < $2)
           ORDER BY m.created_at DESC
           LIMIT $3"#,
        user_id,
        before,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    // Cache resolved fees per asset so we don't call resolve_fees() once
    // per order. Most users have orders concentrated in a few assets.
    let mut fee_cache: std::collections::HashMap<Uuid, i32> = std::collections::HashMap::new();

    let mut result = Vec::new();
    for o in orders {
        let total = o.price_cents.saturating_mul(o.quantity as i64);
        let bps = match fee_cache.get(&o.asset_id) {
            Some(b) => *b,
            None => {
                let resolved = validation::resolve_fees(pool, o.asset_id)
                    .await
                    .map(|r| r.taker_fee_bps)
                    .unwrap_or(500);
                fee_cache.insert(o.asset_id, resolved);
                resolved
            }
        };
        let fee = super::models::calculate_fee_cents(total, bps);
        result.push(super::models::MyOrderResponse {
            id: o.id.to_string(),
            asset: o.asset_name,
            asset_id: o.asset_id,
            side: o.side,
            price_cents: o.price_cents,
            qty: o.quantity,
            filled: o.quantity_filled,
            fee: fee,
            status: o.status,
            created_at: o.created_at,
        });
    }

    Ok(result)
}

/// Get recent trades for an asset (for the trade tape).
pub async fn get_recent_trades(
    pool: &PgPool,
    asset_id: Uuid,
    limit: i64,
) -> Result<Vec<RecentTrade>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT id, price_cents, quantity, 
                  (price_cents * quantity) as "total_cents!",
                  executed_at
           FROM trade_history
           WHERE asset_id = $1
           ORDER BY executed_at DESC
           LIMIT $2"#,
        asset_id,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows
        .into_iter()
        .map(|r| RecentTrade {
            id: r.id,
            price_cents: r.price_cents,
            quantity: r.quantity,
            total_cents: r.total_cents,
            executed_at: r.executed_at,
            is_buyer_maker: false, // We just mock this for now
        })
        .collect())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyTradeResponse {
    pub id: Uuid,
    pub date: chrono::DateTime<Utc>,
    pub asset: String,
    pub side: String,
    pub price: i64,
    pub qty: i32,
    pub total: i64,
    pub fee: i64,
    pub net: i64,
    pub pl: Option<i64>,
}

/// Get trade history for a specific user, paginated by `executed_at`.
///
/// Pass `before` for the next page (trades strictly older than the cursor).
/// `limit` is clamped to [1, MAX_PAGE_SIZE].
pub async fn get_user_trades_history(
    pool: &PgPool,
    user_id: Uuid,
    before: Option<chrono::DateTime<chrono::Utc>>,
    limit: Option<i64>,
) -> Result<Vec<MyTradeResponse>, AppError> {
    let limit = clamp_page_size(limit);
    let raw = sqlx::query!(
        r#"
        SELECT
            t.id,
            t.executed_at,
            a.title as asset_name,
            t.buyer_user_id,
            t.seller_user_id,
            t.price_cents,
            t.quantity,
            t.fee_cents,
            t.buyer_fee_cents,
            t.seller_fee_cents
        FROM trade_history t
        JOIN assets a ON t.asset_id = a.id
        WHERE (t.buyer_user_id = $1 OR t.seller_user_id = $1)
          AND ($2::timestamptz IS NULL OR t.executed_at < $2)
        ORDER BY t.executed_at DESC
        LIMIT $3
        "#,
        user_id,
        before,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut out = Vec::new();
    for r in raw {
        let is_buyer = r.buyer_user_id == user_id;
        let side = if is_buyer { "buy" } else { "sell" };
        let total = r.price_cents.saturating_mul(r.quantity as i64);

        // Show the fee actually charged to THIS user (per-side after maker/taker
        // split). Backfilled rows pre-migration-095 carry the legacy `fee_cents`
        // on the seller side only; the new columns default to 0 for those, so
        // we fall back to legacy when both sides are zero.
        let user_fee = if is_buyer {
            r.buyer_fee_cents
        } else {
            r.seller_fee_cents
        };
        let fee = if user_fee > 0 { user_fee } else { r.fee_cents };

        // Buyer cash out = price × qty + buyer fee. Seller cash in = price × qty - seller fee.
        let net = if is_buyer { total + fee } else { total - fee };

        // P&L requires the buyer's cost basis for the same asset, which we
        // don't compute in this query. Return None instead of a fake value.
        // TODO: implement real P&L using weighted-avg cost basis (see
        // /Users/martin/Projects/poool/docs/MASTERPLAN.md §2.13 Steuer-Report).
        let pl: Option<i64> = None;

        out.push(MyTradeResponse {
            id: r.id,
            date: r.executed_at,
            asset: r.asset_name,
            side: side.to_string(),
            price: r.price_cents,
            qty: r.quantity,
            total,
            fee,
            net,
            pl,
        });
    }

    Ok(out)
}

/// Get 24-hour ticker data for an asset.
pub async fn get_ticker(pool: &PgPool, asset_id: Uuid) -> Result<TickerResponse, AppError> {
    let now = Utc::now();
    let twenty_four_hours_ago = now - chrono::Duration::hours(24);

    let stats = sqlx::query!(
        r#"SELECT
            COUNT(*)::bigint as "trade_count!",
            COALESCE(SUM(quantity), 0)::bigint as "volume_tokens!",
            COALESCE(SUM(price_cents * quantity), 0)::bigint as "volume_cents!",
            MAX(price_cents) as high_cents,
            MIN(price_cents) as low_cents
           FROM trade_history
           WHERE asset_id = $1 AND executed_at >= $2"#,
        asset_id,
        twenty_four_hours_ago
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    // Last trade price
    let last_price: Option<i64> = sqlx::query_scalar(
        "SELECT price_cents FROM trade_history WHERE asset_id = $1 ORDER BY executed_at DESC LIMIT 1",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    // Price 24h ago (the first trade within the window, or the last trade before the window)
    let price_24h_ago: Option<i64> = sqlx::query_scalar(
        r#"SELECT price_cents FROM trade_history
           WHERE asset_id = $1 AND executed_at <= $2
           ORDER BY executed_at DESC
           LIMIT 1"#,
    )
    .bind(asset_id)
    .bind(twenty_four_hours_ago)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    let change_cents = match (last_price, price_24h_ago) {
        (Some(last), Some(ago)) => last - ago,
        _ => 0,
    };

    let change_pct = match (last_price, price_24h_ago) {
        (Some(_), Some(ago)) if ago > 0 => (change_cents as f64 / ago as f64) * 100.0,
        _ => 0.0,
    };

    Ok(TickerResponse {
        asset_id,
        last_price_cents: last_price,
        change_24h_cents: change_cents,
        change_24h_pct: change_pct,
        high_24h_cents: stats.high_cents,
        low_24h_cents: stats.low_cents,
        volume_24h_tokens: stats.volume_tokens,
        volume_24h_cents: stats.volume_cents,
        trade_count_24h: stats.trade_count,
    })
}

// ═══════════════════════════════════════════════════════════════
// ── FEE CALCULATION ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Look up the applicable fees for a trade on a given asset and calculate the fee amount.
///
/// Returns `(fee_cents, fee_bps)` for the given trade total.
pub async fn calculate_trade_fee(
    pool: &PgPool,
    asset_id: Uuid,
    total_cents: i64,
    is_taker: bool,
) -> Result<(i64, i32), AppError> {
    let fees = validation::resolve_fees(pool, asset_id).await?;

    let bps = if is_taker {
        fees.taker_fee_bps
    } else {
        fees.maker_fee_bps
    };

    let fee_cents = super::models::calculate_fee_cents(total_cents, bps);

    Ok((fee_cents, bps))
}

pub async fn get_secondary_assets(
    pool: &PgPool,
) -> Result<Vec<super::models::SecondaryAsset>, AppError> {
    let raw_assets = sqlx::query!(
        r#"
        SELECT
            a.id,
            a.slug,
            a.title,
            a.asset_type,
            COALESCE(a.location_city, '') as "location_city!",
            COALESCE(a.location_country, '') as "location_country!",
            a.token_price_cents,
            a.tokens_total,
            a.annual_yield_bps,
            a.description,
            a.total_value_cents,
            a.tokens_available,
            a.land_size_sqm,
            a.building_size_sqm,
            a.bedrooms,
            a.bathrooms,
            a.funding_status,
            a.location_description,
            a.occupancy_rate_bps,
            a.lease_type,
            a.property_type,
            a.term_months,
            a.capital_appreciation_bps,
            ARRAY(
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls!"
        FROM assets a
        WHERE a.published = true
          AND a.asset_type != 'commodity'
          -- Only include assets that are actually TRADABLE on the secondary
          -- market. `funding_in_progress` / `funding_open` belong on the
          -- PRIMARY listing pages — surfacing them here led to users
          -- clicking "Trade" only to be rejected by `check_asset_tradable`.
          AND a.funding_status = 'funded'
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut results = Vec::new();
    for row in raw_assets {
        let stats = super::charts::get_chart_summary(pool, row.id).await.ok();
        let price = stats
            .as_ref()
            .and_then(|s| s.last_price_cents)
            .unwrap_or(row.token_price_cents);
        let change24h = stats.as_ref().and_then(|s| s.change_24h_pct).unwrap_or(0.0);
        let volume24h = stats.as_ref().and_then(|s| s.volume_24h).unwrap_or(0);

        let sell_orders: i64 = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(quantity - quantity_filled), 0)::bigint FROM market_orders WHERE asset_id = $1 AND side = 'sell' AND status IN ('open', 'partially_filled')",
            row.id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

        let buy_interest: i64 = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(quantity - quantity_filled), 0)::bigint FROM market_orders WHERE asset_id = $1 AND side = 'buy' AND status IN ('open', 'partially_filled')",
            row.id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

        let candles_query = super::charts::CandleQuery {
            interval: Some("1d".into()),
            from: Some(Utc::now() - chrono::Duration::days(365)),
            to: Some(Utc::now()),
            limit: Some(365),
        };
        let sparkline = super::charts::get_candles(pool, row.id, candles_query)
            .await
            .map(|resp| {
                resp.candles
                    .into_iter()
                    .map(|c| c.close_cents as f64 / 100.0)
                    .collect::<Vec<f64>>()
            })
            .unwrap_or_default();

        let processed_images = row
            .image_urls
            .into_iter()
            .map(|url| crate::storage::service::rewrite_gcs_url(&url))
            .collect();
        let funding_progress_pct = if row.tokens_total > 0 {
            (((row.tokens_total - row.tokens_available) as f64 / row.tokens_total as f64) * 100.0)
                .clamp(0.0, 100.0)
        } else {
            0.0
        };

        results.push(super::models::SecondaryAsset {
            id: row.id.to_string(),
            slug: row.slug,
            name: row.title,
            r#type: row.asset_type,
            location: format!("{}, {}", row.location_city, row.location_country),
            country: row.location_country,
            images: processed_images,
            price,
            change24h,
            volume24h,
            roi: row.annual_yield_bps.unwrap_or(0) as f64 / 100.0,
            occupancy: (row.occupancy_rate_bps.unwrap_or(0) / 100) as i32,
            sell_orders,
            buy_interest,
            total_supply: row.tokens_total,
            sparkline,
            description: row.description,
            property_value: row.total_value_cents,
            land_size: row.land_size_sqm.map(|s| format!("{:.0} m²", s)),
            building_size_sqm: row.building_size_sqm.map(|s| format!("{:.0} m²", s)),
            bedrooms: row.bedrooms,
            bathrooms: row.bathrooms,
            rent_status: Some(row.funding_status.clone()),
            location_desc: row.location_description,
            lease_type: row.lease_type,
            property_type: row.property_type,
            funding_status: row.funding_status,
            tokens_available: row.tokens_available,
            funding_progress_pct,
            term_months: row.term_months,
            capital_appreciation_bps: row.capital_appreciation_bps,
        });
    }

    Ok(results)
}

// ═══════════════════════════════════════════════════════════════
// ── ORDERBOOK FALLBACK (POSTGRESQL) ───────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Build a full orderbook snapshot directly from PostgreSQL.
/// Used as a fallback when Redis is unavailable.
pub async fn get_orderbook_snapshot_from_db(
    pool: &PgPool,
    asset_id: Uuid,
    depth: Option<usize>,
) -> Result<OrderbookSnapshot, AppError> {
    let limit = depth.unwrap_or(20) as i64;

    // Asks: Lowest price first
    let asks = sqlx::query_as!(
        PriceLevel,
        r#"SELECT price_cents, SUM(quantity - quantity_filled)::integer as "total_quantity!", COUNT(*)::integer as "order_count!"
           FROM market_orders
           WHERE asset_id = $1 AND side = 'sell' AND status IN ('open', 'partially_filled')
           GROUP BY price_cents
           ORDER BY price_cents ASC
           LIMIT $2"#,
        asset_id,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    // Bids: Highest price first
    let bids = sqlx::query_as!(
        PriceLevel,
        r#"SELECT price_cents, SUM(quantity - quantity_filled)::integer as "total_quantity!", COUNT(*)::integer as "order_count!"
           FROM market_orders
           WHERE asset_id = $1 AND side = 'buy' AND status IN ('open', 'partially_filled')
           GROUP BY price_cents
           ORDER BY price_cents DESC
           LIMIT $2"#,
        asset_id,
        limit
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let spread = match (asks.first(), bids.first()) {
        (Some(best_ask), Some(best_bid)) => Some(best_ask.price_cents - best_bid.price_cents),
        _ => None,
    };

    Ok(OrderbookSnapshot {
        asset_id,
        bids,
        asks,
        spread_cents: spread,
        last_price_cents: None, // Filled by caller
        timestamp: Utc::now(),
    })
}

/// Helper to get the best bid/ask from the database when Redis is unavailable.
pub async fn get_best_price_from_db(
    pool: &PgPool,
    asset_id: Uuid,
    side: OrderSide,
) -> Result<i64, AppError> {
    let opposing_side = match side {
        OrderSide::Buy => "sell",
        OrderSide::Sell => "buy",
    };

    let price: Option<i64> = sqlx::query_scalar(
        &format!(
            "SELECT price_cents FROM market_orders WHERE asset_id = $1 AND side = $2 AND status IN ('open', 'partially_filled') ORDER BY price_cents {} LIMIT 1",
            if opposing_side == "sell" { "ASC" } else { "DESC" }
        )
    )
    .bind(asset_id)
    .bind(opposing_side)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    price.ok_or_else(|| {
        AppError::OrderRejected(format!(
            "No {} orders available. Try a limit order instead.",
            opposing_side
        ))
    })
}
