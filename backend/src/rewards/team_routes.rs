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
use sqlx::PgPool;
use uuid::Uuid;

// ── Helpers ─────────────────────────────────────────────────────────────────

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
        r#"SELECT id, display_name, public_slug, status, created_at
           FROM developer_teams WHERE id = $1"#,
        team_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Database)?;

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
        }))
    }))
    .into_response())
}

pub async fn update_team(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateTeamPayload>,
) -> Result<axum::response::Response, ApiError> {
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
    }))
    .execute(&state.db)
    .await;

    Ok(Json(serde_json::json!({"status": "updated", "team_id": team_id})).into_response())
}

pub async fn list_team_members(
    dev: DeveloperUser,
    State(state): State<AppState>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    let rows = sqlx::query!(
        r#"SELECT m.id, m.user_id, m.role, m.status, m.invited_at, m.joined_at,
                  u.email::text                       AS email,
                  NULLIF(TRIM(BOTH ' ' FROM (
                      COALESCE(up.first_name, '') || ' ' || COALESCE(up.last_name, '')
                  )), '')                              AS full_name,
                  al.id                                AS link_id,
                  al.code                              AS link_code
           FROM developer_team_memberships m
           LEFT JOIN users         u  ON u.id = m.user_id
           LEFT JOIN user_profiles up ON up.user_id = m.user_id
           LEFT JOIN affiliate_links al ON al.team_id = m.team_id
                                       AND al.attribution_user_id = m.user_id
                                       AND al.link_type = 'team_business'
                                       AND al.status = 'active'
           WHERE m.team_id = $1
             AND m.status IN ('invited', 'pending_developer_approval', 'active')
           ORDER BY m.status DESC, m.created_at DESC"#,
        team_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let members: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "membership_id": r.id,
                "user_id": r.user_id,
                "email": r.email,
                "full_name": r.full_name,
                "role": r.role,
                "status": r.status,
                "invited_at": r.invited_at,
                "joined_at": r.joined_at,
                "link_id": r.link_id,
                "link_code": r.link_code,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "team_id": team_id, "members": members })).into_response())
}

pub async fn invite_member(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Json(payload): Json<InvitePayload>,
) -> Result<axum::response::Response, ApiError> {
    let email = validate_email(&payload.email)?;
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;

    let (membership_id, token) =
        crate::rewards::team_members::invite_by_email(&state.db, team_id, &email, dev.user.id)
            .await
            .map_err(|e| match e {
                crate::error::AppError::BadRequest(m) => ApiError::BadRequest(m),
                crate::error::AppError::Conflict(m) => ApiError::Conflict(m),
                _ => ApiError::Internal("invite failed".into()),
            })?;

    // Token leakage guard: nur in dev-builds geben wir den plain-Token zurück
    // als Convenience für lokales Testing. In Release-Builds bekommt nur das
    // Email-Outbox-System den Token zu sehen — auch der Developer hier nicht.
    let mut body = serde_json::json!({
        "membership_id": membership_id,
        "status": "invited",
    });
    #[cfg(debug_assertions)]
    {
        body["preview_token"] = serde_json::Value::String(token);
        body["note"] = serde_json::Value::String(
            "Dev-build only: preview_token is NEVER returned in release builds.".into(),
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = token; // silence unused-var warning in release
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
        .map_err(|e| match e {
            crate::error::AppError::NotFound(m) => ApiError::NotFound(m),
            crate::error::AppError::Forbidden(m) => ApiError::Forbidden(m),
            crate::error::AppError::BadRequest(m) => ApiError::BadRequest(m),
            _ => ApiError::Internal("approve failed".into()),
        })?;
    Ok(Json(serde_json::json!({"status": "active"})).into_response())
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
            _ => ApiError::Internal("remove failed".into()),
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
        .map_err(|_| ApiError::Internal("summary failed".into()))?;
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
        .map_err(|_| ApiError::Internal("by_member failed".into()))?;
    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to, "rows": rows
    }))
    .into_response())
}

pub async fn team_customers(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<CustomersQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let rows = crate::rewards::team_reports::team_customers(
        &state.db,
        team_id,
        q.attribution_user_id,
        q.limit.unwrap_or(50).clamp(1, 200),
        q.offset.unwrap_or(0).max(0),
    )
    .await
    .map_err(|_| ApiError::Internal("customers failed".into()))?;
    Ok(Json(serde_json::json!({ "team_id": team_id, "rows": rows })).into_response())
}

pub async fn team_products(
    dev: DeveloperUser,
    State(state): State<AppState>,
    Query(q): Query<ProductsQuery>,
) -> Result<axum::response::Response, ApiError> {
    let team_id = get_or_create_default_team(&state.db, dev.user.id).await?;
    let from = parse_date_query(q.from, 30);
    let to = parse_date_query(q.to, 0);
    let rows = crate::rewards::team_reports::team_products(&state.db, team_id, from, to)
        .await
        .map_err(|_| ApiError::Internal("products failed".into()))?;
    Ok(Json(serde_json::json!({
        "team_id": team_id, "from": from, "to": to, "rows": rows
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
        .map_err(|e| match e {
            crate::error::AppError::NotFound(m) => ApiError::NotFound(m),
            crate::error::AppError::BadRequest(m) => ApiError::BadRequest(m),
            _ => ApiError::Internal("accept failed".into()),
        })?;
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
    .map_err(|e| match e {
        crate::error::AppError::NotFound(m) => ApiError::NotFound(m),
        crate::error::AppError::BadRequest(m) => ApiError::BadRequest(m),
        crate::error::AppError::Conflict(m) => ApiError::Conflict(m),
        _ => ApiError::Internal("self-request failed".into()),
    })?;
    Ok(
        Json(serde_json::json!({"membership_id": id, "status": "pending_developer_approval"}))
            .into_response(),
    )
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
                  al.code        AS business_link_code
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
        .route(
            "/api/developer/affiliate/team/members/:id/approve",
            post(approve_member),
        )
        .route(
            "/api/developer/affiliate/team/members/:id/remove",
            post(remove_member),
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
        // Member self-service endpoints
        .route(
            "/api/affiliate/team/accept-invitation",
            post(accept_invitation),
        )
        .route("/api/affiliate/team/self-request", post(self_request_join))
        .route("/api/affiliate/team/my-membership", get(my_membership))
}
