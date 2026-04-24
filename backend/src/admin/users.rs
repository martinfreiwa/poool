use super::extractors::{AdminUser, ApiError};
use crate::auth::routes::AppState;
use crate::common::sanitize;
use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

/// GET /api/admin/users - List all users with roles, KYC, and balances.
pub async fn api_admin_users(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    // Audit every PII read — admin listing exposes emails, names, balances
    // and KYC state. We want after-the-fact forensics on who looked at
    // what, even without a mutation.
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, 'admin.pii_access', 'users', $2)"#,
    )
    .bind(admin.user.id)
    .bind(serde_json::json!({"scope": "list", "endpoint": "GET /api/admin/users"}))
    .execute(&state.db)
    .await;
    // Verify the user has admin privileges
    // Fetch users with profiles, roles, KYC, and balance in a single query
    // to avoid N+1 problem (previously 3 extra queries per user)
    let rows = sqlx::query_as::<
        _,
        (
            String,         // id
            String,         // email
            String,         // status
            Option<String>, // first_name
            Option<String>, // last_name
            String,         // created_at
            Option<String>, // roles (comma-separated via string_agg)
            Option<String>, // kyc_status
            i64,            // balance_cents
        ),
    >(
        r#"
        SELECT
            u.id::text,
            u.email,
            u.status,
            p.first_name,
            p.last_name,
            u.created_at::text,
            (
                SELECT string_agg(r.name, ',')
                FROM user_roles ur
                JOIN roles r ON r.id = ur.role_id
                WHERE ur.user_id = u.id
            ) AS roles,
            (
                SELECT kr.status
                FROM kyc_records kr
                WHERE kr.user_id = u.id
                ORDER BY kr.created_at DESC
                LIMIT 1
            ) AS kyc_status,
            COALESCE((
                SELECT SUM(w.balance_cents)
                FROM wallets w
                WHERE w.user_id = u.id AND w.wallet_type = 'cash'
            ), 0)::bigint AS balance_cents
        FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        ORDER BY u.created_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&state.db)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to fetch admin users: {e}");
            return Err(ApiError::Internal("Failed to fetch users".to_string()));
        }
    };

    let users_json: Vec<serde_json::Value> = rows
        .iter()
        .map(
            |(
                id,
                email,
                status,
                first_name,
                last_name,
                created_at,
                roles_str,
                kyc_status,
                balance_cents,
            )| {
                let roles: Vec<&str> = roles_str
                    .as_deref()
                    .map(|s| s.split(',').collect())
                    .unwrap_or_default();

                serde_json::json!({
                    "id": id,
                    "email": email,
                    "status": status,
                    "first_name": first_name,
                    "last_name": last_name,
                    "roles": roles,
                    "kyc_status": kyc_status,
                    "balance_cents": balance_cents,
                    "created_at": created_at
                })
            },
        )
        .collect();

    Ok(Json(users_json).into_response())
}

/// GET /api/admin/users/:user_id - Full user detail with all related data.
pub async fn api_admin_user_detail(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&user_id)?;

    // Audit the read — user detail returns PII (profile, DOB, address,
    // tax_id, payment methods, KYC records, transactions).
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.pii_access', 'users', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(uid)
    .bind(serde_json::json!({"scope": "detail", "endpoint": "GET /api/admin/users/:id"}))
    .execute(&state.db)
    .await;

    //  Core user + profile
    let user_row = sqlx::query_as::<
        _,
        (
            String,         // id
            String,         // email
            bool,           // email_verified
            Option<String>, // avatar_url
            String,         // status
            String,         // created_at
            String,         // updated_at
        ),
    >(
        r#"SELECT id::text, email, email_verified, avatar_url, status,
                  created_at::text, updated_at::text
           FROM users WHERE id = $1"#,
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await;

    let user = match user_row {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Err(ApiError::NotFound("User not found".to_string()));
        }
        Err(e) => {
            tracing::error!("Failed to fetch user {user_id}: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    };

    //  Profile
    let profile = sqlx::query_as::<
        _,
        (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        r#"SELECT first_name, last_name, display_name,
                  date_of_birth::text, nationality,
                  address_line_1, address_line_2, city,
                  state_province, postal_code, country
           FROM user_profiles WHERE user_id = $1"#,
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let profile_json = profile.as_ref().map(|p| {
        serde_json::json!({
            "first_name": p.0, "last_name": p.1, "display_name": p.2,
            "date_of_birth": p.3, "nationality": p.4,
            "address_line_1": p.5, "address_line_2": p.6,
            "city": p.7, "state_province": p.8,
            "postal_code": p.9, "country": p.10
        })
    });

    //  Roles
    let roles: Vec<String> = sqlx::query_scalar(
        r#"SELECT r.name FROM user_roles ur
           JOIN roles r ON r.id = ur.role_id
           WHERE ur.user_id = $1"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    //  KYC status (latest)
    let kyc_status: Option<String> = sqlx::query_scalar(
        r#"SELECT status FROM kyc_records WHERE user_id = $1
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    //  Wallets
    let wallets = sqlx::query_as::<_, (String, String, i64)>(
        r#"SELECT id::text, wallet_type, balance_cents
           FROM wallets WHERE user_id = $1"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let cash_balance: i64 = wallets.iter().filter(|w| w.1 == "cash").map(|w| w.2).sum();
    let rewards_balance: i64 = wallets
        .iter()
        .filter(|w| w.1 == "rewards")
        .map(|w| w.2)
        .sum();

    let wallets_json: Vec<serde_json::Value> = wallets.iter().map(|w| {
        serde_json::json!({"id": w.0, "wallet_type": w.1, "balance_cents": w.2, "currency": "USD"})
    }).collect();

    //  Wallet IDs for transaction query
    let wallet_ids: Vec<sqlx::types::Uuid> =
        sqlx::query_scalar("SELECT id FROM wallets WHERE user_id = $1")
            .bind(uid)
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    //  Transactions (last 100)
    let transactions_json: Vec<serde_json::Value> = if !wallet_ids.is_empty() {
        let txs = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                i64,
                Option<String>,
                Option<String>,
                String,
            ),
        >(
            r#"SELECT id::text, type, status, amount_cents, description,
                      external_ref_id, created_at::text
               FROM wallet_transactions
               WHERE wallet_id = ANY($1)
               ORDER BY created_at DESC LIMIT 100"#,
        )
        .bind(&wallet_ids)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        txs.iter()
            .map(|t| {
                serde_json::json!({
                    "id": t.0, "type": t.1, "status": t.2, "amount_cents": t.3,
                    "description": t.4, "external_ref_id": t.5, "created_at": t.6
                })
            })
            .collect()
    } else {
        vec![]
    };

    //  KYC Records
    let kyc_records = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<bool>,
            Option<bool>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ),
    >(
        r#"SELECT id::text, provider, status, provider_ref_id,
                  document_type, pep_check_passed, sanctions_check,
                  rejection_reason, verified_at::text, expires_at::text,
                  created_at::text
           FROM kyc_records WHERE user_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let kyc_json: Vec<serde_json::Value> = kyc_records
        .iter()
        .map(|k| {
            serde_json::json!({
                "id": k.0, "provider": k.1, "status": k.2, "provider_ref_id": k.3,
                "document_type": k.4, "pep_check_passed": k.5, "sanctions_check": k.6,
                "rejection_reason": k.7, "verified_at": k.8, "expires_at": k.9, "created_at": k.10
            })
        })
        .collect();

    //  Investments
    let investments = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            i32,
            i64,
            i64,
            i64,
            String,
            Option<String>,
            String,
        ),
    >(
        r#"SELECT i.id::text, i.asset_id::text, COALESCE(a.title, 'Unknown'),
                  i.tokens_owned, i.purchase_value_cents, i.current_value_cents,
                  i.total_rental_cents, i.status, i.payout_expected_at::text,
                  i.purchased_at::text
           FROM investments i
           LEFT JOIN assets a ON a.id = i.asset_id
           WHERE i.user_id = $1
           ORDER BY i.purchased_at DESC"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let investments_json: Vec<serde_json::Value> = investments
        .iter()
        .map(|inv| {
            serde_json::json!({
                "id": inv.0, "asset_id": inv.1, "asset_title": inv.2,
                "tokens_owned": inv.3, "purchase_value_cents": inv.4,
                "current_value_cents": inv.5, "total_rental_cents": inv.6,
                "status": inv.7, "payout_expected_at": inv.8, "purchased_at": inv.9
            })
        })
        .collect();

    //  Orders
    let orders = sqlx::query_as::<
        _,
        (
            String,
            String,
            i64,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
        ),
    >(
        r#"SELECT id::text, order_number, total_cents, status,
                  payment_method, payment_ref_id,
                  created_at::text, completed_at::text
           FROM orders WHERE user_id = $1
           ORDER BY created_at DESC LIMIT 50"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let orders_json: Vec<serde_json::Value> = orders
        .iter()
        .map(|o| {
            serde_json::json!({
                "id": o.0, "order_number": o.1, "total_cents": o.2, "status": o.3,
                "payment_method": o.4, "payment_ref_id": o.5,
                "created_at": o.6, "completed_at": o.7
            })
        })
        .collect();

    //  Sessions
    let sessions =
        sqlx::query_as::<_, (String, Option<String>, Option<String>, bool, String, String)>(
            r#"SELECT id::text, host(ip_address)::text, user_agent,
                  remember_me, created_at::text, expires_at::text
           FROM user_sessions WHERE user_id = $1
           AND expires_at > NOW()
           ORDER BY created_at DESC"#,
        )
        .bind(uid)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let sessions_json: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.0, "ip_address": s.1, "user_agent": s.2,
                "remember_me": s.3, "created_at": s.4, "expires_at": s.5
            })
        })
        .collect();

    //  OAuth Accounts
    let oauth = sqlx::query_as::<_, (String, String, Option<String>, String)>(
        r#"SELECT id::text, provider, provider_email, created_at::text
           FROM oauth_accounts WHERE user_id = $1"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let oauth_json: Vec<serde_json::Value> = oauth
        .iter()
        .map(|o| {
            serde_json::json!({
                "id": o.0, "provider": o.1, "provider_email": o.2, "created_at": o.3
            })
        })
        .collect();

    //  Settings
    let settings = sqlx::query_as::<_, (bool, Option<String>)>(
        r#"SELECT totp_enabled, language FROM user_settings WHERE user_id = $1"#,
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let settings_json = settings.map(|s| {
        serde_json::json!({
            "totp_enabled": s.0, "language": s.1
        })
    });

    //  Payment Methods
    let payment_methods = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            bool,
            String,
            String,
        ),
    >(
        r#"SELECT id::text, method_type, brand, last_four, holder_name,
                  is_default, status, created_at::text
           FROM payment_methods WHERE user_id = $1
           ORDER BY is_default DESC, created_at DESC"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let payment_methods_json: Vec<serde_json::Value> = payment_methods
        .iter()
        .map(|pm| {
            serde_json::json!({
                "id": pm.0, "method_type": pm.1, "brand": pm.2, "last_four": pm.3,
                "holder_name": pm.4, "is_default": pm.5, "status": pm.6, "created_at": pm.7
            })
        })
        .collect();

    //  Audit Logs (last 50)
    let audit_logs = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            String,
        ),
    >(
        r#"SELECT id, action, entity_type, entity_id::text,
                  host(ip_address)::text,
                  previous_state, new_state, created_at::text
           FROM audit_logs
           WHERE actor_user_id = $1 OR entity_id = $1
           ORDER BY created_at DESC LIMIT 50"#,
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let audit_json: Vec<serde_json::Value> = audit_logs
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.0, "action": a.1, "entity_type": a.2, "entity_id": a.3,
                "ip_address": a.4, "previous_state": a.5, "new_state": a.6, "created_at": a.7
            })
        })
        .collect();

    //  Compose full response
    Ok(Json(serde_json::json!({
        "id": user.0,
        "email": user.1,
        "email_verified": user.2,
        "avatar_url": user.3,
        "status": user.4,
        "created_at": user.5,
        "updated_at": user.6,
        "first_name": profile.as_ref().and_then(|p| p.0.clone()),
        "last_name": profile.as_ref().and_then(|p| p.1.clone()),
        "roles": roles,
        "kyc_status": kyc_status,
        "cash_balance_cents": cash_balance,
        "rewards_balance_cents": rewards_balance,
        "profile": profile_json,
        "settings": settings_json,
        "wallets": wallets_json,
        "transactions": transactions_json,
        "kyc_records": kyc_json,
        "investments": investments_json,
        "orders": orders_json,
        "sessions": sessions_json,
        "oauth_accounts": oauth_json,
        "payment_methods": payment_methods_json,
        "audit_logs": audit_json
    }))
    .into_response())
}

//
//  Admin User Updates API
//

/// Payload for updating a user's profile from the admin dashboard.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
pub struct AdminUpdateProfilePayload {
    /// User's first name.
    pub first_name: Option<String>,
    /// User's last name.
    pub last_name: Option<String>,
    /// User's date of birth (ISO 8601).
    pub date_of_birth: Option<String>,
    /// User's nationality.
    pub nationality: Option<String>,
    /// User's phone number.
    pub phone_number: Option<String>,
    /// First line of user's address.
    pub address_line_1: Option<String>,
    /// Second line of user's address.
    pub address_line_2: Option<String>,
    /// User's city.
    pub city: Option<String>,
    /// User's state or province.
    pub state_province: Option<String>,
    /// User's postal or ZIP code.
    pub postal_code: Option<String>,
    /// User's country.
    pub country: Option<String>,
    /// User's tax identification number.
    pub tax_id: Option<String>,
    /// Manual override for the user's loyalty tier.
    pub tier: Option<String>,
}

/// POST /api/admin/users/:user_id/profile - Update a user's profile data.
///
/// Updates PII data and can optionally override the user's loyalty tier.
pub async fn api_admin_user_update_profile(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateProfilePayload>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = _admin.user.id;

    let uid = ApiError::parse_uuid(&user_id)?;

    let mut payload = payload;
    if let Some(ref v) = payload.first_name {
        payload.first_name = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.last_name {
        payload.last_name = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.nationality {
        payload.nationality = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.phone_number {
        payload.phone_number = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.address_line_1 {
        payload.address_line_1 = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.address_line_2 {
        payload.address_line_2 = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.city {
        payload.city = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.state_province {
        payload.state_province = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.postal_code {
        payload.postal_code = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.country {
        payload.country = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.tax_id {
        payload.tax_id = Some(sanitize::sanitize_text(v));
    }
    if let Some(ref v) = payload.tier {
        payload.tier = Some(sanitize::sanitize_text(v));
    }

    let parsed_dob = if let Some(d) = &payload.date_of_birth {
        if d.trim().is_empty() {
            None
        } else {
            match d.parse::<chrono::NaiveDate>() {
                Ok(date) => Some(date),
                Err(_) => {
                    return Err(ApiError::BadRequest(
                        "Invalid Date of Birth format. Expected YYYY-MM-DD".to_string(),
                    ))
                }
            }
        }
    } else {
        None
    };

    // 1. Update Profile Information
    let profile_result = sqlx::query(
        r#"INSERT INTO user_profiles (user_id, first_name, last_name, date_of_birth, nationality, phone_number, address_line_1, address_line_2, city, state_province, postal_code, country, tax_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
           ON CONFLICT (user_id) DO UPDATE SET
           first_name = EXCLUDED.first_name,
           last_name = EXCLUDED.last_name,
           date_of_birth = EXCLUDED.date_of_birth,
           nationality = EXCLUDED.nationality,
           phone_number = EXCLUDED.phone_number,
           address_line_1 = EXCLUDED.address_line_1,
           address_line_2 = EXCLUDED.address_line_2,
           city = EXCLUDED.city,
           state_province = EXCLUDED.state_province,
           postal_code = EXCLUDED.postal_code,
           country = EXCLUDED.country,
           tax_id = EXCLUDED.tax_id"#
    )
    .bind(uid)
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(parsed_dob)
    .bind(&payload.nationality)
    .bind(&payload.phone_number)
    .bind(&payload.address_line_1)
    .bind(&payload.address_line_2)
    .bind(&payload.city)
    .bind(&payload.state_province)
    .bind(&payload.postal_code)
    .bind(&payload.country)
    .bind(&payload.tax_id)
    .execute(&state.db)
    .await;

    if let Err(e) = profile_result {
        tracing::error!("Failed to update user profile: {}", e);
        return Err(ApiError::Internal(format!(
            "Failed to update profile: {}",
            e
        )));
    }

    // 2. Handle Tier Override if provided
    if let Some(tier_name) = &payload.tier {
        let tier_id: Option<i32> = sqlx::query_scalar("SELECT id FROM tiers WHERE name = $1")
            .bind(tier_name)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

        if let Some(tid) = tier_id {
            let _ = sqlx::query(
                r#"INSERT INTO user_tiers (user_id, tier_id, invested_12m)
                   VALUES ($1, $2, 0)
                   ON CONFLICT (user_id) DO UPDATE SET
                   tier_id = EXCLUDED.tier_id,
                   updated_at = NOW()"#,
            )
            .bind(uid)
            .bind(tid)
            .execute(&state.db)
            .await;

            // Log the tier change in audit_logs
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
                   VALUES ($1, 'user.tier_override', 'users', $2, $3)"#,
            )
            .bind(admin_user)
            .bind(uid)
            .bind(serde_json::json!({"new_tier": tier_name}))
            .execute(&state.db)
            .await;
        }
    }

    // 3. Log profile update in audit_logs
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.profile_update', 'users', $2, $3)"#,
    )
    .bind(admin_user)
    .bind(uid)
    .bind(serde_json::json!({
        "first_name": payload.first_name,
        "last_name": payload.last_name,
        "nationality": payload.nationality,
        "country": payload.country,
        "city": payload.city,
        "phone_number": payload.phone_number,
    }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({"success": true})).into_response())
}

/// Payload for adjusting a user's wallet balance from the admin dashboard.
#[derive(serde::Deserialize)]
pub struct AdminUpdateBalancePayload {
    /// Type of wallet (e.g., 'cash', 'rewards', 'cashback').
    pub wallet_type: String,
    /// Amount to adjust (in cents). Positive for credits, negative for debits.
    pub amount_cents: i64,
    /// Reason for the adjustment (required for audit logs).
    pub reason: String,
}

/// POST /api/admin/users/:user_id/balance - Adjust a user's wallet balance.
pub async fn api_admin_user_update_balance(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateBalancePayload>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = _admin.user.id;

    let uid = ApiError::parse_uuid(&user_id)?;

    if payload.amount_cents == 0 {
        return Err(ApiError::BadRequest("Amount must not be zero".to_string()));
    }

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(_) => {
            return Err(ApiError::Internal("Failed to start tx".to_string()));
        }
    };

    // If it's a reward category, we treat it as an update to the 'rewards' wallet
    // BUT we also update the rewards_balances categorization table.
    let (target_wallet_type, reward_col) = match payload.wallet_type.as_str() {
        "cashback" => ("rewards", Some("cashback")),
        "referrals" => ("rewards", Some("referrals")),
        "promotions" => ("rewards", Some("promotions")),
        _ => (payload.wallet_type.as_str(), None),
    };

    // Get or create wallet
    let wallet_row: Option<(uuid::Uuid, i64)> = match sqlx::query_as(
        r#"SELECT id, balance_cents FROM wallets WHERE user_id = $1 AND wallet_type = $2 FOR UPDATE"#
    )
    .bind(uid)
    .bind(target_wallet_type)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => Some(row),
        Ok(None) => None,
        Err(_) => return Err(ApiError::Internal("Failed to query wallet".to_string())),
    };

    let (wallet_id, current_balance) = if let Some((id, bal)) = wallet_row {
        (id, bal)
    } else {
        match sqlx::query_scalar(
            r#"INSERT INTO wallets (user_id, wallet_type, currency) VALUES ($1, $2, 'USD') RETURNING id"#
        )
        .bind(uid)
        .bind(target_wallet_type)
        .fetch_one(&mut *tx).await {
            Ok(id) => (id, 0_i64),
            Err(_) => return Err(ApiError::Internal("Failed to create wallet".to_string())),
        }
    };

    if payload.amount_cents < 0 && current_balance + payload.amount_cents < 0 {
        return Err(ApiError::BadRequest(format!(
            "Insufficient funds: trying to deduct {}, but wallet only has {}",
            -payload.amount_cents, current_balance
        )));
    }

    // Add transaction
    let tx_type = if payload.amount_cents >= 0 {
        "admin_credit"
    } else {
        "admin_debit"
    };
    let insert_tx = sqlx::query(
        r#"INSERT INTO wallet_transactions (wallet_id, type, amount_cents, status, description) VALUES ($1, $2, $3, 'completed', $4)"#
    )
    .bind(wallet_id)
    .bind(tx_type)
    .bind(payload.amount_cents)
    .bind(format!("Admin adjustment [{}]: {}", payload.wallet_type, payload.reason))
    .execute(&mut *tx).await;

    if insert_tx.is_err() {
        return Err(ApiError::Internal(
            "Failed to insert transaction".to_string(),
        ));
    }

    // Update wallet balance
    let update_wallet_res = sqlx::query(
        r#"UPDATE wallets SET balance_cents = balance_cents + $1, updated_at = NOW() WHERE id = $2 RETURNING balance_cents"#
    )
    .bind(payload.amount_cents)
    .bind(wallet_id)
    .fetch_one(&mut *tx).await;

    let new_bal: i64 = match update_wallet_res {
        Ok(r) => sqlx::Row::get(&r, "balance_cents"),
        Err(_) => {
            return Err(ApiError::Internal("Failed to update balance".to_string()));
        }
    };

    // If reward category, update rewards_balances
    if let Some(col) = reward_col {
        let sql = format!(
            "INSERT INTO rewards_balances (user_id, {}) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET {} = rewards_balances.{} + EXCLUDED.{}",
            col, col, col, col
        );
        let _ = sqlx::query(&sql)
            .bind(uid)
            .bind(payload.amount_cents)
            .execute(&mut *tx)
            .await;
    }

    // Log in audit log
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, 'admin.balance_update', 'users', $2, $3)"#
    )
    .bind(admin_user)
    .bind(uid)
    .bind(serde_json::json!({"user_id": uid, "wallet_id": wallet_id, "category": payload.wallet_type, "amount_cents": payload.amount_cents, "new_balance": new_bal, "reason": payload.reason}))
    .execute(&mut *tx).await;

    if tx.commit().await.is_err() {
        return Err(ApiError::Internal("Failed to commit tx".to_string()));
    }

    Ok(Json(serde_json::json!({"success": true, "new_balance_cents": new_bal})).into_response())
}

//
//  Admin Deposits API
//

/// Payload for updating a user's account status (active, suspended, etc.).
#[derive(serde::Deserialize)]
pub struct AdminUpdateStatusPayload {
    status: String,
}

/// POST /api/admin/users/:user_id/status - Update a user's account status.
pub async fn api_admin_user_update_status(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateStatusPayload>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = _admin.user.id;

    let uid = ApiError::parse_uuid(&user_id)?;

    if payload.status != "active" && payload.status != "suspended" && payload.status != "frozen" {
        return Err(ApiError::BadRequest(
            "Status must be 'active', 'suspended', or 'frozen'".to_string(),
        ));
    }

    let result = sqlx::query(r#"UPDATE users SET status = $1, updated_at = NOW() WHERE id = $2"#)
        .bind(&payload.status)
        .bind(uid)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"#
            )
            .bind(admin_user)
            .bind(String::from("admin.user_status_update"))
            .bind(String::from("users"))
            .bind(uid)
            .bind(serde_json::json!({"status": payload.status}))
            .execute(&state.db).await;
            Ok(Json(serde_json::json!({"success": true})).into_response())
        }
        Err(e) => {
            tracing::error!("Failed to update user status: {}", e);
            Err(ApiError::Internal(
                "Failed to update user status".to_string(),
            ))
        }
    }
}

/// Payload for updating a user's roles.
#[derive(serde::Deserialize)]
pub struct AdminUpdateRolesPayload {
    roles: Vec<String>,
}

/// POST /api/admin/users/:user_id/roles - Update a user's roles.
///
/// Gated: only super_admin can mutate roles. Self-modification is refused.
/// Every proposed role must be in the ASSIGNABLE_ROLES allowlist.
pub async fn api_admin_user_update_roles(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(payload): Json<AdminUpdateRolesPayload>,
) -> Result<axum::response::Response, ApiError> {
    let uid = ApiError::parse_uuid(&user_id)?;

    if !admin.is_super_admin(&state.db).await {
        return Err(ApiError::Forbidden(
            "Only super_admin may modify user roles".to_string(),
        ));
    }
    if admin.user.id == uid {
        return Err(ApiError::Forbidden(
            "Admins may not modify their own roles".to_string(),
        ));
    }
    for role_name in &payload.roles {
        let valid = crate::admin::extractors::ASSIGNABLE_ROLES.contains(&role_name.as_str())
            || crate::admin::extractors::ELEVATED_ROLES.contains(&role_name.as_str());
        if !valid {
            return Err(ApiError::BadRequest(format!(
                "Role '{}' is not assignable",
                role_name
            )));
        }
    }

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to start transaction: {e}");
            return Err(ApiError::Internal("Database error".to_string()));
        }
    };

    // 1. Remove all existing roles for this user
    let delete_res = sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(uid)
        .execute(&mut *tx)
        .await;

    if let Err(e) = delete_res {
        tracing::error!("Failed to delete roles for user {uid}: {e}");
        return Err(ApiError::Internal("Failed to update roles".to_string()));
    }

    // 2. Insert new roles
    for role_name in &payload.roles {
        // Find role ID by name
        let role_id: Option<sqlx::types::Uuid> =
            sqlx::query_scalar("SELECT id FROM roles WHERE name = $1")
                .bind(role_name)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);

        if let Some(rid) = role_id {
            let insert_res =
                sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2)")
                    .bind(uid)
                    .bind(rid)
                    .execute(&mut *tx)
                    .await;

            if let Err(e) = insert_res {
                tracing::error!("Failed to insert role {role_name} for user {uid}: {e}");
                return Err(ApiError::Internal("Failed to update roles".to_string()));
            }
        }
    }

    // 3. Log audit
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state)
           VALUES ($1, 'admin.roles_update', 'users', $2, $3)"#,
    )
    .bind(admin.user.id)
    .bind(uid)
    .bind(serde_json::json!({ "new_roles": payload.roles }))
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit roles update transaction: {e}");
        return Err(ApiError::Internal("Failed to save roles".to_string()));
    }

    Ok(Json(serde_json::json!({"success": true})).into_response())
}

/// DELETE /api/admin/users/:user_id/sessions - Revoke all active sessions for a user.
pub async fn api_admin_user_revoke_sessions(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let admin_user = _admin.user.id;

    let uid = ApiError::parse_uuid(&user_id)?;

    let result = sqlx::query(r#"DELETE FROM user_sessions WHERE user_id = $1"#)
        .bind(uid)
        .execute(&state.db)
        .await;

    match result {
        Ok(pg_res) => {
            let _ = sqlx::query(
                r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)"#
            )
            .bind(admin_user)
            .bind(String::from("admin.revoke_sessions"))
            .bind(String::from("user"))
            .bind(uid)
            .bind(serde_json::json!({"deleted_count": pg_res.rows_affected()}))
            .execute(&state.db).await;
            Ok(
                Json(serde_json::json!({"success": true, "deleted_count": pg_res.rows_affected()}))
                    .into_response(),
            )
        }
        Err(e) => {
            tracing::error!("Failed to revoke sessions: {}", e);
            Err(ApiError::Internal("Failed to revoke sessions".to_string()))
        }
    }
}

/// POST /api/admin/users/:id/force-password-reset
pub async fn api_admin_user_force_password_reset(
    _admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<axum::response::Response, ApiError> {
    let admin = _admin.user.clone();
    let uid = ApiError::parse_uuid(&user_id)?;

    // Verify user exists
    let exists: Option<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);
    if exists.is_none() {
        return Err(ApiError::NotFound("User not found".to_string()));
    }

    let mut tx = match state.db.begin().await {
        Ok(t) => t,
        Err(_) => {
            return Err(ApiError::Internal(
                "Failed to start transaction".to_string(),
            ));
        }
    };

    // 1. Set force_password_reset flag (upsert user_settings)
    let _ = sqlx::query(
        r#"INSERT INTO user_settings (user_id, force_password_reset)
           VALUES ($1, TRUE)
           ON CONFLICT (user_id) DO UPDATE SET force_password_reset = TRUE, updated_at = NOW()"#,
    )
    .bind(uid)
    .execute(&mut *tx)
    .await;

    // 2. Revoke all active sessions so user must log in again
    let _ = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
        .bind(uid)
        .execute(&mut *tx)
        .await;

    // 3. Audit log
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, new_state) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(admin.id)
    .bind("user.force_password_reset")
    .bind("users")
    .bind(uid)
    .bind(serde_json::json!({"force_password_reset": true, "sessions_revoked": true}))
    .execute(&mut *tx)
    .await;

    match tx.commit().await {
        Ok(_) => {
            Ok(Json(serde_json::json!({"success": true, "message": "User must change password on next login. All sessions revoked."}))
                .into_response())
        }
        Err(e) => {
            tracing::error!("Failed to force password reset: {}", e);
            Err(ApiError::Internal("Failed to update user".to_string()))
        }
    }
}
