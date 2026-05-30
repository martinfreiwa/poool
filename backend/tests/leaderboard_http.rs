//! HTTP-level integration tests for `/api/leaderboard/*` and
//! `/api/community/leaderboard`.
//!
//! These exercise the full Axum router that the production binary serves —
//! same handlers, same middleware (CSRF + Sentry context + security
//! headers). The router is built via the library entry point
//! `poool_backend::build_platform_router`, so any drift between the
//! test-time router and the production one would fail the build.
//!
//! ## Running
//!
//! All tests in this file are `#[ignore]`d because they require a live
//! Postgres database. To exercise them:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test leaderboard_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! Each test seeds its own data, mints a session cookie directly in
//! `user_sessions`, drives the router via `tower::ServiceExt::oneshot`,
//! and cleans up at the end. Cases:
//!
//! 1. `me_returns_rank_for_authed_user` — GET /api/leaderboard/me with a
//!    valid session returns 200 + JSON with a `rank` field.
//! 2. `me_returns_401_unauthed` — GET without a session returns 401.
//! 3. `refresh_returns_200_for_admin` — POST /api/leaderboard/refresh as
//!    admin returns 200. Regression guard for the auth wiring.
//! 4. `refresh_returns_403_for_regular_user` — POST as non-admin returns
//!    403.
//! 5. `refresh_get_returns_405` — GET on the refresh endpoint returns 405
//!    Method Not Allowed. Regression for audit fix 4 (was previously
//!    accepted as both GET and POST).
//! 6. `prefs_partial_preserves_other_fields` — PUT /api/leaderboard/preferences
//!    with a partial body preserves the other columns. Regression for
//!    audit fix 3.
//! 7. `community_leaderboard_anonymizes_hidden_users` — GET
//!    /api/community/leaderboard?period=alltime swaps display_name for a
//!    placeholder when `leaderboard_preferences.visible = false`, and sets
//!    `anonymized: true` on the JSON row.

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
// Shared test infrastructure
// ──────────────────────────────────────────────────────────────────────

/// Connect to the test database. Requires `DATABASE_URL` to be set.
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

/// Build an `AppState` against the given pool. Templates load from the
/// shared `frontend/platform` directory so the binary's pages compile too.
fn make_state(pool: PgPool) -> AppState {
    // dotenvy + config::from_env are heavy; we synthesise a minimal Config
    // by going through `from_env` after setting the required env vars.
    if std::env::var("DATABASE_URL").is_err() {
        panic!("DATABASE_URL must be set for HTTP integration tests");
    }
    // Force a non-production env to relax CSRF cookie security flags etc.
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
        webauthn: test_webauthn(),
    }
}

/// Insert a minimal user row and return the UUID. `email_verified = TRUE`
/// is required because `get_user_by_session` filters unverified users.
async fn insert_user(pool: &PgPool, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status, email_verified)
         VALUES ($1, $2, 'x', $3, TRUE)",
    )
    .bind(id)
    .bind(format!("{}@http.test", id))
    .bind(status)
    .execute(pool)
    .await
    .expect("insert user");
    id
}

/// Insert a 24-hour session row for `user_id` and return the session token
/// to attach as a `poool_session` cookie.
async fn mint_session(pool: &PgPool, user_id: Uuid) -> String {
    // Hand-rolled hex token avoids pulling in the auth::service module
    // generator (which would force an indirect dep on its other helpers).
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

/// Grant the user the `admin` role so `is_admin` checks pass.
async fn grant_admin(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_roles (user_id, role_id, is_active)
           SELECT $1, r.id, TRUE FROM roles r WHERE r.name = 'admin'
           ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("grant admin role");
}

/// Insert a leaderboard_scores row for the user so /me can resolve a rank.
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

/// Best-effort cleanup — removes every test-owned row keyed by user_id.
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

/// Build the canonical request: GET with a session cookie. CSRF cookies
/// are not required for GET.
fn get_with_session(uri: &str, session_token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = session_token {
        builder = builder.header("cookie", format!("poool_session={}", t));
    }
    builder.body(Body::empty()).unwrap()
}

/// Build a mutating (POST/PUT/DELETE) request with session + CSRF cookie
/// and matching `X-CSRF-Token` header. Body is JSON-encoded.
fn mutating_with_session(
    method: Method,
    uri: &str,
    session_token: &str,
    json_body: serde_json::Value,
) -> Request<Body> {
    let csrf = "test-csrf-1234567890";
    Request::builder()
        .method(method)
        .uri(uri)
        .header(
            "cookie",
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&json_body).unwrap()))
        .unwrap()
}

/// Drain the response body to a String.
async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

// ──────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn me_returns_rank_for_authed_user() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    seed_leaderboard_row(&pool, user, 10_000, 1).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/leaderboard/me", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;
    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("\"rank\""),
        "response JSON must include a `rank` field; got {}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn me_returns_401_unauthed() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/leaderboard/me", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn refresh_returns_200_for_admin() {
    let pool = pool().await;
    let admin = insert_user(&pool, "active").await;
    grant_admin(&pool, admin).await;
    let session = mint_session(&pool, admin).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/leaderboard/refresh",
        &session,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
}

#[ignore]
#[tokio::test]
async fn refresh_returns_403_for_regular_user() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/leaderboard/refresh",
        &session,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn refresh_get_returns_405() {
    // Regression guard for audit fix 4: the refresh endpoint must be
    // POST-only so it is shielded by CSRF and can't be triggered by a
    // GET link prefetch or accidental navigation.
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    grant_admin(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/leaderboard/refresh", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;

    assert_eq!(
        status,
        StatusCode::METHOD_NOT_ALLOWED,
        "GET on /api/leaderboard/refresh must be rejected as 405"
    );
}

#[ignore]
#[tokio::test]
async fn prefs_partial_preserves_other_fields() {
    // Regression guard for audit fix 3: PUT with a partial body must not
    // clobber unspecified columns. Set a baseline of all-three, then PUT
    // only `visible: false`, then re-read the row and verify `show_avatar`
    // and `display_name` are unchanged.
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Step 1 — baseline: visible=true, show_avatar=true, display_name="X"
    let req1 = mutating_with_session(
        Method::PUT,
        "/api/leaderboard/preferences",
        &session,
        serde_json::json!({
            "visible": true,
            "show_avatar": true,
            "display_name": "X",
        }),
    );
    let resp1 = app.clone().oneshot(req1).await.expect("oneshot 1");
    assert_eq!(resp1.status(), StatusCode::OK, "baseline PUT failed");

    // Step 2 — partial: PUT only {visible: false}. Other fields must persist.
    let req2 = mutating_with_session(
        Method::PUT,
        "/api/leaderboard/preferences",
        &session,
        serde_json::json!({ "visible": false }),
    );
    let resp2 = app.oneshot(req2).await.expect("oneshot 2");
    assert_eq!(resp2.status(), StatusCode::OK, "partial PUT failed");

    // Re-read from DB and confirm preservation.
    let row: (bool, bool, Option<String>) = sqlx::query_as(
        "SELECT visible, show_avatar, display_name
         FROM leaderboard_preferences WHERE user_id = $1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("read prefs");

    cleanup_user(&pool, user).await;

    assert!(!row.0, "visible should be false");
    assert!(
        row.1,
        "show_avatar must remain true after partial PUT (regression for fix 3)"
    );
    assert_eq!(
        row.2.as_deref(),
        Some("X"),
        "display_name must remain 'X' after partial PUT (regression for fix 3)"
    );
}

#[ignore]
#[tokio::test]
async fn community_leaderboard_anonymizes_hidden_users() {
    // Two users with XP. One has leaderboard_preferences.visible=false.
    // The viewer is a third user, so the hidden user's display_name must
    // be replaced with an "Investor #..." placeholder and the JSON row
    // must have `anonymized: true`.
    //
    // The community DB is optional. If it isn't configured we skip
    // gracefully — community leaderboard handler returns 500 in that case
    // and there's nothing to assert against.
    let pool = pool().await;

    let community_url = std::env::var("COMMUNITY_DATABASE_URL").ok();
    if community_url.is_none() {
        eprintln!("COMMUNITY_DATABASE_URL not set — skipping community leaderboard test");
        return;
    }
    let community_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(community_url.as_deref().unwrap())
        .await
        .expect("connect community DB");

    let visible_user = insert_user(&pool, "active").await;
    let hidden_user = insert_user(&pool, "active").await;
    let viewer = insert_user(&pool, "active").await;
    let session = mint_session(&pool, viewer).await;

    // Mark hidden_user as opted-out
    sqlx::query(
        r#"INSERT INTO leaderboard_preferences (user_id, visible, show_avatar)
           VALUES ($1, FALSE, TRUE)
           ON CONFLICT (user_id) DO UPDATE SET visible = FALSE"#,
    )
    .bind(hidden_user)
    .execute(&pool)
    .await
    .expect("insert hidden pref");

    // Seed community_profiles rows so both users appear on the all-time
    // leaderboard (which reads `community_profiles.xp_total` directly,
    // see `community::xp::get_user_leaderboard`). XP must beat any
    // pre-existing seed users so our two rows land in the top-N.
    for (uid, xp) in [(visible_user, 10_000_000i32), (hidden_user, 9_000_000i32)] {
        sqlx::query(
            r#"INSERT INTO community_profiles (user_id, xp_total, level, level_name, login_streak)
               VALUES ($1, $2, 1, 'Seedling', 0)
               ON CONFLICT (user_id) DO UPDATE SET xp_total = EXCLUDED.xp_total"#,
        )
        .bind(uid)
        .bind(xp)
        .execute(&community_pool)
        .await
        .expect("seed community_profiles");
    }

    // Build state WITH the community pool wired up so this route works.
    let mut state = make_state(pool.clone());
    state.community_db = Some(community_pool.clone());

    let app = build_platform_router(state);
    let req = get_with_session(
        "/api/community/leaderboard?period=alltime&limit=10",
        Some(&session),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    // Cleanup before assertions.
    for uid in [visible_user, hidden_user, viewer] {
        cleanup_user(&pool, uid).await;
        let _ = sqlx::query("DELETE FROM community_profiles WHERE user_id = $1")
            .bind(uid)
            .execute(&community_pool)
            .await;
    }

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse json");
    let board = parsed
        .get("leaderboard")
        .and_then(|v| v.as_array())
        .expect("leaderboard array");

    let hidden_row = board
        .iter()
        .find(|r| r.get("user_id").and_then(|u| u.as_str()) == Some(&hidden_user.to_string()))
        .unwrap_or_else(|| panic!("no row for hidden user in {}", body));
    assert_eq!(
        hidden_row.get("anonymized").and_then(|v| v.as_bool()),
        Some(true),
        "hidden user's row must have anonymized=true"
    );
    let display = hidden_row
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert!(
        display.starts_with("Investor #"),
        "hidden user's display_name must be the anonymized placeholder; got {}",
        display
    );

    let visible_row = board
        .iter()
        .find(|r| r.get("user_id").and_then(|u| u.as_str()) == Some(&visible_user.to_string()))
        .unwrap_or_else(|| panic!("no row for visible user in {}", body));
    assert_eq!(
        visible_row.get("anonymized").and_then(|v| v.as_bool()),
        Some(false),
        "non-opted-out user must NOT be anonymized"
    );
}

// ──────────────────────────────────────────────────────────────────────
// Production-readiness regression tests (Points 6, 7, 9)
// ──────────────────────────────────────────────────────────────────────

/// 8. GET /api/leaderboard sets `Cache-Control: private, max-age=30` and an
///    `ETag` derived from the cached `last_updated` + query params, so
///    browsers can short-circuit subsequent requests inside that window.
#[ignore]
#[tokio::test]
async fn rankings_emit_cache_control_and_etag() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    seed_leaderboard_row(&pool, user, 1_000_000, 1).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/leaderboard?metric=invested&page=1", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let headers = resp.headers().clone();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("private, max-age=30"),
        "expected `private, max-age=30` Cache-Control",
    );
    let etag = headers
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        etag.starts_with("\"lb-") && etag.ends_with('"'),
        "expected ETag like `\"lb-<hex>\"`; got {:?}",
        etag,
    );
}

/// 9. Round-trip the ETag: the second request with `If-None-Match` set to
///    the first response's ETag must come back 304 Not Modified (no body),
///    still carrying the Cache-Control header so the browser keeps the
///    cached copy fresh.
#[ignore]
#[tokio::test]
async fn rankings_etag_round_trip_returns_304() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    seed_leaderboard_row(&pool, user, 1_000_000, 1).await;

    let state = make_state(pool.clone());

    let first = build_platform_router(state.clone())
        .oneshot(get_with_session(
            "/api/leaderboard?metric=invested&page=1",
            Some(&session),
        ))
        .await
        .expect("first");
    let etag = first
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let _ = body_string(first).await;
    assert!(!etag.is_empty(), "first response must include an ETag");

    let mut second_req =
        get_with_session("/api/leaderboard?metric=invested&page=1", Some(&session));
    second_req
        .headers_mut()
        .insert("if-none-match", etag.parse().unwrap());

    let second = build_platform_router(state)
        .oneshot(second_req)
        .await
        .expect("second");
    let second_status = second.status();
    let second_headers = second.headers().clone();
    let second_body = body_string(second).await;

    cleanup_user(&pool, user).await;

    assert_eq!(
        second_status,
        StatusCode::NOT_MODIFIED,
        "ETag round-trip must short-circuit to 304",
    );
    assert!(
        second_body.is_empty(),
        "304 response must have an empty body; got {:?}",
        second_body,
    );
    assert_eq!(
        second_headers
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("private, max-age=30"),
        "304 response must still carry the Cache-Control header",
    );
}

/// 10. PUT /api/leaderboard/preferences writes an `audit_logs` row so the
///     visibility change leaves an immutable trail.
#[ignore]
#[tokio::test]
async fn prefs_put_writes_audit_log_row() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;

    let baseline_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_logs WHERE actor_user_id = $1
         AND action LIKE 'leaderboard.prefs.update%'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("baseline count");

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PUT,
        "/api/leaderboard/preferences",
        &session,
        serde_json::json!({ "visible": true, "show_avatar": true, "display_name": "Audit Test" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;
    assert_eq!(status, StatusCode::OK);

    let post_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_logs WHERE actor_user_id = $1
         AND action LIKE 'leaderboard.prefs.update%'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("post count");

    let last_action: Option<String> = sqlx::query_scalar(
        "SELECT action FROM audit_logs WHERE actor_user_id = $1
         AND action LIKE 'leaderboard.prefs.update%'
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("last action");

    let _ = sqlx::query("DELETE FROM audit_logs WHERE actor_user_id = $1")
        .bind(user)
        .execute(&pool)
        .await;
    cleanup_user(&pool, user).await;

    assert_eq!(
        post_count,
        baseline_count + 1,
        "PUT /preferences must insert exactly one audit_logs row",
    );
    let action = last_action.expect("audit row should exist");
    assert!(
        action.contains("visible=Some(true)") && action.contains("display_name_set=true"),
        "audit action string should capture the changed fields; got {:?}",
        action,
    );
}

/// 11. The 61st request inside a minute from the same user must be
///     rate-limited (429 Too Many Requests). We use the disabled limiter in
///     the test rig by default — here we override with a real one so the
///     production behaviour is exercised end-to-end.
#[ignore]
#[tokio::test]
async fn rankings_rate_limited_after_burst() {
    use std::time::Duration as StdDuration;
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    seed_leaderboard_row(&pool, user, 1_000_000, 1).await;

    // Override the limiter to 3 req / 60s so we don't actually need to fire
    // 60 requests to hit the cap.
    let mut state = make_state(pool.clone());
    state.leaderboard_rate_limiter =
        poool_backend::auth::rate_limit::RateLimiter::new(3, StdDuration::from_secs(60));

    let app = build_platform_router(state);

    let mut statuses = Vec::new();
    for _ in 0..5 {
        let req = get_with_session("/api/leaderboard?metric=invested&page=1", Some(&session));
        let resp = app.clone().oneshot(req).await.expect("oneshot");
        statuses.push(resp.status());
        let _ = body_string(resp).await;
    }

    cleanup_user(&pool, user).await;

    let oks = statuses.iter().filter(|s| s.is_success()).count();
    let throttled = statuses
        .iter()
        .filter(|s| **s == StatusCode::TOO_MANY_REQUESTS)
        .count();
    assert_eq!(oks, 3, "first 3 requests inside the window should succeed");
    assert_eq!(
        throttled, 2,
        "requests 4+ inside the window should be 429; got statuses {:?}",
        statuses,
    );
}
