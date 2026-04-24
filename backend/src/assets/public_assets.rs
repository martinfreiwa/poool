//! Hardcoded property data for the public, logged-out property detail pages
//! linked from `landing-v2.html`.
//!
//! The landing cards on `/landing-v2.html` are static marketing content (not
//! backed by the `assets` table). For each card we expose a matching detail
//! page at `/p/:slug` using the same `property-public.html` template the real
//! dashboard property page uses.
//!
//! If you change a card on `landing-v2.html`, update the matching entry here
//! (and vice versa). The landing HTML and this file are intentionally the
//! two places that must stay in sync.
//!
//! To add a new card: add a new branch in [`lookup()`] below, then add the
//! corresponding `<a href="/p/{slug}">` card in `landing-v2.html`.

use super::models::PropertyDisplayData;

/// Build a synthetic [`PropertyDisplayData`] for a hardcoded public property.
///
/// Returns `None` if the slug is not one of the 8 landing-card properties.
pub fn lookup(slug: &str) -> Option<PropertyDisplayData> {
    let data = match slug {
        "sunset-luxury-villa" => PublicPropertySpec {
            slug: "sunset-luxury-villa",
            title: "Sunset Luxury Villa",
            short_description: "Contemporary villa steps from Echo Beach, Canggu's most popular surf break. Four-bedroom retreat with infinity pool, fully managed and producing income from day one.",
            long_description: "Set on a quiet lane just minutes from Canggu's Echo Beach, Sunset Luxury Villa pairs modern architecture with traditional Balinese craftsmanship. The property spans 128 m² of land and offers four en-suite bedrooms, a fully-equipped kitchen, and a wrap-around infinity pool overlooking the rice paddies.\n\nProfessionally managed on our behalf with year-round bookings. Investors earn monthly rental income distributed automatically after standard charges and fees.",
            asset_type: "property",
            location_city: "Canggu",
            location_country: "Indonesia",
            location_description: "Canggu is Bali's fastest-growing coastal community, famed for its surf breaks, boutique cafés and established expat scene. Strong year-round occupancy and a mature short-term rental market make it one of the island's highest-return areas.",
            bedrooms: 4,
            lease_type: "leasehold",
            term_months: 300,
            total_value_cents: 500_000_00,       // USD 500,000
            token_price_cents: 500_00,           // USD 500 / share
            tokens_total: 1000,
            tokens_available: 320,               // 68% funded
            annual_yield_bps: 640,               // 6.40%
            capital_appreciation_bps: 600,       // 6.00%
            land_size_sqm: 128.0,
            investor_count: 87,
            image_urls: vec![
                "/static/images/seed/villa1.webp",
                "/static/images/seed/villa1_2.webp",
                "/static/images/seed/villa1_3.webp",
                "/static/images/seed/villa1_4.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "ocean-breeze-penthouse" => PublicPropertySpec {
            slug: "ocean-breeze-penthouse",
            title: "Ocean Breeze Penthouse",
            short_description: "Two-bedroom freehold penthouse in the heart of Seminyak with sweeping ocean views and rooftop terrace.",
            long_description: "Ocean Breeze Penthouse occupies the top floor of a boutique building minutes from Seminyak's beach clubs and dining scene. Two spacious bedrooms, floor-to-ceiling windows, and a private rooftop terrace with plunge pool.\n\nSold as a full freehold — investors own a share of the underlying title. Managed by our partner on a flexible short- and mid-term rental basis.",
            asset_type: "property",
            location_city: "Seminyak",
            location_country: "Indonesia",
            location_description: "Seminyak is Bali's most established luxury destination. Known for premium villas, fashion boutiques and world-class restaurants. Freehold title here is rare and commands a strong resale premium.",
            bedrooms: 2,
            lease_type: "freehold",
            term_months: 0,
            total_value_cents: 380_000_00,
            token_price_cents: 380_00,
            tokens_total: 1000,
            tokens_available: 90,                // 91% funded
            annual_yield_bps: 420,               // 4.20%
            capital_appreciation_bps: 700,       // 7.00%
            land_size_sqm: 95.0,
            investor_count: 142,
            image_urls: vec![
                "/static/images/seed/villa4_1.webp",
                "/static/images/seed/villa4_2.webp",
                "/static/images/seed/villa2_1.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "echo-beach-loft" => PublicPropertySpec {
            slug: "echo-beach-loft",
            title: "Echo Beach Loft",
            short_description: "Architect-designed loft steps from Echo Beach. Open-plan living with exposed concrete and timber finishes.",
            long_description: "A contemporary two-bedroom loft just a short walk from one of Canggu's best-known surf breaks. Open-plan kitchen and living area opening onto a shared pool deck.\n\nEarly-stage funding — join on the ground floor. Completion is scheduled and income begins once the property is live.",
            asset_type: "property",
            location_city: "Canggu",
            location_country: "Indonesia",
            location_description: "Echo Beach sits on Canggu's surf coast — one of the busiest year-round tourism zones in Bali. High short-stay demand and steady appreciation.",
            bedrooms: 2,
            lease_type: "leasehold",
            term_months: 300,
            total_value_cents: 280_000_00,
            token_price_cents: 280_00,
            tokens_total: 1000,
            tokens_available: 920,               // 8% funded
            annual_yield_bps: 820,               // 8.20%
            capital_appreciation_bps: 800,       // 8.00%
            land_size_sqm: 80.0,
            investor_count: 18,
            image_urls: vec![
                "/static/images/seed/villa3_2.webp",
                "/static/images/seed/villa3_1.webp",
                "/static/images/seed/villa2_2.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "rice-terrace-retreat" => PublicPropertySpec {
            slug: "rice-terrace-retreat",
            title: "Rice Terrace Retreat",
            short_description: "Three-bedroom villa overlooking Ubud's iconic rice terraces. Long leasehold, high yield, turnkey operation.",
            long_description: "Wake up to views across the paddies in this thoughtfully-designed three-bedroom villa. Private pool, covered outdoor living, and direct access to hiking trails.\n\nOperated as a boutique wellness retreat with high repeat-booking rates. Long leasehold term offers both rental income and capital growth potential.",
            asset_type: "property",
            location_city: "Ubud",
            location_country: "Indonesia",
            location_description: "Ubud is Bali's cultural heart — known for yoga retreats, artisan workshops and rainforest scenery. A strong alternative to coastal areas with resilient demand year-round.",
            bedrooms: 3,
            lease_type: "long_leasehold",
            term_months: 480,
            total_value_cents: 420_000_00,
            token_price_cents: 420_00,
            tokens_total: 1000,
            tokens_available: 550,               // 45% funded
            annual_yield_bps: 760,               // 7.60%
            capital_appreciation_bps: 720,       // 7.20%
            land_size_sqm: 180.0,
            investor_count: 54,
            image_urls: vec![
                "/static/images/seed/villa5.webp",
                "/static/images/seed/villa6.webp",
                "/static/images/seed/villa8.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "tropical-garden-villa" => PublicPropertySpec {
            slug: "tropical-garden-villa",
            title: "Tropical Garden Villa",
            short_description: "Three-bedroom villa wrapped in tropical gardens near Ubud's central market. Serene setting, strong rental track record.",
            long_description: "Tucked behind stone walls on the edge of Ubud town, Tropical Garden Villa offers three en-suite bedrooms, a private pool and a mature tropical garden.\n\nFully managed and trading since 2023 with strong guest reviews. Consistent monthly rental distributions expected throughout the leasehold term.",
            asset_type: "property",
            location_city: "Ubud",
            location_country: "Indonesia",
            location_description: "Central Ubud offers walkable access to galleries, restaurants and the Monday market — a top-tier cultural tourism destination.",
            bedrooms: 3,
            lease_type: "leasehold",
            term_months: 300,
            total_value_cents: 340_000_00,
            token_price_cents: 340_00,
            tokens_total: 1000,
            tokens_available: 320,               // 68% funded
            annual_yield_bps: 700,               // 7.00%
            capital_appreciation_bps: 650,       // 6.50%
            land_size_sqm: 150.0,
            investor_count: 61,
            image_urls: vec![
                "/static/images/seed/villa6.webp",
                "/static/images/seed/villa5.webp",
                "/static/images/seed/villa8.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "cliffside-sunset-estate" => PublicPropertySpec {
            slug: "cliffside-sunset-estate",
            title: "Cliffside Sunset Estate",
            short_description: "Five-bedroom clifftop estate above Uluwatu with panoramic ocean views. Premium location, long-term appreciation play.",
            long_description: "Perched on the cliffs of Uluwatu with direct sunset views over the Indian Ocean, this five-bedroom estate is one of the premier luxury rentals in southern Bali.\n\nOperated by a leading villa management company with corporate and high-net-worth clientele. Limited supply of clifftop land makes this a strong capital appreciation story.",
            asset_type: "property",
            location_city: "Uluwatu",
            location_country: "Indonesia",
            location_description: "Uluwatu is the premium corner of Bali — home to world-class surf, luxury resorts and some of the island's highest land values. A scarcity-driven appreciation market.",
            bedrooms: 5,
            lease_type: "leasehold",
            term_months: 360,
            total_value_cents: 820_000_00,
            token_price_cents: 500_00,
            tokens_total: 1640,
            tokens_available: 950,               // 42% funded
            annual_yield_bps: 560,               // 5.60%
            capital_appreciation_bps: 900,       // 9.00%
            land_size_sqm: 340.0,
            investor_count: 73,
            image_urls: vec![
                "/static/images/seed/villa1.webp",
                "/static/images/seed/villa4_1.webp",
                "/static/images/seed/villa4_2.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "beachfront-bungalow" => PublicPropertySpec {
            slug: "beachfront-bungalow",
            title: "Beachfront Bungalow",
            short_description: "One-bedroom beachfront bungalow in Sanur. Fully funded — now earning monthly rental income for investors.",
            long_description: "A charming one-bedroom bungalow directly on Sanur's east-facing beach. Features include an outdoor terrace, covered dining area and private access to the sand.\n\nThis property is fully funded. Investors who participated now receive monthly rental distributions; new investment is not currently available on this asset.",
            asset_type: "property",
            location_city: "Sanur",
            location_country: "Indonesia",
            location_description: "Sanur is Bali's quieter east-coast resort town — popular with families and long-stay travellers. Stable demand with strong repeat-visitor rates.",
            bedrooms: 1,
            lease_type: "leasehold",
            term_months: 300,
            total_value_cents: 210_000_00,
            token_price_cents: 210_00,
            tokens_total: 1000,
            tokens_available: 0,                 // 100% funded
            annual_yield_bps: 680,               // 6.80%
            capital_appreciation_bps: 500,       // 5.00%
            land_size_sqm: 65.0,
            investor_count: 214,
            image_urls: vec![
                "/static/images/seed/villa2_1.webp",
                "/static/images/seed/villa2_2.webp",
                "/static/images/seed/villa3_1.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        "modern-jungle-retreat" => PublicPropertySpec {
            slug: "modern-jungle-retreat",
            title: "Modern Jungle Retreat",
            short_description: "Four-bedroom villa carved into the Canggu jungle. Fully funded — monthly rental income active.",
            long_description: "An architect-led jungle villa on the quieter north side of Canggu. Four bedrooms arranged around a central courtyard and infinity pool overlooking a dense ravine.\n\nThis property is fully funded and income-generating. Investors receive rental distributions monthly; no further investment slots are available on this asset.",
            asset_type: "property",
            location_city: "Canggu",
            location_country: "Indonesia",
            location_description: "North Canggu combines jungle scenery with quick access to beach and dining districts — one of the island's most in-demand emerging micro-markets.",
            bedrooms: 4,
            lease_type: "leasehold",
            term_months: 300,
            total_value_cents: 620_000_00,
            token_price_cents: 500_00,
            tokens_total: 1240,
            tokens_available: 0,                 // 100% funded
            annual_yield_bps: 720,               // 7.20%
            capital_appreciation_bps: 700,       // 7.00%
            land_size_sqm: 210.0,
            investor_count: 189,
            image_urls: vec![
                "/static/images/seed/villa3_1.webp",
                "/static/images/seed/villa3_2.webp",
                "/static/images/seed/villa1_3.webp",
            ],
            video_url: None,
            google_maps_url: None,
        },

        _ => return None,
    };

    Some(data.into_display_data())
}

/// Static description of a public property. Keeps the `lookup()` table compact
/// and close to the shape of the real DB row without pulling in `MarketplaceAsset`.
struct PublicPropertySpec {
    slug: &'static str,
    title: &'static str,
    short_description: &'static str,
    long_description: &'static str,
    asset_type: &'static str,
    location_city: &'static str,
    location_country: &'static str,
    location_description: &'static str,
    bedrooms: i32,
    lease_type: &'static str,
    term_months: i32,
    total_value_cents: i64,
    token_price_cents: i64,
    tokens_total: i32,
    tokens_available: i32,
    annual_yield_bps: i32,
    capital_appreciation_bps: i32,
    land_size_sqm: f64,
    investor_count: i64,
    image_urls: Vec<&'static str>,
    video_url: Option<&'static str>,
    google_maps_url: Option<&'static str>,
}

impl PublicPropertySpec {
    fn into_display_data(self) -> PropertyDisplayData {
        let tokens_sold = self.tokens_total - self.tokens_available;
        let funded_pct = if self.tokens_total > 0 {
            ((tokens_sold as f64 / self.tokens_total as f64) * 100.0) as i32
        } else {
            0
        };

        let available_cents = self.tokens_available as i64 * self.token_price_cents;
        let total_value_dollars = self.total_value_cents / 100;
        let available_dollars = available_cents / 100;

        let annual_yield_pct = self.annual_yield_bps as f64 / 100.0;
        let cap_appreciation_pct = self.capital_appreciation_bps as f64 / 100.0;
        let projected_return = annual_yield_pct + cap_appreciation_pct;
        let annual_return = projected_return / 100.0;
        let five_year_return = ((1.0 + annual_return).powi(5) - 1.0) * 100.0;
        let annualised_net = annual_yield_pct * 0.85 + cap_appreciation_pct;

        let price_per_sqm = if self.land_size_sqm > 0.0 {
            (total_value_dollars as f64 / self.land_size_sqm) as i64
        } else {
            0
        };

        // Render the long description as HTML paragraphs, matching
        // `PropertyDisplayData::from_asset`'s behaviour.
        let long_description_html = {
            let trimmed = self.long_description.trim();
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
        };

        let mut image_urls: Vec<String> = self.image_urls.iter().map(|s| s.to_string()).collect();
        if !image_urls.is_empty() && image_urls.len() < 5 {
            let mut index = 0usize;
            while image_urls.len() < 5 {
                image_urls.push(image_urls[index % image_urls.len()].clone());
                index += 1;
            }
        }
        let cover_image_url = image_urls.first().cloned();

        PropertyDisplayData {
            id: self.slug.to_string(),
            title: self.title.to_string(),
            slug: self.slug.to_string(),
            short_description: Some(self.short_description.to_string()),
            description: Some(self.long_description.to_string()),
            asset_type: self.asset_type.to_string(),
            location_city: Some(self.location_city.to_string()),
            location_country: Some(self.location_country.to_string()),
            bedrooms: Some(self.bedrooms),
            bathrooms: Some(self.bedrooms),
            lease_type: Some(self.lease_type.to_string()),
            term_months: Some(self.term_months),
            image_urls,
            cover_image_url,
            funding_status: if self.tokens_available == 0 {
                "funding_closed".to_string()
            } else {
                "funding_in_progress".to_string()
            },
            google_maps_url: self.google_maps_url.map(|s| s.to_string()),
            video_url: self.video_url.map(|s| s.to_string()),
            youtube_video_id: None,
            long_description: long_description_html,
            location_description: Some(self.location_description.to_string()),

            total_value_usd: super::models::format_number(total_value_dollars),
            total_value_cents: self.total_value_cents,
            tokens_total: self.tokens_total,
            tokens_available: self.tokens_available,
            tokens_sold,
            funded_percentage: funded_pct,
            funded_percentage_display: format!("{}", funded_pct),
            available_usd: super::models::format_number(available_dollars),
            investor_count: self.investor_count,
            price_per_sqm: super::models::format_number(price_per_sqm),
            annual_yield_percent: format!("{:.2}", annual_yield_pct),
            capital_appreciation_percent: format!("{:.2}", cap_appreciation_pct),
            projected_return_percent: format!("{:.2}", projected_return),
            five_year_total_return_percent: format!("{:.2}", five_year_return),
            annualised_net_return_percent: format!("{:.2}", annualised_net),
            building_size_sqm: Some(format!("{}", self.land_size_sqm as i64)),
            land_size_sqm: Some(format!("{}", self.land_size_sqm as i64)),
            platform_fee_usd: super::models::format_number(total_value_dollars * 5 / 100),
            total_investment_cost_usd: super::models::format_number(
                total_value_dollars * 105 / 100,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_slug_returns_none() {
        assert!(lookup("does-not-exist").is_none());
    }

    #[test]
    fn known_slugs_resolve() {
        for slug in [
            "sunset-luxury-villa",
            "ocean-breeze-penthouse",
            "echo-beach-loft",
            "rice-terrace-retreat",
            "tropical-garden-villa",
            "cliffside-sunset-estate",
            "beachfront-bungalow",
            "modern-jungle-retreat",
        ] {
            let data = lookup(slug).unwrap_or_else(|| panic!("slug {} should resolve", slug));
            assert_eq!(data.slug, slug);
            assert!(!data.title.is_empty());
            assert!(data.funded_percentage >= 0 && data.funded_percentage <= 100);
        }
    }

    #[test]
    fn fully_funded_properties_report_100_percent() {
        for slug in ["beachfront-bungalow", "modern-jungle-retreat"] {
            let data = lookup(slug).unwrap();
            assert_eq!(
                data.funded_percentage, 100,
                "{} should be fully funded",
                slug
            );
            assert_eq!(data.tokens_available, 0);
        }
    }
}
