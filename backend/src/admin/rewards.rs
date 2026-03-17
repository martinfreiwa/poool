use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

//
//  Admin Rewards API
//

/// GET /api/admin/rewards  All rewards data: tiers, user tiers, balances, and referrals
pub async fn api_admin_rewards(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // 1. Tiers
    let tiers_rows = sqlx::query!(
        r#"SELECT name, min_invest, max_invest, cashback_pct::float8 as "cashback_pct!", 
                  badge_color, sort_order, referral_bonus FROM tiers ORDER BY sort_order ASC"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let tiers: Vec<serde_json::Value> = tiers_rows.iter().map(|r| {
        serde_json::json!({
            "name": r.name, "min_invest": r.min_invest, "max_invest": r.max_invest,
            "cashback_pct": r.cashback_pct, "badge_color": r.badge_color, "sort_order": r.sort_order,
            "referral_bonus": r.referral_bonus
        })
    }).collect();

    // 2. User Tiers
    let ut_rows = sqlx::query!(
        r#"SELECT u.id::text, u.email, COALESCE(up.first_name, '') as fn, COALESCE(up.last_name, '') as ln,
                  t.name as tier_name, ut.invested_12m
           FROM user_tiers ut
           JOIN users u ON u.id = ut.user_id
           JOIN tiers t ON t.id = ut.tier_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           ORDER BY ut.invested_12m DESC LIMIT 1000"#
    ).fetch_all(&state.db).await.unwrap_or_default();

    let user_tiers: Vec<serde_json::Value> = ut_rows
        .iter()
        .map(|r| {
            let name = format!(
                "{} {}",
                r.r#fn.clone().unwrap_or_default(),
                r.ln.clone().unwrap_or_default()
            )
            .trim()
            .to_string();
            serde_json::json!({
                "user_id": r.id,
                "name": if name.is_empty() { r.email.clone() } else { name },
                "email": r.email, "tier": r.tier_name, "invested_12m": r.invested_12m
            })
        })
        .collect();

    // 3. Rewards Balances
    let bal_rows = sqlx::query!(
        r#"SELECT rb.user_id::text, u.email, COALESCE(up.first_name, '') as fn, COALESCE(up.last_name, '') as ln,
                  rb.cashback, rb.referrals as referrals_amt, rb.promotions
           FROM rewards_balances rb
           JOIN users u ON u.id = rb.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           ORDER BY (rb.cashback + rb.referrals + rb.promotions) DESC LIMIT 1000"#
    ).fetch_all(&state.db).await.unwrap_or_default();

    let balances: Vec<serde_json::Value> = bal_rows.iter().map(|r| {
        let name = format!("{} {}", r.r#fn.clone().unwrap_or_default(), r.ln.clone().unwrap_or_default()).trim().to_string();
        serde_json::json!({
            "user_id": r.user_id,
            "name": if name.is_empty() { r.email.clone() } else { name },
            "email": r.email, "cashback": r.cashback, "referrals_amt": r.referrals_amt, "promotions": r.promotions
        })
    }).collect();

    // 4. Referrals
    let ref_rows = sqlx::query!(
        r#"SELECT rt.id::text, rt.status, rt.referrer_reward, rt.referred_reward, rt.created_at::text,
                  u1.email as ref_email, COALESCE(up1.first_name, '') as ref_fn, COALESCE(up1.last_name, '') as ref_ln,
                  u2.email as red_email, COALESCE(up2.first_name, '') as red_fn, COALESCE(up2.last_name, '') as red_ln
           FROM referral_tracking rt
           JOIN users u1 ON u1.id = rt.referrer_id
           LEFT JOIN user_profiles up1 ON up1.user_id = u1.id
           JOIN users u2 ON u2.id = rt.referred_id
           LEFT JOIN user_profiles up2 ON up2.user_id = u2.id
           ORDER BY rt.created_at DESC LIMIT 500"#
    ).fetch_all(&state.db).await.unwrap_or_default();

    let referrals: Vec<serde_json::Value> = ref_rows.iter().map(|r| {
        let r_name = format!("{} {}", r.ref_fn.clone().unwrap_or_default(), r.ref_ln.clone().unwrap_or_default()).trim().to_string();
        let ed_name = format!("{} {}", r.red_fn.clone().unwrap_or_default(), r.red_ln.clone().unwrap_or_default()).trim().to_string();
        serde_json::json!({
            "id": r.id, "status": r.status, "referrer_reward": r.referrer_reward, "referred_reward": r.referred_reward,
            "created_at": r.created_at,
            "referrer_name": if r_name.is_empty() { r.ref_email.clone() } else { r_name },
            "referrer_email": r.ref_email,
            "referred_name": if ed_name.is_empty() { r.red_email.clone() } else { ed_name },
            "referred_email": r.red_email
        })
    }).collect();

    // 5. Referral Codes
    let code_rows = sqlx::query!(
        r#"SELECT rc.code, rc.created_at::text,
                  u.email, COALESCE(up.first_name, '') as fn, COALESCE(up.last_name, '') as ln
           FROM referral_codes rc
           JOIN users u ON u.id = rc.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           ORDER BY rc.created_at DESC LIMIT 500"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let referral_codes: Vec<serde_json::Value> = code_rows
        .iter()
        .map(|r| {
            let name = format!(
                "{} {}",
                r.r#fn.clone().unwrap_or_default(),
                r.ln.clone().unwrap_or_default()
            )
            .trim()
            .to_string();
            serde_json::json!({
                "code": r.code,
                "created_at": r.created_at,
                "user_name": if name.is_empty() { r.email.clone() } else { name },
                "user_email": r.email,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "tiers": tiers,
        "user_tiers": user_tiers,
        "balances": balances,
        "referrals": referrals,
        "referral_codes": referral_codes
    }))
    .into_response())
}

//
//  Admin Rewards Management API
//

/// Payload for adjusting a user's rewards balance from the admin dashboard.
#[derive(serde::Deserialize)]
pub struct AdminAdjustRewardsPayload {
    /// Amount of cashback to add/subtract (in cents).
    pub cashback: i64,
    /// Amount of referrals to add/subtract (in cents).
    pub referrals: i64,
    /// Amount of promotions to add/subtract (in cents).
    pub promotions: i64,
}

/// POST /api/admin/rewards/balances/:user_id/adjust
pub async fn api_admin_rewards_balance_adjust(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminAdjustRewardsPayload>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&user_id)?;

    let result = sqlx::query(
        "INSERT INTO rewards_balances (user_id, cashback, referrals, promotions) VALUES ($1, $2, $3, $4) ON CONFLICT (user_id) DO UPDATE SET cashback = rewards_balances.cashback + EXCLUDED.cashback, referrals = rewards_balances.referrals + EXCLUDED.referrals, promotions = rewards_balances.promotions + EXCLUDED.promotions"
    )
    .bind(uid)
    .bind(payload.cashback)
    .bind(payload.referrals)
    .bind(payload.promotions)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"updated"})).into_response()),
        Err(e) => {
            tracing::error!("Failed to adjust rewards balance {user_id}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

/// Payload for updating a reward tier's configuration.
#[derive(serde::Deserialize)]
pub struct AdminUpdateTierPayload {
    /// Minimum investment required for this tier (in cents).
    pub min_invest: i64,
    /// Optional maximum investment for this tier (in cents).
    pub max_invest: Option<i64>,
    /// Cashback percentage offered to users in this tier.
    pub cashback_pct: f64,
    /// Referral bonus amount (in cents) for this tier.
    pub referral_bonus: i64,
    /// CSS color code for the tier's badge.
    pub badge_color: String,
    /// Global sort order for this tier in lists.
    pub sort_order: i32,
}

/// PATCH /api/admin/rewards/tiers/:tier_name - Update a tier's configuration.
///
/// Updates fields like investment limits, cashback rates, and rewards.
pub async fn api_admin_tier_update(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(tier_name): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateTierPayload>,
) -> Result<axum::response::Response, ApiError> {
    let result = sqlx::query(
        r#"UPDATE tiers SET 
           min_invest = $1, max_invest = $2, cashback_pct = $3, 
           referral_bonus = $4, badge_color = $5, sort_order = $6,
           updated_at = NOW()
           WHERE name = $7"#,
    )
    .bind(payload.min_invest)
    .bind(payload.max_invest)
    .bind(payload.cashback_pct)
    .bind(payload.referral_bonus)
    .bind(payload.badge_color)
    .bind(payload.sort_order)
    .bind(&tier_name)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            Ok(Json(serde_json::json!({"status":"updated"})).into_response())
        }
        Ok(_) => return Err(ApiError::NotFound("Tier not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to update tier {tier_name}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

/// Payload for creating a reward tier's configuration.
#[derive(serde::Deserialize)]
pub struct AdminCreateTierPayload {
    /// Name of the new tier.
    pub name: String,
    /// Minimum investment required for this tier (in cents).
    pub min_invest: i64,
    /// Optional maximum investment for this tier (in cents).
    pub max_invest: Option<i64>,
    /// Cashback percentage offered to users in this tier.
    pub cashback_pct: f64,
    /// Referral bonus amount (in cents) for this tier.
    pub referral_bonus: i64,
    /// CSS color code for the tier's badge.
    pub badge_color: String,
    /// Global sort order for this tier in lists.
    pub sort_order: i32,
}

/// POST /api/admin/rewards/tiers - Create a new tier.
pub async fn api_admin_tier_create(
    _admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<AdminCreateTierPayload>,
) -> Result<axum::response::Response, ApiError> {
    let result = sqlx::query(
        r#"INSERT INTO tiers (name, min_invest, max_invest, cashback_pct, referral_bonus, badge_color, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#
    )
    .bind(&payload.name)
    .bind(payload.min_invest)
    .bind(payload.max_invest)
    .bind(payload.cashback_pct)
    .bind(payload.referral_bonus)
    .bind(payload.badge_color)
    .bind(payload.sort_order)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Ok(Json(serde_json::json!({"status":"created"})).into_response()),
        Err(e) => {
            tracing::error!("Failed to create tier {}: {e}", payload.name);
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}

/// PATCH /api/admin/rewards/referrals/:ref_id - Update referral status.
///
/// Handles status transitions (pending, qualified, paid, flagged) and
/// automatically credits users when marked as 'paid'.
pub async fn api_admin_referral_update(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(ref_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&ref_id)?;

    let new_status = match body.get("status").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return Err(ApiError::BadRequest("Status is required".to_string()));
        }
    };

    let result = sqlx::query(
        "UPDATE referral_tracking SET status = $1, qualified_at = CASE WHEN $1 IN ('qualified', 'paid') THEN NOW() ELSE qualified_at END WHERE id = $2"
    )
    .bind(new_status)
    .bind(uid)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            // If status changed to 'paid', we should credit the users
            if new_status == "paid" {
                // Fetch rewards amounts and user IDs
                let row = sqlx::query!(
                    "SELECT referrer_id, referred_id, referrer_reward, referred_reward FROM referral_tracking WHERE id = $1",
                    uid
                ).fetch_optional(&state.db).await.unwrap_or(None);

                if let Some(r) = row {
                    // Credit Referrer
                    let _ = sqlx::query(
                        "INSERT INTO rewards_balances (user_id, referrals) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET referrals = rewards_balances.referrals + EXCLUDED.referrals"
                    ).bind(r.referrer_id).bind(r.referrer_reward).execute(&state.db).await;

                    // Credit Referred
                    let _ = sqlx::query(
                        "INSERT INTO rewards_balances (user_id, referrals) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET referrals = rewards_balances.referrals + EXCLUDED.referrals"
                    ).bind(r.referred_id).bind(r.referred_reward).execute(&state.db).await;
                }
            }
            Ok(Json(serde_json::json!({"status":"updated"})).into_response())
        }
        Ok(_) => return Err(ApiError::NotFound("Referral not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to update referral {ref_id}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    }
}
