use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::CookieJar;
use minijinja::context;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::HashMap;
use std::str::FromStr;

use super::models::{CommodityDisplayData, MarketplaceAsset, PropertyDisplayData};
use crate::auth::routes::AppState;

#[derive(Debug, Serialize)]
struct AssetDocumentDisplay {
    title: String,
    document_type: String,
    download_url: String,
    file_size_label: String,
}

fn is_investor_visible_asset_document(document_type: &str) -> bool {
    matches!(
        document_type,
        "proof_of_title"
            | "legal_basis"
            | "building_permit"
            | "site_plan"
            | "expose"
            | "appraisal"
            | "financial"
            | "floor_plan"
            | "other"
    )
}

fn parse_percent_to_bps(value: &str) -> Option<i32> {
    let decimal = Decimal::from_str(value.trim()).ok()?;
    let bps = decimal * Decimal::from(100);
    bps.round().to_i32()
}

fn format_bps_percent(bps: i32) -> String {
    let whole = bps / 100;
    let frac = (bps.abs() % 100) as u32;
    if frac == 0 {
        whole.to_string()
    } else if frac % 10 == 0 {
        format!("{}.{:01}", whole, frac / 10)
    } else {
        format!("{}.{:02}", whole, frac)
    }
}

fn format_file_size(bytes: Option<i64>) -> String {
    let Some(bytes) = bytes.filter(|b| *b > 0) else {
        return "File".to_string();
    };
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Escape HTML special characters to prevent XSS in manually-built HTML
/// (HTMX tab handlers bypass MiniJinja's auto-escaping).
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn safe_internal_error_page() -> Html<String> {
    Html(
        r#"<h1>Internal Server Error</h1><p>We could not load this page right now. Please try again.</p>"#
            .to_string(),
    )
}

fn safe_fragment_error(message: &str) -> Html<String> {
    Html(format!(
        r#"<div class="properties-section" role="alert" style="grid-column:1/-1;text-align:center;padding:40px;color:#B42318;">{}</div>"#,
        html_escape(message)
    ))
}

fn commodity_status_label(status: &str) -> &'static str {
    match status {
        "funding_open" | "funding_in_progress" => "Available",
        "funded" => "Funded",
        "rented" => "Rented",
        "exited" => "Exited",
        _ => "Upcoming",
    }
}

fn render_commodity_card(asset: &CommodityDisplayData) -> String {
    let slug = html_escape(&asset.slug);
    let title = html_escape(&asset.title);
    let location_city = html_escape(asset.location_city.as_deref().unwrap_or("Bali"));
    let location_country = html_escape(asset.location_country.as_deref().unwrap_or("ID"));
    let status = html_escape(&asset.funding_status);
    let duration_label = asset
        .term_months
        .filter(|months| *months > 0)
        .map(|months| format!("{} months", months))
        .unwrap_or_else(|| "N/A".to_string());
    let duration_data = if duration_label == "N/A" {
        String::new()
    } else {
        html_escape(&duration_label)
    };
    let hectares_label = asset
        .land_size_hectares
        .as_deref()
        .map(|hectares| format!("{} ha", hectares))
        .unwrap_or_else(|| "60 ha".to_string());
    let hectares_label = html_escape(&hectares_label);
    let price_dollars = asset.total_value_cents / 100;
    let price_usd = html_escape(&asset.total_value_usd);
    let funded_pct = asset.funded_percentage.clamp(0, 100);
    let status_label = commodity_status_label(&asset.funding_status);
    let annual_yield = html_escape(&asset.annual_yield_percent);
    let annual_yield_data = html_escape(&asset.annual_yield_percent);
    let appreciation = html_escape(&asset.capital_appreciation_percent);

    let mut images_html = String::new();
    let mut dots_html = String::new();
    let urls: Vec<String> = if asset.image_urls.is_empty() {
        vec![asset
            .cover_image_url
            .clone()
            .unwrap_or_else(|| "/static/images/seed/villa1.webp".to_string())]
    } else {
        asset.image_urls.clone()
    };

    for (index, url) in urls.iter().take(5).enumerate() {
        let active = if index == 0 { " active" } else { "" };
        let image_url = html_escape(&crate::storage::service::rewrite_gcs_url(url));
        images_html.push_str(&format!(
            r#"<img src="{image_url}" loading="lazy" class="property-image{active}" style="object-fit: cover; object-position: center;" alt="{title}">"#,
        ));
        dots_html.push_str(&format!(
            r#"<div class="property-dot{active}" data-property-id="{slug}" data-image-index="{index}"></div>"#,
        ));
    }

    format!(
        r##"<div class="property-card" data-property-id="{slug}"
                data-location="{location_city}, {location_country}"
                data-area="{location_city}"
                data-asset-type="commodity"
                data-funding-status="{status}"
                data-commodity-type="agriculture"
                data-price="{price_dollars}"
                data-yield="{annual_yield_data}"
                data-duration="{duration_data}">
            <div class="property-gallery">
                <div class="property-image-container">
                    {images_html}
                    <button class="property-nav-arrow property-nav-prev" onclick="event.stopPropagation(); cardPrevImage(this)" aria-label="Previous image">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
                    </button>
                    <button class="property-nav-arrow property-nav-next" onclick="event.stopPropagation(); cardNextImage(this)" aria-label="Next image">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
                    </button>
                    <div class="property-dots">{dots_html}</div>
                </div>
                <div class="property-badge ds-badge ds-badge--overlay ds-badge--commodity">
                    <span class="badge-text">Agricultural</span>
                </div>
            </div>
            <div class="property-content">
                <div class="card-meta-row">
                    <div class="card-meta-item">
                        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--text-secondary)" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22L2 12 12 2l10 10z"/><path d="M7.5 7.5L12 12"/></svg>
                        <span>{hectares_label}</span>
                    </div>
                    <div class="card-meta-divider"></div>
                    <div class="card-meta-item">
                        <img src="/static/images/{location_country}.webp" onerror="this.style.display='none'" width="16" height="16" style="border-radius:50%;object-fit:cover;flex-shrink:0;" alt="{location_country}">
                        <span>{location_city}, {location_country}</span>
                    </div>
                    <div class="card-meta-divider"></div>
                    <div class="card-meta-item">
                        <span>{status_label}</span>
                    </div>
                </div>
                <div class="property-heading">
                    <h3 class="property-title">
                        <a class="property-card-link" href="/commodity/{slug}" aria-label="View commodity {title}">{title}</a>
                    </h3>
                </div>
                <div class="property-pricing">
                    <div class="price-wrapper">
                        <span class="property-price">USD {price_usd}</span>
                        <span class="funded-percentage">{funded_pct}% funded</span>
                    </div>
                    <div class="property-progress ds-progress">
                        <div class="ds-progress__fill" style="width: {funded_pct}%"></div>
                    </div>
                </div>
                <div class="investment-details">
                    <div class="investment-row">
                        <span class="investment-label">Investment duration</span>
                        <span class="investment-value">{duration_label}</span>
                    </div>
                    <div class="investment-row">
                        <span class="investment-label">Projected return</span>
                        <span class="investment-value">{appreciation}%</span>
                    </div>
                    <div class="investment-row">
                        <span class="investment-label">Projected annualised net return</span>
                        <span class="investment-value">{annual_yield}%</span>
                    </div>
                </div>
            </div>
        </div>"##
    )
}

pub async fn page_marketplace(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    if !crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/auth/login").into_response();
    }

    let assets = match sqlx::query_as!(
        MarketplaceAsset,
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            a.short_description,
            a.description,
            a.asset_type,
            a.location_city,
            a.location_country,
            a.total_value_cents,
            a.token_price_cents,
            a.tokens_total,
            a.tokens_available,
            a.annual_yield_bps,
            a.capital_appreciation_bps,
            a.funding_status,
            ARRAY(
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
            a.bedrooms,
            a.bathrooms,
            a.building_size_sqm,
            a.lease_type,
            a.term_months,
            a.area,
            a.land_size_sqm,
            (
                SELECT COUNT(DISTINCT o.user_id)
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.asset_id = a.id
                  AND o.status = 'completed'
            ) AS "investor_count?",
            a.video_url,
            a.google_maps_url,
            a.location_description
        FROM assets a
        
        WHERE a.published = true
          AND a.asset_type != 'commodity'
          AND a.funding_status IN ('funding_open', 'funding_in_progress')
        ORDER BY a.featured DESC, a.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(assets) => assets,
        Err(e) => {
            tracing::error!("DATABASE ERROR fetching marketplace assets: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Html(
                    r#"<main class="ds-main marketplace-main"><div class="marketplace-content-wrapper"><div class="marketplace-error-state" role="alert" style="text-align:center;padding:80px 20px;color:#B42318;">We could not load properties right now. Please try again.</div></div></main>"#
                        .to_string(),
                ),
            )
                .into_response();
        }
    };

    let display_assets: Vec<PropertyDisplayData> =
        assets.iter().map(PropertyDisplayData::from_asset).collect();

    let is_empty = display_assets.is_empty();

    match state.templates.get_template("marketplace.html") {
        Ok(template) => {
            match template.render(context! { assets => display_assets, empty => is_empty }) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        safe_internal_error_page(),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Template missing: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                safe_internal_error_page(),
            )
                .into_response()
        }
    }
}

pub async fn page_property(
    jar: CookieJar,
    State(state): State<AppState>,
    path_slug: Option<Path<String>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // Accept slug from either /property/:slug (path) or /property?id=slug (query)
    let slug = path_slug
        .map(|Path(s)| s)
        .or_else(|| params.get("id").cloned());

    let Some(slug) = slug else {
        return Redirect::to("/marketplace").into_response();
    };

    let asset = match sqlx::query_as!(
        MarketplaceAsset,
        r#"
            SELECT
                a.id,
                a.title,
                a.slug,
                a.short_description,
                a.description,
                a.asset_type,
                a.location_city,
                a.location_country,
                a.total_value_cents,
                a.token_price_cents,
                a.tokens_total,
                a.tokens_available,
                a.annual_yield_bps,
                a.capital_appreciation_bps,
                a.funding_status,
                ARRAY(
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
                a.bedrooms,
            a.bathrooms,
            a.building_size_sqm,
            a.lease_type,
                a.term_months,
                a.area,
                a.land_size_sqm,
                (
                    SELECT COUNT(DISTINCT o.user_id)
                    FROM order_items oi
                    JOIN orders o ON oi.order_id = o.id
                    WHERE oi.asset_id = a.id
                      AND o.status = 'completed'
                ) AS "investor_count?",
                a.video_url,
                a.google_maps_url,
                a.location_description
            FROM assets a
            
            WHERE a.slug = $1 AND a.published = true
            "#,
        slug
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(asset)) => asset,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h1>Property not found</h1>".to_string()),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("DATABASE ERROR fetching property detail: {}", e);
            return safe_internal_error_page().into_response();
        }
    };

    // Record the page view in asset_views (fire-and-forget)
    // Idempotent by date and user to avoid unbounded writes per reload
    {
        let pool = state.db.clone();
        let asset_id = asset.id;
        let user_id = user.id;
        tokio::spawn(async move {
            // First check if already viewed today to avoid huge insert locks/errors
            let already_viewed: bool = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM asset_views WHERE asset_id = $1 AND user_id = $2 AND DATE(viewed_at) = CURRENT_DATE)",
                asset_id,
                user_id
            )
            .fetch_one(&pool)
            .await
            .unwrap_or(Some(false))
            .unwrap_or(false);

            if !already_viewed {
                if let Err(e) =
                    sqlx::query("INSERT INTO asset_views (asset_id, user_id) VALUES ($1, $2)")
                        .bind(asset_id)
                        .bind(user_id)
                        .execute(&pool)
                        .await
                {
                    tracing::warn!("Failed to record asset view: {}", e);
                }
            }
        });
    }

    // Fetch similar properties (up to 3, excluding the current one)
    let similar_assets = match sqlx::query_as!(
        MarketplaceAsset,
        r#"
            SELECT
                a.id,
                a.title,
                a.slug,
                a.short_description,
                a.description,
                a.asset_type,
                a.location_city,
                a.location_country,
                a.total_value_cents,
                a.token_price_cents,
                a.tokens_total,
                a.tokens_available,
                a.annual_yield_bps,
                a.capital_appreciation_bps,
                a.funding_status,
                ARRAY(
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
                a.bedrooms,
            a.bathrooms,
            a.building_size_sqm,
            a.lease_type,
                a.term_months,
                a.area,
                a.land_size_sqm,
                (
                    SELECT COUNT(DISTINCT o.user_id)
                    FROM order_items oi
                    JOIN orders o ON oi.order_id = o.id
                    WHERE oi.asset_id = a.id
                      AND o.status = 'completed'
                ) AS "investor_count?",
                a.video_url,
                a.google_maps_url,
                a.location_description
            FROM assets a
            
            WHERE a.published = true
              AND a.asset_type != 'commodity'
              AND a.id != $1
              AND a.funding_status IN ('funding_open', 'funding_in_progress')
            ORDER BY a.featured DESC, a.created_at DESC
            LIMIT 3
            "#,
        asset.id
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(assets) => assets,
        Err(e) => {
            tracing::error!("DATABASE ERROR fetching similar properties: {}", e);
            vec![]
        }
    };

    let documents: Vec<AssetDocumentDisplay> =
        match sqlx::query_as::<_, (uuid::Uuid, String, String, Option<i64>)>(
            r#"
        SELECT id, document_type, COALESCE(title, document_type), file_size_bytes
        FROM asset_documents
        WHERE asset_id = $1
          AND document_type = ANY($2::text[])
        ORDER BY document_type, created_at
        "#,
        )
        .bind(asset.id)
        .bind(&[
            "proof_of_title",
            "legal_basis",
            "building_permit",
            "site_plan",
            "expose",
            "appraisal",
            "financial",
            "floor_plan",
            "other",
        ])
        .fetch_all(&state.db)
        .await
        {
            Ok(rows) => rows
                .into_iter()
                .map(
                    |(id, document_type, title, file_size_bytes)| AssetDocumentDisplay {
                        title,
                        document_type,
                        download_url: format!("/api/documents/{}/download", id),
                        file_size_label: format_file_size(file_size_bytes),
                    },
                )
                .collect(),
            Err(e) => {
                tracing::error!("DATABASE ERROR fetching property documents: {}", e);
                Vec::new()
            }
        };

    // Convert to display-friendly data with pre-computed values
    let mut display_data = PropertyDisplayData::from_asset(&asset);

    let platform_fee_pct: f64 = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse::<f64>().ok())
    .unwrap_or(5.0);

    let fee_pct_display = if platform_fee_pct == platform_fee_pct.floor() {
        format!("{:.0}", platform_fee_pct)
    } else {
        format!("{:.1}", platform_fee_pct)
    };

    display_data.update_fee(platform_fee_pct);

    let similar_display: Vec<PropertyDisplayData> = similar_assets
        .iter()
        .map(|a| {
            let mut d = PropertyDisplayData::from_asset(a);
            d.update_fee(platform_fee_pct);
            d
        })
        .collect();

    match state.templates.get_template("property.html") {
        Ok(template) => match template
            .render(context! { asset => display_data, similar_assets => similar_display, documents => documents, fee_pct => platform_fee_pct, fee_pct_display => fee_pct_display })
        {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error: {}", e);
                Html(format!("<h1>Internal Server Error</h1><p>{}</p>", e)).into_response()
            }
        },
        Err(e) => {
            tracing::error!("Template missing: {}", e);
            Html("<h1>Internal Server Error: Template Missing</h1>".to_string()).into_response()
        }
    }
}

/// Public property detail page (no auth required).
///
/// Renders a published live property when one exists for the slug, otherwise
/// falls back to a clearly labelled landing-page preview from
/// [`super::public_assets`]. Uses the `property-public.html` template — the
/// same layout as the authenticated property page, but without the sidebar or
/// authenticated chrome.
///
/// Linked from the real-estate cards on `/landing-v2.html`. Returns 404 when
/// neither live inventory nor a public preview exists for the slug.
pub async fn page_property_public(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let live_asset = match sqlx::query_as!(
        MarketplaceAsset,
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            a.short_description,
            a.description,
            a.asset_type,
            a.location_city,
            a.location_country,
            a.total_value_cents,
            a.token_price_cents,
            a.tokens_total,
            a.tokens_available,
            a.annual_yield_bps,
            a.capital_appreciation_bps,
            a.funding_status,
            ARRAY(
                SELECT image_url
                FROM asset_images
                WHERE asset_id = a.id
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
            a.bedrooms,
            a.bathrooms,
            a.building_size_sqm,
            a.lease_type,
            a.term_months,
            a.area,
            a.land_size_sqm,
            (
                SELECT COUNT(DISTINCT o.user_id)
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.asset_id = a.id
                  AND o.status = 'completed'
            ) AS "investor_count?",
            a.video_url,
            a.google_maps_url,
            a.location_description
        FROM assets a
        WHERE a.slug = $1
          AND a.published = true
          AND a.asset_type != 'commodity'
        "#,
        slug.as_str()
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(asset) => asset,
        Err(e) => {
            tracing::error!(
                "Public property asset lookup failed for slug {}: {}",
                slug,
                e
            );
            return safe_internal_error_page().into_response();
        }
    };

    let mut display_data = if let Some(asset) = live_asset {
        PropertyDisplayData::from_asset(&asset)
    } else {
        match super::public_assets::lookup(&slug) {
            Some(d) => d,
            None => {
                return (
                    axum::http::StatusCode::NOT_FOUND,
                    Html("<h1>Property not found</h1>".to_string()),
                )
                    .into_response();
            }
        }
    };

    // Use integer basis points for fee display. Missing settings are allowed in
    // local/dev, but malformed values are logged so operators can fix config.
    let platform_fee_bps = match sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(value)) => parse_percent_to_bps(&value)
            .map(|bps| bps.max(0))
            .unwrap_or_else(|| {
                tracing::error!(
                    "Invalid platform_fee_percent setting for public property page: {}",
                    value
                );
                500
            }),
        Ok(None) => {
            tracing::warn!("platform_fee_percent missing; using 5% public display fallback");
            500
        }
        Err(e) => {
            tracing::error!(
                "Failed to read platform_fee_percent for public property page: {}",
                e
            );
            return safe_internal_error_page().into_response();
        }
    };

    let fee_pct_display = format_bps_percent(platform_fee_bps);
    display_data.update_fee_bps(platform_fee_bps);

    match state.templates.get_template("property-public.html") {
        Ok(template) => match template.render(context! {
            asset => display_data,
            fee_pct => fee_pct_display,
            fee_pct_display => fee_pct_display,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error (property-public): {}", e);
                Html(format!("<h1>Internal Server Error</h1><p>{}</p>", e)).into_response()
            }
        },
        Err(e) => {
            tracing::error!("Template missing (property-public): {}", e);
            Html("<h1>Internal Server Error: Template Missing</h1>".to_string()).into_response()
        }
    }
}

pub async fn page_commodity(
    jar: CookieJar,
    State(state): State<AppState>,
    path_slug: Option<Path<String>>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    if !crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/auth/login").into_response();
    }

    // Accept slug from either /commodity/:slug (path) or /commodity?id=slug (query)
    let slug = path_slug
        .map(|Path(s)| s)
        .or_else(|| params.get("id").cloned());

    let Some(slug) = slug.filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::NOT_FOUND,
            Html("<h1>Commodity not found</h1>".to_string()),
        )
            .into_response();
    };

    let asset = match sqlx::query_as!(
        super::models::CommodityAsset,
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            a.short_description,
            a.description,
            a.asset_type,
            a.location_city,
            a.location_country,
            a.total_value_cents,
            a.token_price_cents,
            a.tokens_total,
            a.tokens_available,
            a.annual_yield_bps,
            a.capital_appreciation_bps,
            a.funding_status,
            ARRAY(
                SELECT image_url
                FROM asset_images
                WHERE asset_id = a.id
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
            a.term_months,
            a.area,
            a.land_size_sqm,
            a.google_maps_url,
            a.video_url,
            (
                SELECT COUNT(DISTINCT o.user_id)
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.asset_id = a.id
                  AND o.status = 'completed'
            ) AS "investor_count?",
            a.operator_name,
            a.fixed_roi_bps,
            a.revenue_min_cents,
            a.revenue_max_cents,
            a.expenses_cents,
            a.net_profit_min_cents,
            a.net_profit_max_cents,
            a.investor_payout_cents,
            a.operator_split_pct,
            a.poool_split_pct,
            a.location_description
        FROM assets a
        WHERE a.slug = $1
          AND a.published = true
          AND a.asset_type = 'commodity'
        "#,
        slug
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(asset)) => asset,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h1>Commodity not found</h1>".to_string()),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "Commodity detail query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load commodity</h1>".to_string()),
            )
                .into_response();
        }
    };

    // Convert to display data
    let mut display_data = super::models::CommodityDisplayData::from_asset(&asset);

    // Query milestones/roadmap for this asset
    let milestones = match sqlx::query!(
        r#"
        SELECT title, description, month_index, is_completed
        FROM asset_milestones
        WHERE asset_id = $1
        ORDER BY COALESCE(month_index, 0), created_at ASC
        "#,
        asset.id
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(milestones) => milestones,
        Err(e) => {
            tracing::error!(asset_id = %asset.id, error = %e, "Commodity milestones query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load commodity</h1>".to_string()),
            )
                .into_response();
        }
    };

    // Build milestones context
    let milestones_ctx: Vec<minijinja::Value> = milestones
        .iter()
        .map(|m| {
            context! {
                title => m.title,
                description => m.description,
                month_index => m.month_index,
                is_completed => m.is_completed,
            }
        })
        .collect();

    let documents = match sqlx::query!(
        r#"
        SELECT id, document_type, title, file_size_bytes
        FROM asset_documents
        WHERE asset_id = $1
        ORDER BY document_type, created_at
        "#,
        asset.id
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .map(|d| AssetDocumentDisplay {
                title: d.title,
                document_type: d.document_type,
                download_url: format!("/api/documents/{}/download", d.id),
                file_size_label: format_file_size(d.file_size_bytes),
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!(asset_id = %asset.id, error = %e, "Commodity document query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load commodity</h1>".to_string()),
            )
                .into_response();
        }
    };

    // Query similar commodities (same asset_type, excluding current)
    let similar = match sqlx::query_as!(
        MarketplaceAsset,
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            a.short_description,
            a.description,
            a.asset_type,
            a.location_city,
            a.location_country,
            a.total_value_cents,
            a.token_price_cents,
            a.tokens_total,
            a.tokens_available,
            a.annual_yield_bps,
            a.capital_appreciation_bps,
            a.funding_status,
            ARRAY(
                SELECT image_url
                FROM asset_images
                WHERE asset_id = a.id
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
            a.bedrooms,
            a.bathrooms,
            a.building_size_sqm,
            a.lease_type,
            a.term_months,
            a.area,
            a.land_size_sqm,
            (
                SELECT COUNT(DISTINCT o.user_id)
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.asset_id = a.id
                  AND o.status = 'completed'
            ) AS "investor_count?",
            a.video_url,
            a.google_maps_url,
            a.location_description
        FROM assets a
        WHERE a.published = true
          AND a.asset_type = 'commodity'
          AND a.id != $1
        ORDER BY a.featured DESC, a.created_at DESC
        LIMIT 4
        "#,
        asset.id
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(similar) => similar,
        Err(e) => {
            tracing::error!(asset_id = %asset.id, error = %e, "Similar commodities query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load commodity</h1>".to_string()),
            )
                .into_response();
        }
    };
    let similar_assets = similar
        .iter()
        .map(PropertyDisplayData::from_asset)
        .collect::<Vec<_>>();

    // Fetch platform fee for display
    let platform_fee_bps = match sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(value)) => match parse_percent_to_bps(&value) {
            Some(bps) if bps >= 0 => bps,
            _ => {
                tracing::warn!(value = %value, "Invalid platform_fee_percent; using default");
                500
            }
        },
        Ok(None) => 500,
        Err(e) => {
            tracing::error!(error = %e, "Platform fee setting query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load commodity</h1>".to_string()),
            )
                .into_response();
        }
    };
    let fee_pct_display = format_bps_percent(platform_fee_bps);

    display_data.update_fee_bps(platform_fee_bps);

    match state.templates.get_template("commodity.html") {
        Ok(template) => match template.render(context! {
            asset => display_data,
            milestones => milestones_ctx,
            documents => documents,
            similar_assets => similar_assets,
            fee_pct => fee_pct_display,
            fee_pct_display => fee_pct_display,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!(error = %e, "Commodity template rendering error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Unable to load commodity</h1>".to_string()),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "Commodity template missing");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load commodity</h1>".to_string()),
            )
                .into_response()
        }
    }
}

/// GET /marketplace/tab – HTMX handler for asset tabs
pub async fn api_marketplace_tab(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let _user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
    };

    let tab = params
        .get("tab")
        .map(|s: &String| s.as_str())
        .unwrap_or("available");

    // Filter by funding status based on tab
    let status_filter: Vec<String> = match tab {
        "available" => vec![
            "funding_open".to_string(),
            "funding_in_progress".to_string(),
        ],
        "funded" => vec!["funded".to_string()],
        "exited" => vec!["exited".to_string()],
        _ => vec![
            "funding_open".to_string(),
            "funding_in_progress".to_string(),
        ],
    };

    let assets = match sqlx::query(
        r#"SELECT a.*, ARRAY(SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY is_cover DESC, created_at ASC) as image_urls
           FROM assets a
           WHERE a.published = true AND a.funding_status = ANY($1) AND a.asset_type != 'commodity'
           ORDER BY a.featured DESC, a.created_at DESC"#
    )
    .bind(status_filter)
    .fetch_all(&state.db)
    .await
    {
        Ok(assets) => assets,
        Err(e) => {
            tracing::error!("DATABASE ERROR fetching marketplace tab assets: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Html(
                    r#"<div id="marketplace-properties-section" class="properties-section"><div class="marketplace-error-state" role="alert" style="grid-column:1/-1;text-align:center;padding:40px;color:#B42318;">We could not load properties right now. Please try again.</div></div>"#
                        .to_string(),
                ),
            )
                .into_response();
        }
    };

    if assets.is_empty() {
        return Html(r#"<div id="marketplace-properties-section" class="properties-section"><div style="text-align:center; padding:40px; color:#666;">No properties found in this category.</div></div>"#.to_string()).into_response();
    }

    let mut html = String::from(
        r#"<div id="marketplace-properties-section" class="properties-section"><div id="property-grid" class="property-grid">"#,
    );

    use sqlx::Row;
    for asset in assets {
        let slug = html_escape(&asset.get::<String, _>("slug"));
        let title = html_escape(&asset.get::<String, _>("title"));
        let location_city = asset
            .get::<Option<String>, _>("location_city")
            .unwrap_or_else(|| "Bali".to_string());
        let location_country = asset
            .get::<Option<String>, _>("location_country")
            .unwrap_or_else(|| "ID".to_string());
        let area = asset.get::<Option<String>, _>("area").unwrap_or_default();
        let asset_type = asset.get::<String, _>("asset_type");
        let funding_status = asset.get::<String, _>("funding_status");
        let location_attr = html_escape(&format!("{}, {}", location_city, location_country));
        let area_attr = html_escape(&area);
        let asset_type_attr = html_escape(&asset_type);
        let funding_status_attr = html_escape(&funding_status);
        let total_value_cents = asset.get::<i64, _>("total_value_cents");
        let price_dollars = total_value_cents / 100;
        let price_usd = format_usd(price_dollars);

        let yield_bps = asset.get::<Option<i32>, _>("annual_yield_bps").unwrap_or(0);
        let appreciation_bps = asset
            .get::<Option<i32>, _>("capital_appreciation_bps")
            .unwrap_or(0);
        let term_months = asset.get::<Option<i32>, _>("term_months");

        let tokens_total = asset.get::<i32, _>("tokens_total");
        let tokens_available = asset.get::<i32, _>("tokens_available");
        let funded_pct = if tokens_total > 0 {
            (tokens_total - tokens_available) as f64 / tokens_total as f64 * 100.0
        } else {
            0.0
        };

        let image_urls: Vec<String> = asset.get("image_urls");
        let _cover_image = html_escape(
            &image_urls
                .first()
                .cloned()
                .unwrap_or_else(|| "/static/images/seed/villa1.webp".to_string()),
        );

        let bedrooms = asset.get::<Option<i32>, _>("bedrooms");
        let bathrooms = asset.get::<Option<i32>, _>("bathrooms");
        let building_sqm = asset.get::<Option<rust_decimal::Decimal>, _>("building_size_sqm");
        let land_sqm = asset.get::<Option<rust_decimal::Decimal>, _>("land_size_sqm");
        let lease_type = asset
            .get::<Option<String>, _>("lease_type")
            .unwrap_or_else(|| "Leasehold".to_string());
        let lease_label = lease_type_label(&lease_type);
        let badge_class = lease_badge_class(&lease_type);

        let sqm_label = building_sqm
            .or(land_sqm)
            .map(|sqm| format!("{:.0} m²", sqm))
            .unwrap_or_else(|| "— m²".to_string());
        let card_metrics_html = format!(
            r##"<div class="card-meta-item"><img src="/static/images/icons/Bed.svg" alt="Bedrooms" width="16" height="16"><span>{beds}</span></div>
                        <div class="card-meta-divider"></div>
                        <div class="card-meta-item"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M22 13a3 3 0 0 0-3-3H5a3 3 0 0 0-3 3v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2Z"/><path d="M7 17v2"/><path d="M17 17v2"/><path d="M21 10V6a2 2 0 0 0-2-2"/></svg><span>{baths}</span></div>
                        <div class="card-meta-divider"></div>
                        <div class="card-meta-item"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 4h16v16H4z"/><path d="M9 4v16"/><path d="M4 9h16"/></svg><span>{sqm}</span></div>"##,
            beds = bedrooms
                .map(|b| b.to_string())
                .unwrap_or_else(|| "—".to_string()),
            baths = bathrooms
                .map(|b| b.to_string())
                .unwrap_or_else(|| "—".to_string()),
            sqm = html_escape(&sqm_label),
        );

        let duration_label = match term_months {
            Some(m) if m > 0 => format!("{} months", m),
            _ => "N/A".to_string(),
        };
        let duration_attr = html_escape(&duration_label);
        let duration_months_attr = term_months
            .filter(|months| *months > 0)
            .map(|months| months.to_string())
            .unwrap_or_default();
        let yield_pct = (yield_bps as f64) / 100.0;
        let appreciation_pct = (appreciation_bps as f64) / 100.0;

        let mut images_html = String::new();
        let mut dots_html = String::new();
        let default_images = vec!["/static/images/seed/villa1.webp".to_string()];
        let urls = if image_urls.is_empty() {
            &default_images
        } else {
            &image_urls
        };

        for (i, url) in urls.iter().take(5).enumerate() {
            let active = if i == 0 { " active" } else { "" };
            let image_url = html_escape(&crate::storage::service::rewrite_gcs_url(url));
            images_html.push_str(&format!(
                r#"<div class="property-image{}" data-bg-image="{}" style="background-image: url('{}'); background-size: cover; background-position: center;"></div>"#,
                active, image_url, image_url
            ));
            dots_html.push_str(&format!(
                r#"<div class="property-dot{}" data-property-id="{}" data-image-index="{}"></div>"#,
                active, slug, i
            ));
        }

        html.push_str(&format!(
            r##"<div class="property-card" data-property-id="{slug}" data-location="{location_attr}" data-area="{area_attr}" data-asset-type="{asset_type_attr}" data-funding-status="{funding_status_attr}" data-duration="{duration_attr}" data-duration-months="{duration_months_attr}" data-price="{price_dollars}" data-card-url="/property/{slug}" onclick="window.location.href='/property/{slug}'">
                <div class="property-gallery">
                    <div class="property-image-container">
                        {images_html}
                        <button class="property-nav-arrow property-nav-prev" onclick="event.stopPropagation(); cardPrevImage(this)" aria-label="Previous image">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg>
                        </button>
                        <button class="property-nav-arrow property-nav-next" onclick="event.stopPropagation(); cardNextImage(this)" aria-label="Next image">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#333" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
                        </button>
                        <div class="property-dots">{dots_html}</div>
                    </div>
                    <div class="property-badge ds-badge ds-badge--overlay {badge_class}">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#3538CD" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 9h.01M9 15h6M15 9h.01"/></svg>
                        <span class="badge-text">{lease_label}</span>
                    </div>
                </div>
                <div class="property-content">
                    <div class="card-meta-row">
                        {card_metrics_html}
                    </div>
                    <div class="property-heading">
                        <h3 class="property-title"><a href="/property/{slug}" class="property-title-link" onclick="event.stopPropagation()">{title}</a></h3>
                    </div>
                    <div class="property-pricing">
                        <div class="price-wrapper">
                            <span class="property-price">USD {price_usd}</span>
                            <span class="funded-percentage">{funded_pct:.1}% funded</span>
                        </div>
                        <div class="property-progress ds-progress">
                            <div class="ds-progress__fill progress-bar" style="width: {funded_pct:.1}%;"></div>
                        </div>
                    </div>
                    <div class="investment-details">
                        <div class="investment-row">
                            <span class="investment-label">Investment duration</span>
                            <span class="investment-value">{duration_label}</span>
                        </div>
                        <div class="investment-row">
                            <span class="investment-label">Projected return</span>
                            <span class="investment-value">{appreciation_pct:.2}%</span>
                        </div>
                        <div class="investment-row">
                            <span class="investment-label">Projected annualised net return</span>
                            <span class="investment-value">{yield_pct:.2}%</span>
                        </div>
                    </div>
                </div>
            </div>"##,
            slug = slug,
            location_attr = location_attr,
            area_attr = area_attr,
            asset_type_attr = asset_type_attr,
            funding_status_attr = funding_status_attr,
            duration_attr = duration_attr,
            duration_months_attr = duration_months_attr,
            price_dollars = price_dollars,
            images_html = images_html, dots_html = dots_html,
            badge_class = badge_class,
            lease_label = lease_label,
            card_metrics_html = card_metrics_html,
            title = title,
            price_usd = price_usd,
            funded_pct = funded_pct,
            duration_label = duration_label,
            appreciation_pct = appreciation_pct,
            yield_pct = yield_pct,
        ));
    }

    html.push_str("</div></div>");
    Html(html).into_response()
}

/// GET /commodities-marketplace  Commodities marketplace (protected).
pub async fn page_commodities_marketplace(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let assets = match sqlx::query_as!(
        super::models::CommodityAsset,
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            a.short_description,
            a.description,
            a.asset_type,
            a.location_city,
            a.location_country,
            a.total_value_cents,
            a.token_price_cents,
            a.tokens_total,
            a.tokens_available,
            a.annual_yield_bps,
            a.capital_appreciation_bps,
            a.funding_status,
            ARRAY(
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
            a.term_months,
            a.area,
            a.land_size_sqm,
            (
                SELECT COUNT(DISTINCT o.user_id)
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.asset_id = a.id
                  AND o.status = 'completed'
            ) AS "investor_count?",
            a.video_url,
            a.google_maps_url,
            a.operator_name,
            a.fixed_roi_bps,
            a.revenue_min_cents,
            a.revenue_max_cents,
            a.expenses_cents,
            a.net_profit_min_cents,
            a.net_profit_max_cents,
            a.investor_payout_cents,
            a.operator_split_pct,
            a.poool_split_pct,
            a.location_description
        FROM assets a
        
        WHERE a.published = true
          AND a.asset_type = 'commodity'
          AND a.funding_status IN ('funding_open', 'funding_in_progress')
        ORDER BY a.featured DESC, a.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(assets) => assets,
        Err(e) => {
            tracing::error!(error = %e, "Commodities marketplace query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                safe_internal_error_page(),
            )
                .into_response();
        }
    };

    let display_assets: Vec<super::models::CommodityDisplayData> = assets
        .iter()
        .map(super::models::CommodityDisplayData::from_asset)
        .collect();

    let is_empty = display_assets.is_empty();

    match state.templates.get_template("commodities-marketplace.html") {
        Ok(template) => {
            match template
                .render(context! { assets => display_assets, empty => is_empty, user => user })
            {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!(error = %e, "Commodities marketplace template rendering error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        safe_internal_error_page(),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Commodities marketplace template missing");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                safe_internal_error_page(),
            )
                .into_response()
        }
    }
}

/// GET /commodities-marketplace/tab – HTMX handler for commodities tabs
pub async fn api_commodities_tab(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let _user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
    };

    let tab = params.get("tab").map(|s| s.as_str()).unwrap_or("available");

    let status_filter: Vec<String> = match tab {
        "available" => vec![
            "funding_open".to_string(),
            "funding_in_progress".to_string(),
        ],
        "funded" => vec!["funded".to_string()],
        "exited" => vec!["exited".to_string()],
        _ => vec![
            "funding_open".to_string(),
            "funding_in_progress".to_string(),
        ],
    };

    let assets = match sqlx::query_as!(
        super::models::CommodityAsset,
        r#"
        SELECT
            a.id,
            a.title,
            a.slug,
            a.short_description,
            a.description,
            a.asset_type,
            a.location_city,
            a.location_country,
            a.total_value_cents,
            a.token_price_cents,
            a.tokens_total,
            a.tokens_available,
            a.annual_yield_bps,
            a.capital_appreciation_bps,
            a.funding_status,
            ARRAY(
                SELECT image_url
                FROM asset_images
                WHERE asset_id = a.id
                ORDER BY is_cover DESC, created_at ASC
            ) AS "image_urls?",
            a.term_months,
            a.area,
            a.land_size_sqm,
            a.google_maps_url,
            a.video_url,
            (
                SELECT COUNT(DISTINCT o.user_id)
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.asset_id = a.id
                  AND o.status = 'completed'
            ) AS "investor_count?",
            a.operator_name,
            a.fixed_roi_bps,
            a.revenue_min_cents,
            a.revenue_max_cents,
            a.expenses_cents,
            a.net_profit_min_cents,
            a.net_profit_max_cents,
            a.investor_payout_cents,
            a.operator_split_pct,
            a.poool_split_pct,
            a.location_description
        FROM assets a
        WHERE a.published = true
          AND a.funding_status = ANY($1)
          AND a.asset_type = 'commodity'
        ORDER BY a.featured DESC, a.created_at DESC
        "#,
        &status_filter
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(assets) => assets,
        Err(e) => {
            tracing::error!(error = %e, tab = tab, "Commodities tab query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                safe_fragment_error("We could not load commodities right now. Please try again."),
            )
                .into_response();
        }
    };

    if assets.is_empty() {
        return Html(r#"<div id="commodities-properties-section" class="properties-section"><div role="status" style="text-align:center; padding:40px; color:#666;">No commodities found in this category.</div></div>"#).into_response();
    }

    let mut html = String::from(
        r#"<div id="commodities-properties-section" class="properties-section"><div id="property-grid" class="property-grid">"#,
    );

    for asset in assets {
        let display = CommodityDisplayData::from_asset(&asset);
        html.push_str(&render_commodity_card(&display));
    }

    html.push_str("</div></div>");
    Html(html).into_response()
}

#[derive(Serialize)]
pub struct SearchResult {
    pub title: String,
    pub subtitle: String,
    pub url: String,
    pub icon: String,
    pub image_url: Option<String>,
}

pub async fn api_asset_search(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if !crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        return (axum::http::StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let q = params.get("q").cloned().unwrap_or_default().to_lowercase();
    if q.len() < 2 {
        return Json(Vec::<SearchResult>::new()).into_response();
    }

    let search_query = format!("%{}%", q);
    let assets = sqlx::query!(
        r#"
        SELECT
            a.title,
            a.slug,
            a.asset_type,
            a.location_city,
            a.location_country,
            (
                SELECT ai.image_url
                FROM asset_images ai
                WHERE ai.asset_id = a.id
                ORDER BY ai.is_cover DESC, ai.sort_order ASC, ai.created_at ASC
                LIMIT 1
            ) AS image_url
        FROM assets a
        WHERE a.published = true
          AND (LOWER(a.title) LIKE $1 OR LOWER(a.location_city) LIKE $1 OR LOWER(a.location_country) LIKE $1 OR LOWER(a.slug) LIKE $1)
        ORDER BY a.featured DESC, a.created_at DESC
        LIMIT 8
        "#,
        search_query
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let results: Vec<SearchResult> = assets
        .into_iter()
        .map(|a| {
            let icon = if a.asset_type == "commodity" {
                "🌽"
            } else {
                "🏠"
            };
            let url = if a.asset_type == "commodity" {
                format!("/commodity/{}", a.slug)
            } else {
                format!("/property/{}", a.slug)
            };

            SearchResult {
                title: a.title,
                subtitle: format!(
                    "{} · {}, {}",
                    a.asset_type,
                    a.location_city.unwrap_or_default(),
                    a.location_country.unwrap_or_default()
                ),
                url,
                icon: icon.to_string(),
                image_url: crate::storage::service::rewrite_gcs_url_opt(a.image_url.as_deref()),
            }
        })
        .collect();

    Json(results).into_response()
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Format an integer dollar amount as comma-separated string, e.g. 1334000 → "1,334,000"
fn format_usd(dollars: i64) -> String {
    let s = dollars.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*ch);
    }
    result
}

/// CSS class for the badge based on lease_type
fn lease_badge_class(lease_type: &str) -> &'static str {
    match lease_type {
        "freehold" => "badge-freehold",
        "leasehold" => "badge-standard",
        _ => "badge-standard",
    }
}

fn lease_type_label(lease_type: &str) -> &'static str {
    match lease_type {
        "freehold" => "Freehold",
        "leasehold" => "Leasehold",
        "long_leasehold" => "Long Leasehold",
        _ => "Leasehold",
    }
}
