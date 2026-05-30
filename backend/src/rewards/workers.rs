//! Hintergrund-Worker für das Rewards-/Affiliate-System.
//!
//! Aktuell hier:
//!   * `run_affiliate_rollup_worker` — schreibt täglich `affiliate_daily_rollups`
//!     aus rohen Clicks/Referrals/Commissions. Läuft alle 15 Minuten und
//!     UPSERTet die heutige + gestrige Partition (idempotent).
//!   * `ensure_referral_clicks_partitions` — täglicher Maintenance-Tick der
//!     `referral_clicks_ensure_future_partitions(3)` aufruft, damit
//!     monatliche Partitionen stets >= 3 Monate im Voraus existieren.
//!
//! Holdback- und Tier-Worker leben weiterhin in `service.rs` (historisch).

use sqlx::PgPool;
use tokio::time::{Duration, MissedTickBehavior};

const ROLLUP_INTERVAL_SECS: u64 = 15 * 60;
const PARTITION_MAINT_INTERVAL_SECS: u64 = 24 * 60 * 60;
const PARTITION_RETENTION_INTERVAL_SECS: u64 = 24 * 60 * 60;
const LEADERBOARD_REFRESH_INTERVAL_SECS: u64 = 15 * 60;

/// Drop click partitions older than this many months. 24 months keeps a
/// 2-year audit window which covers most fraud-investigation and Tax-year
/// reconciliation needs. Override via env `REFERRAL_CLICKS_RETENTION_MONTHS`.
const DEFAULT_RETENTION_MONTHS: i32 = 24;

/// Pflege-Worker für die `affiliate_daily_rollups`-Tabelle.
///
/// Rechnet bei jedem Tick die Aggregate für HEUTE und GESTERN neu. Damit
/// werden Late-Arriving-Rows (z.B. Click mit timestamp +1s am Tagesübergang
/// oder eine Commission die mit Verzögerung committed) zuverlässig erfasst.
/// Für Tage älter als gestern: stable, kein Update mehr.
///
/// Idempotent über den aktuellen Rollup-Key
/// `(rollup_date, link_id, currency)`.
pub async fn run_affiliate_rollup_worker(pool: PgPool) {
    // 30s Offset damit Worker nicht im Boot-Sturm starten
    tokio::time::sleep(Duration::from_secs(30)).await;
    let mut interval = tokio::time::interval(Duration::from_secs(ROLLUP_INTERVAL_SECS));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!(
        "📊 Affiliate rollup worker armed (interval = {}s)",
        ROLLUP_INTERVAL_SECS
    );

    loop {
        interval.tick().await;
        let started = std::time::Instant::now();
        match recompute_rollups_for_recent_days(&pool, 2).await {
            Ok(rows) => tracing::info!(
                "📊 Affiliate rollup worker: upserted {} rollup rows in {}ms",
                rows,
                started.elapsed().as_millis()
            ),
            Err(e) => tracing::error!("📊 Affiliate rollup worker failed: {}", e),
        }
    }
}

/// Rechnet Rollups für die letzten `days_back` Tage neu (heute = day 0).
/// Aufrufer-Pattern: Worker, Admin-Reseed, Tests.
pub async fn recompute_rollups_for_recent_days(
    pool: &PgPool,
    days_back: i32,
) -> Result<u64, sqlx::Error> {
    // Einzelner UPSERT-Query der per CTE Clicks + Referrals + Commissions
    // pro (date, link_id) joint und in den Rollup schreibt. Eine Round-Trip,
    // Postgres-seitig planoptimiert.
    let res = sqlx::query(
        r#"
        WITH date_range AS (
            SELECT generate_series(
                (CURRENT_DATE - ($1::int) * INTERVAL '1 day')::date,
                CURRENT_DATE,
                INTERVAL '1 day'
            )::date AS d
        ),
        link_dates AS (
            -- Cross product nur für Links mit irgendeiner Activity im Range
            SELECT DISTINCT d.d, l.id AS link_id
            FROM date_range d
            CROSS JOIN affiliate_links l
            WHERE EXISTS (
                SELECT 1 FROM referral_clicks rc
                WHERE rc.link_id = l.id
                  AND rc.created_at >= d.d
                  AND rc.created_at < d.d + INTERVAL '1 day'
            )
            OR EXISTS (
                SELECT 1 FROM affiliate_referrals ar
                WHERE ar.link_id = l.id
                  AND ar.created_at >= d.d
                  AND ar.created_at < d.d + INTERVAL '1 day'
            )
            OR EXISTS (
                SELECT 1 FROM affiliate_commissions ac
                WHERE ac.link_id = l.id
                  AND ac.created_at >= d.d
                  AND ac.created_at < d.d + INTERVAL '1 day'
            )
        ),
        per_link AS (
            SELECT
                ld.d AS rollup_date,
                ld.link_id,
                l.payout_user_id,
                l.attribution_user_id,
                l.team_id,
                l.link_type,
                COALESCE((
                    SELECT COUNT(*) FROM referral_clicks rc
                    WHERE rc.link_id = ld.link_id
                      AND rc.created_at >= ld.d
                      AND rc.created_at < ld.d + INTERVAL '1 day'
                ), 0)::int AS clicks_count,
                COALESCE((
                    SELECT COUNT(*) FROM affiliate_referrals ar
                    WHERE ar.link_id = ld.link_id
                      AND ar.created_at >= ld.d
                      AND ar.created_at < ld.d + INTERVAL '1 day'
                ), 0)::int AS signups_count,
                COALESCE((
                    SELECT COUNT(*) FROM affiliate_referrals ar
                    WHERE ar.link_id = ld.link_id
                      AND ar.status IN ('qualified', 'paid')
                      AND COALESCE(ar.qualified_at, ar.updated_at) >= ld.d
                      AND COALESCE(ar.qualified_at, ar.updated_at) < ld.d + INTERVAL '1 day'
                ), 0)::int AS qualified_count,
                COALESCE((
                    -- Bug 1+2 fix: use the new self-contained
                    -- ac.gross_amount_cents column instead of joining to
                    -- investments via source_order_id (which contained
                    -- orders.id in production, not investments.id).
                    SELECT SUM(ac.gross_amount_cents)::bigint
                    FROM affiliate_commissions ac
                    WHERE ac.link_id = ld.link_id
                      AND ac.created_at >= ld.d
                      AND ac.created_at < ld.d + INTERVAL '1 day'
                ), 0) AS gross_revenue_cents,
                COALESCE((
                    SELECT SUM(ac.provisional_amount_cents)::bigint
                    FROM affiliate_commissions ac
                    WHERE ac.link_id = ld.link_id
                      AND ac.created_at >= ld.d
                      AND ac.created_at < ld.d + INTERVAL '1 day'
                ), 0) AS commission_cents
            FROM link_dates ld
            JOIN affiliate_links l ON l.id = ld.link_id
        )
        INSERT INTO affiliate_daily_rollups
            (rollup_date, link_id, payout_user_id, attribution_user_id, team_id,
             link_type, clicks_count, signups_count, qualified_count,
             gross_revenue_cents, commission_cents, updated_at)
        SELECT rollup_date, link_id, payout_user_id, attribution_user_id, team_id,
               link_type, clicks_count, signups_count, qualified_count,
               gross_revenue_cents, commission_cents, NOW()
        FROM per_link
        ON CONFLICT (rollup_date, link_id, currency) DO UPDATE SET
            clicks_count        = EXCLUDED.clicks_count,
            signups_count       = EXCLUDED.signups_count,
            qualified_count     = EXCLUDED.qualified_count,
            gross_revenue_cents = EXCLUDED.gross_revenue_cents,
            commission_cents    = EXCLUDED.commission_cents,
            payout_user_id      = EXCLUDED.payout_user_id,
            attribution_user_id = EXCLUDED.attribution_user_id,
            team_id             = EXCLUDED.team_id,
            link_type           = EXCLUDED.link_type,
            updated_at          = NOW()
        "#,
    )
    .bind(days_back)
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}

/// Sorgt dafür dass `referral_clicks` immer mindestens 3 Monate Partitionen
/// im Voraus hat. Tägliches Tick reicht; Funktion ist idempotent.
///
/// Startup-Tick: zusätzlich beim Boot einmal ausführen, damit ein Backend
/// das >30 Tage offline war beim Wieder-Anlauf Partitionen für seinen
/// Aufholbedarf erzeugt, BEVOR die erste Click-INSERT scheitert.
pub async fn run_referral_clicks_partition_maint_worker(pool: PgPool) {
    // Startup heal — wider window than the daily tick to cover prolonged downtime.
    if let Err(e) = ensure_partitions(&pool, 6).await {
        tracing::error!("📦 referral_clicks startup partition-heal failed: {}", e);
    } else {
        tracing::info!("📦 referral_clicks: startup partition-heal complete");
    }

    let mut interval = tokio::time::interval(Duration::from_secs(PARTITION_MAINT_INTERVAL_SECS));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    interval.tick().await; // consume immediate-first-tick after startup heal

    tracing::info!(
        "📦 referral_clicks partition maintainer armed (interval = {}s)",
        PARTITION_MAINT_INTERVAL_SECS
    );

    loop {
        interval.tick().await;
        if let Err(e) = ensure_partitions(&pool, 3).await {
            tracing::error!("📦 referral_clicks partition maint failed: {}", e);
        }
    }
}

async fn ensure_partitions(pool: &PgPool, months_ahead: i32) -> Result<(), sqlx::Error> {
    let n: i32 = sqlx::query_scalar("SELECT referral_clicks_ensure_future_partitions($1)")
        .bind(months_ahead)
        .fetch_one(pool)
        .await?;
    if n > 0 {
        tracing::info!("📦 referral_clicks: {} new monthly partition(s) created", n);
    }
    Ok(())
}

/// Phase-6: refreshes the `affiliate_leaderboard_public` materialised
/// view every 15 minutes so the public leaderboard page stays current
/// without an operator hand-running `REFRESH MATERIALIZED VIEW`.
///
/// `CONCURRENTLY` keeps the page readable during the refresh. Postgres
/// requires a unique index on the matview for this — we created one in
/// mig 192 (`idx_affiliate_leaderboard_public_user`).
pub async fn run_affiliate_leaderboard_refresh_worker(pool: PgPool) {
    // 60s boot offset so the worker doesn't crowd the first-tick storm.
    tokio::time::sleep(Duration::from_secs(60)).await;
    let mut interval =
        tokio::time::interval(Duration::from_secs(LEADERBOARD_REFRESH_INTERVAL_SECS));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!(
        "🏆 affiliate-leaderboard matview refresh armed (interval = {}s)",
        LEADERBOARD_REFRESH_INTERVAL_SECS
    );

    loop {
        interval.tick().await;
        let started = std::time::Instant::now();
        match sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY affiliate_leaderboard_public")
            .execute(&pool)
            .await
        {
            Ok(_) => tracing::info!(
                "🏆 affiliate-leaderboard refresh OK in {}ms",
                started.elapsed().as_millis()
            ),
            Err(e) => tracing::error!("🏆 affiliate-leaderboard refresh failed: {}", e),
        }
    }
}

/// Retention worker — drops `referral_clicks_YYYY_MM` partitions older than
/// `REFERRAL_CLICKS_RETENTION_MONTHS` (default 24). Idempotent.
///
/// Drop is irreversible — guarded by the env var so operators can pause
/// retention entirely by setting `REFERRAL_CLICKS_RETENTION_MONTHS=0`.
pub async fn run_referral_clicks_partition_retention_worker(pool: PgPool) {
    let months: i32 = std::env::var("REFERRAL_CLICKS_RETENTION_MONTHS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_RETENTION_MONTHS);
    if months <= 0 {
        tracing::info!("🗑  referral_clicks retention DISABLED (retention_months <= 0)");
        return;
    }

    tokio::time::sleep(Duration::from_secs(120)).await; // boot offset
    let mut interval =
        tokio::time::interval(Duration::from_secs(PARTITION_RETENTION_INTERVAL_SECS));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    tracing::info!(
        "🗑  referral_clicks retention armed (retain {} months, interval {}s)",
        months,
        PARTITION_RETENTION_INTERVAL_SECS
    );

    loop {
        interval.tick().await;
        let dropped: Result<i32, _> =
            sqlx::query_scalar("SELECT referral_clicks_drop_old_partitions($1)")
                .bind(months)
                .fetch_one(&pool)
                .await;
        match dropped {
            Ok(n) if n > 0 => {
                tracing::info!("🗑  referral_clicks: dropped {} old monthly partition(s)", n)
            }
            Ok(_) => {}
            Err(e) => tracing::error!("🗑  referral_clicks retention failed: {}", e),
        }
    }
}
