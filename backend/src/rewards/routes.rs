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

    // Extract IP and UserAgent
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Record the click in the background
    tokio::spawn(async move {
        let _ = sqlx::query(
            r#"
            INSERT INTO referral_clicks (code, ip_address, user_agent, subid)
            SELECT $1::varchar, $2::inet, $3, $4
            WHERE EXISTS (SELECT 1 FROM referral_codes WHERE code = $1::varchar)
            "#,
        )
        .bind(code_clone)
        .bind(ip)
        .bind(user_agent)
        .bind(subid_clone)
        .execute(&state.db)
        .await;
    });

    // If subid is present, store it as code|subid. Otherwise, just code.
    let cookie_val = if let Some(sid) = subid {
        format!("{}|{}", code, sid)
    } else {
        code
    };

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
