use super::models::{AnnualLimit, InvestmentItem, PortfolioResponse};
use uuid::Uuid;

pub async fn get_portfolio(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<PortfolioResponse, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT
            i.id,
            i.asset_id,
            a.title AS asset_title,
            a.slug  AS asset_slug,
            (
                SELECT image_url 
                FROM asset_images 
                WHERE asset_id = a.id 
                ORDER BY is_cover DESC, sort_order ASC 
                LIMIT 1
            ) AS cover_image,
            i.tokens_owned,
            i.purchase_value_cents,
            (i.tokens_owned * a.token_price_cents) AS "current_value_cents!",
            i.total_rental_cents,
            CASE WHEN i.purchase_value_cents > 0
                 THEN (((i.tokens_owned * a.token_price_cents) - i.purchase_value_cents) * 10000 / i.purchase_value_cents)::INT
                 ELSE 0
            END AS "appreciation_pct_bps!",
            i.status,
            i.payout_expected_at,
            i.purchased_at,
            COALESCE(a.occupancy_rate_bps, 0) AS "occupancy_rate_bps!",
            COALESCE(a.annual_yield_bps, 0) AS "annual_yield_bps!"
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE i.user_id = $1
        ORDER BY i.purchased_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    // Calculate monthly income from dividend_payouts this month
    let monthly_income_row = sqlx::query!(
        r#"
        SELECT COALESCE(SUM(amount_cents), 0)::BIGINT as "total!"
        FROM dividend_payouts
        WHERE user_id = $1
          AND payout_type = 'rental'
          AND status = 'paid'
          AND EXTRACT(MONTH FROM paid_at) = EXTRACT(MONTH FROM NOW())
          AND EXTRACT(YEAR FROM paid_at) = EXTRACT(YEAR FROM NOW())
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    let monthly_income_cents = monthly_income_row.total;

    let annual_limit_row = sqlx::query!(
        r#"
        SELECT annual_limit_cents, invested_12m_cents, available_cents, limit_year
        FROM investment_limits
        WHERE user_id = $1 AND limit_year = EXTRACT(YEAR FROM NOW())::INTEGER
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    // Build response
    let total_value_cents: i64 = rows.iter().map(|r| r.current_value_cents).sum();
    let total_purchase_cents: i64 = rows.iter().map(|r| r.purchase_value_cents).sum();
    let total_rental_cents: i64 = rows.iter().map(|r| r.total_rental_cents).sum();
    let total_appreciation_cents = total_value_cents - total_purchase_cents;

    let occupancy_rate_bps = if total_value_cents > 0 {
        (rows
            .iter()
            .map(|r| r.occupancy_rate_bps as i64 * r.current_value_cents)
            .sum::<i64>()
            / total_value_cents) as i32
    } else {
        0
    };

    let annual_yield_bps = if total_value_cents > 0 {
        (rows
            .iter()
            .map(|r| r.annual_yield_bps as i64 * r.current_value_cents)
            .sum::<i64>()
            / total_value_cents) as i32
    } else {
        0
    };

    let investment_count = rows.len();

    let investments = rows
        .into_iter()
        .map(|r| InvestmentItem {
            id: r.id,
            asset_id: r.asset_id,
            asset_title: r.asset_title,
            asset_slug: r.asset_slug,
            cover_image: r.cover_image,
            tokens_owned: r.tokens_owned,
            purchase_value_cents: r.purchase_value_cents,
            current_value_cents: r.current_value_cents,
            total_rental_cents: r.total_rental_cents,
            appreciation_pct_bps: r.appreciation_pct_bps,
            status: r.status,
            payout_expected_at: r.payout_expected_at.map(|t| t.to_rfc3339()),
            purchased_at: r.purchased_at.to_rfc3339(),
        })
        .collect();

    Ok(PortfolioResponse {
        investments,
        total_value_cents,
        total_purchase_cents,
        total_rental_cents,
        total_appreciation_cents,
        monthly_income_cents,
        occupancy_rate_bps,
        annual_yield_bps,
        investment_count,
        annual_limit: annual_limit_row.map(|l| AnnualLimit {
            annual_limit_cents: l.annual_limit_cents,
            invested_12m_cents: l.invested_12m_cents,
            available_cents: l.available_cents.unwrap_or(0),
            limit_year: l.limit_year,
        }),
    })
}
