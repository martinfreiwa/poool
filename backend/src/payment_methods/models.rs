use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow)]
pub struct PaymentMethod {
    pub id: Uuid,
    pub user_id: Uuid,
    pub method_type: String,

    // Token details
    pub processor_type: Option<String>,
    pub processor_token: Option<String>,
    pub customer_id: Option<String>,

    // Extracted / Display details
    pub brand: Option<String>,
    pub last_four: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub holder_name: Option<String>,

    // Bank specific details
    pub routing_number: Option<String>,
    pub bank_country: Option<String>,
    pub bank_system: Option<String>, // e.g. "ach", "sepa", "bacs", "bsb", "ifsc", "swift"

    // Common
    pub label: Option<String>,
    pub is_default: bool,
    pub status: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AttachCardTokenForm {
    pub stripe_payment_method_id: String,
    pub holder_name: String,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddBankForm {
    pub bank_name: String,
    pub account_holder_name: String,
    pub account_number: String,
    pub routing_code: String,
    pub bank_country: Option<String>,
    pub bank_system: Option<String>, // "ach" | "bacs" | "sepa" | "bsb" | "ifsc" | "swift"
    pub label: Option<String>,
}
