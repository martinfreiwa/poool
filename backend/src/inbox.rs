//! Phase-3 P1: in-app inbox (bell icon + dropdown).
//!
//! Routes:
//!   GET    /api/inbox                 — paginated list (?limit, ?before)
//!   GET    /api/inbox/unread-count    — { count: number } for the badge
//!   POST   /api/inbox/:id/read        — mark one read
//!   POST   /api/inbox/read-all        — mark all read
//!
//! All routes require a logged-in user (session cookie). The `community`
//! module owns its own separate `/api/community/notifications` surface
//! (different DB pool). The inbox here is for platform-wide events:
//! KYC, payouts, investments, **affiliate commission earned**, payout
//! released, team membership changes.

use crate::auth::middleware;
use crate::auth::routes::AppState;
use crate::common::notifications as svc;
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    /// ISO 8601 — `created_at` of the last row the client already has.
    before: Option<chrono::DateTime<chrono::Utc>>,
}

async fn list_inbox(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "auth required"})),
            )
                .into_response()
        }
    };
    match svc::list_notifications_for_user(&state.db, user.id, q.limit.unwrap_or(25), q.before)
        .await
    {
        Ok(rows) => Json(serde_json::json!({ "items": rows })).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn unread_count(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    // Fail-silent: anonymous = 0. Used by the bell badge poller on every
    // page; an auth error should NOT spam the console.
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Json(serde_json::json!({ "count": 0 })).into_response(),
    };
    match svc::unread_count_for_user(&state.db, user.id).await {
        Ok(c) => Json(serde_json::json!({ "count": c })).into_response(),
        Err(_) => Json(serde_json::json!({ "count": 0 })).into_response(),
    }
}

async fn mark_read(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, "auth required").into_response(),
    };
    match svc::mark_one_read(&state.db, user.id, id).await {
        Ok(true) => Json(serde_json::json!({ "ok": true })).into_response(),
        Ok(false) => Json(serde_json::json!({ "ok": true, "noop": true })).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn mark_all_read(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return (axum::http::StatusCode::UNAUTHORIZED, "auth required").into_response(),
    };
    match svc::mark_all_read(&state.db, user.id).await {
        Ok(n) => Json(serde_json::json!({ "ok": true, "updated": n })).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Build the authenticated in-app inbox API router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/inbox", get(list_inbox))
        .route("/api/inbox/unread-count", get(unread_count))
        .route("/api/inbox/:id/read", post(mark_read))
        .route("/api/inbox/read-all", post(mark_all_read))
}
