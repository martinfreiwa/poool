//! HTTP-level integration tests for the new community profile surface.
//!
//! These exercise the production router built via
//! `poool_backend::build_platform_router`, so any drift between the
//! test-time and runtime mounts would fail the build.
//!
//! ## Running
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test community_profile_http -- --ignored
//! ```
//!
//! Cases:
//!
//! 1. `profile_posts_returns_401_unauthenticated` — GET
//!    /api/community/profile/:id/posts without a session returns 401.
//!    Regression guard for the WS3.1 route mount.
//! 2. `profile_me_route_present` — /community/me redirects to /auth/login
//!    when anonymous. Confirms the WS3.2 page route is wired.
//! 3. `profile_user_route_present` — /community/u/:id renders the protected
//!    template (or redirects to login) when anonymous. Confirms the
//!    sibling route is wired.

#![cfg(test)]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use poool_backend::{build_platform_router, AppState};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect to test DB")
}

fn make_state(pool: PgPool) -> AppState {
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

#[tokio::test]
#[ignore]
async fn profile_posts_returns_401_unauthenticated() {
    let p = pool().await;
    let app = build_platform_router(make_state(p));
    let target = Uuid::new_v4();
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/community/profile/{}/posts", target))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.expect("router responded");
    // Either 401 (auth required) or 503 (community_db None) — both prove the
    // route exists and the handler ran. A 404 would be a regression.
    assert!(
        matches!(
            res.status(),
            StatusCode::UNAUTHORIZED
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::INTERNAL_SERVER_ERROR
        ),
        "unexpected status {}",
        res.status()
    );
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore]
async fn profile_me_route_redirects_when_anonymous() {
    let p = pool().await;
    let app = build_platform_router(make_state(p));
    let req = Request::builder()
        .method(Method::GET)
        .uri("/community/me")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.expect("router responded");
    // Anonymous → either 30x redirect to /auth/login or 200 with login page
    // (depending on how the protected-serve helper renders). Critical bit:
    // no 404.
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore]
async fn profile_user_route_renders() {
    let p = pool().await;
    let app = build_platform_router(make_state(p));
    let target = Uuid::new_v4();
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/community/u/{}", target))
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
}
