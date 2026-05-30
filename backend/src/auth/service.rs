use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
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
    password_hash::{rand_core::OsRng as PasswordOsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
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

/// Register a password user and persist legal consent plus an email
/// verification token in the same transaction as account creation.
pub async fn register_user_with_consent_and_verification(
    pool: &PgPool,
    email: &str,
    password: &str,
    terms_version: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(User, String), AppError> {
    let email = email.trim().to_lowercase();

    validation::validate_email(&email)?;
    validation::validate_password(password)?;

    let existing = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE email = $1")
        .bind(&email)
        .fetch_one(pool)
        .await?;

    if existing > 0 {
        return Err(AppError::Conflict(
            "An account with this email already exists.".to_string(),
        ));
    }

    let password_hash = hash_password(password)?;
    let verification_token = generate_session_token();
    let verification_hash = crate::config::hash_token(&verification_token);
    let verification_expires_at = Utc::now() + Duration::hours(24);

    let mut tx = pool.begin().await?;

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

    sqlx::query("INSERT INTO user_profiles (user_id) VALUES ($1)")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO wallets (user_id, wallet_type, balance_cents) VALUES ($1, 'cash', 0), ($1, 'rewards', 0)",
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_roles (user_id, role_id)
        SELECT $1, id FROM roles WHERE name = 'investor'
        "#,
    )
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO user_settings (user_id) VALUES ($1)")
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO user_consents (user_id, terms_version, ip_address, user_agent) VALUES ($1, $2, $3, $4)"
    )
    .bind(user.id)
    .bind(terms_version)
    .bind(ip_address)
    .bind(user_agent)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user.id)
    .bind(&verification_hash)
    .bind(verification_expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(
        "New user registered pending email verification: {}",
        user.id
    );

    Ok((user, verification_token))
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
                    sentry::capture_message("Failed login: unknown email", sentry::Level::Warning);
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
                scope.set_user(Some(sentry::User {
                    id: Some(user.id.to_string()),
                    ..Default::default()
                }));
            },
            || {
                sentry::capture_message("Failed login: OAuth-only account", sentry::Level::Warning);
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
                            "Login blocked: email not verified",
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
                        ..Default::default()
                    }));
                },
                || {
                    sentry::capture_message("Failed login: wrong password", sentry::Level::Warning);
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
pub async fn rotate_session_token(pool: &PgPool, old_token: &str) -> Result<String, AppError> {
    let new_token = generate_session_token();
    let affected = sqlx::query(
        "UPDATE user_sessions
            SET session_token = $1, is_2fa_verified = TRUE
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
          AND u.email_verified = TRUE
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
        tracing::warn!(
            "Session {}… not found, expired, unverified, or inactive in DB",
            tok_preview
        );
        Ok(None)
    }
}

/// Look up a user by session token for frozen-account self-service flows.
///
/// Standard protected routes intentionally use [`get_user_by_session`], which
/// only returns active users. Frozen users still need one narrow authenticated
/// path to request compliance review, so this variant permits `status='frozen'`
/// while keeping email verification, expiry, and 2FA-session enforcement.
pub async fn get_user_by_session_allowing_frozen(
    pool: &PgPool,
    session_token: &str,
) -> Result<Option<User>, AppError> {
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
          AND u.status IN ('active', 'frozen')
          AND u.email_verified = TRUE
        "#,
        session_token
    )
    .fetch_optional(pool)
    .await?;

    let tok_preview = &session_token[..8.min(session_token.len())];

    if let Some(r) = row {
        if r.totp_enabled.unwrap_or(false) && !r.is_2fa_verified {
            tracing::warn!(
                "Session {}… denied for frozen self-service: totp_enabled={} is_2fa_verified={}",
                tok_preview,
                r.totp_enabled.unwrap_or(false),
                r.is_2fa_verified
            );
            return Ok(None);
        }

        tracing::info!(
            "Session {}… valid for frozen self-service user {}",
            tok_preview,
            r.email
        );

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
        tracing::warn!(
            "Session {}… not found, expired, unverified, inactive, or non-frozen for self-service",
            tok_preview
        );
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
        user_id = %user.id,
        provider = provider,
        "New OAuth user registered"
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
    validation::validate_email(&email)?;

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

    let mut tx = pool.begin().await?;

    let token_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(&mut *tx)
    .await?;

    let outbox_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO password_reset_email_outbox (
            user_id,
            password_reset_token_id,
            recipient_email,
            subject,
            html_body
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(user.id)
    .bind(token_id)
    .bind(&email)
    .bind(subject)
    .bind(body)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    crate::common::email::send_password_reset_outbox_item(pool, outbox_id).await;

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

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Queue via durable outbox — token stays in DB regardless of email delivery
    // outcome so the user can always request a resend or click a cached link.
    let subject = "Verify your POOOL email";
    let body = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Welcome to POOOL!</h2>
  <p>Please click the link below to verify your email address.</p>
  <p><a href="{base_url}/auth/verify-email?token={token}" style="display:inline-block;padding:12px 24px;background:#3D00F5;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Verify Email</a></p>
  <p style="color:#717680;font-size:13px;margin-top:32px;">Link expires in 24 hours. If you didn't create an account, ignore this email.</p>
</div>"#
    );

    let outbox_id = sqlx::query_scalar::<_, uuid::Uuid>(
        r#"INSERT INTO transactional_email_outbox
               (user_id, event_type, recipient_email, subject, html_body)
           VALUES ($1, 'verify_email', $2, $3, $4)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(email)
    .bind(subject)
    .bind(&body)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(id) = outbox_id {
        crate::common::email::send_transactional_outbox_item(pool, id).await;
    }

    tracing::info!("Queued email verification for {}", email);

    Ok(())
}

/// Send an email verification message for an already-persisted token.
/// Kept for backwards-compat with resend flows; prefer the outbox path in `create_email_verification_token`.
pub async fn send_email_verification(
    email: &str,
    base_url: &str,
    token: &str,
) -> Result<(), AppError> {
    let subject = "Verify your POOOL email";
    let body = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto;padding:32px 24px;">
  <h2 style="color:#01011C;">Welcome to POOOL!</h2>
  <p>Please click the link below to verify your email address:</p>
  <p><a href="{base_url}/auth/verify-email?token={token}" style="display:inline-block;padding:12px 24px;background:#3D00F5;color:#fff;text-decoration:none;border-radius:8px;font-weight:600;">Verify Email</a></p>
</div>"#
    );

    crate::common::email::send_email(email, subject, &body).await?;

    Ok(())
}

/// Verify a user's email using a token.
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

    let qr_code_base64 = totp
        .get_qr_base64()
        .map_err(|e| AppError::Internal(format!("Failed to generate TOTP QR code: {}", e)))?;
    Ok((totp.get_secret_base32(), totp.get_url(), qr_code_base64))
}

const TOTP_SECRET_PREFIX: &str = "enc:v1";
const TOTP_SETUP_PREFIX: &str = "totp_setup:v1";

fn totp_encryption_key() -> Result<[u8; 32], AppError> {
    let (key_name, raw) = std::env::var("TOTP_SECRET_ENCRYPTION_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| ("TOTP_SECRET_ENCRYPTION_KEY", value))
        .or_else(|| {
            std::env::var("ENCRYPTION_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|value| ("ENCRYPTION_KEY", value))
        })
        .ok_or_else(|| {
            AppError::Internal(
                "TOTP_SECRET_ENCRYPTION_KEY or ENCRYPTION_KEY is not configured.".to_string(),
            )
        })?;

    let trimmed = raw.trim();
    let bytes = if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(trimmed)
            .map_err(|_| AppError::Internal(format!("{} is not valid hex.", key_name)))?
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(trimmed)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed))
            .unwrap_or_else(|_| trimmed.as_bytes().to_vec())
    };

    bytes
        .try_into()
        .map_err(|_| AppError::Internal(format!("{} must decode to 32 bytes.", key_name)))
}

fn encrypt_secret_payload(prefix: &str, plaintext: &str) -> Result<String, AppError> {
    let key = totp_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AppError::Internal("Failed to initialize TOTP encryption.".to_string()))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| AppError::Internal("Failed to encrypt TOTP secret.".to_string()))?;
    Ok(format!(
        "{}:{}:{}",
        prefix,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce_bytes),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

fn decrypt_secret_payload(prefix: &str, value: &str) -> Result<String, AppError> {
    let Some(rest) = value.strip_prefix(prefix).and_then(|s| s.strip_prefix(':')) else {
        return Err(AppError::BadRequest(
            "Invalid TOTP setup token.".to_string(),
        ));
    };
    let mut parts = rest.splitn(2, ':');
    let nonce = parts
        .next()
        .ok_or_else(|| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    let ciphertext = parts
        .next()
        .ok_or_else(|| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(nonce)
        .map_err(|_| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    let ciphertext = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(ciphertext)
        .map_err(|_| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    if nonce.len() != 12 {
        return Err(AppError::BadRequest(
            "Invalid TOTP setup token.".to_string(),
        ));
    }

    let key = totp_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AppError::Internal("Failed to initialize TOTP encryption.".to_string()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| AppError::BadRequest("Invalid or expired TOTP setup token.".to_string()))?;
    String::from_utf8(plaintext)
        .map_err(|_| AppError::BadRequest("Invalid TOTP setup token.".to_string()))
}

pub fn build_totp_setup_token(user_id: Uuid, secret_b32: &str) -> Result<String, AppError> {
    let expires_at = (Utc::now() + Duration::minutes(10)).timestamp();
    encrypt_secret_payload(
        TOTP_SETUP_PREFIX,
        &format!("{}:{}:{}", user_id, expires_at, secret_b32),
    )
}

pub fn read_totp_setup_token(token: &str, expected_user_id: Uuid) -> Result<String, AppError> {
    let payload = decrypt_secret_payload(TOTP_SETUP_PREFIX, token)?;
    let mut parts = payload.splitn(3, ':');
    let user_id = parts
        .next()
        .ok_or_else(|| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    let expires_at = parts
        .next()
        .ok_or_else(|| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    let secret = parts
        .next()
        .ok_or_else(|| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    let user_id = Uuid::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    if user_id != expected_user_id {
        return Err(AppError::Forbidden(
            "TOTP setup token does not match this session.".to_string(),
        ));
    }
    let expires_at = expires_at
        .parse::<i64>()
        .map_err(|_| AppError::BadRequest("Invalid TOTP setup token.".to_string()))?;
    if Utc::now().timestamp() > expires_at {
        return Err(AppError::BadRequest(
            "This 2FA setup session expired. Refresh the page and try again.".to_string(),
        ));
    }
    Ok(secret.to_string())
}

pub fn encrypt_totp_secret(secret_b32: &str) -> Result<String, AppError> {
    encrypt_secret_payload(TOTP_SECRET_PREFIX, secret_b32)
}

pub fn decrypt_stored_totp_secret(stored_secret: &str) -> Result<String, AppError> {
    if stored_secret.starts_with(&format!("{}:", TOTP_SECRET_PREFIX)) {
        decrypt_secret_payload(TOTP_SECRET_PREFIX, stored_secret)
    } else {
        tracing::warn!(
            "Using legacy plaintext TOTP secret; migrate this user to encrypted storage"
        );
        Ok(stored_secret.to_string())
    }
}

pub async fn user_totp_enabled(pool: &PgPool, user_id: Uuid) -> Result<bool, AppError> {
    let enabled = sqlx::query_scalar::<_, bool>(
        "SELECT COALESCE(totp_enabled, FALSE) FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(false);
    Ok(enabled)
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
    let encrypted_secret = encrypt_totp_secret(secret_b32)?;
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO user_settings (user_id, totp_secret, totp_enabled, updated_at)
        VALUES ($1, $2, TRUE, NOW())
        ON CONFLICT (user_id)
        DO UPDATE SET totp_secret = EXCLUDED.totp_secret, totp_enabled = TRUE, updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&encrypted_secret)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id)
        VALUES ($1, 'totp_enabled', 'user', $2)
        "#,
    )
    .bind(user_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
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
    pub avatar_url: Option<String>,
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
               p.postal_code,
               u.avatar_url
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
        avatar_url,
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
        avatar_url,
    }))
}

// ─── Private helpers ───────────────────────────────────────────

/// Hash a password with Argon2id (recommended for financial applications).
fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut PasswordOsRng);
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

// ─── Passkey / WebAuthn ────────────────────────────────────────

use webauthn_rs::prelude::{
    DiscoverableAuthentication, DiscoverableKey, Passkey, PasskeyRegistration,
    RegisterPublicKeyCredential, PublicKeyCredential as WebAuthnPublicKeyCredential, Webauthn,
};

/// Store a passkey challenge (registration or authentication) in DB.
/// Returns the challenge_id UUID to be sent to the frontend.
pub async fn store_passkey_challenge(
    pool: &PgPool,
    user_id: Option<Uuid>,
    kind: &str,
    state: &serde_json::Value,
) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO passkey_challenges (id, user_id, kind, state_data)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(kind)
    .bind(state)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Fetch and delete a passkey challenge row. Returns (user_id, state_data).
/// Returns NotFound / Gone if missing or expired.
pub async fn consume_passkey_challenge(
    pool: &PgPool,
    challenge_id: Uuid,
    expected_kind: &str,
) -> Result<(Option<Uuid>, serde_json::Value), AppError> {
    #[derive(sqlx::FromRow)]
    struct ChallengeRow {
        user_id: Option<Uuid>,
        state_data: serde_json::Value,
    }
    let row = sqlx::query_as::<_, ChallengeRow>(
        "DELETE FROM passkey_challenges WHERE id = $1 AND kind = $2 AND expires_at > NOW() RETURNING user_id, state_data",
    )
    .bind(challenge_id)
    .bind(expected_kind)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Challenge expired or not found.".to_string()))?;

    Ok((row.user_id, row.state_data))
}

/// Start passkey registration for an authenticated user.
/// Returns (challenge_id, CreationChallengeResponse as JSON).
pub async fn start_passkey_registration(
    pool: &PgPool,
    webauthn: &Webauthn,
    user_id: Uuid,
    email: &str,
    display_name: &str,
) -> Result<(Uuid, serde_json::Value), AppError> {
    // Collect existing credential IDs to exclude (avoid duplicate registrations).
    let existing: Vec<String> =
        sqlx::query_scalar("SELECT credential_id FROM passkey_credentials WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await?;

    let exclude_credentials = if existing.is_empty() {
        None
    } else {
        use webauthn_rs::prelude::CredentialID;
        let ids: Vec<CredentialID> = existing
            .iter()
            .filter_map(|s| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(s)
                    .ok()
                    .map(|b| b.into())
            })
            .collect();
        Some(ids)
    };

    let (ccr, reg_state) = webauthn
        .start_passkey_registration(user_id, email, display_name, exclude_credentials)
        .map_err(|e| AppError::Internal(format!("WebAuthn registration start failed: {e}")))?;

    let state_json = serde_json::to_value(&reg_state)
        .map_err(|e| AppError::Internal(format!("WebAuthn state serialisation failed: {e}")))?;

    let challenge_id = store_passkey_challenge(pool, Some(user_id), "register", &state_json).await?;
    let options = serde_json::to_value(&ccr)
        .map_err(|e| AppError::Internal(format!("WebAuthn CCR serialisation failed: {e}")))?;

    Ok((challenge_id, options))
}

/// Finish passkey registration: verify the browser response and persist the credential.
pub async fn finish_passkey_registration(
    pool: &PgPool,
    webauthn: &Webauthn,
    challenge_id: Uuid,
    expected_user_id: Uuid,
    credential_json: serde_json::Value,
    name: Option<String>,
) -> Result<(), AppError> {
    let (stored_user_id, state_json) =
        consume_passkey_challenge(pool, challenge_id, "register").await?;

    // Verify this challenge belongs to the authenticated user.
    if stored_user_id != Some(expected_user_id) {
        return Err(AppError::Unauthorized("Challenge user mismatch.".to_string()));
    }

    let reg_state: PasskeyRegistration = serde_json::from_value(state_json)
        .map_err(|e| AppError::Internal(format!("WebAuthn state deserialisation failed: {e}")))?;

    let rpkc: RegisterPublicKeyCredential = serde_json::from_value(credential_json)
        .map_err(|_| AppError::BadRequest("Invalid credential format.".to_string()))?;

    let passkey = webauthn
        .finish_passkey_registration(&rpkc, &reg_state)
        .map_err(|e| AppError::Unauthorized(format!("Passkey registration failed: {e}")))?;

    let credential_id = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(passkey.cred_id().as_ref());

    let passkey_json = serde_json::to_value(&passkey)
        .map_err(|e| AppError::Internal(format!("Passkey serialisation failed: {e}")))?;

    let passkey_name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Passkey".to_string());

    sqlx::query(
        "INSERT INTO passkey_credentials (user_id, credential_id, passkey_data, name) VALUES ($1, $2, $3, $4) ON CONFLICT (credential_id) DO NOTHING",
    )
    .bind(expected_user_id)
    .bind(&credential_id)
    .bind(passkey_json)
    .bind(&passkey_name)
    .execute(pool)
    .await?;

    Ok(())
}

/// Start discoverable (username-less) passkey authentication.
/// Returns (challenge_id, RequestChallengeResponse as JSON).
pub async fn start_passkey_authentication(
    pool: &PgPool,
    webauthn: &Webauthn,
) -> Result<(Uuid, serde_json::Value), AppError> {
    let (rcr, auth_state) = webauthn
        .start_discoverable_authentication()
        .map_err(|e| AppError::Internal(format!("WebAuthn auth start failed: {e}")))?;

    let state_json = serde_json::to_value(&auth_state)
        .map_err(|e| AppError::Internal(format!("WebAuthn auth state serialisation failed: {e}")))?;

    let challenge_id = store_passkey_challenge(pool, None, "authenticate", &state_json).await?;
    let options = serde_json::to_value(&rcr)
        .map_err(|e| AppError::Internal(format!("WebAuthn RCR serialisation failed: {e}")))?;

    Ok((challenge_id, options))
}

/// Finish discoverable passkey authentication.
/// Returns the authenticated User on success.
pub async fn finish_passkey_authentication(
    pool: &PgPool,
    webauthn: &Webauthn,
    challenge_id: Uuid,
    credential_json: serde_json::Value,
) -> Result<User, AppError> {
    let (_, state_json) = consume_passkey_challenge(pool, challenge_id, "authenticate").await?;

    let auth_state: DiscoverableAuthentication = serde_json::from_value(state_json)
        .map_err(|e| AppError::Internal(format!("WebAuthn state deserialisation failed: {e}")))?;

    let pkc: WebAuthnPublicKeyCredential = serde_json::from_value(credential_json)
        .map_err(|_| AppError::BadRequest("Invalid credential format.".to_string()))?;

    // Look up the credential by ID to find which user and passkey to use.
    let cred_id_b64 = &pkc.id;
    #[derive(sqlx::FromRow)]
    struct PkRow {
        user_id: Uuid,
        passkey_data: serde_json::Value,
    }
    let row = sqlx::query_as::<_, PkRow>(
        "SELECT user_id, passkey_data FROM passkey_credentials WHERE credential_id = $1",
    )
    .bind(cred_id_b64)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Passkey not registered on this account.".to_string()))?;

    let user_id: Uuid = row.user_id;
    let passkey: Passkey = serde_json::from_value(row.passkey_data)
        .map_err(|e| AppError::Internal(format!("Passkey deserialisation failed: {e}")))?;

    let auth_result = webauthn
        .finish_discoverable_authentication(&pkc, auth_state, &[DiscoverableKey::from(&passkey)])
        .map_err(|e| AppError::Unauthorized(format!("Passkey authentication failed: {e}")))?;

    // Update sign_count to guard against cloning.
    let mut passkey = passkey;
    if passkey.update_credential(&auth_result) == Some(true) {
        if let Ok(json) = serde_json::to_value(&passkey) {
            let _ = sqlx::query(
                "UPDATE passkey_credentials SET passkey_data = $1 WHERE credential_id = $2",
            )
            .bind(json)
            .bind(cred_id_b64)
            .execute(pool)
            .await;
        }
    }

    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1 AND status = 'active'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Account not found or inactive.".to_string()))?;

    Ok(user)
}

/// List all passkeys registered by a user.
pub async fn list_user_passkeys(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<super::models::PasskeyCredential>, AppError> {
    let rows = sqlx::query_as::<_, super::models::PasskeyCredential>(
        "SELECT id, user_id, credential_id, name, created_at FROM passkey_credentials WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Delete a single passkey credential by ID, verifying ownership.
pub async fn delete_passkey(
    pool: &PgPool,
    user_id: Uuid,
    passkey_id: Uuid,
) -> Result<(), AppError> {
    let result =
        sqlx::query("DELETE FROM passkey_credentials WHERE id = $1 AND user_id = $2")
            .bind(passkey_id)
            .bind(user_id)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Passkey not found.".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install_test_totp_key() {
        std::env::set_var(
            "TOTP_SECRET_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
    }

    #[test]
    fn totp_secret_encryption_round_trips_and_is_not_plaintext() {
        install_test_totp_key();

        let encrypted = encrypt_totp_secret("JBSWY3DPEHPK3PXP").expect("encrypt secret");

        assert!(encrypted.starts_with("enc:v1:"));
        assert!(!encrypted.contains("JBSWY3DPEHPK3PXP"));
        assert_eq!(
            decrypt_stored_totp_secret(&encrypted).expect("decrypt secret"),
            "JBSWY3DPEHPK3PXP"
        );
    }

    #[test]
    fn totp_setup_token_is_bound_to_user() {
        install_test_totp_key();

        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let token = build_totp_setup_token(user_id, "JBSWY3DPEHPK3PXP").expect("build token");

        assert!(token.starts_with("totp_setup:v1:"));
        assert!(!token.contains("JBSWY3DPEHPK3PXP"));
        assert_eq!(
            read_totp_setup_token(&token, user_id).expect("read token"),
            "JBSWY3DPEHPK3PXP"
        );
        assert!(read_totp_setup_token(&token, other_user_id).is_err());
    }
}
