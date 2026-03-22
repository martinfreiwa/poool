use super::models::{AnnualLimit, InvestmentItem, PortfolioResponse};
use uuid::Uuid;

pub async fn get_portfolio(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<PortfolioResponse, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
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
            (i.tokens_owned * a.token_price_cents) AS current_value_cents,
            i.total_rental_cents,
            CASE WHEN i.purchase_value_cents > 0
                 THEN (((i.tokens_owned * a.token_price_cents) - i.purchase_value_cents) * 10000 / i.purchase_value_cents)::INT
                 ELSE 0
            END AS appreciation_pct_bps,
            i.status,
            i.payout_expected_at,
            i.purchased_at,
            COALESCE(a.occupancy_rate_bps, 0) AS occupancy_rate_bps,
            COALESCE(a.annual_yield_bps, 0) AS annual_yield_bps,
            a.chain_contract_address,
            a.chain_tx_hash
        FROM investments i
        JOIN assets a ON a.id = i.asset_id
        WHERE i.user_id = $1
        ORDER BY i.purchased_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut total_value_cents: i64 = 0;
    let mut total_purchase_cents: i64 = 0;
    let mut total_rental_cents: i64 = 0;
    let mut total_occupancy_weighted: i64 = 0;
    let mut total_yield_weighted: i64 = 0;

    let mut mapped_rows = Vec::new();

    for r in &rows {
        let id: Uuid = r.get("id");
        let asset_id: Uuid = r.get("asset_id");
        let asset_title: String = r.get("asset_title");
        let asset_slug: String = r.get("asset_slug");
        let cover_image: Option<String> = r.get("cover_image");
        let tokens_owned: i32 = r.get("tokens_owned");
        let purchase_value_cents: i64 = r.get("purchase_value_cents");
        let current_value_cents: i64 = r.get("current_value_cents");
        let total_rental_cents_val: i64 = r.get("total_rental_cents");
        let appreciation_pct_bps: i32 = r.get("appreciation_pct_bps");

        let status: String = r.get("status");
        let payout_expected_at: Option<chrono::DateTime<chrono::Utc>> = r.get("payout_expected_at");
        let purchased_at: chrono::DateTime<chrono::Utc> = r.get("purchased_at");
        let occupancy_rate_bps: i32 = r.get("occupancy_rate_bps");
        let annual_yield_bps: i32 = r.get("annual_yield_bps");
        let chain_contract_address: Option<String> = r.get("chain_contract_address");
        let chain_tx_hash: Option<String> = r.get("chain_tx_hash");

        total_value_cents += current_value_cents;
        total_purchase_cents += purchase_value_cents;
        total_rental_cents += total_rental_cents_val;
        total_occupancy_weighted += occupancy_rate_bps as i64 * current_value_cents;
        total_yield_weighted += annual_yield_bps as i64 * current_value_cents;

        mapped_rows.push(InvestmentItem {
            id,
            asset_id,
            asset_title,
            asset_slug,
            cover_image,
            tokens_owned,
            purchase_value_cents,
            current_value_cents,
            total_rental_cents: total_rental_cents_val,
            appreciation_pct_bps,

            status,
            payout_expected_at: payout_expected_at.map(|t| t.to_rfc3339()),
            purchased_at: purchased_at.to_rfc3339(),
            is_within_48h: (chrono::Utc::now().signed_duration_since(purchased_at))
                <= chrono::Duration::hours(48),
            chain_contract_address,
            chain_tx_hash,
        });
    }

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
    let total_appreciation_cents = total_value_cents - total_purchase_cents;

    let occupancy_rate_bps = if total_value_cents > 0 {
        (total_occupancy_weighted / total_value_cents) as i32
    } else {
        0
    };

    let annual_yield_bps = if total_value_cents > 0 {
        (total_yield_weighted / total_value_cents) as i32
    } else {
        0
    };

    let investment_count = mapped_rows.len();

    let investments = mapped_rows;

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

/// 48-Hour Cooling-Off Cancellation Logic (Phase 17.1)
pub async fn cancel_investment(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    investment_id: Uuid,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    use sqlx::Row;
    let inv_opt = sqlx::query(
        r#"
        SELECT asset_id, tokens_owned, purchase_value_cents, status,
               ((NOW() - purchased_at) <= INTERVAL '48 HOURS') AS is_within_48h
        FROM investments
        WHERE id = $1 AND user_id = $2
        FOR UPDATE
        "#,
    )
    .bind(investment_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    let inv = inv_opt.ok_or_else(|| "Investment not found".to_string())?;

    let inv_asset_id: Uuid = inv.get("asset_id");
    let inv_tokens_owned: i32 = inv.get("tokens_owned");
    let inv_purchase_value_cents: i64 = inv.get("purchase_value_cents");
    let inv_status: String = inv.get("status");
    let inv_is_within_48h: bool = inv.get("is_within_48h");

    if inv_status != "funding_in_progress" {
        return Err("Investment cannot be cancelled in its current state".to_string());
    }

    if !inv_is_within_48h {
        return Err("The 48-hour cooling-off period has expired for this investment.".to_string());
    }

    // 1. Credit wallet
    let wallet_id: Uuid = sqlx::query_scalar!(
        r#"
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES ($1, 'cash', 'USD', $2)
        ON CONFLICT (user_id, wallet_type, currency) DO UPDATE
        SET balance_cents = wallets.balance_cents + $2, updated_at = NOW()
        RETURNING id
        "#,
        user_id,
        inv_purchase_value_cents
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // 2. Log Transaction
    sqlx::query!(
        r#"
        INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description)
        VALUES ($1, 'refund', 'completed', $2, 'USD', '48h Cooling-off period cancellation refund')
        "#,
        wallet_id, inv_purchase_value_cents
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // 3. Mark as refunded
    sqlx::query!(
        "UPDATE investments SET status = 'refunded', updated_at = NOW() WHERE id = $1",
        investment_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // 4. Replenish Asset available tokens
    sqlx::query!(
        "UPDATE assets SET tokens_available = tokens_available + $1, updated_at = NOW() WHERE id = $2",
        inv_tokens_owned, inv_asset_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    // 5. Best-effort cancel associated pending orders
    sqlx::query!(
        r#"
        UPDATE orders SET status = 'failed'
        WHERE user_id = $1 AND status = 'pending' AND id IN (
            SELECT order_id FROM order_items WHERE asset_id = $2
        )
        "#,
        user_id,
        inv_asset_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(())
}
