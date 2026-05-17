//! Phase-4: opt-in public affiliate leaderboard.
//!
//! Materialised view `affiliate_leaderboard_public` (mig 192) snapshots
//! the top affiliates by paid commission. The view is refreshed every
//! 15 min by the worker below; reads hit the view directly so the public
//! page is O(1) regardless of commission volume.
//!
//! All public reads go through `list_public_leaderboard` so the privacy
//! filter (opt_in + active) cannot be bypassed by ad-hoc SQL.
//!
//! Lives in its own module — same resilience reason as `payout_methods.rs`.

use crate::error::AppError;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub display_name: String,
    pub public_avatar_url: Option<String>,
    pub current_tier: Option<String>,
    pub paid_cents: i64,
    pub qualified_referrals: i64,
}

/// Parse the scope query param. Default = "month".
pub fn parse_scope(s: Option<&str>) -> &'static str {
    match s {
        Some("lifetime") | Some("all") | Some("all-time") => "lifetime",
        _ => "month",
    }
}

/// Read the top-N entries from the matview. `scope` is "month" or "lifetime".
/// Ranking comes from the requested bucket, ties broken by qualified_referrals.
pub async fn list_public_leaderboard(
    pool: &PgPool,
    scope: &str,
    limit: i64,
) -> Result<Vec<LeaderboardEntry>, AppError> {
    let limit_clamped = limit.clamp(1, 100);
    let order_col = if scope == "lifetime" {
        "lifetime_paid_cents"
    } else {
        "month_paid_cents"
    };
    // Non-macro so we can interpolate the safe whitelisted column name.
    let sql = format!(
        r#"SELECT display_name, public_avatar_url, current_tier,
                  {col} AS paid_cents,
                  qualified_referrals,
                  ROW_NUMBER() OVER (ORDER BY {col} DESC, qualified_referrals DESC) AS rank
             FROM affiliate_leaderboard_public
            ORDER BY {col} DESC, qualified_referrals DESC
            LIMIT $1"#,
        col = order_col
    );
    let rows = sqlx::query(&sql)
        .bind(limit_clamped)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("leaderboard list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|r| LeaderboardEntry {
            rank: r.try_get("rank").unwrap_or(0),
            display_name: r.try_get("display_name").unwrap_or_else(|_| "—".into()),
            public_avatar_url: r.try_get("public_avatar_url").ok(),
            current_tier: r.try_get("current_tier").ok(),
            paid_cents: r.try_get("paid_cents").unwrap_or(0),
            qualified_referrals: r.try_get("qualified_referrals").unwrap_or(0),
        })
        .collect())
}

/// Flip the per-affiliate opt-in flag + optionally update the public
/// display name + avatar URL. Returns the new opt-in state.
pub async fn set_opt_in(
    pool: &PgPool,
    user_id: Uuid,
    opt_in: bool,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<bool, AppError> {
    // Light validation — schema CHECK already enforces HTTPS for avatar.
    if let Some(name) = display_name {
        let trimmed = name.trim();
        if trimmed.len() > 60 {
            return Err(AppError::BadRequest(
                "display name must be ≤ 60 chars".into(),
            ));
        }
    }
    if let Some(url) = avatar_url {
        if !url.is_empty() && !url.starts_with("https://") {
            return Err(AppError::BadRequest(
                "avatar URL must be HTTPS".into(),
            ));
        }
        if url.len() > 512 {
            return Err(AppError::BadRequest(
                "avatar URL must be ≤ 512 chars".into(),
            ));
        }
    }
    let res = sqlx::query(
        r#"UPDATE affiliates
              SET public_leaderboard_opt_in = $2,
                  public_display_name = COALESCE($3, public_display_name),
                  public_avatar_url   = COALESCE($4, public_avatar_url),
                  updated_at = NOW()
            WHERE user_id = $1 AND status = 'active'"#,
    )
    .bind(user_id)
    .bind(opt_in)
    .bind(display_name)
    .bind(avatar_url)
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("opt-in update: {e}")))?;
    if res.rows_affected() == 0 {
        return Err(AppError::Forbidden(
            "Only active affiliates can change leaderboard opt-in".into(),
        ));
    }
    Ok(opt_in)
}

/// Refresh the matview. Uses CONCURRENTLY so reads aren't blocked.
/// Requires the unique index on affiliate_user_id (set up in mig 192).
pub async fn refresh_matview(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY affiliate_leaderboard_public")
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("matview refresh: {e}")))?;
    Ok(())
}

/// Background worker that refreshes the public leaderboard matview every
/// 15 min. Quiet failure mode — a single bad refresh shouldn't crash the
/// worker; the next tick retries.
pub async fn run_leaderboard_refresh_worker(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15 * 60));
    // Skip the first immediate tick — the matview was refreshed on the
    // last process exit (or is empty on a fresh boot). Hot-restart loops
    // shouldn't all hammer the DB simultaneously.
    interval.tick().await;
    loop {
        interval.tick().await;
        if let Err(e) = refresh_matview(&pool).await {
            tracing::warn!(error = %e, "leaderboard worker: refresh failed (will retry in 15m)");
        } else {
            tracing::debug!("leaderboard worker: matview refreshed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scope_normalises_aliases() {
        assert_eq!(parse_scope(None), "month");
        assert_eq!(parse_scope(Some("month")), "month");
        assert_eq!(parse_scope(Some("garbage")), "month");
        assert_eq!(parse_scope(Some("lifetime")), "lifetime");
        assert_eq!(parse_scope(Some("all")), "lifetime");
        assert_eq!(parse_scope(Some("all-time")), "lifetime");
    }
}
