use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;

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
pub async fn page_affiliate_promo(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-promo.html").await
}

/// GET /affiliate/onboarding — Render the 5-step compliance wizard (Phase 19).
pub async fn page_affiliate_onboarding(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-onboarding.html").await
}

/// GET /affiliate/dashboard — Render the affiliate dashboard page (Phase 19).
pub async fn page_affiliate_dashboard(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-dashboard.html").await
}

/// GET /affiliate/referrals — Render the affiliate referrals & payouts page (Phase 19).
pub async fn page_affiliate_referrals(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-referrals.html").await
}

/// GET /affiliate/materials — Render the affiliate marketing materials page (Phase 19).
pub async fn page_affiliate_materials(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-materials.html").await
}

/// GET /affiliate/settings — Render the affiliate settings and tax info page (Phase 19).
pub async fn page_affiliate_settings(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "affiliate-settings.html").await
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
    let ip = raw_ip.split(',').last().unwrap_or("0.0.0.0").trim().to_string();

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Trivial click fraud prevention: if they already have the exact same cookie, don't re-record the click.
    let already_clicked = jar.get("poool_ref").map(|c| c.value()) == Some(&code_clone);

    // Record the click in the background only if it's uniquely new and IP isn't hammering us
    if !already_clicked {
        // Enforce a strict click rate limit of 10 clicks per IP per 15 minutes globally to stop bot swarms
        if state.auth_rate_limiter.check(&format!("click_throttle:{}", ip)).await.is_ok() {
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
    let cookie_val = format!("{}|{}|{}", 
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
        tracing::warn!("Rate limit exceeded for affiliate onboarding: user={}", user_id);
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
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Exam not passed"})),
        )
            .into_response();
    }

    // (Exam Answers validation block removed as it was deprecated in model updates)

    // ── Policy Acceptance Validation ────────────────────────────────
    if form.accepted_policies.len() != REQUIRED_POLICIES.len() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("All {} policies must be accepted.", REQUIRED_POLICIES.len())
            })),
        )
            .into_response();
    }

    for required in REQUIRED_POLICIES {
        if !form.accepted_policies.iter().any(|p| p == required) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Missing required policy acceptance: {}", required)
                })),
            )
                .into_response();
        }
    }

    // ── Input Validation ────────────────────────────────────────────
    if form.traffic_source.trim().is_empty() || form.audience_size.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Traffic source and audience size are required."})),
        )
            .into_response();
    }

    let main_url = form.main_url.trim();
    if main_url.is_empty() || (!main_url.starts_with("http://") && !main_url.starts_with("https://")) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "A valid URL starting with http:// or https:// is required."})),
        )
            .into_response();
    }

    if form.phone_number.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Phone number is required."})),
        )
            .into_response();
    }

    if form.tax_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Tax ID is required."})),
        )
            .into_response();
    }

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // ── Duplicate Application Guard ─────────────────────────────────
    // Block resubmission if already pending_approval or active
    let existing_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM affiliates WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

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

    let result = sqlx::query!(
        r#"
        INSERT INTO affiliates (
            user_id, referral_code, status,
            traffic_source, audience_size, main_url, phone_number, tax_id, company_name
        )
        VALUES ($1, $2, 'pending_approval', $3, $4, $5, $6, $7, $8)
        ON CONFLICT (user_id) DO UPDATE SET 
            status = 'pending_approval',
            traffic_source = EXCLUDED.traffic_source,
            audience_size = EXCLUDED.audience_size,
            main_url = EXCLUDED.main_url,
            phone_number = EXCLUDED.phone_number,
            tax_id = EXCLUDED.tax_id,
            company_name = EXCLUDED.company_name
        "#,
        user_id,
        temp_code,
        form.traffic_source.trim(),
        form.audience_size.trim(),
        main_url,
        form.phone_number.trim(),
        form.tax_id.trim(),
        form.company_name.as_deref().map(|s| s.trim())
    )
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
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if active affiliate
    let affiliate = sqlx::query!(
        "SELECT referral_code FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    let affiliate = match affiliate {
        Some(a) => a,
        None => return Err(crate::error::AppError::Forbidden("Only active affiliates can request payouts".into())),
    };

    // Calculate payable amount
    let payable: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(provisional_amount_cents), 0)::bigint FROM affiliate_commissions WHERE affiliate_id = $1 AND status = 'payable'",
        user_id
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);

    if payable < 5000 {
        return Err(crate::error::AppError::BadRequest("A minimum of $50 in payable commissions is required to request a payout.".into()));
    }

    // Get user email
    let user_email = sqlx::query_scalar!("SELECT email FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or_default()
        .unwrap_or_else(|| "Unknown".to_string());

    // Send email to admin
    let amount_dollars = (payable as f64) / 100.0;
    let _ = crate::common::email::send_email(
        "admin@poool.app",
        "Affiliate Commission Payout Request",
        &format!(
            "<h3>Payout Request</h3><p>Affiliate <b>{}</b> (code: <code>{}</code>) has manually requested a payout of their payable commissions totaling <b>${:.2}</b>.</p><p>Please log into the Admin Rewards Dashboard under the <b>Pending Payouts</b> tab to approve and batch this payout to their cash wallet.</p>",
            user_email, affiliate.referral_code, amount_dollars
        )
    ).await;

    Ok(Json(serde_json::json!({"success": true, "message": "Admin notified for approval"})).into_response())
}

/// GET /api/affiliate/subid-stats
/// Returns clicks and registrations grouped by SubID.
pub async fn api_affiliate_subid_stats(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if active affiliate
    let affiliate = sqlx::query!(
        "SELECT referral_code FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    let affiliate = match affiliate {
        Some(a) => a,
        None => return Err(crate::error::AppError::Forbidden("Only active affiliates can view subID stats".into())),
    };

    // Query clicks grouped by subid (non-macro: avoids COALESCE/COUNT type inference issues)
    use sqlx::Row as _;
    let clicks = sqlx::query(
        r#"SELECT COALESCE(subid, 'unknown') as sub_id, COUNT(*)::bigint as click_count
           FROM referral_clicks
           WHERE code = $1
           GROUP BY sub_id"#
    )
    .bind(&affiliate.referral_code)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Query registrations grouped by subid
    let regs = sqlx::query(
        r#"SELECT COALESCE(sub_id, 'unknown') as sub_id, COUNT(*)::bigint as reg_count
           FROM affiliate_referrals
           WHERE affiliate_id = $1
           GROUP BY sub_id"#
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

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
    .await
    .unwrap_or_default();

    // Combine into a map
    use std::collections::HashMap;
    let mut stats: HashMap<String, serde_json::Value> = HashMap::new();

    for c in &clicks {
        let subid: String = c.try_get("sub_id").unwrap_or_else(|_| "unknown".to_string());
        let clicks_count: i64 = c.try_get("click_count").unwrap_or(0);
        stats.insert(subid.clone(), serde_json::json!({
            "sub_id": subid,
            "clicks": clicks_count,
            "registrations": 0,
            "earned_cents": 0,
            "pending_cents": 0
        }));
    }

    for r in &regs {
        let subid: String = r.try_get("sub_id").unwrap_or_else(|_| "unknown".to_string());
        let reg_count: i64 = r.try_get("reg_count").unwrap_or(0);
        if let Some(entry) = stats.get_mut(&subid) {
            entry.as_object_mut().unwrap().insert("registrations".to_string(), serde_json::json!(reg_count));
        } else {
            stats.insert(subid.clone(), serde_json::json!({
                "sub_id": subid,
                "clicks": 0,
                "registrations": reg_count,
                "earned_cents": 0,
                "pending_cents": 0
            }));
        }
    }

    for rev in &revenues {
        let subid: String = rev.try_get("sub_id").unwrap_or_else(|_| "unknown".to_string());
        let earned: i64 = rev.try_get("earned_cents").unwrap_or(0);
        let pending: i64 = rev.try_get("pending_cents").unwrap_or(0);
        if let Some(entry) = stats.get_mut(&subid) {
            let obj = entry.as_object_mut().unwrap();
            obj.insert("earned_cents".to_string(), serde_json::json!(earned));
            obj.insert("pending_cents".to_string(), serde_json::json!(pending));
        } else {
            stats.insert(subid.clone(), serde_json::json!({
                "sub_id": subid,
                "clicks": 0,
                "registrations": 0,
                "earned_cents": earned,
                "pending_cents": pending
            }));
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
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Verify affiliate is active
    let aff = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if aff.is_none() {
        return Err(crate::error::AppError::Forbidden("Only active affiliates can re-accept policies".into()));
    }

    // Validate all required policies are present
    if form.accepted_policies.len() != REQUIRED_POLICIES.len() {
        return Err(crate::error::AppError::BadRequest(
            format!("All {} policies must be accepted.", REQUIRED_POLICIES.len())
        ));
    }

    for required in REQUIRED_POLICIES {
        if !form.accepted_policies.iter().any(|p| p == required) {
            return Err(crate::error::AppError::BadRequest(
                format!("Missing required policy: {}", required)
            ));
        }
    }

    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut tx = state.db.begin().await.map_err(|_| crate::error::AppError::Internal("Transaction error".into()))?;

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
        "UPDATE affiliates SET accepted_policy_version = $1, updated_at = NOW() WHERE user_id = $2"
    )
    .bind(service::CURRENT_POLICY_VERSION)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| crate::error::AppError::Internal("Failed to update policy version".into()))?;

    tx.commit().await.map_err(|_| crate::error::AppError::Internal("Commit error".into()))?;

    tracing::info!(user_id = %user_id, version = service::CURRENT_POLICY_VERSION, "Affiliate re-accepted policies");

    Ok(Json(serde_json::json!({"success": true, "accepted_version": service::CURRENT_POLICY_VERSION})).into_response())
}

/// GET /api/affiliate/commissions/export
/// Exports affiliate commissions as a CSV file for download.
pub async fn api_affiliate_commissions_export(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    let affiliate = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if affiliate.is_none() {
        return Err(crate::error::AppError::Forbidden("Only active affiliates can export commissions".into()));
    }

    let date_from = params
        .get("from")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let date_to = params
        .get("to")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let page: i64 = params.get("page").and_then(|p| p.parse().ok()).unwrap_or(1).max(1);
    let limit: i64 = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(50).min(200);
    let offset = (page - 1) * limit;

    let fmt = params.get("format").map(|s| s.as_str()).unwrap_or("json");

    // Build query with optional date filters and pagination
    // Non-macro: avoids COALESCE/date-param type inference issues at compile time
    use sqlx::Row as _;
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
           LIMIT $4 OFFSET $5"#
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint FROM affiliate_commissions ac
           WHERE ac.affiliate_id = $1
             AND ($2::date IS NULL OR ac.created_at::date >= $2)
             AND ($3::date IS NULL OR ac.created_at::date <= $3)"#
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0i64);

    if fmt == "csv" {
        // Return as downloadable CSV
        let mut csv = "date,sub_id,tier,amount_usd,status\n".to_string();
        for r in &rows {
            let created_at: String = r.try_get("created_at").unwrap_or_default();
            let sub_id: String = r.try_get("sub_id").unwrap_or_default();
            let tier: String = r.try_get("tier_at_execution").unwrap_or_default();
            let amount_cents: i64 = r.try_get("provisional_amount_cents").unwrap_or(0);
            let status: String = r.try_get::<Option<String>, _>("status").unwrap_or_default().unwrap_or_default();
            csv.push_str(&format!(
                "{},{},{},{:.2},{}\n",
                created_at,
                sub_id,
                tier,
                amount_cents as f64 / 100.0,
                status
            ));
        }

        return Ok((
            axum::http::StatusCode::OK,
            [
                ("Content-Type", "text/csv"),
                ("Content-Disposition", "attachment; filename=\"commissions.csv\""),
            ],
            csv,
        )
            .into_response());
    }

    let commissions: Vec<serde_json::Value> = rows.iter().map(|r| {
        let created_at: String = r.try_get("created_at").unwrap_or_default();
        let sub_id: String = r.try_get("sub_id").unwrap_or_default();
        let tier: String = r.try_get("tier_at_execution").unwrap_or_default();
        let amount_cents: i64 = r.try_get("provisional_amount_cents").unwrap_or(0);
        let status: Option<String> = r.try_get("status").unwrap_or_default();
        serde_json::json!({
            "created_at": created_at,
            "sub_id": sub_id,
            "tier": tier,
            "amount_cents": amount_cents,
            "status": status
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "commissions": commissions,
        "total": total,
        "page": page,
        "limit": limit,
        "pages": (total as f64 / limit as f64).ceil() as i64
    })).into_response())
}

/// POST /api/affiliate/postback
/// Saves the affiliate's S2S postback webhook URL
pub async fn api_affiliate_postback_save(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<crate::rewards::models::PostbackPayload>,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if active affiliate
    let affiliate = sqlx::query!(
        "SELECT referral_code FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if affiliate.is_none() {
        return Err(crate::error::AppError::Forbidden("Only active affiliates can set postback URLs".into()));
    }

    let url = payload.postback_url.unwrap_or_default().trim().to_string();
    let opt_url = if url.is_empty() { None } else { Some(url) };

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
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Check if affiliate is active
    let affiliate = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if affiliate.is_none() {
        return Err(crate::error::AppError::Forbidden("Only active affiliates can view referral details".into()));
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
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    // Must be an active affiliate
    let aff = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if aff.is_none() {
        return Err(crate::error::AppError::Forbidden("Only active affiliates can upload tax documents".into()));
    }

    let bucket = match &state.config.gcs_bucket {
        Some(b) => b.clone(),
        None => return Err(crate::error::AppError::Internal("File storage not configured".into())),
    };

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_filename = String::from("tax_document");

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "file" {
            original_filename = field.file_name().unwrap_or("tax_document").to_string();
            let bytes = field.bytes().await.map_err(|_| crate::error::AppError::BadRequest("Failed to read file".into()))?;
            if bytes.len() > 10 * 1024 * 1024 {
                return Err(crate::error::AppError::BadRequest("File must be ≤ 10 MB".into()));
            }
            file_bytes = Some(bytes.to_vec());
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| crate::error::AppError::BadRequest("No file uploaded".into()))?;

    // Sanitise filename and build the GCS object path
    let safe_name = original_filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect::<String>();
    let gcs_path = format!("affiliates/{}/tax_docs/{}", user_id, safe_name);

    // Upload to GCS (private bucket)
    crate::storage::service::upload_private(&bucket, &gcs_path, file_bytes, "application/octet-stream")
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload tax document for affiliate {}: {}", user_id, e);
            crate::error::AppError::Internal("Upload failed".into())
        })?;

    // Store the path on the affiliate record (non-macro: column added in migration 076)
    sqlx::query(
        "UPDATE affiliates SET tax_document_gcs_path = $1, updated_at = NOW() WHERE user_id = $2"
    )
    .bind(&gcs_path)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| { tracing::error!("Failed to store tax doc path: {}", e); crate::error::AppError::Internal("DB error".into()) })?;

    tracing::info!(user_id = %user_id, gcs_path = %gcs_path, "Affiliate tax document uploaded");

    Ok(Json(serde_json::json!({"success": true, "gcs_path": gcs_path})).into_response())
}

/// POST /api/affiliate/materials/upload
/// Uploads a custom marketing material for admin review before use.
pub async fn api_affiliate_upload_material(
    jar: CookieJar,
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<axum::response::Response, crate::error::AppError> {
    let user_id = require_user_id(&jar, &state).await.map_err(|_| crate::error::AppError::Unauthorized("Invalid session".into()))?;

    let aff = sqlx::query!(
        "SELECT user_id FROM affiliates WHERE user_id = $1 AND status = 'active'",
        user_id
    )
    .fetch_optional(&state.db)
    .await?;

    if aff.is_none() {
        return Err(crate::error::AppError::Forbidden("Only active affiliates can upload marketing materials".into()));
    }

    let bucket = match &state.config.gcs_bucket {
        Some(b) => b.clone(),
        None => return Err(crate::error::AppError::Internal("File storage not configured".into())),
    };

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_filename = String::from("material");
    let mut asset_name = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "file" => {
                original_filename = field.file_name().unwrap_or("material").to_string();
                let bytes = field.bytes().await.map_err(|_| crate::error::AppError::BadRequest("Failed to read file".into()))?;
                if bytes.len() > 20 * 1024 * 1024 {
                    return Err(crate::error::AppError::BadRequest("File must be ≤ 20 MB".into()));
                }
                file_bytes = Some(bytes.to_vec());
            }
            "name" => {
                asset_name = field.text().await.unwrap_or_default().trim().to_string();
            }
            _ => {}
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| crate::error::AppError::BadRequest("No file uploaded".into()))?;

    if asset_name.is_empty() {
        asset_name = original_filename.clone();
    }

    let safe_name = original_filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect::<String>();
    let gcs_path = format!(
        "affiliates/{}/materials/{}_{}",
        user_id,
        uuid::Uuid::new_v4().as_fields().0,
        safe_name
    );

    crate::storage::service::upload_private(&bucket, &gcs_path, file_bytes, "application/octet-stream")
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload affiliate material: {}", e);
            crate::error::AppError::Internal("Upload failed".into())
        })?;

    // Insert into affiliate_materials table (non-macro: table added in migration 076)
    let material_id: String = sqlx::query_scalar(
        r#"INSERT INTO affiliate_materials (affiliate_id, asset_name, gcs_path, status)
           VALUES ($1, $2, $3, 'pending_review')
           RETURNING id::text"#
    )
    .bind(user_id)
    .bind(&asset_name)
    .bind(&gcs_path)
    .fetch_one(&state.db)
    .await
    .map_err(|e| { tracing::error!("Failed to insert material: {}", e); crate::error::AppError::Internal("DB error".into()) })?;

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
        "status": "pending_review"
    })).into_response())
}
