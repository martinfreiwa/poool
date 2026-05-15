use super::models::{
    AffiliateSettingsResponse, CommissionRecord, PayoutSettings, RewardsOverview,
    SaveAffiliateSettingsForm, SavePayoutSettingsForm, TierInfo,
};
use crate::error::AppError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::Engine as _;
use chrono::NaiveDate;
use rand::RngCore;
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use url::Url;
use uuid::Uuid;

const MAX_POSTBACK_URL_LEN: usize = 512;
const TAX_ID_SECRET_PREFIX: &str = "tax_id:v1";

pub async fn validate_postback_url(url: &str) -> Result<String, AppError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed.len() > MAX_POSTBACK_URL_LEN {
        return Err(AppError::BadRequest(
            "Postback URL must be 512 characters or fewer.".into(),
        ));
    }

    let parsed = Url::parse(trimmed)
        .map_err(|_| AppError::BadRequest("Postback URL must be a valid HTTPS URL.".into()))?;
    if parsed.scheme() != "https" {
        return Err(AppError::BadRequest("Postback URL must use HTTPS.".into()));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AppError::BadRequest(
            "Postback URL must not include embedded credentials.".into(),
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Postback URL must include a host.".into()))?;
    let host_lower = host.trim_end_matches('.').to_ascii_lowercase();
    if matches!(
        host_lower.as_str(),
        "localhost" | "metadata.google.internal" | "metadata"
    ) || host_lower.ends_with(".localhost")
    {
        return Err(AppError::BadRequest(
            "Postback URL host is not allowed.".into(),
        ));
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_postback_ip(ip) {
            return Err(AppError::BadRequest(
                "Postback URL host is not allowed.".into(),
            ));
        }
        return Ok(trimmed.to_string());
    }

    let port = parsed.port_or_known_default().unwrap_or(443);
    let resolved = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| AppError::BadRequest("Postback URL host could not be resolved.".into()))?;

    for addr in resolved {
        if is_blocked_postback_ip(addr.ip()) {
            return Err(AppError::BadRequest(
                "Postback URL host resolves to a private or reserved address.".into(),
            ));
        }
    }

    Ok(trimmed.to_string())
}

fn is_blocked_postback_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || ip.octets()[0] == 0
                || ip.octets()[0] >= 224
                || ip == std::net::Ipv4Addr::new(169, 254, 169, 254)
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast()
        }
    }
}

fn url_encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn cents_to_decimal_string(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.saturating_abs();
    format!("{}{}.{:02}", sign, abs / 100, abs % 100)
}

fn redact_url_query(url: &Url) -> String {
    let mut redacted = url.clone();
    if redacted.query().is_some() {
        redacted.set_query(Some("[redacted]"));
    }
    redacted.to_string()
}

fn build_postback_url(
    template: &str,
    event: &str,
    subid: Option<&str>,
    payout_cents: i64,
) -> Result<Url, AppError> {
    let encoded_subid = url_encode(subid.unwrap_or(""));
    let encoded_event = url_encode(event);
    let payout = cents_to_decimal_string(payout_cents);
    let encoded_payout = url_encode(&payout);

    let mut url = Url::parse(
        &template
            .replace("{subid}", &encoded_subid)
            .replace("{event}", &encoded_event)
            .replace("{payout}", &encoded_payout),
    )
    .map_err(|_| AppError::BadRequest("Stored postback URL is invalid.".into()))?;

    if !template.contains("{subid}") {
        url.query_pairs_mut()
            .append_pair("subid", subid.unwrap_or(""));
    }
    if !template.contains("{event}") {
        url.query_pairs_mut().append_pair("event", event);
    }
    if !template.contains("{payout}") {
        url.query_pairs_mut().append_pair("payout", &payout);
    }
    url.query_pairs_mut()
        .append_pair("currency", "USD")
        .append_pair("status", "approved");

    Ok(url)
}

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
    result.sort_by_key(|b| std::cmp::Reverse(b.clicks));

    Ok(result)
}

// Legacy `check_and_qualify_referral` removed in the GAP-07 closure (audit
// finding: parallel referral_tracking + affiliate_referrals systems caused
// double-payout risk). See migration 155. Historical referral_tracking rows
// are read by leaderboard / dashboard / community surfaces and gated against
// new affiliate commissions by the double-payout guard in
// `check_and_track_affiliate_commission`.

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

fn normalize_required_field(value: &str, field: &str, max_len: usize) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{} is required.", field)));
    }
    if trimmed.chars().count() > max_len {
        return Err(AppError::BadRequest(format!(
            "{} must be {} characters or fewer.",
            field, max_len
        )));
    }
    Ok(trimmed.to_string())
}

fn normalize_optional_field(
    value: Option<&String>,
    max_len: usize,
) -> Result<Option<String>, AppError> {
    match value.map(|v| v.trim()).filter(|v| !v.is_empty()) {
        Some(v) if v.chars().count() > max_len => Err(AppError::BadRequest(format!(
            "Value must be {} characters or fewer.",
            max_len
        ))),
        Some(v) => Ok(Some(v.to_string())),
        None => Ok(None),
    }
}

fn validate_tax_class(value: &str) -> Result<String, AppError> {
    match value.trim() {
        "id_individual" | "id_entity" | "foreign" => Ok(value.trim().to_string()),
        _ => Err(AppError::BadRequest(
            "Invalid tax classification selected.".into(),
        )),
    }
}

fn validate_payout_method(value: &str) -> Result<String, AppError> {
    match value.trim() {
        "poool_wallet" => Ok("poool_wallet".to_string()),
        _ => Err(AppError::BadRequest(
            "Selected payout method is not currently available.".into(),
        )),
    }
}

fn tax_id_encryption_key() -> Result<[u8; 32], AppError> {
    let raw = std::env::var("TAX_ID_ENCRYPTION_KEY")
        .map_err(|_| AppError::Internal("TAX_ID_ENCRYPTION_KEY is not configured.".to_string()))?;

    let trimmed = raw.trim();
    let bytes = if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(trimmed).map_err(|_| {
            AppError::Internal("TAX_ID_ENCRYPTION_KEY is not valid hex.".to_string())
        })?
    } else {
        base64::engine::general_purpose::STANDARD
            .decode(trimmed)
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(trimmed))
            .unwrap_or_else(|_| trimmed.as_bytes().to_vec())
    };

    bytes.try_into().map_err(|_| {
        AppError::Internal("TAX_ID_ENCRYPTION_KEY must decode to 32 bytes.".to_string())
    })
}

fn encrypt_tax_id_payload(plaintext: &str) -> Result<String, AppError> {
    let key = tax_id_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| AppError::Internal("Failed to initialize Tax ID encryption.".to_string()))?;
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| AppError::Internal("Failed to encrypt Tax ID.".to_string()))?;
    Ok(format!(
        "{}:{}:{}",
        TAX_ID_SECRET_PREFIX,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(nonce_bytes),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(ciphertext)
    ))
}

fn tax_id_last4(value: &str) -> Option<String> {
    let compact: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if compact.is_empty() {
        return None;
    }
    let suffix_len = compact.len().min(4);
    Some(compact[compact.len() - suffix_len..].to_string())
}

fn mask_tax_id_last4(value: Option<&str>) -> Option<String> {
    let suffix = value?.trim();
    if suffix.is_empty() {
        return None;
    }
    Some(format!("***-**-{}", suffix))
}

pub struct TaxIdStorage {
    pub encrypted: String,
    pub last4: Option<String>,
    pub key_version: i16,
}

/// Current tax-id encryption key version. Bump when rotating
/// TAX_ID_ENCRYPTION_KEY and re-encrypting existing rows.
pub const TAX_ID_KEY_VERSION: i16 = 1;

pub fn encrypt_tax_id_for_storage(tax_id: &str) -> Result<TaxIdStorage, AppError> {
    let normalized = normalize_required_field(tax_id, "Tax ID", 50)?;
    Ok(TaxIdStorage {
        encrypted: encrypt_tax_id_payload(&normalized)?,
        last4: tax_id_last4(&normalized),
        key_version: TAX_ID_KEY_VERSION,
    })
}

fn tax_status(is_tax_ready: bool, tax_class: Option<&str>, tax_id: Option<&str>) -> String {
    if tax_class.is_none() || tax_id.map(|v| v.trim().is_empty()).unwrap_or(true) {
        "Incomplete".to_string()
    } else if is_tax_ready {
        "Verified".to_string()
    } else {
        "Pending review".to_string()
    }
}

fn payout_status(
    affiliate_status: &str,
    tax_ready: bool,
    tax_document_on_file: bool,
) -> (String, Option<String>) {
    match affiliate_status {
        "active" if tax_ready && tax_document_on_file => ("Active".to_string(), None),
        "active" if !tax_ready => (
            "On hold".to_string(),
            Some("Tax details are pending compliance review.".to_string()),
        ),
        "active" => (
            "On hold".to_string(),
            Some("Tax document is required before payouts can be released.".to_string()),
        ),
        "suspended" => (
            "Suspended".to_string(),
            Some("Affiliate account is suspended.".to_string()),
        ),
        _ => (
            "Under review".to_string(),
            Some("Affiliate account is not active yet.".to_string()),
        ),
    }
}

fn build_affiliate_settings_response(
    tax_class: Option<String>,
    tax_id_last4: Option<String>,
    tax_name: Option<String>,
    vat_number: Option<String>,
    payout_method: Option<String>,
    affiliate_status: String,
    tax_ready: bool,
    tax_document_on_file: bool,
) -> AffiliateSettingsResponse {
    let (payout_status, payout_hold_reason) =
        payout_status(&affiliate_status, tax_ready, tax_document_on_file);
    AffiliateSettingsResponse {
        tax_status: tax_status(tax_ready, tax_class.as_deref(), tax_id_last4.as_deref()),
        tax_id_masked: mask_tax_id_last4(tax_id_last4.as_deref()),
        tax_class,
        tax_name,
        vat_number,
        payout_method: payout_method.unwrap_or_else(|| "poool_wallet".to_string()),
        payout_status,
        payout_hold_reason,
        tax_document_on_file,
        tax_ready,
    }
}

/// Fetch affiliate tax and payout settings without exposing raw tax IDs.
pub async fn get_affiliate_settings(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<AffiliateSettingsResponse, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
            a.tax_recipient_class,
            a.is_tax_ready,
            a.status,
            a.tax_id_encrypted,
            a.tax_id_last4,
            a.company_name,
            a.tax_document_gcs_path,
            ps.payment_method,
            ps.full_name,
            ps.vat_number
        FROM affiliates a
        LEFT JOIN payout_settings ps ON ps.user_id = a.user_id
        WHERE a.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| {
        AppError::Forbidden("Affiliate account is required to manage payout settings.".into())
    })?;

    let payout_full_name: Option<String> = row.try_get("full_name")?;
    let affiliate_company_name: Option<String> = row.try_get("company_name")?;
    let tax_name = payout_full_name.or(affiliate_company_name);
    let tax_id_mask_suffix: Option<String> = row.try_get("tax_id_last4")?;

    Ok(build_affiliate_settings_response(
        row.try_get("tax_recipient_class")?,
        tax_id_mask_suffix,
        tax_name,
        row.try_get("vat_number")?,
        row.try_get("payment_method")?,
        row.try_get("status")?,
        row.try_get::<Option<bool>, _>("is_tax_ready")?
            .unwrap_or(false),
        row.try_get::<Option<String>, _>("tax_document_gcs_path")?
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
    ))
}

/// Atomically update affiliate tax metadata and payout settings.
pub async fn save_affiliate_settings(
    pool: &PgPool,
    user_id: Uuid,
    form: SaveAffiliateSettingsForm,
) -> Result<AffiliateSettingsResponse, AppError> {
    if !form.tax_certified {
        return Err(AppError::BadRequest(
            "Tax certification must be accepted before saving.".into(),
        ));
    }

    let tax_class = validate_tax_class(&form.tax_class)?;
    let payout_method = validate_payout_method(&form.payout_method)?;
    let tax_name = normalize_required_field(&form.tax_name, "Business / full name", 255)?;
    let submitted_tax_id = normalize_optional_field(form.tax_id.as_ref(), 50)?;
    let vat_number = normalize_optional_field(form.vat_number.as_ref(), 50)?;

    let mut tx = pool.begin().await?;

    let affiliate = sqlx::query(
        r#"
        SELECT
            tax_recipient_class,
            is_tax_ready,
            status,
            tax_id_encrypted,
            tax_id_last4,
            company_name,
            tax_document_gcs_path
        FROM affiliates
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let affiliate = affiliate.ok_or_else(|| {
        AppError::Forbidden("Affiliate account is required to manage payout settings.".into())
    })?;

    let current_tax_class: Option<String> = affiliate.try_get("tax_recipient_class")?;
    let current_tax_ready = affiliate
        .try_get::<Option<bool>, _>("is_tax_ready")?
        .unwrap_or(false);
    let current_status: String = affiliate.try_get("status")?;
    let current_tax_id_encrypted: Option<String> = affiliate.try_get("tax_id_encrypted")?;
    let current_tax_id_last4: Option<String> = affiliate.try_get("tax_id_last4")?;
    let current_tax_name: Option<String> = affiliate.try_get("company_name")?;
    let current_tax_document: Option<String> = affiliate.try_get("tax_document_gcs_path")?;

    let payout_row = sqlx::query(
        "SELECT payment_method, full_name, vat_number FROM payout_settings WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let current_payout_method: Option<String> = payout_row
        .as_ref()
        .and_then(|row| row.try_get("payment_method").ok());
    let current_vat_number: Option<String> = payout_row
        .as_ref()
        .and_then(|row| row.try_get("vat_number").ok());

    let submitted_tax_id = submitted_tax_id.as_deref();
    let (next_tax_id_encrypted, next_tax_id_last4, next_tax_id_key_version, tax_id_changed) =
        if let Some(tax_id) = submitted_tax_id {
            let storage = encrypt_tax_id_for_storage(tax_id)?;
            (storage.encrypted, storage.last4, storage.key_version, true)
        } else if let Some(current_encrypted) = current_tax_id_encrypted {
            (
                current_encrypted,
                current_tax_id_last4.clone(),
                TAX_ID_KEY_VERSION,
                false,
            )
        } else {
            return Err(AppError::BadRequest("Tax ID is required.".into()));
        };

    let tax_data_changed = current_tax_class.as_deref() != Some(tax_class.as_str())
        || tax_id_changed
        || current_tax_name.as_deref() != Some(tax_name.as_str());
    let next_tax_ready = if tax_data_changed {
        false
    } else {
        current_tax_ready
    };

    sqlx::query(
        r#"
        UPDATE affiliates
        SET tax_recipient_class = $1,
            tax_id_encrypted = $2,
            tax_id_last4 = $3,
            tax_id_key_version = $4,
            company_name = $5,
            is_tax_ready = $6,
            updated_at = NOW()
        WHERE user_id = $7
        "#,
    )
    .bind(&tax_class)
    .bind(&next_tax_id_encrypted)
    .bind(&next_tax_id_last4)
    .bind(next_tax_id_key_version)
    .bind(&tax_name)
    .bind(next_tax_ready)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO payout_settings (user_id, payment_method, full_name, vat_number)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id) DO UPDATE SET
            payment_method = EXCLUDED.payment_method,
            full_name = EXCLUDED.full_name,
            vat_number = EXCLUDED.vat_number,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&payout_method)
    .bind(&tax_name)
    .bind(&vat_number)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state, metadata)
        VALUES ($1, 'AFFILIATE_SETTINGS_UPDATED', 'affiliates', $1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(serde_json::json!({
        "tax_class": current_tax_class,
        "tax_id_masked": current_tax_id_last4
            .as_deref()
            .and_then(|last4| mask_tax_id_last4(Some(last4))),
        "tax_name": current_tax_name,
        "payout_method": current_payout_method,
        "vat_number": current_vat_number,
        "tax_ready": current_tax_ready
    }))
    .bind(serde_json::json!({
        "tax_class": tax_class.clone(),
        "tax_id_masked": mask_tax_id_last4(next_tax_id_last4.as_deref()),
        "tax_name": tax_name.clone(),
        "payout_method": payout_method.clone(),
        "vat_number": vat_number.clone(),
        "tax_ready": next_tax_ready
    }))
    .bind(serde_json::json!({
        "tax_certified": true,
        "tax_data_changed": tax_data_changed,
        "payout_hold_applied": tax_data_changed && current_tax_ready
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(build_affiliate_settings_response(
        Some(tax_class),
        next_tax_id_last4,
        Some(tax_name),
        vat_number,
        Some(payout_method),
        current_status,
        next_tax_ready,
        current_tax_document
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
    ))
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
    // 1. Check if this user is tracked as an affiliate referral.
    // Tier-Rate kommt aus dem affiliates-Profil des payout_user_id (für
    // Team-Business-Links also vom Developer, nicht vom Mitarbeiter).
    let referral = sqlx::query!(
        r#"SELECT ar.id, ar.affiliate_id, ar.status, ar.created_at, ar.sub_id,
                  ar.link_id, ar.attribution_user_id, ar.payout_user_id,
                  a.commission_rate_bps, a.current_tier, a.status as aff_status
           FROM affiliate_referrals ar
           JOIN affiliates a ON a.user_id = ar.payout_user_id
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
    // Fail-CLOSED: a DB error here returns the underlying error rather than silently
    // letting the commission through. Better to fail a checkout than double-pay.
    let already_rewarded_legacy: bool = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM referral_tracking WHERE referred_id = $1 AND status IN ('qualified', 'paid')",
        user_id
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!(user_id = %user_id, error = ?e, "Double-payout guard query failed — failing closed");
        e
    })?
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

    // 3. Create provisional commission. affiliate_id bleibt = payout_user_id
    // für Rückwärtskompatibilität (FK + bestehende Indizes auf affiliate_id).
    // Die neuen Spalten link_id / attribution_user_id / payout_user_id
    // entkoppeln Reporting (attribution) von Vergütung (payout).
    sqlx::query!(
        r#"INSERT INTO affiliate_commissions
              (referral_id, affiliate_id, link_id, attribution_user_id, payout_user_id,
               source_order_id, provisional_amount_cents, status, tier_at_execution)
           VALUES ($1, $2, $3, $4, $5, $6, $7, 'provisionally_tracked', $8)"#,
        referral.id,
        referral.payout_user_id,
        referral.link_id,
        referral.attribution_user_id,
        referral.payout_user_id,
        order_id,
        commission_cents,
        referral.current_tier.unwrap_or_else(|| "Access".to_string())
    )
    .execute(&mut **tx)
    .await?;

    // 3b. `affiliate_live_counters` wird automatisch durch DB-Trigger
    // `trg_affiliate_commissions_counter_sync` (Migration 161) gepflegt.
    // Trigger reagiert auf INSERT/UPDATE/DELETE und propagiert Status-Übergänge
    // (provisionally_tracked → payable → paid → clawed_back) in die Buckets.

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
        link_id = ?referral.link_id,
        attribution = ?referral.attribution_user_id,
        payout = ?referral.payout_user_id,
        referred_user = %user_id,
        commission_cents = commission_cents,
        "Affiliate commission tracked (provisionally)"
    );

    Ok(Some((
        referral.payout_user_id,
        referral.sub_id,
        commission_cents,
    )))
}

/// Attribuiert einen neu registrierten User über einen affiliate_link-Code.
///
/// Wird beim Signup aufgerufen wenn der Referral-Cookie einen Code enthält.
/// Funktioniert für Personal- und Team-Business-Links: das Datenmodell trägt
/// `link_id`, `attribution_user_id` und `payout_user_id` getrennt; nur
/// `payout_user_id` bekommt am Ende die Provision, `attribution_user_id` ist
/// die Reporting-Zuordnung (z.B. der ausführende Team-Mitarbeiter).
///
/// `affiliate_id` wird weiter mitgeschrieben = `payout_user_id`, damit ältere
/// Read-Pfade (FK + Indizes auf affiliate_id) ohne Code-Änderung weiter funktionieren.
pub async fn attribute_affiliate_referral(
    pool: &PgPool,
    affiliate_code: &str,
    referred_user_id: Uuid,
    subid: Option<String>,
    utm_source: Option<String>,
    ip_address: Option<String>,
) -> Result<bool, AppError> {
    // 1) Code → aktiver Link (Personal oder Team-Business).
    let link = match super::team_links::find_active_by_code(pool, affiliate_code).await? {
        Some(l) => l,
        None => return Ok(false),
    };

    // 2) Self-Referral-Guard: referred darf weder Attribution-User noch
    // Payout-User sein. CHECK-Constraints in DB sind redundant — wir fangen
    // hier sauber ab um klaren Log + return zu liefern.
    if referred_user_id == link.attribution_user_id || referred_user_id == link.payout_user_id {
        tracing::warn!(
            link_id = %link.id,
            referred = %referred_user_id,
            "Affiliate attribution blocked: self-referral (referred == attribution or payout)"
        );
        return Ok(false);
    }

    // 2.5) Bei Team-Business: referred darf nicht selbst Mitglied desselben
    // Teams sein (Member-wirbt-Kollegen-für-eigenen-Boss verhindern).
    // Fail-CLOSED: DB-Fehler hier blockt Attribution. Wir geben den Klick
    // verloren statt ein potentielles Fraud-Setup durchzulassen.
    if let Some(team_id) = link.team_id {
        let same_team: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(
                   SELECT 1 FROM developer_team_memberships
                   WHERE team_id = $1 AND user_id = $2
                     AND status IN ('invited', 'pending_developer_approval', 'active')
               )"#,
        )
        .bind(team_id)
        .bind(referred_user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!(
                link_id = %link.id, team_id = %team_id, error = ?e,
                "Same-team self-referral guard query failed — failing closed"
            );
            e
        })?;
        if same_team {
            tracing::warn!(
                link_id = %link.id,
                referred = %referred_user_id,
                team_id = %team_id,
                "Affiliate attribution blocked: referred user is member of same team"
            );
            return Ok(false);
        }
    }

    // 3) IP-Overlap-Fraud-Check (F.1) — geprüft gegen payout_user_id.
    // Fail-CLOSED bei DB-Fehler: lieber Klick verlieren als Fraud durchlassen.
    if let Some(ref ip) = ip_address {
        let ip_overlap: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM audit_logs WHERE actor_user_id = $1 AND host(ip_address) = $2"
        )
        .bind(&link.payout_user_id)
        .bind(ip)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!(
                payout_user = %link.payout_user_id, ip = %ip, error = ?e,
                "IP-overlap fraud guard query failed — failing closed"
            );
            e
        })?;
        if ip_overlap {
            tracing::warn!(
                payout_user = %link.payout_user_id,
                referred = %referred_user_id,
                ip = %ip,
                "Fraud Matrix Trip: Blocked affiliate attribution due to IP overlap"
            );
            return Ok(false);
        }
    }

    // 4) Already-Attributed-Guard (referred_user_id ist UNIQUE).
    let existing = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM affiliate_referrals WHERE referred_user_id = $1",
        referred_user_id
    )
    .fetch_one(pool)
    .await?;
    if existing.unwrap_or(0) > 0 {
        return Ok(false);
    }

    // 5) Insert mit allen drei Zuordnungs-Spalten + affiliate_id = payout_user_id
    // für Rückwärtskompatibilität bestehender Read-Pfade.
    sqlx::query!(
        r#"INSERT INTO affiliate_referrals
              (affiliate_id, referred_user_id, link_id, attribution_user_id, payout_user_id,
               status, sub_id, utm_source)
           VALUES ($1, $2, $3, $4, $5, 'registered', $6, $7)"#,
        link.payout_user_id,
        referred_user_id,
        link.id,
        link.attribution_user_id,
        link.payout_user_id,
        subid,
        utm_source
    )
    .execute(pool)
    .await?;

    tracing::info!(
        link_id = %link.id,
        link_type = %link.link_type,
        attribution = %link.attribution_user_id,
        payout = %link.payout_user_id,
        referred = %referred_user_id,
        "Affiliate referral attributed"
    );

    // 6) S2S-Postback fire-and-forget (an payout-user; Team-Business → Developer).
    trigger_s2s_postback(
        pool.clone(),
        link.payout_user_id,
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

        if let Err(e) = validate_postback_url(&url).await {
            tracing::warn!(
                affiliate_id = %affiliate_id,
                error = %e,
                "Skipping unsafe affiliate S2S postback URL"
            );
            return;
        }

        let final_url = match build_postback_url(&url, &event, subid.as_deref(), payout_cents) {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!(
                    affiliate_id = %affiliate_id,
                    error = %e,
                    "Skipping invalid affiliate S2S postback URL"
                );
                return;
            }
        };
        let redacted_url = redact_url_query(&final_url);

        // 3. Fire-and-forget HTTP GET
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build();

        if let Ok(client) = client {
            match client.get(final_url.clone()).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        tracing::info!("S2S Postback fired successfully to [{}]", redacted_url);
                    } else {
                        tracing::warn!(
                            "S2S Postback failed (Status {}): [{}]",
                            resp.status(),
                            redacted_url
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("S2S Postback network error to [{}]: {}", redacted_url, e);
                }
            }
        }
    });
}

/// The current policy version string. Bump this when policies change to force re-acceptance.
pub const CURRENT_POLICY_VERSION: &str = "1.1";

/// Returns full affiliate dashboard data for a given user.
/// Dashboard-Context für Mode-Filter (Phase 5).
///   * `All` — bisheriges Verhalten: alle Commissions wo user payout-recipient ist
///   * `Personal` — nur Personal-Links (attribution = payout = user)
///   * `Business` — nur Team-Business-Links wo user die Attribution ist
///     (commissions sind reporting-only, payout fließt an Developer)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardContext {
    /// No filter — sum across personal + any team-business links the user is payout for.
    All,
    /// Personal links only.
    Personal,
    /// Team-business links where the user is the attribution_user (NOT payout).
    Business,
}

impl DashboardContext {
    /// Parses query-string context value. Unknown values map to `All`.
    pub fn from_query(value: Option<&str>) -> Self {
        match value.map(|s| s.trim().to_lowercase()).as_deref() {
            Some("personal") => Self::Personal,
            Some("business") => Self::Business,
            _ => Self::All,
        }
    }
}

pub async fn get_affiliate_dashboard(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<serde_json::Value, AppError> {
    get_affiliate_dashboard_with_context(pool, user_id, DashboardContext::All).await
}

pub async fn get_affiliate_dashboard_with_context(
    pool: &PgPool,
    user_id: Uuid,
    context: DashboardContext,
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
            // No affiliate-profile means user has no personal-affiliate context.
            // In Business mode we still want to show data if they're a team member,
            // so we fall through with a stub profile.
            if context == DashboardContext::Business {
                // Allow Business view without affiliate profile.
                return business_only_dashboard(pool, user_id).await;
            }
            return Ok(serde_json::json!({
                "is_affiliate": false,
                "status": "none"
            }));
        }
    };

    if profile.status.as_deref() != Some("active") && context != DashboardContext::Business {
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

    // 2. Referrals — filtered by context
    let referral_stats = sqlx::query!(
        r#"SELECT
               COUNT(*) FILTER (WHERE ar.status = 'attributed') as "attributed!",
               COUNT(*) FILTER (WHERE ar.status = 'registered') as "registered!",
               COUNT(*) FILTER (WHERE ar.status = 'under_holdback') as "under_holdback!",
               COUNT(*) FILTER (WHERE ar.status = 'qualified') as "qualified!",
               COUNT(*) as "total!"
           FROM affiliate_referrals ar
           JOIN affiliate_links al ON al.id = ar.link_id
           WHERE
             CASE
               WHEN $2::text = 'personal' THEN al.link_type = 'personal'    AND ar.attribution_user_id = $1
               WHEN $2::text = 'business' THEN al.link_type = 'team_business' AND ar.attribution_user_id = $1
               ELSE ar.payout_user_id = $1
             END"#,
        user_id,
        context_str(context)
    )
    .fetch_one(pool)
    .await?;

    // 3. Earnings — filtered by context.
    // In Business mode, the user is NOT the payout recipient — the report shows
    // commission they generated for the developer (informational only).
    let earnings = sqlx::query!(
        r#"SELECT
               COALESCE(SUM(ac.provisional_amount_cents) FILTER (WHERE ac.status = 'provisionally_tracked'), 0)::BIGINT as "provisional!",
               COALESCE(SUM(ac.provisional_amount_cents) FILTER (WHERE ac.status = 'on_hold'), 0)::BIGINT as "on_hold!",
               COALESCE(SUM(ac.provisional_amount_cents) FILTER (WHERE ac.status = 'payable'), 0)::BIGINT as "payable!",
               COALESCE(SUM(ac.provisional_amount_cents) FILTER (WHERE ac.status = 'paid'), 0)::BIGINT as "paid!",
               COALESCE(SUM(ac.provisional_amount_cents), 0)::BIGINT as "total!"
           FROM affiliate_commissions ac
           JOIN affiliate_links al ON al.id = ac.link_id
           WHERE
             CASE
               WHEN $2::text = 'personal' THEN al.link_type = 'personal'    AND ac.attribution_user_id = $1
               WHEN $2::text = 'business' THEN al.link_type = 'team_business' AND ac.attribution_user_id = $1
               ELSE ac.payout_user_id = $1
             END"#,
        user_id,
        context_str(context)
    )
    .fetch_one(pool)
    .await?;

    // 4. Click count — filtered by context via affiliate_links join.
    let clicks: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*)::BIGINT
           FROM referral_clicks rc
           LEFT JOIN affiliate_links al ON al.id = rc.link_id
           WHERE
             CASE
               WHEN $2::text = 'personal' THEN al.link_type = 'personal'    AND al.attribution_user_id = $1
               WHEN $2::text = 'business' THEN al.link_type = 'team_business' AND al.attribution_user_id = $1
               ELSE (al.attribution_user_id = $1 OR al.payout_user_id = $1)
             END"#,
        user_id,
        context_str(context)
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    // 5. Recent commissions — filtered by context
    let recent_commissions = sqlx::query!(
        r#"SELECT ac.provisional_amount_cents, ac.status, ac.tier_at_execution,
                  ac.created_at::text as "created_at",
                  al.link_type
           FROM affiliate_commissions ac
           JOIN affiliate_links al ON al.id = ac.link_id
           WHERE
             CASE
               WHEN $2::text = 'personal' THEN al.link_type = 'personal'    AND ac.attribution_user_id = $1
               WHEN $2::text = 'business' THEN al.link_type = 'team_business' AND ac.attribution_user_id = $1
               ELSE ac.payout_user_id = $1
             END
           ORDER BY ac.created_at DESC
           LIMIT 10"#,
        user_id,
        context_str(context)
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
                "link_type": c.link_type,
                "created_at": c.created_at
            })
        })
        .collect();

    // GAP-08: Check if affiliate needs to re-accept updated policies
    let policy_reacceptance_required = accepted_policy_version_str != CURRENT_POLICY_VERSION;

    Ok(serde_json::json!({
        "is_affiliate": true,
        "status": "active",
        "context": context_str(context),
        "referral_code": profile.referral_code,
        "current_tier": profile.current_tier,
        "commission_rate_bps": profile.commission_rate_bps,
        "postback_url": profile.postback_url,
        "referral_url": format!("https://app.poool.com/r/{}", profile.referral_code),
        "policy_reacceptance_required": policy_reacceptance_required,
        "current_policy_version": CURRENT_POLICY_VERSION,
        "tier_thresholds": get_affiliate_tier_thresholds(pool).await,
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

fn context_str(c: DashboardContext) -> &'static str {
    match c {
        DashboardContext::All => "all",
        DashboardContext::Personal => "personal",
        DashboardContext::Business => "business",
    }
}

/// Renders a Business-only dashboard for users who are team members but
/// don't have a personal affiliate profile. Earnings are always informational
/// here (the team owner is the payout recipient).
async fn business_only_dashboard(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<serde_json::Value, AppError> {
    let active_business_link = sqlx::query!(
        r#"SELECT code FROM affiliate_links
           WHERE attribution_user_id = $1
             AND link_type = 'team_business'
             AND status = 'active'
           LIMIT 1"#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    let referral_stats = sqlx::query!(
        r#"SELECT
               COUNT(*) FILTER (WHERE status = 'attributed') as "attributed!",
               COUNT(*) FILTER (WHERE status = 'registered') as "registered!",
               COUNT(*) FILTER (WHERE status = 'under_holdback') as "under_holdback!",
               COUNT(*) FILTER (WHERE status = 'qualified') as "qualified!",
               COUNT(*) as "total!"
           FROM affiliate_referrals
           WHERE attribution_user_id = $1"#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    let earnings = sqlx::query!(
        r#"SELECT
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'provisionally_tracked'), 0)::BIGINT as "provisional!",
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'on_hold'), 0)::BIGINT as "on_hold!",
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'payable'), 0)::BIGINT as "payable!",
               COALESCE(SUM(provisional_amount_cents) FILTER (WHERE status = 'paid'), 0)::BIGINT as "paid!",
               COALESCE(SUM(provisional_amount_cents), 0)::BIGINT as "total!"
           FROM affiliate_commissions
           WHERE attribution_user_id = $1"#,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(serde_json::json!({
        "is_affiliate": active_business_link.is_some(),
        "status": "active",
        "context": "business",
        "referral_code": active_business_link.as_ref().map(|r| r.code.clone()),
        "referral_url": active_business_link.as_ref().map(|r| format!("https://app.poool.com/r/{}", r.code)),
        "informational_only": true,
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
        "clicks": 0,
        "recent_commissions": []
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
            // Set qualified_at when transitioning to 'qualified' so the rolling
            // 12-month tier-volume lookback has a stable reference timestamp.
            let referral_res = sqlx::query!(
                r#"UPDATE affiliate_referrals
                   SET status = $1::text,
                       qualified_at = CASE
                           WHEN $1::text = 'qualified' AND qualified_at IS NULL THEN NOW()
                           ELSE qualified_at
                       END,
                       updated_at = NOW()
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
                            drop(url);
                            trigger_s2s_postback(
                                pool.clone(),
                                aff_id,
                                "approved".to_string(),
                                Some(sub_id),
                                payout_cents,
                            )
                            .await;
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

// ─── Affiliate Tier Ladder (Phase 1, blueprint Point 7) ──────────────────────
// Tiers and commission rates are now seeded in the `affiliate_tiers` table
// (migration 123_affiliate_volume_tiers.sql). Tier eligibility is based on the
// affiliate's own qualified-referral VOLUME within a rolling 12-month lookback,
// not lifetime count.
//
// Approved ladder (commission_rate_bps): Access 50, Plus 75, Pro 100,
// Elite 150, Premium 200, Platinum 275, Signature 350, Sovereign 450.

/// Runs once per day, scans all active affiliates and advances their tier based
/// on their own qualified-referral volume in the trailing 12 months.
///
/// Implements blueprint Point 7:
///   - Volume-based (sum of qualifying investment values), not count
///   - Rolling 12-month lookback — older volume falls off automatically
///   - Single-level: only the affiliate's own direct referrals count
pub async fn run_affiliate_tier_progression_worker(pool: PgPool) {
    // Small startup offset so it doesn't overlap with the holdback worker boot
    tokio::time::sleep(std::time::Duration::from_secs(90)).await;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 60 * 60));

    tracing::info!("🏆 Affiliate tier progression worker starting (runs every 24h)");

    loop {
        interval.tick().await;
        tracing::info!("🏆 Affiliate tier progression worker: scanning for tier upgrades...");

        // Fetch all active affiliates with their own qualified-referral volume
        // over the trailing 12 months. Volume = SUM of qualifying investments'
        // purchase value where the referral is qualified or paid AND the
        // qualified_at timestamp falls inside the rolling window.
        //
        // qualified_at is set by the holdback worker when status flips to
        // 'qualified'; older rows without an explicit qualified_at fall back
        // to updated_at via the migration backfill.
        let affiliates = sqlx::query!(
            r#"SELECT a.user_id,
                      a.current_tier,
                      a.commission_rate_bps,
                      COALESCE(SUM(i.purchase_value_cents) FILTER (
                          WHERE ar.status IN ('qualified', 'paid')
                            AND COALESCE(ar.qualified_at, ar.updated_at) >= NOW() - INTERVAL '12 months'
                      ), 0)::bigint AS "volume_12m_cents!"
               FROM affiliates a
               LEFT JOIN affiliate_referrals ar ON ar.affiliate_id = a.user_id
               LEFT JOIN investments i           ON i.id = ar.qualifying_investment_id
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

        // Snapshot the current ladder once per cycle. Cheap (8 rows) and avoids
        // a per-affiliate roundtrip.
        let ladder = match sqlx::query!(
            r#"SELECT name, commission_rate_bps, min_volume_cents
               FROM affiliate_tiers
               ORDER BY min_volume_cents DESC"#
        )
        .fetch_all(&pool)
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(
                    "Tier progression worker: failed to load affiliate_tiers: {}",
                    e
                );
                continue;
            }
        };

        let mut upgraded = 0u32;

        for aff in affiliates {
            let volume_12m = aff.volume_12m_cents;

            // Highest tier whose min_volume_cents threshold is satisfied.
            // ladder is ordered DESC so the first match wins.
            let (new_tier, new_rate_bps) = ladder
                .iter()
                .find(|t| volume_12m >= t.min_volume_cents)
                .map(|t| (t.name.as_str(), t.commission_rate_bps))
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
                    "volume_12m_cents": volume_12m
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
                        "<h3>Tier Upgrade!</h3><p>Congratulations! Based on your qualified referral volume in the last 12 months (${:.2}), you have been promoted to the <b>{}</b> tier.</p><p>Your new commission rate is <b>{} bps ({:.2}%)</b>. This rate applies to all future commissions.</p><p>Log into your <a href=\"https://poool.app/affiliate/dashboard\">Affiliate Dashboard</a> to see your updated tier.</p>",
                        (volume_12m as f64) / 100.0, new_tier, new_rate_bps, (new_rate_bps as f64) / 100.0
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

/// Returns the affiliate tier ladder for the dashboard response.
/// Reads from `affiliate_tiers` so it stays in sync with the worker.
pub async fn get_affiliate_tier_thresholds(pool: &PgPool) -> Vec<serde_json::Value> {
    sqlx::query!(
        r#"SELECT name, commission_rate_bps, min_volume_cents, sort_order
           FROM affiliate_tiers
           ORDER BY sort_order ASC"#
    )
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|r| {
                serde_json::json!({
                    "tier": r.name,
                    "commission_rate_bps": r.commission_rate_bps,
                    "min_volume_cents": r.min_volume_cents,
                    "sort_order": r.sort_order,
                })
            })
            .collect()
    })
    .unwrap_or_default()
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

/// Scans affiliate referral clicks for shared IP clusters across multiple active affiliates.
pub async fn scan_affiliate_ip_overlaps(pool: &PgPool) -> Result<Vec<serde_json::Value>, AppError> {
    let overlaps = sqlx::query!(
        r#"
        SELECT
            host(rc.ip_address)::text AS ip_address,
            array_agg(DISTINCT a.user_id::text ORDER BY a.user_id::text) AS affiliate_ids,
            array_agg(DISTINCT u.email ORDER BY u.email) AS affiliate_emails,
            COUNT(*)::bigint AS click_count
        FROM referral_clicks rc
        JOIN affiliates a ON a.referral_code = rc.code
        JOIN users u ON u.id = a.user_id
        WHERE a.status = 'active'
          AND rc.ip_address IS NOT NULL
        GROUP BY rc.ip_address
        HAVING COUNT(DISTINCT a.user_id) > 1
        ORDER BY click_count DESC
        LIMIT 100
        "#
    )
    .fetch_all(pool)
    .await?;

    let flags: Vec<serde_json::Value> = overlaps
        .iter()
        .map(|r| {
            serde_json::json!({
                "type": "ip_overlap",
                "ip_address": r.ip_address,
                "affiliate_ids": r.affiliate_ids,
                "affiliate_emails": r.affiliate_emails,
                "click_count": r.click_count.unwrap_or(0),
                "description": "Multiple active affiliates generated referral clicks from the same IP address"
            })
        })
        .collect();

    Ok(flags)
}

/// Converts affiliate fraud findings into Cytoscape-compatible graph elements.
pub fn affiliate_fraud_flags_to_cytoscape_elements(
    flags: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut elements = Vec::new();
    let mut seen_nodes = std::collections::HashSet::new();

    for (idx, flag) in flags.iter().enumerate() {
        match flag
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
        {
            "circular_ring" => {
                let Some(a_id) = flag.get("affiliate_a_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let Some(b_id) = flag.get("affiliate_b_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let a_label = flag
                    .get("affiliate_a_email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Affiliate A");
                let b_label = flag
                    .get("affiliate_b_email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Affiliate B");
                push_fraud_node(&mut elements, &mut seen_nodes, a_id, a_label);
                push_fraud_node(&mut elements, &mut seen_nodes, b_id, b_label);
                elements.push(serde_json::json!({
                    "data": {
                        "id": format!("ring-{idx}-a-b"),
                        "source": a_id,
                        "target": b_id,
                        "label": "RING"
                    }
                }));
                elements.push(serde_json::json!({
                    "data": {
                        "id": format!("ring-{idx}-b-a"),
                        "source": b_id,
                        "target": a_id,
                        "label": "RING"
                    }
                }));
            }
            "ip_overlap" => {
                let ip = flag
                    .get("ip_address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("shared IP");
                let ip_node = format!("ip-{idx}");
                push_fraud_node(&mut elements, &mut seen_nodes, &ip_node, ip);

                let ids = flag
                    .get("affiliate_ids")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let emails = flag
                    .get("affiliate_emails")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                for (affiliate_idx, id_value) in ids.iter().enumerate() {
                    let Some(affiliate_id) = id_value.as_str() else {
                        continue;
                    };
                    let label = emails
                        .get(affiliate_idx)
                        .and_then(|v| v.as_str())
                        .unwrap_or("Affiliate");
                    push_fraud_node(&mut elements, &mut seen_nodes, affiliate_id, label);
                    elements.push(serde_json::json!({
                        "data": {
                            "id": format!("ip-{idx}-{affiliate_idx}"),
                            "source": ip_node,
                            "target": affiliate_id,
                            "label": "IP Overlap"
                        }
                    }));
                }
            }
            _ => {}
        }
    }

    elements
}

fn push_fraud_node(
    elements: &mut Vec<serde_json::Value>,
    seen_nodes: &mut std::collections::HashSet<String>,
    id: &str,
    label: &str,
) {
    if seen_nodes.insert(id.to_string()) {
        elements.push(serde_json::json!({
            "data": {
                "id": id,
                "label": label
            }
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postback_url_builder_encodes_values_and_redacts_query() {
        let url = build_postback_url(
            "https://tracker.example/postback?click={subid}&amount={payout}",
            "qualified referral",
            Some("email <blast>&1"),
            12345,
        )
        .expect("postback URL should build");

        let rendered = url.as_str();
        assert!(rendered.contains("click=email+%3Cblast%3E%261"));
        assert!(rendered.contains("amount=123.45"));
        assert!(rendered.contains("event=qualified+referral"));
        assert!(rendered.contains("currency=USD"));
        assert_eq!(
            redact_url_query(&url),
            "https://tracker.example/postback?[redacted]"
        );
    }

    #[test]
    fn cents_to_decimal_string_uses_integer_minor_units() {
        assert_eq!(cents_to_decimal_string(0), "0.00");
        assert_eq!(cents_to_decimal_string(1), "0.01");
        assert_eq!(cents_to_decimal_string(12345), "123.45");
        assert_eq!(cents_to_decimal_string(-12345), "-123.45");
    }

    #[tokio::test]
    async fn postback_validation_rejects_unsafe_hosts_and_schemes() {
        assert!(validate_postback_url("http://tracker.example/postback")
            .await
            .is_err());
        assert!(validate_postback_url("https://localhost/postback")
            .await
            .is_err());
        assert!(validate_postback_url("https://127.0.0.1/postback")
            .await
            .is_err());
        assert!(
            validate_postback_url("https://169.254.169.254/latest/meta-data")
                .await
                .is_err()
        );
        assert!(
            validate_postback_url("https://user:pass@example.com/postback")
                .await
                .is_err()
        );
    }
}
