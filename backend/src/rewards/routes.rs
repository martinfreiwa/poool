use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use sqlx::Row;

use crate::auth::middleware;
use crate::auth::routes::AppState;

use super::service;

async fn require_user_id(
    jar: &CookieJar,
    state: &AppState,
) -> Result<uuid::Uuid, axum::response::Response> {
    match middleware::get_current_user(jar, &state.db).await {
        Some(user) => Ok(user.id),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Not authenticated"})),
        )
            .into_response()),
    }
}

async fn is_active_affiliate(state: &AppState, user_id: uuid::Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM affiliates WHERE user_id = $1 AND status = 'active')",
        user_id
    )
    .fetch_one(&state.db)
    .await
    .map(|active| active.unwrap_or(false))
}

fn html_escape(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn get_rewards_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_rewards_overview(&state.db, user_id).await {
        Ok(overview) => Json(overview).into_response(),
        Err(e) => {
            tracing::error!("Failed to get rewards for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load rewards."})),
            )
                .into_response()
        }
    }
}

pub async fn get_tiers_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let _user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_all_tiers(&state.db).await {
        Ok(tiers) => Json(tiers).into_response(),
        Err(e) => {
            tracing::error!("Failed to get tiers: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load tiers."})),
            )
                .into_response()
        }
    }
}

pub async fn get_campaigns_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_campaign_breakdown(&state.db, user_id).await {
        Ok(campaigns) => Json(campaigns).into_response(),
        Err(e) => {
            tracing::error!(
                "Failed to get campaign breakdown for user {}: {}",
                user_id,
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load campaign data."})),
            )
                .into_response()
        }
    }
}

fn cookie_is_secure() -> bool {
    matches!(
        std::env::var("POOOL_ENV").as_deref(),
        Ok("production") | Ok("prod") | Ok("staging")
    )
}

/// GET /rewards — Render the rewards page.
pub async fn page_rewards(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "rewards.html").await
}

/// GET /affiliate — Render the affiliate promo/landing page (Phase 19).
pub async fn page_affiliate_promo(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-promo.html").await
}

/// GET /affiliate/onboarding — Render the 5-step compliance wizard (Phase 19).
pub async fn page_affiliate_onboarding(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(user) => user,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let status =
        match sqlx::query_scalar::<_, String>("SELECT status FROM affiliates WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
        {
            Ok(status) => status,
            Err(err) => {
                tracing::error!(
                    user_id = %user.id,
                    error = %err,
                    "Failed to check affiliate onboarding status"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Unable to load affiliate onboarding</h1>".to_string()),
                )
                    .into_response();
            }
        };

    match status.as_deref() {
        Some("active") | Some("pending_approval") => {
            return Redirect::to("/affiliate/dashboard").into_response();
        }
        Some("suspended") => return Redirect::to("/support").into_response(),
        _ => {}
    }

    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-onboarding.html").await
}

/// GET /affiliate/dashboard — Render the affiliate dashboard page (Phase 19).
pub async fn page_affiliate_dashboard(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-dashboard.html").await
}

/// GET /affiliate/referrals — Render the affiliate referrals & payouts page (Phase 19).
pub async fn page_affiliate_referrals(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(user) => user,
        None => return Redirect::to("/auth/login").into_response(),
    };

    match is_active_affiliate(&state, user.id).await {
        Ok(true) => {}
        Ok(false) => return Redirect::to("/affiliate/onboarding").into_response(),
        Err(err) => {
            tracing::error!(
                user_id = %user.id,
                error = %err,
                "Failed to verify affiliate status for referrals page"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Unable to load affiliate referrals</h1>".to_string()),
            )
                .into_response();
        }
    }

    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-referrals.html").await
}

/// GET /affiliate/materials — Render the affiliate marketing materials page (Phase 19).
pub async fn page_affiliate_materials(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-materials.html").await
}

/// GET /affiliate/settings — Render the affiliate settings and tax info page (Phase 19).
pub async fn page_affiliate_settings(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-settings.html").await
}

async fn require_active_affiliate_user_id(
    jar: &CookieJar,
    state: &AppState,
) -> Result<uuid::Uuid, crate::error::AppError> {
    let user_id = require_user_id(jar, state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    let affiliate = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if affiliate.is_none() {
        return Err(crate::error::AppError::Forbidden(
            "Only active affiliates can use marketing materials".into(),
        ));
    }

    Ok(user_id)
}

/// GET /rewards/:code — Set referral cookie and redirect to signup.
pub async fn page_referral_landing(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    axum::extract::Path(code): axum::extract::Path<String>,
) -> impl IntoResponse {
    let code_clone = code.clone();
    let subid = params.get("subid").cloned();
    let subid_clone = subid.clone();
    let utm_source = params.get("utm_source").cloned();

    // Properly extract IP (preventing comma-separated sql crashes and parsing last appended LB ip)
    let raw_ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("0.0.0.0");

    // Cloud run / standard LBs append client IP to the end, or it's just the IP if direct.
    let ip = raw_ip
        .split(',')
        .last()
        .unwrap_or("0.0.0.0")
        .trim()
        .to_string();

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Trivial click fraud prevention: if they already have the exact same cookie, don't re-record the click.
    let already_clicked = jar.get("poool_ref").map(|c| c.value()) == Some(&code_clone);

    // Record the click in the background only if it's uniquely new and IP isn't hammering us
    if !already_clicked {
        // Enforce a strict click rate limit of 10 clicks per IP per 15 minutes globally to stop bot swarms
        if state
            .auth_rate_limiter
            .check(&format!("click_throttle:{}", ip))
            .await
            .is_ok()
        {
            tokio::spawn(async move {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO referral_clicks (code, ip_address, user_agent, subid)
                    SELECT $1::varchar, $2::inet, $3, $4
                    WHERE (
                        EXISTS (SELECT 1 FROM referral_codes WHERE code = $1::varchar)
                        OR EXISTS (SELECT 1 FROM affiliates WHERE referral_code = $1::varchar AND status = 'active')
                    )
                    AND NOT EXISTS (
                        SELECT 1 FROM referral_clicks
                        WHERE code = $1::varchar
                          AND ip_address = $2::inet
                          AND created_at > NOW() - INTERVAL '24 hours'
                    )
                    "#,
                )
                .bind(code_clone)
                .bind(ip)
                .bind(user_agent)
                .bind(subid_clone)
                .execute(&state.db)
                .await;
            });
        } else {
            tracing::warn!("Click throttle triggered for IP: {}", ip);
        }
    }

    // Store data in the format: code|subid|utm_source
    // This maintains backward compatibility with the legacy code|subid parsing logic.
    let cookie_val = format!(
        "{}|{}|{}",
        code,
        subid.unwrap_or_default(),
        utm_source.unwrap_or_default()
    );

    let cookie = axum_extra::extract::cookie::Cookie::build((
        crate::auth::middleware::REFERRAL_COOKIE,
        cookie_val,
    ))
    .path("/")
    .http_only(true)
    .secure(cookie_is_secure())
    .same_site(axum_extra::extract::cookie::SameSite::Lax)
    .max_age(time::Duration::days(30));

    (
        jar.add(cookie),
        axum::response::Redirect::to("/auth/signup"),
    )
}

/// GET /tier — Redirect to the Tier tab on the Rewards page.
pub async fn page_tier(_jar: CookieJar, _state: State<AppState>) -> impl IntoResponse {
    axum::response::Redirect::permanent("/rewards#tier")
}

/// GET /api/rewards/payout-settings — Fetch user's payout settings.
pub async fn get_payout_settings_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_payout_settings(&state.db, user_id).await {
        Ok(settings) => Json(serde_json::json!({ "payout_settings": settings })).into_response(),
        Err(e) => {
            tracing::error!("Failed to get payout settings for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load payout settings."})),
            )
                .into_response()
        }
    }
}

/// POST /api/rewards/payout-settings — Save/update user's payout settings.
pub async fn save_payout_settings_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<super::models::SavePayoutSettingsForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::save_payout_settings(&state.db, user_id, form).await {
        Ok(saved) => Json(serde_json::json!({ "payout_settings": saved })).into_response(),
        Err(e) => {
            tracing::error!("Failed to save payout settings for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save payout settings."})),
            )
                .into_response()
        }
    }
}

/// GET /api/affiliate/settings — Fetch affiliate tax and payout settings.
pub async fn get_affiliate_settings_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_affiliate_settings(&state.db, user_id).await {
        Ok(settings) => Json(serde_json::json!({
            "success": true,
            "settings": settings
        }))
        .into_response(),
        Err(e) => e.into_response(),
    }
}

/// POST /api/affiliate/settings — Save affiliate tax and payout settings.
pub async fn save_affiliate_settings_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<super::models::SaveAffiliateSettingsForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::save_affiliate_settings(&state.db, user_id, form).await {
        Ok(settings) => Json(serde_json::json!({
            "success": true,
            "settings": settings
        }))
        .into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/rewards/commissions?from=YYYY-MM-DD&to=YYYY-MM-DD — List commissions.
pub async fn list_commissions_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let date_from = params
        .get("from")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let date_to = params
        .get("to")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    match service::list_commissions(&state.db, user_id, date_from, date_to).await {
        Ok(commissions) => Json(serde_json::json!({ "commissions": commissions })).into_response(),
        Err(e) => {
            tracing::error!("Failed to list commissions for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load commissions."})),
            )
                .into_response()
        }
    }
}

/// Required policy names — must match the onboarding wizard exactly.
const REQUIRED_POLICIES: &[&str] = &[
    "Affiliate Terms & Conditions",
    "Affiliate Code of Conduct",
    "Approved Marketing Materials Policy",
    "Qualified Referral & Payout Policy",
    "Affiliate Privacy Notice",
];

const ALLOWED_TRAFFIC_SOURCES: &[&str] = &["newsletter", "youtube", "twitter", "wealth", "other"];
const ALLOWED_AUDIENCE_SIZES: &[&str] = &["under5k", "5k_50k", "over50k"];
const EXAM_ANSWERS: &[(&str, &str)] = &[
    ("q1", "no"),
    ("q2", "no"),
    ("q3", "30days"),
    ("q4", "no"),
    ("q5", "no"),
];

fn json_error(status: StatusCode, message: impl Into<String>) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": message.into() }))).into_response()
}

fn validate_exam_answers(
    answers: Option<&std::collections::HashMap<String, String>>,
) -> Result<(), String> {
    let answers = answers.ok_or_else(|| "All compliance exam answers are required.".to_string())?;

    for (question, expected) in EXAM_ANSWERS {
        let answer = answers
            .get(*question)
            .ok_or_else(|| format!("Missing compliance exam answer: {}", question))?;
        if answer != expected {
            return Err("One or more compliance exam answers are incorrect.".to_string());
        }
    }

    Ok(())
}

fn validate_url_field(raw: &str) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() || value.len() > 512 {
        return Err("A valid URL up to 512 characters is required.".to_string());
    }

    let parsed = url::Url::parse(value).map_err(|_| "A valid URL is required.".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("A valid http:// or https:// URL with a host is required.".to_string());
    }

    Ok(value.to_string())
}

fn validate_bounded_field(label: &str, raw: &str, max_len: usize) -> Result<String, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(format!("{} is required.", label));
    }
    if value.len() > max_len {
        return Err(format!(
            "{} must be {} characters or fewer.",
            label, max_len
        ));
    }
    Ok(value.to_string())
}

fn validate_phone_number(raw: &str) -> Result<String, String> {
    let value = validate_bounded_field("Phone number", raw, 50)?;
    let digit_count = value.chars().filter(|c| c.is_ascii_digit()).count();
    let chars_ok = value
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '(' | ')' | ' ' | '.'));
    if !chars_ok || !(7..=20).contains(&digit_count) {
        return Err(
            "Phone number must contain 7 to 20 digits and only phone-safe characters.".to_string(),
        );
    }
    Ok(value)
}

async fn has_approved_kyc(state: &AppState, user_id: uuid::Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM kyc_records WHERE user_id = $1 AND status = 'approved')",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
}

pub async fn submit_affiliate_onboarding_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(form): Json<super::models::SubmitOnboardingForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // ── Rate Limiting (3 attempts per 15 minutes per user) ──────────
    if let Err(retry_after) = state
        .auth_rate_limiter
        .check(&format!("affiliate_onboard:{}", user_id))
        .await
    {
        tracing::warn!(
            "Rate limit exceeded for affiliate onboarding: user={}",
            user_id
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": format!("Too many attempts. Please wait {} seconds.", retry_after)
            })),
        )
            .into_response();
    }

    // ── Server-Side Exam Validation ─────────────────────────────────
    if !form.exam_passed {
        return json_error(StatusCode::BAD_REQUEST, "Exam not passed");
    }

    if let Err(message) = validate_exam_answers(form.exam_answers.as_ref()) {
        return json_error(StatusCode::BAD_REQUEST, message);
    }

    match has_approved_kyc(&state, user_id).await {
        Ok(true) => {}
        Ok(false) => {
            return json_error(
                StatusCode::FORBIDDEN,
                "Complete identity verification before submitting an affiliate application.",
            );
        }
        Err(err) => {
            tracing::error!(
                user_id = %user_id,
                error = %err,
                "Failed to check KYC status for affiliate onboarding"
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify identity status.",
            );
        }
    }

    // ── Policy Acceptance Validation ────────────────────────────────
    if form.accepted_policies.len() != REQUIRED_POLICIES.len() {
        return json_error(
            StatusCode::BAD_REQUEST,
            format!("All {} policies must be accepted.", REQUIRED_POLICIES.len()),
        );
    }

    for required in REQUIRED_POLICIES {
        if !form.accepted_policies.iter().any(|p| p == required) {
            return json_error(
                StatusCode::BAD_REQUEST,
                format!("Missing required policy acceptance: {}", required),
            );
        }
    }

    // ── Input Validation ────────────────────────────────────────────
    let traffic_source = form.traffic_source.trim();
    if !ALLOWED_TRAFFIC_SOURCES.contains(&traffic_source) {
        return json_error(StatusCode::BAD_REQUEST, "Select a valid traffic source.");
    }

    let audience_size = form.audience_size.trim();
    if !ALLOWED_AUDIENCE_SIZES.contains(&audience_size) {
        return json_error(StatusCode::BAD_REQUEST, "Select a valid audience size.");
    }

    let main_url = match validate_url_field(&form.main_url) {
        Ok(value) => value,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };

    let phone_number = match validate_phone_number(&form.phone_number) {
        Ok(value) => value,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };

    let tax_id = match validate_bounded_field("Tax ID", &form.tax_id, 50) {
        Ok(value) => value,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };
    let tax_id_storage = match service::encrypt_tax_id_for_storage(&tax_id) {
        Ok(storage) => storage,
        Err(err) => {
            tracing::error!(
                user_id = %user_id,
                error = %err,
                "Failed to encrypt affiliate onboarding Tax ID"
            );
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to secure tax details.",
            );
        }
    };

    let company_name = match form.company_name.as_deref() {
        Some(value) if value.trim().is_empty() => None,
        Some(value) if value.trim().len() <= 255 => Some(value.trim().to_string()),
        Some(_) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Company name must be 255 characters or fewer.",
            )
        }
        None => None,
    };

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ── Duplicate Application Guard ─────────────────────────────────
    // Block resubmission if already pending_approval or active
    let existing_status: Option<String> =
        match sqlx::query_scalar("SELECT status FROM affiliates WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
        {
            Ok(status) => status,
            Err(err) => {
                tracing::error!(
                    user_id = %user_id,
                    error = %err,
                    "Failed to check existing affiliate application status"
                );
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to check existing affiliate application.",
                );
            }
        };

    match existing_status.as_deref() {
        Some("pending_approval") => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Your application is already under review. Please wait for admin approval."
                })),
            )
                .into_response();
        }
        Some("active") => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "You are already an active affiliate."
                })),
            )
                .into_response();
        }
        Some("suspended") => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "Your affiliate account is suspended. Please contact support."
                })),
            )
                .into_response();
        }
        _ => {} // 'terminated', 'pending_onboarding', or no record — allow (re)application
    }

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to begin transaction: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save application."})),
            )
                .into_response();
        }
    };

    // Generate unique unseen code to satisfy DB UNIQUE NOT NULL constraint
    let temp_code = format!("PEND_{}", uuid::Uuid::new_v4().as_fields().0);

    let result = sqlx::query(
        r#"
        INSERT INTO affiliates (
            user_id, referral_code, status,
            traffic_source, audience_size, main_url, phone_number,
            tax_id, tax_id_encrypted, tax_id_last4, company_name
        )
        VALUES ($1, $2, 'pending_approval', $3, $4, $5, $6, NULL, $7, $8, $9)
        ON CONFLICT (user_id) DO UPDATE SET 
            status = 'pending_approval',
            traffic_source = EXCLUDED.traffic_source,
            audience_size = EXCLUDED.audience_size,
            main_url = EXCLUDED.main_url,
            phone_number = EXCLUDED.phone_number,
            tax_id = NULL,
            tax_id_encrypted = EXCLUDED.tax_id_encrypted,
            tax_id_last4 = EXCLUDED.tax_id_last4,
            company_name = EXCLUDED.company_name
        "#,
    )
    .bind(user_id)
    .bind(temp_code)
    .bind(traffic_source)
    .bind(audience_size)
    .bind(main_url)
    .bind(phone_number)
    .bind(&tax_id_storage.encrypted)
    .bind(&tax_id_storage.last4)
    .bind(company_name.as_deref())
    .execute(&mut *tx)
    .await;

    if let Err(e) = result {
        tracing::error!("Failed to save affiliate application (DB error): {}", e);
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save application."})),
        )
            .into_response();
    }

    for policy in &form.accepted_policies {
        let res = sqlx::query!(
            r#"
            INSERT INTO affiliate_policy_acceptances (affiliate_id, policy_name, policy_version, ip_address)
            VALUES ($1, $2, '1.0', $3)
            "#,
            user_id,
            policy,
            ip
        )
        .execute(&mut *tx)
        .await;

        if let Err(e) = res {
            tracing::error!("Failed to save policy acceptance: {}", e);
            let _ = tx.rollback().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to save application policies."})),
            )
                .into_response();
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit transaction: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save application."})),
        )
            .into_response();
    }

    tracing::info!(user_id = %user_id, "Affiliate onboarding application submitted");

    // Notifications
    let user_row = sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or_default();

    if let Some(user_record) = user_row {
        // To Admin
        let _ = crate::common::email::send_email(
            "admin@poool.app",
            "New Affiliate Application",
            &format!("<p>A new POOOL Affiliate application has been submitted by user <b>{}</b> ({}). Please log into the Admin portal to review it.</p>", user_id, user_record.email)
        ).await;

        // To User
        let _ = crate::common::email::send_email(
            &user_record.email,
            "Your POOOL Affiliate Application has been received",
            "<h3>Application Received</h3><p>We have successfully received your application for the POOOL Affiliate Partner Syndicate.</p><p>Our compliance team will review your application shortly. You will receive another email once your account has been approved.</p>"
        ).await;
    }

    Json(serde_json::json!({"success": true, "status": "pending_approval"})).into_response()
}

/// GET /api/affiliate/dashboard — Returns full affiliate dashboard metrics.
pub async fn get_affiliate_dashboard_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(r) => return r,
    };

    match service::get_affiliate_dashboard(&state.db, user_id).await {
        Ok(data) => {
            // A.4 Security Gap Fix: Return explicit 403 if user is not an affiliate
            if data.get("is_affiliate").and_then(|v| v.as_bool()) == Some(false) {
                tracing::warn!(user_id = %user_id, "Unauthorized access attempt to affiliate dashboard API");
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "You must be an active affiliate to access this endpoint."})),
                )
                    .into_response();
            }
            Json(data).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get affiliate dashboard: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load affiliate data"})),
            )
                .into_response()
        }
    }
}

/// POST /api/affiliate/payout/request
/// Triggered by the affiliate on their dashboard to notify admin of payout readiness.
pub async fn api_affiliate_payout_request(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, crate::error::AppError> {
    use sqlx::Row as _;

    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    let mut tx = state.db.begin().await?;

    let affiliate = sqlx::query(
        "SELECT referral_code FROM affiliates WHERE user_id = $1 AND status = 'active' FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let affiliate = match affiliate {
        Some(a) => a,
        None => {
            return Err(crate::error::AppError::Forbidden(
                "Only active affiliates can request payouts".into(),
            ))
        }
    };
    let referral_code: String = affiliate.try_get("referral_code")?;

    let payable: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(provisional_amount_cents), 0)::bigint FROM affiliate_commissions WHERE affiliate_id = $1 AND status = 'payable'",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    if payable < 5000 {
        return Err(crate::error::AppError::BadRequest(
            "A minimum of $50 in payable commissions is required to request a payout.".into(),
        ));
    }

    let user_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

    let request_id = match sqlx::query(
        r#"
        INSERT INTO affiliate_payout_requests (affiliate_id, amount_cents, status)
        VALUES ($1, $2, 'requested')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(payable)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(row) => row.try_get::<uuid::Uuid, _>("id")?,
        Err(sqlx::Error::Database(db_err))
            if db_err.constraint() == Some("idx_affiliate_payout_requests_open") =>
        {
            return Err(crate::error::AppError::Conflict(
                "A payout request is already pending review.".into(),
            ));
        }
        Err(e) => return Err(e.into()),
    };

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'affiliate.payout_requested', 'affiliate_payout_requests', $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(request_id)
    .bind(serde_json::json!({
        "affiliate_id": user_id,
        "amount_cents": payable,
        "status": "requested"
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let amount_display = crate::common::currency::format_usd(payable);
    let notification_sent = crate::common::email::send_email(
        "admin@poool.app",
        "Affiliate Commission Payout Request",
        &format!(
            "<h3>Payout Request</h3><p>Affiliate <b>{}</b> (code: <code>{}</code>) has manually requested a payout of their payable commissions totaling <b>{}</b>.</p><p>Please log into the Admin Rewards Dashboard under the <b>Pending Payouts</b> tab to approve and batch this payout to their cash wallet.</p>",
            html_escape(&user_email),
            html_escape(&referral_code),
            amount_display
        )
    )
    .await
    .is_ok();

    Ok(Json(serde_json::json!({
        "success": true,
        "request_id": request_id,
        "notification_sent": notification_sent,
        "message": if notification_sent {
            "Payout request logged and admin notification sent."
        } else {
            "Payout request logged. Admin notification will need follow-up."
        }
    }))
    .into_response())
}

/// GET /api/affiliate/subid-stats
/// Returns clicks and registrations grouped by SubID.
pub async fn api_affiliate_subid_stats(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if active affiliate
    let affiliate = sqlx::query!(
        "SELECT referral_code FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    let affiliate = match affiliate {
        Some(a) => a,
        None => {
            return Err(crate::error::AppError::Forbidden(
                "Only active affiliates can view subID stats".into(),
            ))
        }
    };

    // Query clicks grouped by subid (non-macro: avoids COALESCE/COUNT type inference issues)
    use sqlx::Row as _;
    let clicks = sqlx::query(
        r#"SELECT COALESCE(subid, 'unknown') as sub_id, COUNT(*)::bigint as click_count
           FROM referral_clicks
           WHERE code = $1
           GROUP BY sub_id"#,
    )
    .bind(&affiliate.referral_code)
    .fetch_all(&state.db)
    .await?;

    // Query registrations grouped by subid
    let regs = sqlx::query(
        r#"SELECT COALESCE(sub_id, 'unknown') as sub_id, COUNT(*)::bigint as reg_count
           FROM affiliate_referrals
           WHERE affiliate_id = $1
           GROUP BY sub_id"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    // GAP-15: Revenue dimension — commission totals grouped by subid
    let revenues = sqlx::query(
        r#"SELECT COALESCE(ar.sub_id, 'unknown') as sub_id,
                  COALESCE(SUM(ac.provisional_amount_cents) FILTER (WHERE ac.status IN ('payable', 'paid')), 0)::bigint as earned_cents,
                  COALESCE(SUM(ac.provisional_amount_cents) FILTER (WHERE ac.status = 'provisionally_tracked'), 0)::bigint as pending_cents
           FROM affiliate_referrals ar
           LEFT JOIN affiliate_commissions ac ON ac.referral_id = ar.id
           WHERE ar.affiliate_id = $1
           GROUP BY ar.sub_id"#
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    // Combine into a map
    use std::collections::HashMap;
    let mut stats: HashMap<String, serde_json::Value> = HashMap::new();

    for c in &clicks {
        let subid: String = c
            .try_get("sub_id")
            .unwrap_or_else(|_| "unknown".to_string());
        let clicks_count: i64 = c.try_get("click_count").unwrap_or(0);
        stats.insert(
            subid.clone(),
            serde_json::json!({
                "sub_id": subid,
                "clicks": clicks_count,
                "registrations": 0,
                "earned_cents": 0,
                "pending_cents": 0
            }),
        );
    }

    for r in &regs {
        let subid: String = r
            .try_get("sub_id")
            .unwrap_or_else(|_| "unknown".to_string());
        let reg_count: i64 = r.try_get("reg_count").unwrap_or(0);
        if let Some(entry) = stats.get_mut(&subid) {
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("registrations".to_string(), serde_json::json!(reg_count));
            }
        } else {
            stats.insert(
                subid.clone(),
                serde_json::json!({
                    "sub_id": subid,
                    "clicks": 0,
                    "registrations": reg_count,
                    "earned_cents": 0,
                    "pending_cents": 0
                }),
            );
        }
    }

    for rev in &revenues {
        let subid: String = rev
            .try_get("sub_id")
            .unwrap_or_else(|_| "unknown".to_string());
        let earned: i64 = rev.try_get("earned_cents").unwrap_or(0);
        let pending: i64 = rev.try_get("pending_cents").unwrap_or(0);
        if let Some(entry) = stats.get_mut(&subid) {
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("earned_cents".to_string(), serde_json::json!(earned));
                obj.insert("pending_cents".to_string(), serde_json::json!(pending));
            }
        } else {
            stats.insert(
                subid.clone(),
                serde_json::json!({
                    "sub_id": subid,
                    "clicks": 0,
                    "registrations": 0,
                    "earned_cents": earned,
                    "pending_cents": pending
                }),
            );
        }
    }

    let stats_list: Vec<_> = stats.into_values().collect();

    Ok(Json(serde_json::json!({"stats": stats_list})).into_response())
}

/// POST /api/affiliate/policy-reaccept
/// Allows an affiliate to re-accept the current policy version after an update.
pub async fn api_affiliate_policy_reaccept(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(form): Json<super::models::SubmitOnboardingForm>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Verify affiliate is active
    let aff = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if aff.is_none() {
        return Err(crate::error::AppError::Forbidden(
            "Only active affiliates can re-accept policies".into(),
        ));
    }

    // Validate all required policies are present
    if form.accepted_policies.len() != REQUIRED_POLICIES.len() {
        return Err(crate::error::AppError::BadRequest(format!(
            "All {} policies must be accepted.",
            REQUIRED_POLICIES.len()
        )));
    }

    for required in REQUIRED_POLICIES {
        if !form.accepted_policies.iter().any(|p| p == required) {
            return Err(crate::error::AppError::BadRequest(format!(
                "Missing required policy: {}",
                required
            )));
        }
    }

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| crate::error::AppError::Internal("Transaction error".into()))?;

    // Insert new policy acceptance records with the current version
    for policy in &form.accepted_policies {
        sqlx::query!(
            r#"INSERT INTO affiliate_policy_acceptances (affiliate_id, policy_name, policy_version, ip_address)
               VALUES ($1, $2, $3, $4)"#,
            user_id,
            policy,
            service::CURRENT_POLICY_VERSION,
            ip
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| crate::error::AppError::Internal("Failed to save policy acceptance".into()))?;
    }

    // Update the accepted version on the affiliate record (non-macro: column added in migration 076)
    sqlx::query(
        "UPDATE affiliates SET accepted_policy_version = $1, updated_at = NOW() WHERE user_id = $2",
    )
    .bind(service::CURRENT_POLICY_VERSION)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| crate::error::AppError::Internal("Failed to update policy version".into()))?;

    tx.commit()
        .await
        .map_err(|_| crate::error::AppError::Internal("Commit error".into()))?;

    tracing::info!(user_id = %user_id, version = service::CURRENT_POLICY_VERSION, "Affiliate re-accepted policies");

    Ok(Json(
        serde_json::json!({"success": true, "accepted_version": service::CURRENT_POLICY_VERSION}),
    )
    .into_response())
}

fn format_cents_decimal(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.saturating_abs();
    format!("{}{}.{:02}", sign, absolute / 100, absolute % 100)
}

fn csv_escape(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// GET /api/affiliate/commissions/export
/// Exports affiliate commissions as a CSV file for download.
pub async fn api_affiliate_commissions_export(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    if !is_active_affiliate(&state, user_id).await? {
        return Err(crate::error::AppError::Forbidden(
            "Only active affiliates can export commissions".into(),
        ));
    }

    let date_from = params
        .get("from")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let date_to = params
        .get("to")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let limit: i64 = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(50)
        .clamp(1, 200);
    let offset = (page - 1) * limit;

    let fmt = params.get("format").map(|s| s.as_str()).unwrap_or("json");

    // Build query with optional date filters and pagination
    // Non-macro: avoids COALESCE/date-param type inference issues at compile time
    let rows = sqlx::query(
        r#"SELECT ac.provisional_amount_cents, ac.status, ac.tier_at_execution,
                  ac.created_at::text as created_at,
                  COALESCE(ar.sub_id, '') as sub_id
           FROM affiliate_commissions ac
           LEFT JOIN affiliate_referrals ar ON ar.id = ac.referral_id
           WHERE ac.affiliate_id = $1
             AND ($2::date IS NULL OR ac.created_at::date >= $2)
             AND ($3::date IS NULL OR ac.created_at::date <= $3)
           ORDER BY ac.created_at DESC
           LIMIT $4 OFFSET $5"#,
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(crate::error::AppError::Database)?;

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM affiliate_commissions ac
           WHERE ac.affiliate_id = $1
             AND ($2::date IS NULL OR ac.created_at::date >= $2)
             AND ($3::date IS NULL OR ac.created_at::date <= $3)"#,
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .fetch_one(&state.db)
    .await
    .map_err(crate::error::AppError::Database)?;

    if fmt == "csv" {
        // Return as downloadable CSV
        let mut csv = "date,sub_id,tier,amount_usd,status\n".to_string();
        for r in &rows {
            let created_at: String = r
                .try_get("created_at")
                .map_err(crate::error::AppError::Database)?;
            let sub_id: String = r
                .try_get("sub_id")
                .map_err(crate::error::AppError::Database)?;
            let tier: String = r
                .try_get("tier_at_execution")
                .map_err(crate::error::AppError::Database)?;
            let amount_cents: i64 = r
                .try_get("provisional_amount_cents")
                .map_err(crate::error::AppError::Database)?;
            let status: String = r
                .try_get::<Option<String>, _>("status")
                .map_err(crate::error::AppError::Database)?
                .unwrap_or_else(String::new);
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                csv_escape(&created_at),
                csv_escape(&sub_id),
                csv_escape(&tier),
                format_cents_decimal(amount_cents),
                csv_escape(&status)
            ));
        }

        return Ok((
            axum::http::StatusCode::OK,
            [
                ("Content-Type", "text/csv"),
                (
                    "Content-Disposition",
                    "attachment; filename=\"commissions.csv\"",
                ),
            ],
            csv,
        )
            .into_response());
    }

    let commissions: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let created_at: String = r
                .try_get("created_at")
                .map_err(crate::error::AppError::Database)?;
            let sub_id: String = r
                .try_get("sub_id")
                .map_err(crate::error::AppError::Database)?;
            let tier: String = r
                .try_get("tier_at_execution")
                .map_err(crate::error::AppError::Database)?;
            let amount_cents: i64 = r
                .try_get("provisional_amount_cents")
                .map_err(crate::error::AppError::Database)?;
            let status: Option<String> = r
                .try_get("status")
                .map_err(crate::error::AppError::Database)?;
            Ok::<serde_json::Value, crate::error::AppError>(serde_json::json!({
                "created_at": created_at,
                "sub_id": sub_id,
                "tier": tier,
                "amount_cents": amount_cents,
                "status": status
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let pages = if total <= 0 {
        0
    } else {
        (total + limit - 1) / limit
    };

    Ok(Json(serde_json::json!({
        "commissions": commissions,
        "total": total,
        "page": page,
        "limit": limit,
        "pages": pages
    }))
    .into_response())
}

/// POST /api/affiliate/postback
/// Saves the affiliate's S2S postback webhook URL
pub async fn api_affiliate_postback_save(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<crate::rewards::models::PostbackPayload>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if active affiliate
    let affiliate = sqlx::query!(
        "SELECT referral_code FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if affiliate.is_none() {
        return Err(crate::error::AppError::Forbidden(
            "Only active affiliates can set postback URLs".into(),
        ));
    }

    let url = payload.postback_url.unwrap_or_default();
    let validated_url = service::validate_postback_url(&url).await?;
    let opt_url = if validated_url.is_empty() {
        None
    } else {
        Some(validated_url)
    };

    sqlx::query!(
        "UPDATE affiliates SET postback_url = $1 WHERE user_id = $2",
        opt_url,
        user_id
    )
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({"success": true})).into_response())
}

/// GET /api/affiliate/referrals
/// Provides a detailed list of referrals and their commissions, used for the Referrals & Payouts Funnel
pub async fn api_affiliate_referrals_list(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if affiliate is active
    if !is_active_affiliate(&state, user_id).await? {
        return Err(crate::error::AppError::Forbidden(
            "Only active affiliates can view referral details".into(),
        ));
    }

    // Join referrals with commissions
    let referrals = sqlx::query!(
        r#"SELECT 
               ar.id::text as referral_id,
               ar.status as referral_status,
               ar.created_at::text as created_at,
               ar.holdback_expires_at::text as holdback_expires_at,
               c.status as commission_status,
               COALESCE(c.provisional_amount_cents, 0) as provisional_amount_cents,
               u.email as referred_email
           FROM affiliate_referrals ar
           JOIN users u ON u.id = ar.referred_user_id
           LEFT JOIN affiliate_commissions c ON c.referral_id = ar.id
           WHERE ar.affiliate_id = $1
           ORDER BY ar.created_at DESC"#,
        user_id
    )
    .fetch_all(&state.db)
    .await?;

    let list: Vec<serde_json::Value> = referrals
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "referral_id": r.referral_id.clone(),
                "status": r.commission_status.clone().or(r.referral_status.clone()).unwrap_or_default(),
                "created_at": r.created_at.clone(),
                "holdback_expires_at": r.holdback_expires_at.clone(),
                "amount_cents": r.provisional_amount_cents,
                "email": r.referred_email.clone()
            })
        })
        .collect();

    Ok(Json(serde_json::json!({"success": true, "data": list})).into_response())
}

/// POST /api/affiliate/tax-document
/// Uploads a W-9 or W-8BEN tax form for the affiliate.
/// Stores the GCS path on the affiliate profile and is required before payout release.
pub async fn api_affiliate_upload_tax_document(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state)
        .await
        .map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Must be an active affiliate
    let aff = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if aff.is_none() {
        return Err(crate::error::AppError::Forbidden(
            "Only active affiliates can upload tax documents".into(),
        ));
    }

    let bucket = state.config.gcs_bucket.clone();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_filename = String::from("tax_document");

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "file" {
            original_filename = field.file_name().unwrap_or("tax_document").to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| crate::error::AppError::BadRequest("Failed to read file".into()))?;
            if bytes.len() > 10 * 1024 * 1024 {
                return Err(crate::error::AppError::BadRequest(
                    "File must be ≤ 10 MB".into(),
                ));
            }
            file_bytes = Some(bytes.to_vec());
        }
    }

    let file_bytes =
        file_bytes.ok_or_else(|| crate::error::AppError::BadRequest("No file uploaded".into()))?;

    // Sanitise filename and build the GCS object path
    let safe_name = original_filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let gcs_path = format!("affiliates/{}/tax_docs/{}", user_id, safe_name);

    // Upload to GCS if configured, otherwise fall back to local filesystem.
    let gcs_path = if let Some(ref b) = bucket {
        crate::storage::service::upload_private(
            b,
            &gcs_path,
            file_bytes,
            "application/octet-stream",
        )
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to upload tax document for affiliate {}: {}",
                user_id,
                e
            );
            crate::error::AppError::Internal("Upload failed".into())
        })?
    } else {
        crate::storage::service::upload_local(&gcs_path, file_bytes)
            .await
            .map_err(|e| {
                tracing::error!("Local tax doc save failed: {}", e);
                crate::error::AppError::Internal("Upload failed".into())
            })?
    };

    // Store the path on the affiliate record (non-macro: column added in migration 076)
    sqlx::query(
        "UPDATE affiliates SET tax_document_gcs_path = $1, updated_at = NOW() WHERE user_id = $2",
    )
    .bind(&gcs_path)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to store tax doc path: {}", e);
        crate::error::AppError::Internal("DB error".into())
    })?;

    tracing::info!(user_id = %user_id, gcs_path = %gcs_path, "Affiliate tax document uploaded");

    Ok(Json(serde_json::json!({"success": true, "gcs_path": gcs_path})).into_response())
}

/// POST /api/affiliate/materials/upload
/// Uploads a custom marketing material for admin review before use.
const AFFILIATE_MATERIAL_MAX_BYTES: usize = 20 * 1024 * 1024;

fn sanitize_affiliate_material_filename(filename: &str) -> String {
    let cleaned = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string();

    if cleaned.is_empty() {
        "material".to_string()
    } else {
        cleaned
    }
}

fn content_type_matches(declared: Option<&str>, allowed: &[&str]) -> bool {
    let Some(declared) = declared else {
        return true;
    };
    let declared = declared
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    allowed.iter().any(|candidate| declared == *candidate)
}

fn looks_like_safe_svg(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let normalized = text.trim_start().to_ascii_lowercase();
    (normalized.starts_with("<svg")
        || (normalized.starts_with("<?xml") && normalized.contains("<svg")))
        && !normalized.contains("<script")
        && !normalized.contains("onload=")
        && !normalized.contains("onerror=")
        && !normalized.contains("<foreignobject")
}

fn validate_affiliate_material_upload(
    filename: &str,
    declared_content_type: Option<&str>,
    bytes: &[u8],
) -> Result<&'static str, crate::error::AppError> {
    if bytes.is_empty() {
        return Err(crate::error::AppError::BadRequest(
            "Uploaded file is empty".into(),
        ));
    }
    if bytes.len() > AFFILIATE_MATERIAL_MAX_BYTES {
        return Err(crate::error::AppError::BadRequest(
            "File must be ≤ 20 MB".into(),
        ));
    }

    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let detected = match extension.as_str() {
        "png" if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) => {
            ("image/png", &["image/png"][..])
        }
        "jpg" | "jpeg" if bytes.starts_with(&[0xff, 0xd8, 0xff]) => {
            ("image/jpeg", &["image/jpeg", "image/jpg"][..])
        }
        "pdf" if bytes.starts_with(b"%PDF-") => ("application/pdf", &["application/pdf"][..]),
        "svg" if looks_like_safe_svg(bytes) => (
            "image/svg+xml",
            &["image/svg+xml", "text/xml", "application/xml"][..],
        ),
        "mp4" if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" => ("video/mp4", &["video/mp4"][..]),
        "zip"
            if bytes.starts_with(b"PK\x03\x04")
                || bytes.starts_with(b"PK\x05\x06")
                || bytes.starts_with(b"PK\x07\x08") =>
        {
            (
                "application/zip",
                &["application/zip", "application/x-zip-compressed"][..],
            )
        }
        _ => {
            return Err(crate::error::AppError::BadRequest(
                "Unsupported file type. Upload PNG, JPG, SVG, PDF, MP4, or ZIP.".into(),
            ))
        }
    };

    if !content_type_matches(declared_content_type, detected.1) {
        return Err(crate::error::AppError::BadRequest(
            "Declared content type does not match the uploaded file.".into(),
        ));
    }

    Ok(detected.0)
}

/// GET /api/affiliate/materials
/// Lists the authenticated affiliate's custom marketing material review statuses.
pub async fn api_affiliate_materials_list(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, crate::error::AppError> {
    use sqlx::Row;

    let user_id = require_active_affiliate_user_id(&jar, &state).await?;

    let rows = sqlx::query(
        r#"SELECT id::text, asset_name, status, review_note, created_at, reviewed_at
           FROM affiliate_materials
           WHERE affiliate_id = $1
           ORDER BY created_at DESC
           LIMIT 50"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let materials: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "asset_name": row.get::<String, _>("asset_name"),
                "status": row.get::<String, _>("status"),
                "review_note": row.get::<Option<String>, _>("review_note"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
                "reviewed_at": row
                    .get::<Option<chrono::DateTime<chrono::Utc>>, _>("reviewed_at")
                    .map(|ts| ts.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "materials": materials })).into_response())
}

pub async fn api_affiliate_upload_material(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_active_affiliate_user_id(&jar, &state).await?;

    let bucket = state.config.gcs_bucket.clone();

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_filename = String::from("material");
    let mut declared_content_type: Option<String> = None;
    let mut asset_name = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "file" => {
                original_filename = field.file_name().unwrap_or("material").to_string();
                declared_content_type = field.content_type().map(|ct| ct.to_string());
                let bytes = field.bytes().await.map_err(|_| {
                    crate::error::AppError::BadRequest("Failed to read file".into())
                })?;
                file_bytes = Some(bytes.to_vec());
            }
            "name" => {
                asset_name = field
                    .text()
                    .await
                    .map_err(|_| {
                        crate::error::AppError::BadRequest("Invalid material name".into())
                    })?
                    .trim()
                    .to_string();
            }
            _ => {}
        }
    }

    let file_bytes =
        file_bytes.ok_or_else(|| crate::error::AppError::BadRequest("No file uploaded".into()))?;

    if asset_name.is_empty() {
        asset_name = original_filename.clone();
    }
    if asset_name.chars().count() > 120 {
        return Err(crate::error::AppError::BadRequest(
            "Material name must be 120 characters or fewer".into(),
        ));
    }

    let content_type = validate_affiliate_material_upload(
        &original_filename,
        declared_content_type.as_deref(),
        &file_bytes,
    )?;
    let safe_name = sanitize_affiliate_material_filename(&original_filename);
    let gcs_path = format!(
        "affiliates/{}/materials/{}_{}",
        user_id,
        uuid::Uuid::new_v4().as_fields().0,
        safe_name
    );

    let gcs_path = if let Some(ref b) = bucket {
        crate::storage::service::upload_private(b, &gcs_path, file_bytes, content_type)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload affiliate material: {}", e);
                crate::error::AppError::Internal("Upload failed".into())
            })?
    } else {
        crate::storage::service::upload_local(&gcs_path, file_bytes)
            .await
            .map_err(|e| {
                tracing::error!("Local affiliate material save failed: {}", e);
                crate::error::AppError::Internal("Upload failed".into())
            })?
    };

    // Insert into affiliate_materials table (non-macro: table added in migration 076)
    let material_id: String = sqlx::query_scalar(
        r#"INSERT INTO affiliate_materials (affiliate_id, asset_name, gcs_path, status)
           VALUES ($1, $2, $3, 'pending_review')
           RETURNING id::text"#,
    )
    .bind(user_id)
    .bind(&asset_name)
    .bind(&gcs_path)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert material: {}", e);
        crate::error::AppError::Internal("DB error".into())
    })?;

    // Notify admin
    let _ = crate::common::email::send_email(
        "admin@poool.app",
        "New Affiliate Marketing Material Pending Review",
        &format!("<p>Affiliate <b>{}</b> has uploaded a custom marketing material named \"<b>{}</b>\" pending your review.</p><p>Please log into the Admin Affiliate Compliance panel to approve or reject it.</p>", user_id, asset_name)
    ).await;

    tracing::info!(user_id = %user_id, gcs_path = %gcs_path, "Affiliate material uploaded for review");

    Ok(Json(serde_json::json!({
        "success": true,
        "material_id": material_id,
        "status": "pending_review",
        "content_type": content_type
    }))
    .into_response())
}

#[cfg(test)]
mod affiliate_material_upload_tests {
    use super::validate_affiliate_material_upload;

    #[test]
    fn accepts_png_with_matching_signature() {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(b"data");
        assert_eq!(
            validate_affiliate_material_upload("banner.png", Some("image/png"), &bytes).unwrap(),
            "image/png"
        );
    }

    #[test]
    fn rejects_html_disguised_as_svg() {
        let err = validate_affiliate_material_upload(
            "creative.svg",
            Some("image/svg+xml"),
            br#"<svg><script>alert(1)</script></svg>"#,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("Unsupported file type"));
    }

    #[test]
    fn rejects_declared_content_type_mismatch() {
        let err = validate_affiliate_material_upload("guide.pdf", Some("image/png"), b"%PDF-1.4\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Declared content type"));
    }

    #[test]
    fn rejects_unknown_extension_even_with_bytes() {
        let err =
            validate_affiliate_material_upload("payload.html", Some("text/html"), b"<html></html>")
                .unwrap_err()
                .to_string();
        assert!(err.contains("Unsupported file type"));
    }
}
