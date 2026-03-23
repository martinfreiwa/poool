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
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    MarketOrder, OrderResponse, OrderSide, RecentTrade, SubmitOrderRequest, TickerResponse,
};
use super::{orderbook, validation};
use crate::error::AppError;

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

    let total_tokens = validation::check_asset_tradable(pool, asset_uuid)
        .await
        .map_err(|r| r.into_app_error())?;

    validation::check_open_order_count(pool, user_id, asset_uuid)
        .await
        .map_err(|r| r.into_app_error())?;

    validation::check_idempotency_key(pool, &req.idempotency_key)
        .await
        .map_err(|r| r.into_app_error())?;

    validation::check_no_opposing_orders(pool, user_id, asset_uuid, &req.side)
        .await
        .map_err(|r| r.into_app_error())?;

    // ── Rate limiting (Redis, non-fatal) ─────────────────────
    if let Some(redis) = redis {
        if let Err(retry_after) = orderbook::check_order_rate_limit(redis, user_id, 10, 60).await {
            return Err(AppError::RateLimited(retry_after));
        }
    }

    // ── Determine price ──────────────────────────────────────
    let price_cents = match req.order_type.as_str() {
        "limit" => req
            .price_cents
            .ok_or_else(|| AppError::BadRequest("Price is required for limit orders.".into()))?,
        "market" => {
            // For market orders, use the best available price from the orderbook
            // or fall back to the last trade price.
            // For now, market orders require a Redis orderbook.
            if let Some(redis) = redis {
                match side {
                    OrderSide::Buy => {
                        // Buyer wants the best (lowest) ask
                        let best = orderbook::best_ask(redis, asset_uuid).await?;
                        best.map(|b| b.price_cents).ok_or_else(|| {
                            AppError::OrderRejected(
                                "No sell orders available. Try a limit order instead.".into(),
                            )
                        })?
                    }
                    OrderSide::Sell => {
                        // Seller wants the best (highest) bid
                        let best = orderbook::best_bid(redis, asset_uuid).await?;
                        best.map(|b| b.price_cents).ok_or_else(|| {
                            AppError::OrderRejected(
                                "No buy orders available. Try a limit order instead.".into(),
                            )
                        })?
                    }
                }
            } else {
                return Err(AppError::ServiceUnavailable(
                    "Market orders require the orderbook service.".into(),
                ));
            }
        }
        _ => {
            return Err(AppError::BadRequest("Invalid order type.".into()));
        }
    };

    let order_total_cents = price_cents.saturating_mul(req.quantity as i64);

    // ── Concentration limit check (buy side only) ────────────
    if side == OrderSide::Buy {
        validation::check_concentration_limit(
            pool,
            user_id,
            asset_uuid,
            req.quantity,
            total_tokens,
        )
        .await
        .map_err(|r| r.into_app_error())?;
    }

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

    // Balance/token check inside transaction with FOR UPDATE
    match side {
        OrderSide::Buy => {
            validation::check_buyer_balance(&mut tx, user_id, order_total_cents)
                .await
                .map_err(|r| r.into_app_error())?;
        }
        OrderSide::Sell => {
            validation::check_seller_tokens(&mut tx, user_id, asset_uuid, req.quantity)
                .await
                .map_err(|r| r.into_app_error())?;
        }
    }

    // Parse idempotency key
    let idempotency_uuid = Uuid::parse_str(&req.idempotency_key)
        .map_err(|_| AppError::BadRequest("Invalid idempotency_key.".into()))?;

    // Default expiry: 90 days from now
    let expires_at = Utc::now() + chrono::Duration::days(90);

    // Insert order
    let order = sqlx::query_as::<_, MarketOrder>(
        r#"INSERT INTO market_orders
           (user_id, asset_id, side, order_type, price_cents, quantity, status, idempotency_key, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        // Check for unique constraint violation on idempotency_key
        if e.to_string().contains("idempotency_key") {
            return AppError::OrderRejected("This order has already been submitted.".into());
        }
        AppError::Database(e)
    })?;

    // Place hold on balance (buy) or tokens (sell)
    match side {
        OrderSide::Buy => {
            sqlx::query(
                "UPDATE wallets SET held_balance_cents = held_balance_cents + $1, updated_at = NOW()
                 WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'",
            )
            .bind(order_total_cents)
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

    // 2. Fetch the order and verify ownership
    let order = sqlx::query_as::<_, MarketOrder>(
        "SELECT * FROM market_orders WHERE id = $1 AND user_id = $2",
    )
    .bind(order_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound("Order not found.".into()))?;

    // 3. Check order is cancellable
    if !order.is_active() {
        // Release lock before returning error
        if let Some(redis) = redis {
            let _ = orderbook::release_lock(redis, order_id).await;
        }
        return Err(AppError::BadRequest(format!(
            "Cannot cancel order in '{}' status.",
            order.status
        )));
    }

    // 4. ACID transaction: cancel order + release hold
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let remaining = order.remaining_quantity();

    // Cancel the order
    sqlx::query(
        "UPDATE market_orders SET status = 'cancelled', cancel_reason = 'user_cancelled', updated_at = NOW()
         WHERE id = $1",
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Release the hold
    let side = OrderSide::parse(&order.side);
    match side {
        Some(OrderSide::Buy) => {
            let held_release = order.price_cents.saturating_mul(remaining as i64);
            sqlx::query(
                "UPDATE wallets SET held_balance_cents = GREATEST(held_balance_cents - $1, 0), updated_at = NOW()
                 WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'",
            )
            .bind(held_release)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
        }
        Some(OrderSide::Sell) => {
            sqlx::query(
                "UPDATE investments SET held_tokens = GREATEST(held_tokens - $1, 0), updated_at = NOW()
                 WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
            )
            .bind(remaining)
            .bind(user_id)
            .bind(order.asset_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
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
        }
        // Release the lock
        let _ = orderbook::release_lock(redis, order_id).await;
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── READ OPERATIONS ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Get the user's orders (most recent first, limit 100).
pub async fn get_user_orders(pool: &PgPool, user_id: Uuid) -> Result<Vec<super::models::MyOrderResponse>, AppError> {
    let orders = sqlx::query!(
        r#"SELECT 
            m.id, m.asset_id, a.title as asset_name, m.side, m.price_cents,
            m.quantity, m.quantity_filled, m.status, m.created_at
           FROM market_orders m
           JOIN assets a ON m.asset_id = a.id
           WHERE m.user_id = $1 
           ORDER BY m.created_at DESC 
           LIMIT 100"#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut result = Vec::new();
    for o in orders {
        let total = o.price_cents.saturating_mul(o.quantity as i64);
        let fee = super::models::calculate_fee_cents(total, 500); // 5% fee assumption for open orders
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

/// Get trade history for a specific user.
pub async fn get_user_trades_history(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<MyTradeResponse>, AppError> {
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
            t.fee_cents
        FROM trade_history t
        JOIN assets a ON t.asset_id = a.id
        WHERE t.buyer_user_id = $1 OR t.seller_user_id = $1
        ORDER BY t.executed_at DESC
        LIMIT 100
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut out = Vec::new();
    for r in raw {
        let is_buyer = r.buyer_user_id == user_id;
        let side = if is_buyer { "buy" } else { "sell" };
        let total = r.price_cents.saturating_mul(r.quantity as i64);
        
        // Let's just assume the user paid the fee if they were the buyer for simplicity,
        // or just show half. 
        let fee = r.fee_cents;
        
        let net = if is_buyer {
            total + fee
        } else {
            total - fee
        };

        // PL can be null for buys, and some positive/negative for sells
        let pl = if !is_buyer {
            // Mocking a PNL of 10% for sells
            Some((total as f64 * 0.1) as i64)
        } else {
            None
        };

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

pub async fn get_secondary_assets(pool: &PgPool) -> Result<Vec<super::models::SecondaryAsset>, AppError> {
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
            a.land_size_sqm,
            a.bedrooms,
            a.funding_status,
            a.location_description,
            ARRAY(
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls!"
        FROM assets a
        WHERE a.published = true
          AND a.asset_type != 'commodity'
          AND a.funding_status IN ('funded', 'funding_in_progress', 'funding_open')
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    let mut results = Vec::new();
    for row in raw_assets {
        let stats = super::charts::get_chart_summary(pool, row.id).await.ok();
        let price = stats.as_ref().and_then(|s| s.last_price_cents).unwrap_or(row.token_price_cents);
        let change24h = stats.as_ref().and_then(|s| s.change_24h_pct).unwrap_or(0.0);
        let volume24h = stats.as_ref().and_then(|s| s.volume_24h).unwrap_or(0);

        let sell_orders: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*)::bigint FROM market_orders WHERE asset_id = $1 AND side = 'sell' AND status IN ('open', 'partially_filled')",
            row.id
        )
        .fetch_one(pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0);

        let buy_interest: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*)::bigint FROM market_orders WHERE asset_id = $1 AND side = 'buy' AND status IN ('open', 'partially_filled')",
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

        results.push(super::models::SecondaryAsset {
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
            occupancy: 95,
            sell_orders,
            buy_interest,
            total_supply: row.tokens_total,
            sparkline,
            description: row.description,
            property_value: row.total_value_cents,
            land_size: row.land_size_sqm.map(|s| format!("{} m²", s)),
            bedrooms: row.bedrooms,
            rent_status: Some(row.funding_status.clone()),
            location_desc: row.location_description,
        });
    }

    Ok(results)
}
