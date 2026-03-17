use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RewardsOverview {
    pub total_balance: i64,
    pub cashback: i64,
    pub referrals: i64,
    pub promotions: i64,
    pub tier_name: String,
    pub tier_target: Option<String>,
    pub tier_target_amount: Option<i64>,
    pub invested_12m: i64,
    pub progress_pct: i32,
    pub referral_code: Option<String>,
    pub referral_url: Option<String>,
    pub total_clicks: i64,
    pub total_signups: i64,
    pub qualified_investors: i64,
    pub network_total_in: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TierInfo {
    pub id: i32,
    pub name: String,
    pub min_invest: i64,
    pub badge_color: String,
    pub sort_order: i32,
    pub cashback_pct: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CampaignMetrics {
    pub subid: String,
    pub clicks: i64,
    pub signups: i64,
    pub qualified: i64,
    pub revenue_cents: i64,
    pub cvr: f64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PayoutSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub payment_method: String,
    pub account_email: Option<String>,
    pub full_name: Option<String>,
    pub street_address: Option<String>,
    pub postcode: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub vat_number: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CommissionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub amount_cents: i64,
    pub payment_method: String,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SavePayoutSettingsForm {
    pub payment_method: String,
    pub account_email: Option<String>,
    pub full_name: Option<String>,
    pub street_address: Option<String>,
    pub postcode: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub vat_number: Option<String>,
}
