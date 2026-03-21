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

    // Parse amount
    let amount_str = form.amount.replace([',', '.'], "");
    let amount_cents: i64 = match amount_str.parse::<i64>() {
        Ok(v) if v > 0 => {
            if currency == "USD" {
                v * 100 // Input is dollars, convert to cents
            } else {
                v // IDR input is already in smallest unit
            }
        }
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Html(r#"<div class="auth-error-message" style="color:#F04438;background:#FEF3F2;border:1px solid #FEE4E2;border-radius:8px;padding:12px 16px;font-size:14px;">Invalid amount. Please enter a positive number.</div>"#.to_string()),
            ).into_response();
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
                format!("Rp {}", amount_str)
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
/// Verifies the payment and atomically credits the user's wallet.
/// Idempotent: calling twice with the same reference won't double-credit.
pub async fn payment_webhook(
    State(state): State<AppState>,
    Json(payload): Json<WebhookPayload>,
) -> axum::response::Response {
    // Ensure requests have the correct secret signature to prevent unauthorized calls
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
    if payload.signature.as_deref() != Some(secret.as_str()) {
        tracing::warn!("Webhook rejected: invalid or missing signature");
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized webhook signature"})),
        )
            .into_response();
    }
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
    _jar: CookieJar,
    State(_state): State<AppState>,
) -> axum::response::Response {
    // In a real staging environment, this would come from a secure config or DB.
    // For this MVP, we use the constants from service.rs.
    let usd: serde_json::Value =
        serde_json::from_str(service::BANK_DETAILS_USD).unwrap_or_default();
    let idr: serde_json::Value =
        serde_json::from_str(service::BANK_DETAILS_IDR).unwrap_or_default();

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
    let cart_rows = sqlx::query_as::<
        _,
        (
            uuid::Uuid, uuid::Uuid, i32, i64, String, String, Option<String>, Option<String>,
            Option<String>, String, Option<i32>, String, i32, Option<String>,
        ),
    >(
        r#"
        SELECT
            ci.id, ci.asset_id, ci.tokens_quantity, ci.token_price_cents,
            a.title, a.slug,
            a.location_city, a.location_country, a.short_description,
            a.asset_type, a.annual_yield_bps, a.funding_status, a.tokens_available,
            (SELECT image_url FROM asset_images ai WHERE ai.asset_id = a.id ORDER BY ai.is_cover DESC, ai.sort_order ASC, ai.created_at ASC LIMIT 1) as cover_image_url
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
            let total = r.2 as i64 * r.3;
            cart_total_cents += total;
            serde_json::json!({
                "id": r.0.to_string(), "asset_id": r.1.to_string(), "tokens_quantity": r.2,
                "token_price_cents": r.3, "total_cents": total, "title": r.4, "slug": r.5,
                "location_city": r.6, "location_country": r.7, "short_description": r.8,
                "asset_type": r.9, "annual_yield_bps": r.10, "funding_status": r.11,
                "tokens_available": r.12, "cover_image_url": r.13
            })
        })
        .collect();

    let cart_json = serde_json::json!({
        "items": cart_items,
        "count": cart_items.len(),
        "total_cents": cart_total_cents
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
    let usd: serde_json::Value =
        serde_json::from_str(service::BANK_DETAILS_USD).unwrap_or_default();
    let idr: serde_json::Value =
        serde_json::from_str(service::BANK_DETAILS_IDR).unwrap_or_default();
    let bank_json = serde_json::json!({ "USD": usd, "IDR": idr }).to_string();

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
                            "application/json".parse().unwrap(),
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
                                let bucket = state
                                    .config
                                    .gcs_bucket
                                    .clone()
                                    .unwrap_or_else(|| "poool-storage".to_string());
                                let object_path = format!("proofs/{}/{}", user.id, name);
                                if let Ok(url) = crate::storage::service::upload_private(
                                    &bucket,
                                    &object_path,
                                    data.to_vec(),
                                    &ctype,
                                )
                                .await
                                {
                                    proof_url = Some(url);
                                } else {
                                    tracing::warn!(
                                        "Failed to upload proof of transfer for user {}",
                                        user.id
                                    );
                                }
                            }
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

            // Return JSON with redirect URL so the frontend fetch() can reliably read it.
            // Also include HX-Redirect for any HTMX-based callers.
            let mut headers = HeaderMap::new();
            headers.insert("HX-Redirect", redirect_url.parse().unwrap());
            headers.insert(
                axum::http::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
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
                    headers.insert("HX-Redirect", redirect_url.parse().unwrap());
                    headers.insert(
                        axum::http::header::CONTENT_TYPE,
                        "application/json".parse().unwrap(),
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
        Ok(_) => {
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
