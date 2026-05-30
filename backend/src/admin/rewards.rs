use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use axum::{
    extract::{Json, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

const AFFILIATE_REJECTION_REASON_MAX_CHARS: usize = 1000;

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
pub struct AffiliateFraudScanQuery {
    #[serde(rename = "type")]
    scan_type: Option<String>,
}

//
//  Admin Rewards API
//

/// GET /api/admin/rewards  All rewards data: tiers, user tiers, balances, and referrals
pub async fn api_admin_rewards(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "rewards.manage")
        .await?;

    // 1. Tiers
    let tiers_rows = sqlx::query!(
        r#"SELECT name, min_invest, max_invest, cashback_pct::float8 as "cashback_pct!", 
                  badge_color, sort_order, referral_bonus FROM tiers ORDER BY sort_order ASC"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Admin rewards: failed to fetch tiers: {}", e);
        vec![]
    });

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
    ).fetch_all(&state.db).await.unwrap_or_else(|e| { tracing::error!("Admin rewards: failed to fetch user_tiers: {}", e); vec![] });

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
    ).fetch_all(&state.db).await.unwrap_or_else(|e| { tracing::error!("Admin rewards: failed to fetch balances: {}", e); vec![] });

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
    ).fetch_all(&state.db).await.unwrap_or_else(|e| { tracing::error!("Admin rewards: failed to fetch referrals: {}", e); vec![] });

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
    .unwrap_or_else(|e| {
        tracing::error!("Admin rewards: failed to fetch referral_codes: {}", e);
        vec![]
    });

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

    // 6. Affiliate Applications
    let app_rows = sqlx::query!(
        r#"SELECT a.user_id::text as id, u.email, 
                  a.traffic_source, a.audience_size, a.main_url, a.phone_number, a.company_name, a.created_at::text,
                  COALESCE(up.first_name, '') as fn, COALESCE(up.last_name, '') as ln, a.status
           FROM affiliates a
           JOIN users u ON u.id = a.user_id
           LEFT JOIN user_profiles up ON up.user_id = a.user_id
           WHERE a.status = 'pending_approval' OR a.status = 'active'
           ORDER BY a.created_at DESC"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_else(|e| { tracing::error!("Admin rewards: failed to fetch affiliate applications: {}", e); vec![] });

    let applications: Vec<serde_json::Value> = app_rows
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
                "user_email": r.email,
                "user_name": if name.is_empty() { r.email.clone() } else { name },
                "traffic_source": r.traffic_source,
                "audience_size": r.audience_size,
                "main_url": r.main_url,
                "phone_number": r.phone_number,
                "company_name": r.company_name,
                "created_at": r.created_at,
                "status": r.status
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "tiers": tiers,
        "user_tiers": user_tiers,
        "balances": balances,
        "referrals": referrals,
        "referral_codes": referral_codes,
        "applications": applications
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
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminAdjustRewardsPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "rewards.manage")
        .await?;

    let uid = ApiError::parse_uuid(&user_id)?;
    let admin_user = admin.user.clone();

    // Validate: at least one non-zero adjustment
    if payload.cashback == 0 && payload.referrals == 0 && payload.promotions == 0 {
        return Err(ApiError::BadRequest(
            "At least one adjustment amount must be non-zero".to_string(),
        ));
    }

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin rewards adjust tx: {e}");
        ApiError::Internal("Server error".to_string())
    })?;

    // Check current balances to prevent negative results
    let current = sqlx::query_as::<_, (i64, i64, i64)>(
        "SELECT COALESCE(cashback, 0), COALESCE(referrals, 0), COALESCE(promotions, 0) FROM rewards_balances WHERE user_id = $1 FOR UPDATE"
    )
    .bind(uid)
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None)
    .unwrap_or((0, 0, 0));

    let new_cashback = current.0 + payload.cashback;
    let new_referrals = current.1 + payload.referrals;
    let new_promotions = current.2 + payload.promotions;

    if new_cashback < 0 || new_referrals < 0 || new_promotions < 0 {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest(
            "Adjustment would result in negative balance".to_string(),
        ));
    }

    let result = sqlx::query(
        "INSERT INTO rewards_balances (user_id, cashback, referrals, promotions) VALUES ($1, $2, $3, $4) ON CONFLICT (user_id) DO UPDATE SET cashback = rewards_balances.cashback + EXCLUDED.cashback, referrals = rewards_balances.referrals + EXCLUDED.referrals, promotions = rewards_balances.promotions + EXCLUDED.promotions"
    )
    .bind(uid)
    .bind(payload.cashback)
    .bind(payload.referrals)
    .bind(payload.promotions)
    .execute(&mut *tx)
    .await;

    match result {
        Ok(_) => {
            // Record audit log
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(admin_user.id)
            .bind("rewards.balance_adjusted")
            .bind("rewards_balances")
            .bind(uid)
            .bind(serde_json::json!({
                "cashback_delta": payload.cashback,
                "referrals_delta": payload.referrals,
                "promotions_delta": payload.promotions,
                "new_cashback": new_cashback,
                "new_referrals": new_referrals,
                "new_promotions": new_promotions,
            }))
            .execute(&mut *tx).await;

            tx.commit().await.map_err(|e| {
                tracing::error!("Failed to commit rewards adjust tx: {e}");
                ApiError::Internal("Server error".to_string())
            })?;
            Ok(Json(serde_json::json!({"status":"updated"})).into_response())
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!("Failed to adjust rewards balance {user_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
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
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(tier_name): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateTierPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "rewards.manage")
        .await?;

    let result = sqlx::query(
        r#"UPDATE tiers SET 
           min_invest = $1, max_invest = $2, cashback_pct = $3, 
           referral_bonus = $4, badge_color = $5, sort_order = $6
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
        Ok(_) => Err(ApiError::NotFound("Tier not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to update tier {tier_name}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
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
    admin: AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<AdminCreateTierPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "rewards.manage")
        .await?;

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
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

// `api_admin_referral_update` removed (audit GAP-07, migration 155). The
// legacy referral_tracking system is no longer written to from any path.
// Existing rows remain visible to dashboards / leaderboard for history.

//
// Admin Affiliate Management API
//

/// GET /api/admin/rewards/affiliates/pending
pub async fn api_admin_affiliates_pending(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let rows = sqlx::query!(
        r#"SELECT a.user_id::text as id, u.email,
                  a.traffic_source, a.audience_size, a.main_url, a.phone_number, a.company_name,
                  a.tax_id_last4,
                  a.created_at::text, COALESCE(up.first_name, '') as first_name, COALESCE(up.last_name, '') as last_name
           FROM affiliates a
           JOIN users u ON u.id = a.user_id
           LEFT JOIN user_profiles up ON up.user_id = a.user_id
           WHERE a.status = 'pending_approval'
           ORDER BY a.created_at DESC"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Admin affiliate pending query failed: {e}");
        ApiError::Database(e)
    })?;

    let counts = sqlx::query!(
        r#"SELECT
              COUNT(*) FILTER (WHERE status = 'pending_approval') as "pending!",
              COUNT(*) FILTER (WHERE status = 'active') as "active!",
              COUNT(*) FILTER (WHERE status = 'terminated') as "rejected!"
           FROM affiliates"#
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Admin affiliate count query failed: {e}");
        ApiError::Database(e)
    })?;

    // Per-application backend fraud signals: detect re-use of phone, tax ID,
    // company name, and email domain across other affiliates. Cheap aggregate
    // counts; complements the frontend heuristics (disposable domains, etc.).
    // Dupe detection now matches on `tax_id_encrypted` (full ciphertext)
    // rather than the dropped plaintext column. Ciphertext equality requires
    // identical (key, nonce, plaintext) — but nonces are random per row, so
    // identical plaintext yields different ciphertext. The cheap fallback is
    // `tax_id_last4` for a coarse signal; admins follow up via the case
    // workflow. A deterministic hash (HMAC over plaintext with a separate
    // key) would be the strong fix; tracked separately.
    let dupe_rows = sqlx::query!(
        r#"WITH siblings AS (
              SELECT a.user_id, a.phone_number, a.tax_id_last4, a.company_name,
                     LOWER(SPLIT_PART(u.email, '@', 2)) AS email_domain
              FROM affiliates a
              JOIN users u ON u.id = a.user_id
           )
           SELECT s.user_id::text AS id,
                  COALESCE((SELECT COUNT(*) FROM siblings s2
                            WHERE s2.phone_number IS NOT NULL
                              AND s2.phone_number = s.phone_number
                              AND s2.user_id <> s.user_id), 0) AS "phone_dupe!",
                  COALESCE((SELECT COUNT(*) FROM siblings s2
                            WHERE s2.tax_id_last4 IS NOT NULL
                              AND s2.tax_id_last4 = s.tax_id_last4
                              AND s2.user_id <> s.user_id), 0) AS "tax_last4_dupe!",
                  COALESCE((SELECT COUNT(*) FROM siblings s2
                            WHERE s2.company_name IS NOT NULL
                              AND LOWER(s2.company_name) = LOWER(s.company_name)
                              AND s2.user_id <> s.user_id), 0) AS "company_dupe!",
                  COALESCE((SELECT COUNT(*) FROM siblings s2
                            WHERE s2.email_domain = s.email_domain
                              AND s2.user_id <> s.user_id), 0) AS "domain_count!"
           FROM siblings s
           WHERE s.user_id IN (SELECT a.user_id FROM affiliates a WHERE a.status = 'pending_approval')"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Admin affiliate dupe scan failed: {e}");
        ApiError::Database(e)
    })?;

    let dupes: std::collections::HashMap<String, &_> = dupe_rows
        .iter()
        .map(|r| (r.id.clone().unwrap_or_default(), r))
        .collect();

    let pending: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let user_name = format!(
                "{} {}",
                r.first_name.clone().unwrap_or_default(),
                r.last_name.clone().unwrap_or_default()
            )
            .trim()
            .to_string();

            let mut signals: Vec<serde_json::Value> = Vec::new();
            if let Some(d) = r.id.as_ref().and_then(|id| dupes.get(id)) {
                if d.phone_dupe > 0 {
                    signals.push(serde_json::json!({ "kind": "phone_dupe", "score": 35, "label": format!("Phone reused by {} other affiliate(s)", d.phone_dupe) }));
                }
                if d.tax_last4_dupe > 0 {
                    // Coarser signal than plaintext tax-id match (we now key
                    // on last4 only). Lower score to reflect higher false-
                    // positive rate; admins still see the flag and can dig in.
                    signals.push(serde_json::json!({ "kind": "tax_last4_dupe", "score": 25, "label": format!("Tax ID last-4 reused by {} other affiliate(s)", d.tax_last4_dupe) }));
                }
                if d.company_dupe > 0 {
                    signals.push(serde_json::json!({ "kind": "company_dupe", "score": 15, "label": format!("Company name matches {} other affiliate(s)", d.company_dupe) }));
                }
                if d.domain_count >= 5 {
                    signals.push(serde_json::json!({ "kind": "domain_volume", "score": 10, "label": format!("Email domain has {} affiliates already", d.domain_count) }));
                }
            }

            serde_json::json!({
                "id": r.id, "email": r.email, "user_name": if user_name.is_empty() { r.email.clone() } else { user_name },
                "traffic_source": r.traffic_source,
                "audience_size": r.audience_size, "main_url": r.main_url, "phone_number": r.phone_number,
                "company_name": r.company_name,
                "tax_id_last4": r.tax_id_last4,
                "created_at": r.created_at,
                "fraud_signals": signals
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "pending": pending,
        "counts": {
            "pending": counts.pending,
            "active": counts.active,
            "rejected": counts.rejected
        }
    }))
    .into_response())
}

/// Payload for approving an affiliate application.
#[derive(serde::Deserialize)]
pub struct AdminApproveAffiliatePayload {
    /// The public referral code assigned by the admin.
    pub referral_code: String,
    /// Commission rate in basis points. 50 bps = 0.50%.
    pub commission_rate_bps: i32,
}

/// POST /api/admin/rewards/affiliates/:id/approve
pub async fn api_admin_affiliate_approve(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<AdminApproveAffiliatePayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let uid = ApiError::parse_uuid(&id)?;
    let referral_code = payload.referral_code.trim().to_uppercase();
    let commission_rate_bps = payload.commission_rate_bps;

    if referral_code.len() < 3 || referral_code.len() > 20 {
        return Err(ApiError::BadRequest(
            "Referral code must be 3-20 characters.".to_string(),
        ));
    }

    if !referral_code
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(ApiError::BadRequest(
            "Referral code may only contain letters, numbers, underscores, and hyphens."
                .to_string(),
        ));
    }

    if !(1..=450).contains(&commission_rate_bps) {
        return Err(ApiError::BadRequest(
            "Commission rate must be 1-450 basis points.".to_string(),
        ));
    }

    // E.3 KYC Gating: Do not allow approval if KYC is not approved
    let kyc_res = crate::kyc::service::get_kyc_status(&state.db, uid).await;
    match kyc_res {
        Ok(kyc) if kyc.status != "approved" => {
            return Err(ApiError::BadRequest(format!(
                "Cannot approve affiliate: User KYC status is '{}' (must be 'approved')",
                kyc.status
            )));
        }
        Err(_) => {
            return Err(ApiError::BadRequest(
                "Cannot approve affiliate: Could not verify KYC status".to_string(),
            ))
        }
        _ => {}
    }

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin affiliate approval tx: {e}");
        ApiError::Internal("Server error".to_string())
    })?;

    // Lock the affiliate row to prevent concurrent approval.
    let current_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM affiliates WHERE user_id = $1 FOR UPDATE")
            .bind(uid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lock affiliate row: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

    match current_status.as_deref() {
        Some("pending_approval") => {}
        Some("active") => {
            let _ = tx.rollback().await;
            return Err(ApiError::BadRequest(
                "Affiliate is already active".to_string(),
            ));
        }
        Some(other) => {
            let _ = tx.rollback().await;
            return Err(ApiError::BadRequest(format!(
                "Cannot approve affiliate in '{}' status",
                other
            )));
        }
        None => {
            let _ = tx.rollback().await;
            return Err(ApiError::NotFound(
                "Affiliate application not found".to_string(),
            ));
        }
    }

    let code_owner: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM affiliates WHERE referral_code = $1 AND user_id <> $2",
    )
    .bind(&referral_code)
    .bind(uid)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to validate affiliate referral code uniqueness: {e}");
        ApiError::Internal("Database error".to_string())
    })?;

    if code_owner.is_some() {
        let _ = tx.rollback().await;
        return Err(ApiError::Conflict(
            "Referral code is already assigned to another affiliate.".to_string(),
        ));
    }

    let result = sqlx::query!(
        r#"UPDATE affiliates
           SET status = 'active', referral_code = $1, commission_rate_bps = $2, approved_at = NOW()
           WHERE user_id = $3 AND status = 'pending_approval'"#,
        referral_code,
        commission_rate_bps,
        uid
    )
    .execute(&mut *tx)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            sqlx::query!(
                r#"INSERT INTO affiliate_links
                      (code, link_type, attribution_user_id, payout_user_id, team_id, status)
                   VALUES ($1, 'personal', $2, $2, NULL, 'active')
                   ON CONFLICT (code) DO UPDATE SET
                      attribution_user_id = EXCLUDED.attribution_user_id,
                      payout_user_id = EXCLUDED.payout_user_id,
                      team_id = NULL,
                      status = 'active',
                      deactivated_at = NULL,
                      deactivated_reason = NULL,
                      updated_at = NOW()"#,
                referral_code,
                uid
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upsert affiliate personal link on approval: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

            sqlx::query!(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)",
                admin.user.id,
                "affiliate.approved",
                "affiliate",
                uid,
                serde_json::json!({ "referral_code": referral_code, "commission_rate_bps": commission_rate_bps })
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write affiliate approval audit log: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

            tx.commit().await.map_err(|e| {
                tracing::error!("Failed to commit affiliate approval tx: {e}");
                ApiError::Internal("Server error".to_string())
            })?;

            tracing::info!(
                affiliate_id = %uid,
                referral_code = %referral_code,
                "Affiliate application approved"
            );

            // Branded approval via the durable outbox — POOOL email shell,
            // retry on provider failure, suppression check, admin workflow
            // toggle, List-Unsubscribe support.
            let _ = crate::email::trigger_transactional_email(
                &state.db,
                &uid,
                "affiliate_approved",
                serde_json::json!({ "referral_code": referral_code }),
            )
            .await;

            Ok(
                Json(serde_json::json!({"status": "approved", "referral_code": referral_code}))
                    .into_response(),
            )
        }
        Ok(_) => {
            let _ = tx.rollback().await;
            Err(ApiError::BadRequest(
                "Affiliate application not found or already processed".to_string(),
            ))
        }
        Err(e) => {
            let _ = tx.rollback().await;
            if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                return Err(ApiError::Conflict(
                    "Referral code is already assigned to another affiliate.".to_string(),
                ));
            }
            tracing::error!("Failed to approve affiliate {}: {}", id, e);
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// Payload for rejecting an affiliate application.
#[derive(serde::Deserialize)]
pub struct AdminRejectAffiliatePayload {
    /// The administrative reason for rejection.
    pub reason: String,
}

/// POST /api/admin/rewards/affiliates/:id/reject
pub async fn api_admin_affiliate_reject(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<AdminRejectAffiliatePayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let uid = ApiError::parse_uuid(&id)?;
    let reason = crate::common::sanitize::sanitize_text(payload.reason.trim());
    if reason.is_empty() {
        return Err(ApiError::BadRequest(
            "A rejection reason is required.".to_string(),
        ));
    }
    if reason.chars().count() > AFFILIATE_REJECTION_REASON_MAX_CHARS {
        return Err(ApiError::BadRequest(format!(
            "Rejection reason must be {} characters or fewer.",
            AFFILIATE_REJECTION_REASON_MAX_CHARS
        )));
    }

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin affiliate rejection tx: {e}");
        ApiError::Internal("Server error".to_string())
    })?;

    let current_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM affiliates WHERE user_id = $1 FOR UPDATE")
            .bind(uid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to lock affiliate row for rejection: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

    match current_status.as_deref() {
        Some("pending_approval") => {}
        Some(other) => {
            let _ = tx.rollback().await;
            return Err(ApiError::BadRequest(format!(
                "Cannot reject affiliate in '{}' status",
                other
            )));
        }
        None => {
            let _ = tx.rollback().await;
            return Err(ApiError::NotFound(
                "Affiliate application not found".to_string(),
            ));
        }
    }

    let result = sqlx::query!(
        r#"UPDATE affiliates
           SET status = 'terminated'
           WHERE user_id = $1 AND status = 'pending_approval'"#,
        uid
    )
    .execute(&mut *tx)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            sqlx::query!(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state) VALUES ($1, $2, $3, $4, $5, $6)",
                admin.user.id,
                "affiliate.rejected",
                "affiliate",
                uid,
                serde_json::json!({ "status": "pending_approval" }),
                serde_json::json!({ "status": "terminated", "reason": reason.clone() })
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write affiliate rejection audit log: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

            tx.commit().await.map_err(|e| {
                tracing::error!("Failed to commit affiliate rejection tx: {e}");
                ApiError::Internal("Server error".to_string())
            })?;

            // Branded rejection via the durable outbox.
            let _ = crate::email::trigger_transactional_email(
                &state.db,
                &uid,
                "affiliate_rejected",
                serde_json::json!({ "reason": reason }),
            )
            .await;

            Ok(Json(serde_json::json!({"status": "rejected"})).into_response())
        }
        Ok(_) => {
            let _ = tx.rollback().await;
            Err(ApiError::BadRequest(
                "Affiliate application not found or already processed".to_string(),
            ))
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!("Failed to reject affiliate {}: {}", id, e);
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// Payload for requesting more info from a pending affiliate.
#[derive(serde::Deserialize)]
pub struct AdminRequestInfoPayload {
    /// The questions / additional info requested from the applicant.
    pub message: String,
}

/// POST /api/admin/rewards/affiliates/:id/request-info
///
/// Sends a clarification email to a pending applicant without changing status.
/// Logs an audit entry so the request is visible in the audit trail.
pub async fn api_admin_affiliate_request_info(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<AdminRequestInfoPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let uid = ApiError::parse_uuid(&id)?;
    let message = crate::common::sanitize::sanitize_text(payload.message.trim());
    if message.is_empty() {
        return Err(ApiError::BadRequest("A message is required.".to_string()));
    }
    if message.chars().count() > AFFILIATE_REJECTION_REASON_MAX_CHARS {
        return Err(ApiError::BadRequest(format!(
            "Message must be {} characters or fewer.",
            AFFILIATE_REJECTION_REASON_MAX_CHARS
        )));
    }

    let row = sqlx::query!(
        r#"SELECT u.email, a.status FROM affiliates a
           JOIN users u ON u.id = a.user_id
           WHERE a.user_id = $1"#,
        uid
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch affiliate for request-info: {e}");
        ApiError::Internal("Database error".to_string())
    })?;

    let Some(row) = row else {
        return Err(ApiError::NotFound(
            "Affiliate application not found".to_string(),
        ));
    };

    let status = row.status.as_deref().unwrap_or("");
    if status != "pending_approval" {
        return Err(ApiError::BadRequest(format!(
            "Cannot request more info for affiliate in '{}' status",
            status
        )));
    }

    sqlx::query!(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)",
        admin.user.id,
        "affiliate.info_requested",
        "affiliate",
        uid,
        serde_json::json!({ "message": message.clone() })
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to log info-requested audit: {e}");
        ApiError::Internal("Database error".to_string())
    })?;

    let _ = crate::email::trigger_transactional_email(
        &state.db,
        &uid,
        "affiliate_application_info_requested",
        serde_json::json!({ "message": message }),
    )
    .await;

    Ok(Json(serde_json::json!({ "status": "info_requested" })).into_response())
}

/// Payload for suspending an affiliate
#[derive(serde::Deserialize)]
pub struct AdminSuspendAffiliatePayload {
    /// The reason for suspending the active affiliate
    pub reason: String,
}

/// POST /api/admin/rewards/affiliates/:id/suspend
pub async fn api_admin_affiliate_suspend(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(payload): Json<AdminSuspendAffiliatePayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let uid = ApiError::parse_uuid(&id)?;
    let reason = crate::common::sanitize::sanitize_text(payload.reason.trim());
    if reason.is_empty() {
        return Err(ApiError::BadRequest(
            "A suspension reason is required.".to_string(),
        ));
    }

    let result = sqlx::query!(
        r#"UPDATE affiliates 
           SET status = 'suspended'
           WHERE user_id = $1 AND status = 'active'"#,
        uid
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            let _ = sqlx::query!(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)",
                admin.user.id,
                "affiliate.suspended",
                "affiliate",
                uid,
                serde_json::json!({ "reason": reason.clone() })
            ).execute(&state.db).await;

            // Branded suspension via the durable outbox.
            let _ = crate::email::trigger_transactional_email(
                &state.db,
                &uid,
                "affiliate_suspended",
                serde_json::json!({ "reason": reason }),
            )
            .await;

            Ok(Json(serde_json::json!({"status": "suspended"})).into_response())
        }
        Ok(_) => Err(ApiError::BadRequest(
            "Affiliate is not active or could not be found".to_string(),
        )),
        Err(e) => {
            tracing::error!("Failed to suspend affiliate {}: {}", id, e);
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// GET /api/admin/rewards/affiliates/payouts/pending
/// Groups all 'payable' commissions by affiliate so the admin knows who needs to be paid.
pub async fn api_admin_affiliate_payouts_pending(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    use sqlx::Row;

    let rows = sqlx::query(
        r#"SELECT 
            u.id as user_id, u.email, 
            a.referral_code,
            a.tax_document_gcs_path IS NOT NULL as tax_document_uploaded,
            pr.id as payout_request_id,
            pr.amount_cents as payout_request_amount_cents,
            pr.requested_at::text as payout_requested_at,
            pr.status as payout_request_status,
            COALESCE(up.first_name, '') as fn, COALESCE(up.last_name, '') as ln,
            SUM(ac.provisional_amount_cents)::bigint as total_payable_cents,
            COUNT(ac.id)::bigint as commission_count
           FROM affiliate_commissions ac
           JOIN affiliates a ON a.user_id = ac.affiliate_id
           JOIN users u ON u.id = a.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           LEFT JOIN LATERAL (
               SELECT id, amount_cents, requested_at, status
               FROM affiliate_payout_requests
               WHERE affiliate_id = a.user_id
                 AND status IN ('requested', 'processing')
               ORDER BY requested_at ASC
               LIMIT 1
           ) pr ON true
           WHERE ac.status = 'payable'
           GROUP BY u.id, u.email, a.referral_code, a.tax_document_gcs_path, pr.id, pr.amount_cents, pr.requested_at, pr.status, up.first_name, up.last_name
           ORDER BY total_payable_cents DESC"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch pending payouts: {}", e);
        ApiError::Internal("Database error".into())
    })?;

    let payouts = rows
        .into_iter()
        .map(|r| {
            let email: String = r.try_get("email").unwrap_or_default();
            let user_id: uuid::Uuid = r.try_get("user_id").unwrap_or_default();
            let referral_code: String = r.try_get("referral_code").unwrap_or_default();
            let total_payable_cents: i64 = r.try_get("total_payable_cents").unwrap_or(0);
            let commission_count: i64 = r.try_get("commission_count").unwrap_or(0);
            let tax_document_uploaded: bool = r.try_get("tax_document_uploaded").unwrap_or(false);
            let payout_request_id: Option<uuid::Uuid> =
                r.try_get("payout_request_id").unwrap_or(None);
            let payout_request_amount_cents: Option<i64> =
                r.try_get("payout_request_amount_cents").unwrap_or(None);
            let payout_requested_at: Option<String> =
                r.try_get("payout_requested_at").unwrap_or(None);
            let payout_request_status: Option<String> =
                r.try_get("payout_request_status").unwrap_or(None);
            let mut name = format!(
                "{} {}",
                r.try_get::<Option<String>, _>("fn")
                    .unwrap_or(None)
                    .unwrap_or_default(),
                r.try_get::<Option<String>, _>("ln")
                    .unwrap_or(None)
                    .unwrap_or_default()
            )
            .trim()
            .to_string();
            if name.is_empty() {
                name = email.clone();
            }
            serde_json::json!({
                "affiliate_id": user_id,
                "email": email,
                "name": name,
                "referral_code": referral_code,
                "total_payable_cents": total_payable_cents,
                "commission_count": commission_count,
                "tax_document_uploaded": tax_document_uploaded,
                "payout_blocked_reason": if tax_document_uploaded { serde_json::Value::Null } else { serde_json::json!("Tax document required before release") },
                "payout_request_id": payout_request_id,
                "payout_request_amount_cents": payout_request_amount_cents,
                "payout_requested_at": payout_requested_at,
                "payout_request_status": payout_request_status,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(payouts).into_response())
}

/// POST /api/admin/rewards/affiliates/:id/payout
/// Executes a batch payout for all currently 'payable' commissions for the given affiliate.
pub async fn api_admin_affiliate_batch_payout(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(affiliate_id): axum::extract::Path<uuid::Uuid>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    // We execute this in an ACID transaction
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Tx start failed: {}", e);
        ApiError::Internal("Tx start failed".into())
    })?;

    // 1. Lock the payable commissions
    let commissions = sqlx::query!(
        r#"SELECT id, provisional_amount_cents 
           FROM affiliate_commissions 
           WHERE affiliate_id = $1 AND status = 'payable'
           FOR UPDATE SKIP LOCKED"#,
        affiliate_id
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Could not lock commissions".into()))?;

    if commissions.is_empty() {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest(
            "No payable commissions found for this affiliate (or locked).".into(),
        ));
    }

    let total_payable_cents: i64 = commissions.iter().map(|c| c.provisional_amount_cents).sum();

    // B.3 Minimum Payout Threshold ($50.00)
    if total_payable_cents < 5000 {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest(
            "Total payable balance is below the minimum threshold of $50.00".into(),
        ));
    }

    // GAP-10: Tax document gate — require W-9/W-8BEN before releasing payout
    // Non-macro: tax_document_gcs_path column added in migration 076
    let has_tax_doc: bool = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT tax_document_gcs_path IS NOT NULL FROM affiliates WHERE user_id = $1",
    )
    .bind(affiliate_id)
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None)
    .flatten()
    .unwrap_or(false);

    if !has_tax_doc {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest("This affiliate has not yet uploaded a W-9/W-8BEN tax document. Payout is blocked until a valid tax form is on file.".into()));
    }

    // 2. Fetch system treasury wallet
    let treasury_wallet = sqlx::query!(
        r#"SELECT id, balance_cents 
           FROM wallets 
           WHERE wallet_type = 'affiliate_treasury' AND currency = 'USD'
           LIMIT 1
           FOR UPDATE"#
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Could not access treasury wallet".into()))?;

    let treasury_wallet = match treasury_wallet {
        Some(w) => w,
        None => {
            let _ = tx.rollback().await;
            return Err(ApiError::BadRequest(
                "Affiliate Treasury Wallet not configured.".into(),
            ));
        }
    };

    if treasury_wallet.balance_cents < total_payable_cents {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest(
            "Insufficient funds in Affiliate Treasury Wallet.".into(),
        ));
    }

    // 3. Fetch/Create Affiliate's cash wallet
    let dest_wallet = sqlx::query!(
        r#"INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
           VALUES ($1, 'cash', 'USD', 0)
           ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET balance_cents = wallets.balance_cents
           RETURNING id"#,
        affiliate_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Could not get destination wallet".into()))?;

    // 4. Create payout batch
    let batch_id = uuid::Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO payout_batches (id, affiliate_id, total_amount_cents, status, created_by_admin_id, paid_at)
           VALUES ($1, $2, $3, 'paid', $4, NOW())"#,
        batch_id,
        affiliate_id,
        total_payable_cents,
        admin.user.id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to create payout batch".into()))?;

    // 5. Update only the exact commission rows that this transaction locked and summed.
    let commission_ids: Vec<uuid::Uuid> = commissions.iter().map(|c| c.id).collect();
    let updated_commissions = sqlx::query(
        r#"UPDATE affiliate_commissions
           SET status = 'paid', payout_batch_id = $1, updated_at = NOW()
           WHERE id = ANY($2)
             AND affiliate_id = $3
             AND status = 'payable'"#,
    )
    .bind(batch_id)
    .bind(&commission_ids)
    .bind(affiliate_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to update commissions".into()))?;

    if updated_commissions.rows_affected() != commissions.len() as u64 {
        let _ = tx.rollback().await;
        return Err(ApiError::Internal(
            "Payout commission set changed while processing batch".into(),
        ));
    }

    // 6. Move money (Debit Treasury)
    sqlx::query!(
        r#"UPDATE wallets SET balance_cents = balance_cents - $1, updated_at = NOW() WHERE id = $2"#,
        total_payable_cents,
        treasury_wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to debit treasury".into()))?;

    // Credit Affiliate
    sqlx::query!(
        r#"UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE id = $2"#,
        total_payable_cents,
        dest_wallet.id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to credit affiliate".into()))?;

    // 7. Write wallet transactions
    sqlx::query!(
        r#"INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description, external_ref_id, completed_at)
           VALUES ($1, 'fee', 'completed', $2, 'USD', 'Affiliate Payout Debit', $3, NOW())"#,
        treasury_wallet.id,
        -total_payable_cents,
        batch_id.to_string()
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to log treasury tx".into()))?;

    sqlx::query!(
        r#"INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description, external_ref_id, completed_at)
           VALUES ($1, 'reward', 'completed', $2, 'USD', 'Affiliate Commission Payout', $3, NOW())"#,
        dest_wallet.id,
        total_payable_cents,
        batch_id.to_string()
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to log affiliate tx".into()))?;

    // 8. Write Audit Log
    sqlx::query!(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
           VALUES ($1, 'AFFILIATE_BATCH_PAYOUT_EXECUTED', 'payout_batches', $2, $3)"#,
        admin.user.id,
        batch_id,
        serde_json::json!({
            "affiliate_id": affiliate_id,
            "amount_cents": total_payable_cents,
            "commission_count": commissions.len(),
            "dest_wallet_id": dest_wallet.id
        })
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to write audit log".into()))?;

    sqlx::query(
        r#"UPDATE affiliate_payout_requests
           SET status = 'paid',
               processed_at = NOW(),
               processed_by_admin_id = $1,
               payout_batch_id = $2,
               updated_at = NOW()
           WHERE affiliate_id = $3
             AND status IN ('requested', 'processing')"#,
    )
    .bind(admin.user.id)
    .bind(batch_id)
    .bind(affiliate_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to update payout requests".into()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("Commit failed".into()))?;

    // Phase-3 fresh: issue the affiliate invoice for this batch+affiliate.
    // Non-fatal — the payout is already committed; an invoice failure
    // (e.g. transient pool issue) is loggable but does not roll back the
    // money movement. Issuing here also fires the in-app bell ping via
    // `notify_payout_released` inside `issue_affiliate_invoice`.
    if let Err(e) =
        crate::rewards::service::issue_affiliate_invoice(&state.db, batch_id, affiliate_id).await
    {
        tracing::warn!(
            batch_id = %batch_id,
            affiliate_id = %affiliate_id,
            error = %e,
            "issue_affiliate_invoice failed post-payout (non-fatal)"
        );
    }

    // Branded payout notification via the durable outbox.
    let _ = crate::email::trigger_transactional_email(
        &state.db,
        &affiliate_id,
        "affiliate_payout_released",
        serde_json::json!({
            "amount_cents": total_payable_cents,
            "currency": "USD",
        }),
    )
    .await;

    tracing::info!(
        "Admin {} executed batch payout {} for affiliate {} amount {}",
        admin.user.id,
        batch_id,
        affiliate_id,
        total_payable_cents
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "batch_id": batch_id,
        "amount_cents": total_payable_cents,
        "commission_count": commissions.len()
    }))
    .into_response())
}

/// Payload for POST /api/admin/rewards/affiliates/:id/clawback
#[derive(serde::Deserialize)]
pub struct ClawbackPayload {
    /// Administrative reason for the clawback
    pub reason: String,
    /// Specific paid commission IDs to reverse. Required (non-empty).
    /// Each must belong to the affiliate and currently be in status='paid'.
    pub commission_ids: Vec<uuid::Uuid>,
}

/// GET /api/admin/rewards/affiliates/fraud-scan
/// Scans the affiliate network for circular referral rings and flags them for review.
pub async fn api_admin_affiliate_fraud_scan(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AffiliateFraudScanQuery>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;
    let scan_type = query.scan_type.as_deref().unwrap_or("circular");
    if !matches!(scan_type, "circular" | "ip_overlap") {
        return Err(ApiError::BadRequest(
            "Unsupported fraud scan type".to_string(),
        ));
    }

    let flags = match scan_type {
        "ip_overlap" => crate::rewards::service::scan_affiliate_ip_overlaps(&state.db).await,
        _ => crate::rewards::service::scan_affiliate_fraud_rings(&state.db).await,
    }
    .map_err(|e| {
        tracing::error!("Fraud scan failed: {}", e);
        ApiError::Internal("Fraud scan failed".into())
    })?;

    let count = flags.len();
    let elements = crate::rewards::service::affiliate_fraud_flags_to_cytoscape_elements(&flags);
    tracing::info!(
        "Affiliate fraud scan complete: {} finding(s) detected for {}",
        count,
        scan_type
    );

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
        VALUES ($1, 'affiliate_fraud.scan_viewed', 'affiliate_fraud_scan', $2)
        "#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({
        "scan_type": scan_type,
        "count": count,
    }))
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    Ok(axum::response::Json(serde_json::json!({
        "success": true,
        "scan_type": scan_type,
        "flags": flags,
        "elements": elements,
        "count": count
    }))
    .into_response())
}

// ─── Affiliate Conduct Incidents (Phase 1 monitoring — blueprint Point 2 §F) ───
// Records persuasion / advisory-selling / cold-outreach / side-deal indicators.
// Distinct from referral-ring fraud detection (which is automated).

#[allow(missing_docs)]
#[derive(serde::Deserialize)]
pub struct ConductIncidentCreatePayload {
    pub affiliate_id: String,
    pub incident_type: String,
    pub severity: Option<String>,
    pub source: Option<String>,
    pub description: String,
    pub evidence_url: Option<String>,
    pub content_snippet: Option<String>,
    pub action_taken: Option<String>,
}

#[allow(missing_docs)]
#[derive(serde::Deserialize)]
pub struct ConductIncidentListQuery {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub affiliate_id: Option<String>,
    pub limit: Option<i64>,
}

/// POST /api/admin/rewards/affiliates/conduct-incidents
/// Record a conduct incident (advisory selling, cold outreach, side deals, etc.).
pub async fn api_admin_affiliate_conduct_incident_create(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<ConductIncidentCreatePayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let affiliate_id = ApiError::parse_uuid(&payload.affiliate_id)?;
    let severity = payload.severity.as_deref().unwrap_or("minor");
    if !matches!(severity, "minor" | "serious" | "critical") {
        return Err(ApiError::BadRequest("Invalid severity".into()));
    }
    let source = payload.source.as_deref().unwrap_or("manual");
    if !matches!(
        source,
        "user_complaint" | "support_escalation" | "admin_review" | "automated" | "manual"
    ) {
        return Err(ApiError::BadRequest("Invalid source".into()));
    }

    let row = sqlx::query!(
        r#"
        INSERT INTO affiliate_conduct_incidents
            (affiliate_id, incident_type, severity, source, description,
             evidence_url, content_snippet, action_taken, reported_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, created_at
        "#,
        affiliate_id,
        payload.incident_type,
        severity,
        source,
        payload.description,
        payload.evidence_url,
        payload.content_snippet,
        payload.action_taken,
        admin.user.id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::from)?;

    sqlx::query(
        r#"
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
        VALUES ($1, 'affiliate_conduct.incident_created', 'affiliate_conduct_incident', $2, $3)
        "#,
    )
    .bind(admin.user.id)
    .bind(row.id)
    .bind(serde_json::json!({
        "affiliate_id": affiliate_id,
        "incident_type": payload.incident_type,
        "severity": severity,
    }))
    .execute(&state.db)
    .await
    .ok();

    Ok(axum::response::Json(serde_json::json!({
        "success": true,
        "incident_id": row.id,
        "created_at": row.created_at,
    }))
    .into_response())
}

/// GET /api/admin/rewards/affiliates/conduct-incidents
/// List conduct incidents (filterable by status / severity / affiliate).
pub async fn api_admin_affiliate_conduct_incident_list(
    admin: AdminUser,
    State(state): State<AppState>,
    Query(query): Query<ConductIncidentListQuery>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let status = query.status.as_deref();
    let severity = query.severity.as_deref();
    let affiliate_id = query
        .affiliate_id
        .as_deref()
        .map(ApiError::parse_uuid)
        .transpose()?;
    let limit = query.limit.unwrap_or(100).clamp(1, 500);

    let rows = sqlx::query!(
        r#"
        SELECT i.id, i.affiliate_id, i.incident_type, i.severity, i.status,
               i.source, i.description, i.evidence_url, i.content_snippet,
               i.action_taken, i.reported_by, i.reviewed_by, i.reviewed_at,
               i.review_notes, i.created_at, i.updated_at,
               u.email AS affiliate_email
          FROM affiliate_conduct_incidents i
          LEFT JOIN users u ON u.id = i.affiliate_id
         WHERE ($1::text IS NULL OR i.status = $1)
           AND ($2::text IS NULL OR i.severity = $2)
           AND ($3::uuid IS NULL OR i.affiliate_id = $3)
         ORDER BY i.created_at DESC
         LIMIT $4
        "#,
        status,
        severity,
        affiliate_id,
        limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "affiliate_id": r.affiliate_id,
                "affiliate_email": r.affiliate_email,
                "incident_type": r.incident_type,
                "severity": r.severity,
                "status": r.status,
                "source": r.source,
                "description": r.description,
                "evidence_url": r.evidence_url,
                "content_snippet": r.content_snippet,
                "action_taken": r.action_taken,
                "created_at": r.created_at,
                "updated_at": r.updated_at,
            })
        })
        .collect();

    Ok(axum::response::Json(serde_json::json!({
        "success": true,
        "count": items.len(),
        "incidents": items,
    }))
    .into_response())
}

/// POST /api/admin/rewards/affiliates/:id/clawback
/// Reverses paid commissions if fraud is detected post-payout.
pub async fn api_admin_affiliate_clawback(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(payload): axum::extract::Json<ClawbackPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    if payload.commission_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "commission_ids must list at least one paid commission".into(),
        ));
    }
    if payload.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("reason is required".into()));
    }

    let affiliate_id = ApiError::parse_uuid(&id)?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| ApiError::Internal("Transaction start failed".into()))?;

    // Lock only the requested commissions, scoped to this affiliate and status='paid'.
    // Any id that doesn't match (wrong affiliate, wrong status, or not found) is
    // silently excluded — the count check below rejects partial matches.
    let commissions = sqlx::query!(
        "SELECT id, provisional_amount_cents FROM affiliate_commissions \
         WHERE affiliate_id = $1 AND status = 'paid' AND id = ANY($2) FOR UPDATE",
        affiliate_id,
        &payload.commission_ids
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Fetch error".into()))?;

    if commissions.len() != payload.commission_ids.len() {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest(
            "One or more commission_ids do not belong to this affiliate or are not in 'paid' status".into(),
        ));
    }

    let total_clawback_cents: i64 = commissions.iter().map(|c| c.provisional_amount_cents).sum();
    let locked_ids: Vec<uuid::Uuid> = commissions.iter().map(|c| c.id).collect();

    // Deduct from affiliate wallet — cap at available balance to prevent negative
    let dest_wallet = sqlx::query!(
        "SELECT id, balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = 'cash' AND currency = 'USD' FOR UPDATE",
        affiliate_id
    )
    .fetch_optional(&mut *tx).await.map_err(|_| ApiError::Internal("Wallet error".into()))?;

    let actual_deducted = if let Some(w) = &dest_wallet {
        // Never deduct more than what's available — affiliate may have already withdrawn
        let deductible = std::cmp::min(total_clawback_cents, w.balance_cents);
        if deductible > 0 {
            sqlx::query!("UPDATE wallets SET balance_cents = balance_cents - $1, updated_at = NOW() WHERE id = $2", deductible, w.id)
                .execute(&mut *tx).await
                .map_err(|_| ApiError::Internal("Failed to debit affiliate wallet".into()))?;
        }
        deductible
    } else {
        0i64
    };

    // Refund treasury with the amount actually deducted from the affiliate
    if actual_deducted > 0 {
        sqlx::query!("UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE wallet_type = 'affiliate_treasury' AND currency = 'USD'", actual_deducted)
            .execute(&mut *tx).await
            .map_err(|_| ApiError::Internal("Failed to credit treasury".into()))?;
    }

    // Update only the locked commissions to 'clawed_back'
    sqlx::query!(
        "UPDATE affiliate_commissions SET status = 'clawed_back', updated_at = NOW() \
         WHERE id = ANY($1) AND status = 'paid'",
        &locked_ids
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to update commissions".into()))?;

    sqlx::query!(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata) VALUES ($1, 'affiliate_clawback', 'affiliates', $2, $3)",
        admin.user.id,
        affiliate_id,
        serde_json::json!({
            "commission_ids": locked_ids,
            "commission_count": locked_ids.len(),
            "clawback_total_cents": total_clawback_cents,
            "actual_deducted_cents": actual_deducted,
            "shortfall_cents": total_clawback_cents - actual_deducted,
            "reason": payload.reason
        })
    ).execute(&mut *tx).await
    .map_err(|_| ApiError::Internal("Failed to write audit log".into()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("Commit failed".into()))?;

    tracing::info!(
        admin_id = %admin.user.id,
        affiliate_id = %affiliate_id,
        commission_count = locked_ids.len(),
        total_cents = total_clawback_cents,
        deducted_cents = actual_deducted,
        "[P0-FINANCIAL] Affiliate clawback executed"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "commission_ids": locked_ids,
        "commission_count": locked_ids.len(),
        "clawed_back_cents": total_clawback_cents,
        "actual_deducted_cents": actual_deducted,
        "shortfall_cents": total_clawback_cents - actual_deducted
    }))
    .into_response())
}

// ── Admin Materials Review (GAP-11) ─────────────────────────────────────────

/// GET /api/admin/rewards/affiliates/materials
/// Lists all affiliate custom marketing materials pending review.
pub async fn api_admin_affiliate_materials_list(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    // Non-macro: affiliate_materials table added in migration 076
    let rows = sqlx::query(
        r#"SELECT am.id::text as id, am.asset_name, am.gcs_path, am.status,
                  am.created_at::text as created_at, am.review_note,
                  u.email, a.referral_code
           FROM affiliate_materials am
           JOIN affiliates a ON a.user_id = am.affiliate_id
           JOIN users u ON u.id = am.affiliate_id
           ORDER BY am.created_at DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to fetch affiliate materials: {}", e);
        ApiError::Internal("Database error".into())
    })?;

    let materials: Vec<serde_json::Value> = rows.iter().map(|r| {
        use sqlx::Row;
        serde_json::json!({
            "id": r.try_get::<Option<String>, _>("id").unwrap_or_default(),
            "affiliate_email": r.try_get::<Option<String>, _>("email").unwrap_or_default(),
            "affiliate_code": r.try_get::<Option<String>, _>("referral_code").unwrap_or_default(),
            "asset_name": r.try_get::<Option<String>, _>("asset_name").unwrap_or_default(),
            "gcs_path": r.try_get::<Option<String>, _>("gcs_path").unwrap_or_default(),
            "status": r.try_get::<Option<String>, _>("status").unwrap_or_default(),
            "review_note": r.try_get::<Option<String>, _>("review_note").unwrap_or_default(),
            "created_at": r.try_get::<Option<String>, _>("created_at").unwrap_or_default()
        })
    }).collect();

    Ok(Json(serde_json::json!({ "materials": materials })).into_response())
}

/// Payload for approving or rejecting a marketing material.
#[derive(serde::Deserialize)]
pub struct AdminMaterialReviewPayload {
    /// The action to take: "approve" or "reject".
    pub action: String,
    /// Optional review note / reason for rejection.
    pub note: Option<String>,
}

/// POST /api/admin/rewards/affiliates/materials/:id/review
/// Approves or rejects a submitted affiliate marketing material.
pub async fn api_admin_affiliate_material_review(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(material_id): axum::extract::Path<String>,
    Json(payload): Json<AdminMaterialReviewPayload>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let mid = ApiError::parse_uuid(&material_id)?;

    let new_status = match payload.action.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        other => {
            return Err(ApiError::BadRequest(format!(
                "Invalid action '{}'. Use 'approve' or 'reject'.",
                other
            )))
        }
    };

    // Non-macro: affiliate_materials table added in migration 076
    let result = sqlx::query(
        r#"UPDATE affiliate_materials
           SET status = $1, review_note = $2, reviewed_by = $3, reviewed_at = NOW(), updated_at = NOW()
           WHERE id = $4 AND status = 'pending_review'"#
    )
    .bind(new_status)
    .bind(payload.note.as_deref())
    .bind(admin.user.id)
    .bind(mid)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("Failed to review material {}: {}", material_id, e);
        ApiError::Internal("Database error".into())
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "Material not found or already reviewed".into(),
        ));
    }

    // Notify affiliate (non-macro fetch)
    let mat = sqlx::query(
        "SELECT am.asset_name, am.affiliate_id FROM affiliate_materials am WHERE am.id = $1",
    )
    .bind(mid)
    .fetch_optional(&state.db)
    .await
    .unwrap_or_default();

    if let Some(m) = mat {
        use sqlx::Row;
        let affiliate_id_val: uuid::Uuid = m.try_get("affiliate_id").unwrap_or(uuid::Uuid::nil());
        let asset_name_val: String = m.try_get("asset_name").unwrap_or_default();
        // Branded approval/rejection via the durable outbox.
        let (event_type, metadata) = if new_status == "approved" {
            (
                "affiliate_material_approved",
                serde_json::json!({ "material_name": asset_name_val }),
            )
        } else {
            (
                "affiliate_material_rejected",
                serde_json::json!({
                    "material_name": asset_name_val,
                    "reason": payload.note.as_deref().unwrap_or("No reason provided"),
                }),
            )
        };
        let _ = crate::email::trigger_transactional_email(
            &state.db,
            &affiliate_id_val,
            event_type,
            metadata,
        )
        .await;
    }

    Ok(Json(serde_json::json!({"success": true, "new_status": new_status})).into_response())
}

/// GET /api/admin/rewards/affiliates/batches/:batch_id/sepa.xml
///
/// Phase-3 fresh: render the SEPA pain.001.001.03 XML for a payout batch.
/// Returns 400 if any affiliate in the batch has no full IBAN on file.
pub async fn api_admin_affiliate_sepa_export(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(batch_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    admin
        .require_permission(&state.db, "affiliates.manage")
        .await?;

    let batch_uuid = ApiError::parse_uuid(&batch_id)?;

    let xml = crate::rewards::service::generate_sepa_pain001_for_batch(&state.db, batch_uuid)
        .await
        .map_err(|e| match e {
            crate::error::AppError::BadRequest(m) => ApiError::BadRequest(m),
            other => ApiError::Internal(other.to_string()),
        })?;

    let fname = format!("poool-sepa-batch-{}.xml", batch_uuid);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", fname))
            .map_err(|e| ApiError::Internal(format!("bad filename header: {e}")))?,
    );
    Ok((axum::http::StatusCode::OK, headers, xml).into_response())
}
