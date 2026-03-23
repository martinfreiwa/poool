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

async fn check_velocity(c_pool: &PgPool, core_pool: &PgPool) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // Check last 10 minutes
    let rows = sqlx::query(
        "SELECT asset_id, count(*) as post_count \
         FROM posts \
         WHERE asset_id IS NOT NULL \
         AND created_at >= NOW() - INTERVAL '10 minutes' \
         GROUP BY asset_id \
         HAVING count(*) >= 5"
    )
    .fetch_all(c_pool)
    .await?;

    for row in rows {
        let asset_id: Uuid = row.try_get("asset_id")?;
        let count: i64 = row.try_get("post_count")?;

        tracing::warn!("Pump & Dump Warning: Asset {} has {} mentions in the last 10 minutes!", asset_id, count);

        // Alert Admins by creating a special admin notification or just logging for now.
        // If an admin_alerts table is added later, we can insert into it.
        tracing::error!("PUMP & DUMP ALERT: Asset {} mentioned {} times in 10 mins", asset_id, count);
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

async fn run_badge_evaluations(c_pool: &PgPool, core_pool: &PgPool) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    tracing::info!("Starting Gamification Badge evaluations...");

    // 1. First Timber / First Investment Badge
    // For anyone who has at least 1 investment
    let rows = sqlx::query("SELECT DISTINCT user_id FROM investments")
        .fetch_all(core_pool).await?;

    for row in rows {
        let user_id: Uuid = row.try_get("user_id")?;
        assign_badge(c_pool, user_id, "first_investment").await?;
    }

    // 2. Whale Badge (> $10k equivalent value total)
    let whales = sqlx::query(
        "SELECT user_id, SUM(current_value_cents) as total 
         FROM investments 
         GROUP BY user_id 
         HAVING SUM(current_value_cents) >= 1000000"
    ).fetch_all(core_pool).await?;

    for w in whales {
        let user_id: Uuid = w.try_get("user_id")?;
        assign_badge(c_pool, user_id, "whale").await?;
    }

    // 3. Diversified Badge (invests in > 3 distinct assets)
    let diverse = sqlx::query(
        "SELECT user_id, COUNT(DISTINCT asset_id) as c
         FROM investments
         GROUP BY user_id
         HAVING COUNT(DISTINCT asset_id) >= 3"
    ).fetch_all(core_pool).await?;

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

async fn assign_badge(c_pool: &PgPool, user_id: Uuid, badge_code: &str) -> Result<(), crate::error::AppError> {
    // 1. Find badge UUID
    let badge_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM badges WHERE code = $1")
        .bind(badge_code)
        .fetch_optional(c_pool)
        .await?;

    if let Some(bid) = badge_id {
        // 2. Insert if not exists
        sqlx::query("INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(user_id)
            .bind(bid)
            .execute(c_pool)
            .await?;

        // 3. Award XP for badge (idempotent via daily cap + reason check)
        let _ = crate::community::xp::award_xp(c_pool, user_id, "badge_earned", Some(badge_code), None).await;
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

async fn retry_circle_auto_joins(c_pool: &PgPool, core_pool: &PgPool) -> Result<(), crate::error::AppError> {
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

        // 2. Check if already in a circle (community DB)
        let already_in_circle: Option<Uuid> = sqlx::query_scalar(
            "SELECT circle_id FROM circle_members WHERE user_id = $1 LIMIT 1"
        )
        .bind(referred_id)
        .fetch_optional(c_pool)
        .await?;

        if already_in_circle.is_some() {
            continue; // already in a circle, skip
        }

        // 3. Ensure community profile exists
        sqlx::query(
            "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING"
        )
        .bind(referred_id)
        .execute(c_pool)
        .await?;

        // 4. Try auto-join
        match crate::community::circles::auto_join_referrer_circle(c_pool, referred_id, referrer_id).await {
            Ok(()) => joined += 1,
            Err(e) => tracing::debug!(user_id = %referred_id, "Circle retry skipped: {}", e),
        }
    }

    if joined > 0 {
        tracing::info!("Circle retry worker: auto-joined {} users to referrer circles", joined);
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

async fn run_gdpr_anonymization(c_pool: &PgPool, core_pool: &PgPool) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // 1. Fetch recently deleted users from Core DB
    let deleted_users = sqlx::query(
        "SELECT id FROM users WHERE status = 'deleted' AND updated_at >= NOW() - INTERVAL '1 day'"
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
               WHERE user_id = $1 AND display_name != 'Deleted User'"#
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
        tracing::info!("GDPR Worker: Anonymized {} community profiles.", anonymized_count);
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

async fn run_weekly_digest(_c_pool: &PgPool, core_pool: &PgPool) -> Result<(), crate::error::AppError> {
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
        "#
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
