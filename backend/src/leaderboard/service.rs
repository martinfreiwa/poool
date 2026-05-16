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
            referral_network_value_cents,
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
            referral_network_value_cents    = EXCLUDED.referral_network_value_cents,
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
                ROW_NUMBER() OVER (ORDER BY affiliate_count DESC, referral_network_value_cents DESC, computed_at ASC) AS r_aff,
                ROW_NUMBER() OVER (ORDER BY referral_network_value_cents DESC, affiliate_count DESC, computed_at ASC) AS r_rev,
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

/// Run [`refresh_all_scores`] and, on success, stamp the in-process
/// `last_updated` cache with the current UTC timestamp so subsequent
/// `/api/leaderboard` reads skip the `SELECT MAX(computed_at)` query.
/// Audit task C1.
pub async fn refresh_all_scores_and_cache(
    pool: &PgPool,
    cache: &LastUpdatedCache,
) -> Result<(), AppError> {
    refresh_all_scores(pool).await?;
    *cache.write().await = Some(chrono::Utc::now());
    Ok(())
}

/// Snapshot the current top-N for every metric into `leaderboard_snapshots`.
/// Intended to run once per day (UTC) from a tokio task in lib.rs.
///
/// Privacy:
///   - Only opted-in users (`leaderboard_preferences.visible = true`) are
///     snapshotted. Otherwise an opted-out user's historical rank could be
///     reconstructed by anyone with read access to the snapshots table.
///   - `top_n` caps how many ranks per metric get snapshotted; default 100
///     keeps the table compact (~600 rows/day with 6 metrics × top 100).
///
/// Idempotency:
///   - The table has `UNIQUE(user_id, metric, snapshot_date)`. The query
///     uses `ON CONFLICT … DO UPDATE` so re-running the task on the same
///     day overwrites with the freshest rank/value — safe to call multiple
///     times if the cron jitter overlaps.
pub async fn write_daily_snapshot(pool: &PgPool, top_n: i64) -> Result<u64, AppError> {
    // Build one INSERT per metric. Doing them in a loop instead of a giant
    // UNION ALL keeps each statement readable and lets a single metric
    // failure not kill the others.
    let metrics: &[(&str, &str)] = &[
        ("invested", "total_invested_cents"),
        ("assets", "asset_count"),
        ("roi", "portfolio_roi_bps"),
        ("affiliates", "affiliate_count"),
        ("revenue", "referral_network_value_cents"),
        ("highest_inv", "highest_investment_cents"),
    ];

    let mut total_rows = 0u64;
    for (metric, val_col) in metrics {
        // SAFETY: val_col is a hand-written literal from the allowlist
        // above — same pattern as `metric_columns` in this module. NOT
        // user-controlled, so `format!` interpolation is safe.
        //
        // The rank is re-derived via ROW_NUMBER over the VISIBLE-ONLY
        // subset, NOT inherited from `leaderboard_scores.rank_*` which
        // counts ALL users (including hidden). Otherwise a visible user
        // at global rank #500 (because 499 hidden users sit above them
        // in the precomputed ranks) would never get snapshotted under
        // the `<= $2` top-N cap, even though they're visible-rank #1.
        let sql = format!(
            r#"
            WITH visible_ranked AS (
                SELECT ls.user_id,
                       ls.{val_col}::BIGINT AS metric_value,
                       ROW_NUMBER() OVER (
                           ORDER BY ls.{val_col} DESC, ls.computed_at ASC
                       )::INT AS visible_rank
                FROM leaderboard_scores ls
                JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
                WHERE lp.visible = TRUE
            )
            INSERT INTO leaderboard_snapshots
                (user_id, metric, rank, metric_value, snapshot_date)
            SELECT user_id, $1, visible_rank, metric_value, (NOW() AT TIME ZONE 'UTC')::DATE
            FROM visible_ranked
            WHERE visible_rank <= $2
            ON CONFLICT (user_id, metric, snapshot_date) DO UPDATE SET
                rank         = EXCLUDED.rank,
                metric_value = EXCLUDED.metric_value
            "#,
            val_col = val_col,
        );

        let affected = sqlx::query(&sql)
            .bind(metric)
            .bind(top_n)
            .execute(pool)
            .await?
            .rows_affected();
        total_rows += affected;
        tracing::debug!(
            metric = %metric,
            rows = affected,
            "leaderboard snapshot written"
        );
    }

    tracing::info!(
        total_rows = total_rows,
        top_n = top_n,
        "leaderboard daily snapshot complete"
    );
    Ok(total_rows)
}

/// Prune `leaderboard_snapshots` rows older than the retention horizon.
/// Default 13 months matches the inline contract in migration 177.
///
/// Returns the number of deleted rows. Safe to run concurrently — the
/// DELETE is set-based with no row locks held across statements; multiple
/// instances racing produce a smaller count on the loser, never a
/// duplicate-delete error.
pub async fn prune_old_snapshots(pool: &PgPool, retain_days: i64) -> Result<u64, AppError> {
    let cutoff = format!("NOW() - INTERVAL '{} days'", retain_days);
    let sql = format!(
        "DELETE FROM leaderboard_snapshots WHERE snapshot_date < ({})::DATE",
        cutoff,
    );
    let deleted = sqlx::query(&sql).execute(pool).await?.rows_affected();
    tracing::info!(
        retain_days = retain_days,
        deleted = deleted,
        "leaderboard snapshots pruned"
    );
    Ok(deleted)
}

/// Fetch a user's rank trajectory for a metric over the last `days` window.
/// Returns rows newest-first so the UI can render a sparkline by reversing
/// in JS without a server-side ORDER BY ASC duplicate.
///
/// Privacy:
///   - The user can always read their OWN snapshots regardless of
///     visibility (their own history is theirs).
///   - This function does NOT join `leaderboard_preferences` because the
///     snapshot writer already only stores opted-in users. If a user
///     toggled visible OFF after their snapshot was written, that row
///     stays — it represents historical state at that point in time.
///   - The route caller is responsible for confirming `user_id ==
///     current_user`. Cross-user reads are not exposed.
pub async fn get_user_snapshots(
    pool: &PgPool,
    user_id: Uuid,
    metric: &str,
    days: i64,
) -> Result<Vec<UserSnapshotPoint>, AppError> {
    // Allowlist the metric input — same pattern as `metric_columns`. Any
    // value outside the set falls back to "invested" rather than running
    // the query with a hostile string.
    let safe_metric = match metric {
        "assets" | "roi" | "affiliates" | "revenue" | "highest_inv" => metric,
        _ => "invested",
    };
    let cutoff_days = days.clamp(1, 400); // hard cap matches 13-month retention

    let rows = sqlx::query_as::<_, UserSnapshotPoint>(
        r#"SELECT snapshot_date, rank, metric_value
           FROM leaderboard_snapshots
           WHERE user_id = $1
             AND metric  = $2
             AND snapshot_date >= (NOW() - ($3 || ' days')::INTERVAL)::DATE
           ORDER BY snapshot_date DESC"#,
    )
    .bind(user_id)
    .bind(safe_metric)
    .bind(cutoff_days.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Refresh ONE timeframe (`weekly` or `monthly`) into its dedicated
/// precomputed table. Mirrors [`refresh_all_scores`] structure but the
/// underlying CTE filters investments + referrals by a sliding window
/// (`NOW() - INTERVAL '<n> days'`).
///
/// Closes the Bereich-1+7 audit gap: previously the read path recomputed
/// these ranks on EVERY GET /api/leaderboard?timeframe=weekly|monthly.
/// Now reads become a flat indexed SELECT, identical in cost to the all-
/// time path.
///
/// Background-worker contract: call this for each timeframe on the same
/// cadence as `refresh_all_scores` (see lib.rs leaderboard task). The
/// two operations are independent — partial failure of one timeframe
/// does not invalidate the others.
///
/// `timeframe` must be exactly `"weekly"` or `"monthly"`; any other
/// value returns `BadRequest` since the target table name is derived
/// from it. The table name interpolation is safe because the match
/// produces only `&'static str` literals (same A3 safety pattern as
/// `metric_columns`).
pub async fn refresh_timeframed_scores(pool: &PgPool, timeframe: &str) -> Result<(), AppError> {
    // SAFETY (audit task A3 pattern): every match arm returns a static
    // literal that we control. Never interpolate a user-derived string.
    let (table, cutoff_sql): (&'static str, &'static str) = match timeframe {
        "weekly" => ("leaderboard_scores_weekly", "NOW() - INTERVAL '7 days'"),
        "monthly" => ("leaderboard_scores_monthly", "NOW() - INTERVAL '30 days'"),
        other => {
            return Err(AppError::BadRequest(format!(
                "refresh_timeframed_scores: unsupported timeframe '{}'",
                other
            )))
        }
    };

    // Step 0: full-wipe the timeframe table. The window is small enough
    // (≤30 days) that incremental upserts add complexity for negligible
    // gain — most users will roll off the window naturally. TRUNCATE is
    // O(1) and bypasses row-level overhead.
    let truncate_sql = format!("TRUNCATE TABLE {table}");
    sqlx::query(&truncate_sql).execute(pool).await?;

    // Step 1: insert fresh metrics for the timeframe. Same CTE shape as
    // the all-time refresh but with cutoff predicates on investments
    // and referral_tracking.
    let insert_sql = format!(
        r#"
        INSERT INTO {table} (
            user_id,
            total_invested_cents,
            asset_count,
            portfolio_roi_bps,
            affiliate_count,
            referral_network_value_cents,
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
                MAX(i.purchase_value_cents)              AS highest_inv,
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
        ) inv_agg ON inv_agg.user_id = u.id
        LEFT JOIN (
            SELECT
                rt.referrer_id,
                COUNT(DISTINCT rt.referred_id)            AS aff_count,
                COALESCE(SUM(inv.purchase_value_cents), 0) AS network_value
            FROM referral_tracking rt
            LEFT JOIN investments inv ON inv.user_id = rt.referred_id
                AND inv.status = 'active'
                AND inv.purchased_at >= {cutoff}
            WHERE rt.created_at >= {cutoff}
            GROUP BY rt.referrer_id
        ) ref_agg ON ref_agg.referrer_id = u.id
        WHERE u.status = 'active'
          AND (inv_agg.total_invested > 0 OR ref_agg.aff_count > 0)
        "#,
        table = table,
        cutoff = cutoff_sql,
    );
    sqlx::query(&insert_sql).execute(pool).await?;

    // Step 2: assign ranks per metric over the freshly-inserted rows.
    let rank_sql = format!(
        r#"
        UPDATE {table} ts SET
            rank_invested    = sub.r_inv,
            rank_assets      = sub.r_ast,
            rank_roi         = sub.r_roi,
            rank_affiliates  = sub.r_aff,
            rank_ref_revenue = sub.r_rev,
            rank_highest_inv = sub.r_hi
        FROM (
            SELECT user_id,
                ROW_NUMBER() OVER (ORDER BY total_invested_cents DESC, computed_at ASC)                      AS r_inv,
                ROW_NUMBER() OVER (ORDER BY asset_count DESC, total_invested_cents DESC, computed_at ASC)     AS r_ast,
                ROW_NUMBER() OVER (ORDER BY portfolio_roi_bps DESC, total_invested_cents DESC, computed_at ASC) AS r_roi,
                ROW_NUMBER() OVER (ORDER BY affiliate_count DESC, referral_network_value_cents DESC, computed_at ASC) AS r_aff,
                ROW_NUMBER() OVER (ORDER BY referral_network_value_cents DESC, affiliate_count DESC, computed_at ASC) AS r_rev,
                ROW_NUMBER() OVER (ORDER BY highest_investment_cents DESC, computed_at ASC)                  AS r_hi
            FROM {table}
        ) sub WHERE ts.user_id = sub.user_id
        "#,
        table = table,
    );
    sqlx::query(&rank_sql).execute(pool).await?;

    tracing::info!(
        timeframe = %timeframe,
        table = %table,
        "Leaderboard: refreshed timeframed scores"
    );
    Ok(())
}

// ─── Read Path ─────────────────────────────────────────────────────

// SAFETY / AUDIT NOTE (audit task A3):
// The values returned by this function are interpolated directly into SQL
// via `format!()` in the read-path queries (alltime + timeframed). That is
// safe ONLY because every arm of the `match` returns a hand-written
// `&'static str` literal that is part of the schema — never user input.
//
// Contract for future contributors:
//   1. Both fields of the returned tuple MUST remain `&'static str`. If you
//      change the return type to allow non-literals you also have to switch
//      the call sites to use a parameterized identifier strategy (e.g.
//      `sqlx::QueryBuilder` with an allowlist), or you will introduce a SQL
//      injection sink the moment someone passes a runtime string through.
//   2. Every returned string MUST match `^[a-z_]+$`. The `debug_assert!`
//      below is a tripwire for development/test builds; production builds
//      rely on the match-arm literal guarantee, not the assert.
//   3. The `_` (default) arm exists so that unknown / hostile `metric_type`
//      strings collapse to a safe fallback rather than being interpolated.
/// Resolve (rank_column, value_column) pair from metric key string.
///
/// Returns identifiers from a closed allowlist of schema columns. The result
/// is intended for direct interpolation into the `format!()`-built SELECT/
/// ORDER BY clauses in `get_rankings_alltime` and `get_my_rank_alltime`.
/// Callers MUST NOT pass these strings into a query alongside any other
/// runtime-derived identifier — that would defeat the allowlist guarantee.
fn metric_columns(metric_type: &str) -> (&'static str, &'static str) {
    let pair: (&'static str, &'static str) = match metric_type {
        "assets" => ("rank_assets", "asset_count"),
        "roi" => ("rank_roi", "portfolio_roi_bps"),
        "affiliates" => ("rank_affiliates", "affiliate_count"),
        "revenue" => ("rank_ref_revenue", "referral_network_value_cents"),
        "highest_inv" => ("rank_highest_inv", "highest_investment_cents"),
        _ => ("rank_invested", "total_invested_cents"), // default: "invested"
    };
    // Tripwire: catch typos/regressions in the allowlist during dev/test.
    // No-op in release builds.
    debug_assert!(
        pair.0.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            && pair.1.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
        "metric_columns must return allowlisted lowercase/underscore identifiers; got ({}, {})",
        pair.0,
        pair.1,
    );
    pair
}

/// Cache handle used by [`get_rankings`] to skip the per-request
/// `SELECT MAX(computed_at)` query (audit task C1).
///
/// Writers (the background refresh task and the admin POST refresh handler)
/// stamp the inner `Option` with `Some(Utc::now())` after a successful
/// `refresh_all_scores`. Readers see a hot cache and format it directly.
/// On a cold cache (server just booted; nothing refreshed yet), the read
/// path falls back to one `SELECT MAX(computed_at)` and warms the cache.
pub type LastUpdatedCache =
    std::sync::Arc<tokio::sync::RwLock<Option<chrono::DateTime<chrono::Utc>>>>;

/// Fetch the leaderboard rankings.
///
/// Supports two modes:
///   - **All-time** (`timeframe == "alltime"` or absent): reads directly from
///     the precomputed `leaderboard_scores` table — instant.
///   - **Weekly/Monthly** (`timeframe == "weekly" | "monthly"`): computes
///     time-filtered metrics at query time by aggregating only investments
///     purchased within the timeframe window, then ranking inline with ROW_NUMBER.
///
/// `last_updated_cache` is an in-process cache of the most recent
/// `leaderboard_scores.computed_at` (see [`LastUpdatedCache`]). When `Some`,
/// the all-time read path skips the `SELECT MAX(computed_at)` SQL.
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
    last_updated_cache: Option<&LastUpdatedCache>,
) -> Result<LeaderboardResponse, AppError> {
    let offset = (page - 1) * per_page;

    let (rankings, my_rank, total_participants, last_updated) =
        if timeframe == "weekly" || timeframe == "monthly" {
            // ── Timeframe-filtered: read precomputed table ──
            // (Background worker hydrates leaderboard_scores_weekly /
            // _monthly on the same cadence as the all-time table.)
            get_rankings_timeframed(
                pool,
                current_user_id,
                metric_type,
                timeframe,
                per_page,
                offset,
                tier_id,
                search.as_deref(),
            )
            .await?
        } else {
            // ── All-time: read from precomputed table ──
            let resolved_last_updated = resolve_last_updated(pool, last_updated_cache).await?;
            get_rankings_alltime(
                pool,
                current_user_id,
                metric_type,
                per_page,
                offset,
                tier_id,
                search.as_deref(),
                resolved_last_updated,
            )
            .await?
        };

    // Audit task B1: derive `has_more` from the count query, not from
    // `rankings.len() == per_page`. With the old heuristic, a page that
    // happened to fill exactly to `per_page` and was also the last page
    // (i.e. total is an exact multiple of per_page) would report
    // `has_more: true`, suggesting an empty next page to API consumers.
    let has_more = (offset + rankings.len() as i64) < total_participants;

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
///
/// `last_updated` is passed in by the caller (resolved via the in-process
/// cache; see [`LastUpdatedCache`] and [`resolve_last_updated`]). This
/// avoids running `SELECT MAX(computed_at)` per request (audit task C1).
#[allow(clippy::too_many_arguments)]
async fn get_rankings_alltime(
    pool: &PgPool,
    current_user_id: Uuid,
    metric_type: &str,
    limit: i64,
    offset: i64,
    tier_id: Option<i32>,
    search: Option<&str>,
    last_updated: Option<String>,
) -> Result<(Vec<LeaderboardEntry>, MyRank, i64, Option<String>), AppError> {
    let (rank_col, val_col) = metric_columns(metric_type);

    // Production behavior (audit fix):
    //   1. Only users with leaderboard_preferences.visible = true appear publicly.
    //      Hidden users get filtered out entirely (not pseudonymized).
    //   2. The current viewer ($5) NEVER appears in the public listing — even if
    //      they are visible. They see their own standing in the "Your Standing"
    //      card via get_my_rank_*, which avoids the awkward self-row in the table.
    //   3. Ranks are recomputed with ROW_NUMBER over the visible set so they
    //      stay sequential (1, 2, 3 …) instead of inheriting gaps from hidden
    //      users via the precomputed rank columns.
    //   4. Tier filter treats NULL user_tiers.tier_id as Intro (1) so Intro
    //      users without a row aren't silently excluded.
    let query = format!(
        r#"
        WITH visible AS (
            SELECT ls.*
            FROM leaderboard_scores ls
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
            WHERE ls.{rank_col} IS NOT NULL
              AND COALESCE(lp.visible, false) = true
              AND ls.user_id <> $5
        ),
        raw_data AS (
            SELECT
                ROW_NUMBER() OVER (ORDER BY v.{val_col} DESC, v.computed_at ASC)::INT AS rank,
                v.{val_col}::BIGINT                     AS metric_value,
                v.total_invested_cents,
                v.asset_count,
                v.portfolio_roi_bps,
                v.affiliate_count,
                v.referral_network_value_cents,
                v.highest_investment_cents,
                v.user_id,
                u.avatar_url,
                COALESCE(t.name, 'Intro')               AS tier_name,
                COALESCE(t.badge_color, '#D0D5DD')      AS tier_badge_color,
                TRUE                                    AS is_visible,
                COALESCE(lp.show_avatar, false)          AS show_avatar_pref,
                COALESCE(
                    lp.display_name,
                    up.display_name,
                    COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                )                                       AS full_name,
                COALESCE(ut.tier_id, 1)                 AS tier_id
            FROM visible v
            JOIN users u ON u.id = v.user_id
            LEFT JOIN user_profiles up   ON up.user_id = v.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = v.user_id
            LEFT JOIN tiers t            ON t.id = COALESCE(ut.tier_id, 1)
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = v.user_id
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

    let (mut rankings, user_ids) = rows_to_entries_with_ids(&rows, current_user_id);
    // Populate asset-mix donut data for the top-N rows only (bento cards).
    // Cap at 3 — the table view doesn't render the mix and we don't want
    // to pay for the extra GROUP BY when the listing is page 2+.
    let top_n = user_ids.iter().take(3).cloned().collect::<Vec<_>>();
    if !top_n.is_empty() {
        enrich_with_asset_mix(pool, &mut rankings[..top_n.len()], &top_n).await?;
    }

    let count_query = format!(
        r#"
        WITH raw_data AS (
            SELECT
                ls.user_id,
                COALESCE(
                    lp.display_name,
                    up.display_name,
                    COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                )                                       AS full_name,
                COALESCE(ut.tier_id, 1)                 AS tier_id
            FROM leaderboard_scores ls
            LEFT JOIN user_profiles up   ON up.user_id = ls.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = ls.user_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
            WHERE ls.{rank_col} IS NOT NULL
              AND COALESCE(lp.visible, false) = true
              AND ls.user_id <> $3
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

    // `last_updated` is already resolved by the caller (via the in-process
    // cache — see `resolve_last_updated`). No per-request MAX() query here.
    Ok((rankings, my_rank, total_participants, last_updated))
}

/// Resolve the most recent `leaderboard_scores.computed_at` formatted as an
/// ISO-8601 string. Reads the in-process cache first; on a cold cache (or
/// when no cache handle is provided, e.g. tests), runs a single
/// `SELECT MAX(computed_at)` and writes the result back into the cache so
/// subsequent requests are free.
pub async fn resolve_last_updated(
    pool: &PgPool,
    cache: Option<&LastUpdatedCache>,
) -> Result<Option<String>, AppError> {
    // Fast path: hot cache.
    if let Some(c) = cache {
        if let Some(ts) = *c.read().await {
            return Ok(Some(format_last_updated(ts)));
        }
    }

    // Slow path: ask Postgres for the current MAX. If the table is empty
    // this returns None and the cache stays cold (next refresh will warm it).
    let ts: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar(r#"SELECT MAX(computed_at) FROM leaderboard_scores"#)
            .fetch_one(pool)
            .await?;

    if let (Some(ts), Some(c)) = (ts, cache) {
        // Race-tolerant: writers stamp `Some(Utc::now())` after each
        // successful refresh; readers either see Some and short-circuit or
        // race on cold start and overwrite with the same value. Either way
        // the cache settles on a monotonically non-decreasing timestamp.
        *c.write().await = Some(ts);
    }

    Ok(ts.map(format_last_updated))
}

fn format_last_updated(ts: chrono::DateTime<chrono::Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Timeframe-filtered rankings: read from the precomputed table for the
/// requested timeframe (`leaderboard_scores_weekly` or `_monthly`).
///
/// Pre-2026-05-16: this function ran a 6-CTE live aggregation per request,
/// re-computing ROW_NUMBER() over the timeframe-filtered investment set.
/// O(n log n) per call. Closed by audit Bereich-1+7: the background worker
/// (lib.rs) now hydrates the per-timeframe tables, and this function is a
/// flat indexed SELECT identical to the all-time path.
///
/// `timeframe` is `"weekly"` or `"monthly"`. The match below produces only
/// `&'static str` literals (audit A3 SQL-injection safety pattern).
#[allow(clippy::too_many_arguments)]
async fn get_rankings_timeframed(
    pool: &PgPool,
    current_user_id: Uuid,
    metric_type: &str,
    timeframe: &str, // "weekly" | "monthly"
    limit: i64,
    offset: i64,
    tier_id: Option<i32>,
    search: Option<&str>,
) -> Result<(Vec<LeaderboardEntry>, MyRank, i64, Option<String>), AppError> {
    let table: &'static str = match timeframe {
        "weekly" => "leaderboard_scores_weekly",
        "monthly" => "leaderboard_scores_monthly",
        _ => "leaderboard_scores", // defensive fallback; caller already guards
    };
    let (rank_col, val_col) = metric_columns(metric_type);

    // Read `MAX(computed_at)` for THIS timeframe so the response's
    // `last_updated` reflects when the timeframe table itself was hydrated,
    // not when the all-time table was last refreshed.
    let last_updated_ts: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar(&format!("SELECT MAX(computed_at) FROM {table}"))
            .fetch_one(pool)
            .await?;
    let last_updated = last_updated_ts.map(format_last_updated);

    // Same shape as get_rankings_alltime — same CTE visible → raw_data
    // → filtered SELECT pattern. Differs only in the source table name.
    let query = format!(
        r#"
        WITH visible AS (
            SELECT ls.*
            FROM {table} ls
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
            WHERE ls.{rank_col} IS NOT NULL
              AND COALESCE(lp.visible, false) = true
              AND ls.user_id <> $5
        ),
        raw_data AS (
            SELECT
                ROW_NUMBER() OVER (ORDER BY v.{val_col} DESC, v.computed_at ASC)::INT AS rank,
                v.{val_col}::BIGINT                     AS metric_value,
                v.total_invested_cents,
                v.asset_count,
                v.portfolio_roi_bps,
                v.affiliate_count,
                v.referral_network_value_cents,
                v.highest_investment_cents,
                v.user_id,
                u.avatar_url,
                COALESCE(t.name, 'Intro')               AS tier_name,
                COALESCE(t.badge_color, '#D0D5DD')      AS tier_badge_color,
                TRUE                                    AS is_visible,
                COALESCE(lp.show_avatar, false)          AS show_avatar_pref,
                COALESCE(
                    lp.display_name,
                    up.display_name,
                    COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                )                                       AS full_name,
                COALESCE(ut.tier_id, 1)                 AS tier_id
            FROM visible v
            JOIN users u ON u.id = v.user_id
            LEFT JOIN user_profiles up   ON up.user_id = v.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = v.user_id
            LEFT JOIN tiers t            ON t.id = COALESCE(ut.tier_id, 1)
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = v.user_id
        )
        SELECT * FROM raw_data
        WHERE ($3::int IS NULL OR tier_id = $3::int)
          AND ($4::text IS NULL OR full_name ILIKE '%' || $4::text || '%')
        ORDER BY rank ASC
        LIMIT $1 OFFSET $2
        "#,
        table = table,
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

    let (mut rankings, user_ids) = rows_to_entries_with_ids(&rows, current_user_id);
    let top_n = user_ids.iter().take(3).cloned().collect::<Vec<_>>();
    if !top_n.is_empty() {
        enrich_with_asset_mix(pool, &mut rankings[..top_n.len()], &top_n).await?;
    }

    let count_query = format!(
        r#"
        WITH raw_data AS (
            SELECT
                ls.user_id,
                COALESCE(
                    lp.display_name,
                    up.display_name,
                    COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                )                                       AS full_name,
                COALESCE(ut.tier_id, 1)                 AS tier_id
            FROM {table} ls
            LEFT JOIN user_profiles up   ON up.user_id = ls.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = ls.user_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
            WHERE ls.{rank_col} IS NOT NULL
              AND COALESCE(lp.visible, false) = true
              AND ls.user_id <> $3
        )
        SELECT COUNT(*)::BIGINT FROM raw_data
        WHERE ($1::int IS NULL OR tier_id = $1::int)
          AND ($2::text IS NULL OR full_name ILIKE '%' || $2::text || '%')
        "#,
        table = table,
        rank_col = rank_col,
    );
    let total_participants: i64 = sqlx::query_scalar(&count_query)
        .bind(tier_id)
        .bind(search)
        .bind(current_user_id)
        .fetch_one(pool)
        .await?;

    // My rank within this timeframe — read directly from the precomputed
    // rank column on this user's row in the timeframe table.
    let my_rank_query = format!(
        "SELECT {rank_col} FROM {table} WHERE user_id = $1",
        table = table,
        rank_col = rank_col,
    );
    let my_rank_value: Option<i32> = sqlx::query_scalar(&my_rank_query)
        .bind(current_user_id)
        .fetch_optional(pool)
        .await?
        .flatten();
    let my_metric_query = format!(
        "SELECT {val_col}::BIGINT FROM {table} WHERE user_id = $1",
        table = table,
        val_col = val_col,
    );
    let my_metric_value: Option<i64> = sqlx::query_scalar(&my_metric_query)
        .bind(current_user_id)
        .fetch_optional(pool)
        .await?;
    let my_rank = MyRank {
        rank: my_rank_value,
        metric_value: my_metric_value.unwrap_or(0),
        metrics: Default::default(),
    };

    Ok((rankings, my_rank, total_participants, last_updated))
}

/// LEGACY: live-aggregation path retained for emergency fallback if the
/// timeframe tables ever desync. Not wired by default. To re-enable,
/// route `get_rankings` to call this function for the weekly/monthly
/// branch.
#[allow(dead_code, clippy::too_many_arguments)]
async fn get_rankings_timeframed_live(
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
        visible AS (
            -- Filter to visible users + exclude the viewer (they see themselves
            -- in the "Your Standing" card). Audit fix: privacy + UX.
            SELECT m.*
            FROM merged m
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = m.user_id
            WHERE COALESCE(lp.visible, false) = true
              AND m.user_id <> $3
        ),
        ranked AS (
            SELECT
                v.*,
                ROW_NUMBER() OVER (ORDER BY {order_expr})::INT  AS rank,
                {val_expr}::BIGINT                         AS metric_value
            FROM visible v
        ),
        enriched AS (
            SELECT
                r.rank,
                r.metric_value,
                r.total_invested    AS total_invested_cents,
                r.unique_assets     AS asset_count,
                r.weighted_roi_bps  AS portfolio_roi_bps,
                r.aff_count         AS affiliate_count,
                r.network_value     AS referral_network_value_cents,
                r.highest_inv       AS highest_investment_cents,
                r.user_id,
                u.avatar_url,
                COALESCE(t.name, 'Intro')               AS tier_name,
                COALESCE(t.badge_color, '#D0D5DD')       AS tier_badge_color,
                TRUE                                     AS is_visible,
                COALESCE(lp.show_avatar, false)           AS show_avatar_pref,
                COALESCE(
                    lp.display_name,
                    up.display_name,
                    COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                )                                        AS full_name,
                COALESCE(ut.tier_id, 1)                  AS tier_id
            FROM ranked r
            JOIN users u ON u.id = r.user_id
            LEFT JOIN user_profiles up   ON up.user_id = r.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = r.user_id
            LEFT JOIN tiers t            ON t.id = COALESCE(ut.tier_id, 1)
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

    let (mut rankings, user_ids) = rows_to_entries_with_ids(&rows, current_user_id);
    // Top-N enrichment — same rationale as the alltime branch.
    let top_n = user_ids.iter().take(3).cloned().collect::<Vec<_>>();
    if !top_n.is_empty() {
        enrich_with_asset_mix(pool, &mut rankings[..top_n.len()], &top_n).await?;
    }

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
        visible AS (
            SELECT m.*
            FROM merged m
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = m.user_id
            WHERE COALESCE(lp.visible, false) = true
              AND m.user_id <> $1
        ),
        enriched AS (
            SELECT
                v.user_id,
                COALESCE(
                    lp.display_name,
                    up.display_name,
                    COALESCE(up.first_name || ' ' || LEFT(COALESCE(up.last_name, ''), 1) || '.', 'Investor')
                )                                        AS full_name,
                COALESCE(ut.tier_id, 1)                  AS tier_id
            FROM visible v
            LEFT JOIN user_profiles up   ON up.user_id = v.user_id
            LEFT JOIN user_tiers ut      ON ut.user_id = v.user_id
            LEFT JOIN leaderboard_preferences lp ON lp.user_id = v.user_id
        )
        SELECT COUNT(*)::BIGINT FROM enriched
        WHERE ($2::int IS NULL OR tier_id = $2::int)
          AND ($3::text IS NULL OR full_name ILIKE '%' || $3::text || '%')
        "#,
        cutoff = cutoff_sql,
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

/// Convert query rows to `LeaderboardEntry` structs *and* return the
/// parallel user-id list for downstream enrichment (asset-mix donut).
fn rows_to_entries_with_ids(
    rows: &[sqlx::postgres::PgRow],
    current_user_id: Uuid,
) -> (Vec<LeaderboardEntry>, Vec<Uuid>) {
    use sqlx::Row;
    let user_ids: Vec<Uuid> = rows.iter().map(|r| r.get::<Uuid, _>("user_id")).collect();
    let entries = rows_to_entries(rows, current_user_id);
    (entries, user_ids)
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
                    referral_network_value_cents: r
                        .get::<Option<i64>, _>("referral_network_value_cents")
                        .unwrap_or(0),
                    highest_investment_cents: r
                        .get::<Option<i64>, _>("highest_investment_cents")
                        .unwrap_or(0),
                },
                asset_mix: Vec::new(), // populated separately for top-N rows
            }
        })
        .collect()
}

/// Populate `asset_mix` for the top-N entries of the listing. One round-trip
/// per call: a single SQL with `WHERE user_id = ANY($1) GROUP BY user_id,
/// asset_type` produces all slices for the top-N user-ids at once. Cheap
/// because `investments` is indexed on `user_id`.
///
/// Slices are sorted by `invested_cents` descending so the bento donut's
/// dominant wedge is always slice[0].
async fn enrich_with_asset_mix(
    pool: &PgPool,
    entries: &mut [LeaderboardEntry],
    user_ids: &[Uuid],
) -> Result<(), AppError> {
    if user_ids.is_empty() {
        return Ok(());
    }

    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT
            i.user_id,
            a.asset_type::text                                      AS asset_type,
            SUM(i.purchase_value_cents)::BIGINT                     AS invested_cents,
            COUNT(DISTINCT i.asset_id)::INTEGER                     AS asset_count
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE i.user_id = ANY($1)
          AND i.status = 'active'
        GROUP BY i.user_id, a.asset_type
        ORDER BY i.user_id, invested_cents DESC
        "#,
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;

    // Index slices by user_id so we don't O(N²) scan per entry.
    use std::collections::HashMap;
    let mut by_user: HashMap<Uuid, Vec<AssetMixSlice>> = HashMap::new();
    for r in &rows {
        let uid: Uuid = r.get("user_id");
        by_user.entry(uid).or_default().push(AssetMixSlice {
            asset_type: r.get("asset_type"),
            invested_cents: r.get::<Option<i64>, _>("invested_cents").unwrap_or(0),
            asset_count: r.get::<Option<i32>, _>("asset_count").unwrap_or(0),
        });
    }

    // Backend serializes Uuid as a string in the entries' display, but we
    // need to look up by Uuid. The user_id is not exposed on LeaderboardEntry,
    // so we rely on the entries arriving in the same order they were queried.
    // To recover the user_id we re-fetch from a parallel slice — see the
    // caller pattern in get_rankings_alltime / _timeframed.
    for (entry, uid) in entries.iter_mut().zip(user_ids.iter()) {
        if let Some(slices) = by_user.remove(uid) {
            entry.asset_mix = slices;
        }
    }

    Ok(())
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
         affiliate_count, referral_network_value_cents, highest_investment_cents
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
                referral_network_value_cents: r
                    .get::<Option<i64>, _>("referral_network_value_cents")
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
            network_value  AS referral_network_value_cents,
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
                referral_network_value_cents: r
                    .get::<Option<i64>, _>("referral_network_value_cents")
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
/// Timeframed paths now read from the precomputed `leaderboard_scores_weekly`
/// / `_monthly` tables (same audit-driven optimisation as `get_rankings`).
pub async fn get_user_rank(
    pool: &PgPool,
    user_id: Uuid,
    metric_type: &str,
    timeframe: &str,
) -> Result<MyRank, AppError> {
    let (rank_col, val_col) = metric_columns(metric_type);
    let table: &'static str = match timeframe {
        "weekly" => "leaderboard_scores_weekly",
        "monthly" => "leaderboard_scores_monthly",
        _ => return get_my_rank_alltime(pool, user_id, rank_col, val_col).await,
    };

    // Same shape as get_my_rank_alltime: read the precomputed rank +
    // metric value plus the total visible participant count.
    let rank_query = format!(
        "SELECT {rank_col} FROM {table} WHERE user_id = $1",
        table = table,
        rank_col = rank_col,
    );
    let rank_value: Option<i32> = sqlx::query_scalar(&rank_query)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .flatten();

    let metric_query = format!(
        "SELECT {val_col}::BIGINT FROM {table} WHERE user_id = $1",
        table = table,
        val_col = val_col,
    );
    let metric_value: Option<i64> = sqlx::query_scalar(&metric_query)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    let total_query = format!(
        r#"SELECT COUNT(*)::BIGINT
           FROM {table} ls
           LEFT JOIN leaderboard_preferences lp ON lp.user_id = ls.user_id
           WHERE ls.{rank_col} IS NOT NULL
             AND COALESCE(lp.visible, false) = true"#,
        table = table,
        rank_col = rank_col,
    );
    let _total: i64 = sqlx::query_scalar(&total_query).fetch_one(pool).await?;

    Ok(MyRank {
        rank: rank_value,
        metric_value: metric_value.unwrap_or(0),
        metrics: Default::default(),
    })
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
