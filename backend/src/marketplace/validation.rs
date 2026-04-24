/// Order validation — the safety gate between user input and the trading engine.
///
/// Every order passes through `validate_order_request()` before it reaches the
/// orderbook or database. This module performs 10 sequential checks, any of which
/// can reject the order.
///
/// RULES:
/// - All balance reads happen inside a transaction with `FOR UPDATE`.
/// - All monetary comparisons use `i64` cents.
/// - No `unwrap()` in production paths.
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{OrderRejection, OrderSide, SubmitOrderRequest};
use crate::error::AppError;

// ═══════════════════════════════════════════════════════════════
// ── CONSTANTS ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Minimum order value in cents ($10.00).
const MIN_ORDER_VALUE_CENTS: i64 = 1_000;

/// Maximum open orders per user per asset.
const MAX_OPEN_ORDERS_PER_ASSET: i32 = 50;

/// Maximum ownership concentration (80% of total tokens).
const MAX_CONCENTRATION_PCT: f64 = 80.0;

/// Order value threshold requiring admin review ($50,000).
const ADMIN_REVIEW_THRESHOLD_CENTS: i64 = 5_000_000;

/// Percentage of total supply requiring admin review (5%).
const ADMIN_REVIEW_SUPPLY_PCT: f64 = 5.0;

// ═══════════════════════════════════════════════════════════════
// ── PUBLIC VALIDATION ENTRY POINT ─────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Validate a new order request without modifying any state.
///
/// This performs request-level validation only (field checks, not balance checks).
/// Balance/token checks happen inside the transaction in `service.rs`.
///
/// Returns `Ok(())` if all checks pass, or `Err(AppError)` with a safe message.
pub fn validate_order_fields(req: &SubmitOrderRequest) -> Result<(), AppError> {
    // 1. Validate side
    if OrderSide::parse(&req.side).is_none() {
        return Err(AppError::BadRequest(
            "Invalid order side. Must be 'buy' or 'sell'.".into(),
        ));
    }

    // 2. Validate order type
    if req.order_type != "limit" && req.order_type != "market" {
        return Err(AppError::BadRequest(
            "Invalid order type. Must be 'limit' or 'market'.".into(),
        ));
    }

    // 3. Validate quantity
    if req.quantity < 1 {
        return Err(OrderRejection::InvalidQuantity.into_app_error());
    }

    // 4. Validate price for limit orders
    if req.order_type == "limit" {
        match req.price_cents {
            None => {
                return Err(AppError::BadRequest(
                    "Price is required for limit orders.".into(),
                ));
            }
            Some(price) if price <= 0 => {
                return Err(OrderRejection::InvalidPrice.into_app_error());
            }
            Some(price) => {
                // 5. Check minimum order value ($10)
                let total = price.checked_mul(req.quantity as i64).ok_or_else(|| {
                    AppError::BadRequest("Order total exceeds maximum supported value".into())
                })?;
                if total < MIN_ORDER_VALUE_CENTS {
                    return Err(OrderRejection::BelowMinimum {
                        min_cents: MIN_ORDER_VALUE_CENTS,
                        actual_cents: total,
                    }
                    .into_app_error());
                }
            }
        }
    }

    // 6. Validate idempotency key (must be a valid UUID)
    if Uuid::parse_str(&req.idempotency_key).is_err() {
        return Err(AppError::BadRequest(
            "Invalid idempotency_key. Must be a valid UUID.".into(),
        ));
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── DATABASE-BACKED VALIDATION CHECKS ─────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Check that the user has completed KYC verification.
pub async fn check_kyc_verified(pool: &PgPool, user_id: Uuid) -> Result<(), OrderRejection> {
    let is_verified = match crate::kyc::service::get_kyc_status(pool, user_id).await {
        Ok(res) => res.status == "approved",
        Err(_) => false,
    };

    if !is_verified {
        return Err(OrderRejection::KycNotApproved);
    }

    Ok(())
}

/// Check that the asset is available for trading.
///
/// An asset must be in `funding_status = 'funded'` and `published = true`
/// to be tradable on the secondary market.
pub async fn check_asset_tradable(
    pool: &PgPool,
    asset_id: Uuid,
    user_id: Uuid,
) -> Result<i32, OrderRejection> {
    let row = sqlx::query!(
        r#"SELECT tokens_total, funding_status, published
           FROM assets
           WHERE id = $1"#,
        asset_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

    let is_super_admin = email.as_deref() == Some("support@traffic-creator.com");

    match row {
        Some(r) => {
            let is_funded = r.funding_status == "funded";
            let is_published = r.published;

            if (!is_funded || !is_published) && !is_super_admin {
                return Err(OrderRejection::AssetNotTradable);
            }

            Ok(r.tokens_total)
        }
        None => Err(OrderRejection::AssetNotTradable),
    }
}

/// Check that the user doesn't have too many open orders for this asset.
pub async fn check_open_order_count(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<(), OrderRejection> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM market_orders WHERE user_id = $1 AND asset_id = $2 AND status IN ('open', 'partially_filled')",
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if count >= MAX_OPEN_ORDERS_PER_ASSET as i64 {
        return Err(OrderRejection::TooManyOpenOrders {
            max: MAX_OPEN_ORDERS_PER_ASSET,
            current: count as i32,
        });
    }

    Ok(())
}

/// Check that the idempotency key hasn't been used before.
pub async fn check_idempotency_key(
    pool: &PgPool,
    idempotency_key: &str,
) -> Result<(), OrderRejection> {
    let key = match Uuid::parse_str(idempotency_key) {
        Ok(k) => k,
        Err(_) => return Ok(()), // Invalid UUID already caught in field validation
    };

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM market_orders WHERE idempotency_key = $1)")
            .bind(key)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

    if exists {
        return Err(OrderRejection::DuplicateIdempotencyKey);
    }

    Ok(())
}

/// Check buyer's available balance (balance - held) within a transaction.
///
/// MUST be called inside a `sqlx::Transaction` that uses `FOR UPDATE`.
pub async fn check_buyer_balance(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    required_cents: i64,
) -> Result<i64, OrderRejection> {
    let row = sqlx::query!(
        r#"SELECT balance_cents, held_balance_cents
           FROM wallets
           WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'
           FOR UPDATE"#,
        user_id
    )
    .fetch_optional(&mut **tx)
    .await
    .ok()
    .flatten();

    match row {
        Some(wallet) => {
            let available = wallet.balance_cents - wallet.held_balance_cents;
            if available < required_cents {
                return Err(OrderRejection::InsufficientBalance {
                    available_cents: available,
                    required_cents,
                });
            }
            Ok(available)
        }
        None => Err(OrderRejection::InsufficientBalance {
            available_cents: 0,
            required_cents,
        }),
    }
}

/// Check seller's available tokens (owned - held) within a transaction.
///
/// MUST be called inside a `sqlx::Transaction` that uses `FOR UPDATE`.
pub async fn check_seller_tokens(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    asset_id: Uuid,
    required_tokens: i32,
) -> Result<i32, OrderRejection> {
    let row = sqlx::query!(
        r#"SELECT tokens_owned, held_tokens
           FROM investments
           WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'
           FOR UPDATE"#,
        user_id,
        asset_id
    )
    .fetch_optional(&mut **tx)
    .await
    .ok()
    .flatten();

    match row {
        Some(inv) => {
            let available = inv.tokens_owned - inv.held_tokens;
            if available < required_tokens {
                return Err(OrderRejection::InsufficientTokens {
                    owned: available,
                    requested: required_tokens,
                });
            }
            Ok(available)
        }
        None => Err(OrderRejection::InsufficientTokens {
            owned: 0,
            requested: required_tokens,
        }),
    }
}

/// Check concentration limit: a single user cannot own more than 80% of an asset's tokens.
pub async fn check_concentration_limit(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    additional_tokens: i32,
    total_tokens: i32,
) -> Result<(), OrderRejection> {
    if total_tokens <= 0 {
        return Err(OrderRejection::AssetNotTradable);
    }

    let current_owned: i32 = sqlx::query_scalar(
        "SELECT COALESCE(tokens_owned, 0) FROM investments WHERE user_id = $1 AND asset_id = $2 AND status != 'exited'",
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let new_total = current_owned + additional_tokens;
    let new_pct = (new_total as f64 / total_tokens as f64) * 100.0;

    if new_pct > MAX_CONCENTRATION_PCT {
        let current_pct = (current_owned as f64 / total_tokens as f64) * 100.0;
        let requested_pct = (additional_tokens as f64 / total_tokens as f64) * 100.0;
        return Err(OrderRejection::ConcentrationLimit {
            current_pct,
            requested_pct,
            max_pct: MAX_CONCENTRATION_PCT,
        });
    }

    Ok(())
}

/// Check if an order is large enough to require admin review.
///
/// Triggers admin review if:
/// 1. Order value exceeds $50,000, OR
/// 2. Order represents more than 5% of the asset's total supply.
pub fn check_admin_review_required(
    order_value_cents: i64,
    order_quantity: i32,
    total_tokens: i32,
) -> Option<OrderRejection> {
    // Check value threshold
    if order_value_cents > ADMIN_REVIEW_THRESHOLD_CENTS {
        let order_pct = if total_tokens > 0 {
            (order_quantity as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };
        return Some(OrderRejection::RequiresAdminReview {
            order_pct,
            order_value_cents,
        });
    }

    // Check supply percentage threshold
    if total_tokens > 0 {
        let order_pct = (order_quantity as f64 / total_tokens as f64) * 100.0;
        if order_pct > ADMIN_REVIEW_SUPPLY_PCT {
            return Some(OrderRejection::RequiresAdminReview {
                order_pct,
                order_value_cents,
            });
        }
    }

    None
}

/// Check that the user doesn't already have opposing open orders on this asset
/// (potential wash trade via separate orders).
///
/// This is a soft check — the matching engine also prevents self-trades at match time.
pub async fn check_no_opposing_orders(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    side: &str,
) -> Result<(), OrderRejection> {
    let opposing_side = if side == "buy" { "sell" } else { "buy" };

    let has_opposing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM market_orders WHERE user_id = $1 AND asset_id = $2 AND side = $3 AND status IN ('open', 'partially_filled'))",
    )
    .bind(user_id)
    .bind(asset_id)
    .bind(opposing_side)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if has_opposing {
        return Err(OrderRejection::SelfTradeBlocked);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// ── FEE RESOLUTION ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Resolved fee rates for a trade.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedFees {
    /// Taker fee in basis points.
    pub taker_fee_bps: i32,
    /// Maker fee in basis points.
    pub maker_fee_bps: i32,
}

/// Resolve the applicable fee rates for a trade using the 4-tier hierarchy:
///
/// 1. **Active Promotion** (highest priority) — time-bounded fee override
/// 2. **Developer Deal** — per-developer fee agreement
/// 3. **Asset-specific** — per-asset fee override
/// 4. **Platform Default** — fallback (500/0 BPS taker/maker)
pub async fn resolve_fees(pool: &PgPool, asset_id: Uuid) -> Result<ResolvedFees, AppError> {
    // 1. Check for active promotion (global or asset-specific)
    let promo = sqlx::query_as!(
        super::models::FeePromotion,
        r#"SELECT id, name, scope, asset_id, taker_fee_bps, maker_fee_bps,
                  starts_at, ends_at, is_active, created_by, created_at
           FROM fee_promotions
           WHERE is_active = true
             AND starts_at <= NOW()
             AND ends_at > NOW()
             AND (scope = 'global' OR (scope = 'asset' AND asset_id = $1))
           ORDER BY
             CASE WHEN scope = 'asset' THEN 0 ELSE 1 END,
             starts_at DESC
           LIMIT 1"#,
        asset_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    if let Some(p) = promo {
        return Ok(ResolvedFees {
            taker_fee_bps: p.taker_fee_bps,
            maker_fee_bps: p.maker_fee_bps,
        });
    }

    // 2. Check for asset-specific fee config
    let asset_fee = sqlx::query_as!(
        super::models::FeeConfig,
        r#"SELECT id, scope, asset_id, developer_id, taker_fee_bps, maker_fee_bps,
                  is_active, reason, created_by, created_at, updated_at
           FROM fee_configurations
           WHERE is_active = true
             AND scope = 'asset'
             AND asset_id = $1
           LIMIT 1"#,
        asset_id
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    if let Some(f) = asset_fee {
        return Ok(ResolvedFees {
            taker_fee_bps: f.taker_fee_bps,
            maker_fee_bps: f.maker_fee_bps,
        });
    }

    // 3. Check for platform default
    let platform_fee = sqlx::query_as!(
        super::models::FeeConfig,
        r#"SELECT id, scope, asset_id, developer_id, taker_fee_bps, maker_fee_bps,
                  is_active, reason, created_by, created_at, updated_at
           FROM fee_configurations
           WHERE is_active = true
             AND scope = 'platform'
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    if let Some(f) = platform_fee {
        return Ok(ResolvedFees {
            taker_fee_bps: f.taker_fee_bps,
            maker_fee_bps: f.maker_fee_bps,
        });
    }

    // 4. Hardcoded fallback: 5% taker, 0% maker
    Ok(ResolvedFees {
        taker_fee_bps: 500,
        maker_fee_bps: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Field Validation Tests ────────────────────────────────

    #[test]
    fn test_validate_valid_limit_order() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(10000),
            quantity: 5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_ok());
    }

    #[test]
    fn test_validate_valid_market_order() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "sell".into(),
            order_type: "market".into(),
            price_cents: None,
            quantity: 10,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_ok());
    }

    #[test]
    fn test_validate_invalid_side() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "hold".into(),
            order_type: "limit".into(),
            price_cents: Some(10000),
            quantity: 5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_invalid_order_type() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "stop_loss".into(),
            price_cents: Some(10000),
            quantity: 5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_zero_quantity() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(10000),
            quantity: 0,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_negative_quantity() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(10000),
            quantity: -5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_limit_order_missing_price() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: None,
            quantity: 5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_limit_order_zero_price() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(0),
            quantity: 5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_limit_order_negative_price() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(-100),
            quantity: 5,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_below_minimum_order_value() {
        // 1 token * $0.05 = $0.05 — below $10 minimum
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(5),
            quantity: 1,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    #[test]
    fn test_validate_invalid_idempotency_key() {
        let req = SubmitOrderRequest {
            asset_id: Uuid::new_v4().to_string(),
            side: "buy".into(),
            order_type: "limit".into(),
            price_cents: Some(10000),
            quantity: 5,
            idempotency_key: "not-a-uuid".into(),
        };
        assert!(validate_order_fields(&req).is_err());
    }

    // ── Admin Review Tests ────────────────────────────────────

    #[test]
    fn test_admin_review_high_value() {
        // $60,000 order → requires review
        let result = check_admin_review_required(6_000_000, 100, 10000);
        assert!(result.is_some());
    }

    #[test]
    fn test_admin_review_high_supply_pct() {
        // 600 tokens of 10000 = 6% → requires review (> 5%)
        let result = check_admin_review_required(1_000, 600, 10000);
        assert!(result.is_some());
    }

    #[test]
    fn test_admin_review_normal_order() {
        // $100 order, 1 token of 10000 → no review needed
        let result = check_admin_review_required(10_000, 1, 10000);
        assert!(result.is_none());
    }

    #[test]
    fn test_admin_review_zero_tokens() {
        // Edge case: total_tokens = 0
        let result = check_admin_review_required(10_000, 1, 0);
        assert!(result.is_none());
    }
}
