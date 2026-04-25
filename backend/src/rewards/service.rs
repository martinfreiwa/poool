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
        friend_reward_cents: 3000, // $30 — TODO: move to platform_settings table
        user_reward_cents: 3000,   // $30
        investment_required_cents: 100_000, // $1,000
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

// ─── Affiliate Commission System ────────────────────────────────

/// Called within the checkout/approval transaction to track an affiliate commission.
///
/// If the buyer was referred by an *active* affiliate (via `affiliate_referrals`),
/// this creates a provisional commission entry in `affiliate_commissions` and
/// transitions the referral to `under_holdback` with a 30-day expiry.
pub async fn check_and_track_affiliate_commission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    order_id: Uuid,
    order_total_cents: i64,
) -> Result<Option<(Uuid, Option<String>, i64)>, AppError> {
    // 1. Check if this user is tracked as an affiliate referral
    let referral = sqlx::query!(
        r#"SELECT ar.id, ar.affiliate_id, ar.status, ar.created_at, ar.sub_id,
                  a.commission_rate_bps, a.current_tier, a.status as aff_status
           FROM affiliate_referrals ar
           JOIN affiliates a ON a.user_id = ar.affiliate_id
           WHERE ar.referred_user_id = $1
             AND a.status = 'active'
           LIMIT 1"#,
        user_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let referral = match referral {
        Some(r) => r,
        None => return Ok(None), // Not an affiliate referral or already processed
    };

    // 1b. GAP-07: Guard against double-payout with the legacy referral_tracking system.
    // If this user already has a qualifying reward in the old system, skip the new commission.
    let already_rewarded_legacy: bool = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM referral_tracking WHERE referred_id = $1 AND status IN ('qualified', 'paid')",
        user_id
    )
    .fetch_one(&mut **tx)
    .await
    .unwrap_or(Some(false))
    .unwrap_or(false);

    if already_rewarded_legacy {
        tracing::warn!(
            user_id = %user_id,
            "Affiliate commission skipped: user already has a qualifying reward in legacy referral_tracking (double-payout guard)"
        );
        return Ok(None);
    }

    // 2. Time-to-Conversion Expiry (90-day strict window for FIRST conversion)
    // "they are lifetime referrals" — meaning if they convert once within 90 days, subsequent orders are tracked forever.
    let is_first_commission: bool = sqlx::query_scalar!(
        "SELECT COUNT(*) = 0 FROM affiliate_commissions WHERE referral_id = $1",
        referral.id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(true);

    if is_first_commission {
        let created_at = referral.created_at.unwrap_or_else(chrono::Utc::now);
        let days_since_attributed = (chrono::Utc::now() - created_at).num_days();
        if days_since_attributed > 90 {
            tracing::info!("Affiliate referral {} expired: first investment occurred {} days after attribution (max 90)", referral.id, days_since_attributed);
            // Mark the referral as disqualified/expired
            let _ = sqlx::query!(
                "UPDATE affiliate_referrals SET status = 'expired', updated_at = NOW() WHERE id = $1",
                referral.id
            ).execute(&mut **tx).await;
            return Ok(None);
        }
    }

    // 3. Calculate commission (basis points: 50 bps = 0.50%)
    let commission_cents =
        (order_total_cents * referral.commission_rate_bps.unwrap_or(50) as i64) / 10_000;

    if commission_cents <= 0 {
        return Ok(None);
    }

    // 3. Create provisional commission
    sqlx::query!(
        r#"INSERT INTO affiliate_commissions
           (referral_id, affiliate_id, source_order_id, provisional_amount_cents, status, tier_at_execution)
           VALUES ($1, $2, $3, $4, 'provisionally_tracked', $5)"#,
        referral.id,
        referral.affiliate_id,
        order_id,
        commission_cents,
        referral.current_tier.unwrap_or_else(|| "Access".to_string())
    )
    .execute(&mut **tx)
    .await?;

    // 4. Transition referral to holdback or qualified states
    if is_first_commission {
        // GAP-06: Set first_investment_done as an intermediate state so that the funnel
        // accurately reflects the user has invested, before the holdback clock starts.
        // We do this in the same transaction so the referral passes through both states atomically.
        sqlx::query!(
            r#"UPDATE affiliate_referrals
               SET status = 'first_investment_done', updated_at = NOW()
               WHERE id = $1 AND status IN ('registered', 'kyc_approved', 'attributed')"#,
            referral.id
        )
        .execute(&mut **tx)
        .await?;

        // Now immediately transition to under_holdback and start the 30-day clock
        sqlx::query!(
            r#"UPDATE affiliate_referrals
               SET status = 'under_holdback',
                   holdback_expires_at = NOW() + INTERVAL '30 days',
                   qualifying_investment_id = (
                       SELECT id FROM investments WHERE user_id = $1 AND status = 'active' ORDER BY created_at DESC LIMIT 1
                   ),
                   updated_at = NOW()
               WHERE id = $2"#,
            user_id,
            referral.id
        )
        .execute(&mut **tx)
        .await?;
    } else {
        // If they were already qualified, the new commission will be evaluated by a discrete commission-level worker later,
        // or just stay provisionally tracked. But don't regress the referral status to under_holdback if it's already paid/qualified!
        let _ = sqlx::query!(
            r#"UPDATE affiliate_referrals SET updated_at = NOW() WHERE id = $1"#,
            referral.id
        )
        .execute(&mut **tx)
        .await;
    }

    tracing::info!(
        affiliate_id = ?referral.affiliate_id,
        referred_user = %user_id,
        commission_cents = commission_cents,
        "Affiliate commission tracked (provisionally)"
    );

    Ok(Some((
        referral.affiliate_id.unwrap_or_default(),
        referral.sub_id,
        commission_cents,
    )))
}

/// Attributes a newly registered user to an affiliate via cookie referral code.
///
/// Called during signup when the referral cookie matches an active affiliate's code.
/// Creates an entry in `affiliate_referrals` with `status = 'attributed'`.
pub async fn attribute_affiliate_referral(
    pool: &PgPool,
    affiliate_code: &str,
    referred_user_id: Uuid,
    subid: Option<String>,
    utm_source: Option<String>,
    ip_address: Option<String>,
) -> Result<bool, AppError> {
    // 1. Resolve code to an active affiliate
    let affiliate = sqlx::query!(
        r#"SELECT user_id FROM affiliates
           WHERE referral_code = $1 AND status = 'active'"#,
        affiliate_code
    )
    .fetch_optional(pool)
    .await?;

    let affiliate = match affiliate {
        Some(a) => a,
        None => return Ok(false), // Code doesn't match an active affiliate
    };

    // 2. Prevent self-referral
    if affiliate.user_id == referred_user_id {
        return Ok(false);
    }

    // 2.5 Prevent Self-Referral via IP Overlap Fraud Matrix (F.1)
    if let Some(ref ip) = ip_address {
        let ip_overlap: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM audit_logs WHERE actor_user_id = $1 AND host(ip_address) = $2"
        )
        .bind(&affiliate.user_id)
        .bind(ip)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if ip_overlap {
            tracing::warn!("Fraud Matrix Trip: Blocked affiliate attribution due to IP overlap (Affiliate: {}, User: {}, IP: {})", affiliate.user_id, referred_user_id, ip);
            return Ok(false);
        }
    }

    // 3. Check if this user is already attributed
    let existing = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM affiliate_referrals WHERE referred_user_id = $1",
        referred_user_id
    )
    .fetch_one(pool)
    .await?;

    if existing.unwrap_or(0) > 0 {
        return Ok(false); // Already attributed
    }

    // 4. Create the attribution record
    sqlx::query!(
        r#"INSERT INTO affiliate_referrals (affiliate_id, referred_user_id, status, sub_id, utm_source)
           VALUES ($1, $2, 'registered', $3, $4)"#,
        affiliate.user_id,
        referred_user_id,
        subid,
        utm_source
    )
    .execute(pool)
    .await?;

    tracing::info!(
        affiliate = %affiliate.user_id,
        referred = %referred_user_id,
        "Affiliate referral attributed"
    );

    // Fire S2S Postback for Registration
    trigger_s2s_postback(
        pool.clone(),
        affiliate.user_id,
        "registration".to_string(),
        subid.clone(),
        0,
    )
    .await;

    Ok(true)
}

/// Asynchronously fires an S2S Postback URL if the affiliate has one configured.
/// Replaces macros `{subid}`, `{event}`, and `{payout}` dynamically.
pub async fn trigger_s2s_postback(
    pool: PgPool,
    affiliate_id: Uuid,
    event: String,
    subid: Option<String>,
    payout_cents: i64,
) {
    tokio::spawn(async move {
        // 1. Fetch the postback URL
        let url = match sqlx::query_scalar!(
            "SELECT postback_url FROM affiliates WHERE user_id = $1 AND status = 'active'",
            affiliate_id
        )
        .fetch_optional(&pool)
        .await
        {
            Ok(Some(Some(u))) => u,
            _ => return, // No URL or DB error
        };

        if url.trim().is_empty() {
            return;
        }

        // 2. Replace Macros
        let payout = format!("{:.2}", (payout_cents as f64) / 100.0);
        let final_url = url
            .replace("{subid}", subid.as_deref().unwrap_or(""))
            .replace("{event}", &event)
            .replace("{payout}", &payout);

        // 3. Fire-and-forget HTTP GET
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        if let Ok(client) = client {
            match client.get(&final_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        tracing::info!("S2S Postback fired successfully to [{}]", final_url);
                    } else {
                        tracing::warn!(
                            "S2S Postback failed (Status {}): [{}]",
                            resp.status(),
                            final_url
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("S2S Postback network error to [{}]: {}", final_url, e);
                }
            }
        }
    });
}

/// The current policy version string. Bump this when policies change to force re-acceptance.
pub const CURRENT_POLICY_VERSION: &str = "1.1";

/// Returns full affiliate dashboard data for a given user.
pub async fn get_affiliate_dashboard(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<serde_json::Value, AppError> {
    // 1. Fetch affiliate profile
    let profile = sqlx::query!(
        r#"SELECT referral_code, current_tier, commission_rate_bps, status, postback_url, created_at::text
           FROM affiliates WHERE user_id = $1"#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let profile = match profile {
        Some(p) => p,
        None => {
            return Ok(serde_json::json!({
                "is_affiliate": false,
                "status": "none"
            }))
        }
    };

    if profile.status.as_deref() != Some("active") {
        return Ok(serde_json::json!({
            "is_affiliate": true,
            "status": profile.status,
            "referral_code": profile.referral_code
        }));
    }

    // GAP-08: Fetch accepted policy version (column added in migration 076 — non-macro for compat)
    let accepted_policy_version_str: String = sqlx::query_scalar(
        "SELECT COALESCE(accepted_policy_version, '1.0') FROM affiliates WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
    .unwrap_or_else(|| "1.0".to_string());

    // 2. Count referrals by status
    let referral_stats = sqlx::query!(
        r#"SELECT
               COUNT(*) FILTER (WHERE status = 'attributed') as "attributed!",
               COUNT(*) FILTER (WHERE status = 'registered') as "registered!",
               COUNT(*) FILTER (WHERE status = 'under_holdback') as "under_holdback!",
               COUNT(*) FILTER (WHERE status = 'qualified') as "qualified!",
               COUNT(*) as "total!"
           FROM affiliate_referrals
           WHERE affiliate_id = $1"#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    // 3. Commission earnings
    let earnings = sqlx::query!(
        r#"SELECT
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'provisionally_tracked'), 0)::BIGINT as "provisional!",
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'on_hold'), 0)::BIGINT as "on_hold!",
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'payable'), 0)::BIGINT as "payable!",
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'paid'), 0)::BIGINT as "paid!",
               COALESCE(SUM(provisional_amount_cents), 0)::BIGINT as "total!"
           FROM affiliate_commissions
           WHERE affiliate_id = $1"#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    // 4. Click count (from referral_clicks if using the affiliate's code)
    let clicks: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*)::BIGINT FROM referral_clicks WHERE code = $1",
        profile.referral_code
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    // 5. Recent commissions list
    let recent_commissions = sqlx::query!(
        r#"SELECT ac.provisional_amount_cents, ac.status, ac.tier_at_execution, ac.created_at::text as "created_at"
           FROM affiliate_commissions ac
           WHERE ac.affiliate_id = $1
           ORDER BY ac.created_at DESC
           LIMIT 10"#,
        user_id
    )
    .fetch_all(pool)
    .await?;

    let commissions_list: Vec<serde_json::Value> = recent_commissions
        .iter()
        .map(|c| {
            serde_json::json!({
                "amount_cents": c.provisional_amount_cents,
                "status": c.status,
                "tier": c.tier_at_execution,
                "created_at": c.created_at
            })
        })
        .collect();

    // GAP-08: Check if affiliate needs to re-accept updated policies
    let policy_reacceptance_required = accepted_policy_version_str != CURRENT_POLICY_VERSION;

    Ok(serde_json::json!({
        "is_affiliate": true,
        "status": "active",
        "referral_code": profile.referral_code,
        "current_tier": profile.current_tier,
        "commission_rate_bps": profile.commission_rate_bps,
        "postback_url": profile.postback_url,
        "referral_url": format!("https://app.poool.com/r/{}", profile.referral_code),
        "policy_reacceptance_required": policy_reacceptance_required,
        "current_policy_version": CURRENT_POLICY_VERSION,
        "tier_thresholds": get_affiliate_tier_thresholds(),
        "referrals": {
            "attributed": referral_stats.attributed,
            "registered": referral_stats.registered,
            "under_holdback": referral_stats.under_holdback,
            "qualified": referral_stats.qualified,
            "total": referral_stats.total
        },
        "earnings": {
            "provisional_cents": earnings.provisional,
            "on_hold_cents": earnings.on_hold,
            "payable_cents": earnings.payable,
            "paid_cents": earnings.paid,
            "total_cents": earnings.total
        },
        "clicks": clicks,
        "recent_commissions": commissions_list
    }))
}

// ─── Nightly Holdback Worker (Step 3.3) ──────────────────────────────────────

/// Processes all `under_holdback` referrals whose 30-day holdback window has expired.
///
/// For each expired holdback:
/// - If the qualifying investment is still `active` → transition to `qualified`,
///   mark all associated `provisionally_tracked` commissions as `payable`.
/// - If the investment was cancelled/refunded → transition to `disqualified`,
///   mark commissions as `disqualified`.
///
/// This runs as a Tokio background task every 6 hours. It is idempotent — safe to
/// run multiple times (uses ACID transactions per referral).
pub async fn run_affiliate_holdback_worker(pool: PgPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 60 * 60));
    // Small startup delay to avoid slamming the DB on boot
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;

    tracing::info!("🔄 Affiliate holdback worker starting (runs every 6h)");

    loop {
        interval.tick().await;
        tracing::info!("🔄 Affiliate holdback worker: scanning expired holdbacks...");

        // Fetch all referrals whose holdback window has passed
        let expired = sqlx::query!(
            r#"
            SELECT ar.id, ar.affiliate_id, ar.referred_user_id, ar.qualifying_investment_id
            FROM affiliate_referrals ar
            WHERE ar.status = 'under_holdback'
              AND ar.holdback_expires_at <= NOW()
            FOR UPDATE SKIP LOCKED
            "#
        )
        .fetch_all(&pool)
        .await;

        let expired = match expired {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Holdback worker: failed to query expired holdbacks: {}", e);
                continue;
            }
        };

        if expired.is_empty() {
            tracing::info!("Holdback worker: no expired holdbacks found.");
            continue;
        }

        tracing::info!(
            "Holdback worker: processing {} expired holdbacks",
            expired.len()
        );

        let mut qualified_count = 0u32;
        let mut disqualified_count = 0u32;

        for referral in expired {
            // Check if the qualifying investment is still active
            let investment_active: bool = if let Some(inv_id) = referral.qualifying_investment_id {
                sqlx::query_scalar!(
                    "SELECT status = 'active' FROM investments WHERE id = $1",
                    inv_id
                )
                .fetch_optional(&pool)
                .await
                .unwrap_or(None)
                .flatten()
                .unwrap_or(false)
            } else {
                // No specific investment ID recorded — fall back to checking if the user
                // has any active investment at all
                let count: i64 = sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM investments WHERE user_id = $1 AND status = 'active'",
                    referral.referred_user_id
                )
                .fetch_one(&pool)
                .await
                .unwrap_or(Some(0))
                .unwrap_or(0);
                count > 0
            };

            let new_referral_status = if investment_active {
                "qualified"
            } else {
                "disqualified"
            };
            let new_commission_status = if investment_active {
                "payable"
            } else {
                "disqualified"
            };

            // Execute within an ACID transaction
            let mut tx = match pool.begin().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!(
                        "Holdback worker: failed to begin tx for referral {}: {}",
                        referral.id,
                        e
                    );
                    continue;
                }
            };

            // 1. Update the referral status
            let referral_res = sqlx::query!(
                r#"UPDATE affiliate_referrals
                   SET status = $1, updated_at = NOW()
                   WHERE id = $2 AND status = 'under_holdback'"#,
                new_referral_status,
                referral.id
            )
            .execute(&mut *tx)
            .await;

            if let Err(e) = referral_res {
                tracing::error!(
                    "Holdback worker: failed to update referral {}: {}",
                    referral.id,
                    e
                );
                let _ = tx.rollback().await;
                continue;
            }

            // 2. Update all associated commissions
            let commission_res = sqlx::query!(
                r#"UPDATE affiliate_commissions
                   SET status = $1, updated_at = NOW()
                   WHERE referral_id = $2
                     AND status IN ('provisionally_tracked', 'on_hold')"#,
                new_commission_status,
                referral.id
            )
            .execute(&mut *tx)
            .await;

            if let Err(e) = commission_res {
                tracing::error!(
                    "Holdback worker: failed to update commissions for referral {}: {}",
                    referral.id,
                    e
                );
                let _ = tx.rollback().await;
                continue;
            }

            // 3. Write audit log
            let audit_res = sqlx::query!(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
                   VALUES ($1, 'AFFILIATE_COMMISSION_STATUS_CHANGED', 'affiliate_referrals', $2, $3)"#,
                referral.affiliate_id,
                referral.id,
                serde_json::json!({
                    "new_status": new_referral_status,
                    "commission_status": new_commission_status,
                    "investment_active": investment_active,
                    "processed_by": "holdback_worker"
                })
            )
            .execute(&mut *tx)
            .await;

            if let Err(e) = audit_res {
                // Non-fatal — log but don't rollback for an audit failure
                tracing::warn!(
                    "Holdback worker: failed to write audit log for referral {}: {}",
                    referral.id,
                    e
                );
            }

            if let Err(e) = tx.commit().await {
                tracing::error!(
                    "Holdback worker: failed to commit tx for referral {}: {}",
                    referral.id,
                    e
                );
            } else if investment_active {
                qualified_count += 1;
                tracing::info!(
                    affiliate_id = ?referral.affiliate_id,
                    referral_id = ?referral.id,
                    "✅ Holdback expired → QUALIFIED (commission now payable)"
                );

                // Send email notification for commission earned
                if let Some(aff_id) = referral.affiliate_id {
                    let user_email: Option<String> =
                        sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                            .bind(aff_id)
                            .fetch_optional(&pool)
                            .await
                            .unwrap_or_default();

                    if let Some(email) = user_email {
                        let _ = crate::common::email::send_email(
                            &email,
                            "You earned a new POOOL Affiliate Commission!",
                            "<h3>Commission Qualified</h3><p>Great news! The 30-day holdback period for one of your referred investments has ended.</p><p>The underlying commission has successfully upgraded to <b>Payable</b> status and will be included in the next batch payout cycle.</p><p>Log into your POOOL Affiliate Dashboard to track your earnings.</p>"
                        ).await;
                    }

                    // Fire S2S Postback Webhook if configured
                    if let Ok(Some(postback)) = sqlx::query!(
                        r#"SELECT a.postback_url, ar.sub_id, 
                           COALESCE((SELECT SUM(provisional_amount_cents)::bigint FROM affiliate_commissions WHERE referral_id = $2), 0) as "payout_cents!"
                           FROM affiliates a
                           JOIN affiliate_referrals ar ON ar.id = $2
                           WHERE a.user_id = $1 AND a.postback_url IS NOT NULL"#,
                        aff_id, referral.id
                    )
                    .fetch_optional(&pool)
                    .await {
                        if let Some(url) = postback.postback_url {
                            let sub_id = postback.sub_id.unwrap_or_default();
                            let payout_cents = postback.payout_cents;

                            // Construct final URL
                            let mut parsed = url.clone();
                            if !parsed.contains('?') {
                                parsed.push('?');
                            } else if !parsed.ends_with('?') && !parsed.ends_with('&') {
                                parsed.push('&');
                            }
                            let webhook_url = format!("{}subid={}&payout={}&currency=USD&status=approved", parsed, sub_id, (payout_cents as f64) / 100.0);

                            // Fire asynchronously without blocking the loop
                            tokio::spawn(async move {
                                let client = reqwest::Client::new();
                                match client.get(&webhook_url).send().await {
                                    Ok(res) => {
                                        tracing::info!("✅ Postback Webhook successful. URL: {}, HTTP Status: {}", webhook_url, res.status());
                                    }
                                    Err(e) => {
                                        tracing::warn!("❌ Postback Webhook failed. URL: {}, Error: {}", webhook_url, e);
                                    }
                                }
                            });
                        }
                    }
                }
            } else {
                disqualified_count += 1;
                tracing::info!(
                    affiliate_id = ?referral.affiliate_id,
                    referral_id = ?referral.id,
                    "❌ Holdback expired → DISQUALIFIED (investment no longer active)"
                );
            }
        }

        tracing::info!(
            "🔄 Holdback worker cycle 1 complete: {} qualified, {} disqualified",
            qualified_count,
            disqualified_count
        );

        // --- PASS 2: Lifetime Commissions ---
        let expired_commissions = sqlx::query!(
            r#"
            SELECT c.id, c.affiliate_id, c.source_order_id, c.referral_id
            FROM affiliate_commissions c
            JOIN affiliate_referrals ar ON c.referral_id = ar.id
            WHERE c.status IN ('provisionally_tracked', 'on_hold')
              AND c.created_at <= NOW() - INTERVAL '30 days'
              AND ar.status IN ('qualified', 'paid')
            FOR UPDATE OF c SKIP LOCKED
            "#
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        let mut lifetime_matured = 0;
        let mut lifetime_failed = 0;

        for commission in expired_commissions {
            let order_active: bool = sqlx::query_scalar!(
                "SELECT status IN ('completed', 'approved') FROM orders WHERE id = $1 LIMIT 1",
                commission.source_order_id
            )
            .fetch_optional(&pool)
            .await
            .unwrap_or(None)
            .flatten()
            .unwrap_or(false);

            let new_commission_status = if order_active {
                "payable"
            } else {
                "disqualified"
            };

            let update_res = sqlx::query!(
                "UPDATE affiliate_commissions SET status = $1, updated_at = NOW() WHERE id = $2",
                new_commission_status,
                commission.id
            )
            .execute(&pool)
            .await;

            if update_res.is_ok() {
                if order_active {
                    lifetime_matured += 1;
                } else {
                    lifetime_failed += 1;
                }
            }
        }

        if lifetime_matured > 0 || lifetime_failed > 0 {
            tracing::info!(
                "🔄 Holdback worker cycle 2 (Lifetime) complete: {} matured, {} disqualified",
                lifetime_matured,
                lifetime_failed
            );
        }
    }
}

// ─── Affiliate Tier Thresholds ────────────────────────────────────────────────
// (qualified_referrals_required, tier_name, commission_rate_bps)
const AFFILIATE_TIERS: &[(i64, &str, i32)] = &[
    (0, "Access", 50),
    (3, "Bronze", 60),
    (10, "Silver", 75),
    (25, "Gold", 90),
    (50, "Platinum", 110),
    (100, "Diamond", 130),
    (200, "Elite", 150),
    (500, "Ambassador", 175),
];

/// Runs once per day, scans all active affiliates and advances their tier based on
/// the total number of lifetime qualified referrals.
///
/// Tier thresholds (qualified referral count → tier name, commission_rate_bps):
///   0 → Access (50 bps), 3 → Bronze (60), 10 → Silver (75), 25 → Gold (90),
///   50 → Platinum (110), 100 → Diamond (130), 200 → Elite (150), 500 → Ambassador (175)
pub async fn run_affiliate_tier_progression_worker(pool: PgPool) {
    // Small startup offset so it doesn't overlap with the holdback worker boot
    tokio::time::sleep(std::time::Duration::from_secs(90)).await;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));

    tracing::info!("🏆 Affiliate tier progression worker starting (runs every 24h)");

    loop {
        interval.tick().await;
        tracing::info!("🏆 Affiliate tier progression worker: scanning for tier upgrades...");

        // Fetch all active affiliates and their current qualified referral counts
        let affiliates = sqlx::query!(
            r#"SELECT a.user_id, a.current_tier, a.commission_rate_bps,
                      COUNT(ar.id) FILTER (WHERE ar.status IN ('qualified', 'paid')) AS qualified_count
               FROM affiliates a
               LEFT JOIN affiliate_referrals ar ON ar.affiliate_id = a.user_id
               WHERE a.status = 'active'
               GROUP BY a.user_id, a.current_tier, a.commission_rate_bps"#
        )
        .fetch_all(&pool)
        .await;

        let affiliates = match affiliates {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!("Tier progression worker: failed to query affiliates: {}", e);
                continue;
            }
        };

        let mut upgraded = 0u32;

        for aff in affiliates {
            let count = aff.qualified_count.unwrap_or(0);

            // Determine the highest tier the affiliate qualifies for
            let (new_tier, new_rate_bps) = AFFILIATE_TIERS
                .iter()
                .rev()
                .find(|(threshold, _, _)| count >= *threshold)
                .map(|(_, name, bps)| (*name, *bps))
                .unwrap_or(("Access", 50));

            let current = aff.current_tier.as_deref().unwrap_or("Access");
            let current_bps = aff.commission_rate_bps.unwrap_or(50);

            // Only update if the tier actually changed
            if current == new_tier && current_bps == new_rate_bps {
                continue;
            }

            let update_res = sqlx::query!(
                r#"UPDATE affiliates
                   SET current_tier = $1, commission_rate_bps = $2, updated_at = NOW()
                   WHERE user_id = $3 AND status = 'active'"#,
                new_tier,
                new_rate_bps,
                aff.user_id
            )
            .execute(&pool)
            .await;

            if let Err(e) = update_res {
                tracing::error!(
                    "Tier progression worker: failed to update tier for {}: {}",
                    aff.user_id,
                    e
                );
                continue;
            }

            // Write audit log
            let _ = sqlx::query!(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
                   VALUES ($1, 'AFFILIATE_TIER_UPGRADED', 'affiliates', $1, $2)"#,
                aff.user_id,
                serde_json::json!({
                    "old_tier": current,
                    "new_tier": new_tier,
                    "old_rate_bps": current_bps,
                    "new_rate_bps": new_rate_bps,
                    "qualified_count": count
                })
            )
            .execute(&pool)
            .await;

            upgraded += 1;
            tracing::info!(
                affiliate_id = ?aff.user_id,
                old_tier = current,
                new_tier = new_tier,
                "🏆 Affiliate tier upgraded"
            );

            // Send notification email
            let user_email: Option<String> =
                sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                    .bind(aff.user_id)
                    .fetch_optional(&pool)
                    .await
                    .unwrap_or_default();

            if let Some(email) = user_email {
                let _ = crate::common::email::send_email(
                    &email,
                    &format!("You've been promoted to {} Affiliate!", new_tier),
                    &format!(
                        "<h3>Tier Upgrade!</h3><p>Congratulations! Based on your {} qualified referrals, you have been promoted to the <b>{}</b> tier.</p><p>Your new commission rate is <b>{} bps ({:.2}%)</b>. This rate applies to all future commissions.</p><p>Log into your <a href=\"https://poool.app/affiliate/dashboard\">Affiliate Dashboard</a> to see your updated tier.</p>",
                        count, new_tier, new_rate_bps, (new_rate_bps as f64) / 100.0
                    )
                ).await;
            }
        }

        tracing::info!(
            "🏆 Tier progression worker complete: {} affiliate(s) upgraded",
            upgraded
        );
    }
}

/// Returns the affiliate tier thresholds table for use in the dashboard response.
pub fn get_affiliate_tier_thresholds() -> Vec<serde_json::Value> {
    AFFILIATE_TIERS
        .iter()
        .map(|(threshold, name, bps)| {
            serde_json::json!({
                "tier": name,
                "min_qualified_referrals": threshold,
                "commission_rate_bps": bps
            })
        })
        .collect()
}

// ─── Fraud Ring Detection ─────────────────────────────────────────────────────

/// Scans for circular referral rings (A refers B, B refers A) and same-IP clusters.
/// Returns a list of flagged affiliate pairs for admin review.
pub async fn scan_affiliate_fraud_rings(pool: &PgPool) -> Result<Vec<serde_json::Value>, AppError> {
    // Detect circular rings: affiliate A referred user B who is also an affiliate that referred A (or a user in A's network)
    let rings = sqlx::query!(
        r#"SELECT
               a1.user_id::text AS affiliate_a,
               u1.email AS email_a,
               a2.user_id::text AS affiliate_b,
               u2.email AS email_b
           FROM affiliate_referrals ar1
           JOIN affiliates a1 ON a1.user_id = ar1.affiliate_id
           JOIN affiliates a2 ON a2.user_id = ar1.referred_user_id
           JOIN affiliate_referrals ar2 ON ar2.affiliate_id = a2.user_id
                                       AND ar2.referred_user_id = a1.user_id
           JOIN users u1 ON u1.id = a1.user_id
           JOIN users u2 ON u2.id = a2.user_id
           WHERE a1.status = 'active' AND a2.status = 'active'"#
    )
    .fetch_all(pool)
    .await?;

    let flags: Vec<serde_json::Value> = rings
        .iter()
        .map(|r| {
            serde_json::json!({
                "type": "circular_ring",
                "affiliate_a_id": r.affiliate_a,
                "affiliate_a_email": r.email_a,
                "affiliate_b_id": r.affiliate_b,
                "affiliate_b_email": r.email_b,
                "description": "Circular referral ring detected: each affiliate referred the other"
            })
        })
        .collect();

    Ok(flags)
}
