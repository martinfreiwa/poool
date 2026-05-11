use super::models::*;
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

// ─── Timeframe Helpers ─────────────────────────────────────────────
/// Returns the SQL interval clause for a given timeframe string.
/// "weekly"  → "NOW() - INTERVAL '7 days'"
/// "monthly" → "NOW() - INTERVAL '30 days'"
/// _         → None (all-time)
fn timeframe_cutoff_sql(timeframe: &str) -> Option<&'static str> {
    match timeframe {
        "weekly" => Some("NOW() - INTERVAL '7 days'"),
        "monthly" => Some("NOW() - INTERVAL '30 days'"),
        _ => None, // "alltime" or default
    }
}

// ─── Background Refresh ────────────────────────────────────────────

/// Recompute ALL-TIME metrics for every active user and upsert into `leaderboard_scores`.
/// Called periodically by the background worker (every 15 minutes).
///
/// This function does NOT handle timeframe filtering — that is done at query time
/// in `get_rankings()`. This function only materializes *all-time* raw metrics
/// so that the read path can cheaply JOIN against them.
pub async fn refresh_all_scores(pool: &PgPool) -> Result<(), AppError> {
    // Step 0: Remove ghost entries — users who no longer have any active investments
    //         AND no referral activity. This ensures the leaderboard doesn't show
    //         stale data for users who sold everything.
    sqlx::query(
        r#"
        DELETE FROM leaderboard_scores
        WHERE user_id NOT IN (
            SELECT DISTINCT user_id FROM investments WHERE status = 'active'
            UNION
            SELECT DISTINCT referrer_id FROM referral_tracking
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Step 0b: Remove rows for users whose account is no longer active
    //          (suspended, deleted, frozen, etc.). They may still own active
    //          investments, but they should not appear in public rankings.
    sqlx::query(
        r#"
        DELETE FROM leaderboard_scores
        WHERE user_id IN (
            SELECT id FROM users WHERE status != 'active'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Step 1: Upsert fresh all-time metrics for every user with activity.
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
            COALESCE(inv_agg.unique_assets, 0)::INTEGER,
            COALESCE(inv_agg.weighted_roi_bps, 0)::INTEGER,
            COALESCE(ref_agg.aff_count, 0)::INTEGER,
            COALESCE(ref_agg.network_value, 0),
            COALESCE(inv_agg.highest_inv, 0),
            NOW()
        FROM users u
        LEFT JOIN (
            SELECT
                i.user_id,
                SUM(i.purchase_value_cents)              AS total_invested,
                COUNT(DISTINCT i.asset_id)               AS unique_assets,
                MAX(i.purchase_value_cents)               AS highest_inv,
                -- Weighted basis-point ROI. Cast to NUMERIC before the multiply
                -- so a 100k * 450 bps investment is not truncated by integer
                -- division when the denominator (SUM purchase_value_cents) is
                -- large. ROUND the final result to a whole bps integer for
                -- storage; sub-bps precision is irrelevant for ranking.
                COALESCE(
                    ROUND(
                        SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                        / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                    ),
                    0
                )                                         AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
            GROUP BY i.user_id
        ) inv_agg ON inv_agg.user_id = u.id
        LEFT JOIN (
            SELECT
                rt.referrer_id,
                COUNT(DISTINCT rt.referred_id)            AS aff_count,
                COALESCE(SUM(inv.purchase_value_cents), 0) AS network_value
            FROM referral_tracking rt
            LEFT JOIN investments inv ON inv.user_id = rt.referred_id AND inv.status = 'active'
            GROUP BY rt.referrer_id
        ) ref_agg ON ref_agg.referrer_id = u.id
        WHERE u.status = 'active'
          AND (inv_agg.total_invested > 0 OR ref_agg.aff_count > 0)
        ON CONFLICT (user_id) DO UPDATE SET
            total_invested_cents      = EXCLUDED.total_invested_cents,
            asset_count               = EXCLUDED.asset_count,
            portfolio_roi_bps         = EXCLUDED.portfolio_roi_bps,
            affiliate_count           = EXCLUDED.affiliate_count,
            referral_revenue_cents    = EXCLUDED.referral_revenue_cents,
            highest_investment_cents  = EXCLUDED.highest_investment_cents,
            computed_at               = NOW()
        "#,
    )
    .execute(pool)
    .await?;

    // Step 2: Assign all-time ranks (these are fast precomputed ranks for the default view).
    sqlx::query(
        r#"
        UPDATE leaderboard_scores ls SET
            rank_invested    = sub.r_inv,
            rank_assets      = sub.r_ast,
            rank_roi         = sub.r_roi,
            rank_affiliates  = sub.r_aff,
            rank_ref_revenue = sub.r_rev,
            rank_highest_inv = sub.r_hi
        FROM (
            SELECT user_id,
                ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC)                     AS r_inv,
                ROW_NUMBER() OVER (ORDER BY asset_count DESC, total_invested_cents DESC, computed_at ASC)    AS r_ast,
                ROW_NUMBER() OVER (ORDER BY portfolio_roi_bps DESC, total_invested_cents DESC, computed_at ASC) AS r_roi,
                ROW_NUMBER() OVER (ORDER BY affiliate_count DESC, referral_revenue_cents DESC, computed_at ASC) AS r_aff,
                ROW_NUMBER() OVER (ORDER BY referral_revenue_cents DESC, affiliate_count DESC, computed_at ASC) AS r_rev,
                ROW_NUMBER() OVER (ORDER BY highest_investment_cents DESC, computed_at ASC)                  AS r_hi
            FROM leaderboard_scores
        ) sub WHERE ls.user_id = sub.user_id
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Leaderboard: refreshed metrics and ranks for all-time view");
    Ok(())
}

// ─── Read Path ─────────────────────────────────────────────────────

/// Resolve (rank_column, value_column) pair from metric key string.
fn metric_columns(metric_type: &str) -> (&str, &str) {
    match metric_type {
        "assets" => ("rank_assets", "asset_count"),
        "roi" => ("rank_roi", "portfolio_roi_bps"),
        "affiliates" => ("rank_affiliates", "affiliate_count"),
        "revenue" => ("rank_ref_revenue", "referral_revenue_cents"),
        "highest_inv" => ("rank_highest_inv", "highest_investment_cents"),
        _ => ("rank_invested", "total_invested_cents"), // default: "invested"
    }
}

/// Fetch the leaderboard rankings.
///
/// Supports two modes:
///   - **All-time** (`timeframe == "alltime"` or absent): reads directly from
///     the precomputed `leaderboard_scores` table — instant.
///   - **Weekly/Monthly** (`timeframe == "weekly" | "monthly"`): computes
///     time-filtered metrics at query time by aggregating only investments
///     purchased within the timeframe window, then ranking inline with ROW_NUMBER.
#[allow(clippy::too_many_arguments)]
pub async fn get_rankings(
    pool: &PgPool,
    current_user_id: Uuid,
    metric_type: &str,
    timeframe: &str,
    page: i64,
    per_page: i64,
    tier_id: Option<i32>,
    search: Option<String>,
) -> Result<LeaderboardResponse, AppError> {
    let offset = (page - 1) * per_page;

    let (rankings, my_rank, total_participants, last_updated) =
        if let Some(cutoff) = timeframe_cutoff_sql(timeframe) {
            // ── Timeframe-filtered: compute metrics at query time ──
            get_rankings_timeframed(
                pool,
                current_user_id,
                metric_type,
                cutoff,
                per_page,
                offset,
                tier_id,
                search.as_deref(),
            )
            .await?
        } else {
            // ── All-time: read from precomputed table ──
            get_rankings_alltime(
                pool,
                current_user_id,
                metric_type,
                per_page,
                offset,
                tier_id,
                search.as_deref(),
            )
            .await?
        };

    let has_more = rankings.len() as i64 == per_page;

    Ok(LeaderboardResponse {
        rankings,
        my_rank,
        total_participants,
        metric_type: metric_type.to_string(),
        timeframe: timeframe.to_string(),
        last_updated,
        has_more,
    })
}

/// All-time rankings: read directly from precomputed `leaderboard_scores`.
#[allow(clippy::too_many_arguments)]
async fn get_rankings_alltime(
    pool: &PgPool,
    current_user_id: Uuid,
    metric_type: &str,
    limit: i64,
    offset: i64,
    tier_id: Option<i32>,
    search: Option<&str>,
) -> Result<(Vec<LeaderboardEntry>, MyRank, i64, Option<String>), AppError> {
    let (rank_col, val_col) = metric_columns(metric_type);

    let query = format!(
        r#"
        WITH raw_data AS (
            SELECT
                ls.{rank_col}                           AS rank,
                ls.{val_col}::BIGINT                    AS metric_value,
                ls.total_invested_cents,
                ls.asset_count,
                ls.portfolio_roi_bps,
                ls.affiliate_count,
                ls.referral_revenue_cents,
                ls.highest_investment_cents,
                ls.user_id,
                u.avatar_url,
                COALESCE(t.name, 'Intro')               AS tier_name,
                COALESCE(t.badge_color, '#D0D5DD')      AS tier_badge_color,
                COALESCE(lp.visible, false)              AS is_visible,
                COALESCE(lp.show_avatar, false)          AS show_avatar_pref,
                CASE
                    WHEN COALESCE(lp.visible, false) OR ls.user_id = $5
                    THEN COALESCE(
                        lp.display_name,
                        up.display_name,
                        COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                    )
                    ELSE 'Investor #' || substring(ls.user_id::text from 1 for 6)
                END                                     AS full_name,
                ut.tier_id
            FROM leaderboard_scores ls
            JOIN users u ON u.id = ls.user_id
            LEFT JOIN user_profiles up   ON up.user_id = ls.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = ls.user_id
            LEFT JOIN tiers t            ON t.id = ut.tier_id
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
        .bind(limit)
        .bind(offset)
        .bind(tier_id)
        .bind(search)
        .bind(current_user_id)
        .fetch_all(pool)
        .await?;

    let rankings = rows_to_entries(&rows, current_user_id);

    let count_query = format!(
        r#"
        WITH raw_data AS (
            SELECT
                ls.user_id,
                COALESCE(lp.visible, false)              AS is_visible,
                CASE
                    WHEN COALESCE(lp.visible, false) OR ls.user_id = $3
                    THEN COALESCE(
                        lp.display_name,
                        up.display_name,
                        COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                    )
                    ELSE 'Investor #' || substring(ls.user_id::text from 1 for 6)
                END                                     AS full_name,
                ut.tier_id
            FROM leaderboard_scores ls
            JOIN users u ON u.id = ls.user_id
            LEFT JOIN user_profiles up   ON up.user_id = ls.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = ls.user_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
            WHERE ls.{rank_col} IS NOT NULL
        )
        SELECT COUNT(*)::BIGINT FROM raw_data
        WHERE ($1::int IS NULL OR tier_id = $1::int)
          AND ($2::text IS NULL OR full_name ILIKE '%' || $2::text || '%')
        "#,
        rank_col = rank_col,
    );
    let total_participants: i64 = sqlx::query_scalar(&count_query)
        .bind(tier_id)
        .bind(search)
        .bind(current_user_id)
        .fetch_one(pool)
        .await?;

    // My rank
    let my_rank = get_my_rank_alltime(pool, current_user_id, rank_col, val_col).await?;

    // Last updated
    let last_updated: Option<String> = sqlx::query_scalar(
        r#"SELECT to_char(MAX(computed_at), 'YYYY-MM-DD"T"HH24:MI:SS"Z"') FROM leaderboard_scores"#,
    )
    .fetch_one(pool)
    .await?;

    Ok((rankings, my_rank, total_participants, last_updated))
}

/// Timeframe-filtered rankings: compute metrics at query time.
/// This aggregates only investments purchased within the timeframe window,
/// then ranks them inline with ROW_NUMBER.
#[allow(clippy::too_many_arguments)]
async fn get_rankings_timeframed(
    pool: &PgPool,
    current_user_id: Uuid,
    metric_type: &str,
    cutoff_sql: &str, // e.g. "NOW() - INTERVAL '7 days'"
    limit: i64,
    offset: i64,
    tier_id: Option<i32>,
    search: Option<&str>,
) -> Result<(Vec<LeaderboardEntry>, MyRank, i64, Option<String>), AppError> {
    // Map metric_type to the ORDER BY expression used for ranking.
    let (order_expr, val_expr) = match metric_type {
        "assets" => ("unique_assets DESC, total_invested DESC", "unique_assets"),
        "roi" => (
            "weighted_roi_bps DESC, total_invested DESC",
            "weighted_roi_bps",
        ),
        "affiliates" => ("aff_count DESC, network_value DESC", "aff_count"),
        "revenue" => ("network_value DESC, aff_count DESC", "network_value"),
        "highest_inv" => ("highest_inv DESC", "highest_inv"),
        _ => ("total_invested DESC", "total_invested"), // default "invested"
    };

    // Build the main query. We compute metrics from the raw source tables,
    // filtered by the timeframe cutoff, then rank inline.
    let query = format!(
        r#"
        WITH inv_agg AS (
            SELECT
                i.user_id,
                SUM(i.purchase_value_cents)::BIGINT              AS total_invested,
                COUNT(DISTINCT i.asset_id)               AS unique_assets,
                MAX(i.purchase_value_cents)               AS highest_inv,
                -- Weighted bps ROI in NUMERIC space; see refresh_all_scores
                -- for the precision rationale. Rounded to whole bps.
                COALESCE(
                    ROUND(
                        SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                        / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                    ),
                    0
                )                                         AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
              AND i.purchased_at >= {cutoff}
            GROUP BY i.user_id
        ),
        ref_agg AS (
            SELECT
                rt.referrer_id,
                COUNT(DISTINCT rt.referred_id)            AS aff_count,
                COALESCE(SUM(inv.purchase_value_cents)::BIGINT, 0::BIGINT) AS network_value
            FROM referral_tracking rt
            LEFT JOIN investments inv ON inv.user_id = rt.referred_id
                AND inv.status = 'active'
                AND inv.purchased_at >= {cutoff}
            WHERE rt.created_at >= {cutoff}
            GROUP BY rt.referrer_id
        ),
        merged AS (
            SELECT
                COALESCE(ia.user_id, ra.referrer_id)     AS user_id,
                COALESCE(ia.total_invested, 0)           AS total_invested,
                COALESCE(ia.unique_assets, 0)::INT       AS unique_assets,
                COALESCE(ia.weighted_roi_bps, 0)::INT    AS weighted_roi_bps,
                COALESCE(ia.highest_inv, 0)              AS highest_inv,
                COALESCE(ra.aff_count, 0)::INT           AS aff_count,
                COALESCE(ra.network_value, 0)            AS network_value
            FROM inv_agg ia
            FULL OUTER JOIN ref_agg ra ON ia.user_id = ra.referrer_id
            WHERE COALESCE(ia.total_invested, 0) > 0 OR COALESCE(ra.aff_count, 0) > 0
        ),
        ranked AS (
            SELECT
                m.*,
                ROW_NUMBER() OVER (ORDER BY {order_expr})::INT  AS rank,
                {val_expr}::BIGINT                         AS metric_value
            FROM merged m
        ),
        enriched AS (
            SELECT
                r.rank,
                r.metric_value,
                r.total_invested    AS total_invested_cents,
                r.unique_assets     AS asset_count,
                r.weighted_roi_bps  AS portfolio_roi_bps,
                r.aff_count         AS affiliate_count,
                r.network_value     AS referral_revenue_cents,
                r.highest_inv       AS highest_investment_cents,
                r.user_id,
                u.avatar_url,
                COALESCE(t.name, 'Intro')               AS tier_name,
                COALESCE(t.badge_color, '#D0D5DD')       AS tier_badge_color,
                COALESCE(lp.visible, false)              AS is_visible,
                COALESCE(lp.show_avatar, false)           AS show_avatar_pref,
                CASE
                    WHEN COALESCE(lp.visible, false) OR r.user_id = $3
                    THEN COALESCE(
                        lp.display_name,
                        up.display_name,
                        COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                    )
                    ELSE 'Investor #' || substring(r.user_id::text from 1 for 6)
                END                                      AS full_name,
                ut.tier_id
            FROM ranked r
            JOIN users u ON u.id = r.user_id
            LEFT JOIN user_profiles up   ON up.user_id = r.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = r.user_id
            LEFT JOIN tiers t            ON t.id = ut.tier_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = r.user_id
        )
        SELECT * FROM enriched
        WHERE ($4::int IS NULL OR tier_id = $4::int)
          AND ($5::text IS NULL OR full_name ILIKE '%' || $5::text || '%')
        ORDER BY rank ASC
        LIMIT $1 OFFSET $2
        "#,
        cutoff = cutoff_sql,
        order_expr = order_expr,
        val_expr = val_expr,
    );

    let rows = sqlx::query(&query)
        .bind(limit)
        .bind(offset)
        .bind(current_user_id)
        .bind(tier_id)
        .bind(search)
        .fetch_all(pool)
        .await?;

    let rankings = rows_to_entries(&rows, current_user_id);

    let count_query = format!(
        r#"
        WITH inv_agg AS (
            SELECT
                i.user_id,
                SUM(i.purchase_value_cents)::BIGINT              AS total_invested,
                COUNT(DISTINCT i.asset_id)               AS unique_assets,
                MAX(i.purchase_value_cents)               AS highest_inv,
                -- See refresh_all_scores for precision rationale.
                COALESCE(
                    ROUND(
                        SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                        / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                    ),
                    0
                )                                         AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active'
              AND i.purchased_at >= {cutoff}
            GROUP BY i.user_id
        ),
        ref_agg AS (
            SELECT
                rt.referrer_id,
                COUNT(DISTINCT rt.referred_id)            AS aff_count,
                COALESCE(SUM(inv.purchase_value_cents)::BIGINT, 0::BIGINT) AS network_value
            FROM referral_tracking rt
            LEFT JOIN investments inv ON inv.user_id = rt.referred_id
                AND inv.status = 'active'
                AND inv.purchased_at >= {cutoff}
            WHERE rt.created_at >= {cutoff}
            GROUP BY rt.referrer_id
        ),
        merged AS (
            SELECT
                COALESCE(ia.user_id, ra.referrer_id)     AS user_id,
                COALESCE(ia.total_invested, 0)           AS total_invested,
                COALESCE(ia.unique_assets, 0)::INT       AS unique_assets,
                COALESCE(ia.weighted_roi_bps, 0)::INT    AS weighted_roi_bps,
                COALESCE(ia.highest_inv, 0)              AS highest_inv,
                COALESCE(ra.aff_count, 0)::INT           AS aff_count,
                COALESCE(ra.network_value, 0)            AS network_value
            FROM inv_agg ia
            FULL OUTER JOIN ref_agg ra ON ia.user_id = ra.referrer_id
            WHERE COALESCE(ia.total_invested, 0) > 0 OR COALESCE(ra.aff_count, 0) > 0
        ),
        ranked AS (
            SELECT
                m.*,
                ROW_NUMBER() OVER (ORDER BY {order_expr})::INT  AS rank,
                {val_expr}::BIGINT                         AS metric_value
            FROM merged m
        ),
        enriched AS (
            SELECT
                r.user_id,
                CASE
                    WHEN COALESCE(lp.visible, false) OR r.user_id = $1
                    THEN COALESCE(
                        lp.display_name,
                        up.display_name,
                        COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                    )
                    ELSE 'Investor #' || substring(r.user_id::text from 1 for 6)
                END                                      AS full_name,
                ut.tier_id
            FROM ranked r
            JOIN users u ON u.id = r.user_id
            LEFT JOIN user_profiles up   ON up.user_id = r.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = r.user_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = r.user_id
        )
        SELECT COUNT(*)::BIGINT FROM enriched
        WHERE ($2::int IS NULL OR tier_id = $2::int)
          AND ($3::text IS NULL OR full_name ILIKE '%' || $3::text || '%')
        "#,
        cutoff = cutoff_sql,
        order_expr = order_expr,
        val_expr = val_expr,
    );
    let total_participants: i64 = sqlx::query_scalar(&count_query)
        .bind(current_user_id)
        .bind(tier_id)
        .bind(search)
        .fetch_one(pool)
        .await?;

    // My rank for this timeframe
    let my_rank = get_my_rank_timeframed(pool, current_user_id, metric_type, cutoff_sql).await?;

    Ok((rankings, my_rank, total_participants, None))
}

/// Convert query rows to `LeaderboardEntry` structs.
fn rows_to_entries(rows: &[sqlx::postgres::PgRow], current_user_id: Uuid) -> Vec<LeaderboardEntry> {
    use sqlx::Row;
    rows.iter()
        .map(|r| {
            let user_id: Uuid = r.get("user_id");
            let is_visible: bool = r.get("is_visible");
            let show_avatar_pref: bool = r.get("show_avatar_pref");
            let is_current = user_id == current_user_id;

            let avatar_url = if (is_visible && show_avatar_pref) || is_current {
                r.get::<Option<String>, _>("avatar_url")
            } else {
                None
            };

            LeaderboardEntry {
                rank: r.get::<Option<i32>, _>("rank").unwrap_or(0),
                display_name: r.get::<String, _>("full_name"),
                avatar_url,
                tier_name: r.get("tier_name"),
                tier_badge_color: r.get("tier_badge_color"),
                metric_value: r.get::<Option<i64>, _>("metric_value").unwrap_or(0),
                is_current_user: is_current,
                metrics: LeaderboardMetrics {
                    total_invested_cents: r
                        .get::<Option<i64>, _>("total_invested_cents")
                        .unwrap_or(0),
                    asset_count: r.get::<Option<i32>, _>("asset_count").unwrap_or(0),
                    portfolio_roi_bps: r.get::<Option<i32>, _>("portfolio_roi_bps").unwrap_or(0),
                    affiliate_count: r.get::<Option<i32>, _>("affiliate_count").unwrap_or(0),
                    referral_revenue_cents: r
                        .get::<Option<i64>, _>("referral_revenue_cents")
                        .unwrap_or(0),
                    highest_investment_cents: r
                        .get::<Option<i64>, _>("highest_investment_cents")
                        .unwrap_or(0),
                },
            }
        })
        .collect()
}

// ─── My Rank Helpers ───────────────────────────────────────────────

/// Get the current user's rank from precomputed all-time data.
async fn get_my_rank_alltime(
    pool: &PgPool,
    user_id: Uuid,
    rank_col: &str,
    val_col: &str,
) -> Result<MyRank, AppError> {
    let query = format!(
        "SELECT {rank_col} AS rank, {val_col}::BIGINT AS metric_value,
         total_invested_cents, asset_count, portfolio_roi_bps,
         affiliate_count, referral_revenue_cents, highest_investment_cents
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
        Some(r) => Ok(MyRank {
            rank: r.get("rank"),
            metric_value: r.get::<Option<i64>, _>("metric_value").unwrap_or(0),
            metrics: LeaderboardMetrics {
                total_invested_cents: r.get::<Option<i64>, _>("total_invested_cents").unwrap_or(0),
                asset_count: r.get::<Option<i32>, _>("asset_count").unwrap_or(0),
                portfolio_roi_bps: r.get::<Option<i32>, _>("portfolio_roi_bps").unwrap_or(0),
                affiliate_count: r.get::<Option<i32>, _>("affiliate_count").unwrap_or(0),
                referral_revenue_cents: r
                    .get::<Option<i64>, _>("referral_revenue_cents")
                    .unwrap_or(0),
                highest_investment_cents: r
                    .get::<Option<i64>, _>("highest_investment_cents")
                    .unwrap_or(0),
            },
        }),
        None => Ok(MyRank::default()),
    }
}

/// Get the current user's rank from timeframe-filtered data (computed at query time).
async fn get_my_rank_timeframed(
    pool: &PgPool,
    user_id: Uuid,
    metric_type: &str,
    cutoff_sql: &str,
) -> Result<MyRank, AppError> {
    let (order_expr, val_expr) = match metric_type {
        "assets" => ("unique_assets DESC, total_invested DESC", "unique_assets"),
        "roi" => (
            "weighted_roi_bps DESC, total_invested DESC",
            "weighted_roi_bps",
        ),
        "affiliates" => ("aff_count DESC, network_value DESC", "aff_count"),
        "revenue" => ("network_value DESC, aff_count DESC", "network_value"),
        "highest_inv" => ("highest_inv DESC", "highest_inv"),
        _ => ("total_invested DESC", "total_invested"),
    };

    let query = format!(
        r#"
        WITH inv_agg AS (
            SELECT i.user_id,
                SUM(i.purchase_value_cents)::BIGINT              AS total_invested,
                COUNT(DISTINCT i.asset_id)               AS unique_assets,
                MAX(i.purchase_value_cents)               AS highest_inv,
                -- See refresh_all_scores for precision rationale.
                COALESCE(
                    ROUND(
                        SUM(i.purchase_value_cents::NUMERIC * COALESCE(a.annual_yield_bps, 0)::NUMERIC)
                        / NULLIF(SUM(i.purchase_value_cents::NUMERIC), 0)
                    ),
                    0
                ) AS weighted_roi_bps
            FROM investments i
            JOIN assets a ON a.id = i.asset_id
            WHERE i.status = 'active' AND i.purchased_at >= {cutoff}
            GROUP BY i.user_id
        ),
        ref_agg AS (
            SELECT rt.referrer_id,
                COUNT(DISTINCT rt.referred_id)            AS aff_count,
                COALESCE(SUM(inv.purchase_value_cents)::BIGINT, 0::BIGINT) AS network_value
            FROM referral_tracking rt
            LEFT JOIN investments inv ON inv.user_id = rt.referred_id
                AND inv.status = 'active' AND inv.purchased_at >= {cutoff}
            WHERE rt.created_at >= {cutoff}
            GROUP BY rt.referrer_id
        ),
        merged AS (
            SELECT
                COALESCE(ia.user_id, ra.referrer_id)     AS user_id,
                COALESCE(ia.total_invested, 0)           AS total_invested,
                COALESCE(ia.unique_assets, 0)::INT       AS unique_assets,
                COALESCE(ia.weighted_roi_bps, 0)::INT    AS weighted_roi_bps,
                COALESCE(ia.highest_inv, 0)              AS highest_inv,
                COALESCE(ra.aff_count, 0)::INT           AS aff_count,
                COALESCE(ra.network_value, 0)            AS network_value
            FROM inv_agg ia
            FULL OUTER JOIN ref_agg ra ON ia.user_id = ra.referrer_id
            WHERE COALESCE(ia.total_invested, 0) > 0 OR COALESCE(ra.aff_count, 0) > 0
        ),
        ranked AS (
            SELECT m.*,
                ROW_NUMBER() OVER (ORDER BY {order_expr})::INT AS rank,
                {val_expr}::BIGINT AS metric_value
            FROM merged m
        )
        SELECT rank, metric_value,
            total_invested AS total_invested_cents,
            unique_assets  AS asset_count,
            weighted_roi_bps AS portfolio_roi_bps,
            aff_count      AS affiliate_count,
            network_value  AS referral_revenue_cents,
            highest_inv    AS highest_investment_cents
        FROM ranked WHERE user_id = $1
        "#,
        cutoff = cutoff_sql,
        order_expr = order_expr,
        val_expr = val_expr,
    );

    let row = sqlx::query(&query)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    use sqlx::Row;
    match row {
        Some(r) => Ok(MyRank {
            rank: r.get("rank"),
            metric_value: r.get::<Option<i64>, _>("metric_value").unwrap_or(0),
            metrics: LeaderboardMetrics {
                total_invested_cents: r.get::<Option<i64>, _>("total_invested_cents").unwrap_or(0),
                asset_count: r.get::<Option<i32>, _>("asset_count").unwrap_or(0),
                portfolio_roi_bps: r.get::<Option<i32>, _>("portfolio_roi_bps").unwrap_or(0),
                affiliate_count: r.get::<Option<i32>, _>("affiliate_count").unwrap_or(0),
                referral_revenue_cents: r
                    .get::<Option<i64>, _>("referral_revenue_cents")
                    .unwrap_or(0),
                highest_investment_cents: r
                    .get::<Option<i64>, _>("highest_investment_cents")
                    .unwrap_or(0),
            },
        }),
        None => Ok(MyRank::default()),
    }
}

/// Public-facing helper: get user's rank for a given metric + timeframe.
pub async fn get_user_rank(
    pool: &PgPool,
    user_id: Uuid,
    metric_type: &str,
    timeframe: &str,
) -> Result<MyRank, AppError> {
    if let Some(cutoff) = timeframe_cutoff_sql(timeframe) {
        get_my_rank_timeframed(pool, user_id, metric_type, cutoff).await
    } else {
        let (rank_col, val_col) = metric_columns(metric_type);
        get_my_rank_alltime(pool, user_id, rank_col, val_col).await
    }
}

// ─── Preferences (unchanged) ──────────────────────────────────────

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
///
/// Partial-update semantics: fields omitted from the request (None) preserve
/// the existing saved value rather than overwriting it with a default. This
/// lets the UI toggle one field at a time (e.g. just `visible`) without
/// silently resetting `show_avatar` or `display_name`.
pub async fn update_preferences(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdatePreferencesRequest,
) -> Result<LeaderboardPreferences, AppError> {
    // Normalize incoming display_name: trim whitespace, treat empty as None.
    let display_name = req.display_name.as_deref().map(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    // For INSERT (new row) we need concrete defaults; for UPDATE we COALESCE
    // against the existing column so omitted fields keep their saved value.
    let visible_default = req.visible.unwrap_or(false);
    let show_avatar_default = req.show_avatar.unwrap_or(false);
    let display_name_default = display_name.clone().unwrap_or(None);

    sqlx::query!(
        r#"
        INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            visible      = COALESCE($5, leaderboard_preferences.visible),
            show_avatar  = COALESCE($6, leaderboard_preferences.show_avatar),
            display_name = CASE
                WHEN $7::bool THEN $8
                ELSE leaderboard_preferences.display_name
            END,
            updated_at = NOW()
        "#,
        user_id,
        visible_default,
        show_avatar_default,
        display_name_default,
        req.visible,
        req.show_avatar,
        req.display_name.is_some(),
        display_name_default,
    )
    .execute(pool)
    .await?;

    // Re-read the row so we return the actual merged state to the caller.
    get_preferences(pool, user_id).await
}
