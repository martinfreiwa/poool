export interface InvestmentItem {
    id: string; // Uuid
    asset_id: string; // Uuid
    asset_title: string;
    asset_slug: string;
    tokens_owned: number; // i32
    purchase_value_cents: number; // i64
    current_value_cents: number; // i64
    total_rental_cents: number; // i64
    appreciation_pct_bps: number; // i32
    status: string;
    payout_expected_at: string | null;
    purchased_at: string;
}

export interface AnnualLimit {
    annual_limit_cents: number;
    invested_12m_cents: number;
    available_cents: number;
    limit_year: number;
}

export interface PortfolioResponse {
    investments: InvestmentItem[];
    total_value_cents: number;
    total_purchase_cents: number;
    total_rental_cents: number;
    total_appreciation_cents: number;
    monthly_income_cents: number;
    occupancy_rate_bps: number;
    annual_yield_bps: number;
    investment_count: number;
    annual_limit: AnnualLimit | null;
}
