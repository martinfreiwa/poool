//! HTTP-level integration tests for the 2026-05-16 MyCircle rework
//! endpoints. Mirrors the pattern in `community_profile_http.rs`:
//! `community_db: None` so handlers that need the community pool return
//! 503; auth + routing layers still exercise normally.
//!
//! ## Running
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test community_circles_http -- --ignored
//! ```

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

fn req(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn req_json(method: Method, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// ─── 1. Discover route mounted ────────────────────────────────────────
#[ignore]
#[tokio::test]
async fn discover_route_mounted() {
    let app = build_platform_router(make_state(pool().await));
    let res = app
        .oneshot(req(Method::GET, "/api/community/circles/discover"))
        .await
        .expect("router responded");
    // Without auth this must be 401, NOT 404. (Or 503 if pool=None hit first
    // — both prove the route is registered.)
    assert!(
        matches!(
            res.status(),
            StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
        ),
        "unexpected status {}",
        res.status()
    );
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
}

// ─── 2. Search route mounted ──────────────────────────────────────────
#[ignore]
#[tokio::test]
async fn search_route_mounted() {
    let app = build_platform_router(make_state(pool().await));
    let res = app
        .oneshot(req(Method::GET, "/api/community/circles/search?q=test"))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        res.status(),
        StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
    ));
}

// ─── 3. By-slug route mounted ─────────────────────────────────────────
#[ignore]
#[tokio::test]
async fn by_slug_route_mounted() {
    let app = build_platform_router(make_state(pool().await));
    let res = app
        .oneshot(req(Method::GET, "/api/community/circles/by-slug/foo-bar"))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
}

// ─── 4. My-circles route mounted ──────────────────────────────────────
#[ignore]
#[tokio::test]
async fn my_circles_route_mounted() {
    let app = build_platform_router(make_state(pool().await));
    let res = app
        .oneshot(req(Method::GET, "/api/community/me/circles"))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        res.status(),
        StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
    ));
}

// ─── 5. Ban POST rejected without CSRF header ─────────────────────────
#[ignore]
#[tokio::test]
async fn ban_post_rejects_without_csrf() {
    let app = build_platform_router(make_state(pool().await));
    let cid = Uuid::new_v4();
    let target = Uuid::new_v4();
    let body = serde_json::json!({ "user_id": target, "reason": "test" }).to_string();
    let res = app
        .oneshot(req_json(
            Method::POST,
            &format!("/api/community/circles/{}/bans", cid),
            &body,
        ))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    // Either CSRF-rejected (403) or auth-rejected (401). 200 would mean
    // the CSRF guard is missing — regression.
    assert!(
        matches!(
            res.status(),
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
        ),
        "CSRF guard missing on ban endpoint — got {}",
        res.status()
    );
}

// ─── 6. Moderator POST rejected without CSRF header ───────────────────
#[ignore]
#[tokio::test]
async fn moderator_post_rejects_without_csrf() {
    let app = build_platform_router(make_state(pool().await));
    let cid = Uuid::new_v4();
    let uid = Uuid::new_v4();
    let body = serde_json::json!({ "moderator": true }).to_string();
    let res = app
        .oneshot(req_json(
            Method::POST,
            &format!("/api/community/circles/{}/moderator/{}", cid, uid),
            &body,
        ))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        res.status(),
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
    ));
}

// ─── 7. Unban DELETE rejected without CSRF ────────────────────────────
#[ignore]
#[tokio::test]
async fn unban_delete_rejects_without_csrf() {
    let app = build_platform_router(make_state(pool().await));
    let cid = Uuid::new_v4();
    let uid = Uuid::new_v4();
    let res = app
        .oneshot(req(
            Method::DELETE,
            &format!("/api/community/circles/{}/bans/{}", cid, uid),
        ))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        res.status(),
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
    ));
}

// ─── 8. Bans GET route mounted ────────────────────────────────────────
#[ignore]
#[tokio::test]
async fn bans_get_route_mounted() {
    let app = build_platform_router(make_state(pool().await));
    let cid = Uuid::new_v4();
    let res = app
        .oneshot(req(
            Method::GET,
            &format!("/api/community/circles/{}/bans", cid),
        ))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        res.status(),
        StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
    ));
}

// ─── 9. Profile-banner PUT rejected without CSRF ──────────────────────
#[ignore]
#[tokio::test]
async fn profile_banner_put_rejects_without_csrf() {
    let app = build_platform_router(make_state(pool().await));
    let body = serde_json::json!({ "banner_url": "https://x.test/img.jpg" }).to_string();
    let res = app
        .oneshot(req_json(
            Method::PUT,
            "/api/community/profile/banner",
            &body,
        ))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
    assert!(matches!(
        res.status(),
        StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED | StatusCode::SERVICE_UNAVAILABLE
    ));
}

// ─── 10. Circle settings page route mounted ───────────────────────────
#[ignore]
#[tokio::test]
async fn circle_settings_page_route_mounted() {
    let app = build_platform_router(make_state(pool().await));
    let res = app
        .oneshot(req(Method::GET, "/community/circle/foo-bar/settings"))
        .await
        .expect("router responded");
    assert_ne!(res.status(), StatusCode::NOT_FOUND);
}
