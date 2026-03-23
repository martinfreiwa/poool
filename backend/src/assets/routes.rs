use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Redirect},
};
use axum_extra::extract::CookieJar;
use minijinja::context;
use std::collections::HashMap;

use super::models::{MarketplaceAsset, PropertyDisplayData};
use crate::auth::routes::AppState;

/// Escape HTML special characters to prevent XSS in manually-built HTML
/// (HTMX tab handlers bypass MiniJinja's auto-escaping).
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn page_marketplace(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    if !crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/auth/login").into_response();
    }

    let assets = sqlx::query_as!(
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
    .map_err(|e| {
        tracing::error!("DATABASE ERROR fetching assets: {}", e);
        e
    })
    .unwrap_or_default();

    let is_empty = assets.is_empty();

    match state.templates.get_template("marketplace.html") {
        Ok(template) => match template.render(context! { assets => assets, empty => is_empty }) {
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

    let asset = if let Some(slug) = slug {
        sqlx::query_as!(
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
        .unwrap_or_default()
    } else {
        None
    };

    // Record the page view in asset_views (fire-and-forget)
    // Idempotent by date and user to avoid unbounded writes per reload
    if let Some(ref a) = asset {
        let pool = state.db.clone();
        let asset_id = a.id;
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
    let similar_assets = if let Some(ref a) = asset {
        sqlx::query_as!(
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
            a.id
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    // Convert similar assets to display-friendly data
    let similar_display: Vec<PropertyDisplayData> = similar_assets
        .iter()
        .map(PropertyDisplayData::from_asset)
        .collect();

    // Convert to display-friendly data with pre-computed values
    let display_data = asset.as_ref().map(PropertyDisplayData::from_asset);

    match state.templates.get_template("property.html") {
        Ok(template) => match template
            .render(context! { asset => display_data, similar_assets => similar_display })
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

pub async fn page_commodity(
    jar: CookieJar,
    State(state): State<AppState>,
    path_slug: Option<Path<String>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if !crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/auth/login").into_response();
    }

    // Accept slug from either /commodity/:slug (path) or /commodity?id=slug (query)
    let slug = path_slug
        .map(|Path(s)| s)
        .or_else(|| params.get("id").cloned());

    let asset = if let Some(ref slug) = slug {
        sqlx::query_as!(
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
            WHERE a.slug = $1 AND a.published = true
            "#,
            slug
        )
        .fetch_optional(&state.db)
        .await
        .unwrap_or_default()
    } else {
        None
    };

    // Convert to display data
    let display_data = asset
        .as_ref()
        .map(super::models::CommodityDisplayData::from_asset);

    // Query milestones/roadmap for this asset
    let milestones = if let Some(ref a) = asset {
        sqlx::query!(
            r#"
            SELECT title, description, month_index, is_completed
            FROM asset_milestones
            WHERE asset_id = $1
            ORDER BY COALESCE(month_index, 0), created_at ASC
            "#,
            a.id
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
    } else {
        vec![]
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

    // Query similar commodities (same asset_type, excluding current)
    let similar_assets = if let Some(ref a) = asset {
        let similar = sqlx::query_as!(
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
            a.id
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        similar
            .iter()
            .map(PropertyDisplayData::from_asset)
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    match state.templates.get_template("commodity.html") {
        Ok(template) => match template.render(context! {
            asset => display_data,
            milestones => milestones_ctx,
            similar_assets => similar_assets,
        }) {
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
    let status_filter = match tab {
        "available" => vec!["funding_open", "funding_in_progress"],
        "funded" => vec!["funded"],
        "exited" => vec!["exited"],
        _ => vec!["funding_open", "funding_in_progress"],
    };

    let assets = sqlx::query(
        r#"SELECT a.*, ARRAY(SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY is_cover DESC, created_at ASC) as image_urls
           FROM assets a
           WHERE a.published = true AND a.funding_status = ANY($1) AND a.asset_type != 'commodity'
           ORDER BY a.featured DESC, a.created_at DESC"#
    )
    .bind(status_filter)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

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
        let location_city = html_escape(
            &asset
                .get::<Option<String>, _>("location_city")
                .unwrap_or_else(|| "Bali".to_string()),
        );
        let location_country = html_escape(
            &asset
                .get::<Option<String>, _>("location_country")
                .unwrap_or_else(|| "ID".to_string()),
        );
        let total_value_cents = asset.get::<i64, _>("total_value_cents");
        let price_usd = format_usd(total_value_cents / 100);

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
                .unwrap_or_else(|| "/images/villa1.webp".to_string()),
        );

        let bedrooms = asset.get::<Option<i32>, _>("bedrooms");
        let lease_type = asset
            .get::<Option<String>, _>("lease_type")
            .unwrap_or_else(|| "Leasehold".to_string());
        let lease_label = lease_type_label(&lease_type);
        let badge_class = lease_badge_class(&lease_type);

        let status = asset.get::<String, _>("funding_status");
        let funding_status_label = match status.as_str() {
            "funding_open" | "funding_in_progress" => "Available",
            "funded" => "Funded",
            "rented" => "Rented",
            "exited" => "Exited",
            _ => "Upcoming",
        };

        let bedrooms_html = match bedrooms {
            Some(b) => {
                let stroke = "#535862";
                format!(
                    "<div class=\"card-meta-item\"><svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M2 4v16\"/><path d=\"M2 8h18a2 2 0 0 1 2 2v10\"/><path d=\"M2 17h20\"/><path d=\"M6 8v9\"/></svg><span>{b}</span></div><div class=\"card-meta-divider\"></div>",
                    stroke = stroke,
                    b = b,
                )
            }
            None => String::new(),
        };

        let duration_label = match term_months {
            Some(m) if m > 0 => format!("{} months", m),
            _ => "N/A".to_string(),
        };
        let yield_pct = (yield_bps as f64) / 100.0;
        let appreciation_pct = (appreciation_bps as f64) / 100.0;

        let mut images_html = String::new();
        let mut dots_html = String::new();
        let default_images = vec!["/images/villa1.webp".to_string()];
        let urls = if image_urls.is_empty() {
            &default_images
        } else {
            &image_urls
        };

        for (i, url) in urls.iter().enumerate() {
            let active = if i == 0 { " active" } else { "" };
            images_html.push_str(&format!(
                r#"<div class="property-image{}" style="background-image: url('{}'); background-size: cover; background-position: center;"></div>"#,
                active, html_escape(&crate::storage::service::rewrite_gcs_url(url))
            ));
            dots_html.push_str(&format!(r#"<div class="property-dot{}"></div>"#, active));
        }

        html.push_str(&format!(
            r##"<div class="property-card" data-property-id="{slug}" onclick="window.location.href='/property/{slug}'">
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
                    <div class="property-badge {badge_class}">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#3538CD" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 9h.01M9 15h6M15 9h.01"/></svg>
                        <span class="badge-text">{lease_label}</span>
                    </div>
                </div>
                <div class="property-content">
                    <div class="card-meta-row">
                        {bedrooms_html}
                        <div class="card-meta-item">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
                            <span>{funding_status_label}</span>
                        </div>
                        <div class="card-meta-divider"></div>
                        <div class="card-meta-item">
                            <img src="/images/{location_country}.webp" onerror="this.style.display='none'" width="16" height="16" style="border-radius:50%;object-fit:cover;flex-shrink:0;" alt="{location_country}">
                            <span>{location_city}, {location_country}</span>
                        </div>
                    </div>
                    <div class="property-heading">
                        <h3 class="property-title">{title}</h3>
                    </div>
                    <div class="property-pricing">
                        <div class="price-wrapper">
                            <span class="property-price">USD {price_usd}</span>
                            <span class="funded-percentage">{funded_pct:.1}% funded</span>
                        </div>
                        <div class="property-progress">
                            <div class="progress-bar" style="width: {funded_pct:.1}%;"></div>
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
            images_html = images_html, dots_html = dots_html,
            badge_class = badge_class,
            lease_label = lease_label,
            bedrooms_html = bedrooms_html,
            location_city = location_city,
            location_country = location_country,
            title = title,
            price_usd = price_usd,
            funded_pct = funded_pct,
            funding_status_label = funding_status_label,
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

    let assets = sqlx::query_as!(
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
          AND a.funding_status IN ('funding_open', 'funding_in_progress')
        ORDER BY a.featured DESC, a.created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let is_empty = assets.is_empty();

    match state.templates.get_template("commodities-marketplace.html") {
        Ok(template) => {
            match template.render(context! { assets => assets, empty => is_empty, user => user }) {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    tracing::error!("Template rendering error: {}", e);
                    Html(format!("<h1>Internal Server Error</h1><p>{}</p>", e)).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Template missing: {}", e);
            Html("<h1>Internal Server Error: Template Missing</h1>".to_string()).into_response()
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

    let status_filter = match tab {
        "available" => vec!["funding_open", "funding_in_progress"],
        "funded" => vec!["funded"],
        "exited" => vec!["exited"],
        _ => vec!["funding_open", "funding_in_progress"],
    };

    let assets = sqlx::query(
        r#"SELECT a.*, ARRAY(SELECT image_url FROM asset_images WHERE asset_id = a.id ORDER BY is_cover DESC, created_at ASC) as image_urls
           FROM assets a
           WHERE a.published = true AND a.funding_status = ANY($1) AND a.asset_type = 'commodity'
           ORDER BY a.featured DESC, a.created_at DESC"#
    )
    .bind(status_filter)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    if assets.is_empty() {
        return Html(r#"<div id="commodities-properties-section" class="properties-section"><div style="text-align:center; padding:40px; color:#666;">No commodities found in this category.</div></div>"#).into_response();
    }

    let mut html = String::from(
        r#"<div id="commodities-properties-section" class="properties-section"><div id="property-grid" class="property-grid">"#,
    );

    use sqlx::Row;
    for asset in assets {
        let slug = html_escape(&asset.get::<String, _>("slug"));
        let title = html_escape(&asset.get::<String, _>("title"));
        let location_city = html_escape(
            &asset
                .get::<Option<String>, _>("location_city")
                .unwrap_or_else(|| "Bali".to_string()),
        );
        let location_country = html_escape(
            &asset
                .get::<Option<String>, _>("location_country")
                .unwrap_or_else(|| "ID".to_string()),
        );
        let price_cents = asset.get::<i64, _>("total_value_cents");
        let price_usd = format_usd(price_cents / 100);
        let yield_bps = asset.get::<Option<i32>, _>("annual_yield_bps").unwrap_or(0);
        let appreciation_bps = asset
            .get::<Option<i32>, _>("capital_appreciation_bps")
            .unwrap_or(0);
        let term_months = asset.get::<Option<i32>, _>("term_months");

        let tokens_total = asset.get::<i32, _>("tokens_total");
        let tokens_available = asset.get::<i32, _>("tokens_available");
        let funded_pct: f64 = if tokens_total > 0 {
            (tokens_total - tokens_available) as f64 / tokens_total as f64 * 100.0
        } else {
            0.0
        };

        let image_urls: Vec<String> = asset.get("image_urls");
        let _cover_image = html_escape(
            &image_urls
                .first()
                .cloned()
                .unwrap_or_else(|| "/images/villa1.webp".to_string()),
        );

        let status = asset.get::<String, _>("funding_status");
        let funding_status_label = match status.as_str() {
            "funding_open" | "funding_in_progress" => "Available",
            "funded" => "Funded",
            "rented" => "Rented",
            "exited" => "Exited",
            _ => "Upcoming",
        };

        let duration_label = match term_months {
            Some(m) if m > 0 => format!("{} months", m),
            _ => "N/A".to_string(),
        };

        // Convert land_size_sqm to hectares (1 ha = 10,000 sqm)
        use rust_decimal::prelude::ToPrimitive;
        let land_sqm: Option<rust_decimal::Decimal> =
            asset.get::<Option<rust_decimal::Decimal>, _>("land_size_sqm");
        let hectares_html = match land_sqm {
            Some(sqm) => {
                let ha = (sqm / rust_decimal::Decimal::from(10_000))
                    .to_f64()
                    .unwrap_or(0.0);
                let ha_display = if ha == ha.floor() {
                    format!("{:.0}", ha)
                } else {
                    format!("{:.1}", ha)
                };
                let stroke = "#535862";
                format!(
                    "<div class=\"card-meta-item\"><svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M12 22L2 12 12 2l10 10z\"/><path d=\"M7.5 7.5L12 12\"/></svg><span>{ha} ha</span></div><div class=\"card-meta-divider\"></div>",
                    stroke = stroke,
                    ha = ha_display,
                )
            }
            None => {
                let stroke = "#535862";
                format!(
                    "<div class=\"card-meta-item\"><svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M12 22L2 12 12 2l10 10z\"/><path d=\"M7.5 7.5L12 12\"/></svg><span>60 ha</span></div><div class=\"card-meta-divider\"></div>",
                    stroke = stroke,
                )
            }
        };

        let yield_pct = (yield_bps as f64) / 100.0;
        let appreciation_pct = (appreciation_bps as f64) / 100.0;

        let mut images_html = String::new();
        let mut dots_html = String::new();
        let default_images = vec!["/images/villa1.webp".to_string()];
        let urls = if image_urls.is_empty() {
            &default_images
        } else {
            &image_urls
        };

        for (i, url) in urls.iter().enumerate() {
            let active = if i == 0 { " active" } else { "" };
            images_html.push_str(&format!(
                r#"<div class="property-image{}" style="background-image: url('{}'); background-size: cover; background-position: center;"></div>"#,
                active, html_escape(&crate::storage::service::rewrite_gcs_url(url))
            ));
            dots_html.push_str(&format!(r#"<div class="property-dot{}"></div>"#, active));
        }

        html.push_str(&format!(
            r##"<div class="property-card" data-property-id="{slug}"
                    data-location="{location_city}, {location_country}"
                    data-asset-type="commodity"
                    data-funding-status="{status}"
                    onclick="window.location.href='/commodity/{slug}'">
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
                    <div class="property-badge badge-commodity">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#3538CD" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>
                        <span class="badge-text">Commodity</span>
                    </div>
                </div>
                <div class="property-content">
                    <div class="card-meta-row">
                        {hectares_html}
                        <div class="card-meta-item">
                            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#414651" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
                            <span>{funding_status_label}</span>
                        </div>
                        <div class="card-meta-divider"></div>
                        <div class="card-meta-item">
                            <img src="/images/{location_country}.webp" onerror="this.style.display='none'" width="16" height="16" style="border-radius:50%;object-fit:cover;flex-shrink:0;" alt="{location_country}">
                            <span>{location_city}, {location_country}</span>
                        </div>
                    </div>
                    <div class="property-heading">
                        <h3 class="property-title">{title}</h3>
                    </div>
                    <div class="property-pricing">
                        <div class="price-wrapper">
                            <span class="property-price">USD {price_usd}</span>
                            <span class="funded-percentage">{funded_pct:.1}% funded</span>
                        </div>
                        <div class="property-progress">
                            <div class="progress-bar" style="width: {funded_pct:.1}%;"></div>
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
            images_html = images_html, dots_html = dots_html,
            hectares_html = hectares_html,
            location_city = location_city,
            location_country = location_country,
            status = status,
            title = title,
            price_usd = price_usd,
            funded_pct = funded_pct,
            funding_status_label = funding_status_label,
            duration_label = duration_label,
            yield_pct = yield_pct,
            appreciation_pct = appreciation_pct,
        ));
    }

    html.push_str("</div></div>");
    Html(html).into_response()
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

/// Human-readable label for lease_type DB value
fn lease_type_label(lease_type: &str) -> &'static str {
    match lease_type {
        "freehold" => "Freehold",
        "leasehold" => "Standard Leasehold",
        _ => "Standard Leasehold",
    }
}

/// CSS class for the badge based on lease_type
fn lease_badge_class(lease_type: &str) -> &'static str {
    match lease_type {
        "freehold" => "badge-freehold",
        "leasehold" => "badge-standard",
        _ => "badge-standard",
    }
}
