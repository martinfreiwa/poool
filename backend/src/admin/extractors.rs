//! Admin extractor types – eliminates auth boilerplate from handlers.
//!
//! Instead of every handler manually checking `is_admin()`, handlers
//! simply declare `AdminUser` as a parameter and Axum does the rest.

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;

use crate::auth::middleware;
use crate::auth::models::User;

// ─── ApiError ──────────────────────────────────────────────────
// JSON-returning error type for API handlers (as opposed to the
// HTML-returning `AppError` used by HTMX auth forms).

/// API-specific error type that returns JSON `{"error": "..."}`.
///
/// Use this as the error type for all `Result<Json<T>, ApiError>` handlers.
#[derive(Debug)]
pub enum ApiError {
    /// 500 – internal; details hidden from client and logged.
    Internal(String),
    /// 404
    NotFound(String),
    /// 400
    BadRequest(String),
    /// 401
    Unauthorized(String),
    /// 403
    Forbidden(String),
    /// 409
    Conflict(String),
    /// 429 — rate-limit exceeded; message tells caller when to retry.
    TooManyRequests(String),
    /// Database error – wrapped and hidden from client.
    Database(sqlx::Error),
}

impl std::fmt::Display for ApiError {
    /// Client-safe Display — omits internal detail for `Internal`/`Database`
    /// so a stray `format!("{}", err)` cannot leak stack context. For
    /// server-side logging, use the `Debug` impl (or `.detail()`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Internal(_) => write!(f, "Internal"),
            ApiError::NotFound(msg) => write!(f, "NotFound: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "BadRequest: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::TooManyRequests(msg) => write!(f, "TooManyRequests: {}", msg),
            ApiError::Database(_) => write!(f, "Database"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, client_message) = match &self {
            ApiError::Internal(msg) => {
                tracing::error!("API internal error: {}", msg);
                sentry::capture_message(msg, sentry::Level::Error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred. Please try again.".to_string(),
                )
            }
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ApiError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg.clone()),
            ApiError::Database(err) => {
                tracing::error!("API database error: {}", err);
                sentry::capture_error(err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred. Please try again.".to_string(),
                )
            }
        };

        (
            status,
            axum::Json(serde_json::json!({ "error": client_message })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::Database(err)
    }
}

/// D1 fix: lossless `AppError → ApiError` mapping. Use via
/// `.map_err(ApiError::from)` instead of hand-written
/// `_ => ApiError::Internal("foo failed")` which discarded the underlying
/// type + message. The variant is preserved so 4xx errors return as 4xx
/// (not 500) and the actual message reaches the client / Sentry.
impl From<crate::error::AppError> for ApiError {
    fn from(err: crate::error::AppError) -> Self {
        use crate::error::AppError;
        match err {
            AppError::NotFound(m) => ApiError::NotFound(m),
            AppError::BadRequest(m) => ApiError::BadRequest(m),
            AppError::Unauthorized(m) => ApiError::Unauthorized(m),
            AppError::Forbidden(m) => ApiError::Forbidden(m),
            AppError::Conflict(m) => ApiError::Conflict(m),
            AppError::Database(e) => ApiError::Database(e),
            AppError::RateLimited(secs) => ApiError::TooManyRequests(format!(
                "Too many requests. Retry after {} seconds.",
                secs
            )),
            AppError::Internal(m) => ApiError::Internal(m),
            // Marketplace + payment-specific variants — surface as BadRequest
            // with the same human-readable message the AppError emits via Display.
            other => ApiError::BadRequest(format!("{}", other)),
        }
    }
}

impl ApiError {
    /// Parse a string as a UUID, returning `ApiError::BadRequest` on failure.
    ///
    /// Replaces the repetitive `match id.parse() { Ok(u) => u, Err(_) => return ... }` pattern.
    pub fn parse_uuid(s: &str) -> Result<uuid::Uuid, ApiError> {
        s.parse()
            .map_err(|_| ApiError::BadRequest(format!("Invalid ID format: {}", s)))
    }
}

// ─── AdminUser extractor ───────────────────────────────────────

/// An authenticated admin user, extracted from the session cookie.
///
/// Use this as a handler parameter to enforce admin access:
/// ```text
/// async fn my_handler(admin: AdminUser, State(s): State<AppState>) -> Result<Json<...>, ApiError> {
///     // admin.user gives you the verified admin User
/// }
/// ```
///
/// Returns 401 if not logged in, 403 if not an admin.
pub struct AdminUser {
    /// The verified admin [`User`] record from the session.
    pub user: User,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    sqlx::PgPool: axum::extract::FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = <sqlx::PgPool as axum::extract::FromRef<S>>::from_ref(state);

        // Extract session cookie
        let jar = CookieJar::from_headers(&parts.headers);
        let user = middleware::get_current_user(&jar, &pool)
            .await
            .ok_or_else(|| ApiError::Unauthorized("Authentication required".to_string()))?;

        // Check admin role
        let is_admin: bool = sqlx::query_scalar(
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
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if !is_admin {
            return Err(ApiError::Forbidden("Admin access required".to_string()));
        }

        Ok(AdminUser { user })
    }
}

impl AdminUser {
    /// Enforce fine-grained admin permission (e.g. `"admins.manage"`,
    /// `"roles.edit"`). Returns `Err(ApiError::Forbidden)` if the admin lacks
    /// the permission. Use after `AdminUser` extraction to replace the legacy
    /// `check_permission(&jar, ...)` pattern.
    pub async fn require_permission(
        &self,
        pool: &sqlx::PgPool,
        permission: &str,
    ) -> Result<(), ApiError> {
        if middleware::has_permission(pool, self.user.id, permission).await {
            Ok(())
        } else {
            Err(ApiError::Forbidden(format!(
                "Missing permission: {}",
                permission
            )))
        }
    }

    /// Returns true if this admin has the `super_admin` role. Used to gate
    /// role-mutation endpoints so only super admins can create/promote/demote
    /// other admins.
    pub async fn is_super_admin(&self, pool: &sqlx::PgPool) -> bool {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM user_roles ur
                JOIN roles r ON r.id = ur.role_id
                WHERE ur.user_id = $1
                  AND r.name = 'super_admin'
                  AND ur.is_active = TRUE
            )
            "#,
        )
        .bind(self.user.id)
        .fetch_one(pool)
        .await
        .unwrap_or(false)
    }
}

/// Roles that require super_admin to assign. Any attempt to grant one of
/// these from a non-super-admin caller is rejected.
pub const ELEVATED_ROLES: &[&str] = &["admin", "super_admin"];

/// Roles any admin (not just super_admin) may request. Elevated roles
/// (`ELEVATED_ROLES`) are excluded — they require a separate super_admin gate.
/// `developer` is included so super_admin can manually assign it via the API;
/// it is also auto-assigned by `api_developer_create_draft` on first submission.
pub const ASSIGNABLE_ROLES: &[&str] = &[
    "compliance",
    "support",
    "finance",
    "kyc_reviewer",
    "developer",
];
