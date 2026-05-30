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

/// Summary emitted by one Circle ops snapshot run.
#[derive(Debug, Clone, Copy, Default)]
pub struct CircleOpsSnapshotSummary {
    /// Number of Circle daily analytics rows inserted or updated.
    pub snapshots_upserted: u64,
    /// Number of report-backlog alerts inserted, updated, or resolved.
    pub report_backlog_alerts_touched: u64,
    /// Number of moderation-SLA alerts inserted, updated, or resolved.
    pub moderation_sla_alerts_touched: u64,
    /// Number of critical alert notification intents queued for fan-out.
    pub auto_critical_notifications_enqueued: u64,
}

/// Summary emitted by one Circle ops alert notification fan-out pass.
#[derive(Debug, Clone, Copy, Default)]
pub struct CircleOpsAlertFanoutSummary {
    pub claimed: u64,
    pub enqueued: u64,
    pub skipped: u64,
    pub failed: u64,
    pub slack_sent: u64,
    pub pagerduty_sent: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CircleOpsAlertDeliveryMonitorSummary {
    pub checked: u64,
    pub sent: u64,
    pub skipped: u64,
    pub pending: u64,
    pub unhealthy: u64,
    pub missing: u64,
    pub alerts_touched: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CircleResourceRetentionSummary {
    pub resources_soft_deleted: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CircleResourceObjectCleanupSummary {
    pub objects_considered: u64,
    pub objects_deleted_or_absent: u64,
    pub objects_failed: u64,
    pub resource_rows_marked: u64,
    pub version_rows_marked: u64,
}

/// Background worker that materializes bounded Circle analytics snapshots and
/// operational alerts for the Manage/Ops surface.
pub async fn circle_ops_snapshot_worker(c_pool: PgPool) {
    if std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()) == "development" {
        let schema_ready = sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.circle_daily_analytics') IS NOT NULL",
        )
        .fetch_one(&c_pool)
        .await
        .unwrap_or(false);
        if !schema_ready {
            tracing::warn!(
                "Circle ops snapshot worker disabled in development because circle_daily_analytics is not present"
            );
            return;
        }
    }

    let interval_secs: u64 = std::env::var("POOOL_CIRCLE_OPS_SNAPSHOT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n.max(300))
        .unwrap_or(24 * 60 * 60);
    tracing::info!(
        interval_secs = interval_secs,
        "Circle ops snapshot worker starting (override via POOOL_CIRCLE_OPS_SNAPSHOT_SECS)"
    );

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::time::sleep(Duration::from_secs(45)).await;

    let mut consecutive_failures: u32 = 0;
    loop {
        interval.tick().await;
        let started = std::time::Instant::now();
        match run_circle_ops_snapshot_once(&c_pool).await {
            Ok(summary) => {
                consecutive_failures = 0;
                if let Err(resolve_err) =
                    resolve_circle_failed_worker_alert(&c_pool, "circle_ops_snapshot_worker").await
                {
                    tracing::warn!(
                        error = %resolve_err,
                        "Circle ops snapshot worker could not resolve failed_worker alert"
                    );
                }
                tracing::info!(
                    metric_name = "circle_ops_snapshot_duration_ms",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    snapshots_upserted = summary.snapshots_upserted,
                    report_backlog_alerts_touched = summary.report_backlog_alerts_touched,
                    moderation_sla_alerts_touched = summary.moderation_sla_alerts_touched,
                    auto_critical_notifications_enqueued =
                        summary.auto_critical_notifications_enqueued,
                    "Circle ops snapshot OK"
                );
            }
            Err(e) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                tracing::error!(
                    metric_name = "circle_ops_snapshot_failure",
                    consecutive_failures = consecutive_failures,
                    error = %e.detail(),
                    "Circle ops snapshot failed"
                );
                if consecutive_failures >= 3 {
                    if let Err(alert_err) = upsert_circle_failed_worker_alert(
                        &c_pool,
                        "circle_ops_snapshot_worker",
                        consecutive_failures,
                        &e.to_string(),
                    )
                    .await
                    {
                        tracing::warn!(
                            error = %alert_err,
                            "Circle ops snapshot worker could not write failed_worker alert"
                        );
                    }
                }
            }
        }
    }
}

/// Run one idempotent Circle ops snapshot pass.
pub async fn run_circle_ops_snapshot_once(
    c_pool: &PgPool,
) -> Result<CircleOpsSnapshotSummary, crate::error::AppError> {
    let snapshots_upserted = upsert_circle_daily_analytics(c_pool).await?;
    let report_backlog_alerts_touched = refresh_circle_report_backlog_alerts(c_pool).await?;
    let moderation_sla_alerts_touched = refresh_circle_moderation_sla_alerts(c_pool).await?;
    let auto_critical_notifications_enqueued =
        enqueue_auto_critical_circle_ops_alert_notifications_once(c_pool).await?;

    Ok(CircleOpsSnapshotSummary {
        snapshots_upserted,
        report_backlog_alerts_touched,
        moderation_sla_alerts_touched,
        auto_critical_notifications_enqueued,
    })
}

/// Background worker that applies Circle Resource retention policy. This is a
/// conservative soft-delete pass; actual object deletion remains a separate,
/// auditable storage-cleanup step.
pub async fn circle_resource_retention_worker(c_pool: PgPool) {
    let interval_secs: u64 = std::env::var("POOOL_CIRCLE_RESOURCE_RETENTION_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n.max(900))
        .unwrap_or(24 * 60 * 60);
    tracing::info!(
        interval_secs = interval_secs,
        "Circle resource retention worker starting (override via POOOL_CIRCLE_RESOURCE_RETENTION_SECS)"
    );

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::time::sleep(Duration::from_secs(75)).await;

    loop {
        interval.tick().await;
        let started = std::time::Instant::now();
        match run_circle_resource_retention_once(&c_pool).await {
            Ok(summary) => {
                tracing::info!(
                    metric_name = "circle_resource_retention",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    resources_soft_deleted = summary.resources_soft_deleted,
                    "Circle resource retention OK"
                );
            }
            Err(e) => {
                tracing::error!(
                    metric_name = "circle_resource_retention_failure",
                    error = %e,
                    "Circle resource retention failed"
                );
            }
        }
    }
}

pub async fn run_circle_resource_retention_once(
    c_pool: &PgPool,
) -> Result<CircleResourceRetentionSummary, crate::error::AppError> {
    let resources_soft_deleted = sqlx::query(
        r#"
        WITH due_resources AS (
            UPDATE circle_resources resource
               SET upload_status = 'deleted',
                   is_active = FALSE,
                   deleted_at = NOW(),
                   deleted_by = NULL,
                   deletion_reason = COALESCE(resource.deletion_reason, 'retention_policy_due'),
                   document_lifecycle_notes = COALESCE(
                       resource.document_lifecycle_notes,
                       'Auto soft-deleted by Circle Resource retention worker'
                   ),
                   updated_at = NOW()
             WHERE resource.retention_policy = 'delete_after_expiry'
               AND resource.retention_until IS NOT NULL
               AND resource.retention_until <= NOW()
               AND resource.legal_hold = FALSE
               AND resource.deleted_at IS NULL
             RETURNING
               resource.id,
               resource.circle_id,
               resource.title,
               resource.access_scope,
               resource.retention_until,
               resource.storage_object_path IS NOT NULL AS has_private_file
        )
        INSERT INTO community_audit_logs (
            actor_user_id,
            action,
            entity_type,
            entity_id,
            target_user_id,
            details
        )
        SELECT
            NULL,
            'circle.resource.retention_soft_delete',
            'circle_resource',
            due_resources.id,
            NULL,
            JSONB_BUILD_OBJECT(
                'circle_id', due_resources.circle_id,
                'title', due_resources.title,
                'access_scope', due_resources.access_scope,
                'retention_until', due_resources.retention_until,
                'has_private_file', due_resources.has_private_file,
                'deletion_reason', 'retention_policy_due'
            )
        FROM due_resources
        "#,
    )
    .execute(c_pool)
    .await?
    .rows_affected();

    Ok(CircleResourceRetentionSummary {
        resources_soft_deleted,
    })
}

/// Background worker that physically removes private Circle Resource objects
/// after the retention/lifecycle layer has already soft-deleted the DB rows.
/// It only runs when a GCS bucket is configured.
pub async fn circle_resource_object_cleanup_worker(c_pool: PgPool, default_bucket: String) {
    let interval_secs: u64 = std::env::var("POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n.max(900))
        .unwrap_or(24 * 60 * 60);
    tracing::info!(
        interval_secs = interval_secs,
        default_bucket = %default_bucket,
        "Circle resource object cleanup worker starting (override via POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_SECS)"
    );

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::time::sleep(Duration::from_secs(90)).await;

    loop {
        interval.tick().await;
        let started = std::time::Instant::now();
        match run_circle_resource_object_cleanup_once(&c_pool, &default_bucket).await {
            Ok(summary) => {
                tracing::info!(
                    metric_name = "circle_resource_object_cleanup",
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    objects_considered = summary.objects_considered,
                    objects_deleted_or_absent = summary.objects_deleted_or_absent,
                    objects_failed = summary.objects_failed,
                    resource_rows_marked = summary.resource_rows_marked,
                    version_rows_marked = summary.version_rows_marked,
                    "Circle resource object cleanup OK"
                );
            }
            Err(e) => {
                tracing::error!(
                    metric_name = "circle_resource_object_cleanup_failure",
                    error = %e,
                    "Circle resource object cleanup failed"
                );
            }
        }
    }
}

pub async fn run_circle_resource_object_cleanup_once(
    c_pool: &PgPool,
    default_bucket: &str,
) -> Result<CircleResourceObjectCleanupSummary, crate::error::AppError> {
    use sqlx::Row;

    let grace_days: i32 = std::env::var("POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_GRACE_DAYS")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .map(|n| n.max(0))
        .unwrap_or(7);
    let limit: i64 = std::env::var("POOOL_CIRCLE_RESOURCE_OBJECT_CLEANUP_LIMIT")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|n| n.clamp(1, 250))
        .unwrap_or(50);

    let target_rows = sqlx::query(
        r#"
        WITH due_paths AS (
            SELECT resource.storage_object_path
              FROM circle_resources resource
             WHERE resource.storage_object_path IS NOT NULL
               AND resource.deleted_at IS NOT NULL
               AND resource.deleted_at <= NOW() - ($1::INT * INTERVAL '1 day')
               AND resource.legal_hold = FALSE
               AND resource.storage_deleted_at IS NULL
               AND COALESCE(resource.storage_delete_next_attempt_at, NOW()) <= NOW()
            UNION
            SELECT version.storage_object_path
              FROM circle_resource_versions version
              JOIN circle_resources resource ON resource.id = version.resource_id
             WHERE version.storage_object_path IS NOT NULL
               AND resource.deleted_at IS NOT NULL
               AND resource.deleted_at <= NOW() - ($1::INT * INTERVAL '1 day')
               AND resource.legal_hold = FALSE
               AND version.storage_deleted_at IS NULL
               AND COALESCE(version.storage_delete_next_attempt_at, NOW()) <= NOW()
        )
        SELECT storage_object_path
          FROM due_paths
         WHERE storage_object_path IS NOT NULL
         ORDER BY storage_object_path
         LIMIT $2
        "#,
    )
    .bind(grace_days)
    .bind(limit)
    .fetch_all(c_pool)
    .await?;

    let mut summary = CircleResourceObjectCleanupSummary {
        objects_considered: target_rows.len() as u64,
        ..CircleResourceObjectCleanupSummary::default()
    };

    for row in target_rows {
        let storage_object_path = row.try_get::<String, _>("storage_object_path")?;
        match delete_circle_resource_storage_object(default_bucket, &storage_object_path).await {
            Ok(_deleted_or_absent) => {
                summary.objects_deleted_or_absent =
                    summary.objects_deleted_or_absent.saturating_add(1);
                let (resource_rows, version_rows) =
                    mark_circle_resource_object_deleted(c_pool, &storage_object_path).await?;
                summary.resource_rows_marked =
                    summary.resource_rows_marked.saturating_add(resource_rows);
                summary.version_rows_marked =
                    summary.version_rows_marked.saturating_add(version_rows);
            }
            Err(e) => {
                summary.objects_failed = summary.objects_failed.saturating_add(1);
                mark_circle_resource_object_delete_failed(c_pool, &storage_object_path, &e).await?;
                crate::metrics::record_storage_gcs_error(
                    "resource.delete",
                    kind_for_circle_resource_gcs_error(&e),
                );
            }
        }
    }

    Ok(summary)
}

async fn delete_circle_resource_storage_object(
    default_bucket: &str,
    stored_path: &str,
) -> Result<bool, crate::error::AppError> {
    let (bucket, object_path) =
        crate::storage::reconciler::extract_bucket_and_path(stored_path, default_bucket)
            .ok_or_else(|| {
                crate::error::AppError::Internal(format!(
                    "unparseable Circle resource storage path: {}",
                    stored_path
                ))
            })?;

    match crate::storage::service::delete_object(&bucket, &object_path).await {
        Ok(()) => Ok(true),
        Err(e) => {
            let error_text = e.to_string();
            if error_text.contains("404") || error_text.to_ascii_lowercase().contains("not found") {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

async fn mark_circle_resource_object_deleted(
    c_pool: &PgPool,
    storage_object_path: &str,
) -> Result<(u64, u64), crate::error::AppError> {
    let mut tx = c_pool.begin().await?;

    let resource_rows = sqlx::query(
        r#"
        WITH updated AS (
            UPDATE circle_resources
               SET storage_deleted_at = NOW(),
                   storage_delete_last_error = NULL,
                   storage_delete_next_attempt_at = NULL,
                   updated_at = NOW()
             WHERE storage_object_path = $1
               AND deleted_at IS NOT NULL
               AND legal_hold = FALSE
               AND storage_deleted_at IS NULL
             RETURNING id, circle_id, title, storage_object_path
        )
        INSERT INTO community_audit_logs (
            actor_user_id,
            action,
            entity_type,
            entity_id,
            target_user_id,
            details
        )
        SELECT
            NULL,
            'circle.resource.object_delete',
            'circle_resource',
            updated.id,
            NULL,
            JSONB_BUILD_OBJECT(
                'circle_id', updated.circle_id,
                'title', updated.title,
                'storage_object_path', updated.storage_object_path
            )
        FROM updated
        "#,
    )
    .bind(storage_object_path)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let version_rows = sqlx::query(
        r#"
        WITH updated AS (
            UPDATE circle_resource_versions version
               SET storage_deleted_at = NOW(),
                   storage_delete_last_error = NULL,
                   storage_delete_next_attempt_at = NULL
              FROM circle_resources resource
             WHERE version.resource_id = resource.id
               AND version.storage_object_path = $1
               AND resource.deleted_at IS NOT NULL
               AND resource.legal_hold = FALSE
               AND version.storage_deleted_at IS NULL
             RETURNING version.id, version.resource_id, version.circle_id, version.storage_object_path
        )
        INSERT INTO community_audit_logs (
            actor_user_id,
            action,
            entity_type,
            entity_id,
            target_user_id,
            details
        )
        SELECT
            NULL,
            'circle.resource.version.object_delete',
            'circle_resource_version',
            updated.id,
            NULL,
            JSONB_BUILD_OBJECT(
                'resource_id', updated.resource_id,
                'circle_id', updated.circle_id,
                'storage_object_path', updated.storage_object_path
            )
        FROM updated
        "#,
    )
    .bind(storage_object_path)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    tx.commit().await?;
    Ok((resource_rows, version_rows))
}

async fn mark_circle_resource_object_delete_failed(
    c_pool: &PgPool,
    storage_object_path: &str,
    error: &crate::error::AppError,
) -> Result<(), crate::error::AppError> {
    let error_text = bounded_circle_resource_delete_error(error);
    let mut tx = c_pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE circle_resources
           SET storage_delete_attempts = storage_delete_attempts + 1,
               storage_delete_last_error = $2,
               storage_delete_next_attempt_at =
                   NOW() + (LEAST(storage_delete_attempts + 1, 24)::INT * INTERVAL '1 hour'),
               updated_at = NOW()
         WHERE storage_object_path = $1
           AND deleted_at IS NOT NULL
           AND legal_hold = FALSE
           AND storage_deleted_at IS NULL
        "#,
    )
    .bind(storage_object_path)
    .bind(&error_text)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE circle_resource_versions version
           SET storage_delete_attempts = storage_delete_attempts + 1,
               storage_delete_last_error = $2,
               storage_delete_next_attempt_at =
                   NOW() + (LEAST(storage_delete_attempts + 1, 24)::INT * INTERVAL '1 hour')
          FROM circle_resources resource
         WHERE version.resource_id = resource.id
           AND version.storage_object_path = $1
           AND resource.deleted_at IS NOT NULL
           AND resource.legal_hold = FALSE
           AND version.storage_deleted_at IS NULL
        "#,
    )
    .bind(storage_object_path)
    .bind(&error_text)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

fn bounded_circle_resource_delete_error(error: &crate::error::AppError) -> String {
    let mut value = error.to_string();
    if value.chars().count() > 500 {
        value = value.chars().take(500).collect();
    }
    value
}

fn kind_for_circle_resource_gcs_error(error: &crate::error::AppError) -> &'static str {
    let error_text = error.to_string().to_ascii_lowercase();
    if error_text.contains("auth") {
        "auth"
    } else if error_text.contains("404") || error_text.contains("not found") {
        "not_found"
    } else if error_text.contains("timeout") {
        "timeout"
    } else {
        "other"
    }
}

/// Queue a Circle ops alert notification in the community-side outbox.
/// A separate worker bridges this row into the core transactional email queue.
pub async fn enqueue_circle_ops_alert_notification_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    alert_id: Uuid,
    trigger_action: &str,
    target_user_id: Option<Uuid>,
    payload: serde_json::Value,
) -> Result<(), crate::error::AppError> {
    let recipient_role = if target_user_id.is_some() {
        "assigned_operator"
    } else {
        "platform_admin_fallback"
    };

    sqlx::query(
        r#"
        INSERT INTO circle_ops_alert_notifications (
            alert_id,
            trigger_action,
            target_user_id,
            recipient_role,
            payload
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (
            alert_id,
            channel,
            trigger_action,
            (COALESCE(target_user_id, '00000000-0000-0000-0000-000000000000'::UUID))
        )
        WHERE status = ANY (ARRAY['queued'::TEXT, 'sending'::TEXT])
        DO UPDATE SET
            payload = EXCLUDED.payload,
            recipient_role = EXCLUDED.recipient_role,
            status = 'queued',
            attempts = 0,
            next_attempt_at = NOW(),
            last_error = NULL,
            updated_at = NOW()
        "#,
    )
    .bind(alert_id)
    .bind(trigger_action)
    .bind(target_user_id)
    .bind(recipient_role)
    .bind(payload)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn upsert_circle_daily_analytics(c_pool: &PgPool) -> Result<u64, crate::error::AppError> {
    let result = sqlx::query(
        r#"
        WITH target AS (
            SELECT (CURRENT_DATE - INTERVAL '1 day')::DATE AS snapshot_date
        )
        INSERT INTO circle_daily_analytics (
            circle_id,
            snapshot_date,
            member_count,
            active_members,
            posts_count,
            comments_count,
            qna_answer_rate_bps,
            reported_content_count,
            top_tags
        )
        SELECT
            c.id,
            target.snapshot_date,
            COALESCE(m.member_count, 0)::INT,
            COALESCE(a.active_members, 0)::INT,
            COALESCE(p.posts_count, 0)::INT,
            COALESCE(cm.comments_count, 0)::INT,
            COALESCE(q.qna_answer_rate_bps, 0)::INT,
            COALESCE(r.reported_content_count, 0)::INT,
            COALESCE(t.top_tags, '[]'::JSONB)
        FROM circles c
        CROSS JOIN target
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::INT AS member_count
            FROM circle_members member_row
            WHERE member_row.circle_id = c.id
        ) m ON TRUE
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::INT AS posts_count
            FROM posts post_row
            WHERE post_row.circle_id = c.id
              AND post_row.is_hidden = FALSE
              AND post_row.created_at >= target.snapshot_date
              AND post_row.created_at < target.snapshot_date + INTERVAL '1 day'
        ) p ON TRUE
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::INT AS comments_count
            FROM comments comment_row
            JOIN posts commented_post ON commented_post.id = comment_row.post_id
            WHERE commented_post.circle_id = c.id
              AND comment_row.is_hidden = FALSE
              AND comment_row.created_at >= target.snapshot_date
              AND comment_row.created_at < target.snapshot_date + INTERVAL '1 day'
        ) cm ON TRUE
        LEFT JOIN LATERAL (
            SELECT COUNT(DISTINCT actor_id)::INT AS active_members
            FROM (
                SELECT post_actor.user_id AS actor_id
                FROM posts post_actor
                WHERE post_actor.circle_id = c.id
                  AND post_actor.created_at >= target.snapshot_date
                  AND post_actor.created_at < target.snapshot_date + INTERVAL '1 day'
                UNION
                SELECT comment_actor.user_id AS actor_id
                FROM comments comment_actor
                JOIN posts comment_post ON comment_post.id = comment_actor.post_id
                WHERE comment_post.circle_id = c.id
                  AND comment_actor.created_at >= target.snapshot_date
                  AND comment_actor.created_at < target.snapshot_date + INTERVAL '1 day'
            ) activity
        ) a ON TRUE
        LEFT JOIN LATERAL (
            SELECT CASE
                WHEN COUNT(*) = 0 THEN 0
                ELSE ROUND(
                    COUNT(*) FILTER (WHERE qa_status IN ('answered', 'official_answer'))::NUMERIC
                    * 10000
                    / COUNT(*)::NUMERIC
                )::INT
            END AS qna_answer_rate_bps
            FROM posts question_post
            WHERE question_post.circle_id = c.id
              AND question_post.post_type IN ('question', 'due_diligence')
              AND question_post.is_hidden = FALSE
              AND question_post.created_at >= target.snapshot_date
              AND question_post.created_at < target.snapshot_date + INTERVAL '1 day'
        ) q ON TRUE
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::INT AS reported_content_count
            FROM content_reports report_row
            JOIN posts reported_post ON reported_post.id = report_row.post_id
            WHERE reported_post.circle_id = c.id
              AND report_row.created_at >= target.snapshot_date
              AND report_row.created_at < target.snapshot_date + INTERVAL '1 day'
        ) r ON TRUE
        LEFT JOIN LATERAL (
            SELECT COALESCE(
                JSONB_AGG(JSONB_BUILD_OBJECT('tag', tag_row.tag, 'count', tag_row.tag_count)
                    ORDER BY tag_row.tag_count DESC, tag_row.tag ASC),
                '[]'::JSONB
            ) AS top_tags
            FROM (
                SELECT tag_values.tag, COUNT(*)::INT AS tag_count
                FROM posts tagged_post
                CROSS JOIN LATERAL UNNEST(tagged_post.content_tags) AS tag_values(tag)
                WHERE tagged_post.circle_id = c.id
                  AND tagged_post.is_hidden = FALSE
                  AND tagged_post.created_at >= target.snapshot_date - INTERVAL '6 days'
                  AND tagged_post.created_at < target.snapshot_date + INTERVAL '1 day'
                GROUP BY tag_values.tag
                ORDER BY tag_count DESC, tag ASC
                LIMIT 10
            ) tag_row
        ) t ON TRUE
        WHERE COALESCE(c.analytics_enabled, TRUE) = TRUE
        ON CONFLICT (circle_id, snapshot_date) DO UPDATE SET
            member_count = EXCLUDED.member_count,
            active_members = EXCLUDED.active_members,
            posts_count = EXCLUDED.posts_count,
            comments_count = EXCLUDED.comments_count,
            qna_answer_rate_bps = EXCLUDED.qna_answer_rate_bps,
            reported_content_count = EXCLUDED.reported_content_count,
            top_tags = EXCLUDED.top_tags
        "#,
    )
    .execute(c_pool)
    .await?;

    Ok(result.rows_affected())
}

async fn refresh_circle_report_backlog_alerts(
    c_pool: &PgPool,
) -> Result<u64, crate::error::AppError> {
    let warning_threshold: i64 = std::env::var("POOOL_CIRCLE_REPORT_BACKLOG_WARNING")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let critical_threshold: i64 = std::env::var("POOOL_CIRCLE_REPORT_BACKLOG_CRITICAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);

    let upserted = sqlx::query(
        r#"
        WITH backlog AS (
            SELECT p.circle_id, COUNT(*)::BIGINT AS pending_reports
            FROM content_reports cr
            JOIN posts p ON p.id = cr.post_id
            WHERE cr.status = 'pending'
              AND p.circle_id IS NOT NULL
            GROUP BY p.circle_id
            HAVING COUNT(*) >= $1
        )
        INSERT INTO circle_ops_alerts (circle_id, alert_type, severity, summary, details)
        SELECT
            circle_id,
            'report_backlog',
            CASE WHEN pending_reports >= $2 THEN 'critical' ELSE 'warning' END,
            'Circle has pending report backlog',
            JSONB_BUILD_OBJECT(
                'pending_reports', pending_reports,
                'warning_threshold', $1,
                'critical_threshold', $2
            )
        FROM backlog
        ON CONFLICT (circle_id, alert_type) WHERE status = 'open' AND circle_id IS NOT NULL
        DO UPDATE SET
            severity = EXCLUDED.severity,
            summary = EXCLUDED.summary,
            details = EXCLUDED.details,
            created_at = NOW()
        "#,
    )
    .bind(warning_threshold)
    .bind(critical_threshold)
    .execute(c_pool)
    .await?
    .rows_affected();

    let resolved = sqlx::query(
        r#"
        WITH current_backlog AS (
            SELECT p.circle_id
            FROM content_reports cr
            JOIN posts p ON p.id = cr.post_id
            WHERE cr.status = 'pending'
              AND p.circle_id IS NOT NULL
            GROUP BY p.circle_id
            HAVING COUNT(*) >= $1
        )
        UPDATE circle_ops_alerts alert
        SET status = 'resolved', resolved_at = NOW()
        WHERE alert.alert_type = 'report_backlog'
          AND alert.status = 'open'
          AND alert.circle_id IS NOT NULL
          AND NOT EXISTS (
              SELECT 1 FROM current_backlog current
              WHERE current.circle_id = alert.circle_id
          )
        "#,
    )
    .bind(warning_threshold)
    .execute(c_pool)
    .await?
    .rows_affected();

    Ok(upserted + resolved)
}

async fn refresh_circle_moderation_sla_alerts(
    c_pool: &PgPool,
) -> Result<u64, crate::error::AppError> {
    let sla_hours: i32 = std::env::var("POOOL_CIRCLE_MODERATION_SLA_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(48);

    let upserted = sqlx::query(
        r#"
        WITH stale AS (
            SELECT p.circle_id,
                   COUNT(*)::BIGINT AS stale_reports,
                   MIN(cr.created_at) AS oldest_report_at
            FROM content_reports cr
            JOIN posts p ON p.id = cr.post_id
            WHERE cr.status = 'pending'
              AND p.circle_id IS NOT NULL
              AND cr.created_at < NOW() - ($1::INT * INTERVAL '1 hour')
            GROUP BY p.circle_id
        )
        INSERT INTO circle_ops_alerts (circle_id, alert_type, severity, summary, details)
        SELECT
            circle_id,
            'moderation_sla',
            'critical',
            'Circle has reports older than moderation SLA',
            JSONB_BUILD_OBJECT(
                'stale_reports', stale_reports,
                'sla_hours', $1,
                'oldest_report_at', oldest_report_at
            )
        FROM stale
        ON CONFLICT (circle_id, alert_type) WHERE status = 'open' AND circle_id IS NOT NULL
        DO UPDATE SET
            severity = EXCLUDED.severity,
            summary = EXCLUDED.summary,
            details = EXCLUDED.details,
            created_at = NOW()
        "#,
    )
    .bind(sla_hours)
    .execute(c_pool)
    .await?
    .rows_affected();

    let resolved = sqlx::query(
        r#"
        WITH current_stale AS (
            SELECT p.circle_id
            FROM content_reports cr
            JOIN posts p ON p.id = cr.post_id
            WHERE cr.status = 'pending'
              AND p.circle_id IS NOT NULL
              AND cr.created_at < NOW() - ($1::INT * INTERVAL '1 hour')
            GROUP BY p.circle_id
        )
        UPDATE circle_ops_alerts alert
        SET status = 'resolved', resolved_at = NOW()
        WHERE alert.alert_type = 'moderation_sla'
          AND alert.status = 'open'
          AND alert.circle_id IS NOT NULL
          AND NOT EXISTS (
              SELECT 1 FROM current_stale current
              WHERE current.circle_id = alert.circle_id
          )
        "#,
    )
    .bind(sla_hours)
    .execute(c_pool)
    .await?
    .rows_affected();

    Ok(upserted + resolved)
}

async fn enqueue_auto_critical_circle_ops_alert_notifications_once(
    c_pool: &PgPool,
) -> Result<u64, crate::error::AppError> {
    let cooldown_hours: i32 = std::env::var("POOOL_CIRCLE_OPS_AUTO_CRITICAL_COOLDOWN_HOURS")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .map(|n| n.clamp(1, 168))
        .unwrap_or(6);

    let email_queued = enqueue_auto_critical_circle_ops_alert_notifications_for_channel(
        c_pool,
        "email",
        cooldown_hours,
    )
    .await?;
    let slack_queued = if std::env::var("POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL").is_ok() {
        enqueue_auto_critical_circle_ops_alert_notifications_for_channel(
            c_pool,
            "slack",
            cooldown_hours,
        )
        .await?
    } else {
        0
    };
    let pagerduty_queued = if std::env::var("POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY").is_ok() {
        enqueue_auto_critical_circle_ops_alert_notifications_for_channel(
            c_pool,
            "pagerduty",
            cooldown_hours,
        )
        .await?
    } else {
        0
    };

    Ok(email_queued + slack_queued + pagerduty_queued)
}

async fn enqueue_auto_critical_circle_ops_alert_notifications_for_channel(
    c_pool: &PgPool,
    channel: &str,
    cooldown_hours: i32,
) -> Result<u64, crate::error::AppError> {
    let result = sqlx::query(
        r#"
        INSERT INTO circle_ops_alert_notifications (
            alert_id,
            channel,
            trigger_action,
            target_user_id,
            recipient_role,
            payload
        )
        SELECT
            alert.id,
            $2,
            'auto_critical',
            CASE WHEN $2 = 'email' THEN alert.assigned_to_user_id ELSE NULL END,
            CASE
                WHEN $2 <> 'email' THEN 'platform_admin_fallback'
                WHEN alert.assigned_to_user_id IS NULL THEN 'platform_admin_fallback'
                ELSE 'assigned_operator'
            END,
            JSONB_BUILD_OBJECT(
                'summary', alert.summary,
                'severity', alert.severity,
                'alert_type', alert.alert_type,
                'channel', $2,
                'circle_id', alert.circle_id,
                'circle_name', circle.name,
                'circle_slug', circle.slug,
                'trigger_action', 'auto_critical',
                'auto_critical_enqueued_at', NOW(),
                'cooldown_hours', $1,
                'details', COALESCE(alert.details, '{}'::JSONB)
            )
          FROM circle_ops_alerts alert
          LEFT JOIN circles circle ON circle.id = alert.circle_id
         WHERE alert.status = 'open'
           AND alert.severity = 'critical'
           AND alert.alert_type IN ('report_backlog', 'moderation_sla')
           AND (alert.snoozed_until IS NULL OR alert.snoozed_until <= NOW())
           AND NOT EXISTS (
                SELECT 1
                 FROM circle_ops_alert_notifications existing
                 WHERE existing.alert_id = alert.id
                   AND existing.channel = $2
                   AND existing.trigger_action = 'auto_critical'
                   AND existing.status IN ('queued', 'sending', 'enqueued', 'failed')
                   AND existing.created_at >= NOW() - ($1::INT * INTERVAL '1 hour')
           )
        ON CONFLICT (
            alert_id,
            channel,
            trigger_action,
            (COALESCE(target_user_id, '00000000-0000-0000-0000-000000000000'::UUID))
        )
        WHERE status = ANY (ARRAY['queued'::TEXT, 'sending'::TEXT])
        DO NOTHING
        "#,
    )
    .bind(cooldown_hours)
    .bind(channel)
    .execute(c_pool)
    .await?;

    Ok(result.rows_affected())
}

async fn upsert_circle_failed_worker_alert(
    c_pool: &PgPool,
    worker_name: &str,
    consecutive_failures: u32,
    error_message: &str,
) -> Result<u64, crate::error::AppError> {
    let result = sqlx::query(
        r#"
        INSERT INTO circle_ops_alerts (circle_id, alert_type, severity, summary, details)
        VALUES (
            NULL,
            'failed_worker',
            'critical',
            'Circle background worker is failing repeatedly',
            JSONB_BUILD_OBJECT(
                'worker', $1,
                'consecutive_failures', $2,
                'last_error', LEFT($3, 1000)
            )
        )
        ON CONFLICT (alert_type) WHERE status = 'open' AND circle_id IS NULL
        DO UPDATE SET
            severity = EXCLUDED.severity,
            summary = EXCLUDED.summary,
            details = EXCLUDED.details,
            created_at = NOW()
        "#,
    )
    .bind(worker_name)
    .bind(consecutive_failures as i64)
    .bind(error_message)
    .execute(c_pool)
    .await?;

    Ok(result.rows_affected())
}

async fn resolve_circle_failed_worker_alert(
    c_pool: &PgPool,
    worker_name: &str,
) -> Result<u64, crate::error::AppError> {
    let result = sqlx::query(
        r#"
        UPDATE circle_ops_alerts
        SET status = 'resolved',
            resolved_at = NOW(),
            details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                'recovered_worker', $1,
                'recovered_at', NOW()
            )
        WHERE circle_id IS NULL
          AND alert_type = 'failed_worker'
          AND status = 'open'
          AND details->>'worker' = $1
        "#,
    )
    .bind(worker_name)
    .execute(c_pool)
    .await?;

    Ok(result.rows_affected())
}

/// Background worker that bridges Circle ops alert notification intents into
/// the core transactional email outbox.
pub async fn circle_ops_alert_fanout_worker(c_pool: PgPool, core_pool: PgPool) {
    let interval_secs: u64 = std::env::var("POOOL_CIRCLE_OPS_ALERT_FANOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n.max(60))
        .unwrap_or(60);
    tracing::info!(
        interval_secs = interval_secs,
        "Circle ops alert fanout worker starting (override via POOOL_CIRCLE_OPS_ALERT_FANOUT_SECS)"
    );

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::time::sleep(Duration::from_secs(30)).await;

    loop {
        interval.tick().await;
        match process_circle_ops_alert_fanout_once(&c_pool, &core_pool, 25).await {
            Ok(summary) => {
                if summary.claimed > 0 {
                    tracing::info!(
                        metric_name = "circle_ops_alert_fanout",
                        claimed = summary.claimed,
                        enqueued = summary.enqueued,
                        skipped = summary.skipped,
                        failed = summary.failed,
                        slack_sent = summary.slack_sent,
                        pagerduty_sent = summary.pagerduty_sent,
                        "Circle ops alert fanout pass complete"
                    );
                }
            }
            Err(err) => {
                tracing::error!(
                    metric_name = "circle_ops_alert_fanout_failure",
                    error = %err,
                    "Circle ops alert fanout worker failed"
                );
            }
        }
    }
}

pub async fn circle_ops_alert_delivery_monitor_worker(c_pool: PgPool, core_pool: PgPool) {
    let interval_secs: u64 = std::env::var("POOOL_CIRCLE_OPS_ALERT_DELIVERY_MONITOR_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n.max(60))
        .unwrap_or(5 * 60);
    tracing::info!(
        interval_secs = interval_secs,
        "Circle ops alert delivery monitor starting (override via POOOL_CIRCLE_OPS_ALERT_DELIVERY_MONITOR_SECS)"
    );

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tokio::time::sleep(Duration::from_secs(45)).await;

    loop {
        interval.tick().await;
        match monitor_circle_ops_alert_delivery_once(&c_pool, &core_pool, 100).await {
            Ok(summary) => {
                if summary.checked > 0 || summary.alerts_touched > 0 {
                    tracing::info!(
                        metric_name = "circle_ops_alert_delivery_monitor",
                        checked = summary.checked,
                        sent = summary.sent,
                        skipped = summary.skipped,
                        pending = summary.pending,
                        unhealthy = summary.unhealthy,
                        missing = summary.missing,
                        alerts_touched = summary.alerts_touched,
                        "Circle ops alert delivery monitor pass complete"
                    );
                }
            }
            Err(err) => {
                tracing::error!(
                    metric_name = "circle_ops_alert_delivery_monitor_failure",
                    error = %err,
                    "Circle ops alert delivery monitor failed"
                );
            }
        }
    }
}

pub async fn monitor_circle_ops_alert_delivery_once(
    c_pool: &PgPool,
    core_pool: &PgPool,
    batch_size: i64,
) -> Result<CircleOpsAlertDeliveryMonitorSummary, crate::error::AppError> {
    use sqlx::Row;

    let pending_minutes: i32 = std::env::var("POOOL_CIRCLE_OPS_ALERT_DELIVERY_PENDING_MINUTES")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .map(|n| n.max(1))
        .unwrap_or(30);
    let failure_attempts: i32 = std::env::var("POOOL_CIRCLE_OPS_ALERT_DELIVERY_FAILURE_ATTEMPTS")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .map(|n| n.clamp(1, 10))
        .unwrap_or(5);

    let rows = sqlx::query(
        r#"
        SELECT id, alert_id, enqueued_email_outbox_id
          FROM circle_ops_alert_notifications
         WHERE status = 'enqueued'
           AND enqueued_email_outbox_id IS NOT NULL
           AND (
                delivery_checked_at IS NULL
             OR delivery_checked_at <= NOW() - INTERVAL '1 minute'
           )
         ORDER BY COALESCE(delivery_checked_at, created_at), created_at
         LIMIT $1
        "#,
    )
    .bind(batch_size.clamp(1, 250))
    .fetch_all(c_pool)
    .await?;

    let mut summary = CircleOpsAlertDeliveryMonitorSummary {
        checked: rows.len() as u64,
        ..CircleOpsAlertDeliveryMonitorSummary::default()
    };

    for row in rows {
        let notification_id: Uuid = row.try_get("id")?;
        let outbox_id: Uuid = row.try_get("enqueued_email_outbox_id")?;
        let outbox = sqlx::query(
            r#"
            SELECT status,
                   attempts,
                   last_error,
                   sent_at,
                   created_at < NOW() - ($2::INT * INTERVAL '1 minute') AS stale_delivery,
                   status = 'failed' AND attempts >= $3 AS exhausted_delivery
              FROM transactional_email_outbox
             WHERE id = $1
               AND event_type = 'community_ops_alert_on_call'
             LIMIT 1
            "#,
        )
        .bind(outbox_id)
        .bind(pending_minutes)
        .bind(failure_attempts)
        .fetch_optional(core_pool)
        .await?;

        let Some(outbox) = outbox else {
            update_circle_ops_alert_delivery_state(
                c_pool,
                notification_id,
                "missing",
                None,
                Some("core transactional email outbox row missing"),
                None,
                true,
            )
            .await?;
            summary.missing += 1;
            summary.unhealthy += 1;
            continue;
        };

        let status = outbox
            .try_get::<String, _>("status")
            .unwrap_or_else(|_| "missing".to_string());
        let attempts = outbox.try_get::<i32, _>("attempts").unwrap_or(0);
        let last_error = outbox
            .try_get::<Option<String>, _>("last_error")
            .ok()
            .flatten();
        let sent_at = outbox
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("sent_at")
            .ok()
            .flatten();
        let stale_delivery = outbox.try_get::<bool, _>("stale_delivery").unwrap_or(false);
        let exhausted_delivery = outbox
            .try_get::<bool, _>("exhausted_delivery")
            .unwrap_or(false);
        let unhealthy = stale_delivery || exhausted_delivery || status == "missing";

        match status.as_str() {
            "sent" => summary.sent += 1,
            "skipped" => summary.skipped += 1,
            "queued" | "sending" | "failed" if !unhealthy => summary.pending += 1,
            _ if unhealthy => summary.unhealthy += 1,
            _ => {}
        }

        update_circle_ops_alert_delivery_state(
            c_pool,
            notification_id,
            &status,
            Some(attempts),
            last_error.as_deref(),
            sent_at,
            unhealthy,
        )
        .await?;
    }

    summary.alerts_touched =
        refresh_circle_ops_alert_delivery_health(c_pool, failure_attempts, pending_minutes).await?;
    Ok(summary)
}

/// Process one bounded batch of ready Circle ops alert notification rows.
pub async fn process_circle_ops_alert_fanout_once(
    c_pool: &PgPool,
    core_pool: &PgPool,
    batch_size: i64,
) -> Result<CircleOpsAlertFanoutSummary, crate::error::AppError> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        WITH picked AS (
            SELECT id
            FROM circle_ops_alert_notifications
            WHERE status IN ('queued', 'failed')
              AND next_attempt_at <= NOW()
              AND attempts < 10
            ORDER BY next_attempt_at ASC, created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
        )
        UPDATE circle_ops_alert_notifications notification
           SET status = 'sending',
               attempts = attempts + 1,
               updated_at = NOW()
          FROM picked
         WHERE notification.id = picked.id
         RETURNING notification.id,
                   notification.alert_id,
                   notification.channel,
                   notification.trigger_action,
                   notification.target_user_id,
                   notification.recipient_role,
                   notification.attempts,
                   notification.payload
        "#,
    )
    .bind(batch_size.clamp(1, 100))
    .fetch_all(c_pool)
    .await?;

    let mut summary = CircleOpsAlertFanoutSummary {
        claimed: rows.len() as u64,
        ..CircleOpsAlertFanoutSummary::default()
    };

    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let alert_id: Uuid = row.try_get("alert_id")?;
        let channel: String = row
            .try_get("channel")
            .unwrap_or_else(|_| "email".to_string());
        let trigger_action: String = row.try_get("trigger_action")?;
        let target_user_id: Option<Uuid> = row.try_get("target_user_id")?;
        let attempts: i32 = row.try_get("attempts")?;
        let payload: serde_json::Value = row
            .try_get("payload")
            .unwrap_or_else(|_| serde_json::json!({}));

        let disposition = match channel.as_str() {
            "email" => {
                bridge_circle_ops_alert_notification(
                    c_pool,
                    core_pool,
                    id,
                    alert_id,
                    &trigger_action,
                    target_user_id,
                    attempts,
                    payload,
                )
                .await
            }
            "slack" | "pagerduty" => {
                bridge_circle_ops_alert_external_webhook(
                    c_pool,
                    id,
                    alert_id,
                    &channel,
                    &trigger_action,
                    attempts,
                    payload,
                )
                .await
            }
            _ => Err(crate::error::AppError::Internal(format!(
                "Unsupported Circle ops alert notification channel: {channel}"
            ))),
        };

        match disposition {
            Ok(FanoutDisposition::Enqueued) => {
                summary.enqueued += 1;
                if channel == "slack" {
                    summary.slack_sent += 1;
                } else if channel == "pagerduty" {
                    summary.pagerduty_sent += 1;
                }
            }
            Ok(FanoutDisposition::Skipped) => summary.skipped += 1,
            Err(err) => {
                summary.failed += 1;
                mark_circle_ops_alert_notification_failed(c_pool, id, attempts, &err.to_string())
                    .await?;
            }
        }
    }

    Ok(summary)
}

enum FanoutDisposition {
    Enqueued,
    Skipped,
}

async fn bridge_circle_ops_alert_notification(
    c_pool: &PgPool,
    core_pool: &PgPool,
    notification_id: Uuid,
    alert_id: Uuid,
    trigger_action: &str,
    target_user_id: Option<Uuid>,
    attempts: i32,
    payload: serde_json::Value,
) -> Result<FanoutDisposition, crate::error::AppError> {
    let recipient =
        resolve_circle_ops_alert_notification_recipient(core_pool, target_user_id).await?;
    let Some((user_id, email)) = recipient else {
        sqlx::query(
            r#"
            UPDATE circle_ops_alert_notifications
               SET status = 'skipped',
                   last_error = 'no active assigned user or platform admin fallback recipient',
                   updated_at = NOW()
             WHERE id = $1
            "#,
        )
        .bind(notification_id)
        .execute(c_pool)
        .await?;
        return Ok(FanoutDisposition::Skipped);
    };

    let summary = payload_text(&payload, "summary").unwrap_or("Circle ops alert");
    let severity = payload_text(&payload, "severity").unwrap_or("warning");
    let subject = format!("[POOOL Ops] {} Circle alert: {}", severity, summary);
    let html_body = render_circle_ops_alert_email(alert_id, trigger_action, &payload);

    let outbox_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO transactional_email_outbox (
            user_id,
            event_type,
            recipient_email,
            subject,
            html_body
        )
        VALUES ($1, 'community_ops_alert_on_call', $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(&subject)
    .bind(&html_body)
    .fetch_one(core_pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE circle_ops_alert_notifications
           SET status = 'enqueued',
               enqueued_email_outbox_id = $2,
               last_error = NULL,
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(notification_id)
    .bind(outbox_id)
    .execute(c_pool)
    .await?;

    tracing::info!(
        notification_id = %notification_id,
        alert_id = %alert_id,
        trigger_action = %trigger_action,
        attempts = attempts,
        outbox_id = %outbox_id,
        "Circle ops alert notification bridged to transactional email outbox"
    );

    Ok(FanoutDisposition::Enqueued)
}

async fn bridge_circle_ops_alert_external_webhook(
    c_pool: &PgPool,
    notification_id: Uuid,
    alert_id: Uuid,
    channel: &str,
    trigger_action: &str,
    attempts: i32,
    payload: serde_json::Value,
) -> Result<FanoutDisposition, crate::error::AppError> {
    let request =
        build_circle_ops_alert_webhook_request(alert_id, channel, trigger_action, &payload)?;
    let Some((url, body)) = request else {
        sqlx::query(
            r#"
            UPDATE circle_ops_alert_notifications
               SET status = 'skipped',
                   last_error = $2,
                   updated_at = NOW()
             WHERE id = $1
            "#,
        )
        .bind(notification_id)
        .bind(format!("{channel} webhook is not configured"))
        .execute(c_pool)
        .await?;
        return Ok(FanoutDisposition::Skipped);
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| crate::error::AppError::Internal(format!("webhook client: {e}")))?;

    let response =
        client.post(&url).json(&body).send().await.map_err(|e| {
            crate::error::AppError::Internal(format!("{channel} webhook failed: {e}"))
        })?;
    let status = response.status();

    if !status.is_success() {
        sqlx::query(
            r#"
            UPDATE circle_ops_alert_notifications
               SET provider_response_status = $2,
                   provider_response_at = NOW(),
                   updated_at = NOW()
             WHERE id = $1
            "#,
        )
        .bind(notification_id)
        .bind(status.as_u16() as i32)
        .execute(c_pool)
        .await?;
        return Err(crate::error::AppError::Internal(format!(
            "{channel} webhook returned HTTP {status}"
        )));
    }

    sqlx::query(
        r#"
        UPDATE circle_ops_alert_notifications
           SET status = 'enqueued',
               provider_response_status = $2,
               provider_response_at = NOW(),
               last_error = NULL,
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(notification_id)
    .bind(status.as_u16() as i32)
    .execute(c_pool)
    .await?;

    tracing::info!(
        notification_id = %notification_id,
        alert_id = %alert_id,
        channel = channel,
        trigger_action = trigger_action,
        attempts = attempts,
        status = %status,
        "Circle ops alert external webhook delivered"
    );

    Ok(FanoutDisposition::Enqueued)
}

fn build_circle_ops_alert_webhook_request(
    alert_id: Uuid,
    channel: &str,
    trigger_action: &str,
    payload: &serde_json::Value,
) -> Result<Option<(String, serde_json::Value)>, crate::error::AppError> {
    let summary = payload_text(payload, "summary").unwrap_or("Circle ops alert");
    let severity = payload_text(payload, "severity").unwrap_or("warning");
    let alert_type = payload_text(payload, "alert_type").unwrap_or("ops_alert");
    let circle_slug = payload_text(payload, "circle_slug").unwrap_or("platform");
    let headline = format!("[POOOL Ops] {severity} {alert_type}: {summary}");

    match channel {
        "slack" => {
            let Some(url) =
                optional_circle_ops_alert_webhook_url("POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL")?
            else {
                return Ok(None);
            };
            Ok(Some((
                url,
                serde_json::json!({
                    "text": headline,
                    "blocks": [
                        {
                            "type": "section",
                            "text": {
                                "type": "mrkdwn",
                                "text": format!("*{}*\\nCircle: `{}`\\nTrigger: `{}`", headline, circle_slug, trigger_action)
                            }
                        },
                        {
                            "type": "context",
                            "elements": [
                                {
                                    "type": "mrkdwn",
                                    "text": format!("Alert `{}` · Channel `slack`", alert_id)
                                }
                            ]
                        }
                    ],
                    "metadata": {
                        "event_type": "poool_circle_ops_alert",
                        "event_payload": {
                            "alert_id": alert_id,
                            "trigger_action": trigger_action,
                            "severity": severity,
                            "alert_type": alert_type,
                            "circle_slug": circle_slug
                        }
                    }
                }),
            )))
        }
        "pagerduty" => {
            let routing_key = match std::env::var("POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY") {
                Ok(value) if !value.trim().is_empty() => value,
                _ => return Ok(None),
            };
            let url = match optional_circle_ops_alert_webhook_url(
                "POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL",
            )? {
                Some(url) => url,
                None => normalize_circle_ops_alert_webhook_url(
                    "https://events.pagerduty.com/v2/enqueue",
                    crate::storage::service::is_local_fallback_allowed(),
                )?,
            };
            Ok(Some((
                url,
                serde_json::json!({
                    "routing_key": routing_key,
                    "event_action": "trigger",
                    "dedup_key": format!("poool-circle-ops-{}", alert_id),
                    "payload": {
                        "summary": headline,
                        "source": "poool-community",
                        "severity": severity,
                        "component": "community-circles",
                        "group": circle_slug,
                        "class": alert_type,
                        "custom_details": {
                            "alert_id": alert_id,
                            "trigger_action": trigger_action,
                            "payload": payload
                        }
                    }
                }),
            )))
        }
        _ => Ok(None),
    }
}

fn optional_circle_ops_alert_webhook_url(
    env_key: &str,
) -> Result<Option<String>, crate::error::AppError> {
    let raw = match std::env::var(env_key) {
        Ok(value) if value.trim().is_empty() => return Ok(None),
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(crate::error::AppError::BadRequest(format!(
                "{env_key} must be valid UTF-8."
            )));
        }
    };

    normalize_circle_ops_alert_webhook_url(
        &raw,
        crate::storage::service::is_local_fallback_allowed(),
    )
    .map(Some)
}

fn normalize_circle_ops_alert_webhook_url(
    raw_url: &str,
    allow_local_http: bool,
) -> Result<String, crate::error::AppError> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(crate::error::AppError::BadRequest(
            "Invalid Circle ops alert webhook URL.".into(),
        ));
    }

    let parsed = url::Url::parse(trimmed).map_err(|_| {
        crate::error::AppError::BadRequest("Invalid Circle ops alert webhook URL.".into())
    })?;
    if !parsed.username().is_empty() || parsed.password().is_some() || parsed.fragment().is_some() {
        return Err(crate::error::AppError::BadRequest(
            "Circle ops alert webhook URL must not contain credentials or fragments.".into(),
        ));
    }

    let is_local_http = parsed.scheme() == "http"
        && allow_local_http
        && matches!(
            parsed.host_str(),
            Some("localhost") | Some("127.0.0.1") | Some("::1")
        );
    if parsed.scheme() != "https" && !is_local_http {
        return Err(crate::error::AppError::BadRequest(
            "Circle ops alert webhook URL must use HTTPS outside local development.".into(),
        ));
    }

    Ok(parsed.to_string())
}

#[cfg(test)]
mod circle_ops_alert_webhook_tests {
    use super::normalize_circle_ops_alert_webhook_url;

    #[test]
    fn webhook_url_validation_accepts_https_endpoints() {
        let normalized = normalize_circle_ops_alert_webhook_url(
            " https://hooks.slack.com/services/T000/B000/secret ",
            false,
        )
        .expect("https webhook should be valid");

        assert_eq!(
            normalized,
            "https://hooks.slack.com/services/T000/B000/secret"
        );
    }

    #[test]
    fn webhook_url_validation_rejects_public_http_endpoints() {
        assert!(normalize_circle_ops_alert_webhook_url(
            "http://hooks.slack.com/services/T000/B000/secret",
            false,
        )
        .is_err());
    }

    #[test]
    fn webhook_url_validation_allows_local_http_for_development_mocks() {
        let normalized =
            normalize_circle_ops_alert_webhook_url("http://127.0.0.1:18080/slack", true)
                .expect("local HTTP mock should be valid in development");

        assert_eq!(normalized, "http://127.0.0.1:18080/slack");
    }

    #[test]
    fn webhook_url_validation_rejects_credentials_and_fragments() {
        assert!(normalize_circle_ops_alert_webhook_url(
            "https://user:pass@example.com/hook",
            false
        )
        .is_err());
        assert!(
            normalize_circle_ops_alert_webhook_url("https://example.com/hook#secret", false)
                .is_err()
        );
    }
}

async fn resolve_circle_ops_alert_notification_recipient(
    core_pool: &PgPool,
    target_user_id: Option<Uuid>,
) -> Result<Option<(Uuid, String)>, crate::error::AppError> {
    if let Some(user_id) = target_user_id {
        if let Some(row) = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, email FROM users WHERE id = $1 AND status <> 'deleted'",
        )
        .bind(user_id)
        .fetch_optional(core_pool)
        .await?
        {
            return Ok(Some(row));
        }
    }

    let fallback = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, email FROM users WHERE email = 'admin@poool.app' AND status <> 'deleted' LIMIT 1",
    )
    .fetch_optional(core_pool)
    .await?;
    Ok(fallback)
}

async fn mark_circle_ops_alert_notification_failed(
    c_pool: &PgPool,
    notification_id: Uuid,
    attempts: i32,
    error: &str,
) -> Result<(), crate::error::AppError> {
    let capped = attempts.clamp(1, 6) as u32;
    let retry_delay_seconds = (60_i64 * 2_i64.pow(capped - 1)).min(1_800);
    sqlx::query(
        r#"
        UPDATE circle_ops_alert_notifications
           SET status = 'failed',
               last_error = LEFT($2, 1000),
               next_attempt_at = NOW() + ($3::INT * INTERVAL '1 second'),
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(notification_id)
    .bind(error)
    .bind(retry_delay_seconds as i32)
    .execute(c_pool)
    .await?;
    Ok(())
}

async fn update_circle_ops_alert_delivery_state(
    c_pool: &PgPool,
    notification_id: Uuid,
    status: &str,
    attempts: Option<i32>,
    last_error: Option<&str>,
    sent_at: Option<chrono::DateTime<chrono::Utc>>,
    unhealthy: bool,
) -> Result<(), crate::error::AppError> {
    sqlx::query(
        r#"
        UPDATE circle_ops_alert_notifications
           SET email_outbox_status = $2,
               email_outbox_attempts = $3,
               email_outbox_last_error = $4,
               email_outbox_sent_at = $5,
               delivery_checked_at = NOW(),
               delivery_alerted_at = CASE
                 WHEN $6 THEN COALESCE(delivery_alerted_at, NOW())
                 ELSE delivery_alerted_at
               END,
               updated_at = NOW()
         WHERE id = $1
        "#,
    )
    .bind(notification_id)
    .bind(status)
    .bind(attempts)
    .bind(last_error)
    .bind(sent_at)
    .bind(unhealthy)
    .execute(c_pool)
    .await?;
    Ok(())
}

async fn refresh_circle_ops_alert_delivery_health(
    c_pool: &PgPool,
    failure_attempts: i32,
    pending_minutes: i32,
) -> Result<u64, crate::error::AppError> {
    let upserted = sqlx::query(
        r#"
        WITH unhealthy AS (
            SELECT
                COUNT(*)::BIGINT AS total_unhealthy,
                COUNT(*) FILTER (WHERE email_outbox_status = 'missing')::BIGINT AS missing_count,
                COUNT(*) FILTER (
                    WHERE email_outbox_status = 'failed'
                      AND COALESCE(email_outbox_attempts, 0) >= $1
                )::BIGINT AS exhausted_failed_count,
                COUNT(*) FILTER (
                    WHERE email_outbox_status IN ('queued', 'sending', 'failed')
                      AND created_at < NOW() - ($2::INT * INTERVAL '1 minute')
                )::BIGINT AS stale_count,
                MIN(created_at) AS oldest_unhealthy_at
            FROM circle_ops_alert_notifications
            WHERE status = 'enqueued'
              AND (
                   email_outbox_status = 'missing'
                OR (
                    email_outbox_status = 'failed'
                    AND COALESCE(email_outbox_attempts, 0) >= $1
                )
                OR (
                    email_outbox_status IN ('queued', 'sending', 'failed')
                    AND created_at < NOW() - ($2::INT * INTERVAL '1 minute')
                )
              )
        )
        INSERT INTO circle_ops_alerts (circle_id, alert_type, severity, summary, details)
        SELECT
            NULL,
            'notification_delivery',
            CASE WHEN missing_count > 0 OR exhausted_failed_count > 0 THEN 'critical' ELSE 'warning' END,
            'Circle ops alert email delivery is unhealthy',
            JSONB_BUILD_OBJECT(
                'total_unhealthy', total_unhealthy,
                'missing_count', missing_count,
                'exhausted_failed_count', exhausted_failed_count,
                'stale_count', stale_count,
                'oldest_unhealthy_at', oldest_unhealthy_at,
                'failure_attempts_threshold', $1,
                'pending_minutes_threshold', $2
            )
        FROM unhealthy
        WHERE total_unhealthy > 0
        ON CONFLICT (alert_type) WHERE status = 'open' AND circle_id IS NULL
        DO UPDATE SET
            severity = EXCLUDED.severity,
            summary = EXCLUDED.summary,
            details = EXCLUDED.details,
            created_at = NOW()
        "#,
    )
    .bind(failure_attempts)
    .bind(pending_minutes)
    .execute(c_pool)
    .await?
    .rows_affected();

    let resolved = sqlx::query(
        r#"
        WITH unhealthy AS (
            SELECT 1
            FROM circle_ops_alert_notifications
            WHERE status = 'enqueued'
              AND (
                   email_outbox_status = 'missing'
                OR (
                    email_outbox_status = 'failed'
                    AND COALESCE(email_outbox_attempts, 0) >= $1
                )
                OR (
                    email_outbox_status IN ('queued', 'sending', 'failed')
                    AND created_at < NOW() - ($2::INT * INTERVAL '1 minute')
                )
              )
            LIMIT 1
        )
        UPDATE circle_ops_alerts
           SET status = 'resolved',
               resolved_at = NOW(),
               details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                   'delivery_recovered_at', NOW()
               )
         WHERE circle_id IS NULL
           AND alert_type = 'notification_delivery'
           AND status = 'open'
           AND NOT EXISTS (SELECT 1 FROM unhealthy)
        "#,
    )
    .bind(failure_attempts)
    .bind(pending_minutes)
    .execute(c_pool)
    .await?
    .rows_affected();

    Ok(upserted + resolved)
}

fn payload_text<'a>(payload: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|value| value.as_str())
}

fn render_circle_ops_alert_email(
    alert_id: Uuid,
    trigger_action: &str,
    payload: &serde_json::Value,
) -> String {
    let summary = escape_email_html(payload_text(payload, "summary").unwrap_or("Circle ops alert"));
    let severity = escape_email_html(payload_text(payload, "severity").unwrap_or("warning"));
    let alert_type = escape_email_html(payload_text(payload, "alert_type").unwrap_or("ops_alert"));
    let circle = escape_email_html(payload_text(payload, "circle_name").unwrap_or("Platform-wide"));
    let action = escape_email_html(trigger_action);
    let base_url = std::env::var("PUBLIC_APP_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("BASE_URL").ok())
        .unwrap_or_else(|| "https://platform.poool.app".to_string());
    let ops_url = format!(
        "{}/admin/community/circles.html",
        base_url.trim_end_matches('/')
    );
    let ops_url_html = escape_email_html(&ops_url);

    format!(
        r#"
        <h1>Circle ops alert requires attention</h1>
        <p><strong>{summary}</strong></p>
        <table>
          <tr><td>Severity</td><td>{severity}</td></tr>
          <tr><td>Type</td><td>{alert_type}</td></tr>
          <tr><td>Scope</td><td>{circle}</td></tr>
          <tr><td>Trigger</td><td>{action}</td></tr>
          <tr><td>Alert ID</td><td>{alert_id}</td></tr>
        </table>
        <p><a href="{ops_url_html}">Open Circle Ops Alerts</a></p>
        "#
    )
}

fn escape_email_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
