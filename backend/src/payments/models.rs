/// Payment data models – Rust structs for deposits, invoices, and checkout.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Database Models ───────────────────────────────────────────

/// A deposit request (intent) from the `deposit_requests` table.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct DepositRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: String,
    pub amount_cents: i64,
    pub provider: String,
    pub provider_reference: Option<String>,
    pub status: String,
    pub payment_method: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An invoice from the `invoices` table.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Invoice {
    pub id: Uuid,
    pub invoice_number: String,
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub company_entity: String,
    pub subtotal_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
    pub currency: String,
    pub pdf_url: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// An order from the `orders` table (extended).
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub order_number: String,
    pub total_cents: i64,
    pub status: String,
    pub payment_method: Option<String>,
    pub payment_ref_id: Option<String>,
    pub currency: String,
    pub payment_currency: Option<String>,
    pub fx_rate: Option<sqlx::types::Decimal>,
    pub fx_provider: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ─── Form Data (from HTMX / JSON requests) ────────────────────

/// Deposit initiation form.
#[derive(Debug, Deserialize)]
pub struct InitiateDepositForm {
    pub currency: String, // "USD" or "IDR"
    pub amount: String,   // e.g. "1000" or "15000000"
}

/// Webhook payload from payment provider (simplified, generic).
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub provider_reference: String,
    pub status: String, // "paid", "failed", "expired"
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub signature: Option<String>,
}

/// Checkout form submitted by the user.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CheckoutForm {
    pub payment_currency: Option<String>, // "USD" or "IDR" – which wallet to pay from
}

// ─── Response types ────────────────────────────────────────────

/// Deposit response returned to the UI.
#[derive(Debug, Serialize)]
pub struct DepositResponse {
    pub deposit_id: Uuid,
    pub provider: String,
    pub provider_reference: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub status: String,
    pub instructions: String,
}

/// Checkout result returned to the UI.
#[derive(Debug, Serialize)]
pub struct CheckoutResult {
    pub order_id: Uuid,
    pub order_number: String,
    pub total_cents: i64,
    pub currency: String,
    pub items_purchased: i32,
    pub invoice_number: Option<String>,
}

/// Invoice view for the frontend.
#[derive(Debug, Serialize)]
pub struct InvoiceView {
    pub id: Uuid,
    pub invoice_number: String,
    pub total_cents: i64,
    pub currency: String,
    pub status: String,
    pub issued_at: String,
    pub pdf_url: Option<String>,
}
