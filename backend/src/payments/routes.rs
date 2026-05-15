use axum::extract::FromRequest;
/// Payment route handlers – deposits, checkout, webhooks, and invoices.
///
/// Thin HTTP layer that delegates all business logic to `service.rs`.
use axum::{
    extract::{Form, Path, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::CookieJar;

use crate::auth::middleware;
use crate::auth::routes::AppState;

use super::models::*;
use super::service;

const DEFAULT_TEST_BANK_DETAILS_EMAILS: &str = "support@traffic-creator.com";
const TEST_BANK_DETAILS_EMAILS_ENV: &str = "POOOL_TEST_BANK_DETAILS_EMAILS";

fn app_env_allows_local_upload_placeholder(app_env: &str) -> bool {
    matches!(
        app_env.trim().to_ascii_lowercase().as_str(),
        "development" | "dev" | "local" | "test"
    )
}

fn email_matches_csv(email: &str, configured_emails: &str) -> bool {
    configured_emails
        .split(',')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .any(|candidate| candidate.eq_ignore_ascii_case(email))
}

fn should_use_test_bank_details_with_config(email: &str, configured_emails: Option<&str>) -> bool {
    let configured_emails = configured_emails
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_TEST_BANK_DETAILS_EMAILS);

    email_matches_csv(email, configured_emails)
}

fn should_use_test_bank_details(email: &str) -> bool {
    should_use_test_bank_details_with_config(
        email,
        std::env::var(TEST_BANK_DETAILS_EMAILS_ENV).ok().as_deref(),
    )
}

fn bank_details_for_user(email: &str) -> (serde_json::Value, serde_json::Value) {
    let (usd_raw, idr_raw) = if should_use_test_bank_details(email) {
        (
            service::TEST_BANK_DETAILS_USD,
            service::TEST_BANK_DETAILS_IDR,
        )
    } else {
        (service::BANK_DETAILS_USD, service::BANK_DETAILS_IDR)
    };

    let usd = serde_json::from_str(usd_raw).unwrap_or_default();
    let idr = serde_json::from_str(idr_raw).unwrap_or_default();
    (usd, idr)
}

fn local_test_proof_url(user_id: uuid::Uuid, file_name: &str) -> String {
    let sanitized_name: String = file_name
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect();
    let proof_name = if sanitized_name.is_empty() {
        "proof.bin"
    } else {
        sanitized_name.as_str()
    };

    format!("local-test-proof://{user_id}/{proof_name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_details_email_matching_is_case_insensitive_and_trimmed() {
        assert!(email_matches_csv(
            "support@traffic-creator.com",
            "qa@example.com, SUPPORT@TRAFFIC-CREATOR.COM "
        ));
        assert!(!email_matches_csv(
            "investor@example.com",
            "qa@example.com, support@traffic-creator.com"
        ));
    }

    #[test]
    fn test_bank_details_default_e2e_account_uses_sandbox_details() {
        assert!(should_use_test_bank_details_with_config(
            "support@traffic-creator.com",
            None
        ));
        assert!(!should_use_test_bank_details_with_config(
            "investor@example.com",
            None
        ));
    }

    #[test]
    fn test_bank_details_env_allowlist_overrides_default_fixture() {
        assert!(should_use_test_bank_details_with_config(
            "qa@example.com",
            Some("qa@example.com")
        ));
        assert!(!should_use_test_bank_details_with_config(
            "support@traffic-creator.com",
            Some("qa@example.com")
        ));
    }

    #[test]
    fn test_local_upload_placeholder_is_restricted_to_local_envs() {
        assert!(app_env_allows_local_upload_placeholder("development"));
        assert!(app_env_allows_local_upload_placeholder("test"));
        assert!(app_env_allows_local_upload_placeholder(" LOCAL "));
        assert!(!app_env_allows_local_upload_placeholder("production"));
        assert!(!app_env_allows_local_upload_placeholder("staging"));
    }

    #[test]
    fn test_local_test_proof_url_sanitizes_file_names() {
        let user_id = uuid::Uuid::nil();
        assert_eq!(
            local_test_proof_url(user_id, "../fake proof.pdf"),
            "local-test-proof://00000000-0000-0000-0000-000000000000/..fakeproof.pdf"
        );
    }
}

// ─── Deposit Handlers ───────────────────────────────────────────

/// POST /api/payments/deposit – Initiate a bank deposit (USD or IDR).
///
/// Creates a deposit_request and returns instructions (VA number, wire details).
pub async fn initiate_deposit(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<InitiateDepositForm>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // Validate currency
    let currency = form.currency.to_uppercase();
    if currency != "USD" && currency != "IDR" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Invalid currency. Supported: USD, IDR</div>"#.to_string()),
        ).into_response();
    }

    // Parse amount — previous impl stripped both "," and ".", which silently
    // mis-interpreted "50.25" as 5025 then *100 = $5,025 (100× overcharge).
    // USD accepts dollars with up to 2 decimal places. IDR accepts whole rupiah
    // only (no sub-unit). Thousands separators (",") are tolerated.
    let amount_cents: i64 = {
        use rust_decimal::prelude::*;
        let raw = form.amount.trim().replace(',', "");
        let parsed = Decimal::from_str(&raw)
            .ok()
            .filter(|d| d.is_sign_positive());
        let cents = match (currency.as_str(), parsed) {
            ("USD", Some(d)) => {
                // Reject >2 decimal places (fractions of a cent).
                if d.scale() > 2 {
                    None
                } else {
                    (d * Decimal::from(100)).to_i64()
                }
            }
            ("IDR", Some(d)) => {
                // IDR is quoted in whole rupiah; disallow fractional.
                if d.scale() > 0 {
                    None
                } else {
                    d.to_i64()
                }
            }
            _ => None,
        };
        match cents {
            Some(c) if c > 0 => c,
            _ => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Invalid amount. Please enter a positive number.</div>"#.to_string()),
                ).into_response();
            }
        }
    };

    // Reject unreasonably large deposits (max $1,000,000 USD or equivalent)
    const MAX_DEPOSIT_USD_CENTS: i64 = 100_000_000;
    const MAX_DEPOSIT_IDR: i64 = 1_550_000_000_000; // ~$100M at 15,500 rate
    let max_allowed = if currency == "USD" {
        MAX_DEPOSIT_USD_CENTS
    } else {
        MAX_DEPOSIT_IDR
    };
    if amount_cents > max_allowed {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Amount exceeds maximum allowed deposit.</div>"#.to_string()),
        ).into_response();
    }

    match service::create_deposit_request(&state.db, user.id, &currency, amount_cents).await {
        Ok(response) => {
            // Return success HTML for HTMX swap
            let ref_id = response.provider_reference.unwrap_or_default();
            let amount_display = if currency == "USD" {
                format!("${}.{:02}", amount_cents / 100, (amount_cents % 100).abs())
            } else {
                format!("Rp {}", amount_cents)
            };
            let instructions = response.instructions;
            let status_html = format!(
                "<div class=\"deposit-success\" style=\"background:#ECFDF3;border:1px solid #ABEFC6;border-radius:12px;padding:20px;margin-top:16px;\">\
                    <div style=\"display:flex;align-items:center;gap:8px;margin-bottom:12px;\">\
                        <span style=\"font-weight:600;color:#067647;font-size:16px;\">✅ Deposit Request Created</span>\
                    </div>\
                    <div style=\"font-size:14px;color:#344054;line-height:1.6;\">\
                        <p><strong>Reference:</strong> {}</p>\
                        <p><strong>Amount:</strong> {}</p>\
                        <p style=\"margin-top:8px;\">{}</p>\
                    </div>\
                    <p style=\"font-size:12px;color:#667085;margin-top:12px;\">Your balance will be updated automatically once the payment is confirmed.</p>\
                </div>",
                ref_id, amount_display, instructions
            );
            Html(status_html).into_response()
        }
        Err(e) => {
            tracing::error!("Deposit request failed: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Failed to create deposit request. Please try again.</div>"#.to_string()),
            ).into_response()
        }
    }
}

/// POST /api/webhooks/payments – Generic webhook handler for payment providers.
///
/// Verifies HMAC-SHA256(secret, "{timestamp}.{raw_body}") against X-Signature
/// header with constant-time compare, rejects >5min old timestamps to thwart
/// replay, then atomically credits the user's wallet. Idempotent.
pub async fn payment_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let secret = match std::env::var("PAYMENT_WEBHOOK_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::error!("PAYMENT_WEBHOOK_SECRET is not set — rejecting all webhooks");
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Webhook processing unavailable"})),
            )
                .into_response();
        }
    };

    let sig_header = match headers.get("x-signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s.trim().to_string(),
        None => {
            tracing::warn!("Webhook rejected: missing X-Signature header");
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing signature"})),
            )
                .into_response();
        }
    };
    let ts_header = match headers.get("x-timestamp").and_then(|v| v.to_str().ok()) {
        Some(s) => s.trim().to_string(),
        None => {
            tracing::warn!("Webhook rejected: missing X-Timestamp header");
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Missing timestamp"})),
            )
                .into_response();
        }
    };

    let ts: i64 = match ts_header.parse() {
        Ok(v) => v,
        Err(_) => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid timestamp"})),
            )
                .into_response();
        }
    };
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        tracing::warn!("Webhook rejected: timestamp outside 5min window");
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Stale timestamp"})),
        )
            .into_response();
    }

    let provided = match hex::decode(&sig_header) {
        Ok(b) => b,
        Err(_) => {
            tracing::warn!("Webhook rejected: signature not hex");
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature encoding"})),
            )
                .into_response();
        }
    };
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "HMAC init failed"})),
            )
                .into_response();
        }
    };
    mac.update(ts_header.as_bytes());
    mac.update(b".");
    mac.update(&body);
    if mac.verify_slice(&provided).is_err() {
        tracing::warn!("Webhook rejected: HMAC mismatch");
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid signature"})),
        )
            .into_response();
    }

    // Replay defence: a valid signature is reusable for ±300s. Without a
    // nonce store, an attacker who captures one webhook can flood the
    // endpoint with replays. confirm_deposit is idempotent so balances
    // are safe, but each replay still opens a Postgres tx → easy DoS.
    // SET NX with TTL > the freshness window rejects replays cheaply.
    // Fail-open if Redis is unavailable — the HMAC + 5min freshness +
    // confirm_deposit's idempotency still protect correctness.
    if let Some(redis) = &state.redis {
        if let Ok(mut conn) = redis.get().await {
            let key = format!("webhook:nonce:{}", sig_header);
            let acquired: Option<String> = deadpool_redis::redis::cmd("SET")
                .arg(&key)
                .arg("1")
                .arg("NX")
                .arg("EX")
                .arg(360i64) // a hair over the 300s freshness window
                .query_async(&mut *conn)
                .await
                .unwrap_or(Some("ok".to_string())); // on RPC error, fail-open
            if acquired.is_none() {
                tracing::warn!("Webhook rejected: replay detected (nonce already consumed)");
                return (
                    axum::http::StatusCode::OK,
                    Json(serde_json::json!({"ok": true, "deduped": true})),
                )
                    .into_response();
            }
        }
    }

    let payload: WebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Webhook body parse failed: {}", e);
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid payload"})),
            )
                .into_response();
        }
    };

    if payload.status != "paid" {
        tracing::info!(
            ref_id = %payload.provider_reference,
            status = %payload.status,
            "Webhook received with non-paid status, skipping"
        );
        return (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({"ok": true})),
        )
            .into_response();
    }

    match service::confirm_deposit(&state.db, &payload.provider_reference).await {
        Ok(deposit_id) => {
            tracing::info!(deposit_id = %deposit_id, "Webhook: deposit confirmed");
            (
                axum::http::StatusCode::OK,
                Json(serde_json::json!({"ok": true, "deposit_id": deposit_id.to_string()})),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Webhook deposit confirmation failed: {}", e);
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    }
}

// ─── API Handlers ──────────────────────────────────────────────

/// GET /api/payments/bank-details – Return bank transfer details for instructions.
pub async fn get_bank_details(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
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

    let (usd, idr) = bank_details_for_user(&user.email);

    Json(serde_json::json!({
        "USD": usd,
        "IDR": idr
    }))
    .into_response()
}

// ─── Checkout Handler ───────────────────────────────────────────

/// GET /checkout – Serve the checkout page.
///
/// The new checkout page fetches cart data and wallet balance via
/// `/api/cart` and `/api/wallets` on the client side.
pub async fn checkout_page(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // Quick check: redirect to cart if empty
    let has_items: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cart_items WHERE user_id = $1)")
            .bind(user.id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(false);

    if !has_items {
        return Redirect::to("/cart").into_response();
    }

    // KYC gate — must be approved to proceed to checkout
    let kyc_approved = match crate::kyc::service::get_kyc_status(&state.db, user.id).await {
        Ok(kyc) => kyc.status == "approved",
        Err(_) => false,
    };
    if !kyc_approved {
        tracing::warn!(user_id = %user.id, "Blocked checkout page: KYC not approved");
        return Redirect::to("/kyc?reason=required").into_response();
    }

    tracing::info!("Checkout page hit: user={}", user.id);

    // Fetch Cart
    #[derive(sqlx::FromRow)]
    struct CartItemRow {
        id: uuid::Uuid,
        asset_id: uuid::Uuid,
        tokens_quantity: i32,
        token_price_cents: i64,
        title: String,
        slug: String,
        location_city: Option<String>,
        location_country: Option<String>,
        short_description: Option<String>,
        asset_type: String,
        annual_yield_bps: Option<i32>,
        funding_status: String,
        tokens_available: i32,
        tokens_total: i32,
        cover_image_url: Option<String>,
        bedrooms: Option<i32>,
        bathrooms: Option<i32>,
        building_size_sqm: Option<f64>,
        land_size_sqm: Option<f64>,
    }

    let cart_rows = sqlx::query_as::<_, CartItemRow>(
        r#"
        SELECT
            ci.id, ci.asset_id, ci.tokens_quantity, ci.token_price_cents,
            a.title, a.slug,
            a.location_city, a.location_country, a.short_description,
            a.asset_type, a.annual_yield_bps, a.funding_status, a.tokens_available, a.tokens_total,
            (SELECT image_url FROM asset_images ai WHERE ai.asset_id = a.id ORDER BY ai.is_cover DESC, ai.sort_order ASC, ai.created_at ASC LIMIT 1) as cover_image_url,
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
    .await
    .unwrap_or_default();

    let mut cart_total_cents = 0;
    let cart_items: Vec<serde_json::Value> = cart_rows
        .into_iter()
        .map(|r| {
            let total = r.tokens_quantity as i64 * r.token_price_cents;
            cart_total_cents += total;
            serde_json::json!({
                "id": r.id.to_string(), "asset_id": r.asset_id.to_string(), "tokens_quantity": r.tokens_quantity,
                "token_price_cents": r.token_price_cents, "total_cents": total, "title": r.title, "slug": r.slug,
                "location_city": r.location_city, "location_country": r.location_country, "short_description": r.short_description,
                "asset_type": r.asset_type, "annual_yield_bps": r.annual_yield_bps, "funding_status": r.funding_status,
                "tokens_available": r.tokens_available, "tokens_total": r.tokens_total, 
                "cover_image_url": r.cover_image_url.as_ref().map(|u| crate::storage::service::rewrite_gcs_url(u)),
                "bedrooms": r.bedrooms, "bathrooms": r.bathrooms, "building_size_sqm": r.building_size_sqm, "land_size_sqm": r.land_size_sqm
            })
        })
        .collect();

    let platform_fee_pct: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .and_then(|v: String| v.parse().ok())
    .unwrap_or(rust_decimal::Decimal::ZERO);

    let fee_cents =
        service::calculate_platform_fee_cents(cart_total_cents, platform_fee_pct).unwrap_or(0);
    let grand_total_cents = cart_total_cents + fee_cents;
    let fee_pct_display = platform_fee_pct.normalize().to_string();

    let cart_json = serde_json::json!({
        "items": cart_items,
        "count": cart_items.len(),
        "total_cents": cart_total_cents,
        "fee_cents": fee_cents,
        "fee_pct": fee_pct_display,
        "grand_total_cents": grand_total_cents,
        "usd_to_idr_rate": crate::config::DEFAULT_USD_TO_IDR_RATE_I64
    })
    .to_string();

    // Fetch Wallets
    let wallet_rows = sqlx::query_as::<_, (uuid::Uuid, String, String, i64)>(
        r#"
        SELECT id, wallet_type, currency, balance_cents
        FROM wallets
        WHERE user_id = $1
        ORDER BY wallet_type, currency
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let wallets: Vec<serde_json::Value> = wallet_rows
        .into_iter()
        .map(|(id, wtype, currency, balance)| {
            serde_json::json!({
                "id": id.to_string(), "wallet_type": wtype, "currency": currency,
                "balance_cents": balance,
                "balance_display": if currency == "IDR" {
                    format!("Rp {}", balance) // Simplified display for context
                } else {
                    format!("${}.{:02}", balance / 100, (balance % 100).abs())
                }
            })
        })
        .collect();
    let wallet_json = serde_json::json!({ "wallets": wallets }).to_string();

    // Fetch Bank Details
    let (usd, idr) = bank_details_for_user(&user.email);
    let bank_json = serde_json::json!({ "USD": usd, "IDR": idr }).to_string();

    // Check if user is a referred investor for affiliate disclosure display
    let is_referral_user: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM affiliate_referrals WHERE referred_user_id = $1 LIMIT 1)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let template = match state.templates.get_template("checkout.html") {
        Ok(t) => t,
        Err(_) => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Html("<h1>Checkout page not found</h1>".to_string()),
            )
                .into_response();
        }
    };

    let html = match template.render(minijinja::context! {
        cart_json => cart_json,
        wallet_json => wallet_json,
        bank_json => bank_json,
        is_referral_user => is_referral_user,
    }) {
        Ok(content) => content,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Internal Server Error</h1>".to_string()),
            )
                .into_response();
        }
    };
    Html(html).into_response()
}

/// POST /checkout – Execute the checkout (purchase all cart items).
///
/// Atomically: validates, deducts wallet, updates assets, creates
/// order + investments, generates invoice, clears cart.
///
/// Accepts both:
///  - `multipart/form-data` (browser checkout with file upload for bank transfers)
///  - `application/x-www-form-urlencoded` (wallet-only checkout, no file)
pub async fn handle_checkout(
    _jar: CookieJar,
    State(state): State<AppState>,
    request: axum::http::Request<axum::body::Body>,
) -> axum::response::Response {
    let (parts, body) = request.into_parts();

    let client_ip = parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("0.0.0.0").trim().to_string())
        .unwrap_or_else(|| "0.0.0.0".to_string());

    // Authenticate from the cookie jar extracted from headers
    let cookie_jar = axum_extra::extract::CookieJar::from_headers(&parts.headers);
    let user = match middleware::get_current_user(&cookie_jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // KYC gate — hard block, prevents checkout even if cart was somehow populated
    let kyc_approved = match crate::kyc::service::get_kyc_status(&state.db, user.id).await {
        Ok(kyc) => kyc.status == "approved",
        Err(_) => false,
    };
    if !kyc_approved {
        tracing::warn!(user_id = %user.id, "Blocked checkout submission: KYC not approved");
        return (
            axum::http::StatusCode::FORBIDDEN,
            Html(r#"<div style="background:#FEF3F2;border:1px solid #FEE4E2;border-radius:12px;padding:16px;margin-top:16px;font-size:14px;color:#B42318;"><strong>Identity verification required.</strong> Please complete your KYC verification before making a purchase. <a href="/kyc" style="color:#B42318;font-weight:600;">Verify now →</a></div>"#.to_string()),
        ).into_response();
    }

    let content_type = parts
        .headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // --- Masterplan Priority 1: Checkout Idempotency ---
    let idempotency_key = parts
        .headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let _ip_address = parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0.0.0.0")
        .split(',')
        .next()
        .unwrap_or("0.0.0.0")
        .trim()
        .to_string();

    if let Some(key) = &idempotency_key {
        let insert_res = sqlx::query(
            "INSERT INTO idempotency_keys (key, user_id, request_path, request_method) VALUES ($1, $2, '/checkout', 'POST') ON CONFLICT (key) DO NOTHING"
        )
        .bind(&key)
        .bind(user.id)
        .execute(&state.db)
        .await;

        match insert_res {
            Ok(res) if res.rows_affected() == 0 => {
                let existing = sqlx::query_as::<_, (Option<i32>, Option<serde_json::Value>)>(
                    "SELECT response_status, response_body FROM idempotency_keys WHERE key = $1 AND user_id = $2"
                )
                .bind(&key)
                .bind(user.id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

                if let Some((response_status, response_body)) = existing {
                    tracing::info!(user_id = %user.id, key = %key, "Idempotency key hit, returning cached response");
                    if let Some(body) = response_body {
                        let mut headers = HeaderMap::new();
                        headers.insert(
                            axum::http::header::CONTENT_TYPE,
                            "application/json".parse().unwrap_or_else(|_| {
                                axum::http::HeaderValue::from_static("application/json")
                            }),
                        );
                        if let Some(redirect) = body.get("redirect_url").and_then(|v| v.as_str()) {
                            if let Ok(hx_redir) = redirect.parse() {
                                headers.insert("HX-Redirect", hx_redir);
                            }
                        }
                        return (
                            axum::http::StatusCode::from_u16(response_status.unwrap_or(200) as u16)
                                .unwrap_or(axum::http::StatusCode::OK),
                            headers,
                            Json(body),
                        )
                            .into_response();
                    } else {
                        return (
                            axum::http::StatusCode::CONFLICT,
                            Html(r#"<div style="color:#B42318;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:12px;padding:16px;margin-top:16px;">This checkout is already processing. Please wait and refresh the page.</div>"#.to_string()),
                        ).into_response();
                    }
                }
            }
            _ => {}
        }
    }

    let mut payment_currency_opt: Option<String> = None;
    let mut payment_method_opt: Option<String> = None;
    let mut proof_url: Option<String> = None;
    let mut proof_upload_failed = false;
    let mut bank_transfer_ack = false;
    // 19.8: 3 required general disclosures + 3 referral-only disclosures.
    // Backend rejects checkout if any required box is missing.
    let mut disclosure_general_1 = false;
    let mut disclosure_general_2 = false;
    let mut disclosure_general_3 = false;
    let mut disclosure_referral_1 = false;
    let mut disclosure_referral_2 = false;
    let mut disclosure_referral_3 = false;

    if content_type.contains("multipart/form-data") {
        // Rebuild the request to extract multipart
        let rebuilt = axum::http::Request::from_parts(parts, body);
        let mut multipart: axum::extract::Multipart =
            match axum::extract::Multipart::from_request(rebuilt, &state).await {
                Ok(m) => m,
                Err(e) => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        Html(format!(
                            "<div style='color:red'>Multipart error: {}</div>",
                            e
                        )),
                    )
                        .into_response();
                }
            };
        while let Ok(Some(field)) = multipart.next_field().await {
            if let Some(name) = field.name() {
                match name {
                    "payment_currency" => {
                        if let Ok(text) = field.text().await {
                            payment_currency_opt = Some(text);
                        }
                    }
                    "payment_method" => {
                        if let Ok(text) = field.text().await {
                            payment_method_opt = Some(text);
                        }
                    }
                    "proof_of_transfer" => {
                        let ctype = field
                            .content_type()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "application/octet-stream".to_string());
                        let name = field
                            .file_name()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "proof.bin".to_string());
                        if let Ok(data) = field.bytes().await {
                            if !data.is_empty() {
                                let Some(bucket) = state.config.gcs_bucket.clone() else {
                                    if app_env_allows_local_upload_placeholder(
                                        &state.config.app_env,
                                    ) {
                                        tracing::warn!(
                                            user_id = %user.id,
                                            app_env = %state.config.app_env,
                                            "GCS_BUCKET_NAME not configured; using local-only proof placeholder"
                                        );
                                        proof_url = Some(local_test_proof_url(user.id, &name));
                                    } else {
                                        tracing::error!("GCS_BUCKET_NAME not configured — cannot upload proof of transfer");
                                        proof_upload_failed = true;
                                    }
                                    continue;
                                };
                                let object_path = format!("proofs/{}/{}", user.id, name);
                                match crate::storage::service::upload_private(
                                    &bucket,
                                    &object_path,
                                    data.to_vec(),
                                    &ctype,
                                )
                                .await
                                {
                                    Ok(url) => {
                                        proof_url = Some(url);
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            user_id = %user.id,
                                            bucket = %bucket,
                                            path = %object_path,
                                            error = %e,
                                            "GCS upload failed for proof of transfer"
                                        );
                                        proof_upload_failed = true;
                                    }
                                }
                            }
                        }
                    }
                    "bank_transfer_ack" => {
                        if let Ok(text) = field.text().await {
                            bank_transfer_ack = text == "on" || text == "true";
                        }
                    }
                    "disclosure_general_1" => {
                        if let Ok(text) = field.text().await {
                            disclosure_general_1 = text == "on" || text == "true";
                        }
                    }
                    "disclosure_general_2" => {
                        if let Ok(text) = field.text().await {
                            disclosure_general_2 = text == "on" || text == "true";
                        }
                    }
                    "disclosure_general_3" => {
                        if let Ok(text) = field.text().await {
                            disclosure_general_3 = text == "on" || text == "true";
                        }
                    }
                    "disclosure_referral_1" => {
                        if let Ok(text) = field.text().await {
                            disclosure_referral_1 = text == "on" || text == "true";
                        }
                    }
                    "disclosure_referral_2" => {
                        if let Ok(text) = field.text().await {
                            disclosure_referral_2 = text == "on" || text == "true";
                        }
                    }
                    "disclosure_referral_3" => {
                        if let Ok(text) = field.text().await {
                            disclosure_referral_3 = text == "on" || text == "true";
                        }
                    }
                    _ => {}
                }
            }
        }
    } else {
        // application/x-www-form-urlencoded (wallet checkout, no file upload)
        use axum::extract::FromRequest;
        let rebuilt = axum::http::Request::from_parts(parts, body);
        let form = axum::extract::Form::<std::collections::HashMap<String, String>>::from_request(
            rebuilt, &state,
        )
        .await;
        if let Ok(axum::extract::Form(map)) = form {
            payment_currency_opt = map.get("payment_currency").cloned();
            payment_method_opt = map.get("payment_method").cloned();
            // 19.8 — parse new disclosure checkboxes from URL-encoded form.
            let check = |k: &str| {
                map.get(k)
                    .map(|s| s == "on" || s == "true")
                    .unwrap_or(false)
            };
            disclosure_general_1 = check("disclosure_general_1");
            disclosure_general_2 = check("disclosure_general_2");
            disclosure_general_3 = check("disclosure_general_3");
            bank_transfer_ack = check("bank_transfer_ack");
            disclosure_referral_1 = check("disclosure_referral_1");
            disclosure_referral_2 = check("disclosure_referral_2");
            disclosure_referral_3 = check("disclosure_referral_3");
        }
    }

    let payment_currency = payment_currency_opt
        .unwrap_or_else(|| "USD".to_string())
        .trim()
        .to_uppercase();

    let payment_method = payment_method_opt
        .unwrap_or_else(|| "wallet".to_string())
        .trim()
        .to_lowercase();

    if payment_currency != "USD" && payment_currency != "IDR" {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Invalid payment currency.</div>"#.to_string()),
        ).into_response();
    }

    // ── 19.8: Validate disclosure checkboxes against user type ──────
    // Direct users must accept the 3 general disclosures. Referral users
    // must additionally accept the 3 referral-specific disclosures. The
    // backend is the authority — frontend hiding/showing checkboxes
    // does not relax this.
    let is_referral_user: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM affiliate_referrals WHERE referred_user_id = $1 LIMIT 1)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let agreed_to_general = disclosure_general_1 && disclosure_general_2 && disclosure_general_3;
    let agreed_to_referral =
        disclosure_referral_1 && disclosure_referral_2 && disclosure_referral_3;

    if !agreed_to_general {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">You must accept all three general investment disclosures to continue.</div>"#.to_string()),
        ).into_response();
    }
    if is_referral_user && !agreed_to_referral {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">You must accept all three referral disclosures to continue.</div>"#.to_string()),
        ).into_response();
    }

    // Bank transfer orders require proof of transfer to prevent bypassing the UI requirement.
    if payment_method == "bank_transfer" || payment_method == "bank" {
        if !bank_transfer_ack {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">You must confirm that the bank transfer reference and fee instructions will be followed.</div>"#.to_string()),
            ).into_response();
        }
        if proof_upload_failed {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">File upload failed. Please try again or contact support if the issue persists.</div>"#.to_string()),
            ).into_response();
        }
        if proof_url.is_none() {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Proof of transfer is required for bank transfer payments.</div>"#.to_string()),
            ).into_response();
        }
    }

    match service::execute_checkout(
        &state.db,
        user.id,
        &payment_currency,
        &payment_method,
        proof_url,
    )
    .await
    {
        Ok(result) => {
            // Determine redirect based on payment method:
            // - wallet payments are completed immediately → /payment-success
            // - bank transfers are pending verification → /payment-in-progress
            let redirect_url = if payment_method == "wallet" {
                "/payment-success"
            } else {
                "/payment-in-progress"
            };

            tracing::info!(
                order = %result.order_number,
                invoice = ?result.invoice_number,
                redirect = %redirect_url,
                "Checkout success"
            );

            // If wallet transaction, trigger milestones immediately
            if payment_method == "wallet" {
                if let Some(c_pool) = &state.community_db {
                    for asset_id in result.purchased_asset_ids {
                        let _ = crate::community::service::trigger_investment_milestones(
                            &state.db, c_pool, user.id, asset_id,
                        )
                        .await;
                    }
                }
            }

            // 19.9: Log Investment Disclosures for tracking. Per-row state
            // captures whether the user actually ticked all general boxes
            // and (for referral users) all referral boxes. Policy version
            // is read from platform_settings.legal_disclosure_version so
            // a policy bump is reflected on the next checkout without code.
            let policy_version: String = sqlx::query_scalar(
                "SELECT value FROM platform_settings WHERE key = 'legal_disclosure_version'",
            )
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

            let _ = sqlx::query!(
                r#"INSERT INTO investment_disclosures_log
                   (user_id, order_id, is_referral_user, agreed_to_general,
                    agreed_to_referral, ip_address, policy_version)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
                user.id,
                result.order_id,
                is_referral_user,
                agreed_to_general,
                if is_referral_user {
                    Some(agreed_to_referral)
                } else {
                    None
                },
                client_ip,
                policy_version
            )
            .execute(&state.db)
            .await;

            // Return JSON with redirect URL so the frontend fetch() can reliably read it.
            // Also include HX-Redirect for any HTMX-based callers.
            let mut headers = HeaderMap::new();
            let header_val = redirect_url
                .parse()
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("/portfolio"));
            headers.insert("HX-Redirect", header_val);
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                "application/json"
                    .parse()
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/json")),
            );

            let json_body = serde_json::json!({
                "success": true,
                "redirect_url": redirect_url,
                "order_number": result.order_number,
            });

            // Masterplan Priority 1: Save idempotency result
            if let Some(key) = &idempotency_key {
                let _ = sqlx::query(
                    "UPDATE idempotency_keys SET response_status = 200, response_body = $1 WHERE key = $2"
                )
                .bind(&json_body)
                .bind(key)
                .execute(&state.db)
                .await;
            }

            (headers, Json(json_body)).into_response()
        }
        Err(error_message) => {
            // Double-Submit Protection:
            // If the cart is empty, it might be because a concurrent request just cleared it.
            // Check if there's a very recent order (last 30 seconds) for this user.
            if error_message == "Your cart is empty" {
                let recent_order = sqlx::query!(
                    "SELECT order_number FROM orders WHERE user_id = $1 AND created_at > NOW() - INTERVAL '30 seconds' ORDER BY created_at DESC LIMIT 1",
                    user.id
                )
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

                if let Some(order) = recent_order {
                    let redirect_url = if payment_method == "wallet" {
                        "/payment-success"
                    } else {
                        "/payment-in-progress"
                    };

                    tracing::info!(
                        user_id = %user.id,
                        order = %order.order_number,
                        "Double-submit detected (empty cart but recent order). Redirecting to success."
                    );

                    let mut headers = HeaderMap::new();
                    let header_val = redirect_url
                        .parse()
                        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("/portfolio"));
                    headers.insert("HX-Redirect", header_val);
                    headers.insert(
                        axum::http::header::CONTENT_TYPE,
                        "application/json".parse().unwrap_or_else(|_| {
                            axum::http::HeaderValue::from_static("application/json")
                        }),
                    );

                    return (
                        headers,
                        Json(serde_json::json!({
                            "success": true,
                            "redirect_url": redirect_url,
                            "order_number": order.order_number,
                        })),
                    )
                        .into_response();
                }
            }

            tracing::warn!(user_id = %user.id, error = %error_message, "Checkout failed");

            // Return contextual error HTML for HTMX swap (SVG icon)
            let error_html = format!(
                "<div style=\"background:#FEF3F2;border:1px solid #FEE4E2;border-radius:12px;padding:16px;margin-top:16px;display:flex;align-items:flex-start;gap:10px;\">\
                    <svg width=\"20\" height=\"20\" viewBox=\"0 0 24 24\" fill=\"none\" style=\"flex-shrink:0;margin-top:2px;\">\
                        <circle cx=\"12\" cy=\"12\" r=\"10\" stroke=\"#B42318\" stroke-width=\"2\"/>\
                        <path d=\"M15 9l-6 6M9 9l6 6\" stroke=\"#B42318\" stroke-width=\"2\" stroke-linecap=\"round\"/>\
                    </svg>\
                    <div>\
                        <div style=\"font-weight:600;color:#B42318;font-size:15px;margin-bottom:4px;\">Payment Failed</div>\
                        <p style=\"font-size:14px;color:#B42318;margin:0;\">{}</p>\
                    </div>\
                </div>",
                error_message
            );

            // On failure, delete the idempotency key so the user can easily retry
            if let Some(key) = &idempotency_key {
                let _ = sqlx::query("DELETE FROM idempotency_keys WHERE key = $1")
                    .bind(key)
                    .execute(&state.db)
                    .await;
            }

            (axum::http::StatusCode::BAD_REQUEST, Html(error_html)).into_response()
        }
    }
}

// ─── Invoice Handlers ───────────────────────────────────────────

/// GET /api/orders/latest – Return the user's most recent order as JSON.
pub async fn api_latest_order(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
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

    let latest_order_res = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            i64,
            String,
            String,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, order_number, total_cents, payment_currency, status, payment_method, created_at
        FROM orders
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await;

    match latest_order_res {
        Ok(Some(order)) => {
            let (order_id, number, total, currency, status, method, created_at) = order;

            // Get order items
            let items = sqlx::query_as::<
                _,
                (
                    i32,
                    i64,
                    String,
                    Option<uuid::Uuid>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    Option<i32>,
                    Option<String>,
                    Option<i32>,
                    Option<String>,
                ),
            >(
                r#"
                SELECT
                    oi.tokens_quantity,
                    oi.token_price_cents,
                    COALESCE(a.title, 'Unknown Asset'),
                    oi.asset_id,
                    a.slug,
                    a.location_city,
                    a.location_country,
                    a.short_description,
                    a.asset_type,
                    a.annual_yield_bps,
                    a.funding_status,
                    a.tokens_available,
                    (SELECT image_url
                     FROM asset_images ai
                     WHERE ai.asset_id = a.id
                     ORDER BY ai.is_cover DESC, ai.sort_order ASC, ai.created_at ASC
                     LIMIT 1) as cover_image_url
                FROM order_items oi
                LEFT JOIN assets a ON oi.asset_id = a.id
                WHERE oi.order_id = $1
                "#,
            )
            .bind(order_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let items_json: Vec<_> = items
                .into_iter()
                .map(
                    |(
                        qty,
                        price,
                        title,
                        asset_id,
                        slug,
                        location_city,
                        location_country,
                        short_description,
                        asset_type,
                        annual_yield_bps,
                        funding_status,
                        tokens_available,
                        cover_image_url,
                    )| {
                    serde_json::json!({
                        "tokens_quantity": qty,
                        "token_price_cents": price,
                        "total_cents": (qty as i64) * price,
                        "asset_title": title,
                        "asset_id": asset_id.map(|id| id.to_string()),
                        "slug": slug,
                        "location_city": location_city,
                        "location_country": location_country,
                        "short_description": short_description,
                        "asset_type": asset_type,
                        "annual_yield_bps": annual_yield_bps,
                        "funding_status": funding_status,
                        "tokens_available": tokens_available,
                        "cover_image_url": cover_image_url.as_ref().map(|u| crate::storage::service::rewrite_gcs_url(u))
                    })
                },
                )
                .collect();

            Json(serde_json::json!({
                "id": order_id.to_string(),
                "order_number": number,
                "total_cents": total,
                "payment_currency": currency,
                "status": status,
                "created_at": created_at.to_rfc3339(),
                "payment_method": method,
                "items": items_json
            }))
            .into_response()
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No orders found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch latest order: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch orders"})),
            )
                .into_response()
        }
    }
}

/// GET /api/invoices – Return the user's invoices as JSON.
pub async fn list_invoices(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
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

    match service::get_user_invoices(&state.db, user.id).await {
        Ok(invoices) => Json(serde_json::json!({"invoices": invoices})).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch invoices: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch invoices"})),
            )
                .into_response()
        }
    }
}

/// GET /api/deposits – Return the user's deposit history as JSON.
pub async fn list_deposits(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
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

    match service::get_user_deposits(&state.db, user.id).await {
        Ok(deposits) => Json(serde_json::json!({"deposits": deposits})).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch deposits: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch deposits"})),
            )
                .into_response()
        }
    }
}

// ─── Wallet Balance API (multi-currency) ────────────────────────

/// GET /api/wallets – Return all wallet balances for the current user.
pub async fn list_wallets(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
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

    let wallets = sqlx::query_as::<_, (uuid::Uuid, String, String, i64)>(
        r#"
        SELECT id, wallet_type, currency, balance_cents
        FROM wallets
        WHERE user_id = $1
        ORDER BY wallet_type, currency
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await;

    match wallets {
        Ok(rows) => {
            let wallet_list: Vec<serde_json::Value> = rows
                .into_iter()
                .map(|(id, wtype, currency, balance)| {
                    serde_json::json!({
                        "id": id.to_string(),
                        "wallet_type": wtype,
                        "currency": currency,
                        "balance_cents": balance,
                        "balance_display": if currency == "IDR" {
                            format!("Rp {}", format_idr_simple(balance))
                        } else {
                            format!("${}.{:02}", balance / 100, (balance % 100).abs())
                        }
                    })
                })
                .collect();

            Json(serde_json::json!({"wallets": wallet_list})).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch wallets: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to fetch wallets"})),
            )
                .into_response()
        }
    }
}

/// Simple IDR formatter for the routes layer.
fn format_idr_simple(cents: i64) -> String {
    let val = cents.to_string();
    let mut result = String::new();
    let bytes = val.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            result.push('.');
        }
        result.push(c as char);
    }
    result
}

/// GET /api/deposits/:id/status – Check the status of a specific deposit (polling/details).
#[allow(dead_code)]
pub async fn api_deposit_status(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> axum::response::Response {
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

    let deposit = sqlx::query_as::<_, DepositRequest>(
        "SELECT * FROM deposit_requests WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await;

    match deposit {
        Ok(Some(d)) => Json(d).into_response(),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Deposit not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch deposit status: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/orders/:id – Return a specific order's details as JSON.
#[allow(dead_code)]
pub async fn api_order_by_id(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> axum::response::Response {
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

    // Fetch order
    let order_res = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            i64,
            String,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, order_number, total_cents, payment_currency, status, created_at
        FROM orders
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await;

    match order_res {
        Ok(Some(order)) => {
            let (order_id, number, total, currency, status, created_at) = order;

            // Get order items
            let items = sqlx::query_as::<_, (i32, i64, String)>(
                r#"
                SELECT oi.tokens_quantity, oi.token_price_cents, COALESCE(a.title, 'Unknown Asset')
                FROM order_items oi
                LEFT JOIN assets a ON oi.asset_id = a.id
                WHERE oi.order_id = $1
                "#,
            )
            .bind(order_id)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let items_json: Vec<_> = items
                .into_iter()
                .map(|(qty, price, title)| {
                    serde_json::json!({
                        "tokens_quantity": qty,
                        "token_price_cents": price,
                        "total_cents": (qty as i64) * price,
                        "asset_title": title
                    })
                })
                .collect();

            Json(serde_json::json!({
                "id": order_id.to_string(),
                "order_number": number,
                "total_cents": total,
                "payment_currency": currency,
                "status": status,
                "created_at": created_at.to_rfc3339(),
                "items": items_json
            }))
            .into_response()
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Order not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch order: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}
// ─── Admin Order Management ────────────────────────────────────
// These endpoints should ideally have admin middleware protection
// In this project, admin routes are typically under /api/admin/...

/// POST /api/admin/orders/:id/approve – Approve a pending order.
pub async fn api_admin_approve_order(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> axum::response::Response {
    // Basic admin check (this should be replaced with robust RBAC if available)
    let _user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    if !middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    match service::approve_order(&state.db, id, _user.id).await {
        Ok((user_id, asset_ids)) => {
            if let Some(c_pool) = &state.community_db {
                for asset_id in asset_ids {
                    let _ = crate::community::service::trigger_investment_milestones(
                        &state.db, c_pool, user_id, asset_id,
                    )
                    .await;
                }
            }

            Json(serde_json::json!({"success": true, "message": "Order approved successfully"}))
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

/// POST /api/admin/orders/:id/reject – Reject a pending order.
pub async fn api_admin_reject_order(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> axum::response::Response {
    let _user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    if !middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required"})),
        )
            .into_response();
    }

    match service::reject_order(&state.db, id, _user.id).await {
        Ok(_) => {
            Json(serde_json::json!({"success": true, "message": "Order rejected successfully"}))
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}
