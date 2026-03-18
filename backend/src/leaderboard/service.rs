use super::models::*;
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

/// Recompute scores for all active users and update `leaderboard_scores`.
/// This is designed to be called periodically (e.g., every 15 minutes).
pub async fn refresh_all_scores(pool: &PgPool) -> Result<(), AppError> {
    // 1. Compute and upsert 6 raw metrics for each user with active account.
    sqlx::query(
        r#"
        INSERT INTO leaderboard_scores (
            user_id, 
            total_invested_cents, 
            asset_count, 
            portfolio_roi_bps, 
            affiliate_count, 
            referral_revenue_cents, 
            highest_investment_cents,
            computed_at
        )
        SELECT
            u.id,
            COALESCE(inv_agg.total_invested, 0),
            COALESCE(inv_agg.unique_assets, 0),
            COALESCE(inv_agg.weighted_roi_bps, 0),
            COALESCE(ref_agg.aff_count, 0),
            COALESCE(ref_agg.network_value, 0),
            COALESCE(inv_agg.highest_inv, 0),
            NOW()
        FROM users u
        LEFT JOIN (
            SELECT 
                i.user_id, 
                SUM(i.purchase_value_cents) as total_invested, 
                COUNT(DISTINCT i.asset_id) as unique_assets,
                MAX(i.purchase_value_cents) as highest_inv,
                COALESCE(
                    (SUM(i.purchase_value_cents * COALESCE(a.annual_yield_bps, 0)) / NULLIF(SUM(i.purchase_value_cents), 0)), 
                0) as weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
            GROUP BY i.user_id
        ) inv_agg ON inv_agg.user_id = u.id
        LEFT JOIN (
            SELECT 
                rt.referrer_id, 
                COUNT(DISTINCT rt.referred_id) as aff_count,
                COALESCE(SUM(inv.purchase_value_cents), 0) as network_value
            FROM referral_tracking rt
            LEFT JOIN investments inv ON inv.user_id = rt.referred_id AND inv.status = 'active'
            GROUP BY rt.referrer_id
        ) ref_agg ON ref_agg.referrer_id = u.id
        WHERE u.status = 'active'
          AND (inv_agg.total_invested > 0 OR ref_agg.aff_count > 0)
        ON CONFLICT (user_id) DO UPDATE SET
            total_invested_cents = EXCLUDED.total_invested_cents,
            asset_count = EXCLUDED.asset_count,
            portfolio_roi_bps = EXCLUDED.portfolio_roi_bps,
            affiliate_count = EXCLUDED.affiliate_count,
            referral_revenue_cents = EXCLUDED.referral_revenue_cents,
            highest_investment_cents = EXCLUDED.highest_investment_cents,
            computed_at = NOW()
        "#,
    )
    .execute(pool)
    .await?;

    // 2. Assign ranks for all 6 metrics
    sqlx::query(
        r#"
        UPDATE leaderboard_scores ls SET 
            rank_invested = sub.r_inv,
            rank_assets = sub.r_ast,
            rank_roi = sub.r_roi,
            rank_affiliates = sub.r_aff,
            rank_ref_revenue = sub.r_rev,
            rank_highest_inv = sub.r_hi
        FROM (
            SELECT user_id, 
                ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC) as r_inv,
                ROW_NUMBER() OVER (ORDER BY asset_count DESC, total_invested_cents DESC, computed_at ASC) as r_ast,
                ROW_NUMBER() OVER (ORDER BY portfolio_roi_bps DESC, total_invested_cents DESC, computed_at ASC) as r_roi,
                ROW_NUMBER() OVER (ORDER BY affiliate_count DESC, referral_revenue_cents DESC, computed_at ASC) as r_aff,
                ROW_NUMBER() OVER (ORDER BY referral_revenue_cents DESC, affiliate_count DESC, computed_at ASC) as r_rev,
                ROW_NUMBER() OVER (ORDER BY highest_investment_cents DESC, computed_at ASC) as r_hi
            FROM leaderboard_scores
        ) sub WHERE ls.user_id = sub.user_id
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Leaderboard metrics and ranks refreshed successfully");
    Ok(())
}

/// Fetch the top N rankings for a given metric type.
pub async fn get_rankings(
    pool: &PgPool,
    current_user_id: Uuid,
    metric_type: &str,
    page: i64,
    per_page: i64,
    tier_id: Option<i32>,
    search: Option<String>,
) -> Result<LeaderboardResponse, AppError> {
    let offset = (page - 1) * per_page;

    let (rank_col, val_col) = match metric_type {
        "assets" => ("rank_assets", "asset_count"),
        "roi" => ("rank_roi", "portfolio_roi_bps"),
        "affiliates" => ("rank_affiliates", "affiliate_count"),
        "revenue" => ("rank_ref_revenue", "referral_revenue_cents"),
        "highest_inv" => ("rank_highest_inv", "highest_investment_cents"),
        _ => ("rank_invested", "total_invested_cents"), // default
    };

    // Fetch rankings with user profile data + preferences
    let query = format!(
        r#"
        WITH raw_data AS (
            SELECT
                ls.{rank_col} as rank,
                ls.{val_col} as metric_value,
                ls.total_invested_cents,
                ls.asset_count,
                ls.portfolio_roi_bps,
                ls.affiliate_count,
                ls.referral_revenue_cents,
                ls.highest_investment_cents,
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
        val_col = val_col,
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
                metric_value: r.get::<Option<i64>, _>("metric_value").unwrap_or(0),
                is_current_user: is_current,
                metrics: LeaderboardMetrics {
                    total_invested_cents: r.get::<Option<i64>, _>("total_invested_cents").unwrap_or(0),
                    asset_count: r.get::<Option<i32>, _>("asset_count").unwrap_or(0),
                    portfolio_roi_bps: r.get::<Option<i32>, _>("portfolio_roi_bps").unwrap_or(0),
                    affiliate_count: r.get::<Option<i32>, _>("affiliate_count").unwrap_or(0),
                    referral_revenue_cents: r.get::<Option<i64>, _>("referral_revenue_cents").unwrap_or(0),
                    highest_investment_cents: r.get::<Option<i64>, _>("highest_investment_cents").unwrap_or(0),
                },
            }
        })
        .collect();

    // Total participants
    let total_participants: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM leaderboard_scores")
            .fetch_one(pool)
            .await?;

    // Get my rank
    let my_rank = get_my_rank_inner(pool, current_user_id, rank_col, val_col).await?;

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
        metric_type: metric_type.to_string(),
        last_updated,
        has_more,
    })
}

/// Get the current user's rank and score breakdown.
async fn get_my_rank_inner(
    pool: &PgPool,
    user_id: Uuid,
    rank_col: &str,
    val_col: &str,
) -> Result<MyRank, AppError> {
    let query = format!(
        "SELECT {rank_col} as rank, {val_col} as metric_value,
         total_invested_cents, asset_count, portfolio_roi_bps, affiliate_count, referral_revenue_cents, highest_investment_cents
         FROM leaderboard_scores WHERE user_id = $1",
        rank_col = rank_col,
        val_col = val_col,
    );

    let row = sqlx::query(&query)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    use sqlx::Row;
    match row {
        Some(r) => {
            Ok(MyRank {
                rank: r.get("rank"),
                metric_value: r.get::<Option<i64>, _>("metric_value").unwrap_or(0),
                delta_weekly: 0, // removed snapshot logic as multi-metric snapshot is unsupported
                metrics: LeaderboardMetrics {
                    total_invested_cents: r.get::<Option<i64>, _>("total_invested_cents").unwrap_or(0),
                    asset_count: r.get::<Option<i32>, _>("asset_count").unwrap_or(0),
                    portfolio_roi_bps: r.get::<Option<i32>, _>("portfolio_roi_bps").unwrap_or(0),
                    affiliate_count: r.get::<Option<i32>, _>("affiliate_count").unwrap_or(0),
                    referral_revenue_cents: r.get::<Option<i64>, _>("referral_revenue_cents").unwrap_or(0),
                    highest_investment_cents: r.get::<Option<i64>, _>("highest_investment_cents").unwrap_or(0),
                },
            })
        }
        None => Ok(MyRank {
            rank: None,
            metric_value: 0,
            delta_weekly: 0,
            metrics: LeaderboardMetrics {
                total_invested_cents: 0,
                asset_count: 0,
                portfolio_roi_bps: 0,
                affiliate_count: 0,
                referral_revenue_cents: 0,
                highest_investment_cents: 0,
            },
        }),
    }
}

/// Get the user's rank directly (public-facing helper).
pub async fn get_user_rank(
    pool: &PgPool,
    user_id: Uuid,
    metric_type: &str,
) -> Result<MyRank, AppError> {
    let (rank_col, val_col) = match metric_type {
        "assets" => ("rank_assets", "asset_count"),
        "roi" => ("rank_roi", "portfolio_roi_bps"),
        "affiliates" => ("rank_affiliates", "affiliate_count"),
        "revenue" => ("rank_ref_revenue", "referral_revenue_cents"),
        "highest_inv" => ("rank_highest_inv", "highest_investment_cents"),
        _ => ("rank_invested", "total_invested_cents"), // default
    };
    get_my_rank_inner(pool, user_id, rank_col, val_col).await
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
