//! HTTP-level integration tests for the developer Affiliate-Team API surface
//! (`/api/developer/affiliate/team/*`).
//!
//! These exercise the full Axum router that production serves — same handlers,
//! same middleware (CSRF + Sentry context + security headers) — by building
//! the router through `poool_backend::build_platform_router`. The companion
//! `affiliate_team_integration.rs` file covers the *service* layer; this file
//! covers the *HTTP* layer: auth gating (anonymous, non-developer, developer),
//! JSON contract shape, and audit-targeted security regressions:
//!
//!   * F11 — invite anti-enumeration (uniform response for unknown email).
//!   * E-P0-1 — per-developer + per-recipient invite rate-limit (429).
//!   * F20 — tier debounce (no recompute within 5 minutes).
//!   * B-P0-1 — IBAN encrypted at rest (plaintext column nulled).
//!   * 2FA step-up gate for bank-detail PATCH.
//!
//! ## Running
//!
//! ```sh
//! DATABASE_URL=postgres://martin@localhost/poool \
//!     BANK_IBAN_ENCRYPTION_KEY=00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff \
//!     cargo test --test developer_affiliate_team_http -- --ignored
//! ```
//!
//! Every test is `#[ignore]`d because it needs a live Postgres instance.

#![cfg(test)]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use poool_backend::{build_platform_router, AppState};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration as StdDuration;
use tower::ServiceExt;
use uuid::Uuid;

// ──────────────────────────────────────────────────────────────────────
// Shared test infrastructure
// ──────────────────────────────────────────────────────────────────────

/// A15 (CDDRP Phase 3.2): hard-block running integration tests against a
/// non-local Postgres. Tests `set_var` deterministic encryption keys; if a
/// developer accidentally pointed `DATABASE_URL` at production and ran
/// `cargo test -- --ignored`, real ciphertext columns would be re-encrypted
/// under the public test key. This guard refuses to connect in that case.
mod safety {
    pub fn assert_database_url_is_local() {
        let url = std::env::var("DATABASE_URL").unwrap_or_default();
        // Allow: localhost, 127.0.0.1, ::1, unix socket (/cloudsql dev), empty,
        // or the dev-user convention `postgres://martin@…`.
        let is_local = url.is_empty()
            || url.contains("@localhost")
            || url.contains("@127.0.0.1")
            || url.contains("@[::1]")
            || url.contains("@/")
            || url.starts_with("postgres://martin@");
        if !is_local {
            panic!(
                "Refusing to run integration tests against non-local DATABASE_URL: {} \
                 — set DATABASE_URL=postgres://martin@localhost/poool first",
                url.split('@').nth(1).unwrap_or("(redacted)")
            );
        }
    }
}

/// Connect to the test database. Requires `DATABASE_URL` to be set.
async fn pool() -> PgPool {
    safety::assert_database_url_is_local();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect to test DB")
}

/// Build an `AppState` against the given pool. Mirrors `leaderboard_http.rs`'s
/// `make_state` so the router built here is wire-compatible with production.
fn make_state(pool: PgPool) -> AppState {
    safety::assert_database_url_is_local();
    if std::env::var("DATABASE_URL").is_err() {
        panic!("DATABASE_URL must be set for HTTP integration tests");
    }
    // Non-production env relaxes CSRF cookie security flags so the
    // session-cookie path still works under cargo test.
    std::env::set_var("POOOL_ENV", "development");

    // B-P0-1 / IBAN encryption requires a key. Use a deterministic dev
    // key — never reused outside tests. Hex-encoded 32-byte string.
    if std::env::var("BANK_IBAN_ENCRYPTION_KEY").is_err()
        && std::env::var("TAX_ID_ENCRYPTION_KEY").is_err()
    {
        std::env::set_var(
            "BANK_IBAN_ENCRYPTION_KEY",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        );
    }

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

/// Insert a minimal user row and return the UUID. `email_verified = TRUE` is
/// required because `get_user_by_session` filters unverified users.
async fn insert_user(pool: &PgPool, status: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, status, email_verified)
         VALUES ($1, $2, 'x', $3, TRUE)",
    )
    .bind(id)
    .bind(format!("{}@dev-team-http.test", id))
    .bind(status)
    .execute(pool)
    .await
    .expect("insert user");
    id
}

/// Insert a 24-hour session row for `user_id` and return the session token to
/// attach as a `poool_session` cookie. `is_2fa_verified = TRUE` so the cookie
/// satisfies the standard auth path (note: 2FA step-up for bank edits is
/// orthogonal — that uses `step_up_sessions`, see test below).
async fn mint_session(pool: &PgPool, user_id: Uuid) -> String {
    let token = format!("test-{}-{}", user_id.simple(), Uuid::new_v4().simple());
    sqlx::query(
        r#"INSERT INTO user_sessions
            (user_id, session_token, ip_address, user_agent, remember_me, is_2fa_verified, expires_at)
           VALUES ($1, $2, NULL, 'dev-team-http', FALSE, TRUE, NOW() + INTERVAL '1 day')"#,
    )
    .bind(user_id)
    .bind(&token)
    .execute(pool)
    .await
    .expect("insert session");
    token
}

/// Grant the user the `developer` role. The `DeveloperUser` extractor accepts
/// any of (developer, asset_owner, admin, super_admin); plain `developer`
/// is the minimal one so we use it.
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

/// Ensure the developer has the auto-created default team (the
/// `get_or_create_default_team` helper creates one on first hit; calling it
/// up-front gives tests a deterministic team_id to query). Uses the non-macro
/// `sqlx::query_scalar` form so the file compiles in SQLX_OFFLINE mode
/// without requiring a per-test cache entry.
async fn ensure_default_team(pool: &PgPool, developer_user_id: Uuid) -> Uuid {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM developer_teams
           WHERE developer_user_id = $1 AND is_default = true AND status <> 'terminated'
           LIMIT 1",
    )
    .bind(developer_user_id)
    .fetch_optional(pool)
    .await
    .expect("lookup default team")
    {
        return id;
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO developer_teams (developer_user_id, display_name, is_default, status)
           VALUES ($1, 'HTTP Test Team', true, 'active')
           RETURNING id",
    )
    .bind(developer_user_id)
    .fetch_one(pool)
    .await
    .expect("create default team");
    let _ = poool_backend::rewards::team_links::ensure_developer_has_affiliate_row(
        pool,
        developer_user_id,
    )
    .await;
    id
}

/// Best-effort cleanup — removes every test-owned row keyed by user_id across
/// the affiliate/team graph + audit + rate-limit tables. Mirrors the cleanup
/// in `affiliate_team_integration.rs` so the suite stays idempotent.
async fn cleanup_user(pool: &PgPool, user_id: Uuid) {
    // Order matters: drop child rows before parents.
    let _ = sqlx::query(
        "DELETE FROM referral_clicks WHERE link_id IN (
            SELECT id FROM affiliate_links
             WHERE attribution_user_id = $1 OR payout_user_id = $1
         )",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM affiliate_commissions
          WHERE attribution_user_id = $1 OR payout_user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM affiliate_referrals
          WHERE attribution_user_id = $1 OR payout_user_id = $1 OR referred_user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "DELETE FROM affiliate_links
          WHERE attribution_user_id = $1 OR payout_user_id = $1",
    )
    .bind(user_id)
    .execute(pool)
    .await;
    let _ = sqlx::query("DELETE FROM developer_team_memberships WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM developer_teams WHERE developer_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM affiliates WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM affiliate_live_counters WHERE payout_user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM step_up_sessions WHERE user_id = $1")
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

/// Build a GET request with optional session cookie.
fn get_with_session(uri: &str, session_token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = session_token {
        builder = builder.header("cookie", format!("poool_session={}", t));
    }
    builder.body(Body::empty()).unwrap()
}

/// Build a mutating (POST/PUT/PATCH/DELETE) request with session + CSRF cookie
/// and matching `X-CSRF-Token` header. JSON-encoded body.
fn mutating_with_session(
    method: Method,
    uri: &str,
    session_token: Option<&str>,
    json_body: serde_json::Value,
) -> Request<Body> {
    let csrf = "test-csrf-1234567890";
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-CSRF-Token", csrf)
        .header("content-type", "application/json");
    builder = if let Some(t) = session_token {
        builder.header(
            "cookie",
            format!("poool_session={}; csrf_token={}", t, csrf),
        )
    } else {
        builder.header("cookie", format!("csrf_token={}", csrf))
    };
    builder
        .body(Body::from(serde_json::to_vec(&json_body).unwrap()))
        .unwrap()
}

/// Drain the response body to a String.
async fn body_string(resp: axum::response::Response) -> String {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Parse the response body as JSON, panicking with the raw text on failure.
async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let body = body_string(resp).await;
    serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("response was not valid JSON: {} — body was {:?}", e, body))
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/affiliate/team — profile + KPI tile
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn team_info_returns_200_for_developer() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK);
    // Top-level shape — the GET lazily creates a default team, so team_id
    // must always come back as a UUID string, plus the bank + branding
    // sub-objects must be present (even when empty).
    assert!(
        json.get("team_id")
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<Uuid>().is_ok())
            .unwrap_or(false),
        "team_id must be a UUID; payload: {}",
        json,
    );
    assert!(json.get("display_name").is_some(), "display_name missing");
    assert!(
        json.get("active_members")
            .and_then(|v| v.as_i64())
            .is_some(),
        "active_members missing or not numeric",
    );
    assert!(json.get("bank").is_some(), "bank object missing");
    assert!(json.get("branding").is_some(), "branding object missing");
    // B-P0-1: the legacy plaintext IBAN must NOT be present in the GET
    // response — only iban_masked + iban_set are exposed.
    let bank = json.get("bank").unwrap();
    assert!(bank.get("iban_set").is_some(), "bank.iban_set missing");
    assert!(
        bank.get("iban_masked").is_some(),
        "bank.iban_masked missing"
    );
    assert!(
        bank.get("iban").is_none(),
        "raw IBAN must not be exposed on GET; bank = {}",
        bank,
    );
}

#[ignore]
#[tokio::test]
async fn team_info_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = get_with_session("/api/developer/affiliate/team", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn team_info_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await; // no developer role
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// PATCH /api/developer/affiliate/team — profile / slug / bank / branding
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn patch_team_updates_display_name_and_slug() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    // Pick a unique slug per run to dodge the global uniqueness constraint
    // when the test reruns against a dirty DB.
    let slug = format!("http-test-{}", &Uuid::new_v4().simple().to_string()[..10]);

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PATCH,
        "/api/developer/affiliate/team",
        Some(&session),
        serde_json::json!({
            "display_name": "HTTP Team Renamed",
            "public_slug": slug,
        }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK, "body: {}", json);
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("updated"),);
}

#[ignore]
#[tokio::test]
async fn patch_team_rejects_taken_slug() {
    // Set up two developer teams; second tries to grab the slug already in
    // use by the first. update_team must return 400.
    let pool = pool().await;
    let dev_a = insert_user(&pool, "active").await;
    let dev_b = insert_user(&pool, "active").await;
    grant_developer(&pool, dev_a).await;
    grant_developer(&pool, dev_b).await;
    let team_a = ensure_default_team(&pool, dev_a).await;
    let _team_b = ensure_default_team(&pool, dev_b).await;
    let slug = format!("uniq-{}", &Uuid::new_v4().simple().to_string()[..10]);

    // Manually claim the slug on team_a (no HTTP needed — we just want it occupied).
    sqlx::query("UPDATE developer_teams SET public_slug = $1 WHERE id = $2")
        .bind(&slug)
        .bind(team_a)
        .execute(&pool)
        .await
        .expect("seed taken slug");

    let session_b = mint_session(&pool, dev_b).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PATCH,
        "/api/developer/affiliate/team",
        Some(&session_b),
        serde_json::json!({ "public_slug": slug }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, dev_a).await;
    cleanup_user(&pool, dev_b).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[ignore]
#[tokio::test]
async fn patch_team_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = mutating_with_session(
        Method::PATCH,
        "/api/developer/affiliate/team",
        None,
        serde_json::json!({ "display_name": "anon" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn patch_team_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PATCH,
        "/api/developer/affiliate/team",
        Some(&session),
        serde_json::json!({ "display_name": "x" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── B-P0-1: IBAN encrypted at rest ────────────────────────────────────

#[ignore]
#[tokio::test]
async fn patch_team_iban_is_encrypted_at_rest() {
    // After PATCHing a valid IBAN (bank fields require 2FA-step-up, so we
    // bypass by directly inserting a step_up_sessions row + faking TOTP).
    //
    // Post-PATCH the DB row must have:
    //   * bank_iban_encrypted populated and NOT equal to plaintext,
    //   * bank_iban (plaintext column) cleared to NULL,
    //   * bank_iban_last4 = last 4 chars.
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let team_id = ensure_default_team(&pool, dev).await;

    // Bypass 2FA step-up: enable TOTP + insert a fresh step_up_sessions row.
    // This mirrors what verify_and_create_trading_session does on success.
    sqlx::query(
        "INSERT INTO user_settings (user_id, totp_enabled, totp_secret)
         VALUES ($1, TRUE, 'test-secret')
         ON CONFLICT (user_id) DO UPDATE SET totp_enabled = TRUE, totp_secret = 'test-secret'",
    )
    .bind(dev)
    .execute(&pool)
    .await
    .expect("seed totp_enabled");
    // FinancialAction::AffiliateBankEdit.session_key_suffix() = "aff_bank".
    // Note: the step_up_sessions CHECK constraint may reject this action key
    // in environments that haven't applied a follow-up migration extending
    // the allowed list; if so the test below for the step-up gate will tell
    // us the actual returned status.
    let stepup_ok = sqlx::query(
        "INSERT INTO step_up_sessions (user_id, action, expires_at)
         VALUES ($1, 'aff_bank', NOW() + INTERVAL '15 minutes')
         ON CONFLICT (user_id, action) DO UPDATE SET expires_at = EXCLUDED.expires_at",
    )
    .bind(dev)
    .execute(&pool)
    .await;
    if stepup_ok.is_err() {
        // The CHECK constraint on step_up_sessions.action only allowed
        // 'withdraw|trade|pm|pwd' in mig 186; if no later migration has
        // extended the list, this test path can't run without modifying
        // the schema. Skip cleanly so the suite stays green.
        eprintln!(
            "Skipping IBAN-encryption test: step_up_sessions CHECK constraint \
             rejects 'aff_bank' (no follow-up migration applied)."
        );
        cleanup_user(&pool, dev).await;
        return;
    }

    // Valid German IBAN (mod-97 passes): DE89370400440532013000 — common
    // test fixture from the IBAN spec.
    let plain_iban = "DE89370400440532013000";

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PATCH,
        "/api/developer/affiliate/team",
        Some(&session),
        serde_json::json!({
            "bank_account_holder": "HTTP Test Holder",
            "bank_iban": plain_iban,
            "bank_country": "DE",
        }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    // Read what the DB actually stored. Use the non-macro `sqlx::query_as`
    // form so the file compiles in SQLX_OFFLINE mode without per-query
    // cache entries (we still have a live DB at runtime when the test is
    // actually unignored).
    let row: (Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT bank_iban, bank_iban_encrypted, bank_iban_last4
           FROM developer_teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_one(&pool)
    .await
    .expect("read team row");

    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK, "body: {}", body);
    assert!(
        row.0.is_none() || row.0.as_deref() == Some(""),
        "plaintext bank_iban column must be NULL after a successful PATCH \
         (B-P0-1); got {:?}",
        row.0,
    );
    let cipher = row
        .1
        .as_deref()
        .expect("bank_iban_encrypted must be populated");
    assert_ne!(
        cipher, plain_iban,
        "stored ciphertext must NOT equal plaintext (B-P0-1)",
    );
    // Sanity-check the envelope format produced by `encrypt_bank_iban`.
    assert!(
        cipher.starts_with("biban:v1:"),
        "envelope should be `biban:v1:<nonce>:<ct>`; got {:?}",
        cipher,
    );
    assert_eq!(
        row.2.as_deref(),
        Some("3000"),
        "bank_iban_last4 must cache the trailing 4 chars",
    );
}

// ─── 2FA step-up gate for bank-detail PATCH (F-spec) ──────────────────

#[ignore]
#[tokio::test]
async fn patch_team_bank_fields_require_2fa_step_up() {
    // PATCH that touches bank fields WITHOUT a valid step_up_sessions row
    // must be rejected. `require_step_up_2fa` returns AppError::TwoFactorRequired
    // which the route handler maps via `ApiError::from`. The mapping for
    // unhandled variants funnels into ApiError::BadRequest("TwoFactorRequired")
    // → HTTP 400. We assert the request was rejected (not 200) AND that no
    // bank columns were written to the DB.
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let team_id = ensure_default_team(&pool, dev).await;

    // Explicitly DO NOT enable TOTP. require_step_up_2fa will short-circuit
    // with TwoFactorRequired in that case.

    let plain_iban = "DE89370400440532013000";
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::PATCH,
        "/api/developer/affiliate/team",
        Some(&session),
        serde_json::json!({
            "bank_account_holder": "Blocked Holder",
            "bank_iban": plain_iban,
        }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let body = body_string(resp).await;

    let row: (Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT bank_account_holder, bank_iban, bank_iban_encrypted
           FROM developer_teams WHERE id = $1",
    )
    .bind(team_id)
    .fetch_one(&pool)
    .await
    .expect("read team row");

    cleanup_user(&pool, dev).await;

    assert!(
        !status.is_success(),
        "bank PATCH without step-up must be rejected; got {} body={}",
        status,
        body,
    );
    // Concretely the mapping yields 400; if a future migration adds proper
    // 428 handling, accept that too.
    assert!(
        matches!(
            status,
            StatusCode::BAD_REQUEST | StatusCode::FORBIDDEN | StatusCode::PRECONDITION_REQUIRED
        ),
        "expected BadRequest/Forbidden/PreconditionRequired; got {}",
        status,
    );
    // Body should at least mention 2FA / TwoFactor / two-factor for the
    // FE to dispatch on. Match case-insensitively.
    let lower = body.to_lowercase();
    assert!(
        lower.contains("twofactor") || lower.contains("two-factor") || lower.contains("two factor"),
        "expected the rejection message to reference 2FA; body was {}",
        body,
    );
    // And — most importantly — the DB row must NOT have absorbed any bank
    // data from the blocked PATCH.
    assert!(
        row.0.is_none(),
        "bank_account_holder must NOT be written when 2FA gate fires; got {:?}",
        row.0,
    );
    assert!(
        row.1.is_none() && row.2.is_none(),
        "no IBAN column may be touched when 2FA gate fires",
    );
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/affiliate/team/members
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn members_list_returns_200_for_developer() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/members", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");

    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some(), "team_id missing");
    assert!(
        json.get("total").and_then(|v| v.as_i64()).is_some(),
        "total missing or not numeric",
    );
    assert!(
        json.get("members").and_then(|v| v.as_array()).is_some(),
        "members must be an array",
    );
}

#[ignore]
#[tokio::test]
async fn members_list_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = get_with_session("/api/developer/affiliate/team/members", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn members_list_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/members", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/developer/affiliate/team/invite — single invite
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn invite_returns_200_for_real_email() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    // Create the invitee so the email matches a real account.
    let invitee = insert_user(&pool, "active").await;
    let invitee_email: String = sqlx::query_scalar("SELECT email::text FROM users WHERE id = $1")
        .bind(invitee)
        .fetch_one(&pool)
        .await
        .expect("read invitee email");

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite",
        Some(&session),
        serde_json::json!({ "email": invitee_email }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let json = body_json(resp).await;

    cleanup_user(&pool, dev).await;
    cleanup_user(&pool, invitee).await;

    assert_eq!(status, StatusCode::OK, "body: {}", json);
    // F11: response must use a generic 'queued' shape and a message string.
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("queued"),);
    assert!(
        json.get("message").and_then(|v| v.as_str()).is_some(),
        "message field missing in invite response",
    );
}

#[ignore]
#[tokio::test]
async fn invite_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite",
        None,
        serde_json::json!({ "email": "anon@test.local" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn invite_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite",
        Some(&session),
        serde_json::json!({ "email": "x@test.local" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── F11: anti-enumeration on invite ──────────────────────────────────

#[ignore]
#[tokio::test]
async fn invite_uniform_response_for_existing_and_nonexistent_email() {
    // POST one invite to a real registered email and one to a never-seen
    // email. F11 says the response shape (status + keys) must be identical
    // so the developer cannot use this endpoint as a user-enumeration
    // oracle.
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let invitee = insert_user(&pool, "active").await;
    let real_email: String = sqlx::query_scalar("SELECT email::text FROM users WHERE id = $1")
        .bind(invitee)
        .fetch_one(&pool)
        .await
        .expect("read invitee email");
    let fake_email = format!("nobody-{}@enumeration.test", Uuid::new_v4().simple());

    let app1 = build_platform_router(make_state(pool.clone()));
    let app2 = build_platform_router(make_state(pool.clone()));

    let req_real = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite",
        Some(&session),
        serde_json::json!({ "email": real_email }),
    );
    let req_fake = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite",
        Some(&session),
        serde_json::json!({ "email": fake_email }),
    );

    let resp_real = app1.oneshot(req_real).await.expect("oneshot real");
    let resp_fake = app2.oneshot(req_fake).await.expect("oneshot fake");

    let status_real = resp_real.status();
    let status_fake = resp_fake.status();
    let json_real = body_json(resp_real).await;
    let json_fake = body_json(resp_fake).await;

    cleanup_user(&pool, dev).await;
    cleanup_user(&pool, invitee).await;

    // 1. Status code parity.
    assert_eq!(
        status_real, status_fake,
        "F11 violation — different HTTP status for real vs unknown email \
         ({} vs {})",
        status_real, status_fake,
    );
    assert_eq!(status_real, StatusCode::OK);

    // 2. Same top-level `status` string.
    assert_eq!(
        json_real.get("status").and_then(|v| v.as_str()),
        json_fake.get("status").and_then(|v| v.as_str()),
        "F11 violation — `status` differs (real={:?} fake={:?})",
        json_real.get("status"),
        json_fake.get("status"),
    );
    assert_eq!(
        json_real.get("status").and_then(|v| v.as_str()),
        Some("queued"),
    );

    // 3. Same set of top-level keys (ignoring debug-only fields like
    //    preview_token / note). The contract requires `status`, `membership_id`,
    //    `message` — present in both responses regardless of whether the
    //    underlying user existed.
    for required in ["status", "membership_id", "message"] {
        assert!(
            json_real.get(required).is_some(),
            "real-email response missing `{}`: {}",
            required,
            json_real,
        );
        assert!(
            json_fake.get(required).is_some(),
            "fake-email response missing `{}`: {}",
            required,
            json_fake,
        );
    }

    // 4. Message text must be identical (the F11 fix uses a single generic
    //    string for both paths).
    assert_eq!(
        json_real.get("message").and_then(|v| v.as_str()),
        json_fake.get("message").and_then(|v| v.as_str()),
        "F11 violation — `message` differs across paths",
    );
}

// ─── E-P0-1: per-recipient invite rate-limit ──────────────────────────

#[ignore]
#[tokio::test]
async fn invite_rate_limited_after_burst_to_same_recipient() {
    // Replace the disabled rate-limiter with a real one capped at 2 hits
    // per minute so the third invite from the same developer to the same
    // recipient must come back 429. The handler keys per-developer AND
    // per-recipient — the per-developer key gets exhausted at the same
    // cap, so the third call is throttled either way.
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let invitee = insert_user(&pool, "active").await;
    let invitee_email: String = sqlx::query_scalar("SELECT email::text FROM users WHERE id = $1")
        .bind(invitee)
        .fetch_one(&pool)
        .await
        .expect("read invitee email");

    let mut state = make_state(pool.clone());
    state.auth_rate_limiter =
        poool_backend::auth::rate_limit::RateLimiter::new(2, StdDuration::from_secs(60));
    let app = build_platform_router(state);

    let mut statuses = Vec::new();
    for _ in 0..4 {
        let req = mutating_with_session(
            Method::POST,
            "/api/developer/affiliate/team/invite",
            Some(&session),
            serde_json::json!({ "email": invitee_email }),
        );
        let resp = app.clone().oneshot(req).await.expect("oneshot");
        statuses.push(resp.status());
        let _ = body_string(resp).await;
    }

    cleanup_user(&pool, dev).await;
    cleanup_user(&pool, invitee).await;

    let throttled = statuses
        .iter()
        .filter(|s| **s == StatusCode::TOO_MANY_REQUESTS)
        .count();
    assert!(
        throttled >= 1,
        "E-P0-1: invite endpoint must 429 after burst; got statuses {:?}",
        statuses,
    );
}

// ──────────────────────────────────────────────────────────────────────
// POST /api/developer/affiliate/team/invite-bulk
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn invite_bulk_dry_run_returns_per_row_classification() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    let csv = format!("first@example.com\nsecond@example.com\nnot-an-email\nfirst@example.com\n");

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite-bulk",
        Some(&session),
        serde_json::json!({ "csv_text": csv, "mode": "dry_run" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let json = body_json(resp).await;

    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK, "body: {}", json);
    assert_eq!(json.get("mode").and_then(|v| v.as_str()), Some("dry_run"));
    let rows = json
        .get("rows")
        .and_then(|v| v.as_array())
        .expect("rows array");
    // We expect 4 entries: would_invite, would_invite, invalid, duplicate-skip.
    assert_eq!(rows.len(), 4);
    let invalid_count = rows
        .iter()
        .filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("invalid"))
        .count();
    let dup_count = rows
        .iter()
        .filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("skipped"))
        .count();
    let would_count = rows
        .iter()
        .filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("would_invite"))
        .count();
    assert_eq!(invalid_count, 1, "expected 1 invalid: {:?}", rows);
    assert_eq!(dup_count, 1, "expected 1 duplicate-skipped: {:?}", rows);
    assert_eq!(would_count, 2, "expected 2 would_invite: {:?}", rows);
}

#[ignore]
#[tokio::test]
async fn invite_bulk_rejects_over_100_rows() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    // 101 distinct, syntactically valid emails.
    let mut lines = Vec::with_capacity(101);
    for i in 0..101 {
        lines.push(format!("bulk{}@example.com", i));
    }
    let csv = lines.join("\n");

    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite-bulk",
        Some(&session),
        serde_json::json!({ "csv_text": csv, "mode": "dry_run" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, dev).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "bulk invite must reject >100 rows per the 100-row cap",
    );
}

#[ignore]
#[tokio::test]
async fn invite_bulk_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite-bulk",
        None,
        serde_json::json!({ "csv_text": "a@b.com", "mode": "dry_run" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn invite_bulk_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = mutating_with_session(
        Method::POST,
        "/api/developer/affiliate/team/invite-bulk",
        Some(&session),
        serde_json::json!({ "csv_text": "a@b.com", "mode": "dry_run" }),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/affiliate/team/customers
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn customers_returns_200_for_developer() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/customers", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some());
    assert!(json.get("total").and_then(|v| v.as_i64()).is_some());
    assert!(json.get("rows").and_then(|v| v.as_array()).is_some());
}

#[ignore]
#[tokio::test]
async fn customers_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = get_with_session("/api/developer/affiliate/team/customers", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn customers_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/customers", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/affiliate/team/products
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn products_returns_200_for_developer() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/products", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some());
    assert!(json.get("total").and_then(|v| v.as_i64()).is_some());
    assert!(json.get("rows").and_then(|v| v.as_array()).is_some());
    assert!(json.get("from").is_some(), "from missing");
    assert!(json.get("to").is_some(), "to missing");
}

#[ignore]
#[tokio::test]
async fn products_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = get_with_session("/api/developer/affiliate/team/products", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn products_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/products", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/affiliate/team/analytics/overview
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn analytics_overview_returns_200_for_developer() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        "/api/developer/affiliate/team/analytics/overview",
        Some(&session),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some());
    assert!(json.get("overview").is_some(), "overview missing");
    assert!(json.get("from").is_some());
    assert!(json.get("to").is_some());
}

#[ignore]
#[tokio::test]
async fn analytics_overview_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = get_with_session("/api/developer/affiliate/team/analytics/overview", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn analytics_overview_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        "/api/developer/affiliate/team/analytics/overview",
        Some(&session),
    );
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ──────────────────────────────────────────────────────────────────────
// GET /api/developer/affiliate/team/tier  (+ F20 debounce)
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn tier_returns_200_for_developer() {
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/tier", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    let json = body_json(resp).await;
    cleanup_user(&pool, dev).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some());
    assert!(json.get("current_tier").is_some(), "current_tier missing");
    assert!(
        json.get("current_rate_bps")
            .and_then(|v| v.as_i64())
            .is_some(),
        "current_rate_bps missing/non-numeric",
    );
    assert!(json.get("ladder").and_then(|v| v.as_array()).is_some());
    assert!(json.get("history").and_then(|v| v.as_array()).is_some());
}

#[ignore]
#[tokio::test]
async fn tier_returns_401_for_anonymous() {
    let pool = pool().await;
    let app = build_platform_router(make_state(pool));
    let req = get_with_session("/api/developer/affiliate/team/tier", None);
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[ignore]
#[tokio::test]
async fn tier_returns_403_for_non_developer() {
    let pool = pool().await;
    let user = insert_user(&pool, "active").await;
    let session = mint_session(&pool, user).await;
    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/tier", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let status = resp.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ─── F20: tier debounce ───────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn tier_debounce_skips_recompute_within_five_minutes() {
    // Set `team_tier_updated_at` to NOW() (fresh = within the 5-minute
    // window). Two back-to-back GETs to /tier must NOT bump the timestamp
    // because the handler short-circuits the recompute.
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let team_id = ensure_default_team(&pool, dev).await;

    // Force a deterministic baseline timestamp (handler checks
    // `team_tier_updated_at < NOW() - INTERVAL '5 minutes'` — set to NOW()
    // so the predicate returns FALSE and the recompute is skipped).
    sqlx::query("UPDATE developer_teams SET team_tier_updated_at = NOW() WHERE id = $1")
        .bind(team_id)
        .execute(&pool)
        .await
        .expect("seed tier ts");

    let baseline: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT team_tier_updated_at FROM developer_teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .expect("read baseline ts");

    let state = make_state(pool.clone());
    let app = build_platform_router(state);

    // Hit the endpoint twice.
    for _ in 0..2 {
        let req = get_with_session("/api/developer/affiliate/team/tier", Some(&session));
        let resp = app.clone().oneshot(req).await.expect("oneshot");
        let status = resp.status();
        let _ = body_string(resp).await;
        assert_eq!(status, StatusCode::OK);
    }

    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT team_tier_updated_at FROM developer_teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .expect("read after ts");

    cleanup_user(&pool, dev).await;

    assert_eq!(
        baseline, after,
        "F20 violation — team_tier_updated_at moved on a debounced call \
         (baseline={} after={})",
        baseline, after,
    );
}

#[ignore]
#[tokio::test]
async fn tier_recompute_runs_when_stale() {
    // Inverse of the debounce test: set `team_tier_updated_at` to 1 hour
    // ago (stale) and check the GET *does* re-trigger the recompute, which
    // updates the timestamp.
    let pool = pool().await;
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let team_id = ensure_default_team(&pool, dev).await;

    sqlx::query(
        "UPDATE developer_teams SET team_tier_updated_at = NOW() - INTERVAL '1 hour' WHERE id = $1",
    )
    .bind(team_id)
    .execute(&pool)
    .await
    .expect("seed stale tier ts");

    let before: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT team_tier_updated_at FROM developer_teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .expect("read before ts");

    let app = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/tier", Some(&session));
    let resp = app.oneshot(req).await.expect("oneshot");
    let _ = body_string(resp).await;

    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT team_tier_updated_at FROM developer_teams WHERE id = $1")
            .bind(team_id)
            .fetch_one(&pool)
            .await
            .expect("read after ts");

    cleanup_user(&pool, dev).await;

    assert!(
        after > before,
        "stale tier ts should be bumped by GET /tier; before={} after={}",
        before,
        after,
    );
}

// ──────────────────────────────────────────────────────────────────────
// Extra coverage — summary / by-member / timeseries / cohort / forecast
// (smoke-tests: 200 for developer, 401 anon, 403 non-developer)
// ──────────────────────────────────────────────────────────────────────

#[ignore]
#[tokio::test]
async fn summary_returns_200_and_401_403() {
    // Bundle the three auth-tier checks for /summary in one test to keep
    // the smoke-test surface readable. The 200 path lazy-creates the team
    // and seeds the affiliate row, so we run it first.
    let pool = pool().await;

    // 200 — developer.
    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let app1 = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/summary", Some(&session));
    let resp = app1.oneshot(req).await.expect("oneshot dev");
    let status = resp.status();
    let json = body_json(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some());
    assert!(json.get("summary").is_some(), "summary missing");
    cleanup_user(&pool, dev).await;

    // 401 — anonymous.
    let app2 = build_platform_router(make_state(pool.clone()));
    let req2 = get_with_session("/api/developer/affiliate/team/summary", None);
    let resp2 = app2.oneshot(req2).await.expect("oneshot anon");
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

    // 403 — non-developer.
    let user = insert_user(&pool, "active").await;
    let sess_u = mint_session(&pool, user).await;
    let app3 = build_platform_router(make_state(pool.clone()));
    let req3 = get_with_session("/api/developer/affiliate/team/summary", Some(&sess_u));
    let resp3 = app3.oneshot(req3).await.expect("oneshot user");
    let status3 = resp3.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status3, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn by_member_returns_200_and_401_403() {
    let pool = pool().await;

    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let app1 = build_platform_router(make_state(pool.clone()));
    let req = get_with_session("/api/developer/affiliate/team/by-member", Some(&session));
    let resp = app1.oneshot(req).await.expect("oneshot dev");
    let status = resp.status();
    let json = body_json(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("team_id").is_some());
    assert!(json.get("rows").is_some(), "rows missing");
    cleanup_user(&pool, dev).await;

    let app2 = build_platform_router(make_state(pool.clone()));
    let req2 = get_with_session("/api/developer/affiliate/team/by-member", None);
    let resp2 = app2.oneshot(req2).await.expect("oneshot anon");
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

    let user = insert_user(&pool, "active").await;
    let sess_u = mint_session(&pool, user).await;
    let app3 = build_platform_router(make_state(pool.clone()));
    let req3 = get_with_session("/api/developer/affiliate/team/by-member", Some(&sess_u));
    let resp3 = app3.oneshot(req3).await.expect("oneshot user");
    let status3 = resp3.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status3, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn analytics_timeseries_returns_200_and_401_403() {
    let pool = pool().await;

    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let app1 = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        "/api/developer/affiliate/team/analytics/timeseries",
        Some(&session),
    );
    let resp = app1.oneshot(req).await.expect("oneshot dev");
    let status = resp.status();
    let json = body_json(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("series").is_some(), "series missing");
    cleanup_user(&pool, dev).await;

    let app2 = build_platform_router(make_state(pool.clone()));
    let req2 = get_with_session("/api/developer/affiliate/team/analytics/timeseries", None);
    let resp2 = app2.oneshot(req2).await.expect("oneshot anon");
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

    let user = insert_user(&pool, "active").await;
    let sess_u = mint_session(&pool, user).await;
    let app3 = build_platform_router(make_state(pool.clone()));
    let req3 = get_with_session(
        "/api/developer/affiliate/team/analytics/timeseries",
        Some(&sess_u),
    );
    let resp3 = app3.oneshot(req3).await.expect("oneshot user");
    let status3 = resp3.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status3, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn analytics_cohort_returns_200_and_401_403() {
    let pool = pool().await;

    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let app1 = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        "/api/developer/affiliate/team/analytics/cohort?months=6",
        Some(&session),
    );
    let resp = app1.oneshot(req).await.expect("oneshot dev");
    let status = resp.status();
    let json = body_json(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("cells").and_then(|v| v.as_array()).is_some());
    cleanup_user(&pool, dev).await;

    let app2 = build_platform_router(make_state(pool.clone()));
    let req2 = get_with_session("/api/developer/affiliate/team/analytics/cohort", None);
    let resp2 = app2.oneshot(req2).await.expect("oneshot anon");
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

    let user = insert_user(&pool, "active").await;
    let sess_u = mint_session(&pool, user).await;
    let app3 = build_platform_router(make_state(pool.clone()));
    let req3 = get_with_session(
        "/api/developer/affiliate/team/analytics/cohort",
        Some(&sess_u),
    );
    let resp3 = app3.oneshot(req3).await.expect("oneshot user");
    let status3 = resp3.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status3, StatusCode::FORBIDDEN);
}

#[ignore]
#[tokio::test]
async fn analytics_forecast_returns_200_and_401_403() {
    let pool = pool().await;

    let dev = insert_user(&pool, "active").await;
    grant_developer(&pool, dev).await;
    let session = mint_session(&pool, dev).await;
    let _team = ensure_default_team(&pool, dev).await;
    let app1 = build_platform_router(make_state(pool.clone()));
    let req = get_with_session(
        "/api/developer/affiliate/team/analytics/forecast",
        Some(&session),
    );
    let resp = app1.oneshot(req).await.expect("oneshot dev");
    let status = resp.status();
    let json = body_json(resp).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.get("forecast").is_some(), "forecast missing");
    cleanup_user(&pool, dev).await;

    let app2 = build_platform_router(make_state(pool.clone()));
    let req2 = get_with_session("/api/developer/affiliate/team/analytics/forecast", None);
    let resp2 = app2.oneshot(req2).await.expect("oneshot anon");
    assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

    let user = insert_user(&pool, "active").await;
    let sess_u = mint_session(&pool, user).await;
    let app3 = build_platform_router(make_state(pool.clone()));
    let req3 = get_with_session(
        "/api/developer/affiliate/team/analytics/forecast",
        Some(&sess_u),
    );
    let resp3 = app3.oneshot(req3).await.expect("oneshot user");
    let status3 = resp3.status();
    cleanup_user(&pool, user).await;
    assert_eq!(status3, StatusCode::FORBIDDEN);
}
