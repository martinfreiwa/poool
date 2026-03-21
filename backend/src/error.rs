/// Centralized error handling for the POOOL backend.
///
/// SECURITY: Internal error details (DB errors, stack traces) are NEVER exposed
/// to the client. They are logged server-side via tracing.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

#[derive(Debug)]
pub enum AppError {
    /// Internal server error – details hidden from client, logged server-side.
    Internal(String),
    /// Resource not found.
    NotFound(String),
    /// Invalid input from client.
    BadRequest(String),
    /// Authentication required or failed.
    Unauthorized(String),
    /// Access denied (403).
    Forbidden(String),
    /// Resource conflict (e.g. duplicate email).
    Conflict(String),
    /// Database error – wrapped and hidden from client.
    Database(sqlx::Error),
    /// Rate limit exceeded – retry-after seconds.
    RateLimited(u64),

    // ── Marketplace-specific errors (Phase 1.11) ───────────────────
    /// User's available balance is insufficient for the operation.
    InsufficientBalance {
        /// The user's available balance in cents.
        available_cents: i64,
        /// The amount required in cents.
        required_cents: i64,
    },
    /// User doesn't own enough tokens for the operation.
    InsufficientTokens {
        /// Tokens the user currently holds.
        available: i32,
        /// Tokens required for the operation.
        required: i32,
    },
    /// Step-up 2FA verification is required for this financial operation (HTTP 428).
    TwoFactorRequired,
    /// Trading is currently disabled by admin kill-switch (HTTP 503).
    TradingDisabled,
    /// Self-trading (wash trading) is not allowed (HTTP 403).
    WashTradingBlocked,
    /// Order was rejected for a business-logic reason (HTTP 400).
    OrderRejected(String),
    /// A downstream service is unavailable (HTTP 503).
    ServiceUnavailable(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Internal(msg) => write!(f, "Internal: {}", msg),
            AppError::NotFound(msg) => write!(f, "NotFound: {}", msg),
            AppError::BadRequest(msg) => write!(f, "BadRequest: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::Database(err) => write!(f, "Database: {}", err),
            AppError::RateLimited(secs) => write!(f, "RateLimited: retry after {}s", secs),
            AppError::InsufficientBalance {
                available_cents,
                required_cents,
            } => {
                write!(
                    f,
                    "InsufficientBalance: available={}, required={}",
                    available_cents, required_cents
                )
            }
            AppError::InsufficientTokens {
                available,
                required,
            } => {
                write!(
                    f,
                    "InsufficientTokens: available={}, required={}",
                    available, required
                )
            }
            AppError::TwoFactorRequired => write!(f, "TwoFactorRequired"),
            AppError::TradingDisabled => write!(f, "TradingDisabled"),
            AppError::WashTradingBlocked => write!(f, "WashTradingBlocked"),
            AppError::OrderRejected(reason) => write!(f, "OrderRejected: {}", reason),
            AppError::ServiceUnavailable(msg) => write!(f, "ServiceUnavailable: {}", msg),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, client_message) = match &self {
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                sentry::capture_message(msg, sentry::Level::Error);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred. Please try again.".to_string(),
                )
            }
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Database(err) => {
                tracing::error!("Database error: {}", err);
                sentry::capture_error(err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An unexpected error occurred. Please try again.".to_string(),
                )
            }
            AppError::RateLimited(retry_after) => {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(
                        axum::http::header::RETRY_AFTER,
                        retry_after.to_string(),
                    )],
                    Json(serde_json::json!({
                        "error": format!("Too many requests. Please try again in {} seconds.", retry_after)
                    })),
                )
                    .into_response();
            }
            // ── Marketplace errors ──────────────────────────────────
            AppError::InsufficientBalance { .. } => (
                StatusCode::BAD_REQUEST,
                "Insufficient balance for this operation.".to_string(),
            ),
            AppError::InsufficientTokens { .. } => (
                StatusCode::BAD_REQUEST,
                "Insufficient tokens for this operation.".to_string(),
            ),
            AppError::TwoFactorRequired => (
                // 428 Precondition Required — signals the client to present 2FA modal
                StatusCode::from_u16(428).unwrap_or(StatusCode::FORBIDDEN),
                "Two-factor authentication is required for this operation.".to_string(),
            ),
            AppError::TradingDisabled => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Trading is currently paused. Please try again later.".to_string(),
            ),
            AppError::WashTradingBlocked => (
                StatusCode::FORBIDDEN,
                "Self-trading is not allowed.".to_string(),
            ),
            AppError::OrderRejected(reason) => (StatusCode::BAD_REQUEST, reason.clone()),
            AppError::ServiceUnavailable(msg) => {
                tracing::error!("Service unavailable: {}", msg);
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Service temporarily unavailable. Please try again later.".to_string(),
                )
            }
        };

        // Return JSON for /api/* callers; HTML for HTMX page-level swaps.
        // We detect API callers by checking if the response content-type should be JSON.
        // Since we can't read the request here, we return JSON which works for both:
        // - fetch() callers parse {"error": "..."}
        // - HTMX callers that expected HTML will see a JSON string (acceptable fallback)
        (status, Json(serde_json::json!({ "error": client_message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        tracing::error!("Password hashing error: {}", err);
        AppError::Internal("Password processing failed".to_string())
    }
}
