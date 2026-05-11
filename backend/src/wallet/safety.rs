//! Withdrawal safety checks — Phase 18.6, 18.7, 18.8, 18.9.
//!
//! All checks read their thresholds from `platform_settings` at runtime so
//! operators can tune without redeploying. Defaults match
//! `database/128_withdrawal_safety_settings.sql`.
//!
//! Wired from [`crate::wallet::routes::handle_withdraw`] BEFORE the wallet
//! lock + balance check. Each check returns `Err(SafetyError)` with a
//! short reason string that maps to a redirect query param so the wallet
//! template can render a specific user-facing message.
//!
//! Defense-in-depth ordering:
//!
//! 1. 18.9 step-up 2FA (≥ $500)
//! 2. 18.8 new-account cooldown (first 72h after KYC verified)
//! 3. 18.6 daily volume cap ($10k/UTC-day)
//! 4. 18.7 velocity (> N requests in 24h)
//!
//! The velocity check is intentionally last because it auto-freezes the
//! account; we don't want to freeze for a request that would have failed
//! 2FA anyway.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

/// Reasons a withdrawal can be blocked by safety controls. Each variant maps
/// to a stable redirect query param the wallet UI checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyError {
    /// 18.9 — User must complete a step-up 2FA challenge for this amount.
    TwoFactorRequired,
    /// 18.8 — Account is still in the post-KYC cooldown window and this
    /// withdrawal exceeds the cooldown cap.
    NewAccountCooldown,
    /// 18.6 — Cumulative withdrawals today would exceed the daily cap.
    DailyCapExceeded,
    /// 18.7 — User has too many withdrawals in the rolling window; the
    /// account is auto-frozen pending admin review.
    VelocityFrozen,
}

impl SafetyError {
    /// Stable identifier used in the `?error=` query param so the wallet
    /// template can render a specific user-facing message.
    pub fn query_param(self) -> &'static str {
        match self {
            SafetyError::TwoFactorRequired => "withdraw_2fa_required",
            SafetyError::NewAccountCooldown => "withdraw_new_account_cooldown",
            SafetyError::DailyCapExceeded => "withdraw_daily_cap",
            SafetyError::VelocityFrozen => "withdraw_velocity_frozen",
        }
    }
}

/// Run all withdrawal safety checks in order. Returns `Ok(())` if the
/// withdrawal may proceed, or `Err(SafetyError)` describing the blocker.
///
/// `pool` is the main DB (for kyc_records + withdrawal_requests).
/// `redis` is passed through to the step-up 2FA module for trading-session
/// lookups.
pub async fn check_withdrawal_safety(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    amount_cents: i64,
) -> Result<(), SafetyError> {
    // ─── 18.9: Step-up 2FA on ≥ $500 ────────────────────────────────
    use crate::auth::step_up::{require_step_up_2fa, FinancialAction};
    match require_step_up_2fa(
        pool,
        redis,
        user_id,
        FinancialAction::Withdrawal,
        amount_cents,
    )
    .await
    {
        Ok(()) => {}
        Err(AppError::TwoFactorRequired) => return Err(SafetyError::TwoFactorRequired),
        Err(e) => {
            tracing::error!(user_id = %user_id, "Step-up 2FA precheck DB error: {}", e);
            // Fail closed: an unexpected error here is treated as a 2FA
            // requirement because we can't prove the user satisfied it.
            return Err(SafetyError::TwoFactorRequired);
        }
    }

    // ─── 18.8: New-account cooldown ──────────────────────────────────
    if let Some(cooldown_active_max_cents) = within_new_account_cooldown(pool, user_id).await {
        if amount_cents > cooldown_active_max_cents {
            tracing::info!(
                user_id = %user_id,
                amount_cents,
                cooldown_active_max_cents,
                "Withdrawal blocked: new-account cooldown cap exceeded"
            );
            return Err(SafetyError::NewAccountCooldown);
        }
    }

    // ─── 18.6: Daily volume cap ──────────────────────────────────────
    let daily_cap = setting_i64(pool, "withdrawal_daily_cap_cents", 1_000_000).await;
    let used_today = withdrawn_today_cents(pool, user_id).await;
    if used_today.saturating_add(amount_cents) > daily_cap {
        tracing::info!(
            user_id = %user_id,
            amount_cents,
            used_today,
            daily_cap,
            "Withdrawal blocked: daily cap"
        );
        return Err(SafetyError::DailyCapExceeded);
    }

    // ─── 18.7: Velocity + auto-freeze ────────────────────────────────
    let velocity_threshold = setting_i64(pool, "withdrawal_velocity_threshold", 3).await;
    let velocity_window_h = setting_i64(pool, "withdrawal_velocity_window_hours", 24).await;
    let recent_count = withdrawals_in_window(pool, user_id, velocity_window_h).await;
    if recent_count > velocity_threshold {
        tracing::warn!(
            user_id = %user_id,
            recent_count,
            velocity_threshold,
            window_hours = velocity_window_h,
            "Withdrawal blocked: velocity exceeded — auto-freezing user pending admin review"
        );
        let _ = freeze_user(pool, user_id).await;
        return Err(SafetyError::VelocityFrozen);
    }

    Ok(())
}

/// Returns `Some(max_allowed_cents_during_cooldown)` if the user is still
/// in the post-KYC cooldown window, else `None`.
async fn within_new_account_cooldown(pool: &PgPool, user_id: Uuid) -> Option<i64> {
    let cooldown_hours = setting_i64(pool, "new_account_cooldown_hours", 72).await;
    let cooldown_max = setting_i64(pool, "new_account_max_withdraw_cents", 100_000).await;

    // KYC verified_at is the start of the cooldown window. Users who never
    // verified KYC are blocked from withdrawing earlier in handle_withdraw,
    // so we treat "no verified_at" here as "not in cooldown".
    let verified_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT MAX(verified_at) FROM kyc_records
         WHERE user_id = $1 AND status IN ('approved', 'verified', 'completed')",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let verified_at = verified_at?;
    let elapsed = chrono::Utc::now().signed_duration_since(verified_at);
    if elapsed.num_hours() < cooldown_hours {
        Some(cooldown_max)
    } else {
        None
    }
}

async fn withdrawn_today_cents(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint
         FROM withdrawal_requests
         WHERE user_id = $1
           AND status IN ('pending', 'approved', 'completed', 'processing')
           AND created_at >= date_trunc('day', NOW() AT TIME ZONE 'UTC')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0)
}

async fn withdrawals_in_window(pool: &PgPool, user_id: Uuid, window_hours: i64) -> i64 {
    sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COUNT(*)::bigint FROM withdrawal_requests
         WHERE user_id = $1
           AND created_at >= NOW() - ($2::int || ' hours')::interval",
    )
    .bind(user_id)
    .bind(window_hours as i32)
    .fetch_one(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0)
}

async fn freeze_user(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET status = 'frozen' WHERE id = $1 AND status = 'active'")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn setting_i64(pool: &PgPool, key: &str, default: i64) -> i64 {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value FROM platform_settings WHERE key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    raw.and_then(|v| v.parse::<i64>().ok()).unwrap_or(default)
}
