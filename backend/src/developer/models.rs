/// Developer dashboard data models.
use serde::{Deserialize, Serialize};

/// Form payload for creating a new draft asset.
/// Financial fields are optional with defaults so that "Save & Exit" works
/// even when the user hasn't filled in pricing yet.
#[derive(Debug, Deserialize)]
pub struct CreateDraftAsset {
    #[serde(default = "default_draft_title")]
    pub title: String,
    #[serde(default = "default_asset_type")]
    pub asset_type: String,
    // Property details
    pub property_type: Option<String>,
    pub area: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub lease_type: Option<String>,
    pub lease_term_years: Option<i32>,
    pub land_size_sqm: Option<f64>,
    pub building_size_sqm: Option<f64>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub construction_status: Option<String>,
    pub year_built: Option<i32>,
    // Financials — default to 0 so Save & Exit doesn't fail on partial forms
    #[serde(default)]
    pub total_value_cents: i64,
    #[serde(default = "default_token_price")]
    pub token_price_cents: i64,
    #[serde(default = "default_tokens_total")]
    pub tokens_total: i64,
}

fn default_draft_title() -> String {
    "Untitled Draft".to_string()
}

fn default_asset_type() -> String {
    "real_estate".to_string()
}

fn default_token_price() -> i64 {
    100 // $1 minimum in cents
}

fn default_tokens_total() -> i64 {
    1
}

/// A single metric card on the developer dashboard.
#[derive(Debug, Serialize)]
pub struct DeveloperMetric {
    pub label: String,
    pub value: String,
    pub change_pct: f64,
    pub trend: String,          // "up", "down", or "neutral"
    pub change_display: String, // e.g. "12%"
    pub timeframe: String,      // e.g. "vs last mth"
}

/// A row in the "Top Assets" table.
#[derive(Debug, Serialize)]
pub struct DeveloperTopAsset {
    pub index: usize,
    pub id: String,
    pub title: String,
    pub cover_image_url: String,
    pub total_sales_display: String,
    pub total_sales_cents: i64,
    pub sales_change_pct: f64,
    pub sales_trend: String,
    pub views: i64,
    pub add_to_cart_count: i64,
    pub checkout_starts: i64,
    pub saved_count: i64,
    pub conversion_rate: f64,
    pub conversion_display: String,
    pub funding_pct: f64,
    pub funding_display: String,
    pub status: String,
    pub city: Option<String>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub size_sqm: Option<String>,
    pub total_value_display: String,
    pub total_value_cents: i64,
    pub amount_remaining_display: String,
    pub amount_remaining_cents: i64,
    pub is_rented: bool,
    pub country: Option<String>,
    pub lease_type: Option<String>,
    pub lease_term_years: Option<i32>,
    pub capital_appreciation_bps: Option<i32>,
    pub annual_yield_bps: Option<i32>,
}

/// The full developer dashboard stats payload (API + SSR).
#[derive(Debug, Serialize)]
pub struct DeveloperDashboardStats {
    pub total_assets: i64,
    pub total_funding_target_cents: i64,
    pub total_funding_target_display: String,
    pub total_sales_cents: i64,
    pub total_sales_display: String,
    pub amount_remaining_cents: i64,
    pub amount_remaining_display: String,
    pub total_investors: i64,
    pub total_views: i64,
    pub checkout_starts: i64,
    pub add_to_cart_count: i64,
    pub saved_properties: i64,
    pub new_investors: i64,
    pub avg_conversion_rate: f64,
    pub sold_out_ratio: f64,
    pub avg_investment_cents: i64,
    pub avg_investment_display: String,
    pub metrics: Vec<DeveloperMetric>,
    pub top_assets: Vec<DeveloperTopAsset>,
    pub attention_assets: Vec<DeveloperTopAsset>,
    pub chart_percentage_display: String, // e.g. "+17.6%" or "0%"
    pub chart_trend: String,              // "up", "down", or "neutral"
    pub chart_period_label: String,
    pub chart_y_axis_labels: Vec<String>,
    pub chart_x_axis_labels: Vec<String>,
    pub chart_line_path: String,
    pub chart_area_path: String,
    pub chart_has_data: bool,
}

/// Form payload for updating a draft asset (Steps 3 & 4 content).
#[derive(Debug, Deserialize)]
pub struct UpdateDraftAsset {
    // Title
    pub title: Option<String>,
    // Content fields (Step 4)
    pub short_description: Option<String>,
    pub description: Option<String>,
    pub location_description: Option<String>,
    pub google_maps_url: Option<String>,
    pub video_url: Option<String>,
    // Financial fields (Step 4 — stored as bps, e.g. 10% = 1000)
    pub annual_yield_bps: Option<i32>,
    pub capital_appreciation_bps: Option<i32>,
    pub occupancy_rate_bps: Option<i32>,
    pub investor_share_bps: Option<i32>,
    // Amenities
    pub amenities: Option<serde_json::Value>,
    // Step tracking
    pub submission_step: Option<i32>,
    // Step 2 Fields
    pub property_type: Option<String>,
    pub area: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub lease_type: Option<String>,
    pub lease_term_years: Option<i32>,
    pub land_size_sqm: Option<f64>,
    pub building_size_sqm: Option<f64>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub construction_status: Option<String>,
    pub year_built: Option<i32>,
    pub total_value_cents: Option<i64>,
    pub token_price_cents: Option<i64>,
    pub tokens_total: Option<i32>,
}

/// A row in the developer drafts listing (Submissions page).
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct DraftListItem {
    pub id: String,
    pub title: String,
    pub asset_type: String,
    pub cover_image_url: Option<String>,
    pub submission_step: i32,
    pub project_status: String,
    pub updated_at: String,
}
