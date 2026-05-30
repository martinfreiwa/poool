//! HTTP-level integration tests for the Developer Operations surface
//! (Villa-Returns P2 — monthly operations submission + period documents).
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
//!     cargo test --test developer_operations_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! Page handlers:
//!   * GET  /developer/operations
//!   * GET  /developer/villas/:asset_id/operations/new
//!   * GET  /developer/villas/:asset_id/operations/:log_id    (NEW 2026-05-19)
//!
//! API handlers (all under DeveloperUser extractor + asset-link gate):
//!   * GET  /api/developer/operations/dashboard
//!   * POST /api/developer/villas/:asset_id/operations
//!   * GET  /api/developer/villas/:asset_id/operations
//!   * GET  /api/developer/villas/:asset_id/operations/:log_id   (NEW single-log)
//!   * PUT  /api/developer/villas/:asset_id/operations/:log_id
//!   * PUT  /api/developer/villas/:asset_id/operations/:log_id/submit
//!   * POST /api/developer/villas/:asset_id/operations/:log_id/documents (multipart)
//!   * GET  /api/developer/villas/:asset_id/operations/:log_id/documents
//!   * GET  /api/developer/villas/:asset_id/asset-config
//!
//! The `custom_expenses` aka `expense_other_notes` JSONB array is verified
//! round-trip across both create and update.
//!
//! Per-villa enforcement: a developer with no active row in
//! `developer_asset_links` for the requested asset is rejected with 403 on
//! every write endpoint. The suite asserts this on each write surface.
//!
//! ## Cleanup note
//!
//! Several villa tables (`villa_operations_log`, `villa_capex_events`,
//! `developer_asset_links`) have BEFORE-DELETE trigger guards or
//! append-only invariants that block ordinary `DELETE` from tests. The
//! cleanup helper temporarily switches `session_replication_role = replica`
//! to bypass triggers + FK checks while it drops test rows. The session
//! is then restored before the connection returns to the pool.

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
    .bind(format!("{}@dev-ops.test", id))
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

/// Insert a minimal asset that the developer can be linked to. Title +
/// monetary fields are throwaway — the API only checks that the asset
/// exists and is linked.
async fn insert_asset(pool: &PgPool, developer_user_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let slug = format!("test-villa-{}", id.simple());
    sqlx::query(
        r#"INSERT INTO assets (
                id, developer_user_id, title, slug, asset_type,
                total_value_cents, token_price_cents, tokens_total, tokens_available
            ) VALUES ($1, $2, 'Test Villa', $3, 'real_estate',
                      100000, 100, 1000, 1000)"#,
    )
    .bind(id)
    .bind(developer_user_id)
    .bind(slug)
    .execute(pool)
    .await
    .expect("insert asset");
    id
}

/// Insert an active developer→asset link so per-villa enforcement passes.
async fn link_developer_to_asset(pool: &PgPool, developer_user_id: Uuid, asset_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO developer_asset_links
                (developer_user_id, asset_id, effective_from, effective_until)
           VALUES ($1, $2, NOW(), NULL)"#,
    )
    .bind(developer_user_id)
    .bind(asset_id)
    .execute(pool)
    .await
    .expect("link developer to asset");
}

/// Cleanup helper. `villa_operations_log`, `villa_capex_events`, and
/// `developer_asset_links` carry append-only triggers that block DELETE
/// in normal operation; we set `session_replication_role = replica` to
/// bypass them, then restore. The same trick also bypasses the ON DELETE
/// RESTRICT FK on `assets`.
async fn cleanup_user_and_asset(pool: &PgPool, user_id: Uuid, asset_id: Option<Uuid>) {
    let _ = sqlx::query("SET session_replication_role = 'replica'")
        .execute(pool)
        .await;

    if let Some(asset_id) = asset_id {
        let _ = sqlx::query("DELETE FROM villa_period_documents WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM villa_annual_documents WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM villa_operations_current WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM villa_operations_log WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM villa_capex_events WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM developer_asset_links WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM asset_documents WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM assets WHERE id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
    }
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

    let _ = sqlx::query("SET session_replication_role = 'origin'")
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
    session_token: &str,
    json_body: serde_json::Value,
) -> Request<Body> {
    let csrf = "test-csrf-developer-operations";
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

/// Build a `multipart/form-data` POST with one file field and an arbitrary
/// number of form fields. Mirrors the pattern in `checkout_wallet_http.rs`.
fn multipart_post(
    uri: &str,
    session_token: &str,
    fields: &[(&str, &str)],
    file_field: &str,
    file_name: &str,
    file_content_type: &str,
    file_bytes: &[u8],
) -> Request<Body> {
    let csrf = "test-csrf-developer-operations";
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

    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(
            "cookie",
            format!("poool_session={}; csrf_token={}", session_token, csrf),
        )
        .header("X-CSRF-Token", csrf)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body))
        .unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Minimal valid create-ops payload. The `expense_other_notes` JSONB array
/// is the focus of the C-5 round-trip assertions.
fn create_ops_payload(
    period_year: i32,
    period_month: i32,
    expense_other_notes: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "period_year": period_year,
        "period_month": period_month,
        "gross_rental_idr_cents": 5_000_000_000_i64,
        "nights_available": 30,
        "nights_booked": 20,
        "expense_cleaning_idr_cents": 100_000_000_i64,
        "expense_maintenance_idr_cents": 0,
        "expense_utilities_idr_cents": 0,
        "expense_staff_idr_cents": 0,
        "expense_pool_garden_idr_cents": 0,
        "expense_pest_idr_cents": 0,
        "expense_other_idr_cents": 250_000_i64,
        "expense_property_tax_idr_cents": 0,
        "expense_insurance_idr_cents": 0,
        "expense_accounting_idr_cents": 0,
        "expense_internet_idr_cents": 0,
        "expense_capex_idr_cents": 0,
        "ota_fees_idr_cents": 0,
        "payment_fees_idr_cents": 0,
        "refunds_idr_cents": 0,
        "mgmt_fee_idr_cents": 0,
        "expense_other_notes": expense_other_notes,
    })
}

// ──────────────────────────────────────────────────────────────────────
// Page handlers
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn page_operations_dashboard_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/operations", Some(&session)))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, None).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("developer-operations-body") || body.contains("ops-matrix-tbody"),
        "expected operations dashboard markers; got: {:.200}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn page_operations_dashboard_redirects_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session("/developer/operations", None))
        .await
        .expect("oneshot");
    assert!(
        resp.status().is_redirection(),
        "expected redirect for anon, got {}",
        resp.status()
    );
}

#[ignore]
#[tokio::test]
async fn page_operations_submit_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/developer/villas/{}/operations/new", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("developer-operations-submit-body") || body.contains("dop-form"),
        "expected operations submit markers; got: {:.200}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn page_operations_log_edit_returns_200_for_developer() {
    // NEW route added 2026-05-19. Serves the same template as `…/new`;
    // the page-side JS detects edit mode by URL shape.
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            // log_id can be any i64 — the page renders before the API call
            // so this exercises only the route + auth gate.
            &format!("/developer/villas/{}/operations/999", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK);
}

// ──────────────────────────────────────────────────────────────────────
// /api/developer/operations/dashboard
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn api_operations_dashboard_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/api/developer/operations/dashboard",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, None).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("invalid json: {e}; body: {body}"));
    assert!(
        parsed.get("year").is_some() && parsed.get("assets").is_some(),
        "expected `year` and `assets` keys; got {body}"
    );
}

#[ignore]
#[tokio::test]
async fn api_operations_dashboard_returns_401_anon() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/api/developer/operations/dashboard",
            None,
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn api_operations_dashboard_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            "/api/developer/operations/dashboard",
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, None).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// Create + round-trip the `expense_other_notes` JSONB array (C-5 fix).
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn create_villa_operations_round_trips_expense_other_notes() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let custom = serde_json::json!([
        { "name": "Garbage collection", "amount_idr_cents": 150_000 },
        { "name": "Generator fuel",     "amount_idr_cents": 100_000 },
    ]);

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/operations", asset_id),
        &session,
        create_ops_payload(2025, 1, custom.clone()),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("invalid json: {e}; body: {body}"));
    let notes = parsed
        .get("expense_other_notes")
        .unwrap_or_else(|| panic!("response missing expense_other_notes; got {body}"));
    assert_eq!(
        notes, &custom,
        "expense_other_notes did not round-trip; got {notes}"
    );
}

#[ignore]
#[tokio::test]
async fn create_villa_operations_requires_active_link() {
    // Developer is authenticated and has the developer role, but no
    // developer_asset_links row → 403 on write endpoints (per-villa gate).
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/operations", asset_id),
        &session,
        create_ops_payload(2025, 2, serde_json::json!([])),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// List + get + update + submit
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn list_villa_operations_returns_200() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("invalid json: {e}; body: {body}"));
    assert!(
        parsed.is_array(),
        "list endpoint must return an array; got {body}"
    );
}

#[ignore]
#[tokio::test]
async fn list_villa_operations_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn get_single_villa_operations_log_round_trip() {
    // C-4 fix: single-log GET endpoint used by the edit page.
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Create first to get a log_id.
    let create_req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/operations", asset_id),
        &session,
        create_ops_payload(2025, 3, serde_json::json!([])),
    );
    let create_resp = app.clone().oneshot(create_req).await.expect("create");
    let create_status = create_resp.status();
    let create_body = body_string(create_resp).await;
    assert_eq!(
        create_status,
        StatusCode::OK,
        "create body: {}",
        create_body
    );
    let created: serde_json::Value = serde_json::from_str(&create_body).expect("parse create");
    let log_id = created
        .get("id")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| panic!("create response missing id: {create_body}"));

    // Fetch single log.
    let get_resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations/{}", asset_id, log_id),
            Some(&session),
        ))
        .await
        .expect("get");
    let get_status = get_resp.status();
    let get_body = body_string(get_resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(get_status, StatusCode::OK, "get body: {}", get_body);
    let parsed: serde_json::Value = serde_json::from_str(&get_body).expect("parse get");
    assert_eq!(parsed.get("id").and_then(|v| v.as_i64()), Some(log_id));
}

#[ignore]
#[tokio::test]
async fn update_villa_operations_round_trips_expense_other_notes() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Create.
    let create_req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/operations", asset_id),
        &session,
        create_ops_payload(2025, 4, serde_json::json!([])),
    );
    let create_resp = app.clone().oneshot(create_req).await.expect("create");
    let create_body = body_string(create_resp).await;
    let created: serde_json::Value = serde_json::from_str(&create_body).expect("parse");
    let log_id = created.get("id").and_then(|v| v.as_i64()).expect("log id");

    // Update with new expense_other_notes.
    let updated_notes = serde_json::json!([
        { "name": "Internet upgrade", "amount_idr_cents": 75_000 },
    ]);
    let update_req = mutating_with_session(
        Method::PUT,
        &format!("/api/developer/villas/{}/operations/{}", asset_id, log_id),
        &session,
        create_ops_payload(2025, 4, updated_notes.clone()),
    );
    let update_resp = app.oneshot(update_req).await.expect("update");
    let update_status = update_resp.status();
    let update_body = body_string(update_resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(
        update_status,
        StatusCode::OK,
        "update body: {}",
        update_body
    );
    let parsed: serde_json::Value = serde_json::from_str(&update_body).expect("parse update");
    assert_eq!(
        parsed.get("expense_other_notes"),
        Some(&updated_notes),
        "updated expense_other_notes must round-trip; got {update_body}"
    );
}

#[ignore]
#[tokio::test]
async fn update_villa_operations_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    // No link.

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PUT,
        &format!("/api/developer/villas/{}/operations/1", asset_id),
        &session,
        create_ops_payload(2025, 5, serde_json::json!([])),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn submit_villa_operations_flips_status_to_submitted() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    let create_req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/operations", asset_id),
        &session,
        create_ops_payload(2025, 6, serde_json::json!([])),
    );
    let create_resp = app.clone().oneshot(create_req).await.expect("create");
    let create_body = body_string(create_resp).await;
    let created: serde_json::Value = serde_json::from_str(&create_body).expect("parse");
    let log_id = created.get("id").and_then(|v| v.as_i64()).expect("log id");

    let submit_req = mutating_with_session(
        Method::PUT,
        &format!(
            "/api/developer/villas/{}/operations/{}/submit",
            asset_id, log_id
        ),
        &session,
        serde_json::json!({}),
    );
    let submit_resp = app.oneshot(submit_req).await.expect("submit");
    let submit_status = submit_resp.status();
    let submit_body = body_string(submit_resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(
        submit_status,
        StatusCode::OK,
        "submit body: {}",
        submit_body
    );
    let parsed: serde_json::Value = serde_json::from_str(&submit_body).expect("parse submit");
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("submitted"),
        "status must flip to 'submitted'; got {submit_body}"
    );
}

#[ignore]
#[tokio::test]
async fn submit_villa_operations_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    // No link.

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PUT,
        &format!("/api/developer/villas/{}/operations/1/submit", asset_id),
        &session,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// Document upload / list (multipart)
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn upload_period_document_succeeds() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Need an existing log row so the upload can attach to it.
    let create_req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/operations", asset_id),
        &session,
        create_ops_payload(2025, 7, serde_json::json!([])),
    );
    let create_resp = app.clone().oneshot(create_req).await.expect("create");
    let create_body = body_string(create_resp).await;
    let created: serde_json::Value = serde_json::from_str(&create_body).expect("parse");
    let log_id = created.get("id").and_then(|v| v.as_i64()).expect("log id");

    // Build the multipart request. The handler accepts PDF among other types.
    let pdf_bytes: &[u8] = b"%PDF-1.4\n%fake pdf for test\n%%EOF\n";
    let req = multipart_post(
        &format!(
            "/api/developer/villas/{}/operations/{}/documents",
            asset_id, log_id
        ),
        &session,
        &[("doc_type", "receipt")],
        "file",
        "test-receipt.pdf",
        "application/pdf",
        pdf_bytes,
    );
    let resp = app.oneshot(req).await.expect("upload");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "upload body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse upload");
    assert_eq!(
        parsed.get("doc_type").and_then(|v| v.as_str()),
        Some("receipt")
    );
    assert_eq!(
        parsed.get("period_year").and_then(|v| v.as_i64()),
        Some(2025)
    );
}

#[ignore]
#[tokio::test]
async fn upload_period_document_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    // No link.

    let app = build_platform_router(make_state(pool.clone()));
    let pdf_bytes: &[u8] = b"%PDF-1.4\n%fake\n%%EOF\n";
    let req = multipart_post(
        &format!("/api/developer/villas/{}/operations/1/documents", asset_id),
        &session,
        &[("doc_type", "receipt")],
        "file",
        "test.pdf",
        "application/pdf",
        pdf_bytes,
    );
    let resp = app.oneshot(req).await.expect("upload");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn list_period_documents_returns_200() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    // No log row necessary — listing accepts any log_id and just returns
    // an empty array when nothing matches.
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations/1/documents", asset_id),
            Some(&session),
        ))
        .await
        .expect("list");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert!(parsed.is_array(), "expected JSON array; got {body}");
}

#[ignore]
#[tokio::test]
async fn list_period_documents_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations/1/documents", asset_id),
            Some(&session),
        ))
        .await
        .expect("list");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// /api/developer/villas/:asset_id/asset-config
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn asset_config_returns_200_for_linked_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/asset-config", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert!(
        parsed.get("reserve_pct_bps").is_some(),
        "expected reserve_pct_bps in body; got {body}"
    );
}

#[ignore]
#[tokio::test]
async fn asset_config_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    // No link.

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/asset-config", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn asset_config_returns_401_anon() {
    let pool = pool().await;
    let asset_id = Uuid::new_v4(); // Doesn't matter — auth runs first.
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/asset-config", asset_id),
            None,
        ))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
