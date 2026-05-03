/// Wallet route handlers – view balances and execute deposit/withdraw actions.
use axum::{
    extract::{Form, Query, State},
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
use crate::payment_methods;

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
    let rows = sqlx::query_as::<_, (String, String, DateTime<Utc>, String, i64)>(
        r#"
        SELECT t.type, t.status, t.created_at, w.wallet_type, t.amount_cents
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
        .map(|(idx, (tx_type, status, date, wallet_t, amount))| {
            build_transaction(idx, tx_type, status, date, wallet_t, *amount)
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

/// POST /wallet/deposit
pub async fn handle_deposit(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<DepositForm>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let amount_cents = parse_dollars_to_cents(&form.amount);

    if amount_cents > 0 {
        // We defer to payments service to create the deposit intent
        match crate::payments::service::create_deposit_request(
            &state.db,
            user.id,
            "USD",
            amount_cents,
        )
        .await
        {
            Ok(deposit_res) => {
                let ref_id = deposit_res.provider_reference.unwrap_or_default();
                // Redirect back to wallet with success param to show instructions
                return Redirect::to(&format!(
                    "/wallet?deposit_created=true&ref={}&amount={}",
                    ref_id, amount_cents
                ))
                .into_response();
            }
            Err(e) => {
                tracing::error!(
                    "Failed to create deposit request for user {}: {}",
                    user.id,
                    e
                );
                return Redirect::to("/wallet?error=deposit_failed").into_response();
            }
        }
    }

    Redirect::to("/wallet").into_response()
}

/// POST /wallet/withdraw
pub async fn handle_withdraw(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<WithdrawForm>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let amount_cents = parse_dollars_to_cents(&form.amount);

    if amount_cents > 0 {
        // KYC gate — only approved users may withdraw
        let kyc = crate::kyc::service::get_kyc_status(&state.db, user.id).await;
        let kyc_ok = matches!(&kyc, Ok(r) if matches!(r.status.as_str(), "approved" | "verified" | "completed"));
        if !kyc_ok {
            return Redirect::to("/wallet?error=kyc_required").into_response();
        }

        // Use a transaction with FOR UPDATE lock to prevent TOCTOU double-spend race
        let mut tx = match state.db.begin().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Withdraw TX begin failed: {}", e);
                return Redirect::to("/wallet?error=withdraw_failed").into_response();
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
                return Redirect::to("/wallet?error=insufficient_funds").into_response();
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!("Wallet lookup failed: {}", e);
                return Redirect::to("/wallet?error=withdraw_failed").into_response();
            }
        };

        let available = current_balance - held_balance;
        if available < amount_cents {
            let _ = tx.rollback().await;
            tracing::warn!(
                "Insufficient available funds: user {} balance={} held={} available={} requested={}",
                user.id,
                current_balance,
                held_balance,
                available,
                amount_cents
            );
            return Redirect::to("/wallet?error=insufficient_funds").into_response();
        }

        let pm_uuid = if let Some(pm_id) = &form.payment_method_id {
            Uuid::parse_str(pm_id).ok()
        } else {
            None
        };

        // Deduct balance to freeze funds
        if let Err(e) =
            sqlx::query("UPDATE wallets SET balance_cents = balance_cents - $1 WHERE id = $2")
                .bind(amount_cents)
                .bind(wallet_id)
                .execute(&mut *tx)
                .await
        {
            let _ = tx.rollback().await;
            tracing::error!("Failed to freeze balance: {}", e);
            return Redirect::to("/wallet?error=withdraw_failed").into_response();
        }

        // Create withdrawal request inside the transaction
        let req_id: Result<Uuid, sqlx::Error> = sqlx::query_scalar(
            r#"
            INSERT INTO withdrawal_requests (user_id, amount_cents, currency, payment_method_id, status)
            VALUES ($1, $2, 'USD', $3, 'pending')
            RETURNING id
            "#
        )
        .bind(user.id)
        .bind(amount_cents)
        .bind(pm_uuid)
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
                        return Redirect::to("/wallet?withdraw_requested=true").into_response();
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
                return Redirect::to("/wallet?error=withdraw_failed").into_response();
            }
        }
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
