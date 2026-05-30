//! HTTP-level integration tests for the Developer Annual-Data surface
//! (Villa-Returns C3 — annual documents, CapEx events, forecast
//! suggestions, annual summary rollup).
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
//!     cargo test --test developer_annual_data_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! Page handler:
//!   * GET /developer/villas/:asset_id/annual/:year
//!
//! API handlers (all DeveloperUser + asset-link gated):
//!   * POST /api/developer/villas/:asset_id/annual/:year/documents (multipart)
//!   * GET  /api/developer/villas/:asset_id/annual/:year/documents
//!   * POST /api/developer/villas/:asset_id/capex
//!   * GET  /api/developer/villas/:asset_id/capex
//!   * POST /api/developer/villas/:asset_id/forecast/:year/suggest
//!   * GET  /api/developer/villas/:asset_id/forecast/:year/suggestions
//!   * GET  /api/developer/villas/:asset_id/annual/:year/summary
//!
//! Per-villa enforcement: every write endpoint is also tested without the
//! `developer_asset_links` row to confirm 403.
//!
//! ## Cleanup note
//!
//! `villa_capex_events` and `developer_asset_links` carry append-only
//! triggers that block ordinary DELETE. The cleanup helper temporarily
//! switches `session_replication_role = replica` to bypass triggers + FK
//! checks while it drops test rows. Same trick used in
//! `developer_operations_http.rs`.

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
        webauthn: test_webauthn(),
    }
}

async fn insert_user(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status, email_verified)
         VALUES ($1, $2, 'x', 'active', TRUE)",
    )
    .bind(id)
    .bind(format!("{}@dev-annual.test", id))
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

async fn insert_asset(pool: &PgPool, developer_user_id: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    let slug = format!("test-villa-{}", id.simple());
    sqlx::query(
        r#"INSERT INTO assets (
                id, developer_user_id, title, slug, asset_type,
                total_value_cents, token_price_cents, tokens_total, tokens_available
            ) VALUES ($1, $2, 'Annual Test Villa', $3, 'real_estate',
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

/// Drops fixture rows. Bypasses append-only trigger guards via
/// `session_replication_role = 'replica'`.
async fn cleanup_user_and_asset(pool: &PgPool, user_id: Uuid, asset_id: Option<Uuid>) {
    let _ = sqlx::query("SET session_replication_role = 'replica'")
        .execute(pool)
        .await;

    if let Some(asset_id) = asset_id {
        let _ = sqlx::query("DELETE FROM villa_forecast_suggestions WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
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
    let csrf = "test-csrf-developer-annual";
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

fn multipart_post(
    uri: &str,
    session_token: &str,
    fields: &[(&str, &str)],
    file_field: &str,
    file_name: &str,
    file_content_type: &str,
    file_bytes: &[u8],
) -> Request<Body> {
    let csrf = "test-csrf-developer-annual";
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

// ──────────────────────────────────────────────────────────────────────
// Page handler
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn page_annual_data_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/developer/villas/{}/annual/2025", asset_id),
            Some(&session),
        ))
        .await
        .expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        body.contains("developer-annual-data-body") || body.contains("dad-summary"),
        "expected annual-data markers; got: {:.200}",
        body
    );
}

#[ignore]
#[tokio::test]
async fn page_annual_data_redirects_anon() {
    let pool = pool().await;
    let asset_id = Uuid::new_v4();
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/developer/villas/{}/annual/2025", asset_id),
            None,
        ))
        .await
        .expect("oneshot");
    assert!(
        resp.status().is_redirection(),
        "expected redirect for anon, got {}",
        resp.status()
    );
}

// ──────────────────────────────────────────────────────────────────────
// Annual documents — upload + list (multipart upload requires the full
// transaction path through asset_documents + villa_annual_documents).
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn upload_annual_document_succeeds() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let pdf_bytes: &[u8] = b"%PDF-1.4\n%annual statement fake\n%%EOF\n";
    let req = multipart_post(
        &format!("/api/developer/villas/{}/annual/2025/documents", asset_id),
        &session,
        &[("doc_type", "tax_statement")],
        "file",
        "annual.pdf",
        "application/pdf",
        pdf_bytes,
    );
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app.oneshot(req).await.expect("upload");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse upload");
    assert_eq!(
        parsed.get("doc_type").and_then(|v| v.as_str()),
        Some("tax_statement")
    );
    assert_eq!(
        parsed.get("period_year").and_then(|v| v.as_i64()),
        Some(2025)
    );
}

#[ignore]
#[tokio::test]
async fn upload_annual_document_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    // No link.

    let pdf_bytes: &[u8] = b"%PDF-1.4\nfake\n%%EOF\n";
    let req = multipart_post(
        &format!("/api/developer/villas/{}/annual/2025/documents", asset_id),
        &session,
        &[("doc_type", "tax_statement")],
        "file",
        "annual.pdf",
        "application/pdf",
        pdf_bytes,
    );
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app.oneshot(req).await.expect("upload");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn list_annual_documents_returns_200() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/annual/2025/documents", asset_id),
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
async fn list_annual_documents_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/annual/2025/documents", asset_id),
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
// CapEx — create + list
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn create_capex_event_succeeds() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/capex", asset_id),
        &session,
        serde_json::json!({
            "event_date": "2025-06-15",
            "amount_idr_cents": 12_000_000_i64,
            "category": "renovation",
            "description": "Replaced pool tiles",
            "evidence_doc_id": null,
        }),
    );
    let resp = app.oneshot(req).await.expect("create capex");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("submitted"),
        "developer-submitted CapEx starts in 'submitted' state; got {body}"
    );
    assert_eq!(
        parsed.get("category").and_then(|v| v.as_str()),
        Some("renovation")
    );
}

#[ignore]
#[tokio::test]
async fn create_capex_event_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/capex", asset_id),
        &session,
        serde_json::json!({
            "event_date": "2025-06-15",
            "amount_idr_cents": 12_000_000_i64,
            "category": "renovation",
            "description": "x",
            "evidence_doc_id": null,
        }),
    );
    let resp = app.oneshot(req).await.expect("create");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn list_capex_returns_200() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/capex", asset_id),
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
async fn list_capex_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/capex", asset_id),
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
// Forecast suggestions
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn create_forecast_suggestion_succeeds() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/forecast/2025/suggest", asset_id),
        &session,
        serde_json::json!({
            "projected_occupancy_bps": 7500,
            "projected_adr_idr_cents": 2_500_000_i64,
            "projected_rent_growth_bps": 500,
            "projected_expense_inflation_bps": 300,
            "projected_appreciation_bps": 400,
            "projected_exit_yield_bps": 800,
            "notes": "Test forecast",
        }),
    );
    let resp = app.oneshot(req).await.expect("suggest");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert_eq!(
        parsed.get("forecast_year").and_then(|v| v.as_i64()),
        Some(2025)
    );
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("submitted")
    );
}

#[ignore]
#[tokio::test]
async fn create_forecast_suggestion_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/developer/villas/{}/forecast/2025/suggest", asset_id),
        &session,
        serde_json::json!({
            "projected_occupancy_bps": 7500,
            "projected_adr_idr_cents": 1_000_000_i64,
        }),
    );
    let resp = app.oneshot(req).await.expect("suggest");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn list_forecast_suggestions_returns_200() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!(
                "/api/developer/villas/{}/forecast/2025/suggestions",
                asset_id
            ),
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
async fn list_forecast_suggestions_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!(
                "/api/developer/villas/{}/forecast/2025/suggestions",
                asset_id
            ),
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
// Annual summary rollup
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn annual_summary_returns_200() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;
    link_developer_to_asset(&pool, user, asset_id).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/annual/2025/summary", asset_id),
            Some(&session),
        ))
        .await
        .expect("summary");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse");
    assert!(
        parsed.get("forecast_year").is_some() && parsed.get("months_published").is_some(),
        "summary must include forecast_year + months_published; got {body}"
    );
}

#[ignore]
#[tokio::test]
async fn annual_summary_requires_active_link() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;
    let asset_id = insert_asset(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/annual/2025/summary", asset_id),
            Some(&session),
        ))
        .await
        .expect("summary");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user_and_asset(&pool, user, Some(asset_id)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn annual_summary_returns_401_anon() {
    let pool = pool().await;
    let asset_id = Uuid::new_v4();
    let app = build_platform_router(make_state(pool.clone()));
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/annual/2025/summary", asset_id),
            None,
        ))
        .await
        .expect("summary");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
