/// Marketplace data models — the language of the trading engine.
///
/// KEY RULES:
/// - All monetary values are `i64` cents. NEVER use `f64` for money.
/// - All structs that map to DB rows use `sqlx::FromRow`.
/// - API request/response DTOs are separate from DB models (no leaking internal fields).
/// - Enum variants match DB CHECK constraints exactly.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════
// ── CORE ENUMS ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Side of the order: buy or sell.
/// Stored in DB as VARCHAR: "buy" | "sell".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    /// Buy (bid) order.
    Buy,
    /// Sell (ask) order.
    Sell,
}

impl OrderSide {
    /// Convert to the DB-compatible string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        }
    }

    /// Parse from DB string. Returns `None` for invalid values.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "buy" => Some(OrderSide::Buy),
            "sell" => Some(OrderSide::Sell),
            _ => None,
        }
    }
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Type of the order: limit (price-bounded) or market (immediate execution at best price).
/// Stored in DB as VARCHAR: "limit" | "market".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    /// Limit order — executes at specified price or better.
    Limit,
    /// Market order — executes immediately at the best available price.
    Market,
}

impl OrderType {
    /// Convert to the DB-compatible string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderType::Limit => "limit",
            OrderType::Market => "market",
        }
    }
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════
// ── DATABASE MODELS ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// A market order record from the `market_orders` table.
///
/// Fields use raw DB types (String for enums) because SQLx `query_as!`
/// returns Strings for VARCHAR columns. Parse via `OrderSide::parse()`.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MarketOrder {
    /// Unique order identifier.
    pub id: Uuid,
    /// The user who placed the order.
    pub user_id: Uuid,
    /// The asset being traded.
    pub asset_id: Uuid,
    /// Order side: "buy" | "sell".
    pub side: String,
    /// Order type: "limit" | "market".
    pub order_type: String,
    /// Price per token in cents (always > 0).
    pub price_cents: i64,
    /// Desired quantity of tokens.
    pub quantity: i32,
    /// Quantity already filled.
    pub quantity_filled: i32,
    /// Lifecycle status: "open", "partially_filled", "filled", "cancelled", etc.
    pub status: String,
    /// Client-generated UUID to prevent double-submissions.
    pub idempotency_key: Option<Uuid>,
    /// Reason for cancellation (admin or system).
    pub cancel_reason: Option<String>,
    /// When this order expires (default: created_at + 90 days).
    pub expires_at: Option<DateTime<Utc>>,
    /// When the order was created.
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyOrderResponse {
    pub id: String,
    pub asset: String,
    pub asset_id: Uuid,
    pub side: String,
    pub price_cents: i64,
    pub qty: i32,
    pub filled: i32,
    pub fee: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

impl MarketOrder {
    /// Compute the remaining unfilled quantity.
    pub fn remaining_quantity(&self) -> i32 {
        self.quantity - self.quantity_filled
    }

    /// True if the order is completely filled.
    pub fn is_filled(&self) -> bool {
        self.quantity_filled >= self.quantity
    }

    /// True if the order is still active (can be matched or cancelled).
    pub fn is_active(&self) -> bool {
        self.status == "open" || self.status == "partially_filled"
    }

    /// Format the order as a Redis Sorted Set member.
    /// Format: `"order:{id}:{user_id}:{remaining_qty}:{created_at_epoch}"`
    pub fn redis_member(&self) -> String {
        format!(
            "order:{}:{}:{}:{}",
            self.id,
            self.user_id,
            self.remaining_quantity(),
            self.created_at.timestamp()
        )
    }

    /// Compute the total order value in cents (price × quantity).
    pub fn total_value_cents(&self) -> i64 {
        self.price_cents.saturating_mul(self.quantity as i64)
    }

    /// Compute the remaining order value in cents.
    pub fn remaining_value_cents(&self) -> i64 {
        self.price_cents
            .saturating_mul(self.remaining_quantity() as i64)
    }
}

/// A trade record from the `trade_history` table.
/// This table is append-only — trades are NEVER updated or deleted.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TradeRecord {
    /// Unique trade identifier.
    pub id: Uuid,
    /// The asset that was traded.
    pub asset_id: Uuid,
    /// Reference to the buy order.
    pub buy_order_id: Uuid,
    /// Reference to the sell order.
    pub sell_order_id: Uuid,
    /// The buyer's user ID.
    pub buyer_user_id: Uuid,
    /// The seller's user ID.
    pub seller_user_id: Uuid,
    /// Execution price per token in cents.
    pub price_cents: i64,
    /// Number of tokens traded.
    pub quantity: i32,
    /// Total trade value in cents (generated column: price_cents × quantity).
    pub total_cents: Option<i64>,
    /// Platform fee collected in cents.
    pub fee_cents: i64,
    /// Fee rate in basis points (for audit trail).
    pub fee_bps: i32,
    /// On-chain settlement status.
    pub on_chain_status: String,
    /// On-chain transaction hash.
    pub on_chain_tx_hash: Option<String>,
    /// Reference to settlement batch.
    pub on_chain_batch_id: Option<Uuid>,
    /// When the trade was executed.
    pub executed_at: DateTime<Utc>,
}

/// A P2P/OTC direct offer from the `p2p_offers` table.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct P2POffer {
    /// Unique offer identifier.
    pub id: Uuid,
    /// The asset being offered.
    pub asset_id: Uuid,
    /// The user who created the offer.
    pub maker_user_id: Uuid,
    /// The target user for the offer.
    pub taker_user_id: Uuid,
    /// Offer side: "buy" | "sell".
    pub side: String,
    /// Offered price per token in cents.
    pub price_cents: i64,
    /// Number of tokens offered.
    pub quantity: i32,
    /// Optional message to the taker.
    pub message: Option<String>,
    /// Offer status: "pending", "accepted", "declined", etc.
    pub status: String,
    /// Reference to predecessor (for counter-offer chains).
    pub parent_offer_id: Option<Uuid>,
    /// Reference to executed trade (if accepted).
    pub trade_id: Option<Uuid>,
    /// When the offer expires.
    pub expires_at: DateTime<Utc>,
    /// When the offer was created.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Fee configuration from the `fee_configurations` table.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FeeConfig {
    /// Unique fee config identifier.
    pub id: Uuid,
    /// Scope: "platform", "asset", or "developer".
    pub scope: String,
    /// Asset ID (NULL for platform scope).
    pub asset_id: Option<Uuid>,
    /// Developer ID (NULL except for developer scope).
    pub developer_id: Option<Uuid>,
    /// Taker fee in basis points (e.g., 500 = 5.00%).
    pub taker_fee_bps: i32,
    /// Maker fee in basis points.
    pub maker_fee_bps: i32,
    /// Whether this configuration is active.
    pub is_active: bool,
    /// Reason for fee override.
    pub reason: Option<String>,
    /// Admin who created this config.
    pub created_by: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Fee promotion from the `fee_promotions` table.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FeePromotion {
    /// Unique promotion identifier.
    pub id: Uuid,
    /// Promotion name.
    pub name: String,
    /// Scope: "global" or "asset".
    pub scope: String,
    /// Asset ID (NULL for global scope).
    pub asset_id: Option<Uuid>,
    /// Taker fee in basis points during promotion.
    pub taker_fee_bps: i32,
    /// Maker fee in basis points during promotion.
    pub maker_fee_bps: i32,
    /// Promotion start date.
    pub starts_at: DateTime<Utc>,
    /// Promotion end date.
    pub ends_at: DateTime<Utc>,
    /// Whether the promotion is active.
    pub is_active: bool,
    /// Admin who created this promotion.
    pub created_by: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════
// ── API REQUEST MODELS ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Request body for submitting a new order.
#[derive(Debug, Deserialize, Clone)]
pub struct SubmitOrderRequest {
    /// The asset to trade (UUID or slug).
    pub asset_id: String,
    /// Order side: "buy" or "sell".
    pub side: String,
    /// Order type: "limit" or "market".
    pub order_type: String,
    /// Price in cents (required for limit orders, ignored for market).
    pub price_cents: Option<i64>,
    /// Number of tokens to trade (must be >= 1).
    pub quantity: i32,
    /// Client-generated idempotency key (UUID string).
    pub idempotency_key: String,
}

// ═══════════════════════════════════════════════════════════════
// ── API RESPONSE MODELS ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Response for a successfully submitted order.
#[derive(Debug, Serialize)]
pub struct OrderResponse {
    /// The created order's ID.
    pub order_id: Uuid,
    /// Current status of the order.
    pub status: String,
    /// Human-readable message.
    pub message: String,
    /// Info about immediate fill (for market orders or crossing limit orders).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immediate_fill: Option<ImmediateFillInfo>,
}

/// Info about immediate (partial) fills.
#[derive(Debug, Serialize)]
pub struct ImmediateFillInfo {
    /// Number of tokens filled immediately.
    pub filled_quantity: i32,
    /// Weighted average execution price in cents.
    pub average_price_cents: i64,
    /// Total cost/proceeds in cents.
    pub total_cents: i64,
    /// Remaining quantity still in orderbook.
    pub remaining_quantity: i32,
}

/// Orderbook snapshot for the frontend.
#[derive(Debug, Serialize)]
pub struct OrderbookSnapshot {
    /// The asset this orderbook is for.
    pub asset_id: Uuid,
    /// Buy orders (highest price first).
    pub bids: Vec<PriceLevel>,
    /// Sell orders (lowest price first).
    pub asks: Vec<PriceLevel>,
    /// Spread between best ask and best bid (in cents).
    pub spread_cents: Option<i64>,
    /// Last traded price (from trade_history).
    pub last_price_cents: Option<i64>,
    /// Snapshot timestamp.
    pub timestamp: DateTime<Utc>,
}

/// An aggregated price level in the orderbook.
#[derive(Debug, Serialize, Clone)]
pub struct PriceLevel {
    /// Price in cents.
    pub price_cents: i64,
    /// Total quantity across all orders at this price.
    pub total_quantity: i32,
    /// Number of individual orders at this price.
    pub order_count: i32,
}

/// 24-hour ticker response.
#[derive(Debug, Serialize)]
pub struct TickerResponse {
    /// The asset.
    pub asset_id: Uuid,
    /// Last traded price in cents.
    pub last_price_cents: Option<i64>,
    /// Absolute price change in last 24h (cents).
    pub change_24h_cents: i64,
    /// Percentage change in last 24h (display-only; f64 OK for non-monetary percentages).
    pub change_24h_pct: f64,
    /// Highest price in last 24h (cents).
    pub high_24h_cents: Option<i64>,
    /// Lowest price in last 24h (cents).
    pub low_24h_cents: Option<i64>,
    /// Total tokens traded in last 24h.
    pub volume_24h_tokens: i64,
    /// Total volume in cents in last 24h.
    pub volume_24h_cents: i64,
    /// Number of trades in last 24h.
    pub trade_count_24h: i64,
}

/// A recent trade for the trade tape.
#[derive(Debug, Serialize)]
pub struct RecentTrade {
    /// Trade ID.
    pub id: Uuid,
    /// Execution price in cents.
    pub price_cents: i64,
    /// Quantity traded.
    pub quantity: i32,
    /// Total value in cents.
    pub total_cents: i64,
    /// Whether the trade was buyer-initiated (true) or seller-initiated (false).
    pub is_buyer_maker: bool,
    /// When the trade was executed.
    pub executed_at: DateTime<Utc>,
}

// ═══════════════════════════════════════════════════════════════
// ── INTERNAL ENGINE TYPES ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// A parsed Redis Sorted Set member with extracted fields.
///
/// The Redis member format is: `"order:{id}:{user_id}:{qty}:{timestamp_epoch}"`.
/// The score is the price in cents.
#[derive(Debug, Clone)]
pub struct ParsedOrderMember {
    /// The order's UUID.
    pub order_id: Uuid,
    /// The user who placed the order.
    pub user_id: Uuid,
    /// Remaining quantity at time of insertion.
    pub quantity: i32,
    /// Unix timestamp (seconds) of order creation.
    pub timestamp: i64,
    /// Price in cents (from Redis ZSET score).
    pub price_cents: i64,
    /// The raw member string for ZREM operations.
    pub raw_member: String,
}

impl ParsedOrderMember {
    /// Parse a Redis member string + score into a structured type.
    ///
    /// Returns `None` if the format is invalid (logs a warning).
    pub fn parse(member: &str, score: i64) -> Option<Self> {
        let parts: Vec<&str> = member.split(':').collect();
        if parts.len() != 5 || parts[0] != "order" {
            tracing::warn!("Invalid Redis orderbook member format: {}", member);
            return None;
        }
        Some(Self {
            order_id: Uuid::parse_str(parts[1]).ok()?,
            user_id: Uuid::parse_str(parts[2]).ok()?,
            quantity: parts[3].parse().ok()?,
            timestamp: parts[4].parse().ok()?,
            price_cents: score,
            raw_member: member.to_string(),
        })
    }
}

/// Internal match event — produced by the matching engine, consumed by the settlement pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchEvent {
    /// The sell (ask) order ID.
    pub ask_order_id: Uuid,
    /// The buy (bid) order ID.
    pub bid_order_id: Uuid,
    /// The asset being traded.
    pub asset_id: Uuid,
    /// The seller's user ID.
    pub seller_user_id: Uuid,
    /// The buyer's user ID.
    pub buyer_user_id: Uuid,
    /// Matched execution price in cents.
    pub match_price_cents: i64,
    /// Number of tokens matched.
    pub match_quantity: i32,
    /// When the match occurred.
    pub timestamp: DateTime<Utc>,
}

/// Reasons an order can be rejected during validation.
#[derive(Debug)]
pub enum OrderRejection {
    /// User's available balance is insufficient.
    InsufficientBalance {
        /// Available balance in cents.
        available_cents: i64,
        /// Required amount in cents.
        required_cents: i64,
    },
    /// User doesn't own enough tokens.
    InsufficientTokens {
        /// Tokens the user currently has available.
        owned: i32,
        /// Tokens requested to sell.
        requested: i32,
    },
    /// Order would exceed concentration limit.
    ConcentrationLimit {
        /// Current ownership percentage.
        current_pct: f64,
        /// Requested additional percentage.
        requested_pct: f64,
        /// Maximum allowed percentage.
        max_pct: f64,
    },
    /// Order is large enough to require admin review.
    RequiresAdminReview {
        /// Order percentage of total supply.
        order_pct: f64,
        /// Order total value in cents.
        order_value_cents: i64,
    },
    /// Order total is below the minimum order size.
    BelowMinimum {
        /// Minimum order value in cents.
        min_cents: i64,
        /// Actual order value in cents.
        actual_cents: i64,
    },
    /// Quantity must be >= 1.
    InvalidQuantity,
    /// Price must be > 0 for limit orders.
    InvalidPrice,
    /// Asset is not available for trading.
    AssetNotTradable,
    /// User has not completed KYC.
    KycNotApproved,
    /// Self-trading (wash trading) blocked.
    SelfTradeBlocked,
    /// Too many open orders for this asset.
    TooManyOpenOrders {
        /// Maximum allowed.
        max: i32,
        /// Current count.
        current: i32,
    },
    /// This idempotency key has already been used.
    DuplicateIdempotencyKey,
    /// 2FA step-up required for this trade value.
    TwoFactorRequired,
    /// Rate limit exceeded.
    RateLimited {
        /// Seconds until rate limit resets.
        retry_after_secs: u64,
    },
}

impl OrderRejection {
    /// Convert to a user-friendly error message.
    ///
    /// SECURITY: These are controlled, pre-defined strings — never DB error details.
    pub fn to_user_message(&self) -> String {
        match self {
            Self::InsufficientBalance {
                available_cents,
                required_cents,
            } => {
                format!(
                    "Insufficient balance. Available: ${:.2}, Required: ${:.2}",
                    *available_cents as f64 / 100.0,
                    *required_cents as f64 / 100.0
                )
            }
            Self::InsufficientTokens { owned, requested } => {
                format!(
                    "Insufficient tokens. You own {}, requested {}",
                    owned, requested
                )
            }
            Self::ConcentrationLimit { max_pct, .. } => {
                format!(
                    "Order would exceed the maximum concentration limit of {:.0}%",
                    max_pct
                )
            }
            Self::RequiresAdminReview { .. } => {
                "This order requires admin approval due to its size. You will be notified once reviewed.".into()
            }
            Self::BelowMinimum { min_cents, .. } => {
                format!(
                    "Order value must be at least ${:.2}",
                    *min_cents as f64 / 100.0
                )
            }
            Self::InvalidQuantity => "Quantity must be at least 1 token".into(),
            Self::InvalidPrice => "Price must be greater than $0.00".into(),
            Self::AssetNotTradable => {
                "This asset is currently in its primary funding phase and cannot yet be traded on the secondary market. Please review our <a href='/terms' target='_blank' style='text-decoration:underline'>Terms of Use</a>".into()
            }
            Self::KycNotApproved => "KYC verification is required to trade".into(),
            Self::SelfTradeBlocked => "You cannot trade against your own orders".into(),
            Self::TooManyOpenOrders { max, .. } => {
                format!("Maximum {} open orders per asset allowed", max)
            }
            Self::DuplicateIdempotencyKey => "This order has already been submitted".into(),
            Self::TwoFactorRequired => {
                "Two-factor authentication required for this trade".into()
            }
            Self::RateLimited { retry_after_secs } => {
                format!(
                    "Too many orders. Please try again in {} seconds",
                    retry_after_secs
                )
            }
        }
    }

    /// Convert an `OrderRejection` into the appropriate `AppError`.
    pub fn into_app_error(self) -> crate::error::AppError {
        match self {
            Self::InsufficientBalance {
                available_cents,
                required_cents,
            } => crate::error::AppError::InsufficientBalance {
                available_cents,
                required_cents,
            },
            Self::InsufficientTokens { owned, requested } => {
                crate::error::AppError::InsufficientTokens {
                    available: owned,
                    required: requested,
                }
            }
            Self::TwoFactorRequired => crate::error::AppError::TwoFactorRequired,
            Self::SelfTradeBlocked => crate::error::AppError::WashTradingBlocked,
            Self::RateLimited { retry_after_secs } => {
                crate::error::AppError::RateLimited(retry_after_secs)
            }
            other => crate::error::AppError::OrderRejected(other.to_user_message()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// ── FEE CALCULATION ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Calculate fee in cents from a total in cents and a fee rate in basis points.
///
/// Uses integer math only — no floating point.
/// - 500 BPS = 5.00%
/// - Negative BPS are clamped to 0.
/// - Fee can never exceed the total.
pub fn calculate_fee_cents(total_cents: i64, fee_bps: i32) -> i64 {
    let bps = fee_bps.max(0) as i64;
    let fee = total_cents.saturating_mul(bps) / 10_000;
    fee.min(total_cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fee Calculation Tests ──────────────────────────────────

    #[test]
    fn test_fee_normal() {
        // $100.00 * 5% = $5.00
        assert_eq!(calculate_fee_cents(10_000, 500), 500);
    }

    #[test]
    fn test_fee_small_amount() {
        // $0.01 * 5% = $0.00 (rounds down in integer math)
        assert_eq!(calculate_fee_cents(1, 500), 0);
    }

    #[test]
    fn test_fee_zero_bps() {
        assert_eq!(calculate_fee_cents(10_000, 0), 0);
    }

    #[test]
    fn test_fee_100_percent() {
        // 100% fee = total
        assert_eq!(calculate_fee_cents(10_000, 10_000), 10_000);
    }

    #[test]
    fn test_fee_negative_bps_clamps_to_zero() {
        assert_eq!(calculate_fee_cents(10_000, -100), 0);
    }

    #[test]
    fn test_fee_exceeds_total_clamped() {
        // 200% fee → capped at total
        assert_eq!(calculate_fee_cents(10_000, 20_000), 10_000);
    }

    #[test]
    fn test_fee_zero_total() {
        assert_eq!(calculate_fee_cents(0, 500), 0);
    }

    // ── ParsedOrderMember Tests ───────────────────────────────

    #[test]
    fn test_parse_valid_member() {
        let member = "order:550e8400-e29b-41d4-a716-446655440000:660e8400-e29b-41d4-a716-446655440001:10:1700000000";
        let parsed = ParsedOrderMember::parse(member, 10500).expect("Should parse valid member");
        assert_eq!(parsed.quantity, 10);
        assert_eq!(parsed.price_cents, 10500);
        assert_eq!(parsed.timestamp, 1700000000);
    }

    #[test]
    fn test_parse_invalid_member_format() {
        assert!(ParsedOrderMember::parse("invalid", 100).is_none());
        assert!(ParsedOrderMember::parse("order:abc:def", 100).is_none());
        assert!(ParsedOrderMember::parse("", 100).is_none());
    }

    #[test]
    fn test_parse_wrong_prefix() {
        let member = "trade:550e8400-e29b-41d4-a716-446655440000:660e8400-e29b-41d4-a716-446655440001:10:1700000000";
        assert!(ParsedOrderMember::parse(member, 100).is_none());
    }

    // ── MarketOrder Tests ─────────────────────────────────────

    #[test]
    fn test_remaining_quantity() {
        let order = make_test_order(100, 30);
        assert_eq!(order.remaining_quantity(), 70);
    }

    #[test]
    fn test_is_filled() {
        let mut order = make_test_order(100, 100);
        assert!(order.is_filled());

        order.quantity_filled = 50;
        assert!(!order.is_filled());
    }

    #[test]
    fn test_is_active() {
        let mut order = make_test_order(100, 0);
        order.status = "open".to_string();
        assert!(order.is_active());

        order.status = "partially_filled".to_string();
        assert!(order.is_active());

        order.status = "filled".to_string();
        assert!(!order.is_active());

        order.status = "cancelled".to_string();
        assert!(!order.is_active());
    }

    #[test]
    fn test_total_value_cents() {
        let order = make_test_order(10, 0);
        // price_cents=10500, quantity=10 → 105000
        assert_eq!(order.total_value_cents(), 105_000);
    }

    #[test]
    fn test_redis_member_format() {
        let order = make_test_order(100, 30);
        let member = order.redis_member();
        assert!(member.starts_with("order:"));
        // Should contain remaining qty (70)
        let parts: Vec<&str> = member.split(':').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[3], "70"); // remaining = 100 - 30
    }

    // ── OrderSide Tests ───────────────────────────────────────

    #[test]
    fn test_order_side_parse() {
        assert_eq!(OrderSide::parse("buy"), Some(OrderSide::Buy));
        assert_eq!(OrderSide::parse("sell"), Some(OrderSide::Sell));
        assert_eq!(OrderSide::parse("BUY"), None);
        assert_eq!(OrderSide::parse(""), None);
    }

    #[test]
    fn test_order_side_as_str() {
        assert_eq!(OrderSide::Buy.as_str(), "buy");
        assert_eq!(OrderSide::Sell.as_str(), "sell");
    }

    // ── OrderRejection Tests ──────────────────────────────────

    #[test]
    fn test_rejection_messages_are_safe() {
        // Ensure no message contains SQL or internal error info
        let rejections = vec![
            OrderRejection::InsufficientBalance {
                available_cents: 5000,
                required_cents: 10000,
            },
            OrderRejection::InsufficientTokens {
                owned: 5,
                requested: 10,
            },
            OrderRejection::BelowMinimum {
                min_cents: 1000,
                actual_cents: 500,
            },
            OrderRejection::InvalidQuantity,
            OrderRejection::AssetNotTradable,
            OrderRejection::KycNotApproved,
            OrderRejection::SelfTradeBlocked,
        ];

        for r in rejections {
            let msg = r.to_user_message();
            assert!(!msg.contains("SELECT"), "SQL leaked in message: {}", msg);
            assert!(!msg.contains("sqlx"), "Internal detail leaked: {}", msg);
            assert!(!msg.is_empty(), "Empty rejection message");
        }
    }

    // ── Test Helpers ──────────────────────────────────────────

    fn make_test_order(quantity: i32, filled: i32) -> MarketOrder {
        MarketOrder {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            asset_id: Uuid::new_v4(),
            side: "buy".to_string(),
            order_type: "limit".to_string(),
            price_cents: 10500,
            quantity,
            quantity_filled: filled,
            status: "open".to_string(),
            idempotency_key: None,
            cancel_reason: None,
            expires_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// JSON payload for an asset listed on the secondary marketplace.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryAsset {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub r#type: String,
    pub location: String,
    pub country: String,
    pub images: Vec<String>,
    pub price: i64,
    pub change24h: f64,
    pub volume24h: i64,
    pub roi: f64,
    pub occupancy: i32,
    pub sell_orders: i64,
    pub buy_interest: i64,
    pub total_supply: i32,
    pub sparkline: Vec<f64>,
    pub description: Option<String>,
    pub property_value: i64,
    pub land_size: Option<String>,
    pub building_size_sqm: Option<String>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub rent_status: Option<String>,
    pub location_desc: Option<String>,
    pub lease_type: Option<String>,
    pub property_type: Option<String>,
    pub funding_status: String,
    pub tokens_available: i32,
    pub funding_progress_pct: f64,
    pub term_months: Option<i32>,
    pub capital_appreciation_bps: Option<i32>,
}
