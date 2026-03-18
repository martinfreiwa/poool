use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;

use crate::auth::middleware;
use crate::auth::routes::AppState;

use super::service;

/// Helper to extract the current user ID from session cookie.
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

/// GET /leaderboard — Render the leaderboard HTML page.
pub async fn page_leaderboard(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "leaderboard.html").await
}

/// GET /api/leaderboard — Get rankings.
/// Query params: ?metric=invested|assets|roi|affiliates|revenue|highest_inv&page=1
pub async fn get_rankings(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let metric_type = params
        .get("metric")
        .cloned()
        .unwrap_or_else(|| "invested".to_string());
    let timeframe = params
        .get("timeframe")
        .cloned()
        .unwrap_or_else(|| "alltime".to_string());
    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: i64 = 50;

    let tier_id: Option<i32> = params.get("tier_id").and_then(|t| t.parse().ok());
    let search: Option<String> = params
        .get("search")
        .filter(|s| !s.trim().is_empty())
        .cloned();

    match service::get_rankings(
        &state.db, user_id, &metric_type, &timeframe, page, per_page, tier_id, search,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(e) => {
            tracing::error!("Failed to get leaderboard rankings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load leaderboard."})),
            )
                .into_response()
        }
    }
}

/// GET /api/leaderboard/me — Get the current user's rank.
/// Query params: ?metric=invested|assets|roi|affiliates|revenue|highest_inv
pub async fn get_my_rank(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let metric_type = params
        .get("metric")
        .cloned()
        .unwrap_or_else(|| "invested".to_string());
    let timeframe = params
        .get("timeframe")
        .cloned()
        .unwrap_or_else(|| "alltime".to_string());

    match service::get_user_rank(&state.db, user_id, &metric_type, &timeframe).await {
        Ok(rank) => Json(rank).into_response(),
        Err(e) => {
            tracing::error!("Failed to get user rank for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load rank."})),
            )
                .into_response()
        }
    }
}

/// GET /api/leaderboard/preferences — Get the user's visibility preferences.
pub async fn get_preferences(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_preferences(&state.db, user_id).await {
        Ok(prefs) => Json(prefs).into_response(),
        Err(e) => {
            tracing::error!("Failed to get leaderboard preferences: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load preferences."})),
            )
                .into_response()
        }
    }
}

/// PUT /api/leaderboard/preferences — Update visibility preferences.
pub async fn update_preferences(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(req): Json<super::models::UpdatePreferencesRequest>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_preferences(&state.db, user_id, &req).await {
        Ok(prefs) => Json(prefs).into_response(),
        Err(e) => {
            tracing::error!("Failed to update leaderboard preferences: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to update preferences."})),
            )
                .into_response()
        }
    }
}

/// GET /api/leaderboard/refresh — Trigger a score refresh (admin only or rate-limited).
pub async fn trigger_refresh(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    // Only admins can trigger manual refresh
    if !middleware::is_admin(&jar, &state.db).await {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required."})),
        )
            .into_response();
    }

    match service::refresh_all_scores(&state.db).await {
        Ok(()) => Json(
            serde_json::json!({"status": "success", "message": "Leaderboard scores refreshed."}),
        )
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to refresh leaderboard scores: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to refresh scores."})),
            )
                .into_response()
        }
    }
}
