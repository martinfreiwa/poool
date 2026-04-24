/// Settings route handlers – thin HTTP layer that delegates to the service.
///
/// Each handler:
/// 1. Extracts the authenticated user from session cookie
/// 2. Extracts form/JSON data from the request body
/// 3. Calls the appropriate service function
/// 4. Returns a JSON response ({ success, message })
///
/// All endpoints require a valid session cookie (return 401 if missing/invalid).
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;

use super::models::{
    ApiResponse, ChangeEmailForm, ChangePasswordForm, ChangePhoneForm, UpdateDeveloperLinksForm,
    UpdateDeveloperProfileForm, UpdateLeaderboardForm, UpdateNotificationsForm,
    UpdatePreferencesForm, UpdateProfileForm, UpdateSocialLinksForm,
};
use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;

// ─── Helper ────────────────────────────────────────────────────

/// Extract user ID from session cookie, or return 401.
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

// ─── GET /api/settings ─────────────────────────────────────────

/// Return the full settings for the authenticated user.
pub async fn get_settings_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session_token = jar
        .get(crate::auth::middleware::SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    match service::get_settings(&state.db, user_id, &session_token).await {
        Ok(settings) => Json(settings).into_response(),
        Err(e) => {
            tracing::error!("Failed to get settings for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load settings."})),
            )
                .into_response()
        }
    }
}

// ─── POST /api/settings/profile ────────────────────────────────

/// Save "My Details" tab data.
pub async fn update_profile_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdateProfileForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_profile(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Profile updated successfully.")).into_response(),
        Err(e) => {
            tracing::warn!("Profile update failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                _ => "Failed to update profile.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/preferences ────────────────────────────

/// Save "Preferences" tab data (language, currency).
pub async fn update_preferences_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdatePreferencesForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_preferences(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Preferences updated successfully.")).into_response(),
        Err(e) => {
            tracing::warn!("Preferences update failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                _ => "Failed to update preferences.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/notifications ──────────────────────────

/// Save notification preference toggles.
pub async fn update_notifications_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdateNotificationsForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_notifications(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Notification preferences saved.")).into_response(),
        Err(e) => {
            tracing::warn!("Notifications update failed for user {}: {}", user_id, e);
            Json(ApiResponse::err(
                "Failed to update notification preferences.",
            ))
            .into_response()
        }
    }
}

// ─── POST /api/settings/leaderboard ──────────────────────────

/// Save leaderboard preferences.
pub async fn update_leaderboard_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdateLeaderboardForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_leaderboard(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Leaderboard settings saved.")).into_response(),
        Err(e) => {
            tracing::warn!("Leaderboard update failed for user {}: {}", user_id, e);
            Json(ApiResponse::err("Failed to update leaderboard settings.")).into_response()
        }
    }
}

// ─── POST /api/settings/social ─────────────────────────────────

pub async fn update_social_links_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdateSocialLinksForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_social_links(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Social links saved.")).into_response(),
        Err(e) => {
            tracing::warn!("Social links update failed for user {}: {}", user_id, e);
            Json(ApiResponse::err("Failed to update social links.")).into_response()
        }
    }
}

// ─── POST /api/settings/developer/profile ──────────────────────

pub async fn update_developer_profile_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdateDeveloperProfileForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_developer_profile(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Developer profile saved.")).into_response(),
        Err(e) => {
            tracing::warn!(
                "Developer profile update failed for user {}: {}",
                user_id,
                e
            );
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                crate::error::AppError::Unauthorized(m) => m.clone(),
                _ => "Failed to update developer profile.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/developer/links ────────────────────────

pub async fn update_developer_links_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<UpdateDeveloperLinksForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::update_developer_links(&state.db, user_id, form).await {
        Ok(()) => Json(ApiResponse::ok("Developer links saved.")).into_response(),
        Err(e) => {
            tracing::warn!("Developer links update failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::Unauthorized(m) => m.clone(),
                _ => "Failed to update developer links.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── OAuth connection helpers ──────────────────────────────────

pub async fn list_oauth_connections_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session_token = jar
        .get(crate::auth::middleware::SESSION_COOKIE)
        .map(|c| c.value().to_string())
        .unwrap_or_default();

    match service::get_settings(&state.db, user_id, &session_token).await {
        Ok(settings) => {
            Json(serde_json::json!({ "connections": settings.oauth_accounts })).into_response()
        }
        Err(e) => {
            tracing::warn!("OAuth list failed for user {}: {}", user_id, e);
            Json(serde_json::json!({"error": "Failed to load OAuth connections."})).into_response()
        }
    }
}

pub async fn link_oauth_provider_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> axum::response::Response {
    if require_user_id(&jar, &state).await.is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Not authenticated"})),
        )
            .into_response();
    }
    if provider != "google" {
        return Json(ApiResponse::err(
            "Only Google sign-in linking is supported.",
        ))
        .into_response();
    }
    if !state.config.google_oauth_enabled() {
        return Json(ApiResponse::err("Google sign-in is not configured.")).into_response();
    }

    Json(serde_json::json!({
        "success": true,
        "redirect_url": "/auth/google?link=1"
    }))
    .into_response()
}

pub async fn unlink_oauth_connection_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(connection_id): Path<uuid::Uuid>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::unlink_oauth_account(&state.db, user_id, connection_id).await {
        Ok(()) => Json(ApiResponse::ok("OAuth connection removed.")).into_response(),
        Err(e) => {
            tracing::warn!("OAuth unlink failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                crate::error::AppError::NotFound(m) => m.clone(),
                _ => "Failed to disconnect OAuth account.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/email ──────────────────────────────────

/// Change the user's email (requires current password).
pub async fn change_email_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<ChangeEmailForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::change_email(&state.db, user_id, &form.new_email, &form.current_password).await {
        Ok(()) => Json(ApiResponse::ok("Email changed successfully.")).into_response(),
        Err(e) => {
            tracing::warn!("Email change failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                crate::error::AppError::Conflict(m) => m.clone(),
                _ => "Failed to change email.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/password ───────────────────────────────

/// Change the user's password (requires current password).
pub async fn change_password_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<ChangePasswordForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::change_password(
        &state.db,
        user_id,
        &form.current_password,
        &form.new_password,
        &form.confirm_password,
    )
    .await
    {
        Ok(()) => Json(ApiResponse::ok("Password changed successfully.")).into_response(),
        Err(e) => {
            tracing::warn!("Password change failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                _ => "Failed to change password.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/phone ──────────────────────────────────

/// Change the user's phone number.
pub async fn change_phone_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<ChangePhoneForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::change_phone(&state.db, user_id, &form.new_phone).await {
        Ok(()) => Json(ApiResponse::ok("Phone number updated successfully.")).into_response(),
        Err(e) => {
            tracing::warn!("Phone change failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                _ => "Failed to update phone number.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

// ─── POST /api/settings/2fa/disable ────────────────────────────

/// Disable 2FA for the current user.
pub async fn disable_totp_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::disable_totp(&state.db, user_id).await {
        Ok(()) => Json(ApiResponse::ok("Two-factor authentication disabled.")).into_response(),
        Err(e) => {
            tracing::warn!("Failed to disable 2FA for user {}: {}", user_id, e);
            Json(ApiResponse::err("Failed to disable 2FA.")).into_response()
        }
    }
}

// ─── GET /api/settings/export-data (GDPR Art. 15/20) ──────────

/// Export all user data as a downloadable JSON file.
pub async fn export_data_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::export_user_data(&state.db, user_id).await {
        Ok(data) => {
            let body = serde_json::to_string_pretty(&data).unwrap_or_default();
            (
                StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, "application/json"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"poool_data_export.json\"",
                    ),
                ],
                body,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Data export failed for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to export data."})),
            )
                .into_response()
        }
    }
}

// ─── POST /api/settings/delete-account (GDPR Art. 17) ─────────

/// Selectively delete user account per GDPR + financial regulatory requirements.
pub async fn delete_account_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(form): Json<super::models::DeleteAccountForm>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::delete_account_selective(&state.db, user_id, &form.current_password).await {
        Ok(()) => Json(ApiResponse::ok(
            "Account deletion completed. You will be logged out.",
        ))
        .into_response(),
        Err(e) => {
            tracing::warn!("Account deletion failed for user {}: {}", user_id, e);
            let msg = match &e {
                crate::error::AppError::BadRequest(m) => m.clone(),
                crate::error::AppError::NotFound(m) => m.clone(),
                _ => "Failed to delete account.".to_string(),
            };
            Json(ApiResponse::err(&msg)).into_response()
        }
    }
}

/// GET /settings — Render the canonical user settings page.
pub async fn page_settings(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "settings.html").await
}

/// GET /settings-2 — Legacy route kept for bookmarks.
pub async fn page_settings_2(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "settings.html").await
}

/// GET /settings-3 — Legacy route kept for bookmarks.
pub async fn page_settings_3(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "settings.html").await
}

/// GET /account-deletion — Render the account deletion page.
pub async fn page_account_deletion(
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "account-deletion.html").await
}
