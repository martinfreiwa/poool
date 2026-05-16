/// Wallet route handlers – view balances and execute deposit/withdraw actions.
use axum::{
    extract::{Form, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::models::*;
use crate::auth::middleware;
use crate::auth::routes::AppState;
use crate::common::idempotency::{self, Reservation};
use crate::payment_methods;
use crate::storage::service as storage_svc;

/// Hard cap on a single proof-of-transfer upload. Matches the existing KYC
/// limit so users have a consistent mental model for "max file size".
const MAX_DEPOSIT_PROOF_BYTES: usize = 10 * 1024 * 1024;

// ─── Forms ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DepositForm {
    pub amount: String, // from the UI, e.g. "100"
    pub payment_method_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawForm {
    pub amount: String,
    pub payment_method_id: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct TransactionPagination {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}
// ─── Helpers ────────────────────────────────────────────────────

/// Helper to format cents into "USD X,XXX.XX"
fn format_usd(cents: i64) -> String {
    let abs_cents = cents.unsigned_abs();
    let dollars = abs_cents / 100;
    let remainder = abs_cents % 100;
    // Comma formatting for the dollar part
    let s = dollars.to_string();
    let mut result = String::new();
    let b = s.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if i > 0 && (b.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c as char);
    }

    // Always return positive-looking float string,
    // negative logic is handled separately by the prefix in UI
    format!("USD {}.{:02}", result, remainder)
}

/// Helper to ensure wallets exist for user (always sets currency = 'USD')
pub async fn ensure_wallets(pool: &sqlx::PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
        VALUES ($1, 'cash', 0, 'USD'), ($1, 'rewards', 0, 'USD')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Build a display-ready `WalletTransaction` from a raw DB row.
fn build_transaction(
    idx: usize,
    id: Uuid,
    tx_type_str: &str,
    status_str: &str,
    date: &DateTime<Utc>,
    wallet_type_str: &str,
    amount_cents: i64,
) -> WalletTransaction {
    let tx_type = TransactionType::from_db(tx_type_str);
    let status = TransactionStatus::from_db(status_str);
    let wallet_type = WalletType::from_db(wallet_type_str);

    let (amount_css, amount_prefix, formatted_amount) = if amount_cents >= 0 {
        (
            "amount-positive".to_string(),
            "+".to_string(),
            format_usd(amount_cents),
        )
    } else {
        (
            "amount-negative".to_string(),
            "-".to_string(),
            format_usd(-amount_cents),
        )
    };

    WalletTransaction {
        index: idx,
        id,
        tx_type_label: tx_type.display_label().to_string(),
        tx_type_icon: tx_type.icon_key().to_string(),
        tx_type,
        status_label: status.display_label().to_string(),
        status_css: status.css_class().to_string(),
        status,
        date_display: date.format("%d %b %Y").to_string(),
        date_iso: date.to_rfc3339(),
        wallet_label: wallet_type.display_label().to_string(),
        amount_cents,
        amount_display: formatted_amount,
        amount_prefix,
        amount_css,
    }
}

// ─── Service Layer ──────────────────────────────────────────────

/// Fetch the three wallet balances for a user (cash, rewards, assets).
async fn fetch_balances(pool: &sqlx::PgPool, user_id: Uuid) -> (i64, i64, i64) {
    let cash_balance: i64 = sqlx::query_scalar(
        "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'cash'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    let rewards_balance: i64 = sqlx::query_scalar(
        "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'rewards'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    let asset_balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(current_value_cents), 0) FROM investments WHERE user_id = $1 AND status != 'exited'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    (cash_balance, rewards_balance, asset_balance)
}

/// Fetch the most recent wallet transactions for a user.
async fn fetch_transactions(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    limit: i64,
) -> Vec<WalletTransaction> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>, String, i64)>(
        r#"
        SELECT t.id, t.type, t.status, t.created_at, w.wallet_type, t.amount_cents
        FROM wallet_transactions t
        JOIN wallets w ON w.id = t.wallet_id
        WHERE w.user_id = $1
        ORDER BY t.created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter()
        .enumerate()
        .map(|(idx, (id, tx_type, status, date, wallet_t, amount))| {
            build_transaction(idx, *id, tx_type, status, date, wallet_t, *amount)
        })
        .collect()
}

/// Build HTML fragments for payment methods (cards, banks, options, mobile).
fn build_payment_method_html(
    pms: &[crate::payment_methods::models::PaymentMethod],
) -> (String, String, String, String, String, bool, bool) {
    let mut cards_html = String::new();
    let mut banks_html = String::new();
    let mut mobile_cards_html = String::new();
    let mut mobile_banks_html = String::new();
    let mut options_html = String::new();
    let mut has_cards = false;
    let mut has_banks = false;

    if pms.is_empty() {
        options_html.push_str(
            r#"<option value="" disabled selected>No payment methods available</option>"#,
        );
    }

    for pm in pms {
        let label = pm.label.clone().unwrap_or_else(|| {
            if pm.method_type == "bank" {
                pm.brand
                    .clone()
                    .unwrap_or_else(|| "Bank Account".to_string())
            } else {
                format!(
                    "{} ending in {}",
                    pm.brand.clone().unwrap_or_else(|| "Card".into()),
                    pm.last_four.clone().unwrap_or_else(|| "****".into())
                )
            }
        });

        let sub_label = if let Some(l4) = &pm.last_four {
            format!("ending in {}", l4)
        } else {
            String::new()
        };

        // Option tag for deposit/withdraw modals
        options_html.push_str(&format!(r#"<option value="{}">{}</option>"#, pm.id, label));

        // Delete button (desktop)
        let menu_html = format!(
            r##"<button class="wallet-payment-item-menu" hx-delete="/api/payment-methods/{}" hx-swap="none" onclick="if(confirm('Are you sure you want to delete this payment method?')) {{ document.location.reload(); return true; }} else {{ return false; }}"><img src="/static/images/icons/trash-icon.svg" onerror="this.src='/static/images/icons/dots-vertical.svg'" alt="Delete" width="24" height="24"></button>"##,
            pm.id
        );
        // Delete button (mobile)
        let mobile_menu_html = format!(
            r##"<button class="mobile-card-menu" hx-delete="/api/payment-methods/{}" hx-swap="none" onclick="if(confirm('Delete this payment method?')) {{ document.location.reload(); return true; }} else {{ return false; }}"><svg width="24" height="24" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="5" r="1" fill="#000000"></circle><circle cx="12" cy="12" r="1" fill="#000000"></circle><circle cx="12" cy="19" r="1" fill="#000000"></circle></svg></button>"##,
            pm.id
        );

        if pm.method_type == "card" {
            has_cards = true;
            // Detect brand from pm.brand (Stripe-populated) or fall back to label
            let brand_lower = pm.brand.as_deref().unwrap_or("").to_lowercase();
            let label_lower = label.to_lowercase();
            let img = if brand_lower.contains("visa") || label_lower.contains("visa") {
                "/static/images/icons/visa.webp"
            } else if brand_lower.contains("mastercard") || label_lower.contains("mastercard") {
                "/static/images/icons/mastercard.svg"
            } else {
                // Covers amex, other brands, and unknown cards
                "/static/images/icons/card-default.svg"
            };

            cards_html.push_str(&format!(
                r##"<div id="wallet-payment-cards-item-{}" class="wallet-payment-item">
                    <div class="wallet-payment-item-logo"><img src="{}" alt="{}" width="58" height="40"></div>
                    <div class="wallet-payment-item-spacer"></div>
                    <div class="wallet-payment-item-details"><span class="wallet-payment-item-text">{} {}</span> {}</div>
                </div>"##,
                pm.id, img, label, label, sub_label, menu_html
            ));

            mobile_cards_html.push_str(&format!(
                r##"<div class="mobile-card-item">
                    <div class="mobile-card-content">
                        <div class="mobile-card-icon"><img src="{}" alt="{}" width="58" height="40"></div>
                        <div class="mobile-card-details"><span class="mobile-card-text">{} {}</span></div>
                        {}
                    </div>
                </div>"##,
                img, label, label, sub_label, mobile_menu_html
            ));
        } else {
            has_banks = true;
            banks_html.push_str(&format!(
                r##"<div id="wallet-payment-banks-item-{}" class="wallet-payment-item" style="padding: 16px; border-bottom: 1px solid #EAECF0; display:flex; align-items:center;">
                    <div class="wallet-payment-icon" style="background: #E0E7FF; padding:10px; border-radius: 50%;"><svg width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M3 8H17V16H3V8Z" stroke="#4F46E5" stroke-width="1.66667"></path><path d="M10 4L17 8H3L10 4Z" stroke="#4F46E5" stroke-width="1.66667" stroke-linejoin="round"></path><path d="M6 11V13M10 11V13M14 11V13" stroke="#4F46E5" stroke-width="1.66667" stroke-linecap="round"></path></svg></div>
                    <div class="wallet-payment-item-spacer" style="margin-left: 12px;"></div>
                    <div class="wallet-payment-item-details" style="flex:1; display:flex; justify-content:space-between; align-items:center;"><span class="wallet-payment-item-text" style="color: #101828; font-weight: 500;">{} {}</span> {}</div>
                </div>"##,
                pm.id, label, sub_label, menu_html
            ));

            mobile_banks_html.push_str(&format!(
                r##"<div class="mobile-bank-item" style="padding: 16px; border: 1px solid #EAECF0; border-radius: 12px; margin-bottom: 12px;">
                    <div class="mobile-bank-content" style="display:flex; align-items:center; gap: 12px;">
                        <div class="mobile-bank-icon" style="background:#E0E7FF; border-radius: 50%; padding: 10px;"><svg width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M3 8H17V16H3V8Z" stroke="#4F46E5" stroke-width="1.66667"></path><path d="M10 4L17 8H3L10 4Z" stroke="#4F46E5" stroke-width="1.66667" stroke-linejoin="round"></path><path d="M6 11V13M10 11V13M14 11V13" stroke="#4F46E5" stroke-width="1.66667" stroke-linecap="round"></path></svg></div>
                        <div class="mobile-bank-details" style="flex:1;"><span class="mobile-bank-text" style="color:#101828; font-weight:500;">{} {}</span></div>
                        {}
                    </div>
                </div>"##,
                label, sub_label, mobile_menu_html
            ));
        }
    }

    // Empty-state fallbacks
    if cards_html.is_empty() {
        cards_html = r##"<div class="wallet-payment-empty-state"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#667085" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"></rect><line x1="1" y1="10" x2="23" y2="10"></line></svg><span>No cards added yet</span></div>"##.to_string();
    }
    if banks_html.is_empty() {
        banks_html = r##"<div class="wallet-payment-empty-state"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#667085" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9.5L12 4L21 9.5"></path><path d="M5 20H19"></path><path d="M10 9.5V17M14 9.5V17M6 17V9.5M18 17V9.5"></path></svg><span>No banks added yet</span></div>"##.to_string();
    }

    (
        cards_html,
        banks_html,
        mobile_cards_html,
        mobile_banks_html,
        options_html,
        has_cards,
        has_banks,
    )
}

/// Parse a user-supplied dollar string into cents using string manipulation.
/// Avoids IEEE754 float rounding errors (e.g., 19.99 * 100 != 1999).
fn parse_dollars_to_cents(raw: &str) -> i64 {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if cleaned.is_empty() {
        return 0;
    }
    let parts: Vec<&str> = cleaned.split('.').collect();
    let dollars: i64 = parts[0].parse().unwrap_or(0);
    let cents: i64 = if parts.len() > 1 {
        let frac = parts[1];
        match frac.len() {
            0 => 0,
            1 => frac.parse::<i64>().unwrap_or(0) * 10, // "5" → 50 cents
            _ => frac[..2].parse::<i64>().unwrap_or(0), // "99" or "995" → 99 cents
        }
    } else {
        0
    };
    dollars * 100 + cents
}

// ─── Deposit / Withdraw Handlers ────────────────────────────────

/// POST /wallet/deposit  (multipart/form-data)
///
/// Expected fields:
///   - `amount`     — decimal string ("250.00")
///   - `proof`      — file (PDF / PNG / JPEG / WebP), MANDATORY
///   - `notes`      — optional free-text from the user
///
/// On success:
///   1. Validates amount against admin-configured min/max
///   2. Creates a deposit_requests row (status='pending', unique provider_reference)
///   3. Uploads the proof to GCS at `gs://BUCKET/deposits/{user_id}/{deposit_id}.{ext}`
///   4. Persists the proof path + optional notes
///   5. Redirects to /wallet with a success flash
pub async fn handle_deposit(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let bank = crate::payments::service::fetch_deposit_bank_settings(&state.db).await;

    // ── 1. Parse multipart fields ────────────────────────────────
    let mut amount_raw: Option<String> = None;
    let mut notes: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut declared_mime = String::from("application/octet-stream");

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "amount" => amount_raw = field.text().await.ok(),
            "notes" => notes = field.text().await.ok().filter(|s| !s.trim().is_empty()),
            "proof" => {
                if let Some(ct) = field.content_type() {
                    declared_mime = ct.to_string();
                }
                let mut field = field;
                let mut bytes: Vec<u8> = Vec::with_capacity(64 * 1024);
                loop {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            if bytes.len().saturating_add(chunk.len()) > MAX_DEPOSIT_PROOF_BYTES {
                                return Redirect::to("/wallet?error=proof_too_large")
                                    .into_response();
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                        Ok(None) => break,
                        Err(_) => {
                            return Redirect::to("/wallet?error=proof_read_failed").into_response();
                        }
                    }
                }
                if !bytes.is_empty() {
                    file_bytes = Some(bytes);
                }
            }
            _ => {}
        }
    }

    let amount_cents = parse_dollars_to_cents(amount_raw.as_deref().unwrap_or(""));
    if amount_cents <= 0 {
        return Redirect::to("/wallet?error=invalid_amount").into_response();
    }
    if amount_cents < bank.min_amount_cents {
        return Redirect::to("/wallet?error=amount_too_small").into_response();
    }
    if amount_cents > bank.max_amount_cents {
        return Redirect::to("/wallet?error=amount_too_large").into_response();
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => return Redirect::to("/wallet?error=proof_missing").into_response(),
    };

    // ── 2. Validate the proof file: magic-byte sniff + allow-list ────
    let sniffed = match storage_svc::sniff_mime(&file_bytes) {
        Some(m) => m,
        None => return Redirect::to("/wallet?error=proof_unsupported_format").into_response(),
    };
    if !storage_svc::mime_matches(&declared_mime, sniffed) {
        return Redirect::to("/wallet?error=proof_mime_mismatch").into_response();
    }
    if storage_svc::validate_kyc_mime(sniffed).is_err() {
        return Redirect::to("/wallet?error=proof_unsupported_format").into_response();
    }

    // ── 3. Create the deposit_requests row ──────────────────────────
    let deposit_res = match crate::payments::service::create_deposit_request(
        &state.db,
        user.id,
        "USD",
        amount_cents,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                "Failed to create deposit request for user {}: {}",
                user.id,
                e
            );
            return Redirect::to("/wallet?error=deposit_failed").into_response();
        }
    };

    let deposit_id = deposit_res.deposit_id;
    let provider_ref = deposit_res.provider_reference.clone().unwrap_or_default();

    // ── 4. Upload the proof to GCS (with local fallback for dev) ─────
    let ext = storage_svc::extension_for_mime(sniffed);
    let object_path = format!("deposits/{}/{}.{}", user.id, deposit_id, ext);
    let bucket = state.config.gcs_bucket.clone();
    let proof_path: String = if let Some(ref b) = bucket {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            storage_svc::upload_private(b, &object_path, file_bytes.clone(), sniffed),
        )
        .await
        {
            Ok(Ok(p)) => p,
            _ => {
                tracing::warn!("GCS deposit proof upload failed, falling back to local");
                match storage_svc::upload_local(&object_path, file_bytes).await {
                    Ok(p) => p,
                    Err(_) => {
                        return Redirect::to("/wallet?error=proof_upload_failed").into_response()
                    }
                }
            }
        }
    } else {
        match storage_svc::upload_local(&object_path, file_bytes).await {
            Ok(p) => p,
            Err(_) => return Redirect::to("/wallet?error=proof_upload_failed").into_response(),
        }
    };

    // ── 5. Persist proof path + notes on the deposit row ────────────
    if let Err(e) = sqlx::query(
        "UPDATE deposit_requests
            SET proof_gcs_path = $1, proof_uploaded_at = NOW(), user_notes = $2
          WHERE id = $3",
    )
    .bind(&proof_path)
    .bind(notes.as_deref())
    .bind(deposit_id)
    .execute(&state.db)
    .await
    {
        tracing::error!(
            "Failed to attach proof to deposit {}: {} (orphaned file at {})",
            deposit_id,
            e,
            proof_path
        );
        // The deposit row exists; admin can still match the wire from the
        // reference. We surface a soft warning rather than failing the user.
        return Redirect::to(&format!(
            "/wallet?deposit_created=true&ref={}&amount={}&proof_warning=1",
            provider_ref, amount_cents
        ))
        .into_response();
    }

    Redirect::to(&format!(
        "/wallet?deposit_created=true&ref={}&amount={}",
        provider_ref, amount_cents
    ))
    .into_response()
}

// ─── Two-step deposit flow ─────────────────────────────────────────
//
// The multi-step deposit modal calls `api_deposit_init` first to get the
// reference + bank details (creates the row), then `handle_deposit_submit`
// after the user has wired the funds and uploaded the proof.

#[derive(Debug, Deserialize)]
pub struct DepositInitPayload {
    pub amount: String,
    /// AMLD5/6 source-of-funds reason. Required when amount >=
    /// `deposit_sof_threshold_cents`. One of the values whitelisted by
    /// [`crate::payments::service::normalize_sof_reason`].
    #[serde(default)]
    pub source_of_funds_reason: Option<String>,
    /// Optional free-text context (required when reason is `other`, useful
    /// for any reason). Capped to 500 chars after trimming.
    #[serde(default)]
    pub source_of_funds_detail: Option<String>,
}

/// POST /api/wallet/deposit/init  – step 1 of the modal flow.
///
/// Validates the requested amount against the admin-configured min/max
/// limits, creates a `deposit_requests` row, and returns everything the
/// step-2 view needs to render the wire instructions: bank details, the
/// unique reference, the expiry, and the expected processing window.
pub async fn api_deposit_init(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DepositInitPayload>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response();
        }
    };

    // ── Idempotency-Key check ────────────────────────────────────
    // A retried POST returns the original deposit reference + bank details
    // instead of allocating a second deposit_requests row.
    let idem_key = match idempotency::try_reserve(
        &state.db,
        &headers,
        user.id,
        "/api/wallet/deposit/init",
        "POST",
    )
    .await
    {
        Reservation::NoKey => None,
        Reservation::Reserved(key) => Some(key),
        Reservation::CachedJson { status, body } => return (status, Json(body)).into_response(),
        Reservation::CachedRedirect { location } => return Redirect::to(&location).into_response(),
        Reservation::InProgress => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "in_progress",
                    "message": "An earlier request with this Idempotency-Key is still processing.",
                })),
            )
                .into_response();
        }
    };

    let bank = crate::payments::service::fetch_deposit_bank_settings(&state.db).await;
    let amount_cents = parse_dollars_to_cents(&payload.amount);
    if amount_cents <= 0 {
        if let Some(ref k) = idem_key {
            idempotency::release(&state.db, k).await;
        }
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Enter an amount greater than zero"})),
        )
            .into_response();
    }
    if amount_cents < bank.min_amount_cents {
        if let Some(ref k) = idem_key {
            idempotency::release(&state.db, k).await;
        }
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "amount_too_small",
                "message": format!("Minimum deposit is {}", format_usd(bank.min_amount_cents))
            })),
        )
            .into_response();
    }
    if amount_cents > bank.max_amount_cents {
        if let Some(ref k) = idem_key {
            idempotency::release(&state.db, k).await;
        }
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "amount_too_large",
                "message": format!("Maximum deposit is {}", format_usd(bank.max_amount_cents))
            })),
        )
            .into_response();
    }

    // ── Source-of-funds gate (AMLD5/6 Art. 18) ───────────────────
    let sof_required = amount_cents >= bank.sof_threshold_cents;
    let sof_doc_required = amount_cents >= bank.sof_doc_threshold_cents;
    let normalized_sof = payload
        .source_of_funds_reason
        .as_deref()
        .and_then(crate::payments::service::normalize_sof_reason)
        .map(|s| s.to_string());

    if sof_required && normalized_sof.is_none() {
        if let Some(ref k) = idem_key {
            idempotency::release(&state.db, k).await;
        }
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "sof_reason_required",
                "message": format!(
                    "A source-of-funds reason is required for deposits of {} or more.",
                    format_usd(bank.sof_threshold_cents)
                )
            })),
        )
            .into_response();
    }

    let sof_detail = payload
        .source_of_funds_detail
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(500).collect::<String>());
    if normalized_sof.as_deref() == Some("other") && sof_detail.is_none() {
        if let Some(ref k) = idem_key {
            idempotency::release(&state.db, k).await;
        }
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "sof_detail_required",
                "message": "Please describe the source of these funds."
            })),
        )
            .into_response();
    }

    let deposit_res = match crate::payments::service::create_deposit_request(
        &state.db,
        user.id,
        "USD",
        amount_cents,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("deposit_init create failed for user {}: {}", user.id, e);
            if let Some(ref k) = idem_key {
                idempotency::release(&state.db, k).await;
            }
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Could not create deposit request"})),
            )
                .into_response();
        }
    };

    // Persist the SoF declaration on the row we just created. Best-effort:
    // if this fails the request still succeeds — admin can backfill from
    // the audit_logs entry below.
    if normalized_sof.is_some() {
        if let Err(e) = sqlx::query(
            "UPDATE deposit_requests
                SET source_of_funds_reason = $1, source_of_funds_detail = $2
              WHERE id = $3",
        )
        .bind(normalized_sof.as_deref())
        .bind(sof_detail.as_deref())
        .bind(deposit_res.deposit_id)
        .execute(&state.db)
        .await
        {
            tracing::error!(
                deposit_id = %deposit_res.deposit_id,
                error = %e,
                "Failed to persist source-of-funds declaration"
            );
        }
    }

    let expires_at = (Utc::now() + chrono::Duration::hours(bank.processing_hours)).to_rfc3339();

    let response_body = serde_json::json!({
        "deposit_id": deposit_res.deposit_id.to_string(),
        "reference": deposit_res.provider_reference.unwrap_or_default(),
        "amount_cents": amount_cents,
        "amount_display": format_usd(amount_cents),
        "currency": "USD",
        "bank": {
            "bank_name": bank.bank_name,
            "account_holder": bank.account_holder,
            "iban": bank.iban,
            "bic": bank.bic,
            "bank_address": bank.bank_address,
        },
        "processing_hours": bank.processing_hours,
        "sof_required": sof_required,
        "sof_doc_required": sof_doc_required,
        "sof_threshold_cents": bank.sof_threshold_cents,
        "sof_doc_threshold_cents": bank.sof_doc_threshold_cents,
        "expires_at": expires_at,
        "submit_url": format!("/wallet/deposit/{}/submit", deposit_res.deposit_id),
    });

    if let Some(ref k) = idem_key {
        idempotency::commit_json(&state.db, k, StatusCode::OK, &response_body).await;
    }

    crate::metrics::record_deposit(
        crate::metrics::deposit_outcome::INITIATED,
        "USD",
        amount_cents,
    );

    Json(response_body).into_response()
}

/// POST /wallet/deposit/:id/submit  – step 2 of the modal flow.
///
/// Multipart form:
///   - `proof`  — file (PDF / PNG / JPEG / WebP), MANDATORY
///   - `notes`  — optional free-text from the user
///
/// Verifies the deposit row belongs to the caller and is still 'pending'
/// (i.e. not yet credited or expired), uploads the proof to GCS, and
/// attaches the GCS path + notes to the row. Redirects to /wallet with
/// a success flash that the existing wallet.js URL-param handler picks up.
pub async fn handle_deposit_submit(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(deposit_id): Path<Uuid>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // ── Idempotency ──────────────────────────────────────────────
    // A double-submit (double-click, network retry) would otherwise upload
    // a second proof file and burn GCS bandwidth. Reservation also unblocks
    // safe browser refresh after a slow upload.
    let idem_key = match idempotency::try_reserve(
        &state.db,
        &headers,
        user.id,
        &format!("/wallet/deposit/{}/submit", deposit_id),
        "POST",
    )
    .await
    {
        Reservation::NoKey => None,
        Reservation::Reserved(k) => Some(k),
        Reservation::CachedRedirect { location } => return Redirect::to(&location).into_response(),
        Reservation::CachedJson { .. } => {
            return Redirect::to("/wallet?deposit_completed=true").into_response();
        }
        Reservation::InProgress => {
            return Redirect::to("/wallet?error=in_progress").into_response();
        }
    };

    // Helper: every "give up early" path goes through this so the
    // idempotency reservation is released and the user can retry with the
    // same key. Side-effect-free errors only — errors after the GCS upload
    // keep the reservation (handled inline below).
    let release_and_redirect = |url: &str| {
        let db = state.db.clone();
        let key = idem_key.clone();
        let url = url.to_string();
        async move {
            if let Some(k) = key {
                idempotency::release(&db, &k).await;
            }
            Redirect::to(&url).into_response()
        }
    };

    // Confirm the deposit exists, belongs to this user, and is still open.
    // Pull source-of-funds bits too so we can enforce the doc-upload gate.
    let deposit_row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT user_id, status, amount_cents, provider_reference,
                proof_gcs_path, source_of_funds_reason, source_of_funds_doc_path
           FROM deposit_requests WHERE id = $1",
    )
    .bind(deposit_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let (owner, status, amount_cents, provider_ref, existing_proof, sof_reason, existing_sof_doc) =
        match deposit_row {
            Some(row) => row,
            None => return release_and_redirect("/wallet?error=deposit_not_found").await,
        };

    if owner != user.id {
        return release_and_redirect("/wallet?error=deposit_not_found").await;
    }
    if status != "pending" && status != "requested" {
        return release_and_redirect("/wallet?error=deposit_not_pending").await;
    }
    if existing_proof.is_some() {
        return release_and_redirect("/wallet?error=proof_already_uploaded").await;
    }

    // ── Parse multipart fields ───────────────────────────────────
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut declared_mime = String::from("application/octet-stream");
    let mut notes: Option<String> = None;
    // Optional supporting document for source-of-funds (AMLD threshold).
    let mut sof_doc_bytes: Option<Vec<u8>> = None;
    let mut sof_doc_mime = String::from("application/octet-stream");

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "notes" => notes = field.text().await.ok().filter(|s| !s.trim().is_empty()),
            "proof" => {
                if let Some(ct) = field.content_type() {
                    declared_mime = ct.to_string();
                }
                let mut field = field;
                let mut bytes: Vec<u8> = Vec::with_capacity(64 * 1024);
                loop {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            if bytes.len().saturating_add(chunk.len()) > MAX_DEPOSIT_PROOF_BYTES {
                                return release_and_redirect("/wallet?error=proof_too_large").await;
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                        Ok(None) => break,
                        Err(_) => {
                            return release_and_redirect("/wallet?error=proof_read_failed").await;
                        }
                    }
                }
                if !bytes.is_empty() {
                    file_bytes = Some(bytes);
                }
            }
            "source_of_funds_doc" => {
                if let Some(ct) = field.content_type() {
                    sof_doc_mime = ct.to_string();
                }
                let mut field = field;
                let mut bytes: Vec<u8> = Vec::with_capacity(64 * 1024);
                loop {
                    match field.chunk().await {
                        Ok(Some(chunk)) => {
                            if bytes.len().saturating_add(chunk.len()) > MAX_DEPOSIT_PROOF_BYTES {
                                return release_and_redirect("/wallet?error=sof_doc_too_large")
                                    .await;
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                        Ok(None) => break,
                        Err(_) => {
                            return release_and_redirect("/wallet?error=sof_doc_read_failed").await;
                        }
                    }
                }
                if !bytes.is_empty() {
                    sof_doc_bytes = Some(bytes);
                }
            }
            _ => {}
        }
    }

    let file_bytes = match file_bytes {
        Some(b) => b,
        None => return release_and_redirect("/wallet?error=proof_missing").await,
    };

    // Enforce the source-of-funds document requirement for large deposits.
    // The threshold is admin-configurable; reload to pick up changes since
    // the row was created.
    let bank = crate::payments::service::fetch_deposit_bank_settings(&state.db).await;
    let needs_sof_doc = amount_cents >= bank.sof_doc_threshold_cents && existing_sof_doc.is_none();
    if needs_sof_doc && sof_doc_bytes.is_none() {
        return release_and_redirect("/wallet?error=sof_doc_required").await;
    }
    // Silence unused-warning suppression for sof_reason — used for audit
    // log enrichment below.
    let _ = sof_reason;

    // ── P0 / C-1 fix: when the AMLD threshold mandates a SoF document,
    // VALIDATE the upload here before we touch the proof row. A failure
    // path that "logs and continues" would let users bypass the gate by
    // submitting a 1-byte file. We pre-sniff the MIME and reject the
    // whole submission on failure so the deposit never gets recorded
    // without its mandatory evidence.
    let mandatory_sof_sniff = if needs_sof_doc {
        match sof_doc_bytes.as_deref().and_then(storage_svc::sniff_mime) {
            Some(m) if storage_svc::mime_matches(&sof_doc_mime, m)
                && storage_svc::validate_kyc_mime(m).is_ok() =>
            {
                Some(m)
            }
            _ => {
                return release_and_redirect("/wallet?error=sof_doc_unsupported_format").await;
            }
        }
    } else {
        None
    };

    // ── Validate proof: magic-byte sniff + allow-list ────────────
    let sniffed = match storage_svc::sniff_mime(&file_bytes) {
        Some(m) => m,
        None => return release_and_redirect("/wallet?error=proof_unsupported_format").await,
    };
    if !storage_svc::mime_matches(&declared_mime, sniffed) {
        return release_and_redirect("/wallet?error=proof_mime_mismatch").await;
    }
    if storage_svc::validate_kyc_mime(sniffed).is_err() {
        return release_and_redirect("/wallet?error=proof_unsupported_format").await;
    }

    // ── Upload to GCS (local fallback for dev) ───────────────────
    let ext = storage_svc::extension_for_mime(sniffed);
    let object_path = format!("deposits/{}/{}.{}", user.id, deposit_id, ext);
    let bucket = state.config.gcs_bucket.clone();
    let proof_path: String = if let Some(ref b) = bucket {
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            storage_svc::upload_private(b, &object_path, file_bytes.clone(), sniffed),
        )
        .await
        {
            Ok(Ok(p)) => p,
            _ => {
                tracing::warn!("GCS deposit proof upload failed, falling back to local");
                match storage_svc::upload_local(&object_path, file_bytes).await {
                    Ok(p) => p,
                    Err(_) => {
                        // H-3 fix: release the reservation so the user can
                        // retry with the same Idempotency-Key after a
                        // transient storage failure (no point locking them
                        // out for 24h on a flaky upload).
                        return release_and_redirect("/wallet?error=proof_upload_failed").await;
                    }
                }
            }
        }
    } else {
        match storage_svc::upload_local(&object_path, file_bytes).await {
            Ok(p) => p,
            Err(_) => {
                return release_and_redirect("/wallet?error=proof_upload_failed").await;
            }
        }
    };

    if let Err(e) = sqlx::query(
        "UPDATE deposit_requests
            SET proof_gcs_path = $1, proof_uploaded_at = NOW(), user_notes = $2
          WHERE id = $3",
    )
    .bind(&proof_path)
    .bind(notes.as_deref())
    .bind(deposit_id)
    .execute(&state.db)
    .await
    {
        tracing::error!(
            "Failed to attach proof to deposit {}: {} (orphaned file at {})",
            deposit_id,
            e,
            proof_path
        );
        // H-3 fix: release so the user can retry; the orphaned GCS file
        // will be cleaned up by the storage GC sweep.
        return release_and_redirect("/wallet?error=proof_save_failed").await;
    }

    // ── Upload SoF document ─────────────────────────────────────
    // For mandatory submissions (`needs_sof_doc`) any failure aborts and
    // logs an explicit error so the user retries. For optional submissions
    // (below the threshold) we keep the best-effort behaviour — chase via
    // email if the upload fails.
    if let Some(doc_bytes) = sof_doc_bytes {
        let sof_sniffed = if needs_sof_doc {
            mandatory_sof_sniff
        } else {
            let s = storage_svc::sniff_mime(&doc_bytes);
            let ok = matches!(s, Some(m) if storage_svc::mime_matches(&sof_doc_mime, m)
                && storage_svc::validate_kyc_mime(m).is_ok());
            if ok {
                s
            } else {
                tracing::warn!(deposit_id = %deposit_id, "Optional SoF doc rejected: unsupported MIME");
                None
            }
        };
        if let Some(sniffed_mime) = sof_sniffed {
            let sof_ext = storage_svc::extension_for_mime(sniffed_mime);
            let sof_path = format!("deposits/{}/{}-sof.{}", user.id, deposit_id, sof_ext);
            let uploaded = if let Some(ref b) = bucket {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(15),
                    storage_svc::upload_private(b, &sof_path, doc_bytes.clone(), sniffed_mime),
                )
                .await
                {
                    Ok(Ok(p)) => Some(p),
                    _ => storage_svc::upload_local(&sof_path, doc_bytes).await.ok(),
                }
            } else {
                storage_svc::upload_local(&sof_path, doc_bytes).await.ok()
            };
            match uploaded {
                Some(p) => {
                    let persisted = sqlx::query(
                        "UPDATE deposit_requests SET source_of_funds_doc_path = $1 WHERE id = $2",
                    )
                    .bind(&p)
                    .bind(deposit_id)
                    .execute(&state.db)
                    .await
                    .is_ok();
                    if needs_sof_doc && !persisted {
                        tracing::error!(deposit_id = %deposit_id, gcs_path = %p, "SoF doc upload succeeded but DB persist failed; aborting deposit");
                        return release_and_redirect("/wallet?error=sof_doc_save_failed").await;
                    }
                }
                None => {
                    if needs_sof_doc {
                        tracing::error!(deposit_id = %deposit_id, "Mandatory SoF doc upload failed; aborting deposit");
                        return release_and_redirect("/wallet?error=sof_doc_upload_failed").await;
                    }
                }
            }
        } else if needs_sof_doc {
            // Defensive — mandatory_sof_sniff should have caught this.
            return release_and_redirect("/wallet?error=sof_doc_unsupported_format").await;
        }
    }

    let ref_str = provider_ref.unwrap_or_default();

    // Best-effort confirmation email — outbox-backed so a failure here
    // doesn't block the redirect.
    let db_clone = state.db.clone();
    let ref_clone = ref_str.clone();
    let user_id = user.id;
    let amount_display = format_usd(amount_cents);
    tokio::spawn(async move {
        let _ = crate::email::trigger_transactional_email(
            &db_clone,
            &user_id,
            "deposit_submitted",
            serde_json::json!({
                "amount_display": amount_display,
                "reference": ref_clone,
                "processing_hours": 24,
            }),
        )
        .await;
    });

    let success_url = format!(
        "/wallet?deposit_completed=true&ref={}&amount={}",
        ref_str, amount_cents
    );

    if let Some(ref k) = idem_key {
        idempotency::commit_redirect(&state.db, k, &success_url).await;
    }

    crate::metrics::record_deposit(
        crate::metrics::deposit_outcome::SUBMITTED,
        "USD",
        amount_cents,
    );

    Redirect::to(&success_url).into_response()
}

#[derive(Debug, Deserialize)]
pub struct UnfreezeRequestPayload {
    /// Optional user-supplied context (capped at 500 chars after trim).
    #[serde(default)]
    pub note: Option<String>,
}

/// POST /api/wallet/unfreeze-request
///
/// User-facing self-service (P1-8): file a review request when the
/// account has been auto-frozen by the withdrawal-velocity guard. Opens
/// a high-severity row in `compliance_alerts` for the compliance team
/// and stamps `users.unfreeze_requested_at` so the same user can't
/// spam the queue (rate-limited to one open request per 24h).
pub async fn api_request_unfreeze(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<UnfreezeRequestPayload>,
) -> impl IntoResponse {
    let session_token = match jar.get(middleware::SESSION_COOKIE) {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response();
        }
    };
    let user =
        match crate::auth::service::get_user_by_session_allowing_frozen(&state.db, &session_token)
            .await
        {
            Ok(Some(user)) => user,
            Ok(None) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "Unauthorized"})),
                )
                    .into_response();
            }
            Err(e) => {
                tracing::error!("Unfreeze request auth lookup failed: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "request_failed"})),
                )
                    .into_response();
            }
        };

    // M-2 fix: collapse the SELECT-then-INSERT race into a single
    // conditional UPDATE. The UPDATE is the atomic gate — it only
    // succeeds when the user is frozen AND not within the 24h
    // rate-limit window. RETURNING gives us the metadata for the alert
    // body in the same round-trip. Two concurrent requests now serialise
    // on the row lock; the loser sees rows_affected==0 and we map it to
    // the right error after a tiny disambiguating SELECT.
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to start unfreeze request transaction: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "request_failed"})),
            )
                .into_response();
        }
    };

    let claimed: Option<(Option<DateTime<Utc>>, Option<String>)> = sqlx::query_as(
        r#"UPDATE users
              SET unfreeze_requested_at = NOW()
            WHERE id = $1
              AND status = 'frozen'
              AND (unfreeze_requested_at IS NULL
                   OR unfreeze_requested_at < NOW() - INTERVAL '24 hours')
            RETURNING frozen_at, frozen_reason"#,
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();

    let (frozen_at, frozen_reason) = match claimed {
        Some(row) => row,
        None => {
            let _ = tx.rollback().await;
            // Disambiguate: not frozen, missing user, or rate-limited.
            let row: Option<(String, Option<DateTime<Utc>>)> = sqlx::query_as(
                "SELECT status, unfreeze_requested_at FROM users WHERE id = $1",
            )
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten();
            return match row {
                None => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "user_not_found"})),
                )
                    .into_response(),
                Some((status, _)) if status != "frozen" => (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({
                        "error": "not_frozen",
                        "message": "Your account is not currently frozen.",
                    })),
                )
                    .into_response(),
                _ => (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(serde_json::json!({
                        "error": "already_requested",
                        "message": "Review already requested. Compliance will respond within 1 business day.",
                    })),
                )
                    .into_response(),
            };
        }
    };

    let note = payload
        .note
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(500).collect::<String>());

    let frozen_for_hours = frozen_at.map(|f| (Utc::now() - f).num_hours()).unwrap_or(0);
    let summary = format!(
        "User requested unfreeze review (frozen {}h ago, reason: {})",
        frozen_for_hours,
        frozen_reason.unwrap_or_else(|| "unknown".to_string())
    );
    let details = serde_json::json!({
        "frozen_at": frozen_at,
        "frozen_for_hours": frozen_for_hours,
        "user_note": note,
    });

    if let Err(e) = sqlx::query(
        r#"INSERT INTO compliance_alerts (user_id, kind, severity, summary, details)
           VALUES ($1, 'manual_review', 'high', $2, $3)"#,
    )
    .bind(user.id)
    .bind(&summary)
    .bind(&details)
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Failed to insert unfreeze compliance alert: {}", e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "request_failed"})),
        )
            .into_response();
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit unfreeze request transaction: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "request_failed"})),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "status": "requested",
        "message": "Compliance review filed. We'll reply within 1 business day.",
    }))
    .into_response()
}

/// POST /api/wallet/withdrawals/:id/cancel
///
/// User-initiated cancellation of a pending withdrawal (P1-4). Allowed
/// only while the request is still `pending` — once admin transitions
/// it to `approved`/`rejected` the funds are downstream and the user
/// must contact support.
///
/// Refunds amount + fee atomically to the cash wallet, transitions the
/// request to `cancelled`, and marks the ledger row as `cancelled`.
pub async fn api_cancel_withdrawal(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(req_id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response();
        }
    };

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Cancel TX begin failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "cancel_failed"})),
            )
                .into_response();
        }
    };

    let row: Option<(Uuid, String, i64, i64, String)> = sqlx::query_as(
        "SELECT user_id, status, amount_cents, fee_cents, currency
           FROM withdrawal_requests WHERE id = $1 FOR UPDATE",
    )
    .bind(req_id)
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten();

    let (owner, status, amount_cents, fee_cents, currency) = match row {
        Some(r) => r,
        None => {
            let _ = tx.rollback().await;
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "not_found"})),
            )
                .into_response();
        }
    };

    if owner != user.id {
        let _ = tx.rollback().await;
        // Don't leak ownership info — return 404 like the not-found case.
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not_found"})),
        )
            .into_response();
    }

    if status != "pending" {
        let _ = tx.rollback().await;
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "not_cancellable",
                "message": format!("Withdrawal is in '{}' state and can no longer be cancelled.", status),
                "status": status,
            })),
        )
            .into_response();
    }

    let refund = amount_cents.saturating_add(fee_cents);

    if let Err(e) = sqlx::query(
        "UPDATE wallets SET balance_cents = balance_cents + $1
           WHERE user_id = $2 AND wallet_type = 'cash' AND currency = $3",
    )
    .bind(refund)
    .bind(user.id)
    .bind(&currency)
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Cancel refund failed for req {}: {}", req_id, e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "cancel_failed"})),
        )
            .into_response();
    }

    if let Err(e) = sqlx::query("UPDATE withdrawal_requests SET status = 'cancelled' WHERE id = $1")
        .bind(req_id)
        .execute(&mut *tx)
        .await
    {
        tracing::error!("Cancel status update failed for req {}: {}", req_id, e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "cancel_failed"})),
        )
            .into_response();
    }

    let _ = sqlx::query(
        "UPDATE wallet_transactions SET status = 'cancelled'
           WHERE external_ref_id = $1 AND type = 'withdrawal'",
    )
    .bind(req_id.to_string())
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        tracing::error!("Cancel commit failed for req {}: {}", req_id, e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "cancel_failed"})),
        )
            .into_response();
    }

    crate::metrics::record_withdrawal(crate::metrics::withdraw_outcome::CANCELLED, amount_cents);

    Json(serde_json::json!({
        "status": "cancelled",
        "refunded_cents": refund,
    }))
    .into_response()
}

/// POST /wallet/withdraw
pub async fn handle_withdraw(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<WithdrawForm>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // ── Idempotency check ───────────────────────────────────────
    // Critical for withdrawals: a double-submit could deduct twice and
    // create two pending withdrawal_requests rows.
    let idem_key =
        match idempotency::try_reserve(&state.db, &headers, user.id, "/wallet/withdraw", "POST")
            .await
        {
            Reservation::NoKey => None,
            Reservation::Reserved(k) => Some(k),
            Reservation::CachedRedirect { location } => {
                return Redirect::to(&location).into_response()
            }
            Reservation::CachedJson { .. } => {
                return Redirect::to("/wallet?withdraw_requested=true").into_response();
            }
            Reservation::InProgress => {
                return Redirect::to("/wallet?error=in_progress").into_response();
            }
        };

    let release_and_redirect = |url: &str| {
        let db = state.db.clone();
        let key = idem_key.clone();
        let url = url.to_string();
        async move {
            if let Some(k) = key {
                idempotency::release(&db, &k).await;
            }
            Redirect::to(&url).into_response()
        }
    };

    let amount_cents = parse_dollars_to_cents(&form.amount);

    if amount_cents > 0 {
        // KYC gate — only approved users may withdraw
        let kyc = crate::kyc::service::get_kyc_status(&state.db, user.id).await;
        let kyc_ok = matches!(&kyc, Ok(r) if matches!(r.status.as_str(), "approved" | "verified" | "completed"));
        if !kyc_ok {
            crate::metrics::record_withdrawal(
                crate::metrics::withdraw_outcome::BLOCKED_SAFETY,
                amount_cents,
            );
            return release_and_redirect("/wallet?error=kyc_required").await;
        }

        // ─── P1-5: Apply withdrawal fee ──────────────────────────────
        // Fee is stored on the row so a tariff change between submission
        // and approval doesn't retroactively reprice the request.
        let wallet_settings =
            crate::payments::service::fetch_deposit_bank_settings(&state.db).await;
        let fee_cents = wallet_settings.withdrawal_fee_cents.max(0);
        let total_debit = amount_cents.saturating_add(fee_cents);

        // ─── 18.6–18.9: Withdrawal safety controls ──────────────────
        // Daily cap, velocity freeze, new-account cooldown, and step-up
        // 2FA all run here BEFORE we touch the wallet so a blocked
        // withdrawal never momentarily debits the balance.
        if let Err(safety) = super::safety::check_withdrawal_safety(
            &state.db,
            state.redis.as_ref(),
            user.id,
            amount_cents,
        )
        .await
        {
            crate::metrics::record_withdrawal(
                crate::metrics::withdraw_outcome::BLOCKED_SAFETY,
                amount_cents,
            );
            return release_and_redirect(&format!("/wallet?error={}", safety.query_param())).await;
        }

        // Use a transaction with FOR UPDATE lock to prevent TOCTOU double-spend race
        let mut tx = match state.db.begin().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Withdraw TX begin failed: {}", e);
                return release_and_redirect("/wallet?error=withdraw_failed").await;
            }
        };

        // Lock the wallet row and check AVAILABLE balance atomically.
        // Available = balance_cents - held_balance_cents. Held funds back
        // open buy orders / pending settlements and must NOT be withdrawable.
        let wallet_row = sqlx::query_as::<_, (Uuid, i64, i64)>(
            "SELECT id, balance_cents, held_balance_cents
             FROM wallets
             WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'
             FOR UPDATE",
        )
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await;

        let (wallet_id, current_balance, held_balance) = match wallet_row {
            Ok(Some(row)) => row,
            Ok(None) => {
                let _ = tx.rollback().await;
                tracing::warn!("No wallet found for user {}", user.id);
                crate::metrics::record_withdrawal(
                    crate::metrics::withdraw_outcome::BLOCKED_FUNDS,
                    amount_cents,
                );
                return release_and_redirect("/wallet?error=insufficient_funds").await;
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!("Wallet lookup failed: {}", e);
                return release_and_redirect("/wallet?error=withdraw_failed").await;
            }
        };

        let available = current_balance - held_balance;
        if available < total_debit {
            let _ = tx.rollback().await;
            tracing::warn!(
                "Insufficient available funds: user {} balance={} held={} available={} requested={} fee={}",
                user.id,
                current_balance,
                held_balance,
                available,
                amount_cents,
                fee_cents,
            );
            crate::metrics::record_withdrawal(
                crate::metrics::withdraw_outcome::BLOCKED_FUNDS,
                amount_cents,
            );
            return release_and_redirect("/wallet?error=insufficient_funds").await;
        }

        let pm_uuid = if let Some(pm_id) = &form.payment_method_id {
            Uuid::parse_str(pm_id).ok()
        } else {
            None
        };

        // Deduct balance to freeze funds — amount + fee both come out so
        // the user can't keep the fee in float.
        if let Err(e) =
            sqlx::query("UPDATE wallets SET balance_cents = balance_cents - $1 WHERE id = $2")
                .bind(total_debit)
                .bind(wallet_id)
                .execute(&mut *tx)
                .await
        {
            let _ = tx.rollback().await;
            tracing::error!("Failed to freeze balance: {}", e);
            return release_and_redirect("/wallet?error=withdraw_failed").await;
        }

        // Create withdrawal request inside the transaction. fee_cents is
        // frozen on the row so a later tariff change doesn't reprice it.
        let req_id: Result<Uuid, sqlx::Error> = sqlx::query_scalar(
            r#"
            INSERT INTO withdrawal_requests
                (user_id, amount_cents, currency, payment_method_id, status, fee_cents)
            VALUES ($1, $2, 'USD', $3, 'pending', $4)
            RETURNING id
            "#,
        )
        .bind(user.id)
        .bind(amount_cents)
        .bind(pm_uuid)
        .bind(fee_cents)
        .fetch_one(&mut *tx)
        .await;

        match req_id {
            Ok(id) => {
                // Add a pending transaction for UI visibility in the ledger
                let _ = sqlx::query(
                    "INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, external_ref_id) VALUES ($1, 'withdrawal', 'pending', $2, $3)"
                )
                .bind(wallet_id)
                .bind(-amount_cents)
                .bind(id.to_string())
                .execute(&mut *tx)
                .await;

                // Commit the atomic operation
                match tx.commit().await {
                    Ok(_) => {
                        tracing::info!(
                            "Created withdrawal request {} for user {} (amount {})",
                            id,
                            user.id,
                            amount_cents
                        );

                        // Best-effort confirmation email so the user knows the
                        // request is in the admin queue. Lookup is cheap; even
                        // if it returns no row we just send without destination.
                        let db_clone = state.db.clone();
                        let user_id = user.id;
                        let amount_display = format_usd(amount_cents);
                        let pm_uuid_owned = pm_uuid;
                        tokio::spawn(async move {
                            let destination = if let Some(pmid) = pm_uuid_owned {
                                sqlx::query_scalar::<_, Option<String>>(
                                    "SELECT label FROM payment_methods WHERE id = $1",
                                )
                                .bind(pmid)
                                .fetch_optional(&db_clone)
                                .await
                                .ok()
                                .flatten()
                                .flatten()
                                .unwrap_or_else(|| "your bank account".to_string())
                            } else {
                                "your bank account".to_string()
                            };

                            let _ = crate::email::trigger_transactional_email(
                                &db_clone,
                                &user_id,
                                "withdraw_requested",
                                serde_json::json!({
                                    "amount_display": amount_display,
                                    "destination": destination,
                                }),
                            )
                            .await;
                        });

                        let success_url = "/wallet?withdraw_requested=true";
                        if let Some(ref k) = idem_key {
                            idempotency::commit_redirect(&state.db, k, success_url).await;
                        }
                        crate::metrics::record_withdrawal(
                            crate::metrics::withdraw_outcome::REQUESTED,
                            amount_cents,
                        );
                        return Redirect::to(success_url).into_response();
                    }
                    Err(e) => {
                        tracing::error!("Withdraw TX commit failed: {}", e);
                        return Redirect::to("/wallet?error=withdraw_failed").into_response();
                    }
                }
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    "Failed to create withdrawal request for user {}: {}",
                    user.id,
                    e
                );
                return release_and_redirect("/wallet?error=withdraw_failed").await;
            }
        }
    }

    if let Some(ref k) = idem_key {
        idempotency::release(&state.db, k).await;
    }
    Redirect::to("/wallet").into_response()
}

// ─── Page Handler ───────────────────────────────────────────────

/// GET /wallet – renders the wallet page with real data from the database.
///
/// This handler uses a proper MiniJinja context to pass all data to the template,
/// replacing the previous fragile string-replacement approach.
pub async fn page_wallet(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let _ = ensure_wallets(&state.db, user.id).await;

    // ── Fetch all data in parallel-ish fashion ──
    let (cash_balance, rewards_balance, asset_balance) = fetch_balances(&state.db, user.id).await;
    let transactions = fetch_transactions(&state.db, user.id, 10).await;

    // Payment methods (single query, no duplicates)
    let pms = payment_methods::service::list_user_payment_methods(&state.db, &user.id, None)
        .await
        .unwrap_or_default();

    let (
        cards_html,
        banks_html,
        mobile_cards_html,
        mobile_banks_html,
        options_html,
        has_cards,
        has_banks,
    ) = build_payment_method_html(&pms);

    let stripe_pk =
        std::env::var("STRIPE_PUBLISHABLE_KEY").unwrap_or_else(|_| "pk_test_MISSING".to_string());

    // ── Build the context ──
    let ctx = WalletPageContext {
        cash_balance: format_usd(cash_balance),
        rewards_balance: format_usd(rewards_balance),
        asset_balance: format_usd(asset_balance),
        cash_cents: cash_balance,
        rewards_cents: rewards_balance,
        asset_cents: asset_balance,
        has_transactions: !transactions.is_empty(),
        transactions,
        cards_html,
        banks_html,
        mobile_cards_html,
        mobile_banks_html,
        has_cards,
        has_banks,
        payment_method_options: options_html,
        stripe_publishable_key: stripe_pk,
    };

    // ── Render with MiniJinja ──
    let template = match state.templates.get_template("wallet.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load wallet.html template: {}", e);
            return Html("<h1>Page not found</h1>".to_string()).into_response();
        }
    };

    let user_display_name = user
        .email
        .split('@')
        .next()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("User")
        .to_string();

    let html = match template.render(minijinja::context! {
        user => user,
        user_display_name => user_display_name,
        is_developer => false,
        cash_balance => ctx.cash_balance,
        rewards_balance => ctx.rewards_balance,
        asset_balance => ctx.asset_balance,
        cash_cents => ctx.cash_cents,
        rewards_cents => ctx.rewards_cents,
        asset_cents => ctx.asset_cents,
        transactions => ctx.transactions,
        has_transactions => ctx.has_transactions,
        cards_html => ctx.cards_html,
        banks_html => ctx.banks_html,
        mobile_cards_html => ctx.mobile_cards_html,
        mobile_banks_html => ctx.mobile_banks_html,
        has_cards => ctx.has_cards,
        has_banks => ctx.has_banks,
        payment_method_options => ctx.payment_method_options,
        stripe_publishable_key => ctx.stripe_publishable_key,
    }) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to render wallet.html: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };

    Html(html).into_response()
}

// ─── JSON API Endpoints ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StepUpVerifyPayload {
    /// 6-digit TOTP code from the user's authenticator app.
    pub code: String,
    /// Which financial action the session should unlock. Only "withdrawal"
    /// is wired to the wallet UI today; the enum leaves room for trade /
    /// payment-method / password-change without an API rev.
    #[serde(default = "default_step_up_action")]
    pub action: String,
}

fn default_step_up_action() -> String {
    "withdrawal".to_string()
}

/// POST /api/wallet/step-up/verify
///
/// Verifies a TOTP code and opens a 15-minute "trading session" in Redis
/// (`auth::step_up::create_trading_session`). After success, subsequent
/// withdraw submissions of the same user pass `check_withdrawal_safety`
/// without re-prompting.
///
/// Returns 200 `{status: "verified"}` on success; 401 on bad code; 400
/// when TOTP is not enrolled. Failures never leak the secret.
pub async fn api_step_up_verify(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<StepUpVerifyPayload>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response();
        }
    };

    let action = match payload.action.as_str() {
        "withdrawal" => crate::auth::step_up::FinancialAction::Withdrawal,
        "trade" => crate::auth::step_up::FinancialAction::Trade,
        "payment_method" => crate::auth::step_up::FinancialAction::PaymentMethodAdd,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Unsupported action"})),
            )
                .into_response();
        }
    };

    let code = payload.code.trim();
    if code.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Authentication code required"})),
        )
            .into_response();
    }

    match crate::auth::step_up::verify_and_create_trading_session(
        &state.db,
        state.redis.as_ref(),
        user.id,
        code,
        action,
    )
    .await
    {
        Ok(()) => Json(serde_json::json!({
            "status": "verified",
            "valid_for_seconds": 900,
        }))
        .into_response(),
        Err(crate::error::AppError::Unauthorized(msg)) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid_code", "message": msg})),
        )
            .into_response(),
        Err(crate::error::AppError::BadRequest(msg)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "totp_not_enrolled", "message": msg})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("step_up_verify failed for user {}: {}", user.id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "verification_failed"})),
            )
                .into_response()
        }
    }
}

/// GET /api/wallet/deposit-settings – bank-wire details + limits used by the deposit modal.
///
/// Sourced from `platform_settings` so admin can change without redeploy.
/// Requires auth: prevents anonymous reconnaissance of corporate wire details.
pub async fn api_deposit_settings(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if middleware::get_current_user(&jar, &state.db)
        .await
        .is_none()
    {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        )
            .into_response();
    }

    let bank = crate::payments::service::fetch_deposit_bank_settings(&state.db).await;
    Json(bank).into_response()
}

/// GET /api/wallet/balance – returns the authenticated user's wallet balances as JSON.
pub async fn api_wallet_balance(
    jar: CookieJar,
    State(state): State<AppState>,
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

    let _ = ensure_wallets(&state.db, user.id).await;
    let (cash_cents, rewards_cents, asset_cents) = fetch_balances(&state.db, user.id).await;

    Json(WalletBalanceResponse {
        cash_display: format_usd(cash_cents),
        rewards_display: format_usd(rewards_cents),
        asset_display: format_usd(asset_cents),
        cash_cents,
        rewards_cents,
        asset_cents,
    })
    .into_response()
}

/// GET /api/wallet/transactions – returns paginated recent wallet transactions as JSON.
/// Query params for the wallet-transactions CSV export.
#[derive(Debug, Deserialize)]
pub struct TransactionExportQuery {
    /// Optional ISO-8601 lower bound, e.g. `2026-01-01T00:00:00Z`.
    pub from: Option<String>,
    /// Optional ISO-8601 upper bound (exclusive). Defaults to NOW().
    pub to: Option<String>,
}

fn parse_export_bound(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn csv_escape(field: &str) -> String {
    // RFC 4180: wrap in double-quotes if the value contains comma,
    // quote, or newline; escape inner quotes by doubling them.
    //
    // OWASP CSV-injection defence (M-1): prefix a leading `=`, `+`, `-`,
    // `@`, TAB or CR with a single quote so spreadsheets don't evaluate
    // it as a formula. The prefix is dropped automatically when the
    // file is parsed back into a column — it's only there to defang the
    // formula trigger for Excel/Sheets/Numbers.
    let starts_with_formula = field
        .chars()
        .next()
        .map(|c| matches!(c, '=' | '+' | '-' | '@' | '\t' | '\r'))
        .unwrap_or(false);
    let defanged = if starts_with_formula {
        let mut s = String::with_capacity(field.len() + 1);
        s.push('\'');
        s.push_str(field);
        s
    } else {
        field.to_string()
    };

    let needs_quote = defanged.contains([',', '"', '\n', '\r']);
    if needs_quote {
        let escaped = defanged.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        defanged
    }
}

/// GET /api/wallet/transactions/export
///
/// Streams the caller's wallet transactions as RFC-4180 CSV. Designed
/// for bookkeeping / tax-prep workflows — columns are stable and the
/// `Content-Disposition` header triggers a download in the browser.
///
/// Query params (both optional):
///   from — ISO-8601 lower bound (`>=`)
///   to   — ISO-8601 upper bound (`<`); defaults to NOW().
///
/// The export is bounded to 10 000 rows so a runaway query can't blow
/// memory. Users with denser histories should narrow the date range.
pub async fn api_wallet_transactions_export(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(params): Query<TransactionExportQuery>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/json".to_string(),
                )],
                serde_json::json!({"error": "Unauthorized"}).to_string(),
            )
                .into_response();
        }
    };

    let from = params
        .from
        .as_deref()
        .and_then(parse_export_bound)
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(365 * 5));
    let to = params
        .to
        .as_deref()
        .and_then(parse_export_bound)
        .unwrap_or_else(Utc::now);

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            i64,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT t.id, t.type, t.status, t.amount_cents, w.wallet_type,
               t.created_at, t.completed_at, w.currency,
               t.description, t.external_ref_id
          FROM wallet_transactions t
          JOIN wallets w ON w.id = t.wallet_id
         WHERE w.user_id = $1
           AND t.created_at >= $2
           AND t.created_at <  $3
         ORDER BY t.created_at DESC
         LIMIT 10000
        "#,
    )
    .bind(user.id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut body = String::with_capacity(rows.len() * 128 + 256);
    body.push_str(
        "transaction_id,created_at,completed_at,type,status,wallet_type,currency,amount_cents,amount_display,description,reference\n",
    );
    for (
        id,
        tx_type,
        status,
        amount,
        wallet_type,
        created_at,
        completed_at,
        currency,
        description,
        reference,
    ) in rows
    {
        let abs = amount.unsigned_abs();
        let sign = if amount < 0 { "-" } else { "" };
        let display = format!("{}{}.{:02}", sign, abs / 100, abs % 100);
        body.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            id,
            created_at.to_rfc3339(),
            completed_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
            csv_escape(&tx_type),
            csv_escape(&status),
            csv_escape(&wallet_type),
            csv_escape(currency.as_deref().unwrap_or("USD")),
            amount,
            csv_escape(&display),
            csv_escape(description.as_deref().unwrap_or("")),
            csv_escape(reference.as_deref().unwrap_or("")),
        ));
    }

    let filename = format!(
        "poool-transactions-{}-to-{}.csv",
        from.format("%Y%m%d"),
        to.format("%Y%m%d"),
    );

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/csv; charset=utf-8".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        body,
    )
        .into_response()
}

pub async fn api_wallet_transactions(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(pagination): Query<TransactionPagination>,
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

    let page = pagination.page.unwrap_or(1).max(1);
    let page_size = pagination.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    // Total count for pagination
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM wallet_transactions t
        JOIN wallets w ON w.id = t.wallet_id
        WHERE w.user_id = $1
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let rows = sqlx::query_as::<_, (Uuid, String, String, i64, String, DateTime<Utc>)>(
        r#"
        SELECT t.id, t.type, t.status, t.amount_cents, w.wallet_type, t.created_at
        FROM wallet_transactions t
        JOIN wallets w ON w.id = t.wallet_id
        WHERE w.user_id = $1
        ORDER BY t.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user.id)
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let transactions: Vec<WalletTransactionApiEntry> = rows
        .iter()
        .map(
            |(id, tx_type, status, amount, wallet_type, created_at)| WalletTransactionApiEntry {
                id: *id,
                tx_type: tx_type.clone(),
                status: status.clone(),
                amount_cents: *amount,
                amount_display: {
                    let abs = amount.unsigned_abs();
                    let sign = if *amount < 0 { "-" } else { "" };
                    format!("{}${}.{:02}", sign, abs / 100, abs % 100)
                },
                wallet_type: wallet_type.clone(),
                created_at: created_at.to_rfc3339(),
            },
        )
        .collect();

    let count = transactions.len();
    Json(WalletTransactionsResponse {
        transactions,
        count,
        total: total as usize,
        page,
        page_size,
    })
    .into_response()
}

// ─── Transaction Detail ─────────────────────────────────────────

/// Raw wallet_transactions row joined with the parent wallet for ownership check.
#[derive(Debug)]
struct TxRow {
    id: Uuid,
    wallet_id: Uuid,
    wallet_type: String,
    user_id: Uuid,
    tx_type: String,
    status: String,
    amount_cents: i64,
    description: Option<String>,
    external_ref_id: Option<String>,
    related_order_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

async fn fetch_owned_transaction(pool: &sqlx::PgPool, user_id: Uuid, tx_id: Uuid) -> Option<TxRow> {
    sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            String,
            Uuid,
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<Uuid>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        ),
    >(
        r#"
        SELECT t.id, t.wallet_id, w.wallet_type, w.user_id, t.type, t.status,
               t.amount_cents, t.description, t.external_ref_id, t.related_order_id,
               t.created_at, t.completed_at
        FROM wallet_transactions t
        JOIN wallets w ON w.id = t.wallet_id
        WHERE t.id = $1 AND w.user_id = $2
        "#,
    )
    .bind(tx_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(
        |(
            id,
            wallet_id,
            wallet_type,
            user_id,
            tx_type,
            status,
            amount_cents,
            description,
            external_ref_id,
            related_order_id,
            created_at,
            completed_at,
        )| TxRow {
            id,
            wallet_id,
            wallet_type,
            user_id,
            tx_type,
            status,
            amount_cents,
            description,
            external_ref_id,
            related_order_id,
            created_at,
            completed_at,
        },
    )
}

fn detail_row(label: &str, value: &str) -> DetailRow {
    DetailRow {
        label: label.to_string(),
        value: value.to_string(),
        mono: false,
        copyable: false,
    }
}

fn mono_row(label: &str, value: &str, copyable: bool) -> DetailRow {
    DetailRow {
        label: label.to_string(),
        value: value.to_string(),
        mono: true,
        copyable,
    }
}

/// Format an UTC timestamp as "08 February 2026 at 14:32 UTC".
fn fmt_full_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%d %B %Y at %H:%M UTC").to_string()
}

async fn build_detail_context(pool: &sqlx::PgPool, row: &TxRow) -> TransactionDetailContext {
    let tx_type = TransactionType::from_db(&row.tx_type);
    let status = TransactionStatus::from_db(&row.status);
    let wallet_type = WalletType::from_db(&row.wallet_type);

    let abs_cents = row.amount_cents.unsigned_abs() as i64;
    let positive = row.amount_cents >= 0;
    let amount_display = format_usd(abs_cents);
    let amount_prefix = if positive {
        "+".to_string()
    } else {
        "-".to_string()
    };
    let amount_css = if positive {
        "amount-positive".to_string()
    } else {
        "amount-negative".to_string()
    };

    // ── Overview section ─────────────────────────────────────────
    let mut overview = DetailSection {
        title: "Overview".to_string(),
        rows: vec![
            mono_row("Transaction ID", &row.id.to_string(), true),
            detail_row("Created", &fmt_full_datetime(&row.created_at)),
        ],
    };
    if let Some(c) = &row.completed_at {
        overview
            .rows
            .push(detail_row("Completed", &fmt_full_datetime(c)));
    }
    overview
        .rows
        .push(detail_row("Wallet", wallet_type.display_label()));
    overview
        .rows
        .push(detail_row("Type", tx_type.display_label()));
    overview
        .rows
        .push(detail_row("Status", status.display_label()));

    let mut sections = vec![overview];

    // ── Type-specific section ────────────────────────────────────
    let mut wire_reference = String::new();
    let mut show_wire_instructions = false;

    match tx_type {
        TransactionType::Deposit => {
            let mut rows: Vec<DetailRow> = Vec::new();
            // Look up the matching deposit_request via provider_reference.
            if let Some(ref_id) = &row.external_ref_id {
                wire_reference = ref_id.clone();
                rows.push(mono_row("Reference", ref_id, true));

                let dep = sqlx::query_as::<
                    _,
                    (
                        String,
                        String,
                        String,
                        i64,
                        Option<String>,
                        Option<DateTime<Utc>>,
                        Option<DateTime<Utc>>,
                    ),
                >(
                    r#"
                    SELECT provider, status, currency, amount_cents, payment_method,
                           expires_at, paid_at
                    FROM deposit_requests
                    WHERE provider_reference = $1 AND user_id = $2
                    LIMIT 1
                    "#,
                )
                .bind(ref_id)
                .bind(row.user_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();

                if let Some((provider, dr_status, currency, dr_amount, pm, expires, paid)) = dep {
                    rows.push(detail_row("Provider", &provider));
                    rows.push(detail_row("Request status", &dr_status));
                    if let Some(method) = pm {
                        rows.push(detail_row("Method", &method));
                    }
                    rows.push(detail_row(
                        "Requested amount",
                        &format!("{} {}", currency, format_usd(dr_amount).replace("USD ", "")),
                    ));
                    if let Some(p) = paid {
                        rows.push(detail_row("Paid at", &fmt_full_datetime(&p)));
                    }
                    if let Some(e) = expires {
                        rows.push(detail_row("Expires", &fmt_full_datetime(&e)));
                    }
                    show_wire_instructions = matches!(dr_status.as_str(), "pending");
                } else {
                    // Fallback: still show wire instructions for unfunded deposit
                    show_wire_instructions =
                        matches!(row.status.as_str(), "pending" | "processing");
                }
            }
            if !rows.is_empty() {
                sections.push(DetailSection {
                    title: "Deposit details".to_string(),
                    rows,
                });
            }
        }
        TransactionType::Withdrawal => {
            let mut rows: Vec<DetailRow> = Vec::new();
            if let Some(ref_id) = &row.external_ref_id {
                rows.push(mono_row("Request ID", ref_id, true));

                if let Ok(req_uuid) = Uuid::parse_str(ref_id) {
                    let wd = sqlx::query_as::<
                        _,
                        (
                            String,
                            String,
                            i64,
                            Option<Uuid>,
                            Option<String>,
                            Option<DateTime<Utc>>,
                        ),
                    >(
                        r#"
                        SELECT status, currency, amount_cents, payment_method_id, admin_notes, approved_at
                        FROM withdrawal_requests
                        WHERE id = $1 AND user_id = $2
                        LIMIT 1
                        "#,
                    )
                    .bind(req_uuid)
                    .bind(row.user_id)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();

                    if let Some((wd_status, currency, wd_amount, pm_id, notes, approved)) = wd {
                        rows.push(detail_row("Request status", &wd_status));
                        rows.push(detail_row(
                            "Requested amount",
                            &format!("{} {}", currency, format_usd(wd_amount).replace("USD ", "")),
                        ));
                        if let Some(pid) = pm_id {
                            // Look up payment method label for the destination
                            if let Ok(Some(pm)) = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, String)>(
                                "SELECT label, brand, last_four, method_type FROM payment_methods WHERE id = $1 AND user_id = $2"
                            )
                            .bind(pid)
                            .bind(row.user_id)
                            .fetch_optional(pool)
                            .await
                            {
                                let (label, brand, last4, method_type) = pm;
                                let dest = label.unwrap_or_else(|| {
                                    if method_type == "bank" {
                                        brand.unwrap_or_else(|| "Bank Account".to_string())
                                    } else {
                                        format!(
                                            "{} ending in {}",
                                            brand.unwrap_or_else(|| "Card".into()),
                                            last4.unwrap_or_else(|| "****".into())
                                        )
                                    }
                                });
                                rows.push(detail_row("Destination", &dest));
                            }
                        }
                        if let Some(a) = approved {
                            rows.push(detail_row("Approved at", &fmt_full_datetime(&a)));
                        }
                        if let Some(n) = notes {
                            if !n.trim().is_empty() {
                                rows.push(detail_row("Admin notes", &n));
                            }
                        }
                    }
                }
            }
            if !rows.is_empty() {
                sections.push(DetailSection {
                    title: "Withdrawal request".to_string(),
                    rows,
                });
            }
        }
        TransactionType::Purchase | TransactionType::Sale => {
            if let Some(order_id) = row.related_order_id {
                sections.push(DetailSection {
                    title: "Order".to_string(),
                    rows: vec![mono_row("Order ID", &order_id.to_string(), true)],
                });
            }
        }
        _ => {
            if let Some(ref_id) = &row.external_ref_id {
                sections.push(DetailSection {
                    title: "Reference".to_string(),
                    rows: vec![mono_row("External reference", ref_id, true)],
                });
            }
        }
    }

    let wire_amount_display = format_usd(abs_cents);

    // P1-3: deposit timeline. Drives the visual stepper on the
    // transaction-detail page so the user can see at a glance where
    // their wire is in the pipeline. Only populated for deposits.
    let timeline = if matches!(tx_type, TransactionType::Deposit) {
        build_deposit_timeline(pool, row).await
    } else {
        Vec::new()
    };

    // P1-4: surface the cancellable-withdrawal id only when the request
    // is still in flight. Status comes from withdrawal_requests, not the
    // ledger tx (which can lag during state transitions).
    let cancellable_withdrawal_id = if matches!(tx_type, TransactionType::Withdrawal) {
        match row
            .external_ref_id
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok())
        {
            Some(req_id) => {
                let cur: Option<String> = sqlx::query_scalar(
                    "SELECT status FROM withdrawal_requests WHERE id = $1 AND user_id = $2",
                )
                .bind(req_id)
                .bind(row.user_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
                if cur.as_deref() == Some("pending") {
                    Some(req_id)
                } else {
                    None
                }
            }
            None => None,
        }
    } else {
        None
    };

    TransactionDetailContext {
        id: row.id,
        tx_type_label: tx_type.display_label().to_string(),
        tx_type_icon: tx_type.icon_key().to_string(),
        status_label: status.display_label().to_string(),
        status_css: status.css_class().to_string(),
        wallet_label: wallet_type.display_label().to_string(),
        amount_cents: row.amount_cents,
        amount_display,
        amount_prefix,
        amount_css,
        date_full: fmt_full_datetime(&row.created_at),
        date_iso: row.created_at.to_rfc3339(),
        description: row.description.clone(),
        sections,
        show_wire_instructions,
        wire_reference,
        wire_amount_display,
        cancellable_withdrawal_id,
        timeline,
    }
}

/// Build the 4-step deposit timeline. Reads the matching
/// `deposit_requests` row to figure out which states are done vs
/// pending, and stamps each completed step with its timestamp.
async fn build_deposit_timeline(pool: &sqlx::PgPool, row: &TxRow) -> Vec<TimelineStep> {
    let ref_id = match &row.external_ref_id {
        Some(r) => r.clone(),
        None => return Vec::new(),
    };

    let dep: Option<(
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        r#"
        SELECT status, created_at, proof_uploaded_at, paid_at, expires_at
          FROM deposit_requests
         WHERE provider_reference = $1 AND user_id = $2
         LIMIT 1
        "#,
    )
    .bind(&ref_id)
    .bind(row.user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some((status, created_at, proof_at, paid_at, expires_at)) = dep else {
        return Vec::new();
    };

    let terminal_bad = matches!(status.as_str(), "failed" | "cancelled" | "expired");

    let fmt_short =
        |dt: chrono::DateTime<chrono::Utc>| dt.format("%d %b %Y · %H:%M UTC").to_string();

    // Step 1 — request created
    let step1 = TimelineStep {
        label: "Submitted".to_string(),
        hint: Some(fmt_short(created_at)),
        state: "done".to_string(),
    };

    // Step 2 — proof uploaded
    let step2 = if let Some(p) = proof_at {
        TimelineStep {
            label: "Proof uploaded".to_string(),
            hint: Some(fmt_short(p)),
            state: "done".to_string(),
        }
    } else if terminal_bad {
        TimelineStep {
            label: "Proof uploaded".to_string(),
            hint: Some("Skipped".to_string()),
            state: "skipped".to_string(),
        }
    } else {
        TimelineStep {
            label: "Proof uploaded".to_string(),
            hint: Some("Waiting for your proof of transfer".to_string()),
            state: "current".to_string(),
        }
    };

    // Step 3 — admin verification
    let step3 = if paid_at.is_some() {
        TimelineStep {
            label: "Verified by compliance".to_string(),
            hint: None,
            state: "done".to_string(),
        }
    } else if terminal_bad {
        TimelineStep {
            label: "Verified by compliance".to_string(),
            hint: Some(match status.as_str() {
                "expired" => "Expired without verification".to_string(),
                "cancelled" => "Cancelled".to_string(),
                _ => "Failed".to_string(),
            }),
            state: "skipped".to_string(),
        }
    } else if proof_at.is_some() {
        TimelineStep {
            label: "Verified by compliance".to_string(),
            hint: Some("Usually within 24 hours".to_string()),
            state: "current".to_string(),
        }
    } else {
        TimelineStep {
            label: "Verified by compliance".to_string(),
            hint: Some(match expires_at {
                Some(e) => format!("By {}", fmt_short(e)),
                None => "Awaiting proof".to_string(),
            }),
            state: "pending".to_string(),
        }
    };

    // Step 4 — credited to wallet
    let step4 = match (paid_at, terminal_bad) {
        (Some(p), _) => TimelineStep {
            label: "Credited to wallet".to_string(),
            hint: Some(fmt_short(p)),
            state: "done".to_string(),
        },
        (None, true) => TimelineStep {
            label: "Credited to wallet".to_string(),
            hint: Some("Skipped".to_string()),
            state: "skipped".to_string(),
        },
        (None, false) => TimelineStep {
            label: "Credited to wallet".to_string(),
            hint: Some("Pending verification".to_string()),
            state: "pending".to_string(),
        },
    };

    vec![step1, step2, step3, step4]
}

/// GET /transactions/:id – render the transaction detail page.
/// GET /wallet/statements/:year/:month
///
/// Renders the user's monthly statement as a print-styled HTML page
/// (P1-2). User saves to PDF via the browser's "Print → Save as PDF"
/// dialog — keeps the implementation free of native PDF deps.
///
/// Includes: opening + closing balance for the period, the period's
/// transactions with running balance, totals by type, the platform's
/// footer (registered entity, support email).
pub async fn page_monthly_statement(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((year, month)): Path<(i32, u32)>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // Reject out-of-range dates early so we don't query NaT.
    if !(2020..=2100).contains(&year) || !(1..=12).contains(&month) {
        return (
            StatusCode::BAD_REQUEST,
            Html("<h1>Invalid statement period</h1>".to_string()),
        )
            .into_response();
    }

    let period_start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| chrono::DateTime::<Utc>::from_naive_utc_and_offset(d, Utc))
        .unwrap_or_else(Utc::now);
    let next_month_first = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.and_hms_opt(0, 0, 0))
    .map(|d| chrono::DateTime::<Utc>::from_naive_utc_and_offset(d, Utc))
    .unwrap_or_else(Utc::now);

    // Closing balance is just the current balance for an open month;
    // for historical months we replay the ledger up to period_end.
    let opening_balance_cents: i64 = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT COALESCE(SUM(t.amount_cents), 0)::bigint
          FROM wallet_transactions t
          JOIN wallets w ON w.id = t.wallet_id
         WHERE w.user_id = $1
           AND w.wallet_type = 'cash'
           AND t.status = 'completed'
           AND t.created_at < $2
        "#,
    )
    .bind(user.id)
    .bind(period_start)
    .fetch_one(&state.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let rows = sqlx::query_as::<_, (Uuid, String, String, i64, DateTime<Utc>, Option<String>)>(
        r#"
        SELECT t.id, t.type, t.status, t.amount_cents, t.created_at, t.description
          FROM wallet_transactions t
          JOIN wallets w ON w.id = t.wallet_id
         WHERE w.user_id = $1
           AND w.wallet_type = 'cash'
           AND t.created_at >= $2
           AND t.created_at <  $3
         ORDER BY t.created_at ASC
        "#,
    )
    .bind(user.id)
    .bind(period_start)
    .bind(next_month_first)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut running = opening_balance_cents;
    let mut total_in = 0i64;
    let mut total_out = 0i64;
    let mut tx_rows: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
    for (id, tx_type, status, amount, created_at, description) in &rows {
        running += amount;
        if *amount >= 0 {
            total_in += amount;
        } else {
            total_out += amount.abs();
        }
        tx_rows.push(serde_json::json!({
            "id": id.to_string(),
            "date": created_at.format("%d %b %Y").to_string(),
            "type": tx_type,
            "status": status,
            "amount_display": format_usd(*amount),
            "amount_class": if *amount >= 0 { "pos" } else { "neg" },
            "balance_display": format_usd(running),
            "description": description.clone().unwrap_or_default(),
        }));
    }

    let template = match state.templates.get_template("wallet/statement.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load wallet/statement.html: {}", e);
            return Html("<h1>Statement template missing</h1>".to_string()).into_response();
        }
    };

    let month_label = period_start.format("%B %Y").to_string();
    let html = match template.render(minijinja::context! {
        user => user,
        period_label => month_label,
        period_start => period_start.format("%d %b %Y").to_string(),
        period_end => (next_month_first - chrono::Duration::seconds(1))
            .format("%d %b %Y").to_string(),
        opening_balance => format_usd(opening_balance_cents),
        closing_balance => format_usd(running),
        total_in_display => format_usd(total_in),
        total_out_display => format_usd(total_out),
        transactions => tx_rows,
        has_transactions => !rows.is_empty(),
        generated_at => Utc::now().format("%d %B %Y at %H:%M UTC").to_string(),
    }) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to render statement: {}", e);
            return Html("<h1>Internal error</h1>".to_string()).into_response();
        }
    };

    Html(html).into_response()
}

pub async fn page_transaction_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(tx_id): Path<Uuid>,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let row = match fetch_owned_transaction(&state.db, user.id, tx_id).await {
        Some(r) if r.user_id == user.id => r,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h1>Transaction not found</h1>".to_string()),
            )
                .into_response()
        }
    };

    let ctx = build_detail_context(&state.db, &row).await;

    let template = match state.templates.get_template("transaction-detail.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load transaction-detail.html template: {}", e);
            return Html("<h1>Page not found</h1>".to_string()).into_response();
        }
    };

    let user_display_name = user
        .email
        .split('@')
        .next()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or("User")
        .to_string();

    let html = match template.render(minijinja::context! {
        user => user,
        user_display_name => user_display_name,
        is_developer => false,
        tx => ctx,
    }) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to render transaction-detail.html: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };

    Html(html).into_response()
}

/// GET /api/wallet/transactions/:id – JSON view of a single transaction.
pub async fn api_transaction_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(tx_id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let row = match fetch_owned_transaction(&state.db, user.id, tx_id).await {
        Some(r) if r.user_id == user.id => r,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Not found"})),
            )
                .into_response()
        }
    };

    let ctx = build_detail_context(&state.db, &row).await;
    Json(ctx).into_response()
}
