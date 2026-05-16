use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
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

/// Enforce the leaderboard rate-limit for `user_id`. On overflow returns a
/// 429 response that the caller should propagate. Key is namespaced per
/// endpoint so the GET/PUT/POST caps don't share a single bucket.
async fn check_rate_limit(
    state: &AppState,
    user_id: uuid::Uuid,
    endpoint: &str,
) -> Result<(), axum::response::Response> {
    let key = format!("lb:{}:{}", endpoint, user_id);
    if let Err(retry_after) = state.leaderboard_rate_limiter.check(&key).await {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Retry-After", retry_after.to_string()),
                (
                    "Cache-Control",
                    "no-store, no-cache, must-revalidate".to_string(),
                ),
            ],
            Json(serde_json::json!({
                "error": "Rate limited. Please retry shortly.",
                "retry_after_seconds": retry_after,
            })),
        )
            .into_response());
    }
    Ok(())
}

/// Build a stable ETag from the cached `last_updated` timestamp and the
/// concrete request parameters. Two requests with identical params + the
/// same `computed_at` get the same ETag → 304 hot path. Any score refresh
/// or filter change naturally invalidates the tag.
fn build_etag(
    last_updated_iso: Option<&str>,
    metric: &str,
    timeframe: &str,
    page: i64,
    per_page: i64,
    tier_id: Option<i32>,
    search: Option<&str>,
) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    last_updated_iso.unwrap_or("cold").hash(&mut h);
    metric.hash(&mut h);
    timeframe.hash(&mut h);
    page.hash(&mut h);
    per_page.hash(&mut h);
    tier_id.unwrap_or(0).hash(&mut h);
    search.unwrap_or("").hash(&mut h);
    // Quote so it's a valid strong ETag per RFC 7232 § 2.3.
    format!("\"lb-{:x}\"", h.finish())
}

/// GET /leaderboard — Render the leaderboard HTML page.
pub async fn page_leaderboard(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "leaderboard.html").await
}

/// GET /api/leaderboard — Get rankings.
/// Query params: ?metric=invested|assets|roi|affiliates|revenue|highest_inv&page=1
#[tracing::instrument(
    skip(jar, state, params, req_headers),
    fields(user_id, metric, timeframe, page)
)]
pub async fn get_rankings(
    jar: CookieJar,
    State(state): State<AppState>,
    req_headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

    if let Err(resp) = check_rate_limit(&state, user_id, "get").await {
        return resp;
    }

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
    let per_page: i64 = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(10)
        .max(1)
        .min(100);

    tracing::Span::current().record("metric", &metric_type);
    tracing::Span::current().record("timeframe", &timeframe);
    tracing::Span::current().record("page", page);

    let tier_id: Option<i32> = params.get("tier_id").and_then(|t| t.parse().ok());
    let search: Option<String> = params
        .get("search")
        .filter(|s| !s.trim().is_empty())
        .cloned();

    // ETag short-circuit: hash the resolved last_updated + concrete query
    // params. Browsers send back the value via If-None-Match; on a match we
    // skip the DB query entirely with 304 Not Modified. Browsers also reuse
    // the cached body for `max-age` seconds with no network at all.
    //
    // We resolve via the service helper (cache-warm on hit, single MAX()
    // on cold cache) so the ETag is stable across the very first request
    // and its immediate retry — otherwise the cold-then-warm transition
    // would invalidate the tag on the second call.
    let last_updated_snapshot =
        service::resolve_last_updated(&state.db, Some(&state.leaderboard_last_refresh))
            .await
            .unwrap_or(None);
    let etag = build_etag(
        last_updated_snapshot.as_deref(),
        &metric_type,
        &timeframe,
        page,
        per_page,
        tier_id,
        search.as_deref(),
    );
    if let Some(inm) = req_headers.get(axum::http::header::IF_NONE_MATCH) {
        if inm.to_str().ok() == Some(etag.as_str()) {
            let mut headers = HeaderMap::new();
            if let Ok(h) = HeaderValue::from_str(&etag) {
                headers.insert(axum::http::header::ETAG, h);
            }
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=30"),
            );
            return (StatusCode::NOT_MODIFIED, headers).into_response();
        }
    }

    match service::get_rankings(
        &state.db,
        user_id,
        &metric_type,
        &timeframe,
        page,
        per_page,
        tier_id,
        search,
        Some(&state.leaderboard_last_refresh),
    )
    .await
    {
        Ok(response) => {
            let mut headers = HeaderMap::new();
            if let Ok(h) = HeaderValue::from_str(&etag) {
                headers.insert(axum::http::header::ETAG, h);
            }
            // `private` so a shared CDN never serves one user's listing to
            // another — visibility prefs are personal. `max-age=30` keeps
            // browsers from spamming the API while score data is fresh.
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=30"),
            );
            (headers, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get leaderboard rankings");
            sentry::capture_message(
                &format!("Leaderboard listing failed: {}", e),
                sentry::Level::Error,
            );
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

/// GET /api/leaderboard/snapshots/me — Return the viewer's rank trajectory
/// for a given metric over the last `days` window. Used by the UI to render
/// "+3 ranks this week" sparklines.
///
/// Query params:
///   - `metric` (default `invested`) — one of the six allowlisted metrics.
///   - `days`   (default 30, max 400) — how far back to look. Caps match
///     the 13-month snapshot retention so the upper bound never returns
///     truncated history.
///
/// Privacy: returns ONLY the viewer's own snapshots; never accepts a
/// `user_id` query param. Rate-limited with the standard leaderboard
/// limiter under key `lb:snapshots:<user_id>`.
pub async fn get_my_snapshots(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if let Err(resp) = check_rate_limit(&state, user_id, "snapshots").await {
        return resp;
    }

    let metric = params
        .get("metric")
        .cloned()
        .unwrap_or_else(|| "invested".to_string());
    let days: i64 = params
        .get("days")
        .and_then(|d| d.parse().ok())
        .unwrap_or(30);

    match service::get_user_snapshots(&state.db, user_id, &metric, days).await {
        Ok(points) => {
            let mut headers = HeaderMap::new();
            // Snapshots are append-only daily — yesterday's data won't change.
            // 5 min cache balances trend freshness vs request load.
            headers.insert(
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("private, max-age=300"),
            );
            (
                headers,
                Json(serde_json::json!({
                    "metric": metric,
                    "days": days,
                    "points": points,
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, "Failed to get user snapshots");
            sentry::capture_message(
                &format!("Leaderboard snapshot read failed: {}", e),
                sentry::Level::Error,
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load snapshots."})),
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
#[tracing::instrument(skip(jar, state, req_headers, req), fields(user_id))]
pub async fn update_preferences(
    jar: CookieJar,
    State(state): State<AppState>,
    req_headers: HeaderMap,
    Json(req): Json<super::models::UpdatePreferencesRequest>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    tracing::Span::current().record("user_id", tracing::field::display(user_id));

    if let Err(resp) = check_rate_limit(&state, user_id, "prefs").await {
        return resp;
    }

    match service::update_preferences(&state.db, user_id, &req).await {
        Ok(prefs) => {
            // Audit-log every successful preference change. Captures actor,
            // IP and UA so privacy-related toggles (visible/show_avatar/
            // display_name) leave an immutable trail per regulatory ask.
            // `audit::log` failures are warned but do not block the response
            // — the user's pref save is already committed.
            let ip = req_headers
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim().to_string());
            let ua = req_headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());
            let action = format!(
                "leaderboard.prefs.update visible={:?} show_avatar={:?} display_name_set={}",
                req.visible,
                req.show_avatar,
                req.display_name.is_some(),
            );
            if let Err(e) = crate::common::audit::log(
                &state.db,
                Some(user_id),
                &action,
                "leaderboard_preferences",
                Some(user_id),
                ip.as_deref(),
                ua.as_deref(),
            )
            .await
            {
                tracing::warn!(error = %e, user_id = %user_id, "audit log for leaderboard prefs failed");
            }
            Json(prefs).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, "Failed to update leaderboard preferences");
            sentry::capture_message(
                &format!("Leaderboard prefs update failed: {}", e),
                sentry::Level::Error,
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to update preferences."})),
            )
                .into_response()
        }
    }
}

/// POST /api/leaderboard/refresh — Trigger a score refresh (admin only).
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

    match service::refresh_all_scores_and_cache(&state.db, &state.leaderboard_last_refresh).await {
        Ok(()) => Json(
            serde_json::json!({"status": "success", "message": "Leaderboard scores refreshed."}),
        )
        .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to refresh leaderboard scores");
            sentry::capture_message(
                &format!("Manual leaderboard refresh failed: {}", e),
                sentry::Level::Error,
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to refresh scores."})),
            )
                .into_response()
        }
    }
}
