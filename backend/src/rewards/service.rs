use super::models::{
    CommissionRecord, PayoutSettings, RewardsOverview, SavePayoutSettingsForm, TierInfo,
};
use crate::error::AppError;
use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

/// Recalculates the user's invested_12m and tier_id based on active investments in the last 12 months.
pub async fn recalculate_user_tier(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let sum: Option<i64> = sqlx::query_scalar!(
        r#"
        SELECT SUM(purchase_value_cents)::BIGINT
        FROM investments
        WHERE user_id = $1 
          AND status = 'active'
          AND purchased_at >= NOW() - INTERVAL '1 year'
        "#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    let invested_12m = sum.unwrap_or(0);

    let tier_id_opt: Option<i32> = sqlx::query_scalar!(
        r#"
        SELECT id FROM tiers
        WHERE min_invest <= $1
        ORDER BY min_invest DESC
        LIMIT 1
        "#,
        invested_12m
    )
    .fetch_optional(pool)
    .await?;

    let tier_id = match tier_id_opt {
        Some(t) => t,
        None => {
            // Fallback to Intro tier (sort_order = 1) if something goes wrong
            sqlx::query_scalar!("SELECT id FROM tiers WHERE sort_order = 1 LIMIT 1")
                .fetch_optional(pool)
                .await?
                .unwrap_or(1)
        }
    };

    sqlx::query!(
        r#"
        INSERT INTO user_tiers (user_id, tier_id, invested_12m, updated_at)
        VALUES ($1, $2, $3, NOW())
        ON CONFLICT (user_id) DO UPDATE SET
            tier_id = EXCLUDED.tier_id,
            invested_12m = EXCLUDED.invested_12m,
            updated_at = NOW()
        "#,
        user_id,
        tier_id,
        invested_12m
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_rewards_overview(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<RewardsOverview, AppError> {
    // 0. Recalculate tier based on rolling 12m active investments
    recalculate_user_tier(pool, user_id).await?;

    // 1. Fetch rewards_balances
    let balances = sqlx::query!(
        r#"
        SELECT cashback, referrals, promotions
        FROM rewards_balances
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let (cashback, referrals, promotions) = match balances {
        Some(b) => (b.cashback, b.referrals, b.promotions),
        None => (0, 0, 0),
    };

    let total_balance = cashback + referrals + promotions;

    // 2. Fetch user tier info
    let tier_data = sqlx::query!(
        r#"
        SELECT t.name as tier_name, ut.invested_12m
        FROM user_tiers ut
        JOIN tiers t ON ut.tier_id = t.id
        WHERE ut.user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let (tier_name, invested_12m) = match tier_data {
        Some(t) => (t.tier_name, t.invested_12m),
        None => ("Intro".to_string(), 0),
    };

    // 3. Find next tier
    let next_tier_row = sqlx::query!(
        r#"
        SELECT name, min_invest
        FROM tiers
        WHERE sort_order > (
            SELECT sort_order FROM tiers WHERE name = $1
        )
        ORDER BY sort_order ASC
        LIMIT 1
        "#,
        tier_name
    )
    .fetch_optional(pool)
    .await?;

    let mut tier_target = None;
    let mut tier_target_amount = None;
    let mut progress_pct = 100; // If max tier (Premium), progress is 100%

    if let Some(nt) = next_tier_row {
        tier_target = Some(nt.name.clone());
        tier_target_amount = Some(nt.min_invest);

        // Calculate progress percentage based on current tier's min_invest
        let current_tier_min =
            sqlx::query!("SELECT min_invest FROM tiers WHERE name = $1", tier_name)
                .fetch_optional(pool)
                .await?
                .map(|r| r.min_invest)
                .unwrap_or(0);

        let range = nt.min_invest - current_tier_min;
        let progress = invested_12m - current_tier_min;

        if range > 0 {
            let pct = (progress as f64 / range as f64) * 100.0;
            progress_pct = pct.clamp(0.0, 100.0) as i32;
        } else {
            progress_pct = 0;
        }
    }

    // 4. Fetch or generate referral code
    let ref_code_row = sqlx::query!(
        r#"
        SELECT code
        FROM referral_codes
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let (referral_code, referral_url) = match ref_code_row {
        Some(r) => {
            let url = format!("https://app.poool.com/rewards/{}", r.code);
            (Some(r.code), Some(url))
        }
        None => {
            // Generate a fresh pseudo-random code
            let new_code = uuid::Uuid::new_v4()
                .simple()
                .to_string()
                .chars()
                .take(8)
                .collect::<String>()
                .to_string();

            // Try to insert it
            let inserted = sqlx::query!(
                "INSERT INTO referral_codes (user_id, code) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING code",
                user_id,
                new_code
            )
            .fetch_optional(pool)
            .await?;

            if let Some(ins) = inserted {
                let url = format!("https://app.poool.com/rewards/{}", ins.code);
                (Some(ins.code), Some(url))
            } else {
                // If it conflicted (extremely rare for uuid), just return None for now
                (None, None)
            }
        }
    };

    // 5. Fetch Partner Metrics
    let mut total_clicks = 0;
    let mut total_signups = 0;
    let mut qualified_investors = 0;
    let mut network_total_in = 0;

    if let Some(ref code) = referral_code {
        // Clicks
        let clicks_row = sqlx::query!(
            "SELECT count(*) as count FROM referral_clicks WHERE code = $1",
            code
        )
        .fetch_one(pool)
        .await?;
        total_clicks = clicks_row.count.unwrap_or(0);

        // Signups
        let signups_row = sqlx::query!(
            "SELECT count(*) as count FROM referral_tracking WHERE referrer_id = $1",
            user_id
        )
        .fetch_one(pool)
        .await?;
        total_signups = signups_row.count.unwrap_or(0);

        // Qualified Investors
        let qualified_row = sqlx::query!("SELECT count(*) as count FROM referral_tracking WHERE referrer_id = $1 AND status = 'qualified'", user_id)
            .fetch_one(pool)
            .await?;
        qualified_investors = qualified_row.count.unwrap_or(0);

        // Network Total In
        let network_row = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(inv.purchase_value_cents), 0)::BIGINT as total
            FROM investments inv
            JOIN referral_tracking rt ON inv.user_id = rt.referred_id
            WHERE rt.referrer_id = $1 AND inv.status = 'active'
            "#,
            user_id
        )
        .fetch_one(pool)
        .await?;
        network_total_in = network_row.total.unwrap_or(0);
    }

    Ok(RewardsOverview {
        total_balance,
        cashback,
        referrals,
        promotions,
        tier_name,
        tier_target,
        tier_target_amount,
        invested_12m,
        progress_pct,
        referral_code,
        referral_url,
        total_clicks,
        total_signups,
        qualified_investors,
        network_total_in,
    })
}

pub async fn get_all_tiers(pool: &PgPool) -> Result<Vec<TierInfo>, AppError> {
    let tiers = sqlx::query_as!(
        TierInfo,
        r#"
        SELECT 
            id, 
            name, 
            min_invest, 
            badge_color, 
            sort_order, 
            cashback_pct::float8 as "cashback_pct!"
        FROM tiers
        ORDER BY sort_order ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(tiers)
}

/// Returns per-campaign (subid) metrics for the given user's referral code.
pub async fn get_campaign_breakdown(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<super::models::CampaignMetrics>, AppError> {
    // 1. Get user's referral code
    let code_row = sqlx::query!(
        "SELECT code FROM referral_codes WHERE user_id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let code = match code_row {
        Some(r) => r.code,
        None => return Ok(vec![]),
    };

    // 2. Clicks per subid
    let click_rows = sqlx::query!(
        r#"
        SELECT COALESCE(subid, '(direct)') as "subid!", count(*) as "clicks!"
        FROM referral_clicks
        WHERE code = $1
        GROUP BY subid
        "#,
        code
    )
    .fetch_all(pool)
    .await?;

    // 3. Signups per subid
    let signup_rows = sqlx::query!(
        r#"
        SELECT COALESCE(subid, '(direct)') as "subid!", count(*) as "signups!"
        FROM referral_tracking
        WHERE referrer_id = $1
        GROUP BY subid
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    // 4. Qualified per subid
    let qualified_rows = sqlx::query!(
        r#"
        SELECT COALESCE(subid, '(direct)') as "subid!", count(*) as "qualified!"
        FROM referral_tracking
        WHERE referrer_id = $1 AND status = 'qualified'
        GROUP BY subid
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    // 5. Revenue per subid
    let revenue_rows = sqlx::query!(
        r#"
        SELECT COALESCE(rt.subid, '(direct)') as "subid!", 
               COALESCE(SUM(inv.purchase_value_cents), 0)::BIGINT as "revenue!"
        FROM referral_tracking rt
        JOIN investments inv ON inv.user_id = rt.referred_id AND inv.status = 'active'
        WHERE rt.referrer_id = $1
        GROUP BY rt.subid
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    // 6. Merge everything into a HashMap keyed by subid
    use std::collections::HashMap;
    let mut map: HashMap<String, super::models::CampaignMetrics> = HashMap::new();

    for r in click_rows {
        let entry = map
            .entry(r.subid.clone())
            .or_insert_with(|| super::models::CampaignMetrics {
                subid: r.subid.clone(),
                clicks: 0,
                signups: 0,
                qualified: 0,
                revenue_cents: 0,
                cvr: 0.0,
            });
        entry.clicks = r.clicks;
    }

    for r in signup_rows {
        let entry = map
            .entry(r.subid.clone())
            .or_insert_with(|| super::models::CampaignMetrics {
                subid: r.subid.clone(),
                clicks: 0,
                signups: 0,
                qualified: 0,
                revenue_cents: 0,
                cvr: 0.0,
            });
        entry.signups = r.signups;
    }

    for r in qualified_rows {
        let entry = map
            .entry(r.subid.clone())
            .or_insert_with(|| super::models::CampaignMetrics {
                subid: r.subid.clone(),
                clicks: 0,
                signups: 0,
                qualified: 0,
                revenue_cents: 0,
                cvr: 0.0,
            });
        entry.qualified = r.qualified;
    }

    for r in revenue_rows {
        let entry = map
            .entry(r.subid.clone())
            .or_insert_with(|| super::models::CampaignMetrics {
                subid: r.subid.clone(),
                clicks: 0,
                signups: 0,
                qualified: 0,
                revenue_cents: 0,
                cvr: 0.0,
            });
        entry.revenue_cents = r.revenue;
    }

    // 7. Compute CVR
    let mut result: Vec<super::models::CampaignMetrics> = map.into_values().collect();
    for m in &mut result {
        m.cvr = if m.clicks > 0 {
            (m.signups as f64 / m.clicks as f64) * 100.0
        } else {
            0.0
        };
    }

    // Sort by clicks descending
    result.sort_by(|a, b| b.clicks.cmp(&a.clicks));

    Ok(result)
}

/// Checks if a referred user has met the investment threshold ($1000 = 100_000 cents).
/// If so, marks the referral as qualified and pays out the reward to both parties.
pub async fn check_and_qualify_referral(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
) -> Result<(), AppError> {
    // 1. Check if this user was referred and is currently in 'pending' status
    let pending_referral = sqlx::query!(
        r#"
        SELECT id, referrer_id, referrer_reward, referred_reward
        FROM referral_tracking
        WHERE referred_id = $1 AND status = 'pending'
        "#,
        user_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let referral = match pending_referral {
        Some(r) => r,
        None => return Ok(()), // Not referred, or already qualified/paid
    };

    // 2. Check total USD invested by this user
    let total_invested: Option<i64> = sqlx::query_scalar!(
        r#"
        SELECT SUM(purchase_value_cents)::BIGINT
        FROM investments
        WHERE user_id = $1 AND status = 'active'
        "#,
        user_id
    )
    .fetch_one(&mut **tx)
    .await?;

    let total = total_invested.unwrap_or(0);

    // 3. If >= $1,000 (100,000 cents), qualify the referral and pay out to balances
    if total >= 100_000 {
        tracing::info!(
            "User {} has invested >= $1000, qualifying referral {}",
            user_id,
            referral.id
        );

        // Update tracking status
        sqlx::query!(
            "UPDATE referral_tracking SET status = 'qualified', qualified_at = NOW() WHERE id = $1",
            referral.id
        )
        .execute(&mut **tx)
        .await?;

        // Pay out referrer
        sqlx::query!(
            r#"
            INSERT INTO rewards_balances (user_id, referrals)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE
            SET referrals = rewards_balances.referrals + $2,
                updated_at = NOW()
            "#,
            referral.referrer_id,
            referral.referrer_reward
        )
        .execute(&mut **tx)
        .await?;

        // Pay out referred
        sqlx::query!(
            r#"
            INSERT INTO rewards_balances (user_id, referrals)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE
            SET referrals = rewards_balances.referrals + $2,
                updated_at = NOW()
            "#,
            user_id,
            referral.referred_reward
        )
        .execute(&mut **tx)
        .await?;

        // Audit Log for Referrer
        sqlx::query!(
            r#"
            INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
            VALUES ($1, 'REFERRAL_REWARD_ISSUED', 'rewards', $2, $3)
            "#,
            referral.referrer_id,
            referral.id,
            serde_json::json!({ "amount": referral.referrer_reward, "role": "referrer" })
        )
        .execute(&mut **tx)
        .await?;

        // Audit Log for Referred
        sqlx::query!(
            r#"
            INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
            VALUES ($1, 'REFERRAL_REWARD_ISSUED', 'rewards', $2, $3)
            "#,
            user_id,
            referral.id,
            serde_json::json!({ "amount": referral.referred_reward, "role": "referred" })
        )
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

/// Fetch payout settings for a given user.
pub async fn get_payout_settings(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<PayoutSettings>, AppError> {
    let row =
        sqlx::query_as::<_, PayoutSettings>("SELECT * FROM payout_settings WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    Ok(row)
}

/// Insert or update payout settings for a given user.
pub async fn save_payout_settings(
    pool: &PgPool,
    user_id: Uuid,
    form: SavePayoutSettingsForm,
) -> Result<PayoutSettings, AppError> {
    let row = sqlx::query_as::<_, PayoutSettings>(
        r#"
        INSERT INTO payout_settings (user_id, payment_method, account_email, full_name, street_address, postcode, city, country, vat_number)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (user_id) DO UPDATE SET
            payment_method = EXCLUDED.payment_method,
            account_email  = EXCLUDED.account_email,
            full_name      = EXCLUDED.full_name,
            street_address = EXCLUDED.street_address,
            postcode       = EXCLUDED.postcode,
            city           = EXCLUDED.city,
            country        = EXCLUDED.country,
            vat_number     = EXCLUDED.vat_number,
            updated_at     = NOW()
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&form.payment_method)
    .bind(&form.account_email)
    .bind(&form.full_name)
    .bind(&form.street_address)
    .bind(&form.postcode)
    .bind(&form.city)
    .bind(&form.country)
    .bind(&form.vat_number)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

/// List commissions for a user, optionally filtered by date range.
pub async fn list_commissions(
    pool: &PgPool,
    user_id: Uuid,
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
) -> Result<Vec<CommissionRecord>, AppError> {
    let rows = sqlx::query_as::<_, CommissionRecord>(
        r#"
        SELECT * FROM commissions
        WHERE user_id = $1
          AND ($2::date IS NULL OR period_start >= $2)
          AND ($3::date IS NULL OR period_end <= $3)
        ORDER BY period_start DESC
        "#,
    )
    .bind(user_id)
    .bind(date_from)
    .bind(date_to)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
