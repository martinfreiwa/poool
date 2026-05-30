#![allow(clippy::items_after_test_module)]

/// Auth HTTP routes – thin handlers that delegate to the service layer.
///
/// Each handler is responsible ONLY for:
/// 1. Extracting data from the HTTP request
/// 2. Calling the appropriate service function
/// 3. Formatting the HTTP response
///
/// NO business logic lives here.
use axum::{
    extract::{Extension, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{delete, get, post},
    Form, Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use minijinja::context;
use std::time::Duration;
use tokio::time::{sleep, timeout, Instant};

use super::middleware;
use super::middleware::SESSION_COOKIE;
use super::models::{
    LoginForm, PasskeyLoginFinishRequest, PasskeyRegisterFinishRequest, SignupForm,
};
use crate::common::{email, validation};

/// Determine whether the session cookie should have the `Secure` flag.
///
/// Defaults to `true` (secure) for maximum safety. Only returns `false`
/// when `POOOL_ENV` is explicitly set to "development" or "local"
/// (i.e. plain HTTP on localhost).
fn cookie_is_secure() -> bool {
    !matches!(
        std::env::var("POOOL_ENV").as_deref(),
        Ok("development") | Ok("dev") | Ok("local")
    )
}
use super::service;
use crate::error::AppError;

/// Shared application state passed to all handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    /// Optional read-replica pool for non-critical reads (Phase 1.1).
    pub db_replica: Option<sqlx::PgPool>,
    /// Optional community database pool (Phase 1.1).
    pub community_db: Option<sqlx::PgPool>,
    pub templates: crate::templates::Templates,
    pub config: crate::config::Config,
    /// Optional Redis connection pool for caching sessions or query results.
    pub redis: Option<deadpool_redis::Pool>,
    /// Rate limiter for auth endpoints (login, signup, password reset).
    pub auth_rate_limiter: super::rate_limit::RateLimiter,
    /// Rate limiter for leaderboard read/write endpoints. Higher cap than
    /// the auth limiter (browsing the leaderboard is benign) but still
    /// enough to block scrapers and to soft-throttle PUT preferences spam.
    pub leaderboard_rate_limiter: super::rate_limit::RateLimiter,
    /// Rate limiter for community mutating endpoints (circle join/leave/
    /// invite/promote/ban). Per-user, separate bucket so circle ops do not
    /// starve auth or leaderboard. Default 30/min in production; tests use
    /// `RateLimiter::disabled()`.
    pub community_rate_limiter: super::rate_limit::RateLimiter,
    /// Rate limiter for storage upload endpoints (avatar, KYC, asset-doc,
    /// asset-image, post-image, developer-logo). Every upload writes to
    /// GCS (cost) + DB, so the cap is tighter than browsing limiters:
    /// 10 per minute per (user, endpoint-class). A legitimate single
    /// upload sails through; bursts of automated abuse get throttled.
    pub storage_rate_limiter: super::rate_limit::RateLimiter,
    /// In-process cache of the most recent `leaderboard_scores.computed_at`.
    /// Written by `refresh_all_scores`; read by the leaderboard handler so
    /// the public list view does not have to run a `SELECT MAX(computed_at)`
    /// query on every request (audit task C1). `None` means the cache is
    /// cold and the read path should hydrate it from the DB on next miss.
    pub leaderboard_last_refresh:
        std::sync::Arc<tokio::sync::RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// WebAuthn / Passkey instance, shared across handlers.
    pub webauthn: std::sync::Arc<webauthn_rs::Webauthn>,
}

// Implement FromRef so the auth middleware extractors can access PgPool
impl axum::extract::FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

/// Returns the auth domain router.
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page).post(login_submit))
        .route("/signup", get(signup_page).post(signup_submit))
        .route("/2fa", get(totp_verify_page).post(totp_verify_submit))
        .route("/2fa/setup", get(totp_setup_page).post(totp_setup_submit))
        .route("/2fa/step-up", get(step_up_page).post(step_up_verify))
        .route("/logout", get(logout_page).post(logout))
        .route("/google", get(google_redirect))
        .route("/google/callback", get(google_callback))
        .route(
            "/forgot-password",
            get(forgot_password_page).post(forgot_password_submit),
        )
        .route(
            "/reset-password",
            get(reset_password_page).post(reset_password_submit),
        )
        .route("/verify-email", get(verify_email_page))
        .route("/resend-verification", post(resend_verification_submit))
        // Passkey / WebAuthn
        .route("/passkey/login/start", post(passkey_login_start))
        .route("/passkey/login/finish", post(passkey_login_finish))
        // Passkey management (authenticated user — registration + list + delete)
        .route("/passkey/register/start", post(passkey_register_start))
        .route("/passkey/register/finish", post(passkey_register_finish))
        .route("/passkey/list", get(passkey_list))
        .route("/passkey/:id", delete(passkey_delete))
        .with_state(state)
}

// ─── Page Renders ──────────────────────────────────────────────

/// GET /auth/login – Render the login page.
pub async fn login_page(
    State(state): State<AppState>,
    Extension(csrf_token): Extension<super::csrf::CsrfToken>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    // If already logged in, skip login page and go to marketplace
    if middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/marketplace").into_response();
    }

    let error = params.get("error").cloned();
    let slug = params
        .get("returnTo")
        .and_then(|r| r.strip_prefix("/p/"))
        .map(|s| s.to_string());
    render_login(&state, error, csrf_token.0, slug).await
}

/// GET /auth/signup – Render the signup page.
pub async fn signup_page(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let error = params.get("error").cloned();
    let slug = params
        .get("returnTo")
        .and_then(|r| r.strip_prefix("/p/"))
        .map(|s| s.to_string());
    render_signup(&state, &jar, error, slug).await
}

/// GET /auth/forgot-password – Render the forgot password page.
pub async fn forgot_password_page(State(state): State<AppState>) -> impl IntoResponse {
    match state.templates.get_template("forgot-password.html") {
        Ok(t) => match t.render(minijinja::context! {}) {
            Ok(c) => Html(c).into_response(),
            Err(_) => Redirect::to("/auth/login").into_response(),
        },
        Err(_) => Redirect::to("/auth/login").into_response(),
    }
}

/// GET /auth/reset-password – Render the reset password page.
pub async fn reset_password_page(State(state): State<AppState>) -> impl IntoResponse {
    match state.templates.get_template("reset-password.html") {
        Ok(t) => match t.render(minijinja::context! {}) {
            Ok(c) => Html(c).into_response(),
            Err(_) => Redirect::to("/auth/login").into_response(),
        },
        Err(_) => Redirect::to("/auth/login").into_response(),
    }
}

/// GET /auth/verify-email – Render the email verification page.
pub async fn verify_email_page(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(token) = params.get("token").filter(|value| !value.trim().is_empty()) {
        return match service::verify_email(&state.db, token).await {
            Ok(()) => Redirect::to("/auth/verify-email?verified=1").into_response(),
            Err(err) => {
                tracing::warn!("Email verification failed: {}", err);
                Redirect::to("/auth/verify-email?error=invalid_token").into_response()
            }
        };
    }

    let (status, title, message) = if params.get("verified").is_some_and(|v| v == "1") {
        (
            "success",
            "Email verified",
            "Your email address is verified. You can continue to POOOL.",
        )
    } else if params.get("error").is_some_and(|v| v == "invalid_token") {
        (
            "error",
            "Verification link expired",
            "This verification link is invalid or expired. Request a new link and try again.",
        )
    } else if let Some(error) = params.get("error").filter(|value| !value.trim().is_empty()) {
        ("error", "Unable to resend email", error.as_str())
    } else {
        (
            "pending",
            "Check your email",
            "We sent a verification link to your email address. Please click the link to verify your account.",
        )
    };

    render_verify_email(&state, status, title, message)
}

// ─── Form Handlers (HTMX) ─────────────────────────────────────

/// POST /auth/login – Handle login form submission via HTMX.
pub async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    // Rate limiting — check before doing expensive Argon2 work. Use the
    // trusted-proxy resolver so spoofed X-Forwarded-For headers cannot be
    // used to carve out unshared buckets.
    let client_ip = crate::common::net::client_ip(&headers);

    // Audit M#9: dual-tier rate-limit via ONE atomic check. Two
    // separate `.check()` calls had a TOCTOU gap during which an
    // attacker rotating IPs could burst the IP bucket without
    // tripping the email bucket. `check_dual` is all-or-nothing.
    let email_key = form.email.trim().to_lowercase();
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check_dual(
            &format!("login:{}", client_ip),
            &format!("login:email:{}", email_key),
        )
        .await
    {
        tracing::warn!(
            client_ip = %client_ip,
            "Rate limit exceeded on login (IP or email bucket)"
        );
        return Ok(login_error_response(
            AppError::RateLimited(retry_after),
            &headers,
        ));
    }

    // 1. Authenticate user (password check)
    let user = match service::authenticate_user(&state.db, &form.email, &form.password).await {
        Ok(user) => user,
        Err(err) => return Ok(login_error_response(err, &headers)),
    };

    // 2. Login-time 2FA challenge is temporarily disabled. Existing 2FA
    // enrollment, setup, settings, and step-up routes remain available.
    let (is_2fa_verified, redirect_to) = (true, "/marketplace");

    // Extract client info for session
    let ip = match crate::common::net::client_ip(&headers).as_str() {
        "unknown" => None,
        s => Some(s.to_string()),
    };

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 3. Create session. Mark it 2FA-verified so enrolled accounts can log
    // in without the temporary login-time challenge.
    let session_token = timeout(
        Duration::from_secs(5),
        service::create_session(
            &state.db,
            user.id,
            form.remember_me(),
            is_2fa_verified,
            ip.as_deref(),
            user_agent.as_deref(),
        ),
    )
    .await
    .map_err(|_| AppError::Internal("Login session creation timed out.".to_string()))??;

    spawn_login_side_effects(
        state.db.clone(),
        state.community_db.clone(),
        user.id,
        ip.clone(),
        user_agent.clone(),
    );

    // Set session cookie
    let max_age_secs = if form.remember_me() {
        30 * 24 * 60 * 60 // 30 days
    } else {
        24 * 60 * 60 // 24 hours
    };

    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(max_age_secs));

    // Rotate CSRF on login — drop pre-auth token so next request mints a
    // fresh one bound to the new session (defends against fixation).
    let jar = jar.add(cookie).add(super::csrf::rotation_cookie());

    if is_htmx_request(&headers) {
        let mut response_headers = HeaderMap::new();
        response_headers.insert("HX-Redirect", redirect_to.parse().unwrap());
        Ok((jar, response_headers, Html("")).into_response())
    } else {
        Ok((jar, Redirect::to(redirect_to)).into_response())
    }
}

fn spawn_login_side_effects(
    db: sqlx::PgPool,
    community_db: Option<sqlx::PgPool>,
    user_id: uuid::Uuid,
    ip: Option<String>,
    user_agent: Option<String>,
) {
    tokio::spawn(async move {
        match timeout(
            Duration::from_secs(2),
            crate::common::audit::log(
                &db,
                Some(user_id),
                "user.login",
                "user",
                Some(user_id),
                ip.as_deref(),
                user_agent.as_deref(),
            ),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                tracing::warn!(user_id = %user_id, error = %err, "Login audit log failed")
            }
            Err(_) => tracing::warn!(user_id = %user_id, "Login audit log timed out"),
        }

        if let Some(c_pool) = community_db {
            match timeout(
                Duration::from_secs(2),
                crate::community::xp::track_login_streak(&c_pool, user_id),
            )
            .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(err)) => {
                    tracing::warn!(user_id = %user_id, error = %err, "Login XP streak update failed")
                }
                Err(_) => tracing::warn!(user_id = %user_id, "Login XP streak update timed out"),
            }
        }
    });
}

// ─── 2FA Routes ───────────────────────────────────────────────

/// GET /auth/2fa – Render 2FA verification page.
pub async fn totp_verify_page(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session_unverified(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    let tmpl = state
        .templates
        .get_template("auth-2fa.html")
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    let html = tmpl
        .render(context! { email => user.email })
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    Ok(Html(html).into_response())
}

/// POST /auth/2fa – Verify TOTP code.
pub async fn totp_verify_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Form(form): Form<super::models::TotpForm>,
) -> Result<Response, AppError> {
    let session_token = match jar.get(SESSION_COOKIE) {
        Some(cookie) => cookie.value().to_string(),
        None => {
            return Ok(auth_form_error_response(
                AppError::Unauthorized("Session expired. Please log in again.".to_string()),
                &headers,
                "/auth/login",
            ));
        }
    };

    let user = match service::get_user_by_session_unverified(&state.db, &session_token).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(auth_form_error_response(
                AppError::Unauthorized("Session expired. Please log in again.".to_string()),
                &headers,
                "/auth/login",
            ));
        }
        Err(error) => return Ok(auth_form_error_response(error, &headers, "/auth/2fa")),
    };

    let client_ip = crate::common::net::client_ip(&headers);
    // Audit M#9: atomic dual-tier check (IP × user_id).
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check_dual(
            &format!("2fa:ip:{}", client_ip),
            &format!("2fa:user:{}", user.id),
        )
        .await
    {
        tracing::warn!(
            client_ip = %client_ip,
            user_id = %user.id,
            "Rate limit exceeded on 2FA verification (IP or user bucket)"
        );
        return Ok(auth_form_error_response(
            AppError::RateLimited(retry_after),
            &headers,
            "/auth/2fa",
        ));
    }

    let settings = match service::get_user_settings(&state.db, user.id).await {
        Ok(settings) => settings,
        Err(error) => return Ok(auth_form_error_response(error, &headers, "/auth/2fa")),
    };
    let Some(secret) = settings.totp_secret else {
        tracing::warn!(user_id = %user.id, "2FA verification attempted without configured TOTP secret");
        return Ok(auth_form_error_response(
            AppError::BadRequest(
                "Two-factor authentication is not configured for this account.".to_string(),
            ),
            &headers,
            "/auth/2fa",
        ));
    };
    let secret = match service::decrypt_stored_totp_secret(&secret) {
        Ok(secret) => secret,
        Err(error) => {
            tracing::error!(user_id = %user.id, error = %error, "Failed to decrypt TOTP secret during 2FA verification");
            return Ok(auth_form_error_response(error, &headers, "/auth/2fa"));
        }
    };

    if !service::verify_totp_code_with_replay_guard(
        state.redis.as_ref(),
        user.id,
        &secret,
        &form.code,
    )
    .await
    {
        tracing::warn!("Invalid 2FA code submitted for user {}", user.id);
        return Ok(auth_form_error_response(
            AppError::Unauthorized("Invalid authentication code.".to_string()),
            &headers,
            "/auth/2fa",
        ));
    }

    // Rotate the session token on privilege elevation — a token captured
    // before 2FA cannot be replayed post-verification.
    let new_token = match service::rotate_session_token(&state.db, &session_token).await {
        Ok(token) => token,
        Err(error) => return Ok(auth_form_error_response(error, &headers, "/auth/2fa")),
    };

    let cookie = Cookie::build((SESSION_COOKIE, new_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax);
    let jar = jar.add(cookie).add(super::csrf::rotation_cookie());

    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", HeaderValue::from_static("/marketplace"));

    Ok((jar, response_headers, Html("")).into_response())
}

/// GET /auth/2fa/setup – Render 2FA setup page (for admins or voluntary).
pub async fn totp_setup_page(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session_unverified(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    // If TOTP is already enrolled, sending the user to settings is a
    // dead-end: they hit setup because something needed step-up auth, not
    // because they wanted to re-enroll. Forward them to the step-up page
    // (preserves return_to) so they can verify and continue.
    if service::user_totp_enabled(&state.db, user.id).await? {
        let return_to = params.get("return_to").cloned().unwrap_or_default();
        let url = if return_to.is_empty() {
            "/auth/2fa/step-up".to_string()
        } else {
            let encoded: String = return_to
                .bytes()
                .flat_map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        vec![b]
                    }
                    _ => format!("%{:02X}", b).into_bytes(),
                })
                .map(|b| b as char)
                .collect();
            format!("/auth/2fa/step-up?return_to={}", encoded)
        };
        return Ok(Redirect::to(&url).into_response());
    }

    let (secret, url, qr_code) = service::generate_totp_secret(&user.email)?;
    let setup_token = service::build_totp_setup_token(user.id, &secret)?;

    let tmpl = state
        .templates
        .get_template("auth-2fa-setup.html")
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    let html = tmpl
        .render(context! {
            email => user.email,
            secret => secret,
            setup_token => setup_token,
            url => url,
            qr_code => qr_code
        })
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    Ok(Html(html).into_response())
}

/// POST /auth/2fa/setup – Verify first TOTP code and enable 2FA.
pub async fn totp_setup_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Form(form): Form<super::models::TotpSetupForm>,
) -> Result<Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session_unverified(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    let client_ip = crate::common::net::client_ip(&headers);
    // Audit M#9: atomic dual-tier check (IP × user_id).
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check_dual(
            &format!("2fa_setup:ip:{}", client_ip),
            &format!("2fa_setup:user:{}", user.id),
        )
        .await
    {
        tracing::warn!(
            client_ip = %client_ip,
            user_id = %user.id,
            "Rate limit exceeded on 2FA setup (IP or user bucket)"
        );
        return Ok(auth_form_error_response(
            AppError::RateLimited(retry_after),
            &headers,
            "/auth/2fa/setup",
        ));
    }

    if service::user_totp_enabled(&state.db, user.id).await? {
        return Ok(auth_form_error_response(
            AppError::Forbidden(
                "Two-factor authentication is already enabled. Disable it from Settings before enrolling again."
                    .to_string(),
            ),
            &headers,
            "/auth/2fa/setup",
        ));
    }

    let setup_secret = match service::read_totp_setup_token(&form.setup_token, user.id) {
        Ok(secret) => secret,
        Err(error) => return Ok(auth_form_error_response(error, &headers, "/auth/2fa/setup")),
    };

    // Verify first code
    if !service::verify_totp_code_with_replay_guard(
        state.redis.as_ref(),
        user.id,
        &setup_secret,
        &form.code,
    )
    .await
    {
        return Ok(auth_form_error_response(
            AppError::BadRequest(
                "Invalid authentication code. Please check your authenticator app.".to_string(),
            ),
            &headers,
            "/auth/2fa/setup",
        ));
    }

    // Enable in DB
    service::enable_totp(&state.db, user.id, &setup_secret).await?;

    // Rotate session token on 2FA enrollment (privilege change).
    let new_token = service::rotate_session_token(&state.db, session_token).await?;

    let cookie = Cookie::build((SESSION_COOKIE, new_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax);
    let jar = jar.add(cookie).add(super::csrf::rotation_cookie());

    let success_html = r#"
        <div class="auth-success-message" role="status" aria-live="polite" tabindex="-1" style="
            margin:0 0 14px;
            padding:12px 14px;
            background:#ECFDF3;
            border:1px solid #ABEFC6;
            border-radius:8px;
            color:#027A48;
            font-size:13px;
            font-weight:600;
            line-height:1.45;
        ">
            Two-factor authentication connected successfully.
            <a href="/marketplace" style="color:#027A48; font-weight:700;">Continue to POOOL</a>
        </div>
    "#;

    Ok((jar, Html(success_html)).into_response())
}

/// GET /auth/2fa/step-up – Render TOTP step-up verification page.
///
/// Used when an existing TOTP-enrolled user needs to re-verify before a
/// high-value financial operation (trade ≥ $500, withdrawal, etc.). Form
/// JS POSTs to the same path (JSON API) and on success redirects to the
/// `return_to` query param.
pub async fn step_up_page(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    // If TOTP isn't enrolled yet, redirect to setup instead.
    if !service::user_totp_enabled(&state.db, user.id).await? {
        // Pass return_to through unmodified — Query gives us the
        // already-decoded value, and Redirect::to escapes as needed.
        let return_to = params.get("return_to").cloned().unwrap_or_default();
        let url = if return_to.is_empty() {
            "/auth/2fa/setup".to_string()
        } else {
            // Re-encode with a tiny inline helper (no extra crate needed).
            let encoded: String = return_to
                .bytes()
                .flat_map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        vec![b]
                    }
                    _ => format!("%{:02X}", b).into_bytes(),
                })
                .map(|b| b as char)
                .collect();
            format!("/auth/2fa/setup?return_to={}", encoded)
        };
        return Ok(Redirect::to(&url).into_response());
    }

    let return_to = params
        .get("return_to")
        .cloned()
        .unwrap_or_else(|| "/".to_string());
    let action = params
        .get("action")
        .cloned()
        .unwrap_or_else(|| "trade".to_string());

    let tmpl = state
        .templates
        .get_template("auth-2fa-step-up.html")
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    let html = tmpl
        .render(context! {
            email => user.email,
            return_to => return_to,
            action => action,
        })
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    Ok(Html(html).into_response())
}

/// POST /auth/2fa/step-up – Verify TOTP code for step-up 2FA (JSON API).
///
/// Called by frontend modals when a financial operation requires re-authentication.
/// Creates a 15-minute trading session in Redis on success.
pub async fn step_up_verify(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::Json(form): axum::Json<super::models::StepUpVerifyForm>,
) -> Result<axum::response::Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    // Parse the action string
    let action = match form.action.as_str() {
        "withdrawal" => super::step_up::FinancialAction::Withdrawal,
        "trade" => super::step_up::FinancialAction::Trade,
        "payment_method" => super::step_up::FinancialAction::PaymentMethodAdd,
        "password_change" => super::step_up::FinancialAction::PasswordChange,
        _ => return Err(AppError::BadRequest("Invalid action type.".to_string())),
    };

    // Verify TOTP and create trading session
    super::step_up::verify_and_create_trading_session(
        &state.db,
        state.redis.as_ref(),
        user.id,
        &form.code,
        action,
    )
    .await?;

    Ok(axum::Json(serde_json::json!({
        "success": true,
        "message": "Two-factor authentication verified."
    }))
    .into_response())
}

/// POST /auth/signup – Handle signup form submission via HTMX.
pub async fn signup_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Form(form): Form<SignupForm>,
) -> Result<Response, AppError> {
    // Rate limiting — prevent mass account creation
    let client_ip = crate::common::net::client_ip(&headers);

    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("signup:{}", client_ip))
        .await
    {
        tracing::warn!("Rate limit exceeded for signup from IP: {}", client_ip);
        return Ok(signup_error_response(
            AppError::RateLimited(retry_after),
            &headers,
        ));
    }

    // ── Terms acceptance guard ──────────────────────────────────
    if !form.terms_accepted() {
        return Ok(signup_error_response(
            AppError::BadRequest(
                "You must accept the Terms and Conditions and Privacy Policy to create an account."
                    .to_string(),
            ),
            &headers,
        ));
    }

    // Determine the referral code to use (form priority, fallback to cookie)
    let referral_code = form.referral_code.clone().or_else(|| {
        jar.get(super::middleware::REFERRAL_COOKIE)
            .map(|c| c.value().to_string())
    });

    // Extract client info
    let ip = match crate::common::net::client_ip(&headers).as_str() {
        "unknown" => None,
        s => Some(s.to_string()),
    };

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ── Record terms consent ────────────────────────────────────
    // Fetch current platform terms version (defaults to "1.0" if not set)
    let terms_version: String =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = 'legal_terms_version'")
            .fetch_optional(&state.db)
            .await?
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

    let (user, verification_token) = match service::register_user_with_consent_and_verification(
        &state.db,
        &form.email,
        &form.password,
        &terms_version,
        ip.as_deref(),
        user_agent.as_deref(),
    )
    .await
    {
        Ok(result) => result,
        Err(err) => return Ok(signup_error_response(err, &headers)),
    };

    // Queue verification email via durable outbox so transient Resend failures
    // are retried without losing the token or blocking signup completion.
    {
        let db = state.db.clone();
        let email_addr = user.email.clone();
        let base = state.config.base_url.clone();
        let uid = user.id;
        let tok = verification_token.clone();
        tokio::spawn(async move {
            let body = format!(
                r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Welcome to POOOL!</h2>
  <p>Please click the link below to verify your email address.</p>
  <p><a href="{base}/auth/verify-email?token={tok}" style="display:inline-block;padding:12px 24px;background:#3D00F5;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Verify Email</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Link expires in 24 hours. If you didn't create an account, ignore this email.</p>
</div>"#
            );
            let outbox_id = sqlx::query_scalar::<_, uuid::Uuid>(
                r#"INSERT INTO transactional_email_outbox
                       (user_id, event_type, recipient_email, subject, html_body)
                   VALUES ($1, 'verify_email', $2, 'Verify your POOOL email', $3)
                   RETURNING id"#,
            )
            .bind(uid)
            .bind(&email_addr)
            .bind(&body)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten();
            if let Some(id) = outbox_id {
                crate::common::email::send_transactional_outbox_item(&db, id).await;
            }
        });
    }

    // ── Referral Attribution (Affiliate-only) ────────────────────
    // The legacy `referral_tracking` parallel path was removed (audit GAP-07,
    // see migration 155). Only the new affiliate attribution writes here.
    // Existing `referral_tracking` rows remain read-only for historical
    // dashboards and the double-payout guard in check_and_track_affiliate_commission.
    if let Some(mut code_str) = referral_code.filter(|c| !c.trim().is_empty()) {
        code_str = code_str.trim().to_string();

        let mut parts = code_str.split('|');
        let just_code = parts.next().unwrap_or("").to_string();
        let subid = parts.next().filter(|s| !s.is_empty()).map(String::from);
        let utm_source = parts.next().filter(|s| !s.is_empty()).map(String::from);

        code_str = just_code;

        if let Err(e) = crate::rewards::service::attribute_affiliate_referral(
            &state.db,
            &code_str,
            user.id,
            subid,
            utm_source,
            ip.clone(),
        )
        .await
        {
            tracing::error!(
                "Failed to attribute affiliate referral for code {}: {}",
                code_str,
                e
            );
        }
    }

    // Create session
    let session_token = service::create_session(
        &state.db,
        user.id,
        false,
        false,
        ip.as_deref(),
        user_agent.as_deref(),
    )
    .await?;

    // Audit log
    crate::common::audit::log(
        &state.db,
        Some(user.id),
        "user.registered",
        "user",
        Some(user.id),
        ip.as_deref(),
        user_agent.as_deref(),
    )
    .await
    .ok();

    // Set a short session cookie for resend/verification UX. Normal
    // authenticated access still requires users.email_verified = TRUE.
    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(24 * 60 * 60));

    // Also clear referral cookie if it was used
    let jar = jar
        .remove(Cookie::from(super::middleware::REFERRAL_COOKIE))
        .add(cookie)
        .add(super::csrf::rotation_cookie());

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "HX-Redirect",
        HeaderValue::from_static("/auth/verify-email?sent=1"),
    );

    Ok((jar, response_headers, Html("")).into_response())
}

// ─── Password Reset & Verification (HTMX) ─────────────────────

/// POST /auth/forgot-password
pub async fn forgot_password_submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<super::models::ForgotPasswordForm>,
) -> Result<Response, AppError> {
    let started_at = Instant::now();

    if let Err(error) = validation::validate_email(&form.email) {
        wait_for_password_reset_response_floor(started_at).await;
        return Ok(auth_form_error_response(
            error,
            &headers,
            "/auth/forgot-password",
        ));
    }

    if state.config.app_env.eq_ignore_ascii_case("production") && !email::resend_configured() {
        tracing::error!(
            "Password reset requested while RESEND_API_KEY is not configured in production"
        );
        wait_for_password_reset_response_floor(started_at).await;
        return Ok(auth_form_error_response(
            AppError::ServiceUnavailable("Password reset email is not configured".to_string()),
            &headers,
            "/auth/forgot-password",
        ));
    }

    // Rate limiting — prevent email bombing. Audit M#9: atomic
    // dual-tier (IP × email) so an attacker rotating IPs can't burst
    // through the IP bucket without tripping the email enumeration cap.
    let client_ip = crate::common::net::client_ip(&headers);
    let email_key = form.email.trim().to_lowercase();
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check_dual(
            &format!("forgot:{}", client_ip),
            &format!("forgot:email:{}", email_key),
        )
        .await
    {
        tracing::warn!(
            client_ip = %client_ip,
            "Rate limit exceeded on forgot-password (IP or email bucket)"
        );
        wait_for_password_reset_response_floor(started_at).await;
        return Ok(auth_form_error_response(
            AppError::RateLimited(retry_after),
            &headers,
            "/auth/forgot-password",
        ));
    }

    if let Err(error) =
        service::create_password_reset_token(&state.db, &form.email, &state.config.base_url).await
    {
        wait_for_password_reset_response_floor(started_at).await;
        return Ok(auth_form_error_response(
            error,
            &headers,
            "/auth/forgot-password",
        ));
    }

    wait_for_password_reset_response_floor(started_at).await;

    let html = r##"
        <div id="forgot-password-success" role="status" aria-live="polite" tabindex="-1" style="text-align: center; padding: 20px;">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" style="margin-bottom: 16px; color: #12B76A;">
                <path d="M12 22C17.5 22 22 17.5 22 12C22 6.5 17.5 2 12 2C6.5 2 2 6.5 2 12C2 17.5 6.5 22 12 22Z" fill="#D1FADF"/>
                <path d="M7.75 12L10.58 14.83L16.25 9.17" stroke="#039855" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <h3 style="color: #101828; font-size: 18px; margin-bottom: 8px;">Check your email</h3>
            <p style="color: #475467; font-size: 14px;">If an active account exists for that address, we've sent password reset instructions.</p>
        </div>
    "##;

    Ok(Html(html).into_response())
}

/// POST /auth/reset-password
pub async fn reset_password_submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<super::models::ResetPasswordForm>,
) -> Result<Response, AppError> {
    let client_ip = crate::common::net::client_ip(&headers);
    let token_key = if form.token.trim().is_empty() {
        "missing".to_string()
    } else {
        crate::config::hash_token(form.token.trim())
    };
    // Audit M#9: atomic dual-tier (IP × token).
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check_dual(
            &format!("reset:{}", client_ip),
            &format!("reset:token:{}", token_key),
        )
        .await
    {
        tracing::warn!(
            client_ip = %client_ip,
            "Rate limit exceeded on reset-password (IP or token bucket)"
        );
        return Ok(auth_form_error_response(
            AppError::RateLimited(retry_after),
            &headers,
            "/auth/reset-password",
        ));
    }

    if form.password != form.confirm_password {
        return Ok(auth_form_error_response(
            AppError::BadRequest("Passwords do not match.".to_string()),
            &headers,
            "/auth/reset-password",
        ));
    }
    if form.token.trim().is_empty() {
        return Ok(auth_form_error_response(
            AppError::BadRequest(
                "This reset link is missing or expired. Please request a new password reset email."
                    .to_string(),
            ),
            &headers,
            "/auth/reset-password",
        ));
    }

    if let Err(err) = service::reset_password(&state.db, form.token.trim(), &form.password).await {
        return Ok(auth_form_error_response(
            err,
            &headers,
            "/auth/reset-password",
        ));
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", HeaderValue::from_static("/auth/login"));

    Ok((response_headers, Html("")).into_response())
}

/// POST /auth/resend-verification
pub async fn resend_verification_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let client_ip = crate::common::net::client_ip(&headers);
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("resend_verification:ip:{}", client_ip))
        .await
    {
        tracing::warn!(
            "Rate limit exceeded for email verification resend from IP: {}",
            client_ip
        );
        return Ok(auth_form_error_response(
            AppError::RateLimited(retry_after),
            &headers,
            "/auth/verify-email",
        ));
    }

    let user = match jar.get(SESSION_COOKIE) {
        Some(cookie) => service::get_user_by_session_unverified(&state.db, cookie.value())
            .await?
            .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?,
        None => {
            return Ok(auth_form_error_response(
                AppError::Unauthorized(
                    "Your session expired. Please sign in to resend the verification email."
                        .to_string(),
                ),
                &headers,
                "/auth/verify-email",
            ));
        }
    };

    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("resend_verification:user:{}", user.id))
        .await
    {
        tracing::warn!(
            "Rate limit exceeded for email verification resend for user: {}",
            user.id
        );
        return Ok(auth_form_error_response(
            AppError::RateLimited(retry_after),
            &headers,
            "/auth/verify-email",
        ));
    }

    if user.email_verified {
        return Ok(Html(
            r#"<div class="auth-success-message" role="status" aria-live="polite" tabindex="-1">Your email is already verified.</div>"#
                .to_string(),
        )
        .into_response());
    }

    if let Err(error) = service::create_email_verification_token(
        &state.db,
        user.id,
        &user.email,
        &state.config.base_url,
    )
    .await
    {
        tracing::warn!(
            "Failed to resend verification email for user {}: {}",
            user.id,
            error
        );
        return Ok(auth_form_error_response(
            error,
            &headers,
            "/auth/verify-email",
        ));
    }

    let html = r#"
        <div class="auth-success-message" role="status" aria-live="polite" tabindex="-1">
            Verification email resent successfully.
        </div>
    "#;

    Ok(Html(html).into_response())
}

// ─── Logout ────────────────────────────────────────────────────

/// GET /logout – Compatibility page that submits the CSRF-protected logout POST.
pub async fn logout_page(
    Extension(csrf_token): Extension<super::csrf::CsrfToken>,
) -> impl IntoResponse {
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Signing out | POOOL</title>
</head>
<body>
  <form id="logout-form" method="post" action="/auth/logout">
    <input type="hidden" name="csrf_token" value="{csrf_token}">
    <button type="submit">Sign out</button>
  </form>
  <script>document.getElementById("logout-form").submit();</script>
</body>
</html>"#,
        csrf_token = csrf_token.0
    ))
    .into_response()
}

fn expired_session_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build()
}

/// POST /logout – Destroy session and redirect to login.
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    // Delete session from DB. The cookie is expired regardless, but a failed
    // DB delete means the server-side session may remain valid elsewhere.
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        if let Err(error) = service::delete_session(&state.db, cookie.value()).await {
            tracing::error!("Failed to delete session during logout: {}", error);
        }
    }

    // Clear cookies — session and CSRF both rotate on logout so the next
    // authenticated session starts with fresh tokens.
    let jar = jar
        .add(expired_session_cookie())
        .add(super::csrf::rotation_cookie());

    (jar, Redirect::to("/auth/login"))
}

// ─── OAuth Routes ──────────────────────────────────────────────

/// GET /auth/google – Redirect to Google OAuth consent screen.
///
/// Generates random `state` (CSRF) and PKCE `code_verifier`; stores both in
/// short-lived HttpOnly cookies for verification on callback.
pub async fn google_redirect(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    use base64::Engine;
    use rand::RngCore;
    use sha2::Digest;

    let is_link_flow = params.get("link").is_some_and(|v| v == "1" || v == "true");
    if middleware::is_authenticated(&jar, &state.db).await && !is_link_flow {
        return Redirect::to("/marketplace").into_response();
    }

    if !state.config.google_oauth_enabled() {
        return Redirect::to("/auth/login?error=oauth_not_configured").into_response();
    }

    let client_id = match state.config.google_client_id.as_ref() {
        Some(id) => id,
        None => return Redirect::to("/auth/login?error=oauth_not_configured").into_response(),
    };
    let redirect_uri = google_oauth_redirect_uri(&state.config, &headers);

    // CSRF state — 32 random bytes, base64url
    let mut state_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let state_token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(state_bytes);

    // PKCE — 32 random bytes verifier, S256 challenge
    let mut verifier_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);
    let mut hasher = sha2::Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=email%20profile&access_type=offline&state={}&code_challenge={}&code_challenge_method=S256",
        client_id,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state_token),
        urlencoding::encode(&code_challenge),
    );

    let state_cookie = Cookie::build(("oauth_state", state_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::minutes(10));
    let verifier_cookie = Cookie::build(("oauth_pkce", code_verifier))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::minutes(10));
    let redirect_cookie = Cookie::build(("oauth_redirect_uri", redirect_uri.clone()))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::minutes(10));

    let jar = jar
        .add(state_cookie)
        .add(verifier_cookie)
        .add(redirect_cookie);
    let jar = if is_link_flow {
        jar.add(
            Cookie::build(("oauth_link", "1"))
                .path("/")
                .http_only(true)
                .secure(cookie_is_secure())
                .same_site(axum_extra::extract::cookie::SameSite::Lax)
                .max_age(time::Duration::minutes(10)),
        )
    } else {
        jar
    };
    (jar, Redirect::to(&auth_url)).into_response()
}

/// GET /auth/google/callback – Handle Google OAuth callback.
pub async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let redirect_uri = jar
        .get("oauth_redirect_uri")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| google_oauth_redirect_uri(&state.config, &headers));

    match google_callback_inner(&state, jar.clone(), params, redirect_uri).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Google OAuth callback error: {}", e);
            let jar = clear_oauth_cookies(jar);
            let error_msg = match &e {
                crate::error::AppError::Unauthorized(msg)
                | crate::error::AppError::BadRequest(msg) => {
                    urlencoding::encode(msg).replace("%20", "+")
                }
                _ => "Google+sign+in+failed.+Please+try+again.".to_string(),
            };
            (
                jar,
                Redirect::to(&format!("/auth/login?error={}", error_msg)),
            )
                .into_response()
        }
    }
}

fn clear_oauth_cookies(jar: CookieJar) -> CookieJar {
    jar.remove(Cookie::from("oauth_state"))
        .remove(Cookie::from("oauth_pkce"))
        .remove(Cookie::from("oauth_link"))
        .remove(Cookie::from("oauth_redirect_uri"))
}

async fn google_callback_inner(
    state: &AppState,
    jar: CookieJar,
    params: std::collections::HashMap<String, String>,
    redirect_uri: String,
) -> Result<Response, AppError> {
    let code = params
        .get("code")
        .ok_or_else(|| AppError::BadRequest("Missing authorization code.".to_string()))?;

    // Verify CSRF state cookie matches query param
    let state_cookie = jar
        .get("oauth_state")
        .ok_or_else(|| AppError::Unauthorized("Missing OAuth state cookie".to_string()))?;
    let state_param = params
        .get("state")
        .ok_or_else(|| AppError::Unauthorized("Missing OAuth state param".to_string()))?;
    if state_cookie.value() != state_param {
        return Err(AppError::Unauthorized(
            "OAuth state mismatch (possible CSRF)".to_string(),
        ));
    }

    // Pull PKCE verifier
    let pkce_cookie = jar
        .get("oauth_pkce")
        .ok_or_else(|| AppError::Unauthorized("Missing OAuth PKCE cookie".to_string()))?;
    let code_verifier = pkce_cookie.value().to_string();

    let client_id = state
        .config
        .google_client_id
        .as_ref()
        .ok_or_else(|| AppError::Internal("Google OAuth not configured".to_string()))?;
    let client_secret = state
        .config
        .google_client_secret
        .as_ref()
        .ok_or_else(|| AppError::Internal("Google OAuth not configured".to_string()))?;
    // Exchange code for access token (with PKCE verifier)
    let client = reqwest::Client::new();
    let token_response = client
        .post(&state.config.google_oauth_token_url)
        .form(&[
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Google token exchange failed: {}", e)))?;

    let token_data: serde_json::Value = token_response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse Google response: {}", e)))?;

    // Check for error in Google response
    if let Some(error) = token_data.get("error") {
        let error_desc = token_data
            .get("error_description")
            .and_then(|d| d.as_str())
            .unwrap_or("unknown");
        tracing::error!("Google OAuth token error: {} — {}", error, error_desc);
        return Err(AppError::Internal(format!(
            "Google OAuth failed: {} — {}",
            error, error_desc
        )));
    }

    let access_token = token_data["access_token"].as_str().ok_or_else(|| {
        let response_keys = token_data
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        tracing::error!(
            response_keys = ?response_keys,
            "Google OAuth token response missing access_token"
        );
        AppError::Internal("No access token in Google response".to_string())
    })?;

    // Fetch user info
    let user_info: serde_json::Value = client
        .get(&state.config.google_oauth_userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Google user info failed: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse Google user info: {}", e)))?;

    let google_id = user_info["id"]
        .as_str()
        .ok_or_else(|| AppError::Internal("No ID in Google user info".to_string()))?;
    let email = user_info["email"]
        .as_str()
        .ok_or_else(|| AppError::Internal("No email in Google user info".to_string()))?;
    let email_verified = user_info["verified_email"].as_bool().unwrap_or(false);
    if !email_verified {
        return Err(AppError::Unauthorized(
            "Google account email is not verified".to_string(),
        ));
    }
    let first_name = user_info["given_name"].as_str().map(|s| s.to_string());
    let last_name = user_info["family_name"].as_str().map(|s| s.to_string());
    let avatar_url = user_info["picture"].as_str().map(|s| s.to_string());

    if jar.get("oauth_link").is_some() {
        let current_user = middleware::get_current_user(&jar, &state.db)
            .await
            .ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))?;

        let existing_user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT user_id FROM oauth_accounts WHERE provider = 'google' AND provider_id = $1",
        )
        .bind(google_id)
        .fetch_optional(&state.db)
        .await?;
        if let Some(existing_user_id) = existing_user_id {
            if existing_user_id != current_user.id {
                return Err(AppError::Conflict(
                    "This Google account is already linked to another POOOL account.".to_string(),
                ));
            }
        }

        sqlx::query(
            r#"INSERT INTO oauth_accounts (user_id, provider, provider_id, provider_email)
               VALUES ($1, 'google', $2, $3)
               ON CONFLICT (provider, provider_id) DO UPDATE
               SET provider_email = EXCLUDED.provider_email"#,
        )
        .bind(current_user.id)
        .bind(google_id)
        .bind(email)
        .execute(&state.db)
        .await?;

        let jar = clear_oauth_cookies(jar);

        return Ok((jar, Redirect::to("/settings#sec-security")).into_response());
    }

    // Find or create user — email_verified=true enforced above
    let user = service::oauth_find_or_create_user(
        &state.db,
        "google",
        google_id,
        email,
        first_name.as_deref(),
        last_name.as_deref(),
        avatar_url.as_deref(),
    )
    .await?;

    // Apply same temporary login-time 2FA bypass as password login. Existing
    // enrollment, setup, settings, and step-up routes remain available.
    let (is_2fa_verified, redirect_to) = (true, "/marketplace");

    // Clear transient OAuth cookies
    let jar = clear_oauth_cookies(jar);

    let session_token =
        service::create_session(&state.db, user.id, true, is_2fa_verified, None, None).await?;

    if let Some(c_pool) = &state.community_db {
        let _ = crate::community::xp::track_login_streak(c_pool, user.id).await;
    }

    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(30 * 24 * 60 * 60));

    let jar = jar.add(cookie);

    Ok((jar, Redirect::to(redirect_to)).into_response())
}

fn google_oauth_redirect_uri(config: &crate::config::Config, headers: &HeaderMap) -> String {
    format!(
        "{}/auth/google/callback",
        effective_public_base_url(config, headers)
    )
}

fn effective_public_base_url(config: &crate::config::Config, headers: &HeaderMap) -> String {
    let configured = config.base_url.trim_end_matches('/').to_string();
    if !is_loopback_base_url(&configured) {
        return configured;
    }

    request_base_url(headers).unwrap_or(configured)
}

fn is_loopback_base_url(url: &str) -> bool {
    url.contains("://localhost")
        || url.contains("://127.0.0.1")
        || url.contains("://[::1]")
        || url.contains("://0.0.0.0")
}

fn request_base_url(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(axum::http::header::HOST))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| valid_host(value))?;

    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| *value == "http" || *value == "https")
        .unwrap_or("https");

    Some(format!("{proto}://{host}"))
}

fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 255
        && !host
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'\\' | b'@' | b';'))
}

// ─── Template helpers ──────────────────────────────────────────

fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers
        .get("HX-Request")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

async fn wait_for_password_reset_response_floor(started_at: Instant) {
    const RESPONSE_FLOOR: Duration = Duration::from_millis(650);

    let elapsed = started_at.elapsed();
    if elapsed < RESPONSE_FLOOR {
        sleep(RESPONSE_FLOOR - elapsed).await;
    }
}

fn auth_form_error_response(error: AppError, headers: &HeaderMap, fallback_path: &str) -> Response {
    let (status, message, retry_after) = match error {
        AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message, None),
        AppError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message, None),
        AppError::Forbidden(message) => (StatusCode::FORBIDDEN, message, None),
        AppError::Conflict(message) => (StatusCode::CONFLICT, message, None),
        AppError::RateLimited(seconds) => (
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Too many attempts. Please try again in {} seconds.",
                seconds
            ),
            Some(seconds),
        ),
        AppError::ServiceUnavailable(message) => {
            tracing::error!("Auth form service unavailable: {}", message);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service temporarily unavailable. Please try again later.".to_string(),
                None,
            )
        }
        AppError::Internal(message) => {
            tracing::error!("Auth form internal error: {}", message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred. Please try again.".to_string(),
                None,
            )
        }
        AppError::Database(err) => {
            tracing::error!("Auth form database error: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred. Please try again.".to_string(),
                None,
            )
        }
        other => {
            tracing::warn!("Unexpected auth form error: {}", other);
            (
                StatusCode::BAD_REQUEST,
                "Unable to process that request. Please try again.".to_string(),
                None,
            )
        }
    };

    if is_htmx_request(headers) {
        let mut response = (StatusCode::OK, Html(render_auth_error_html(&message))).into_response();
        if let Ok(value) = status.as_u16().to_string().parse() {
            response
                .headers_mut()
                .insert("X-POOOL-Auth-Error-Status", value);
        }
        if let Some(seconds) = retry_after {
            if let Ok(value) = seconds.to_string().parse() {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
        }
        return response;
    }

    let encoded_message: String =
        url::form_urlencoded::byte_serialize(message.as_bytes()).collect();
    Redirect::to(&format!("{}?error={}", fallback_path, encoded_message)).into_response()
}

fn login_error_response(error: AppError, headers: &HeaderMap) -> Response {
    auth_error_response(error, headers, "/auth/login")
}

fn signup_error_response(error: AppError, headers: &HeaderMap) -> Response {
    auth_error_response(error, headers, "/auth/signup")
}

fn auth_error_response(error: AppError, headers: &HeaderMap, fallback_path: &str) -> Response {
    let (status, message, retry_after) = match error {
        AppError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message, None),
        AppError::Forbidden(message) => (StatusCode::FORBIDDEN, message, None),
        AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message, None),
        AppError::Conflict(message) => (StatusCode::CONFLICT, message, None),
        AppError::RateLimited(seconds) => (
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Too many attempts. Please try again in {} seconds.",
                seconds
            ),
            Some(seconds),
        ),
        AppError::ServiceUnavailable(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable. Please try again later.".to_string(),
            None,
        ),
        AppError::Internal(message) => {
            tracing::error!("Login internal error: {}", message);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred. Please try again.".to_string(),
                None,
            )
        }
        AppError::Database(err) => {
            tracing::error!("Login database error: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred. Please try again.".to_string(),
                None,
            )
        }
        other => {
            tracing::warn!("Unexpected login error: {}", other);
            (
                StatusCode::BAD_REQUEST,
                "Unable to sign in. Please try again.".to_string(),
                None,
            )
        }
    };

    if is_htmx_request(headers) {
        let mut response = (status, Html(render_auth_error_html(&message))).into_response();
        if let Some(seconds) = retry_after {
            if let Ok(value) = seconds.to_string().parse() {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, value);
            }
        }
        return response;
    }

    let encoded_message: String =
        url::form_urlencoded::byte_serialize(message.as_bytes()).collect();
    Redirect::to(&format!("{}?error={}", fallback_path, encoded_message)).into_response()
}

fn render_auth_error_html(message: &str) -> String {
    let escaped = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");

    format!(
        r#"<div class="auth-error-message" role="alert" aria-live="assertive" tabindex="-1">{}</div>"#,
        escaped
    )
}

fn render_verify_email(state: &AppState, status: &str, title: &str, message: &str) -> Response {
    let tmpl = match state.templates.get_template("verify-email.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load verify-email.html template: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };

    let html = tmpl
        .render(context! {
            status => status,
            heading => title,
            message => message,
        })
        .unwrap_or_else(|e| format!("Template error: {}", e));
    Html(html).into_response()
}

async fn render_login(
    state: &AppState,
    error: Option<String>,
    csrf_token: String,
    property_slug: Option<String>,
) -> Response {
    let tmpl = match state.templates.get_template("login.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load login.html template: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };

    let property = if let Some(ref slug) = property_slug {
        sqlx::query_as!(
            crate::assets::models::MarketplaceAsset,
            r#"
            SELECT
                a.id, a.title, a.slug, a.short_description, a.description,
                a.asset_type, a.location_city, a.location_country,
                a.total_value_cents, a.token_price_cents, a.tokens_total,
                a.tokens_available, a.annual_yield_bps, a.capital_appreciation_bps,
                a.funding_status,
                ARRAY(
                    SELECT image_url FROM asset_images
                    WHERE asset_id = a.id
                    ORDER BY is_cover DESC, sort_order ASC, created_at ASC
                ) AS "image_urls?",
                a.bedrooms, a.bathrooms, a.building_size_sqm, a.lease_type,
                a.term_months, a.area, a.land_size_sqm,
                (
                    SELECT COUNT(DISTINCT user_id)
                    FROM investments
                    WHERE asset_id = a.id AND tokens_owned > 0 AND status != 'exited'
                ) AS "investor_count?",
                COALESCE((
                    SELECT SUM(tokens_owned)::bigint FROM investments
                    WHERE asset_id = a.id AND status != 'exited'
                ), 0) AS "tokens_sold_actual?",
                a.video_url, a.google_maps_url, a.location_description
            FROM assets a
            WHERE a.slug = $1 AND a.published = true AND a.asset_type != 'commodity'
            "#,
            slug.as_str()
        )
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|a| {
            let share_price = format!("{:.0}", a.token_price_cents as f64 / 100.0);
            let d = crate::assets::models::PropertyDisplayData::from_asset(&a);
            (d, share_price)
        })
    } else {
        None
    };

    let (prop_data, share_price) = match property {
        Some((d, p)) => (Some(d), Some(p)),
        None => (None, None),
    };

    let html = tmpl
        .render(context! {
            error => error.unwrap_or_default(),
            csrf_token => csrf_token,
            google_enabled => state.config.google_oauth_enabled(),
            property => prop_data,
            share_price => share_price,
        })
        .unwrap_or_else(|e| format!("Template error: {}", e));
    Html(html).into_response()
}

async fn render_signup(
    state: &AppState,
    jar: &CookieJar,
    error: Option<String>,
    property_slug: Option<String>,
) -> Response {
    let tmpl = match state.templates.get_template("signup.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load signup.html template: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };

    let referral_code = jar
        .get(crate::auth::middleware::REFERRAL_COOKIE)
        .map(|c| c.value().split('|').next().unwrap_or("").to_string());

    let property = if let Some(ref slug) = property_slug {
        sqlx::query_as!(
            crate::assets::models::MarketplaceAsset,
            r#"
            SELECT
                a.id,
                a.title,
                a.slug,
                a.short_description,
                a.description,
                a.asset_type,
                a.location_city,
                a.location_country,
                a.total_value_cents,
                a.token_price_cents,
                a.tokens_total,
                a.tokens_available,
                a.annual_yield_bps,
                a.capital_appreciation_bps,
                a.funding_status,
                ARRAY(
                    SELECT image_url FROM asset_images
                    WHERE asset_id = a.id
                    ORDER BY is_cover DESC, sort_order ASC, created_at ASC
                ) AS "image_urls?",
                a.bedrooms,
                a.bathrooms,
                a.building_size_sqm,
                a.lease_type,
                a.term_months,
                a.area,
                a.land_size_sqm,
                (
                    SELECT COUNT(DISTINCT user_id)
                    FROM investments
                    WHERE asset_id = a.id AND tokens_owned > 0 AND status != 'exited'
                ) AS "investor_count?",
                COALESCE((
                    SELECT SUM(tokens_owned)::bigint FROM investments
                    WHERE asset_id = a.id AND status != 'exited'
                ), 0) AS "tokens_sold_actual?",
                a.video_url,
                a.google_maps_url,
                a.location_description
            FROM assets a
            WHERE a.slug = $1 AND a.published = true AND a.asset_type != 'commodity'
            "#,
            slug.as_str()
        )
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .map(|a| {
            let share_price = format!("{:.0}", a.token_price_cents as f64 / 100.0);
            let d = crate::assets::models::PropertyDisplayData::from_asset(&a);
            (d, share_price)
        })
    } else {
        None
    };

    let (prop_data, share_price) = match property {
        Some((d, p)) => (Some(d), Some(p)),
        None => (None, None),
    };

    let html = tmpl
        .render(context! {
            error => error.unwrap_or_default(),
            google_enabled => state.config.google_oauth_enabled(),
            referral_code => referral_code,
            property => prop_data,
            share_price => share_price,
        })
        .unwrap_or_else(|e| format!("Template error: {}", e));
    Html(html).into_response()
}

/// URL-encoding helper (for OAuth redirect URIs).
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        encoded
    }
}

#[cfg(test)]
mod google_oauth_tests {
    use super::*;
    use crate::auth::rate_limit::RateLimiter;
    use crate::config::Config;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn test_app_state(token_url: String, userinfo_url: String) -> AppState {
        AppState {
            db: PgPoolOptions::new()
                .connect_lazy("postgres://localhost/poool_test")
                .expect("test database URL should parse"),
            db_replica: None,
            community_db: None,
            templates: crate::templates::create_engine(),
            config: Config {
                database_url: "postgres://localhost/poool_test".to_string(),
                server_host: "127.0.0.1".to_string(),
                server_port: 8888,
                base_url: "http://localhost:8888".to_string(),
                google_client_id: Some("test-client".to_string()),
                google_client_secret: Some("test-secret".to_string()),
                google_oauth_token_url: token_url,
                google_oauth_userinfo_url: userinfo_url,
                facebook_app_id: None,
                facebook_app_secret: None,
                didit_api_key: None,
                didit_workflow_id: None,
                didit_webhook_secret: None,
                redis_url: None,
                database_replica_url: None,
                community_database_url: None,
                sentry_dsn: None,
                app_env: "test".to_string(),
                gcs_bucket: None,
                blog_content_source: "sanity".to_string(),
                sanity_project_id: "test".to_string(),
                sanity_dataset: "test".to_string(),
                sanity_api_version: "2026-04-24".to_string(),
                sanity_studio_url: "https://example.test".to_string(),
                sanity_read_token: None,
                sanity_write_token: None,
                metabase_base_url: String::new(),
                metabase_public_dashboard_path: String::new(),
                metabase_dashboard_id: String::new(),
            },
            redis: None,
            auth_rate_limiter: RateLimiter::new(10, Duration::from_secs(60)),
            leaderboard_rate_limiter: RateLimiter::new(60, Duration::from_secs(60)),
            community_rate_limiter: RateLimiter::new(30, Duration::from_secs(60)),
            storage_rate_limiter: RateLimiter::new(10, Duration::from_secs(60)),
            leaderboard_last_refresh: std::sync::Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    async fn one_shot_json_server(body: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind one-shot server");
        let addr = listener.local_addr().expect("read one-shot server addr");

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        format!("http://{addr}")
    }

    fn oauth_jar() -> CookieJar {
        CookieJar::new()
            .add(Cookie::build(("oauth_state", "state-123")).path("/auth"))
            .add(Cookie::build(("oauth_pkce", "verifier-123")).path("/auth"))
    }

    fn oauth_params() -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            ("state".to_string(), "state-123".to_string()),
            ("code".to_string(), "code-123".to_string()),
        ])
    }

    #[tokio::test]
    async fn google_callback_uses_mock_token_endpoint_and_rejects_missing_access_token() {
        let token_url = one_shot_json_server(r#"{"id_token":"redacted-test-token"}"#).await;
        let state = test_app_state(token_url, "http://127.0.0.1:9/userinfo".to_string());

        let err = google_callback_inner(
            &state,
            oauth_jar(),
            oauth_params(),
            "http://localhost:8888/auth/google/callback".to_string(),
        )
        .await
        .expect_err("missing access_token must fail before userinfo or DB access");

        assert!(
            matches!(err, AppError::Internal(message) if message == "No access token in Google response")
        );
    }

    #[tokio::test]
    async fn google_callback_uses_mock_userinfo_endpoint_and_rejects_unverified_email() {
        let token_url = one_shot_json_server(r#"{"access_token":"test-access-token"}"#).await;
        let userinfo_url = one_shot_json_server(
            r#"{"id":"google-123","email":"user@example.test","verified_email":false}"#,
        )
        .await;
        let state = test_app_state(token_url, userinfo_url);

        let err = google_callback_inner(
            &state,
            oauth_jar(),
            oauth_params(),
            "http://localhost:8888/auth/google/callback".to_string(),
        )
        .await
        .expect_err("unverified Google email must fail before DB access");

        assert!(
            matches!(err, AppError::Unauthorized(message) if message == "Google account email is not verified")
        );
    }

    #[test]
    fn callback_error_cleanup_removes_all_transient_oauth_cookies() {
        let jar = oauth_jar()
            .add(Cookie::build(("oauth_link", "1")).path("/auth"))
            .add(
                Cookie::build((
                    "oauth_redirect_uri",
                    "https://app.poool.finance/auth/google/callback",
                ))
                .path("/auth"),
            );
        let jar = clear_oauth_cookies(jar);

        assert!(jar.get("oauth_state").is_none());
        assert!(jar.get("oauth_pkce").is_none());
        assert!(jar.get("oauth_link").is_none());
        assert!(jar.get("oauth_redirect_uri").is_none());
    }

    #[tokio::test]
    async fn oauth_redirect_uri_uses_forwarded_host_when_config_is_localhost() {
        let state = test_app_state(
            "http://127.0.0.1:9/token".to_string(),
            "http://127.0.0.1:9/userinfo".to_string(),
        );
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("app.poool.finance"),
        );

        assert_eq!(
            google_oauth_redirect_uri(&state.config, &headers),
            "https://app.poool.finance/auth/google/callback"
        );
    }

    #[tokio::test]
    async fn oauth_redirect_uri_keeps_configured_public_base_url() {
        let mut state = test_app_state(
            "http://127.0.0.1:9/token".to_string(),
            "http://127.0.0.1:9/userinfo".to_string(),
        );
        state.config.base_url = "https://poool.finance/".to_string();
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("attacker.example"),
        );

        assert_eq!(
            google_oauth_redirect_uri(&state.config, &headers),
            "https://poool.finance/auth/google/callback"
        );
    }
}

/// GET /profile  User profile page (protected).
pub async fn page_profile(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "profile.html").await
}

/// GET /welcome  Welcome page (protected).
pub async fn page_welcome(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "welcome.html").await
}

// ─── Passkey / WebAuthn handlers ──────────────────────────────

type PasskeyJson = Json<serde_json::Value>;

fn passkey_err(msg: impl Into<String>) -> (StatusCode, PasskeyJson) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": msg.into() })),
    )
}

fn passkey_unauthorized(msg: impl Into<String>) -> (StatusCode, PasskeyJson) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": msg.into() })),
    )
}

/// POST /auth/passkey/login/start
/// Public endpoint — starts discoverable authentication.
pub async fn passkey_login_start(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match service::start_passkey_authentication(&state.db, &state.webauthn).await {
        Ok((challenge_id, options)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "challenge_id": challenge_id, "options": options })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("passkey login start failed: {}", e);
            passkey_err("Could not start passkey authentication.").into_response()
        }
    }
}

/// POST /auth/passkey/login/finish
/// Public endpoint — verifies credential, creates session, redirects.
pub async fn passkey_login_finish(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(body): Json<PasskeyLoginFinishRequest>,
) -> impl IntoResponse {
    let user = match service::finish_passkey_authentication(
        &state.db,
        &state.webauthn,
        body.challenge_id,
        body.credential,
    )
    .await
    {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!("passkey login finish failed: {}", e);
            return passkey_unauthorized("Passkey verification failed.").into_response();
        }
    };

    let ip = match crate::common::net::client_ip(&headers).as_str() {
        "unknown" => None,
        s => Some(s.to_string()),
    };
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let session_token =
        match service::create_session(&state.db, user.id, false, true, ip.as_deref(), user_agent.as_deref()).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("passkey session create failed: {}", e);
                return passkey_err("Session creation failed.").into_response();
            }
        };

    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(24 * 60 * 60));
    let jar = jar.add(cookie).add(super::csrf::rotation_cookie());

    (
        StatusCode::OK,
        jar,
        Json(serde_json::json!({ "redirect": "/marketplace" })),
    )
        .into_response()
}

/// POST /auth/passkey/register/start  (authenticated)
pub async fn passkey_register_start(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return passkey_unauthorized("Not authenticated.").into_response(),
    };

    let display = user.email.clone();
    match service::start_passkey_registration(&state.db, &state.webauthn, user.id, &user.email, &display).await {
        Ok((challenge_id, options)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "challenge_id": challenge_id, "options": options })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("passkey register start failed: {}", e);
            passkey_err("Could not start passkey registration.").into_response()
        }
    }
}

/// POST /auth/passkey/register/finish  (authenticated)
pub async fn passkey_register_finish(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<PasskeyRegisterFinishRequest>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return passkey_unauthorized("Not authenticated.").into_response(),
    };

    match service::finish_passkey_registration(
        &state.db,
        &state.webauthn,
        body.challenge_id,
        user.id,
        body.credential,
        body.name,
    )
    .await
    {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response(),
        Err(e) => {
            tracing::warn!("passkey register finish failed for user {}: {}", user.id, e);
            passkey_err(format!("{e}")).into_response()
        }
    }
}

/// GET /auth/passkey/list  (authenticated)
pub async fn passkey_list(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return passkey_unauthorized("Not authenticated.").into_response(),
    };

    match service::list_user_passkeys(&state.db, user.id).await {
        Ok(passkeys) => (StatusCode::OK, Json(serde_json::json!({ "passkeys": passkeys }))).into_response(),
        Err(e) => {
            tracing::error!("passkey list failed: {}", e);
            passkey_err("Could not list passkeys.").into_response()
        }
    }
}

/// DELETE /auth/passkey/:id  (authenticated)
pub async fn passkey_delete(
    State(state): State<AppState>,
    jar: CookieJar,
    Path(id): Path<uuid::Uuid>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return passkey_unauthorized("Not authenticated.").into_response(),
    };

    match service::delete_passkey(&state.db, user.id, id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "success": true }))).into_response(),
        Err(e) => {
            tracing::warn!("passkey delete failed for user {}: {}", user.id, e);
            passkey_err(format!("{e}")).into_response()
        }
    }
}
