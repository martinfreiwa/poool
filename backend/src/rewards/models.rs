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
    /// Reward amount in cents that the referred friend receives on signup.
    pub friend_reward_cents: i64,
    /// Reward amount in cents that the referrer receives when the friend qualifies.
    pub user_reward_cents: i64,
    /// Minimum investment in cents required for a referral to qualify.
    pub investment_required_cents: i64,
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

#[derive(Debug, Serialize)]
pub struct AffiliateSettingsResponse {
    pub tax_class: Option<String>,
    pub tax_id_masked: Option<String>,
    pub tax_name: Option<String>,
    pub vat_number: Option<String>,
    pub payout_method: String,
    pub tax_status: String,
    pub payout_status: String,
    pub payout_hold_reason: Option<String>,
    pub tax_document_on_file: bool,
    pub tax_ready: bool,
}

#[derive(Debug, Deserialize)]
pub struct SaveAffiliateSettingsForm {
    pub tax_class: String,
    pub tax_id: Option<String>,
    pub tax_name: String,
    pub vat_number: Option<String>,
    pub payout_method: String,
    pub tax_certified: bool,
}

#[derive(Debug, Deserialize)]
pub struct SubmitOnboardingForm {
    pub exam_passed: bool,
    pub status: Option<String>,
    pub traffic_source: String,
    pub audience_size: String,
    pub main_url: String,
    pub phone_number: String,
    pub tax_id: String,
    pub company_name: Option<String>,
    pub accepted_policies: Vec<String>,
    /// Server-side exam answer validation — keys are "q1".."q5", values are the selected answer.
    pub exam_answers: Option<std::collections::HashMap<String, String>>,
}

#[derive(serde::Deserialize, Debug)]
pub struct PostbackPayload {
    pub postback_url: Option<String>,
}
