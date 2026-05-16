//! HTTP-Routen für das Developer-Team-Affiliate-System (Phase 2).
//!
//! Authentifizierung erfolgt über den bestehenden `DeveloperUser`-Extractor
//! aus `crate::developer::extractors`. Team-Ownership wird per Helper
//! `require_team_owner` geprüft.
//!
//! Routes:
//!   GET    /api/developer/affiliate/team                      — Team info (default team)
//!   GET    /api/developer/affiliate/team/members              — Liste aller Members + pending requests
//!   POST   /api/developer/affiliate/team/invite               — Developer lädt per Email ein
//!   POST   /api/developer/affiliate/team/members/:id/approve  — Pending request approven
//!   POST   /api/developer/affiliate/team/members/:id/remove   — Member entfernen
//!   GET    /api/developer/affiliate/team/summary              — Period summary
//!   GET    /api/developer/affiliate/team/by-member            — Breakdown per Member
//!   GET    /api/developer/affiliate/team/customers            — Customer-Liste
//!   GET    /api/developer/affiliate/team/products             — Product sales aggregation
//!
//! Member-Self-Service:
//!   POST   /api/affiliate/team/accept-invitation              — Token einlösen
//!   POST   /api/affiliate/team/self-request                   — via Developer-Slug Beitritt requestern
//!   GET    /api/affiliate/team/my-membership                  — Status der eigenen Membership

use crate::admin::extractors::ApiError;
use crate::auth::middleware;
use crate::auth::routes::AppState;
use crate::developer::extractors::DeveloperUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use chrono::NaiveDate;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// SHA-256 hex digest. Used to log a privacy-safe identifier for invite
/// rate-limit hits (we don't want the raw email in logs).
fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Liefert die Default-Team-Id des Developers. Legt automatisch ein leeres
/// Default-Team an, wenn keins existiert (Lazy Onboarding).
async fn get_or_create_default_team(
    pool: &PgPool,
    developer_user_id: Uuid,
) -> Result<Uuid, ApiError> {
    if let Some(row) = sqlx::query!(
        r#"SELECT id FROM developer_teams
           WHERE developer_user_id = $1 AND is_default = true AND status <> 'terminated'
           LIMIT 1"#,
        developer_user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    {
        return Ok(row.id);
    }

    let default_name = sqlx::query_scalar::<_, String>(
        "SELECT COALESCE(NULLIF(TRIM(email), ''), 'My Team') FROM users WHERE id = $1",
    )
    .bind(developer_user_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?
    .unwrap_or_else(|| "My Team".to_string());

    let row = sqlx::query!(
        r#"INSERT INTO developer_teams (developer_user_id, display_name, is_default, status)
           VALUES ($1, $2, true, 'active')
           RETURNING id"#,
        developer_user_id,
        default_name
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;

    // Developer braucht eine affiliates-Row als payout-recipient für
    // Team-Business-Commissions. Wir legen sie bei Bedarf automatisch an.
    let _ = crate::rewards::team_links::ensure_developer_has_affiliate_row(pool, developer_user_id)
        .await;

    Ok(row.id)
}

/// Prüft Team-Ownership.
async fn require_team_owner(
    pool: &PgPool,
    team_id: Uuid,
    developer_user_id: Uuid,
) -> Result<(), ApiError> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM developer_teams WHERE id = $1 AND developer_user_id = $2)",
    )
    .bind(team_id)
    .bind(developer_user_id)
    .fetch_one(pool)
    .await
    .map_err(ApiError::Database)?;
    if !ok {
        return Err(ApiError::Forbidden(
            "Not the owner of this team".to_string(),
        ));
    }
    Ok(())
}

fn parse_date_query(q: Option<String>, default_offset_days: i64) -> NaiveDate {
    q.as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| {
            chrono::Utc::now().date_naive() - chrono::Duration::days(default_offset_days)
        })
}

#[derive(Deserialize)]
pub struct DateRangeQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Deserialize)]
pub struct CustomersQuery {
    pub attribution_user_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct ProductsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Deserialize)]
pub struct InvitePayload {
    pub email: String,
}

#[derive(Deserialize)]
pub struct AcceptInvitationPayload {
    pub token: String,
}

#[derive(Deserialize)]
pub struct SelfRequestPayload {
    pub developer_slug: String,
}

#[derive(Deserialize)]
pub struct RemoveMemberPayload {
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTeamPayload {
    pub display_name: Option<String>,
    pub public_slug: Option<String>,
    // Bank / payout destination. Each field is Option<Option<String>> in spirit:
    // `None` = field not included in this PATCH (don't touch DB), `Some("")` =
    // explicit clear, `Some(value)` = update to value. JSON `null` maps to
    // Some(None) — used by the UI's "Discard bank details" button.
    pub bank_account_holder: Option<String>,
    pub bank_iban: Option<String>,
    pub bank_bic: Option<String>,
    pub bank_name: Option<String>,
    pub bank_country: Option<String>,
    // Phase-4 branding fields (mig 188). Same Option<String> semantics —
    // omit = leave untouched, "" = clear, value = update.
    pub logo_url: Option<String>,
    pub accent_color: Option<String>,
    pub email_from_display: Option<String>,
}

/// Strip spaces / dashes / dots and uppercase. IBAN format check is
/// length-based + checksum-light: 5-34 alphanumeric chars after compaction,
/// first 2 chars country code letters, next 2 chars digits (check digits).
/// Returns the compacted form, or BadRequest with a user-facing message.
fn validate_iban(input: &str) -> Result<String, ApiError> {
    let compact: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '.')
        .collect::<String>()
        .to_uppercase();
    if compact.is_empty() {
        return Ok(String::new());
    }
    if compact.len() < 5 || compact.len() > 34 {
        return Err(ApiError::BadRequest("IBAN must be 5–34 characters".into()));
    }
    if !compact.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::BadRequest(
            "IBAN may only contain letters and digits".into(),
        ));
    }
    let bytes = compact.as_bytes();
    if !bytes[0].is_ascii_alphabetic() || !bytes[1].is_ascii_alphabetic() {
        return Err(ApiError::BadRequest(
            "IBAN must start with a 2-letter country code".into(),
        ));
    }
    if !bytes[2].is_ascii_digit() || !bytes[3].is_ascii_digit() {
        return Err(ApiError::BadRequest(
            "IBAN positions 3–4 must be check digits".into(),
        ));
    }
    // B11 fix: MOD-97 checksum. Catches typos that length-only check misses.
    crate::rewards::service::validate_iban_mod97(&compact).map_err(|e| match e {
        crate::error::AppError::BadRequest(m) => ApiError::BadRequest(m),
        _ => ApiError::BadRequest("IBAN checksum could not be verified.".into()),
    })?;
    Ok(compact)
}

/// BIC/SWIFT: 8 or 11 chars, letters + digits, uppercase. Last 3 optional.
fn validate_bic(input: &str) -> Result<String, ApiError> {
    let compact: String = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_uppercase();
    if compact.is_empty() {
        return Ok(String::new());
    }
    if compact.len() != 8 && compact.len() != 11 {
        return Err(ApiError::BadRequest(
            "BIC must be 8 or 11 characters".into(),
        ));
    }
    if !compact.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::BadRequest(
            "BIC may only contain letters and digits".into(),
        ));
    }
    Ok(compact)
}

/// Normalize a free-text bank field: trim, length-check, return None if empty.
fn normalize_bank_text(input: &str, label: &str, max: usize) -> Result<String, ApiError> {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if trimmed.len() > max {
        return Err(ApiError::BadRequest(format!(
            "{} must be at most {} characters",
            label, max
        )));
    }
    Ok(trimmed)
}

/// 2-letter ISO-3166-1 alpha-2 country code (uppercase).
fn validate_country2(input: &str) -> Result<String, ApiError> {
    let compact: String = input.trim().to_uppercase();
    if compact.is_empty() {
        return Ok(String::new());
    }
    if compact.len() != 2 || !compact.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(ApiError::BadRequest(
            "Country must be a 2-letter ISO code (e.g. DE, AT, US)".into(),
        ));
    }
    Ok(compact)
}

/// Mask an IBAN for display: keep first 4 + last 4 chars, mask the middle.
fn mask_iban(value: &str) -> String {
    let compact: String = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if compact.len() <= 8 {
        return compact;
    }
    let prefix: String = compact.chars().take(4).collect();
    let suffix: String = compact
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{} **** **** {}", prefix, suffix)
}

/// Reserved slugs that would collide with platform routes or admin paths.
/// Sorted alphabetical for binary_search.
const RESERVED_SLUGS: &[&str] = &[
    "admin",
    "affiliate",
    "api",
    "app",
    "assets",
    "auth",
    "billing",
    "blog",
    "checkout",
    "community",
    "dashboard",
    "developer",
    "docs",
    "help",
    "kyc",
    "login",
    "logout",
    "marketplace",
    "marketing",
    "portfolio",
    "profile",
    "register",
    "rewards",
    "settings",
    "signup",
    "static",
    "support",
    "terms",
    "user",
    "wallet",
    "www",
];

/// Slug-Whitelist: lowercase letters, digits, hyphen. 3–40 chars,
/// no leading/trailing hyphen, no reserved platform slug.
fn validate_slug(s: &str) -> Result<String, ApiError> {
    let trimmed = s.trim().to_lowercase();
    if trimmed.is_empty() {
        return Ok(String::new()); // allow clear
    }
    if trimmed.len() < 3 || trimmed.len() > 40 {
        return Err(ApiError::BadRequest("Slug must be 3–40 characters".into()));
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ApiError::BadRequest(
            "Slug may contain only a–z, 0–9 and '-'".into(),
        ));
    }
    if trimmed.starts_with('-') || trimmed.ends_with('-') {
        return Err(ApiError::BadRequest(
            "Slug must not start or end with '-'".into(),
        ));
    }
    if RESERVED_SLUGS.binary_search(&trimmed.as_str()).is_ok() {
        return Err(ApiError::BadRequest(
            "Slug is reserved by the platform".into(),
        ));
    }
    Ok(trimmed)
}

/// Cheap email validation: RFC-style check (presence of `@`, dot in domain,
/// reasonable length). Full RFC 5322 validation is overkill — we just want
/// to block obvious garbage before hitting the DB lookup.
fn validate_email(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 320 {
        return Err(ApiError::BadRequest(
            "Email must be 1–320 characters".into(),
        ));
    }
    let parts: Vec<&str> = trimmed.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(ApiError::BadRequest(
            "Email must contain a single '@'".into(),
        ));
    }
    if !parts[1].contains('.') {
        return Err(ApiError::BadRequest(
            "Email domain must contain a dot".into(),
        ));
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(ApiError::BadRequest(
            "Email must not contain whitespace or control characters".into(),
        ));
    }
    Ok(trimmed.to_string())
}

// ── Developer Endpoints ─────────────────────────────────────────────────────

pub async fn get_team_info(
    dev: DeveloperUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    let row = sqlx::query!(
        r#"SELECT id, display_name, public_slug, status, created_at,
                  bank_account_holder, bank_iban,
                  bank_iban_encrypted, bank_iban_last4,
                  bank_bic, bank_name, bank_country,
                  logo_url, accent_color, email_from_display
           FROM developer_teams WHERE id = $1"#,
        team_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // B-P0-1 fix: prefer encrypted-column path. Plaintext `bank_iban` is
    // legacy + will be dropped in a follow-up mig once all rows are
    // backfilled. For now we read either, never decrypt the cipher for
    // display (only build the masked form from `bank_iban_last4`).
    let bank_iban_set = row.bank_iban_encrypted.is_some()
        || row
            .bank_iban
            .as_deref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
    let bank_iban_masked = if let Some(last4) = row.bank_iban_last4.as_deref() {
        Some(format!("**** **** **** {}", last4))
    } else {
        // Legacy path: row still on plaintext column.
        row.bank_iban.as_deref().map(mask_iban)
    };

    // Counter-Tile: O(1) per PK-Lookup
    let counter = sqlx::query!(
        r#"SELECT lifetime_revenue_cents, lifetime_commission_cents,
                  pending_commission_cents, payable_commission_cents,
                  paid_commission_cents, clawed_back_cents
           FROM affiliate_live_counters WHERE payout_user_id = $1"#,
        dev.user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let active_members: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM developer_team_memberships WHERE team_id = $1 AND status = 'active'",
    )
    .bind(team_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(serde_json::json!({
        "team_id": row.id,
        "display_name": row.display_name,
        "public_slug": row.public_slug,
        "status": row.status,
        "created_at": row.created_at,
        "active_members": active_members,
        "counters": counter.map(|c| serde_json::json!({
            "lifetime_revenue_cents":    c.lifetime_revenue_cents,
            "lifetime_commission_cents": c.lifetime_commission_cents,
            "pending_commission_cents":  c.pending_commission_cents,
            "payable_commission_cents":  c.payable_commission_cents,
            "paid_commission_cents":     c.paid_commission_cents,
            "clawed_back_cents":         c.clawed_back_cents,
        })),
        "bank": {
            "account_holder": row.bank_account_holder,
            "iban_masked":    bank_iban_masked,
            "iban_set":       bank_iban_set,
            "bic":            row.bank_bic,
            "bank_name":      row.bank_name,
            "country":        row.bank_country,
        },
        // Phase-4 branding fields surfaced for the settings page form.
        "branding": {
            "logo_url":           row.logo_url,
            "accent_color":       row.accent_color,
            "email_from_display": row.email_from_display,
        }
    }))
    .into_response())
}

pub async fn update_team(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateTeamPayload>,
) -> Result<axum::response::Response, ApiError> {
    // Phase-3 P1: 2FA step-up for bank-detail edits. Only triggers when
    // the PATCH touches any of the bank columns — display_name / slug
    // edits remain session-only since they don't affect payout routing.
    let touches_bank = payload.bank_account_holder.is_some()
        || payload.bank_iban.is_some()
        || payload.bank_bic.is_some()
        || payload.bank_country.is_some();
    if touches_bank {
        crate::auth::step_up::require_step_up_2fa(
            &state.db,
            state.redis.as_ref(),
            dev.user.id,
            crate::auth::step_up::FinancialAction::AffiliateBankEdit,
            0,
        )
        .await
        .map_err(ApiError::from)?;
    }

    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    let next_name: Option<String> = if let Some(n) = payload.display_name {
        let trimmed = n.trim();
        if trimmed.is_empty() || trimmed.len() > 120 {
            return Err(ApiError::BadRequest(
                "Display name must be 1–120 characters".into(),
            ));
        }
        Some(trimmed.to_string())
    } else {
        None
    };

    let slug_provided = payload.public_slug.is_some();
    let next_slug: Option<Option<String>> = if let Some(s) = payload.public_slug {
        let validated = validate_slug(&s)?;
        if validated.is_empty() {
            Some(None)
        } else {
            // Reject if slug already taken by another team
            let taken: bool = sqlx::query_scalar(
                r#"SELECT EXISTS(
                       SELECT 1 FROM developer_teams
                       WHERE LOWER(public_slug) = LOWER($1)
                         AND id <> $2
                         AND status <> 'terminated'
                   )"#,
            )
            .bind(&validated)
            .bind(team_id)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?;
            if taken {
                return Err(ApiError::BadRequest("Slug already in use".into()));
            }
            Some(Some(validated))
        }
    } else {
        None
    };

    // Bank-detail fields: each one is validated only if the caller included it
    // in this PATCH. Empty string → DB NULL (clear). Returns Option<Option<String>>
    // where outer None = leave column untouched.
    let next_holder: Option<Option<String>> = match payload.bank_account_holder {
        None => None,
        Some(s) => {
            let v = normalize_bank_text(&s, "Account holder", 120)?;
            Some(if v.is_empty() { None } else { Some(v) })
        }
    };
    let next_iban: Option<Option<String>> = match payload.bank_iban {
        None => None,
        Some(s) => {
            let v = validate_iban(&s)?;
            Some(if v.is_empty() { None } else { Some(v) })
        }
    };
    let next_bic: Option<Option<String>> = match payload.bank_bic {
        None => None,
        Some(s) => {
            let v = validate_bic(&s)?;
            Some(if v.is_empty() { None } else { Some(v) })
        }
    };
    let next_bank_name: Option<Option<String>> = match payload.bank_name {
        None => None,
        Some(s) => {
            let v = normalize_bank_text(&s, "Bank name", 120)?;
            Some(if v.is_empty() { None } else { Some(v) })
        }
    };
    let next_country: Option<Option<String>> = match payload.bank_country {
        None => None,
        Some(s) => {
            let v = validate_country2(&s)?;
            Some(if v.is_empty() { None } else { Some(v) })
        }
    };
    // Capture which bank fields are being touched BEFORE the moves below so
    // the audit log can name them precisely (H1 fix).
    let bank_field_changed_holder = next_holder.is_some();
    let bank_field_changed_iban = next_iban.is_some();
    let bank_field_changed_bic = next_bic.is_some();
    let bank_field_changed_name = next_bank_name.is_some();
    let bank_field_changed_country = next_country.is_some();
    let bank_touched = bank_field_changed_holder
        || bank_field_changed_iban
        || bank_field_changed_bic
        || bank_field_changed_name
        || bank_field_changed_country;

    let mut tx = state.db.begin().await.map_err(ApiError::Database)?;
    if let Some(n) = next_name.as_deref() {
        sqlx::query!(
            "UPDATE developer_teams SET display_name = $1, updated_at = NOW() WHERE id = $2",
            n,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(slug_opt) = next_slug {
        sqlx::query!(
            "UPDATE developer_teams SET public_slug = $1, updated_at = NOW() WHERE id = $2",
            slug_opt,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(v) = next_holder {
        sqlx::query!(
            "UPDATE developer_teams SET bank_account_holder = $1, updated_at = NOW() WHERE id = $2",
            v,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(v) = next_iban {
        // B-P0-1 fix: write the encrypted ciphertext envelope, never plaintext.
        // Plaintext column gets NULLed at the same time so a legacy value
        // doesn't shadow the new encrypted form. last4 cached for display.
        match v {
            None => {
                sqlx::query!(
                    r#"UPDATE developer_teams
                          SET bank_iban_encrypted = NULL,
                              bank_iban_last4 = NULL,
                              bank_iban = NULL,
                              updated_at = NOW()
                        WHERE id = $1"#,
                    team_id
                )
                .execute(&mut *tx)
                .await
                .map_err(ApiError::Database)?;
            }
            Some(plain) => {
                let cipher =
                    crate::rewards::service::encrypt_bank_iban(&plain).map_err(ApiError::from)?; // D1
                let last4 = crate::rewards::service::bank_iban_last4(&plain);
                sqlx::query!(
                    r#"UPDATE developer_teams
                          SET bank_iban_encrypted = $1,
                              bank_iban_last4 = $2,
                              bank_iban_key_version = $3,
                              bank_iban = NULL,
                              updated_at = NOW()
                        WHERE id = $4"#,
                    cipher,
                    last4,
                    crate::rewards::service::BANK_IBAN_KEY_VERSION,
                    team_id
                )
                .execute(&mut *tx)
                .await
                .map_err(ApiError::Database)?;
            }
        }
    }
    if let Some(v) = next_bic {
        sqlx::query!(
            "UPDATE developer_teams SET bank_bic = $1, updated_at = NOW() WHERE id = $2",
            v,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(v) = next_bank_name {
        sqlx::query!(
            "UPDATE developer_teams SET bank_name = $1, updated_at = NOW() WHERE id = $2",
            v,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(v) = next_country {
        sqlx::query!(
            "UPDATE developer_teams SET bank_country = $1, updated_at = NOW() WHERE id = $2",
            v,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }

    // ── Phase-4 branding fields ───────────────────────────────────────
    if let Some(raw) = &payload.logo_url {
        let v = raw.trim();
        let final_val: Option<String> = if v.is_empty() {
            None
        } else {
            // Reject anything that isn't HTTPS. We embed this URL in HTML
            // `<img src>` so an attacker-controlled http://… or javascript:
            // payload would be unacceptable. 512-char cap matches schema.
            if !v.starts_with("https://") || v.len() > 512 {
                return Err(ApiError::BadRequest(
                    "Logo URL must be HTTPS and ≤ 512 characters".into(),
                ));
            }
            Some(v.to_string())
        };
        sqlx::query!(
            "UPDATE developer_teams SET logo_url = $1, updated_at = NOW() WHERE id = $2",
            final_val,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(raw) = &payload.accent_color {
        let v = raw.trim();
        let final_val: Option<String> = if v.is_empty() {
            None
        } else {
            // Schema CHECK enforces `^#[0-9A-Fa-f]{6}$` — validate here
            // too so we return a 400 instead of a 500 from the DB.
            let ok =
                v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !ok {
                return Err(ApiError::BadRequest(
                    "Accent color must be a 6-digit hex string like #0000FF".into(),
                ));
            }
            Some(v.to_uppercase())
        };
        sqlx::query!(
            "UPDATE developer_teams SET accent_color = $1, updated_at = NOW() WHERE id = $2",
            final_val,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }
    if let Some(raw) = &payload.email_from_display {
        let v = raw.trim();
        let final_val: Option<String> = if v.is_empty() {
            None
        } else {
            // RFC 5322 limits display-name length; mig caps at 80.
            if v.len() > 80 {
                return Err(ApiError::BadRequest(
                    "Email sender name must be ≤ 80 characters".into(),
                ));
            }
            Some(v.to_string())
        };
        sqlx::query!(
            "UPDATE developer_teams SET email_from_display = $1, updated_at = NOW() WHERE id = $2",
            final_val,
            team_id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Database)?;
    }

    tx.commit().await.map_err(ApiError::Database)?;

    // Audit
    let _ = sqlx::query(
        "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
         VALUES ($1, 'team_settings_updated', 'developer_teams', $2, $3)",
    )
    .bind(dev.user.id)
    .bind(team_id)
    .bind(serde_json::json!({
        "display_name": next_name,
        "public_slug_present": slug_provided,
        "bank_touched": bank_touched,
    }))
    .execute(&state.db)
    .await;

    // H1 fix: bank-detail changes get their own audit row in addition to
    // the generic team_settings_updated. Compliance / chargeback teams need
    // to query "who changed the IBAN" without filtering through every
    // display-name edit.
    if bank_touched {
        let _ = sqlx::query(
            "INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
             VALUES ($1, 'team_bank_details_updated', 'developer_teams', $2, $3)",
        )
        .bind(dev.user.id)
        .bind(team_id)
        .bind(serde_json::json!({
            "fields_changed": {
                "account_holder": bank_field_changed_holder,
                "iban": bank_field_changed_iban,
                "bic": bank_field_changed_bic,
                "bank_name": bank_field_changed_name,
                "country": bank_field_changed_country,
            },
            // Never log the IBAN itself — even in audit_logs. Only the fact
            // it changed. Cipher rotation is tracked via the key_version
            // column.
        }))
        .execute(&state.db)
        .await;
    }

    Ok(Json(serde_json::json!({"status": "updated", "team_id": team_id})).into_response())
}

#[derive(Deserialize)]
pub struct TableQuery {
    pub q: Option<String>,    // search term
    pub sort: Option<String>, // column key
    pub dir: Option<String>,  // 'asc' | 'desc'
    pub limit: Option<i64>,   // page size (5..500)
    pub offset: Option<i64>,  // 0..100_000
}

impl TableQuery {
    fn limit_clamped(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(5, 500)
    }
    fn offset_clamped(&self) -> i64 {
        self.offset.unwrap_or(0).clamp(0, 100_000)
    }
    fn dir_sql(&self) -> &'static str {
        match self.dir.as_deref() {
            Some("asc") => "ASC",
            _ => "DESC",
        }
    }
    /// Whitelist sort column → SQL fragment. Falls back to default if unknown
    /// (prevents SQL injection AND silent bad-input).
    fn sort_sql(&self, allowed: &[(&str, &str)], default: &str) -> String {
        let key = self.sort.as_deref().unwrap_or("");
        let col = allowed
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, sql)| *sql)
            .unwrap_or(default);
        format!("{} {} NULLS LAST", col, self.dir_sql())
    }
    /// LOWER-cased trim of the search; empty → None.
    fn q_lower(&self) -> Option<String> {
        self.q
            .as_deref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
    }
}

#[derive(Deserialize)]
pub struct MembersTableQuery {
    pub q: Option<String>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Comma-separated list of allowed statuses (active, invited,
    /// pending_developer_approval, removed). Empty → all (except removed).
    pub status: Option<String>,
}

impl MembersTableQuery {
    fn table(&self) -> TableQuery {
        TableQuery {
            q: self.q.clone(),
            sort: self.sort.clone(),
            dir: self.dir.clone(),
            limit: self.limit,
            offset: self.offset,
        }
    }
    /// Parse `status=active,invited` into a whitelisted Vec. Unknown values
    /// dropped silently. Empty → default (all non-removed).
    fn status_filter(&self) -> Vec<&'static str> {
        let allowed = ["active", "invited", "pending_developer_approval", "removed"];
        let raw = self.status.as_deref().unwrap_or("");
        if raw.trim().is_empty() {
            return vec!["active", "invited", "pending_developer_approval"];
        }
        raw.split(',')
            .map(|s| s.trim())
            .filter_map(|s| allowed.iter().find(|&&a| a == s).copied())
            .collect()
    }
}

pub async fn list_team_members(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(mq): Query<MembersTableQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let q = mq.table();
    let limit = q.limit_clamped();
    let offset = q.offset_clamped();
    let search = q.q_lower();
    let statuses = mq.status_filter();
    if statuses.is_empty() {
        // All chips deselected → empty result, no SQL needed.
        return Ok(Json(serde_json::json!({
            "team_id": team_id, "total": 0, "limit": limit, "offset": offset, "members": []
        }))
        .into_response());
    }
    // Build whitelisted IN list (every value already in `allowed`, so safe).
    let status_in = statuses
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<_>>()
        .join(",");

    // Sortable columns whitelist → safe SQL expression
    let sort_expr = q.sort_sql(
        &[
            ("full_name", "full_name"),
            ("email", "u.email"),
            ("status", "m.status"),
            ("joined_at", "m.joined_at"),
            ("customers", "customer_count"),
            ("commission", "commission_cents"),
            ("last_sale", "last_sale_at"),
        ],
        // Default sort: active first, then most recent activity
        "m.status DESC, GREATEST(m.joined_at, m.invited_at)",
    );

    // Build the SQL dynamically since ORDER BY can't be parameterized.
    // Search placeholder differs between count ($2) and rows ($4) because
    // count only binds (team_id, search) while rows binds (team_id, limit,
    // offset, search).
    let where_search_count = if search.is_some() {
        " AND (LOWER(COALESCE(up.first_name,'') || ' ' || COALESCE(up.last_name,'')) ILIKE $2
              OR LOWER(u.email::text) ILIKE $2)"
    } else {
        ""
    };
    let where_search_rows = if search.is_some() {
        " AND (LOWER(COALESCE(up.first_name,'') || ' ' || COALESCE(up.last_name,'')) ILIKE $4
              OR LOWER(u.email::text) ILIKE $4)"
    } else {
        ""
    };

    let sql_total = format!(
        r#"SELECT COUNT(*)::BIGINT AS total
           FROM developer_team_memberships m
           LEFT JOIN users u ON u.id = m.user_id
           LEFT JOIN user_profiles up ON up.user_id = m.user_id
           WHERE m.team_id = $1
             AND m.status IN ({}){}"#,
        status_in, where_search_count
    );

    let sql_rows = format!(
        r#"SELECT m.id, m.user_id, m.role, m.status, m.invited_at, m.joined_at,
                  u.email::text AS email,
                  NULLIF(TRIM(BOTH ' ' FROM (
                      COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, '')
                  )), '') AS full_name,
                  al.id AS link_id,
                  al.code AS link_code,
                  -- Enrichment: # of attributed customers + lifetime commission
                  -- earned via this member + first/last conversion timestamps
                  COALESCE(stats.customer_count, 0)::BIGINT AS customer_count,
                  COALESCE(stats.commission_cents, 0)::BIGINT AS commission_cents,
                  stats.last_sale_at,
                  stats.first_sale_at
           FROM developer_team_memberships m
           LEFT JOIN users u ON u.id = m.user_id
           LEFT JOIN user_profiles up ON up.user_id = m.user_id
           LEFT JOIN affiliate_links al ON al.team_id = m.team_id
                                       AND al.attribution_user_id = m.user_id
                                       AND al.link_type = 'team_business'
                                       AND al.status = 'active'
           LEFT JOIN LATERAL (
               SELECT COUNT(DISTINCT ar.referred_user_id) AS customer_count,
                      COALESCE(SUM(ac.provisional_amount_cents), 0) AS commission_cents,
                      MAX(ac.created_at) AS last_sale_at,
                      MIN(ac.created_at) AS first_sale_at
                 FROM affiliate_referrals ar
                 LEFT JOIN affiliate_commissions ac ON ac.referral_id = ar.id
                WHERE ar.attribution_user_id = m.user_id
                  AND ar.link_id IN (SELECT id FROM affiliate_links WHERE team_id = m.team_id)
           ) AS stats ON TRUE
           WHERE m.team_id = $1
             AND m.status IN ({}){}
           ORDER BY {}
           LIMIT $2 OFFSET $3"#,
        status_in, where_search_rows, sort_expr
    );

    let total: i64 = match &search {
        Some(s) => sqlx::query_scalar(&sql_total)
            .bind(team_id)
            .bind(format!("%{}%", s))
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?,
        None => sqlx::query_scalar(&sql_total)
            .bind(team_id)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?,
    };

    let rows: Vec<sqlx::postgres::PgRow> = if let Some(s) = &search {
        sqlx::query(&sql_rows)
            .bind(team_id)
            .bind(limit)
            .bind(offset)
            .bind(format!("%{}%", s))
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?
    } else {
        let sql_no_q = sql_rows.replace(where_search_rows, "");
        sqlx::query(&sql_no_q)
            .bind(team_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?
    };

    use sqlx::Row;
    let members: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "membership_id": r.get::<Uuid, _>("id"),
                "user_id": r.get::<Uuid, _>("user_id"),
                "email": r.get::<Option<String>, _>("email"),
                "full_name": r.get::<Option<String>, _>("full_name"),
                "role": r.get::<String, _>("role"),
                "status": r.get::<String, _>("status"),
                "invited_at": r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("invited_at"),
                "joined_at": r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("joined_at"),
                "link_id": r.get::<Option<Uuid>, _>("link_id"),
                "link_code": r.get::<Option<String>, _>("link_code"),
                "customer_count": r.get::<i64, _>("customer_count"),
                "commission_cents": r.get::<i64, _>("commission_cents"),
                "first_sale_at": r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("first_sale_at"),
                "last_sale_at": r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_sale_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "team_id": team_id,
        "total": total,
        "limit": limit,
        "offset": offset,
        "members": members
    }))
    .into_response())
}

/// POST /api/developer/affiliate/team/invite-bulk
///
/// Phase-7: bulk-invite from a CSV (one email per line). Two-step:
///   * mode="dry_run" → returns per-row classification without
///     enqueuing any invites. Use to preview before commit.
///   * mode="commit"  → actually fires `invite_by_email` for each row
///     the dry-run would have accepted.
///
/// Hard limits: 100 rows per request; per-developer rate-limit gates
/// the commit path (one bulk-commit per minute) so a malicious script
/// can't sweep millions of emails through this endpoint.
#[derive(Deserialize)]
pub struct BulkInvitePayload {
    pub csv_text: String,
    /// "dry_run" (default) or "commit".
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(serde::Serialize)]
struct BulkInviteRow {
    line: usize,
    email: String,
    status: &'static str,
    /// Free-form note (validation failure / "already in team" / "queued").
    note: Option<String>,
}

pub async fn invite_member_bulk(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Json(payload): Json<BulkInvitePayload>,
) -> Result<axum::response::Response, ApiError> {
    let mode = payload.mode.as_deref().unwrap_or("dry_run");
    if mode != "dry_run" && mode != "commit" {
        return Err(ApiError::BadRequest(
            "mode must be 'dry_run' or 'commit'".into(),
        ));
    }
    // Rate-limit commits (the destructive path). Dry-runs unrestricted
    // so the UI can preview freely.
    if mode == "commit"
        && state
            .auth_rate_limiter
            .check(&format!("invite_bulk:dev:{}", dev.user.id))
            .await
            .is_err()
    {
        return Err(ApiError::TooManyRequests(
            "Bulk-commit rate limit — please wait a minute and retry.".into(),
        ));
    }

    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    // Parse: one email per line, strip BOM, trim, skip blanks + comment
    // lines starting with '#'. Hard cap of 100 rows.
    let mut rows: Vec<BulkInviteRow> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let stripped = payload.csv_text.trim_start_matches('\u{feff}').to_string();
    let mut accepted = 0usize;
    let mut skipped = 0usize;
    for (idx, raw_line) in stripped.lines().enumerate() {
        if rows.len() >= 100 {
            return Err(ApiError::BadRequest(
                "Bulk invite is limited to 100 rows per request".into(),
            ));
        }
        // Allow CSV-with-quoted-email-column shapes by stripping
        // surrounding quotes + taking the first comma-separated token.
        let trimmed = raw_line
            .trim()
            .trim_matches('"')
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if !seen.insert(lower.clone()) {
            rows.push(BulkInviteRow {
                line: idx + 1,
                email: trimmed,
                status: "skipped",
                note: Some("duplicate in CSV".into()),
            });
            skipped += 1;
            continue;
        }
        match validate_email(&trimmed) {
            Err(_) => {
                rows.push(BulkInviteRow {
                    line: idx + 1,
                    email: trimmed,
                    status: "invalid",
                    note: Some("not a valid email address".into()),
                });
                skipped += 1;
            }
            Ok(email_norm) => {
                if mode == "dry_run" {
                    rows.push(BulkInviteRow {
                        line: idx + 1,
                        email: email_norm,
                        status: "would_invite",
                        note: None,
                    });
                    accepted += 1;
                } else {
                    // Commit: actually fire the invite. We DON'T apply
                    // the per-developer + per-recipient rate-limit
                    // here because the bulk endpoint has its own
                    // upper bound (100 rows) + the bulk-rate-limit
                    // gate above. Service still enforces uniqueness +
                    // returns Option<None> on skip.
                    match crate::rewards::team_members::invite_by_email(
                        &state.db,
                        team_id,
                        &email_norm,
                        dev.user.id,
                    )
                    .await
                    {
                        Ok(Some(_)) => {
                            rows.push(BulkInviteRow {
                                line: idx + 1,
                                email: email_norm,
                                status: "queued",
                                note: None,
                            });
                            accepted += 1;
                        }
                        Ok(None) => {
                            rows.push(BulkInviteRow {
                                line: idx + 1,
                                email: email_norm,
                                status: "skipped",
                                note: Some("already in a team or no account".into()),
                            });
                            skipped += 1;
                        }
                        Err(e) => {
                            rows.push(BulkInviteRow {
                                line: idx + 1,
                                email: email_norm,
                                status: "error",
                                note: Some(format!("{}", e)),
                            });
                            skipped += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(Json(serde_json::json!({
        "mode": mode,
        "team_id": team_id,
        "total_rows": rows.len(),
        "accepted": accepted,
        "skipped": skipped,
        "rows": rows,
    }))
    .into_response())
}

pub async fn invite_member(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Json(payload): Json<InvitePayload>,
) -> Result<axum::response::Response, ApiError> {
    let email = validate_email(&payload.email)?;

    // E-P0-1 fix: rate-limit per-developer to stop spam abuse. The shared
    // `auth_rate_limiter` is 10 hits / 15 min by default — same cadence
    // login uses. Key by developer user_id, NOT IP (a single VPN-pool IP
    // shouldn't penalize legitimate users; spam is a per-actor problem).
    // Also key by email so retries hammering the same address get blocked
    // faster than spreading across recipients.
    let email_lower = email.to_lowercase();
    if state
        .auth_rate_limiter
        .check(&format!("invite:dev:{}", dev.user.id))
        .await
        .is_err()
    {
        tracing::warn!(
            dev_id = %dev.user.id,
            "Invite rate limit hit (per-developer)"
        );
        return Err(ApiError::TooManyRequests(
            "You've sent too many invitations recently. Please wait a few minutes before sending more.".into(),
        ));
    }
    if state
        .auth_rate_limiter
        .check(&format!("invite:email:{}", email_lower))
        .await
        .is_err()
    {
        tracing::warn!(
            dev_id = %dev.user.id,
            email_hash = %sha256_hex(&email_lower),
            "Invite rate limit hit (per-recipient-email)"
        );
        return Err(ApiError::TooManyRequests(
            "This email has been invited multiple times recently. Please wait before re-sending."
                .into(),
        ));
    }

    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    let result =
        crate::rewards::team_members::invite_by_email(&state.db, team_id, &email, dev.user.id)
            .await
            .map_err(ApiError::from)?; // D1: preserve variant + message

    // F11 fix: always return the SAME response shape regardless of whether
    // the underlying service actually created a membership row or silently
    // skipped (user not registered / already in a team). This denies the
    // developer the user-enumeration oracle they previously had.
    //
    // The dev-only preview_token only appears when a real membership got
    // created (so local testing of the accept-invitation flow still works).
    let (membership_id, token_opt) = match result {
        Some((id, token)) => (Some(id), Some(token)),
        None => (None, None),
    };
    let mut body = serde_json::json!({
        "status": "queued",
        "membership_id": membership_id,
        "message": "Invitation queued. If the email belongs to a registered POOOL user without an existing team membership, they will receive it.",
    });
    #[cfg(debug_assertions)]
    if let Some(token) = token_opt {
        body["preview_token"] = serde_json::Value::String(token);
        body["note"] = serde_json::Value::String(
            "Dev-build only: preview_token is NEVER returned in release builds.".into(),
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = token_opt; // silence unused-var warning in release
    Ok(Json(body).into_response())
}

pub async fn approve_member(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(membership_id): Path<Uuid>,
) -> Result<axum::response::Response, ApiError> {
    // Ownership-Check passiert in approve_pending intern via developer_user_id-Check.
    crate::rewards::team_members::approve_pending(&state.db, membership_id, dev.user.id)
        .await
        .map_err(ApiError::from)?; // D1
    Ok(Json(serde_json::json!({"status": "active"})).into_response())
}

/// Phase-1 fix: resend a fresh invitation token + email for a membership
/// currently in 'invited' status (e.g. token expired, member never received
/// the email). Rate-limited per-developer like the original invite to
/// prevent spam re-sends.
pub async fn resend_invitation(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(membership_id): Path<Uuid>,
) -> Result<axum::response::Response, ApiError> {
    // Per-dev rate limit (cheap reuse of existing limiter).
    if state
        .auth_rate_limiter
        .check(&format!("invite:dev:{}", dev.user.id))
        .await
        .is_err()
    {
        return Err(ApiError::TooManyRequests(
            "You've sent too many invitations recently. Please wait a few minutes.".into(),
        ));
    }

    // Verify membership exists in this dev's team + is in 'invited' state.
    let row = sqlx::query!(
        r#"SELECT m.team_id, m.user_id, t.developer_user_id
              FROM developer_team_memberships m
              JOIN developer_teams t ON t.id = m.team_id
             WHERE m.id = $1 AND m.status = 'invited'"#,
        membership_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("Invited membership not found.".into()))?;

    if row.developer_user_id != dev.user.id {
        return Err(ApiError::Forbidden("Not the owner of this team.".into()));
    }

    crate::rewards::team_members::resend_invitation(&state.db, membership_id, dev.user.id)
        .await
        .map_err(ApiError::from)?;

    Ok(
        Json(serde_json::json!({"status": "resent", "membership_id": membership_id}))
            .into_response(),
    )
}

pub async fn remove_member(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Path(membership_id): Path<Uuid>,
    Json(payload): Json<RemoveMemberPayload>,
) -> Result<axum::response::Response, ApiError> {
    // Ownership-Check inline
    let team_id: Option<Uuid> =
        sqlx::query_scalar("SELECT team_id FROM developer_team_memberships WHERE id = $1")
            .bind(membership_id)
            .fetch_optional(&state.db)
            .await
            .map_err(ApiError::Database)?;
    let team_id = team_id.ok_or_else(|| ApiError::NotFound("Membership not found".into()))?;
    require_team_owner(&state.db, team_id, dev.user.id).await?;

    let reason = payload.reason.as_deref().unwrap_or("removed_by_developer");
    crate::rewards::team_members::remove_member(&state.db, membership_id, dev.user.id, reason)
        .await
        .map_err(|e| match e {
            crate::error::AppError::NotFound(m) => ApiError::NotFound(m),
            other => ApiError::from(other), // D1
        })?;
    Ok(Json(serde_json::json!({"status": "removed"})).into_response())
}

pub async fn team_summary(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<DateRangeQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let from = parse_date_query(q.from, 30);
    let to = parse_date_query(q.to, 0);
    let summary = crate::rewards::team_reports::team_period_summary(&state.db, team_id, from, to)
        .await
        .map_err(ApiError::from)?; // D1 fix: preserve error type/message
    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to, "summary": summary
    }))
    .into_response())
}

pub async fn team_by_member(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<DateRangeQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let from = parse_date_query(q.from, 30);
    let to = parse_date_query(q.to, 0);
    let rows = crate::rewards::team_reports::team_period_by_member(&state.db, team_id, from, to)
        .await
        .map_err(ApiError::from)?; // D1
    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to, "rows": rows
    }))
    .into_response())
}

#[derive(Deserialize)]
pub struct CustomersTableQuery {
    pub attribution_user_id: Option<Uuid>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Comma-separated referral statuses (qualified, paid, under_holdback,
    /// registered, expired, disqualified). Empty → all.
    pub status: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

impl CustomersTableQuery {
    fn table(&self) -> TableQuery {
        TableQuery {
            q: self.q.clone(),
            sort: self.sort.clone(),
            dir: self.dir.clone(),
            limit: self.limit,
            offset: self.offset,
        }
    }
    fn status_filter(&self) -> Vec<&'static str> {
        let allowed = [
            "qualified",
            "paid",
            "under_holdback",
            "registered",
            "expired",
            "disqualified",
            "first_investment_done",
            "kyc_approved",
            "attributed",
        ];
        let raw = self.status.as_deref().unwrap_or("");
        if raw.trim().is_empty() {
            return allowed.to_vec();
        }
        raw.split(',')
            .map(|s| s.trim())
            .filter_map(|s| allowed.iter().find(|&&a| a == s).copied())
            .collect()
    }
}

pub async fn team_customers(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<CustomersTableQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let t = q.table();
    let limit = t.limit_clamped();
    let offset = t.offset_clamped();
    let search = t.q_lower();
    let attr_filter = q.attribution_user_id;
    let statuses = q.status_filter();
    if statuses.is_empty() {
        return Ok(Json(serde_json::json!({
            "team_id": team_id, "total": 0, "limit": limit, "offset": offset, "rows": []
        }))
        .into_response());
    }
    let status_in = statuses
        .iter()
        .map(|s| format!("'{}'", s))
        .collect::<Vec<_>>()
        .join(",");
    // Date range filter on attribution timestamp. Both optional.
    let from_date = q
        .from
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let to_date =
        q.to.as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let sort_expr = t.sort_sql(
        &[
            ("full_name", "full_name"),
            ("email", "ru.email"),
            ("via_member", "attribution_user_name"),
            ("status", "ar.status"),
            ("invested", "gross_invested_cents"),
            ("commission", "commission_earned_cents"),
            ("acquired", "ar.created_at"),
            ("last_activity", "last_activity_at"),
            ("n_purchases", "n_purchases"),
        ],
        "ar.created_at",
    );

    // Note: search placeholder number differs between count ($3) and rows ($5)
    // because count only binds (team_id, attr, search) while rows binds
    // (team_id, attr, limit, offset, search).
    let where_search_count = if search.is_some() {
        "AND (
            LOWER(COALESCE(rup.first_name,'') || ' ' || COALESCE(rup.last_name,'')) ILIKE $3
            OR LOWER(ru.email::text) ILIKE $3
            OR LOWER(COALESCE(aup.first_name,'') || ' ' || COALESCE(aup.last_name,'')) ILIKE $3
        )"
    } else {
        ""
    };
    let where_search_rows = if search.is_some() {
        "AND (
            LOWER(COALESCE(rup.first_name,'') || ' ' || COALESCE(rup.last_name,'')) ILIKE $5
            OR LOWER(ru.email::text) ILIKE $5
            OR LOWER(COALESCE(aup.first_name,'') || ' ' || COALESCE(aup.last_name,'')) ILIKE $5
        )"
    } else {
        ""
    };
    let where_status = format!(" AND ar.status IN ({})", status_in);
    // Date filters appended as fixed literals because dates were validated.
    let where_dates = match (from_date, to_date) {
        (Some(f), Some(t)) => format!(
            " AND ar.created_at >= '{}'::date AND ar.created_at < ('{}'::date + INTERVAL '1 day')",
            f, t
        ),
        (Some(f), None) => format!(" AND ar.created_at >= '{}'::date", f),
        (None, Some(t)) => format!(" AND ar.created_at < ('{}'::date + INTERVAL '1 day')", t),
        _ => String::new(),
    };

    let sql_total = format!(
        r#"SELECT COUNT(*)::BIGINT AS total
           FROM affiliate_referrals ar
           JOIN affiliate_links al ON al.id = ar.link_id
           LEFT JOIN users ru ON ru.id = ar.referred_user_id
           LEFT JOIN user_profiles rup ON rup.user_id = ar.referred_user_id
           LEFT JOIN user_profiles aup ON aup.user_id = ar.attribution_user_id
           WHERE al.team_id = $1
             AND al.link_type = 'team_business'
             AND ($2::uuid IS NULL OR ar.attribution_user_id = $2)
             {}{}{}"#,
        where_status, where_dates, where_search_count
    );

    let sql_rows = format!(
        r#"SELECT
              ar.referred_user_id,
              NULLIF(TRIM(BOTH ' ' FROM (
                  COALESCE(rup.first_name,'') || ' ' || COALESCE(rup.last_name,'')
              )), '') AS full_name,
              ru.email::text AS email,
              ar.attribution_user_id,
              NULLIF(TRIM(BOTH ' ' FROM (
                  COALESCE(aup.first_name,'') || ' ' || COALESCE(aup.last_name,'')
              )), '') AS attribution_user_name,
              ar.status AS referral_status,
              COALESCE(ar.created_at, NOW()) AS created_at,
              COALESCE(inv.sum_cents, 0)::BIGINT AS gross_invested_cents,
              COALESCE(com.sum_cents, 0)::BIGINT AS commission_earned_cents,
              -- Enrichment: number of orders + last activity timestamp.
              COALESCE(act.n_purchases, 0)::BIGINT AS n_purchases,
              act.last_activity_at
           FROM affiliate_referrals ar
           JOIN affiliate_links al ON al.id = ar.link_id
           LEFT JOIN users ru ON ru.id = ar.referred_user_id
           LEFT JOIN user_profiles rup ON rup.user_id = ar.referred_user_id
           LEFT JOIN user_profiles aup ON aup.user_id = ar.attribution_user_id
           LEFT JOIN LATERAL (
               SELECT SUM(i.purchase_value_cents) AS sum_cents
                 FROM investments i
                WHERE i.user_id = ar.referred_user_id AND i.status = 'active'
           ) inv ON TRUE
           LEFT JOIN LATERAL (
               SELECT SUM(ac.provisional_amount_cents) AS sum_cents
                 FROM affiliate_commissions ac
                WHERE ac.referral_id = ar.id
           ) com ON TRUE
           LEFT JOIN LATERAL (
               SELECT COUNT(o.id) AS n_purchases,
                      MAX(o.created_at) AS last_activity_at
                 FROM orders o
                WHERE o.user_id = ar.referred_user_id
                  AND o.status IN ('completed', 'approved')
           ) act ON TRUE
           WHERE al.team_id = $1
             AND al.link_type = 'team_business'
             AND ($2::uuid IS NULL OR ar.attribution_user_id = $2)
             {}{}{}
           ORDER BY {}
           LIMIT $3 OFFSET $4"#,
        where_status, where_dates, where_search_rows, sort_expr
    );

    let total: i64 = match &search {
        Some(s) => sqlx::query_scalar(&sql_total)
            .bind(team_id)
            .bind(attr_filter)
            .bind(format!("%{}%", s))
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?,
        None => sqlx::query_scalar(&sql_total)
            .bind(team_id)
            .bind(attr_filter)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?,
    };

    let rows: Vec<sqlx::postgres::PgRow> = if let Some(s) = &search {
        sqlx::query(&sql_rows)
            .bind(team_id)
            .bind(attr_filter)
            .bind(limit)
            .bind(offset)
            .bind(format!("%{}%", s))
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?
    } else {
        let sql_no_q = sql_rows.replace(where_search_rows, "");
        sqlx::query(&sql_no_q)
            .bind(team_id)
            .bind(attr_filter)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?
    };

    use sqlx::Row;
    let mapped: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "referred_user_id": r.get::<Uuid, _>("referred_user_id"),
                "full_name": r.get::<Option<String>, _>("full_name"),
                "email": r.get::<Option<String>, _>("email"),
                "attribution_user_id": r.get::<Uuid, _>("attribution_user_id"),
                "attribution_user_name": r.get::<Option<String>, _>("attribution_user_name"),
                "referral_status": r.get::<String, _>("referral_status"),
                "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "gross_invested_cents": r.get::<i64, _>("gross_invested_cents"),
                "commission_earned_cents": r.get::<i64, _>("commission_earned_cents"),
                "n_purchases": r.get::<i64, _>("n_purchases"),
                "last_activity_at": r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_activity_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "team_id": team_id,
        "total": total,
        "limit": limit,
        "offset": offset,
        "rows": mapped
    }))
    .into_response())
}

#[derive(Deserialize)]
pub struct ProductsTableQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub dir: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ProductsTableQuery {
    fn table(&self) -> TableQuery {
        TableQuery {
            q: self.q.clone(),
            sort: self.sort.clone(),
            dir: self.dir.clone(),
            limit: self.limit,
            offset: self.offset,
        }
    }
}

pub async fn team_products(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<ProductsTableQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let from = parse_date_query(q.from.clone(), 30);
    let to = parse_date_query(q.to.clone(), 0);
    let t = q.table();
    let limit = t.limit_clamped();
    let offset = t.offset_clamped();
    let search = t.q_lower();

    let sort_expr = t.sort_sql(
        &[
            ("asset_name", "asset_name"),
            ("units_sold", "units_sold"),
            ("gross_revenue", "gross_revenue_cents"),
            ("commission", "commission_cents"),
            ("n_buyers", "n_buyers"),
            ("last_sale", "last_sale_at"),
            ("avg_sale", "avg_sale_cents"),
        ],
        "gross_revenue_cents",
    );

    let where_search = if search.is_some() {
        " AND LOWER(a.title) ILIKE $5"
    } else {
        ""
    };

    let sql = format!(
        r#"WITH commission_orders AS (
              SELECT ac.id AS commission_id,
                     ac.source_order_id AS order_id,
                     ac.gross_amount_cents,
                     ac.provisional_amount_cents,
                     ac.created_at
                FROM affiliate_commissions ac
                JOIN affiliate_links al ON al.id = ac.link_id
               WHERE al.team_id = $1
                 AND al.link_type = 'team_business'
                 AND ac.created_at >= $2::date
                 AND ac.created_at <  ($3::date + INTERVAL '1 day')
           )
           SELECT
              oi.asset_id,
              a.title AS asset_name,
              COUNT(DISTINCT oi.id)::BIGINT AS units_sold,
              COALESCE(SUM(oi.tokens_quantity * oi.token_price_cents)::BIGINT, 0) AS gross_revenue_cents,
              COALESCE(SUM(co.provisional_amount_cents
                  * (oi.tokens_quantity * oi.token_price_cents)
                  / NULLIF(co.gross_amount_cents, 0)
              )::BIGINT, 0) AS commission_cents,
              COUNT(DISTINCT o.user_id)::BIGINT AS n_buyers,
              MAX(co.created_at) AS last_sale_at,
              CASE WHEN COUNT(DISTINCT oi.id) > 0
                   THEN (SUM(oi.tokens_quantity * oi.token_price_cents) / COUNT(DISTINCT oi.id))::BIGINT
                   ELSE 0
              END AS avg_sale_cents
           FROM commission_orders co
           JOIN order_items oi ON oi.order_id = co.order_id
           JOIN orders o      ON o.id = co.order_id
           LEFT JOIN assets a ON a.id = oi.asset_id
           WHERE TRUE{}
           GROUP BY oi.asset_id, a.title
           ORDER BY {}
           LIMIT $4 OFFSET {}"#,
        where_search,
        sort_expr,
        if search.is_some() { "$6" } else { "$5" }
    );

    let rows: Vec<sqlx::postgres::PgRow> = if let Some(s) = &search {
        sqlx::query(&sql)
            .bind(team_id)
            .bind(from)
            .bind(to)
            .bind(limit)
            .bind(format!("%{}%", s))
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?
    } else {
        sqlx::query(&sql)
            .bind(team_id)
            .bind(from)
            .bind(to)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Database)?
    };

    // Total count: distinct assets that match the period (+ search).
    let count_sql = format!(
        r#"SELECT COUNT(DISTINCT oi.asset_id)::BIGINT AS total
           FROM affiliate_commissions ac
           JOIN affiliate_links al ON al.id = ac.link_id
           JOIN order_items oi ON oi.order_id = ac.source_order_id
           LEFT JOIN assets a ON a.id = oi.asset_id
           WHERE al.team_id = $1
             AND al.link_type = 'team_business'
             AND ac.created_at >= $2::date
             AND ac.created_at <  ($3::date + INTERVAL '1 day'){}"#,
        if search.is_some() {
            " AND LOWER(a.title) ILIKE $4"
        } else {
            ""
        }
    );
    let total: i64 = match &search {
        Some(s) => sqlx::query_scalar(&count_sql)
            .bind(team_id)
            .bind(from)
            .bind(to)
            .bind(format!("%{}%", s))
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?,
        None => sqlx::query_scalar(&count_sql)
            .bind(team_id)
            .bind(from)
            .bind(to)
            .fetch_one(&state.db)
            .await
            .map_err(ApiError::Database)?,
    };

    use sqlx::Row;
    let mapped: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "asset_id": r.get::<Uuid, _>("asset_id"),
                "asset_name": r.get::<Option<String>, _>("asset_name"),
                "units_sold": r.get::<i64, _>("units_sold"),
                "gross_revenue_cents": r.get::<i64, _>("gross_revenue_cents"),
                "commission_cents": r.get::<i64, _>("commission_cents"),
                "n_buyers": r.get::<i64, _>("n_buyers"),
                "last_sale_at": r.get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_sale_at"),
                "avg_sale_cents": r.get::<i64, _>("avg_sale_cents"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to,
        "total": total, "limit": limit, "offset": offset,
        "rows": mapped
    }))
    .into_response())
}

/// GET /api/developer/affiliate/team/analytics/overview
/// Comprehensive analytics dashboard payload — period summary + lifetime
/// counters + top performers + deficit members + top assets + next payout.
pub async fn analytics_overview(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<DateRangeQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let from = parse_date_query(q.from, 30);
    let to = parse_date_query(q.to, 0);
    let overview =
        crate::rewards::team_reports::analytics_overview(&state.db, team_id, dev.user.id, from, to)
            .await
            .map_err(ApiError::from)?; // D1
    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to, "overview": overview
    }))
    .into_response())
}

/// GET /api/developer/affiliate/team/tier
/// Returns:
///   * current_tier, current_rate_bps, volume_12m_cents
///   * next_tier (next ladder step + threshold) — null if Sovereign
///   * progress_pct (0..100) toward next tier
///   * ladder: full 8-tier ladder so the frontend can render the progression
///   * developer_personal_tier / rate for side-by-side comparison
///   * recent_history (last 5 promotions from developer_team_tier_history)
pub async fn team_tier_info(
    dev: DeveloperUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    // F20 fix: debounce — only re-trigger recompute_team_tier if the cached
    // value is older than 5 minutes. Without this, every dashboard refresh
    // (every browser tab) takes a row-lock + recomputes from scratch.
    // The background tier-progression worker runs every 24h and keeps it
    // fresh for the slow-changing case; on-demand recompute now only kicks
    // in if the user is staring at a stale tile.
    let stale: bool = sqlx::query_scalar(
        r#"SELECT team_tier_updated_at < NOW() - INTERVAL '5 minutes'
           FROM developer_teams WHERE id = $1"#,
    )
    .bind(team_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(true);
    if stale {
        let _ = sqlx::query!(r#"SELECT * FROM recompute_team_tier($1::uuid)"#, team_id)
            .fetch_optional(&state.db)
            .await;
    }

    let team = sqlx::query!(
        r#"SELECT current_team_tier, team_commission_rate_bps,
                  team_volume_12m_cents, team_tier_updated_at
           FROM developer_teams WHERE id = $1"#,
        team_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Developer's personal tier (for side-by-side compare). Optional —
    // a developer might not have an `affiliates` row.
    let personal = sqlx::query!(
        r#"SELECT current_tier, commission_rate_bps FROM affiliates WHERE user_id = $1"#,
        dev.user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Full ladder for UI rendering.
    let ladder = sqlx::query!(
        r#"SELECT name, commission_rate_bps, min_volume_cents, sort_order
           FROM affiliate_tiers ORDER BY sort_order ASC"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    // Compute next-tier + progress percentage toward it.
    let current_min = ladder
        .iter()
        .find(|t| t.name == team.current_team_tier)
        .map(|t| t.min_volume_cents)
        .unwrap_or(0);
    let next = ladder
        .iter()
        .find(|t| t.min_volume_cents > team.team_volume_12m_cents);
    let (next_name, next_threshold, progress_pct) = match next {
        Some(n) => {
            let span = (n.min_volume_cents - current_min).max(1);
            let into = (team.team_volume_12m_cents - current_min).max(0);
            let pct = ((into as f64 / span as f64) * 100.0).clamp(0.0, 100.0);
            (Some(n.name.clone()), Some(n.min_volume_cents), pct)
        }
        None => (None, None, 100.0),
    };

    // Recent promotion history
    let history = sqlx::query!(
        r#"SELECT old_tier, new_tier, old_bps, new_bps, volume_cents, changed_at
           FROM developer_team_tier_history
           WHERE team_id = $1
           ORDER BY changed_at DESC LIMIT 5"#,
        team_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let ladder_json: Vec<_> = ladder
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "commission_rate_bps": t.commission_rate_bps,
                "min_volume_cents": t.min_volume_cents,
                "sort_order": t.sort_order,
                "is_current": t.name == team.current_team_tier,
            })
        })
        .collect();

    let history_json: Vec<_> = history
        .iter()
        .map(|h| {
            serde_json::json!({
                "old_tier": h.old_tier,
                "new_tier": h.new_tier,
                "old_bps": h.old_bps,
                "new_bps": h.new_bps,
                "volume_cents": h.volume_cents,
                "changed_at": h.changed_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "team_id": team_id,
        "current_tier": team.current_team_tier,
        "current_rate_bps": team.team_commission_rate_bps,
        "volume_12m_cents": team.team_volume_12m_cents,
        "next_tier": next_name,
        "next_threshold_cents": next_threshold,
        "progress_pct": progress_pct,
        "tier_updated_at": team.team_tier_updated_at,
        "developer_personal_tier": personal.as_ref().and_then(|p| p.current_tier.clone()),
        "developer_personal_rate_bps": personal.as_ref().and_then(|p| p.commission_rate_bps),
        "ladder": ladder_json,
        "history": history_json,
    }))
    .into_response())
}

/// GET /api/developer/affiliate/team/analytics/timeseries
/// Daily series for charts — one bucket per day (gap-filled).
pub async fn analytics_timeseries(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<DateRangeQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let from = parse_date_query(q.from, 30);
    let to = parse_date_query(q.to, 0);
    let series = crate::rewards::team_reports::team_daily_timeseries(&state.db, team_id, from, to)
        .await
        .map_err(ApiError::from)?; // D1
    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to, "series": series
    }))
    .into_response())
}

// ── Member Self-Service Endpoints ───────────────────────────────────────────

pub async fn accept_invitation(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<AcceptInvitationPayload>,
) -> Result<axum::response::Response, ApiError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".into()))?;
    crate::rewards::team_members::accept_invitation(&state.db, user.id, &payload.token)
        .await
        .map_err(ApiError::from)?; // D1
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"status": "active"})),
    )
        .into_response())
}

pub async fn self_request_join(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<SelfRequestPayload>,
) -> Result<axum::response::Response, ApiError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".into()))?;
    let id = crate::rewards::team_members::self_request_join(
        &state.db,
        user.id,
        &payload.developer_slug,
    )
    .await
    .map_err(ApiError::from)?; // D1
    Ok(
        Json(serde_json::json!({"membership_id": id, "status": "pending_developer_approval"}))
            .into_response(),
    )
}

/// GET /api/developer/affiliate/team/analytics/cohort?months=12
///
/// Phase-4: cohort retention matrix. Each row is one (cohort_month,
/// period_index) cell — frontend renders a triangular heatmap.
pub async fn analytics_cohort(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let months = q
        .get("months")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(12);
    let cells = crate::rewards::team_reports::team_cohort_retention(&state.db, team_id, months)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({
        "team_id": team_id,
        "months_back": months,
        "cells": cells,
    }))
    .into_response())
}

/// GET /api/developer/affiliate/team/analytics/forecast
///
/// Phase-4: simple linear forecast (trailing-30-day average × 30) of the
/// team's next-month commission revenue. Honest about the method so the
/// frontend can label the chart line "Projection (trailing 30-day avg)".
pub async fn analytics_forecast(
    dev: DeveloperUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let f = crate::rewards::team_reports::team_revenue_forecast(&state.db, team_id)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({
        "team_id": team_id,
        "forecast": f,
    }))
    .into_response())
}

/// GET /api/affiliate/team/invitation-preview?token=XXX
///
/// **Public** (no auth required). The invite link in the email lands on the
/// `/affiliate/team/accept` page which calls this to render the team name +
/// inviter name BEFORE prompting the user to log in / accept.
///
/// Returns 404 (uniform "Invitation not found or expired.") when the token
/// is unknown, already used, or expired — never leaks existence of the
/// underlying team. Rate-limited per-IP to slow down enumeration attacks.
#[derive(Deserialize)]
pub struct InvitationPreviewQuery {
    pub token: String,
}

pub async fn invitation_preview(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(q): Query<InvitationPreviewQuery>,
) -> Result<axum::response::Response, ApiError> {
    // ── IP-based rate limit (5 lookups / 15 min) — token is unauthenticated
    //    so a leaked URL or enumeration sweep is the threat model.
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').last())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "0.0.0.0".to_string());
    if state
        .auth_rate_limiter
        .check(&format!("invite_preview:{}", ip))
        .await
        .is_err()
    {
        return Err(ApiError::TooManyRequests(
            "Too many invitation lookups. Please wait a moment.".into(),
        ));
    }

    let token = q.token.trim();
    if token.is_empty() || token.len() > 200 {
        return Err(ApiError::NotFound(
            "Invitation not found or expired.".into(),
        ));
    }
    let token_hash = crate::rewards::team_members::hash_token(token);

    // Lookup invited (or pending) membership by token-hash. Joining users +
    // teams so we can render `team_name` and `inviter_name`. Status must be
    // 'invited' (the only state where the token-hash is non-NULL).
    let row = sqlx::query!(
        r#"SELECT m.invitation_expires_at,
                  t.display_name AS team_name,
                  COALESCE(NULLIF(TRIM(u.email), ''), 'the team owner') AS "inviter_name!"
           FROM developer_team_memberships m
           JOIN developer_teams t ON t.id = m.team_id
           LEFT JOIN users u ON u.id = t.developer_user_id
           WHERE m.invitation_token_hash = $1
             AND m.status = 'invited'
           LIMIT 1"#,
        token_hash
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let r = row.ok_or_else(|| ApiError::NotFound("Invitation not found or expired.".into()))?;

    let expired = match r.invitation_expires_at {
        Some(exp) => exp < chrono::Utc::now(),
        None => false,
    };
    if expired {
        return Err(ApiError::NotFound(
            "Invitation not found or expired.".into(),
        ));
    }

    // Sanitize inviter — drop the domain so the team owner's full email isn't
    // exposed to an unauthenticated visitor (a leaked URL could otherwise
    // dox the owner). Keep the local part as a courtesy.
    let inviter_display = r
        .inviter_name
        .split('@')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("the team owner")
        .to_string();

    Ok(Json(serde_json::json!({
        "team_name": r.team_name,
        "inviter_name": inviter_display,
        "expires_at": r.invitation_expires_at,
    }))
    .into_response())
}

pub async fn my_membership(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| ApiError::Unauthorized("Authentication required".into()))?;

    let row = sqlx::query!(
        r#"SELECT m.id, m.team_id, m.status, m.role, m.joined_at,
                  t.display_name AS team_name, t.developer_user_id,
                  al.code        AS "business_link_code?"
           FROM developer_team_memberships m
           JOIN developer_teams t ON t.id = m.team_id
           LEFT JOIN affiliate_links al ON al.team_id = m.team_id
                                       AND al.attribution_user_id = m.user_id
                                       AND al.link_type = 'team_business'
                                       AND al.status = 'active'
           WHERE m.user_id = $1
             AND m.status IN ('invited', 'pending_developer_approval', 'active')
           LIMIT 1"#,
        user.id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    match row {
        Some(r) => Ok(Json(serde_json::json!({
            "membership_id": r.id,
            "team_id": r.team_id,
            "team_name": r.team_name,
            "developer_user_id": r.developer_user_id,
            "status": r.status,
            "role": r.role,
            "joined_at": r.joined_at,
            "business_link_code": r.business_link_code,
        }))
        .into_response()),
        None => Ok(Json(serde_json::json!({"status": "none"})).into_response()),
    }
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        // Developer endpoints — gated via DeveloperUser extractor inside each handler
        .route(
            "/api/developer/affiliate/team",
            get(get_team_info).patch(update_team),
        )
        .route(
            "/api/developer/affiliate/team/members",
            get(list_team_members),
        )
        .route("/api/developer/affiliate/team/invite", post(invite_member))
        // Phase-7: bulk-invite from CSV (dry-run + commit modes)
        .route(
            "/api/developer/affiliate/team/invite-bulk",
            post(invite_member_bulk),
        )
        .route(
            "/api/developer/affiliate/team/members/:id/approve",
            post(approve_member),
        )
        .route(
            "/api/developer/affiliate/team/members/:id/remove",
            post(remove_member),
        )
        .route(
            "/api/developer/affiliate/team/members/:id/resend-invitation",
            post(resend_invitation),
        )
        .route("/api/developer/affiliate/team/summary", get(team_summary))
        .route(
            "/api/developer/affiliate/team/by-member",
            get(team_by_member),
        )
        .route(
            "/api/developer/affiliate/team/customers",
            get(team_customers),
        )
        .route("/api/developer/affiliate/team/products", get(team_products))
        .route(
            "/api/developer/affiliate/team/analytics/overview",
            get(analytics_overview),
        )
        .route(
            "/api/developer/affiliate/team/analytics/timeseries",
            get(analytics_timeseries),
        )
        // Phase-4: cohort retention + forecast analytics
        .route(
            "/api/developer/affiliate/team/analytics/cohort",
            get(analytics_cohort),
        )
        .route(
            "/api/developer/affiliate/team/analytics/forecast",
            get(analytics_forecast),
        )
        .route("/api/developer/affiliate/team/tier", get(team_tier_info))
        // Member self-service endpoints
        .route(
            "/api/affiliate/team/accept-invitation",
            post(accept_invitation),
        )
        .route("/api/affiliate/team/self-request", post(self_request_join))
        .route("/api/affiliate/team/my-membership", get(my_membership))
        // Public preview — invite-accept landing renders this BEFORE auth.
        .route(
            "/api/affiliate/team/invitation-preview",
            get(invitation_preview),
        )
}
