/// Wallet domain models – strict types for all wallet entities.
///
/// These models replace the raw SQL tuples that were used previously and provide:
/// - Type-safe serialization via `serde` (for JSON APIs + MiniJinja templates)
/// - Direct DB mapping via `sqlx::FromRow`
/// - Exhaustive enums for status/type fields
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Enums ──────────────────────────────────────────────────────

/// All possible wallet transaction types.
/// Maps to the CHECK constraint on `wallet_transactions.type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Purchase,
    Sale,
    Dividend,
    Reward,
    Refund,
    Fee,
}

impl TransactionType {
    /// Parse from a raw DB string. Falls back to `Deposit` for unknown values to avoid panics.
    pub fn from_db(s: &str) -> Self {
        match s {
            "deposit" => Self::Deposit,
            "withdrawal" => Self::Withdrawal,
            "purchase" => Self::Purchase,
            "sale" => Self::Sale,
            "dividend" => Self::Dividend,
            "reward" => Self::Reward,
            "refund" => Self::Refund,
            "fee" => Self::Fee,
            _ => {
                tracing::warn!("Unknown transaction type '{}', defaulting to Deposit", s);
                Self::Deposit
            }
        }
    }

    /// Display label for the UI (e.g. "Deposit", "Withdrawal")
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Deposit => "Deposit",
            Self::Withdrawal => "Withdraw",
            Self::Purchase => "Investment",
            Self::Sale => "Sale",
            Self::Dividend => "Rent Paid",
            Self::Reward => "Reward",
            Self::Refund => "Refund",
            Self::Fee => "Fee",
        }
    }

    /// CSS-safe icon key used by the frontend to pick the right SVG.
    pub fn icon_key(&self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdrawal => "withdrawal",
            Self::Purchase => "investment",
            Self::Sale => "sale",
            Self::Dividend => "dividend",
            Self::Reward => "reward",
            Self::Refund => "refund",
            Self::Fee => "fee",
        }
    }
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_label())
    }
}

/// All possible transaction statuses.
/// Maps to the CHECK constraint on `wallet_transactions.status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl TransactionStatus {
    pub fn from_db(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "processing" => Self::Processing,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => {
                tracing::warn!("Unknown transaction status '{}', defaulting to Pending", s);
                Self::Pending
            }
        }
    }

    /// Display label for the UI
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Pending | Self::Processing => "In process",
            Self::Completed => "Completed",
            Self::Failed | Self::Cancelled => "Declined",
        }
    }

    /// CSS class suffix for the status badge.
    /// Emits both legacy class and new design-system class for gradual migration.
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Pending | Self::Processing => "status-in-process ds-badge ds-badge--warning",
            Self::Completed => "status-completed ds-badge ds-badge--success",
            Self::Failed | Self::Cancelled => "status-declined ds-badge ds-badge--danger",
        }
    }
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_label())
    }
}

/// Wallet type enum (cash or rewards).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletType {
    Cash,
    Rewards,
}

impl WalletType {
    pub fn from_db(s: &str) -> Self {
        match s {
            "cash" => Self::Cash,
            "rewards" => Self::Rewards,
            _ => {
                tracing::warn!("Unknown wallet type '{}', defaulting to Cash", s);
                Self::Cash
            }
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Cash => "Cash balance",
            Self::Rewards => "Rewards balance",
        }
    }
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_label())
    }
}

// ─── DB Row Structs ─────────────────────────────────────────────

// ─── Serializable Models (JSON API + MiniJinja) ─────────────────

/// A fully typed, display-ready wallet transaction.
/// Used for both the MiniJinja template context AND the JSON API.
#[derive(Debug, Clone, Serialize)]
pub struct WalletTransaction {
    /// 0-indexed row number for template IDs
    pub index: usize,
    /// Transaction type enum
    pub tx_type: TransactionType,
    /// Display label (e.g. "Deposit", "Investment")
    pub tx_type_label: String,
    /// Icon key for SVG selection
    pub tx_type_icon: String,
    /// Status enum
    pub status: TransactionStatus,
    /// Display label (e.g. "Completed", "In process")
    pub status_label: String,
    /// CSS class for the status badge
    pub status_css: String,
    /// Formatted date (e.g. "08 Feb 2026")
    pub date_display: String,
    /// ISO date for `datetime` attributes
    pub date_iso: String,
    /// Wallet type display (e.g. "Cash balance")
    pub wallet_label: String,
    /// Raw amount in cents (signed)
    pub amount_cents: i64,
    /// Formatted amount string (e.g. "USD 175.00")
    pub amount_display: String,
    /// Prefix sign ("+" or "-")
    pub amount_prefix: String,
    /// CSS class for the amount ("amount-positive" or "amount-negative")
    pub amount_css: String,
}

/// The complete wallet page context passed to the MiniJinja template.
#[derive(Debug, Serialize)]
pub struct WalletPageContext {
    /// Formatted cash balance (e.g. "USD 2,732.00")
    pub cash_balance: String,
    /// Formatted rewards balance
    pub rewards_balance: String,
    /// Formatted asset balance
    pub asset_balance: String,
    /// Raw cents values for programmatic use
    pub cash_cents: i64,
    pub rewards_cents: i64,
    pub asset_cents: i64,
    /// List of display-ready transactions
    pub transactions: Vec<WalletTransaction>,
    /// Whether there are any transactions
    pub has_transactions: bool,
    /// Desktop card payment methods HTML fragments
    pub cards_html: String,
    /// Desktop bank payment methods HTML fragments
    pub banks_html: String,
    /// Mobile card payment methods HTML fragments
    pub mobile_cards_html: String,
    /// Mobile bank payment methods HTML fragments
    pub mobile_banks_html: String,
    /// Whether there are any cards
    pub has_cards: bool,
    /// Whether there are any banks
    pub has_banks: bool,
    /// `<option>` tags for the deposit/withdraw modals
    pub payment_method_options: String,
    /// Stripe publishable key for the frontend
    pub stripe_publishable_key: String,
}

/// JSON API response for GET /api/wallet/balance
#[derive(Debug, Serialize)]
pub struct WalletBalanceResponse {
    pub cash_cents: i64,
    pub rewards_cents: i64,
    pub asset_cents: i64,
    pub cash_display: String,
    pub rewards_display: String,
    pub asset_display: String,
}

/// Single transaction entry in the JSON API response
#[derive(Debug, Serialize)]
pub struct WalletTransactionApiEntry {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub status: String,
    pub amount_cents: i64,
    pub amount_usd: f64,
    pub wallet_type: String,
    pub created_at: String,
}

/// JSON API response for GET /api/wallet/transactions
#[derive(Debug, Serialize)]
pub struct WalletTransactionsResponse {
    pub transactions: Vec<WalletTransactionApiEntry>,
    pub count: usize,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}
