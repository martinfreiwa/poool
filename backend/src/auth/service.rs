/// Auth business logic – ISOLATED from HTTP handlers.
///
/// This is the core of the auth system. All business rules live here,
/// making them independently testable and auditable.
///
/// SECURITY INVARIANTS:
/// - Passwords are ALWAYS hashed with Argon2id (never stored as plaintext)
/// - Session tokens are cryptographically random (64 bytes / 512 bits)
/// - Failed logins return generic errors (no user enumeration)
/// - All state mutations happen in atomic DB transactions
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use base64::Engine;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use super::models::{User, UserSettings};
use crate::common::validation;
use crate::error::AppError;

// ─── Registration ──────────────────────────────────────────────

/// Register a new user with email and password.
///
/// Creates user, profile, wallets (cash + rewards), and assigns the
/// default 'investor' role – all in one atomic transaction.
pub async fn register_user(
    pool: &PgPool,
    email: &str,
    password: &str,
    base_url: &str,
) -> Result<User, AppError> {
    let email = email.trim().to_lowercase();

    // Validate inputs
    validation::validate_email(&email)?;
    validation::validate_password(password)?;

    // Check if email already exists
    let existing = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind(&email)
        .fetch_one(pool)
        .await?;

    if existing > 0 {
        return Err(AppError::Conflict(
            "An account with this email already exists.".to_string(),
        ));
    }

    // Hash password with Argon2id
    let password_hash = hash_password(password)?;

    // Begin atomic transaction
    let mut tx = pool.begin().await?;

    // 1. Create user
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, email_verified, status)
        VALUES ($1, $2, FALSE, 'active')
        RETURNING *
        "#,
    )
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&mut *tx)
    .await?;

    // 2. Create empty profile
    sqlx::query("INSERT INTO user_profiles (user_id) VALUES ($1)")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    // 3. Create wallets (cash + rewards, both starting at 0)
    sqlx::query(
        "INSERT INTO wallets (user_id, wallet_type, balance_cents) VALUES ($1, 'cash', 0), ($1, 'rewards', 0)",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    // 4. Assign default 'investor' role
    sqlx::query(
        r#"
        INSERT INTO user_roles (user_id, role_id)
        SELECT $1, id FROM roles WHERE name = 'investor'
        "#,
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    // 5. Create user settings
    sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1)")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    // Commit everything atomically
    tx.commit().await?;

    tracing::info!("New user registered: {} ({})", user.id, email);

    // Automatically generate and dispatch verification email
    let _ = create_email_verification_token(pool, user.id, &email, base_url).await;

    Ok(user)
}

// ─── Login ─────────────────────────────────────────────────────

/// Dummy Argon2id hash lazily computed once at first use. Used as a
/// verification target when authentication fails on email lookup so the
/// unknown-email path takes roughly the same wall-clock time as the
/// wrong-password path, defeating enumeration via timing side-channel.
fn dummy_argon2_hash() -> &'static str {
    use std::sync::OnceLock;
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let salt = argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
        let argon2 = argon2::Argon2::default();
        argon2
            .hash_password(b"dummy-password-never-matches-any-real-password", &salt)
            .map(|h| h.to_string())
            .unwrap_or_else(|_| String::from(
                "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$FKJjxOQmGa/wlvKjYvJL6sY9SqwV9lWKt1uHxDkt8VQ"
            ))
    })
}

/// Authenticate a user with email and password.
///
/// SECURITY: Returns a generic error for both "user not found" and
/// "wrong password" to prevent user enumeration attacks.
pub async fn authenticate_user(
    pool: &PgPool,
    email: &str,
    password: &str,
) -> Result<User, AppError> {
    let email = email.trim().to_lowercase();

    let user_opt =
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND status = 'active'")
            .bind(&email)
            .fetch_optional(pool)
            .await?;

    let user = match user_opt {
        Some(u) => u,
        None => {
            // Timing-attack defense: run an Argon2 verify against a dummy hash
            // so unknown-email responses take roughly the same time as
            // wrong-password responses. Without this, response time reveals
            // whether an email is registered.
            let _ = verify_password(password, dummy_argon2_hash());
            sentry::with_scope(
                |scope| {
                    scope.set_tag("security.event", "failed_login");
                    scope.set_tag("security.reason", "unknown_email");
                },
                || {
                    sentry::capture_message(
                        &format!("Failed login: unknown email {}", email),
                        sentry::Level::Warning,
                    );
                },
            );
            return Err(AppError::Unauthorized(
                "Invalid email or password.".to_string(),
            ));
        }
    };

    // Verify password
    let password_hash = user.password_hash.as_ref().ok_or_else(|| {
        // User registered via OAuth and has no password set
        sentry::with_scope(
            |scope| {
                scope.set_tag("security.event", "failed_login");
                scope.set_tag("security.reason", "oauth_only_account");
            },
            || {
                sentry::capture_message(
                    &format!("Failed login: OAuth-only account {}", email),
                    sentry::Level::Warning,
                );
            },
        );
        AppError::Unauthorized(
            "This account uses social login. Please sign in with Google or Facebook.".to_string(),
        )
    })?;

    match verify_password(password, password_hash) {
        Ok(()) => {
            if !user.email_verified {
                sentry::with_scope(
                    |scope| {
                        scope.set_tag("security.event", "failed_login");
                        scope.set_tag("security.reason", "email_not_verified");
                    },
                    || {
                        sentry::capture_message(
                            &format!("Login blocked: email not verified {}", email),
                            sentry::Level::Info,
                        );
                    },
                );
                return Err(AppError::Forbidden(
                    "Please verify your email before signing in. Check your inbox for the verification link.".to_string(),
                ));
            }
            Ok(user)
        }
        Err(_) => {
            // Security: track failed login (wrong password)
            sentry::with_scope(
                |scope| {
                    scope.set_tag("security.event", "failed_login");
                    scope.set_tag("security.reason", "wrong_password");
                    scope.set_user(Some(sentry::User {
                        id: Some(user.id.to_string()),
                        email: Some(email.clone()),
                        ..Default::default()
                    }));
                },
                || {
                    sentry::capture_message(
                        &format!("Failed login: wrong password for {}", email),
                        sentry::Level::Warning,
                    );
                },
            );
            Err(AppError::Unauthorized(
                "Invalid email or password.".to_string(),
            ))
        }
    }
}

// ─── Sessions ──────────────────────────────────────────────────

/// Create a new session for a user.
/// Returns the session token to be set as a cookie.
pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    remember_me: bool,
    is_2fa_verified: bool,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<String, AppError> {
    let session_token = generate_session_token();

    let expires_at = if remember_me {
        Utc::now() + Duration::days(30)
    } else {
        Utc::now() + Duration::hours(24)
    };

    sqlx::query(
        r#"
        INSERT INTO user_sessions (user_id, session_token, ip_address, user_agent, remember_me, is_2fa_verified, expires_at)
        VALUES ($1, $2, $3::inet, $4, $5, $6, $7)
        "#,
    )
    .bind(user_id)
    .bind(&session_token)
    .bind(ip_address)
    .bind(user_agent)
    .bind(remember_me)
    .bind(is_2fa_verified)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(session_token)
}

/// Verify a session's 2FA status.
pub async fn verify_session_2fa(pool: &PgPool, session_token: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE user_sessions SET is_2fa_verified = TRUE WHERE session_token = $1")
        .bind(session_token)
        .execute(pool)
        .await?;

    Ok(())
}

/// Rotate a session token after a privilege change (e.g., 2FA verification).
/// Issues a brand-new session token bound to the same session row and
/// invalidates the previous token. Defends against session-fixation: any
/// attacker that captured the pre-2FA token cannot re-use it after the
/// user completes the step-up.
///
/// Returns the new session token that must be written back to the cookie.
pub async fn rotate_session_token(
    pool: &PgPool,
    old_token: &str,
) -> Result<String, AppError> {
    let new_token = generate_session_token();
    let affected = sqlx::query(
        "UPDATE user_sessions
            SET session_token = $1, is_2fa_verified = TRUE, updated_at = NOW()
            WHERE session_token = $2",
    )
    .bind(&new_token)
    .bind(old_token)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        return Err(AppError::Unauthorized("Session expired.".to_string()));
    }
    Ok(new_token)
}

/// Look up a user by their session token.
///
/// SECURITY: Only returns active sessions that have passed 2FA (if enabled).
pub async fn get_user_by_session(
    pool: &PgPool,
    session_token: &str,
) -> Result<Option<User>, AppError> {
    // 1. Fetch user + session + 2FA settings
    let row = sqlx::query!(
        r#"
        SELECT u.id, u.email, u.password_hash, u.email_verified, u.avatar_url, u.status, u.created_at, u.updated_at,
               s.is_2fa_verified,
               COALESCE(us.totp_enabled, FALSE) as totp_enabled
        FROM users u
        JOIN user_sessions s ON u.id = s.user_id
        LEFT JOIN user_settings us ON u.id = us.user_id
        WHERE s.session_token = $1
          AND s.expires_at > NOW()
          AND u.status = 'active'
        "#,
        session_token
    )
    .fetch_optional(pool)
    .await?;

    // Session tokens are bearer credentials; only log a short prefix.
    let tok_preview = &session_token[..8.min(session_token.len())];

    if let Some(r) = row {
        // 2. Enforcement: If 2FA enabled globally but session hasn't verified it, deny access
        if r.totp_enabled.unwrap_or(false) && !r.is_2fa_verified {
            tracing::warn!(
                "Session {}… denied: totp_enabled={} is_2fa_verified={}",
                tok_preview,
                r.totp_enabled.unwrap_or(false),
                r.is_2fa_verified
            );
            return Ok(None);
        }

        tracing::info!("Session {}… valid for user {}", tok_preview, r.email);

        Ok(Some(User {
            id: r.id,
            email: r.email,
            password_hash: r.password_hash,
            email_verified: r.email_verified,
            avatar_url: r.avatar_url,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    } else {
        tracing::warn!("Session {}… not found or expired in DB", tok_preview);
        Ok(None)
    }
}

/// Look up a user by their session token, EVEN IF 2FA IS NOT YET VERIFIED.
/// Used internaly for the 2FA verification flow.
pub async fn get_user_by_session_unverified(
    pool: &PgPool,
    session_token: &str,
) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.* FROM users u
        JOIN user_sessions s ON u.id = s.user_id
        WHERE s.session_token = $1
          AND s.expires_at > NOW()
          AND u.status = 'active'
        "#,
    )
    .bind(session_token)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

/// Delete a session (logout).
pub async fn delete_session(pool: &PgPool, session_token: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM user_sessions WHERE session_token = $1")
        .bind(session_token)
        .execute(pool)
        .await?;

    Ok(())
}

// ─── OAuth ─────────────────────────────────────────────────────

/// Find or create a user via OAuth provider data.
///
/// If the OAuth account already exists, returns the linked user.
/// If the email exists but no OAuth link, creates the link.
/// If neither exists, creates a new user + OAuth link.
pub async fn oauth_find_or_create_user(
    pool: &PgPool,
    provider: &str,
    provider_id: &str,
    email: &str,
    first_name: Option<&str>,
    last_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, AppError> {
    let email = email.trim().to_lowercase();

    // Check if OAuth account already exists
    let existing_user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.* FROM users u
        JOIN oauth_accounts oa ON u.id = oa.user_id
        WHERE oa.provider = $1 AND oa.provider_id = $2
        "#,
    )
    .bind(provider)
    .bind(provider_id)
    .fetch_optional(pool)
    .await?;

    if let Some(user) = existing_user {
        // Back-fill profile data from provider if user has blank fields
        update_oauth_profile(pool, user.id, first_name, last_name, avatar_url).await;
        return Ok(user);
    }

    // Check if a user with this email already exists
    let existing_email_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(pool)
        .await?;

    if let Some(user) = existing_email_user {
        // Refuse to auto-link a provider account to a local user whose email
        // was never verified locally — attacker could have pre-registered
        // victim's email and await the victim's first OAuth sign-in to take
        // over the account. Require the existing record to have
        // email_verified=TRUE before linking.
        if !user.email_verified {
            tracing::warn!(
                user_id = %user.id,
                "Refusing OAuth auto-link: existing account not email_verified"
            );
            return Err(AppError::Unauthorized(
                "An account exists for this email but was not verified. Please verify via the original sign-up method first.".to_string(),
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO oauth_accounts (user_id, provider, provider_id, provider_email)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (provider, provider_id) DO NOTHING
            "#,
        )
        .bind(user.id)
        .bind(provider)
        .bind(provider_id)
        .bind(&email)
        .execute(pool)
        .await?;

        update_oauth_profile(pool, user.id, first_name, last_name, avatar_url).await;

        return Ok(user);
    }

    // Create new user + OAuth account
    let mut tx = pool.begin().await?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, email_verified, status, avatar_url)
        VALUES ($1, TRUE, 'active', $2)
        RETURNING *
        "#,
    )
    .bind(&email)
    .bind(avatar_url)
    .fetch_one(&mut *tx)
    .await?;

    // Link OAuth
    sqlx::query(
        r#"
        INSERT INTO oauth_accounts (user_id, provider, provider_id, provider_email)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user.id)
    .bind(provider)
    .bind(provider_id)
    .bind(&email)
    .execute(&mut *tx)
    .await?;

    // Create profile, wallets, role, settings
    sqlx::query("INSERT INTO user_profiles (user_id, first_name, last_name) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(first_name)
        .bind(last_name)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO wallets (user_id, wallet_type, balance_cents) VALUES ($1, 'cash', 0), ($1, 'rewards', 0)",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) SELECT $1, id FROM roles WHERE name = 'investor'",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1)")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    tracing::info!(
        "New OAuth user registered: {} ({}) via {}",
        user.id,
        email,
        provider
    );

    Ok(user)
}

/// Back-fill user profile with OAuth provider data.
/// Only updates fields that are currently NULL or empty — never overwrites
/// data the user has manually set.
async fn update_oauth_profile(
    pool: &PgPool,
    user_id: Uuid,
    first_name: Option<&str>,
    last_name: Option<&str>,
    avatar_url: Option<&str>,
) {
    // Update avatar if user doesn't have one yet
    if let Some(url) = avatar_url {
        let _ = sqlx::query(
            "UPDATE users SET avatar_url = $1 WHERE id = $2 AND (avatar_url IS NULL OR avatar_url = '')",
        )
        .bind(url)
        .bind(user_id)
        .execute(pool)
        .await;
    }

    // Fill in name fields if currently blank
    if first_name.is_some() || last_name.is_some() {
        let _ = sqlx::query(
            r#"UPDATE user_profiles
               SET first_name = COALESCE(NULLIF(first_name, ''), $1),
                   last_name  = COALESCE(NULLIF(last_name, ''), $2)
               WHERE user_id = $3"#,
        )
        .bind(first_name)
        .bind(last_name)
        .bind(user_id)
        .execute(pool)
        .await;
    }
}

// ─── Password Reset ────────────────────────────────────────────

/// Generate a password reset token for an email.
pub async fn create_password_reset_token(
    pool: &PgPool,
    email: &str,
    base_url: &str,
) -> Result<(), AppError> {
    let email = email.trim().to_lowercase();

    // Check if user exists
    let user =
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND status = 'active'")
            .bind(&email)
            .fetch_optional(pool)
            .await?;

    let user = match user {
        Some(u) => u,
        None => return Ok(()), // Don't expose if user exists or not
    };

    let token = generate_session_token();
    let token_hash = crate::config::hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(1);

    sqlx::query(
        r#"
        INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    // Send Email using Resend
    let subject = "Reset your POOOL password";
    let body = format!(
        r#"
        <h2>Password Reset</h2>
        <p>You requested a password reset. Click the link below to set a new password:</p>
        <p><a href="{}/auth/reset-password?token={}">Reset Password</a></p>
        <p>If you did not request this, please ignore this email.</p>
        "#,
        base_url, token
    );

    crate::common::email::send_email(&email, subject, &body).await?;

    tracing::info!("Sent Password Reset link to {}", email);

    Ok(())
}

/// Reset a user's password using a reset token.
pub async fn reset_password(
    pool: &PgPool,
    token: &str,
    new_password: &str,
) -> Result<(), AppError> {
    validation::validate_password(new_password)?;

    let mut tx = pool.begin().await?;

    // Hash the incoming token to compare with stored hash
    let token_hash = crate::config::hash_token(token);

    // Find valid token
    let token_row = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT id, user_id FROM password_reset_tokens
        WHERE token_hash = $1 AND expires_at > NOW() AND used_at IS NULL
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid or expired password reset link.".to_string()))?;

    let (token_id, user_id) = token_row;

    let password_hash = hash_password(new_password)?;

    // Update user password
    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&password_hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Mark the used token AND every other outstanding reset token for this
    // user as consumed — if the real user's mailbox was also compromised,
    // any unclicked reset links lingering there are now dead.
    sqlx::query(
        "UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE (id = $1 OR user_id = $2)
              AND used_at IS NULL",
    )
    .bind(token_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // Invalidate every active session for this user. A password reset
    // implies "I did not control this account a moment ago," so existing
    // session tokens — possibly held by an attacker — must not survive.
    sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

// ─── Email Verification ────────────────────────────────────────

/// Generate and send an email verification token.
pub async fn create_email_verification_token(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
    base_url: &str,
) -> Result<(), AppError> {
    let token = generate_session_token();
    let token_hash = crate::config::hash_token(&token);
    let expires_at = Utc::now() + Duration::hours(24);

    sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    // Send Email using Resend
    let subject = "Verify your POOOL email";
    let body = format!(
        r#"
        <h2>Welcome to POOOL!</h2>
        <p>Please click the link below to verify your email address:</p>
        <p><a href="{}/auth/verify-email?token={}">Verify Email</a></p>
        "#,
        base_url, token
    );

    crate::common::email::send_email(email, subject, &body).await?;

    tracing::info!("Sent Email Verification link to {}", email);

    Ok(())
}

/// Verify a user's email using a token.
#[allow(dead_code)]
pub async fn verify_email(pool: &PgPool, token: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    // Hash the incoming token to compare with stored hash
    let token_hash = crate::config::hash_token(token);

    let token_row = sqlx::query_as::<_, (Uuid, Uuid)>(
        r#"
        SELECT id, user_id FROM email_verification_tokens
        WHERE token_hash = $1 AND expires_at > NOW()
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        AppError::BadRequest("Invalid or expired email verification link.".to_string())
    })?;

    let (token_id, user_id) = token_row;

    // Update user status
    sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Delete the token
    sqlx::query("DELETE FROM email_verification_tokens WHERE id = $1")
        .bind(token_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

// ─── User Settings & 2FA ───────────────────────────────────────

/// Get global user settings.
pub async fn get_user_settings(pool: &PgPool, user_id: Uuid) -> Result<UserSettings, AppError> {
    let settings =
        sqlx::query_as::<_, UserSettings>("SELECT * FROM user_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("User settings not found.".to_string()))?;

    Ok(settings)
}

/// Start TOTP enrollment by generating a new secret.
/// Returns the private secret (base32), the otpauth URL, and the QR code as a base64 data URI.
pub fn generate_totp_secret(email: &str) -> Result<(String, String, String), AppError> {
    let secret_bytes = Secret::generate_secret()
        .to_bytes()
        .map_err(|e| AppError::Internal(format!("Failed to generate TOTP secret: {}", e)))?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("POOOL".to_string()),
        email.to_string(),
    )
    .map_err(|e| AppError::Internal(format!("Failed to configure TOTP: {}", e)))?;

    let qr_code_base64 = totp.get_qr_base64().unwrap_or_default();
    Ok((totp.get_secret_base32(), totp.get_url(), qr_code_base64))
}

/// Verify a TOTP code against a secret.
pub fn verify_totp_code(secret_b32: &str, code: &str) -> bool {
    let Ok(secret_bytes) = Secret::Encoded(secret_b32.to_string()).to_bytes() else {
        return false;
    };

    let totp = match TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("POOOL".to_string()),
        "".to_string(), // account name not needed for verification
    ) {
        Ok(t) => t,
        Err(_) => return false,
    };

    totp.check_current(code).unwrap_or(false)
}

/// Verify a TOTP code with Redis-backed replay protection.
///
/// TOTP codes are valid for up to 90s (30s window × skew=1). Without
/// replay protection, a captured code can be reused within that window.
/// This helper stores `totp_used:{user_id}:{code}` in Redis with a 120s
/// TTL and refuses any second attempt.
///
/// If Redis is unavailable, falls back to the base check and logs a
/// warning. Operators should ensure Redis is reachable in production.
pub async fn verify_totp_code_with_replay_guard(
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    secret_b32: &str,
    code: &str,
) -> bool {
    if !verify_totp_code(secret_b32, code) {
        return false;
    }
    let Some(pool) = redis else {
        tracing::warn!(
            "TOTP verified without replay guard (Redis unavailable) for user {}",
            user_id
        );
        return true;
    };
    let mut conn = match pool.get().await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Redis unavailable for TOTP replay guard: {}", e);
            return true;
        }
    };
    let key = format!("totp_used:{}:{}", user_id, code);
    // SET key "1" NX EX 120 — succeeds only if key didn't exist.
    let res: Result<Option<String>, _> = redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(120)
        .query_async(&mut *conn)
        .await;
    match res {
        Ok(Some(_)) => true,
        Ok(None) => {
            tracing::warn!("TOTP replay blocked for user {}", user_id);
            sentry::capture_message(
                &format!("TOTP replay blocked: user {}", user_id),
                sentry::Level::Warning,
            );
            false
        }
        Err(e) => {
            tracing::warn!("TOTP replay check failed: {}", e);
            true
        }
    }
}

/// Enable TOTP for a user after successful verification of the first code.
pub async fn enable_totp(pool: &PgPool, user_id: Uuid, secret_b32: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE user_settings SET totp_secret = $1, totp_enabled = TRUE WHERE user_id = $2",
    )
    .bind(secret_b32)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if a user is an administrator.
pub async fn is_admin(pool: &PgPool, user_id: Uuid) -> Result<bool, AppError> {
    let has_role = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM user_roles ur
        JOIN roles r ON ur.role_id = r.id
        WHERE ur.user_id = $1 AND r.name IN ('admin', 'super_admin') AND ur.is_active = TRUE
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(has_role > 0)
}

// ─── User Profile (for /api/me) ────────────────────────────────

/// User profile data returned by the /api/me endpoint.
/// Contains only display-safe fields – never password hashes or internal IDs.
#[derive(serde::Serialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: String,
    pub initials: String,
    pub role: String,
    /// All roles assigned to this user (e.g. ["investor", "developer", "admin"]).
    pub roles: Vec<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub address_line_1: Option<String>,
    pub postal_code: Option<String>,
}

/// Get the user's display profile by their session token.
///
/// Fetches user data from users + user_profiles, then fetches ALL roles
/// separately to ensure multi-role users see all their roles.
/// Used by the /api/me endpoint.
pub async fn get_user_profile(
    pool: &PgPool,
    session_token: &str,
) -> Result<Option<UserProfile>, AppError> {
    // 1. Fetch user + profile data (no role join — avoids LIMIT 1 hiding roles)
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"
        SELECT u.id,
               u.email,
               p.first_name,
               p.last_name,
               p.phone_number,
               p.country,
               p.city,
               p.address_line_1,
               p.postal_code
        FROM users u
        JOIN user_sessions s ON u.id = s.user_id
        LEFT JOIN user_profiles p ON u.id = p.user_id
        WHERE s.session_token = $1
          AND s.expires_at > NOW()
          AND u.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(session_token)
    .fetch_optional(pool)
    .await?;

    let Some((
        user_id,
        email,
        first_name,
        last_name,
        phone_number,
        country,
        city,
        address_line_1,
        postal_code,
    )) = row
    else {
        return Ok(None);
    };

    // 2. Fetch ALL roles for this user
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT r.name FROM user_roles ur JOIN roles r ON ur.role_id = r.id WHERE ur.user_id = $1 ORDER BY r.name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // Primary role: pick the highest-privilege role for backwards compat
    let role = if roles.iter().any(|r| r == "super_admin") {
        "super_admin".to_string()
    } else if roles.iter().any(|r| r == "admin") {
        "admin".to_string()
    } else if roles.iter().any(|r| r == "developer") {
        "developer".to_string()
    } else {
        roles
            .first()
            .cloned()
            .unwrap_or_else(|| "investor".to_string())
    };

    let first = first_name.clone().unwrap_or_default();
    let last = last_name.clone().unwrap_or_default();

    // Build display name: "First Last", or email username if no name set
    let name = if first.is_empty() && last.is_empty() {
        email.split('@').next().unwrap_or("User").to_string()
    } else {
        format!("{} {}", first, last).trim().to_string()
    };

    // Generate initials from name
    let initials = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    let initials = if initials.is_empty() {
        email
            .chars()
            .next()
            .unwrap_or('U')
            .to_uppercase()
            .to_string()
    } else {
        initials
    };

    Ok(Some(UserProfile {
        id: user_id.to_string(),
        name,
        email,
        initials,
        role,
        roles,
        first_name,
        last_name,
        phone_number,
        country,
        city,
        address_line_1,
        postal_code,
    }))
}

// ─── Private helpers ───────────────────────────────────────────

/// Hash a password with Argon2id (recommended for financial applications).
fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash.
fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash format: {}", e)))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Unauthorized("Invalid email or password.".to_string()))?;

    Ok(())
}

/// Generate a cryptographically secure session token.
/// 64 random bytes → URL-safe base64 = 86 chars, 512 bits of entropy.
fn generate_session_token() -> String {
    let mut bytes = [0u8; 64];
    rand::Rng::fill(&mut rand::thread_rng(), &mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
