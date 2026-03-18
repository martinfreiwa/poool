use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct MarketplaceAsset {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub asset_type: String,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub total_value_cents: i64,
    pub token_price_cents: i64,
    pub tokens_total: i32,
    pub tokens_available: i32,
    pub annual_yield_bps: Option<i32>,
    pub capital_appreciation_bps: Option<i32>,
    pub funding_status: String,
    pub cover_image_url: Option<String>,
    pub bedrooms: Option<i32>,
    pub lease_type: Option<String>,
    pub term_months: Option<i32>,
    pub area: Option<String>,
    pub land_size_sqm: Option<rust_decimal::Decimal>,
    /// Number of unique investors who completed orders for this asset
    pub investor_count: Option<i64>,
}

/// Template-friendly representation with pre-computed display values.
/// All monetary values are formatted as strings for direct use in templates.
#[derive(Debug, Serialize)]
pub struct PropertyDisplayData {
    // Pass through the raw asset
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub asset_type: String,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub bedrooms: Option<i32>,
    pub lease_type: Option<String>,
    pub term_months: Option<i32>,
    pub cover_image_url: Option<String>,
    pub funding_status: String,

    // Pre-computed display values
    pub total_value_usd: String, // e.g. "1,150,000"
    pub total_value_cents: i64,
    pub tokens_total: i32,
    pub tokens_available: i32,
    pub tokens_sold: i32,
    pub funded_percentage: i32,            // e.g. 84
    pub funded_percentage_display: String, // e.g. "84"
    pub available_usd: String,             // e.g. "183,950"
    pub investor_count: i64,
    pub price_per_sqft: String,                 // e.g. "1,137"
    pub annual_yield_percent: String,           // e.g. "7.45"
    pub capital_appreciation_percent: String,   // e.g. "5.49"
    pub projected_return_percent: String,       // capital_appreciation + annual_yield
    pub five_year_total_return_percent: String, // approx
    pub annualised_net_return_percent: String,
    pub land_size_sqm: Option<String>,
    pub land_size_sqft: Option<String>,
}

impl PropertyDisplayData {
    pub fn from_asset(asset: &MarketplaceAsset) -> Self {
        let tokens_sold = asset.tokens_total - asset.tokens_available;
        let funded_pct = if asset.tokens_total > 0 {
            ((tokens_sold as f64 / asset.tokens_total as f64) * 100.0) as i32
        } else {
            0
        };

        let available_cents = asset.tokens_available as i64 * asset.token_price_cents;
        let total_value_dollars = asset.total_value_cents / 100;
        let available_dollars = available_cents / 100;

        let annual_yield_bps = asset.annual_yield_bps.unwrap_or(0);
        let cap_appreciation_bps = asset.capital_appreciation_bps.unwrap_or(0);

        let annual_yield_pct = annual_yield_bps as f64 / 100.0;
        let cap_appreciation_pct = cap_appreciation_bps as f64 / 100.0;

        // Projected return = yield + appreciation
        let projected_return = annual_yield_pct + cap_appreciation_pct;

        // 5 year total return: compound (1 + annual_return)^5 - 1
        let annual_return = projected_return / 100.0;
        let five_year_return = ((1.0 + annual_return).powi(5) - 1.0) * 100.0;

        // Annualised net return (after ~2% fees estimate)
        let annualised_net = annual_yield_pct * 0.85 + cap_appreciation_pct;

        // Price per sqft
        let land_sqm = asset
            .land_size_sqm
            .map(|d| {
                use rust_decimal::prelude::ToPrimitive;
                d.to_f64().unwrap_or(0.0)
            })
            .unwrap_or(0.0);
        let land_sqft = land_sqm * 10.7639;
        let price_per_sqft = if land_sqft > 0.0 {
            (total_value_dollars as f64 / land_sqft) as i64
        } else {
            0
        };

        PropertyDisplayData {
            id: asset.id.to_string(),
            title: asset.title.clone(),
            slug: asset.slug.clone(),
            short_description: asset.short_description.clone(),
            asset_type: asset.asset_type.clone(),
            location_city: asset.location_city.clone(),
            location_country: asset.location_country.clone(),
            bedrooms: asset.bedrooms,
            lease_type: asset.lease_type.clone(),
            term_months: asset.term_months,
            cover_image_url: asset.cover_image_url.clone(),
            funding_status: asset.funding_status.clone(),
            total_value_usd: format_number(total_value_dollars),
            total_value_cents: asset.total_value_cents,
            tokens_total: asset.tokens_total,
            tokens_available: asset.tokens_available,
            tokens_sold,
            funded_percentage: funded_pct,
            funded_percentage_display: format!("{}", funded_pct),
            available_usd: format_number(available_dollars),
            investor_count: asset.investor_count.unwrap_or(0),
            price_per_sqft: format_number(price_per_sqft),
            annual_yield_percent: format!("{:.2}", annual_yield_pct),
            capital_appreciation_percent: format!("{:.2}", cap_appreciation_pct),
            projected_return_percent: format!("{:.2}", projected_return),
            five_year_total_return_percent: format!("{:.2}", five_year_return),
            annualised_net_return_percent: format!("{:.2}", annualised_net),
            land_size_sqm: asset.land_size_sqm.map(|d| format!("{}", d)),
            land_size_sqft: if land_sqft > 0.0 {
                Some(format_number(land_sqft as i64))
            } else {
                None
            },
        }
    }
}

/// Format a number with thousands separators (e.g. 1234567 -> "1,234,567")
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len + len / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(b as char);
    }
    result
}
