use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::storage::service::rewrite_gcs_url;

const DEFAULT_PROPERTY_IMAGE_URL: &str = "/static/images/seed/villa1.webp";

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct MarketplaceAsset {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub description: Option<String>,
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
    pub image_urls: Option<Vec<String>>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub lease_type: Option<String>,
    pub term_months: Option<i32>,
    pub area: Option<String>,
    pub building_size_sqm: Option<rust_decimal::Decimal>,
    pub land_size_sqm: Option<rust_decimal::Decimal>,
    /// Number of unique active holders (from `investments`, tokens_owned > 0, status != 'exited')
    pub investor_count: Option<i64>,
    /// Sum of tokens currently held across all active investments (single source of truth).
    /// When `Some`, overrides drift in `assets.tokens_available`.
    pub tokens_sold_actual: Option<i64>,
    pub video_url: Option<String>,
    pub google_maps_url: Option<String>,
    pub location_description: Option<String>,
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
    pub description: Option<String>,
    pub asset_type: String,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub lease_type: Option<String>,
    pub term_months: Option<i32>,
    pub area: Option<String>,
    pub image_urls: Vec<String>,
    pub cover_image_url: Option<String>,
    pub funding_status: String,
    pub google_maps_url: Option<String>,
    pub video_url: Option<String>,
    /// Extracted YouTube video ID (e.g. "dQw4w9WgXcQ")
    pub youtube_video_id: Option<String>,
    /// The full description rendered as HTML paragraphs
    pub long_description: Option<String>,
    /// Location-specific description text
    pub location_description: Option<String>,

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
    pub price_per_sqm: String,                  // e.g. "12,234"
    pub annual_yield_percent: String,           // e.g. "7.45"
    pub capital_appreciation_percent: String,   // e.g. "5.49"
    pub projected_return_percent: String,       // capital_appreciation + annual_yield
    pub five_year_total_return_percent: String, // approx
    pub annualised_net_return_percent: String,
    pub building_size_sqm: Option<String>,
    pub land_size_sqm: Option<String>,
    pub platform_fee_usd: String,          // 5% of total_value
    pub total_investment_cost_usd: String, // total_value + 5% fee
    pub token_price_usd: i64,              // per-share price in whole dollars
    pub is_public_preview: bool,
    pub public_data_notice: Option<String>,

    // ── CMS fields populated from migration 116 columns ────────────────────
    /// Short label for property investment type (e.g. "Fractional ownership").
    pub investment_type: Option<String>,
    /// Long-form description shown in the Investment Type section.
    pub investment_type_description: Option<String>,
    /// Short label for leasing strategy (e.g. "Long-term rental").
    pub leasing_strategy_type: Option<String>,
    /// Long-form description shown in the Leasing Strategy section.
    pub leasing_strategy_description: Option<String>,
    /// Asset-specific risk disclosure shown in the Risk Notification section.
    pub risk_notification: Option<String>,
    /// Calculator slider default amount, in whole USD (already converted).
    pub default_investment_amount_usd: Option<i64>,
    /// Calculator default annual property value growth as percent string (e.g. "10").
    pub default_value_growth_percent: Option<String>,
    /// Calculator default annual rental yield as percent string (e.g. "12").
    pub default_rental_yield_percent: Option<String>,
    /// Property Developer card logo URL.
    pub developer_logo_url: Option<String>,
    /// Property Developer card name.
    pub developer_name: Option<String>,
    /// Property Developer card description.
    pub developer_description: Option<String>,
    /// Property Developer card website URL.
    pub developer_website: Option<String>,
    /// Property Developer card Facebook URL.
    pub developer_facebook: Option<String>,
    /// Property Developer card Instagram URL.
    pub developer_instagram: Option<String>,
    /// Property Developer card YouTube URL.
    pub developer_youtube: Option<String>,
    /// Info badges (JSONB array of {icon_url,title,subtitle}). None falls
    /// back to the legacy hardcoded badges in the property template.
    pub info_badges: Option<serde_json::Value>,
    /// Leasing strategy items (JSONB array of {title,description}). None
    /// falls back to the legacy hardcoded three items.
    pub leasing_items: Option<serde_json::Value>,
    /// Risk notification items (JSONB array of {title,body}). None / empty
    /// falls back to splitting `risk_notification` text, then hardcoded.
    pub risk_notification_items: Option<serde_json::Value>,
    /// Roadmap milestones populated from `asset_milestones`. None / empty
    /// falls back to the legacy hardcoded Funding Timeline.
    pub milestones: Option<Vec<MilestoneDisplay>>,
}

/// Single roadmap row exposed to templates.
#[derive(Debug, Serialize)]
pub struct MilestoneDisplay {
    pub title: String,
    pub description: Option<String>,
    pub milestone_date: Option<String>,
    pub month_index: Option<i32>,
    pub is_completed: bool,
}

/// Editable property-page content fetched separately from the core asset row.
/// Populated from columns added in migration 116.
#[derive(Debug, sqlx::FromRow)]
pub struct AssetPageContent {
    /// Investment type label.
    pub investment_type: Option<String>,
    /// Investment type long-form description.
    pub investment_type_description: Option<String>,
    /// Leasing strategy label.
    pub leasing_strategy_type: Option<String>,
    /// Leasing strategy long-form description.
    pub leasing_strategy_description: Option<String>,
    /// Risk-notification text.
    pub risk_notification: Option<String>,
    /// Calculator default investment amount, in cents.
    pub default_investment_amount_cents: Option<i64>,
    /// Calculator default value growth, basis points.
    pub default_value_growth_bps: Option<i32>,
    /// Calculator default rental yield, basis points.
    pub default_rental_yield_bps: Option<i32>,
    /// Developer card logo URL.
    pub developer_logo_url: Option<String>,
    /// Developer card name.
    pub developer_name: Option<String>,
    /// Developer card description.
    pub developer_description: Option<String>,
    /// Developer card website URL.
    pub developer_website: Option<String>,
    /// Developer card Facebook URL.
    pub developer_facebook: Option<String>,
    /// Developer card Instagram URL.
    pub developer_instagram: Option<String>,
    /// Developer card YouTube URL.
    pub developer_youtube: Option<String>,
    /// Info badges JSONB array.
    pub info_badges: Option<serde_json::Value>,
    /// Leasing strategy items JSONB array.
    pub leasing_items: Option<serde_json::Value>,
    /// Risk notification items JSONB array of {title, body}.
    pub risk_notification_items: Option<serde_json::Value>,
}

impl AssetPageContent {
    /// Format a basis-points value as a percent string with no trailing zeros (e.g. 1050 → "10.5").
    fn bps_to_percent(bps: i32) -> String {
        let v = bps as f64 / 100.0;
        if v == v.floor() {
            format!("{:.0}", v)
        } else {
            format!("{:.1}", v)
        }
    }

    /// Apply this CMS content to a built `PropertyDisplayData`.
    pub fn apply_to(self, display: &mut PropertyDisplayData) {
        display.investment_type = self.investment_type;
        display.investment_type_description = self.investment_type_description;
        display.leasing_strategy_type = self.leasing_strategy_type;
        display.leasing_strategy_description = self.leasing_strategy_description;
        display.risk_notification = self.risk_notification;
        display.default_investment_amount_usd =
            self.default_investment_amount_cents.map(|c| c / 100);
        display.default_value_growth_percent =
            self.default_value_growth_bps.map(Self::bps_to_percent);
        display.default_rental_yield_percent =
            self.default_rental_yield_bps.map(Self::bps_to_percent);
        display.developer_logo_url = self.developer_logo_url;
        display.developer_name = self.developer_name;
        display.developer_description = self.developer_description;
        display.developer_website = self.developer_website;
        display.developer_facebook = self.developer_facebook;
        display.developer_instagram = self.developer_instagram;
        display.developer_youtube = self.developer_youtube;
        display.info_badges = self.info_badges.filter(|v| {
            v.as_array().map(|a| !a.is_empty()).unwrap_or(false)
        });
        display.leasing_items = self.leasing_items.filter(|v| {
            v.as_array().map(|a| !a.is_empty()).unwrap_or(false)
        });
        display.risk_notification_items = self.risk_notification_items.filter(|v| {
            v.as_array().map(|a| !a.is_empty()).unwrap_or(false)
        });
    }
}

impl PropertyDisplayData {
    pub fn from_asset(asset: &MarketplaceAsset) -> Self {
        // Prefer dynamic count from `investments` (single source of truth) over drift-prone
        // `assets.tokens_available`. Falls back to stored field if subquery absent.
        let tokens_sold = asset
            .tokens_sold_actual
            .map(|n| n.clamp(0, asset.tokens_total as i64) as i32)
            .unwrap_or(asset.tokens_total - asset.tokens_available);
        let tokens_available_dyn = (asset.tokens_total - tokens_sold).max(0);
        let funded_pct = if asset.tokens_total > 0 {
            ((tokens_sold as f64 / asset.tokens_total as f64) * 100.0) as i32
        } else {
            0
        };

        let available_cents = tokens_available_dyn as i64 * asset.token_price_cents;
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

        // Price per sqm
        let land_sqm = asset
            .land_size_sqm
            .map(|d| {
                use rust_decimal::prelude::ToPrimitive;
                d.to_f64().unwrap_or(0.0)
            })
            .unwrap_or(0.0);
        let price_per_sqm = if land_sqm > 0.0 {
            (total_value_dollars as f64 / land_sqm) as i64
        } else {
            0
        };

        // Extract YouTube video ID from video_url if present
        let youtube_video_id = extract_youtube_id(asset.video_url.as_deref());

        // Build long_description as HTML paragraphs from the description field
        let long_description = asset.description.as_ref().and_then(|desc| {
            let trimmed = desc.trim();
            if trimmed.is_empty() {
                None
            } else {
                // Split on double newlines for paragraphs
                let paragraphs: Vec<&str> = trimmed.split("\n\n").collect();
                let html: String = paragraphs
                    .iter()
                    .map(|p| format!("<p>{}</p>", escape_html(p.trim())))
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(html)
            }
        });

        let image_urls: Vec<String> = asset
            .image_urls
            .as_ref()
            .map(|urls| urls.iter().map(|u| rewrite_gcs_url(u)).collect())
            .unwrap_or_default();
        let cover_image_url = image_urls
            .first()
            .cloned()
            .or_else(|| Some(DEFAULT_PROPERTY_IMAGE_URL.to_string()));

        PropertyDisplayData {
            id: asset.id.to_string(),
            title: asset.title.clone(),
            slug: asset.slug.clone(),
            short_description: asset.short_description.clone(),
            description: asset.description.clone(),
            asset_type: asset.asset_type.clone(),
            location_city: asset.location_city.clone(),
            location_country: asset.location_country.clone(),
            bedrooms: asset.bedrooms,
            bathrooms: asset.bathrooms,
            lease_type: asset.lease_type.clone(),
            term_months: asset.term_months,
            area: asset.area.clone(),
            image_urls,
            cover_image_url,
            funding_status: asset.funding_status.clone(),
            google_maps_url: asset.google_maps_url.clone(),
            video_url: asset.video_url.clone(),
            youtube_video_id,
            long_description,
            location_description: asset.location_description.clone(),
            total_value_usd: format_number(total_value_dollars),
            total_value_cents: asset.total_value_cents,
            tokens_total: asset.tokens_total,
            tokens_available: tokens_available_dyn,
            tokens_sold,
            funded_percentage: funded_pct,
            funded_percentage_display: format!("{}", funded_pct),
            available_usd: format_number(available_dollars),
            investor_count: asset.investor_count.unwrap_or(0),
            price_per_sqm: format_number(price_per_sqm),
            annual_yield_percent: format!("{:.2}", annual_yield_pct),
            capital_appreciation_percent: format!("{:.2}", cap_appreciation_pct),
            projected_return_percent: format!("{:.2}", projected_return),
            five_year_total_return_percent: format!("{:.2}", five_year_return),
            annualised_net_return_percent: format!("{:.2}", annualised_net),
            building_size_sqm: asset.building_size_sqm.map(|d| format!("{:.0}", d)),
            land_size_sqm: asset.land_size_sqm.map(|d| format!("{}", d)),
            platform_fee_usd: format_number(total_value_dollars * 5 / 100),
            total_investment_cost_usd: format_number(total_value_dollars * 105 / 100),
            token_price_usd: asset.token_price_cents / 100,
            is_public_preview: false,
            public_data_notice: None,
            investment_type: None,
            investment_type_description: None,
            leasing_strategy_type: None,
            leasing_strategy_description: None,
            risk_notification: None,
            default_investment_amount_usd: None,
            default_value_growth_percent: None,
            default_rental_yield_percent: None,
            developer_logo_url: None,
            developer_name: None,
            developer_description: None,
            developer_website: None,
            developer_facebook: None,
            developer_instagram: None,
            developer_youtube: None,
            info_badges: None,
            leasing_items: None,
            risk_notification_items: None,
            milestones: None,
        }
    }

    pub fn update_fee(&mut self, fee_pct: f64) {
        let total_value_dollars = self.total_value_cents / 100;
        let fee_dollars = ((total_value_dollars as f64) * fee_pct / 100.0).round() as i64;
        self.platform_fee_usd = format_number(fee_dollars);
        self.total_investment_cost_usd = format_number(total_value_dollars + fee_dollars);
    }

    pub fn update_fee_bps(&mut self, fee_bps: i32) {
        let total_value_dollars = self.total_value_cents / 100;
        let fee_dollars = total_value_dollars.saturating_mul(fee_bps.max(0) as i64) / 10_000;
        self.platform_fee_usd = format_number(fee_dollars);
        self.total_investment_cost_usd = format_number(total_value_dollars + fee_dollars);
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Format a number with thousands separators (e.g. 1234567 -> "1,234,567")
pub(crate) fn format_number(n: i64) -> String {
    crate::common::currency::format_thousands(n)
}

/// Extract YouTube video ID from a URL string.
/// Handles formats: youtube.com/watch?v=ID, youtu.be/ID, youtube.com/embed/ID
fn extract_youtube_id(url: Option<&str>) -> Option<String> {
    let url = url?.trim();
    if url.is_empty() {
        return None;
    }
    // youtu.be/VIDEO_ID
    if let Some(rest) = url
        .strip_prefix("https://youtu.be/")
        .or_else(|| url.strip_prefix("http://youtu.be/"))
    {
        return Some(
            rest.split(['?', '&', '#'])
                .next()
                .unwrap_or(rest)
                .to_string(),
        );
    }
    // youtube.com/embed/VIDEO_ID
    if let Some(idx) = url.find("/embed/") {
        let after = &url[idx + 7..];
        return Some(
            after
                .split(['?', '&', '#', '/'])
                .next()
                .unwrap_or(after)
                .to_string(),
        );
    }
    // youtube.com/watch?v=VIDEO_ID
    if url.contains("youtube.com") {
        if let Some(v_idx) = url.find("v=") {
            let after = &url[v_idx + 2..];
            return Some(after.split(['&', '#']).next().unwrap_or(after).to_string());
        }
    }
    // Bare video ID (11 chars, alphanumeric + _ + -)
    if url.len() == 11
        && url
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Some(url.to_string());
    }
    None
}

// ─── Commodity-specific structs ────────────────────────────────────────────

/// DB query struct for commodity assets — includes all commodity-specific columns
/// that `MarketplaceAsset` omits.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CommodityAsset {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub description: Option<String>,
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
    pub image_urls: Option<Vec<String>>,
    pub term_months: Option<i32>,
    pub area: Option<String>,
    pub land_size_sqm: Option<rust_decimal::Decimal>,
    pub google_maps_url: Option<String>,
    pub video_url: Option<String>,
    /// Number of unique active holders (from `investments`, tokens_owned > 0, status != 'exited')
    pub investor_count: Option<i64>,
    /// Sum of tokens currently held across all active investments.
    pub tokens_sold_actual: Option<i64>,

    // Commodity-specific fields
    pub operator_name: Option<String>,
    pub fixed_roi_bps: Option<i32>,
    pub revenue_min_cents: Option<i64>,
    pub revenue_max_cents: Option<i64>,
    pub expenses_cents: Option<i64>,
    pub net_profit_min_cents: Option<i64>,
    pub net_profit_max_cents: Option<i64>,
    pub investor_payout_cents: Option<i64>,
    pub operator_split_pct: Option<i32>,
    pub poool_split_pct: Option<i32>,
    pub location_description: Option<String>,
}

/// Template-friendly representation for commodity detail pages.
/// All monetary values are formatted as strings for direct use in MiniJinja templates.
#[derive(Debug, Serialize)]
pub struct CommodityDisplayData {
    // ── Base asset fields ──
    pub id: String,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub description: Option<String>,
    pub asset_type: String,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub image_urls: Vec<String>,
    pub cover_image_url: Option<String>,
    pub funding_status: String,
    pub google_maps_url: Option<String>,
    pub video_url: Option<String>,
    /// Extracted YouTube video ID (e.g. "dQw4w9WgXcQ")
    pub youtube_video_id: Option<String>,
    /// The full description rendered as HTML paragraphs
    pub long_description: Option<String>,

    // ── Funding / token metrics ──
    pub total_value_usd: String,
    pub total_value_cents: i64,
    pub tokens_total: i32,
    pub tokens_available: i32,
    pub tokens_sold: i32,
    pub funded_percentage: i32,
    pub funded_percentage_display: String,
    pub available_usd: String,
    pub investor_count: i64,
    pub min_investment_usd: String,

    // ── Commodity-specific display values ──
    pub operator_name: Option<String>,
    pub term_months: Option<i32>,
    pub fixed_roi_percent: String,
    pub fixed_roi_bps: i32,

    // Revenue & financials (pre-formatted)
    pub revenue_min_usd: Option<String>,
    pub revenue_max_usd: Option<String>,
    pub revenue_display: Option<String>,
    pub expenses_usd: Option<String>,
    pub net_profit_min_usd: Option<String>,
    pub net_profit_max_usd: Option<String>,
    pub net_profit_display: Option<String>,
    pub investor_payout_usd: Option<String>,

    // Split percentages
    pub operator_split_pct: Option<i32>,
    pub poool_split_pct: Option<i32>,

    // Land info
    pub land_size_sqm: Option<String>,
    pub land_size_hectares: Option<String>,
    pub per_hectare_cost_usd: Option<String>,

    // Location
    pub location_description: Option<String>,

    // Platform fee
    pub platform_fee_usd: String,
    pub total_investment_cost_usd: String,

    // Yield display (for card-style display)
    pub annual_yield_percent: String,
    pub capital_appreciation_percent: String,
    pub projected_return_percent: String,
}

impl CommodityDisplayData {
    /// Build a template-friendly `CommodityDisplayData` from a raw `CommodityAsset`.
    pub fn from_asset(asset: &CommodityAsset) -> Self {
        let tokens_sold = asset
            .tokens_sold_actual
            .map(|n| n.clamp(0, asset.tokens_total as i64) as i32)
            .unwrap_or(asset.tokens_total - asset.tokens_available);
        let tokens_available_dyn = (asset.tokens_total - tokens_sold).max(0);
        let funded_pct = if asset.tokens_total > 0 {
            ((tokens_sold as f64 / asset.tokens_total as f64) * 100.0) as i32
        } else {
            0
        };

        let available_cents = tokens_available_dyn as i64 * asset.token_price_cents;
        let total_value_dollars = asset.total_value_cents / 100;
        let available_dollars = available_cents / 100;
        let min_investment_dollars = asset.token_price_cents / 100;

        // ROI
        let fixed_roi_bps = asset
            .fixed_roi_bps
            .unwrap_or(asset.annual_yield_bps.unwrap_or(0));
        let fixed_roi_pct = fixed_roi_bps as f64 / 100.0;

        // Yield / appreciation for card displays
        let annual_yield_bps = asset.annual_yield_bps.unwrap_or(0);
        let cap_appreciation_bps = asset.capital_appreciation_bps.unwrap_or(0);
        let annual_yield_pct = annual_yield_bps as f64 / 100.0;
        let cap_appreciation_pct = cap_appreciation_bps as f64 / 100.0;
        let projected_return = annual_yield_pct + cap_appreciation_pct;

        // Land hectares
        let land_sqm = asset
            .land_size_sqm
            .map(|d| {
                use rust_decimal::prelude::ToPrimitive;
                d.to_f64().unwrap_or(0.0)
            })
            .unwrap_or(0.0);
        let hectares = land_sqm / 10_000.0;
        let per_ha_cost = if hectares > 0.0 {
            (total_value_dollars as f64 / hectares) as i64
        } else {
            0
        };

        // Revenue display helpers
        let revenue_min = asset.revenue_min_cents.map(|c| c / 100);
        let revenue_max = asset.revenue_max_cents.map(|c| c / 100);
        let revenue_display = match (revenue_min, revenue_max) {
            (Some(min), Some(max)) => {
                Some(format!("${} – ${}", format_number(min), format_number(max)))
            }
            (Some(min), None) => Some(format!("${}", format_number(min))),
            _ => None,
        };

        let net_min = asset.net_profit_min_cents.map(|c| c / 100);
        let net_max = asset.net_profit_max_cents.map(|c| c / 100);
        let net_profit_display = match (net_min, net_max) {
            (Some(min), Some(max)) => {
                Some(format!("${} – ${}", format_number(min), format_number(max)))
            }
            (Some(min), None) => Some(format!("${}", format_number(min))),
            _ => None,
        };

        let image_urls: Vec<String> = asset
            .image_urls
            .as_ref()
            .map(|urls| urls.iter().map(|u| rewrite_gcs_url(u)).collect())
            .unwrap_or_default();
        let cover_image_url = image_urls
            .first()
            .cloned()
            .or_else(|| Some(DEFAULT_PROPERTY_IMAGE_URL.to_string()));

        CommodityDisplayData {
            id: asset.id.to_string(),
            title: asset.title.clone(),
            slug: asset.slug.clone(),
            short_description: asset.short_description.clone(),
            description: asset.description.clone(),
            asset_type: asset.asset_type.clone(),
            location_city: asset.location_city.clone(),
            location_country: asset.location_country.clone(),
            image_urls,
            cover_image_url,
            funding_status: asset.funding_status.clone(),
            google_maps_url: asset.google_maps_url.clone(),
            video_url: asset.video_url.clone(),
            youtube_video_id: extract_youtube_id(asset.video_url.as_deref()),
            long_description: asset.description.as_ref().and_then(|desc| {
                let trimmed = desc.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    let paragraphs: Vec<&str> = trimmed.split("\n\n").collect();
                    let html: String = paragraphs
                        .iter()
                        .map(|p| format!("<p>{}</p>", p.trim()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    Some(html)
                }
            }),

            total_value_usd: format_number(total_value_dollars),
            total_value_cents: asset.total_value_cents,
            tokens_total: asset.tokens_total,
            tokens_available: tokens_available_dyn,
            tokens_sold,
            funded_percentage: funded_pct,
            funded_percentage_display: format!("{}", funded_pct),
            available_usd: format_number(available_dollars),
            investor_count: asset.investor_count.unwrap_or(0),
            min_investment_usd: format_number(min_investment_dollars),

            operator_name: asset.operator_name.clone(),
            term_months: asset.term_months,
            fixed_roi_percent: format!("{:.2}", fixed_roi_pct),
            fixed_roi_bps,

            revenue_min_usd: revenue_min.map(format_number),
            revenue_max_usd: revenue_max.map(format_number),
            revenue_display,
            expenses_usd: asset.expenses_cents.map(|c| format_number(c / 100)),
            net_profit_min_usd: net_min.map(format_number),
            net_profit_max_usd: net_max.map(format_number),
            net_profit_display,
            investor_payout_usd: asset.investor_payout_cents.map(|c| format_number(c / 100)),

            operator_split_pct: asset.operator_split_pct,
            poool_split_pct: asset.poool_split_pct,

            land_size_sqm: asset.land_size_sqm.map(|d| format!("{}", d)),
            land_size_hectares: if hectares > 0.0 {
                Some(if hectares == hectares.floor() {
                    format!("{:.0}", hectares)
                } else {
                    format!("{:.1}", hectares)
                })
            } else {
                None
            },
            per_hectare_cost_usd: if per_ha_cost > 0 {
                Some(format_number(per_ha_cost))
            } else {
                None
            },

            location_description: asset.location_description.clone(),

            platform_fee_usd: format_number(total_value_dollars * 5 / 100),
            total_investment_cost_usd: format_number(total_value_dollars * 105 / 100),

            annual_yield_percent: format!("{:.2}", annual_yield_pct),
            capital_appreciation_percent: format!("{:.2}", cap_appreciation_pct),
            projected_return_percent: format!("{:.2}", projected_return),
        }
    }

    /// Recalculate platform fee from basis points using integer cents.
    pub fn update_fee_bps(&mut self, fee_bps: i32) {
        let bps = fee_bps.max(0) as i128;
        let total_cents = self.total_value_cents.max(0) as i128;
        let fee_cents = (total_cents.saturating_mul(bps) + 5_000) / 10_000;
        let total_with_fee_cents = total_cents.saturating_add(fee_cents);

        self.platform_fee_usd = format_number((fee_cents / 100) as i64);
        self.total_investment_cost_usd = format_number((total_with_fee_cents / 100) as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_commodity_asset() -> CommodityAsset {
        CommodityAsset {
            id: uuid::Uuid::new_v4(),
            title: "Agro Investment: Cabai Rawit".to_string(),
            slug: "cabai-rawit-q2-2026".to_string(),
            short_description: Some("Premium chili plantation".to_string()),
            description: Some("Full description".to_string()),
            asset_type: "commodity".to_string(),
            location_city: Some("Lebak".to_string()),
            location_country: Some("ID".to_string()),
            total_value_cents: 45_000_000, // $450,000
            token_price_cents: 100_000,    // $1,000
            tokens_total: 450,
            tokens_available: 50,
            annual_yield_bps: Some(3500), // 35%
            capital_appreciation_bps: Some(0),
            funding_status: "funding_in_progress".to_string(),
            image_urls: Some(vec!["https://example.com/img1.webp".to_string()]),
            term_months: Some(12),
            area: Some("Banten".to_string()),
            land_size_sqm: Some(rust_decimal::Decimal::from(600_000)), // 60 ha
            google_maps_url: Some("https://maps.google.com/test".to_string()),
            video_url: None,
            investor_count: Some(42),
            tokens_sold_actual: None,
            operator_name: Some("PT. NEO AGRO SOLUTIONS".to_string()),
            fixed_roi_bps: Some(3500),
            revenue_min_cents: Some(540_000_000), // $5.4M
            revenue_max_cents: Some(900_000_000), // $9M
            expenses_cents: Some(45_000_000),     // $450K
            net_profit_min_cents: Some(495_000_000),
            net_profit_max_cents: Some(855_000_000),
            investor_payout_cents: Some(60_750_000), // $607,500
            operator_split_pct: Some(55),
            poool_split_pct: Some(45),
            location_description: Some("Located in Lebak Regency, Banten Province".to_string()),
        }
    }

    #[test]
    fn test_commodity_display_data_from_asset() {
        let asset = sample_commodity_asset();
        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(display.total_value_usd, "450,000");
        assert_eq!(display.tokens_sold, 400);
        assert_eq!(display.funded_percentage, 88);
        assert_eq!(display.investor_count, 42);
        assert_eq!(display.min_investment_usd, "1,000");
        assert_eq!(
            display.operator_name.as_deref(),
            Some("PT. NEO AGRO SOLUTIONS")
        );
        assert_eq!(display.fixed_roi_percent, "35.00");
        assert_eq!(display.fixed_roi_bps, 3500);
    }

    #[test]
    fn test_monetary_values_always_cents() {
        let asset = sample_commodity_asset();
        assert!(asset.total_value_cents > 100_000);
        assert_eq!(asset.total_value_cents, 45_000_000);
        assert_eq!(asset.token_price_cents, 100_000);
    }

    #[test]
    fn test_format_number_thousands_separator() {
        assert_eq!(format_number(1_234_567), "1,234,567");
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(450_000), "450,000");
    }

    #[test]
    fn test_funded_percentage_edge_cases() {
        let mut asset = sample_commodity_asset();

        // All sold
        asset.tokens_available = 0;
        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(display.funded_percentage, 100);

        // None sold
        asset.tokens_available = asset.tokens_total;
        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(display.funded_percentage, 0);
    }

    #[test]
    fn test_optional_fields_handle_none() {
        let mut asset = sample_commodity_asset();
        asset.annual_yield_bps = None;
        asset.capital_appreciation_bps = None;
        asset.land_size_sqm = None;
        asset.image_urls = None;
        asset.fixed_roi_bps = None;
        asset.revenue_min_cents = None;
        asset.revenue_max_cents = None;

        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(display.annual_yield_percent, "0.00");
        assert_eq!(display.capital_appreciation_percent, "0.00");
        assert!(display.image_urls.is_empty());
        assert!(display.land_size_sqm.is_none());
        assert!(display.revenue_display.is_none());
        assert_eq!(display.fixed_roi_bps, 0);
    }

    #[test]
    fn test_zero_tokens_no_division_by_zero() {
        let mut asset = sample_commodity_asset();
        asset.tokens_total = 0;
        asset.tokens_available = 0;

        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(display.funded_percentage, 0);
    }

    #[test]
    fn test_hectare_conversion() {
        let asset = sample_commodity_asset();
        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(display.land_size_hectares.as_deref(), Some("60"));
        assert_eq!(display.per_hectare_cost_usd.as_deref(), Some("7,500"));
    }

    #[test]
    fn test_revenue_display_formatting() {
        let asset = sample_commodity_asset();
        let display = CommodityDisplayData::from_asset(&asset);
        assert_eq!(
            display.revenue_display.as_deref(),
            Some("$5,400,000 – $9,000,000")
        );
        assert_eq!(display.expenses_usd.as_deref(), Some("450,000"));
        assert_eq!(display.investor_payout_usd.as_deref(), Some("607,500"));
        assert_eq!(display.operator_split_pct, Some(55));
        assert_eq!(display.poool_split_pct, Some(45));
    }

    #[test]
    fn test_update_fee_bps_uses_integer_cents() {
        let asset = sample_commodity_asset();
        let mut display = CommodityDisplayData::from_asset(&asset);

        display.update_fee_bps(250);

        assert_eq!(display.platform_fee_usd, "11,250");
        assert_eq!(display.total_investment_cost_usd, "461,250");
    }
}
