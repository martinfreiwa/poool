//! End-to-end workflow tests for the POOOL developer journey.
//!
//! These exercise the full HTTP/JSON contract that a real developer hits as
//! they walk from "I'd like to be a developer" all the way to "an admin has
//! approved my monthly operations log". Every test drives the same router the
//! production binary serves — built via `poool_backend::build_platform_router`
//! — so any drift between test-time and prod routing fails the build.
//!
//! ## Workflow under test (happy path)
//!
//!   1. Anonymous user signs up & a session is minted.
//!   2. User POSTs `/api/developer/apply` with the 11-field application →
//!      202 Accepted + `developer_applications` row with `status='pending'`.
//!      The user does NOT yet have the developer role.
//!   3. User attempts `/api/developer/draft` → 403 (no role).
//!   4. KYC completes externally — simulated here by directly inserting a
//!      `kyc_records (user_id, status='approved', verified_at=NOW())` row.
//!   5. Admin GETs `/api/admin/developer-applications?status=pending` and
//!      sees the application in the queue.
//!   6. Admin POSTs `/api/admin/developer-applications/:id/approve` → 200;
//!      user gains the `developer` role; application status flips to
//!      `approved`; `kyc_verified_at` is snapshotted on the row.
//!   7. User POSTs `/api/developer/draft` → 201/200 + draft UUID.
//!   8. User PUTs `/api/developer/draft/:id` with content + financials → 200.
//!   9. User POSTs `/api/developer/draft/:id/submit` → 200; the draft's
//!      `developer_projects.status` transitions to `submitted`.
//!  10. Admin POSTs `/api/admin/submissions/:asset_id/approve` → 200; the
//!      asset is published and `developer_projects.status` becomes `live`.
//!  11. The asset is now live. The user is linked to it via
//!      `developer_asset_links` (inserted directly, since that admin
//!      surface is exercised by the admin/villa_developer_access module
//!      and isn't part of this test's scope).
//!  12. User POSTs `/api/developer/villas/:asset_id/operations` for period
//!      2026-03 with full operations payload **including** `custom_expenses`
//!      as `expense_other_notes` JSONB.
//!  13. User PUTs `…/operations/:log_id/submit` to lock the log.
//!  14. Admin PUTs `/api/admin/villas/:asset_id/operations/:log_id/approve`
//!      → 200; status transitions draft → submitted → approved.
//!  15. The `expense_other_notes` payload round-trips through a fresh GET
//!      with the literal `<script>` text preserved (sanitisation is the
//!      view-layer's job, not the JSON boundary's).
//!  16. Cleanup: every seeded row is removed by UUID.
//!
//! ## Security-rejection paths (each is a separate `#[ignore]`d test)
//!
//!  1. Anonymous tries onboarding apply → 401.
//!  2. Authed user without KYC tries to be approved → admin's POST returns
//!     400 + application moves to `status='needs_kyc'` + user stays role-less.
//!  3. Non-admin (with the `developer` role!) tries to approve → 403.
//!  4. User tries to POST draft before being granted developer role → 403
//!     (C-2 regression guard).
//!  5. User tries to POST draft after rejection → 403.
//!  6. Developer A tries to read Developer B's draft → 403 (ownership).
//!  7. Developer A tries to submit operations for a villa they don't own →
//!     403 via `require_asset_link`.
//!  8. User tries to apply twice while pending — documents the *current*
//!     behaviour: each POST returns 202 + inserts a fresh row (no UNIQUE
//!     constraint on user_id; the apply handler is intentionally append-only
//!     so re-submission with corrections is allowed).
//!  9. Admin tries to approve an already-approved application → 409
//!     Conflict (see `api_admin_approve_developer_application`).
//! 10. `custom_expenses` with a `<script>` payload — round-trips through DB
//!     with the literal text preserved on both INSERT and GET. The view
//!     layer is responsible for HTML-escaping, not the JSON boundary.
//!
//! ## Running
//!
//! Every test is `#[ignore]`d because they all hit a live Postgres database.
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     cargo test --test developer_workflow_e2e -- --ignored
//! ```
//!
//! ## Notes
//!
//! - The asset-approval admin endpoint is `POST /api/admin/submissions/
//!   :asset_id/approve` (see `backend/src/admin/submissions.rs`). This
//!   flips `assets.published = TRUE` and updates `developer_projects.status`
//!   to `'live'`. It's reused here as step 10 of the happy path.
//! - `developer_asset_links` is append-only (see migration 142 + its trigger
//!   `fn_developer_asset_links_guard`); cleanup sets `effective_until`
//!   rather than DELETEing — and the rows go away anyway when the asset is
//!   dropped via `ON DELETE CASCADE`.
//! - The application apply handler doesn't impose a UNIQUE constraint on
//!   user_id (see migration 203). Test #8 documents that as the *current*
//!   behaviour — see comment on the test.

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
// Shared test infrastructure (mirrors the pattern in leaderboard_http.rs)
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

/// Build an `AppState` against the given pool. Same shape as the helper in
/// `leaderboard_http.rs` so the constructed router behaves identically to
/// the production binary.
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

/// Insert a minimal user row and return its UUID. `email_verified = TRUE`
/// is required because `get_user_by_session` filters unverified users.
async fn insert_user(pool: &PgPool, email_prefix: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status, email_verified)
         VALUES ($1, $2, 'x', 'active', TRUE)",
    )
    .bind(id)
    .bind(format!("{}-{}@dev-e2e.test", email_prefix, id))
    .execute(pool)
    .await
    .expect("insert user");
    id
}

/// Insert a 24-hour session row for `user_id` and return the session token
/// to attach as a `poool_session` cookie.
async fn mint_session(pool: &PgPool, user_id: Uuid) -> String {
    let token = format!("test-{}-{}", user_id.simple(), Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO user_sessions
            (user_id, session_token, ip_address, user_agent, remember_me, expires_at)
           VALUES ($1, $2, NULL, 'dev-e2e', FALSE, NOW() + INTERVAL '1 day')"#,
    )
    .bind(user_id)
    .bind(&token)
    .execute(pool)
    .await
    .expect("insert session");
    token
}

/// Grant the user the `developer` role.
async fn grant_role(pool: &PgPool, user_id: Uuid, role_name: &str) {
    sqlx::query(
        r#"INSERT INTO user_roles (user_id, role_id, is_active)
           SELECT $1, r.id, TRUE FROM roles r WHERE r.name = $2
           ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE"#,
    )
    .bind(user_id)
    .bind(role_name)
    .execute(pool)
    .await
    .expect("grant role");
}

/// Simulate Didit KYC completion — INSERT INTO kyc_records (status='approved').
async fn seed_kyc_approved(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO kyc_records (user_id, status, verified_at)
           VALUES ($1, 'approved', NOW())"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed kyc approved");
}

/// Insert a `developer_asset_links` row authorising the developer to submit
/// operations for the asset (Villa-Returns P2 onboarding step).
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

/// Best-effort cleanup — removes every test-owned row keyed by user_id +
/// asset_id. `developer_asset_links` is append-only at the trigger level so
/// we revoke (set `effective_until`) BEFORE the asset is dropped; the asset's
/// `ON DELETE CASCADE` then takes care of the row.
async fn cleanup(pool: &PgPool, user_ids: &[Uuid], asset_ids: &[Uuid]) {
    // Asset-side first (FKs cascade from assets back into images / docs /
    // operations / capex / change_requests / asset_links).
    for asset_id in asset_ids {
        let _ = sqlx::query(
            "UPDATE developer_asset_links SET effective_until = NOW()
             WHERE asset_id = $1 AND effective_until IS NULL",
        )
        .bind(asset_id)
        .execute(pool)
        .await;
        let _ = sqlx::query("DELETE FROM developer_projects WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM villa_operations_log WHERE asset_id = $1")
            .bind(asset_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM asset_images WHERE asset_id = $1")
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

    for user_id in user_ids {
        let _ = sqlx::query("DELETE FROM developer_applications WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM kyc_records WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM developer_projects WHERE developer_id = $1")
            .bind(user_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM audit_logs WHERE actor_user_id = $1")
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
}

// ─── HTTP helpers ───────────────────────────────────────────────────────

/// GET request with optional session cookie. CSRF cookies are not required
/// for GET — `csrf_middleware` only validates POST/PUT/DELETE/PATCH.
fn get_with_session(uri: &str, session_token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = session_token {
        builder = builder.header("cookie", format!("poool_session={}", t));
    }
    builder.body(Body::empty()).unwrap()
}

/// Mutating (POST/PUT/DELETE) request: session + matching CSRF cookie +
/// `X-CSRF-Token` header (Double-Submit-Cookie pattern). Body is JSON.
fn mutating_with_session(
    method: Method,
    uri: &str,
    session_token: &str,
    json_body: serde_json::Value,
) -> Request<Body> {
    let csrf = "dev-e2e-csrf-1234567890";
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

/// Drain a response body into a String.
async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The 11-field developer-application payload sent by `developer-onboarding.html`.
/// Matches the field names in `api_developer_apply` (see routes.rs).
fn apply_payload() -> serde_json::Value {
    serde_json::json!({
        "first_name": "Dev",
        "last_name": "Eloper",
        "phone": "+62 812 3456 7890",
        "whatsapp": "+62 812 3456 7890",
        "nationality": "Indonesian",
        "country": "ID",
        "website": "https://example.com",
        "assets_count": "1-3",
        "asset_value": "1-3M",
        "monthly_income": "10-50k",
        "bio": "10 years building Bali villas."
    })
}

/// A minimal "create draft" payload that satisfies `validate_draft_shape`.
fn create_draft_payload(title: &str) -> serde_json::Value {
    serde_json::json!({
        "title": title,
        "asset_type": "real_estate",
        "property_type": "villa",
        "area": "seminyak",
        "address": "Jl. Test 1",
        "city": "Denpasar",
        "country": "ID",
        "lease_type": "leasehold",
        "lease_term_years": 25,
        "land_size_sqm": 400.0,
        "building_size_sqm": 250.0,
        "bedrooms": 3,
        "bathrooms": 3,
        "construction_status": "ready",
        "year_built": 2023,
        "total_value_cents": 50_000_000_i64,   // $500k
        "token_price_cents": 50_000_i64,        // $500 per share
        "tokens_total": 1_000_i64               // server re-derives this
    })
}

/// A fully-populated monthly operations payload for period 2026-03. Includes
/// custom expenses ("Pool tile repair") in `expense_other_notes`.
fn operations_payload(custom_expenses: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "period_year": 2026,
        "period_month": 3,
        "currency_code": "IDR",
        "gross_rental_idr_cents": 100_000_000_i64,
        "nights_available": 31,
        "nights_booked": 25,
        "expense_cleaning_idr_cents":    2_000_000_i64,
        "expense_maintenance_idr_cents": 1_500_000_i64,
        "expense_utilities_idr_cents":   3_000_000_i64,
        "expense_staff_idr_cents":       5_000_000_i64,
        "expense_pool_garden_idr_cents": 1_000_000_i64,
        "expense_pest_idr_cents":          200_000_i64,
        "expense_other_idr_cents":       5_000_000_i64,
        "expense_property_tax_idr_cents":  500_000_i64,
        "expense_insurance_idr_cents":     500_000_i64,
        "expense_accounting_idr_cents":    300_000_i64,
        "expense_internet_idr_cents":      200_000_i64,
        "expense_capex_idr_cents": 0_i64,
        "ota_fees_idr_cents":      1_000_000_i64,
        "payment_fees_idr_cents":    500_000_i64,
        "refunds_idr_cents":               0_i64,
        "mgmt_fee_idr_cents":      5_000_000_i64,
        "expense_other_notes": custom_expenses
    })
}

/// Seed the absolute minimum for an asset to be approvable by the admin
/// submission endpoint: one `asset_images` row (the approve handler refuses
/// to publish an asset with zero images).
async fn seed_asset_image(pool: &PgPool, asset_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO asset_images (asset_id, image_url, is_cover, sort_order)
           VALUES ($1, 'https://example.com/cover.jpg', TRUE, 0)"#,
    )
    .bind(asset_id)
    .execute(pool)
    .await
    .expect("seed asset image");
}

/// Read the application_id from a 202 response body so we can step 5-6.
fn extract_application_id(body: &str) -> Uuid {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("apply response was not JSON: {} — body: {}", e, body));
    let s = parsed
        .get("application_id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("apply response missing application_id: {}", body));
    Uuid::parse_str(s).expect("application_id parses as UUID")
}

/// Read the asset_id from a 201/200 draft-create response body.
fn extract_asset_id(body: &str) -> Uuid {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("draft response was not JSON: {} — body: {}", e, body));
    let s = parsed
        .get("asset_id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("draft response missing asset_id: {}", body));
    Uuid::parse_str(s).expect("asset_id parses as UUID")
}

/// Read the operations log id from a create/update response body.
fn extract_log_id(body: &str) -> i64 {
    let parsed: serde_json::Value = serde_json::from_str(body)
        .unwrap_or_else(|e| panic!("ops response was not JSON: {} — body: {}", e, body));
    parsed
        .get("id")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| panic!("ops response missing id: {}", body))
}

// ══════════════════════════════════════════════════════════════════════
// HAPPY PATH — full apply-to-approval workflow
// ══════════════════════════════════════════════════════════════════════

/// Steps 1–15 (cleanup is the test's drop). Every step asserts the visible
/// state change so we know exactly which step regressed if the test fails.
#[ignore]
#[tokio::test]
async fn happy_path_apply_to_payout() {
    let pool = pool().await;

    // Step 1 — Sign up user + admin and mint their session cookies.
    let user_id = insert_user(&pool, "happy").await;
    let user_session = mint_session(&pool, user_id).await;
    let admin_id = insert_user(&pool, "happy-admin").await;
    grant_role(&pool, admin_id, "admin").await;
    let admin_session = mint_session(&pool, admin_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Step 2 — POST /api/developer/apply → 202 + pending row.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("apply oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::ACCEPTED, "apply body: {}", body);
    let application_id = extract_application_id(&body);

    let pending_status: String =
        sqlx::query_scalar("SELECT status FROM developer_applications WHERE id = $1")
            .bind(application_id)
            .fetch_one(&pool)
            .await
            .expect("read application status");
    assert_eq!(pending_status, "pending");

    // Step 3 — User attempts to create a draft → 403 (no developer role).
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &user_session,
            create_draft_payload("Premature Draft"),
        ))
        .await
        .expect("premature draft oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "draft before approval must return 403 (C-2 regression guard)",
    );

    // Step 4 — External KYC completes. Simulated as a direct INSERT.
    seed_kyc_approved(&pool, user_id).await;

    // Step 5 — Admin GETs the queue and sees our application.
    let resp = app
        .clone()
        .oneshot(get_with_session(
            "/api/admin/developer-applications?status=pending",
            Some(&admin_session),
        ))
        .await
        .expect("queue oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "queue body: {}", body);
    assert!(
        body.contains(&application_id.to_string()),
        "queue did not contain the application: {}",
        body,
    );

    // Step 6 — Admin approves the application. Grants role + flips status.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!(
                "/api/admin/developer-applications/{}/approve",
                application_id
            ),
            &admin_session,
            serde_json::json!({}),
        ))
        .await
        .expect("approve oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "approve body: {}", body);

    let (approved_status, kyc_snapshot): (String, Option<chrono::DateTime<chrono::Utc>>) =
        sqlx::query_as("SELECT status, kyc_verified_at FROM developer_applications WHERE id = $1")
            .bind(application_id)
            .fetch_one(&pool)
            .await
            .expect("read approved app");
    assert_eq!(approved_status, "approved");
    assert!(
        kyc_snapshot.is_some(),
        "kyc_verified_at must be snapshotted at approval time",
    );

    let has_developer_role: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1 AND r.name = 'developer' AND ur.is_active = TRUE
        )"#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("check role");
    assert!(
        has_developer_role,
        "user must have developer role post-approval"
    );

    // Step 7 — User POSTs /api/developer/draft → 200 + asset_id.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &user_session,
            create_draft_payload("E2E Happy Villa"),
        ))
        .await
        .expect("create draft oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "create draft body: {}", body);
    let asset_id = extract_asset_id(&body);

    // Step 8 — User PUTs content + financials on the draft.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!("/api/developer/draft/{}", asset_id),
            &user_session,
            serde_json::json!({
                "title": "E2E Happy Villa",
                "short_description": "Test villa.",
                "description": "Full description.",
                "annual_yield_bps": 1_000,
                "capital_appreciation_bps": 800,
                "occupancy_rate_bps": 8_500,
                "investor_share_bps": 7_000,
                "submission_step": 4,
            }),
        ))
        .await
        .expect("update draft oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "update draft body: {}", body);

    // Pre-step-9 — seed an image; the submit handler refuses zero-image drafts.
    seed_asset_image(&pool, asset_id).await;

    // Step 9 — Submit the draft for review.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/developer/draft/{}/submit", asset_id),
            &user_session,
            serde_json::json!({}),
        ))
        .await
        .expect("submit draft oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "submit draft body: {}", body);

    let project_status: String =
        sqlx::query_scalar("SELECT status FROM developer_projects WHERE asset_id = $1")
            .bind(asset_id)
            .fetch_one(&pool)
            .await
            .expect("project status");
    assert!(
        project_status == "submitted" || project_status == "in_review",
        "expected post-submit status submitted/in_review, got '{}'",
        project_status,
    );

    // Step 10 — Admin approves the asset via /api/admin/submissions/:id/approve.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/admin/submissions/{}/approve", asset_id),
            &admin_session,
            serde_json::json!({}),
        ))
        .await
        .expect("admin approve asset oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "admin approve asset body: {}", body);

    let (published, live_status): (bool, String) = sqlx::query_as(
        "SELECT a.published, COALESCE(dp.status, 'draft')
         FROM assets a LEFT JOIN developer_projects dp ON dp.asset_id = a.id
         WHERE a.id = $1",
    )
    .bind(asset_id)
    .fetch_one(&pool)
    .await
    .expect("post-approval asset state");
    assert!(published, "asset must be published post-approval");
    assert_eq!(live_status, "live");

    // Step 11 — Link developer to asset (Villa-Returns onboarding hand-off).
    link_developer_to_asset(&pool, user_id, asset_id).await;

    // Step 12 — POST monthly operations for 2026-03 with custom_expenses.
    let custom_expenses = serde_json::json!([
        {"name": "Pool tile repair", "amount_idr_cents": 5_000_000_i64}
    ]);
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/developer/villas/{}/operations", asset_id),
            &user_session,
            operations_payload(custom_expenses.clone()),
        ))
        .await
        .expect("ops create oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "ops create body: {}", body);
    let log_id = extract_log_id(&body);

    // Step 13 — Submit the operations log to lock it.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!(
                "/api/developer/villas/{}/operations/{}/submit",
                asset_id, log_id
            ),
            &user_session,
            serde_json::json!({}),
        ))
        .await
        .expect("ops submit oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "ops submit body: {}", body);

    let submitted_status: String =
        sqlx::query_scalar("SELECT status FROM villa_operations_log WHERE id = $1")
            .bind(log_id)
            .fetch_one(&pool)
            .await
            .expect("ops status");
    assert_eq!(submitted_status, "submitted");

    // Step 14 — Admin approves the operations log. Note: the admin/dev 4-eyes
    // CHECK forbids the same user approving their own submission, which we
    // satisfy: `submitted_by` is the developer, `admin_id` is a different
    // user. transition() flips status submitted → approved.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::PUT,
            &format!(
                "/api/admin/villas/{}/operations/{}/approve",
                asset_id, log_id
            ),
            &admin_session,
            serde_json::json!({}),
        ))
        .await
        .expect("admin ops approve oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "admin ops approve body: {}", body);

    let approved_status: String =
        sqlx::query_scalar("SELECT status FROM villa_operations_log WHERE id = $1")
            .bind(log_id)
            .fetch_one(&pool)
            .await
            .expect("ops approved status");
    assert_eq!(approved_status, "approved");

    // Step 15 — Round-trip the custom_expenses JSONB through GET. The DB-level
    // column is `expense_other_notes`; the wire shape is the same key.
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations/{}", asset_id, log_id),
            Some(&user_session),
        ))
        .await
        .expect("ops get oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "ops get body: {}", body);
    let parsed: serde_json::Value = serde_json::from_str(&body).expect("parse ops get json");
    assert_eq!(
        parsed.get("expense_other_notes"),
        Some(&custom_expenses),
        "custom_expenses must round-trip via expense_other_notes JSONB; body: {}",
        body,
    );

    // Step 16 — Cleanup.
    cleanup(&pool, &[user_id, admin_id], &[asset_id]).await;
}

// ══════════════════════════════════════════════════════════════════════
// SECURITY-REJECTION PATHS (one #[ignore]d test each)
// ══════════════════════════════════════════════════════════════════════

/// 1. Anonymous tries onboarding apply → 401.
#[ignore]
#[tokio::test]
async fn reject_anonymous_apply() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool.clone()));

    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            "no-such-session-token",
            apply_payload(),
        ))
        .await
        .expect("apply oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// 2. Authed user without KYC tries to be approved → admin's POST returns
///    400 + application moves to status='needs_kyc' + user stays role-less.
#[ignore]
#[tokio::test]
async fn reject_approval_without_kyc_flips_to_needs_kyc() {
    let pool = pool().await;
    let user_id = insert_user(&pool, "no-kyc").await;
    let user_session = mint_session(&pool, user_id).await;
    let admin_id = insert_user(&pool, "no-kyc-admin").await;
    grant_role(&pool, admin_id, "admin").await;
    let admin_session = mint_session(&pool, admin_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Apply (no KYC yet).
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("apply oneshot");
    let body = body_string(resp).await;
    let application_id = extract_application_id(&body);

    // Admin tries to approve. C-3 fix: 400 + needs_kyc.
    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            &format!(
                "/api/admin/developer-applications/{}/approve",
                application_id
            ),
            &admin_session,
            serde_json::json!({}),
        ))
        .await
        .expect("approve oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    let new_status: String =
        sqlx::query_scalar("SELECT status FROM developer_applications WHERE id = $1")
            .bind(application_id)
            .fetch_one(&pool)
            .await
            .expect("read status");

    let has_developer_role: bool = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM user_roles ur JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1 AND r.name = 'developer' AND ur.is_active = TRUE
        )"#,
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("role check");

    cleanup(&pool, &[user_id, admin_id], &[]).await;

    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {}", body);
    assert_eq!(new_status, "needs_kyc");
    assert!(
        !has_developer_role,
        "user must remain role-less when KYC missing"
    );
}

/// 3. Non-admin (with the `developer` role!) tries to approve → 403. Proves
///    the approve endpoint enforces admin-not-developer.
#[ignore]
#[tokio::test]
async fn reject_non_admin_approve() {
    let pool = pool().await;
    let attacker_id = insert_user(&pool, "attacker").await;
    grant_role(&pool, attacker_id, "developer").await;
    let attacker_session = mint_session(&pool, attacker_id).await;

    // Need a real application_id to point the call at — but the auth check
    // runs before the lookup, so a random UUID is fine for this test.
    let fake_app_id = Uuid::new_v4();

    let app = build_platform_router(make_state(pool.clone()));

    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/admin/developer-applications/{}/approve", fake_app_id),
            &attacker_session,
            serde_json::json!({}),
        ))
        .await
        .expect("approve oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup(&pool, &[attacker_id], &[]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

/// 4. C-2 regression: a user who has applied but has NOT been approved
///    cannot POST /api/developer/draft (must be 403).
#[ignore]
#[tokio::test]
async fn reject_draft_before_approval() {
    let pool = pool().await;
    let user_id = insert_user(&pool, "pre-approval").await;
    let user_session = mint_session(&pool, user_id).await;
    let app = build_platform_router(make_state(pool.clone()));

    // Apply but skip the admin approval step.
    let _ = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("apply oneshot");

    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &user_session,
            create_draft_payload("Premature"),
        ))
        .await
        .expect("draft oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup(&pool, &[user_id], &[]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

/// 5. A user whose application was rejected MUST NOT have the developer
///    role and so MUST NOT be able to POST a draft (403).
#[ignore]
#[tokio::test]
async fn reject_draft_after_rejection() {
    let pool = pool().await;
    let user_id = insert_user(&pool, "rejected").await;
    let user_session = mint_session(&pool, user_id).await;
    let admin_id = insert_user(&pool, "rejected-admin").await;
    grant_role(&pool, admin_id, "admin").await;
    let admin_session = mint_session(&pool, admin_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("apply oneshot");
    let body = body_string(resp).await;
    let application_id = extract_application_id(&body);

    // Admin rejects.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!(
                "/api/admin/developer-applications/{}/reject",
                application_id
            ),
            &admin_session,
            serde_json::json!({"notes": "not at this time"}),
        ))
        .await
        .expect("reject oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = body_string(resp).await;

    // User now tries to create a draft → still 403 (no role was granted).
    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &user_session,
            create_draft_payload("Should Fail"),
        ))
        .await
        .expect("draft oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup(&pool, &[user_id, admin_id], &[]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

/// 6. Developer A tries to read Developer B's draft → 403 (ownership).
///    The GET handler resolves owner_id and rejects if it doesn't match.
#[ignore]
#[tokio::test]
async fn reject_cross_developer_draft_read() {
    let pool = pool().await;
    let dev_a = insert_user(&pool, "dev-a").await;
    grant_role(&pool, dev_a, "developer").await;
    let dev_a_session = mint_session(&pool, dev_a).await;

    let dev_b = insert_user(&pool, "dev-b").await;
    grant_role(&pool, dev_b, "developer").await;
    let dev_b_session = mint_session(&pool, dev_b).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Dev A creates a draft.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &dev_a_session,
            create_draft_payload("Dev A's draft"),
        ))
        .await
        .expect("create draft oneshot");
    let body = body_string(resp).await;
    let asset_id = extract_asset_id(&body);

    // Dev B tries to read it via the asset-detail endpoint that explicitly
    // checks owner_id ≠ user.id.
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/assets/{}", asset_id),
            Some(&dev_b_session),
        ))
        .await
        .expect("cross-read oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup(&pool, &[dev_a, dev_b], &[asset_id]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

/// 7. Developer A tries to submit operations for a villa they are NOT
///    linked to → 403 (`DeveloperUser::require_asset_link`).
#[ignore]
#[tokio::test]
async fn reject_ops_create_without_asset_link() {
    let pool = pool().await;
    let dev_a = insert_user(&pool, "ops-a").await;
    grant_role(&pool, dev_a, "developer").await;
    let dev_b = insert_user(&pool, "ops-b").await;
    grant_role(&pool, dev_b, "developer").await;
    let dev_b_session = mint_session(&pool, dev_b).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Dev A creates + owns the draft.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &mint_session(&pool, dev_a).await,
            create_draft_payload("Dev A's villa"),
        ))
        .await
        .expect("create draft oneshot");
    let body = body_string(resp).await;
    let asset_id = extract_asset_id(&body);

    // Dev B tries to POST operations for it.
    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/developer/villas/{}/operations", asset_id),
            &dev_b_session,
            operations_payload(serde_json::json!([])),
        ))
        .await
        .expect("ops create oneshot");
    let status = resp.status();
    let _ = body_string(resp).await;

    cleanup(&pool, &[dev_a, dev_b], &[asset_id]).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

/// 8. Apply twice while pending. The apply handler does NOT enforce a
///    UNIQUE constraint on user_id (see migration 203) — submitting again
///    creates a fresh row. This test documents the current behaviour as a
///    regression guard, so any future move to either an idempotent UPDATE
///    or a 409 conflict will trip this assertion and require a deliberate
///    update.
#[ignore]
#[tokio::test]
async fn double_apply_documents_current_behaviour() {
    let pool = pool().await;
    let user_id = insert_user(&pool, "dbl-apply").await;
    let user_session = mint_session(&pool, user_id).await;
    let app = build_platform_router(make_state(pool.clone()));

    // First apply.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("first apply oneshot");
    let first_status = resp.status();
    let _ = body_string(resp).await;

    // Second apply.
    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("second apply oneshot");
    let second_status = resp.status();
    let _ = body_string(resp).await;

    let row_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM developer_applications WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count rows");

    cleanup(&pool, &[user_id], &[]).await;

    // Both must succeed; both must insert.
    assert_eq!(first_status, StatusCode::ACCEPTED);
    assert_eq!(second_status, StatusCode::ACCEPTED);
    assert_eq!(
        row_count, 2,
        "current behaviour is append-only; if this number changes the apply \
         handler now de-dupes — update the test + audit the new semantics",
    );
}

/// 9. Admin tries to approve an already-approved application → 409
///    Conflict (see `api_admin_approve_developer_application`, where the
///    early-return for `current_status == "approved"` lives).
#[ignore]
#[tokio::test]
async fn reject_double_approve_returns_conflict() {
    let pool = pool().await;
    let user_id = insert_user(&pool, "dbl-approve").await;
    let user_session = mint_session(&pool, user_id).await;
    let admin_id = insert_user(&pool, "dbl-approve-admin").await;
    grant_role(&pool, admin_id, "admin").await;
    let admin_session = mint_session(&pool, admin_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Apply, KYC, approve once.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/apply",
            &user_session,
            apply_payload(),
        ))
        .await
        .expect("apply oneshot");
    let body = body_string(resp).await;
    let application_id = extract_application_id(&body);

    seed_kyc_approved(&pool, user_id).await;

    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!(
                "/api/admin/developer-applications/{}/approve",
                application_id
            ),
            &admin_session,
            serde_json::json!({}),
        ))
        .await
        .expect("first approve oneshot");
    let first_status = resp.status();
    let _ = body_string(resp).await;
    assert_eq!(first_status, StatusCode::OK);

    // Approve again.
    let resp = app
        .oneshot(mutating_with_session(
            Method::POST,
            &format!(
                "/api/admin/developer-applications/{}/approve",
                application_id
            ),
            &admin_session,
            serde_json::json!({}),
        ))
        .await
        .expect("second approve oneshot");
    let second_status = resp.status();
    let _ = body_string(resp).await;

    cleanup(&pool, &[user_id, admin_id], &[]).await;

    assert_eq!(second_status, StatusCode::CONFLICT);
}

/// 10. `custom_expenses` with a `<script>` payload. The JSON boundary must
///     NOT mutate / HTML-escape the literal text — sanitisation belongs to
///     the view layer. Both the immediate response and the subsequent GET
///     must contain the literal `<script>...</script>` substring.
#[ignore]
#[tokio::test]
async fn custom_expenses_preserves_literal_script_text() {
    let pool = pool().await;
    let user_id = insert_user(&pool, "xss").await;
    grant_role(&pool, user_id, "developer").await;
    let user_session = mint_session(&pool, user_id).await;

    let app = build_platform_router(make_state(pool.clone()));

    // Create a draft owned by this user, then link them to it as an asset.
    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            "/api/developer/draft",
            &user_session,
            create_draft_payload("XSS Villa"),
        ))
        .await
        .expect("create draft oneshot");
    let body = body_string(resp).await;
    let asset_id = extract_asset_id(&body);
    link_developer_to_asset(&pool, user_id, asset_id).await;

    // Custom expenses with a literal <script> tag.
    let suspicious = "Pool <script>alert(1)</script>";
    let custom_expenses = serde_json::json!([
        {"name": suspicious, "amount_idr_cents": 100_i64}
    ]);

    let resp = app
        .clone()
        .oneshot(mutating_with_session(
            Method::POST,
            &format!("/api/developer/villas/{}/operations", asset_id),
            &user_session,
            operations_payload(custom_expenses.clone()),
        ))
        .await
        .expect("ops create oneshot");
    let status = resp.status();
    let body = body_string(resp).await;
    assert_eq!(status, StatusCode::OK, "ops create body: {}", body);
    let log_id = extract_log_id(&body);

    assert!(
        body.contains("<script>"),
        "create response must preserve literal <script> text; got {}",
        body,
    );

    // DB-level: column stores the raw JSONB with the same text.
    let db_notes: serde_json::Value =
        sqlx::query_scalar("SELECT expense_other_notes FROM villa_operations_log WHERE id = $1")
            .bind(log_id)
            .fetch_one(&pool)
            .await
            .expect("db notes");
    let db_first_name = db_notes
        .get(0)
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(
        db_first_name, suspicious,
        "DB JSONB must store literal `<script>` text (view layer is responsible for HTML escaping)",
    );

    // GET round-trip preserves the same text.
    let resp = app
        .oneshot(get_with_session(
            &format!("/api/developer/villas/{}/operations/{}", asset_id, log_id),
            Some(&user_session),
        ))
        .await
        .expect("ops get oneshot");
    let body = body_string(resp).await;
    assert!(
        body.contains("<script>"),
        "GET response must preserve literal <script> text; got {}",
        body,
    );

    cleanup(&pool, &[user_id], &[asset_id]).await;
}
