/// Auth data models – Rust structs mapping to database tables.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Database Models ───────────────────────────────────────────

/// A user record from the `users` table.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    /// Argon2id hash. Never serialised — `#[serde(skip)]` stops the hash
    /// escaping via any `Json(user)` / `serde_json::to_string(&user)` call
    /// (defence-in-depth against a future handler that forgets to strip
    /// it). Still deserialised from the DB row via `FromRow`.
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub email_verified: bool,
    pub avatar_url: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A session record from the `user_sessions` table.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub remember_me: bool,
    pub is_2fa_verified: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// An OAuth account from the `oauth_accounts` table.
#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct OAuthAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_id: String,
    pub provider_email: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// User settings record from the `user_settings` table.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserSettings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    pub language: Option<String>,
    pub email_notifications: Option<bool>,
    pub push_notifications: Option<bool>,
    pub updated_at: DateTime<Utc>,
}

// ─── Form Data (from HTMX requests) ───────────────────────────

/// Login form data submitted via HTMX.
#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
    pub remember: Option<String>, // checkbox sends "on" or absent
}

impl LoginForm {
    pub fn remember_me(&self) -> bool {
        self.remember.as_deref() == Some("on")
    }
}

/// Signup form data submitted via HTMX.
#[derive(Debug, Deserialize)]
pub struct SignupForm {
    pub email: String,
    pub password: String,
    pub terms_accepted: Option<String>, // checkbox sends "on" or absent
    pub referral_code: Option<String>,
}

impl SignupForm {
    pub fn terms_accepted(&self) -> bool {
        self.terms_accepted.as_deref() == Some("on")
    }
}

/// Forgot password form data submitted via HTMX.
#[derive(Debug, Deserialize)]
pub struct ForgotPasswordForm {
    pub email: String,
}

/// Reset password form data submitted via HTMX.
#[derive(Debug, Deserialize)]
pub struct ResetPasswordForm {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

/// Resend verification email form data
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ResendVerificationForm {}

// ─── Response helpers ──────────────────────────────────────────

/// User data safe to expose to templates (no password hash!).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct UserView {
    pub id: Uuid,
    pub email: String,
    pub avatar_url: Option<String>,
    pub status: String,
}

impl From<User> for UserView {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            avatar_url: u.avatar_url,
            status: u.status,
        }
    }
}

/// TOTP verification form.
#[derive(Debug, serde::Deserialize)]
pub struct TotpForm {
    pub code: String,
}

/// TOTP setup form.
#[derive(Debug, serde::Deserialize)]
pub struct TotpSetupForm {
    pub setup_token: String,
    pub code: String,
}

/// Step-up 2FA verification form (JSON API — Phase 1.4).
#[derive(Debug, serde::Deserialize)]
pub struct StepUpVerifyForm {
    pub code: String,
    /// The financial action to authorize: "withdrawal", "trade", "payment_method", "password_change"
    pub action: String,
}
