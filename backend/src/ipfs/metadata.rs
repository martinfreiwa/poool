/// ERC-1155 metadata builder — generates compliant JSON from database asset data.
///
/// Follows the ERC-1155 Metadata JSON Schema:
/// https://eips.ethereum.org/EIPS/eip-1155#metadata
///
/// 🔴 SECURITY: Never include PII in metadata. No user names, emails, or KYC data.
/// Only public asset information and legal entity details.
use serde::Serialize;
use sqlx::PgPool;

// ═══════════════════════════════════════════════════════════════
// ── METADATA TYPES ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Top-level ERC-1155 metadata JSON.
#[derive(Debug, Serialize)]
pub struct AssetMetadata {
    /// Asset name (e.g., "Coastal Modern Villa – Bukit Peninsula")
    pub name: String,
    /// Short description of the tokenized asset
    pub description: String,
    /// URL to the asset detail page on POOOL
    pub external_url: String,
    /// Structured properties
    pub properties: AssetProperties,
}

/// Structured properties for the asset metadata.
#[derive(Debug, Serialize)]
pub struct AssetProperties {
    /// Issuer information
    pub issuer: String,
    /// Asset class (real_estate, agriculture, etc.)
    pub asset_class: String,
    /// Property sub-type (villa, apartment, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_type: Option<String>,
    /// Token standard used
    pub token_standard: String,
    /// Blockchain network
    pub chain: String,

    /// SPV (Special Purpose Vehicle) details
    pub spv: SpvDetails,
    /// Property / asset location
    pub location: LocationDetails,
    /// Token offering terms
    pub offering: OfferingTerms,
    /// Financial projections
    pub financials: FinancialDetails,
    /// Associated legal documents (IPFS URIs)
    pub documents: DocumentLinks,

    /// Schema version for future compatibility
    pub schema_version: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// SPV entity details — critical for proving legal ownership.
#[derive(Debug, Serialize)]
pub struct SpvDetails {
    /// Legal name of the SPV entity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_name: Option<String>,
    /// Registration / incorporation number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_number: Option<String>,
    /// Jurisdiction (e.g., "Indonesia")
    pub jurisdiction: String,
}

/// Location details.
#[derive(Debug, Serialize)]
pub struct LocationDetails {
    /// City  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// Country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Full address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// GPS latitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    /// GPS longitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
    /// Land size in square meters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub land_size_sqm: Option<i32>,
    /// Building size in square meters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_size_sqm: Option<i32>,
}

/// Token offering terms — all monetary values in cents.
#[derive(Debug, Serialize)]
pub struct OfferingTerms {
    /// Total number of tokens for this asset
    pub total_tokens: i32,
    /// Price per token in USD cents
    pub token_price_cents_usd: i64,
    /// Total asset valuation in USD cents
    pub total_valuation_cents_usd: i64,
    /// Currency code
    pub currency: String,
    /// When the offering was created
    pub offering_date: String,
    /// Current funding status
    pub funding_status: String,
}

/// Financial projection details.
#[derive(Debug, Serialize)]
pub struct FinancialDetails {
    /// Projected annual yield in basis points (850 = 8.50%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_yield_bps: Option<i32>,
    /// Term in months
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_months: Option<i32>,
    /// Operator name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_name: Option<String>,
}

/// IPFS document links (populated after pinning).
#[derive(Debug, Serialize)]
pub struct DocumentLinks {
    /// SPV operating agreement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spv_operating_agreement: Option<String>,
    /// Property title / certificate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_title: Option<String>,
    /// Independent appraisal report
    #[serde(skip_serializing_if = "Option::is_none")]
    pub independent_appraisal: Option<String>,
    /// Key Facts Statement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_facts_statement: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
// ── DB ROW TYPE ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Raw DB row for the metadata query. Uses named fields for clarity.
#[derive(Debug, sqlx::FromRow)]
struct AssetRow {
    id: uuid::Uuid,
    title: String,
    description: String,
    asset_type: String,
    property_type: Option<String>,
    tokens_total: i32,
    token_price_cents: i64,
    total_value_cents: i64,
    funding_status: String,
    location_city: Option<String>,
    location_country: Option<String>,
    location_address: Option<String>,
    location_description: Option<String>,
    location_lat: Option<f64>,
    location_lng: Option<f64>,
    land_size_sqm: Option<i32>,
    building_size_sqm: Option<i32>,
    annual_yield_bps: Option<i32>,
    term_months: Option<i32>,
    operator_name: Option<String>,
    chain_network: Option<String>,
    chain_metadata_uri: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// ═══════════════════════════════════════════════════════════════
// ── METADATA BUILDER ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════

/// Build ERC-1155 compliant metadata JSON for an asset from the database.
///
/// This reads the asset's data from PostgreSQL and constructs the full
/// metadata object. It does NOT pin to IPFS — call `pin_json()` separately.
pub async fn build_metadata(
    pool: &PgPool,
    asset_id: uuid::Uuid,
) -> Result<AssetMetadata, String> {
    let row = sqlx::query_as::<_, AssetRow>(
        r#"SELECT 
            id, title, description, asset_type, property_type,
            tokens_total, token_price_cents, total_value_cents, funding_status,
            location_city, location_country, location_address, location_description,
            location_lat::float8, location_lng::float8,
            land_size_sqm::int4, building_size_sqm::int4,
            annual_yield_bps, term_months, operator_name,
            chain_network, chain_metadata_uri, created_at, updated_at
        FROM assets WHERE id = $1"#,
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("DB error fetching asset: {}", e))?
    .ok_or_else(|| format!("Asset {} not found", asset_id))?;

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "https://platform.poool.app".to_string());
    let chain = row.chain_network.clone().unwrap_or_else(|| "polygon".to_string());

    let metadata = AssetMetadata {
        name: row.title.clone(),
        description: row.description.clone(),
        external_url: format!("{}/property/{}", base_url, asset_id),
        properties: AssetProperties {
            issuer: "PT POOOL Indonesia".to_string(),
            asset_class: row.asset_type.clone(),
            property_type: row.property_type.clone(),
            token_standard: "ERC-1155".to_string(),
            chain,
            spv: SpvDetails {
                legal_name: None,
                registration_number: None,
                jurisdiction: "Indonesia".to_string(),
            },
            location: LocationDetails {
                city: row.location_city,
                country: row.location_country,
                address: row.location_address,
                description: row.location_description,
                lat: row.location_lat,
                lng: row.location_lng,
                land_size_sqm: row.land_size_sqm,
                building_size_sqm: row.building_size_sqm,
            },
            offering: OfferingTerms {
                total_tokens: row.tokens_total,
                token_price_cents_usd: row.token_price_cents,
                total_valuation_cents_usd: row.total_value_cents,
                currency: "USD".to_string(),
                offering_date: row.created_at.format("%Y-%m-%d").to_string(),
                funding_status: row.funding_status,
            },
            financials: FinancialDetails {
                annual_yield_bps: row.annual_yield_bps,
                term_months: row.term_months,
                operator_name: row.operator_name,
            },
            documents: DocumentLinks {
                spv_operating_agreement: None,
                property_title: None,
                independent_appraisal: None,
                key_facts_statement: None,
            },
            schema_version: "1.0.0".to_string(),
            updated_at: row.updated_at.to_rfc3339(),
        },
    };

    Ok(metadata)
}

/// Convert AssetMetadata to a serde_json::Value for pinning.
pub fn metadata_to_json(metadata: &AssetMetadata) -> Result<serde_json::Value, String> {
    serde_json::to_value(metadata).map_err(|e| format!("Failed to serialize metadata: {}", e))
}
