/// XP System — Append-only XP ledger, level calculation, and awarding.
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

// ─── XP Amounts per Reason ──────────────────────────────────────────

pub fn xp_for_reason(reason: &str) -> i32 {
    match reason {
        "post_created" => 10,
        "comment_created" => 5,
        "reaction_given" => 2,
        "reaction_received" => 3,
        "follow_gained" => 5,
        "profile_completed" => 25,
        "first_post" => 25,
        "first_investment" => 50,
        "investment_milestone_5" => 100,
        "investment_milestone_10" => 200,
        "investment_milestone_25" => 500,
        "investment_milestone_50" => 1000,
        "circle_created" => 20,
        "circle_joined" => 10,
        "circle_invite_accepted" => 10,
        "daily_login" => 5,
        "login_streak_7" => 50,
        "login_streak_30" => 200,
        "badge_earned" => 15,
        "referral_signup" => 25,
        "referral_first_investment" => 50,
        "onboarding_complete" => 50,
        "admin_grant" => 0,  // variable, set explicitly
        "admin_revoke" => 0, // variable, set explicitly
        _ => 0,
    }
}

// ─── Daily Caps ─────────────────────────────────────────────────────

const DAILY_POST_XP_CAP: i32 = 50; // Max 5 posts worth per day
const DAILY_COMMENT_XP_CAP: i32 = 30; // Max 6 comments worth per day
const DAILY_REACTION_XP_CAP: i32 = 20; // Max 10 reactions per day

async fn check_daily_cap(pool: &PgPool, user_id: Uuid, reason: &str) -> Result<bool, AppError> {
    let cap = match reason {
        "post_created" => DAILY_POST_XP_CAP,
        "comment_created" => DAILY_COMMENT_XP_CAP,
        "reaction_given" => DAILY_REACTION_XP_CAP,
        _ => return Ok(true), // no cap for other reasons
    };

    let today_total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM xp_ledger WHERE user_id = $1 AND reason = $2 AND created_at::date = CURRENT_DATE"
    )
    .bind(user_id)
    .bind(reason)
    .fetch_one(pool)
    .await?;

    Ok(today_total < cap as i64)
}

// ─── Award XP ───────────────────────────────────────────────────────

/// Award XP for a specific reason. Respects daily caps.
/// Returns the amount awarded (0 if capped).
pub async fn award_xp(
    pool: &PgPool,
    user_id: Uuid,
    reason: &str,
    description: Option<&str>,
    custom_amount: Option<i32>,
) -> Result<i32, AppError> {
    // Check daily cap
    if !check_daily_cap(pool, user_id, reason).await? {
        return Ok(0);
    }

    let amount = custom_amount.unwrap_or_else(|| xp_for_reason(reason));
    if amount == 0 {
        return Ok(0);
    }

    // Ensure profile exists
    sqlx::query(
        "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    // Insert XP entry
    sqlx::query(
        "INSERT INTO xp_ledger (user_id, amount, reason, description) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(amount)
    .bind(reason)
    .bind(description)
    .execute(pool)
    .await?;

    // Update total XP on profile
    sqlx::query("UPDATE community_profiles SET xp_total = xp_total + $1 WHERE user_id = $2")
        .bind(amount)
        .bind(user_id)
        .execute(pool)
        .await?;

    // Recalculate level
    update_user_level(pool, user_id).await?;

    Ok(amount)
}

// ─── Level Calculation ──────────────────────────────────────────────

/// Update user level based on current xp_total.
pub async fn update_user_level(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<(i32, String)>, AppError> {
    let xp_total: i32 =
        sqlx::query_scalar("SELECT xp_total FROM community_profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or(0);

    let (new_level, new_name): (i32, String) = sqlx::query_as(
        "SELECT level, name FROM xp_levels WHERE min_xp <= $1 ORDER BY level DESC LIMIT 1",
    )
    .bind(xp_total)
    .fetch_optional(pool)
    .await?
    .unwrap_or((1, "Seedling".to_string()));

    // Get old level
    let old_level: i32 =
        sqlx::query_scalar("SELECT level FROM community_profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or(1);

    // Update profile
    sqlx::query("UPDATE community_profiles SET level = $1, level_name = $2 WHERE user_id = $3")
        .bind(new_level)
        .bind(&new_name)
        .bind(user_id)
        .execute(pool)
        .await?;

    if new_level > old_level {
        let notif_content = format!(
            "Congratulations! You've leveled up to {} (Level {}).",
            new_name, new_level
        );
        let _ = crate::community::notifications::notify_user(
            pool,
            user_id,
            None,
            "level_up",
            None,
            &notif_content,
            Some("/community?tab=xp-tracker"),
        )
        .await;

        Ok(Some((new_level, new_name)))
    } else {
        Ok(None)
    }
}

// ─── XP History ─────────────────────────────────────────────────────

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct XpEntry {
    pub id: Uuid,
    pub amount: i32,
    pub reason: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_xp_history(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<XpEntry>, AppError> {
    let entries = sqlx::query_as::<_, XpEntry>(
        "SELECT id, amount, reason, description, created_at FROM xp_ledger WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(user_id)
    .bind(limit.clamp(1, 50))
    .bind(offset.max(0))
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

// ─── XP Summary ─────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct XpSummary {
    pub user_id: Uuid,
    pub xp_total: i32,
    pub level: i32,
    pub level_name: String,
    pub level_icon: String,
    pub next_level_xp: i32,
    pub xp_to_next: i32,
    pub progress_pct: f32,
    pub login_streak: i32,
}

pub async fn get_xp_summary(pool: &PgPool, user_id: Uuid) -> Result<XpSummary, AppError> {
    let (xp_total, level, level_name, login_streak): (i32, i32, String, i32) = sqlx::query_as(
        "SELECT xp_total, level, level_name, login_streak FROM community_profiles WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or((0, 1, "Seedling".to_string(), 0));

    let icon: String = sqlx::query_scalar("SELECT icon FROM xp_levels WHERE level = $1")
        .bind(level)
        .fetch_optional(pool)
        .await?
        .unwrap_or_else(|| "🌱".to_string());

    let next_level_xp: i32 = sqlx::query_scalar("SELECT min_xp FROM xp_levels WHERE level = $1")
        .bind(level + 1)
        .fetch_optional(pool)
        .await?
        .unwrap_or(i32::MAX);

    let current_level_xp: i32 = sqlx::query_scalar("SELECT min_xp FROM xp_levels WHERE level = $1")
        .bind(level)
        .fetch_optional(pool)
        .await?
        .unwrap_or(0);

    let range = (next_level_xp - current_level_xp).max(1);
    let progress = (xp_total - current_level_xp) as f32 / range as f32;

    Ok(XpSummary {
        user_id,
        xp_total,
        level,
        level_name,
        level_icon: icon,
        next_level_xp,
        xp_to_next: (next_level_xp - xp_total).max(0),
        progress_pct: progress.clamp(0.0, 1.0) * 100.0,
        login_streak,
    })
}

// ─── User Leaderboard ───────────────────────────────────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UserLeaderboardEntry {
    pub user_id: Uuid,
    pub xp_total: i32,
    pub level: i32,
    pub level_name: String,
    pub circle_id: Option<Uuid>,
    pub login_streak: i32,
}

pub async fn get_user_leaderboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<UserLeaderboardEntry>, AppError> {
    let entries = sqlx::query_as::<_, UserLeaderboardEntry>(
        r#"SELECT user_id, xp_total, level, level_name, circle_id, login_streak
           FROM community_profiles
           ORDER BY xp_total DESC
           LIMIT $1"#,
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

/// Time window for the global XP leaderboard.
///
/// `Alltime` reads precomputed `community_profiles.xp_total` (fast).
/// `Week`/`Month` aggregates the `xp_ledger` over the cutoff window — slower
/// but always consistent with the ledger of record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardPeriod {
    Week,
    Month,
    Alltime,
}

impl LeaderboardPeriod {
    /// Parse from the query-string value. Unknown values fall back to `Alltime`.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(str::to_ascii_lowercase).as_deref() {
            Some("week") | Some("weekly") | Some("7d") => Self::Week,
            Some("month") | Some("monthly") | Some("30d") => Self::Month,
            _ => Self::Alltime,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Week => "week",
            Self::Month => "month",
            Self::Alltime => "alltime",
        }
    }
}

impl std::fmt::Display for LeaderboardPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Windowed global XP leaderboard.
///
/// For `Alltime` this is the same precomputed read as `get_user_leaderboard`.
/// For `Week`/`Month` it aggregates `xp_ledger.amount` over the cutoff window
/// and then joins back to `community_profiles` for the user's current level
/// and login streak (those fields are properties of "now", not the window).
///
/// The cutoff window is `NOW() - INTERVAL '7 days'` for `Week` and `NOW() -
/// INTERVAL '30 days'` for `Month`, mirroring the investor leaderboard's
/// timeframe semantics. Users with zero ledger entries in the window are
/// excluded from windowed views (no point ranking idle users at 0 XP).
pub async fn get_user_leaderboard_for_period(
    pool: &PgPool,
    period: LeaderboardPeriod,
    limit: i64,
) -> Result<Vec<UserLeaderboardEntry>, AppError> {
    let limit = limit.clamp(1, 100);

    match period {
        LeaderboardPeriod::Alltime => get_user_leaderboard(pool, limit).await,
        LeaderboardPeriod::Week | LeaderboardPeriod::Month => {
            let cutoff = match period {
                LeaderboardPeriod::Week => "NOW() - INTERVAL '7 days'",
                LeaderboardPeriod::Month => "NOW() - INTERVAL '30 days'",
                LeaderboardPeriod::Alltime => unreachable!(),
            };
            let query = format!(
                r#"
                SELECT
                    agg.user_id                                       AS user_id,
                    GREATEST(agg.xp_window, 0)::INTEGER               AS xp_total,
                    COALESCE(cp.level, 1)                             AS level,
                    COALESCE(cp.level_name, 'Seedling')               AS level_name,
                    cp.circle_id                                      AS circle_id,
                    COALESCE(cp.login_streak, 0)                      AS login_streak
                FROM (
                    SELECT user_id, SUM(amount)::BIGINT AS xp_window
                    FROM xp_ledger
                    WHERE created_at >= {cutoff}
                    GROUP BY user_id
                ) agg
                LEFT JOIN community_profiles cp ON cp.user_id = agg.user_id
                WHERE agg.xp_window > 0
                ORDER BY xp_total DESC, agg.user_id ASC
                LIMIT $1
                "#,
                cutoff = cutoff,
            );
            let entries = sqlx::query_as::<_, UserLeaderboardEntry>(&query)
                .bind(limit)
                .fetch_all(pool)
                .await?;
            Ok(entries)
        }
    }
}

// ─── XP Aggregation Worker ──────────────────────────────────────────

/// Sync XP totals for all profiles and update circle XP.
/// Called by background worker every 5 minutes.
pub async fn aggregate_xp(pool: &PgPool) -> Result<(), AppError> {
    // 1. Sync user XP totals from ledger
    sqlx::query(
        r#"
        UPDATE community_profiles cp SET
            xp_total = COALESCE(agg.total, 0)
        FROM (
            SELECT user_id, SUM(amount)::INTEGER AS total
            FROM xp_ledger
            GROUP BY user_id
        ) agg
        WHERE cp.user_id = agg.user_id AND cp.xp_total != agg.total
        "#,
    )
    .execute(pool)
    .await?;

    // 2. Update levels for all users
    sqlx::query(
        r#"
        UPDATE community_profiles cp SET
            level = xl.level,
            level_name = xl.name
        FROM (
            SELECT DISTINCT ON (cp2.user_id)
                cp2.user_id,
                xl2.level,
                xl2.name
            FROM community_profiles cp2
            CROSS JOIN xp_levels xl2
            WHERE xl2.min_xp <= cp2.xp_total
            ORDER BY cp2.user_id, xl2.level DESC
        ) xl
        WHERE cp.user_id = xl.user_id AND (cp.level != xl.level OR cp.level_name != xl.name)
        "#,
    )
    .execute(pool)
    .await?;

    // 3. Aggregate circle XP
    sqlx::query(
        r#"
        UPDATE circles c SET
            total_xp = COALESCE(agg.circle_xp, 0),
            member_count = COALESCE(agg.cnt, 0),
            updated_at = NOW()
        FROM (
            SELECT cm.circle_id,
                   SUM(cp.xp_total)::BIGINT AS circle_xp,
                   COUNT(*)::INTEGER AS cnt
            FROM circle_members cm
            JOIN community_profiles cp ON cp.user_id = cm.user_id
            GROUP BY cm.circle_id
        ) agg
        WHERE c.id = agg.circle_id
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("XP aggregation complete");
    Ok(())
}

// ─── Login Streak Tracker (M4-BE.9) ─────────────────────────────────

/// Track daily login streak. Call on each successful login.
/// Awards: daily_login (5 XP), login_streak_7 (50 XP), login_streak_30 (200 XP).
pub async fn track_login_streak(pool: &PgPool, user_id: Uuid) -> Result<i32, AppError> {
    // Ensure profile exists
    sqlx::query(
        "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    // Get current streak state
    let row: Option<(i32, Option<chrono::NaiveDate>)> = sqlx::query_as(
        "SELECT login_streak, last_login_date FROM community_profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let (current_streak, last_login) = row.unwrap_or((0, None));
    let today = chrono::Utc::now().date_naive();

    // Already logged in today — no update needed
    if last_login == Some(today) {
        return Ok(current_streak);
    }

    let new_streak = if let Some(last) = last_login {
        let diff = (today - last).num_days();
        if diff == 1 {
            // Consecutive day — increment
            current_streak + 1
        } else {
            // Gap — reset to 1
            1
        }
    } else {
        // First login ever
        1
    };

    // Update profile
    sqlx::query(
        "UPDATE community_profiles SET login_streak = $1, last_login_date = $2 WHERE user_id = $3",
    )
    .bind(new_streak)
    .bind(today)
    .bind(user_id)
    .execute(pool)
    .await?;

    // Award daily login XP
    let _ = award_xp(
        pool,
        user_id,
        "daily_login",
        Some("Daily login bonus"),
        None,
    )
    .await;

    // Award streak milestones
    if new_streak == 7 {
        let _ = award_xp(
            pool,
            user_id,
            "login_streak_7",
            Some("7-day login streak!"),
            None,
        )
        .await;
    } else if new_streak == 30 {
        let _ = award_xp(
            pool,
            user_id,
            "login_streak_30",
            Some("30-day login streak!"),
            None,
        )
        .await;
    }

    // Award Gamification Challenge
    let _ =
        crate::community::challenges::increment_progress(pool, user_id, "login_streak", 1).await;

    tracing::debug!(user_id = %user_id, streak = new_streak, "Login streak updated");
    Ok(new_streak)
}

// ─── Level-Gated Feature Enforcement (M4-BE.10) ─────────────────────

/// Feature gates that require a minimum user level.
pub enum GatedFeature {
    CreatePost,        // Level 1 (anyone)
    CreateComment,     // Level 1
    GiveReaction,      // Level 1
    CreateCircle,      // Level 2
    InviteToCircle,    // Level 3
    MarketInsightPost, // Level 3
    ReviewPost,        // Level 4
}

impl GatedFeature {
    pub fn min_level(&self) -> i32 {
        match self {
            GatedFeature::CreatePost => 1,
            GatedFeature::CreateComment => 1,
            GatedFeature::GiveReaction => 1,
            GatedFeature::CreateCircle => 2,
            GatedFeature::InviteToCircle => 3,
            GatedFeature::MarketInsightPost => 3,
            GatedFeature::ReviewPost => 4,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GatedFeature::CreatePost => "create posts",
            GatedFeature::CreateComment => "comment",
            GatedFeature::GiveReaction => "react",
            GatedFeature::CreateCircle => "create a circle",
            GatedFeature::InviteToCircle => "invite to circle",
            GatedFeature::MarketInsightPost => "post market insights",
            GatedFeature::ReviewPost => "write reviews",
        }
    }
}

/// Check if a user's level meets the gate requirement.
/// Returns Ok(()) if allowed, Err if level is too low.
pub async fn check_level_gate(
    pool: &PgPool,
    user_id: Uuid,
    feature: GatedFeature,
) -> Result<(), AppError> {
    let user_level: i32 =
        sqlx::query_scalar("SELECT level FROM community_profiles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .unwrap_or(1);

    let required = feature.min_level();
    if user_level < required {
        return Err(AppError::BadRequest(format!(
            "Level {} required to {}. You are Level {}.",
            required,
            feature.label(),
            user_level
        )));
    }

    Ok(())
}
