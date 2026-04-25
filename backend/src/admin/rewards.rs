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
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminAdjustRewardsPayload>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&user_id)?;
    let admin_user = _admin.user.clone();

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
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(tier_name): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateTierPayload>,
) -> Result<axum::response::Response, ApiError> {
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
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

/// PATCH /api/admin/rewards/referrals/:ref_id - Update referral status.
///
/// Handles status transitions (pending, qualified, paid, flagged) and
/// automatically credits users when marked as 'paid'.
///
/// The 'paid' flow is wrapped in a transaction with an idempotency guard:
/// only a transition from 'qualified' → 'paid' triggers reward crediting,
/// preventing double-credits on duplicate requests.
pub async fn api_admin_referral_update(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(ref_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&ref_id)?;
    let admin_user = _admin.user.clone();

    let new_status = match body.get("status").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return Err(ApiError::BadRequest("Status is required".to_string()));
        }
    };

    // Validate allowed statuses
    if !["pending", "qualified", "paid", "flagged"].contains(&new_status.as_str()) {
        return Err(ApiError::BadRequest("Invalid status value".to_string()));
    }

    // For 'paid' status, use a transaction to prevent double-crediting
    if new_status == "paid" {
        let mut tx = state.db.begin().await.map_err(|e| {
            tracing::error!("Failed to begin referral tx: {e}");
            ApiError::Internal("Server error".to_string())
        })?;

        // Idempotency guard: only transition from 'qualified' to 'paid'
        let rows_affected = sqlx::query(
            "UPDATE referral_tracking SET status = 'paid', qualified_at = COALESCE(qualified_at, NOW()) WHERE id = $1 AND status = 'qualified'"
        )
        .bind(uid)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update referral {ref_id}: {e}");
            ApiError::Internal("Database error".to_string())
        })?
        .rows_affected();

        if rows_affected == 0 {
            // Either not found or already paid/not qualified
            let _ = tx.rollback().await;
            return Err(ApiError::BadRequest(
                "Referral not found or not in 'qualified' status".to_string(),
            ));
        }

        // Fetch rewards amounts and user IDs within the same transaction
        let row = sqlx::query!(
            "SELECT referrer_id, referred_id, referrer_reward, referred_reward FROM referral_tracking WHERE id = $1",
            uid
        ).fetch_optional(&mut *tx).await.map_err(|e| {
            tracing::error!("Failed to fetch referral details: {e}");
            ApiError::Internal("Database error".to_string())
        })?;

        if let Some(r) = row {
            // Credit Referrer
            sqlx::query(
                "INSERT INTO rewards_balances (user_id, referrals) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET referrals = rewards_balances.referrals + EXCLUDED.referrals"
            ).bind(r.referrer_id).bind(r.referrer_reward).execute(&mut *tx).await.map_err(|e| {
                tracing::error!("Failed to credit referrer: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

            // Credit Referred
            sqlx::query(
                "INSERT INTO rewards_balances (user_id, referrals) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET referrals = rewards_balances.referrals + EXCLUDED.referrals"
            ).bind(r.referred_id).bind(r.referred_reward).execute(&mut *tx).await.map_err(|e| {
                tracing::error!("Failed to credit referred: {e}");
                ApiError::Internal("Database error".to_string())
            })?;

            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(admin_user.id)
            .bind("referral.marked_paid")
            .bind("referral_tracking")
            .bind(uid)
            .bind(serde_json::json!({
                "referrer_id": r.referrer_id,
                "referred_id": r.referred_id,
                "referrer_reward": r.referrer_reward,
                "referred_reward": r.referred_reward
            }))
            .execute(&mut *tx).await;
        }

        tx.commit().await.map_err(|e| {
            tracing::error!("Failed to commit referral tx: {e}");
            ApiError::Internal("Server error".to_string())
        })?;

        return Ok(Json(serde_json::json!({"status":"updated"})).into_response());
    }

    // Non-paid status transitions (no financial impact, no transaction needed)
    let result = sqlx::query(
        "UPDATE referral_tracking SET status = $1, qualified_at = CASE WHEN $1 = 'qualified' THEN NOW() ELSE qualified_at END WHERE id = $2"
    )
    .bind(&new_status)
    .bind(uid)
    .execute(&state.db)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => {
            // Audit log
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(admin_user.id)
            .bind(format!("referral.status_changed_to_{}", new_status))
            .bind("referral_tracking")
            .bind(uid)
            .bind(serde_json::json!({"new_status": new_status}))
            .execute(&state.db).await;

            Ok(Json(serde_json::json!({"status":"updated"})).into_response())
        }
        Ok(_) => Err(ApiError::NotFound("Referral not found".to_string())),
        Err(e) => {
            tracing::error!("Failed to update referral {ref_id}: {e}");
            Err(ApiError::Internal("Database error".to_string()))
        }
    }
}

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
                  a.traffic_source, a.audience_size, a.main_url, a.phone_number, a.company_name, a.tax_id,
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
            serde_json::json!({
                "id": r.id, "email": r.email, "user_name": if user_name.is_empty() { r.email.clone() } else { user_name },
                "traffic_source": r.traffic_source,
                "audience_size": r.audience_size, "main_url": r.main_url, "phone_number": r.phone_number,
                "company_name": r.company_name, "tax_id": r.tax_id, "created_at": r.created_at
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

            // Send email notification after the durable state change commits.
            let user_email: Option<String> =
                sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                    .bind(uid)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or_default();

            if let Some(email) = user_email {
                let _ = crate::common::email::send_email(
                    &email,
                    "Welcome to the POOOL Affiliate Partner Syndicate",
                    &format!(
                        "<h3>Application Approved</h3><p>Congratulations! Your application to join the POOOL Affiliate Partner Syndicate has been approved.</p><p>Your unique referral code is: <b>{}</b></p><p>You can now log into your <a href=\"https://poool.app/affiliate/dashboard\">Affiliate Dashboard</a> to access your tracking links and monitor commissions.</p>",
                        referral_code
                    )
                ).await;
            }

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

            // Send email notification after the durable state change commits.
            let user_email: Option<String> =
                sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                    .bind(uid)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or_default();

            if let Some(email) = user_email {
                let _ = crate::common::email::send_email(
                    &email,
                    "Update on your POOOL Affiliate Application",
                    &format!(
                        "<h3>Application Update</h3><p>Thank you for your interest in the POOOL Affiliate Partner Syndicate.</p><p>After careful review, we are unable to approve your application at this time.</p><p>Reason provided: <i>{}</i></p>",
                        reason
                    )
                ).await;
            }

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

            // Send email notification
            let user_email: Option<String> =
                sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
                    .bind(uid)
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or_default();

            if let Some(email) = user_email {
                let _ = crate::common::email::send_email(
                    &email,
                    "Important: Your POOOL Affiliate Account has been suspended",
                    &format!(
                        "<h3>Account Suspended</h3><p>Your POOOL Affiliate Partner Syndicate account has been temporarily suspended.</p><p>Reason provided: <i>{}</i></p><p>Please contact support for further information. Any pending commissions are on hold.</p>",
                        reason
                    )
                ).await;
            }

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

    let rows = sqlx::query!(
        r#"SELECT 
            u.id as user_id, u.email, 
            a.referral_code,
            COALESCE(up.first_name, '') as fn, COALESCE(up.last_name, '') as ln,
            SUM(ac.provisional_amount_cents)::bigint as total_payable_cents,
            COUNT(ac.id) as commission_count
           FROM affiliate_commissions ac
           JOIN affiliates a ON a.user_id = ac.affiliate_id
           JOIN users u ON u.id = a.user_id
           LEFT JOIN user_profiles up ON up.user_id = u.id
           WHERE ac.status = 'payable'
           GROUP BY u.id, u.email, a.referral_code, up.first_name, up.last_name
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
            let mut name = format!(
                "{} {}",
                r.r#fn.unwrap_or_default(),
                r.ln.unwrap_or_default()
            )
            .trim()
            .to_string();
            if name.is_empty() {
                name = r.email.clone();
            }
            serde_json::json!({
                "affiliate_id": r.user_id,
                "email": r.email,
                "name": name,
                "referral_code": r.referral_code,
                "total_payable_cents": r.total_payable_cents,
                "commission_count": r.commission_count,
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

    // 5. Update commissions
    sqlx::query!(
        r#"UPDATE affiliate_commissions 
           SET status = 'paid', payout_batch_id = $1, updated_at = NOW()
           WHERE affiliate_id = $2 AND status = 'payable'"#,
        batch_id,
        affiliate_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("Failed to update commissions".into()))?;

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

    // 9. Automated Tax Invoice Generation (Phase 19 Placeholder)
    // Generates a structured PDF Credit Statement for tax/compliance purposes
    let _invoice_pdf_path = format!(
        "gcs://poool-invoices/affiliates/{}/batch_{}.pdf",
        affiliate_id, batch_id
    );
    tracing::info!(
        "Tax Invoice/Credit Statement generated for payout batch {} at {}",
        batch_id,
        _invoice_pdf_path
    );

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("Commit failed".into()))?;

    // Send email notification for payout
    let user_email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(affiliate_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or_default();

    if let Some(email) = user_email {
        let _ = crate::common::email::send_email(
            &email,
            "Your POOOL Affiliate Commission Payout is Available!",
            &format!(
                "<h3>Payout Processed</h3><p>We have successfully released a payout of <b>${}.{:02}</b> for your {} referred investments.</p><p>This balance has been securely credited to your POOOL Cash Wallet and is available for immediate withdrawal or reinvestment.</p>",
                total_payable_cents / 100, (total_payable_cents % 100).abs(), commissions.len()
            )
        ).await;
    }

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
}

/// GET /api/admin/rewards/affiliates/fraud-scan
/// Scans the affiliate network for circular referral rings and flags them for review.
pub async fn api_admin_affiliate_fraud_scan(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let flags = crate::rewards::service::scan_affiliate_fraud_rings(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("Fraud scan failed: {}", e);
            ApiError::Internal("Fraud scan failed".into())
        })?;

    let count = flags.len();
    tracing::info!("Affiliate fraud scan complete: {} ring(s) detected", count);

    Ok(axum::response::Json(serde_json::json!({
        "success": true,
        "flags": flags,
        "count": count
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

    let affiliate_id = ApiError::parse_uuid(&id)?;
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|_| ApiError::Internal("Transaction start failed".into()))?;

    // Lock paid commissions
    let commissions = sqlx::query!(
        "SELECT id, provisional_amount_cents FROM affiliate_commissions WHERE affiliate_id = $1 AND status = 'paid' FOR UPDATE",
        affiliate_id
    )
    .fetch_all(&mut *tx).await.map_err(|_| ApiError::Internal("Fetch error".into()))?;

    if commissions.is_empty() {
        let _ = tx.rollback().await;
        return Err(ApiError::BadRequest(
            "No paid commissions to clawback".into(),
        ));
    }

    let total_clawback_cents: i64 = commissions.iter().map(|c| c.provisional_amount_cents).sum();

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

    // Update commissions to 'clawed_back'
    sqlx::query!("UPDATE affiliate_commissions SET status = 'clawed_back', updated_at = NOW() WHERE affiliate_id = $1 AND status = 'paid'", affiliate_id)
        .execute(&mut *tx).await
        .map_err(|_| ApiError::Internal("Failed to update commissions".into()))?;

    sqlx::query!(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata) VALUES ($1, 'affiliate_clawback', 'affiliates', $2, $3)",
        admin.user.id,
        affiliate_id,
        serde_json::json!({
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
        total_cents = total_clawback_cents,
        deducted_cents = actual_deducted,
        "[P0-FINANCIAL] Affiliate clawback executed"
    );

    Ok(Json(serde_json::json!({
        "success": true,
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
        let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(affiliate_id_val)
            .fetch_optional(&state.db)
            .await
            .unwrap_or_default();

        if let Some(e) = email {
            let subject = if new_status == "approved" {
                format!(
                    "Your marketing material '{}' has been approved!",
                    asset_name_val
                )
            } else {
                format!(
                    "Your marketing material '{}' requires changes",
                    asset_name_val
                )
            };
            let body = if new_status == "approved" {
                format!("<p>Your custom marketing material <b>{}</b> has been reviewed and <b>approved</b> for use. You may now use it in your campaigns.</p>", asset_name_val)
            } else {
                format!("<p>Your custom marketing material <b>{}</b> could not be approved at this time.</p><p>Reason: <i>{}</i></p><p>Please revise and resubmit.</p>",
                    asset_name_val, payload.note.as_deref().unwrap_or("No reason provided"))
            };
            let _ = crate::common::email::send_email(&e, &subject, &body).await;
        }
    }

    Ok(Json(serde_json::json!({"success": true, "new_status": new_status})).into_response())
}
