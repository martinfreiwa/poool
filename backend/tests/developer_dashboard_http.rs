//! HTTP-level integration tests for the Developer Dashboard surface
//! (`/developer/dashboard`, `/api/developer/dashboard/stats`, and the two
//! HTMX fragment endpoints).
//!
//! These exercise the real Axum router that the production binary boots —
//! same middleware stack (CSRF, security headers), same auth wiring. The
//! router is built via the library entry point
//! `poool_backend::build_platform_router`.
//!
//! ## Running
//!
//! All tests are `#[ignore]`d because they require a live Postgres
//! database. To run them:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test developer_dashboard_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! For each endpoint the suite asserts three cases:
//!   1. 200 OK with a valid developer session
//!   2. 401 Unauthorized when no session cookie is presented
//!   3. 403 Forbidden (or 302 redirect for page routes) when the session
//!      belongs to a non-developer user
//!
//! Endpoints under test:
//!   * GET  /developer/dashboard                      (page render)
//!   * GET  /api/developer/dashboard/stats            (JSON stats)
//!   * GET  /developer/dashboard/fragments/chart      (HTMX fragment)
//!   * GET  /developer/dashboard/fragments/assets     (HTMX fragment)
//!
//! Each test seeds its own user(s) with UUID-isolated emails, runs the
//! request, and cleans up at the end via `DELETE FROM users WHERE id = $1`
//! (which cascades through `user_roles` and `user_sessions`).

#![cfg(test)]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use poool_backend::{build_platform_router, AppState};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

// ──────────────────────────────────────────────────────────────────────
// Shared test infrastructure (mirrors leaderboard_http.rs deliberately so
// the cross-file pattern stays consistent for future maintainers).
// ──────────────────────────────────────────────────────────────────────

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to test DB")
}

fn make_state(pool: PgPool) -> AppState {
    if std::env::var("DATABASE_URL").is_err() {
        panic!("DATABASE_URL must be set for HTTP integration tests");
    }
    std::env::set_var("POOOL_ENV", "development");
    let config = poool_backend::config::Config::from_env();

    AppState {
        db: pool.clone(),
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

/// Seed a minimal user. `email_verified = TRUE` is required because the
/// session middleware filters unverified accounts.
async fn insert_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status, email_verified)
         VALUES ($1, $2, 'x', 'active', TRUE)",
    )
    .bind(id)
    .bind(format!("{}@dev-dash.test", id))
    .execute(pool)
    .await
    .expect("insert user");
    id
}

/// Insert a 24-hour session row for `user_id` and return the token to
/// attach as a `poool_session` cookie.
async fn mint_session(pool: &PgPool, user_id: Uuid) -> String {
    let token = format!("test-{}-{}", user_id.simple(), Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO user_sessions
            (user_id, session_token, ip_address, user_agent, remember_me, is_2fa_verified, expires_at)
           VALUES ($1, $2, NULL, 'test', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
    )
    .bind(user_id)
    .bind(&token)
    .execute(pool)
    .await
    .expect("insert session");
    token
}

/// Grant the `developer` role to `user_id`. Looks up the role id rather than
/// hard-coding so this still works against fresh schemas.
async fn grant_developer(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_roles (user_id, role_id, is_active)
           SELECT $1, r.id, TRUE FROM roles r WHERE r.name = 'developer'
           ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("grant developer role");
}

/// Best-effort cleanup. ON DELETE CASCADE on `user_roles` and
/// `user_sessions` drops dependents.
async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

fn get_with_session(uri: &str, session_token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = session_token {
        builder = builder.header("cookie", format!("poool_session={}", t));
    }
    builder.body(Body::empty()).unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

// ──────────────────────────────────────────────────────────────────────
// /developer/dashboard — page render
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn dashboard_page_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/dashboard", Some(&session)))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    // Assert against a stable structural ID that the template guarantees.
    assert!(
        body.contains("developer-dashboard-body") || body.contains("dashboard-content-wrapper"),
        "expected developer dashboard markers in HTML; got body starting with: {:.200}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn dashboard_page_redirects_anon_to_login() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/dashboard", None))
        .await
        .expect("oneshot");
    // Page handlers redirect (302/303/307) anonymous users to /auth/login.
    assert!(
        resp.status().is_redirection(),
        "expected redirect for anonymous request, got {}",
        resp.status()
    );
}

#[ignore]
#[tokio::test]
async fn dashboard_page_redirects_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/dashboard", Some(&session)))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    // Page handler redirects non-developer users to /developer/application-form.
    assert!(
        status.is_redirection(),
        "expected redirect for non-developer user, got {}",
        status
    );
}

// ──────────────────────────────────────────────────────────────────────
// /api/developer/dashboard/stats — JSON
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn dashboard_stats_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/api/developer/dashboard/stats",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    // Body must be valid JSON (the handler returns a JSON object).
    let _: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("invalid json: {e}; body: {body}"));
}

#[ignore]
#[tokio::test]
async fn dashboard_stats_returns_401_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/api/developer/dashboard/stats", None))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn dashboard_stats_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/api/developer/dashboard/stats",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// /developer/dashboard/fragments/chart — HTMX fragment
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn fragment_chart_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/developer/dashboard/fragments/chart?period=all",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK);
}

#[ignore]
#[tokio::test]
async fn fragment_chart_returns_401_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/developer/dashboard/fragments/chart",
            None,
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn fragment_chart_redirects_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/developer/dashboard/fragments/chart",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    // Fragment handler issues a 303 redirect to /marketplace for non-devs.
    assert!(
        status.is_redirection(),
        "expected redirect for non-developer user, got {}",
        status
    );
}

// ──────────────────────────────────────────────────────────────────────
// /developer/dashboard/fragments/assets — HTMX fragment
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn fragment_assets_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/developer/dashboard/fragments/assets?period=all",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK);
}

#[ignore]
#[tokio::test]
async fn fragment_assets_returns_401_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/developer/dashboard/fragments/assets",
            None,
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn fragment_assets_redirects_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/developer/dashboard/fragments/assets",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert!(
        status.is_redirection(),
        "expected redirect for non-developer user, got {}",
        status
    );
}
