/// Auth middleware – extracts the current user from the session cookie.
///
/// Provides helper functions for session-based auth checks.
/// Used by route handlers to verify authentication.
use axum_extra::extract::cookie::CookieJar;
use sqlx::PgPool;

use super::models::User;
use super::service;
use uuid::Uuid;

/// The cookie name used for sessions.
pub const SESSION_COOKIE: &str = "poool_session";

/// The cookie name used for referral tracking.
pub const REFERRAL_COOKIE: &str = "poool_referral";

/// Extract the current user from a session cookie.
///
/// Returns `Some(User)` if the session is valid, `None` otherwise.
/// This is the core auth check used by all protected routes.
pub async fn get_current_user(jar: &CookieJar, pool: &PgPool) -> Option<User> {
    let session_token = jar.get(SESSION_COOKIE)?.value().to_string();

    match service::get_user_by_session(pool, &session_token).await {
        Ok(user_opt) => user_opt,
        Err(e) => {
            tracing::error!("get_user_by_session failed returning Err: {}", e);
            None
        }
    }
}

/// Check if a request is authenticated.
///
/// Returns `true` if the session cookie is present and valid.
pub async fn is_authenticated(jar: &CookieJar, pool: &PgPool) -> bool {
    get_current_user(jar, pool).await.is_some()
}

/// Check if the current user has admin privileges.
///
/// Returns `true` if the user has the 'admin' or 'super_admin' role
/// in the `user_roles` table. Returns `false` for unauthenticated users.
pub async fn is_admin(jar: &CookieJar, pool: &PgPool) -> bool {
    let user = match get_current_user(jar, pool).await {
        Some(u) => u,
        None => {
            tracing::warn!("is_admin check failed: no current user found");
            return false;
        }
    };

    let has_admin: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_roles ur
            JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1
            AND r.name IN ('admin', 'super_admin')
            AND ur.is_active = TRUE
        )
        "#,
    )
    .bind(user.id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !has_admin {
        tracing::warn!(
            "is_admin check failed: user {} has no active admin role",
            user.email
        );
    }

    has_admin
}

/// Check if the current user has a specific granular permission.
///
/// Returns `true` if any of the user's active roles grant the permission
/// or the special 'all' permission.
pub async fn has_permission(pool: &PgPool, user_id: Uuid, permission: &str) -> bool {
    let has_perm: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_roles ur
            JOIN admin_permissions ap ON ap.role_id = ur.role_id
            WHERE ur.user_id = $1
            AND (ap.permission = $2 OR ap.permission = 'all')
            AND ur.is_active = TRUE
        )
        "#,
    )
    .bind(user_id)
    .bind(permission)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    has_perm
}

/// Helper for route handlers to check permissions using the CookieJar.
pub async fn check_permission(jar: &CookieJar, pool: &PgPool, permission: &str) -> bool {
    if let Some(user) = get_current_user(jar, pool).await {
        has_permission(pool, user.id, permission).await
    } else {
        false
    }
}
