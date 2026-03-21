/// Step-up 2FA — Phase 1.4, 1.5, 1.6
///
/// Implements:
/// - Financial action classification with amount thresholds
/// - Trading session creation/validation via Redis (15-min TTL)
/// - Step-up verification for sensitive financial operations
///
/// SECURITY INVARIANTS:
/// - Sessions are stored in Redis, never in browser-accessible storage
/// - TOTP secrets are never logged or included in error messages
/// - Threshold amounts use i64 cents — never floats
use uuid::Uuid;

use crate::error::AppError;

// ─── Threshold Constants (Phase 1.6) ───────────────────────────

/// Withdrawals above this amount require step-up 2FA (in cents).
pub const WITHDRAWAL_2FA_THRESHOLD_CENTS: i64 = 10_000; // $100

/// Trades above this amount require step-up 2FA (in cents).
pub const TRADE_2FA_THRESHOLD_CENTS: i64 = 50_000; // $500

/// If wallet balance exceeds this, user MUST set up 2FA (in cents).
pub const WALLET_2FA_SETUP_THRESHOLD_CENTS: i64 = 100_000; // $1,000

/// Trading session TTL in seconds (Phase 1.5).
const TRADING_SESSION_TTL_SECS: u64 = 900; // 15 minutes

// ─── Financial Action Classification ────────────────────────────

/// Categories of financial operations that may require step-up 2FA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinancialAction {
    /// Cash withdrawal to external account.
    Withdrawal,
    /// Token purchase (primary or secondary market).
    Trade,
    /// Adding a new payment method (card/bank).
    PaymentMethodAdd,
    /// Changing account password.
    PasswordChange,
}

impl FinancialAction {
    /// Returns the 2FA threshold in cents for this action.
    /// Operations below this amount do NOT require step-up 2FA.
    /// Returns `None` if the action always requires 2FA regardless of amount.
    pub fn threshold_cents(self) -> Option<i64> {
        match self {
            FinancialAction::Withdrawal => Some(WITHDRAWAL_2FA_THRESHOLD_CENTS),
            FinancialAction::Trade => Some(TRADE_2FA_THRESHOLD_CENTS),
            FinancialAction::PaymentMethodAdd => None, // always requires 2FA
            FinancialAction::PasswordChange => None,   // always requires 2FA
        }
    }

    /// Returns the Redis key suffix for this action type.
    fn session_key_suffix(self) -> &'static str {
        match self {
            FinancialAction::Withdrawal => "withdraw",
            FinancialAction::Trade => "trade",
            FinancialAction::PaymentMethodAdd => "pm",
            FinancialAction::PasswordChange => "pwd",
        }
    }
}

// ─── Step-Up 2FA Check (Phase 1.4) ─────────────────────────────

/// Check whether a financial operation requires step-up 2FA.
///
/// Returns `Ok(())` if the operation can proceed, or
/// `Err(AppError::TwoFactorRequired)` if the user needs to verify.
///
/// Logic:
/// 1. If user doesn't have TOTP enabled → skip (can't require what isn't set up)
/// 2. If amount is below threshold → skip
/// 3. If valid trading session exists in Redis → skip
/// 4. Otherwise → TwoFactorRequired
pub async fn require_step_up_2fa(
    db: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    action: FinancialAction,
    amount_cents: i64,
) -> Result<(), AppError> {
    // 1. Check if user has TOTP enabled
    let totp_enabled: bool = sqlx::query_scalar(
        "SELECT COALESCE(totp_enabled, FALSE) FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .unwrap_or(false);

    if !totp_enabled {
        // User hasn't set up 2FA — can't enforce step-up
        // (Task 1.6 separate check forces 2FA setup for high-balance accounts)
        return Ok(());
    }

    // 2. Check amount threshold
    if let Some(threshold) = action.threshold_cents() {
        if amount_cents < threshold {
            return Ok(());
        }
    }

    // 3. Check for existing trading session in Redis
    if check_trading_session(redis, user_id, action).await {
        return Ok(());
    }

    // 4. Step-up 2FA required
    tracing::info!(
        user_id = %user_id,
        action = ?action,
        amount_cents = amount_cents,
        "Step-up 2FA required for financial operation"
    );
    Err(AppError::TwoFactorRequired)
}

/// Check whether a user with a high wallet balance needs to set up 2FA.
///
/// Returns `Err(AppError::TwoFactorRequired)` if the user's wallet balance
/// exceeds the threshold and they haven't enabled TOTP yet.
/// This is a softer check — it signals the frontend to prompt 2FA setup,
/// not a hard block.
#[allow(dead_code)]
pub async fn check_2fa_setup_required(db: &sqlx::PgPool, user_id: Uuid) -> Result<bool, AppError> {
    // Check if 2FA is already enabled
    let totp_enabled: bool = sqlx::query_scalar(
        "SELECT COALESCE(totp_enabled, FALSE) FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .unwrap_or(false);

    if totp_enabled {
        return Ok(false); // Already set up
    }

    // Check wallet balance
    let balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(balance_cents, 0) FROM wallets WHERE user_id = $1 AND wallet_type = 'cash'",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .unwrap_or(0);

    Ok(balance >= WALLET_2FA_SETUP_THRESHOLD_CENTS)
}

// ─── Trading Session (Phase 1.5) ───────────────────────────────

/// Create a trading session after successful TOTP verification.
///
/// Stores a session key in Redis with a 15-minute TTL.
/// The session is scoped to the user and action type.
///
/// Returns `Ok(())` on success, or silently succeeds if Redis is unavailable
/// (the next `require_step_up_2fa` call will re-prompt).
pub async fn create_trading_session(
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    action: FinancialAction,
) -> Result<(), AppError> {
    let redis_pool = match redis {
        Some(r) => r,
        None => {
            tracing::warn!(
                "No Redis available — trading session not persisted (2FA will re-prompt)"
            );
            return Ok(());
        }
    };

    let key = format!(
        "trading_session:{}:{}",
        user_id,
        action.session_key_suffix()
    );
    let mut conn = redis_pool.get().await.map_err(|e| {
        tracing::error!("Failed to get Redis connection for trading session: {}", e);
        AppError::ServiceUnavailable("Session storage unavailable".to_string())
    })?;

    let result: Result<(), redis::RedisError> = redis::cmd("SET")
        .arg(&key)
        .arg("1")
        .arg("EX")
        .arg(TRADING_SESSION_TTL_SECS)
        .query_async(&mut *conn)
        .await;

    match result {
        Ok(()) => {
            tracing::info!(
                user_id = %user_id,
                action = ?action,
                ttl_secs = TRADING_SESSION_TTL_SECS,
                "Trading session created"
            );
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to create trading session in Redis: {}", e);
            Err(AppError::ServiceUnavailable(
                "Failed to create trading session".to_string(),
            ))
        }
    }
}

/// Check if a valid trading session exists for the given user and action.
///
/// Returns `true` if a session exists (user recently verified 2FA).
/// Returns `false` if no session, Redis unavailable, or any error.
async fn check_trading_session(
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    action: FinancialAction,
) -> bool {
    let redis_pool = match redis {
        Some(r) => r,
        None => return false,
    };

    let key = format!(
        "trading_session:{}:{}",
        user_id,
        action.session_key_suffix()
    );
    if let Ok(mut conn) = redis_pool.get().await {
        let exists: Result<i32, redis::RedisError> =
            redis::cmd("EXISTS").arg(&key).query_async(&mut *conn).await;
        match exists {
            Ok(1) => {
                tracing::debug!(
                    "Valid trading session found for user {} action {:?}",
                    user_id,
                    action
                );
                true
            }
            Ok(_) => false,
            Err(e) => {
                tracing::warn!("Redis EXISTS check for trading session failed: {}", e);
                false
            }
        }
    } else {
        tracing::warn!("Failed to get Redis connection for trading session check");
        false
    }
}

/// Verify a TOTP code and create a trading session for the specified action.
///
/// This is the main entry point for step-up verification:
/// 1. Looks up the user's TOTP secret
/// 2. Verifies the provided code
/// 3. Creates a 15-minute trading session in Redis
pub async fn verify_and_create_trading_session(
    db: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    code: &str,
    action: FinancialAction,
) -> Result<(), AppError> {
    // 1. Get TOTP secret
    let secret: Option<String> = sqlx::query_scalar(
        "SELECT totp_secret FROM user_settings WHERE user_id = $1 AND totp_enabled = TRUE",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .flatten();

    let secret = secret.ok_or_else(|| {
        AppError::BadRequest(
            "Two-factor authentication is not enabled on your account.".to_string(),
        )
    })?;

    // 2. Verify TOTP code
    if !super::service::verify_totp_code(&secret, code) {
        return Err(AppError::Unauthorized(
            "Invalid authentication code.".to_string(),
        ));
    }

    // 3. Create trading session
    create_trading_session(redis, user_id, action).await?;

    Ok(())
}
