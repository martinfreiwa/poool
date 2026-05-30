//! HTTP-level integration tests for the **Developer Onboarding** surface
//! (`/developer/onboarding`, `/developer/application-form`,
//! `/developer/submission-success`, `/developer/document-upload-step3`,
//! and the `POST /api/developer/apply` API).
//!
//! Exercises the full Axum router that the production binary serves —
//! same handlers, same middleware (CSRF + Sentry context + security
//! headers). The router is built via `poool_backend::build_platform_router`
//! so any drift between the test-time router and the production one would
//! fail the build.
//!
//! ## Running
//!
//! All tests are `#[ignore]`d because they require a live Postgres
//! database. To exercise them:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test developer_onboarding_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//! Onboarding pages (HTML):
//!   * `GET /developer/onboarding` — 200 (authed, no role required);
//!     redirect to /auth/login for anonymous.
//!   * `GET /developer/application-form` — same shape; the form itself is
//!     pre-developer-role.
//!   * `GET /developer/submission-success?title=Sample` — 200 + body
//!     contains the `submitted-asset-title` ID. (The JS that fills the
//!     title runs client-side; server-side body is identical with and
//!     without `?title=`.)
//!   * `GET /developer/document-upload-step3` — 200 with developer role;
//!     redirect (to `/developer/application-form`) without it.
//!
//! `POST /api/developer/apply` (the C-1 fix at `routes.rs:482-607`):
//!   * `apply_persists_all_fields_and_returns_202` — happy path: full
//!     11-field payload → 202; row exists in `developer_applications`
//!     with `status='pending'` and every submitted field populated.
//!   * `apply_does_not_grant_developer_role` — **C-1 regression guard**:
//!     after a successful apply, the user does NOT have the `developer`
//!     role.
//!   * `apply_anonymous_returns_401` — POST without session → 401.
//!   * `apply_then_dashboard_stats_returns_403` — same applicant trying
//!     to access `/api/developer/dashboard/stats` → 403 (no role yet).

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
    .bind(format!("{}@dev-onboarding-http.test", id))
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
           VALUES ($1, $2, NULL, 'dev-onboarding-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
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
    let _ = sqlx::query("DELETE FROM audit_logs WHERE actor_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM developer_applications WHERE user_id = $1")
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
    let csrf = "test-csrf-onboarding-1234567890";
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

/// Canonical full-payload "Become a Developer" body. Field names mirror
/// the JSON keys read by `api_developer_apply` (see
/// `backend/src/developer/routes.rs:521-531`). All 11 text fields are
/// included so we can assert per-field persistence.
fn apply_payload_full() -> serde_json::Value {
    serde_json::json!({
        "first_name": "Test",
        "last_name": "Applicant",
        "phone": "+14155551234",
        "whatsapp": "+14155551234",
        "nationality": "US",
        "country": "United States",
        "website": "https://example.com",
        "assets_count": "5",
        "asset_value": "1000000",
        "monthly_income": "10000",
        "bio": "A short biography paragraph for the applicant.",
    })
}

// ──────────────────────────────────────────────────────────────────────
// GET /developer/onboarding
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn onboarding_page_returns_200_for_authed_user() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/onboarding", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "authed user must see the onboarding page (no developer role required)"
    );
}

#[ignore]
#[tokio::test]
async fn onboarding_page_redirects_anonymous_to_login() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/onboarding", None);
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let _ = body_string(resp).await;

    assert!(
        status == StatusCode::SEE_OTHER
            || status == StatusCode::TEMPORARY_REDIRECT
            || status == StatusCode::FOUND
            || status == StatusCode::MOVED_PERMANENTLY
            || status == StatusCode::PERMANENT_REDIRECT,
        "anonymous request must redirect; got {}",
        status
    );
    assert_eq!(
        location.as_deref(),
        Some("/auth/login"),
        "anonymous user must be redirected to /auth/login"
    );
}

// ──────────────────────────────────────────────────────────────────────
// GET /developer/application-form
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn application_form_page_returns_200_for_authed_user() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/application-form", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "application-form is pre-role and must be 200 for any authed user"
    );
}

#[ignore]
#[tokio::test]
async fn application_form_page_redirects_anonymous_to_login() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/application-form", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let _ = body_string(resp).await;

    assert!(
        status == StatusCode::SEE_OTHER
            || status == StatusCode::TEMPORARY_REDIRECT
            || status == StatusCode::FOUND
            || status == StatusCode::MOVED_PERMANENTLY
            || status == StatusCode::PERMANENT_REDIRECT,
        "anonymous request must redirect; got {}",
        status
    );
    assert_eq!(location.as_deref(), Some("/auth/login"));
}

// ──────────────────────────────────────────────────────────────────────
// GET /developer/submission-success
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn submission_success_page_with_title_query_renders_and_contains_anchor_id() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/submission-success?title=Sample", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK, "submission-success must be 200");
    assert!(
        body.contains("submitted-asset-title"),
        "page must contain the `submitted-asset-title` anchor element so the JS can fill it; body excerpt: {}",
        &body[..body.len().min(500)]
    );
}

#[ignore]
#[tokio::test]
async fn submission_success_page_without_title_query_keeps_hidden_attribute() {
    // The title element is server-rendered with the `hidden` attribute and
    // a client-side script removes it when `?title=` is present. With no
    // query param the JS doesn't fire on the server, so the element keeps
    // its `hidden` attribute in the response body.
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/submission-success", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        body.contains("submitted-asset-title"),
        "page must still contain the title element ID"
    );
    assert!(
        body.contains("hidden"),
        "title element must have the `hidden` attribute on initial render (JS unhides it client-side)"
    );
}

#[ignore]
#[tokio::test]
async fn submission_success_page_redirects_non_developer() {
    // submission-success is gated behind `require_developer_page` so a
    // user without the role gets a redirect (to application-form).
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/submission-success", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert!(
        status.is_redirection(),
        "non-developer must be redirected; got {}",
        status
    );
    assert!(
        location
            .as_deref()
            .map(|l| l.starts_with("/developer/") || l.starts_with("/auth/"))
            .unwrap_or(false),
        "redirect should target a developer or auth route; got {:?}",
        location
    );
}

// ──────────────────────────────────────────────────────────────────────
// GET /developer/document-upload-step3
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn document_upload_step3_returns_200_for_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    grant_developer(&pool, user).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/document-upload-step3", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "document upload page must be 200 for users with developer role"
    );
}

#[ignore]
#[tokio::test]
async fn document_upload_step3_redirects_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/document-upload-step3", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_user(&pool, user).await;

    assert!(
        status.is_redirection(),
        "non-developer must be redirected; got {}",
        status
    );
}

#[ignore]
#[tokio::test]
async fn document_upload_step3_redirects_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/developer/document-upload-step3", None);
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let location = resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let _ = body_string(resp).await;

    assert!(
        status.is_redirection(),
        "anonymous request must redirect; got {}",
        status
    );
    assert_eq!(location.as_deref(), Some("/auth/login"));
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/developer/apply  (the C-1 fix surface)
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn apply_persists_all_fields_and_returns_202() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    // Sanity: user starts with no developer role.
    assert!(
        !user_is_developer(&pool, user).await,
        "fixture user must start without the developer role"
    );

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/apply",
        Some(&session),
        apply_payload_full(),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    // Read the persisted application row back. There must be exactly one
    // row for this user, status='pending', and every text field
    // populated from the payload above.
    let row: Option<(
        Option<String>, // first_name
        Option<String>, // last_name
        Option<String>, // phone
        Option<String>, // whatsapp
        Option<String>, // nationality
        Option<String>, // country
        Option<String>, // website
        Option<String>, // assets_count
        Option<String>, // asset_value
        Option<String>, // monthly_income
        Option<String>, // bio
        String,         // status
    )> = sqlx::query_as(
        "SELECT first_name, last_name, phone, whatsapp,
                nationality, country, website,
                assets_count, asset_value, monthly_income, bio,
                status
         FROM developer_applications WHERE user_id = $1
         ORDER BY submitted_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("read application row");

    cleanup_user(&pool, user).await;

    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "POST /api/developer/apply must return 202 Accepted; body: {}",
        body
    );
    // Confirm the JSON envelope shape so any silent breaking change here trips this test.
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("response is JSON");
    assert_eq!(
        parsed.get("ok").and_then(|v| v.as_bool()),
        Some(true),
        "ok=true must be present; body: {}",
        body
    );
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("pending"),
        "status field must be 'pending' on the wire; body: {}",
        body
    );

    let row = row.expect("an application row must exist after a successful POST");
    assert_eq!(row.0.as_deref(), Some("Test"), "first_name persisted");
    assert_eq!(row.1.as_deref(), Some("Applicant"), "last_name persisted");
    assert_eq!(row.2.as_deref(), Some("+14155551234"), "phone persisted");
    assert_eq!(row.3.as_deref(), Some("+14155551234"), "whatsapp persisted");
    assert_eq!(row.4.as_deref(), Some("US"), "nationality persisted");
    assert_eq!(row.5.as_deref(), Some("United States"), "country persisted");
    assert_eq!(
        row.6.as_deref(),
        Some("https://example.com"),
        "website persisted"
    );
    assert_eq!(row.7.as_deref(), Some("5"), "assets_count persisted");
    assert_eq!(row.8.as_deref(), Some("1000000"), "asset_value persisted");
    assert_eq!(row.9.as_deref(), Some("10000"), "monthly_income persisted");
    assert_eq!(
        row.10.as_deref(),
        Some("A short biography paragraph for the applicant."),
        "bio persisted"
    );
    assert_eq!(
        row.11, "pending",
        "freshly-submitted application must have status='pending'"
    );
}

/// **C-1 regression guard.** The pre-fix handler granted the `developer`
/// role to anyone who POSTed once. The fix removed that side-effect — a
/// successful POST must persist a row but NOT mutate `user_roles`.
#[ignore]
#[tokio::test]
async fn apply_does_not_grant_developer_role() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    assert!(
        !user_is_developer(&pool, user).await,
        "fixture user must start without the developer role"
    );

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/apply",
        Some(&session),
        apply_payload_full(),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;
    assert_eq!(status, StatusCode::ACCEPTED, "happy path must be 202");

    let promoted = user_is_developer(&pool, user).await;

    cleanup_user(&pool, user).await;

    assert!(
        !promoted,
        "C-1 regression: POST /api/developer/apply must NOT grant the developer role"
    );
}

#[ignore]
#[tokio::test]
async fn apply_anonymous_returns_401() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/apply",
        None,
        apply_payload_full(),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let _ = body_string(resp).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "anonymous POST must be 401"
    );
}

/// After a successful application, the applicant still cannot access the
/// developer dashboard JSON. This is the end-to-end proof that the C-1
/// fix actually disconnects "applied" from "is a developer".
#[ignore]
#[tokio::test]
async fn apply_then_dashboard_stats_returns_403() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));

    // 1) Apply.
    let apply_resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            Some(&session),
            apply_payload_full(),
        ))
        .await
        .expect("apply");
    let apply_status = apply_resp.status();
    let _ = body_string(apply_resp).await;
    assert_eq!(
        apply_status,
        StatusCode::ACCEPTED,
        "apply must succeed first"
    );

    // 2) Probe the developer-only stats endpoint.
    let stats_resp = app
        .oneshot(get_with_session(
            "/api/developer/dashboard/stats",
            Some(&session),
        ))
        .await
        .expect("stats");
    let stats_status = stats_resp.status();
    let _ = body_string(stats_resp).await;

    cleanup_user(&pool, user).await;

    assert_eq!(
        stats_status,
        StatusCode::FORBIDDEN,
        "applicant must still be denied 403 — no developer role until admin approves"
    );
}

/// Whitespace + empty-string handling: the handler trims values and stores
/// `None` for empty strings. The row should still be inserted so the
/// admin queue captures the submission even if the applicant left optional
/// fields blank.
#[ignore]
#[tokio::test]
async fn apply_trims_empty_strings_to_null() {
    let pool = pool().await;
    let user = insert_user(&pool).await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/apply",
        Some(&session),
        serde_json::json!({
            "first_name": "Solo",
            "last_name": "",
            "website": "   ",
            "country": "France",
        }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    let row: Option<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT first_name, last_name, website, country
             FROM developer_applications WHERE user_id = $1
             ORDER BY submitted_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_optional(&pool)
    .await
    .expect("read");

    cleanup_user(&pool, user).await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let row = row.expect("row exists");
    assert_eq!(row.0.as_deref(), Some("Solo"));
    assert!(
        row.1.is_none(),
        "empty-string last_name must persist as NULL; got {:?}",
        row.1
    );
    assert!(
        row.2.is_none(),
        "whitespace-only website must persist as NULL; got {:?}",
        row.2
    );
    assert_eq!(row.3.as_deref(), Some("France"));
}
