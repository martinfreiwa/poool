//! HTTP-level integration tests for the Developer **Drafts** API
//! (`/api/developer/draft*`, `/api/developer/drafts`).
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
//!     cargo test --test developer_drafts_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! * `POST /api/developer/draft` — happy path (201 + UUID echoed),
//!   401 anonymous, **403 for non-developer (C-2 regression guard)** —
//!   the handler used to silently grant the `developer` role to any
//!   authenticated caller; that side-effect must be gone.
//! * `GET /api/developer/draft/:id` — owner gets 200, foreign developer
//!   gets 404 (because the `WHERE developer_user_id = $2` filter makes
//!   it indistinguishable from "doesn't exist").
//! * `PUT /api/developer/draft/:id` — sanitize-on-update + the C-6
//!   regression guard: a draft created with NULL yield fields and a
//!   PUT that does not touch them must keep them NULL. The old bug
//!   silently defaulted them to `10` ("10% rental yield" mirage).
//! * `DELETE /api/developer/draft/:id` — soft-deletes the asset, only
//!   when project status = `draft`.
//! * `GET /api/developer/drafts` — list scoped to current developer.
//! * `POST /api/developer/draft/:id/submit` — moves project to
//!   `submitted` state (requires at least one image).
//! * `POST /api/developer/draft/:id/duplicate` — creates a clone with a
//!   new asset UUID and `(Copy)` suffix.

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
    .bind(format!("{}@dev-drafts-http.test", id))
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
           VALUES ($1, $2, NULL, 'dev-drafts-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
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

/// Returns TRUE iff the user has the `developer` role active.
async fn user_is_developer(pool: &PgPool, user_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM user_roles ur
              JOIN roles r ON r.id = ur.role_id
             WHERE ur.user_id = $1
               AND r.name = 'developer'
               AND COALESCE(ur.is_active, TRUE) = TRUE
         )",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
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
    let csrf = "test-csrf-drafts-1234567890";
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

/// Standard create-draft payload — minimum financials so `validate_draft_shape`
/// passes. NOTE: yield fields (`annual_yield_bps`, etc.) are deliberately
/// NOT included on the create payload — only `UpdateDraftAsset` accepts them.
fn create_payload_minimal(title: &str) -> serde_json::Value {
    serde_json::json!({
        "title": title,
        "asset_type": "real_estate",
        "total_value_cents": 1_000_000,
        "token_price_cents": 10_000,
        "tokens_total": 100,
    })
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/developer/draft
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn create_draft_returns_200_and_uuid_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/draft",
        Some(&session),
        create_payload_minimal("HTTP Test Draft"),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    // Handler returns Ok(Json(...)) which is 200, not 201. We accept either.
    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "expected 200/201, got {}; body: {}",
        status,
        body
    );
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let asset_id_str = parsed
        .get("asset_id")
        .and_then(|v| v.as_str())
        .expect("asset_id present in response");
    Uuid::parse_str(asset_id_str).expect("asset_id is a valid UUID");
}

#[ignore]
#[tokio::test]
async fn create_draft_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/draft",
        None,
        create_payload_minimal("Anon Draft"),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// C-2 regression guard (2026-05-19 audit). The old handler self-promoted
/// any authenticated caller to the `developer` role on first POST. The
/// fix changes this to `require_developer_api(...)` so a non-developer
/// gets 403 and the role grant side-effect is GONE.
#[ignore]
#[tokio::test]
async fn create_draft_returns_403_for_non_developer_and_does_not_grant_role() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    // Sanity: user is NOT a developer at the start.
    assert!(
        !user_is_developer(&pool, user).await,
        "fixture user must start without the developer role"
    );

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/draft",
        Some(&session),
        create_payload_minimal("Should Not Self-Promote"),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let _ = body_string(resp).await;

    // The fix-or-regression check: role must still NOT be granted.
    let post_promoted = user_is_developer(&pool, user).await;

    cleanup_user(&pool, user).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "non-developer POST must be 403 (C-2 fix); old behaviour was 200 + silent role grant"
    );
    assert!(
        !post_promoted,
        "C-2 regression: handler must NOT auto-grant the developer role"
    );
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/draft/:id  (single draft fetch)
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn get_draft_returns_200_for_owner() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session),
            create_payload_minimal("Get-Owner Draft"),
        ))
        .await
        .expect("create draft");
    let body = body_string(create_resp).await;
    let asset_id = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");

    let get_resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/draft/{}", asset_id),
            Some(&session),
        ))
        .await
        .expect("get draft");

    let status = get_resp.status();
    let body = body_string(get_resp).await;
    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("Get-Owner Draft"),
        "title round-trip failed; body: {}",
        body
    );
}

/// A different developer must NOT be able to fetch developer A's draft.
/// The handler scopes by `developer_user_id = $2`, so the response is 404
/// (indistinguishable from "doesn't exist") rather than 403.
#[ignore]
#[tokio::test]
async fn get_draft_returns_404_for_foreign_developer() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let owner_session = mint_session(&pool, owner).await;

    let intruder = insert_user(&pool).await;
    grant_developer(&pool, intruder).await;
    let intruder_session = mint_session(&pool, intruder).await;

    let app = build_platform_router(make_state(pool.clone()));

    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&owner_session),
            create_payload_minimal("Foreign-Locked Draft"),
        ))
        .await
        .expect("create");
    let body = body_string(create_resp).await;
    let asset_id = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");

    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/draft/{}", asset_id),
            Some(&intruder_session),
        ))
        .await
        .expect("get");

    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, owner).await;
    cleanup_user(&pool, intruder).await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "owner-scoped GET must return 404 for foreign developer"
    );
}

// ──────────────────────────────────────────────────────────────────────
// PUT /api/developer/draft/:id
// ──────────────────────────────────────────────────────────────────────

/// C-6 regression guard. A draft created without yield fields must keep
/// the `annual_yield_bps` column NULL after a PUT that does not touch
/// those fields. The old bug silently inserted `10` ("10%") because the
/// data model treated 0/missing as "default 10".
#[ignore]
#[tokio::test]
async fn update_draft_does_not_default_yield_fields_to_10() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session),
            create_payload_minimal("Yield-NULL Draft"),
        ))
        .await
        .expect("create draft");
    let body = body_string(create_resp).await;
    let asset_id_str = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");
    let asset_id = Uuid::parse_str(&asset_id_str).expect("uuid");

    // Sanity: NULL after create.
    let before: (Option<i32>, Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps
         FROM assets WHERE id = $1",
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("read before");
    assert_eq!(
        before,
        (None, None, None),
        "create handler must NOT default yield fields"
    );

    // PUT update touches only `short_description` — yield fields must
    // remain NULL.
    let put_resp = app
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/draft/{}", asset_id),
            Some(&session),
            serde_json::json!({"short_description": "tiny tweak"}),
        ))
        .await
        .expect("put");
    let put_status = put_resp.status();
    let put_body = body_string(put_resp).await;
    assert_eq!(put_status, StatusCode::OK, "body: {}", put_body);

    let after: (Option<i32>, Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps
         FROM assets WHERE id = $1",
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("read after");

    cleanup_user(&pool, user).await;

    assert_eq!(
        after,
        (None, None, None),
        "C-6 regression: PUT must NOT auto-default yield fields to 10 (1000 bps)"
    );
}

/// Sanitize behaviour — the PUT handler runs `sanitize_text` /
/// `sanitize_multiline` on `title`, `short_description`, `description`,
/// `location_*`, etc. A title with a `<script>` payload must be stripped.
#[ignore]
#[tokio::test]
async fn update_draft_sanitizes_user_supplied_text() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session),
            create_payload_minimal("Sanitize Draft"),
        ))
        .await
        .expect("create");
    let body = body_string(create_resp).await;
    let asset_id_str = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");
    let asset_id = Uuid::parse_str(&asset_id_str).expect("uuid");

    let put_resp = app
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/draft/{}", asset_id),
            Some(&session),
            serde_json::json!({
                "title": "Hello <script>alert(1)</script> World",
                "short_description": "Hi <img src=x onerror=alert(1)>",
            }),
        ))
        .await
        .expect("put");

    let put_status = put_resp.status();
    let _ = body_string(put_resp).await;
    assert_eq!(put_status, StatusCode::OK);

    let (title, short_desc): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT title, short_description FROM assets WHERE id = $1")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .expect("read");

    cleanup_user(&pool, user).await;

    let title = title.unwrap_or_default();
    let short_desc = short_desc.unwrap_or_default();
    assert!(
        !title.to_lowercase().contains("<script"),
        "title must have <script> stripped; got {:?}",
        title
    );
    assert!(
        !short_desc.to_lowercase().contains("onerror"),
        "short_description must have onerror= stripped; got {:?}",
        short_desc
    );
}

#[ignore]
#[tokio::test]
async fn update_draft_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PUT,
        &format!("/api/developer/draft/{}", Uuid::new_v4()),
        None,
        serde_json::json!({"title": "should not work"}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn update_draft_returns_403_for_foreign_developer() {
    let pool = pool().await;
    let owner = insert_user(&pool).await;
    grant_developer(&pool, owner).await;
    let owner_session = mint_session(&pool, owner).await;

    let intruder = insert_user(&pool).await;
    grant_developer(&pool, intruder).await;
    let intruder_session = mint_session(&pool, intruder).await;

    let app = build_platform_router(make_state(pool.clone()));

    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&owner_session),
            create_payload_minimal("Foreign-Locked PUT Draft"),
        ))
        .await
        .expect("create");
    let body = body_string(create_resp).await;
    let asset_id = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");

    let put_resp = app
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/draft/{}", asset_id),
            Some(&intruder_session),
            serde_json::json!({"title": "hijack attempt"}),
        ))
        .await
        .expect("put");

    let status = put_resp.status();
    let _ = body_string(put_resp).await;

    cleanup_user(&pool, owner).await;
    cleanup_user(&pool, intruder).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "foreign developer must not be able to PUT another developer's draft"
    );
}

// ──────────────────────────────────────────────────────────────────────
// DELETE /api/developer/draft/:id
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn delete_draft_soft_deletes_for_owner() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session),
            create_payload_minimal("Delete-Me Draft"),
        ))
        .await
        .expect("create");
    let body = body_string(create_resp).await;
    let asset_id_str = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");
    let asset_id = Uuid::parse_str(&asset_id_str).expect("uuid");

    let del_resp = app
        .oneshot(mutating_with_session(
            Method::DELETE,
            &format!("/api/developer/draft/{}", asset_id),
            Some(&session),
            serde_json::json!({}),
        ))
        .await
        .expect("delete");

    let status = del_resp.status();
    let _ = body_string(del_resp).await;
    assert_eq!(status, StatusCode::OK);

    let deleted_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM assets WHERE id = $1")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .expect("read");

    cleanup_user(&pool, user).await;

    assert!(
        deleted_at.is_some(),
        "DELETE must set deleted_at (soft delete), not hard delete"
    );
}

#[ignore]
#[tokio::test]
async fn delete_draft_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::DELETE,
        &format!("/api/developer/draft/{}", Uuid::new_v4()),
        None,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn delete_draft_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::DELETE,
        &format!("/api/developer/draft/{}", Uuid::new_v4()),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/drafts (listing)
// ──────────────────────────────────────────────────────────────────────

/// The list endpoint must scope by `developer_user_id`, i.e. developer A
/// should never see developer B's drafts. Create one under each developer
/// and assert B's list contains only their own.
#[ignore]
#[tokio::test]
async fn list_drafts_scopes_to_current_developer() {
    let pool = pool().await;
    let dev_a = insert_user(&pool).await;
    grant_developer(&pool, dev_a).await;
    let session_a = mint_session(&pool, dev_a).await;

    let dev_b = insert_user(&pool).await;
    grant_developer(&pool, dev_b).await;
    let session_b = mint_session(&pool, dev_b).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Create one draft for each developer.
    let _ = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session_a),
            create_payload_minimal("Dev-A Draft"),
        ))
        .await
        .expect("a create");
    let _ = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session_b),
            create_payload_minimal("Dev-B Draft"),
        ))
        .await
        .expect("b create");

    let list_resp = app
        .oneshot(get_with_session("/api/developer/drafts", Some(&session_b)))
        .await
        .expect("list");
    let status = list_resp.status();
    let body = body_string(list_resp).await;

    cleanup_user(&pool, dev_a).await;
    cleanup_user(&pool, dev_b).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("Dev-B Draft"),
        "dev B's draft should appear in dev B's list; body: {}",
        body
    );
    assert!(
        !body.contains("Dev-A Draft"),
        "dev A's draft MUST NOT appear in dev B's list; body: {}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn list_drafts_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/drafts", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn list_drafts_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/drafts", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/developer/draft/:id/submit
// ──────────────────────────────────────────────────────────────────────

/// Happy path: create draft → seed at least one image → POST /submit →
/// developer_projects row flips to status='submitted'.
#[ignore]
#[tokio::test]
async fn submit_draft_transitions_to_submitted_state() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session),
            create_payload_minimal("Submit-Me Draft"),
        ))
        .await
        .expect("create");
    let body = body_string(create_resp).await;
    let asset_id_str = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("asset_id").and_then(|s| s.as_str()).map(String::from))
        .expect("asset_id");
    let asset_id = Uuid::parse_str(&asset_id_str).expect("uuid");

    // Submit requires at least one image — insert one directly.
    sqlx::query(
        "INSERT INTO asset_images (asset_id, image_url, is_cover, sort_order)
         VALUES ($1, 'https://example.test/img.jpg', true, 0)",
    )
    .bind(asset_id)
    .execute(&pool)
    .await
    .expect("seed image");

    let submit_resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/developer/draft/{}/submit", asset_id),
            Some(&session),
            serde_json::json!({}),
        ))
        .await
        .expect("submit");
    let status = submit_resp.status();
    let body = body_string(submit_resp).await;
    assert_eq!(status, StatusCode::OK, "body: {}", body);

    let new_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM developer_projects WHERE asset_id = $1")
            .bind(asset_id)
            .fetch_optional(&pool)
            .await
            .expect("read project status");

    cleanup_user(&pool, user).await;

    assert_eq!(
        new_status.as_deref(),
        Some("submitted"),
        "submit must move developer_projects.status to 'submitted'"
    );
}

#[ignore]
#[tokio::test]
async fn submit_draft_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/draft/{}/submit", Uuid::new_v4()),
        None,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn submit_draft_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/draft/{}/submit", Uuid::new_v4()),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/developer/draft/:id/duplicate
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn duplicate_draft_creates_new_asset() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let create_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            Some(&session),
            create_payload_minimal("Dup-Source"),
        ))
        .await
        .expect("create");
    let body = body_string(create_resp).await;
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let asset_id_str = parsed
        .get("asset_id")
        .and_then(|v| v.as_str())
        .expect("asset_id");
    let asset_id = Uuid::parse_str(asset_id_str).expect("uuid");

    let dup_resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/developer/draft/{}/duplicate", asset_id),
            Some(&session),
            serde_json::json!({}),
        ))
        .await
        .expect("duplicate");
    let status = dup_resp.status();
    let body = body_string(dup_resp).await;
    assert_eq!(status, StatusCode::OK, "body: {}", body);

    let dup_parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let new_id_str = dup_parsed
        .get("new_asset_id")
        .and_then(|v| v.as_str())
        .expect("new_asset_id");
    let new_id = Uuid::parse_str(new_id_str).expect("new uuid");
    assert_ne!(new_id, asset_id, "duplicate must mint a new asset UUID");

    let new_title: Option<String> = sqlx::query_scalar("SELECT title FROM assets WHERE id = $1")
        .bind(new_id)
        .fetch_optional(&pool)
        .await
        .expect("read");

    cleanup_user(&pool, user).await;

    let new_title = new_title.expect("duplicate row exists");
    assert!(
        new_title.contains("(Copy)"),
        "duplicate title must carry the (Copy) suffix; got {:?}",
        new_title
    );
}

#[ignore]
#[tokio::test]
async fn duplicate_draft_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/draft/{}/duplicate", Uuid::new_v4()),
        None,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn duplicate_draft_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/draft/{}/duplicate", Uuid::new_v4()),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
