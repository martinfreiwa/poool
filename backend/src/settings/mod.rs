pub mod models;
pub mod routes;
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};

/// Compose all settings-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // HTML page — one canonical template, legacy URLs retained
        .route("/settings", get(page_settings))
        .route("/settings-2", get(page_settings_2))
        .route("/settings-3", get(page_settings_3))
        .route("/account-deletion", get(page_account_deletion))
        // JSON API
        .route("/api/settings", get(get_settings_handler))
        .route("/api/settings/profile", post(update_profile_handler))
        .route(
            "/api/settings/preferences",
            post(update_preferences_handler),
        )
        .route(
            "/api/settings/notifications",
            post(update_notifications_handler),
        )
        .route(
            "/api/settings/leaderboard",
            post(update_leaderboard_handler),
        )
        .route("/api/settings/social", post(update_social_links_handler))
        .route(
            "/api/settings/developer/profile",
            post(update_developer_profile_handler),
        )
        .route(
            "/api/settings/developer/links",
            post(update_developer_links_handler),
        )
        .route("/api/settings/oauth", get(list_oauth_connections_handler))
        .route(
            "/api/settings/oauth/:provider/link",
            post(link_oauth_provider_handler),
        )
        .route(
            "/api/settings/oauth/:connection_id",
            delete(unlink_oauth_connection_handler),
        )
        .route("/api/settings/2fa/disable", post(disable_totp_handler))
        .route("/api/settings/email", post(change_email_handler))
        .route("/api/settings/password", post(change_password_handler))
        .route("/api/settings/phone", post(change_phone_handler))
        // GDPR Compliance (Phase 10.5)
        .route("/api/settings/export-data", get(export_data_handler))
        .route("/api/settings/delete-account", post(delete_account_handler))
}
