/// Step-up 2FA — Phase 1.4, 1.5, 1.6
///
/// Implements:
/// - Financial action classification with amount thresholds
/// - Trading session creation/validation via Redis plus DB fallback (15-min TTL)
/// - Step-up verification for sensitive financial operations
///
/// SECURITY INVARIANTS:
/// - Sessions are stored server-side, never in browser-accessible storage
/// - TOTP secrets are never logged or included in error messages
/// - Threshold amounts use i64 cents — never floats
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::error::AppError;

// ─── Threshold Constants (Phase 1.6) ───────────────────────────

/// Withdrawals above this amount require step-up 2FA (in cents).
/// Phase 18.9: raised from $100 to $500 to match `IMPLEMENTATION_ROADMAP.md`.
pub const WITHDRAWAL_2FA_THRESHOLD_CENTS: i64 = 50_000; // $500

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
    /// Phase-3 P1: editing the affiliate / team-business IBAN. Always
    /// requires 2FA because the IBAN dictates where every future payout
    /// lands — account-takeover via session cookie would otherwise let an
    /// attacker silently divert funds.
    AffiliateBankEdit,
    /// Phase-3 P1: requesting an affiliate payout. Always requires 2FA
    /// so a stolen session cookie can't trigger withdrawals against the
    /// authenticated affiliate's payable balance.
    AffiliatePayoutRequest,
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
            FinancialAction::AffiliateBankEdit => None, // always
            FinancialAction::AffiliatePayoutRequest => None, // always
        }
    }

    /// Returns the Redis key suffix for this action type.
    fn session_key_suffix(self) -> &'static str {
        match self {
            FinancialAction::Withdrawal => "withdraw",
            FinancialAction::Trade => "trade",
            FinancialAction::PaymentMethodAdd => "pm",
            FinancialAction::PasswordChange => "pwd",
            FinancialAction::AffiliateBankEdit => "aff_bank",
            FinancialAction::AffiliatePayoutRequest => "aff_payout",
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
/// 3. If valid server-side step-up session exists → skip
/// 4. Otherwise → TwoFactorRequired
pub async fn require_step_up_2fa(
    db: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    action: FinancialAction,
    amount_cents: i64,
) -> Result<(), AppError> {
    // 1. Check amount threshold — operations below the threshold skip step-up.
    if let Some(threshold) = action.threshold_cents() {
        if amount_cents < threshold {
            return Ok(());
        }
    }

    // 2. TOTP enrollment required at/above threshold. No implicit bypass for
    //    unenrolled users — step-up must fail closed so high-value operations
    //    cannot proceed without 2FA. Frontend handles the enrollment prompt
    //    via the TwoFactorRequired response.
    let totp_enabled: bool = sqlx::query_scalar(
        "SELECT COALESCE(totp_enabled, FALSE) FROM user_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .unwrap_or(false);

    if !totp_enabled {
        // Audit L#10: do NOT log `amount_cents` — log aggregators would
        // otherwise see a per-user "wants to do $X txn" stream. The
        // coarse action enum is operationally useful and not PII.
        tracing::info!(
            user_id = %user_id,
            action = ?action,
            "Step-up 2FA required — user has not enrolled TOTP"
        );
        return Err(AppError::TwoFactorRequired);
    }

    // 3. Check for existing server-side step-up session
    if check_trading_session(db, redis, user_id, action).await {
        return Ok(());
    }

    // 4. Step-up 2FA required
    tracing::info!(
        user_id = %user_id,
        action = ?action,
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
/// Stores a session key server-side with a 15-minute TTL.
/// The session is scoped to the user and action type.
///
/// Redis is the fast path, but the PostgreSQL row is the durable fallback for
/// environments where Redis is unavailable or intentionally disabled.
pub async fn create_trading_session(
    db: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    action: FinancialAction,
) -> Result<(), AppError> {
    let expires_at = Utc::now() + Duration::seconds(TRADING_SESSION_TTL_SECS as i64);

    sqlx::query(
        r#"
        INSERT INTO step_up_sessions (user_id, action, expires_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, action)
        DO UPDATE SET expires_at = EXCLUDED.expires_at, updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(action.session_key_suffix())
    .bind(expires_at)
    .execute(db)
    .await?;

    let Some(redis_pool) = redis else {
        tracing::debug!(
            user_id = %user_id,
            action = ?action,
            ttl_secs = TRADING_SESSION_TTL_SECS,
            "Step-up session stored in PostgreSQL fallback; Redis unavailable"
        );
        return Ok(());
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
/// Returns `false` if no session or any storage error occurs.
async fn check_trading_session(
    db: &sqlx::PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    action: FinancialAction,
) -> bool {
    let key = format!(
        "trading_session:{}:{}",
        user_id,
        action.session_key_suffix()
    );

    if let Some(redis_pool) = redis {
        if let Ok(mut conn) = redis_pool.get().await {
            let exists: Result<i32, redis::RedisError> =
                redis::cmd("EXISTS").arg(&key).query_async(&mut *conn).await;
            match exists {
                Ok(1) => {
                    tracing::debug!(
                        "Valid Redis step-up session found for user {} action {:?}",
                        user_id,
                        action
                    );
                    return true;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Redis EXISTS check for trading session failed: {}", e);
                }
            }
        } else {
            tracing::warn!("Failed to get Redis connection for trading session check");
        }
    }

    let action_key = action.session_key_suffix();
    let valid_db_session = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM step_up_sessions
             WHERE user_id = $1
               AND action = $2
               AND expires_at > NOW()
        )",
    )
    .bind(user_id)
    .bind(action_key)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .unwrap_or(false);

    if valid_db_session {
        let _ = sqlx::query(
            "UPDATE step_up_sessions SET updated_at = NOW() WHERE user_id = $1 AND action = $2",
        )
        .bind(user_id)
        .bind(action_key)
        .execute(db)
        .await;
        tracing::debug!(
            "Valid PostgreSQL step-up session found for user {} action {:?}",
            user_id,
            action
        );
    }

    valid_db_session
}

/// Verify a TOTP code and create a trading session for the specified action.
///
/// This is the main entry point for step-up verification:
/// 1. Looks up the user's TOTP secret
/// 2. Verifies the provided code
/// 3. Creates a 15-minute server-side step-up session
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
    let secret = super::service::decrypt_stored_totp_secret(&secret)?;

    // 2. Verify TOTP code with Redis-backed replay protection
    if !super::service::verify_totp_code_with_replay_guard(redis, user_id, &secret, code).await {
        return Err(AppError::Unauthorized(
            "Invalid authentication code.".to_string(),
        ));
    }

    // 3. Create trading session
    create_trading_session(db, redis, user_id, action).await?;

    Ok(())
}
