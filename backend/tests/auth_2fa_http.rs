//! HTTP-level integration tests for the TOTP setup and step-up routes.
//!
//! These tests exercise the production Axum router through
//! `poool_backend::build_platform_router`. They are ignored by default because
//! they require a live local Postgres database.
//!
//! Running:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test auth_2fa_http -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(test)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt;
use poool_backend::{build_platform_router, AppState};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use tower::ServiceExt;
use uuid::Uuid;

const TEST_TOTP_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A15 (CDDRP Phase 3.2): hard-block running integration tests against a
/// non-local Postgres. Tests `set_var` deterministic encryption keys; if a
/// developer accidentally pointed `DATABASE_URL` at production and ran
/// `cargo test -- --ignored`, real ciphertext columns would be re-encrypted
/// under the public test key. This guard refuses to connect in that case.
mod safety {
    pub fn assert_database_url_is_local() {
        let url = std::env::var("DATABASE_URL").unwrap_or_default();
        // Allow: localhost, 127.0.0.1, ::1, unix socket (/cloudsql dev), empty,
        // or the dev-user convention `postgres://martin@…`.
        let is_local = url.is_empty()
            || url.contains("@localhost")
            || url.contains("@127.0.0.1")
            || url.contains("@[::1]")
            || url.contains("@/")
            || url.starts_with("postgres://martin@");
        if !is_local {
            panic!(
                "Refusing to run integration tests against non-local DATABASE_URL: {} \
                 — set DATABASE_URL=postgres://martin@localhost/poool first",
                url.split('@').nth(1).unwrap_or("(redacted)")
            );
        }
    }
}

fn install_test_totp_key() {
    safety::assert_database_url_is_local();
    std::env::set_var("TOTP_SECRET_ENCRYPTION_KEY", TEST_TOTP_KEY);
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
    safety::assert_database_url_is_local();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to test DB")
}

fn make_state(pool: PgPool) -> AppState {
    std::env::set_var("POOOL_ENV", "development");
    install_test_totp_key();

    let config = poool_backend::config::Config::from_env();
    // `Config::from_env` loads local dotenv files. Re-install the test key so a
    // blank local `.env` value cannot mask the explicit test configuration.
    install_test_totp_key();

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
    }
}

async fn insert_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, email_verified, status)
         VALUES ($1, $2, 'x', TRUE, 'active')",
    )
    .bind(user_id)
    .bind(format!("{}@auth-2fa-http.test", user_id))
    .execute(pool)
    .await
    .expect("insert user");
    user_id
}

async fn mint_session(pool: &PgPool, user_id: Uuid) -> String {
    let token = format!("test-{}-{}", user_id.simple(), Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO user_sessions
            (user_id, session_token, ip_address, user_agent, remember_me, is_2fa_verified, expires_at)
           VALUES ($1, $2, NULL, 'auth-2fa-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
    )
    .bind(user_id)
    .bind(&token)
    .execute(pool)
    .await
    .expect("insert session");
    token
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM audit_logs WHERE actor_user_id = $1")
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
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

fn get_with_session(uri: &str, session_token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::COOKIE, format!("poool_session={}", session_token))
        .body(Body::empty())
        .unwrap()
}

fn form_post_with_session(
    uri: &str,
    session_token: &str,
    fields: &[(&str, &str)],
) -> Request<Body> {
    let csrf = "test-csrf-2fa";
    let mut encoded = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in fields {
        encoded.append_pair(key, value);
    }

    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(encoded.finish()))
        .unwrap()
}

fn json_post_with_session(
    uri: &str,
    session_token: &str,
    json_body: serde_json::Value,
) -> Request<Body> {
    let csrf = "test-csrf-2fa";
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            header::COOKIE,
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&json_body).unwrap()))
        .unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn current_totp_code(secret_b32: &str) -> String {
    let secret_bytes = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .expect("decode totp secret");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("POOOL".to_string()),
        "auth-2fa-http".to_string(),
    )
    .expect("build totp");
    totp.generate_current().expect("generate current totp")
}

fn extract_totp_secret(html: &str) -> String {
    let marker = "id=\"totp-secret\"";
    let start = html.find(marker).expect("totp secret marker");
    let after_open = html[start..].find('>').expect("totp secret open tag") + start + 1;
    let end = html[after_open..]
        .find("</code>")
        .expect("totp secret close tag")
        + after_open;
    html[after_open..end].trim().to_string()
}

fn extract_setup_token(html: &str) -> String {
    let marker = "name=\"setup_token\" value=\"";
    let start = html.find(marker).expect("setup token input") + marker.len();
    let end = html[start..].find('"').expect("setup token value end") + start;
    html[start..end].to_string()
}

#[ignore]
#[tokio::test]
async fn totp_setup_get_and_submit_enable_encrypted_secret() {
    init_tracing();
    install_test_totp_key();
    let pool = pool().await;
    let user_id = insert_user(&pool).await;
    let session = mint_session(&pool, user_id).await;
    let app = build_platform_router(make_state(pool.clone()));

    let get_resp = app
        .clone()
        .oneshot(get_with_session("/auth/2fa/setup", &session))
        .await
        .expect("setup GET");
    let get_status = get_resp.status();
    let get_body = body_string(get_resp).await;

    assert_eq!(get_status, StatusCode::OK, "body: {}", get_body);
    assert!(get_body.contains("Set up two-factor authentication"));
    assert!(get_body.contains("data:image/png;base64,"));

    let secret = extract_totp_secret(&get_body);
    let setup_token = extract_setup_token(&get_body);
    let code = current_totp_code(&secret);

    let post_resp = app
        .oneshot(form_post_with_session(
            "/auth/2fa/setup",
            &session,
            &[("setup_token", &setup_token), ("code", &code)],
        ))
        .await
        .expect("setup POST");
    let post_status = post_resp.status();
    let post_body = body_string(post_resp).await;

    assert_eq!(post_status, StatusCode::OK, "body: {}", post_body);
    assert!(post_body.contains("Two-factor authentication connected successfully"));

    let row = sqlx::query_as::<_, (bool, Option<String>)>(
        "SELECT totp_enabled, totp_secret FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("fetch user settings");

    cleanup_user(&pool, user_id).await;

    assert!(row.0, "TOTP should be enabled");
    let stored_secret = row.1.expect("stored totp secret");
    assert!(stored_secret.starts_with("enc:v1:"));
    assert_ne!(stored_secret, secret);
    assert!(!stored_secret.contains(&secret));
}

#[ignore]
#[tokio::test]
async fn step_up_get_and_post_accept_enrolled_totp_code() {
    init_tracing();
    install_test_totp_key();
    let pool = pool().await;
    let user_id = insert_user(&pool).await;
    let session = mint_session(&pool, user_id).await;
    let (secret, _, _) =
        poool_backend::auth::service::generate_totp_secret("step-up@auth-2fa-http.test")
            .expect("generate totp secret");
    poool_backend::auth::service::enable_totp(&pool, user_id, &secret)
        .await
        .expect("enable totp");
    let app = build_platform_router(make_state(pool.clone()));

    let get_resp = app
        .clone()
        .oneshot(get_with_session(
            "/auth/2fa/step-up?return_to=/wallet&action=withdrawal",
            &session,
        ))
        .await
        .expect("step-up GET");
    let get_status = get_resp.status();
    let get_body = body_string(get_resp).await;

    assert_eq!(get_status, StatusCode::OK, "body: {}", get_body);
    assert!(get_body.contains("code-input"), "body: {}", get_body);

    let code = current_totp_code(&secret);
    let post_resp = app
        .oneshot(json_post_with_session(
            "/auth/2fa/step-up",
            &session,
            serde_json::json!({
                "code": code,
                "action": "withdrawal"
            }),
        ))
        .await
        .expect("step-up POST");
    let post_status = post_resp.status();
    let post_body = body_string(post_resp).await;

    cleanup_user(&pool, user_id).await;

    assert_eq!(post_status, StatusCode::OK, "body: {}", post_body);
    assert!(post_body.contains("\"success\":true"));
}
