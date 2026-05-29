//! HTTP-level integration tests for the Developer **Asset Detail +
//! Change-Requests** API.
//!
//! Exercises the full Axum router that the production binary serves.
//! The router is built via `poool_backend::build_platform_router` so
//! handlers + CSRF + security headers all match production.
//!
//! ## Running
//!
//! All tests are `#[ignore]`d because they require a live Postgres
//! database. To exercise them:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test developer_change_requests_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! * `GET  /api/developer/assets/:id` — owner gets full detail (200 +
//!   investors/financials/etc. arrays), foreign developer gets 403,
//!   anonymous gets 401.
//! * `PUT  /api/developer/assets/:id` → `change_requests::submit_edit`
//!   — for a draft, applies directly (`mode=direct`); for an
//!   approved/live asset, creates a pending change request
//!   (`mode=review` + `change_request_id` UUID).
//! * `GET  /api/developer/assets/:id/pending-changes` →
//!   `change_requests::get_pending` — returns the pending CR for the
//!   owner, NULL once none exist.
//! * Ownership enforcement: developer B cannot read /api/developer/assets/:id
//!   nor submit edits for developer A's asset (403).
//!
//! ## Out of scope
//!
//! The asset-detail "Settings" panel renders three controls
//! (`toggle-featured`, `toggle-published`, `select-funding-status`)
//! plus a "Freeze trading" button (`btn-freeze`). As of 2026-05-19 the
//! backend has NOT added developer-facing endpoints for any of these —
//! the frontend handlers route them through `showToast` because they
//! are admin-gated actions. Once developer-side endpoints land, add
//! tests for them in this file.

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
    .bind(format!("{}@dev-cr-http.test", id))
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
           VALUES ($1, $2, NULL, 'dev-cr-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
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
        "INSERT INTO roles (name, description)
         VALUES ('developer', 'Asset issuer / developer')
         ON CONFLICT (name) DO NOTHING",
    )
    .execute(pool)
    .await
    .expect("ensure developer role");

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

/// Seed an asset directly (skip the create-draft handler) so we can pick
/// the exact `developer_projects.status` we need. Returns the asset UUID.
async fn seed_asset(
    pool: &PgPool,
    developer_user_id: Uuid,
    title: &str,
    project_status: &str,
) -> Uuid {
    let asset_id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO assets (
            id, developer_user_id, title, slug, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, featured, published, updated_at, submission_step
        ) VALUES (
            $1, $2, $3, $4, 'real_estate',
            1000000, 10000, 100, 100,
            'upcoming', false, false, NOW(), 2
        )"#,
    )
    .bind(asset_id)
    .bind(developer_user_id)
    .bind(title)
    .bind(format!("dev-cr-{}", asset_id))
    .execute(pool)
    .await
    .expect("insert asset");

    sqlx::query(
        "INSERT INTO developer_projects (developer_id, asset_id, project_name, status)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(developer_user_id)
    .bind(asset_id)
    .bind(title)
    .bind(project_status)
    .execute(pool)
    .await
    .expect("insert developer_projects");

    asset_id
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM audit_logs WHERE actor_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM asset_change_requests WHERE developer_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "DELETE FROM developer_projects WHERE asset_id IN
            (SELECT id FROM assets WHERE developer_user_id = $1)",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM asset_images WHERE asset_id IN
            (SELECT id FROM assets WHERE developer_user_id = $1)",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query("DELETE FROM assets WHERE developer_user_id = $1")
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

// ──────────────────────────────────────────────────────────────────────
// Request builders
// ──────────────────────────────────────────────────────────────────────

fn get_with_session(uri: &str, session_token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = session_token {
        builder = builder.header("cookie", format!("poool_session={}", t));
    }
    builder.body(Body::empty()).unwrap()
}

fn mutating_with_session(
    method: Method,
    uri: &str,
    session_token: Option<&str>,
    json_body: serde_json::Value,
) -> Request<Body> {
    let csrf = "test-csrf-cr-1234567890";
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-CSRF-Token", csrf)
        .header("content-type", "application/json");

    builder = match session_token {
        Some(t) => builder.header(
            "cookie",
            format!("poool_session={}; csrf_token={}", t, csrf),
        ),
        None => builder.header("cookie", format!("csrf_token={}", csrf)),
    };

    builder
        .body(Body::from(serde_json::to_vec(&json_body).unwrap()))
        .unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/assets/:id
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn get_asset_returns_200_for_owner() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let session = mint_session(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "CR Owner Asset", "draft").await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;
    cleanup_user(&pool, owner).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("CR Owner Asset"),
        "asset detail must include the title; body: {}",
        body
    );
    // Detail JSON shape: investors / documents / images arrays should
    // appear (possibly empty).
    assert!(body.contains("\"investors\""), "missing investors key");
    assert!(body.contains("\"documents\""), "missing documents key");
    assert!(body.contains("\"images\""), "missing images key");
}

#[ignore]
#[tokio::test]
async fn get_asset_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(&format!("/api/developer/assets/{}", Uuid::new_v4()), None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn get_asset_returns_403_for_non_developer() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Non-Dev Reader Asset", "draft").await;

    // Authenticated but NOT a developer.
    let regular = insert_user(&pool).await;
    let session = mint_session(&pool, regular).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, owner).await;
    cleanup_user(&pool, regular).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn get_asset_returns_403_for_foreign_developer() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Foreign-Locked Asset", "draft").await;

    let intruder = insert_user(&pool).await;
    grant_developer(&pool, intruder).await;
    let intruder_session = mint_session(&pool, intruder).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}", asset_id),
            Some(&intruder_session),
        ))
        .await
        .expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, owner).await;
    cleanup_user(&pool, intruder).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "foreign developer must not read another developer's asset"
    );
}

// ──────────────────────────────────────────────────────────────────────
// PUT /api/developer/assets/:id → change_requests::submit_edit
// ──────────────────────────────────────────────────────────────────────

/// Draft assets accept direct edits. Response shape: `{"mode":"direct"}`
/// and the asset table itself gets updated.
#[ignore]
#[tokio::test]
async fn submit_edit_draft_applies_directly() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let session = mint_session(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Edit-Me Draft", "draft").await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/assets/{}", asset_id),
            Some(&session),
            serde_json::json!({"title": "Edited Draft Title"}),
        ))
        .await
        .expect("put");

    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "body: {}", body);

    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        parsed.get("mode").and_then(|v| v.as_str()),
        Some("direct"),
        "draft edits must be applied directly; body: {}",
        body
    );

    let new_title: Option<String> = sqlx::query_scalar("SELECT title FROM assets WHERE id = $1")
        .bind(asset_id)
        .fetch_one(&pool)
        .await
        .expect("read title");

    cleanup_user(&pool, owner).await;

    assert_eq!(new_title.as_deref(), Some("Edited Draft Title"));
}

/// Approved / live assets must go through the review queue.
#[ignore]
#[tokio::test]
async fn submit_edit_approved_asset_creates_change_request() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let session = mint_session(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Approved Live Asset", "approved").await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/assets/{}", asset_id),
            Some(&session),
            serde_json::json!({"title": "Proposed New Title"}),
        ))
        .await
        .expect("put");

    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "body: {}", body);

    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        parsed.get("mode").and_then(|v| v.as_str()),
        Some("review"),
        "approved-asset edits must enter the review queue; body: {}",
        body
    );
    let cr_id_str = parsed
        .get("change_request_id")
        .and_then(|v| v.as_str())
        .expect("change_request_id present");
    Uuid::parse_str(cr_id_str).expect("change_request_id is a UUID");

    // The asset's title must NOT have changed — the change request is
    // still pending admin approval.
    let still_old: Option<String> = sqlx::query_scalar("SELECT title FROM assets WHERE id = $1")
        .bind(asset_id)
        .fetch_one(&pool)
        .await
        .expect("read title");

    cleanup_user(&pool, owner).await;

    assert_eq!(
        still_old.as_deref(),
        Some("Approved Live Asset"),
        "approved asset's title must NOT change until admin approves the CR"
    );
}

#[ignore]
#[tokio::test]
async fn submit_edit_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PUT,
        &format!("/api/developer/assets/{}", Uuid::new_v4()),
        None,
        serde_json::json!({"title": "anon"}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn submit_edit_returns_403_for_foreign_developer() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Foreign PUT Lock", "draft").await;

    let intruder = insert_user(&pool).await;
    grant_developer(&pool, intruder).await;
    let intruder_session = mint_session(&pool, intruder).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/assets/{}", asset_id),
            Some(&intruder_session),
            serde_json::json!({"title": "hijack attempt"}),
        ))
        .await
        .expect("put");

    let status = resp.status();
    cleanup_user(&pool, owner).await;
    cleanup_user(&pool, intruder).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "submit_edit must reject foreign developer with 403"
    );
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/assets/:id/pending-changes
// ──────────────────────────────────────────────────────────────────────

/// With no pending CR, the response is `{ "pending": null }`.
#[ignore]
#[tokio::test]
async fn pending_changes_returns_null_when_none_exist() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let session = mint_session(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "No-Pending Asset", "approved").await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}/pending-changes", asset_id),
            Some(&session),
        ))
        .await
        .expect("get pending");

    let status = resp.status();
    let body = body_string(resp).await;
    cleanup_user(&pool, owner).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert!(
        parsed.get("pending").map(|v| v.is_null()).unwrap_or(false),
        "expected `pending` to be JSON null when no CR exists; got {}",
        body
    );
}

/// After a developer PUTs an edit to an approved asset, the
/// pending-changes endpoint must surface the just-created CR.
#[ignore]
#[tokio::test]
async fn pending_changes_returns_just_created_change_request() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let session = mint_session(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Pending-CR Asset", "approved").await;

    let app = build_platform_router(make_state(pool.clone()));

    // 1) Submit an edit that lands in the review queue.
    let put_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/assets/{}", asset_id),
            Some(&session),
            serde_json::json!({"title": "Proposed Edit"}),
        ))
        .await
        .expect("put edit");
    let put_status = put_resp.status();
    let put_body = body_string(put_resp).await;
    assert_eq!(put_status, StatusCode::OK, "body: {}", put_body);

    // 2) Read the pending-changes endpoint.
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}/pending-changes", asset_id),
            Some(&session),
        ))
        .await
        .expect("get pending");
    let status = resp.status();
    let body = body_string(resp).await;
    cleanup_user(&pool, owner).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let pending = parsed
        .get("pending")
        .filter(|v| !v.is_null())
        .expect("pending must be populated after PUT");
    assert_eq!(
        pending.get("status").and_then(|v| v.as_str()),
        Some("pending"),
        "fresh CR must carry status='pending'; body: {}",
        body
    );
    // Diff should include the title.
    let proposed = pending
        .get("proposed_values")
        .and_then(|v| v.as_object())
        .expect("proposed_values is an object");
    assert!(
        proposed.contains_key("title"),
        "proposed_values must reflect the changed `title` field; got {:?}",
        proposed
    );
}

#[ignore]
#[tokio::test]
async fn pending_changes_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        &format!("/api/developer/assets/{}/pending-changes", Uuid::new_v4()),
        None,
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn pending_changes_returns_403_for_foreign_developer() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let asset_id = seed_asset(&pool, owner, "Pending Foreign Lock", "approved").await;

    let intruder = insert_user(&pool).await;
    grant_developer(&pool, intruder).await;
    let intruder_session = mint_session(&pool, intruder).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}/pending-changes", asset_id),
            Some(&intruder_session),
        ))
        .await
        .expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, owner).await;
    cleanup_user(&pool, intruder).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "foreign developer must not read another developer's pending CRs"
    );
}
