use super::models::*;
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

/// Recompute scores for all active users and update `leaderboard_scores`.
/// This is designed to be called periodically (e.g., every 15 minutes).
pub async fn refresh_all_scores(pool: &PgPool) -> Result<(), AppError> {
    // 1. Get max values across all users for normalisation
    let max_invested: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(total), 0)::BIGINT FROM (
            SELECT SUM(purchase_value_cents) as total FROM investments WHERE status = 'active' GROUP BY user_id
        ) sub",
    )
    .fetch_one(pool)
    .await?;

    let max_network: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(net_total), 0)::BIGINT FROM (
            SELECT rt.referrer_id, COALESCE(SUM(inv.purchase_value_cents), 0) as net_total
            FROM referral_tracking rt
            JOIN investments inv ON inv.user_id = rt.referred_id AND inv.status = 'active'
            GROUP BY rt.referrer_id
        ) sub",
    )
    .fetch_one(pool)
    .await?;

    let max_diversity: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(cnt), 0)::BIGINT FROM (
            SELECT COUNT(DISTINCT asset_id) as cnt FROM investments WHERE status = 'active' GROUP BY user_id
        ) sub",
    )
    .fetch_one(pool)
    .await?;

    // Max tier sort_order (should be 5 for Premium)
    let max_tier: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 5)::BIGINT FROM tiers")
            .fetch_one(pool)
            .await?;

    // 2. Compute and upsert scores for each user with active investments
    // Use a single SQL statement for efficiency
    sqlx::query(
        r#"
        INSERT INTO leaderboard_scores (user_id, invest_score, referral_score, tier_score, diversity_score, total_score, computed_at)
        SELECT
            u.id,
            -- invest_score: (user_invested / max_invested) * 1000
            CASE WHEN $1::BIGINT > 0 THEN LEAST(1000, (COALESCE(inv_agg.total_invested, 0) * 1000 / $1::BIGINT)::INTEGER) ELSE 0 END,
            -- referral_score: (user_network / max_network) * 1000
            CASE WHEN $2::BIGINT > 0 THEN LEAST(1000, (COALESCE(ref_agg.network_value, 0) * 1000 / $2::BIGINT)::INTEGER) ELSE 0 END,
            -- tier_score: (tier_sort / max_tier) * 1000
            CASE WHEN $4::BIGINT > 0 THEN LEAST(1000, (COALESCE(t.sort_order, 1) * 1000 / $4::BIGINT)::INTEGER) ELSE 200 END,
            -- diversity_score: (unique_assets / max_assets) * 1000
            CASE WHEN $3::BIGINT > 0 THEN LEAST(1000, (COALESCE(inv_agg.unique_assets, 0) * 1000 / $3::BIGINT)::INTEGER) ELSE 0 END,
            -- total_score: weighted composite
            (
                CASE WHEN $1::BIGINT > 0 THEN LEAST(1000, (COALESCE(inv_agg.total_invested, 0) * 1000 / $1::BIGINT)::INTEGER) ELSE 0 END * 40 +
                CASE WHEN $2::BIGINT > 0 THEN LEAST(1000, (COALESCE(ref_agg.network_value, 0) * 1000 / $2::BIGINT)::INTEGER) ELSE 0 END * 25 +
                CASE WHEN $4::BIGINT > 0 THEN LEAST(1000, (COALESCE(t.sort_order, 1) * 1000 / $4::BIGINT)::INTEGER) ELSE 200 END * 20 +
                CASE WHEN $3::BIGINT > 0 THEN LEAST(1000, (COALESCE(inv_agg.unique_assets, 0) * 1000 / $3::BIGINT)::INTEGER) ELSE 0 END * 15
            ) / 100,
            NOW()
        FROM users u
        LEFT JOIN (
            SELECT user_id, SUM(purchase_value_cents) as total_invested, COUNT(DISTINCT asset_id) as unique_assets
            FROM investments WHERE status = 'active'
            GROUP BY user_id
        ) inv_agg ON inv_agg.user_id = u.id
        LEFT JOIN (
            SELECT rt.referrer_id, COALESCE(SUM(inv.purchase_value_cents), 0) as network_value
            FROM referral_tracking rt
            JOIN investments inv ON inv.user_id = rt.referred_id AND inv.status = 'active'
            GROUP BY rt.referrer_id
        ) ref_agg ON ref_agg.referrer_id = u.id
        LEFT JOIN user_tiers ut ON ut.user_id = u.id
        LEFT JOIN tiers t ON t.id = ut.tier_id
        WHERE u.status = 'active'
          AND (inv_agg.total_invested IS NOT NULL AND inv_agg.total_invested > 0)
        ON CONFLICT (user_id) DO UPDATE SET
            invest_score = EXCLUDED.invest_score,
            referral_score = EXCLUDED.referral_score,
            tier_score = EXCLUDED.tier_score,
            diversity_score = EXCLUDED.diversity_score,
            total_score = EXCLUDED.total_score,
            computed_at = NOW()
        "#,
    )
    .bind(max_invested)
    .bind(max_network)
    .bind(max_diversity)
    .bind(max_tier)
    .execute(pool)
    .await?;

    // 3. Assign ranks (all-time)
    sqlx::query(
        "UPDATE leaderboard_scores ls SET rank_alltime = sub.rn FROM (
            SELECT user_id, ROW_NUMBER() OVER (ORDER BY total_score DESC, invest_score DESC, computed_at ASC) as rn
            FROM leaderboard_scores
        ) sub WHERE ls.user_id = sub.user_id",
    )
    .execute(pool)
    .await?;

    tracing::info!("Leaderboard scores refreshed successfully");
    Ok(())
}

/// Fetch the top N rankings for a given timeframe.
pub async fn get_rankings(
    pool: &PgPool,
    current_user_id: Uuid,
    timeframe: &str,
    page: i64,
    per_page: i64,
    tier_id: Option<i32>,
    search: Option<String>,
) -> Result<LeaderboardResponse, AppError> {
    let offset = (page - 1) * per_page;

    let rank_col = match timeframe {
        "weekly" => "rank_weekly",
        "monthly" => "rank_monthly",
        _ => "rank_alltime",
    };

    // Fetch rankings with user profile data + preferences
    let query = format!(
        r#"
        WITH raw_data AS (
            SELECT
                ls.{rank_col} as rank,
                ls.total_score,
                ls.invest_score,
                ls.referral_score,
                ls.tier_score,
                ls.diversity_score,
                ls.user_id,
                u.avatar_url,
                COALESCE(t.name, 'Intro') as tier_name,
                COALESCE(t.badge_color, '#D0D5DD') as tier_badge_color,
                COALESCE(lp.visible, false) as is_visible,
                COALESCE(lp.show_avatar, false) as show_avatar_pref,
                CASE 
                    WHEN COALESCE(lp.visible, false) OR ls.user_id = $5 THEN COALESCE(lp.display_name, up.display_name, COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor'))
                    ELSE 'Investor #' || substring(ls.user_id::text from 1 for 6)
                END as full_name,
                ut.tier_id
            FROM leaderboard_scores ls
            JOIN users u ON u.id = ls.user_id
            LEFT JOIN user_profiles up ON up.user_id = ls.user_id
            LEFT JOIN user_tiers ut ON ut.user_id = ls.user_id
            LEFT JOIN tiers t ON t.id = ut.tier_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
            WHERE ls.{rank_col} IS NOT NULL
        )
        SELECT * FROM raw_data
        WHERE ($3::int IS NULL OR tier_id = $3::int)
          AND ($4::text IS NULL OR full_name ILIKE '%' || $4::text || '%')
        ORDER BY rank ASC
        LIMIT $1 OFFSET $2
        "#,
        rank_col = rank_col,
    );

    let rows = sqlx::query(&query)
        .bind(per_page)
        .bind(offset)
        .bind(tier_id)
        .bind(search)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?;

    use sqlx::Row;
    let rankings: Vec<LeaderboardEntry> = rows
        .iter()
        .map(|r| {
            let user_id: Uuid = r.get("user_id");
            let is_visible: bool = r.get("is_visible");
            let show_avatar_pref: bool = r.get("show_avatar_pref");
            let is_current = user_id == current_user_id;

            let display_name = r.get::<String, _>("full_name");

            let avatar_url = if (is_visible && show_avatar_pref) || is_current {
                r.get::<Option<String>, _>("avatar_url")
            } else {
                None
            };

            LeaderboardEntry {
                rank: r.get::<Option<i32>, _>("rank").unwrap_or(0),
                display_name,
                avatar_url,
                tier_name: r.get("tier_name"),
                tier_badge_color: r.get("tier_badge_color"),
                total_score: r.get::<Option<i32>, _>("total_score").unwrap_or(0),
                is_current_user: is_current,
                score_breakdown: ScoreBreakdown {
                    invest_score: r.get::<Option<i32>, _>("invest_score").unwrap_or(0),
                    referral_score: r.get::<Option<i32>, _>("referral_score").unwrap_or(0),
                    tier_score: r.get::<Option<i32>, _>("tier_score").unwrap_or(0),
                    diversity_score: r.get::<Option<i32>, _>("diversity_score").unwrap_or(0),
                },
            }
        })
        .collect();

    // Total participants
    let total_participants: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM leaderboard_scores WHERE total_score > 0")
            .fetch_one(pool)
            .await?;

    // Get my rank
    let my_rank = get_my_rank_inner(pool, current_user_id, rank_col).await?;

    // Last updated timestamp
    let last_updated: Option<String> = sqlx::query_scalar(
        "SELECT to_char(MAX(computed_at), 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') FROM leaderboard_scores",
    )
    .fetch_one(pool)
    .await?;

    let has_more = rankings.len() as i64 == per_page;

    Ok(LeaderboardResponse {
        rankings,
        my_rank,
        total_participants,
        timeframe: timeframe.to_string(),
        last_updated,
        has_more,
    })
}

/// Get the current user's rank and score breakdown.
async fn get_my_rank_inner(
    pool: &PgPool,
    user_id: Uuid,
    rank_col: &str,
) -> Result<MyRank, AppError> {
    let query = format!(
        "SELECT {rank_col} as rank, total_score, invest_score, referral_score, tier_score, diversity_score
         FROM leaderboard_scores WHERE user_id = $1",
        rank_col = rank_col,
    );

    let row = sqlx::query(&query)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    use sqlx::Row;
    match row {
        Some(r) => {
            // Calculate weekly delta from snapshots
            let prev_rank: Option<i32> = sqlx::query_scalar(
                "SELECT rank_position FROM leaderboard_snapshots
                 WHERE user_id = $1 AND snapshot_type = 'weekly'
                 ORDER BY snapshot_date DESC LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

            let current_rank = r.get::<Option<i32>, _>("rank").unwrap_or(0);
            let delta = match prev_rank {
                Some(prev) => prev - current_rank, // positive = moved up
                None => 0,
            };

            Ok(MyRank {
                rank: r.get("rank"),
                total_score: r.get::<Option<i32>, _>("total_score").unwrap_or(0),
                delta_weekly: delta,
                score_breakdown: ScoreBreakdown {
                    invest_score: r.get::<Option<i32>, _>("invest_score").unwrap_or(0),
                    referral_score: r.get::<Option<i32>, _>("referral_score").unwrap_or(0),
                    tier_score: r.get::<Option<i32>, _>("tier_score").unwrap_or(0),
                    diversity_score: r.get::<Option<i32>, _>("diversity_score").unwrap_or(0),
                },
            })
        }
        None => Ok(MyRank {
            rank: None,
            total_score: 0,
            delta_weekly: 0,
            score_breakdown: ScoreBreakdown {
                invest_score: 0,
                referral_score: 0,
                tier_score: 0,
                diversity_score: 0,
            },
        }),
    }
}

/// Get the user's rank directly (public-facing helper).
pub async fn get_user_rank(
    pool: &PgPool,
    user_id: Uuid,
    timeframe: &str,
) -> Result<MyRank, AppError> {
    let rank_col = match timeframe {
        "weekly" => "rank_weekly",
        "monthly" => "rank_monthly",
        _ => "rank_alltime",
    };
    get_my_rank_inner(pool, user_id, rank_col).await
}

/// Get leaderboard preferences for a user.
pub async fn get_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<LeaderboardPreferences, AppError> {
    let row = sqlx::query!(
        "SELECT visible, show_avatar, display_name FROM leaderboard_preferences WHERE user_id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(r) => LeaderboardPreferences {
            visible: r.visible,
            show_avatar: r.show_avatar,
            display_name: r.display_name,
        },
        None => LeaderboardPreferences {
            visible: false,
            show_avatar: false,
            display_name: None,
        },
    })
}

/// Update leaderboard preferences for a user.
pub async fn update_preferences(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdatePreferencesRequest,
) -> Result<LeaderboardPreferences, AppError> {
    let visible = req.visible.unwrap_or(false);
    let show_avatar = req.show_avatar.unwrap_or(false);
    let display_name = req.display_name.as_deref().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    sqlx::query!(
        r#"
        INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            visible = $2,
            show_avatar = $3,
            display_name = $4,
            updated_at = NOW()
        "#,
        user_id,
        visible,
        show_avatar,
        display_name,
    )
    .execute(pool)
    .await?;

    Ok(LeaderboardPreferences {
        visible,
        show_avatar,
        display_name,
    })
}
