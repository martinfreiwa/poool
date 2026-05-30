//! HTTP-level integration tests for the **admin** review surface for
//! "Become a Developer" applications:
//!
//!   * `GET  /api/admin/developer-applications`             (list / filter)
//!   * `POST /api/admin/developer-applications/:id/approve` (C-3 KYC-gated)
//!   * `POST /api/admin/developer-applications/:id/reject`
//!
//! Exercises the full Axum router that the production binary serves —
//! same handlers, same middleware. The router is built via
//! `poool_backend::build_platform_router` so any drift between the
//! test-time router and the production one would fail the build.
//!
//! ## Running
//!
//! All tests are `#[ignore]`d because they require a live Postgres
//! database. To exercise them:
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test admin_developer_applications_http -- --ignored
//! ```
//!
//! ## Coverage
//!
//!  List:
//!    * `list_returns_pending_applications_for_admin` — seed 2 pending
//!      apps, GET → 200, both visible.
//!    * `list_status_filter_pending_only_returns_pending` — `?status=pending`
//!      filters out non-pending rows.
//!    * `list_anonymous_returns_401`.
//!    * `list_developer_only_returns_403` — non-admin (developer role only).
//!
//!  Approve (C-3 KYC gating + role grant + audit trail):
//!    * `approve_without_kyc_returns_400_and_flips_to_needs_kyc` —
//!      applicant has no `kyc_records` → 400, application status
//!      becomes `needs_kyc`, applicant still has no developer role.
//!    * `approve_with_kyc_grants_role_and_marks_approved` — happy path:
//!      grants role, sets `status='approved'`, `reviewed_by`, `reviewed_at`,
//!      `kyc_verified_at`, and writes the `admin.developer_application_approved`
//!      audit row.
//!    * `approve_as_non_admin_returns_403`.
//!    * `approve_anonymous_returns_401`.
//!    * `approve_already_approved_returns_409` — concurrency / double-submit
//!      guard surfaces 409 Conflict.
//!
//!  Reject:
//!    * `reject_marks_application_rejected` — notes persisted, applicant
//!      did NOT receive developer role.
//!    * `reject_as_non_admin_returns_403`.

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
    .bind(format!("{}@admin-dev-apps-http.test", id))
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
           VALUES ($1, $2, NULL, 'admin-dev-apps-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
    )
    .bind(user_id)
    .bind(&token)
    .execute(pool)
    .await
    .expect("insert session");
    token
}

/// Assign the named role to the user, ensuring the role row exists first.
/// The `admin` role already has the special `'all'` permission grant
/// (migration 006), so an `admin`-tagged user passes every
/// `admin.require_permission(...)` check the handlers issue.
async fn assign_role(pool: &PgPool, user_id: Uuid, role_name: &str) {
    sqlx::query("INSERT INTO roles (name) VALUES ($1) ON CONFLICT (name) DO NOTHING")
        .bind(role_name)
        .execute(pool)
        .await
        .ok();
    sqlx::query(
        r#"INSERT INTO user_roles (user_id, role_id, is_active)
           SELECT $1, id, TRUE FROM roles WHERE name = $2
           ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE"#,
    )
    .bind(user_id)
    .bind(role_name)
    .execute(pool)
    .await
    .expect("assign role");
}

/// Insert a `developer_applications` row in the requested status and
/// return its ID. Uses the canonical 11-field payload so the row looks
/// realistic in the admin list response.
async fn seed_application(pool: &PgPool, user_id: Uuid, status: &str) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO developer_applications (
              user_id, first_name, last_name, phone, whatsapp,
              nationality, country, website,
              assets_count, asset_value, monthly_income, bio,
              status
           ) VALUES ($1, 'Seed', 'Applicant', '+10000000000', '+10000000000',
                     'US', 'United States', 'https://example.com',
                     '1', '100000', '5000', 'Seed bio',
                     $2)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("seed application");
    id
}

/// Stamp an `approved` kyc_records row for the user. The approve handler
/// looks for `status='approved' AND verified_at IS NOT NULL` so we set
/// both.
async fn approve_kyc(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO kyc_records (user_id, provider, status, verified_at)
         VALUES ($1, 'sumsub', 'approved', NOW())",
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert kyc record");
}

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
    let _ = sqlx::query("DELETE FROM kyc_records WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM developer_applications WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    // Some rows also reference user via `reviewed_by`; clear those.
    let _ =
        sqlx::query("UPDATE developer_applications SET reviewed_by = NULL WHERE reviewed_by = $1")
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

/// Drop an application + the applicant user atomically. The admin user
/// is cleaned up separately by the test.
async fn cleanup_application(pool: &PgPool, app_id: Uuid) {
    let _ = sqlx::query("DELETE FROM developer_applications WHERE id = $1")
        .bind(app_id)
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
    let csrf = "test-csrf-admin-dev-apps-1234567890";
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
// GET /api/admin/developer-applications
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn list_returns_pending_applications_for_admin() {
    let pool = pool().await;
    let admin = insert_user(&pool).await;
    assign_role(&pool, admin, "admin").await;
    let session = mint_session(&pool, admin).await;

    let app1_user = insert_user(&pool).await;
    let app2_user = insert_user(&pool).await;
    let app1_id = seed_application(&pool, app1_user, "pending").await;
    let app2_id = seed_application(&pool, app2_user, "pending").await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/admin/developer-applications", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_application(&pool, app1_id).await;
    cleanup_application(&pool, app2_id).await;
    cleanup_user(&pool, app1_user).await;
    cleanup_user(&pool, app2_user).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "admin list must return 200; body: {}",
        body
    );

    let parsed: serde_json::Value = serde_json::from_str(&body).expect("response is JSON");
    let items = parsed
        .get("applications")
        .and_then(|v| v.as_array())
        .expect("`applications` array on response");

    let ids: Vec<String> = items
        .iter()
        .filter_map(|i| i.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert!(
        ids.contains(&app1_id.to_string()),
        "app1 must appear in admin list; ids: {:?}",
        ids
    );
    assert!(
        ids.contains(&app2_id.to_string()),
        "app2 must appear in admin list; ids: {:?}",
        ids
    );
}

#[ignore]
#[tokio::test]
async fn list_status_filter_pending_only_returns_pending() {
    let pool = pool().await;
    let admin = insert_user(&pool).await;
    assign_role(&pool, admin, "admin").await;
    let session = mint_session(&pool, admin).await;

    let pending_user = insert_user(&pool).await;
    let rejected_user = insert_user(&pool).await;
    let pending_id = seed_application(&pool, pending_user, "pending").await;
    let rejected_id = seed_application(&pool, rejected_user, "rejected").await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        "/api/admin/developer-applications?status=pending",
        Some(&session),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_application(&pool, pending_id).await;
    cleanup_application(&pool, rejected_id).await;
    cleanup_user(&pool, pending_user).await;
    cleanup_user(&pool, rejected_user).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    let items = parsed
        .get("applications")
        .and_then(|v| v.as_array())
        .expect("applications array");

    // All rows must have status='pending' AND our pending row must be
    // present AND our rejected row must be absent.
    for it in items {
        assert_eq!(
            it.get("status").and_then(|v| v.as_str()),
            Some("pending"),
            "rows under ?status=pending must all be 'pending'; got {}",
            it
        );
    }
    let ids: Vec<String> = items
        .iter()
        .filter_map(|i| i.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert!(
        ids.contains(&pending_id.to_string()),
        "filter must include pending app; ids: {:?}",
        ids
    );
    assert!(
        !ids.contains(&rejected_id.to_string()),
        "filter must exclude rejected app; ids: {:?}",
        ids
    );
}

#[ignore]
#[tokio::test]
async fn list_anonymous_returns_401() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/admin/developer-applications", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn list_developer_only_returns_403() {
    let pool = pool().await;
    let dev = insert_user(&pool).await;
    assign_role(&pool, dev, "developer").await;
    let session = mint_session(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/admin/developer-applications", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let _ = body_string(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "developer role alone must not satisfy the admin extractor"
    );
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/admin/developer-applications/:id/approve
// ──────────────────────────────────────────────────────────────────────

/// C-3 fix: an applicant without an `approved` `kyc_records` row must NOT
/// be approved. The handler flips `status='needs_kyc'`, returns 400, and
/// must NOT grant the developer role.
#[ignore]
#[tokio::test]
async fn approve_without_kyc_returns_400_and_flips_to_needs_kyc() {
    let pool = pool().await;
    let admin = insert_user(&pool).await;
    assign_role(&pool, admin, "admin").await;
    let session = mint_session(&pool, admin).await;

    let applicant = insert_user(&pool).await;
    let app_id = seed_application(&pool, applicant, "pending").await;

    // Sanity: no kyc_records row, no developer role.
    assert!(!user_is_developer(&pool, applicant).await);

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/admin/developer-applications/{}/approve", app_id),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    let row: (String, Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "SELECT status, reviewed_by, reviewed_at FROM developer_applications WHERE id = $1",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("read app row");

    let promoted = user_is_developer(&pool, applicant).await;

    cleanup_application(&pool, app_id).await;
    cleanup_user(&pool, applicant).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "approve without KYC must be 400; body: {}",
        body
    );
    assert!(
        body.contains("applicant must complete KYC before approval"),
        "error message must explain the gate; body: {}",
        body
    );
    assert_eq!(
        row.0, "needs_kyc",
        "application status must flip to 'needs_kyc' to triage the row"
    );
    assert!(
        row.1.is_some(),
        "reviewed_by must be recorded (admin attempted)"
    );
    assert!(row.2.is_some(), "reviewed_at must be set");
    assert!(
        !promoted,
        "C-3 regression: applicant without KYC must not receive the developer role"
    );
}

#[ignore]
#[tokio::test]
async fn approve_with_kyc_grants_role_and_marks_approved() {
    let pool = pool().await;
    let admin = insert_user(&pool).await;
    assign_role(&pool, admin, "admin").await;
    let session = mint_session(&pool, admin).await;

    let applicant = insert_user(&pool).await;
    let app_id = seed_application(&pool, applicant, "pending").await;
    approve_kyc(&pool, applicant).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/admin/developer-applications/{}/approve", app_id),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    let row: (
        String,
        Option<Uuid>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT status, reviewed_by, reviewed_at, kyc_verified_at
         FROM developer_applications WHERE id = $1",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("read app row");

    let promoted = user_is_developer(&pool, applicant).await;

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_logs
          WHERE action = 'admin.developer_application_approved'
            AND entity_id = $1",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("audit count");

    cleanup_application(&pool, app_id).await;
    cleanup_user(&pool, applicant).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(status, StatusCode::OK, "approve happy path: body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
    assert_eq!(
        parsed.get("status").and_then(|v| v.as_str()),
        Some("approved"),
        "response status field must be 'approved'"
    );

    assert_eq!(row.0, "approved", "application row status must be approved");
    assert_eq!(
        row.1,
        Some(admin),
        "reviewed_by must be the admin who approved"
    );
    assert!(row.2.is_some(), "reviewed_at must be set");
    assert!(
        row.3.is_some(),
        "kyc_verified_at must be snapshotted at approval time"
    );

    assert!(
        promoted,
        "applicant must now have the developer role after approval"
    );

    assert_eq!(
        audit_count, 1,
        "approval must write exactly one `admin.developer_application_approved` audit log"
    );
}

#[ignore]
#[tokio::test]
async fn approve_as_non_admin_returns_403() {
    let pool = pool().await;
    let dev = insert_user(&pool).await;
    assign_role(&pool, dev, "developer").await;
    let session = mint_session(&pool, dev).await;

    let applicant = insert_user(&pool).await;
    let app_id = seed_application(&pool, applicant, "pending").await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/admin/developer-applications/{}/approve", app_id),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_application(&pool, app_id).await;
    cleanup_user(&pool, applicant).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "developer role alone must not be able to approve"
    );
}

#[ignore]
#[tokio::test]
async fn approve_anonymous_returns_401() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!(
            "/api/admin/developer-applications/{}/approve",
            Uuid::new_v4()
        ),
        None,
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Concurrency / double-submit guard: re-approving an already-approved
/// application must surface 409 Conflict (see
/// `developer_applications.rs:155`).
#[ignore]
#[tokio::test]
async fn approve_already_approved_returns_409() {
    let pool = pool().await;
    let admin = insert_user(&pool).await;
    assign_role(&pool, admin, "admin").await;
    let session = mint_session(&pool, admin).await;

    let applicant = insert_user(&pool).await;
    let app_id = seed_application(&pool, applicant, "approved").await;
    // KYC is irrelevant — the 409 check fires before the KYC gate.

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/admin/developer-applications/{}/approve", app_id),
        Some(&session),
        serde_json::json!({}),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    cleanup_application(&pool, app_id).await;
    cleanup_user(&pool, applicant).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "double-approve must be 409; body: {}",
        body
    );
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/admin/developer-applications/:id/reject
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn reject_marks_application_rejected() {
    let pool = pool().await;
    let admin = insert_user(&pool).await;
    assign_role(&pool, admin, "admin").await;
    let session = mint_session(&pool, admin).await;

    let applicant = insert_user(&pool).await;
    let app_id = seed_application(&pool, applicant, "pending").await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/admin/developer-applications/{}/reject", app_id),
        Some(&session),
        serde_json::json!({ "notes": "Doesn't meet criteria" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let body = body_string(resp).await;

    let row: (
        String,
        Option<String>,
        Option<Uuid>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = sqlx::query_as(
        "SELECT status, review_notes, reviewed_by, reviewed_at
         FROM developer_applications WHERE id = $1",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("read app row");

    let promoted = user_is_developer(&pool, applicant).await;

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM audit_logs
          WHERE action = 'admin.developer_application_rejected'
            AND entity_id = $1",
    )
    .bind(app_id)
    .fetch_one(&pool)
    .await
    .expect("audit count");

    cleanup_application(&pool, app_id).await;
    cleanup_user(&pool, applicant).await;
    cleanup_user(&pool, admin).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "reject must return 200; body: {}",
        body
    );

    assert_eq!(
        row.0, "rejected",
        "application status must become 'rejected'"
    );
    assert_eq!(
        row.1.as_deref(),
        Some("Doesn't meet criteria"),
        "review_notes must persist verbatim"
    );
    assert_eq!(
        row.2,
        Some(admin),
        "reviewed_by must be the rejecting admin"
    );
    assert!(row.3.is_some(), "reviewed_at must be set");

    assert!(!promoted, "rejection must NOT grant the developer role");
    assert_eq!(
        audit_count, 1,
        "rejection must write exactly one audit_logs row"
    );
}

#[ignore]
#[tokio::test]
async fn reject_as_non_admin_returns_403() {
    let pool = pool().await;
    let dev = insert_user(&pool).await;
    assign_role(&pool, dev, "developer").await;
    let session = mint_session(&pool, dev).await;

    let applicant = insert_user(&pool).await;
    let app_id = seed_application(&pool, applicant, "pending").await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        &format!("/api/admin/developer-applications/{}/reject", app_id),
        Some(&session),
        serde_json::json!({ "notes": "should not work" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup_application(&pool, app_id).await;
    cleanup_user(&pool, applicant).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "developer role alone must not be able to reject"
    );
}
