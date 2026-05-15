/// Payments service – core business logic for deposits, checkout, and invoicing.
///
/// All financial operations use PostgreSQL transactions with strict row locking
/// to guarantee ACID compliance. No floats for money – everything in cents (BIGINT).
use chrono::{Datelike, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;

// ─── Constants & Config ─────────────────────────────────────────

pub const BANK_DETAILS_USD: &str = r#"{
    "bank": "Deutsche Bank AG",
    "bank_address": "Taunusanlage 12, 60325 Frankfurt am Main, Germany",
    "account_name": "POOOL GmbH",
    "company_address": "Königsallee 61, 40215 Düsseldorf, Germany",
    "iban": "DE89 3704 0044 0532 0130 00",
    "bic_swift": "DEUTDEDB"
}"#;

pub const BANK_DETAILS_IDR: &str = r#"{
    "bank": "Bank Central Asia (BCA)",
    "bank_address": "Menara BCA, Grand Indonesia, Jl. M.H. Thamrin No.1, Jakarta 10310, Indonesia",
    "account_name": "PT POOOL Indonesia",
    "company_address": "Jl. Sudirman Kav 1, Jakarta 10000, Indonesia",
    "account_number": ""
}"#;

pub const TEST_BANK_DETAILS_USD: &str = r#"{
    "bank": "POOOL Sandbox Bank",
    "bank_address": "Test environment only",
    "account_name": "POOOL E2E Test Account",
    "company_address": "Do not transfer funds",
    "iban": "TEST-USD-NO-REAL-TRANSFER",
    "bic_swift": "TESTONLY"
}"#;

pub const TEST_BANK_DETAILS_IDR: &str = r#"{
    "bank": "POOOL Sandbox Bank",
    "bank_address": "Test environment only",
    "account_name": "POOOL E2E Test Account",
    "company_address": "Do not transfer funds",
    "account_number": "TEST-IDR-NO-REAL-TRANSFER"
}"#;

// ─── FX Rate Cache (Phase 1.10 — Decimal-based) ────────────────

use rust_decimal::Decimal;
use tokio::sync::RwLock;

/// Cached FX rate stored as Decimal for exact arithmetic.
static CACHED_IDR_RATE: std::sync::OnceLock<RwLock<Option<(Decimal, u64)>>> =
    std::sync::OnceLock::new();

fn fx_cache() -> &'static RwLock<Option<(Decimal, u64)>> {
    CACHED_IDR_RATE.get_or_init(|| RwLock::new(None))
}

async fn completed_subtotal_for_user_asset(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<i64, String> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(oi.subtotal_cents), 0)::bigint
        FROM orders o
        JOIN order_items oi ON oi.order_id = o.id
        WHERE o.user_id = $1
          AND oi.asset_id = $2
          AND o.status = 'completed'
        "#,
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| format!("Completed investment subtotal lookup failed: {}", e))
}

async fn upsert_active_investment(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    asset_id: Uuid,
    tokens_qty: i32,
    subtotal: i64,
) -> Result<(), String> {
    sqlx::query(
        r#"
        INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status)
        VALUES ($1, $2, $3, $4, $4, 'active')
        ON CONFLICT (user_id, asset_id) DO UPDATE
        SET tokens_owned = investments.tokens_owned + $3,
            purchase_value_cents = investments.purchase_value_cents + $4,
            current_value_cents = investments.current_value_cents + $4,
            status = 'active',
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(asset_id)
    .bind(tokens_qty)
    .bind(subtotal)
    .execute(&mut *conn)
    .await
    .map_err(|e| format!("Investment upsert failed: {}", e))?;

    Ok(())
}

async fn allocate_order_item_investment(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    asset_id: Uuid,
    tokens_qty: i32,
    subtotal: i64,
) -> Result<(), String> {
    let completed_subtotal = completed_subtotal_for_user_asset(conn, user_id, asset_id).await?;
    let expected_with_item = completed_subtotal.saturating_add(subtotal);
    let existing = sqlx::query_as::<_, (i32, i64, String)>(
        r#"
        SELECT tokens_owned, purchase_value_cents, status
        FROM investments
        WHERE user_id = $1 AND asset_id = $2
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| format!("Investment lookup failed: {}", e))?;

    if let Some((_tokens_owned, purchase_value_cents, status)) = existing {
        let pending_allocation_already_reflected =
            purchase_value_cents >= expected_with_item || status == "funding_in_progress";

        if pending_allocation_already_reflected {
            if status == "funding_in_progress" {
                sqlx::query(
                    "UPDATE investments SET status = 'active', updated_at = NOW()
                     WHERE user_id = $1 AND asset_id = $2",
                )
                .bind(user_id)
                .bind(asset_id)
                .execute(&mut *conn)
                .await
                .map_err(|e| format!("Investment status activation failed: {}", e))?;
            }
            return Ok(());
        }
    }

    upsert_active_investment(conn, user_id, asset_id, tokens_qty, subtotal).await
}

async fn revert_pending_order_item_investment(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    asset_id: Uuid,
    tokens_qty: i32,
    subtotal: i64,
) -> Result<(), String> {
    let completed_subtotal = completed_subtotal_for_user_asset(conn, user_id, asset_id).await?;
    let expected_with_pending = completed_subtotal.saturating_add(subtotal);
    let existing = sqlx::query_as::<_, (i32, i64, String)>(
        r#"
        SELECT tokens_owned, purchase_value_cents, status
        FROM investments
        WHERE user_id = $1 AND asset_id = $2
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| format!("Investment lookup failed: {}", e))?;

    let Some((tokens_owned, purchase_value_cents, status)) = existing else {
        return Ok(());
    };

    let pending_allocation_reflected =
        status == "funding_in_progress" || purchase_value_cents >= expected_with_pending;
    if !pending_allocation_reflected {
        return Ok(());
    }

    if tokens_owned <= tokens_qty || purchase_value_cents <= subtotal {
        sqlx::query(
            r#"
            UPDATE investments
            SET tokens_owned = 0,
                purchase_value_cents = 0,
                current_value_cents = 0,
                status = 'failed',
                updated_at = NOW()
            WHERE user_id = $1 AND asset_id = $2
            "#,
        )
        .bind(user_id)
        .bind(asset_id)
        .execute(&mut *conn)
        .await
        .map_err(|e| format!("Failed to clear pending investment: {}", e))?;
    } else {
        sqlx::query(
            r#"
            UPDATE investments
            SET tokens_owned = tokens_owned - $1,
                purchase_value_cents = purchase_value_cents - $2,
                current_value_cents = current_value_cents - $2,
                updated_at = NOW()
            WHERE user_id = $3 AND asset_id = $4
            "#,
        )
        .bind(tokens_qty)
        .bind(subtotal)
        .bind(user_id)
        .bind(asset_id)
        .execute(&mut *conn)
        .await
        .map_err(|e| format!("Failed to revert pending investment allocation: {}", e))?;
    }

    Ok(())
}

/// Default IDR rate as Decimal (fallback when API is unreachable).
fn default_idr_rate() -> Decimal {
    Decimal::from(crate::config::DEFAULT_USD_TO_IDR_RATE_I64)
}

/// Fetch the latest USD to IDR exchange rate as Decimal, cached for 1 hour.
///
/// Phase 1.10: Uses rust_decimal::Decimal instead of f64 to prevent
/// IEEE754 rounding errors in currency conversion.
pub async fn get_usd_to_idr_rate() -> Decimal {
    let now = chrono::Utc::now().timestamp() as u64;

    // Check cache first
    {
        let cache = fx_cache().read().await;
        if let Some((rate, timestamp)) = *cache {
            if now - timestamp < 3600 {
                return rate;
            }
        }
    }

    // Fetch from API
    #[derive(serde::Deserialize)]
    struct FxResponse {
        rates: std::collections::HashMap<String, f64>,
    }

    let client = reqwest::Client::new();
    if let Ok(resp) = client
        .get("https://open.er-api.com/v6/latest/USD")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        if let Ok(data) = resp.json::<FxResponse>().await {
            if let Some(rate_f64) = data.rates.get("IDR") {
                // Convert f64 to Decimal using string representation for accuracy
                let rate_str = format!("{:.6}", rate_f64);
                if let Ok(rate) = rate_str.parse::<Decimal>() {
                    let mut cache = fx_cache().write().await;
                    *cache = Some((rate, now));
                    return rate;
                }
            }
        }
    }

    // Fallback if API is down
    default_idr_rate()
}

/// Get the FX rate as f64 for backward compatibility (display, non-financial use).
#[allow(dead_code)]
pub async fn get_usd_to_idr_rate_f64() -> f64 {
    use rust_decimal::prelude::ToPrimitive;
    get_usd_to_idr_rate()
        .await
        .to_f64()
        .unwrap_or(crate::config::DEFAULT_USD_TO_IDR_RATE)
}

// ─── Wallet Helpers ─────────────────────────────────────────────

/// Ensure a wallet exists for a given user, type, and currency.
/// Returns the wallet ID.
#[allow(dead_code)]
pub async fn ensure_wallet(
    pool: &PgPool,
    user_id: Uuid,
    wallet_type: &str,
    currency: &str,
) -> Result<Uuid, sqlx::Error> {
    let wallet_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES ($1, $2, $3, 0)
        ON CONFLICT (user_id, wallet_type, currency) DO UPDATE
        SET updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(wallet_type)
    .bind(currency)
    .fetch_one(pool)
    .await?;

    Ok(wallet_id)
}

/// Get the balance of a specific wallet.
#[allow(dead_code)]
pub async fn get_wallet_balance(
    pool: &PgPool,
    user_id: Uuid,
    wallet_type: &str,
    currency: &str,
) -> Result<i64, sqlx::Error> {
    let balance: Option<i64> = sqlx::query_scalar(
        "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = $2 AND currency = $3",
    )
    .bind(user_id)
    .bind(wallet_type)
    .bind(currency)
    .fetch_optional(pool)
    .await?;

    Ok(balance.unwrap_or(0))
}

// ─── Deposit Logic ──────────────────────────────────────────────

/// Create a deposit request (intent). Returns the deposit request details.
///
/// For USD: generates Stripe-like instructions (or manual wire).
/// For IDR: would generate an OCBC Virtual Account (mocked for now).
pub async fn create_deposit_request(
    pool: &PgPool,
    user_id: Uuid,
    currency: &str,
    amount_cents: i64,
) -> Result<DepositResponse, sqlx::Error> {
    // Determine provider based on currency
    let provider = match currency {
        "IDR" => "ocbc",
        "USD" => "stripe",
        _ => "manual",
    };

    // Generate a mock provider reference (in production, call the PSP API)
    let provider_ref = format!(
        "{}-{}-{}",
        provider.to_uppercase(),
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    // Instructions depend on currency
    let instructions = match currency {
        "IDR" => format!(
            "Transfer Rp {} to Virtual Account: {}. This VA expires in 24 hours.",
            format_idr(amount_cents),
            &provider_ref
        ),
        "USD" => {
            let display = crate::common::currency::format_usd(amount_cents);
            format!(
                "Wire {} to POOOL GmbH, IBAN: DE89370400440532013000, Reference: {}",
                display, &provider_ref
            )
        }
        _ => "Please contact support for deposit instructions.".to_string(),
    };

    // Set expiry (24 hours from now)
    let expires_at = Utc::now() + chrono::Duration::hours(24);

    // Insert deposit request
    let deposit_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO deposit_requests (user_id, currency, amount_cents, provider, provider_reference, status, expires_at)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(currency)
    .bind(amount_cents)
    .bind(provider)
    .bind(&provider_ref)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(DepositResponse {
        deposit_id,
        provider: provider.to_string(),
        provider_reference: Some(provider_ref),
        amount_cents,
        currency: currency.to_string(),
        status: "pending".to_string(),
        instructions,
    })
}

/// Confirm a deposit (called by webhook or admin approval).
/// This is the ATOMIC operation that credits the user's wallet.
pub async fn confirm_deposit(pool: &PgPool, provider_reference: &str) -> Result<Uuid, String> {
    confirm_deposit_with_audit(pool, provider_reference, None, None).await
}

/// Confirm a deposit with explicit audit context for admin/four-eyes actions.
pub async fn confirm_deposit_with_audit(
    pool: &PgPool,
    provider_reference: &str,
    audit_actor_user_id: Option<Uuid>,
    admin_notes: Option<String>,
) -> Result<Uuid, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("TX begin failed: {}", e))?;

    // 1. Fetch the deposit request with a row lock to prevent double-processing
    let deposit = sqlx::query_as::<_, (Uuid, Uuid, String, i64, String)>(
        r#"
        SELECT id, user_id, currency, amount_cents, status
        FROM deposit_requests
        WHERE provider_reference = $1
        FOR UPDATE
        "#,
    )
    .bind(provider_reference)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Deposit lookup failed: {}", e))?;

    let (deposit_id, user_id, currency, amount_cents, status) = match deposit {
        Some(d) => d,
        None => return Err("Deposit request not found".to_string()),
    };

    // 2. Idempotency: if already paid, return success without re-crediting
    if status == "paid" {
        tx.commit()
            .await
            .map_err(|e| format!("TX commit failed: {}", e))?;
        return Ok(deposit_id);
    }

    if status != "pending" {
        return Err(format!("Deposit is in status '{}', cannot confirm", status));
    }

    // 3. Mark deposit as paid
    sqlx::query("UPDATE deposit_requests SET status = 'paid', paid_at = NOW() WHERE id = $1")
        .bind(deposit_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Deposit status update failed: {}", e))?;

    // 4. Ensure the specific currency wallet exists and credit it
    let wallet_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES ($1, 'cash', $2, $3)
        ON CONFLICT (user_id, wallet_type, currency) DO UPDATE
        SET balance_cents = wallets.balance_cents + $3
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&currency)
    .bind(amount_cents)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Wallet credit failed: {}", e))?;

    // 5. Log the transaction
    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description, external_ref_id)
        VALUES ($1, 'deposit', 'completed', $2, $3, 'Bank deposit confirmed', $4)
        "#,
    )
    .bind(wallet_id)
    .bind(amount_cents)
    .bind(&currency)
    .bind(provider_reference)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Transaction log failed: {}", e))?;

    // 6. Audit log
    let actor_user_id = audit_actor_user_id.unwrap_or(user_id);
    let audit_state = serde_json::json!({
        "status": "paid",
        "credited_user_id": user_id,
        "amount_cents": amount_cents,
        "currency": currency,
        "provider_reference": provider_reference,
        "admin_notes": admin_notes,
        "source": if audit_actor_user_id.is_some() { "admin" } else { "system_or_webhook" }
    });
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'deposit.confirmed', 'deposit_request', $2, $3)
        "#,
    )
    .bind(actor_user_id)
    .bind(deposit_id)
    .bind(audit_state)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // 7. Commit the entire atomic operation
    tx.commit()
        .await
        .map_err(|e| format!("TX commit failed: {}", e))?;

    tracing::info!(
        deposit_id = %deposit_id,
        user_id = %user_id,
        amount_cents = amount_cents,
        currency = %currency,
        "Deposit confirmed and wallet credited"
    );

    Ok(deposit_id)
}

// ─── Checkout Logic ─────────────────────────────────────────────

/// Execute a checkout: purchase all items in the user's cart.
///
/// This is the CORE ATOMIC TRANSACTION:
/// 1. Lock asset rows (FOR UPDATE) to prevent race conditions
/// 2. Verify sufficient token availability for each asset
/// 3. Verify sufficient wallet balance
/// 4. Deduct wallet balance
/// 5. Reduce asset token availability
/// 6. Create order + order_items
/// 7. Create/update investments only for completed wallet payments
/// 8. Clear cart
/// 9. Generate invoice
pub async fn execute_checkout(
    pool: &PgPool,
    user_id: Uuid,
    payment_currency: &str,
    payment_method: &str,
    proof_url: Option<String>,
) -> Result<CheckoutResult, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("checkout".into()),
        message: Some(format!(
            "Checkout started: user={} method={} currency={}",
            user_id, payment_method, payment_currency
        )),
        level: sentry::Level::Info,
        ..Default::default()
    });

    // We fetch the FX rate before entering the DB transaction to avoid holding locks during I/O
    let fx_rate = if payment_currency == "IDR" {
        Some(get_usd_to_idr_rate().await)
    } else {
        None
    };

    let mut tx = pool.begin().await.map_err(|e| format!("TX begin: {}", e))?;

    // 1. Fetch cart items with asset details (lock assets)
    let cart_items = sqlx::query_as::<_, (Uuid, Uuid, i32, i64, String, i64, i32)>(
        r#"
        SELECT ci.id, ci.asset_id, ci.tokens_quantity, ci.token_price_cents,
               a.title, a.token_price_cents, a.tokens_available
        FROM cart_items ci
        JOIN assets a ON a.id = ci.asset_id
        WHERE ci.user_id = $1
        FOR UPDATE OF a
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Cart fetch failed: {}", e))?;

    if cart_items.is_empty() {
        return Err("Your cart is empty".to_string());
    }

    // 2. Validate availability and calculate total
    let mut subtotal_cents: i64 = 0;
    let mut items_count: i32 = 0;

    for item in &cart_items {
        let (_ci_id, _asset_id, tokens_qty, _ci_price, title, asset_price, tokens_avail) = item;
        if *tokens_avail < *tokens_qty {
            return Err(format!(
                "Insufficient tokens for '{}': requested {}, available {}",
                title, tokens_qty, tokens_avail
            ));
        }
        subtotal_cents += *asset_price * (*tokens_qty as i64);
        items_count += tokens_qty;
    }

    // 2.5 Check investment limits (Phase 17.2)
    // Must come AFTER subtotal_cents is calculated above.
    let current_year = Utc::now().year();
    let limit_info = sqlx::query!(
        r#"
        SELECT available_cents, annual_limit_cents, invested_12m_cents
        FROM investment_limits
        WHERE user_id = $1 AND limit_year = $2
        "#,
        user_id,
        current_year
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Limit check failed: {}", e))?;

    let effective_limit = match limit_info {
        Some(l) => Some((l.annual_limit_cents, l.invested_12m_cents)),
        None => {
            // Fallback to global default in platform_settings.
            // Bootstrap a per-user row so subsequent UPDATE on `invested_12m_cents`
            // (further down in this function) targets a real row.
            let global_default: Option<i64> = sqlx::query_scalar(
                "SELECT value::bigint FROM platform_settings
                 WHERE key = 'default_annual_investment_limit_cents'",
            )
            .fetch_optional(&mut *tx)
            .await
            .ok()
            .flatten();
            if let Some(default_cents) = global_default.filter(|v| *v > 0) {
                let invested_12m: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(SUM(oi.subtotal_cents), 0)::bigint
                     FROM orders o
                     JOIN order_items oi ON oi.order_id = o.id
                     WHERE o.user_id = $1 AND o.status = 'completed'
                       AND o.created_at >= NOW() - INTERVAL '365 days'",
                )
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await
                .unwrap_or(0);
                sqlx::query(
                    r#"INSERT INTO investment_limits
                         (user_id, limit_year, annual_limit_cents, invested_12m_cents, updated_at)
                       VALUES ($1, $2, $3, $4, NOW())
                       ON CONFLICT (user_id, limit_year) DO NOTHING"#,
                )
                .bind(user_id)
                .bind(current_year)
                .bind(default_cents)
                .bind(invested_12m)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Limit bootstrap failed: {}", e))?;
                Some((default_cents, invested_12m))
            } else {
                None
            }
        }
    };

    if let Some((annual_limit, invested_12m)) = effective_limit {
        let available = annual_limit - invested_12m;
        if annual_limit > 0 && available < subtotal_cents {
            let available_usd = crate::common::currency::format_usd(available);
            return Err(format!(
                "Order exceeds your annual investment limit. Available: {}. Please update your profile or contact support.",
                available_usd
            ));
        }
    }

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("checkout".into()),
        message: Some(format!(
            "Cart validated: {} items, subtotal={}",
            items_count,
            crate::common::currency::format_usd(subtotal_cents)
        )),
        level: sentry::Level::Info,
        ..Default::default()
    });

    // 2.8 Calculate platform fee
    let platform_fee_pct: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Platform fee config lookup failed: {}", e))?
    .and_then(|v: String| v.parse().ok())
    .unwrap_or(rust_decimal::Decimal::from(0));

    let fee_cents = calculate_platform_fee_cents(subtotal_cents, platform_fee_pct)?;
    let grand_total_cents = subtotal_cents
        .checked_add(fee_cents)
        .ok_or("Grand total amount too large to process")?;

    // 3. Handle FX if payment currency differs from asset currency (USD)
    // Phase 1.10: Uses Decimal arithmetic to avoid IEEE754 rounding errors
    let (final_deduct_cents, fx_rate_applied): (i64, Option<Decimal>) = if payment_currency == "IDR"
    {
        use rust_decimal::prelude::ToPrimitive;
        let rate = fx_rate.unwrap_or_else(default_idr_rate);
        // Decimal math: (grand_total_cents * rate) / 100 = IDR whole amount
        let total_dec = Decimal::from(grand_total_cents);
        let idr_total_dec = (total_dec * rate) / Decimal::from(100);
        let idr_total = idr_total_dec
            .to_i64()
            .ok_or("IDR total amount too large to process")?;
        (idr_total, Some(rate))
    } else {
        (grand_total_cents, None)
    };

    let mut wallet_id_used = None;

    // 4. Handle Wallet Deduction if applicable
    if payment_method == "wallet" {
        // Check wallet balance (with row lock)
        let wallet_row = sqlx::query_as::<_, (Uuid, i64)>(
            r#"
            SELECT id, balance_cents FROM wallets
            WHERE user_id = $1 AND wallet_type = 'cash' AND currency = $2
            FOR UPDATE
            "#,
        )
        .bind(user_id)
        .bind(payment_currency)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Wallet lookup failed: {}", e))?;

        let (wallet_id, balance) = match wallet_row {
            Some(w) => w,
            None => {
                return Err(format!(
                    "No {} wallet found. Please deposit funds first.",
                    payment_currency
                ))
            }
        };

        if balance < final_deduct_cents {
            let needed = if payment_currency == "IDR" {
                format!("Rp {}", format_idr(final_deduct_cents))
            } else {
                crate::common::currency::format_usd(final_deduct_cents)
            };
            return Err(format!(
                "Insufficient {} balance. Required: {}. Please deposit funds.",
                payment_currency, needed
            ));
        }

        // Deduct wallet
        sqlx::query("UPDATE wallets SET balance_cents = balance_cents - $1 WHERE id = $2")
            .bind(final_deduct_cents)
            .bind(wallet_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Wallet deduction failed: {}", e))?;

        wallet_id_used = Some(wallet_id);

        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("checkout".into()),
            message: Some(format!(
                "Wallet deducted: {} {} cents",
                payment_currency, final_deduct_cents
            )),
            level: sentry::Level::Info,
            ..Default::default()
        });
    }

    // 5. Generate order details
    let order_number = format!(
        "ORD-{}-{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        &Uuid::new_v4().to_string()[..6]
    );
    let order_status = if payment_method == "wallet" {
        "completed"
    } else {
        "pending"
    };
    let completed_at = if payment_method == "wallet" {
        Some(Utc::now())
    } else {
        None
    };

    // 6. Create order
    let fx_rate_decimal = fx_rate_applied;
    let fx_provider = if fx_rate_applied.is_some() {
        Some("open.er-api.com")
    } else {
        None
    };

    let order_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO orders (user_id, order_number, total_cents, payment_currency, currency, fx_rate, fx_provider, status, payment_method, proof_of_transfer_url, completed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&order_number)
    .bind(grand_total_cents)
    .bind(payment_currency) // e.g. IDR
    .bind("USD")
    .bind(fx_rate_decimal)
    .bind(fx_provider)
    .bind(order_status)
    .bind(payment_method)
    .bind(proof_url)
    .bind(completed_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Order creation failed: {}", e))?;

    // 7. Create order items and reserve asset availability.
    // Wallet payments are completed immediately, so they allocate investments now.
    // Bank transfers remain pending review and allocate investments only after approval.
    for item in &cart_items {
        let (_ci_id, asset_id, tokens_qty, _ci_price, _title, asset_price, _tokens_avail) = item;
        let subtotal = *asset_price * (*tokens_qty as i64);

        // Order item
        sqlx::query(
            r#"
            INSERT INTO order_items (order_id, asset_id, tokens_quantity, token_price_cents, subtotal_cents)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(order_id)
        .bind(asset_id)
        .bind(tokens_qty)
        .bind(asset_price)
        .bind(subtotal)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Order item creation failed: {}", e))?;

        // Reduce asset token availability (Reservation)
        sqlx::query("UPDATE assets SET tokens_available = tokens_available - $1 WHERE id = $2")
            .bind(tokens_qty)
            .bind(asset_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Asset update failed: {}", e))?;

        // Auto-update funding_status to 'funded' if all tokens are sold
        sqlx::query(
            r#"UPDATE assets SET funding_status = 'funded', updated_at = NOW()
               WHERE id = $1 AND tokens_available <= 0
                 AND funding_status IN ('funding_open', 'funding_in_progress')"#,
        )
        .bind(asset_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Funding status update failed: {}", e))?;

        if payment_method == "wallet" {
            upsert_active_investment(&mut tx, user_id, *asset_id, *tokens_qty, subtotal).await?;
        }
    }

    // 7.5 Update investment limits (Phase 17.2)
    sqlx::query!(
        "UPDATE investment_limits SET invested_12m_cents = invested_12m_cents + $1, updated_at = NOW() WHERE user_id = $2 AND limit_year = $3",
        subtotal_cents, user_id, current_year
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Limit update failed: {}", e))?;

    // 8. Log wallet transaction (only if wallet used)
    if let Some(w_id) = wallet_id_used {
        sqlx::query(
            r#"
            INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description, related_order_id)
            VALUES ($1, 'purchase', 'completed', $2, $3, 'Asset purchase', $4)
            "#,
        )
        .bind(w_id)
        .bind(-final_deduct_cents)
        .bind(payment_currency)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Wallet TX log failed: {}", e))?;
    }

    // 8.5 Credit platform fee wallet (if fee exists and payment method is wallet).
    // Require rows_affected == 1 so a duplicated or missing platform_fee
    // wallet aborts the tx rather than losing the fee credit.
    if fee_cents > 0 && payment_method == "wallet" {
        let affected = sqlx::query(
            "UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW()
             WHERE wallet_type = 'platform_fee' AND currency = 'USD'",
        )
        .bind(fee_cents)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Platform fee wallet credit failed: {}", e))?
        .rows_affected();
        if affected != 1 {
            return Err(format!(
                "Platform fee wallet not uniquely matched (affected={})",
                affected
            ));
        }
    }

    // 9. Clear cart
    sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Cart clear failed: {}", e))?;

    // 10. Generate invoice
    let invoice_number = generate_invoice_number(&mut tx).await?;
    sqlx::query(
        r#"
        INSERT INTO invoices (invoice_number, order_id, user_id, subtotal_cents, total_cents, currency, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'issued')
        "#,
    )
    .bind(&invoice_number)
    .bind(order_id)
    .bind(user_id)
    .bind(subtotal_cents)
    .bind(grand_total_cents)
    .bind("USD")
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Invoice creation failed: {}", e))?;

    // 11. Audit log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
        VALUES ($1, $2, 'order', $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(if payment_method == "wallet" {
        "checkout.completed"
    } else {
        "checkout.pending_review"
    })
    .bind(order_id)
    .bind(serde_json::json!({
        "payment_method": payment_method,
        "order_status": order_status
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Audit log failed: {}", e))?;

    let postback_data = if payment_method == "wallet" {
        // 11.5 Legacy referral_tracking qualification removed (audit GAP-07,
        // migration 155). Only the affiliate-commission path runs below.

        // 11.6 Track Affiliate Commission (Phase 18) after completed payment only.
        match crate::rewards::service::check_and_track_affiliate_commission(
            &mut tx,
            user_id,
            order_id,
            grand_total_cents,
        )
        .await
        {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(
                    "Failed to track affiliate commission for user {}: {}",
                    user_id,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // 12. Commit everything
    tx.commit().await.map_err(|e| format!("TX commit: {}", e))?;

    if let Some((affiliate_id, sub_id, comm_cents)) = postback_data {
        crate::rewards::service::trigger_s2s_postback(
            pool.clone(),
            affiliate_id,
            "commission".to_string(),
            sub_id,
            comm_cents,
        )
        .await;
    }

    tracing::info!(
        order_id = %order_id,
        order_number = %order_number,
        user_id = %user_id,
        method = %payment_method,
        "Checkout executed successfully"
    );

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("checkout".into()),
        message: Some(format!(
            "Checkout complete: order={} total={}",
            order_number,
            crate::common::currency::format_usd(grand_total_cents)
        )),
        level: sentry::Level::Info,
        ..Default::default()
    });

    Ok(CheckoutResult {
        order_id,
        order_number,
        total_cents: grand_total_cents,
        currency: "USD".to_string(),
        items_purchased: items_count,
        invoice_number: Some(invoice_number),
        purchased_asset_ids: cart_items.iter().map(|item| item.1).collect(),
    })
}

/// Get user's invoices.
pub async fn get_user_invoices(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<InvoiceView>, sqlx::Error> {
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            i64,
            String,
            String,
            chrono::DateTime<Utc>,
            Option<String>,
        ),
    >(
        r#"
        SELECT id, invoice_number, total_cents, currency, status, issued_at, pdf_url
        FROM invoices
        WHERE user_id = $1
        ORDER BY issued_at DESC
        LIMIT 50
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, num, total, currency, status, issued_at, pdf_url)| InvoiceView {
                id,
                invoice_number: num,
                total_cents: total,
                currency,
                status,
                issued_at: issued_at.format("%d %b %Y").to_string(),
                pdf_url,
            },
        )
        .collect())
}

/// Get user's deposit history.
pub async fn get_user_deposits(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<DepositRequest>, sqlx::Error> {
    let deposits = sqlx::query_as::<_, DepositRequest>(
        r#"
        SELECT id, user_id, currency, amount_cents, provider, provider_reference,
               status, payment_method, expires_at, paid_at, created_at, updated_at
        FROM deposit_requests
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(deposits)
}

/// Cleanup expired pending orders and restore their asset tokens.
/// Industry standard: run this periodically via a background worker or cron job.
pub async fn cleanup_expired_orders(pool: &PgPool) -> Result<i32, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Find pending bank orders older than 48 hours
    let expired_orders = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM orders
         WHERE status = 'pending'
           AND payment_method IN ('bank', 'bank_transfer')
           AND created_at < NOW() - INTERVAL '48 hours'
         FOR UPDATE",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let count = expired_orders.len() as i32;

    for (order_id,) in expired_orders {
        // 2. We reuse our reject_order logic to restore tokens and mark failed
        // But reject_order takes a pool and makes its own TX, so we should refactor it
        // or just call it directly for each. To keep it simple for now, we'll mark them.

        // This is a simplified version of reject_order logic applied in a batch-friendly way:
        let items = sqlx::query_as::<_, (i32, Uuid)>(
            "SELECT tokens_quantity, asset_id FROM order_items WHERE order_id = $1",
        )
        .bind(order_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM orders WHERE id = $1")
            .bind(order_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        for (qty, asset_id) in items {
            // 2. Restore tokens to asset
            sqlx::query("UPDATE assets SET tokens_available = tokens_available + $1 WHERE id = $2")
                .bind(qty)
                .bind(asset_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| e.to_string())?;

            // Restore funding_status if asset was 'funded' but now has tokens available again
            sqlx::query(
                r#"UPDATE assets SET funding_status = 'funding_in_progress', updated_at = NOW()
                   WHERE id = $1 AND tokens_available > 0 AND funding_status = 'funded'"#,
            )
            .bind(asset_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

            // 3. Subtract tokens from user's investment record
            // We fetch the subtotal for this item's contribution from order_items (though we already have asset_id/qty)
            let subtotal: i64 = sqlx::query_scalar(
                "SELECT subtotal_cents FROM order_items WHERE order_id = $1 AND asset_id = $2",
            )
            .bind(order_id)
            .bind(asset_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

            revert_pending_order_item_investment(&mut tx, user_id, asset_id, qty, subtotal).await?;
        }

        // 3. Mark order as failed (expired)
        sqlx::query("UPDATE orders SET status = 'failed' WHERE id = $1")
            .bind(order_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    if count > 0 {
        tracing::info!(
            "♻️ Cleanup: {} expired bank orders purged and tokens restored.",
            count
        );
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(count)
}

// ─── Helpers ────────────────────────────────────────────────────

/// Calculate the final deduction amount (in cents) and the applied FX rate.
/// If `payment_currency` is "IDR", it applies the mock rate of 15,500 IDR / USD.
#[allow(dead_code)]
pub fn calculate_fx_deduction(
    total_usd_cents: i64,
    payment_currency: &str,
) -> (i64, Option<rust_decimal::Decimal>) {
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::Decimal;
    if payment_currency == "IDR" {
        let rate = Decimal::from(crate::config::DEFAULT_USD_TO_IDR_RATE_I64);
        let idr_total_dec = (Decimal::from(total_usd_cents) * rate) / Decimal::from(100);
        let idr_total = idr_total_dec.to_i64().unwrap_or(i64::MAX); // Better ceiling than zero on theoretical overflow
        (idr_total, Some(rate))
    } else {
        (total_usd_cents, None)
    }
}

/// Calculate platform checkout fee in cents from a percentage stored in
/// `platform_settings`. Uses Decimal all the way through and rounds up to avoid
/// under-collecting fractional cents.
pub fn calculate_platform_fee_cents(
    subtotal_cents: i64,
    platform_fee_pct: Decimal,
) -> Result<i64, String> {
    if subtotal_cents < 0 {
        return Err("Subtotal cannot be negative".to_string());
    }
    if platform_fee_pct < Decimal::ZERO {
        return Err("Platform fee percent cannot be negative".to_string());
    }

    use rust_decimal::prelude::ToPrimitive;
    let fee_cents_dec = Decimal::from(subtotal_cents)
        .checked_mul(platform_fee_pct)
        .and_then(|fee| fee.checked_div(Decimal::from(100)))
        .ok_or_else(|| "Fee amount too large to process".to_string())?;
    fee_cents_dec
        .ceil()
        .to_i64()
        .ok_or_else(|| "Fee amount too large to process".to_string())
}

/// Admin: Approve a pending order (e.g. after manual bank transfer verified).
pub async fn approve_order(
    pool: &sqlx::PgPool,
    order_id: uuid::Uuid,
    admin_user_id: uuid::Uuid,
) -> Result<(uuid::Uuid, Vec<uuid::Uuid>), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Fetch order and lock it
    let (status, user_id, order_num) = sqlx::query_as::<_, (String, uuid::Uuid, String)>(
        "SELECT status, user_id, order_number FROM orders WHERE id = $1 FOR UPDATE",
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Order not found: {}", e))?;

    if status != "pending" {
        return Err(format!("Cannot approve order with status: {}", status));
    }

    let order_items = sqlx::query_as::<_, (Uuid, i32, i64)>(
        "SELECT asset_id, tokens_quantity, subtotal_cents FROM order_items WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Failed to get order items: {}", e))?;

    let order_assets: Vec<Uuid> = order_items
        .iter()
        .map(|(asset_id, _, _)| *asset_id)
        .collect();

    // 2. Allocate investments only after manual payment verification.
    for (asset_id, tokens_qty, subtotal) in &order_items {
        allocate_order_item_investment(&mut tx, user_id, *asset_id, *tokens_qty, *subtotal).await?;
    }

    // 3. Update order status to completed
    sqlx::query("UPDATE orders SET status = 'completed', completed_at = NOW() WHERE id = $1")
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update order status: {}", e))?;

    // Auto-update funding_status to 'funded'
    sqlx::query(
        r#"
        UPDATE assets SET funding_status = 'funded', updated_at = NOW()
        WHERE id IN (SELECT asset_id FROM order_items WHERE order_id = $1)
          AND tokens_available <= 0
          AND funding_status IN ('funding_open', 'funding_in_progress')
        "#,
    )
    .bind(order_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Failed to update funding status: {}", e))?;

    // 4. Audit Log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(admin_user_id)
    .bind("ORDER_APPROVED")
    .bind("orders")
    .bind(order_id)
    .bind(serde_json::json!({
        "order_number": order_num,
        "admin_action": true,
        "customer_user_id": user_id.to_string()
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Audit log failed: {}", e))?;

    // 4.4 Mark eligible for primary on-chain settlement (T+1 batch).
    // Best-effort — logging-only on failure so a chain-config issue
    // never blocks order approval. The worker also has filters that
    // skip ineligible items, so a missed seed is recoverable later
    // via the admin "Run primary settlement now" button.
    let delay_secs = sqlx::query_scalar::<_, String>(
        "SELECT value FROM platform_settings WHERE key = 'chain_primary_settle_delay_secs'",
    )
    .fetch_optional(&mut *tx)
    .await
    .ok()
    .flatten()
    .and_then(|v| v.parse::<i64>().ok())
    .unwrap_or(86_400);
    if let Err(e) =
        crate::blockchain::primary_settlement::mark_order_eligible(&mut tx, order_id, delay_secs)
            .await
    {
        tracing::error!(
            "Failed to mark order {} eligible for primary settlement: {}",
            order_id,
            e
        );
    }

    // 4.5 Legacy referral_tracking qualification removed (audit GAP-07,
    // migration 155). Only the affiliate-commission path runs below.

    // 4.6 Track Affiliate Commission (Phase 18)
    let order_total: i64 = sqlx::query_scalar("SELECT total_cents FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

    let postback_result = crate::rewards::service::check_and_track_affiliate_commission(
        &mut tx,
        user_id,
        order_id,
        order_total,
    )
    .await;

    let postback_data = match postback_result {
        Ok(data) => data,
        Err(e) => {
            tracing::error!(
                "Failed to track affiliate commission for user {}: {}",
                user_id,
                e
            );
            None
        }
    };

    tx.commit().await.map_err(|e| e.to_string())?;

    if let Some((affiliate_id, sub_id, comm_cents)) = postback_data {
        crate::rewards::service::trigger_s2s_postback(
            pool.clone(),
            affiliate_id,
            "commission".to_string(),
            sub_id,
            comm_cents,
        )
        .await;
    }

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("admin.order".into()),
        message: Some(format!(
            "Order approved: {} for user {}",
            order_num, user_id
        )),
        level: sentry::Level::Info,
        ..Default::default()
    });

    Ok((user_id, order_assets))
}

/// Admin: Reject a pending order.
pub async fn reject_order(
    pool: &sqlx::PgPool,
    order_id: uuid::Uuid,
    admin_user_id: uuid::Uuid,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    // 1. Fetch order, lock it
    let (status, user_id, order_num) = sqlx::query_as::<_, (String, uuid::Uuid, String)>(
        "SELECT status, user_id, order_number FROM orders WHERE id = $1 FOR UPDATE",
    )
    .bind(order_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Order not found: {}", e))?;

    if status != "pending" {
        return Err(format!("Cannot reject order with status: {}", status));
    }

    // 2. Update order status to failed
    sqlx::query("UPDATE orders SET status = 'failed' WHERE id = $1")
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to update order status: {}", e))?;

    // 3. Mark investments as failed (or similar status)
    // Note: investments table uses (user_id, asset_id) as the unique identifier.
    // Fail all funding_in_progress investments for assets in this order.
    let order_items = sqlx::query_as::<_, (i32, uuid::Uuid)>(
        "SELECT tokens_quantity, asset_id FROM order_items WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Failed to fetch order items: {}", e))?;

    for (qty, asset_id) in &order_items {
        // We need to fetch the subtotal for this item from order_items
        let subtotal: i64 = sqlx::query_scalar(
            "SELECT subtotal_cents FROM order_items WHERE order_id = $1 AND asset_id = $2",
        )
        .bind(order_id)
        .bind(asset_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Failed to fetch subtotal details: {}", e))?;

        revert_pending_order_item_investment(&mut tx, user_id, *asset_id, *qty, subtotal).await?;

        // 4. Restore tokens to the asset
        sqlx::query("UPDATE assets SET tokens_available = tokens_available + $1 WHERE id = $2")
            .bind(qty)
            .bind(asset_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to return tokens to asset {}: {}", asset_id, e))?;

        // Revert funding_status if asset was 'funded' but now has tokens available again
        sqlx::query(
            r#"UPDATE assets SET funding_status = 'funding_in_progress', updated_at = NOW()
               WHERE id = $1 AND tokens_available > 0 AND funding_status = 'funded'"#,
        )
        .bind(asset_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            format!(
                "Failed to revert funding status for asset {}: {}",
                asset_id, e
            )
        })?;
    }

    // 5. Audit Log
    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(admin_user_id)
    .bind("ORDER_REJECTED")
    .bind("orders")
    .bind(order_id)
    .bind(serde_json::json!({
        "order_number": order_num,
        "admin_action": true,
        "customer_user_id": user_id.to_string()
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Audit log failed: {}", e))?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Generate a sequential invoice number: INV-YYYY-NNNNN
async fn generate_invoice_number(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<String, String> {
    let seq_val: i64 = sqlx::query_scalar("SELECT nextval('invoice_number_seq')")
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| format!("Invoice seq failed: {}", e))?;

    let year = Utc::now().format("%Y");
    Ok(format!("INV-{}-{:05}", year, seq_val))
}

/// Format IDR amount (cents to display string with thousands separators).
fn format_idr(cents: i64) -> String {
    // IDR doesn't use sub-units, so cents == actual IDR value
    let is_negative = cents < 0;
    let val = cents.abs().to_string();
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
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_idr() {
        // Test basic formatting
        assert_eq!(format_idr(15000), "15.000");
        assert_eq!(format_idr(1500000), "1.500.000");
        assert_eq!(format_idr(100), "100");
        assert_eq!(format_idr(0), "0");

        // Test very large numbers
        assert_eq!(format_idr(15000000000), "15.000.000.000");

        // Test negative numbers (which should format cleanly with the negative sign starting)
        assert_eq!(format_idr(-100), "-100");
        assert_eq!(format_idr(-1500), "-1.500");
        assert_eq!(format_idr(-1500000), "-1.500.000");
    }

    #[test]
    fn test_calculate_fx_deduction() {
        // Test base USD no-op conversion
        let (cents_usd, rate_usd) = calculate_fx_deduction(50000, "USD");
        assert_eq!(cents_usd, 50000);
        assert_eq!(rate_usd, None);

        // Test IDR conversion (1 USD = 15,500 IDR)
        // 50000 cents = $500.00
        // $500.00 * 15,500 = 7,750,000 IDR
        let (cents_idr, rate_idr) = calculate_fx_deduction(50000, "IDR");
        assert_eq!(cents_idr, 7750000);
        assert_eq!(rate_idr, Some(Decimal::from(15500)));

        // Test rounding / decimals logic
        // 1 cent = $0.01
        // $0.01 * 15,500 = 155 IDR
        let (cents_idr_small, _) = calculate_fx_deduction(1, "IDR");
        assert_eq!(cents_idr_small, 155);
    }

    #[test]
    fn test_calculate_platform_fee_cents_decimal_percent() {
        let fee = calculate_platform_fee_cents(10_000, Decimal::new(25, 1)).unwrap();
        assert_eq!(fee, 250);
    }

    #[test]
    fn test_calculate_platform_fee_cents_rounds_fractional_cent_up() {
        let fee = calculate_platform_fee_cents(101, Decimal::new(25, 1)).unwrap();
        assert_eq!(fee, 3);
    }

    #[test]
    fn test_calculate_platform_fee_cents_zero_percent() {
        let fee = calculate_platform_fee_cents(10_000, Decimal::ZERO).unwrap();
        assert_eq!(fee, 0);
    }

    #[test]
    fn test_calculate_platform_fee_cents_small_fractional_percent() {
        let fee = calculate_platform_fee_cents(99, Decimal::new(1, 2)).unwrap();
        assert_eq!(fee, 1);
    }

    #[test]
    fn test_calculate_platform_fee_cents_rejects_negative_subtotal() {
        let err = calculate_platform_fee_cents(-1, Decimal::new(25, 1)).unwrap_err();
        assert_eq!(err, "Subtotal cannot be negative");
    }

    #[test]
    fn test_calculate_platform_fee_cents_rejects_negative_percent() {
        let err = calculate_platform_fee_cents(10_000, Decimal::new(-1, 0)).unwrap_err();
        assert_eq!(err, "Platform fee percent cannot be negative");
    }

    #[test]
    fn test_calculate_platform_fee_cents_rejects_overflow() {
        let err = calculate_platform_fee_cents(i64::MAX, Decimal::from(i64::MAX)).unwrap_err();
        assert_eq!(err, "Fee amount too large to process");
    }
}
