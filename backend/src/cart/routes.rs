/// Cart route handlers – CRUD for cart_items via HTMX + JSON API.
///
/// The cart is backed by the `cart_items` PostgreSQL table.
/// Each user can have at most one row per asset (enforced by UNIQUE constraint).
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form, Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::middleware;
use crate::auth::routes::AppState;

// ─── Form data ──────────────────────────────────────────────────

/// Submitted by `property-detail-cart.js` when clicking "Add to Cart".
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AddToCartForm {
    pub property_id: Option<String>,
    pub investment_amount: Option<String>,
    pub property_title: Option<String>,
    pub property_image: Option<String>,
    pub location: Option<String>,
    pub unit_price: Option<String>,
    pub monthly_rent: Option<String>,
    pub appreciation: Option<String>,
    pub funded_percentage: Option<String>,
    pub duration: Option<String>,
    pub projected_return: Option<String>,
    pub annualized_return: Option<String>,
}

/// Submitted when removing an item from the cart page.
#[derive(Debug, Deserialize)]
pub struct RemoveFromCartForm {
    pub cart_item_id: String,
}

/// Submitted when updating a quantity on the cart page.
#[derive(Debug, Deserialize)]
pub struct UpdateCartForm {
    pub cart_item_id: String,
    pub tokens_quantity: i32,
}

// ─── API response types ─────────────────────────────────────────

/// A single cart item returned by GET /api/cart.
#[derive(Debug, Serialize)]
pub struct CartItemView {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub tokens_quantity: i32,
    pub token_price_cents: i64,
    pub total_cents: i64,
    // Joined from assets table
    pub title: String,
    pub slug: String,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub short_description: Option<String>,
    pub asset_type: String,
    pub annual_yield_bps: Option<i32>,
    pub funding_status: String,
    pub tokens_available: i32,
    pub tokens_total: i32,
    pub bedrooms: Option<i32>,
    pub bathrooms: Option<i32>,
    pub building_size_sqm: Option<f64>,
    pub land_size_sqm: Option<f64>,
    /// Cover image URL from asset_images (first cover or first by sort order).
    pub cover_image_url: Option<String>,
}

/// Helper to format cents into "USD X,XXX.XX" with commas
fn format_cart_usd(cents: i64) -> String {
    let abs_cents = cents.unsigned_abs();
    let dollars = abs_cents / 100;
    let remainder = abs_cents % 100;
    let s = dollars.to_string();
    let mut result = String::new();
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if i > 0 && (b.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c as char);
    }
    format!("USD {}.{:02}", result, remainder)
}

fn format_idr(cents: i64) -> String {
    // Must match the rate in payments/service.rs::execute_checkout (15,500)
    // TODO: Centralize FX rate into a shared config / live API call
    let idr_conversion_rate: i64 = crate::config::DEFAULT_USD_TO_IDR_RATE_I64;
    // Integer math: cents → dollars → IDR (no float rounding)
    let idr_val = (cents / 100) * idr_conversion_rate;
    let is_negative = idr_val < 0;
    let val = idr_val.abs().to_string();
    let mut result = String::new();
    if is_negative {
        result.push('-');
    }
    let bytes = val.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push('.');
        }
        result.push(c as char);
    }
    format!("Rp {}", result)
}

// ─── Handlers ───────────────────────────────────────────────────

/// POST /cart/add – Add a property/asset to the current user's cart.
///
/// Called from property detail page. If the asset is already in the
/// cart, increases the token quantity.
pub async fn add_to_cart(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<AddToCartForm>,
) -> impl IntoResponse {
    // 1. Authenticate
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // 2. Enforce KYC — must be approved before adding to cart
    let kyc_approved = match crate::kyc::service::get_kyc_status(&state.db, user.id).await {
        Ok(kyc) => kyc.status == "approved",
        Err(_) => false,
    };
    if !kyc_approved {
        tracing::warn!(user_id = %user.id, "Blocked add-to-cart: KYC not approved");
        return Redirect::to("/kyc?reason=required").into_response();
    }

    // 3. Parse the investment amount (form sends e.g. "2000" or "2,000")
    let raw_amount = form.investment_amount.unwrap_or_else(|| "500".to_string());
    let amount_str = raw_amount.replace(',', "");
    // Parse dollars to cents using string manipulation to avoid float rounding errors
    let amount_cents: i64 = {
        let parts: Vec<&str> = amount_str.split('.').collect();
        let dollars: i64 = parts[0].parse().unwrap_or(500);
        let cents: i64 = if parts.len() > 1 {
            let frac = parts[1];
            match frac.len() {
                0 => 0,
                1 => frac.parse::<i64>().unwrap_or(0) * 10,
                _ => frac[..2].parse::<i64>().unwrap_or(0),
            }
        } else {
            0
        };
        let total = dollars * 100 + cents;
        if total <= 0 {
            50_000
        } else {
            total
        } // Default $500
    };

    // 3. Resolve asset_id from property_id – can be a UUID *or* a slug
    //    (the marketplace links use UUID; direct /property/:slug uses the slug)
    let property_id = form.property_id.unwrap_or_else(|| "property-1".to_string());

    // Try by UUID first, then fall back to slug lookup
    // Use a transaction with FOR UPDATE to prevent TOCTOU race on tokens_available
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to begin add-to-cart transaction: {}", e);
            return Redirect::to("/cart").into_response();
        }
    };

    let asset_row = if let Ok(uuid) = property_id.parse::<Uuid>() {
        sqlx::query_as::<_, (Uuid, i64, i32)>(
            "SELECT id, token_price_cents, tokens_available FROM assets WHERE id = $1 FOR UPDATE LIMIT 1",
        )
        .bind(uuid)
        .fetch_optional(&mut *tx)
        .await
    } else {
        sqlx::query_as::<_, (Uuid, i64, i32)>(
            "SELECT id, token_price_cents, tokens_available FROM assets WHERE slug = $1 FOR UPDATE LIMIT 1",
        )
        .bind(&property_id)
        .fetch_optional(&mut *tx)
        .await
    };

    let (asset_id, token_price_cents, _tokens_available) = match asset_row {
        Ok(Some(row)) => row,
        _ => {
            // Asset not found – still redirect to cart (graceful degradation).
            tracing::warn!(
                "Asset not found for id/slug '{}', skipping cart add",
                property_id
            );
            let _ = tx.commit().await;
            return Redirect::to("/cart").into_response();
        }
    };

    // 4. Calculate tokens to buy (capped at available)
    let tokens_to_buy = if token_price_cents > 0 {
        let desired = std::cmp::max(1, (amount_cents / token_price_cents) as i32);
        std::cmp::min(desired, _tokens_available)
    } else {
        1
    };

    // 5. Upsert into cart_items (UNIQUE on user_id + asset_id)
    let result = sqlx::query(
        r#"
        INSERT INTO cart_items (user_id, asset_id, tokens_quantity, token_price_cents)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, asset_id) DO UPDATE
        SET tokens_quantity = LEAST(cart_items.tokens_quantity + $3, $5),
            token_price_cents = $4
        "#,
    )
    .bind(user.id)
    .bind(asset_id)
    .bind(tokens_to_buy)
    .bind(token_price_cents)
    .bind(_tokens_available)
    .execute(&mut *tx)
    .await;

    if let Err(e) = &result {
        tracing::error!("Cart upsert failed: {}", e);
        let _ = tx.rollback().await;
        return Redirect::to("/cart").into_response();
    }

    let _ = tx.commit().await;

    match result {
        Ok(_) => {
            tracing::info!(
                user_id = %user.id,
                asset_id = %asset_id,
                tokens = tokens_to_buy,
                "Item added to cart"
            );
        }
        Err(e) => {
            tracing::error!("Failed to add to cart: {}", e);
        }
    }

    // 6. Redirect to cart page
    Redirect::to("/cart").into_response()
}

/// POST /cart/remove – Remove an item from the cart.
pub async fn remove_from_cart(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<RemoveFromCartForm>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let cart_item_id = match Uuid::parse_str(&form.cart_item_id) {
        Ok(id) => id,
        Err(_) => return Redirect::to("/cart").into_response(),
    };

    let _ = sqlx::query("DELETE FROM cart_items WHERE id = $1 AND user_id = $2")
        .bind(cart_item_id)
        .bind(user.id)
        .execute(&state.db)
        .await;

    Redirect::to("/cart").into_response()
}

/// POST /cart/update – Update the quantity/amount for a cart item.
pub async fn update_cart_item(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<UpdateCartForm>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let cart_item_id = match Uuid::parse_str(&form.cart_item_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid cart item ID"})),
            )
                .into_response()
        }
    };

    let requested_tokens = std::cmp::max(1, form.tokens_quantity);

    // Use a transaction with FOR UPDATE to prevent TOCTOU race on tokens_available
    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to begin update_cart_item transaction: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Server error"})),
            )
                .into_response();
        }
    };

    // Fetch available tokens for this asset with row lock
    let max_avail: Option<i32> = sqlx::query_scalar(
        "SELECT a.tokens_available FROM assets a JOIN cart_items ci ON ci.asset_id = a.id WHERE ci.id = $1 FOR UPDATE OF a"
    )
    .bind(cart_item_id)
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    let tokens = match max_avail {
        Some(avail) => std::cmp::min(requested_tokens, avail),
        None => requested_tokens,
    };

    let result =
        sqlx::query("UPDATE cart_items SET tokens_quantity = $1 WHERE id = $2 AND user_id = $3")
            .bind(tokens)
            .bind(cart_item_id)
            .bind(user.id)
            .execute(&mut *tx)
            .await;

    match result {
        Ok(_) => {
            let _ = tx.commit().await;
            Json(serde_json::json!({"success": true, "tokens_quantity": tokens})).into_response()
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!("Failed to update cart item: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to update cart item"})),
            )
                .into_response()
        }
    }
}

/// GET /api/cart – Return the logged-in user's cart items as JSON.
pub async fn api_cart(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
                .into_response();
        }
    };

    let rows = sqlx::query(
        r#"
        SELECT
            ci.id, ci.asset_id, ci.tokens_quantity, a.token_price_cents as token_price_cents,
            a.title, a.slug,
            a.location_city, a.location_country, a.short_description,
            a.asset_type, a.annual_yield_bps, a.funding_status, a.tokens_available, a.tokens_total,
            a.bedrooms, a.bathrooms,
            a.building_size_sqm::FLOAT8 as building_size_sqm,
            a.land_size_sqm::FLOAT8 as land_size_sqm,
            (SELECT image_url FROM asset_images ai WHERE ai.asset_id = a.id ORDER BY ai.is_cover DESC, ai.sort_order ASC, ai.created_at ASC LIMIT 1) as cover_image_url
        FROM cart_items ci
        JOIN assets a ON a.id = ci.asset_id
        WHERE ci.user_id = $1
        ORDER BY ci.created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await;

    use sqlx::Row;

    match rows {
        Ok(rows) => {
            let views: Vec<CartItemView> = rows
                .iter()
                .map(|r| {
                    let tokens_qty: i32 = r.get("tokens_quantity");
                    let price_cents: i64 = r.get("token_price_cents");
                    CartItemView {
                        id: r.get("id"),
                        asset_id: r.get("asset_id"),
                        tokens_quantity: tokens_qty,
                        token_price_cents: price_cents,
                        total_cents: tokens_qty as i64 * price_cents,
                        title: r.get("title"),
                        slug: r.get("slug"),
                        location_city: r.get("location_city"),
                        location_country: r.get("location_country"),
                        short_description: r.get("short_description"),
                        asset_type: r.get("asset_type"),
                        annual_yield_bps: r.get("annual_yield_bps"),
                        funding_status: r.get("funding_status"),
                        tokens_available: r.get("tokens_available"),
                        tokens_total: r.get("tokens_total"),
                        bedrooms: r.get("bedrooms"),
                        bathrooms: r.get("bathrooms"),
                        building_size_sqm: r.get("building_size_sqm"),
                        land_size_sqm: r.get("land_size_sqm"),
                        cover_image_url: r.get("cover_image_url"),
                    }
                })
                .collect();

            Json(serde_json::json!({
                "items": views,
                "count": views.len(),
                "total_cents": views.iter().map(|i| i.total_cents).sum::<i64>()
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch cart: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch cart"})),
            )
                .into_response()
        }
    }
}

/// GET /cart – Serve the cart page.
///
/// Reads cart_items from the DB. If the cart has items, injects
/// them into the HTML. Otherwise, serves the default empty-state page.
pub async fn page_cart(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // Fetch cart items with asset details
    let rows = sqlx::query(
        r#"
        SELECT
            ci.id, ci.asset_id, ci.tokens_quantity, a.token_price_cents as token_price_cents,
            a.title, a.slug,
            a.location_city, a.location_country,
            a.asset_type, a.annual_yield_bps, a.funding_status,
            a.tokens_available, a.tokens_total,
            (SELECT image_url FROM asset_images ai WHERE ai.asset_id = a.id ORDER BY ai.is_cover DESC, ai.sort_order ASC, ai.created_at ASC LIMIT 1) as image_url,
            a.bedrooms, a.bathrooms,
            a.building_size_sqm::FLOAT8 as building_size_sqm,
            a.land_size_sqm::FLOAT8 as land_size_sqm
        FROM cart_items ci
        JOIN assets a ON a.id = ci.asset_id
        WHERE ci.user_id = $1
        ORDER BY ci.created_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(rows) => rows,
        Err(e) => {
            println!("Cart SQL Error: {:?}", e);
            vec![]
        }
    };

    // Read the base cart.html
    let template = match state.templates.get_template("cart.html") {
        Ok(t) => t,
        Err(_) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response();
        }
    };
    let html = match template.render(minijinja::context! {}) {
        Ok(content) => content,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Internal Server Error</h1>".to_string()),
            )
                .into_response();
        }
    };

    if rows.is_empty() {
        // No items – hide static demo items and show the real empty cart state.
        // Replace the Cart Content section (static demo items) with just the empty state marker
        let mut empty_html = {
            let start_marker = "<!-- Cart Content -->";
            let end_marker = "<!-- Empty Cart State -->";
            if let (Some(start), Some(end)) = (html.find(start_marker), html.find(end_marker)) {
                // Replace the static demo items with just the empty cart content wrapper (no items)
                // We MUST re-open cart-page-content because it is closed properly by the template later
                format!(
                    "{}<!-- Cart Content -->\n<div id=\"cart-page-content\" class=\"cart-page-content\">\n<!-- Empty Cart State -->{}",
                    &html[..start],
                    &html[end + end_marker.len()..]
                )
            } else {
                html.clone()
            }
        };
        // Show the empty cart state container (hidden by default in static HTML)
        empty_html = empty_html.replace(
            r#"id="cart-page-empty-container" class="cart-empty-wrapper" style="display: none""#,
            r#"id="cart-page-empty-container" class="cart-empty-wrapper""#,
        );
        empty_html = empty_html.replace(
            r#"class="mobile-cart-empty-container" style="display: none""#,
            r#"class="mobile-cart-empty-container""#,
        );
        // Add cart-empty-state class to main to fix CSS grid layout for the empty cart view
        empty_html = empty_html.replace(
            r#"<main id="cart-main" class="ds-main cart-page-main">"#,
            r#"<main id="cart-main" class="ds-main cart-page-main cart-empty-state">"#,
        );
        return Html(empty_html).into_response();
    }

    // Build populated cart HTML
    let mut cart_items_html = String::new();
    let mut mobile_items_html = String::new();
    let mut summary_items_html = String::new();
    let mut total_cents: i64 = 0;
    let mut primary_items: Vec<(String, i64, i64)> = Vec::new(); // (Title, Price, Tokens)

    use sqlx::Row;


    for (idx, row) in rows.iter().enumerate() {
        let ci_id: Uuid = row.get("id");
        let _asset_id: Uuid = row.get("asset_id");
        let tokens_qty: i32 = row.get("tokens_quantity");
        let token_price_cents: i64 = row.get("token_price_cents");
        let title: String = row.get("title");
        let slug: String = row.get("slug");
        let location_city: Option<String> = row.get("location_city");
        let location_country: Option<String> = row.get("location_country");
        let _asset_type: String = row.get("asset_type");
        let annual_yield_bps: Option<i32> = row.get("annual_yield_bps");
        let funding_status: String = row.get("funding_status");
        let tokens_available: i32 = row.get("tokens_available");

        let tokens_total: i32 = row.get("tokens_total");
        let image_url: Option<String> = row.get("image_url");
        let bedrooms: Option<i32> = row.get("bedrooms");
        let bathrooms: Option<i32> = row.get("bathrooms");
        let building_sqm: Option<f64> = row.get("building_size_sqm");
        let land_sqm: Option<f64> = row.get("land_size_sqm");

        let item_total = tokens_qty as i64 * token_price_cents;
        total_cents += item_total;

        if funding_status == "funding_open" || funding_status == "funding_in_progress" {
            primary_items.push((title.clone(), token_price_cents, tokens_qty as i64));
        }

        // Build per-item summary line for the Order Summary card

        let truncated_title = if title.len() > 30 {
            format!("{}…", &title[..29])
        } else {
            title.clone()
        };
        let item_usd = format_cart_usd(item_total);
        let item_idr = format_idr(item_total);
        let token_price_usd = token_price_cents / 100;
        summary_items_html.push_str(&format!(
            r#"<div class="cart-summary-item">
                <div class="cart-summary-item__row">
                    <span class="cart-summary-item__name">{title}</span>
                    <span class="cart-summary-item__usd">{usd}</span>
                </div>
                <div class="cart-summary-item__row cart-summary-item__row--sub">
                    <span class="cart-summary-item__qty">{qty} × ${price}</span>
                    <span class="cart-summary-item__idr">≈ {idr}</span>
                </div>
            </div>"#,
            title = truncated_title,
            qty = tokens_qty,
            price = token_price_usd,
            usd = item_usd,
            idr = item_idr,
        ));

        let _bps = annual_yield_bps.unwrap_or(0);

        // Map country code to flag emoji
        let flag = location_country
            .as_deref()
            .map(|cc| {
                let cc = cc.to_uppercase();
                if cc.len() == 2 {
                    let mut flag = String::new();
                    for c in cc.chars() {
                        if let Some(regional) = char::from_u32(0x1F1E6 + (c as u32 - 'A' as u32)) {
                            flag.push(regional);
                        }
                    }
                    flag
                } else {
                    String::new()
                }
            })
            .unwrap_or_default();

        let location = match (&location_city, &location_country) {
            (Some(city), Some(country)) => format!("{} {}, {}", flag, city, country),
            (Some(city), None) => city.clone(),
            (None, Some(country)) => format!("{} {}", flag, country),
            _ => "Location N/A".to_string(),
        };

        let price_display = format_cart_usd(item_total);
        let token_price_display = format!("${}", token_price_cents / 100);
        let yield_display = annual_yield_bps
            .map(|bps| format!("{:.1}%", bps as f64 / 100.0))
            .unwrap_or_else(|| "0.0%".to_string());

        let total_tokens_f64 = tokens_total as f64;
        let available_tokens_f64 = tokens_available as f64;

        let funded_pct = if total_tokens_f64 > 0.0 {
            let sold_tokens = total_tokens_f64 - available_tokens_f64;
            ((sold_tokens / total_tokens_f64) * 100.0) as i32
        } else {
            0
        };

        // Build property details chips HTML

        let mut property_details_parts: Vec<String> = Vec::new();
        if let Some(beds) = bedrooms {
            let label = if beds == 1 { "Bed" } else { "Beds" };
            property_details_parts.push(format!(
                "<span class=\"cart-item-card__detail-chip\"><svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M2 4v16\"/><path d=\"M2 8h18a2 2 0 0 1 2 2v10\"/><path d=\"M2 17h20\"/><path d=\"M6 8v9\"/></svg> {} {}</span>",
                beds, label
            ));
        }
        if let Some(baths) = bathrooms {
            let label = if baths == 1 { "Bath" } else { "Baths" };
            property_details_parts.push(format!(
                "<span class=\"cart-item-card__detail-chip\"><svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M4 12h16a1 1 0 0 1 1 1v3a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4v-3a1 1 0 0 1 1-1z\"/><path d=\"M6 12V5a2 2 0 0 1 2-2h3v2.25\"/></svg> {} {}</span>",
                baths, label
            ));
        }
        if let Some(bsqm) = building_sqm {
            if bsqm > 0.0 {
                let display = if bsqm == bsqm.floor() {
                    format!("{:.0}", bsqm)
                } else {
                    format!("{:.1}", bsqm)
                };
                property_details_parts.push(format!(
                    "<span class=\"cart-item-card__detail-chip\"><svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"3\" y=\"3\" width=\"18\" height=\"18\" rx=\"2\"/><path d=\"M3 9h18\"/><path d=\"M9 3v18\"/></svg> {} m\u{00b2}</span>",
                    display
                ));
            }
        }
        if let Some(lsqm) = land_sqm {
            if lsqm > 0.0 {
                let display = if lsqm == lsqm.floor() {
                    format!("{:.0}", lsqm)
                } else {
                    format!("{:.1}", lsqm)
                };
                property_details_parts.push(format!(
                    "<span class=\"cart-item-card__detail-chip\"><svg width=\"14\" height=\"14\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M2 22 16 8\"/><path d=\"M3.47 12.53 5 11l1.53 1.53a3.5 3.5 0 0 1 0 4.94L5 19l-1.53-1.53a3.5 3.5 0 0 1 0-4.94Z\"/><path d=\"M7.47 8.53 9 7l1.53 1.53a3.5 3.5 0 0 1 0 4.94L9 15l-1.53-1.53a3.5 3.5 0 0 1 0-4.94Z\"/><path d=\"M11.47 4.53 13 3l1.53 1.53a3.5 3.5 0 0 1 0 4.94L13 11l-1.53-1.53a3.5 3.5 0 0 1 0-4.94Z\"/><path d=\"M20 2h2v2a4 4 0 0 1-4 4h-2V6a4 4 0 0 1 4-4Z\"/></svg> {} m\u{00b2} land</span>",
                    display
                ));
            }
        }

        let property_details_html = if property_details_parts.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="cart-item-card__property-details">{}</div>"#,
                property_details_parts.join("")
            )
        };

        // Each cart item card
        cart_items_html.push_str(&format!(
            r##"<div id="cart-item-{idx}" class="cart-item-card" data-cart-id="{cart_id}">
                <div class="cart-item-card__image-wrapper">
                    <a href="/property/{slug}">
                        <img class="cart-item-card__image" src="{image_url}" alt="{title}">
                    </a>
                </div>
                <div class="cart-item-card__body">
                    <div class="cart-item-card__header-row">
                        <div class="cart-item-card__title-group">
                            <a href="/property/{slug}" class="cart-item-card__title">{title}</a>
                        </div>
                        <form method="POST" action="/cart/remove" class="cart-item-card__remove-form">
                            <input type="hidden" name="cart_item_id" value="{cart_id}">
                            <button type="submit" class="cart-item-card__remove-btn" title="Remove from cart">
                                <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
                                    <path d="M2.5 5H17.5M6.66667 5V3.33333C6.66667 2.89131 6.84226 2.46738 7.15482 2.15482C7.46738 1.84226 7.89131 1.66667 8.33333 1.66667H11.6667C12.1087 1.66667 12.5326 1.84226 12.8452 2.15482C13.1577 2.46738 13.3333 2.89131 13.3333 3.33333V5M15.8333 5V16.6667C15.8333 17.1087 15.6577 17.5326 15.3452 17.8452C15.0326 18.1577 14.6087 18.3333 14.1667 18.3333H5.83333C5.39131 18.3333 4.96738 18.1577 4.65482 17.8452C4.34226 17.5326 4.16667 17.1087 4.16667 16.6667V5H15.8333Z" stroke="currentColor" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"/>
                                </svg>
                            </button>
                        </form>
                    </div>

                    <div class="cart-item-card__meta-row">
                        <span class="cart-item-card__location">
                            <img src="/static/images/Cart/marker-pin-01.svg" alt="" width="14" height="14">
                            {location}
                        </span>
                        {property_details}
                    </div>

                    <div class="cart-item-card__bottom-row">
                        <div class="cart-item-card__stats">
                            <div class="cart-item-card__stat">
                                <span class="stat-label">Share Price</span>
                                <span class="stat-value">{token_price}</span>
                            </div>
                            <div class="cart-item-card__stat">
                                <span class="stat-label">Shares</span>
                                <span class="stat-value" id="cart-item-{idx}-tokens-label">{tokens_qty}</span>
                            </div>
                            <div class="cart-item-card__stat">
                                <span class="stat-label">Yield</span>
                                <span class="stat-value stat-value--yield">{yield_display}</span>
                            </div>
                        </div>

                        <div class="cart-item-card__price-controls">
                            <span id="cart-item-{idx}-price" class="cart-item-card__price">{price}</span>
                            <div class="cart-item-card__quantity">
                                <button class="quantity-btn quantity-btn--minus"
                                        data-item-id="cart-item-{idx}"
                                        data-cart-id="{cart_id}"
                                        data-unit-price="{unit_price}"
                                        data-available="{available_tokens}"
                                        data-total="{total_tokens}"
                                        data-change="-1"
                                        onclick="handleQuantityChange(this)">
                                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                                        <path d="M4 8H12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                                    </svg>
                                </button>
                                <input type="number"
                                       class="quantity-input"
                                       id="cart-item-{idx}-qty"
                                       value="{tokens_qty}"
                                       data-item-id="cart-item-{idx}"
                                       data-cart-id="{cart_id}"
                                       data-unit-price="{unit_price}"
                                       data-available="{available_tokens}"
                                       data-total="{total_tokens}"
                                       onchange="handleQuantityInput(this)"
                                       onblur="handleQuantityInput(this)" />
                                <button class="quantity-btn quantity-btn--plus"
                                        data-item-id="cart-item-{idx}"
                                        data-cart-id="{cart_id}"
                                        data-unit-price="{unit_price}"
                                        data-available="{available_tokens}"
                                        data-total="{total_tokens}"
                                        data-change="1"
                                        onclick="handleQuantityChange(this)">
                                    <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
                                        <path d="M8 4V12M4 8H12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
                                    </svg>
                                </button>
                            </div>
                        </div>
                    </div>

                    <div class="cart-item-card__progress">
                        <div class="cart-item-card__progress-track">
                            <div id="cart-item-{idx}-progress" class="cart-item-card__progress-fill" style="width:{funded_pct}%;"></div>
                        </div>
                        <span id="cart-item-{idx}-funded-text" class="cart-item-card__progress-label">{funded_pct}% funded</span>
                    </div>
                </div>
            </div>"##,
            idx = idx,
            cart_id = ci_id,
            slug = slug.replace('&', "&amp;").replace('<', "&lt;"),
            title = title.replace('&', "&amp;").replace('<', "&lt;"),
            location = location.replace('&', "&amp;").replace('<', "&lt;"),
            token_price = token_price_display,
            tokens_qty = tokens_qty,
            yield_display = yield_display,
            price = price_display,
            unit_price = token_price_cents / 100,
            available_tokens = tokens_available,
            total_tokens = tokens_total,
            funded_pct = funded_pct,
            image_url = image_url.as_deref().unwrap_or("/static/images/Portfolio asset details/Property image.png"),
            property_details = property_details_html,
        ));

        // Build mobile cart item card
        // Build mobile property details
        let mobile_property_details = if property_details_parts.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div style="display:flex; flex-wrap:wrap; gap:6px; margin-top:8px;">{}</div>"#,
                property_details_parts.join("")
            )
        };

        mobile_items_html.push_str(&format!(
            r##"<div class="mobile-cart-item-card" data-cart-id="{cart_id}">
                <div class="mobile-cart-item-info">
                    <div class="mobile-cart-item-header">
                        <a href="/property/{slug}"><img src="{image_url}" class="mobile-cart-item-image" alt="{title}" style="object-fit:cover;"></a>
                        <div class="mobile-cart-item-details">
                            <a href="/property/{slug}" style="text-decoration:none;"><div class="mobile-cart-item-title">{title}</div></a>
                            {mobile_property_details}
                            <div class="mobile-cart-metrics">
                                <div class="mobile-cart-metric">
                                    <div class="mobile-cart-metric-label"><span>Share Price</span></div>
                                    <div class="mobile-cart-metric-value">{token_price}</div>
                                </div>
                                <div class="mobile-cart-metric">
                                    <div class="mobile-cart-metric-label"><span>Annual Yield</span></div>
                                    <div class="mobile-cart-metric-value">{yield_display}</div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                
                <div class="mobile-cart-controls-section">
                    <div class="mobile-cart-loading-bar" style="margin-top: 10px; width: 100%;">
                        <div class="progress-container" style="flex: 1; height: 6px; background: #EAECF0; border-radius: 3px; position: relative;">
                            <div style="width: {funded_pct}%; background: #0000FF; border-radius: 3px; position: absolute; left: 0; top: 0; height: 100%;"></div>
                        </div>
                        <span style="font-size: 12px; color: #535862;">{funded_pct}% funded</span>
                    </div>

                    <div style="display:flex; justify-content:space-between; width:100%; align-items:center;">
                        <div class="mobile-cart-quantity-controls" style="width:auto; height:36px; display:flex; gap:8px; align-items:center;">
                            <button class="mobile-cart-minus-btn" data-item-id="cart-item-mobile-{idx}" data-cart-id="{cart_id}" data-unit-price="{unit_price}" data-available="{available_tokens}" data-total="{total_tokens}" data-change="-1" onclick="handleQuantityChange(this)">
                                <div class="mobile-cart-minus-icon"></div>
                            </button>
                            <input type="number" 
                                   class="quantity-input" 
                                   id="cart-item-mobile-{idx}-qty" 
                                   value="{tokens_qty}" 
                                   data-item-id="cart-item-mobile-{idx}" 
                                   data-cart-id="{cart_id}" 
                                   data-unit-price="{unit_price}" 
                                   data-available="{available_tokens}" 
                                   data-total="{total_tokens}" 
                                   onchange="handleQuantityInput(this)" 
                                   onblur="handleQuantityInput(this)" 
                                   style="margin:0 4px; padding:0; width:48px; height:32px; border:1px solid #e9eaeb; border-radius:6px; text-align:center; font-weight:600; appearance:textfield; -moz-appearance:textfield;" />
                            <button class="mobile-cart-plus-btn" data-item-id="cart-item-mobile-{idx}" data-cart-id="{cart_id}" data-unit-price="{unit_price}" data-available="{available_tokens}" data-total="{total_tokens}" data-change="1" onclick="handleQuantityChange(this)">
                                <div class="mobile-cart-plus-icon"></div>
                            </button>
                        </div>
                        <div class="mobile-cart-price-display" style="width:auto; padding:4px 16px;">
                            <span class="mobile-cart-price-text" id="cart-item-mobile-{idx}-price">{price}</span>
                        </div>
                    </div>
                    
                    <div class="mobile-cart-actions">
                        <form method="POST" action="/cart/remove" style="width:100%;">
                            <input type="hidden" name="cart_item_id" value="{cart_id}">
                            <button type="submit" class="mobile-cart-remove-btn" style="width:100%; display:flex; gap:8px;">
                                <svg width="16" height="16" viewBox="0 0 20 20" fill="none"><path d="M2.5 5H17.5M6.66667 5V3.33333C6.66667 2.89131 6.84226 2.46738 7.15482 2.15482C7.46738 1.84226 7.89131 1.66667 8.33333 1.66667H11.6667C12.1087 1.66667 12.5326 1.84226 12.8452 2.15482C13.1577 2.46738 13.3333 2.89131 13.3333 3.33333V5M15.8333 5V16.6667C15.8333 17.1087 15.6577 17.5326 15.3452 17.8452C15.0326 18.1577 14.6087 18.3333 14.1667 18.3333H5.83333C5.39131 18.3333 4.96738 18.1577 4.65482 17.8452C4.34226 17.5326 4.16667 17.1087 4.16667 16.6667V5H15.8333Z" stroke="#F04438" stroke-width="1.67" stroke-linecap="round" stroke-linejoin="round"/></svg>
                                <span class="mobile-cart-remove-text" style="color: #A4A7AE;">Remove</span>
                            </button>
                        </form>
                    </div>
                </div>
            </div>"##,
            idx = idx,
            slug = slug.replace('&', "&amp;").replace('<', "&lt;"),
            cart_id = ci_id,
            title = title.replace('&', "&amp;").replace('<', "&lt;"),
            token_price = token_price_display,
            tokens_qty = tokens_qty,
            yield_display = yield_display,
            price = price_display,
            unit_price = token_price_cents / 100,
            available_tokens = tokens_available,
            total_tokens = tokens_total,
            funded_pct = funded_pct,
            image_url = image_url.as_deref().unwrap_or("/static/images/Portfolio asset details/Property image.png"),
            mobile_property_details = mobile_property_details,
        ));
    }

    let total_display = format_cart_usd(total_cents);

    let idr_display = format_idr(total_cents);

    // Compute dynamic rewards banner text
    // Pre-calculate KYC status on backend to avoid frontend flicker
    let mut is_kyc_approved = false;
    if let Ok(kyc) = crate::kyc::service::get_kyc_status(&state.db, user.id).await {
        is_kyc_approved = kyc.status == "approved";
    }
    let payment_display = if is_kyc_approved {
        ""
    } else {
        "display: none;"
    };
    let kyc_display = if is_kyc_approved {
        "display: none;"
    } else {
        ""
    };

    let item_count = rows.len();
    let subtotal_idr = format_idr(total_cents);

    // KFS Logic for Primary Offerings
    let mut kfs_checkbox_html = String::new();
    let mut kfs_modal_html = String::new();

    if !primary_items.is_empty() {
        let mut primary_list_html = String::new();
        for (p_title, _price, _qty) in primary_items {
            primary_list_html.push_str(&format!("<li><strong style=\"color:#101828;\">{}</strong><br/><span style=\"color:#475467; font-size:13px;\">Primary Offering — Subject to funding targets.</span></li>", p_title));
        }

        kfs_checkbox_html = r#"
            <div class="cart-terms" id="cart-kfs-row" style="margin-top: 8px;">
                <input type="checkbox" id="cart-kfs-checkbox" onchange="cartTermsChanged()" />
                <label for="cart-kfs-checkbox">
                    I have read and acknowledged the 
                    <a href="javascript:void(0)" onclick="openKFSModal(event)">Key Facts Statement (KFS)</a>
                </label>
            </div>
        "#.to_string();

        kfs_modal_html = format!(r#"
            <!-- KFS Modal -->
            <div id="kfs-modal" style="display:none; position:fixed; top:0; left:0; right:0; bottom:0; background:rgba(0,0,0,0.5); z-index:9999; align-items:center; justify-content:center; opacity:0; transition:opacity 0.3s ease;">
                <div style="background:#fff; border-radius:12px; width:90%; max-width:500px; padding:24px; box-shadow:0 12px 24px rgba(0,0,0,0.1); transform:translateY(20px); transition:transform 0.3s ease;" id="kfs-modal-content">
                    <h3 style="margin:0 0 16px 0; font-family:'TT Norms Pro', sans-serif; font-size:20px; color:#101828;">Key Facts Statement (KFS)</h3>
                    <p style="font-size:14px; color:#475467; line-height:1.5;">You are purchasing shares in <strong>Primary Offerings</strong>. Please acknowledge the following risks and conditions before proceeding:</p>
                    
                    <ul style="margin:16px 0; padding-left:20px; line-height:1.6; font-size:14px; display:flex; flex-direction:column; gap:8px;">
                        {list}
                    </ul>

                    <div style="background:#FEF0C7; border:1px solid #DC6803; border-radius:8px; padding:12px; margin-bottom:24px;">
                        <h4 style="margin:0 0 4px 0; color:#B54708; font-size:13px;">Escrow & Refund Policy</h4>
                        <p style="margin:0; font-size:12px; color:#B54708; line-height:1.4;">Primary offering funds will be held in a regulated escrow account. If the minimum funding target is not met by the campaign deadline, this purchase will be automatically aborted and your funds will be fully refunded to your POOOL wallet.</p>
                    </div>

                    <div style="display:flex; justify-content:flex-end; gap:12px;">
                        <button onclick="closeKFSModal()" style="padding:10px 16px; border:1px solid #D0D5DD; background:#fff; border-radius:8px; cursor:pointer; font-weight:600; font-family:'TT Norms Pro';">Cancel</button>
                        <button onclick="acceptKFSModal()" style="padding:10px 16px; border:none; background:var(--primary-color, #0000FF); color:#fff; border-radius:8px; cursor:pointer; font-weight:600; font-family:'TT Norms Pro';">Acknowledge & Accept</button>
                    </div>
                </div>
            </div>
            <script>
                function openKFSModal(e) {{
                    e.preventDefault();
                    const modal = document.getElementById('kfs-modal');
                    const content = document.getElementById('kfs-modal-content');
                    modal.style.display = 'flex';
                    // Trigger reflow
                    void modal.offsetWidth;
                    modal.style.opacity = '1';
                    content.style.transform = 'translateY(0)';
                }}
                function closeKFSModal() {{
                    const modal = document.getElementById('kfs-modal');
                    const content = document.getElementById('kfs-modal-content');
                    modal.style.opacity = '0';
                    content.style.transform = 'translateY(20px)';
                    setTimeout(() => {{ modal.style.display = 'none'; }}, 300);
                }}
                function acceptKFSModal() {{
                    const cb = document.getElementById('cart-kfs-checkbox');
                    if(cb) {{
                        cb.checked = true;
                        cartTermsChanged();
                    }}
                    closeKFSModal();
                }}
            </script>
        "#, list = primary_list_html);
    }

    let summary_html = format!(

        r##"<div id="cart-page-summary" class="cart-page-summary">
            <!-- Proceed to Payment Summary Box -->
            <div class="cart-summary-container" id="payment-summary-box" style="{payment_vis}">
                <!-- Header: title + timer -->
                <div class="cart-summary-top-row">
                    <h3 class="cart-summary-heading" style="margin:0;">Order Summary <span style="font-weight:400; font-size:14px; color:#717680;">({item_count} {item_label})</span></h3>
                    <div style="color:#B42318; font-size:13px; font-weight:600; display:flex; align-items:center; gap:6px; white-space:nowrap;">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline></svg>
                        Reserved for <span id="checkout-timer">10:00</span>
                    </div>
                </div>

                <!-- Per-item breakdown -->
                <div class="cart-summary-items">
                    {summary_items}
                </div>

                <!-- Subtotal / Fee / Tax -->
                <div class="cart-summary-line-items">
                    <div class="summary-line">
                        <span class="summary-line-label">Subtotal</span>
                        <div class="summary-line-values">
                            <span class="summary-line-value" id="cart-subtotal-amount">{total}</span>
                            <span class="summary-line-idr">≈ {subtotal_idr}</span>
                        </div>
                    </div>
                    <div class="summary-line">
                        <span class="summary-line-label">Platform Fee <img src="/static/images/help-circle.svg" class="help-icon-small" alt="Help" title="Standard platform transaction fee" /></span>
                        <div class="summary-line-values">
                            <span class="summary-line-value" id="cart-fee-amount">USD 0.00</span>
                            <span class="summary-line-idr">≈ Rp 0</span>
                        </div>
                    </div>
                    <div class="summary-line">
                        <span class="summary-line-label">Tax</span>
                        <div class="summary-line-values">
                            <span class="summary-line-value">USD 0.00</span>
                            <span class="summary-line-idr">≈ Rp 0</span>
                        </div>
                    </div>
                </div>

                <!-- Promo Code -->
                <div class="promo-code-container">
                    <div class="promo-input-group">
                        <input type="text" class="promo-input" placeholder="Promo / Referral code" id="promo-code-input" />
                        <button class="promo-apply-btn">Apply</button>
                    </div>
                </div>

                <!-- Total -->
                <div class="cart-summary-divider"></div>
                <div class="cart-summary-header">
                    <div class="cart-summary-title">Total</div>
                    <div style="display:flex; flex-direction:column; align-items:flex-end; gap:4px;">
                        <div class="cart-summary-amount" id="cart-summary-amount">{total}</div>
                        <div style="font-size:14px; font-weight:500; color:#475467;" id="cart-total-idr">≈ {total_idr}</div>
                    </div>
                </div>

                <!-- Terms & Conditions -->
                <div class="cart-terms" id="cart-terms-row">
                    <input type="checkbox" id="cart-terms-checkbox" onchange="cartTermsChanged()" />
                    <label for="cart-terms-checkbox">
                        I agree to the
                        <a href="/terms" target="_blank">Terms and Conditions</a> and
                        <a href="/privacy-policy" target="_blank">Privacy Policy</a>
                    </label>
                </div>
                {kfs_checkbox}

                <!-- CTA Button -->

                <div class="cart-summary-actions">
                    <a href="/checkout" id="cart-proceed-btn" class="proceed-payment-btn proceed-payment-btn--disabled" style="background:var(--primary-color, #0000FF);" onclick="return cartProceed(event)">
                        <span class="button-text" style="color:#98FB96;">Proceed to Checkout</span>
                        <svg class="btn-arrow" width="20" height="20" viewBox="0 0 20 20" fill="none"><path d="M4.16667 10H15.8333M15.8333 10L10 4.16667M15.8333 10L10 15.8333" stroke="#98FB96" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
                    </a>
                </div>
                <script>
                  (function() {{
                    // Restore from sessionStorage if user navigates back
                    if (sessionStorage.getItem('terms_accepted') === '1') {{
                      var cb = document.getElementById('cart-terms-checkbox');
                      if (cb) {{ cb.checked = true; cartTermsChanged(); }}
                    }}
                  }})();
                  function cartTermsChanged() {{
                    var checked = document.getElementById('cart-terms-checkbox').checked;
                    var kfsEl = document.getElementById('cart-kfs-checkbox');
                    var kfsChecked = kfsEl ? kfsEl.checked : true;
                    var btn = document.getElementById('cart-proceed-btn');

                    if (checked && kfsChecked) {{
                      btn.classList.remove('proceed-payment-btn--disabled');
                      sessionStorage.setItem('terms_accepted', '1');
                    }} else {{
                      btn.classList.add('proceed-payment-btn--disabled');
                      sessionStorage.removeItem('terms_accepted');
                    }}
                  }}
                  function cartProceed(e) {{
                    var checked = document.getElementById('cart-terms-checkbox').checked;
                    var kfsEl = document.getElementById('cart-kfs-checkbox');
                    var kfsChecked = kfsEl ? kfsEl.checked : true;

                    if (!checked || !kfsChecked) {{
                      e.preventDefault();
                      var termsRow = document.getElementById('cart-terms-row');
                      var kfsRow = document.getElementById('cart-kfs-row');
                      var rowToShake = (!kfsChecked && checked && kfsRow) ? kfsRow : termsRow;

                      if (rowToShake) {{
                        rowToShake.style.outline = '2px solid #F04438';
                        rowToShake.style.borderRadius = '8px';
                        rowToShake.style.padding = '8px';
                        rowToShake.style.background = '#FEF3F2';
                        rowToShake.style.animation = 'none';
                        rowToShake.offsetHeight; // force reflow
                        rowToShake.style.animation = 'cart-shake 0.4s ease';
                        setTimeout(function() {{
                          rowToShake.style.outline = '';
                          rowToShake.style.borderRadius = '';
                          rowToShake.style.padding = '';
                          rowToShake.style.background = '';
                          rowToShake.style.animation = '';
                        }}, 2000);
                      }}

                      return false;
                    }}
                    return true;
                  }}
                </script>
                <style>
                  @keyframes cart-shake {{
                    0%, 100% {{ transform: translateX(0); }}
                    20% {{ transform: translateX(-4px); }}
                    40% {{ transform: translateX(4px); }}
                    60% {{ transform: translateX(-3px); }}
                    80% {{ transform: translateX(3px); }}
                  }}
                </style>

                <!-- Payment Method Icons -->
                <div class="cart-payment-icons">
                    <span class="cart-payment-label">Accepted payments</span>
                    <div class="cart-payment-icon-row">
                        <!-- Visa -->
                        <svg width="38" height="24" viewBox="0 0 38 24" fill="none"><rect x="0.5" y="0.5" width="37" height="23" rx="3.5" fill="white" stroke="#E9EAEB"/><path d="M15.5 16.5H13.2L14.7 7.5H17L15.5 16.5Z" fill="#3C58BF"/><path d="M24.1 7.7C23.6 7.5 22.8 7.3 21.8 7.3C19.5 7.3 17.9 8.5 17.9 10.1C17.9 11.3 19 11.9 19.8 12.3C20.7 12.7 21 12.9 21 13.3C21 13.8 20.4 14.1 19.8 14.1C19 14.1 18.5 14 17.8 13.7L17.5 13.5L17.2 15.4C17.8 15.6 18.8 15.8 19.8 15.8C22.3 15.8 23.8 14.6 23.8 12.9C23.8 12 23.2 11.3 22 10.7C21.3 10.3 20.9 10.1 20.9 9.7C20.9 9.3 21.3 8.9 22.2 8.9C22.9 8.9 23.5 9 23.9 9.2L24.1 9.3L24.4 7.5L24.1 7.7Z" fill="#3C58BF"/><path d="M27.3 7.5H25.5C24.9 7.5 24.5 7.7 24.3 8.3L20.8 16.5H23.3L23.8 15.1H26.8L27.1 16.5H29.3L27.3 7.5ZM24.5 13.3C24.7 12.7 25.6 10.4 25.6 10.4L26.3 13.3H24.5Z" fill="#3C58BF"/><path d="M12.1 7.5L9.8 13.5L9.6 12.5C9.1 11.1 7.8 9.5 6.3 8.7L8.4 16.5H10.9L14.6 7.5H12.1Z" fill="#3C58BF"/><path d="M8.3 7.5H4.6L4.5 7.7C7.5 8.5 9.5 10.3 10.1 12.5L9.4 8.3C9.3 7.7 8.9 7.5 8.3 7.5Z" fill="#FDB731"/></svg>
                        <!-- Mastercard -->
                        <svg width="38" height="24" viewBox="0 0 38 24" fill="none"><rect x="0.5" y="0.5" width="37" height="23" rx="3.5" fill="white" stroke="#E9EAEB"/><circle cx="15" cy="12" r="6" fill="#EB001B"/><circle cx="23" cy="12" r="6" fill="#F79E1B"/><path d="M19 7.5C20.5 8.7 21.5 10.2 21.5 12C21.5 13.8 20.5 15.3 19 16.5C17.5 15.3 16.5 13.8 16.5 12C16.5 10.2 17.5 8.7 19 7.5Z" fill="#FF5F00"/></svg>
                        <!-- Bank Transfer -->
                        <svg width="38" height="24" viewBox="0 0 38 24" fill="none"><rect x="0.5" y="0.5" width="37" height="23" rx="3.5" fill="white" stroke="#E9EAEB"/><path d="M19 5L10 10H28L19 5Z" fill="#98A2B3"/><rect x="12" y="11" width="2" height="6" rx="0.5" fill="#98A2B3"/><rect x="16" y="11" width="2" height="6" rx="0.5" fill="#98A2B3"/><rect x="20" y="11" width="2" height="6" rx="0.5" fill="#98A2B3"/><rect x="24" y="11" width="2" height="6" rx="0.5" fill="#98A2B3"/><rect x="10" y="18" width="18" height="2" rx="0.5" fill="#98A2B3"/></svg>
                    </div>
                </div>

                <!-- Trust Badges -->
                <div class="cart-trust-section">
                    <div class="cart-trust-badge">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#717680" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg>
                        <span>256-bit SSL Encrypted</span>
                    </div>
                    <div class="cart-trust-badge">
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#717680" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path></svg>
                        <span>Regulated &amp; Compliant</span>
                    </div>
                </div>
            </div>


            <!-- KYC Summary Box (hidden by default, shown by kyc-banner.js) -->
            <div class="cart-summary-container" id="kyc-summary-box" style="{kyc_vis}">
                <div style="display:flex; flex-direction:column; align-items:center; text-align:center; padding:16px 0; gap:16px;">
                    <div style="width:48px; height:48px; display:flex; align-items:center; justify-content:center; background:#FEF0C7; border-radius:50%;">
                        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#DC6803" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>
                    </div>
                    <div>
                        <h3 style="margin:0; font-family:'TT Norms Pro', sans-serif; font-weight:600; font-size:18px; color:#101828; margin-bottom:8px;">Identity Verification Required</h3>
                        <p style="margin:0; font-family:'TT Norms Pro', sans-serif; font-size:14px; color:#475467;">Please complete your identity verification to proceed to checkout and secure your investment.</p>
                    </div>
                    <a href="/kyc" style="display:flex; align-items:center; justify-content:center; width:100%; padding:12px; background:#181D27; color:#FFFFFF; text-decoration:none; border-radius:8px; font-weight:600; font-size:16px;">Verify Identity</a>
                </div>
            </div>
        </div>"##,
        payment_vis = payment_display,
        kyc_vis = kyc_display,
        item_count = item_count,
        item_label = if item_count == 1 { "item" } else { "items" },
        summary_items = summary_items_html,
        total = total_display,
        subtotal_idr = subtotal_idr,
        total_idr = idr_display,
        kfs_checkbox = kfs_checkbox_html,
    );


    // Build the items container (left column) with "Add More" ghost card at the bottom
    let populated_content = format!(
        r##"<!-- Cart Content --><div id="cart-page-content" class="cart-page-content"><div id="cart-page-items-container" class="cart-items-container"><div class="cart-items-list">{items_html}<a href="/marketplace" class="cart-add-more-card">
            <div class="cart-add-more-card__image-placeholder">
                <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="m21 15-5-5L5 21"/></svg>
            </div>
            <div class="cart-add-more-card__body">
                <div class="cart-add-more-card__header-row">
                    <div class="cart-add-more-card__title-group">
                        <span class="cart-add-more-card__title">Add another property</span>
                        <span class="cart-add-more-card__badge-placeholder"></span>
                    </div>
                    <div class="cart-add-more-card__plus-icon">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="none"><path d="M12 5V19M5 12H19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg>
                    </div>
                </div>
                <div class="cart-add-more-card__meta-row">
                    <span class="cart-add-more-card__line-placeholder" style="width:100px;"></span>
                    <span class="cart-add-more-card__line-placeholder" style="width:60px;"></span>
                    <span class="cart-add-more-card__line-placeholder" style="width:70px;"></span>
                </div>
                <div class="cart-add-more-card__bottom-row">
                    <div class="cart-add-more-card__stats-placeholder">
                        <div class="cart-add-more-card__stat-placeholder"><span class="cart-add-more-card__line-placeholder" style="width:55px; height:8px;"></span><span class="cart-add-more-card__line-placeholder" style="width:40px; height:12px;"></span></div>
                        <div class="cart-add-more-card__stat-placeholder"><span class="cart-add-more-card__line-placeholder" style="width:35px; height:8px;"></span><span class="cart-add-more-card__line-placeholder" style="width:40px; height:12px;"></span></div>
                        <div class="cart-add-more-card__stat-placeholder"><span class="cart-add-more-card__line-placeholder" style="width:25px; height:8px;"></span><span class="cart-add-more-card__line-placeholder" style="width:40px; height:12px;"></span></div>
                    </div>
                    <span class="cart-add-more-card__subtitle">Browse marketplace to diversify</span>
                </div>
                <div class="cart-add-more-card__progress-placeholder"><div class="cart-add-more-card__line-placeholder" style="width:100%; height:6px; border-radius:3px;"></div></div>
            </div>
        </a>
        <!-- Support Card -->
        <div class="cart-support-card">
            <div class="cart-support-card__icon">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg>
            </div>
            <div class="cart-support-card__text">
                <span class="cart-support-card__title">Need help?</span>
                <span class="cart-support-card__desc">Our team is here. <a href="/support" class="cart-support-card__link">Contact support</a> or call <strong>+62 361 123 456</strong></span>
            </div>
        </div>
        </div></div>{summary}{kfs_modal}
        <!-- Empty Cart State -->"##,
        items_html = cart_items_html,
        summary = summary_html,
        kfs_modal = kfs_modal_html,
    );

    // Replace the entire cart content section between the two known anchors
    let mut modified_html = html.clone();

    // Inject mobile template right before Mobile Empty Cart State
    let mobile_cart_full_html = format!(
        r#"
        <style>.mobile-cart-empty-container {{ display: none !important; }}</style>
        {}
        <div class="mobile-cart-checkout-section">
            <div class="mobile-cart-checkout-content">
                <div class="mobile-cart-checkout-header">
                    <span class="mobile-cart-total-text">Total</span>
                    <span class="mobile-cart-total-text">{}</span>
                </div>
                <a href="/checkout" class="mobile-cart-checkout-btn" style="text-decoration:none;">
                    <span class="mobile-cart-checkout-text">Proceed to Payment</span>
                    <svg class="mobile-cart-arrow-icon" viewBox="0 0 20 20" fill="none"><path d="M4.16667 10H15.8333M15.8333 10L10 4.16667M15.8333 10L10 15.8333" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
                </a>
            </div>
        </div>
        <!-- Mobile Empty Cart State -->
    "#,
        mobile_items_html, total_display
    );
    // Slicing out desktop empty state removes the original Cart Content
    // Slicing out desktop empty state removes the original Cart Content
    let start_marker = "<!-- Cart Content -->";
    let end_marker = "<!-- Empty Cart State -->";

    if let (Some(start), Some(end)) = (
        modified_html.find(start_marker),
        modified_html.find(end_marker),
    ) {
        modified_html = format!(
            "{}{}{}",
            &modified_html[..start],
            populated_content,
            &modified_html[end + end_marker.len()..]
        );
    }

    // Now inject mobile after slicing happens
    modified_html =
        modified_html.replace("<!-- Mobile Empty Cart State -->", &mobile_cart_full_html);

    Html(modified_html).into_response()
}
