/// Models for leaderboard data structures.
pub mod models;
/// HTTP route handlers for leaderboard pages and API.
pub mod routes;
/// Business logic: score computation, rank refresh, caching.
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

/// Compose all leaderboard-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // HTML page
        .route("/leaderboard", get(page_leaderboard))
        .route("/leaderboard.html", get(page_leaderboard))
        // JSON API
        .route("/api/leaderboard", get(get_rankings))
        .route("/api/leaderboard/me", get(get_my_rank))
        .route("/api/leaderboard/preferences", get(get_preferences))
        .route("/api/leaderboard/preferences", put(update_preferences))
        // Manual refresh is admin-only and side-effecting — POST so it can't
        // be triggered by a casual GET (link prefetch, accidental nav, etc.)
        // and so CSRF middleware enforces the token.
        .route("/api/leaderboard/refresh", post(trigger_refresh))
}
