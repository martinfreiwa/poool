/// Settings business logic – ISOLATED from HTTP handlers.
///
/// All settings-related DB operations live here, keeping the route
/// handlers thin and the logic independently testable.
///
/// SECURITY INVARIANTS:
/// - Email/password changes ALWAYS require current password verification
/// - Email changes reset email_verified to FALSE
/// - All sensitive changes are logged to audit_logs
/// - Input validation prevents injection and XSS
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use chrono::Datelike;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{
    DeveloperLinks, DeveloperProfileSettings, SettingsResponse, SocialLinks,
    UpdateDeveloperLinksForm, UpdateDeveloperProfileForm, UpdateLeaderboardForm,
    UpdateNotificationsForm, UpdatePreferencesForm, UpdateProfileForm, UpdateSocialLinksForm,
};
use crate::common::sanitize;
use crate::error::AppError;

// ─── Allowed values ────────────────────────────────────────────

const ALLOWED_LANGUAGES: &[&str] = &["en", "de", "fr", "es", "id", "zh"];
const ALLOWED_CURRENCIES: &[&str] = &["USD", "EUR", "GBP", "AUD", "SGD", "IDR", "JPY", "CHF"];

// ─── GET: Full settings ────────────────────────────────────────

/// Fetch all settings for a user. Joins users, user_profiles,
/// user_settings, roles, and kyc_records into a single response.
pub async fn get_settings(
    pool: &PgPool,
    user_id: Uuid,
    session_token: &str,
) -> Result<SettingsResponse, AppError> {
    let row = sqlx::query(
        r#"
        SELECT u.email,
               p.first_name,
               p.middle_name,
               p.last_name,
               p.gender,
               p.phone_number,
               p.country,
               COALESCE(s.timezone, 'UTC') as timezone,
               r.name as role_name,
               EXISTS (
                   SELECT 1 FROM user_roles ur2
                   JOIN roles r2 ON r2.id = ur2.role_id
                   WHERE ur2.user_id = u.id AND r2.name = 'developer'
               ) as is_developer,
               EXISTS (
                   SELECT 1 FROM user_roles ur3
                   JOIN roles r3 ON r3.id = ur3.role_id
                   WHERE ur3.user_id = u.id
                     AND r3.name IN ('admin', 'super_admin', 'compliance', 'finance')
               ) as is_admin,
               COALESCE(s.language, 'en') as language,
               COALESCE(s.currency, 'USD') as currency,
               u.avatar_url,
               u.email_verified,
               TO_CHAR(p.date_of_birth, 'YYYY-MM-DD') as date_of_birth,
               p.nationality,
               p.address_line_1,
               p.address_line_2,
               p.city,
               p.state_province,
               p.postal_code,
               p.tax_id,
               p.annual_income_cents,
               k.status as kyc_status,
               COALESCE(s.email_notifications, TRUE) as email_notifications,
               COALESCE(s.push_notifications, TRUE) as push_notifications,
               COALESCE(s.totp_enabled, FALSE) as totp_enabled,
               COALESCE(lb_p.visible, FALSE) as lb_visible,
               COALESCE(lb_p.show_avatar, FALSE) as lb_avatar,
               lb_p.display_name as lb_display_name,
               lb_p.bio as lb_bio,
               p.social_twitter_url,
               p.social_linkedin_url,
               p.social_instagram_url,
               p.social_telegram_url,
               p.social_discord,
               p.social_website_url,
               dp.company_name as dev_company_name,
               dp.logo_url as dev_logo_url,
               dp.description as dev_description,
               dp.website_url as dev_website_url,
               dp.github_url as dev_github_url,
               dp.x_url as dev_twitter_url,
               dp.linkedin_url as dev_linkedin_url,
               dp.youtube_url as dev_youtube_url
        FROM users u
        LEFT JOIN user_profiles p ON u.id = p.user_id
        LEFT JOIN user_settings s ON u.id = s.user_id
        LEFT JOIN user_roles ur ON u.id = ur.user_id
        LEFT JOIN roles r ON ur.role_id = r.id
        LEFT JOIN leaderboard_preferences lb_p ON u.id = lb_p.user_id
        LEFT JOIN developer_profiles dp ON u.id = dp.user_id
        LEFT JOIN LATERAL (
            SELECT status FROM kyc_records
            WHERE user_id = u.id
            ORDER BY created_at DESC LIMIT 1
        ) k ON TRUE
        WHERE u.id = $1 AND u.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    use chrono::Datelike;
    use sqlx::Row;

    let mut response = SettingsResponse {
        email: row.try_get("email").unwrap_or_default(),
        first_name: row.try_get("first_name").unwrap_or_default(),
        middle_name: row.try_get("middle_name").unwrap_or_default(),
        last_name: row.try_get("last_name").unwrap_or_default(),
        gender: row.try_get("gender").unwrap_or_default(),
        phone_number: row.try_get("phone_number").unwrap_or_default(),
        country: row.try_get("country").unwrap_or_default(),
        timezone: row
            .try_get("timezone")
            .unwrap_or_else(|_| "UTC".to_string()),
        role: row
            .try_get("role_name")
            .unwrap_or_else(|_| "investor".to_string()),
        is_developer: row.try_get("is_developer").unwrap_or(false),
        is_admin: row.try_get("is_admin").unwrap_or(false),
        language: row.try_get("language").unwrap_or_else(|_| "en".to_string()),
        currency: row
            .try_get("currency")
            .unwrap_or_else(|_| "USD".to_string()),
        annual_income_cents: row.try_get("annual_income_cents").ok(),
        avatar_url: row.try_get("avatar_url").unwrap_or_default(),
        email_verified: row.try_get("email_verified").unwrap_or(false),
        date_of_birth: row.try_get("date_of_birth").unwrap_or_default(),
        nationality: row.try_get("nationality").unwrap_or_default(),
        address_line_1: row.try_get("address_line_1").unwrap_or_default(),
        address_line_2: row.try_get("address_line_2").unwrap_or_default(),
        city: row.try_get("city").unwrap_or_default(),
        state_province: row.try_get("state_province").unwrap_or_default(),
        postal_code: row.try_get("postal_code").unwrap_or_default(),
        tax_id: row.try_get("tax_id").unwrap_or_default(),
        kyc_status: row.try_get("kyc_status").unwrap_or_default(),
        email_notifications: row.try_get("email_notifications").unwrap_or(true),
        push_notifications: row.try_get("push_notifications").unwrap_or(true),
        totp_enabled: row.try_get("totp_enabled").unwrap_or(false),
        referral_code: None,
        tier_name: None,
        investment_limit_cents: None,
        invested_12m_cents: None,
        limit_available_cents: None,
        active_sessions: vec![],
        oauth_accounts: vec![],
        latest_terms_version: None,
        latest_terms_accepted_at: None,
        lb_visible: row.try_get("lb_visible").unwrap_or(false),
        lb_avatar: row.try_get("lb_avatar").unwrap_or(false),
        lb_display_name: row.try_get("lb_display_name").ok(),
        lb_bio: row.try_get("lb_bio").ok(),
        social_links: SocialLinks {
            twitter: row.try_get("social_twitter_url").ok(),
            linkedin: row.try_get("social_linkedin_url").ok(),
            instagram: row.try_get("social_instagram_url").ok(),
            telegram: row.try_get("social_telegram_url").ok(),
            discord: row.try_get("social_discord").ok(),
            website: row.try_get("social_website_url").ok(),
        },
        developer_profile: None,
    };

    let dev_company_name: Option<String> = row.try_get("dev_company_name").ok();
    let dev_logo_url: Option<String> = row.try_get("dev_logo_url").ok();
    let dev_description: Option<String> = row.try_get("dev_description").ok();
    let dev_links = DeveloperLinks {
        website: row.try_get("dev_website_url").ok(),
        github: row.try_get("dev_github_url").ok(),
        twitter: row.try_get("dev_twitter_url").ok(),
        linkedin: row.try_get("dev_linkedin_url").ok(),
        youtube: row.try_get("dev_youtube_url").ok(),
    };
    if dev_company_name.is_some()
        || dev_logo_url.is_some()
        || dev_description.is_some()
        || dev_links.website.is_some()
        || dev_links.github.is_some()
        || dev_links.twitter.is_some()
        || dev_links.linkedin.is_some()
        || dev_links.youtube.is_some()
    {
        response.developer_profile = Some(DeveloperProfileSettings {
            company_name: dev_company_name,
            logo_url: dev_logo_url,
            description: dev_description,
            links: dev_links,
        });
    }

    // ─── Fetch Referrals & Tiers ──────────────────────────────

    let ref_row = sqlx::query("SELECT code FROM referral_codes WHERE user_id = $1 LIMIT 1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    if let Some(r) = ref_row {
        response.referral_code = r.try_get("code").ok();
    }

    let tier_row = sqlx::query(
        "SELECT t.name FROM user_tiers ut JOIN tiers t ON ut.tier_id = t.id WHERE ut.user_id = $1 LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    if let Some(r) = tier_row {
        response.tier_name = r.try_get("name").ok();
    }

    // ─── Fetch Investment Limits ──────────────────────────────

    let current_year = chrono::Utc::now().naive_utc().year();
    let limit_row = sqlx::query(
        "SELECT annual_limit_cents, invested_12m_cents, available_cents FROM investment_limits WHERE user_id = $1 AND limit_year = $2 LIMIT 1"
    )
    .bind(user_id)
    .bind(current_year)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = limit_row {
        response.investment_limit_cents = r.try_get("annual_limit_cents").ok();
        response.invested_12m_cents = r.try_get("invested_12m_cents").ok();
        response.limit_available_cents = r.try_get("available_cents").ok();
    }

    // ─── Fetch Active Sessions ────────────────────────────────

    let sessions = sqlx::query(
        r#"SELECT ip_address, user_agent, TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI') as created_at, TO_CHAR(expires_at, 'YYYY-MM-DD HH24:MI') as expires_at, (session_token = $2) as is_current
           FROM user_sessions WHERE user_id = $1 AND expires_at > NOW() ORDER BY created_at DESC"#
    )
    .bind(user_id)
    .bind(session_token)
    .fetch_all(pool)
    .await?;

    for s in sessions {
        response
            .active_sessions
            .push(super::models::UserSessionInfo {
                ip_address: s.try_get("ip_address").ok(),
                user_agent: s.try_get("user_agent").ok(),
                created_at: s.try_get("created_at").unwrap_or_default(),
                expires_at: s.try_get("expires_at").unwrap_or_default(),
                is_current: s.try_get("is_current").unwrap_or(false),
            });
    }

    // ─── Fetch OAuth Accounts ─────────────────────────────────

    let oauths = sqlx::query(
        r#"SELECT id::text as id, provider, provider_email, TO_CHAR(created_at, 'YYYY-MM-DD HH24:MI') as created_at
           FROM oauth_accounts WHERE user_id = $1 ORDER BY created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    for o in oauths {
        response
            .oauth_accounts
            .push(super::models::OauthAccountInfo {
                id: o.try_get("id").unwrap_or_default(),
                provider: o.try_get("provider").unwrap_or_default(),
                email: o.try_get("provider_email").ok(),
                created_at: o.try_get("created_at").unwrap_or_default(),
            });
    }

    // ─── Fetch Consents ───────────────────────────────────────

    let consent_row = sqlx::query(
        "SELECT terms_version, TO_CHAR(accepted_at, 'YYYY-MM-DD HH24:MI') as accepted_at FROM user_consents WHERE user_id = $1 ORDER BY accepted_at DESC LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = consent_row {
        response.latest_terms_version = r.try_get("terms_version").ok();
        response.latest_terms_accepted_at = r.try_get("accepted_at").ok();
    }

    Ok(response)
}

// ─── UPDATE: Profile (My Details tab) ──────────────────────────

/// Sanitize an optional string field using the common sanitizer.
fn sanitize_opt(opt: &Option<String>) -> Option<String> {
    opt.as_ref().map(|s| sanitize::sanitize_text(s))
}

fn sanitize_nullable_text(opt: &Option<String>) -> Option<String> {
    opt.as_ref()
        .map(|s| sanitize::sanitize_text(s))
        .filter(|s| !s.is_empty())
}

fn validate_short_text(
    field: &str,
    value: &Option<String>,
    max_len: usize,
) -> Result<(), AppError> {
    if let Some(value) = value {
        if value.len() > max_len {
            return Err(AppError::BadRequest(format!(
                "{} must be {} characters or less.",
                field, max_len
            )));
        }
    }
    Ok(())
}

fn validate_required_short_text(
    field: &str,
    value: &Option<String>,
    max_len: usize,
) -> Result<(), AppError> {
    if let Some(value) = value {
        if value.trim().is_empty() {
            return Err(AppError::BadRequest(format!("{} is required.", field)));
        }
    }
    validate_short_text(field, value, max_len)
}

fn validate_iso_code(field: &str, value: &Option<String>) -> Result<(), AppError> {
    if let Some(value) = value {
        let trimmed = value.trim();
        if !trimmed.is_empty()
            && (!(2..=3).contains(&trimmed.len())
                || !trimmed.chars().all(|c| c.is_ascii_uppercase()))
        {
            return Err(AppError::BadRequest(format!(
                "{} must be a 2 or 3-letter uppercase ISO code.",
                field
            )));
        }
    }
    Ok(())
}

fn validate_date_of_birth(value: &Option<String>) -> Result<(), AppError> {
    let Some(value) = value else {
        return Ok(());
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let dob = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Date of birth must use YYYY-MM-DD.".to_string()))?;
    let today = chrono::Utc::now().date_naive();
    let min_date = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
    if dob < min_date || dob >= today {
        return Err(AppError::BadRequest(
            "Date of birth must be a valid past date.".to_string(),
        ));
    }
    let latest_allowed = today
        .with_year(today.year() - 18)
        .unwrap_or_else(|| today - chrono::Duration::days(18 * 365));
    if dob > latest_allowed {
        return Err(AppError::BadRequest(
            "You must be at least 18 years old.".to_string(),
        ));
    }
    Ok(())
}

fn validate_timezone(value: &str) -> Result<(), AppError> {
    let value = value.trim();
    let allowed_prefixes = [
        "Africa/",
        "America/",
        "Asia/",
        "Atlantic/",
        "Australia/",
        "Europe/",
        "Pacific/",
    ];
    let has_allowed_chars = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'));
    if value == "UTC"
        || (value.len() <= 64
            && allowed_prefixes.iter().any(|p| value.starts_with(p))
            && has_allowed_chars)
    {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "Timezone must be a valid IANA timezone identifier.".to_string(),
        ))
    }
}

fn clean_external_url(opt: &Option<String>) -> Option<String> {
    let url = opt.as_ref()?.trim();
    if url.is_empty() {
        return None;
    }
    match url::Url::parse(url) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => Some(url.to_string()),
        _ => None,
    }
}

/// Update user profile fields (name, phone, country, timezone).
pub async fn update_profile(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdateProfileForm,
) -> Result<(), AppError> {
    validate_required_short_text("First name", &form.first_name, 100)?;
    validate_short_text("Middle name", &form.middle_name, 100)?;
    validate_required_short_text("Last name", &form.last_name, 100)?;
    validate_short_text("City", &form.city, 100)?;
    validate_short_text("State / region", &form.state_province, 100)?;
    validate_short_text("Postal code", &form.postal_code, 20)?;
    validate_short_text("Tax ID", &form.tax_id, 50)?;
    validate_short_text("Address line 1", &form.address_line_1, 255)?;
    validate_short_text("Address line 2", &form.address_line_2, 255)?;
    validate_date_of_birth(&form.date_of_birth)?;
    validate_iso_code("Country", &form.country)?;
    validate_iso_code("Nationality", &form.nationality)?;
    if let Some(ref gender) = form.gender {
        if !gender.is_empty() && !["male", "female", "other"].contains(&gender.as_str()) {
            return Err(AppError::BadRequest(
                "Gender must be male, female, or other.".to_string(),
            ));
        }
    }

    // Validate phone format (optional, but if provided must be valid)
    if let Some(ref phone) = form.phone_number {
        if !phone.is_empty() {
            let trimmed = phone.trim();
            if !trimmed.starts_with('+') || trimmed.len() < 7 || trimmed.len() > 20 {
                return Err(AppError::BadRequest(
                    "Phone number must start with '+' and be 7-20 characters.".to_string(),
                ));
            }
            // Check digits only after the +
            if !trimmed[1..]
                .chars()
                .all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
            {
                return Err(AppError::BadRequest(
                    "Phone number contains invalid characters.".to_string(),
                ));
            }
        }
    }

    if let Some(ref timezone) = form.timezone {
        validate_timezone(timezone)?;
    }

    // Upsert user_profiles with all fields
    sqlx::query(
        r#"
        INSERT INTO user_profiles (user_id, first_name, middle_name, last_name, gender, phone_number, country,
            date_of_birth, nationality, address_line_1, address_line_2, city,
            state_province, postal_code, tax_id, annual_income_cents)
        VALUES ($1, $2, $3, $4, $5, $6, $7,
            $8::DATE, $9, $10, $11, $12,
            $13, $14, $15, $16)
        ON CONFLICT (user_id) DO UPDATE SET
            first_name     = CASE WHEN $17 THEN EXCLUDED.first_name ELSE user_profiles.first_name END,
            middle_name    = CASE WHEN $18 THEN EXCLUDED.middle_name ELSE user_profiles.middle_name END,
            last_name      = CASE WHEN $19 THEN EXCLUDED.last_name ELSE user_profiles.last_name END,
            gender         = CASE WHEN $20 THEN EXCLUDED.gender ELSE user_profiles.gender END,
            phone_number   = CASE WHEN $21 THEN EXCLUDED.phone_number ELSE user_profiles.phone_number END,
            country        = CASE WHEN $22 THEN EXCLUDED.country ELSE user_profiles.country END,
            date_of_birth  = CASE WHEN $23 THEN EXCLUDED.date_of_birth ELSE user_profiles.date_of_birth END,
            nationality    = CASE WHEN $24 THEN EXCLUDED.nationality ELSE user_profiles.nationality END,
            address_line_1 = CASE WHEN $25 THEN EXCLUDED.address_line_1 ELSE user_profiles.address_line_1 END,
            address_line_2 = CASE WHEN $26 THEN EXCLUDED.address_line_2 ELSE user_profiles.address_line_2 END,
            city           = CASE WHEN $27 THEN EXCLUDED.city ELSE user_profiles.city END,
            state_province = CASE WHEN $28 THEN EXCLUDED.state_province ELSE user_profiles.state_province END,
            postal_code    = CASE WHEN $29 THEN EXCLUDED.postal_code ELSE user_profiles.postal_code END,
            tax_id         = CASE WHEN $30 THEN EXCLUDED.tax_id ELSE user_profiles.tax_id END,
            annual_income_cents = CASE WHEN $31 THEN EXCLUDED.annual_income_cents ELSE user_profiles.annual_income_cents END,
            updated_at     = NOW()
        "#,
    )
    .bind(user_id)
    .bind(sanitize_nullable_text(&form.first_name))
    .bind(sanitize_nullable_text(&form.middle_name))
    .bind(sanitize_nullable_text(&form.last_name))
    .bind(sanitize_nullable_text(&form.gender))
    .bind(form.phone_number.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
    .bind(form.country.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
    .bind(form.date_of_birth.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(sanitize_nullable_text(&form.nationality))
    .bind(sanitize_nullable_text(&form.address_line_1))
    .bind(sanitize_nullable_text(&form.address_line_2))
    .bind(sanitize_nullable_text(&form.city))
    .bind(sanitize_nullable_text(&form.state_province))
    .bind(sanitize_nullable_text(&form.postal_code))
    .bind(sanitize_nullable_text(&form.tax_id))
    .bind(form.annual_income_cents)
    .bind(form.first_name.is_some())
    .bind(form.middle_name.is_some())
    .bind(form.last_name.is_some())
    .bind(form.gender.is_some())
    .bind(form.phone_number.is_some())
    .bind(form.country.is_some())
    .bind(form.date_of_birth.is_some())
    .bind(form.nationality.is_some())
    .bind(form.address_line_1.is_some())
    .bind(form.address_line_2.is_some())
    .bind(form.city.is_some())
    .bind(form.state_province.is_some())
    .bind(form.postal_code.is_some())
    .bind(form.tax_id.is_some())
    .bind(form.annual_income_cents.is_some())
    .execute(pool)
    .await?;

    // Update timezone in user_settings
    if let Some(ref tz) = form.timezone {
        sqlx::query(
            r#"INSERT INTO user_settings (user_id, timezone) VALUES ($1, $2)
               ON CONFLICT (user_id) DO UPDATE SET timezone = $2, updated_at = NOW()"#,
        )
        .bind(user_id)
        .bind(tz)
        .execute(pool)
        .await?;
    }

    Ok(())
}

// ─── UPDATE: Notifications ────────────────────────────────────

/// Update notification preferences.
pub async fn update_notifications(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdateNotificationsForm,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO user_settings (user_id, email_notifications, push_notifications)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id) DO UPDATE
           SET email_notifications = $2, push_notifications = $3, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(form.email_notifications)
    .bind(form.push_notifications)
    .execute(pool)
    .await?;

    Ok(())
}

// ─── UPDATE: Leaderboard ───────────────────────────────────────

/// Update leaderboard preferences.
pub async fn update_leaderboard(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdateLeaderboardForm,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO leaderboard_preferences (user_id, visible, show_avatar, display_name, bio)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (user_id) DO UPDATE
           SET visible = $2, show_avatar = $3, display_name = $4, bio = $5, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(form.visible)
    .bind(form.show_avatar)
    .bind(sanitize_opt(&form.display_name).filter(|s| !s.is_empty()))
    .bind(sanitize_opt(&form.bio).filter(|s| !s.is_empty()))
    .execute(pool)
    .await?;

    Ok(())
}

// ─── UPDATE: Preferences ───────────────────────────────────────

/// Update user preferences (language, currency, timezone).
pub async fn update_preferences(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdatePreferencesForm,
) -> Result<(), AppError> {
    // Validate language
    if !ALLOWED_LANGUAGES.contains(&form.language.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Language must be one of: {}",
            ALLOWED_LANGUAGES.join(", ")
        )));
    }

    // Validate currency
    if !ALLOWED_CURRENCIES.contains(&form.currency.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Currency must be one of: {}",
            ALLOWED_CURRENCIES.join(", ")
        )));
    }

    sqlx::query(
        r#"INSERT INTO user_settings (user_id, language, currency, timezone)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id) DO UPDATE
           SET language = $2, currency = $3, timezone = $4, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(&form.language)
    .bind(&form.currency)
    .bind(sanitize::sanitize_text(&form.timezone))
    .execute(pool)
    .await?;

    Ok(())
}

// ─── UPDATE: Social Links ──────────────────────────────────────

pub async fn update_social_links(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdateSocialLinksForm,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO user_profiles (
            user_id, social_twitter_url, social_linkedin_url, social_instagram_url,
            social_telegram_url, social_discord, social_website_url
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (user_id) DO UPDATE SET
            social_twitter_url = $2,
            social_linkedin_url = $3,
            social_instagram_url = $4,
            social_telegram_url = $5,
            social_discord = $6,
            social_website_url = $7,
            updated_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(clean_external_url(&form.twitter))
    .bind(clean_external_url(&form.linkedin))
    .bind(clean_external_url(&form.instagram))
    .bind(clean_external_url(&form.telegram))
    .bind(sanitize_opt(&form.discord).filter(|s| !s.is_empty()))
    .bind(clean_external_url(&form.website))
    .execute(pool)
    .await?;

    Ok(())
}

// ─── UPDATE: Developer Profile ─────────────────────────────────

async fn ensure_developer(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let is_dev = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS (
            SELECT 1 FROM user_roles ur
            JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1 AND r.name = 'developer'
        )"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if is_dev {
        Ok(())
    } else {
        Err(AppError::Unauthorized(
            "Developer settings are only available to developer accounts.".to_string(),
        ))
    }
}

pub async fn update_developer_profile(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdateDeveloperProfileForm,
) -> Result<(), AppError> {
    ensure_developer(pool, user_id).await?;

    let company_name = sanitize_opt(&form.company_name)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("Company name is required.".to_string()))?;
    if company_name.len() > 255 {
        return Err(AppError::BadRequest(
            "Company name must be 255 characters or less.".to_string(),
        ));
    }
    let description = form
        .description
        .as_ref()
        .map(|s| sanitize::sanitize_multiline(s))
        .filter(|s| !s.is_empty());
    if description.as_ref().is_some_and(|s| s.len() > 1000) {
        return Err(AppError::BadRequest(
            "Developer description must be 1000 characters or less.".to_string(),
        ));
    }

    sqlx::query(
        r#"INSERT INTO developer_profiles (user_id, company_name, description)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id) DO UPDATE
           SET company_name = $2, description = $3, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(company_name)
    .bind(description)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_developer_links(
    pool: &PgPool,
    user_id: Uuid,
    form: UpdateDeveloperLinksForm,
) -> Result<(), AppError> {
    ensure_developer(pool, user_id).await?;

    sqlx::query(
        r#"INSERT INTO developer_profiles (
              user_id, company_name, website_url, github_url, x_url, linkedin_url, youtube_url
           )
           VALUES ($1, 'Developer', $2, $3, $4, $5, $6)
           ON CONFLICT (user_id) DO UPDATE
           SET website_url = $2, github_url = $3, x_url = $4,
               linkedin_url = $5, youtube_url = $6, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(clean_external_url(&form.website))
    .bind(clean_external_url(&form.github))
    .bind(clean_external_url(&form.twitter))
    .bind(clean_external_url(&form.linkedin))
    .bind(clean_external_url(&form.youtube))
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_developer_logo(
    pool: &PgPool,
    user_id: Uuid,
    logo_url: &str,
) -> Result<(), AppError> {
    ensure_developer(pool, user_id).await?;

    sqlx::query(
        r#"INSERT INTO developer_profiles (user_id, company_name, logo_url)
           VALUES ($1, 'Developer', $2)
           ON CONFLICT (user_id) DO UPDATE SET logo_url = $2, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(logo_url)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn unlink_oauth_account(
    pool: &PgPool,
    user_id: Uuid,
    connection_id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let password_hash: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
    let remaining_oauth_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
    if password_hash.is_none() && remaining_oauth_count <= 1 {
        return Err(AppError::BadRequest(
            "Add a password before disconnecting your last social sign-in.".to_string(),
        ));
    }

    let deleted = sqlx::query("DELETE FROM oauth_accounts WHERE id = $1 AND user_id = $2")
        .bind(connection_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    if deleted == 0 {
        return Err(AppError::NotFound(
            "OAuth connection not found.".to_string(),
        ));
    }

    tx.commit().await?;

    Ok(())
}

// ─── CHANGE: Email ─────────────────────────────────────────────

/// Change user's email address. Requires current password verification.
/// Resets email_verified to false (security measure).
pub async fn change_email(
    pool: &PgPool,
    user_id: Uuid,
    new_email: &str,
    current_password: &str,
) -> Result<(), AppError> {
    let new_email = new_email.trim().to_lowercase();

    // Validate new email format
    crate::common::validation::validate_email(&new_email)?;

    // Fetch current password hash
    let password_hash: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .flatten();

    let password_hash = password_hash.ok_or_else(|| {
        AppError::BadRequest(
            "Cannot change email for accounts using social login only.".to_string(),
        )
    })?;

    // Verify current password
    verify_password(current_password, &password_hash)?;

    // Check new email doesn't already exist
    let existing =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE email = $1 AND id != $2")
            .bind(&new_email)
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    if existing > 0 {
        return Err(AppError::Conflict(
            "An account with this email already exists.".to_string(),
        ));
    }

    // Update email and reset verification
    sqlx::query("UPDATE users SET email = $1, email_verified = FALSE WHERE id = $2")
        .bind(&new_email)
        .bind(user_id)
        .execute(pool)
        .await?;

    // Audit log
    crate::common::audit::log(
        pool,
        Some(user_id),
        "email_changed",
        "user",
        Some(user_id),
        None,
        None,
    )
    .await
    .ok();

    tracing::info!("User {} changed email to {}", user_id, new_email);

    Ok(())
}

// ─── CHANGE: Password ──────────────────────────────────────────

/// Change user's password. Requires current password verification.
pub async fn change_password(
    pool: &PgPool,
    user_id: Uuid,
    current_session_token: &str,
    current_password: &str,
    new_password: &str,
    confirm_password: &str,
) -> Result<(), AppError> {
    // Check confirm matches
    if new_password != confirm_password {
        return Err(AppError::BadRequest(
            "New password and confirmation do not match.".to_string(),
        ));
    }

    // Validate new password strength
    crate::common::validation::validate_password(new_password)?;

    // Fetch current password hash
    let password_hash: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .flatten();

    let password_hash = password_hash.ok_or_else(|| {
        AppError::BadRequest(
            "Cannot change password for accounts using social login only.".to_string(),
        )
    })?;

    // Verify current password
    verify_password(current_password, &password_hash)?;
    if verify_password(new_password, &password_hash).is_ok() {
        return Err(AppError::BadRequest(
            "New password must be different from your current password.".to_string(),
        ));
    }

    // Hash new password
    let new_hash = hash_password(new_password)?;

    let mut tx = pool.begin().await?;

    // Update password
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // Revoke all other active sessions after a credential change.
    sqlx::query("DELETE FROM user_sessions WHERE user_id = $1 AND session_token != $2")
        .bind(user_id)
        .bind(current_session_token)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Audit log
    crate::common::audit::log(
        pool,
        Some(user_id),
        "password_changed",
        "user",
        Some(user_id),
        None,
        None,
    )
    .await
    .ok();

    tracing::info!("User {} changed password", user_id);

    Ok(())
}

// ─── CHANGE: Phone ─────────────────────────────────────────────

/// Change user's phone number.
pub async fn change_phone(pool: &PgPool, user_id: Uuid, new_phone: &str) -> Result<(), AppError> {
    let trimmed = new_phone.trim();

    // If not empty, validate format
    if !trimmed.is_empty() {
        if !trimmed.starts_with('+') || trimmed.len() < 7 || trimmed.len() > 20 {
            return Err(AppError::BadRequest(
                "Phone number must start with '+' and be 7-20 characters.".to_string(),
            ));
        }
        if !trimmed[1..]
            .chars()
            .all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
        {
            return Err(AppError::BadRequest(
                "Phone number contains invalid characters.".to_string(),
            ));
        }
    }

    let phone_value = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };

    sqlx::query(
        r#"INSERT INTO user_profiles (user_id, phone_number)
           VALUES ($1, $2)
           ON CONFLICT (user_id) DO UPDATE
           SET phone_number = $2, updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(&phone_value)
    .execute(pool)
    .await?;

    crate::common::audit::log(
        pool,
        Some(user_id),
        "phone_changed",
        "user",
        Some(user_id),
        None,
        None,
    )
    .await
    .ok();

    Ok(())
}

// ─── Private helpers ───────────────────────────────────────────

/// Hash a password with Argon2id.
fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {}", e)))?;
    Ok(hash.to_string())
}

/// Verify a password against an Argon2id hash.
fn verify_password(password: &str, hash: &str) -> Result<(), AppError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash format: {}", e)))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::BadRequest("Current password is incorrect.".to_string()))?;

    Ok(())
}

// ─── 2FA Management ────────────────────────────────────────────

/// Disable 2FA for a user.
pub async fn disable_totp(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    current_session_token: &str,
    current_password: &str,
    code: &str,
) -> Result<(), AppError> {
    let row = sqlx::query(
        r#"
        SELECT u.password_hash, us.totp_secret, COALESCE(us.totp_enabled, FALSE) as totp_enabled
        FROM users u
        LEFT JOIN user_settings us ON us.user_id = u.id
        WHERE u.id = $1 AND u.status = 'active'
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found.".to_string()))?;

    use sqlx::Row;
    let password_hash: Option<String> = row.try_get("password_hash").unwrap_or(None);
    let password_hash = password_hash.ok_or_else(|| {
        AppError::BadRequest(
            "Add a password before changing two-factor authentication.".to_string(),
        )
    })?;
    verify_password(current_password, &password_hash)?;

    let totp_enabled: bool = row.try_get("totp_enabled").unwrap_or(false);
    if !totp_enabled {
        return Ok(());
    }

    let secret: Option<String> = row.try_get("totp_secret").unwrap_or(None);
    let secret = secret.ok_or_else(|| {
        AppError::BadRequest("Two-factor authentication is not configured.".to_string())
    })?;
    let secret = crate::auth::service::decrypt_stored_totp_secret(&secret)?;
    if !crate::auth::service::verify_totp_code_with_replay_guard(redis, user_id, &secret, code)
        .await
    {
        return Err(AppError::Unauthorized(
            "Invalid authentication code.".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE user_settings SET totp_enabled = FALSE, totp_secret = NULL, updated_at = NOW() WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // Force other sessions to re-authenticate after a 2FA downgrade.
    sqlx::query("DELETE FROM user_sessions WHERE user_id = $1 AND session_token != $2")
        .bind(user_id)
        .bind(current_session_token)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    crate::common::audit::log(
        pool,
        Some(user_id),
        "totp_disabled",
        "user",
        Some(user_id),
        None,
        None,
    )
    .await
    .ok();

    Ok(())
}

// ─── GDPR: Data Export (Art. 15/20) ────────────────────────────

/// Export all user data as a JSON value for GDPR data portability.
/// Returns a comprehensive JSON object containing all personal data.
pub async fn export_user_data(pool: &PgPool, user_id: Uuid) -> Result<serde_json::Value, AppError> {
    use sqlx::Row;

    // 1. User account
    let user = sqlx::query(
        r#"SELECT email, status, email_verified, avatar_url,
                  TO_CHAR(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at
           FROM users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let user_data = match user {
        Some(r) => serde_json::json!({
            "email": r.try_get::<Option<String>, _>("email").unwrap_or(None),
            "status": r.try_get::<Option<String>, _>("status").unwrap_or(None),
            "email_verified": r.try_get::<Option<bool>, _>("email_verified").unwrap_or(None),
            "avatar_url": r.try_get::<Option<String>, _>("avatar_url").unwrap_or(None),
            "created_at": r.try_get::<Option<String>, _>("created_at").unwrap_or(None),
        }),
        None => return Err(AppError::NotFound("User not found.".to_string())),
    };

    // 2. Profile
    let profile = sqlx::query(
        r#"SELECT first_name, last_name, phone_number, country, nationality,
                  address_line_1, address_line_2, city, state_province, postal_code,
                  TO_CHAR(date_of_birth, 'YYYY-MM-DD') as date_of_birth,
                  tax_id, annual_income_cents
           FROM user_profiles WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let profile_data = profile.map(|r| {
        serde_json::json!({
            "first_name": r.try_get::<Option<String>, _>("first_name").unwrap_or(None),
            "last_name": r.try_get::<Option<String>, _>("last_name").unwrap_or(None),
            "phone_number": r.try_get::<Option<String>, _>("phone_number").unwrap_or(None),
            "country": r.try_get::<Option<String>, _>("country").unwrap_or(None),
            "nationality": r.try_get::<Option<String>, _>("nationality").unwrap_or(None),
            "address_line_1": r.try_get::<Option<String>, _>("address_line_1").unwrap_or(None),
            "address_line_2": r.try_get::<Option<String>, _>("address_line_2").unwrap_or(None),
            "city": r.try_get::<Option<String>, _>("city").unwrap_or(None),
            "state_province": r.try_get::<Option<String>, _>("state_province").unwrap_or(None),
            "postal_code": r.try_get::<Option<String>, _>("postal_code").unwrap_or(None),
            "date_of_birth": r.try_get::<Option<String>, _>("date_of_birth").unwrap_or(None),
            "tax_id": r.try_get::<Option<String>, _>("tax_id").unwrap_or(None),
            "annual_income_cents": r.try_get::<Option<i64>, _>("annual_income_cents").unwrap_or(None),
        })
    });

    // 3. Investments
    let investments = sqlx::query(
        r#"SELECT i.asset_id, a.title as asset_title, i.tokens_owned,
                  i.purchase_value_cents, i.total_rental_cents, i.status,
                  TO_CHAR(i.purchased_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as purchased_at
           FROM investments i
           JOIN assets a ON a.id = i.asset_id
           WHERE i.user_id = $1 ORDER BY i.purchased_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let investments_data: Vec<serde_json::Value> = investments
        .iter()
        .map(|r| {
            serde_json::json!({
                "asset_id": r.try_get::<Option<Uuid>, _>("asset_id").unwrap_or(None).map(|u| u.to_string()),
                "asset_title": r.try_get::<Option<String>, _>("asset_title").unwrap_or(None),
                "tokens_owned": r.try_get::<Option<i32>, _>("tokens_owned").unwrap_or(None),
                "purchase_value_cents": r.try_get::<Option<i64>, _>("purchase_value_cents").unwrap_or(None),
                "total_rental_cents": r.try_get::<Option<i64>, _>("total_rental_cents").unwrap_or(None),
                "status": r.try_get::<Option<String>, _>("status").unwrap_or(None),
                "purchased_at": r.try_get::<Option<String>, _>("purchased_at").unwrap_or(None),
            })
        })
        .collect();

    // 4. Wallet balances
    let wallets = sqlx::query(
        r#"SELECT wallet_type, currency, balance_cents
           FROM wallets WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let wallets_data: Vec<serde_json::Value> = wallets
        .iter()
        .map(|r| {
            serde_json::json!({
                "wallet_type": r.try_get::<Option<String>, _>("wallet_type").unwrap_or(None),
                "currency": r.try_get::<Option<String>, _>("currency").unwrap_or(None),
                "balance_cents": r.try_get::<Option<i64>, _>("balance_cents").unwrap_or(None),
            })
        })
        .collect();

    // 5. Wallet transactions (last 1000)
    let transactions = sqlx::query(
        r#"SELECT wt.type, wt.status, wt.amount_cents, wt.currency, wt.description,
                  TO_CHAR(wt.created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at
           FROM wallet_transactions wt
           JOIN wallets w ON w.id = wt.wallet_id
           WHERE w.user_id = $1
           ORDER BY wt.created_at DESC LIMIT 1000"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let transactions_data: Vec<serde_json::Value> = transactions
        .iter()
        .map(|r| {
            serde_json::json!({
                "type": r.try_get::<Option<String>, _>("type").unwrap_or(None),
                "status": r.try_get::<Option<String>, _>("status").unwrap_or(None),
                "amount_cents": r.try_get::<Option<i64>, _>("amount_cents").unwrap_or(None),
                "currency": r.try_get::<Option<String>, _>("currency").unwrap_or(None),
                "description": r.try_get::<Option<String>, _>("description").unwrap_or(None),
                "created_at": r.try_get::<Option<String>, _>("created_at").unwrap_or(None),
            })
        })
        .collect();

    // 6. Settings
    let settings = sqlx::query(
        r#"SELECT language, currency, timezone, email_notifications, push_notifications
           FROM user_settings WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let settings_data = settings.map(|r| {
        serde_json::json!({
            "language": r.try_get::<Option<String>, _>("language").unwrap_or(None),
            "currency": r.try_get::<Option<String>, _>("currency").unwrap_or(None),
            "timezone": r.try_get::<Option<String>, _>("timezone").unwrap_or(None),
            "email_notifications": r.try_get::<Option<bool>, _>("email_notifications").unwrap_or(None),
            "push_notifications": r.try_get::<Option<bool>, _>("push_notifications").unwrap_or(None),
        })
    });

    // 7. KYC records (anonymized — status only, no docs)
    let kyc = sqlx::query(
        r#"SELECT status, TO_CHAR(created_at, 'YYYY-MM-DD"T"HH24:MI:SS"Z"') as created_at
           FROM kyc_records WHERE user_id = $1 ORDER BY created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let kyc_data: Vec<serde_json::Value> = kyc
        .iter()
        .map(|r| {
            serde_json::json!({
                "status": r.try_get::<Option<String>, _>("status").unwrap_or(None),
                "created_at": r.try_get::<Option<String>, _>("created_at").unwrap_or(None),
            })
        })
        .collect();

    crate::common::audit::log(
        pool,
        Some(user_id),
        "data_export_requested",
        "user",
        Some(user_id),
        None,
        None,
    )
    .await
    .ok();

    Ok(serde_json::json!({
        "export_date": chrono::Utc::now().to_rfc3339(),
        "user_id": user_id.to_string(),
        "account": user_data,
        "profile": profile_data,
        "investments": investments_data,
        "wallets": wallets_data,
        "transactions": transactions_data,
        "settings": settings_data,
        "kyc_records": kyc_data,
    }))
}

// ─── GDPR: Selective Account Deletion (Art. 17) ────────────────

/// Selectively delete a user account per GDPR + financial regulations.
///
/// Per Masterplan §1.8 Q7:
/// - ✅ DELETE: Personal profile data (name, address, phone), preferences, sessions
/// - ✅ ANONYMIZE: User record (email→deleted hash, name→cleared)
/// - ❌ KEEP: KYC records, wallet transactions, audit logs, investments (regulatory retention 5-10 years)
///
/// Requires password verification for security.
pub async fn delete_account_selective(
    pool: &PgPool,
    user_id: Uuid,
    current_password: &str,
    confirm_phrase: &str,
) -> Result<(), AppError> {
    use sqlx::Row;

    if confirm_phrase != "DELETE" {
        return Err(AppError::BadRequest(
            "Type DELETE to confirm account deletion.".to_string(),
        ));
    }

    // 1. Verify the user exists and get their password hash
    let user_row =
        sqlx::query("SELECT password_hash, email FROM users WHERE id = $1 AND status = 'active'")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found or already deleted.".to_string()))?;

    let password_hash: Option<String> = user_row.try_get("password_hash").unwrap_or(None);
    let _email: String = user_row.try_get("email").unwrap_or_default();

    let password_hash = password_hash.ok_or_else(|| {
        AppError::BadRequest("Add a password before deleting your account.".to_string())
    })?;
    verify_password(current_password, &password_hash)?;

    // 5. Begin transaction for atomic check and deletion
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to start deletion transaction: {}", e)))?;

    // 2. Check for non-zero wallet balance — cannot delete with funds
    let balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(balance_cents), 0)::bigint FROM wallets WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    if balance > 0 {
        return Err(AppError::BadRequest(
            "Cannot delete account with remaining wallet balance. Please withdraw all funds first."
                .to_string(),
        ));
    }

    // 3. Check for active investments
    let active_investments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM investments WHERE user_id = $1 AND status IN ('funding_in_progress', 'active')",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    if active_investments > 0 {
        return Err(AppError::BadRequest(
            "Cannot delete account with active investments. Please wait for all investments to complete or cancel them first."
                .to_string(),
        ));
    }

    // 4. Generate anonymized identifier
    let anon_hash = format!("deleted_{}", &user_id.to_string()[..8]);
    let anon_email = format!("{}@deleted.poool.co", anon_hash);

    // 5a. Anonymize user record — keep the row but clear all PII
    sqlx::query(
        r#"UPDATE users SET
            email = $2,
            password_hash = NULL,
            avatar_url = NULL,
            email_verified = FALSE,
            status = 'deleted',
            updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(user_id)
    .bind(&anon_email)
    .execute(&mut *tx)
    .await?;

    // 5b. Clear all profile PII
    sqlx::query(
        r#"UPDATE user_profiles SET
            first_name = NULL,
            middle_name = NULL,
            last_name = NULL,
            gender = NULL,
            display_name = NULL,
            phone_number = NULL,
            address_line_1 = NULL,
            address_line_2 = NULL,
            city = NULL,
            state_province = NULL,
            postal_code = NULL,
            tax_id = NULL,
            nationality = NULL,
            date_of_birth = NULL,
            annual_income_cents = NULL,
            social_twitter_url = NULL,
            social_linkedin_url = NULL,
            social_instagram_url = NULL,
            social_telegram_url = NULL,
            social_discord = NULL,
            social_website_url = NULL,
            updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // 5c. Delete all sessions (force logout)
    sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5d. Delete user settings and preferences
    sqlx::query("DELETE FROM user_settings WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5e. Delete leaderboard preferences
    sqlx::query("DELETE FROM leaderboard_preferences WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5f. Delete notifications
    sqlx::query("DELETE FROM notifications WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5g. Delete cart items
    sqlx::query("DELETE FROM cart_items WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5h. Delete oauth accounts
    sqlx::query("DELETE FROM oauth_accounts WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5i. Delete password reset tokens
    sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5j. Delete referral codes
    sqlx::query("DELETE FROM referral_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5k. Delete user tiers
    sqlx::query("DELETE FROM user_tiers WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5l. Delete user consents
    sqlx::query("DELETE FROM user_consents WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5m. Delete developer profile branding and links if present.
    sqlx::query("DELETE FROM developer_profiles WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    // 5n. Anonymize support tickets (keep for audit, clear PII)
    sqlx::query(
        r#"UPDATE support_tickets SET
            subject = 'Deleted user ticket',
            message = 'Deleted user message',
            updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    // ── KEEP (regulatory): ──
    // - kyc_records (5-10 year retention)
    // - wallet_transactions (financial records)
    // - investments (ownership records)
    // - audit_logs (immutable by design)
    // - orders / order_items (financial records)
    // - dividend_payouts (financial records)
    // - wallets (balance = 0, kept for reconciliation)

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to commit account deletion: {}", e)))?;

    // 6. Audit log the deletion (immutable record, best-effort after commit)
    crate::common::audit::log(
        pool,
        Some(user_id),
        &format!("account_deleted_gdpr:anonymized_to:{}", anon_email),
        "user",
        Some(user_id),
        None,
        None,
    )
    .await
    .ok();

    tracing::info!(
        "GDPR account deletion completed for user {} (anonymized to {})",
        user_id,
        anon_email
    );

    Ok(())
}
