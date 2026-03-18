#![recursion_limit = "256"]
#![allow(clippy::type_complexity)]
#![deny(missing_docs)]

/*!
POOOL Backend  Main entry point.

This file sets up the Tokio runtime, initializes configuration, structured logging,
the database connection pool, and the Axum HTTP router.

The server serves both the static frontend (HTML/CSS/JS) and a JSON API.
Routes are conceptually grouped into:
1. Public endpoints (Auth, Static assets)
2. Protected HTML Views (Dashboard, Settings, Developer pages, Admin pages)
3. Internal API Endpoints (Cart, Checkout, User Settings, Admin Stats)
*/

/// Admin module containing handlers for platform-wide oversight, user management, and financial auditing.
pub mod admin;
#[allow(missing_docs)]
pub mod assets;
mod auth;
/// Blog module containing handlers for blog articles, authors, and categories.
mod blog;
mod cart;
mod common;
mod config;
mod db;
mod developer;
mod email;
mod error;
mod kyc;
mod leaderboard;
/// Legal module containing handlers for terms of service, privacy policy, and other legal documents.
pub mod legal;
mod payment_methods;
mod payments;
mod portfolio;
mod rewards;
mod settings;
mod storage;
#[allow(missing_docs)]
pub mod support; // Added support module
mod templates;
mod wallet;

use auth::routes::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json, Redirect},
    routing::{get, post},
    Router, ServiceExt,
};
use axum_extra::extract::cookie::CookieJar;

use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── 1. Configuration ─────────────────────────────────────────
    let config = config::Config::from_env();

    // ── 2. Sentry Initialisation (must happen before tracing setup) ─
    // The guard must be kept alive for the whole process lifetime.
    // If SENTRY_DSN is absent (local dev / CI), Sentry is a no-op.
    let _sentry_guard = config.sentry_dsn.as_ref().map(|dsn| {
        let env_name = config.app_env.clone();
        let sample_rate = if env_name == "production" { 0.2 } else { 1.0 };

        sentry::init((
            dsn.as_str(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                environment: Some(env_name.into()),
                traces_sample_rate: sample_rate,
                send_default_pii: false,
                attach_stacktrace: true,
                server_name: std::env::var("K_REVISION").ok().map(|s| s.into()),
                ..Default::default()
            },
        ))
    });
    if _sentry_guard.is_some() {
        tracing::info!("Sentry initialised");
    }

    // ── 3. Tracing / Logging Setup ───────────────────────────────────
    // sentry_tracing integrates so that every `tracing::error!` call
    // is automatically forwarded to Sentry as a breadcrumb / event.

    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            ),
        )
        .with(sentry_tracing::layer())
        .init();

    tracing::info!("Starting POOOL Backend...");

    //  2. Database & State Initialization
    let pool = db::create_pool(&config.database_url).await;

    // ── 4. Run database migrations ────────────────────────────────
    // Reads .sql files from ../database/ in alphanumeric order, tracks which
    // have been applied in a `_schema_migrations` table. Idempotent.
    run_migrations(&pool).await;

    let templates = templates::create_engine();

    let redis_pool = if let Some(url) = &config.redis_url {
        use deadpool_redis::{Config, Runtime};
        let cfg = Config::from_url(url);
        match cfg.create_pool(Some(Runtime::Tokio1)) {
            Ok(p) => {
                tracing::info!("Redis pool initialized at {}", url);
                Some(p)
            }
            Err(e) => {
                tracing::error!("Failed to create Redis pool: {}", e);
                None
            }
        }
    } else {
        None
    };

    let auth_rate_limiter = if let Some(ref rp) = redis_pool {
        tracing::info!("Rate limiter: using Redis backend (shared across instances)");
        auth::rate_limit::RateLimiter::new_redis(
            rp.clone(),
            10,
            std::time::Duration::from_secs(15 * 60),
        )
    } else {
        tracing::info!("Rate limiter: using in-memory backend (single instance only)");
        auth::rate_limit::RateLimiter::new(10, std::time::Duration::from_secs(15 * 60))
    };

    let state = AppState {
        db: pool.clone(),
        templates,
        config: config.clone(),
        redis: redis_pool,
        auth_rate_limiter: auth_rate_limiter.clone(),
    };

    // Spawn background tasks
    tokio::spawn(email::run_email_scheduler(pool.clone()));
    tokio::spawn(support::sla::monitor_sla_breaches(pool.clone()));

    // Rate limiter cleanup (every 10 minutes)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10 * 60));
        loop {
            interval.tick().await;
            auth_rate_limiter.cleanup().await;
        }
    });

    // Payments: Token reclaim worker
    let cleanup_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60)); // 15 mins
        loop {
            interval.tick().await;
            if let Err(e) = crate::payments::service::cleanup_expired_orders(&cleanup_pool).await {
                tracing::error!("Error cleaning up expired orders: {}", e);
            }
        }
    });

    // Housekeeping: Purge expired sessions and used password reset tokens
    let housekeeping_pool = pool.clone();
    tokio::spawn(async move {
        // Run every 6 hours
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
        loop {
            interval.tick().await;
            // 1. Delete expired sessions
            match sqlx::query("DELETE FROM user_sessions WHERE expires_at < NOW()")
                .execute(&housekeeping_pool)
                .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        tracing::info!(
                            "Housekeeping: purged {} expired sessions",
                            res.rows_affected()
                        );
                    }
                }
                Err(e) => tracing::error!("Housekeeping: failed to purge sessions: {}", e),
            }

            // 2. Delete used or expired password reset tokens (older than 24h)
            match sqlx::query(
                "DELETE FROM password_reset_tokens WHERE used_at IS NOT NULL OR expires_at < NOW() - interval '24 hours'"
            )
            .execute(&housekeeping_pool)
            .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        tracing::info!(
                            "Housekeeping: purged {} spent/expired reset tokens",
                            res.rows_affected()
                        );
                    }
                }
                Err(e) => tracing::error!("Housekeeping: failed to purge reset tokens: {}", e),
            }

            // 3. Delete expired email verification tokens (older than 48h)
            match sqlx::query(
                "DELETE FROM email_verification_tokens WHERE expires_at < NOW() - interval '48 hours'"
            )
            .execute(&housekeeping_pool)
            .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        tracing::info!(
                            "Housekeeping: purged {} expired email verification tokens",
                            res.rows_affected()
                        );
                    }
                }
                Err(e) => {
                    // Table might not exist yet — suppress error
                    if !e.to_string().contains("does not exist") {
                        tracing::error!("Housekeeping: failed to purge email tokens: {}", e);
                    }
                }
            }
        }
    });

    //  3. Router configuration
    //
    // Two routers for host-based routing:
    //   • www_router   → Landing page (www.poool.app)
    //   • platform_router → Dashboard / API (platform.poool.app)
    //   • poool.app (bare domain) → 301 redirect to www.poool.app
    //   • localhost → serves platform (dev default)

    // ── WWW Landing Page Router ───────────────────────────────────────
    let www_router = Router::new()
        .route("/health", get(handle_health))
        // Language-specific landing pages
        .nest_service("/id", ServeDir::new("../frontend/www/id"))
        // Shared assets referenced by the Angular SPA
        .nest_service("/fonts", ServeDir::new("../frontend/www/fonts"))
        .nest_service("/png", ServeDir::new("../frontend/www/png"))
        .nest_service("/svg", ServeDir::new("../frontend/www/svg"))
        .nest_service("/webp", ServeDir::new("../frontend/www/webp"))
        .nest_service("/webm", ServeDir::new("../frontend/www/webm"))
        .route_service("/robots.txt", ServeFile::new("../frontend/www/robots.txt"))
        .route_service(
            "/sitemap.xml",
            ServeFile::new("../frontend/www/sitemap.xml"),
        )
        // All other paths → serve from /en/ (Angular SPA root)
        // This handles /, /chunk-*.js, /styles-*.css, /main-*.js, etc.
        .fallback_service(
            ServeDir::new("../frontend/www/en")
                .fallback(ServeFile::new("../frontend/www/en/index.html")),
        )
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(axum::middleware::from_fn(apply_security_headers));

    // ── Platform Router (login, dashboard, API) ──────────────────────
    let platform_router = Router::new()
        // ── Authentication ─────────────────────────────────────────────
        .nest("/auth", auth::routes::router(state.clone()))
        .route("/logout", get(auth::routes::logout))
        // ── Domain Routers (each module owns its routes) ────────────────
        .merge(assets::router())
        .merge(kyc::router())
        .merge(rewards::router())
        .merge(leaderboard::router())
        .merge(settings::router())
        .merge(storage::router())
        .merge(wallet::router())
        .merge(cart::router())
        .merge(payments::router())
        .merge(portfolio::router())
        .merge(payment_methods::router())
        .merge(developer::router())
        .merge(legal::router())
        .merge(admin::router())
        .merge(blog::router())
        // ── Support (merged router handles /support and /api/support) ──
        .merge(support::router(state.clone()))
        // ── User-facing utility API ────────────────────────────────────
        .route("/api/me", get(api_me))
        .route("/api/user/legal-status", get(api_user_legal_status))
        .route("/api/user/legal-accept", post(api_user_legal_accept))
        // ── Deposit & order status polling ────────────────────────────
        .route("/api/orders/:order_id", get(api_order_detail))
        .route("/api/deposits/:deposit_id/status", get(api_deposit_status))
        // ── Reports (still in main.rs pending Phase 5 extraction) ─────
        .route("/api/admin/reports/:report_type", get(api_admin_reports))
        // ── Profile / KYC page redirect ───────────────────────────────
        .route("/profile", get(auth::routes::page_profile))
        .route("/welcome", get(auth::routes::page_welcome))
        // ── Payment result pages ──────────────────────────────────────
        .route("/payment-success", get(page_payment_success))
        .route("/payment-in-progress", get(page_payment_in_progress))
        // ── Community (demo) ──────────────────────────────────────────
        .route("/community", get(page_community))
        // ── Static file serving & fallbacks ───────────────────────────
        .route("/", get(handle_root))
        .nest_service("/en", ServeDir::new("../frontend/www/en"))
        .nest_service("/id", ServeDir::new("../frontend/www/id"))
        .nest_service("/fonts", ServeDir::new("../frontend/www/fonts"))
        .nest_service("/static", ServeDir::new("../frontend/platform/static"))
        .nest_service("/images", ServeDir::new("../frontend/platform/images"))
        .nest_service("/uploads", ServeDir::new("../uploads"))
        .route("/health", get(handle_health))
        .fallback_service(
            ServeDir::new("../frontend/platform")
                .fallback(ServeFile::new("../frontend/platform/404.html")),
        )
        .with_state(state.clone())
        .layer(axum::extract::DefaultBodyLimit::max(25 * 1024 * 1024)) // 25 MB for file uploads
        .layer(tower_http::compression::CompressionLayer::new())
        .layer({
            let cors = tower_http::cors::CorsLayer::new()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]);

            // In production/staging, restrict CORS to configured domains.
            // In development, allow any origin for local testing convenience.
            let is_dev = matches!(
                std::env::var("POOOL_ENV").as_deref(),
                Ok("development") | Ok("dev") | Ok("local")
            );

            if is_dev {
                cors.allow_origin(tower_http::cors::Any)
            } else {
                // Dynamically allow origins based on BASE_URL
                let base_url = std::env::var("BASE_URL")
                    .unwrap_or_else(|_| "https://platform.poool.app".to_string());
                let mut origins: Vec<axum::http::HeaderValue> = Vec::new();
                if let Ok(v) = base_url.parse() {
                    origins.push(v);
                }

                let base_host = base_url.replace("https://", "").replace("http://", "");
                let bare_domain = base_host.replace("platform.", "");

                let scheme = if base_url.starts_with("https") {
                    "https"
                } else {
                    "http"
                };

                let www_url = format!("{}://www.{}", scheme, bare_domain);
                if let Ok(v) = www_url.parse() {
                    origins.push(v);
                }

                let bare_url = format!("{}://{}", scheme, bare_domain);
                if let Ok(v) = bare_url.parse() {
                    origins.push(v);
                }

                cors.allow_origin(origins)
            }
        })
        .layer(tower::limit::concurrency::ConcurrencyLimitLayer::new(100))
        .layer(axum::middleware::from_fn(auth::csrf::csrf_middleware))
        .layer(axum::middleware::from_fn(apply_security_headers))
        // Sentry user context: attach user.id + email to every Sentry event
        .layer(axum::middleware::from_fn_with_state(
            state,
            sentry_user_context,
        ))
        // Sentry layers MUST be outermost (last added = wraps everything)
        .layer(sentry::integrations::tower::SentryLayer::new_from_top())
        .layer(sentry::integrations::tower::NewSentryLayer::new_from_top());

    // ── Host-based dispatch ───────────────────────────────────────────
    let app = HostDispatch {
        www: www_router,
        platform: platform_router,
    };

    //  Start server
    let addr = SocketAddr::from((
        config
            .server_host
            .parse::<std::net::IpAddr>()
            .unwrap_or([0, 0, 0, 0].into()),
        config.server_port,
    ));
    tracing::info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn apply_security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::STRICT_TRANSPORT_SECURITY,
        axum::http::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        axum::http::header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static("default-src 'self'; script-src 'self' 'unsafe-inline' blob: https://cdn.jsdelivr.net https://unpkg.com https://js.stripe.com https://browser.sentry-cdn.com https://cdnjs.cloudflare.com https://cdn.quilljs.com; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdnjs.cloudflare.com https://cdn.quilljs.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob: https:; connect-src 'self' https: wss: https://*.ingest.de.sentry.io; frame-src https://js.stripe.com; worker-src 'self' blob:; base-uri 'self'; form-action 'self';"),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("x-permitted-cross-domain-policies"),
        axum::http::HeaderValue::from_static("none"),
    );
    headers.insert(
        axum::http::header::HeaderName::from_static("permissions-policy"),
        axum::http::HeaderValue::from_static("accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()"),
    );

    response
}

/// Sentry user-context middleware: attaches user.id + email to every Sentry event
/// so that errors can be correlated with affected users. Skips static-asset paths
/// to avoid unnecessary DB lookups.
async fn sentry_user_context(
    State(state): State<AppState>,
    jar: CookieJar,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> impl axum::response::IntoResponse {
    let path = req.uri().path();
    // Skip static-asset paths — no session to look up
    if !path.starts_with("/static/")
        && !path.starts_with("/fonts/")
        && !path.starts_with("/images/")
        && !path.starts_with("/en/")
        && !path.starts_with("/id/")
    {
        if let Some(user) = crate::auth::middleware::get_current_user(&jar, &state.db).await {
            sentry::configure_scope(|scope| {
                scope.set_user(Some(sentry::User {
                    id: Some(user.id.to_string()),
                    email: Some(user.email.clone()),
                    ..Default::default()
                }));
            });
        }
    }
    next.run(req).await
}

//  Root redirect

//  Root redirect

/// GET /api/user/legal-status — Check whether logged-in user has accepted the current Terms version.
/// Returns { needs_reaccept: bool, current_version: "1.0", accepted_version: "1.0"|null }
/// The frontend uses this to show/hide the re-acceptance banner on every page load.
async fn api_user_legal_status(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    // Fetch current terms version from platform settings
    let current_version: String =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = 'legal_terms_version'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

    // Fetch user's most recently accepted version
    let accepted_version: Option<String> = sqlx::query_scalar(
        "SELECT terms_version FROM user_consents WHERE user_id = $1 ORDER BY accepted_at DESC LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let needs_reaccept = accepted_version.as_deref() != Some(current_version.as_str());

    Json(serde_json::json!({
        "needs_reaccept": needs_reaccept,
        "current_version": current_version,
        "accepted_version": accepted_version,
    }))
    .into_response()
}

/// POST /api/user/legal-accept — Record user's acceptance of the current Terms version.
/// Called when user clicks "Accept" on the re-acceptance banner.
async fn api_user_legal_accept(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let current_version: String =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = 'legal_terms_version'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

    let result = sqlx::query(
        "INSERT INTO user_consents (user_id, terms_version, ip_address, user_agent) VALUES ($1, $2, $3, $4)"
    )
    .bind(user.id)
    .bind(&current_version)
    .bind(ip.as_deref())
    .bind(user_agent.as_deref())
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            Json(serde_json::json!({"status": "success", "accepted_version": current_version}))
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to record legal acceptance: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to record acceptance"})),
            )
                .into_response()
        }
    }
}

/// GET /api/admin/reports/:report_type — Generate aggregated report data with date filtering.
///
/// Supports report types: financial-summary, user-growth, kyc-status, asset-performance,
/// aml-compliance, rewards-liability, referral-effectiveness, multi-currency, invoice-summary,
/// deposit-summary, order-summary, investment-summary, support-summary, audit-summary.
///
/// Query params: ?from=YYYY-MM-DD&to=YYYY-MM-DD
async fn api_admin_reports(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Path(report_type): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    // Admin auth check
    if !auth::middleware::is_admin(&jar, &state.db).await {
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Forbidden"})),
        )
            .into_response();
    }

    let date_from = params.get("from").cloned().unwrap_or_default();
    let date_to = params.get("to").cloned().unwrap_or_default();

    use sqlx::Row;

    match report_type.as_str() {
        "financial-summary" | "monthly-financial" => {
            let rows = sqlx::query(
                r#"SELECT
                     TO_CHAR(created_at, 'YYYY-MM') as month,
                     type as transaction_type,
                     status,
                     currency,
                     COUNT(*) as tx_count,
                     COALESCE(SUM(amount_cents), 0)::bigint as total_cents
                   FROM wallet_transactions
                   WHERE ($1::text = '' OR created_at >= $1::date)
                     AND ($2::text = '' OR created_at <= ($2::date + interval '1 day'))
                   GROUP BY month, type, status, currency
                   ORDER BY month DESC, type"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "month": r.get::<String, _>("month"),
                        "transaction_type": r.get::<String, _>("transaction_type"),
                        "status": r.get::<String, _>("status"),
                        "currency": r.get::<String, _>("currency"),
                        "tx_count": r.get::<i64, _>("tx_count"),
                        "total_cents": r.get::<i64, _>("total_cents"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "financial-summary", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "user-growth" => {
            let rows = sqlx::query(
                r#"SELECT
                     TO_CHAR(created_at, 'YYYY-MM-DD') as signup_date,
                     COUNT(*) as signups,
                     SUM(CASE WHEN email_verified THEN 1 ELSE 0 END) as verified
                   FROM users
                   WHERE ($1::text = '' OR created_at >= $1::date)
                     AND ($2::text = '' OR created_at <= ($2::date + interval '1 day'))
                   GROUP BY signup_date
                   ORDER BY signup_date DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "date": r.get::<String, _>("signup_date"),
                        "signups": r.get::<i64, _>("signups"),
                        "verified": r.get::<i64, _>("verified"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "user-growth", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "kyc" | "kyc-status" => {
            let rows = sqlx::query(
                r#"SELECT
                     k.id::text, u.email, k.status, k.document_type,
                     k.pep_check_passed, k.sanctions_check,
                     k.rejection_reason,
                     k.created_at::text, k.verified_at::text, k.expires_at::text
                   FROM kyc_records k
                   JOIN users u ON k.user_id = u.id
                   WHERE ($1::text = '' OR k.created_at >= $1::date)
                     AND ($2::text = '' OR k.created_at <= ($2::date + interval '1 day'))
                   ORDER BY k.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "status": r.get::<String, _>("status"),
                        "document_type": r.get::<Option<String>, _>("document_type"),
                        "pep_check_passed": r.get::<Option<bool>, _>("pep_check_passed"),
                        "sanctions_check": r.get::<Option<bool>, _>("sanctions_check"),
                        "rejection_reason": r.get::<Option<String>, _>("rejection_reason"),
                        "created_at": r.get::<String, _>("created_at"),
                        "verified_at": r.get::<Option<String>, _>("verified_at"),
                        "expires_at": r.get::<Option<String>, _>("expires_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "kyc-status", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "aml" | "aml-compliance" => {
            let rows = sqlx::query(
                r#"SELECT
                     k.id::text, u.email,
                     COALESCE(p.first_name, '') || ' ' || COALESCE(p.last_name, '') as full_name,
                     k.status, k.pep_check_passed, k.sanctions_check,
                     k.rejection_reason, k.created_at::text
                   FROM kyc_records k
                   JOIN users u ON k.user_id = u.id
                   LEFT JOIN user_profiles p ON k.user_id = p.user_id
                   WHERE (k.pep_check_passed = true OR k.sanctions_check = true
                          OR k.status = 'rejected')
                     AND ($1::text = '' OR k.created_at >= $1::date)
                     AND ($2::text = '' OR k.created_at <= ($2::date + interval '1 day'))
                   ORDER BY k.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "full_name": r.get::<String, _>("full_name"),
                        "status": r.get::<String, _>("status"),
                        "pep_check_passed": r.get::<Option<bool>, _>("pep_check_passed"),
                        "sanctions_check": r.get::<Option<bool>, _>("sanctions_check"),
                        "rejection_reason": r.get::<Option<String>, _>("rejection_reason"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "aml-compliance", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "assets" | "asset-performance" => {
            let rows = sqlx::query(
                r#"SELECT
                     a.id::text, a.title, a.asset_type, a.funding_status,
                     a.total_value_cents, a.token_price_cents,
                     a.tokens_total, a.tokens_available,
                     a.annual_yield_bps, a.capital_appreciation_bps,
                     a.occupancy_rate_bps, a.featured, a.published,
                     a.created_at::text
                   FROM assets a
                   WHERE ($1::text = '' OR a.created_at >= $1::date)
                     AND ($2::text = '' OR a.created_at <= ($2::date + interval '1 day'))
                   ORDER BY a.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    let total: i32 = r.get::<Option<i32>, _>("tokens_total").unwrap_or(0);
                    let available: i32 = r.get::<Option<i32>, _>("tokens_available").unwrap_or(0);
                    let pct = if total > 0 {
                        (total - available) as f64 / total as f64 * 100.0
                    } else {
                        0.0
                    };
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "title": r.get::<Option<String>, _>("title"),
                        "asset_type": r.get::<Option<String>, _>("asset_type"),
                        "funding_status": r.get::<Option<String>, _>("funding_status"),
                        "total_value_cents": r.get::<Option<i64>, _>("total_value_cents"),
                        "token_price_cents": r.get::<Option<i64>, _>("token_price_cents"),
                        "tokens_total": total,
                        "tokens_available": available,
                        "pct_funded": format!("{:.1}", pct),
                        "annual_yield_bps": r.get::<Option<i32>, _>("annual_yield_bps"),
                        "featured": r.get::<Option<bool>, _>("featured"),
                        "published": r.get::<Option<bool>, _>("published"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "asset-performance", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "investments" | "investment-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     i.id::text, u.email,
                     a.title as asset_title,
                     i.tokens_owned, i.purchase_value_cents, i.current_value_cents,
                     i.total_rental_cents, i.status, i.created_at::text
                   FROM investments i
                   JOIN users u ON i.user_id = u.id
                   JOIN assets a ON i.asset_id = a.id
                   WHERE ($1::text = '' OR i.created_at >= $1::date)
                     AND ($2::text = '' OR i.created_at <= ($2::date + interval '1 day'))
                   ORDER BY i.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "asset_title": r.get::<Option<String>, _>("asset_title"),
                        "tokens_owned": r.get::<Option<i32>, _>("tokens_owned"),
                        "purchase_value_cents": r.get::<Option<i64>, _>("purchase_value_cents"),
                        "current_value_cents": r.get::<Option<i64>, _>("current_value_cents"),
                        "total_rental_cents": r.get::<Option<i64>, _>("total_rental_cents"),
                        "status": r.get::<Option<String>, _>("status"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "investment-summary", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "tax-pl" | "tax-reporting" => {
            let rows = sqlx::query(
                r#"SELECT
                     u.email, t.fiscal_year as year, t.total_dividends_cents as total_profit_cents, 
                     t.capital_gains_cents, t.withholding_tax_cents, t.status, 
                     t.generated_at::text, t.pdf_url as report_url
                   FROM tax_reports t
                   JOIN users u ON t.user_id = u.id
                   WHERE ($1::text = '' OR t.created_at >= $1::date)
                     AND ($2::text = '' OR t.created_at <= ($2::date + interval '1 day'))
                   ORDER BY t.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "email": r.get::<String, _>("email"),
                        "year": r.get::<i32, _>("year"),
                        "total_profit_cents": r.get::<i64, _>("total_profit_cents"),
                        "capital_gains_cents": r.get::<i64, _>("capital_gains_cents"),
                        "withholding_tax_cents": r.get::<i64, _>("withholding_tax_cents"),
                        "status": r.get::<String, _>("status"),
                        "generated_at": r.get::<Option<String>, _>("generated_at"),
                        "report_url": r.get::<Option<String>, _>("report_url"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "tax-reporting", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "tax-withholding" => {
            let rows = sqlx::query(
                r#"SELECT
                     u.email, t.fiscal_year as year, t.withholding_tax_cents, t.status, 
                     t.generated_at::text, t.pdf_url as report_url
                   FROM tax_reports t
                   JOIN users u ON t.user_id = u.id
                   WHERE ($1::text = '' OR t.created_at >= $1::date)
                     AND ($2::text = '' OR t.created_at <= ($2::date + interval '1 day'))
                   ORDER BY t.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "email": r.get::<String, _>("email"),
                        "year": r.get::<i32, _>("year"),
                        "withholding_tax_cents": r.get::<i64, _>("withholding_tax_cents"),
                        "status": r.get::<String, _>("status"),
                        "generated_at": r.get::<Option<String>, _>("generated_at"),
                        "report_url": r.get::<Option<String>, _>("report_url"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "tax-withholding", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "orders" | "order-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     o.id::text, o.order_number, u.email,
                     o.total_cents, o.currency, o.status,
                     o.payment_method, o.payment_ref_id,
                     o.created_at::text, o.completed_at::text
                   FROM orders o
                   JOIN users u ON o.user_id = u.id
                   WHERE ($1::text = '' OR o.created_at >= $1::date)
                     AND ($2::text = '' OR o.created_at <= ($2::date + interval '1 day'))
                   ORDER BY o.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "order_number": r.get::<Option<String>, _>("order_number"),
                        "email": r.get::<String, _>("email"),
                        "total_cents": r.get::<Option<i64>, _>("total_cents"),
                        "currency": r.get::<Option<String>, _>("currency"),
                        "status": r.get::<Option<String>, _>("status"),
                        "payment_method": r.get::<Option<String>, _>("payment_method"),
                        "payment_ref_id": r.get::<Option<String>, _>("payment_ref_id"),
                        "created_at": r.get::<String, _>("created_at"),
                        "completed_at": r.get::<Option<String>, _>("completed_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "order-summary", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "rewards-liability" => {
            let rows = sqlx::query(
                r#"SELECT
                     u.email,
                     rb.cashback, rb.referrals, rb.promotions,
                     (rb.cashback + rb.referrals + rb.promotions) as total_cents,
                     t.name as tier_name,
                     ut.invested_12m
                   FROM rewards_balances rb
                   JOIN users u ON rb.user_id = u.id
                   LEFT JOIN user_tiers ut ON rb.user_id = ut.user_id
                   LEFT JOIN tiers t ON ut.tier_id = t.id
                   WHERE ($1::text = '' OR rb.updated_at >= $1::date)
                     AND ($2::text = '' OR rb.updated_at <= ($2::date + interval '1 day'))
                   ORDER BY (rb.cashback + rb.referrals + rb.promotions) DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "email": r.get::<String, _>("email"),
                        "cashback_cents": r.get::<Option<i64>, _>("cashback"),
                        "referral_cents": r.get::<Option<i64>, _>("referrals"),
                        "promotion_cents": r.get::<Option<i64>, _>("promotions"),
                        "total_cents": r.get::<Option<i64>, _>("total_cents"),
                        "tier": r.get::<Option<String>, _>("tier_name"),
                        "invested_12m_cents": r.get::<Option<i64>, _>("invested_12m"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "rewards-liability", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "referral-effectiveness" => {
            let rows = sqlx::query(
                r#"SELECT
                     u1.email as referrer_email,
                     u2.email as referred_email,
                     rt.status,
                     rt.referrer_reward, rt.referred_reward,
                     rt.created_at::text, rt.qualified_at::text
                   FROM referral_tracking rt
                   JOIN users u1 ON rt.referrer_id = u1.id
                   JOIN users u2 ON rt.referred_id = u2.id
                   WHERE ($1::text = '' OR rt.created_at >= $1::date)
                     AND ($2::text = '' OR rt.created_at <= ($2::date + interval '1 day'))
                   ORDER BY rt.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "referrer_email": r.get::<String, _>("referrer_email"),
                        "referred_email": r.get::<String, _>("referred_email"),
                        "status": r.get::<String, _>("status"),
                        "referrer_reward_cents": r.get::<i64, _>("referrer_reward"),
                        "referred_reward_cents": r.get::<i64, _>("referred_reward"),
                        "created_at": r.get::<String, _>("created_at"),
                        "qualified_at": r.get::<Option<String>, _>("qualified_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "referral-effectiveness", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "multi-currency" => {
            let rows = sqlx::query(
                r#"SELECT
                     currency,
                     wallet_type,
                     COUNT(*) as wallet_count,
                     COALESCE(SUM(balance_cents), 0)::bigint as total_balance_cents
                   FROM wallets
                   GROUP BY currency, wallet_type
                   ORDER BY currency, wallet_type"#,
            )
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "currency": r.get::<String, _>("currency"),
                        "wallet_type": r.get::<String, _>("wallet_type"),
                        "wallet_count": r.get::<i64, _>("wallet_count"),
                        "total_balance_cents": r.get::<i64, _>("total_balance_cents"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "multi-currency", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "invoices" | "invoice-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     i.id::text, i.invoice_number, u.email,
                     i.subtotal_cents, i.tax_cents, i.total_cents,
                     i.currency, i.status, i.issued_at::text
                   FROM invoices i
                   JOIN orders o ON i.order_id = o.id
                   JOIN users u ON o.user_id = u.id
                   WHERE ($1::text = '' OR i.issued_at >= $1::date)
                     AND ($2::text = '' OR i.issued_at <= ($2::date + interval '1 day'))
                   ORDER BY i.issued_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "invoice_number": r.get::<Option<String>, _>("invoice_number"),
                        "email": r.get::<String, _>("email"),
                        "subtotal_cents": r.get::<Option<i64>, _>("subtotal_cents"),
                        "tax_cents": r.get::<Option<i64>, _>("tax_cents"),
                        "total_cents": r.get::<Option<i64>, _>("total_cents"),
                        "currency": r.get::<Option<String>, _>("currency"),
                        "status": r.get::<Option<String>, _>("status"),
                        "issued_at": r.get::<Option<String>, _>("issued_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "invoice-summary", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "support" | "support-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     t.id::text, t.subject, u.email,
                     t.status, t.priority,
                     t.created_at::text, t.updated_at::text
                   FROM support_tickets t
                   JOIN users u ON t.user_id = u.id
                   WHERE ($1::text = '' OR t.created_at >= $1::date)
                     AND ($2::text = '' OR t.created_at <= ($2::date + interval '1 day'))
                   ORDER BY t.created_at DESC"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "subject": r.get::<String, _>("subject"),
                        "email": r.get::<String, _>("email"),
                        "status": r.get::<String, _>("status"),
                        "priority": r.get::<String, _>("priority"),
                        "created_at": r.get::<String, _>("created_at"),
                        "updated_at": r.get::<Option<String>, _>("updated_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "support-summary", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "audit" | "audit-summary" => {
            let rows = sqlx::query(
                r#"SELECT
                     a.id::text,
                     COALESCE(u.email, 'system') as actor_email,
                     a.action, a.entity_type, a.entity_id::text,
                     a.ip_address::text, a.created_at::text
                   FROM audit_logs a
                   LEFT JOIN users u ON a.actor_user_id = u.id
                   WHERE ($1::text = '' OR a.created_at >= $1::date)
                     AND ($2::text = '' OR a.created_at <= ($2::date + interval '1 day'))
                   ORDER BY a.created_at DESC
                   LIMIT 5000"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "actor_email": r.get::<String, _>("actor_email"),
                        "action": r.get::<Option<String>, _>("action"),
                        "entity_type": r.get::<Option<String>, _>("entity_type"),
                        "entity_id": r.get::<Option<String>, _>("entity_id"),
                        "ip_address": r.get::<Option<String>, _>("ip_address"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "audit-summary", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        "wallet-transactions" => {
            let rows = sqlx::query(
                r#"SELECT
                     wt.id::text, u.email,
                     w.wallet_type, wt.type as tx_type,
                     wt.status, wt.amount_cents, wt.currency,
                     wt.description, wt.external_ref_id,
                     wt.created_at::text
                   FROM wallet_transactions wt
                   JOIN wallets w ON wt.wallet_id = w.id
                   JOIN users u ON w.user_id = u.id
                   WHERE ($1::text = '' OR wt.created_at >= $1::date)
                     AND ($2::text = '' OR wt.created_at <= ($2::date + interval '1 day'))
                   ORDER BY wt.created_at DESC
                   LIMIT 5000"#,
            )
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.get::<String, _>("id"),
                        "email": r.get::<String, _>("email"),
                        "wallet_type": r.get::<String, _>("wallet_type"),
                        "tx_type": r.get::<String, _>("tx_type"),
                        "status": r.get::<String, _>("status"),
                        "amount_cents": r.get::<i64, _>("amount_cents"),
                        "currency": r.get::<String, _>("currency"),
                        "description": r.get::<Option<String>, _>("description"),
                        "external_ref_id": r.get::<Option<String>, _>("external_ref_id"),
                        "created_at": r.get::<String, _>("created_at"),
                    })
                })
                .collect();

            Json(serde_json::json!({"report_type": "wallet-transactions", "date_from": date_from, "date_to": date_to, "rows": data})).into_response()
        }

        _ => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Unknown report type: '{}'", report_type),
                "available_types": [
                    "financial-summary", "user-growth", "kyc-status", "aml-compliance",
                    "asset-performance", "investment-summary", "order-summary",
                    "rewards-liability", "referral-effectiveness", "multi-currency",
                    "invoice-summary", "support-summary", "audit-summary", "wallet-transactions"
                ]
            })),
        )
            .into_response(),
    }
}

/// GET /api/orders/:order_id — returns order + items for the logged-in user
async fn api_order_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    use sqlx::Row;
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let order = sqlx::query(
        "SELECT id, order_number, total_cents, currency, payment_method, status, created_at FROM orders WHERE id::text = $1 AND user_id = $2"
    )
        .bind(&order_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await;

    match order {
        Ok(Some(row)) => {
            let order_uuid: uuid::Uuid = row.get("id");
            let items_rows = sqlx::query(
                "SELECT oi.tokens_quantity, oi.token_price_cents, oi.subtotal_cents, a.title as asset_title FROM order_items oi LEFT JOIN assets a ON oi.asset_id = a.id WHERE oi.order_id = $1"
            )
                .bind(order_uuid)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();

            let items: Vec<serde_json::Value> = items_rows
                .iter()
                .map(|ir| {
                    serde_json::json!({
                        "asset_title": ir.get::<Option<String>, _>("asset_title"),
                        "tokens_quantity": ir.get::<i32, _>("tokens_quantity"),
                        "token_price_cents": ir.get::<i64, _>("token_price_cents"),
                        "subtotal_cents": ir.get::<i64, _>("subtotal_cents"),
                    })
                })
                .collect();

            // Check for invoice
            let invoice = sqlx::query("SELECT pdf_url FROM invoices WHERE order_id = $1 LIMIT 1")
                .bind(order_uuid)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

            Json(serde_json::json!({
                "id": order_id,
                "order_number": row.get::<Option<String>, _>("order_number"),
                "total_cents": row.get::<i64, _>("total_cents"),
                "currency": row.get::<Option<String>, _>("currency"),
                "payment_method": row.get::<Option<String>, _>("payment_method"),
                "status": row.get::<Option<String>, _>("status"),
                "created_at": row.get::<Option<String>, _>("created_at"),
                "items": items,
                "invoice_url": invoice.and_then(|i| i.get::<Option<String>, _>("pdf_url")),
            }))
            .into_response()
        }
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Order not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to load order: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/deposits/:deposit_id/status — returns deposit request status for polling
async fn api_deposit_status(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(deposit_id): Path<String>,
) -> impl IntoResponse {
    use sqlx::Row;
    let user = match crate::auth::middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Unauthorized"})),
            )
                .into_response()
        }
    };

    let deposit = sqlx::query(
        "SELECT id, amount_cents, currency, status, provider, provider_reference, order_id, created_at FROM deposit_requests WHERE id::text = $1 AND user_id = $2"
    )
        .bind(&deposit_id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await;

    match deposit {
        Ok(Some(row)) => Json(serde_json::json!({
            "id": deposit_id,
            "amount_cents": row.get::<i64, _>("amount_cents"),
            "currency": row.get::<Option<String>, _>("currency"),
            "status": row.get::<Option<String>, _>("status"),
            "provider": row.get::<Option<String>, _>("provider"),
            "provider_reference": row.get::<Option<String>, _>("provider_reference"),
            "order_id": row.get::<Option<String>, _>("order_id"),
            "created_at": row.get::<Option<String>, _>("created_at"),
        }))
        .into_response(),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Deposit not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to load deposit: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Server error"})),
            )
                .into_response()
        }
    }
}

/// GET /health — Health check endpoint for Cloud Run and uptime monitors.
/// Returns 200 OK with a JSON status payload.
async fn handle_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Root `/` redirect (platform only). If authenticated, go to marketplace; otherwise go to login.
async fn handle_root(jar: CookieJar, State(state): State<AppState>) -> Redirect {
    if crate::auth::middleware::is_authenticated(&jar, &state.db).await {
        Redirect::to("/marketplace")
    } else {
        Redirect::to("/auth/login")
    }
}

// ── Host-based routing service ────────────────────────────────────────────
//
// Dispatches incoming requests to the correct router based on the `Host` header:
//   • www.poool.app     → www_router (landing page)
//   • poool.app         → 301 redirect to https://www.poool.app{path}
//   • platform.poool.app / localhost / anything else → platform_router

/// A tower `Service` that dispatches to either the www or platform router
/// based on the HTTP `Host` header.
#[derive(Clone)]
struct HostDispatch {
    www: Router,
    platform: Router,
}

impl tower::Service<axum::http::Request<axum::body::Body>> for HostDispatch {
    type Response = axum::response::Response;
    type Error = std::convert::Infallible;
    type Future =
        Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::http::Request<axum::body::Body>) -> Self::Future {
        let host = req
            .headers()
            .get(axum::http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase();

        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "https://platform.poool.app".to_string());
        let base_host = base_url.replace("https://", "").replace("http://", "");
        let bare_domain = base_host.replace("platform.", "");
        let www_domain = format!("www.{}", bare_domain);

        if host == bare_domain {
            // Bare domain → permanent redirect to www
            let path_and_query = req
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str().to_string())
                .unwrap_or_else(|| "/".to_string());
            Box::pin(async move {
                Ok(
                    Redirect::permanent(&format!("https://{}{}", www_domain, path_and_query))
                        .into_response(),
                )
            })
        } else if host == www_domain {
            // WWW → landing page
            let mut router = self.www.clone();
            Box::pin(async move {
                let resp = tower::Service::call(&mut router, req).await;
                Ok(resp.into_response())
            })
        } else {
            // Everything else (platform, localhost, Cloud Run URL) → platform
            let mut router = self.platform.clone();
            Box::pin(async move {
                let resp = tower::Service::call(&mut router, req).await;
                Ok(resp.into_response())
            })
        }
    }
}

//  Protected page handlers
//
// Each handler checks the session cookie. If valid, serves the HTML file.
// If not authenticated, redirects to the login page.
//
// These are intentionally separate functions (not a macro) for clarity
// and to make it easy to add per-page logic later (e.g. user data injection).

/// GET /payment-success  Payment success page (protected).
async fn page_payment_success(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "payment-success.html").await
}

/// GET /community — Community demo page (protected).
async fn page_community(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "community.html").await
}

/// GET /payment-in-progress  Payment in progress page (protected).
async fn page_payment_in_progress(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    common::routes_helper::serve_protected(jar, &state, "payment-in-progress.html").await
}

//  API Endpoints

/// GET /api/me  Return current user's profile data as JSON.
///
/// Used by the frontend `user-data.js` to replace hardcoded
/// placeholder names with real user data on all pages.
///
/// Returns 401 if not authenticated.
async fn api_me(jar: CookieJar, State(state): State<AppState>) -> axum::response::Response {
    let session_token = match jar.get(auth::middleware::SESSION_COOKIE) {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Not authenticated"
                })),
            )
                .into_response();
        }
    };

    match auth::service::get_user_profile(&state.db, &session_token).await {
        Ok(Some(profile)) => Json(profile).into_response(),
        Ok(None) => (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Session expired"
            })),
        )
            .into_response(),
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Internal server error"
            })),
        )
            .into_response(),
    }
}

// NOTE: KYC submission is now handled by the `kyc` module's own router.

// ─── Automated Migration Runner ──────────────────────────────────────────────

/// Run all pending SQL migrations from `../database/` in alphanumeric order.
///
/// Migrations are tracked in a `_schema_migrations` table. Each migration file
/// is run exactly once. If a migration fails, the error is logged and the server
/// continues (allowing partial migration scenarios to be debugged).
async fn run_migrations(pool: &sqlx::PgPool) {
    // 1. Ensure tracking table exists
    if let Err(e) = sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS _schema_migrations (
            filename TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    {
        tracing::error!("Failed to create _schema_migrations table: {}", e);
        return;
    }

    // 2. Read migration files from ../database/ (relative to CWD = backend/)
    let migrations_dir = std::path::Path::new("../database");
    if !migrations_dir.exists() {
        tracing::warn!(
            "Migrations directory {:?} not found — skipping migrations",
            migrations_dir
        );
        return;
    }

    let mut files: Vec<String> = match std::fs::read_dir(migrations_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".sql") && !name.starts_with("seed") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect(),
        Err(e) => {
            tracing::error!("Failed to read migrations directory: {}", e);
            return;
        }
    };

    files.sort();

    // 3. Fetch already-applied migrations
    let applied: Vec<String> =
        sqlx::query_scalar::<_, String>("SELECT filename FROM _schema_migrations")
            .fetch_all(pool)
            .await
            .unwrap_or_default();

    let applied_set: std::collections::HashSet<&str> = applied.iter().map(|s| s.as_str()).collect();

    // 4. Apply pending migrations
    let mut applied_count = 0;
    for filename in &files {
        if applied_set.contains(filename.as_str()) {
            continue;
        }

        let filepath = migrations_dir.join(filename);
        let sql = match std::fs::read_to_string(&filepath) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to read migration file {:?}: {}", filepath, e);
                continue;
            }
        };

        tracing::info!("Applying migration: {}", filename);

        // Run migration in a transaction
        let mut tx = match pool.begin().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Failed to begin transaction for migration {}: {}",
                    filename,
                    e
                );
                continue;
            }
        };

        // Split SQL into individual statements — sqlx cannot execute multiple
        // statements in a single prepared-statement call.
        // We must respect dollar-quoted blocks (e.g. DO $$ ... END $$;) which
        // contain semicolons that are NOT statement separators.
        let statements = split_sql_statements(&sql);
        let mut migration_failed = false;
        for stmt in &statements {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                tracing::error!("Migration {} failed on statement: {}", filename, e);
                migration_failed = true;
                break;
            }
        }
        if migration_failed {
            let _ = tx.rollback().await;
            continue;
        }

        // Record as applied
        if let Err(e) = sqlx::query("INSERT INTO _schema_migrations (filename) VALUES ($1)")
            .bind(filename)
            .execute(&mut *tx)
            .await
        {
            tracing::error!("Failed to record migration {}: {}", filename, e);
            let _ = tx.rollback().await;
            continue;
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("Failed to commit migration {}: {}", filename, e);
            continue;
        }

        applied_count += 1;
    }

    if applied_count > 0 {
        tracing::info!("Applied {} new migration(s)", applied_count);
    } else {
        tracing::info!(
            "Database schema is up to date ({} migrations tracked)",
            applied.len()
        );
    }
}

/// Split a SQL string into individual statements, respecting dollar-quoting,
/// single-quoted strings, and SQL comments. Only top-level semicolons are used
/// as statement separators.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = sql.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_dollar_quote = false;
    let mut dollar_tag = String::new();

    while i < len {
        // Check for dollar-quoting: $$ or $tag$
        if chars[i] == '$' && !in_dollar_quote {
            // Try to read a dollar-quote tag: $<optional_identifier>$
            i += 1;
            let mut tag = String::from("$");
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                tag.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == '$' {
                tag.push('$');
                i += 1;
                in_dollar_quote = true;
                dollar_tag = tag.clone();
                current.push_str(&dollar_tag);
                continue;
            } else {
                // Not a dollar-quote, push what we consumed
                current.push_str(&tag);
                continue;
            }
        }

        // Check for closing dollar-quote
        if in_dollar_quote && chars[i] == '$' {
            let mut tag = String::from("$");
            i += 1;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                tag.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == '$' {
                tag.push('$');
                i += 1;
                if tag == dollar_tag {
                    in_dollar_quote = false;
                    current.push_str(&tag);
                    dollar_tag.clear();
                    continue;
                }
            }
            // Not the closing tag, push what we consumed
            current.push_str(&tag);
            continue;
        }

        // Inside a dollar-quoted block — everything is literal
        if in_dollar_quote {
            current.push(chars[i]);
            i += 1;
            continue;
        }

        // Single-line comment: --
        if chars[i] == '-' && i + 1 < len && chars[i + 1] == '-' {
            while i < len && chars[i] != '\n' {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            current.push('/');
            current.push('*');
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                current.push(chars[i]);
                i += 1;
            }
            if i + 1 < len {
                current.push('*');
                current.push('/');
                i += 2;
            }
            continue;
        }

        // Single-quoted string: '...' (with '' escape)
        if chars[i] == '\'' {
            current.push('\'');
            i += 1;
            while i < len {
                if chars[i] == '\'' {
                    current.push('\'');
                    i += 1;
                    if i < len && chars[i] == '\'' {
                        // escaped quote
                        current.push('\'');
                        i += 1;
                    } else {
                        break;
                    }
                } else {
                    current.push(chars[i]);
                    i += 1;
                }
            }
            continue;
        }

        // Top-level semicolon: statement boundary
        if chars[i] == ';' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
            i += 1;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    // Leftover
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    statements
}
