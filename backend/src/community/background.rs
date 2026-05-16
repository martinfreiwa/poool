use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Periodically checks the post creation velocity of assets and flags if > 5 posts in 10 mins.
pub async fn monitor_asset_velocity(community_pool: PgPool, core_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5)); // Every 5 minutes

    loop {
        interval.tick().await;

        if let Err(e) = check_velocity(&community_pool, &core_pool).await {
            tracing::error!("Failed to check asset velocity: {:?}", e);
        }
    }
}

async fn check_velocity(
    c_pool: &PgPool,
    _core_pool: &PgPool,
) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // Check last 10 minutes
    let rows = sqlx::query(
        "SELECT asset_id, count(*) as post_count \
         FROM posts \
         WHERE asset_id IS NOT NULL \
         AND created_at >= NOW() - INTERVAL '10 minutes' \
         GROUP BY asset_id \
         HAVING count(*) >= 5",
    )
    .fetch_all(c_pool)
    .await?;

    for row in rows {
        let asset_id: Uuid = row.try_get("asset_id")?;
        let count: i64 = row.try_get("post_count")?;

        tracing::warn!(
            "Pump & Dump Warning: Asset {} has {} mentions in the last 10 minutes!",
            asset_id,
            count
        );

        // Alert Admins by creating a special admin notification or just logging for now.
        // If an admin_alerts table is added later, we can insert into it.
        tracing::error!(
            "PUMP & DUMP ALERT: Asset {} mentioned {} times in 10 mins",
            asset_id,
            count
        );
    }

    Ok(())
}

/// Core Gamification Engine: Evaluates user stats and assigns badges
pub async fn gamification_worker(community_pool: PgPool, core_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60)); // Every 6 hours

    loop {
        interval.tick().await;
        if let Err(e) = run_badge_evaluations(&community_pool, &core_pool).await {
            tracing::error!("Badge Engine Worker failed: {:?}", e);
        }
    }
}

async fn run_badge_evaluations(
    c_pool: &PgPool,
    core_pool: &PgPool,
) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    tracing::info!("Starting Gamification Badge evaluations...");

    // 1. First Timber / First Investment Badge
    // For anyone who has at least 1 investment
    let rows = sqlx::query("SELECT DISTINCT user_id FROM investments")
        .fetch_all(core_pool)
        .await?;

    for row in rows {
        let user_id: Uuid = row.try_get("user_id")?;
        assign_badge(c_pool, user_id, "first_investment").await?;
    }

    // 2. Whale Badge (> $10k equivalent value total)
    let whales = sqlx::query(
        "SELECT user_id, SUM(current_value_cents) as total 
         FROM investments 
         GROUP BY user_id 
         HAVING SUM(current_value_cents) >= 1000000",
    )
    .fetch_all(core_pool)
    .await?;

    for w in whales {
        let user_id: Uuid = w.try_get("user_id")?;
        assign_badge(c_pool, user_id, "whale").await?;
    }

    // 3. Diversified Badge (invests in > 3 distinct assets)
    let diverse = sqlx::query(
        "SELECT user_id, COUNT(DISTINCT asset_id) as c
         FROM investments
         GROUP BY user_id
         HAVING COUNT(DISTINCT asset_id) >= 3",
    )
    .fetch_all(core_pool)
    .await?;

    for d in diverse {
        let user_id: Uuid = d.try_get("user_id")?;
        assign_badge(c_pool, user_id, "diversified").await?;
    }

    // 4. Dividend Collector Badge (>= 10 payouts)
    let collectors = sqlx::query(
        "SELECT user_id FROM dividend_payouts WHERE status = 'paid' GROUP BY user_id HAVING COUNT(*) >= 10"
    ).fetch_all(core_pool).await?;

    for c in collectors {
        let user_id: Uuid = c.try_get("user_id")?;
        assign_badge(c_pool, user_id, "dividend_king").await?;
    }

    tracing::info!("Gamification Badge evaluations complete.");
    Ok(())
}

async fn assign_badge(
    c_pool: &PgPool,
    user_id: Uuid,
    badge_code: &str,
) -> Result<(), crate::error::AppError> {
    // 1. Find badge UUID
    let badge_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM badges WHERE code = $1")
        .bind(badge_code)
        .fetch_optional(c_pool)
        .await?;

    if let Some(bid) = badge_id {
        // 2. Insert if not exists
        sqlx::query(
            "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user_id)
        .bind(bid)
        .execute(c_pool)
        .await?;

        // 3. Award XP for badge (idempotent via daily cap + reason check)
        let _ =
            crate::community::xp::award_xp(c_pool, user_id, "badge_earned", Some(badge_code), None)
                .await;
    }

    Ok(())
}

/// XP Aggregation Worker: Sync XP totals and update circle rankings.
/// Runs every 5 minutes.
pub async fn xp_aggregation_worker(community_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(5 * 60));

    loop {
        interval.tick().await;
        if let Err(e) = crate::community::xp::aggregate_xp(&community_pool).await {
            tracing::error!("XP aggregation worker failed: {:?}", e);
        }
    }
}

/// Circle Invite Expiry Worker: Mark expired invites.
/// Runs every hour.
pub async fn circle_invite_expiry_worker(community_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60));

    loop {
        interval.tick().await;
        if let Err(e) = expire_circle_invites(&community_pool).await {
            tracing::error!("Circle invite expiry worker failed: {:?}", e);
        }
    }
}

async fn expire_circle_invites(pool: &PgPool) -> Result<(), crate::error::AppError> {
    let result = sqlx::query(
        "UPDATE circle_invites SET status = 'expired' WHERE status = 'pending' AND expires_at < NOW()"
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::info!("Expired {} circle invites", result.rows_affected());
    }
    Ok(())
}

/// Circle Retry Worker (M4-BE.7): Retry auto-joins for referred users not yet in circles.
/// Runs every 30 minutes. Joins the referred user to their referrer's circle if possible.
pub async fn circle_retry_worker(community_pool: PgPool, core_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));

    loop {
        interval.tick().await;
        if let Err(e) = retry_circle_auto_joins(&community_pool, &core_pool).await {
            tracing::error!("Circle retry worker failed: {:?}", e);
        }
    }
}

async fn retry_circle_auto_joins(
    c_pool: &PgPool,
    core_pool: &PgPool,
) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // 1. Get all referred users from core DB
    let referrals = sqlx::query(
        "SELECT referred_id, referrer_id FROM referral_tracking WHERE status IN ('pending', 'completed') LIMIT 100"
    )
    .fetch_all(core_pool)
    .await?;

    if referrals.is_empty() {
        return Ok(());
    }

    let mut joined = 0;
    for row in referrals {
        let referred_id: Uuid = row.try_get("referred_id")?;
        let referrer_id: Uuid = row.try_get("referrer_id")?;

        // 2. Skip if user is already in the REFERRER's circle (multi-circle
        // era — being in some other circle doesn't block joining this one).
        // The referrer's primary circle id lives on community_profiles.
        let referrer_circle: Option<Uuid> =
            sqlx::query_scalar("SELECT circle_id FROM community_profiles WHERE user_id = $1")
                .bind(referrer_id)
                .fetch_optional(c_pool)
                .await?;
        let Some(target_circle) = referrer_circle else {
            continue; // referrer has no primary circle, nothing to auto-join into
        };
        let already_in_target: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM circle_members WHERE user_id = $1 AND circle_id = $2)",
        )
        .bind(referred_id)
        .bind(target_circle)
        .fetch_one(c_pool)
        .await?;
        if already_in_target {
            continue; // already in referrer's circle, skip
        }

        // 3. Ensure community profile exists
        sqlx::query(
            "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(referred_id)
        .execute(c_pool)
        .await?;

        // 4. Try auto-join
        match crate::community::circles::auto_join_referrer_circle(c_pool, referred_id, referrer_id)
            .await
        {
            Ok(()) => joined += 1,
            Err(e) => tracing::debug!(user_id = %referred_id, "Circle retry skipped: {}", e),
        }
    }

    if joined > 0 {
        tracing::info!(
            "Circle retry worker: auto-joined {} users to referrer circles",
            joined
        );
    }

    Ok(())
}

// ─── GDPR Worker (M7-BE.6) ───────────────────────────────────────────────────

/// GDPR Deletion & Anonymization Worker
/// Scans the core DB for users with status = 'deleted' recently,
/// and ensures their community footprints (profile, bio) are anonymized.
pub async fn gdpr_anonymization_worker(community_pool: PgPool, core_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 60)); // Once an hour

    loop {
        interval.tick().await;
        if let Err(e) = run_gdpr_anonymization(&community_pool, &core_pool).await {
            tracing::error!("GDPR worker failed: {:?}", e);
        }
    }
}

async fn run_gdpr_anonymization(
    c_pool: &PgPool,
    core_pool: &PgPool,
) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // 1. Fetch recently deleted users from Core DB
    let deleted_users = sqlx::query(
        "SELECT id FROM users WHERE status = 'deleted' AND updated_at >= NOW() - INTERVAL '1 day'",
    )
    .fetch_all(core_pool)
    .await?;

    if deleted_users.is_empty() {
        return Ok(());
    }

    let mut anonymized_count = 0;

    for row in deleted_users {
        let user_id: Uuid = row.try_get("id")?;

        // 2. Anonymize their community profile
        let result = sqlx::query(
            r#"UPDATE community_profiles 
               SET display_name = 'Deleted User', 
                   bio = NULL, 
                   avatar_emoji = NULL, 
                   is_community_banned = true 
               WHERE user_id = $1 AND display_name != 'Deleted User'"#,
        )
        .bind(user_id)
        .execute(c_pool)
        .await?;

        // 3. Delete any active ban appeals
        sqlx::query("DELETE FROM ban_appeals WHERE user_id = $1")
            .bind(user_id)
            .execute(c_pool)
            .await?;

        if result.rows_affected() > 0 {
            // Remove them from their circle if they are in one
            sqlx::query("DELETE FROM circle_members WHERE user_id = $1")
                .bind(user_id)
                .execute(c_pool)
                .await?;

            anonymized_count += 1;
        }
    }

    if anonymized_count > 0 {
        tracing::info!(
            "GDPR Worker: Anonymized {} community profiles.",
            anonymized_count
        );
    }

    Ok(())
}

// ─── Weekly Digest Worker (M5-BE.6) ────────────────────────────────────────────

/// Weekly Digest Worker
/// Runs daily, finds users who haven't logged in for 7 days, and sends a weekly digest email.
pub async fn weekly_digest_worker(community_pool: PgPool, core_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60)); // Once a day

    loop {
        interval.tick().await;
        if let Err(e) = run_weekly_digest(&community_pool, &core_pool).await {
            tracing::error!("Weekly digest worker failed: {:?}", e);
            // Ignore error
        }
    }
}

async fn run_weekly_digest(
    _c_pool: &PgPool,
    core_pool: &PgPool,
) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // Fetch users inactive for > 7 days who have notifications enabled
    let inactive_users = sqlx::query(
        r#"
        SELECT u.id, u.email, p.first_name 
        FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        LEFT JOIN user_settings s ON s.user_id = u.id
        WHERE u.status = 'active'
          AND COALESCE(s.email_notifications, true) = true
          AND u.id NOT IN (
              SELECT user_id FROM user_sessions WHERE created_at > NOW() - INTERVAL '7 days'
          )
          LIMIT 100
        "#,
    )
    .fetch_all(core_pool)
    .await?;

    if inactive_users.is_empty() {
        return Ok(());
    }

    for row in inactive_users {
        let _user_id: Uuid = row.try_get("id")?;
        let email: String = row.try_get("email")?;
        let _first_name: Option<String> = row.try_get("first_name").unwrap_or(None);

        tracing::info!("Weekly Digest Worker: Would send digest email to {}", email);
        // Note: Actual email sending goes here using the email module
        // crate::email::queue_email(...)
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Circle trending refresh worker (2026-05-16) — keeps
// `circles.recent_post_count` accurate so the Discover > Trending section
// can sort by activity.
//
// Previously the column stayed at its default 0, which made the trending
// section sort identically to "by member_count + created_at" — i.e. not
// actually trending.
// ═══════════════════════════════════════════════════════════════════════

/// Background worker: every 5 minutes, recompute `recent_post_count` for
/// every circle as the count of posts in the last 7 days.
///
/// This is a single UPDATE … FROM that runs in milliseconds even at 100k
/// posts (no per-circle loop, no app-side join). Skips when the community
/// pool is unavailable (handled by the worker setup in lib.rs).
pub async fn circle_trending_refresh_worker(c_pool: PgPool) {
    let interval_secs: u64 = std::env::var("POOOL_CIRCLE_TRENDING_REFRESH_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n.max(60))
        .unwrap_or(5 * 60);
    tracing::info!(
        interval_secs = interval_secs,
        "Circle trending refresh worker starting (override via POOOL_CIRCLE_TRENDING_REFRESH_SECS)"
    );
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    // Small startup delay so a fresh boot doesn't slam the DB before
    // initial migrations / health checks are done.
    tokio::time::sleep(Duration::from_secs(20)).await;
    let mut consecutive_failures: u32 = 0;
    loop {
        interval.tick().await;
        let started = std::time::Instant::now();
        let res = sqlx::query(
            r#"
            UPDATE circles c SET
                recent_post_count = COALESCE(sub.cnt, 0),
                recent_post_count_updated_at = NOW()
            FROM (
                SELECT circle_id, COUNT(*)::INT AS cnt
                FROM posts
                WHERE created_at >= NOW() - INTERVAL '7 days'
                  AND circle_id IS NOT NULL
                  AND is_hidden = false
                GROUP BY circle_id
            ) sub
            WHERE c.id = sub.circle_id
               OR (c.id NOT IN (
                       SELECT DISTINCT circle_id FROM posts
                       WHERE created_at >= NOW() - INTERVAL '7 days'
                         AND circle_id IS NOT NULL
                         AND is_hidden = false
                   )
                   AND c.recent_post_count <> 0)
            "#,
        )
        .execute(&c_pool)
        .await;
        match res {
            Ok(r) => {
                consecutive_failures = 0;
                tracing::info!(
                    metric_name = "circle_trending_refresh_duration_ms",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    rows_affected = r.rows_affected(),
                    "Circle trending refresh OK"
                );
            }
            Err(e) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                tracing::error!(
                    metric_name = "circle_trending_refresh_failure",
                    consecutive_failures = consecutive_failures,
                    error = %e,
                    "Circle trending refresh failed"
                );
                if consecutive_failures >= 3 {
                    tracing::warn!(
                        metric_name = "circle_trending_refresh_consecutive_failures",
                        value = consecutive_failures,
                        "Circle trending refresh has failed {} times in a row",
                        consecutive_failures
                    );
                }
            }
        }
    }
}
