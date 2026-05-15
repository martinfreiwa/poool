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

#[derive(Debug, Serialize, sqlx::FromRow)]
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TeamProductSale {
    pub asset_id: Uuid,
    pub asset_name: Option<String>,
    pub units_sold: i64,
    pub gross_revenue_cents: i64,
    pub commission_cents: i64,
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
    let rows: Vec<TeamProductSale> = sqlx::query_as(
        r#"SELECT
              i.asset_id,
              a.title                                                     AS asset_name,
              COUNT(DISTINCT i.id)::BIGINT                                AS units_sold,
              COALESCE(SUM(i.purchase_value_cents)::BIGINT, 0)            AS gross_revenue_cents,
              COALESCE(SUM(ac.provisional_amount_cents)::BIGINT, 0)       AS commission_cents
           FROM affiliate_commissions ac
           JOIN affiliate_links al ON al.id = ac.link_id
           JOIN investments    i  ON i.id  = ac.source_order_id
           LEFT JOIN assets    a  ON a.id  = i.asset_id
           WHERE al.team_id = $1
             AND al.link_type = 'team_business'
             AND ac.created_at >= $2::date
             AND ac.created_at <  ($3::date + INTERVAL '1 day')
           GROUP BY i.asset_id, a.title
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
