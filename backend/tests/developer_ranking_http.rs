//! HTTP-level integration tests for the Developer Ranking surface.
//!
//! The Developer Ranking page is an alias of the standard leaderboard
//! embedded inside the developer shell, so the API surface it relies on
//! is `/api/leaderboard/me`. We verify both the page render and the
//! underlying API.
//!
//! ## Running
//!
//! All tests are `#[ignore]`d because they require a live Postgres
//! database. To run them:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test developer_ranking_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! Page handler:
//!   * GET /developer/ranking                  (developer-shell wrapper)
//!
//! API handler:
//!   * GET /api/leaderboard/me                 (rank + JSON shape with `rank`)
//!   * GET /api/leaderboard/me (anon)          (401)

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
// Shared infrastructure
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

async fn insert_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status, email_verified)
         VALUES ($1, $2, 'x', 'active', TRUE)",
    )
    .bind(id)
    .bind(format!("{}@dev-rank.test", id))
    .execute(pool)
    .await
    .expect("insert user");
    id
}

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

/// Seed a leaderboard_scores row so the `/me` endpoint can resolve a rank.
async fn seed_leaderboard_row(pool: &PgPool, user_id: Uuid, total_cents: i64, rank: i32) {
    sqlx::query(
        r#"INSERT INTO leaderboard_scores (
                user_id, total_invested_cents, asset_count, portfolio_roi_bps,
                affiliate_count, referral_network_value_cents, highest_investment_cents,
                computed_at, rank_invested
            ) VALUES ($1, $2, 1, 0, 0, 0, $2, NOW(), $3)
            ON CONFLICT (user_id) DO UPDATE SET
                total_invested_cents = EXCLUDED.total_invested_cents,
                rank_invested = EXCLUDED.rank_invested,
                computed_at = NOW()"#,
    )
    .bind(user_id)
    .bind(total_cents)
    .bind(rank)
    .execute(pool)
    .await
    .expect("seed leaderboard row");
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM leaderboard_scores WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM leaderboard_preferences WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
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
// Page handler
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn page_ranking_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/ranking", Some(&session)))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("leaderboard-body") || body.contains("lb-rankings-body"),
        "expected leaderboard markers; got: {:.200}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn page_ranking_redirects_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/ranking", None))
        .await
        .expect("oneshot");
    assert!(
        resp.status().is_redirection(),
        "expected redirect for anon, got {}",
        resp.status()
    );
}

// ──────────────────────────────────────────────────────────────────────
// API handler (alias surface)
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn me_returns_rank_for_developer_session() {
    // The developer ranking page consumes /api/leaderboard/me; verify it
    // returns 200 + a `rank` field for an authenticated developer.
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    seed_leaderboard_row(&pool, user, 10_000, 1).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/api/leaderboard/me", Some(&session)))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("invalid json: {e}; body: {body}"));
    assert!(
        parsed.get("rank").is_some(),
        "response JSON must include a `rank` field; got {body}"
    );
}

#[ignore]
#[tokio::test]
async fn me_returns_401_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/api/leaderboard/me", None))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
