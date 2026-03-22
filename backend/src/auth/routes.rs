/// Auth HTTP routes – thin handlers that delegate to the service layer.
///
/// Each handler is responsible ONLY for:
/// 1. Extracting data from the HTTP request
/// 2. Calling the appropriate service function
/// 3. Formatting the HTTP response
///
/// NO business logic lives here.
use axum::{
    extract::State,
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use minijinja::context;

use super::middleware;
use super::middleware::SESSION_COOKIE;
use super::models::{LoginForm, SignupForm};

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
        .route("/2fa/step-up", post(step_up_verify))
        .route("/logout", get(logout))
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
        .with_state(state)
}

// ─── Page Renders ──────────────────────────────────────────────

/// GET /auth/login – Render the login page.
pub async fn login_page(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    // If already logged in, skip login page and go to marketplace
    if middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/marketplace").into_response();
    }

    let error = params.get("error").cloned();
    render_login(&state, error)
}

/// GET /auth/signup – Render the signup page.
pub async fn signup_page(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let error = params.get("error").cloned();
    render_signup(&state, error)
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
pub async fn verify_email_page(State(state): State<AppState>) -> impl IntoResponse {
    match state.templates.get_template("verify-email.html") {
        Ok(t) => match t.render(minijinja::context! {}) {
            Ok(c) => Html(c).into_response(),
            Err(_) => Redirect::to("/auth/login").into_response(),
        },
        Err(_) => Redirect::to("/auth/login").into_response(),
    }
}

// ─── Form Handlers (HTMX) ─────────────────────────────────────

/// POST /auth/login – Handle login form submission via HTMX.
pub async fn login_submit(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    // Rate limiting — check before doing expensive Argon2 work
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("login:{}", client_ip))
        .await
    {
        tracing::warn!("Rate limit exceeded for login from IP: {}", client_ip);
        return Err(AppError::RateLimited(retry_after));
    }

    // 1. Authenticate user (password check)
    let user = service::authenticate_user(&state.db, &form.email, &form.password).await?;

    // 2. Check 2FA settings
    let settings = service::get_user_settings(&state.db, user.id).await?;
    let _is_admin = service::is_admin(&state.db, user.id).await?;

    // 3. Determine 2FA requirements
    let (is_2fa_verified, redirect_to) = if settings.totp_enabled {
        (false, "/auth/2fa")
    } else {
        (true, "/marketplace")
    };

    // Extract client info for session
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // 4. Create session (starts as unverified if 2FA is needed)
    let session_token = service::create_session(
        &state.db,
        user.id,
        form.remember_me(),
        is_2fa_verified,
        ip.as_deref(),
        user_agent.as_deref(),
    )
    .await?;

    // Audit log
    crate::common::audit::log(
        &state.db,
        Some(user.id),
        "user.login",
        "user",
        Some(user.id),
        ip.as_deref(),
        user_agent.as_deref(),
    )
    .await
    .ok();

    // Track login streak for XP (M4-BE.9)
    if let Some(c_pool) = &state.community_db {
        let _ = crate::community::xp::track_login_streak(c_pool, user.id).await;
    }

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

    let jar = jar.add(cookie);

    // HTMX: send redirect header
    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", redirect_to.parse().unwrap());

    Ok((jar, response_headers, Html("")).into_response())
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
    Form(form): Form<super::models::TotpForm>,
) -> Result<Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session_unverified(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    let settings = service::get_user_settings(&state.db, user.id).await?;
    let secret = settings
        .totp_secret
        .ok_or_else(|| AppError::Internal("2FA not configured.".to_string()))?;

    if !service::verify_totp_code(&secret, &form.code) {
        return Err(AppError::Unauthorized(
            "Invalid authentication code.".to_string(),
        ));
    }

    // Mark session as verified
    service::verify_session_2fa(&state.db, session_token).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", "/marketplace".parse().unwrap());

    Ok((response_headers, Html("")).into_response())
}

/// GET /auth/2fa/setup – Render 2FA setup page (for admins or voluntary).
pub async fn totp_setup_page(
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

    let (secret, url, qr_code) = service::generate_totp_secret(&user.email)?;

    let tmpl = state
        .templates
        .get_template("auth-2fa-setup.html")
        .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;

    let html = tmpl
        .render(context! {
            email => user.email,
            secret => secret,
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
    Form(form): Form<super::models::TotpSetupForm>,
) -> Result<Response, AppError> {
    let session_token = jar
        .get(SESSION_COOKIE)
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?
        .value();

    let user = service::get_user_by_session_unverified(&state.db, session_token)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Session expired.".to_string()))?;

    // Verify first code
    if !service::verify_totp_code(&form.secret, &form.code) {
        return Err(AppError::BadRequest(
            "Invalid authentication code. Please check your authenticator app.".to_string(),
        ));
    }

    // Enable in DB
    service::enable_totp(&state.db, user.id, &form.secret).await?;

    // Mark current session as verified
    service::verify_session_2fa(&state.db, session_token).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", "/marketplace".parse().unwrap());

    Ok((response_headers, Html("")).into_response())
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
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("signup:{}", client_ip))
        .await
    {
        tracing::warn!("Rate limit exceeded for signup from IP: {}", client_ip);
        return Err(AppError::RateLimited(retry_after));
    }

    // ── Terms acceptance guard ──────────────────────────────────
    if !form.terms_accepted() {
        let html = r#"<div id="signup-error" style="color:#D92D20;font-size:14px;padding:8px 12px;background:#FEF3F2;border-radius:8px;border:1px solid #FDA29B;margin-bottom:8px;">
            You must accept the Terms and Conditions and Privacy Policy to create an account.
        </div>"#;
        return Ok(Html(html).into_response());
    }

    // Determine the referral code to use (form priority, fallback to cookie)
    let referral_code = form.referral_code.clone().or_else(|| {
        jar.get(super::middleware::REFERRAL_COOKIE)
            .map(|c| c.value().to_string())
    });

    // Register user (validates, hashes, creates user + wallets + role)
    let user = service::register_user(
        &state.db,
        &form.email,
        &form.password,
        &state.config.base_url,
    )
    .await?;

    // Extract client info
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ── Record terms consent ────────────────────────────────────
    // Fetch current platform terms version (defaults to "1.0" if not set)
    let terms_version: String =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = 'legal_terms_version'")
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "1.0".to_string());

    let _ = sqlx::query(
        "INSERT INTO user_consents (user_id, terms_version, ip_address, user_agent) VALUES ($1, $2, $3, $4)"
    )
    .bind(user.id)
    .bind(&terms_version)
    .bind(ip.as_deref())
    .bind(user_agent.as_deref())
    .execute(&state.db)
    .await;

    // ── Referral System Tracking ─────────────────────────────────
    if let Some(mut code_str) = referral_code.filter(|c| !c.trim().is_empty()) {
        code_str = code_str.trim().to_string();

        let mut subid = None;
        if let Some(idx) = code_str.find('|') {
            subid = Some(code_str[idx + 1..].to_string());
            code_str = code_str[..idx].to_string();
        }

        // 1. Resolve code to referrer user_id and their tier
        let row = sqlx::query!(
            r#"SELECT rc.user_id, referral_bonus 
               FROM referral_codes rc
               JOIN user_tiers ut ON ut.user_id = rc.user_id
               JOIN tiers t ON t.id = ut.tier_id
               WHERE rc.code = $1 LIMIT 1"#,
            code_str
        )
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();

        if let Some(r) = row {
            // Found a valid referrer
            // 2. Insert into referral_tracking (status = 'pending')
            let _ = sqlx::query!(
                "INSERT INTO referral_tracking (referrer_id, referred_id, referrer_reward, referred_reward, status, created_at, subid) \
                 VALUES ($1, $2, $3, $4, 'pending', NOW(), $5)",
                r.user_id,
                user.id,
                r.referral_bonus, // For the referrer: tier's direct bonus
                500, // Fixed initial reward for THE REFERRED ($5.00)
                subid
            )
            .execute(&state.db)
            .await;
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

    // Set session cookie (24h for new signups)
    let cookie = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .http_only(true)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(24 * 60 * 60));

    // Also clear referral cookie if it was used
    let jar = jar
        .remove(Cookie::from(super::middleware::REFERRAL_COOKIE))
        .add(cookie);

    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", "/marketplace".parse().unwrap());

    Ok((jar, response_headers, Html("")).into_response())
}

// ─── Password Reset & Verification (HTMX) ─────────────────────

/// POST /auth/forgot-password
pub async fn forgot_password_submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(form): Form<super::models::ForgotPasswordForm>,
) -> Result<Response, AppError> {
    // Rate limiting — prevent email bombing
    let client_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("forgot:{}", client_ip))
        .await
    {
        tracing::warn!(
            "Rate limit exceeded for forgot-password from IP: {}",
            client_ip
        );
        return Err(AppError::RateLimited(retry_after));
    }

    service::create_password_reset_token(&state.db, &form.email, &state.config.base_url).await?;

    let html = r##"
        <div style="text-align: center; padding: 20px;">
            <svg width="48" height="48" viewBox="0 0 24 24" fill="none" style="margin-bottom: 16px; color: #12B76A;">
                <path d="M12 22C17.5 22 22 17.5 22 12C22 6.5 17.5 2 12 2C6.5 2 2 6.5 2 12C2 17.5 6.5 22 12 22Z" fill="#D1FADF"/>
                <path d="M7.75 12L10.58 14.83L16.25 9.17" stroke="#039855" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            <h3 style="color: #101828; font-size: 18px; margin-bottom: 8px;">Check your email</h3>
            <p style="color: #475467; font-size: 14px;">We've sent password reset instructions to your email.</p>
        </div>
    "##;

    Ok(Html(html).into_response())
}

/// POST /auth/reset-password
pub async fn reset_password_submit(
    State(state): State<AppState>,
    Form(form): Form<super::models::ResetPasswordForm>,
) -> Result<Response, AppError> {
    if form.password != form.confirm_password {
        return Err(AppError::BadRequest("Passwords do not match.".to_string()));
    }
    if form.token.is_empty() {
        return Err(AppError::BadRequest("Missing reset token.".to_string()));
    }

    service::reset_password(&state.db, &form.token, &form.password).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert("HX-Redirect", "/auth/login".parse().unwrap());

    Ok((response_headers, Html("")).into_response())
}

/// POST /auth/resend-verification
pub async fn resend_verification_submit(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Response, AppError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        if let Ok(Some(user)) = service::get_user_by_session(&state.db, cookie.value()).await {
            let _ = service::create_email_verification_token(
                &state.db,
                user.id,
                &user.email,
                &state.config.base_url,
            )
            .await;
        }
    }

    let html = r#"
        <div style="text-align: center; padding: 20px;">
            <p style="color: #039855; font-size: 14px; font-weight: 500;">Verification email resent successfully!</p>
        </div>
    "#;

    Ok(Html(html).into_response())
}

// ─── Logout ────────────────────────────────────────────────────

/// GET /logout – Destroy session and redirect to login.
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    // Delete session from DB
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let _ = service::delete_session(&state.db, cookie.value()).await;
    }

    // Clear cookie
    let jar = jar.remove(Cookie::from(SESSION_COOKIE));

    (jar, Redirect::to("/auth/login"))
}

// ─── OAuth Routes ──────────────────────────────────────────────

/// GET /auth/google – Redirect to Google OAuth consent screen.
pub async fn google_redirect(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    // If already logged in, no need to do OAuth
    if middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/marketplace").into_response();
    }

    if !state.config.google_oauth_enabled() {
        return Redirect::to("/auth/login?error=oauth_not_configured").into_response();
    }

    let client_id = match state.config.google_client_id.as_ref() {
        Some(id) => id,
        None => return Redirect::to("/auth/login?error=oauth_not_configured").into_response(),
    };
    let redirect_uri = format!("{}/auth/google/callback", state.config.base_url);

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=email%20profile&access_type=offline",
        client_id,
        urlencoding::encode(&redirect_uri)
    );

    Redirect::to(&auth_url).into_response()
}

/// GET /auth/google/callback – Handle Google OAuth callback.
pub async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    match google_callback_inner(&state, jar, params).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Google OAuth callback error: {}", e);
            Redirect::to("/auth/login?error=Google+sign+in+failed.+Please+try+again.")
                .into_response()
        }
    }
}

async fn google_callback_inner(
    state: &AppState,
    jar: CookieJar,
    params: std::collections::HashMap<String, String>,
) -> Result<Response, AppError> {
    let code = params
        .get("code")
        .ok_or_else(|| AppError::BadRequest("Missing authorization code.".to_string()))?;

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
    let redirect_uri = format!("{}/auth/google/callback", state.config.base_url);

    // Exchange code for access token
    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
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
        tracing::error!("No access_token in Google response: {:?}", token_data);
        AppError::Internal("No access token in Google response".to_string())
    })?;

    // Fetch user info
    let user_info: serde_json::Value = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
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
    let first_name = user_info["given_name"].as_str().map(|s| s.to_string());
    let last_name = user_info["family_name"].as_str().map(|s| s.to_string());
    let avatar_url = user_info["picture"].as_str().map(|s| s.to_string());

    // Find or create user
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

    // Create session
    let session_token = service::create_session(&state.db, user.id, true, true, None, None).await?;

    // Track login streak for XP (M4-BE.9)
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

    Ok((jar, Redirect::to("/marketplace")).into_response())
}

// ─── Template helpers ──────────────────────────────────────────

fn render_login(state: &AppState, error: Option<String>) -> Response {
    let tmpl = match state.templates.get_template("login.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load login.html template: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };
    let html = tmpl
        .render(context! {
            error => error.unwrap_or_default(),
            google_enabled => state.config.google_oauth_enabled(),
        })
        .unwrap_or_else(|e| format!("Template error: {}", e));
    Html(html).into_response()
}

fn render_signup(state: &AppState, error: Option<String>) -> Response {
    let tmpl = match state.templates.get_template("signup.html") {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load signup.html template: {}", e);
            return Html("<h1>Internal Server Error</h1>".to_string()).into_response();
        }
    };
    let html = tmpl
        .render(context! {
            error => error.unwrap_or_default(),
            google_enabled => state.config.google_oauth_enabled(),
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

/// GET /profile  User profile page (protected).
pub async fn page_profile(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "profile.html").await
}

/// GET /welcome  Welcome page (protected).
pub async fn page_welcome(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "welcome.html").await
}
