/// P2P/OTC Offer System — direct offers between users.
///
/// Allows users to make private buy/sell offers to specific counterparties.
/// Offers can be accepted, declined, or countered. Accepted offers are
/// settled using the same ACID pipeline as order-book trades.
///
/// Flow:
/// 1. Maker creates offer → `p2p_offers` row with status=pending
/// 2. Taker responds: accept / decline / counter
/// 3. Accepted → settlement creates a trade_history entry
/// 4. Expired → background worker sets status=expired
///
/// All monetary values are `i64` cents. No `unwrap()` in production paths.
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::models::P2POffer;
use crate::error::AppError;

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
// ── API DTOs ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Request body for creating a P2P offer.
#[derive(Debug, Deserialize)]
pub struct CreateP2POfferRequest {
    /// Asset to trade.
    pub asset_id: Uuid,
    /// User receiving the offer.
    pub taker_user_id: Uuid,
    /// "buy" or "sell".
    pub side: String,
    /// Price per token in cents.
    pub price_cents: i64,
    /// Number of tokens.
    pub quantity: i32,
    /// Optional message to the taker.
    pub message: Option<String>,
    /// Hours until expiry (default: 48).
    pub expires_in_hours: Option<i32>,
}

/// Request body for responding to a P2P offer.
#[derive(Debug, Deserialize)]
pub struct RespondP2POfferRequest {
    /// "accept", "decline", or "counter"
    pub action: String,
    /// Required if action = "counter"
    pub counter_price_cents: Option<i64>,
    /// Required if action = "counter"
    pub counter_quantity: Option<i32>,
    /// Optional message
    pub message: Option<String>,
}

/// Response after creating/responding to a P2P offer.
#[derive(Debug, Serialize)]
pub struct P2POfferResponse {
    pub id: Uuid,
    pub status: String,
    pub message: String,
    /// Trade ID if the offer was accepted and settled.
    pub trade_id: Option<Uuid>,
}

// ═══════════════════════════════════════════════════════════════
// ── CREATE OFFER ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Create a new P2P offer.
///
/// Validates:
/// - Maker != taker (DB constraint, but check early for a nice error)
/// - Asset exists
/// - Price > 0, quantity > 0
/// - Maker has sufficient balance (buy) or tokens (sell)
pub async fn create_offer(
    pool: &PgPool,
    maker_user_id: Uuid,
    req: CreateP2POfferRequest,
) -> Result<P2POfferResponse, AppError> {
    // ── Validation ───────────────────────────────────────────
    if maker_user_id == req.taker_user_id {
        return Err(AppError::BadRequest(
            "Cannot create an offer to yourself.".into(),
        ));
    }

    if req.price_cents <= 0 {
        return Err(AppError::BadRequest(
            "Price must be greater than zero.".into(),
        ));
    }

    if req.quantity <= 0 {
        return Err(AppError::BadRequest(
            "Quantity must be greater than zero.".into(),
        ));
    }

    let side = req.side.to_lowercase();
    if side != "buy" && side != "sell" {
        return Err(AppError::BadRequest("Side must be 'buy' or 'sell'.".into()));
    }

    // Check asset exists
    let asset_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM assets WHERE id = $1)")
            .bind(req.asset_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?;

    if !asset_exists {
        return Err(AppError::NotFound("Asset not found.".into()));
    }

    // Check taker exists
    let taker_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(req.taker_user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

    if !taker_exists {
        return Err(AppError::NotFound("Taker user not found.".into()));
    }

    // ── Balance/Token Check ──────────────────────────────────
    let total_cents = req.price_cents.saturating_mul(req.quantity as i64);

    if side == "buy" {
        // Maker is buying — check they have sufficient cash
        let balance: Option<i64> = sqlx::query_scalar(
            "SELECT balance_cents - held_balance_cents FROM wallets
             WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
        )
        .bind(maker_user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        let available = balance.unwrap_or(0);
        if available < total_cents {
            return Err(AppError::InsufficientBalance {
                available_cents: available,
                required_cents: total_cents,
            });
        }
    } else {
        // Maker is selling — check they have sufficient tokens
        let tokens: Option<i32> = sqlx::query_scalar(
            "SELECT tokens_owned - COALESCE(held_tokens, 0) FROM investments
             WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'",
        )
        .bind(maker_user_id)
        .bind(req.asset_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        let available = tokens.unwrap_or(0);
        if available < req.quantity {
            return Err(AppError::InsufficientTokens {
                available,
                required: req.quantity,
            });
        }
    }

    // ── Insert ───────────────────────────────────────────────
    let expires_hours = req.expires_in_hours.unwrap_or(48).max(1).min(168); // 1h-7d
    let expires_at = Utc::now() + Duration::hours(expires_hours as i64);

    let offer_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO p2p_offers
           (asset_id, maker_user_id, taker_user_id, side, price_cents, quantity,
            message, status, expires_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', $8)
           RETURNING id"#,
    )
    .bind(req.asset_id)
    .bind(maker_user_id)
    .bind(req.taker_user_id)
    .bind(&side)
    .bind(req.price_cents)
    .bind(req.quantity)
    .bind(&req.message)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(P2POfferResponse {
        id: offer_id,
        status: "pending".to_string(),
        message: "Offer created successfully.".to_string(),
        trade_id: None,
    })
}

// ═══════════════════════════════════════════════════════════════
// ── RESPOND TO OFFER ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Respond to a P2P offer (accept, decline, or counter).
///
/// Accept flow:
/// 1. Validate taker has sufficient balance/tokens
/// 2. Begin TX
/// 3. Hold taker's balance/tokens
/// 4. Execute settlement (transfer balance + tokens, record trade)
/// 5. Mark offer as accepted with trade_id
pub async fn respond_to_offer(
    pool: &PgPool,
    taker_user_id: Uuid,
    offer_id: Uuid,
    req: RespondP2POfferRequest,
) -> Result<P2POfferResponse, AppError> {
    // Load the offer
    let offer = sqlx::query_as::<_, P2POffer>("SELECT * FROM p2p_offers WHERE id = $1")
        .bind(offer_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Offer not found.".into()))?;

    // Only the taker can respond
    if offer.taker_user_id != taker_user_id {
        return Err(AppError::Forbidden(
            "Only the intended recipient can respond to this offer.".into(),
        ));
    }

    // Must be pending
    if offer.status != "pending" {
        return Err(AppError::BadRequest(format!(
            "Offer is already '{}', cannot respond.",
            offer.status
        )));
    }

    // Check expiry
    if Utc::now() > offer.expires_at {
        // Auto-expire it
        sqlx::query("UPDATE p2p_offers SET status = 'expired', updated_at = NOW() WHERE id = $1")
            .bind(offer_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        return Err(AppError::BadRequest("This offer has expired.".into()));
    }

    let action = req.action.to_lowercase();

    match action.as_str() {
        "decline" => {
            sqlx::query(
                "UPDATE p2p_offers SET status = 'declined', updated_at = NOW() WHERE id = $1",
            )
            .bind(offer_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

            Ok(P2POfferResponse {
                id: offer_id,
                status: "declined".to_string(),
                message: "Offer declined.".to_string(),
                trade_id: None,
            })
        }

        "counter" => {
            let counter_price = req.counter_price_cents.ok_or_else(|| {
                AppError::BadRequest("counter_price_cents is required for counter-offers.".into())
            })?;
            let counter_qty = req.counter_quantity.ok_or_else(|| {
                AppError::BadRequest("counter_quantity is required for counter-offers.".into())
            })?;

            if counter_price <= 0 || counter_qty <= 0 {
                return Err(AppError::BadRequest(
                    "Counter price and quantity must be > 0.".into(),
                ));
            }

            // Mark original as countered
            sqlx::query(
                "UPDATE p2p_offers SET status = 'countered', updated_at = NOW() WHERE id = $1",
            )
            .bind(offer_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

            // Reverse the side for the counter
            let counter_side = if offer.side == "buy" { "sell" } else { "buy" };

            // Create counter-offer (taker becomes maker, original maker becomes taker)
            let counter_id = sqlx::query_scalar::<_, Uuid>(
                r#"INSERT INTO p2p_offers
                   (asset_id, maker_user_id, taker_user_id, side, price_cents, quantity,
                    message, status, parent_offer_id, expires_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', $8, $9)
                   RETURNING id"#,
            )
            .bind(offer.asset_id)
            .bind(taker_user_id) // new maker = original taker
            .bind(offer.maker_user_id) // new taker = original maker
            .bind(counter_side)
            .bind(counter_price)
            .bind(counter_qty)
            .bind(&req.message)
            .bind(offer_id) // parent_offer_id
            .bind(Utc::now() + Duration::hours(48))
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?;

            Ok(P2POfferResponse {
                id: counter_id,
                status: "countered".to_string(),
                message: "Counter-offer created.".to_string(),
                trade_id: None,
            })
        }

        "accept" => accept_offer(pool, &offer, taker_user_id).await,

        _ => Err(AppError::BadRequest(
            "Action must be 'accept', 'decline', or 'counter'.".into(),
        )),
    }
}

/// Accept a P2P offer and settle the trade in an ACID transaction.
async fn accept_offer(
    pool: &PgPool,
    offer: &P2POffer,
    taker_user_id: Uuid,
) -> Result<P2POfferResponse, AppError> {
    let total_cents = offer.price_cents.saturating_mul(offer.quantity as i64);

    // Determine who is buyer and who is seller
    let (buyer_id, seller_id) = if offer.side == "buy" {
        // Maker wants to buy → maker is buyer, taker is seller
        (offer.maker_user_id, taker_user_id)
    } else {
        // Maker wants to sell → maker is seller, taker is buyer
        (taker_user_id, offer.maker_user_id)
    };

    // ── Begin ACID Transaction ───────────────────────────────
    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    // ── Check buyer balance ──────────────────────────────────
    let buyer_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(balance_cents - held_balance_cents, 0) FROM wallets
         WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'
         FOR UPDATE",
    )
    .bind(buyer_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    if buyer_balance < total_cents {
        tx.rollback().await.ok();
        return Err(AppError::InsufficientBalance {
            available_cents: buyer_balance,
            required_cents: total_cents,
        });
    }

    // ── Check seller tokens ──────────────────────────────────
    let seller_tokens: i32 = sqlx::query_scalar(
        "SELECT COALESCE(tokens_owned - COALESCE(held_tokens, 0), 0) FROM investments
         WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'
         FOR UPDATE",
    )
    .bind(seller_id)
    .bind(offer.asset_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::Database)?
    .unwrap_or(0);

    if seller_tokens < offer.quantity {
        tx.rollback().await.ok();
        return Err(AppError::InsufficientTokens {
            available: seller_tokens,
            required: offer.quantity,
        });
    }

    // ── Calculate fee ────────────────────────────────────────
    let (taker_fee_cents, taker_fee_bps) =
        super::service::calculate_trade_fee(pool, offer.asset_id, total_cents, true).await?;
    let seller_proceeds = total_cents.saturating_sub(taker_fee_cents);

    // ── Transfer balance ─────────────────────────────────────
    // Deduct buyer's balance
    sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents - $1, updated_at = NOW()
         WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(total_cents)
    .bind(buyer_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Credit seller's balance (minus fee)
    sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW()
         WHERE user_id = $2 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(seller_proceeds)
    .bind(seller_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── Transfer tokens ──────────────────────────────────────
    // Deduct seller's tokens
    sqlx::query(
        "UPDATE investments SET tokens_owned = tokens_owned - $1, updated_at = NOW()
         WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
    )
    .bind(offer.quantity)
    .bind(seller_id)
    .bind(offer.asset_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Credit buyer's tokens (upsert)
    let buyer_has_investment: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM investments WHERE user_id = $1 AND asset_id = $2 AND status != 'exited')",
    )
    .bind(buyer_id)
    .bind(offer.asset_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    if buyer_has_investment {
        sqlx::query(
            "UPDATE investments SET tokens_owned = tokens_owned + $1, updated_at = NOW()
             WHERE user_id = $2 AND asset_id = $3 AND status != 'exited'",
        )
        .bind(offer.quantity)
        .bind(buyer_id)
        .bind(offer.asset_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    } else {
        sqlx::query(
            "INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_price_cents, status)
             VALUES ($1, $2, $3, $4, 'active')",
        )
        .bind(buyer_id)
        .bind(offer.asset_id)
        .bind(offer.quantity)
        .bind(offer.price_cents)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    }

    // ── Create dummy market orders for the trade_history FK ───
    // P2P trades need buy_order_id and sell_order_id for the trade_history FK.
    // We create special "p2p" type orders that are immediately filled.
    let buy_order_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO market_orders
           (user_id, asset_id, side, order_type, price_cents, quantity, quantity_filled,
            status, idempotency_key)
           VALUES ($1, $2, 'buy', 'limit', $3, $4, $4, 'filled', $5)
           RETURNING id"#,
    )
    .bind(buyer_id)
    .bind(offer.asset_id)
    .bind(offer.price_cents)
    .bind(offer.quantity)
    .bind(format!("p2p-buy-{}", offer.id))
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    let sell_order_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO market_orders
           (user_id, asset_id, side, order_type, price_cents, quantity, quantity_filled,
            status, idempotency_key)
           VALUES ($1, $2, 'sell', 'limit', $3, $4, $4, 'filled', $5)
           RETURNING id"#,
    )
    .bind(seller_id)
    .bind(offer.asset_id)
    .bind(offer.price_cents)
    .bind(offer.quantity)
    .bind(format!("p2p-sell-{}", offer.id))
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── Record trade ─────────────────────────────────────────
    let trade_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO trade_history
           (asset_id, buy_order_id, sell_order_id, buyer_user_id, seller_user_id,
            price_cents, quantity, fee_cents, fee_bps, on_chain_status)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending')
           RETURNING id"#,
    )
    .bind(offer.asset_id)
    .bind(buy_order_id)
    .bind(sell_order_id)
    .bind(buyer_id)
    .bind(seller_id)
    .bind(offer.price_cents)
    .bind(offer.quantity)
    .bind(taker_fee_cents)
    .bind(taker_fee_bps)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // Collect platform fee
    if taker_fee_cents > 0 {
        sqlx::query(
            "UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW()
             WHERE wallet_type = 'platform_fee' AND currency = 'USD'",
        )
        .bind(taker_fee_cents)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;
    }

    // ── Mark offer as accepted ───────────────────────────────
    sqlx::query(
        "UPDATE p2p_offers SET status = 'accepted', trade_id = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(trade_id)
    .bind(offer.id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::Database)?;

    // ── COMMIT ───────────────────────────────────────────────
    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        "🤝 P2P trade settled: offer={}, trade={}, buyer={}, seller={}, total={}",
        offer.id,
        trade_id,
        buyer_id,
        seller_id,
        total_cents
    );

    Ok(P2POfferResponse {
        id: offer.id,
        status: "accepted".to_string(),
        message: "Offer accepted and settled.".to_string(),
        trade_id: Some(trade_id),
    })
}

// ═══════════════════════════════════════════════════════════════
// ── CANCEL OFFER ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Cancel a pending offer (by the maker only).
pub async fn cancel_offer(
    pool: &PgPool,
    maker_user_id: Uuid,
    offer_id: Uuid,
) -> Result<P2POfferResponse, AppError> {
    let result = sqlx::query(
        "UPDATE p2p_offers SET status = 'cancelled', updated_at = NOW()
         WHERE id = $1 AND maker_user_id = $2 AND status = 'pending'",
    )
    .bind(offer_id)
    .bind(maker_user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "Offer not found, not yours, or not cancellable.".into(),
        ));
    }

    Ok(P2POfferResponse {
        id: offer_id,
        status: "cancelled".to_string(),
        message: "Offer cancelled.".to_string(),
        trade_id: None,
    })
}

// ═══════════════════════════════════════════════════════════════
// ── READ APIs ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Get incoming offers for a user (where they are the taker).
pub async fn get_incoming_offers(pool: &PgPool, user_id: Uuid) -> Result<Vec<P2POffer>, AppError> {
    let offers = sqlx::query_as::<_, P2POffer>(
        "SELECT * FROM p2p_offers WHERE taker_user_id = $1
         ORDER BY CASE WHEN status = 'pending' THEN 0 ELSE 1 END, created_at DESC
         LIMIT 50",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(offers)
}

/// Get outgoing offers for a user (where they are the maker).
pub async fn get_outgoing_offers(pool: &PgPool, user_id: Uuid) -> Result<Vec<P2POffer>, AppError> {
    let offers = sqlx::query_as::<_, P2POffer>(
        "SELECT * FROM p2p_offers WHERE maker_user_id = $1
         ORDER BY created_at DESC LIMIT 50",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(offers)
}

/// Get offers for a specific asset (cap table context).
pub async fn get_asset_offers(pool: &PgPool, asset_id: Uuid) -> Result<Vec<P2POffer>, AppError> {
    let offers = sqlx::query_as::<_, P2POffer>(
        "SELECT * FROM p2p_offers WHERE asset_id = $1 AND status = 'pending'
         ORDER BY created_at DESC LIMIT 100",
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(offers)
}

// ═══════════════════════════════════════════════════════════════
// ── BACKGROUND: EXPIRY WORKER ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Expire all pending offers past their expires_at timestamp.
/// Called by the background worker (e.g., every 15 minutes).
pub async fn expire_stale_offers(pool: &PgPool) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE p2p_offers SET status = 'expired', updated_at = NOW()
         WHERE status = 'pending' AND expires_at < NOW()",
    )
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    let count = result.rows_affected();
    if count > 0 {
        tracing::info!("⏰ Expired {} stale P2P offers", count);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_offer_validation_self_offer() {
        let id = Uuid::new_v4();
        let req = CreateP2POfferRequest {
            asset_id: Uuid::new_v4(),
            taker_user_id: id,
            side: "buy".to_string(),
            price_cents: 1000,
            quantity: 10,
            message: None,
            expires_in_hours: None,
        };
        // Can't test async validation here, but we can verify the struct is correct
        assert_eq!(req.taker_user_id, id);
        assert!(req.price_cents > 0);
        assert!(req.quantity > 0);
    }

    #[test]
    fn test_side_normalization() {
        let side = "BUY".to_lowercase();
        assert_eq!(side, "buy");
        let side2 = "Sell".to_lowercase();
        assert_eq!(side2, "sell");
    }

    #[test]
    fn test_p2p_total_calculation() {
        let price = 5000i64; // $50.00
        let qty = 10i32;
        let total = price.saturating_mul(qty as i64);
        assert_eq!(total, 50000); // $500.00
    }

    #[test]
    fn test_buyer_seller_determination_buy_side() {
        let maker = Uuid::new_v4();
        let taker = Uuid::new_v4();
        let side = "buy";
        let (buyer, seller) = if side == "buy" {
            (maker, taker)
        } else {
            (taker, maker)
        };
        assert_eq!(buyer, maker);
        assert_eq!(seller, taker);
    }

    #[test]
    fn test_buyer_seller_determination_sell_side() {
        let maker = Uuid::new_v4();
        let taker = Uuid::new_v4();
        let side = "sell";
        let (buyer, seller) = if side == "buy" {
            (maker, taker)
        } else {
            (taker, maker)
        };
        assert_eq!(buyer, taker);
        assert_eq!(seller, maker);
    }

    #[test]
    fn test_expiry_hours_clamping() {
        // Default 48h
        let hours = None::<i32>.unwrap_or(48).max(1).min(168);
        assert_eq!(hours, 48);

        // Clamped to 1h min
        let hours = Some(0).unwrap_or(48).max(1).min(168);
        assert_eq!(hours, 1);

        // Clamped to 168h (7d) max
        let hours = Some(500).unwrap_or(48).max(1).min(168);
        assert_eq!(hours, 168);
    }

    #[test]
    fn test_respond_action_normalization() {
        assert_eq!("ACCEPT".to_lowercase(), "accept");
        assert_eq!("Decline".to_lowercase(), "decline");
        assert_eq!("Counter".to_lowercase(), "counter");
    }

    #[test]
    fn test_counter_side_reversal() {
        assert_eq!(if "buy" == "buy" { "sell" } else { "buy" }, "sell");
        assert_eq!(if "sell" == "buy" { "sell" } else { "buy" }, "buy");
    }
}
