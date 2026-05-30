//! HTTP-level integration tests for checkout, wallet, and payment-method flows.
//!
//! These tests drive the production Axum router in-process and assert database
//! side effects for the highest-risk investor money workflows. They are ignored
//! by default because they require a live local Postgres database.
//!
//! Running:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test checkout_wallet_http -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(test)]

use anyhow::{ensure, Context, Result};
use axum::body::Body;
use axum::http::{header, HeaderName, HeaderValue, Method, Request, StatusCode};
use http_body_util::BodyExt;
use poool_backend::{build_platform_router, AppState};
use rust_decimal::Decimal;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use tower::ServiceExt;
use uuid::Uuid;

const TEST_TOTP_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const TEST_TOTP_SECRET: &str = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP";

fn install_test_totp_key() {
    std::env::set_var("TOTP_SECRET_ENCRYPTION_KEY", TEST_TOTP_KEY);
}

macro_rules! ensure_eq {
    ($left:expr, $right:expr $(,)?) => {{
        let left = &$left;
        let right = &$right;
        ensure!(
            left == right,
            "assertion failed: left != right\n  left: {:?}\n right: {:?}",
            left,
            right
        );
    }};
    ($left:expr, $right:expr, $($arg:tt)+) => {{
        ensure!($left == $right, $($arg)+);
    }};
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
        )
        .try_init();
}

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to test DB")
}

fn test_webauthn() -> std::sync::Arc<webauthn_rs::Webauthn> {
    std::sync::Arc::new(
        webauthn_rs::WebauthnBuilder::new(
            "localhost",
            &url::Url::parse("http://localhost:8888").expect("test WebAuthn origin"),
        )
        .expect("test WebAuthn config")
        .rp_name("POOOL")
        .build()
        .expect("build test WebAuthn instance"),
    )
}

fn make_state(pool: PgPool) -> AppState {
    std::env::set_var("POOOL_ENV", "development");
    install_test_totp_key();
    let mut config = poool_backend::config::Config::from_env();
    install_test_totp_key();
    config.app_env = "development".to_string();
    // Local HTTP integration tests must not depend on workstation GCS/ADC
    // credentials. This forces the checkout proof path onto its local-only
    // placeholder branch, while production keeps using the configured bucket.
    config.gcs_bucket = None;

    AppState {
        db: pool,
        db_replica: None,
        community_db: None,
        templates: poool_backend::templates::create_engine(),
        config,
        redis: None,
        auth_rate_limiter: poool_backend::auth::rate_limit::RateLimiter::disabled(),
        leaderboard_rate_limiter: poool_backend::auth::rate_limit::RateLimiter::disabled(),
        community_rate_limiter: poool_backend::auth::rate_limit::RateLimiter::disabled(),
        storage_rate_limiter: poool_backend::auth::rate_limit::RateLimiter::disabled(),
        leaderboard_last_refresh: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        webauthn: test_webauthn(),
    }
}

async fn insert_test_user(pool: &PgPool, balance_cents: i64) -> Result<Uuid> {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, email_verified, status)
         VALUES ($1, $2, 'x', TRUE, 'active')",
    )
    .bind(user_id)
    .bind(format!("{}@checkout-wallet-http.test", user_id))
    .execute(pool)
    .await
    .context("insert user")?;

    sqlx::query(
        "INSERT INTO kyc_records (user_id, status, provider, verified_at)
         VALUES ($1, 'approved', 'manual', NOW() - INTERVAL '10 days')",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .context("insert kyc")?;

    sqlx::query(
        "INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
         VALUES ($1, 'cash', 'USD', $2), ($1, 'rewards', 'USD', 0)",
    )
    .bind(user_id)
    .bind(balance_cents)
    .execute(pool)
    .await
    .context("insert wallets")?;

    Ok(user_id)
}

async fn insert_test_admin_user(pool: &PgPool) -> Result<Uuid> {
    let admin_id = insert_test_user(pool, 0).await?;

    sqlx::query(
        "INSERT INTO roles (name, description)
         VALUES ('admin', 'Platform administrator')
         ON CONFLICT (name) DO NOTHING",
    )
    .execute(pool)
    .await
    .context("ensure admin role")?;

    sqlx::query(
        "INSERT INTO admin_permissions (role_id, permission)
         SELECT id, 'all' FROM roles WHERE name = 'admin'
         ON CONFLICT (role_id, permission) DO NOTHING",
    )
    .execute(pool)
    .await
    .context("ensure admin permissions")?;

    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id)
         SELECT $1, id FROM roles WHERE name = 'admin'
         ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE",
    )
    .bind(admin_id)
    .execute(pool)
    .await
    .context("grant admin role")?;

    Ok(admin_id)
}

async fn insert_test_asset(pool: &PgPool, developer_user_id: Uuid) -> Result<Uuid> {
    let asset_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO assets
            (id, developer_user_id, title, slug, short_description, asset_type,
             total_value_cents, token_price_cents, tokens_total, tokens_available,
             funding_status, published, featured, min_funding_tokens)
         VALUES
            ($1, $2, $3, $4, 'HTTP integration test asset', 'real_estate',
             100000, 1000, 100, 20, 'funding_open', TRUE, FALSE, 0)",
    )
    .bind(asset_id)
    .bind(developer_user_id)
    .bind(format!("HTTP Test Asset {}", asset_id.simple()))
    .bind(format!("http-test-asset-{}", asset_id.simple()))
    .execute(pool)
    .await
    .context("insert asset")?;
    Ok(asset_id)
}

async fn mint_session(pool: &PgPool, user_id: Uuid) -> Result<String> {
    let token = format!("test-{}-{}", user_id.simple(), Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO user_sessions
            (user_id, session_token, ip_address, user_agent, remember_me, is_2fa_verified, expires_at)
           VALUES ($1, $2, NULL, 'checkout-wallet-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
    )
    .bind(user_id)
    .bind(&token)
    .execute(pool)
    .await
    .context("insert session")?;
    Ok(token)
}

async fn enable_totp_for_user(pool: &PgPool, user_id: Uuid) -> Result<()> {
    install_test_totp_key();
    let encrypted = poool_backend::auth::service::encrypt_totp_secret(TEST_TOTP_SECRET)
        .map_err(|e| anyhow::anyhow!("encrypt TOTP secret: {e}"))?;
    sqlx::query(
        r#"
        INSERT INTO user_settings (user_id, totp_secret, totp_enabled, updated_at)
        VALUES ($1, $2, TRUE, NOW())
        ON CONFLICT (user_id)
        DO UPDATE SET totp_secret = EXCLUDED.totp_secret,
                      totp_enabled = TRUE,
                      updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(encrypted)
    .execute(pool)
    .await
    .context("enable TOTP")?;
    Ok(())
}

fn current_totp_code() -> Result<String> {
    let secret_bytes = Secret::Encoded(TEST_TOTP_SECRET.to_string())
        .to_bytes()
        .context("decode TOTP secret")?;
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("POOOL".to_string()),
        "checkout-wallet-http".to_string(),
    )
    .context("build TOTP")?;
    totp.generate_current().context("generate TOTP")
}

async fn insert_personal_affiliate_referral(pool: &PgPool, referred_user_id: Uuid) -> Result<Uuid> {
    let affiliate_user_id = insert_test_user(pool, 0).await?;
    let code_seed = Uuid::new_v4().simple().to_string();
    let code = format!("T{}", &code_seed[..12]);

    sqlx::query(
        "INSERT INTO affiliates
            (user_id, referral_code, current_tier, commission_rate_bps,
             tax_recipient_class, is_tax_ready, status, approved_at)
         VALUES ($1, $2, 'Access', 50, 'foreign', TRUE, 'active', NOW())",
    )
    .bind(affiliate_user_id)
    .bind(&code)
    .execute(pool)
    .await
    .context("insert affiliate")?;

    let link_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO affiliate_links
            (id, code, link_type, attribution_user_id, payout_user_id, status)
         VALUES ($1, $2, 'personal', $3, $3, 'active')",
    )
    .bind(link_id)
    .bind(&code)
    .bind(affiliate_user_id)
    .execute(pool)
    .await
    .context("insert affiliate link")?;

    sqlx::query(
        "INSERT INTO affiliate_referrals
            (affiliate_id, referred_user_id, status, link_id, attribution_user_id, payout_user_id)
         VALUES ($1, $2, 'kyc_approved', $3, $1, $1)",
    )
    .bind(affiliate_user_id)
    .bind(referred_user_id)
    .bind(link_id)
    .execute(pool)
    .await
    .context("insert affiliate referral")?;

    Ok(affiliate_user_id)
}

async fn cleanup_fixture(pool: &PgPool, user_id: Uuid, asset_id: Option<Uuid>) {
    let _ = sqlx::query("DELETE FROM compliance_alerts WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "DELETE FROM affiliate_commissions
         WHERE source_order_id IN (SELECT id FROM orders WHERE user_id = $1)
            OR referral_id IN (
                SELECT id FROM affiliate_referrals
                 WHERE referred_user_id = $1
                    OR affiliate_id = $1
                    OR attribution_user_id = $1
                    OR payout_user_id = $1
            )",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM affiliate_referrals
         WHERE referred_user_id = $1
            OR affiliate_id = $1
            OR attribution_user_id = $1
            OR payout_user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM affiliate_links
         WHERE attribution_user_id = $1 OR payout_user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query("DELETE FROM affiliate_live_counters WHERE payout_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM affiliates WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "DELETE FROM investment_disclosures_log
         WHERE user_id = $1
            OR order_id IN (SELECT id FROM orders WHERE user_id = $1)",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM invoices
         WHERE user_id = $1
            OR order_id IN (SELECT id FROM orders WHERE user_id = $1)",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE user_id = $1)",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query("DELETE FROM orders WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM investments WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM withdrawal_requests WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM deposit_requests WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM payment_methods WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM idempotency_keys WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM step_up_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM audit_logs WHERE actor_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM wallet_transactions WHERE wallet_id IN (SELECT id FROM wallets WHERE user_id = $1)")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM wallets WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM kyc_records WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM investment_limits WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;

    if let Some(asset_id) = asset_id {
        let _ = sqlx::query("DELETE FROM assets WHERE id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
    }

    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

fn form_post(
    uri: &str,
    session_token: &str,
    fields: &[(&str, &str)],
    extra_headers: &[(&str, &str)],
) -> Request<Body> {
    let csrf = "test-csrf-checkout-wallet";
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in fields {
        encoded.append_pair(key, value);
    }

    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");

    for (key, value) in extra_headers {
        builder = builder.header(
            HeaderName::from_bytes(key.as_bytes()).expect("valid header name"),
            HeaderValue::from_str(value).expect("valid header value"),
        );
    }

    builder.body(Body::from(encoded.finish())).unwrap()
}

fn json_post(
    uri: &str,
    session_token: &str,
    body: serde_json::Value,
    extra_headers: &[(&str, &str)],
) -> Request<Body> {
    let csrf = "test-csrf-checkout-wallet";
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(header::CONTENT_TYPE, "application/json");

    for (key, value) in extra_headers {
        builder = builder.header(
            HeaderName::from_bytes(key.as_bytes()).expect("valid header name"),
            HeaderValue::from_str(value).expect("valid header value"),
        );
    }

    builder
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

fn multipart_post(
    uri: &str,
    session_token: &str,
    fields: &[(&str, &str)],
    file_field: &str,
    file_name: &str,
    file_content_type: &str,
    file_bytes: &[u8],
    extra_headers: &[(&str, &str)],
) -> Request<Body> {
    let csrf = "test-csrf-checkout-wallet";
    let boundary = format!("poool-test-{}", Uuid::new_v4().simple());
    let mut body = Vec::new();

    for (key, value) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", key).as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            file_field, file_name
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", file_content_type).as_bytes());
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        );

    for (key, value) in extra_headers {
        builder = builder.header(
            HeaderName::from_bytes(key.as_bytes()).expect("valid header name"),
            HeaderValue::from_str(value).expect("valid header value"),
        );
    }

    builder.body(Body::from(body)).unwrap()
}

fn multipart_post_files(
    uri: &str,
    session_token: &str,
    fields: &[(&str, &str)],
    files: &[(&str, &str, &str, &[u8])],
    extra_headers: &[(&str, &str)],
) -> Request<Body> {
    let csrf = "test-csrf-checkout-wallet";
    let boundary = format!("poool-test-{}", Uuid::new_v4().simple());
    let mut body = Vec::new();

    for (key, value) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", key).as_bytes(),
        );
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    for (field, filename, content_type, bytes) in files {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                field, filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", boundary),
        );

    for (key, value) in extra_headers {
        builder = builder.header(
            HeaderName::from_bytes(key.as_bytes()).expect("valid header name"),
            HeaderValue::from_str(value).expect("valid header value"),
        );
    }

    builder.body(Body::from(body)).unwrap()
}

fn get_with_session(uri: &str, session_token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::COOKIE, format!("poool_session={}", session_token))
        .body(Body::empty())
        .unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

async fn body_json(resp: axum::response::Response) -> Result<serde_json::Value> {
    let bytes = resp.into_body().collect().await?.to_bytes();
    Ok(serde_json::from_slice(&bytes)?)
}

async fn count_rows(pool: &PgPool, table: &str, user_id: Uuid) -> Result<i64> {
    let sql = format!("SELECT COUNT(*)::bigint FROM {} WHERE user_id = $1", table);
    Ok(sqlx::query_scalar::<_, i64>(&sql)
        .bind(user_id)
        .fetch_one(pool)
        .await?)
}

async fn cart_item(pool: &PgPool, user_id: Uuid, asset_id: Uuid) -> Result<Option<(Uuid, i32)>> {
    Ok(sqlx::query_as::<_, (Uuid, i32)>(
        "SELECT id, tokens_quantity FROM cart_items WHERE user_id = $1 AND asset_id = $2",
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_optional(pool)
    .await?)
}

async fn wallet_balance(pool: &PgPool, user_id: Uuid) -> Result<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?)
}

async fn withdrawal_fee_cents(pool: &PgPool) -> Result<i64> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT value FROM platform_settings WHERE key = 'withdrawal_fee_cents'",
    )
    .fetch_optional(pool)
    .await?;
    Ok(raw
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(500)
        .max(0))
}

fn redirect_location(resp: &axum::response::Response) -> String {
    resp.headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string()
}

fn ensure_redirect_error(resp: &axum::response::Response, expected: &str) -> Result<()> {
    ensure!(
        resp.status().is_redirection(),
        "expected redirect for {expected}, got {}",
        resp.status()
    );
    let location = redirect_location(resp);
    ensure!(
        location.contains(expected),
        "expected redirect location to contain {expected}, got {location}"
    );
    Ok(())
}

#[ignore]
#[tokio::test]
async fn frozen_user_wallet_self_service_banner_and_read_only_actions() {
    init_tracing();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 10_000).await.expect("fixture user");
    let session = mint_session(&pool, user_id).await.expect("session");

    let result = async {
        let app = build_platform_router(make_state(pool.clone()));

        let active_wallet = app
            .clone()
            .oneshot(get_with_session("/wallet", &session))
            .await?;
        ensure_eq!(active_wallet.status(), StatusCode::OK);
        let active_html = body_string(active_wallet).await;
        ensure!(
            !active_html.contains("wallet-frozen-banner"),
            "active users should not see frozen review banner"
        );

        sqlx::query(
            "UPDATE users
             SET status = 'frozen',
                 frozen_at = NOW() - INTERVAL '3 hours',
                 frozen_reason = 'withdrawal_velocity',
                 unfreeze_requested_at = NULL
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&pool)
        .await?;

        let frozen_wallet = app
            .clone()
            .oneshot(get_with_session("/wallet", &session))
            .await?;
        ensure_eq!(frozen_wallet.status(), StatusCode::OK);
        let frozen_html = body_string(frozen_wallet).await;
        ensure!(
            frozen_html.contains("id=\"wallet-frozen-banner\""),
            "frozen wallet should render review banner"
        );
        ensure!(
            frozen_html.contains("data-account-frozen=\"true\""),
            "frozen wallet should expose frozen state to JS"
        );
        ensure!(
            frozen_html.contains("Withdrawal velocity review"),
            "frozen wallet should show a human-readable freeze reason"
        );
        ensure!(
            frozen_html.contains("id=\"wallet-unfreeze-request-btn\""),
            "frozen wallet should expose review request button"
        );
        ensure!(
            frozen_html.contains("id=\"empty-deposit-btn\" type=\"button\" class=\"ds-btn ds-btn--primary wallet-empty__cta-primary\" onclick=\"openDepositModal()\" disabled aria-disabled=\"true\""),
            "deposit CTA should be disabled in frozen state"
        );
        ensure!(
            frozen_html.contains("id=\"wallet-payment-banks-add-button\" class=\"ds-btn ds-btn--secondary ds-btn--full\"\n                  onclick=\"openBankModal()\" disabled aria-disabled=\"true\""),
            "bank add action should be disabled in frozen state"
        );

        let blocked_withdraw = app
            .clone()
            .oneshot(form_post(
                "/wallet/withdraw",
                &session,
                &[("amount", "10.00")],
                &[],
            ))
            .await?;
        ensure!(blocked_withdraw.status().is_redirection());
        ensure_eq!(redirect_location(&blocked_withdraw), "/auth/login");
        ensure_eq!(
            count_rows(&pool, "withdrawal_requests", user_id).await?,
            0
        );

        let request_review = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/unfreeze-request",
                &session,
                serde_json::json!({"note": "Rendered from frozen wallet banner"}),
                &[],
            ))
            .await?;
        ensure_eq!(request_review.status(), StatusCode::OK);

        let requested_wallet = app
            .clone()
            .oneshot(get_with_session("/wallet", &session))
            .await?;
        ensure_eq!(requested_wallet.status(), StatusCode::OK);
        let requested_html = body_string(requested_wallet).await;
        ensure!(requested_html.contains("Review already requested"));
        ensure!(requested_html.contains("Compliance has your request and will respond within 1 business day."));

        Ok(())
    }
    .await;

    cleanup_fixture(&pool, user_id, None).await;
    result.expect("frozen wallet self-service render");
}

#[ignore]
#[tokio::test]
async fn cart_checkout_disclosure_idempotency_and_bank_transfer_paths() {
    init_tracing();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 100_000)
        .await
        .expect("fixture user");
    let asset_id = insert_test_asset(&pool, user_id)
        .await
        .expect("fixture asset");
    let session = mint_session(&pool, user_id).await.expect("session");

    let result = run_checkout_paths(&pool, user_id, asset_id, &session).await;
    cleanup_fixture(&pool, user_id, Some(asset_id)).await;
    result.expect("checkout paths");
}

async fn run_checkout_paths(
    pool: &PgPool,
    user_id: Uuid,
    asset_id: Uuid,
    session: &str,
) -> Result<()> {
    let app = build_platform_router(make_state(pool.clone()));

    let add_resp = app
        .clone()
        .oneshot(form_post(
            "/cart/add",
            session,
            &[
                ("property_id", &asset_id.to_string()),
                ("investment_amount", "30.00"),
            ],
            &[],
        ))
        .await?;
    ensure!(
        add_resp.status().is_redirection(),
        "cart add should redirect, got {}",
        add_resp.status()
    );
    ensure!(
        cart_item(pool, user_id, asset_id).await?.map(|(_, q)| q) == Some(3),
        "cart add should create a 3-token row"
    );

    let cart_id = cart_item(pool, user_id, asset_id)
        .await?
        .context("cart item exists")?
        .0;
    let update_resp = app
        .clone()
        .oneshot(form_post(
            "/cart/update",
            session,
            &[
                ("cart_item_id", &cart_id.to_string()),
                ("tokens_quantity", "4"),
            ],
            &[],
        ))
        .await?;
    ensure!(update_resp.status() == StatusCode::OK);
    let update_json = body_json(update_resp).await?;
    ensure!(update_json["tokens_quantity"] == 4);

    let remove_resp = app
        .clone()
        .oneshot(form_post(
            "/cart/remove",
            session,
            &[("cart_item_id", &cart_id.to_string())],
            &[],
        ))
        .await?;
    ensure!(remove_resp.status().is_redirection());
    ensure!(
        cart_item(pool, user_id, asset_id).await?.is_none(),
        "cart remove should delete the row"
    );

    app.clone()
        .oneshot(form_post(
            "/cart/add",
            session,
            &[
                ("property_id", &asset_id.to_string()),
                ("investment_amount", "30.00"),
            ],
            &[],
        ))
        .await?;

    let initial_balance = wallet_balance(pool, user_id).await?;
    let missing_disclosures = app
        .clone()
        .oneshot(form_post(
            "/checkout",
            session,
            &[("payment_method", "wallet"), ("payment_currency", "USD")],
            &[],
        ))
        .await?;
    let missing_status = missing_disclosures.status();
    let missing_body = body_string(missing_disclosures).await;
    ensure!(
        missing_status == StatusCode::BAD_REQUEST,
        "missing disclosures should be 400, got {} body {}",
        missing_status,
        missing_body
    );
    ensure!(missing_body.contains("general investment disclosures"));
    ensure_eq!(count_rows(pool, "orders", user_id).await?, 0);
    ensure_eq!(wallet_balance(pool, user_id).await?, initial_balance);
    ensure!(cart_item(pool, user_id, asset_id).await?.is_some());

    sqlx::query(
        "UPDATE wallets SET balance_cents = 1000
         WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    let insufficient = app
        .clone()
        .oneshot(form_post(
            "/checkout",
            session,
            &[
                ("payment_method", "wallet"),
                ("payment_currency", "USD"),
                ("disclosure_general_1", "on"),
                ("disclosure_general_2", "on"),
                ("disclosure_general_3", "on"),
            ],
            &[],
        ))
        .await?;
    let insufficient_status = insufficient.status();
    let insufficient_body = body_string(insufficient).await;
    ensure!(
        insufficient_status == StatusCode::BAD_REQUEST,
        "insufficient wallet should be 400, got {} body {}",
        insufficient_status,
        insufficient_body
    );
    ensure!(insufficient_body.contains("Insufficient USD balance"));
    ensure_eq!(count_rows(pool, "orders", user_id).await?, 0);

    sqlx::query(
        "UPDATE wallets SET balance_cents = 100000
         WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    let idempotency_key = format!("checkout-wallet-{}", Uuid::new_v4());
    let success = app
        .clone()
        .oneshot(form_post(
            "/checkout",
            session,
            &[
                ("payment_method", "wallet"),
                ("payment_currency", "USD"),
                ("disclosure_general_1", "on"),
                ("disclosure_general_2", "on"),
                ("disclosure_general_3", "on"),
            ],
            &[("Idempotency-Key", &idempotency_key)],
        ))
        .await?;
    ensure!(success.status() == StatusCode::OK);
    let success_json = body_json(success).await?;
    ensure!(success_json["success"] == true);
    ensure!(success_json["redirect_url"] == "/payment-success");
    let first_order = success_json["order_number"]
        .as_str()
        .context("order number")?
        .to_string();

    let subtotal = 3_000_i64;
    let fee_pct = sqlx::query_scalar::<_, Option<String>>(
        "SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'",
    )
    .fetch_optional(pool)
    .await?
    .flatten()
    .unwrap_or_else(|| "0".to_string())
    .parse::<Decimal>()?;
    let fee = poool_backend::payments::service::calculate_platform_fee_cents(subtotal, fee_pct)
        .map_err(anyhow::Error::msg)
        .context("calculate fee")?;
    let grand_total = subtotal + fee;
    ensure_eq!(wallet_balance(pool, user_id).await?, 100_000 - grand_total);
    ensure_eq!(count_rows(pool, "orders", user_id).await?, 1);
    ensure!(
        cart_item(pool, user_id, asset_id).await?.is_none(),
        "wallet checkout should clear cart"
    );

    let order_row = sqlx::query_as::<_, (Uuid, String, i64, String, Option<String>)>(
        "SELECT id, status, total_cents, payment_method, proof_of_transfer_url
         FROM orders WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    ensure_eq!(order_row.1, "completed");
    ensure_eq!(order_row.2, grand_total);
    ensure_eq!(order_row.3, "wallet");
    ensure!(order_row.4.is_none());

    let investment_tokens: i32 = sqlx::query_scalar(
        "SELECT tokens_owned FROM investments WHERE user_id = $1 AND asset_id = $2",
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_one(pool)
    .await?;
    ensure_eq!(investment_tokens, 3);

    let disclosure = sqlx::query_as::<_, (bool, Option<bool>)>(
        "SELECT agreed_to_general, agreed_to_referral
         FROM investment_disclosures_log WHERE user_id = $1 AND order_id = $2",
    )
    .bind(user_id)
    .bind(order_row.0)
    .fetch_one(pool)
    .await?;
    ensure_eq!(disclosure, (true, None));

    let replay = app
        .clone()
        .oneshot(form_post(
            "/checkout",
            session,
            &[
                ("payment_method", "wallet"),
                ("payment_currency", "USD"),
                ("disclosure_general_1", "on"),
                ("disclosure_general_2", "on"),
                ("disclosure_general_3", "on"),
            ],
            &[("Idempotency-Key", &idempotency_key)],
        ))
        .await?;
    ensure!(replay.status() == StatusCode::OK);
    let replay_json = body_json(replay).await?;
    ensure_eq!(
        replay_json["order_number"].as_str(),
        Some(first_order.as_str())
    );
    ensure_eq!(count_rows(pool, "orders", user_id).await?, 1);
    ensure_eq!(wallet_balance(pool, user_id).await?, 100_000 - grand_total);

    let success_page = app
        .clone()
        .oneshot(get_with_session("/payment-success", session))
        .await?;
    ensure_eq!(success_page.status(), StatusCode::OK);

    app.clone()
        .oneshot(form_post(
            "/cart/add",
            session,
            &[
                ("property_id", &asset_id.to_string()),
                ("investment_amount", "20.00"),
            ],
            &[],
        ))
        .await?;

    let bank_missing_ack = app
        .clone()
        .oneshot(form_post(
            "/checkout",
            session,
            &[
                ("payment_method", "bank_transfer"),
                ("payment_currency", "USD"),
                ("disclosure_general_1", "on"),
                ("disclosure_general_2", "on"),
                ("disclosure_general_3", "on"),
            ],
            &[],
        ))
        .await?;
    let bank_missing_ack_status = bank_missing_ack.status();
    let bank_missing_ack_body = body_string(bank_missing_ack).await;
    ensure_eq!(bank_missing_ack_status, StatusCode::BAD_REQUEST);
    ensure!(bank_missing_ack_body.contains("bank transfer reference"));

    let bank_missing_proof = app
        .clone()
        .oneshot(form_post(
            "/checkout",
            session,
            &[
                ("payment_method", "bank_transfer"),
                ("payment_currency", "USD"),
                ("bank_transfer_ack", "on"),
                ("disclosure_general_1", "on"),
                ("disclosure_general_2", "on"),
                ("disclosure_general_3", "on"),
            ],
            &[],
        ))
        .await?;
    let bank_missing_proof_status = bank_missing_proof.status();
    let bank_missing_proof_body = body_string(bank_missing_proof).await;
    ensure_eq!(bank_missing_proof_status, StatusCode::BAD_REQUEST);
    ensure!(bank_missing_proof_body.contains("Proof of transfer is required"));

    let before_bank_orders = count_rows(pool, "orders", user_id).await?;
    let png_bytes = b"\x89PNG\r\n\x1a\npoool-proof";
    let bank_success = app
        .clone()
        .oneshot(multipart_post(
            "/checkout",
            session,
            &[
                ("payment_method", "bank_transfer"),
                ("payment_currency", "USD"),
                ("bank_transfer_ack", "on"),
                ("disclosure_general_1", "on"),
                ("disclosure_general_2", "on"),
                ("disclosure_general_3", "on"),
            ],
            "proof_of_transfer",
            "proof.png",
            "image/png",
            png_bytes,
            &[],
        ))
        .await?;
    ensure!(bank_success.status() == StatusCode::OK);
    let bank_json = body_json(bank_success).await?;
    ensure!(bank_json["redirect_url"] == "/payment-in-progress");
    ensure_eq!(
        count_rows(pool, "orders", user_id).await?,
        before_bank_orders + 1
    );

    let bank_order = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT status, payment_method, proof_of_transfer_url
         FROM orders WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    ensure_eq!(bank_order.0, "pending");
    ensure_eq!(bank_order.1, "bank_transfer");
    ensure!(
        bank_order
            .2
            .as_deref()
            .unwrap_or_default()
            .starts_with("local-test-proof://"),
        "local development bank proof should use safe placeholder"
    );
    ensure_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::bigint FROM investments WHERE user_id = $1 AND asset_id = $2",
        )
        .bind(user_id)
        .bind(asset_id)
        .fetch_one(pool)
        .await?,
        1,
        "bank transfer must not allocate a second investment before admin approval"
    );

    let in_progress = app
        .oneshot(get_with_session("/payment-in-progress", session))
        .await?;
    ensure_eq!(in_progress.status(), StatusCode::OK);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn wallet_deposit_withdrawal_and_payment_method_paths() {
    init_tracing();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 10_000).await.expect("fixture user");
    let session = mint_session(&pool, user_id).await.expect("session");

    let result = run_wallet_paths(&pool, user_id, &session).await;
    cleanup_fixture(&pool, user_id, None).await;
    result.expect("wallet paths");
}

async fn run_wallet_paths(pool: &PgPool, user_id: Uuid, session: &str) -> Result<()> {
    let app = build_platform_router(make_state(pool.clone()));
    let fee_cents = withdrawal_fee_cents(pool).await?;

    let invalid_card = app
        .clone()
        .oneshot(form_post(
            "/api/payment-methods/card",
            session,
            &[
                ("stripe_payment_method_id", "tok_not_a_payment_method"),
                ("holder_name", "Test User"),
            ],
            &[],
        ))
        .await?;
    ensure_eq!(invalid_card.status(), StatusCode::BAD_REQUEST);
    ensure_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::bigint FROM payment_methods WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?,
        0
    );

    let bank = app
        .clone()
        .oneshot(form_post(
            "/api/payment-methods/bank",
            session,
            &[
                ("bank_name", "POOOL Test Bank"),
                ("account_holder_name", "Test User"),
                ("account_number", "DE89370400440532013000"),
                ("routing_code", "DEUTDEFF"),
                ("bank_country", "DE"),
                ("bank_system", "sepa"),
                ("label", "Primary Test Bank"),
            ],
            &[],
        ))
        .await?;
    ensure_eq!(bank.status(), StatusCode::OK);
    let bank_row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>)>(
        "SELECT method_type, processor_token, last_four, label
         FROM payment_methods WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    ensure_eq!(bank_row.0, "bank");
    ensure!(
        !bank_row
            .1
            .unwrap_or_default()
            .contains("DE89370400440532013000"),
        "stored processor token must not contain the raw account number"
    );
    ensure_eq!(bank_row.2.as_deref(), Some("3000"));
    ensure_eq!(bank_row.3.as_deref(), Some("Primary Test Bank"));

    let deposit_key = format!("deposit-init-{}", Uuid::new_v4());
    let deposit_init = app
        .clone()
        .oneshot(json_post(
            "/api/wallet/deposit/init",
            session,
            serde_json::json!({"amount": "75.00"}),
            &[("Idempotency-Key", &deposit_key)],
        ))
        .await?;
    ensure_eq!(deposit_init.status(), StatusCode::OK);
    let deposit_json = body_json(deposit_init).await?;
    let deposit_id = deposit_json["deposit_id"]
        .as_str()
        .context("deposit_id")?
        .parse::<Uuid>()?;
    ensure_eq!(deposit_json["amount_cents"], 7_500);
    ensure!(deposit_json["reference"]
        .as_str()
        .unwrap_or_default()
        .starts_with("POOOL"));

    let deposit_replay = app
        .clone()
        .oneshot(json_post(
            "/api/wallet/deposit/init",
            session,
            serde_json::json!({"amount": "75.00"}),
            &[("Idempotency-Key", &deposit_key)],
        ))
        .await?;
    ensure_eq!(deposit_replay.status(), StatusCode::OK);
    let deposit_replay_json = body_json(deposit_replay).await?;
    let deposit_id_string = deposit_id.to_string();
    ensure_eq!(
        deposit_replay_json["deposit_id"].as_str(),
        Some(deposit_id_string.as_str())
    );
    ensure_eq!(count_rows(pool, "deposit_requests", user_id).await?, 1);

    let submit_key = format!("deposit-submit-{}", Uuid::new_v4());
    let proof_bytes = b"%PDF-1.4\n%poool-test-proof\n";
    let submit = app
        .clone()
        .oneshot(multipart_post(
            &format!("/wallet/deposit/{}/submit", deposit_id),
            session,
            &[("notes", "HTTP integration proof upload")],
            "proof",
            "proof.pdf",
            "application/pdf",
            proof_bytes,
            &[("Idempotency-Key", &submit_key)],
        ))
        .await?;
    ensure!(
        submit.status().is_redirection(),
        "deposit submit should redirect, got {}",
        submit.status()
    );
    let proof_path: String = sqlx::query_scalar(
        "SELECT proof_gcs_path FROM deposit_requests WHERE id = $1 AND user_id = $2",
    )
    .bind(deposit_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    ensure!(proof_path.ends_with(".pdf"));
    ensure!(proof_path.starts_with("/uploads/deposits/"));

    let submit_replay = app
        .clone()
        .oneshot(multipart_post(
            &format!("/wallet/deposit/{}/submit", deposit_id),
            session,
            &[("notes", "HTTP integration proof upload")],
            "proof",
            "proof.pdf",
            "application/pdf",
            proof_bytes,
            &[("Idempotency-Key", &submit_key)],
        ))
        .await?;
    ensure!(submit_replay.status().is_redirection());
    ensure_eq!(count_rows(pool, "deposit_requests", user_id).await?, 1);

    let overdraw = app
        .clone()
        .oneshot(form_post(
            "/wallet/withdraw",
            session,
            &[("amount", "200.00")],
            &[],
        ))
        .await?;
    ensure!(overdraw.status().is_redirection());
    ensure!(overdraw
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .contains("insufficient_funds"));
    ensure_eq!(wallet_balance(pool, user_id).await?, 10_000);
    ensure_eq!(count_rows(pool, "withdrawal_requests", user_id).await?, 0);

    let withdraw_key = format!("withdraw-{}", Uuid::new_v4());
    let withdraw = app
        .clone()
        .oneshot(form_post(
            "/wallet/withdraw",
            session,
            &[("amount", "25.00")],
            &[("Idempotency-Key", &withdraw_key)],
        ))
        .await?;
    ensure!(withdraw.status().is_redirection());
    ensure!(withdraw
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .contains("withdraw_requested=true"));
    ensure_eq!(
        wallet_balance(pool, user_id).await?,
        10_000 - 2_500 - fee_cents
    );
    ensure_eq!(count_rows(pool, "withdrawal_requests", user_id).await?, 1);

    let withdrawal_row = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT amount_cents, fee_cents, status FROM withdrawal_requests WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    ensure_eq!(withdrawal_row, (2_500, fee_cents, "pending".to_string()));

    let withdraw_tx = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT t.amount_cents, t.type, t.status
         FROM wallet_transactions t
         JOIN wallets w ON w.id = t.wallet_id
         WHERE w.user_id = $1
         ORDER BY t.created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    ensure_eq!(
        withdraw_tx,
        (-2_500, "withdrawal".to_string(), "pending".to_string())
    );

    let withdraw_replay = app
        .oneshot(form_post(
            "/wallet/withdraw",
            session,
            &[("amount", "25.00")],
            &[("Idempotency-Key", &withdraw_key)],
        ))
        .await?;
    ensure!(withdraw_replay.status().is_redirection());
    ensure_eq!(
        wallet_balance(pool, user_id).await?,
        10_000 - 2_500 - fee_cents
    );
    ensure_eq!(count_rows(pool, "withdrawal_requests", user_id).await?, 1);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn high_value_withdrawal_requires_and_accepts_step_up_2fa_without_redis() {
    init_tracing();
    install_test_totp_key();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 100_000)
        .await
        .expect("fixture user");
    let session = mint_session(&pool, user_id).await.expect("session");
    let result = async {
        enable_totp_for_user(&pool, user_id).await?;
        let fee_cents = withdrawal_fee_cents(&pool).await?;
        let app = build_platform_router(make_state(pool.clone()));

        let blocked = app
            .clone()
            .oneshot(form_post(
                "/wallet/withdraw",
                &session,
                &[("amount", "600.00")],
                &[],
            ))
            .await?;
        ensure!(blocked.status().is_redirection());
        ensure!(blocked
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .contains("withdraw_2fa_required"));
        ensure_eq!(wallet_balance(&pool, user_id).await?, 100_000);
        ensure_eq!(count_rows(&pool, "withdrawal_requests", user_id).await?, 0);

        let verify = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/step-up/verify",
                &session,
                serde_json::json!({
                    "code": current_totp_code()?,
                    "action": "withdrawal"
                }),
                &[],
            ))
            .await?;
        let verify_status = verify.status();
        let verify_json = body_json(verify).await?;
        ensure_eq!(
            verify_status,
            StatusCode::OK,
            "step-up response: {verify_json}"
        );
        ensure_eq!(verify_json["status"], "verified");

        let db_session_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM step_up_sessions
                 WHERE user_id = $1 AND action = 'withdraw' AND expires_at > NOW()
            )",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure!(
            db_session_exists,
            "step-up DB fallback session should be active"
        );

        let withdraw_key = format!("withdraw-stepup-{}", Uuid::new_v4());
        let approved = app
            .oneshot(form_post(
                "/wallet/withdraw",
                &session,
                &[("amount", "600.00")],
                &[("Idempotency-Key", &withdraw_key)],
            ))
            .await?;
        ensure!(approved.status().is_redirection());
        ensure!(approved
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .contains("withdraw_requested=true"));
        ensure_eq!(
            wallet_balance(&pool, user_id).await?,
            100_000 - 60_000 - fee_cents
        );

        let withdrawal = sqlx::query_as::<_, (i64, i64, String)>(
            "SELECT amount_cents, fee_cents, status FROM withdrawal_requests WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(withdrawal, (60_000, fee_cents, "pending".to_string()));
        Ok(())
    }
    .await;

    cleanup_fixture(&pool, user_id, None).await;
    result.expect("high-value withdrawal step-up");
}

#[ignore]
#[tokio::test]
async fn withdrawal_safety_blocks_kyc_held_daily_velocity_and_cooldown() {
    init_tracing();
    let pool = pool().await;

    let kyc_user = insert_test_user(&pool, 10_000)
        .await
        .expect("kyc fixture user");
    let held_user = insert_test_user(&pool, 10_000)
        .await
        .expect("held fixture user");
    let daily_user = insert_test_user(&pool, 1_200_000)
        .await
        .expect("daily fixture user");
    let velocity_user = insert_test_user(&pool, 100_000)
        .await
        .expect("velocity fixture user");
    let cooldown_user = insert_test_user(&pool, 200_000)
        .await
        .expect("cooldown fixture user");

    let kyc_session = mint_session(&pool, kyc_user).await.expect("kyc session");
    let held_session = mint_session(&pool, held_user).await.expect("held session");
    let daily_session = mint_session(&pool, daily_user)
        .await
        .expect("daily session");
    let velocity_session = mint_session(&pool, velocity_user)
        .await
        .expect("velocity session");
    let cooldown_session = mint_session(&pool, cooldown_user)
        .await
        .expect("cooldown session");

    let result = async {
        let app = build_platform_router(make_state(pool.clone()));

        sqlx::query(
            "UPDATE kyc_records
             SET status = 'pending', verified_at = NULL
             WHERE user_id = $1",
        )
        .bind(kyc_user)
        .execute(&pool)
        .await?;
        let kyc_blocked = app
            .clone()
            .oneshot(form_post(
                "/wallet/withdraw",
                &kyc_session,
                &[("amount", "25.00")],
                &[],
            ))
            .await?;
        ensure_redirect_error(&kyc_blocked, "kyc_required")?;
        ensure_eq!(wallet_balance(&pool, kyc_user).await?, 10_000);
        ensure_eq!(count_rows(&pool, "withdrawal_requests", kyc_user).await?, 0);

        sqlx::query(
            "UPDATE wallets
             SET held_balance_cents = 8000
             WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
        )
        .bind(held_user)
        .execute(&pool)
        .await?;
        let held_blocked = app
            .clone()
            .oneshot(form_post(
                "/wallet/withdraw",
                &held_session,
                &[("amount", "25.00")],
                &[],
            ))
            .await?;
        ensure_redirect_error(&held_blocked, "insufficient_funds")?;
        let held_wallet = sqlx::query_as::<_, (i64, i64)>(
            "SELECT balance_cents, held_balance_cents
             FROM wallets
             WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD'",
        )
        .bind(held_user)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(held_wallet, (10_000, 8_000));
        ensure_eq!(
            count_rows(&pool, "withdrawal_requests", held_user).await?,
            0
        );

        sqlx::query(
            "INSERT INTO withdrawal_requests (user_id, amount_cents, currency, status, created_at)
             VALUES ($1, 990000, 'USD', 'pending', NOW())",
        )
        .bind(daily_user)
        .execute(&pool)
        .await?;
        let daily_blocked = app
            .clone()
            .oneshot(form_post(
                "/wallet/withdraw",
                &daily_session,
                &[("amount", "200.00")],
                &[],
            ))
            .await?;
        ensure_redirect_error(&daily_blocked, "withdraw_daily_cap")?;
        ensure_eq!(wallet_balance(&pool, daily_user).await?, 1_200_000);
        ensure_eq!(
            count_rows(&pool, "withdrawal_requests", daily_user).await?,
            1
        );

        for minutes_ago in 1..=4 {
            sqlx::query(
                "INSERT INTO withdrawal_requests
                    (user_id, amount_cents, currency, status, created_at)
                 VALUES ($1, 1000, 'USD', 'pending', NOW() - ($2::int || ' minutes')::interval)",
            )
            .bind(velocity_user)
            .bind(minutes_ago)
            .execute(&pool)
            .await?;
        }
        let velocity_blocked = app
            .clone()
            .oneshot(form_post(
                "/wallet/withdraw",
                &velocity_session,
                &[("amount", "25.00")],
                &[],
            ))
            .await?;
        ensure_redirect_error(&velocity_blocked, "withdraw_velocity_frozen")?;
        ensure_eq!(wallet_balance(&pool, velocity_user).await?, 100_000);
        ensure_eq!(
            count_rows(&pool, "withdrawal_requests", velocity_user).await?,
            4
        );
        let frozen_status: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1")
            .bind(velocity_user)
            .fetch_one(&pool)
            .await?;
        ensure_eq!(frozen_status, "frozen".to_string());

        sqlx::query(
            "UPDATE kyc_records
             SET status = 'approved', verified_at = NOW()
             WHERE user_id = $1",
        )
        .bind(cooldown_user)
        .execute(&pool)
        .await?;
        enable_totp_for_user(&pool, cooldown_user).await?;
        sqlx::query(
            "INSERT INTO step_up_sessions (user_id, action, expires_at)
             VALUES ($1, 'withdraw', NOW() + INTERVAL '15 minutes')
             ON CONFLICT (user_id, action)
             DO UPDATE SET expires_at = EXCLUDED.expires_at, updated_at = NOW()",
        )
        .bind(cooldown_user)
        .execute(&pool)
        .await?;
        let cooldown_blocked = app
            .oneshot(form_post(
                "/wallet/withdraw",
                &cooldown_session,
                &[("amount", "1200.00")],
                &[],
            ))
            .await?;
        ensure_redirect_error(&cooldown_blocked, "withdraw_new_account_cooldown")?;
        ensure_eq!(wallet_balance(&pool, cooldown_user).await?, 200_000);
        ensure_eq!(
            count_rows(&pool, "withdrawal_requests", cooldown_user).await?,
            0
        );

        Ok(())
    }
    .await;

    for user_id in [
        kyc_user,
        held_user,
        daily_user,
        velocity_user,
        cooldown_user,
    ] {
        cleanup_fixture(&pool, user_id, None).await;
    }

    result.expect("withdrawal safety blockers");
}

#[ignore]
#[tokio::test]
async fn frozen_user_unfreeze_request_and_admin_reactivation_flow() {
    init_tracing();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 10_000).await.expect("fixture user");
    let admin_id = insert_test_admin_user(&pool).await.expect("fixture admin");
    let session = mint_session(&pool, user_id).await.expect("session");
    let admin_session = mint_session(&pool, admin_id).await.expect("admin session");

    let result = async {
        let app = build_platform_router(make_state(pool.clone()));

        let not_frozen = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/unfreeze-request",
                &session,
                serde_json::json!({"note": "Please review my account."}),
                &[],
            ))
            .await?;
        ensure_eq!(not_frozen.status(), StatusCode::CONFLICT);
        let not_frozen_json = body_json(not_frozen).await?;
        ensure_eq!(not_frozen_json["error"], "not_frozen");
        ensure_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)::bigint FROM compliance_alerts WHERE user_id = $1",
            )
            .bind(user_id)
            .fetch_one(&pool)
            .await?,
            0
        );

        sqlx::query(
            "UPDATE users
             SET status = 'frozen',
                 frozen_at = NOW() - INTERVAL '2 hours',
                 frozen_reason = 'withdrawal_velocity',
                 unfreeze_requested_at = NULL
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&pool)
        .await?;

        let long_note = format!("  {}  ", "x".repeat(620));
        let requested = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/unfreeze-request",
                &session,
                serde_json::json!({"note": long_note}),
                &[],
            ))
            .await?;
        ensure_eq!(requested.status(), StatusCode::OK);
        let requested_json = body_json(requested).await?;
        ensure_eq!(requested_json["status"], "requested");

        let unfreeze_requested_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT unfreeze_requested_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await?;
        ensure!(
            unfreeze_requested_at.is_some(),
            "unfreeze request should stamp the user row"
        );

        let alert = sqlx::query_as::<_, (String, String, String, Option<String>)>(
            "SELECT kind, severity, summary, details->>'user_note'
             FROM compliance_alerts
             WHERE user_id = $1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(alert.0, "manual_review".to_string());
        ensure_eq!(alert.1, "high".to_string());
        ensure!(alert.2.contains("withdrawal_velocity"));
        let stored_note = alert.3.context("stored alert note")?;
        ensure_eq!(stored_note.len(), 500);
        ensure!(stored_note.chars().all(|c| c == 'x'));

        let duplicate = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/unfreeze-request",
                &session,
                serde_json::json!({"note": "second request"}),
                &[],
            ))
            .await?;
        ensure_eq!(duplicate.status(), StatusCode::TOO_MANY_REQUESTS);
        let duplicate_json = body_json(duplicate).await?;
        ensure_eq!(duplicate_json["error"], "already_requested");
        ensure_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)::bigint FROM compliance_alerts WHERE user_id = $1",
            )
            .bind(user_id)
            .fetch_one(&pool)
            .await?,
            1
        );

        let reactivated = app
            .clone()
            .oneshot(json_post(
                &format!("/api/admin/users/{user_id}/status"),
                &admin_session,
                serde_json::json!({"status": "active"}),
                &[],
            ))
            .await?;
        ensure_eq!(reactivated.status(), StatusCode::OK);
        let reactivated_json = body_json(reactivated).await?;
        ensure_eq!(reactivated_json["success"], true);

        let user_state = sqlx::query_as::<
            _,
            (
                String,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<String>,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            "SELECT status, frozen_at, frozen_reason, unfreeze_requested_at
             FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(user_state.0, "active".to_string());
        ensure!(
            user_state.1.is_none(),
            "reactivation should clear frozen_at"
        );
        ensure!(
            user_state.2.is_none(),
            "reactivation should clear frozen_reason"
        );
        ensure!(
            user_state.3.is_none(),
            "reactivation should clear unfreeze request marker"
        );

        let audit_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::bigint
             FROM audit_logs
             WHERE actor_user_id = $1
               AND action = 'admin.user_status_update'
               AND entity_id = $2
               AND new_state->>'status' = 'active'",
        )
        .bind(admin_id)
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(audit_count, 1);

        Ok(())
    }
    .await;

    cleanup_fixture(&pool, user_id, None).await;
    cleanup_fixture(&pool, admin_id, None).await;
    result.expect("unfreeze request and admin reactivation");
}

#[ignore]
#[tokio::test]
async fn deposit_source_of_funds_thresholds_and_supporting_document_paths() {
    init_tracing();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 0).await.expect("fixture user");
    let session = mint_session(&pool, user_id).await.expect("session");
    let result = async {
        let app = build_platform_router(make_state(pool.clone()));

        let missing_reason = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/deposit/init",
                &session,
                serde_json::json!({"amount": "3000.00"}),
                &[("Idempotency-Key", "sof-missing-reason")],
            ))
            .await?;
        ensure_eq!(missing_reason.status(), StatusCode::BAD_REQUEST);
        let missing_reason_json = body_json(missing_reason).await?;
        ensure_eq!(missing_reason_json["error"], "sof_reason_required");

        let missing_detail = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/deposit/init",
                &session,
                serde_json::json!({
                    "amount": "3000.00",
                    "source_of_funds_reason": "other"
                }),
                &[("Idempotency-Key", "sof-missing-detail")],
            ))
            .await?;
        ensure_eq!(missing_detail.status(), StatusCode::BAD_REQUEST);
        let missing_detail_json = body_json(missing_detail).await?;
        ensure_eq!(missing_detail_json["error"], "sof_detail_required");
        ensure_eq!(count_rows(&pool, "deposit_requests", user_id).await?, 0);

        let init = app
            .clone()
            .oneshot(json_post(
                "/api/wallet/deposit/init",
                &session,
                serde_json::json!({
                    "amount": "10000.00",
                    "source_of_funds_reason": "savings",
                    "source_of_funds_detail": "Long-term savings from salary."
                }),
                &[("Idempotency-Key", "sof-large-init")],
            ))
            .await?;
        ensure_eq!(init.status(), StatusCode::OK);
        let init_json = body_json(init).await?;
        ensure_eq!(init_json["sof_required"], true);
        ensure_eq!(init_json["sof_doc_required"], true);
        let deposit_id = Uuid::parse_str(
            init_json["deposit_id"]
                .as_str()
                .context("deposit id in JSON")?,
        )?;

        let stored_sof = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT source_of_funds_reason, source_of_funds_detail
             FROM deposit_requests WHERE id = $1",
        )
        .bind(deposit_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(
            stored_sof,
            (
                Some("savings".to_string()),
                Some("Long-term savings from salary.".to_string())
            )
        );

        let proof_bytes = b"%PDF-1.4\n%poool-proof\n";
        let missing_doc = app
            .clone()
            .oneshot(multipart_post(
                &format!("/wallet/deposit/{deposit_id}/submit"),
                &session,
                &[("notes", "Wire sent without SoF doc")],
                "proof",
                "proof.pdf",
                "application/pdf",
                proof_bytes,
                &[("Idempotency-Key", "sof-submit-missing-doc")],
            ))
            .await?;
        ensure!(missing_doc.status().is_redirection());
        ensure!(missing_doc
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .contains("sof_doc_required"));

        let submitted = app
            .oneshot(multipart_post_files(
                &format!("/wallet/deposit/{deposit_id}/submit"),
                &session,
                &[("notes", "Wire sent with SoF doc")],
                &[
                    (
                        "proof",
                        "proof.pdf",
                        "application/pdf",
                        proof_bytes.as_slice(),
                    ),
                    (
                        "source_of_funds_doc",
                        "source-of-funds.pdf",
                        "application/pdf",
                        proof_bytes.as_slice(),
                    ),
                ],
                &[("Idempotency-Key", "sof-submit-with-doc")],
            ))
            .await?;
        ensure!(submitted.status().is_redirection());
        ensure!(submitted
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .contains("deposit_completed=true"));

        let paths = sqlx::query_as::<_, (Option<String>, Option<String>)>(
            "SELECT proof_gcs_path, source_of_funds_doc_path
             FROM deposit_requests WHERE id = $1",
        )
        .bind(deposit_id)
        .fetch_one(&pool)
        .await?;
        ensure!(paths.0.as_deref().unwrap_or_default().contains("deposits/"));
        ensure!(paths.1.as_deref().unwrap_or_default().contains("-sof."));

        Ok(())
    }
    .await;

    cleanup_fixture(&pool, user_id, None).await;
    result.expect("source-of-funds deposit paths");
}

#[ignore]
#[tokio::test]
async fn referral_checkout_requires_referral_disclosures_and_bank_approval_allocates_once() {
    init_tracing();
    let pool = pool().await;
    let user_id = insert_test_user(&pool, 0).await.expect("fixture user");
    let asset_id = insert_test_asset(&pool, user_id)
        .await
        .expect("fixture asset");
    let session = mint_session(&pool, user_id).await.expect("session");
    let affiliate_user_id = insert_personal_affiliate_referral(&pool, user_id)
        .await
        .expect("fixture affiliate");
    let result = async {
        let app = build_platform_router(make_state(pool.clone()));

        let add = app
            .clone()
            .oneshot(form_post(
                "/cart/add",
                &session,
                &[
                    ("property_id", &asset_id.to_string()),
                    ("investment_amount", "30.00"),
                ],
                &[],
            ))
            .await?;
        ensure!(add.status().is_redirection());
        ensure_eq!(
            cart_item(&pool, user_id, asset_id).await?.map(|(_, q)| q),
            Some(3)
        );

        let referral_disclosure_block = app
            .clone()
            .oneshot(form_post(
                "/checkout",
                &session,
                &[
                    ("payment_method", "bank_transfer"),
                    ("payment_currency", "USD"),
                    ("bank_transfer_ack", "on"),
                    ("disclosure_general_1", "on"),
                    ("disclosure_general_2", "on"),
                    ("disclosure_general_3", "on"),
                ],
                &[],
            ))
            .await?;
        let block_status = referral_disclosure_block.status();
        let block_body = body_string(referral_disclosure_block).await;
        ensure_eq!(block_status, StatusCode::BAD_REQUEST);
        ensure!(block_body.contains("referral disclosures"));
        ensure_eq!(count_rows(&pool, "orders", user_id).await?, 0);

        let proof_bytes = b"\x89PNG\r\n\x1a\npoool-proof";
        let bank_checkout = app
            .clone()
            .oneshot(multipart_post(
                "/checkout",
                &session,
                &[
                    ("payment_method", "bank_transfer"),
                    ("payment_currency", "USD"),
                    ("bank_transfer_ack", "on"),
                    ("disclosure_general_1", "on"),
                    ("disclosure_general_2", "on"),
                    ("disclosure_general_3", "on"),
                    ("disclosure_referral_1", "on"),
                    ("disclosure_referral_2", "on"),
                    ("disclosure_referral_3", "on"),
                ],
                "proof_of_transfer",
                "proof.png",
                "image/png",
                proof_bytes,
                &[("Idempotency-Key", "referral-bank-checkout")],
            ))
            .await?;
        ensure_eq!(bank_checkout.status(), StatusCode::OK);
        let bank_json = body_json(bank_checkout).await?;
        ensure_eq!(bank_json["redirect_url"], "/payment-in-progress");

        let order = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT id, status, payment_method
             FROM orders WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(order.1, "pending");
        ensure_eq!(order.2, "bank_transfer");
        ensure_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*)::bigint FROM investments WHERE user_id = $1 AND asset_id = $2",
            )
            .bind(user_id)
            .bind(asset_id)
            .fetch_one(&pool)
            .await?,
            0,
            "bank transfer must not allocate before admin approval"
        );

        let disclosure = sqlx::query_as::<_, (bool, bool, Option<bool>)>(
            "SELECT is_referral_user, agreed_to_general, agreed_to_referral
             FROM investment_disclosures_log WHERE user_id = $1 AND order_id = $2",
        )
        .bind(user_id)
        .bind(order.0)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(disclosure, (true, true, Some(true)));

        let approved =
            poool_backend::payments::service::approve_order(&pool, order.0, affiliate_user_id)
                .await
                .map_err(anyhow::Error::msg)?;
        ensure_eq!(approved.0, user_id);
        ensure_eq!(approved.1, vec![asset_id]);

        let completed_status: String =
            sqlx::query_scalar("SELECT status FROM orders WHERE id = $1")
                .bind(order.0)
                .fetch_one(&pool)
                .await?;
        ensure_eq!(completed_status, "completed".to_string());

        let investment = sqlx::query_as::<_, (i32, i64, String)>(
            "SELECT tokens_owned, purchase_value_cents, status
             FROM investments WHERE user_id = $1 AND asset_id = $2",
        )
        .bind(user_id)
        .bind(asset_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(investment, (3, 3_000, "active".to_string()));

        let commission = sqlx::query_as::<_, (i64, Option<i64>, Option<i64>, Option<String>)>(
            "SELECT COUNT(*)::bigint,
                    SUM(provisional_amount_cents)::bigint,
                    SUM(gross_amount_cents)::bigint,
                    MIN(TRIM(currency))
             FROM affiliate_commissions WHERE source_order_id = $1",
        )
        .bind(order.0)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(
            commission,
            (1, Some(15), Some(3_150), Some("USD".to_string()))
        );

        let referral_state = sqlx::query_as::<_, (String, bool)>(
            "SELECT status, qualifying_investment_id IS NOT NULL
             FROM affiliate_referrals WHERE referred_user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&pool)
        .await?;
        ensure_eq!(referral_state, ("under_holdback".to_string(), true));

        Ok(())
    }
    .await;

    cleanup_fixture(&pool, affiliate_user_id, None).await;
    cleanup_fixture(&pool, user_id, Some(asset_id)).await;
    result.expect("referral bank approval path");
}
