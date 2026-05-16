//! Reporting-Queries für Developer-Dashboard und Member-Self-View.
//!
//! Hot Path liest `affiliate_daily_rollups` (pre-aggregiert) und
//! `affiliate_live_counters` (O(1)-Tiles). Cold Path Customer-/Product-
//! Listings joinen raw `affiliate_referrals` + `affiliate_commissions`.

use crate::error::AppError;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PeriodSummary {
    pub clicks_count: i64,
    pub signups_count: i64,
    pub qualified_count: i64,
    pub gross_revenue_cents: i64,
    pub commission_cents: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MemberBreakdown {
    pub attribution_user_id: Uuid,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub clicks_count: i64,
    pub signups_count: i64,
    pub qualified_count: i64,
    pub gross_revenue_cents: i64,
    pub commission_cents: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TeamCustomer {
    pub referred_user_id: Uuid,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub attribution_user_id: Uuid,
    pub attribution_user_name: Option<String>,
    pub referral_status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub gross_invested_cents: i64,
    pub commission_earned_cents: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TeamProductSale {
    pub asset_id: Uuid,
    pub asset_name: Option<String>,
    pub units_sold: i64,
    pub gross_revenue_cents: i64,
    pub commission_cents: i64,
}

/// Daily time-series point — one bucket per day in the requested range.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyPoint {
    pub bucket_date: chrono::NaiveDate,
    pub clicks_count: i64,
    pub signups_count: i64,
    pub qualified_count: i64,
    pub gross_revenue_cents: i64,
    pub commission_cents: i64,
}

/// Aggregated overview for the analytics dashboard.
#[derive(Debug, Serialize)]
pub struct AnalyticsOverview {
    /// Period totals.
    pub period: PeriodSummary,
    /// Same-length previous period (e.g. prev 30 days) — for delta cards.
    pub previous_period: PeriodSummary,
    /// Exact window the `previous_period` totals cover — surfaced in the UI
    /// so users can see *which* range "vs prev" actually compares against.
    pub prev_from: chrono::NaiveDate,
    pub prev_to: chrono::NaiveDate,
    /// All-time live counters (from affiliate_live_counters, O(1)).
    pub lifetime_commission_cents: i64,
    pub pending_commission_cents: i64,
    pub payable_commission_cents: i64,
    pub paid_commission_cents: i64,
    pub clawed_back_cents: i64,
    /// Active member count for context.
    pub active_members: i64,
    /// Top 3 members by commission_cents in period.
    pub top_performers: Vec<MemberBreakdown>,
    /// Bottom 3 active members (zero-commission filter applied) — useful
    /// for identifying deficits.
    pub deficit_members: Vec<MemberBreakdown>,
    /// Top 3 assets sold in period.
    pub top_assets: Vec<TeamProductSale>,
    /// Next payout window — calculated from earliest holdback_expires_at
    /// of payable commissions, or NULL if nothing is pending.
    pub next_payout_date: Option<chrono::DateTime<chrono::Utc>>,
    /// Sum of `payable` commission cents — what next payout would cover.
    pub next_payout_amount_cents: i64,
    /// Open payout requests count (pending admin action).
    pub open_payout_requests: i64,
    /// Server timestamp when the overview was computed — used for the
    /// "last refreshed" UI indicator.
    pub computed_at: chrono::DateTime<chrono::Utc>,
}

pub async fn analytics_overview(
    pool: &PgPool,
    team_id: Uuid,
    developer_user_id: Uuid,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
) -> Result<AnalyticsOverview, AppError> {
    let period = team_period_summary(pool, team_id, from_date, to_date).await?;
    // Same-length previous window: e.g. period = 30 days → prev = 30 days
    // immediately before. Always 1 day before `from_date` as the upper bound.
    let period_len = (to_date - from_date).num_days().max(0);
    let prev_to = from_date.pred_opt().unwrap_or(from_date);
    let prev_from = prev_to - chrono::Duration::days(period_len);
    let previous_period = team_period_summary(pool, team_id, prev_from, prev_to).await?;

    // Live counters (O(1) lookup) for all-time view.
    let counter = sqlx::query!(
        r#"SELECT lifetime_commission_cents, pending_commission_cents,
                  payable_commission_cents, paid_commission_cents, clawed_back_cents
           FROM affiliate_live_counters WHERE payout_user_id = $1"#,
        developer_user_id
    )
    .fetch_optional(pool)
    .await?;

    let active_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM developer_team_memberships WHERE team_id = $1 AND status='active'",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await?;

    let by_member = team_period_by_member(pool, team_id, from_date, to_date).await?;
    let top_performers: Vec<MemberBreakdown> = by_member
        .iter()
        .filter(|m| m.commission_cents > 0)
        .take(3)
        .cloned()
        .collect();
    let deficit_members: Vec<MemberBreakdown> = by_member
        .iter()
        .filter(|m| m.commission_cents == 0)
        .take(3)
        .cloned()
        .collect();

    let top_assets: Vec<TeamProductSale> = team_products(pool, team_id, from_date, to_date)
        .await?
        .into_iter()
        .take(3)
        .collect();

    // Next payout window: earliest holdback_expires_at on under-holdback
    // commissions for this developer. Plus payable sum (what gets paid out
    // at next batch).
    let next_payout = sqlx::query!(
        r#"SELECT MIN(ar.holdback_expires_at) AS next_at
           FROM affiliate_commissions ac
           JOIN affiliate_referrals ar ON ar.id = ac.referral_id
           WHERE ac.payout_user_id = $1
             AND ac.status IN ('provisionally_tracked', 'on_hold')
             AND ar.status = 'under_holdback'"#,
        developer_user_id
    )
    .fetch_optional(pool)
    .await?
    .and_then(|r| r.next_at);

    let open_payout_requests: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::BIGINT FROM affiliate_payout_requests \
         WHERE affiliate_id = $1 AND status IN ('requested', 'approved')",
    )
    .bind(developer_user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok(AnalyticsOverview {
        period,
        previous_period,
        prev_from,
        prev_to,
        lifetime_commission_cents: counter
            .as_ref()
            .map(|c| c.lifetime_commission_cents)
            .unwrap_or(0),
        pending_commission_cents: counter
            .as_ref()
            .map(|c| c.pending_commission_cents)
            .unwrap_or(0),
        payable_commission_cents: counter
            .as_ref()
            .map(|c| c.payable_commission_cents)
            .unwrap_or(0),
        paid_commission_cents: counter
            .as_ref()
            .map(|c| c.paid_commission_cents)
            .unwrap_or(0),
        clawed_back_cents: counter.as_ref().map(|c| c.clawed_back_cents).unwrap_or(0),
        active_members,
        top_performers,
        deficit_members,
        top_assets,
        next_payout_date: next_payout,
        next_payout_amount_cents: counter
            .as_ref()
            .map(|c| c.payable_commission_cents)
            .unwrap_or(0),
        open_payout_requests,
        computed_at: chrono::Utc::now(),
    })
}

/// Daily time-series across the requested range — one bucket per calendar
/// day even when a day has no activity (gaps filled by generate_series).
pub async fn team_daily_timeseries(
    pool: &PgPool,
    team_id: Uuid,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
) -> Result<Vec<DailyPoint>, AppError> {
    let rows: Vec<DailyPoint> = sqlx::query_as(
        r#"WITH days AS (
              SELECT generate_series($2::date, $3::date, INTERVAL '1 day')::date AS d
           ),
           agg AS (
              SELECT rollup_date,
                     SUM(clicks_count)::BIGINT          AS clicks_count,
                     SUM(signups_count)::BIGINT         AS signups_count,
                     SUM(qualified_count)::BIGINT       AS qualified_count,
                     SUM(gross_revenue_cents)::BIGINT   AS gross_revenue_cents,
                     SUM(commission_cents)::BIGINT      AS commission_cents
              FROM affiliate_daily_rollups
              WHERE team_id = $1
                AND rollup_date BETWEEN $2 AND $3
              GROUP BY rollup_date
           )
           SELECT d.d                                       AS bucket_date,
                  COALESCE(a.clicks_count, 0)::BIGINT       AS clicks_count,
                  COALESCE(a.signups_count, 0)::BIGINT      AS signups_count,
                  COALESCE(a.qualified_count, 0)::BIGINT    AS qualified_count,
                  COALESCE(a.gross_revenue_cents, 0)::BIGINT AS gross_revenue_cents,
                  COALESCE(a.commission_cents, 0)::BIGINT   AS commission_cents
           FROM days d
           LEFT JOIN agg a ON a.rollup_date = d.d
           ORDER BY d.d"#,
    )
    .bind(team_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Period summary für ein Team (oder einen einzelnen Member innerhalb des Teams).
/// `from_date`/`to_date` inklusive (DATE).
pub async fn team_period_summary(
    pool: &PgPool,
    team_id: Uuid,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
) -> Result<PeriodSummary, AppError> {
    let row: PeriodSummary = sqlx::query_as(
        r#"SELECT
              COALESCE(SUM(clicks_count), 0)::BIGINT          AS clicks_count,
              COALESCE(SUM(signups_count), 0)::BIGINT         AS signups_count,
              COALESCE(SUM(qualified_count), 0)::BIGINT       AS qualified_count,
              COALESCE(SUM(gross_revenue_cents), 0)::BIGINT   AS gross_revenue_cents,
              COALESCE(SUM(commission_cents), 0)::BIGINT      AS commission_cents
           FROM affiliate_daily_rollups
           WHERE team_id = $1
             AND rollup_date BETWEEN $2 AND $3"#,
    )
    .bind(team_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Member-Breakdown: GROUP BY attribution_user_id für Team-Business-Links
/// innerhalb des Teams.
pub async fn team_period_by_member(
    pool: &PgPool,
    team_id: Uuid,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
) -> Result<Vec<MemberBreakdown>, AppError> {
    let rows: Vec<MemberBreakdown> = sqlx::query_as(
        r#"SELECT
              ar.attribution_user_id,
              u.email::text                                             AS email,
              NULLIF(TRIM(BOTH ' ' FROM (
                  COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, '')
              )), '')                                                    AS full_name,
              COALESCE(SUM(ar.clicks_count), 0)::BIGINT                  AS clicks_count,
              COALESCE(SUM(ar.signups_count), 0)::BIGINT                 AS signups_count,
              COALESCE(SUM(ar.qualified_count), 0)::BIGINT               AS qualified_count,
              COALESCE(SUM(ar.gross_revenue_cents), 0)::BIGINT           AS gross_revenue_cents,
              COALESCE(SUM(ar.commission_cents), 0)::BIGINT              AS commission_cents
           FROM affiliate_daily_rollups ar
           LEFT JOIN users u          ON u.id = ar.attribution_user_id
           LEFT JOIN user_profiles up ON up.user_id = ar.attribution_user_id
           WHERE ar.team_id = $1
             AND ar.rollup_date BETWEEN $2 AND $3
           GROUP BY ar.attribution_user_id, u.email, up.first_name, up.last_name
           ORDER BY commission_cents DESC NULLS LAST"#,
    )
    .bind(team_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Customer-Liste für ein Team. Optional gefiltert nach attribution_user_id.
/// Per User-Wahl darf Developer Vor- und Nachname sehen.
pub async fn team_customers(
    pool: &PgPool,
    team_id: Uuid,
    attribution_user_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TeamCustomer>, AppError> {
    // Performance note: previous version had two per-row scalar subqueries
    // (sum(investments) + sum(commissions)) — classic N+1 in SQL form,
    // O(rows × child-table-scan). Rewritten as LATERAL joins so the planner
    // can use indexed lookups + the per-row work happens once per result
    // row inside a join, not nested twice.
    let rows: Vec<TeamCustomer> = sqlx::query_as(
        r#"SELECT
              ar.referred_user_id,
              NULLIF(TRIM(BOTH ' ' FROM (
                  COALESCE(rup.first_name, '') || ' ' || COALESCE(rup.last_name, '')
              )), '')                                                    AS full_name,
              ru.email::text                                              AS email,
              ar.attribution_user_id,
              NULLIF(TRIM(BOTH ' ' FROM (
                  COALESCE(aup.first_name, '') || ' ' || COALESCE(aup.last_name, '')
              )), '')                                                    AS attribution_user_name,
              ar.status                                                   AS referral_status,
              COALESCE(ar.created_at, NOW())                              AS created_at,
              COALESCE(inv.sum_cents, 0)::BIGINT                          AS gross_invested_cents,
              COALESCE(com.sum_cents, 0)::BIGINT                          AS commission_earned_cents
           FROM affiliate_referrals ar
           JOIN affiliate_links al ON al.id = ar.link_id
           LEFT JOIN users         ru  ON ru.id  = ar.referred_user_id
           LEFT JOIN user_profiles rup ON rup.user_id = ar.referred_user_id
           LEFT JOIN user_profiles aup ON aup.user_id = ar.attribution_user_id
           LEFT JOIN LATERAL (
               SELECT SUM(i.purchase_value_cents) AS sum_cents
               FROM investments i
               WHERE i.user_id = ar.referred_user_id AND i.status = 'active'
           ) AS inv ON true
           LEFT JOIN LATERAL (
               SELECT SUM(ac.provisional_amount_cents) AS sum_cents
               FROM affiliate_commissions ac
               WHERE ac.referral_id = ar.id
           ) AS com ON true
           WHERE al.team_id = $1
             AND al.link_type = 'team_business'
             AND ($2::uuid IS NULL OR ar.attribution_user_id = $2)
           ORDER BY ar.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(team_id)
    .bind(attribution_user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Produkt-Aggregation: welche Assets wurden über Team-Business-Links verkauft.
pub async fn team_products(
    pool: &PgPool,
    team_id: Uuid,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
) -> Result<Vec<TeamProductSale>, AppError> {
    // `ac.created_at::date BETWEEN $2 AND $3` is non-sargable — the cast
    // disables the index range scan. Rewritten as half-open range that
    // can use idx_commissions_link_created_desc (migration 165).
    // Bug 1+2 fix: derive product-level sales from order_items joined to the
    // commission's source_order_id (which is `orders.id`, NOT investments.id).
    // Gross revenue comes from `ac.gross_amount_cents` which stores the
    // commission's source order total at INSERT time — no dependency on
    // cumulative UPSERTed `investments.purchase_value_cents`.
    //
    // Per-asset breakdown joins via order_items so multi-asset orders
    // attribute correctly to each asset's row.
    let rows: Vec<TeamProductSale> = sqlx::query_as(
        r#"WITH commission_orders AS (
              SELECT ac.id              AS commission_id,
                     ac.source_order_id AS order_id,
                     ac.gross_amount_cents,
                     ac.provisional_amount_cents
              FROM affiliate_commissions ac
              JOIN affiliate_links al ON al.id = ac.link_id
              WHERE al.team_id = $1
                AND al.link_type = 'team_business'
                AND ac.created_at >= $2::date
                AND ac.created_at <  ($3::date + INTERVAL '1 day')
           )
           SELECT
              oi.asset_id                                        AS asset_id,
              a.title                                            AS asset_name,
              COUNT(DISTINCT oi.id)::BIGINT                      AS units_sold,
              COALESCE(SUM(oi.tokens_quantity * oi.token_price_cents)::BIGINT, 0)
                                                                 AS gross_revenue_cents,
              COALESCE(SUM(co.provisional_amount_cents
                  * (oi.tokens_quantity * oi.token_price_cents)
                  / NULLIF(co.gross_amount_cents, 0)
              )::BIGINT, 0)                                       AS commission_cents
           FROM commission_orders co
           JOIN order_items oi ON oi.order_id = co.order_id
           LEFT JOIN assets a  ON a.id = oi.asset_id
           GROUP BY oi.asset_id, a.title
           ORDER BY gross_revenue_cents DESC"#,
    )
    .bind(team_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Member-Personal-Summary (für Mitarbeiter-Self-View Personal-Mode).
/// Filtert ausschließlich Personal-Links, ignoriert Team-Business-Commissions.
pub async fn member_personal_period_summary(
    pool: &PgPool,
    user_id: Uuid,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
) -> Result<PeriodSummary, AppError> {
    let row: PeriodSummary = sqlx::query_as(
        r#"SELECT
              COALESCE(SUM(clicks_count), 0)::BIGINT       AS clicks_count,
              COALESCE(SUM(signups_count), 0)::BIGINT      AS signups_count,
              COALESCE(SUM(qualified_count), 0)::BIGINT    AS qualified_count,
              COALESCE(SUM(gross_revenue_cents), 0)::BIGINT AS gross_revenue_cents,
              COALESCE(SUM(commission_cents), 0)::BIGINT   AS commission_cents
           FROM affiliate_daily_rollups
           WHERE payout_user_id = $1
             AND link_type = 'personal'
             AND rollup_date BETWEEN $2 AND $3"#,
    )
    .bind(user_id)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

// ═══════════════════════════════════════════════════════════════════════════
// Phase-4: Cohort retention + revenue forecast
// ═══════════════════════════════════════════════════════════════════════════
//
// `team_cohort_retention` returns a row per (cohort_month, period_index)
// where:
//   * cohort_month  = month the referred user signed up
//   * period_index  = N months after cohort_month (0 = same month)
//   * active_users  = customers from that cohort with a paid commission
//                     in period_index
//   * cohort_size   = total customers in the cohort
//
// `team_revenue_forecast` projects the next 30 days of commission
// revenue using a trailing-30-day linear average. The frontend draws it
// as a forecast extension on the analytics line chart.

#[derive(Debug, Serialize)]
pub struct CohortCell {
    pub cohort_month: String,
    pub period_index: i32,
    pub cohort_size: i64,
    pub active_users: i64,
}

pub async fn team_cohort_retention(
    pool: &PgPool,
    team_id: Uuid,
    months_back: i32,
) -> Result<Vec<CohortCell>, AppError> {
    let m = months_back.clamp(1, 24).to_string();
    let rows = sqlx::query!(
        r#"
        WITH team_members AS (
            SELECT user_id FROM developer_team_memberships
             WHERE team_id = $1 AND status = 'active'
        ),
        cohorts AS (
            SELECT ar.referred_user_id,
                   date_trunc('month', ar.created_at)::date AS cohort_month
              FROM affiliate_referrals ar
             WHERE ar.attribution_user_id IN (SELECT user_id FROM team_members)
               AND ar.created_at >= NOW() - ($2 || ' months')::INTERVAL
        ),
        cohort_sizes AS (
            SELECT cohort_month, COUNT(DISTINCT referred_user_id) AS cohort_size
              FROM cohorts GROUP BY cohort_month
        ),
        commissions_per_cohort AS (
            SELECT c.cohort_month,
                   ((EXTRACT(YEAR FROM ac.updated_at) - EXTRACT(YEAR FROM c.cohort_month)) * 12
                   + (EXTRACT(MONTH FROM ac.updated_at) - EXTRACT(MONTH FROM c.cohort_month)))::INT
                       AS period_index_raw,
                   ar.referred_user_id
              FROM cohorts c
              JOIN affiliate_referrals ar ON ar.referred_user_id = c.referred_user_id
              JOIN affiliate_commissions ac ON ac.referral_id = ar.id
             WHERE ac.status = 'paid'
        )
        SELECT to_char(s.cohort_month, 'YYYY-MM')           AS "cohort_month!",
               COALESCE(cpc.period_index_raw, 0)            AS "period_index!",
               s.cohort_size::BIGINT                        AS "cohort_size!",
               COUNT(DISTINCT cpc.referred_user_id)::BIGINT AS "active_users!"
          FROM cohort_sizes s
          LEFT JOIN commissions_per_cohort cpc ON cpc.cohort_month = s.cohort_month
         GROUP BY s.cohort_month, s.cohort_size, cpc.period_index_raw
         ORDER BY s.cohort_month ASC, period_index_raw ASC NULLS FIRST
        "#,
        team_id,
        m
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| CohortCell {
            cohort_month: r.cohort_month,
            period_index: r.period_index,
            cohort_size: r.cohort_size,
            active_users: r.active_users,
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct RevenueForecast {
    pub method: &'static str,
    pub trailing_days: i32,
    pub trailing_total_cents: i64,
    pub daily_avg_cents: i64,
    pub next_30d_projection_cents: i64,
}

pub async fn team_revenue_forecast(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<RevenueForecast, AppError> {
    let trailing_days = 30_i32;
    let trailing_total_cents: i64 = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(ac.provisional_amount_cents), 0)::BIGINT AS "t!"
             FROM affiliate_commissions ac
             JOIN affiliate_referrals ar ON ar.id = ac.referral_id
            WHERE ar.attribution_user_id IN (
                  SELECT user_id FROM developer_team_memberships
                   WHERE team_id = $1 AND status = 'active'
            )
              AND ac.status IN ('paid', 'payable')
              AND ac.created_at >= NOW() - INTERVAL '30 days'"#,
        team_id
    )
    .fetch_one(pool)
    .await?;
    let daily_avg_cents = trailing_total_cents / (trailing_days as i64);
    let next_30d_projection_cents = daily_avg_cents * 30;
    Ok(RevenueForecast {
        method: "trailing_30d_linear",
        trailing_days,
        trailing_total_cents,
        daily_avg_cents,
        next_30d_projection_cents,
    })
}
