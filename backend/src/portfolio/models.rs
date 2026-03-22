use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct InvestmentItem {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub asset_title: String,
    pub asset_slug: String,
    pub cover_image: Option<String>,
    pub tokens_owned: i32,
    pub purchase_value_cents: i64,
    pub current_value_cents: i64,
    pub total_rental_cents: i64,
    pub appreciation_pct_bps: i32,
    pub status: String,
    pub payout_expected_at: Option<String>,
    pub purchased_at: String,
    pub is_within_48h: bool,
    pub chain_contract_address: Option<String>,
    pub chain_tx_hash: Option<String>,
}


#[derive(Serialize)]
pub struct AnnualLimit {
    pub annual_limit_cents: i64,
    pub invested_12m_cents: i64,
    pub available_cents: i64,
    pub limit_year: i32,
}

#[derive(Serialize)]
pub struct PortfolioResponse {
    pub investments: Vec<InvestmentItem>,
    pub total_value_cents: i64,
    pub total_purchase_cents: i64,
    pub total_rental_cents: i64,
    pub total_appreciation_cents: i64,
    pub monthly_income_cents: i64,
    pub occupancy_rate_bps: i32,
    pub annual_yield_bps: i32,
    pub investment_count: usize,
    pub annual_limit: Option<AnnualLimit>,
}
