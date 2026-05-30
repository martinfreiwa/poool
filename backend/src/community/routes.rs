use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Redirect, Response},
    routing::{delete, get, post, put},
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::middleware,
    auth::routes::AppState,
    community::{models, models::*, service, user_bridge, validation},
    error::AppError,
};

#[derive(Deserialize)]
pub struct FeedQuery {
    pub category: Option<String>,
    pub page: Option<i64>,
    pub feed_mode: Option<String>,
    pub sort_by: Option<String>,
    pub circle_id: Option<Uuid>,
    pub post_type: Option<String>,
    pub tag: Option<String>,
    // Phase 2 task 15: when "bookmarks", the HTMX feed partial returns the
    // viewer's bookmarked posts instead of the global feed. Any other value
    // is treated as the default global feed.
    pub source: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateAnnouncementReq {
    pub content: String,
    pub category: String,
    pub image_urls: Option<Vec<String>>,
    pub is_pinned: Option<bool>,
}

const ANNOUNCEMENT_CATEGORIES: &[&str] = &[
    "new_commodity",
    "dividend",
    "platform_update",
    "market_news",
    "farm_update",
];

const ADMIN_CIRCLE_DEFAULT_LIMIT: i64 = 50;
const ADMIN_CIRCLE_MAX_LIMIT: i64 = 100;
const ADMIN_CIRCLE_OPS_ALERT_DEFAULT_LIMIT: i64 = 50;
const ADMIN_CIRCLE_OPS_ALERT_MAX_LIMIT: i64 = 100;
const ADMIN_COMMENTS_DEFAULT_LIMIT: i64 = 200;
const ADMIN_COMMENTS_MAX_LIMIT: i64 = 200;
const MAX_POST_TAGS: usize = 8;

const COMMUNITY_POST_TYPES: &[&str] = &[
    "general",
    "discussion",
    "question",
    "market_insight",
    "property_update",
    "due_diligence",
    "poll",
    "announcement",
    "ama_question",
    "resource",
    "risk_discussion",
    "official_update",
    // Legacy values remain accepted for existing API/back-office flows.
    "milestone",
    "farm_update",
    "review",
];

const OFFICIAL_ONLY_POST_TYPES: &[&str] = &["announcement", "official_update"];

const PRIVILEGED_POST_TAGS: &[&str] = &["official", "featured", "answered"];

const COMMUNITY_POST_TAGS: &[&str] = &[
    "market_insight",
    "question",
    "risk",
    "yield",
    "real_estate",
    "commodity",
    "bali",
    "cocoa",
    "tokenization",
    "property_update",
    "beginner",
    "advanced",
    "official",
    "answered",
    "featured",
    "due_diligence",
    "legal",
    "tax",
    "liquidity",
];

const QA_POST_TYPES: &[&str] = &["question", "due_diligence"];
const QA_STATUSES: &[&str] = &[
    "open",
    "answered",
    "official_answer",
    "needs_clarification",
    "archived",
];
const MAX_CIRCLE_RESOURCE_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

fn validate_announcement_category(category: &str) -> Result<(), AppError> {
    if ANNOUNCEMENT_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "Invalid announcement category.".to_string(),
        ))
    }
}

/// Per-user rate-limit gate for community mutating endpoints
/// (circle join/leave/invite/promote/ban). Returns BadRequest with a
/// human-readable retry hint when exhausted.
///
/// Scoped by `bucket` so different operation classes share a quota with
/// themselves but not with each other (e.g. heavy invite spam doesn't
/// block a single join). Bucket keys are interned strings, never user
/// input — `bucket` MUST be a static literal.
async fn require_community_rate_limit(
    state: &AppState,
    user_id: Uuid,
    bucket: &'static str,
) -> Result<(), AppError> {
    let key = format!("cm:{}:{}", bucket, user_id);
    match state.community_rate_limiter.check(&key).await {
        Ok(_) => Ok(()),
        Err(retry_after_secs) => Err(AppError::BadRequest(format!(
            "Too many community actions. Try again in {} seconds.",
            retry_after_secs
        ))),
    }
}

fn require_csrf_header(headers: &HeaderMap, jar: &CookieJar) -> Result<(), AppError> {
    let cookie_token = jar
        .get("csrf_token")
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_default();
    let header_token = headers
        .get("X-CSRF-Token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !cookie_token.is_empty() && header_token == cookie_token {
        Ok(())
    } else {
        Err(AppError::Forbidden("CSRF token validation failed".into()))
    }
}

fn is_circle_manager_role(role: Option<&str>) -> bool {
    matches!(role, Some("owner" | "admin" | "moderator"))
}

fn is_circle_qa_responder_role(role: Option<&str>) -> bool {
    is_circle_manager_role(role) || matches!(role, Some("verified_expert"))
}

fn canonical_community_code(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_separator = false;

    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if matches!(ch, ' ' | '_' | '-' | '/' | '.')
            && !out.is_empty()
            && !last_was_separator
        {
            out.push('_');
            last_was_separator = true;
        }
    }

    while out.ends_with('_') {
        out.pop();
    }

    out
}

fn normalize_post_type(post_type: &str) -> Result<String, AppError> {
    let normalized = canonical_community_code(post_type);
    if COMMUNITY_POST_TYPES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(AppError::BadRequest(
            "Invalid post type for community content.".to_string(),
        ))
    }
}

fn normalize_post_tags(tags: Option<Vec<String>>) -> Result<Vec<String>, AppError> {
    let mut normalized = Vec::new();

    for tag in tags.unwrap_or_default() {
        let code = canonical_community_code(&tag);
        if code.is_empty() {
            continue;
        }
        if !COMMUNITY_POST_TAGS.contains(&code.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid post tag: {}",
                tag.trim()
            )));
        }
        if !normalized.contains(&code) {
            normalized.push(code);
        }
        if normalized.len() > MAX_POST_TAGS {
            return Err(AppError::BadRequest(format!(
                "Posts can have at most {} tags.",
                MAX_POST_TAGS
            )));
        }
    }

    Ok(normalized)
}

fn normalize_qa_status(status: &str) -> Result<String, AppError> {
    let normalized = canonical_community_code(status);
    if QA_STATUSES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(AppError::BadRequest("Invalid Q&A status.".to_string()))
    }
}

fn is_qa_post_type(post_type: &str) -> bool {
    QA_POST_TYPES.contains(&post_type)
}

async fn user_can_manage_qa_post(
    state: &AppState,
    pool: &sqlx::PgPool,
    user_id: Uuid,
    post_id: Uuid,
) -> Result<bool, AppError> {
    if middleware::has_permission(&state.db, user_id, "community.manage").await {
        return Ok(true);
    }

    let circle_id: Option<Uuid> = sqlx::query_scalar("SELECT circle_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    let Some(circle_id) = circle_id else {
        return Ok(false);
    };

    let role = get_circle_member_role(pool, circle_id, user_id).await?;
    Ok(is_circle_qa_responder_role(role.as_deref()))
}

async fn user_can_publish_privileged_circle_content(
    state: &AppState,
    pool: &sqlx::PgPool,
    user_id: Uuid,
    circle_id: Option<Uuid>,
) -> Result<bool, AppError> {
    if middleware::has_permission(&state.db, user_id, "community.manage").await {
        return Ok(true);
    }

    if let Some(circle_id) = circle_id {
        let role = get_circle_member_role(pool, circle_id, user_id).await?;
        return Ok(is_circle_manager_role(role.as_deref()));
    }

    Ok(false)
}

async fn ensure_post_taxonomy_allowed(
    state: &AppState,
    pool: &sqlx::PgPool,
    user_id: Uuid,
    circle_id: Option<Uuid>,
    post_type: &str,
    content_tags: &[String],
) -> Result<(), AppError> {
    let has_privileged_publish_access =
        user_can_publish_privileged_circle_content(state, pool, user_id, circle_id).await?;

    if OFFICIAL_ONLY_POST_TYPES.contains(&post_type) && !has_privileged_publish_access {
        return Err(AppError::Forbidden(
            "Only Circle moderators or platform admins can publish this post type.".into(),
        ));
    }

    if content_tags
        .iter()
        .any(|tag| PRIVILEGED_POST_TAGS.contains(&tag.as_str()))
        && !has_privileged_publish_access
    {
        return Err(AppError::Forbidden(
            "Only Circle moderators or platform admins can apply official post tags.".into(),
        ));
    }

    let Some(circle_id) = circle_id else {
        return Ok(());
    };

    let (required_tags, allowed_post_types): (Vec<String>, Vec<String>) = sqlx::query_as(
        "SELECT COALESCE(required_post_tags, '{}'::TEXT[]),
                COALESCE(allowed_post_types, '{}'::TEXT[])
         FROM circles
         WHERE id = $1",
    )
    .bind(circle_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    let allowed_post_types: Vec<String> = allowed_post_types
        .iter()
        .map(|item| canonical_community_code(item))
        .filter(|item| !item.is_empty())
        .collect();
    if !allowed_post_types.is_empty() && !allowed_post_types.iter().any(|item| item == post_type) {
        return Err(AppError::Forbidden(
            "This Circle does not allow that post type.".into(),
        ));
    }

    let missing_required_tag = required_tags
        .iter()
        .map(|tag| canonical_community_code(tag))
        .filter(|tag| !tag.is_empty())
        .find(|tag| !content_tags.contains(tag));
    if let Some(tag) = missing_required_tag {
        return Err(AppError::BadRequest(format!(
            "This Circle requires the '{}' tag.",
            tag
        )));
    }

    Ok(())
}

async fn get_circle_member_role(
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Uuid,
) -> Result<Option<String>, AppError> {
    let role =
        sqlx::query_scalar("SELECT role FROM circle_members WHERE circle_id = $1 AND user_id = $2")
            .bind(circle_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    Ok(role)
}

async fn ensure_circle_manage_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Uuid,
) -> Result<String, AppError> {
    if middleware::has_permission(&state.db, user_id, "community.manage").await {
        return Ok("platform_admin".to_string());
    }

    let role = get_circle_member_role(pool, circle_id, user_id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Manage access requires Circle membership.".into()))?;
    if is_circle_manager_role(Some(role.as_str())) {
        Ok(role)
    } else {
        Err(AppError::Forbidden(
            "Only Circle owners, admins, and moderators can manage this Circle.".into(),
        ))
    }
}

async fn is_circle_banned(
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Uuid,
) -> Result<bool, AppError> {
    let banned = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS(
             SELECT 1 FROM circle_bans
             WHERE circle_id = $1
               AND banned_user_id = $2
               AND (expires_at IS NULL OR expires_at > NOW())
           )"#,
    )
    .bind(circle_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(banned)
}

pub async fn ensure_circle_read_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<(), AppError> {
    let row = sqlx::query(
        "SELECT is_public, visibility, token_gate_asset_id, related_asset_id, kyc_required, join_policy
         FROM circles WHERE id = $1",
    )
    .bind(circle_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;
    use sqlx::Row;
    let is_public: bool = row.try_get("is_public").unwrap_or(false);
    let visibility: String = row.try_get("visibility").unwrap_or_else(|_| {
        if is_public {
            "public".into()
        } else {
            "private".into()
        }
    });
    let token_gate_asset_id: Option<Uuid> = row.try_get("token_gate_asset_id").ok().flatten();
    let related_asset_id: Option<Uuid> = row.try_get("related_asset_id").ok().flatten();
    let kyc_required: bool = row.try_get("kyc_required").unwrap_or(false);
    let join_policy: String = row
        .try_get("join_policy")
        .unwrap_or_else(|_| "open".to_string());
    let requires_holding = token_gate_asset_id.is_some() || join_policy == "holder_only";
    let is_gated = token_gate_asset_id.is_some()
        || kyc_required
        || matches!(join_policy.as_str(), "holder_only" | "kyc_required");

    if is_public && visibility == "public" && !is_gated {
        return Ok(());
    }

    let user_id = user_id.ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    if is_circle_banned(pool, circle_id, user_id).await? {
        return Err(AppError::Forbidden("You cannot access this Circle.".into()));
    }

    if middleware::has_permission(&state.db, user_id, "community.manage").await {
        return Ok(());
    }

    let has_required_holding = if requires_holding {
        if token_gate_asset_id.is_some() {
            crate::community::circles::check_token_gate(pool, &state.db, user_id, circle_id)
                .await?;
            true
        } else if let Some(asset_id) = related_asset_id {
            user_has_asset_holding(&state.db, user_id, asset_id).await?
        } else {
            return Err(AppError::Forbidden(
                "This Circle requires a related asset holding.".into(),
            ));
        }
    } else {
        false
    };

    if requires_holding && !has_required_holding {
        return Err(AppError::Forbidden(
            "You must hold the related asset to view this Circle.".into(),
        ));
    }

    if get_circle_member_role(pool, circle_id, user_id)
        .await?
        .is_some()
        || has_required_holding
    {
        Ok(())
    } else if visibility == "hidden"
        && crate::community::circles::has_pending_invite(pool, circle_id, user_id).await?
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You must be a member to view this Circle.".into(),
        ))
    }
}

async fn ensure_circle_write_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Uuid,
) -> Result<String, AppError> {
    ensure_circle_read_access(state, pool, circle_id, Some(user_id)).await?;

    if is_circle_banned(pool, circle_id, user_id).await? {
        return Err(AppError::Forbidden(
            "You cannot post in this Circle.".into(),
        ));
    }

    let role = get_circle_member_role(pool, circle_id, user_id)
        .await?
        .ok_or_else(|| AppError::Forbidden("Join this Circle before posting.".into()))?;

    Ok(role)
}

async fn ensure_post_read_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    post_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<Option<Uuid>, AppError> {
    let circle_id: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM posts WHERE id = $1 AND is_hidden = false")
            .bind(post_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    if let Some(circle_id) = circle_id {
        ensure_circle_read_access(state, pool, circle_id, user_id).await?;
    }

    Ok(circle_id)
}

async fn ensure_post_write_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    post_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    if let Some(circle_id) = ensure_post_read_access(state, pool, post_id, Some(user_id)).await? {
        ensure_circle_write_access(state, pool, circle_id, user_id).await?;
    }
    Ok(())
}

async fn ensure_ama_read_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    ama_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    let circle_id: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM amas WHERE id = $1 AND status != 'draft'")
            .bind(ama_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("AMA not found".into()))?;

    if let Some(circle_id) = circle_id {
        ensure_circle_read_access(state, pool, circle_id, Some(user_id)).await?;
    }

    Ok(circle_id)
}

fn normalize_circle_onboarding_step(step: &str) -> Result<&'static str, AppError> {
    let normalized = canonical_community_code(step);
    CIRCLE_ONBOARDING_STEPS
        .iter()
        .copied()
        .find(|candidate| *candidate == normalized)
        .ok_or_else(|| AppError::BadRequest("Invalid Circle onboarding step.".to_string()))
}

async fn mark_circle_onboarding_step(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    circle_id: Uuid,
    step: &str,
) -> Result<(), AppError> {
    let step = normalize_circle_onboarding_step(step)?;
    sqlx::query(
        r#"
        INSERT INTO circle_onboarding_progress (circle_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (circle_id, user_id) DO NOTHING
        "#,
    )
    .bind(circle_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    let update_sql = match step {
        "rules_read" => {
            "UPDATE circle_onboarding_progress SET rules_read = TRUE, updated_at = NOW() WHERE circle_id = $1 AND user_id = $2"
        }
        "introduced_self" => {
            "UPDATE circle_onboarding_progress SET introduced_self = TRUE, updated_at = NOW() WHERE circle_id = $1 AND user_id = $2"
        }
        "interests_selected" => {
            "UPDATE circle_onboarding_progress SET interests_selected = TRUE, updated_at = NOW() WHERE circle_id = $1 AND user_id = $2"
        }
        "ama_followed" => {
            "UPDATE circle_onboarding_progress SET ama_followed = TRUE, updated_at = NOW() WHERE circle_id = $1 AND user_id = $2"
        }
        "first_question_posted" => {
            "UPDATE circle_onboarding_progress SET first_question_posted = TRUE, updated_at = NOW() WHERE circle_id = $1 AND user_id = $2"
        }
        _ => unreachable!("step allowlist is enforced above"),
    };

    sqlx::query(update_sql)
        .bind(circle_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        UPDATE circle_onboarding_progress
        SET is_completed = (
                rules_read
            AND introduced_self
            AND interests_selected
            AND ama_followed
            AND first_question_posted
          ),
          completed_at = CASE
            WHEN rules_read
             AND introduced_self
             AND interests_selected
             AND ama_followed
             AND first_question_posted
             AND completed_at IS NULL THEN NOW()
            ELSE completed_at
          END,
          updated_at = NOW()
        WHERE circle_id = $1 AND user_id = $2
        "#,
    )
    .bind(circle_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn record_circle_post_engagement(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    circle_id: Uuid,
    post_type: &str,
) -> Result<(), AppError> {
    match post_type {
        "question" | "due_diligence" => {
            let _ = mark_circle_onboarding_step(pool, user_id, circle_id, "first_question_posted")
                .await;
            let _ = crate::community::challenges::increment_circle_progress(
                pool,
                user_id,
                circle_id,
                "circle_due_diligence_question",
                1,
            )
            .await;
        }
        "market_insight" => {
            let _ = crate::community::challenges::increment_circle_progress(
                pool,
                user_id,
                circle_id,
                "circle_market_insight",
                1,
            )
            .await;
        }
        _ => {}
    }

    Ok(())
}

async fn user_has_asset_holding(
    core_db: &sqlx::PgPool,
    user_id: Uuid,
    asset_id: Uuid,
) -> Result<bool, AppError> {
    let has_holding = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
          FROM investments
          WHERE user_id = $1
            AND asset_id = $2
            AND tokens_owned > 0
        )
        "#,
    )
    .bind(user_id)
    .bind(asset_id)
    .fetch_one(core_db)
    .await?;

    Ok(has_holding)
}

async fn ensure_asset_circle_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Uuid,
) -> Result<(Option<Uuid>, bool, bool), AppError> {
    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT is_public,
               visibility,
               join_policy,
               token_gate_asset_id,
               related_asset_id,
               kyc_required
        FROM circles
        WHERE id = $1
        "#,
    )
    .bind(circle_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    let is_public = row.try_get::<bool, _>("is_public").unwrap_or(false);
    let visibility = row
        .try_get::<String, _>("visibility")
        .unwrap_or_else(|_| "private".to_string());
    let join_policy = row
        .try_get::<String, _>("join_policy")
        .unwrap_or_else(|_| "request".to_string());
    let token_gate_asset_id: Option<Uuid> = row.try_get("token_gate_asset_id").ok().flatten();
    let related_asset_id: Option<Uuid> = row.try_get("related_asset_id").ok().flatten();
    let kyc_required = row.try_get::<bool, _>("kyc_required").unwrap_or(false);
    let asset_id = related_asset_id.or(token_gate_asset_id);
    let is_gated = asset_id.is_some()
        || kyc_required
        || matches!(join_policy.as_str(), "holder_only" | "kyc_required");

    let role = get_circle_member_role(pool, circle_id, user_id).await?;
    let has_holding = match asset_id {
        Some(asset_id) => user_has_asset_holding(&state.db, user_id, asset_id).await?,
        None => false,
    };

    if role.is_some()
        || has_holding
        || (is_public && visibility == "public" && !is_gated)
        || middleware::has_permission(&state.db, user_id, "community.manage").await
    {
        return Ok((asset_id, has_holding, role.is_some()));
    }

    Err(AppError::Forbidden(
        "You must hold the related asset or be a Circle member to access this resource.".into(),
    ))
}

async fn require_community_manage(
    state: &AppState,
    admin: &crate::admin::extractors::AdminUser,
) -> Result<(), AppError> {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Missing permission: community.manage".to_string(),
        ))
    }
}

async fn require_community_view_or_manage(
    state: &AppState,
    admin: &crate::admin::extractors::AdminUser,
) -> Result<(), AppError> {
    if crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.view").await
        || crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage")
            .await
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Missing permission: community.view".to_string(),
        ))
    }
}

#[derive(Deserialize)]
pub struct ToggleReactionReq {
    pub reaction_type: String,
}

#[derive(Deserialize)]
pub struct CreateCommentReq {
    pub content: String,
    // 14.8.12 — optional parent for nested replies (depth cap of 2 enforced
    // in service layer).
    pub parent_comment_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct UpdateQaStatusReq {
    pub status: String,
    pub official_answer_comment_id: Option<Uuid>,
    pub faq_candidate: Option<bool>,
    pub featured_question: Option<bool>,
    pub related_resource_url: Option<String>,
    pub related_asset_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct MarkOfficialAnswerReq {
    #[serde(default)]
    pub is_official_answer: Option<bool>,
    #[serde(default)]
    pub is_verified_answer: Option<bool>,
    #[serde(default)]
    pub qa_status: Option<String>,
}

const CIRCLE_ONBOARDING_STEPS: &[&str] = &[
    "rules_read",
    "introduced_self",
    "interests_selected",
    "ama_followed",
    "first_question_posted",
];

/// Helper to assert the community database is available
fn get_community_pool(state: &AppState) -> Result<sqlx::PgPool, AppError> {
    state.community_db.clone().ok_or_else(|| {
        tracing::error!("Community DB is not configured, but a community route was hit.");
        AppError::Internal("Community Database is offline".to_string())
    })
}

/// FIX-F7: Check if user is community-banned before allowing write operations
async fn check_user_not_banned(pool: &sqlx::PgPool, user_id: Uuid) -> Result<(), AppError> {
    let record = sqlx::query!(
        "SELECT is_community_banned, muted_until FROM community_profiles WHERE user_id = $1",
        user_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = record {
        if r.is_community_banned {
            return Err(AppError::Forbidden(
                "Your community access has been suspended. Contact support for more information."
                    .to_string(),
            ));
        }

        if let Some(muted_date) = r.muted_until {
            if muted_date > chrono::Utc::now() {
                return Err(AppError::Forbidden(format!(
                    "Your account is muted until {}. You cannot post or comment.",
                    muted_date.format("%Y-%m-%d %H:%M:%S UTC")
                )));
            }
        }
    }
    Ok(())
}

/// Helper to extract @mentions from content and notify mentioned users
async fn parse_and_notify_mentions(
    core_db: sqlx::PgPool,
    c_pool: sqlx::PgPool,
    content: String,
    author_id: Uuid,
    author_name: String,
    post_id: Uuid,
) {
    // CDDRP Phase 3.2: cap fan-out per post to prevent notification amplification
    // (a single post with hundreds of @mentions would otherwise spam every target).
    const MAX_MENTIONS_PER_POST: usize = 50;

    let mut mentions = std::collections::HashSet::new();
    for word in content.split_whitespace() {
        if word.starts_with("@circle/") {
            // Phase 4: Circle Mentions are not user mentions and must not fan
            // out notifications to Circle members automatically.
            continue;
        }
        if word.starts_with('@') && word.len() > 1 {
            let mention = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
            if mention.len() > 1 {
                mentions.insert(mention[1..].to_string()); // skip '@'
            }
        }
    }

    let original_mention_count = mentions.len();
    let mentions: Vec<String> = mentions.into_iter().take(MAX_MENTIONS_PER_POST).collect();
    if original_mention_count > MAX_MENTIONS_PER_POST {
        tracing::warn!(
            post_id = %post_id,
            author_id = %author_id,
            original_count = original_mention_count,
            capped_at = MAX_MENTIONS_PER_POST,
            "mention fan-out exceeded cap; truncating to prevent notification amplification"
        );
    }

    for mention in mentions {
        let query = format!("{}%", mention);
        let user_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT u.id FROM users u JOIN user_profiles up ON u.id = up.user_id WHERE up.display_name ILIKE $1 LIMIT 1"
        )
        .bind(&query)
        .fetch_optional(&core_db)
        .await
        .unwrap_or(None);

        if let Some(uid) = user_id {
            if uid != author_id {
                let msg = format!("{} mentioned you in a post.", author_name);
                let _ = crate::community::notifications::notify_user(
                    &c_pool,
                    uid,
                    Some(author_id),
                    "mention",
                    Some(post_id),
                    &msg,
                    Some(&format!("/community/feed?post={}", post_id)),
                )
                .await;
            }
        }
    }
}

/// Helper to parse the first URL in the content and fetch its OpenGraph data
async fn parse_and_store_opengraph(c_pool: sqlx::PgPool, content: String, post_id: Uuid) {
    // CDDRP Phase 3.2: cap OG body to 1 MB to prevent memory exhaustion from
    // attacker-controlled URLs that return huge or chunked bodies.
    const MAX_OG_BYTES: usize = 1024 * 1024;

    if let Ok(url_regex) = regex::Regex::new(r"https?://[^\s<]+") {
        if let Some(mat) = url_regex.find(&content) {
            let url = mat.as_str().to_string();

            // CDDRP Phase 3.2 (SSRF hardening): validate the URL host BEFORE we
            // make any outbound request. Mirrors the logic in
            // `backend/src/rewards/service.rs::validate_postback_url` /
            // `is_blocked_postback_ip`. OG is allowed over http OR https.
            let parsed = match url::Url::parse(&url) {
                Ok(u) => u,
                Err(_) => return,
            };
            if !matches!(parsed.scheme(), "http" | "https") {
                return;
            }
            let host = match parsed.host_str() {
                Some(h) => h,
                None => return,
            };
            let host_lower = host.trim_end_matches('.').to_ascii_lowercase();
            if matches!(
                host_lower.as_str(),
                "localhost" | "metadata.google.internal" | "metadata"
            ) || host_lower.ends_with(".localhost")
            {
                return;
            }
            if let Ok(ip) = host.parse::<std::net::IpAddr>() {
                if is_blocked_og_ip(ip) {
                    return;
                }
            } else {
                // Non-literal hostname: TOCTOU-aware DNS check. reqwest will
                // resolve again, but rejecting here drops the obvious cases.
                let port = parsed.port_or_known_default().unwrap_or(80);
                match tokio::net::lookup_host((host, port)).await {
                    Ok(addrs) => {
                        for addr in addrs {
                            if is_blocked_og_ip(addr.ip()) {
                                return;
                            }
                        }
                    }
                    Err(_) => return,
                }
            }

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                // CDDRP Phase 3.2: disable redirects so attacker-controlled
                // 302 → http://169.254.169.254/ cannot bypass our checks.
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default();

            if let Ok(mut res) = client.get(&url).send().await {
                // CDDRP Phase 3.2: bounded body read (1 MB cap). Stream chunks
                // via `Response::chunk()` and bail as soon as we exceed the cap.
                let mut buf: Vec<u8> = Vec::new();
                let mut over_cap = false;
                loop {
                    match res.chunk().await {
                        Ok(Some(chunk)) => {
                            if buf.len().saturating_add(chunk.len()) > MAX_OG_BYTES {
                                over_cap = true;
                                break;
                            }
                            buf.extend_from_slice(&chunk);
                        }
                        Ok(None) => break,
                        Err(_) => return,
                    }
                }
                if over_cap {
                    return;
                }
                let html = String::from_utf8_lossy(&buf);
                let title = extract_meta_tag(&html, "og:title");
                let image = extract_meta_tag(&html, "og:image");
                let desc = extract_meta_tag(&html, "og:description");

                if title.is_some() || image.is_some() || desc.is_some() {
                    let preview = serde_json::json!({
                        "url": url,
                        "title": title.unwrap_or_else(|| url.clone()),
                        "image": image,
                        "description": desc,
                    });

                    let _ = sqlx::query("UPDATE posts SET link_preview = $1 WHERE id = $2")
                        .bind(preview)
                        .bind(post_id)
                        .execute(&c_pool)
                        .await;
                }
            }
        }
    }
}

/// CDDRP Phase 3.2: mirrors `is_blocked_postback_ip` in
/// `backend/src/rewards/service.rs`. Kept inline (rather than `pub use`) because
/// the source helper is private; if it ever becomes `pub`, replace with a call.
fn is_blocked_og_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ip) => {
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
        std::net::IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_multicast()
        }
    }
}

fn extract_meta_tag(html: &str, property: &str) -> Option<String> {
    let re_str = format!(
        r#"(?i)<meta\s+[^>]*?property=["']{}["'][^>]*?content=["']([^"']+)["'][^>]*>"#,
        property
    );
    let re_str_alt = format!(
        r#"(?i)<meta\s+[^>]*?content=["']([^"']+)["'][^>]*?property=["']{}["'][^>]*>"#,
        property
    );

    // Check property before content
    if let Ok(re) = regex::Regex::new(&re_str) {
        if let Some(caps) = re.captures(html) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }

    // Check content before property
    if let Ok(re) = regex::Regex::new(&re_str_alt) {
        if let Some(caps) = re.captures(html) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }

    None
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

/// M6-FEAT.3: map a URL to (embed_kind, embed_id) for inline player cards.
/// Recognised: YouTube (watch?v=, youtu.be/, /embed/, /shorts/) and Loom
/// (loom.com/share/<id>). Anything else returns `None` so the generic OG
/// link card is used instead.
fn extract_video_embed(url: &str) -> Option<(String, String)> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.trim_start_matches("www.");
    match host {
        "youtube.com" | "m.youtube.com" => {
            // /watch?v=ID, /embed/ID, /shorts/ID
            if parsed.path() == "/watch" {
                let id = parsed
                    .query_pairs()
                    .find(|(k, _)| k == "v")
                    .map(|(_, v)| v.to_string())?;
                if !id.is_empty() {
                    return Some(("youtube".into(), id));
                }
            }
            let segs: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
            if segs.len() >= 2 && (segs[0] == "embed" || segs[0] == "shorts") && !segs[1].is_empty()
            {
                return Some(("youtube".into(), segs[1].to_string()));
            }
            None
        }
        "youtu.be" => {
            let segs: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
            segs.first()
                .filter(|s| !s.is_empty())
                .map(|s| ("youtube".into(), s.to_string()))
        }
        "loom.com" => {
            let segs: Vec<&str> = parsed.path().split('/').filter(|s| !s.is_empty()).collect();
            if segs.len() >= 2 && segs[0] == "share" && !segs[1].is_empty() {
                return Some(("loom".into(), segs[1].to_string()));
            }
            None
        }
        _ => None,
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn attr_escape(value: &str) -> String {
    html_escape(value)
}

#[derive(Debug, Clone)]
struct CircleMentionRef {
    slug: String,
    name: String,
    visibility: String,
    is_public: bool,
    viewer_role: Option<String>,
}

#[derive(Debug, Clone)]
enum InlineMentionToken {
    Text(String),
    Hashtag(String),
    Asset(String),
    User(String),
    Circle {
        matched: String,
        circle: CircleMentionRef,
    },
}

fn is_mention_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-'
}

fn is_circle_name_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ' '
}

fn has_token_boundary(rest: &str, len: usize) -> bool {
    rest.get(len..)
        .and_then(|tail| tail.chars().next())
        .map(|ch| !is_circle_name_char(ch))
        .unwrap_or(true)
}

fn normalize_circle_term(value: &str) -> String {
    value.trim().to_lowercase()
}

fn extract_circle_mention_terms(content: &str) -> (Vec<String>, Vec<String>) {
    const MAX_CIRCLE_MENTION_CANDIDATES: usize = 80;
    let mut names = std::collections::BTreeSet::new();
    let mut slugs = std::collections::BTreeSet::new();

    let canonical = regex::Regex::new(r"@circle/([A-Za-z0-9][A-Za-z0-9_-]{0,80})").unwrap();
    for caps in canonical
        .captures_iter(content)
        .take(MAX_CIRCLE_MENTION_CANDIDATES)
    {
        if let Some(slug) = caps.get(1) {
            slugs.insert(normalize_circle_term(slug.as_str()));
        }
    }

    for (idx, ch) in content.char_indices() {
        if ch != '@' {
            continue;
        }
        let rest = &content[idx + 1..];
        if rest.starts_with("circle/") {
            continue;
        }

        let mut end = 0;
        for (offset, next) in rest.char_indices() {
            if offset > 80 || !is_circle_name_char(next) {
                break;
            }
            end = offset + next.len_utf8();
        }
        if end == 0 {
            continue;
        }

        let phrase = rest[..end].trim();
        if phrase.is_empty() {
            continue;
        }

        let words: Vec<&str> = phrase.split_whitespace().take(5).collect();
        for end_word in 1..=words.len() {
            let candidate = words[..end_word].join(" ");
            if candidate.chars().count() < 2 {
                continue;
            }
            let normalized = normalize_circle_term(&candidate);
            names.insert(normalized.clone());
            slugs.insert(normalized.replace(' ', "-"));
        }
    }

    (
        names
            .into_iter()
            .take(MAX_CIRCLE_MENTION_CANDIDATES)
            .collect(),
        slugs
            .into_iter()
            .take(MAX_CIRCLE_MENTION_CANDIDATES)
            .collect(),
    )
}

async fn resolve_circle_mentions_for_content(
    pool: &sqlx::PgPool,
    viewer_id: Option<Uuid>,
    contents: &[String],
) -> Result<Vec<CircleMentionRef>, AppError> {
    let mut names = std::collections::BTreeSet::new();
    let mut slugs = std::collections::BTreeSet::new();
    for content in contents {
        let (content_names, content_slugs) = extract_circle_mention_terms(content);
        names.extend(content_names);
        slugs.extend(content_slugs);
    }
    if names.is_empty() && slugs.is_empty() {
        return Ok(Vec::new());
    }

    use sqlx::Row;
    let names: Vec<String> = names.into_iter().collect();
    let slugs: Vec<String> = slugs.into_iter().collect();
    let rows = sqlx::query(
        r#"
        SELECT c.slug, c.name, c.visibility, c.is_public, cm.role AS viewer_role
        FROM circles c
        LEFT JOIN circle_members cm
          ON cm.circle_id = c.id
         AND cm.user_id = $3
        WHERE LOWER(c.name) = ANY($1)
           OR LOWER(c.slug) = ANY($2)
        ORDER BY GREATEST(length(c.name), length(c.slug)) DESC
        LIMIT 100
        "#,
    )
    .bind(&names)
    .bind(&slugs)
    .bind(viewer_id)
    .fetch_all(pool)
    .await?;

    let mut circles = Vec::with_capacity(rows.len());
    for row in rows {
        circles.push(CircleMentionRef {
            slug: row.try_get("slug")?,
            name: row.try_get("name")?,
            visibility: row
                .try_get("visibility")
                .unwrap_or_else(|_| "public".to_string()),
            is_public: row.try_get("is_public").unwrap_or(false),
            viewer_role: row.try_get("viewer_role").ok().flatten(),
        });
    }
    circles.sort_by(|a, b| {
        let a_len = a.name.len().max(a.slug.len());
        let b_len = b.name.len().max(b.slug.len());
        b_len.cmp(&a_len)
    });
    Ok(circles)
}

fn match_circle_mention_at(
    rest_after_at: &str,
    circles: &[CircleMentionRef],
) -> Option<(usize, CircleMentionRef, String)> {
    if let Some(slug_part) = rest_after_at.strip_prefix("circle/") {
        let slug_len = slug_part
            .chars()
            .take_while(|ch| is_mention_word_char(*ch))
            .map(char::len_utf8)
            .sum::<usize>();
        if slug_len > 0 {
            let slug = &slug_part[..slug_len];
            if let Some(circle) = circles
                .iter()
                .find(|circle| circle.slug.eq_ignore_ascii_case(slug))
            {
                let matched = format!("@circle/{}", slug);
                return Some(("circle/".len() + slug_len + 1, circle.clone(), matched));
            }
        }
    }

    let lower_rest = rest_after_at.to_lowercase();
    for circle in circles {
        let name = circle.name.to_lowercase();
        if lower_rest.starts_with(&name) && has_token_boundary(rest_after_at, circle.name.len()) {
            let matched = format!("@{}", &rest_after_at[..circle.name.len()]);
            return Some((circle.name.len() + 1, circle.clone(), matched));
        }
        let slug = circle.slug.to_lowercase();
        if lower_rest.starts_with(&slug) && has_token_boundary(rest_after_at, circle.slug.len()) {
            let matched = format!("@{}", &rest_after_at[..circle.slug.len()]);
            return Some((circle.slug.len() + 1, circle.clone(), matched));
        }
    }
    None
}

fn tokenize_inline_mentions(
    content: &str,
    circles: &[CircleMentionRef],
) -> Vec<InlineMentionToken> {
    let hashtag_re = regex::Regex::new(r"^#[\w\u00C0-\u024F]+").unwrap();
    let user_re = regex::Regex::new(r"^@[\w\u00C0-\u024F_-]+").unwrap();
    let asset_re = regex::Regex::new(r"^\$[a-zA-Z0-9_-]+").unwrap();

    let mut out = Vec::new();
    let mut cursor = 0;
    while cursor < content.len() {
        let next = content[cursor..]
            .find(['#', '@', '$'])
            .map(|offset| cursor + offset);
        let Some(start) = next else {
            out.push(InlineMentionToken::Text(content[cursor..].to_string()));
            break;
        };
        if start > cursor {
            out.push(InlineMentionToken::Text(content[cursor..start].to_string()));
        }

        let rest = &content[start..];
        if let Some(after_at) = rest.strip_prefix('@') {
            if let Some((len, circle, matched)) = match_circle_mention_at(after_at, circles) {
                out.push(InlineMentionToken::Circle { matched, circle });
                cursor = start + len;
                continue;
            }
            if let Some(matched) = user_re.find(rest).map(|m| m.as_str()) {
                out.push(InlineMentionToken::User(matched[1..].to_string()));
                cursor = start + matched.len();
                continue;
            }
        } else if let Some(matched) = hashtag_re.find(rest).map(|m| m.as_str()) {
            out.push(InlineMentionToken::Hashtag(matched[1..].to_string()));
            cursor = start + matched.len();
            continue;
        } else if let Some(matched) = asset_re.find(rest).map(|m| m.as_str()) {
            out.push(InlineMentionToken::Asset(matched[1..].to_string()));
            cursor = start + matched.len();
            continue;
        }

        let ch_len = rest.chars().next().map(char::len_utf8).unwrap_or(1);
        out.push(InlineMentionToken::Text(rest[..ch_len].to_string()));
        cursor = start + ch_len;
    }
    out
}

fn render_circle_mention(circle: &CircleMentionRef, matched: &str) -> String {
    let viewer_can_see =
        circle.viewer_role.is_some() || (circle.is_public && circle.visibility == "public");

    if viewer_can_see {
        let label = format!("@{}", circle.name);
        return format!(
            "<a class='circle-mention-tag' data-circle-slug='{}' href='/community/circle/{}' aria-label='Open Circle {}'>{}</a>",
            attr_escape(&circle.slug),
            attr_escape(&circle.slug),
            attr_escape(&circle.name),
            html_escape(&label)
        );
    }

    if circle.visibility == "hidden" {
        return "<span class='circle-mention-tag circle-mention-tag--redacted' aria-label='Hidden Circle'>Circle mention unavailable</span>".to_string();
    }

    let _ = matched;
    "<span class='circle-mention-tag circle-mention-tag--private' aria-label='Private Circle'>Private Circle</span>".to_string()
}

fn render_inline_content_with_circle_mentions(
    content: &str,
    circles: &[CircleMentionRef],
) -> String {
    let tokens = tokenize_inline_mentions(content, circles);
    let mut rendered = String::with_capacity(content.len());
    for token in tokens {
        match token {
            InlineMentionToken::Text(text) => rendered.push_str(&text),
            InlineMentionToken::Hashtag(tag) => {
                let tag_lower = tag.to_lowercase();
                rendered.push_str(&format!(
                    "<a class='hashtag-tag' href='/community/hashtag/{}'>#{}</a>",
                    attr_escape(&tag_lower),
                    html_escape(&tag)
                ));
            }
            InlineMentionToken::Asset(slug) => {
                let slug_lower = slug.to_lowercase();
                rendered.push_str(&format!(
                    "<a class='asset-tag' data-asset-slug='{}' href='/marketplace?q={}'>${}</a>",
                    attr_escape(&slug_lower),
                    attr_escape(&slug_lower),
                    html_escape(&slug)
                ));
            }
            InlineMentionToken::User(user) => {
                rendered.push_str(&format!(
                    "<span class='mention-tag' data-handle='{}'>@{}</span>",
                    attr_escape(&user),
                    html_escape(&user)
                ));
            }
            InlineMentionToken::Circle { matched, circle } => {
                rendered.push_str(&render_circle_mention(&circle, &matched));
            }
        }
    }
    rendered
}

async fn hydrate_circle_mentions(
    pool: &sqlx::PgPool,
    viewer_id: Option<Uuid>,
    posts: &mut [PostDisplay],
) -> Result<(), AppError> {
    let contents: Vec<String> = posts.iter().map(|post| post.content.clone()).collect();
    let circles = resolve_circle_mentions_for_content(pool, viewer_id, &contents).await?;
    if circles.is_empty() {
        return Ok(());
    }
    for post in posts {
        post.rendered_content = render_inline_content_with_circle_mentions(&post.content, &circles);
    }
    Ok(())
}

pub fn map_to_post_display(
    p: &models::Post,
    author_name: String,
    author_avatar: Option<String>,
    author_badges: Vec<String>,
    current_user_reacted: bool,
    is_bookmarked: bool,
) -> PostDisplay {
    let mut author_initials = String::new();
    let parts: Vec<&str> = author_name.split_whitespace().collect();
    if parts.len() > 1 {
        author_initials.push(parts[0].chars().next().unwrap_or('?'));
        author_initials.push(parts[1].chars().next().unwrap_or('?'));
    } else if !author_name.is_empty() {
        author_initials.push(author_name.chars().next().unwrap_or('?'));
    } else {
        author_initials.push('?');
    }
    author_initials = author_initials.to_uppercase();

    let link_preview_domain = p.link_preview.as_ref().and_then(|v| {
        v.get("url").and_then(|s| s.as_str()).and_then(|url| {
            url::Url::parse(url)
                .ok()
                .and_then(|u| u.domain().map(|d| d.trim_start_matches("www.").to_string()))
        })
    });

    let raw_content = p
        .content_sanitized
        .clone()
        .unwrap_or_else(|| p.content.clone());
    let rendered_content = if p.post_type == "announcement" {
        raw_content.clone()
    } else {
        render_inline_content_with_circle_mentions(&raw_content, &[])
    };

    let image_urls = p
        .image_urls
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|u| crate::storage::service::rewrite_gcs_url(&u))
        .collect();

    // UX.20: average adult reading speed ≈ 200 wpm. Only surface a badge
    // when the post would take at least a minute to read so we don't
    // clutter every short status.
    let word_count = raw_content.split_whitespace().count();
    let read_time_minutes = if word_count >= 200 {
        Some(((word_count as f32) / 200.0).ceil() as i32)
    } else {
        None
    };

    // M6-FEAT.3: rich-media embed (YouTube/Loom) when the OG link points
    // at one of those providers. Falls back to the generic preview card.
    let (embed_kind, embed_id) = p
        .link_preview
        .as_ref()
        .and_then(|v| v.get("url").and_then(|u| u.as_str()))
        .and_then(extract_video_embed)
        .map(|(k, i)| (Some(k), Some(i)))
        .unwrap_or((None, None));

    PostDisplay {
        id: p.id,
        author_name,
        author_initials,
        author_id: p.user_id,
        author_avatar,
        author_badges,
        post_type: p.post_type.clone(),
        content_tags: p.content_tags.clone().unwrap_or_default(),
        content: raw_content,
        rendered_content,
        asset_id: p.asset_id,
        circle_id: p.circle_id,
        circle_name: None,
        image_urls,
        link_preview: p.link_preview.clone(),
        link_preview_domain,
        reaction_count: p.reaction_count,
        comment_count: p.comment_count,
        qa_status: p.qa_status.clone(),
        official_answer_comment_id: p.official_answer_comment_id,
        faq_candidate: p.faq_candidate,
        featured_question: p.featured_question,
        related_resource_url: p.related_resource_url.clone(),
        related_asset_id: p.related_asset_id,
        current_user_reacted,
        is_bookmarked,
        is_hidden: p.is_hidden,
        is_pinned: p.is_pinned,
        disclaimer_shown: p.disclaimer_shown,
        verified_owner: false,
        created_at: p.created_at,
        created_at_display: p.created_at.format("%b %e, %H:%M").to_string(),
        read_time_minutes,
        embed_kind,
        embed_id,
        // Callers populate this themselves (feed-level) so non-feed paths
        // can opt out of the extra round-trip when they don't render the
        // quote card.
        quoted: None,
        // UX.14: ditto — feed-level callers hydrate this via the flair
        // batch helper. Detail/admin paths are free to skip the lookup.
        author_flair: None,
        author_reputation_flairs: Vec::new(),
        author_top_contributor: false,
        author_tier: None,
    }
}

pub async fn get_feed_data(
    state: &AppState,
    query: &FeedQuery,
    user: Option<&crate::auth::models::User>,
) -> Result<Vec<PostDisplay>, AppError> {
    if query.feed_mode.as_deref() == Some("following") && user.is_none() {
        return Err(AppError::Unauthorized(
            "You must be logged in to view your following feed.".into(),
        ));
    }

    let c_pool = get_community_pool(state)?;

    let limit = 20;
    let offset = (query.page.unwrap_or(1).max(1) - 1) * limit;

    let only_following_user_id = if query.feed_mode.as_deref() == Some("following") {
        user.map(|u| u.id)
    } else {
        None
    };
    let post_type_filter = match query.post_type.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() && value != "all" => Some(normalize_post_type(value)?),
        _ => None,
    };
    let tag_filter = match query.tag.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() && value != "all" => {
            let normalized = normalize_post_tags(Some(vec![value.to_string()]))?;
            normalized.into_iter().next()
        }
        _ => None,
    };

    let posts = service::get_community_feed(
        &c_pool,
        query.category.clone(),
        only_following_user_id,
        query.sort_by.clone(),
        post_type_filter,
        tag_filter,
        limit,
        offset,
        user.map(|u| u.id),
    )
    .await?;

    hydrate_post_displays(state, &c_pool, posts, user).await
}

pub async fn get_circle_feed_data(
    state: &AppState,
    circle_id: Uuid,
    page: Option<i64>,
    sort_by: Option<String>,
    post_type: Option<String>,
    tag: Option<String>,
    user: Option<&crate::auth::models::User>,
) -> Result<Vec<PostDisplay>, AppError> {
    let c_pool = get_community_pool(state)?;
    ensure_circle_read_access(state, &c_pool, circle_id, user.map(|u| u.id)).await?;

    let limit = 20;
    let offset = (page.unwrap_or(1).max(1) - 1) * limit;
    let post_type_filter = match post_type.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() && value != "all" => Some(normalize_post_type(value)?),
        _ => None,
    };
    let tag_filter = match tag.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() && value != "all" => {
            let normalized = normalize_post_tags(Some(vec![value.to_string()]))?;
            normalized.into_iter().next()
        }
        _ => None,
    };
    let posts = service::get_circle_feed(
        &c_pool,
        circle_id,
        sort_by,
        post_type_filter,
        tag_filter,
        limit,
        offset,
        user.map(|u| u.id),
    )
    .await?;

    hydrate_post_displays(state, &c_pool, posts, user).await
}

async fn hydrate_post_displays(
    state: &AppState,
    c_pool: &sqlx::PgPool,
    posts: Vec<models::Post>,
    user: Option<&crate::auth::models::User>,
) -> Result<Vec<PostDisplay>, AppError> {
    use sqlx::Row;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let circle_ids: Vec<Uuid> = posts.iter().filter_map(|p| p.circle_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;
    let badges = service::get_badges_batch(c_pool, &user_ids).await?;
    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();
    let (reacted_post_ids, bookmarked_post_ids) = if let Some(current_user) = user {
        if post_ids.is_empty() {
            (
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
            )
        } else {
            // Single round-trip per signal: fetches all reactions/bookmarks
            // for the visible page in one query, replacing the FE per-post
            // GET /bookmark/status N+1 storm.
            let reacted = sqlx::query_scalar::<_, Uuid>(
                "SELECT post_id FROM reactions WHERE user_id = $1 AND post_id = ANY($2) AND reaction_type = 'fire'",
            )
            .bind(current_user.id)
            .bind(&post_ids)
            .fetch_all(c_pool)
            .await?
            .into_iter()
            .collect();
            let bookmarked = sqlx::query_scalar::<_, Uuid>(
                "SELECT post_id FROM bookmarks WHERE user_id = $1 AND post_id = ANY($2)",
            )
            .bind(current_user.id)
            .bind(&post_ids)
            .fetch_all(c_pool)
            .await?
            .into_iter()
            .collect();
            (reacted, bookmarked)
        }
    } else {
        (
            std::collections::HashSet::new(),
            std::collections::HashSet::new(),
        )
    };

    // UX.16 — batch-fetch quoted post briefs so each card can render the
    // shared post without an extra request per row.
    let quoted_brief_map = fetch_quoted_briefs(state, c_pool, &posts).await;
    // UX.14 — batch-fetch user flairs.
    let flair_map = service::get_flairs_batch(c_pool, &user_ids)
        .await
        .unwrap_or_default();
    // Phase 3 — non-user-editable reputation signals such as Official POOOL,
    // Verified Investor, Asset Holder, and domain expert labels.
    let reputation_flair_map = service::get_reputation_flairs_batch(c_pool, &user_ids)
        .await
        .unwrap_or_default();
    // UX.17 — top-50 by XP gets a "Top Contributor" badge.
    let top_contributor_set = service::get_top_contributor_set(c_pool, 50)
        .await
        .unwrap_or_default();
    // W3.4 — portfolio tier (cross-DB lookup against the core investments table).
    let tier_map = user_bridge::get_portfolio_tiers_batch(&state.db, &user_ids)
        .await
        .unwrap_or_default();
    let circle_name_map: std::collections::HashMap<Uuid, String> = if circle_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        sqlx::query("SELECT id, name FROM circles WHERE id = ANY($1)")
            .bind(&circle_ids)
            .fetch_all(c_pool)
            .await?
            .into_iter()
            .filter_map(|row| {
                let id: Uuid = row.try_get("id").ok()?;
                let name: String = row.try_get("name").ok()?;
                Some((id, name))
            })
            .collect()
    };

    let mut feed = Vec::with_capacity(posts.len());

    for p in posts {
        let author = authors.get(&p.user_id);
        let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();
        let author_name = author
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Anonymous".into());

        let mut display = map_to_post_display(
            &p,
            author_name,
            author.and_then(|a| a.avatar_url.clone()),
            author_badges,
            reacted_post_ids.contains(&p.id),
            bookmarked_post_ids.contains(&p.id),
        );
        if let Some(qid) = p.quoted_post_id {
            display.quoted = quoted_brief_map.get(&qid).cloned();
        }
        display.circle_name = p
            .circle_id
            .and_then(|circle_id| circle_name_map.get(&circle_id).cloned());
        display.author_flair = flair_map.get(&p.user_id).cloned();
        display.author_reputation_flairs = reputation_flair_map
            .get(&p.user_id)
            .cloned()
            .unwrap_or_default();
        display.author_top_contributor = top_contributor_set.contains(&p.user_id);
        display.author_tier = tier_map.get(&p.user_id).cloned();
        feed.push(display);
    }

    hydrate_circle_mentions(c_pool, user.map(|current_user| current_user.id), &mut feed).await?;

    Ok(feed)
}

/// UX.16 — batch-resolve `posts.quoted_post_id` → QuotedPostBrief.
/// One Postgres round-trip + one user-bridge call regardless of page size.
async fn fetch_quoted_briefs(
    state: &AppState,
    c_pool: &sqlx::PgPool,
    posts: &[crate::community::models::Post],
) -> std::collections::HashMap<Uuid, crate::community::models::QuotedPostBrief> {
    use sqlx::Row;
    let quoted_ids: Vec<Uuid> = posts.iter().filter_map(|p| p.quoted_post_id).collect();
    if quoted_ids.is_empty() {
        return std::collections::HashMap::new();
    }
    let rows = match sqlx::query(
        r#"
        SELECT id, user_id, COALESCE(content_sanitized, content) AS content, created_at
        FROM posts
        WHERE id = ANY($1) AND is_hidden = FALSE
        "#,
    )
    .bind(&quoted_ids)
    .fetch_all(c_pool)
    .await
    {
        Ok(r) => r,
        Err(_) => return std::collections::HashMap::new(),
    };
    let user_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect();
    let info_map = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &user_ids,
    )
    .await
    .unwrap_or_default();

    let mut out = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        let id: Uuid = match r.try_get("id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let uid: Uuid = r.try_get("user_id").unwrap_or_default();
        let content: String = r.try_get("content").unwrap_or_default();
        let created_at: chrono::DateTime<chrono::Utc> = r
            .try_get("created_at")
            .unwrap_or_else(|_| chrono::Utc::now());
        let info = info_map.get(&uid);
        let truncated = if content.chars().count() > 280 {
            let mut s: String = content.chars().take(277).collect();
            s.push('…');
            s
        } else {
            content
        };
        out.insert(
            id,
            crate::community::models::QuotedPostBrief {
                id,
                author_name: info
                    .map(|i| i.display_name.clone())
                    .unwrap_or_else(|| "Anonymous".to_string()),
                author_avatar: info.and_then(|i| i.avatar_url.clone()),
                content: truncated,
                created_at_display: created_at.format("%b %e, %H:%M").to_string(),
            },
        );
    }
    out
}

async fn get_feed(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<FeedQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db).await;
    let feed = get_feed_data(&state, &query, user.as_ref()).await?;
    Ok(Json(feed))
}

async fn get_circle_posts(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Query(query): Query<FeedQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db).await;
    let feed = get_circle_feed_data(
        &state,
        circle_id,
        query.page,
        query.sort_by,
        query.post_type,
        query.tag,
        user.as_ref(),
    )
    .await?;
    Ok(Json(feed))
}

async fn get_post_detail(
    Path(post_id): Path<Uuid>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    // allow public read, but if user logged in we can fetch them
    let user = middleware::get_current_user(&jar, &state.db).await;
    let _only_following_user_id: Option<Uuid> = None; // not constrained by following
    let c_pool = get_community_pool(&state)?;

    let post = sqlx::query_as::<_, models::Post>(
        r#"
        SELECT p.*
        FROM posts p
        JOIN community_profiles cp ON p.user_id = cp.user_id
        WHERE p.id = $1
          AND p.is_hidden = false
          AND (cp.is_shadowbanned = false OR p.user_id = $2)
        "#,
    )
    .bind(post_id)
    .bind(user.as_ref().map(|u| u.id))
    .fetch_optional(&c_pool)
    .await
    .map_err(AppError::Database)?;

    let p = match post {
        Some(pt) => pt,
        None => return Err(AppError::NotFound("Post not found".into())),
    };

    if let Some(circle_id) = p.circle_id {
        ensure_circle_read_access(&state, &c_pool, circle_id, user.as_ref().map(|u| u.id)).await?;
    }

    let author_info = user_bridge::get_user_info(&state.db, state.redis.as_ref(), p.user_id)
        .await
        .ok();

    let mut author_badges = vec![];
    if p.user_id != Uuid::nil() {
        let mut b_map = service::get_badges_batch(&c_pool, &[p.user_id]).await?;
        author_badges = b_map.remove(&p.user_id).unwrap_or_default();
    }

    let author_name = author_info
        .as_ref()
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| "Anonymous".into());
    let current_user_reacted = if let Some(ref current_user) = user {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM reactions WHERE post_id = $1 AND user_id = $2 AND reaction_type = 'fire')",
        )
        .bind(p.id)
        .bind(current_user.id)
        .fetch_one(&c_pool)
        .await?
    } else {
        false
    };
    let is_bookmarked = if let Some(ref current_user) = user {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM bookmarks WHERE post_id = $1 AND user_id = $2)",
        )
        .bind(p.id)
        .bind(current_user.id)
        .fetch_one(&c_pool)
        .await?
    } else {
        false
    };

    let mut response = map_to_post_display(
        &p,
        author_name,
        author_info.and_then(|a| a.avatar_url.clone()),
        author_badges,
        current_user_reacted,
        is_bookmarked,
    );
    hydrate_circle_mentions(
        &c_pool,
        user.as_ref().map(|current_user| current_user.id),
        std::slice::from_mut(&mut response),
    )
    .await?;

    Ok(Json(response))
}

async fn create_announcement(
    admin: crate::admin::extractors::AdminUser,
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<CreateAnnouncementReq>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = admin.user.id;

    require_csrf_header(&headers, &jar)?;

    if !crate::auth::middleware::has_permission(&state.db, user_id, "community.manage").await {
        return Err(AppError::Forbidden(
            "Missing permission: community.manage".to_string(),
        ));
    }

    validate_announcement_category(&payload.category)?;

    let c_pool = get_community_pool(&state)?;

    let clean_html = validation::sanitize_html_basic(&payload.content);
    if clean_html.trim().is_empty() || clean_html == "<p><br></p>" {
        return Err(AppError::BadRequest(
            "Announcement content is required.".to_string(),
        ));
    }
    let clean_html_len = clean_html.chars().count();
    let category = payload.category.clone();
    let is_pinned = payload.is_pinned.unwrap_or(false);

    let post_id = service::create_announcement(
        &c_pool,
        user_id,
        payload.content,
        clean_html,
        payload.category,
        payload.image_urls,
        is_pinned,
        serde_json::json!({
            "category": category,
            "is_pinned": is_pinned,
            "content_length": clean_html_len
        }),
    )
    .await?;

    Ok(Json(serde_json::json!({ "id": post_id })))
}

async fn admin_list_announcements(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Query(query): Query<FeedQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;

    if let Some(category) = query.category.as_deref() {
        if !category.is_empty() {
            validate_announcement_category(category)?;
        }
    }

    let c_pool = get_community_pool(&state)?;
    let announcements = service::get_announcements(&c_pool, query.category, 50).await?;

    Ok(Json(announcements))
}

async fn toggle_reaction(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<ToggleReactionReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    // FIX-F7: Check ban before allowing reaction
    check_user_not_banned(&c_pool, user.id).await?;
    ensure_post_read_access(&state, &c_pool, post_id, Some(user.id)).await?;

    let outcome =
        service::toggle_reaction(&c_pool, post_id, user.id, payload.reaction_type).await?;

    // Award XP only when reaction is added (not removed)
    if outcome.added {
        let _ = crate::community::xp::award_xp(
            &c_pool,
            user.id,
            "reaction_given",
            Some("Reacted to a post"),
            None,
        )
        .await;
    }

    Ok(Json(serde_json::json!({
        "added": outcome.added,
        "reaction_count": outcome.reaction_count,
    })))
}

async fn create_comment(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<CreateCommentReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    validation::validate_comment_length(&payload.content)?;

    let clean_html = validation::sanitize_html_basic(&payload.content);
    let c_pool = get_community_pool(&state)?;

    if let Some(reason) = validation::check_automod(&payload.content) {
        // M6-BE.1 Auto Mod
        return Err(AppError::Forbidden(format!(
            "Content violation: {}",
            reason
        )));
    }

    // FIX-F7: Check ban before allowing comment
    check_user_not_banned(&c_pool, user.id).await?;
    ensure_post_write_access(&state, &c_pool, post_id, user.id).await?;

    // Check if post is locked (M6-ADMIN.1)
    let is_locked: Option<bool> = sqlx::query_scalar("SELECT is_locked FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&c_pool)
        .await?;

    if is_locked.unwrap_or(false) {
        return Err(AppError::Forbidden(
            "This thread has been locked by a moderator.".into(),
        ));
    }

    // FIX-CRL: Comment rate limiting (30 comments/hour via Redis)
    if let Some(redis_pool) = state.redis.as_ref() {
        use redis::AsyncCommands;
        if let Ok(mut conn) = redis_pool.get().await {
            let rl_key = format!("community:ratelimit:comments:{}", user.id);
            let count: Option<i64> = conn.get(&rl_key).await.unwrap_or(None);
            if let Some(c) = count {
                if c >= 30 {
                    return Err(AppError::BadRequest(
                        "Rate limit exceeded: Max 30 comments per hour.".into(),
                    ));
                }
            }
            let _: () = conn.incr(&rl_key, 1).await.unwrap_or(());
            let _: () = conn.expire(&rl_key, 3600).await.unwrap_or(());
        }
    }

    let comment_id = service::create_comment(
        &c_pool,
        post_id,
        user.id,
        payload.content.clone(),
        clean_html,
        payload.parent_comment_id,
    )
    .await?;

    let comment_circle_id: Option<Uuid> =
        sqlx::query_scalar("SELECT circle_id FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(&c_pool)
            .await?
            .flatten();
    if let Some(circle_id) = comment_circle_id {
        let _ = crate::community::challenges::increment_circle_progress(
            &c_pool,
            user.id,
            circle_id,
            "circle_comment",
            1,
        )
        .await;
    }

    // Award XP for comment
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "comment_created",
        Some("Posted a comment"),
        None,
    )
    .await;

    let author_name = user_bridge::get_user_info(&state.db, state.redis.as_ref(), user.id)
        .await
        .map(|u| u.display_name)
        .unwrap_or_else(|_| "Someone".to_string());

    // Parse and notify mentions asynchronously
    let core_db_clone = state.db.clone();
    let c_pool_clone = c_pool.clone();
    let content_clone = payload.content;
    tokio::spawn(async move {
        parse_and_notify_mentions(
            core_db_clone,
            c_pool_clone,
            content_clone,
            user.id,
            author_name,
            post_id,
        )
        .await;
    });

    Ok(Json(serde_json::json!({ "id": comment_id })))
}

#[derive(Deserialize)]
struct UpdateCommentReq {
    pub content: String,
}

/// PUT /api/community/comments/:id — owner edits their own comment (14.8.5).
///
/// Reuses the same guards as `create_comment`: ban check, length validation,
/// automod, locked-thread, and HTML sanitisation. The first edit also
/// captures `original_content` for moderation review; subsequent edits
/// preserve it.
async fn update_own_comment(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
    Json(payload): Json<UpdateCommentReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    validation::validate_comment_length(&payload.content)?;
    if let Some(reason) = validation::check_automod(&payload.content) {
        return Err(AppError::Forbidden(format!(
            "Content violation: {}",
            reason
        )));
    }
    let clean_html = validation::sanitize_html_basic(&payload.content);
    let c_pool = get_community_pool(&state)?;
    check_user_not_banned(&c_pool, user.id).await?;

    use sqlx::Row;
    let row = sqlx::query("SELECT user_id, post_id, content FROM comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&c_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Comment not found".into()))?;

    let author_id: Uuid = row.try_get("user_id")?;
    if author_id != user.id {
        return Err(AppError::Forbidden(
            "You can only edit your own comments.".into(),
        ));
    }
    let post_id: Uuid = row.try_get("post_id")?;
    let original_content: String = row.try_get("content")?;
    ensure_post_write_access(&state, &c_pool, post_id, user.id).await?;

    // Refuse edits on locked threads (same rule as create_comment).
    let is_locked: Option<bool> = sqlx::query_scalar("SELECT is_locked FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&c_pool)
        .await?;
    if is_locked.unwrap_or(false) {
        return Err(AppError::Forbidden(
            "This thread has been locked by a moderator.".into(),
        ));
    }

    // First edit captures original_content; subsequent edits leave it alone.
    sqlx::query(
        "UPDATE comments SET
            content = $1,
            content_sanitized = $2,
            edited_at = NOW(),
            original_content = COALESCE(original_content, $3)
         WHERE id = $4",
    )
    .bind(&payload.content)
    .bind(&clean_html)
    .bind(&original_content)
    .bind(comment_id)
    .execute(&c_pool)
    .await?;

    Ok(Json(serde_json::json!({
        "id": comment_id,
        "edited": true,
    })))
}

/// DELETE /api/community/comments/:id — owner deletes their own comment
/// (Phase 3 task 26). Refuses on locked threads and 404s if the row is gone.
async fn delete_own_comment(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;
    let row = sqlx::query("SELECT user_id, post_id FROM comments WHERE id = $1")
        .bind(comment_id)
        .fetch_optional(&c_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Comment not found".into()))?;

    let author_id: Uuid = row.try_get("user_id")?;
    if author_id != user.id {
        return Err(AppError::Forbidden(
            "You can only delete your own comments.".into(),
        ));
    }
    let post_id: Uuid = row.try_get("post_id")?;
    ensure_post_write_access(&state, &c_pool, post_id, user.id).await?;
    let is_locked: Option<bool> = sqlx::query_scalar("SELECT is_locked FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&c_pool)
        .await?;
    if is_locked.unwrap_or(false) {
        return Err(AppError::Forbidden(
            "This thread has been locked by a moderator.".into(),
        ));
    }

    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&c_pool)
        .await?;

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
struct ToggleCommentReactionReq {
    pub reaction_type: String,
}

/// POST /api/community/comments/:id/reactions — toggle a reaction on a
/// comment (14.8.6). Same taxonomy as post reactions; the
/// comment_reactions trigger keeps comments.reaction_count in sync.
async fn toggle_comment_reaction(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
    Json(payload): Json<ToggleCommentReactionReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let allowed = ["fire", "insightful", "clap", "green"];
    if !allowed.contains(&payload.reaction_type.as_str()) {
        return Err(AppError::BadRequest("Invalid reaction type.".into()));
    }

    let c_pool = get_community_pool(&state)?;
    check_user_not_banned(&c_pool, user.id).await?;

    // Comment must exist + not be hidden.
    use sqlx::Row;
    let exists: Option<bool> =
        sqlx::query_scalar("SELECT NOT is_hidden FROM comments WHERE id = $1")
            .bind(comment_id)
            .fetch_optional(&c_pool)
            .await?;
    if !exists.unwrap_or(false) {
        return Err(AppError::NotFound("Comment not found".into()));
    }

    let mut tx = c_pool.begin().await?;
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM comment_reactions
         WHERE comment_id = $1 AND user_id = $2 AND reaction_type = $3",
    )
    .bind(comment_id)
    .bind(user.id)
    .bind(&payload.reaction_type)
    .fetch_optional(&mut *tx)
    .await?;

    let added = if existing.is_some() {
        sqlx::query(
            "DELETE FROM comment_reactions
             WHERE comment_id = $1 AND user_id = $2 AND reaction_type = $3",
        )
        .bind(comment_id)
        .bind(user.id)
        .bind(&payload.reaction_type)
        .execute(&mut *tx)
        .await?;
        false
    } else {
        sqlx::query(
            "INSERT INTO comment_reactions (comment_id, user_id, reaction_type)
             VALUES ($1, $2, $3)",
        )
        .bind(comment_id)
        .bind(user.id)
        .bind(&payload.reaction_type)
        .execute(&mut *tx)
        .await?;
        true
    };

    let count: i32 = sqlx::query("SELECT reaction_count FROM comments WHERE id = $1")
        .bind(comment_id)
        .fetch_one(&mut *tx)
        .await?
        .try_get("reaction_count")?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "added": added,
        "reaction_count": count,
    })))
}

async fn get_comments(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Basic auth check
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_post_read_access(&state, &c_pool, post_id, Some(_user.id)).await?;
    let can_mark_official_answer =
        user_can_manage_qa_post(&state, &c_pool, _user.id, post_id).await?;

    let comments = sqlx::query_as::<_, Comment>(
        r#"
        SELECT c.*
        FROM comments c
        JOIN community_profiles cp ON c.user_id = cp.user_id
        WHERE c.post_id = $1
          AND c.is_hidden = false
          AND (cp.is_shadowbanned = false OR c.user_id = $2)
        ORDER BY c.created_at ASC
        "#,
    )
    .bind(post_id)
    .bind(&_user.id)
    .fetch_all(&c_pool)
    .await?;

    // Batch map authors
    let user_ids: Vec<Uuid> = comments.iter().map(|c| c.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

    let mut result = Vec::with_capacity(comments.len());
    for c in comments {
        let author = authors.get(&c.user_id);
        result.push(serde_json::json!({
            "id": c.id,
            "post_id": c.post_id,
            "author_id": c.user_id,
            "author_name": author.map(|a| a.display_name.clone()).unwrap_or_else(|| "Anonymous".into()),
            "author_avatar": author.and_then(|a| a.avatar_url.clone()),
            "content": c.content,
            "helpful_count": c.helpful_count,
            "created_at": c.created_at,
            "edited_at": c.edited_at,
            "parent_comment_id": c.parent_comment_id,
            "reaction_count": c.reaction_count,
            "is_official_answer": c.is_official_answer,
            "is_verified_answer": c.is_verified_answer,
            "answer_marked_by": c.answer_marked_by,
            "answer_marked_at": c.answer_marked_at,
            "can_mark_official_answer": can_mark_official_answer,
        }));
    }

    Ok(Json(result))
}

async fn update_post_qa_status(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<UpdateQaStatusReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    ensure_post_read_access(&state, &c_pool, post_id, Some(user.id)).await?;
    if !user_can_manage_qa_post(&state, &c_pool, user.id, post_id).await? {
        return Err(AppError::Forbidden(
            "Only Circle moderators, verified experts, or platform admins can update Q&A status."
                .into(),
        ));
    }

    let status = normalize_qa_status(&payload.status)?;
    let mut tx = c_pool.begin().await?;
    use sqlx::Row;
    let post = sqlx::query(
        "SELECT post_type, qa_status FROM posts WHERE id = $1 AND is_hidden = false FOR UPDATE",
    )
    .bind(post_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".into()))?;
    let post_type: String = post.try_get("post_type")?;
    let previous_status: String = post
        .try_get("qa_status")
        .unwrap_or_else(|_| "open".to_string());

    if !is_qa_post_type(&post_type) {
        return Err(AppError::BadRequest(
            "Q&A status can only be set on Question or Due Diligence posts.".into(),
        ));
    }

    if let Some(comment_id) = payload.official_answer_comment_id {
        let belongs_to_post = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM comments WHERE id = $1 AND post_id = $2 AND is_hidden = false)",
        )
        .bind(comment_id)
        .bind(post_id)
        .fetch_one(&mut *tx)
        .await?;
        if !belongs_to_post {
            return Err(AppError::BadRequest(
                "Official answer comment must belong to this post.".into(),
            ));
        }
    }

    sqlx::query(
        r#"
        UPDATE posts
        SET qa_status = $1,
            official_answer_comment_id = COALESCE($2, official_answer_comment_id),
            faq_candidate = COALESCE($3, faq_candidate),
            featured_question = COALESCE($4, featured_question),
            related_resource_url = COALESCE($5, related_resource_url),
            related_asset_id = COALESCE($6, related_asset_id),
            is_locked = CASE WHEN $1 = 'archived' THEN TRUE ELSE is_locked END,
            updated_at = NOW()
        WHERE id = $7
        "#,
    )
    .bind(&status)
    .bind(payload.official_answer_comment_id)
    .bind(payload.faq_candidate)
    .bind(payload.featured_question)
    .bind(payload.related_resource_url.as_deref())
    .bind(payload.related_asset_id)
    .bind(post_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO community_answer_audit_log
          (post_id, comment_id, actor_user_id, action, previous_status, new_status, metadata)
        VALUES ($1, $2, $3, 'qa.status.update', $4, $5, $6)
        "#,
    )
    .bind(post_id)
    .bind(payload.official_answer_comment_id)
    .bind(user.id)
    .bind(&previous_status)
    .bind(&status)
    .bind(serde_json::json!({
        "faq_candidate": payload.faq_candidate,
        "featured_question": payload.featured_question,
        "related_resource_url": payload.related_resource_url,
        "related_asset_id": payload.related_asset_id,
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "post_id": post_id,
        "qa_status": status,
    })))
}

async fn mark_official_answer(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
    Json(payload): Json<MarkOfficialAnswerReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    let mark_official = payload.is_official_answer.unwrap_or(true);
    let mark_verified = payload.is_verified_answer.unwrap_or(mark_official);
    let requested_status = payload
        .qa_status
        .as_deref()
        .map(normalize_qa_status)
        .transpose()?;

    let mut tx = c_pool.begin().await?;
    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT c.post_id, c.is_official_answer, c.is_verified_answer,
               p.post_type, p.qa_status, p.official_answer_comment_id
        FROM comments c
        JOIN posts p ON p.id = c.post_id
        WHERE c.id = $1 AND c.is_hidden = false AND p.is_hidden = false
        FOR UPDATE
        "#,
    )
    .bind(comment_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Comment not found".into()))?;

    let post_id: Uuid = row.try_get("post_id")?;
    let post_type: String = row.try_get("post_type")?;
    let previous_status: String = row
        .try_get("qa_status")
        .unwrap_or_else(|_| "open".to_string());
    let previous_official_comment_id: Option<Uuid> =
        row.try_get("official_answer_comment_id").ok().flatten();
    let previous_official: bool = row.try_get("is_official_answer").unwrap_or(false);
    let previous_verified: bool = row.try_get("is_verified_answer").unwrap_or(false);

    ensure_post_read_access(&state, &c_pool, post_id, Some(user.id)).await?;
    if !is_qa_post_type(&post_type) {
        return Err(AppError::BadRequest(
            "Official answers can only be set on Question or Due Diligence posts.".into(),
        ));
    }
    if !user_can_manage_qa_post(&state, &c_pool, user.id, post_id).await? {
        return Err(AppError::Forbidden(
            "Only Circle moderators, verified experts, or platform admins can mark official answers."
                .into(),
        ));
    }

    let new_status = if mark_official {
        requested_status.unwrap_or_else(|| "official_answer".to_string())
    } else if previous_official_comment_id == Some(comment_id) {
        requested_status.unwrap_or_else(|| "answered".to_string())
    } else {
        previous_status.clone()
    };

    sqlx::query(
        r#"
        UPDATE comments
        SET is_official_answer = $1,
            is_verified_answer = $2,
            answer_marked_by = CASE WHEN $1 OR $2 THEN $3 ELSE answer_marked_by END,
            answer_marked_at = CASE WHEN $1 OR $2 THEN NOW() ELSE answer_marked_at END
        WHERE id = $4
        "#,
    )
    .bind(mark_official)
    .bind(mark_verified)
    .bind(user.id)
    .bind(comment_id)
    .execute(&mut *tx)
    .await?;

    if mark_official {
        sqlx::query(
            "UPDATE posts SET qa_status = $1, official_answer_comment_id = $2, updated_at = NOW() WHERE id = $3",
        )
        .bind(&new_status)
        .bind(comment_id)
        .bind(post_id)
        .execute(&mut *tx)
        .await?;
    } else if previous_official_comment_id == Some(comment_id) {
        sqlx::query(
            "UPDATE posts SET qa_status = $1, official_answer_comment_id = NULL, updated_at = NOW() WHERE id = $2",
        )
        .bind(&new_status)
        .bind(post_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO community_answer_audit_log
          (post_id, comment_id, actor_user_id, action, previous_status, new_status, metadata)
        VALUES ($1, $2, $3, 'qa.official_answer.mark', $4, $5, $6)
        "#,
    )
    .bind(post_id)
    .bind(comment_id)
    .bind(user.id)
    .bind(&previous_status)
    .bind(&new_status)
    .bind(serde_json::json!({
        "previous_official": previous_official,
        "previous_verified": previous_verified,
        "is_official_answer": mark_official,
        "is_verified_answer": mark_verified,
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "post_id": post_id,
        "comment_id": comment_id,
        "qa_status": new_status,
        "is_official_answer": mark_official,
        "is_verified_answer": mark_verified,
    })))
}

async fn get_admin_stats(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    let total_posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(&c_pool)
        .await?;

    let total_comments: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments")
        .fetch_one(&c_pool)
        .await?;

    let total_reactions: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reactions")
        .fetch_one(&c_pool)
        .await?;

    let active_profiles: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM community_profiles")
        .fetch_one(&c_pool)
        .await?;

    let total_circles: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM circles")
        .fetch_one(&c_pool)
        .await?;

    let total_xp: (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(xp_total), 0) FROM community_profiles")
            .fetch_one(&c_pool)
            .await?;

    let pending_reports_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM content_reports WHERE status = 'pending'")
            .fetch_one(&c_pool)
            .await?;

    Ok(Json(serde_json::json!({
        "total_posts": total_posts.0,
        "total_comments": total_comments.0,
        "total_reactions": total_reactions.0,
        "active_profiles": active_profiles.0,
        "total_circles": total_circles.0,
        "total_xp": total_xp.0,
        "pending_reports_count": pending_reports_count.0,
    })))
}

async fn create_user_post(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    if payload.circle_id.is_some() {
        return Err(AppError::BadRequest(
            "Use the Circle post endpoint for Circle-scoped posts.".into(),
        ));
    }

    let (post_id, verified_owner) = create_user_post_for_scope(&state, user, payload).await?;

    Ok(Json(
        serde_json::json!({ "id": post_id, "verified_owner": verified_owner }),
    ))
}

async fn create_circle_post(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(mut payload): Json<CreatePostRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_write_access(&state, &c_pool, circle_id, user.id).await?;
    payload.circle_id = Some(circle_id);

    let (post_id, verified_owner) = create_user_post_for_scope(&state, user, payload).await?;

    Ok(Json(
        serde_json::json!({ "id": post_id, "verified_owner": verified_owner }),
    ))
}

async fn create_user_post_for_scope(
    state: &AppState,
    user: crate::auth::models::User,
    mut payload: CreatePostRequest,
) -> Result<(Uuid, bool), AppError> {
    let c_pool = get_community_pool(state)?;

    let post_type = normalize_post_type(&payload.post_type)?;
    let content_tags = normalize_post_tags(payload.content_tags.take())?;
    ensure_post_taxonomy_allowed(
        state,
        &c_pool,
        user.id,
        payload.circle_id,
        &post_type,
        &content_tags,
    )
    .await?;
    payload.post_type = post_type;
    payload.content_tags = Some(content_tags);

    if let Some(reason) = validation::check_automod(&payload.content) {
        return Err(AppError::Forbidden(format!(
            "Content violation: {}",
            reason
        )));
    }

    // FIX-F7: Check ban before allowing post creation
    check_user_not_banned(&c_pool, user.id).await?;

    // We can assume high_level = false for now until M4 XP system is in place
    let is_high_level_user = false;

    // M3-BE.7 Dynamic Asset-Owner Tags Check — boolean flag, NOT HTML injection (FIX-F4).
    // WS1.3: if the profile has been granted the verified-owner badge via
    // the manual request flow, every new post inherits the flag.
    let mut verified_owner: bool = sqlx::query_scalar(
        "SELECT COALESCE(is_verified_owner, false) FROM community_profiles WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await
    .unwrap_or(None)
    .unwrap_or(false);

    if let Some(aid) = payload.asset_id {
        let is_owner = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM investments WHERE user_id = $1 AND asset_id = $2 AND tokens_owned > 0)"
        )
        .bind(user.id)
        .bind(aid)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

        if is_owner {
            verified_owner = true;
        }
    } else {
        // Fallback: Check if they mention an asset they own
        let owned_assets: Vec<String> = sqlx::query_scalar(
            "SELECT a.title FROM investments i JOIN assets a ON i.asset_id = a.id WHERE i.user_id = $1 AND i.tokens_owned > 0"
        )
        .bind(user.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for name in owned_assets {
            if payload
                .content
                .to_lowercase()
                .contains(&name.to_lowercase())
            {
                verified_owner = true;
                break;
            }
        }
    }

    let post_id = service::create_user_post(
        &c_pool,
        state.redis.as_ref(),
        user.id,
        payload.clone(),
        is_high_level_user,
    )
    .await?;

    if let Some(circle_id) = payload.circle_id {
        let _ =
            record_circle_post_engagement(&c_pool, user.id, circle_id, &payload.post_type).await;
    }

    // Award XP for post creation
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "post_created",
        Some("Created a post"),
        None,
    )
    .await;

    // Getting-Started step 3: first post (once-only bonus on top of post_created).
    let _ = crate::community::xp::award_xp_once(
        &c_pool,
        user.id,
        "first_post",
        Some("Posted your first insight"),
    )
    .await;
    let _ = crate::community::xp::maybe_award_onboarding_complete(&c_pool, user.id).await;

    let author_name = user_bridge::get_user_info(&state.db, state.redis.as_ref(), user.id)
        .await
        .map(|u| u.display_name)
        .unwrap_or_else(|_| "Someone".to_string());

    // Parse and notify mentions asynchronously
    let core_db_clone = state.db.clone();
    let c_pool_clone = c_pool.clone();
    let content_clone = payload.content.clone();
    let c_pool_clone_for_og = c_pool.clone();
    let content_clone_for_og = payload.content.clone();

    tokio::spawn(async move {
        parse_and_notify_mentions(
            core_db_clone,
            c_pool_clone,
            content_clone,
            user.id,
            author_name,
            post_id,
        )
        .await;

        parse_and_store_opengraph(c_pool_clone_for_og, content_clone_for_og, post_id).await;
    });

    Ok((post_id, verified_owner))
}

#[derive(Deserialize)]
pub struct UpdatePostReq {
    pub content: String,
}

async fn update_user_post(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<UpdatePostReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let is_high_level_user = false;

    if let Some(reason) = validation::check_automod(&payload.content) {
        return Err(AppError::Forbidden(format!(
            "Content violation: {}",
            reason
        )));
    }

    service::update_user_post(
        &c_pool,
        post_id,
        user.id,
        payload.content,
        is_high_level_user,
    )
    .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn delete_user_post(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    service::delete_user_post(&c_pool, post_id, user.id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn create_content_report(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<CreateContentReportRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    let report_id =
        service::create_content_report(&c_pool, post_id, user.id, payload.reason, payload.note)
            .await?;

    Ok(Json(serde_json::json!({ "id": report_id })))
}

async fn get_reports(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;

    let c_pool = get_community_pool(&state)?;

    let pending_reports = service::get_pending_reports(&c_pool).await?;

    // Collect distinct reporter IDs and post IDs
    let mut user_ids = std::collections::HashSet::new();
    let mut post_ids = std::collections::HashSet::new();

    for r in &pending_reports {
        user_ids.insert(r.reporter_id);
        post_ids.insert(r.post_id);
    }

    // Fetch posts to get the author IDs and content
    let mut posts_map = std::collections::HashMap::new();
    if !post_ids.is_empty() {
        let p_ids: Vec<Uuid> = post_ids.into_iter().collect();
        let posts: Vec<models::Post> = sqlx::query_as("SELECT * FROM posts WHERE id = ANY($1)")
            .bind(&p_ids)
            .fetch_all(&c_pool)
            .await?;

        for p in posts {
            user_ids.insert(p.user_id);
            posts_map.insert(p.id, p);
        }
    }

    let user_ids_vec: Vec<Uuid> = user_ids.into_iter().collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids_vec).await?;

    let mut response = Vec::with_capacity(pending_reports.len());

    for r in pending_reports {
        let reporter = authors.get(&r.reporter_id);

        let (post_author_id, post_author_name, post_content) =
            if let Some(post) = posts_map.get(&r.post_id) {
                let p_author = authors.get(&post.user_id);
                (
                    post.user_id,
                    p_author
                        .map(|a| a.display_name.clone())
                        .unwrap_or_else(|| "Unknown".into()),
                    post.content_sanitized
                        .clone()
                        .unwrap_or(post.content.clone()),
                )
            } else {
                (
                    Uuid::nil(),
                    "Deleted Post".into(),
                    "[Content Unavailable]".into(),
                )
            };

        response.push(models::AdminReportDisplay {
            id: r.id,
            post_id: r.post_id,
            reporter_id: r.reporter_id,
            reporter_name: reporter
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Unknown".into()),
            post_author_id,
            post_author_name,
            post_content,
            reason: r.reason,
            status: r.status,
            admin_notes: r.admin_notes,
            created_at: r.created_at,
        });
    }

    Ok(Json(response))
}

async fn take_report_action(
    admin: crate::admin::extractors::AdminUser,
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    Json(payload): Json<models::AdminReportActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    require_community_manage(&state, &admin).await?;

    let notes = payload
        .admin_notes
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    if notes.is_empty() {
        return Err(AppError::BadRequest(
            "Admin notes are required.".to_string(),
        ));
    }
    if notes.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "Admin notes must be 1000 characters or fewer.".to_string(),
        ));
    }

    let c_pool = get_community_pool(&state)?;

    service::action_on_report(&c_pool, report_id, admin.user.id, &payload.action, notes).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(Deserialize)]
struct AdminCircleOpsAlertQuery {
    status: Option<String>,
    severity: Option<String>,
    alert_type: Option<String>,
    circle_id: Option<Uuid>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Deserialize)]
struct AdminCircleOpsAlertActionReq {
    action: String,
    note: Option<String>,
    assigned_to_user_id: Option<Uuid>,
    snooze_minutes: Option<i64>,
    workflow_state: Option<String>,
}

fn normalize_ops_alert_workflow_state(value: Option<&str>) -> Result<String, AppError> {
    let normalized = value
        .map(canonical_community_code)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("workflow_state is required.".into()))?;
    match normalized.as_str() {
        "triage"
        | "investigating"
        | "waiting_on_moderator"
        | "waiting_on_policy"
        | "mitigated"
        | "monitoring" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "workflow_state must be triage, investigating, waiting_on_moderator, waiting_on_policy, mitigated, or monitoring."
                .into(),
        )),
    }
}

fn normalize_admin_ops_alert_status(value: Option<&str>) -> Result<String, AppError> {
    let normalized = value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("active")
        .to_ascii_lowercase();
    match normalized.as_str() {
        "active" | "open" | "acknowledged" | "resolved" | "all" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "status must be active, open, acknowledged, resolved, or all".to_string(),
        )),
    }
}

fn normalize_admin_ops_alert_severity(value: Option<&str>) -> Result<String, AppError> {
    let normalized = value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("all")
        .to_ascii_lowercase();
    match normalized.as_str() {
        "all" | "info" | "warning" | "critical" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "severity must be info, warning, critical, or all".to_string(),
        )),
    }
}

fn normalize_admin_ops_alert_type(value: Option<&str>) -> Result<String, AppError> {
    let normalized = value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("all")
        .to_ascii_lowercase();
    match normalized.as_str() {
        "all" | "report_backlog" | "spam_spike" | "failed_worker" | "posting_spike"
        | "moderation_sla" | "notification_delivery" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "alert_type must be report_backlog, spam_spike, failed_worker, posting_spike, moderation_sla, notification_delivery, or all"
                .to_string(),
        )),
    }
}

async fn admin_list_circle_ops_alerts(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminCircleOpsAlertQuery>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    let status = normalize_admin_ops_alert_status(query.status.as_deref())?;
    let severity = normalize_admin_ops_alert_severity(query.severity.as_deref())?;
    let alert_type = normalize_admin_ops_alert_type(query.alert_type.as_deref())?;
    let limit = query
        .limit
        .unwrap_or(ADMIN_CIRCLE_OPS_ALERT_DEFAULT_LIMIT)
        .clamp(1, ADMIN_CIRCLE_OPS_ALERT_MAX_LIMIT);
    let offset = query.offset.unwrap_or(0).max(0);

    let rows = sqlx::query(
        r#"
        SELECT a.id,
               a.circle_id,
               c.name AS circle_name,
               c.slug AS circle_slug,
               a.alert_type,
               a.severity,
               a.status,
               a.summary,
               a.details,
               a.assigned_to_user_id,
               a.escalation_level,
               a.escalated_at,
               a.snoozed_until,
               a.escalation_note,
               a.on_call_notified_at,
               a.workflow_state,
               a.workflow_note,
               a.workflow_updated_at,
               a.workflow_updated_by,
               a.created_at,
               a.resolved_at,
               COUNT(*) OVER()::BIGINT AS total
          FROM circle_ops_alerts a
          LEFT JOIN circles c ON c.id = a.circle_id
         WHERE ($1 = 'all'
                OR ($1 = 'active' AND a.status IN ('open', 'acknowledged'))
                OR a.status = $1)
           AND ($2 = 'all' OR a.severity = $2)
           AND ($3 = 'all' OR a.alert_type = $3)
           AND ($4::UUID IS NULL OR a.circle_id = $4)
         ORDER BY
           CASE
             WHEN a.snoozed_until IS NOT NULL AND a.snoozed_until > NOW() THEN 1
             ELSE 0
           END,
           CASE a.status
             WHEN 'open' THEN 0
             WHEN 'acknowledged' THEN 1
             ELSE 2
           END,
           CASE a.severity
             WHEN 'critical' THEN 0
             WHEN 'warning' THEN 1
             ELSE 2
           END,
           a.escalation_level DESC,
           a.created_at DESC
         LIMIT $5 OFFSET $6
        "#,
    )
    .bind(&status)
    .bind(&severity)
    .bind(&alert_type)
    .bind(query.circle_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let total = rows
        .first()
        .and_then(|row| row.try_get::<i64, _>("total").ok())
        .unwrap_or(0);

    let alerts: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "circle_id": row.try_get::<Option<Uuid>, _>("circle_id").ok().flatten(),
                "circle_name": row.try_get::<Option<String>, _>("circle_name").ok().flatten(),
                "circle_slug": row.try_get::<Option<String>, _>("circle_slug").ok().flatten(),
                "alert_type": row.try_get::<String, _>("alert_type").unwrap_or_default(),
                "severity": row.try_get::<String, _>("severity").unwrap_or_else(|_| "info".to_string()),
                "status": row.try_get::<String, _>("status").unwrap_or_else(|_| "open".to_string()),
                "summary": row.try_get::<String, _>("summary").unwrap_or_default(),
                "details": row.try_get::<serde_json::Value, _>("details").unwrap_or_else(|_| serde_json::json!({})),
                "assigned_to_user_id": row.try_get::<Option<Uuid>, _>("assigned_to_user_id").ok().flatten(),
                "escalation_level": row.try_get::<i32, _>("escalation_level").unwrap_or(0),
                "escalated_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("escalated_at").ok().flatten(),
                "snoozed_until": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("snoozed_until").ok().flatten(),
                "escalation_note": row.try_get::<Option<String>, _>("escalation_note").ok().flatten(),
                "on_call_notified_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("on_call_notified_at").ok().flatten(),
                "workflow_state": row.try_get::<String, _>("workflow_state").unwrap_or_else(|_| "triage".to_string()),
                "workflow_note": row.try_get::<Option<String>, _>("workflow_note").ok().flatten(),
                "workflow_updated_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("workflow_updated_at").ok().flatten(),
                "workflow_updated_by": row.try_get::<Option<Uuid>, _>("workflow_updated_by").ok().flatten(),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
                "resolved_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("resolved_at").ok().flatten(),
            })
        })
        .collect();

    let summary_row = sqlx::query(
        r#"
        SELECT
          COUNT(*) FILTER (WHERE status = 'open')::BIGINT AS open_count,
          COUNT(*) FILTER (WHERE status = 'acknowledged')::BIGINT AS acknowledged_count,
          COUNT(*) FILTER (WHERE severity = 'critical' AND status IN ('open', 'acknowledged'))::BIGINT AS critical_active_count,
          COUNT(*) FILTER (WHERE alert_type = 'failed_worker' AND status IN ('open', 'acknowledged'))::BIGINT AS failed_worker_active_count,
          COUNT(*) FILTER (WHERE escalation_level > 0 AND status IN ('open', 'acknowledged'))::BIGINT AS escalated_active_count,
          COUNT(*) FILTER (WHERE snoozed_until IS NOT NULL AND snoozed_until > NOW() AND status IN ('open', 'acknowledged'))::BIGINT AS snoozed_active_count,
          COUNT(*) FILTER (WHERE workflow_state IN ('waiting_on_moderator', 'waiting_on_policy') AND status IN ('open', 'acknowledged'))::BIGINT AS blocked_workflow_count
          FROM circle_ops_alerts a
         WHERE ($1 = 'all' OR a.severity = $1)
           AND ($2 = 'all' OR a.alert_type = $2)
           AND ($3::UUID IS NULL OR a.circle_id = $3)
        "#,
    )
    .bind(&severity)
    .bind(&alert_type)
    .bind(query.circle_id)
    .fetch_one(&c_pool)
    .await?;

    Ok(Json(serde_json::json!({
        "alerts": alerts,
        "total": total,
        "limit": limit,
        "offset": offset,
        "filters": {
            "status": status,
            "severity": severity,
            "alert_type": alert_type,
            "circle_id": query.circle_id,
        },
        "summary": {
            "open_count": summary_row.try_get::<i64, _>("open_count").unwrap_or(0),
            "acknowledged_count": summary_row.try_get::<i64, _>("acknowledged_count").unwrap_or(0),
            "critical_active_count": summary_row.try_get::<i64, _>("critical_active_count").unwrap_or(0),
            "failed_worker_active_count": summary_row.try_get::<i64, _>("failed_worker_active_count").unwrap_or(0),
            "escalated_active_count": summary_row.try_get::<i64, _>("escalated_active_count").unwrap_or(0),
            "snoozed_active_count": summary_row.try_get::<i64, _>("snoozed_active_count").unwrap_or(0),
            "blocked_workflow_count": summary_row.try_get::<i64, _>("blocked_workflow_count").unwrap_or(0),
        }
    })))
}

async fn admin_take_circle_ops_alert_action(
    admin: crate::admin::extractors::AdminUser,
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(alert_id): Path<Uuid>,
    Json(payload): Json<AdminCircleOpsAlertActionReq>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    require_community_manage(&state, &admin).await?;

    let action = payload.action.trim().to_ascii_lowercase();
    match action.as_str() {
        "acknowledge"
        | "resolve"
        | "assign"
        | "escalate"
        | "snooze"
        | "unsnooze"
        | "mark_on_call_notified"
        | "set_workflow_state" => {}
        _ => {
            return Err(AppError::BadRequest(
                "Circle ops alert action must be acknowledge, resolve, assign, escalate, snooze, unsnooze, mark_on_call_notified, or set_workflow_state."
                    .into(),
            ));
        }
    };
    let note = payload.note.unwrap_or_default().trim().to_string();
    if note.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "Alert action note must be 1000 characters or fewer.".into(),
        ));
    }
    let assigned_to_user_id = if action == "assign" {
        let Some(user_id) = payload.assigned_to_user_id else {
            return Err(AppError::BadRequest(
                "assigned_to_user_id is required for assign actions.".into(),
            ));
        };
        let target_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status <> 'deleted')",
        )
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
        if !target_exists {
            return Err(AppError::NotFound("Assigned user not found.".into()));
        }
        Some(user_id)
    } else {
        None
    };
    let snooze_minutes = if action == "snooze" {
        let minutes = payload.snooze_minutes.ok_or_else(|| {
            AppError::BadRequest("snooze_minutes is required for snooze actions.".into())
        })?;
        if !(5..=10_080).contains(&minutes) {
            return Err(AppError::BadRequest(
                "snooze_minutes must be between 5 and 10080.".into(),
            ));
        }
        Some(minutes as i32)
    } else {
        None
    };
    let workflow_state = if action == "set_workflow_state" {
        Some(normalize_ops_alert_workflow_state(
            payload.workflow_state.as_deref(),
        )?)
    } else {
        None
    };

    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;
    let actor_id = admin.user.id.to_string();
    let row = match action.as_str() {
        "acknowledge" | "resolve" => {
            let new_status = if action == "acknowledge" {
                "acknowledged"
            } else {
                "resolved"
            };
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET status = $2,
                       resolved_at = CASE WHEN $2 = 'resolved' THEN NOW() ELSE resolved_at END,
                       snoozed_until = CASE WHEN $2 = 'resolved' THEN NULL ELSE snoozed_until END,
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $3,
                         'last_platform_action_note', NULLIF($4, ''),
                         'last_platform_action_by', $5,
                         'last_platform_action_at', NOW()
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(new_status)
            .bind(&action)
            .bind(&note)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        "assign" => {
            let user_id = assigned_to_user_id.expect("assign action validated assigned user");
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET assigned_to_user_id = $2,
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $3,
                         'last_platform_action_note', NULLIF($4, ''),
                         'last_platform_action_by', $5,
                         'last_platform_action_at', NOW(),
                         'assigned_to_user_id', $2::TEXT
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(user_id)
            .bind(&action)
            .bind(&note)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        "escalate" => {
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET escalation_level = LEAST(escalation_level + 1, 5),
                       escalated_at = NOW(),
                       escalation_note = NULLIF($2, ''),
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $3,
                         'last_platform_action_note', NULLIF($2, ''),
                         'last_platform_action_by', $4,
                         'last_platform_action_at', NOW(),
                         'escalation_level', LEAST(escalation_level + 1, 5)
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(&note)
            .bind(&action)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        "snooze" => {
            let minutes = snooze_minutes.expect("snooze action validated minutes");
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET snoozed_until = NOW() + ($2::INT * INTERVAL '1 minute'),
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $3,
                         'last_platform_action_note', NULLIF($4, ''),
                         'last_platform_action_by', $5,
                         'last_platform_action_at', NOW(),
                         'snooze_minutes', $2
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(minutes)
            .bind(&action)
            .bind(&note)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        "unsnooze" => {
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET snoozed_until = NULL,
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $2,
                         'last_platform_action_note', NULLIF($3, ''),
                         'last_platform_action_by', $4,
                         'last_platform_action_at', NOW()
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(&action)
            .bind(&note)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        "mark_on_call_notified" => {
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET on_call_notified_at = NOW(),
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $2,
                         'last_platform_action_note', NULLIF($3, ''),
                         'last_platform_action_by', $4,
                         'last_platform_action_at', NOW()
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(&action)
            .bind(&note)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        "set_workflow_state" => {
            let workflow_state = workflow_state
                .as_deref()
                .expect("workflow action validated state");
            sqlx::query(
                r#"
                UPDATE circle_ops_alerts
                   SET workflow_state = $2,
                       workflow_note = NULLIF($3, ''),
                       workflow_updated_at = NOW(),
                       workflow_updated_by = $4,
                       status = CASE WHEN status = 'open' THEN 'acknowledged' ELSE status END,
                       details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                         'last_platform_action', $5,
                         'last_platform_action_note', NULLIF($3, ''),
                         'last_platform_action_by', $6,
                         'last_platform_action_at', NOW(),
                         'workflow_state', $2
                       )
                 WHERE id = $1
                   AND status IN ('open', 'acknowledged')
                 RETURNING alert_type,
                           severity,
                           status,
                           circle_id,
                           summary,
                           assigned_to_user_id,
                           escalation_level,
                           snoozed_until,
                           on_call_notified_at,
                           workflow_state,
                           workflow_note,
                           workflow_updated_at,
                           workflow_updated_by
                "#,
            )
            .bind(alert_id)
            .bind(workflow_state)
            .bind(&note)
            .bind(admin.user.id)
            .bind(&action)
            .bind(&actor_id)
            .fetch_optional(&mut *tx)
            .await?
        }
        _ => unreachable!("action allowlist validated above"),
    }
    .ok_or_else(|| AppError::NotFound("Circle ops alert not found.".into()))?;

    let alert_type = row.try_get::<String, _>("alert_type").unwrap_or_default();
    let severity = row.try_get::<String, _>("severity").unwrap_or_default();
    let updated_status = row
        .try_get::<String, _>("status")
        .unwrap_or_else(|_| "open".into());
    let circle_id = row.try_get::<Option<Uuid>, _>("circle_id").ok().flatten();
    let summary = row.try_get::<String, _>("summary").unwrap_or_default();
    let updated_assignee = row
        .try_get::<Option<Uuid>, _>("assigned_to_user_id")
        .ok()
        .flatten();
    let escalation_level = row.try_get::<i32, _>("escalation_level").unwrap_or(0);
    let snoozed_until = row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("snoozed_until")
        .ok()
        .flatten();
    let on_call_notified_at = row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("on_call_notified_at")
        .ok()
        .flatten();
    let workflow_state = row
        .try_get::<String, _>("workflow_state")
        .unwrap_or_else(|_| "triage".to_string());
    let workflow_note = row
        .try_get::<Option<String>, _>("workflow_note")
        .ok()
        .flatten();
    let workflow_updated_at = row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("workflow_updated_at")
        .ok()
        .flatten();
    let workflow_updated_by = row
        .try_get::<Option<Uuid>, _>("workflow_updated_by")
        .ok()
        .flatten();
    let fanout_queued = matches!(action.as_str(), "escalate" | "mark_on_call_notified");
    if fanout_queued {
        crate::community::background::enqueue_circle_ops_alert_notification_tx(
            &mut tx,
            alert_id,
            &action,
            updated_assignee,
            serde_json::json!({
                "circle_id": circle_id,
                "alert_type": alert_type,
                "severity": severity,
                "status": updated_status,
                "summary": summary,
                "trigger_action": action,
                "assigned_to_user_id": updated_assignee,
                "escalation_level": escalation_level,
                "snoozed_until": snoozed_until,
                "on_call_notified_at": on_call_notified_at,
                "workflow_state": workflow_state,
                "workflow_updated_at": workflow_updated_at,
                "workflow_updated_by": workflow_updated_by,
                "platform_scope": true,
            }),
        )
        .await?;
    }

    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        &format!("platform.circle_ops_alert.{}", action),
        "circle_ops_alert",
        Some(alert_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "alert_type": alert_type,
            "severity": severity,
            "status": updated_status,
            "summary": summary,
            "has_note": !note.is_empty(),
            "assigned_to_user_id": updated_assignee,
            "escalation_level": escalation_level,
            "snoozed_until": snoozed_until,
            "on_call_notified_at": on_call_notified_at,
            "workflow_state": workflow_state,
            "workflow_updated_at": workflow_updated_at,
            "workflow_updated_by": workflow_updated_by,
            "has_workflow_note": workflow_note.is_some(),
            "fanout_queued": fanout_queued,
            "platform_scope": true,
        }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "status": updated_status,
        "assigned_to_user_id": updated_assignee,
        "escalation_level": escalation_level,
        "snoozed_until": snoozed_until,
        "on_call_notified_at": on_call_notified_at,
        "workflow_state": workflow_state,
        "workflow_updated_at": workflow_updated_at,
        "workflow_updated_by": workflow_updated_by,
        "fanout_queued": fanout_queued,
    })))
}

// ─── ADMIN CHALLENGES ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CreateChallengeReq {
    pub title: String,
    pub description: String,
    pub xp_reward: i32,
    pub badge_reward: Option<String>,
    pub requirement_type: String, // e.g., "buy_asset", "write_review", "login_streak"
    pub requirement_value: i32,
    pub frequency: String, // "one_time", "daily", "weekly"
}

async fn admin_list_challenges(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await
    {
        return Err(AppError::Forbidden(
            "Missing permission: community.manage".to_string(),
        ));
    }

    let c_pool = get_community_pool(&state)?;

    let challenges: Vec<crate::community::challenges::Challenge> =
        sqlx::query_as("SELECT * FROM challenges ORDER BY created_at DESC")
            .fetch_all(&c_pool)
            .await
            .map_err(AppError::Database)?;

    Ok(Json(challenges))
}

async fn admin_create_challenge(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateChallengeReq>,
) -> Result<impl IntoResponse, AppError> {
    if !crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await
    {
        return Err(AppError::Forbidden(
            "Missing permission: community.manage".to_string(),
        ));
    }

    let c_pool = get_community_pool(&state)?;

    let challenge = crate::community::challenges::admin_create_challenge(
        &c_pool,
        &payload.title,
        &payload.description,
        payload.xp_reward,
        payload.badge_reward.as_deref(),
        &payload.requirement_type,
        payload.requirement_value,
        &payload.frequency,
    )
    .await?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "challenge.create",
        "challenge",
        Some(challenge.id),
        None,
        Some(serde_json::json!({
            "title": &challenge.title,
            "requirement_type": &challenge.requirement_type,
            "requirement_value": challenge.requirement_value,
            "frequency": &challenge.frequency,
            "xp_reward": challenge.xp_reward,
            "badge_reward": &challenge.badge_reward,
            "is_active": challenge.is_active
        })),
    )
    .await;

    Ok(Json(challenge))
}

#[derive(serde::Deserialize)]
pub struct ToggleChallengeReq {
    pub is_active: bool,
}

async fn admin_toggle_challenge(
    admin: crate::admin::extractors::AdminUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(payload): Json<ToggleChallengeReq>,
) -> Result<impl IntoResponse, AppError> {
    if !crate::auth::middleware::has_permission(&state.db, admin.user.id, "community.manage").await
    {
        return Err(AppError::Forbidden(
            "Missing permission: community.manage".to_string(),
        ));
    }

    let c_pool = get_community_pool(&state)?;

    let challenge =
        crate::community::challenges::admin_toggle_challenge(&c_pool, id, payload.is_active)
            .await?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "challenge.toggle",
        "challenge",
        Some(challenge.id),
        None,
        Some(serde_json::json!({
            "is_active": challenge.is_active,
            "title": &challenge.title
        })),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "success": true, "challenge": challenge }),
    ))
}

#[derive(serde::Serialize)]
pub struct TrendingAssetDisplay {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub slug: String,
    pub asset_type: String,
    pub detail_url: String,
    pub post_count: i64,
}

async fn get_trending_assets(
    State(state): State<AppState>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let trending = service::get_trending_assets(&c_pool).await?;

    if trending.is_empty() {
        return Ok(Json(Vec::<TrendingAssetDisplay>::new()));
    }

    let asset_ids: Vec<Uuid> = trending.iter().map(|(id, _)| *id).collect();

    let assets: Vec<(Uuid, String, String, String, String)> = sqlx::query_as(
        "SELECT id, title, COALESCE(NULLIF(UPPER(LEFT(REPLACE(slug, '-', ''), 8)), ''), 'ASSET') AS symbol, slug, asset_type FROM assets WHERE id = ANY($1)",
    )
    .bind(&asset_ids)
    .fetch_all(&state.db)
    .await?;

    let mut asset_map = std::collections::HashMap::new();
    for a in assets {
        asset_map.insert(a.0, (a.1, a.2, a.3, a.4));
    }

    let mut result = Vec::new();
    for (id, count) in trending {
        if let Some((name, symbol, slug, asset_type)) = asset_map.get(&id) {
            let detail_url = if asset_type == "commodity" {
                format!("/commodity/{}", slug)
            } else {
                format!("/property/{}", slug)
            };
            result.push(TrendingAssetDisplay {
                id,
                name: name.clone(),
                symbol: symbol.clone(),
                slug: slug.clone(),
                asset_type: asset_type.clone(),
                detail_url,
                post_count: count,
            });
        }
    }

    Ok(Json(result))
}

// --- Admin Posts & Users API ---

#[derive(serde::Serialize)]
pub struct AdminPostDisplay {
    pub id: Uuid,
    pub user_id: Uuid,
    pub author_name: String,
    pub post_type: String,
    pub content: String,
    pub is_pinned: bool,
    pub is_hidden: bool,
    pub is_locked: bool,
    pub hidden_reason: Option<String>,
    pub content_tags: Option<Vec<String>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn admin_get_posts(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    let posts: Vec<models::Post> =
        sqlx::query_as("SELECT * FROM posts ORDER BY created_at DESC LIMIT 200")
            .fetch_all(&c_pool)
            .await?;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

    let mut result = Vec::new();
    for p in posts {
        let author_name = authors
            .get(&p.user_id)
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Unknown".into());

        result.push(AdminPostDisplay {
            id: p.id,
            user_id: p.user_id,
            author_name,
            post_type: p.post_type.clone(),
            content: p.content_sanitized.clone().unwrap_or(p.content.clone()),
            is_pinned: p.is_pinned,
            is_hidden: p.is_hidden,
            is_locked: p.is_locked.unwrap_or(false),
            hidden_reason: p.hidden_reason.clone(),
            content_tags: p.content_tags.clone(),
            created_at: p.created_at,
        });
    }

    Ok(Json(result))
}

#[derive(serde::Deserialize)]
pub struct HidePostPayload {
    pub reason: String,
}

async fn admin_hide_post(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<HidePostPayload>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE posts SET is_hidden = true, hidden_reason = $1 WHERE id = $2")
        .bind(&payload.reason)
        .bind(post_id)
        .execute(&c_pool)
        .await?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "post.hide",
        "post",
        Some(post_id),
        None,
        Some(serde_json::json!({"reason": payload.reason})),
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct ToggleLockPayload {
    pub is_locked: bool,
}

async fn admin_toggle_lock_post(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<ToggleLockPayload>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE posts SET is_locked = $1 WHERE id = $2")
        .bind(payload.is_locked)
        .bind(post_id)
        .execute(&c_pool)
        .await?;

    let action = if payload.is_locked {
        "post.lock"
    } else {
        "post.unlock"
    };
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        action,
        "post",
        Some(post_id),
        None,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct UpdateTagsPayload {
    pub tags: Vec<String>,
}

async fn admin_update_post_tags(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<UpdateTagsPayload>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE posts SET content_tags = $1 WHERE id = $2")
        .bind(&payload.tags)
        .bind(post_id)
        .execute(&c_pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn admin_get_post_detail(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    // 1. Fetch Post
    let p: models::Post = sqlx::query_as("SELECT * FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&c_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;

    // 2. Fetch Comments
    let comments: Vec<models::Comment> =
        sqlx::query_as("SELECT * FROM comments WHERE post_id = $1 ORDER BY created_at ASC")
            .bind(post_id)
            .fetch_all(&c_pool)
            .await?;

    // 3. Fetch Reactions
    let reactions: Vec<models::Reaction> =
        sqlx::query_as("SELECT * FROM reactions WHERE post_id = $1 ORDER BY created_at DESC")
            .bind(post_id)
            .fetch_all(&c_pool)
            .await?;

    // 4. Fetch Reports
    let reports: Vec<models::ContentReport> =
        sqlx::query_as("SELECT * FROM content_reports WHERE post_id = $1 ORDER BY created_at DESC")
            .bind(post_id)
            .fetch_all(&c_pool)
            .await?;

    // Collect all unique user IDs to fetch names
    let mut user_ids = std::collections::HashSet::new();
    user_ids.insert(p.user_id);
    for c in &comments {
        user_ids.insert(c.user_id);
    }
    for r in &reactions {
        user_ids.insert(r.user_id);
    }
    for rep in &reports {
        user_ids.insert(rep.reporter_id);
    }

    let user_ids_vec: Vec<Uuid> = user_ids.into_iter().collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids_vec).await?;

    // Build the post
    let post_author_name = authors
        .get(&p.user_id)
        .map(|a| a.display_name.clone())
        .unwrap_or_else(|| "Unknown".into());

    let post_display = AdminPostDisplay {
        id: p.id,
        user_id: p.user_id,
        author_name: post_author_name,
        post_type: p.post_type.clone(),
        content: p.content_sanitized.clone().unwrap_or(p.content.clone()),
        is_pinned: p.is_pinned,
        is_hidden: p.is_hidden,
        is_locked: p.is_locked.unwrap_or(false),
        hidden_reason: p.hidden_reason.clone(),
        content_tags: p.content_tags.clone(),
        created_at: p.created_at,
    };

    // Format Comments
    let mut comments_display = Vec::new();
    for c in comments {
        let name = authors
            .get(&c.user_id)
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Unknown".into());
        comments_display.push(serde_json::json!({
            "id": c.id,
            "user_id": c.user_id,
            "author_name": name,
            "content": c.content_sanitized.unwrap_or(c.content),
            "is_hidden": c.is_hidden,
            "is_pinned": c.is_pinned.unwrap_or(false),
            "created_at": c.created_at,
        }));
    }

    // Format Reactions
    let mut reactions_display = Vec::new();
    for r in reactions {
        let name = authors
            .get(&r.user_id)
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Unknown".into());
        reactions_display.push(serde_json::json!({
            "id": r.id,
            "user_id": r.user_id,
            "author_name": name,
            "reaction_type": r.reaction_type,
            "created_at": r.created_at,
        }));
    }

    // Format Reports
    let mut reports_display = Vec::new();
    for rep in reports {
        let name = authors
            .get(&rep.reporter_id)
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Unknown".into());
        reports_display.push(serde_json::json!({
            "id": rep.id,
            "reporter_id": rep.reporter_id,
            "reporter_name": name,
            "reason": rep.reason,
            "status": rep.status,
            "admin_notes": rep.admin_notes,
            "created_at": rep.created_at,
        }));
    }

    Ok(Json(serde_json::json!({
        "post": post_display,
        "comments": comments_display,
        "reactions": reactions_display,
        "reports": reports_display,
    })))
}

#[derive(serde::Serialize)]
pub struct AdminUserDisplay {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_community_banned: bool,
    pub ban_reason: Option<String>,
    pub warning_count: i32,
    pub post_count: i32,
    pub mod_notes: Option<String>,
    pub muted_until: Option<chrono::DateTime<chrono::Utc>>,
    pub is_shadowbanned: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn admin_get_users(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    // Use query! structure with an anonymous record but dynamic execute
    use sqlx::Row;
    let rows = sqlx::query("SELECT * FROM community_profiles ORDER BY created_at DESC")
        .fetch_all(&c_pool)
        .await?;

    let mut user_ids = Vec::new();
    for row in &rows {
        let u_id: Uuid = row.try_get("user_id")?;
        user_ids.push(u_id);
    }

    let core_users =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

    let mut result = Vec::new();
    for row in rows {
        let u_id: Uuid = row.try_get("user_id")?;
        let is_community_banned: bool = row.try_get("is_community_banned")?;
        let ban_reason: Option<String> = row.try_get("ban_reason")?;
        let warning_count: i32 = row.try_get("warning_count")?;
        let post_count: i32 = row.try_get("post_count")?;
        let mod_notes: Option<String> = row.try_get("mod_notes")?;
        let muted_until: Option<chrono::DateTime<chrono::Utc>> = row.try_get("muted_until")?;
        let is_shadowbanned: bool = row.try_get("is_shadowbanned").unwrap_or(false);
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;

        let user_info = core_users.get(&u_id);

        result.push(AdminUserDisplay {
            user_id: u_id,
            display_name: user_info
                .map(|u| u.display_name.clone())
                .unwrap_or_else(|| "Unknown".into()),
            avatar_url: user_info.and_then(|u| u.avatar_url.clone()),
            is_community_banned,
            ban_reason,
            warning_count,
            post_count,
            mod_notes,
            muted_until,
            is_shadowbanned,
            created_at,
        });
    }

    Ok(Json(result))
}

#[derive(serde::Deserialize)]
pub struct BanUserPayload {
    pub reason: Option<String>,
    pub is_banned: bool,
}

async fn admin_toggle_ban_user(
    admin: crate::admin::extractors::AdminUser,
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<BanUserPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    require_csrf_header(&headers, &jar)?;
    let c_pool = get_community_pool(&state)?;
    let reason = payload
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(str::to_string);
    if payload.is_banned && reason.is_none() {
        return Err(AppError::BadRequest("Ban reason is required.".to_string()));
    }
    if reason
        .as_ref()
        .map(|reason| reason.chars().count() > 1000)
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "Ban reason must be 1000 characters or fewer.".to_string(),
        ));
    }

    use sqlx::Row;
    let mut tx = c_pool.begin().await?;
    let profile = sqlx::query(
        "SELECT is_community_banned, ban_reason FROM community_profiles WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Community user not found.".to_string()))?;

    let previous_is_banned: bool = profile.try_get("is_community_banned")?;
    let previous_ban_reason: Option<String> = profile.try_get("ban_reason")?;

    let result = sqlx::query("UPDATE community_profiles SET is_community_banned = $1, ban_reason = $2, updated_at = NOW() WHERE user_id = $3")
        .bind(payload.is_banned)
        .bind(&reason)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Community user not found.".to_string()));
    }

    let action = if payload.is_banned {
        "user.ban"
    } else {
        "user.unban"
    };
    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        action,
        "user",
        None,
        Some(user_id),
        serde_json::json!({
            "previous_profile": {
                "is_community_banned": previous_is_banned,
                "ban_reason": previous_ban_reason,
            },
            "new_profile": {
                "is_community_banned": payload.is_banned,
                "ban_reason": reason,
            }
        }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct MuteUserPayload {
    pub hours: Option<i32>, // If None, unmute
}

async fn admin_mute_user(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<MuteUserPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    if let Some(hours) = payload.hours {
        if hours <= 0 || hours > 24 * 365 {
            return Err(AppError::BadRequest(
                "Mute duration must be between 1 hour and 1 year.".to_string(),
            ));
        }
    }

    let muted_until = payload
        .hours
        .map(|h| chrono::Utc::now() + chrono::Duration::hours(h as i64));

    let mut tx = c_pool.begin().await?;
    let result = sqlx::query("UPDATE community_profiles SET muted_until = $1 WHERE user_id = $2")
        .bind(muted_until)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Community user not found.".to_string()));
    }

    let action = if payload.hours.is_some() {
        "user.mute"
    } else {
        "user.unmute"
    };
    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        action,
        "user",
        None,
        Some(user_id),
        serde_json::json!({"hours": payload.hours}),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct ShadowbanPayload {
    pub is_shadowbanned: bool,
}

async fn admin_toggle_shadowban(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<ShadowbanPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    let mut tx = c_pool.begin().await?;
    let result =
        sqlx::query("UPDATE community_profiles SET is_shadowbanned = $1 WHERE user_id = $2")
            .bind(payload.is_shadowbanned)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Community user not found.".to_string()));
    }

    let action = if payload.is_shadowbanned {
        "user.shadowban"
    } else {
        "user.unshadowban"
    };
    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        action,
        "user",
        None,
        Some(user_id),
        serde_json::json!({"is_shadowbanned": payload.is_shadowbanned}),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct WarnUserPayload {
    pub reason: String,
}

async fn admin_warn_user(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<WarnUserPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    let reason = payload.reason.trim();
    if reason.is_empty() || reason.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "Warning reason is required and must be 1000 characters or fewer.".to_string(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let result = sqlx::query(
        "UPDATE community_profiles SET warning_count = warning_count + 1 WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Community user not found.".to_string()));
    }

    // WS1.6: admin warnings bypass notification_preferences on purpose —
    // a user can't opt out of receiving a moderator warning about their
    // own behaviour. Direct INSERT (inside the audit transaction) instead
    // of notify_user() which would honour prefs.
    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, actor_id, type, entity_id, content, link_url)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(Some(admin.user.id))
    .bind("system_alert")
    .bind(Option::<Uuid>::None)
    .bind(format!("Warning from Admin: {reason}"))
    .bind(Option::<String>::None)
    .execute(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        "user.warn",
        "user",
        None,
        Some(user_id),
        serde_json::json!({"reason": reason}),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct UpdateModNotesPayload {
    pub notes: String,
}

async fn admin_update_mod_notes(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateModNotesPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    if payload.notes.chars().count() > 5000 {
        return Err(AppError::BadRequest(
            "Moderator notes must be 5000 characters or fewer.".to_string(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let result = sqlx::query("UPDATE community_profiles SET mod_notes = $1 WHERE user_id = $2")
        .bind(&payload.notes)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Community user not found.".to_string()));
    }

    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        "user.mod_notes.update",
        "user",
        None,
        Some(user_id),
        serde_json::json!({"notes_length": payload.notes.chars().count()}),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Serialize)]
pub struct AdminCommentDisplay {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub author_name: String,
    pub content: String,
    pub helpful_count: i32,
    pub is_hidden: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct AdminCommentsQuery {
    limit: Option<i64>,
}

async fn admin_get_comments(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminCommentsQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;
    let limit = query
        .limit
        .unwrap_or(ADMIN_COMMENTS_DEFAULT_LIMIT)
        .clamp(1, ADMIN_COMMENTS_MAX_LIMIT);
    let c_pool = get_community_pool(&state)?;

    let comments: Vec<models::Comment> = sqlx::query_as(
        "SELECT * FROM comments ORDER BY created_at DESC LIMIT $1", // show hidden comments too
    )
    .bind(limit)
    .fetch_all(&c_pool)
    .await?;

    let user_ids: Vec<Uuid> = comments.iter().map(|c| c.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

    let mut result = Vec::with_capacity(comments.len());
    for c in comments {
        let author_name = authors
            .get(&c.user_id)
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Unknown".into());

        result.push(AdminCommentDisplay {
            id: c.id,
            post_id: c.post_id,
            user_id: c.user_id,
            author_name,
            content: c.content_sanitized.clone().unwrap_or(c.content.clone()),
            helpful_count: c.helpful_count,
            is_hidden: c.is_hidden,
            created_at: c.created_at,
        });
    }

    Ok(Json(result))
}

#[derive(serde::Deserialize)]
pub struct HideCommentPayload {
    pub reason: Option<String>,
}

async fn admin_hide_comment(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
    Json(payload): Json<HideCommentPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;
    let comment: (Uuid, bool, String, Option<String>) = sqlx::query_as(
        "SELECT user_id, is_hidden, content, content_sanitized FROM comments WHERE id = $1 FOR UPDATE",
    )
    .bind(comment_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Comment not found.".to_string()))?;

    sqlx::query("UPDATE comments SET is_hidden = true WHERE id = $1")
        .bind(comment_id)
        .execute(&mut *tx)
        .await?;

    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        "comment.hide",
        "comment",
        Some(comment_id),
        Some(comment.0),
        serde_json::json!({
            "reason": payload.reason.as_deref().unwrap_or("Admin hide"),
            "previous_is_hidden": comment.1,
            "content_preview": comment.3.as_deref().unwrap_or(&comment.2).chars().take(160).collect::<String>(),
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn admin_delete_comment(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;
    let comment: (Uuid, Uuid, bool, String, Option<String>) = sqlx::query_as(
        "SELECT post_id, user_id, is_hidden, content, content_sanitized FROM comments WHERE id = $1 FOR UPDATE",
    )
    .bind(comment_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Comment not found.".to_string()))?;

    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&mut *tx)
        .await?;

    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        "comment.delete",
        "comment",
        Some(comment_id),
        Some(comment.1),
        serde_json::json!({
            "post_id": comment.0,
            "previous_is_hidden": comment.2,
            "content_preview": comment.4.as_deref().unwrap_or(&comment.3).chars().take(160).collect::<String>(),
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct TogglePinCommentPayload {
    pub is_pinned: bool,
}

async fn admin_toggle_pin_comment(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
    Json(payload): Json<TogglePinCommentPayload>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;
    let comment: (Uuid, Option<bool>) =
        sqlx::query_as("SELECT user_id, is_pinned FROM comments WHERE id = $1 FOR UPDATE")
            .bind(comment_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found.".to_string()))?;

    sqlx::query("UPDATE comments SET is_pinned = $1 WHERE id = $2")
        .bind(payload.is_pinned)
        .bind(comment_id)
        .execute(&mut *tx)
        .await?;

    let action = if payload.is_pinned {
        "comment.pin"
    } else {
        "comment.unpin"
    };
    log_community_admin_action_tx(
        &mut tx,
        admin.user.id,
        action,
        "comment",
        Some(comment_id),
        Some(comment.0),
        serde_json::json!({
            "previous_is_pinned": comment.1.unwrap_or(false),
            "is_pinned": payload.is_pinned,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Feed & Filter
        .route("/api/community/feed", get(get_feed))
        .route("/api/community/search", get(search_community))
        .route("/api/community/trending-assets", get(get_trending_assets))
        // Announcements
        .route(
            "/api/admin/community/announcements",
            get(admin_list_announcements).post(create_announcement),
        )
        // User Posts
        .route("/api/community/posts", post(create_user_post))
        .route(
            "/api/community/posts/:id",
            axum::routing::get(get_post_detail)
                .put(update_user_post)
                .delete(delete_user_post),
        )
        .route(
            "/api/community/posts/:id/report",
            post(create_content_report),
        )
        .route(
            "/api/community/posts/:id/qa-status",
            put(update_post_qa_status),
        )
        // Reactions
        .route("/api/community/posts/:id/reactions", post(toggle_reaction))
        // Comments
        .route(
            "/api/community/posts/:id/comments",
            get(get_comments).post(create_comment),
        )
        // 14.8.5: comment edit (own)
        .route(
            "/api/community/comments/:id",
            put(update_own_comment).delete(delete_own_comment),
        )
        // 14.8.6: comment reactions
        .route(
            "/api/community/comments/:id/reactions",
            post(toggle_comment_reaction),
        )
        .route(
            "/api/community/comments/:id/official-answer",
            post(mark_official_answer),
        )
        // Admin Stats & Moderation
        .route("/api/admin/community/stats", get(get_admin_stats))
        .route("/api/admin/community/reports", get(get_reports))
        .route(
            "/api/admin/community/reports/:id/action",
            post(take_report_action),
        )
        .route(
            "/api/admin/community/ops-alerts",
            get(admin_list_circle_ops_alerts),
        )
        .route(
            "/api/admin/community/ops-alerts/:id/action",
            post(admin_take_circle_ops_alert_action),
        )
        .route("/api/admin/community/posts", get(admin_get_posts))
        .route("/api/admin/community/posts/:id", get(admin_get_post_detail))
        .route("/api/admin/community/posts/:id/hide", post(admin_hide_post))
        .route(
            "/api/admin/community/posts/:id/lock",
            post(admin_toggle_lock_post),
        )
        .route(
            "/api/admin/community/posts/:id/tags",
            post(admin_update_post_tags),
        )
        .route("/api/admin/community/users", get(admin_get_users))
        .route(
            "/api/admin/community/users/:id/ban",
            post(admin_toggle_ban_user),
        )
        .route("/api/community/appeals", post(submit_ban_appeal))
        .route("/api/admin/community/appeals", get(get_ban_appeals))
        .route(
            "/api/admin/community/appeals/:id/review",
            post(review_ban_appeal),
        )
        .route("/api/admin/community/users/:id/warn", post(admin_warn_user))
        .route(
            "/api/admin/community/users/:id/mod-notes",
            post(admin_update_mod_notes),
        )
        .route("/api/admin/community/users/:id/mute", post(admin_mute_user))
        .route(
            "/api/admin/community/users/:id/shadowban",
            post(admin_toggle_shadowban),
        )
        .route("/api/admin/community/comments", get(admin_get_comments))
        .route(
            "/api/admin/community/comments/:id",
            delete(admin_delete_comment),
        )
        .route(
            "/api/admin/community/comments/:id/hide",
            post(admin_hide_comment),
        )
        .route(
            "/api/admin/community/comments/:id/pin",
            post(admin_toggle_pin_comment),
        )
        // Social Layer
        .route("/api/community/profile/me", get(get_profile_me))
        // Phase 3 task 30: viewer's own moderation history.
        .route(
            "/api/community/profile/me/moderation-log",
            get(get_my_moderation_log),
        )
        // Phase 3 task 31: notification preferences.
        .route(
            "/api/community/notifications/preferences",
            get(get_notification_preferences).put(update_notification_preferences),
        )
        // Phase 3 task 32 verified-owner badge request flow REMOVED 2026-05-15.
        // Replaced by the asset-linked /api/community/verified-owner-requests
        // flow (14.8.16) — see `submit_verified_owner_request` below.
        .route("/api/community/profile", put(update_profile))
        .route("/api/community/profile/:id", get(get_profile))
        // Phase 3 task 20: followers / following list views.
        .route("/api/community/profile/:id/followers", get(list_followers))
        .route("/api/community/profile/:id/following", get(list_following))
        // WS3.1 — per-user community profile data.
        .route("/api/community/profile/:id/posts", get(list_user_posts))
        .route(
            "/api/community/profile/:id/comments",
            get(list_user_comments),
        )
        .route("/api/community/profile/:id/media", get(list_user_media))
        .route(
            "/api/community/profile/:id/activity",
            get(list_user_activity),
        )
        .route("/api/community/profile/me/analytics", get(get_my_analytics))
        .route("/api/community/follow/:id", post(follow_user))
        .route("/api/community/follow/:id", delete(unfollow_user))
        // Block / mute self-service (14.8.2)
        .route(
            "/api/community/users/:id/block",
            post(block_user).delete(unblock_user),
        )
        .route(
            "/api/community/users/:id/mute",
            post(mute_user).delete(unmute_user),
        )
        .route("/api/community/blocks", get(list_blocks))
        .route("/api/community/mutes", get(list_mutes))
        // XP System (M4)
        .route("/api/community/xp", get(get_xp_summary))
        .route("/api/community/xp/history", get(get_xp_history))
        // Circles (M4)
        .route("/api/community/circles", post(create_circle))
        .route("/api/community/circles/me", get(get_my_circle))
        .route(
            "/api/community/circles/leaderboard",
            get(get_circle_leaderboard),
        )
        // Phase 3 task 25: global user XP leaderboard.
        .route("/api/community/leaderboard", get(get_global_leaderboard))
        .route("/api/community/circles/:id", get(get_circle_detail))
        .route(
            "/api/community/circles/:id",
            put(update_circle).delete(delete_own_circle_handler),
        )
        .route(
            "/api/community/circles/:id/posts",
            get(get_circle_posts).post(create_circle_post),
        )
        .route(
            "/api/community/circles/:id/announcements",
            get(get_circle_announcements),
        )
        .route("/api/community/circles/:id/events", get(get_circle_events))
        .route(
            "/api/community/circles/:id/resources",
            get(get_circle_resources),
        )
        .route(
            "/api/community/circles/:id/resources/manage",
            get(get_circle_resource_manage).post(create_circle_resource_manage),
        )
        .route(
            "/api/community/circles/:id/resources/upload",
            post(upload_circle_resource_file),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/manage",
            put(update_circle_resource_manage),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/lifecycle",
            post(update_circle_resource_lifecycle),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/versions",
            get(get_circle_resource_versions).post(create_circle_resource_version),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/versions/upload",
            post(upload_circle_resource_version_file),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/versions/:version_id/access",
            get(get_circle_resource_version_access),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/versions/:version_id/restore",
            post(restore_circle_resource_version),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/versions/:version_id/review",
            post(review_circle_resource_version),
        )
        .route(
            "/api/community/circles/:id/resources/:resource_id/access",
            get(get_circle_resource_access),
        )
        .route(
            "/api/community/circles/:id/manage",
            get(get_circle_manage_summary).put(update_circle_manage_settings),
        )
        .route(
            "/api/community/circles/:id/analytics",
            get(get_circle_analytics),
        )
        .route(
            "/api/community/circles/:id/ops-alerts",
            get(get_circle_ops_alerts),
        )
        .route(
            "/api/community/circles/:id/ops-alerts/:alert_id/action",
            post(take_circle_ops_alert_action),
        )
        .route(
            "/api/community/circles/:id/reports",
            get(get_circle_report_queue),
        )
        .route(
            "/api/community/circles/:id/reports/bulk-action",
            post(take_circle_report_bulk_action),
        )
        .route(
            "/api/community/circles/:id/reports/:report_id/action",
            post(take_circle_report_action),
        )
        .route(
            "/api/community/circles/:id/challenges",
            get(get_circle_challenges),
        )
        .route(
            "/api/community/circles/:id/onboarding",
            get(get_circle_onboarding),
        )
        .route(
            "/api/community/circles/:id/onboarding/:step",
            post(update_circle_onboarding_step),
        )
        .route(
            "/api/community/circles/:id/members",
            get(get_circle_members),
        )
        .route("/api/community/circles/:id/join", post(join_circle))
        .route("/api/community/circles/leave", post(leave_circle))
        .route(
            "/api/community/circles/:id/invite",
            post(send_circle_invite),
        )
        .route(
            "/api/community/circles/:id/kick/:user_id",
            post(kick_circle_member),
        )
        // M4-BE.11: Role management (promote/demote to admin/member)
        .route(
            "/api/community/circles/:id/roles",
            post(update_circle_member_role),
        )
        // M4-BE.12: Transfer ownership to another member
        .route(
            "/api/community/circles/:id/transfer",
            post(transfer_circle_ownership),
        )
        // M4-BE.13: Update circle privacy (public/private)
        .route(
            "/api/community/circles/:id/privacy",
            post(update_circle_privacy),
        )
        // M4-BE.15: Join requests for private circles
        .route(
            "/api/community/circles/:id/request",
            post(request_to_join_circle).delete(cancel_join_request_handler),
        )
        .route(
            "/api/community/circles/:id/requests",
            get(list_join_requests),
        )
        .route(
            "/api/community/circles/requests/mine",
            get(get_my_join_requests_handler),
        )
        .route(
            "/api/community/circles/requests/:req_id/approve",
            post(approve_join_request_handler),
        )
        .route(
            "/api/community/circles/requests/:req_id/decline",
            post(decline_join_request_handler),
        )
        // W3.1: Token-Gated Circles
        .route(
            "/api/community/circles/:id/token-gate",
            post(update_circle_token_gate),
        )
        // ── Multi-circle rework (2026-05-16) ────────────────────────────
        .route(
            "/api/community/circles/discover",
            get(discover_circles_handler),
        )
        .route("/api/community/circles/search", get(search_circles_handler))
        .route(
            "/api/community/circles/by-slug/:slug",
            get(get_circle_by_slug_handler),
        )
        .route("/api/community/me/circles", get(list_my_circles_handler))
        .route(
            "/api/community/profile/banner",
            axum::routing::put(set_profile_banner_handler),
        )
        .route(
            "/api/community/circles/:id/moderator/:user_id",
            post(set_moderator_handler),
        )
        .route(
            "/api/community/circles/:id/bans",
            get(list_circle_bans_handler).post(ban_member_handler),
        )
        .route(
            "/api/community/circles/:id/bans/:user_id",
            axum::routing::delete(unban_member_handler),
        )
        .route("/api/community/invites", get(get_my_invites))
        .route("/api/community/invites/:id/accept", post(accept_invite))
        .route("/api/community/invites/:id/decline", post(decline_invite))
        // Property Reviews (M5)
        .route("/api/community/assets/:id/circle", get(get_asset_circle))
        .route(
            "/api/community/assets/:id/reviews",
            get(list_asset_reviews)
                .put(upsert_asset_review)
                .delete(delete_asset_review),
        )
        .route(
            "/api/community/reviews/:review_id/upvote",
            post(toggle_review_upvote),
        )
        // Challenges (M5)
        .route("/api/community/challenges", get(list_challenges))
        // Challenge submissions (14.8.11 follow-up)
        .route(
            "/api/community/challenges/:id/submit",
            post(submit_challenge_entry),
        )
        .route(
            "/api/community/challenges/:id/submissions",
            get(list_challenge_submissions),
        )
        .route(
            "/api/community/challenges/submissions/:sid/vote",
            post(toggle_submission_vote),
        )
        // Notifications (M5)
        .route("/api/community/notifications", get(list_notifications))
        .route(
            "/api/community/notifications/unread-count",
            get(get_unread_notification_count),
        )
        .route(
            "/api/community/notifications/read-all",
            post(mark_all_notifications_read),
        )
        .route(
            "/api/community/notifications/:id/read",
            post(mark_notification_read),
        )
        // 14.8.15 notification preferences (per-column system) REMOVED 2026-05-15.
        // Canonical endpoint is `/api/community/notifications/preferences`,
        // which is the one `notify_user` actually consults.
        // 14.8.24 — admin community settings
        .route(
            "/api/admin/community/settings",
            get(admin_get_community_settings).put(admin_update_community_settings),
        )
        // 14.8.20 — direct messages
        .route(
            "/api/community/dms/threads",
            get(list_dm_threads).post(create_dm_thread),
        )
        .route(
            "/api/community/dms/threads/:id/messages",
            get(list_dm_messages).post(post_dm_message),
        )
        // 14.8.16 — verified-owner request flow
        .route(
            "/api/community/verified-owner-requests",
            get(list_my_verified_owner_requests).post(submit_verified_owner_request),
        )
        .route(
            "/api/admin/community/verified-owner-requests",
            get(admin_list_verified_owner_requests),
        )
        .route(
            "/api/admin/community/verified-owner-requests/:id",
            axum::routing::patch(admin_review_verified_owner_request),
        )
        // Expert AMAs (M5)
        .route("/api/community/amas", get(list_amas))
        .route("/api/community/amas/:id", get(get_ama_detail))
        .route(
            "/api/community/amas/:id/questions",
            post(submit_ama_question),
        )
        .route(
            "/api/community/amas/:id/questions/:qid/upvote",
            post(toggle_ama_upvote),
        )
        // Admin AMAs
        .route(
            "/api/admin/community/amas",
            get(admin_list_amas).post(admin_create_ama),
        )
        .route("/api/admin/community/amas/:id", get(admin_get_ama_detail))
        .route(
            "/api/admin/community/amas/:id/status",
            post(admin_update_ama_status),
        )
        .route(
            "/api/admin/community/amas/:id/questions/:qid/answer",
            post(admin_answer_question),
        )
        .route(
            "/api/admin/community/amas/:id/questions/:qid/feature",
            post(admin_toggle_featured),
        )
        // Admin Challenges
        .route(
            "/api/admin/community/challenges",
            get(admin_list_challenges).post(admin_create_challenge),
        )
        .route(
            "/api/admin/community/challenges/:id/toggle",
            post(admin_toggle_challenge),
        )
        // Admin Badges (M3-ADMIN)
        .route(
            "/api/admin/community/badges",
            get(admin_list_badges).post(admin_create_badge),
        )
        .route("/api/admin/community/badges/:id", put(admin_update_badge))
        .route(
            "/api/admin/community/users/:id/badge",
            post(admin_grant_badge),
        )
        .route(
            "/api/admin/community/users/:id/badge/:badge_id",
            delete(admin_revoke_badge),
        )
        // Admin User Detail (M3-ADMIN)
        .route(
            "/api/admin/community/users/:id/detail",
            get(admin_get_user_detail),
        )
        // Admin Circles (M4-ADMIN / M5-ADMIN)
        .route("/api/admin/community/circles", get(admin_list_circles))
        .route(
            "/api/admin/community/circles/:id",
            get(admin_get_circle_detail)
                .delete(admin_delete_circle)
                .put(admin_update_circle),
        )
        .route(
            "/api/admin/community/circles/:id/transfer",
            post(admin_transfer_circle),
        )
        .route(
            "/api/admin/community/circles/:id/members/:user_id",
            delete(admin_remove_circle_member),
        )
        // Admin Leaderboard (M4-ADMIN)
        .route(
            "/api/admin/community/leaderboard",
            get(admin_get_leaderboard),
        )
        .route("/api/admin/community/users/:id/xp", post(admin_award_xp))
        // Admin Audit Log (M2-ADMIN.7) + CSV export (CO.13)
        .route(
            "/api/admin/community/audit-log",
            get(admin_get_community_audit_log),
        )
        .route(
            "/api/admin/community/audit-log.csv",
            get(admin_export_community_audit_log_csv),
        )
        // Bookmarks (UX.6)
        .route("/api/community/bookmarks", get(list_bookmarks))
        .route("/api/community/posts/:id/bookmark", post(toggle_bookmark))
        .route(
            "/api/community/posts/:id/bookmark/status",
            get(get_bookmark_status),
        )
        // Polls (UX.11)
        .route("/api/community/posts/:id/poll/vote", post(vote_on_poll))
        .route("/api/community/posts/:id/poll", get(get_poll_results))
        // Hashtags (UX.4)
        .route(
            "/api/community/hashtags/trending",
            get(get_trending_hashtags),
        )
        .route("/api/community/hashtags/:tag", get(get_posts_by_hashtag))
        // 14.8.13: user-facing badge detail
        .route("/api/community/badges/:id", get(get_badge_detail))
        // Phase 3 task 28: autocomplete suggestions for the post composer.
        .route("/api/community/mentions/suggest", get(suggest_mentions))
        .route("/api/community/hashtags/suggest", get(suggest_hashtags))
        .route("/api/community/assets/suggest", get(suggest_assets))
        // M6-FEAT.4 — searchable member directory
        .route("/api/community/members", get(list_community_members))
        // UX.8 — trending posts (sidebar widget)
        .route("/api/community/trending", get(list_trending_posts))
}

// ─── Social Handlers ─────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct UpdateProfileReq {
    pub bio: Option<String>,
    /// UX.14: optional short flair shown next to the display name. Empty
    /// string clears the flair; omitted leaves the existing value alone.
    pub flair: Option<String>,
    /// Privacy toggles. None = leave unchanged. Wired by the
    /// /community/me/edit page; consumed by the feed query, member
    /// directory, and DM-create handler.
    pub is_public_profile: Option<bool>,
    pub allow_dms_from_strangers: Option<bool>,
    /// Cross-DB: maps to `leaderboard_preferences.visible` in core.
    pub leaderboard_visible: Option<bool>,
}

async fn get_profile_me(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let profile = crate::community::service::get_user_profile(&c_pool, user.id).await?;
    let mut reputation_flair_map =
        crate::community::service::get_reputation_flairs_batch(&c_pool, &[user.id])
            .await
            .unwrap_or_default();
    let reputation_flairs = reputation_flair_map.remove(&user.id).unwrap_or_default();

    // 14.8.1: surface ban state + pending-appeal flag so the frontend can render
    // the ban-appeal banner without a second roundtrip. We propagate SQL errors
    // (rather than swallow them) so any binding/decoding issue surfaces in logs.
    use sqlx::Row;
    let ban_state_row = sqlx::query(
        "SELECT is_community_banned, ban_reason FROM community_profiles WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await?;
    let (is_banned, ban_reason): (bool, Option<String>) = match ban_state_row {
        Some(row) => {
            tracing::debug!(
                user_id = %user.id,
                "ban_state row found, decoding columns"
            );
            (
                row.try_get::<bool, _>("is_community_banned")?,
                row.try_get::<Option<String>, _>("ban_reason")?,
            )
        }
        None => {
            tracing::debug!(user_id = %user.id, "ban_state row missing");
            (false, None)
        }
    };

    let has_pending_appeal: bool = if is_banned {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM ban_appeals WHERE user_id = $1 AND status = 'pending'",
        )
        .bind(user.id)
        .fetch_one(&c_pool)
        .await
        .map(|n| n > 0)
        .unwrap_or(false)
    } else {
        false
    };

    // Phase 3 task 19: include avatar_url so the edit-profile modal can paint
    // the current photo without an extra /api/me roundtrip.
    let avatar_url: Option<String> =
        sqlx::query_scalar("SELECT avatar_url FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .flatten();

    // Phase 3 task 30: surface is_shadowbanned + warning_count so the feed
    // can render a moderation banner without an extra fetch.
    let (is_shadowbanned, warning_count): (bool, i32) = sqlx::query_as::<_, (bool, i32)>(
        "SELECT COALESCE(is_shadowbanned, false), COALESCE(warning_count, 0) FROM community_profiles WHERE user_id = $1",
    )
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await?
    .unwrap_or((false, 0));

    // Cross-DB read: leaderboard visibility lives on
    // `leaderboard_preferences.visible` in the core DB. Default TRUE when
    // the row hasn't been created yet (matches the FE optimistic default).
    let leaderboard_visible: bool =
        sqlx::query_scalar("SELECT visible FROM leaderboard_preferences WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
            .ok()
            .flatten()
            .unwrap_or(true);

    Ok(Json(serde_json::json!({
        "user_id": profile.user_id,
        "bio": profile.bio,
        "avatar_url": avatar_url,
        "post_count": profile.post_count,
        "follower_count": profile.follower_count,
        "following_count": profile.following_count,
        "badges": profile.badges,
        "flair": profile.flair,
        "reputation_flairs": reputation_flairs,
        "is_public_profile": profile.is_public_profile,
        "allow_dms_from_strangers": profile.allow_dms_from_strangers,
        "leaderboard_visible": leaderboard_visible,
        "is_community_banned": is_banned,
        "ban_reason": ban_reason,
        "has_pending_appeal": has_pending_appeal,
        "is_shadowbanned": is_shadowbanned,
        "warning_count": warning_count,
    })))
}

// Phase 3 task 31: notification preferences. Stored as a JSONB blob; the
// server treats missing keys as "enabled" so adding a new notification type
// doesn't require a backfill.

async fn get_notification_preferences(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let prefs: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT prefs FROM notification_preferences WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(&c_pool)
            .await?;
    let prefs = prefs.unwrap_or_else(|| serde_json::json!({}));
    Ok(Json(serde_json::json!({ "prefs": prefs })))
}

#[derive(Deserialize)]
struct UpdatePrefsReq {
    pub prefs: serde_json::Value,
}

async fn update_notification_preferences(
    jar: CookieJar,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<UpdatePrefsReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_csrf_header(&headers, &jar)?;
    if !payload.prefs.is_object() {
        return Err(AppError::BadRequest(
            "prefs must be a JSON object keyed by notification type".into(),
        ));
    }
    let c_pool = get_community_pool(&state)?;
    sqlx::query(
        "INSERT INTO notification_preferences (user_id, prefs, updated_at) \
         VALUES ($1, $2, NOW()) \
         ON CONFLICT (user_id) DO UPDATE SET prefs = EXCLUDED.prefs, updated_at = NOW()",
    )
    .bind(user.id)
    .bind(&payload.prefs)
    .execute(&c_pool)
    .await?;
    Ok(Json(
        serde_json::json!({"success": true, "prefs": payload.prefs}),
    ))
}

// Phase 3 task 32 (statement-only verified-owner request) REMOVED 2026-05-15.
// Replaced by the asset-linked verified-owner-requests flow (14.8.16) — see
// `submit_verified_owner_request` further down. The legacy
// `verification_requests` table is dropped by migration 038.

// Phase 3 task 30: return moderation actions taken against the viewer so
// they can see their own moderation history.
async fn get_my_moderation_log(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let rows = sqlx::query_as::<_, (String, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT action, details, created_at
        FROM community_audit_logs
        WHERE target_user_id = $1
          AND action IN ('user.warn', 'user.mute', 'user.unmute', 'user.ban', 'user.unban', 'user.shadowban', 'user.unshadowban')
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(action, details, created_at)| {
            serde_json::json!({
                "action": action,
                "details": details,
                "created_at": created_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "entries": entries })))
}

async fn update_profile(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<UpdateProfileReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    // Community-side bio + flair + new privacy toggles. None for any field
    // = leave unchanged, so the FE can PUT a single key for instant save.
    let bio_set = payload
        .bio
        .as_ref()
        .map(|b| !b.trim().is_empty())
        .unwrap_or(false);
    let flair_set = payload
        .flair
        .as_ref()
        .map(|f| !f.is_empty())
        .unwrap_or(false);
    crate::community::service::update_user_profile(
        &c_pool,
        user.id,
        payload.bio,
        payload.flair.map(Some),
        payload.is_public_profile,
        payload.allow_dms_from_strangers,
    )
    .await?;

    // Getting-Started step 1: bio + flair. Once-only XP grant.
    if bio_set || flair_set {
        let _ = crate::community::xp::award_xp_once(
            &c_pool,
            user.id,
            "profile_completed",
            Some("Added a bio"),
        )
        .await;
        let _ = crate::community::xp::maybe_award_onboarding_complete(&c_pool, user.id).await;
    }

    // Cross-DB: leaderboard visibility lives in the core
    // `leaderboard_preferences` table (per-user UNIQUE row).
    if let Some(visible) = payload.leaderboard_visible {
        sqlx::query(
            r#"
            INSERT INTO leaderboard_preferences (user_id, visible, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (user_id) DO UPDATE
                SET visible = EXCLUDED.visible, updated_at = NOW()
            "#,
        )
        .bind(user.id)
        .bind(visible)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_profile(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db).await; // optional for public view
    let c_pool = get_community_pool(&state)?;

    let profile = crate::community::service::get_user_profile(&c_pool, profile_id).await?;
    let bridge_info =
        crate::community::user_bridge::get_user_info(&state.db, state.redis.as_ref(), profile_id)
            .await
            .unwrap_or_else(|_| crate::community::user_bridge::UserBridgeInfo {
                user_id: profile_id,
                display_name: "Anonymous User".to_string(),
                avatar_url: None,
            });

    let is_following = if let Some(ref u) = user {
        crate::community::service::is_following(&c_pool, u.id, profile_id).await?
    } else {
        false
    };

    // 14.8.2: surface block/mute state so the profile-modal action menu can
    // show the correct toggle label without an extra roundtrip.
    let (is_blocked, is_muted): (bool, bool) = if let Some(ref u) = user {
        let blocked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM block_relationships WHERE actor_user_id = $1 AND target_user_id = $2",
        )
        .bind(u.id)
        .bind(profile_id)
        .fetch_one(&c_pool)
        .await
        .unwrap_or(0);
        let muted: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM mute_relationships WHERE actor_user_id = $1 AND target_user_id = $2",
        )
        .bind(u.id)
        .bind(profile_id)
        .fetch_one(&c_pool)
        .await
        .unwrap_or(0);
        (blocked > 0, muted > 0)
    } else {
        (false, false)
    };

    Ok(Json(serde_json::json!({
        "user_id": profile.user_id,
        "display_name": bridge_info.display_name,
        "avatar_url": bridge_info.avatar_url,
        "bio": profile.bio,
        "follower_count": profile.follower_count,
        "following_count": profile.following_count,
        "post_count": profile.post_count,
        "badges": profile.badges,
        "is_following": is_following,
        "is_blocked": is_blocked,
        "is_muted": is_muted
    })))
}

async fn follow_user(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    if user.id == target_id {
        return Err(AppError::BadRequest("Cannot follow yourself".into()));
    }

    let c_pool = get_community_pool(&state)?;
    crate::community::service::add_follow(&c_pool, user.id, target_id).await?;

    // Award XP to the person being followed (they gained a follower)
    let _ = crate::community::xp::award_xp(
        &c_pool,
        target_id,
        "follow_gained",
        Some("Gained a new follower"),
        None,
    )
    .await;

    // Getting-Started step 2: follower may have just crossed 5-following.
    let _ = crate::community::xp::maybe_award_onboarding_complete(&c_pool, user.id).await;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn unfollow_user(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::service::remove_follow(&c_pool, user.id, target_id).await?;

    Ok(Json(serde_json::json!({"success": true})))
}

// Phase 3 task 20 + WS1.2: paginated list of followers / following.
// Returns up to PAGE_SIZE rows per call plus a has_more flag so the modal
// can render a "Load more" button. Each row carries display_name, avatar_url,
// and is_following so the modal can show a Follow/Unfollow button without
// a second roundtrip.
const RELATIONSHIP_PAGE_SIZE: i64 = 30;

#[derive(Deserialize)]
struct RelationshipQuery {
    page: Option<i64>,
}

async fn list_relationship(
    jar: CookieJar,
    state: &AppState,
    profile_id: Uuid,
    direction: &str,
    page: i64,
) -> Result<Json<serde_json::Value>, AppError> {
    let viewer = middleware::get_current_user(&jar, &state.db).await;
    let c_pool = get_community_pool(state)?;
    let page = page.max(1);
    let offset = (page - 1) * RELATIONSHIP_PAGE_SIZE;
    // Fetch one extra to detect "has_more" cheaply.
    let limit = RELATIONSHIP_PAGE_SIZE + 1;

    let sql = match direction {
        "followers" => "SELECT follower_id AS uid FROM follows WHERE following_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        "following" => "SELECT following_id AS uid FROM follows WHERE follower_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        _ => return Err(AppError::BadRequest("Invalid direction".into())),
    };

    let mut user_ids: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(sql)
        .bind(profile_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&c_pool)
        .await?;

    let has_more = user_ids.len() as i64 > RELATIONSHIP_PAGE_SIZE;
    if has_more {
        user_ids.truncate(RELATIONSHIP_PAGE_SIZE as usize);
    }

    let authors = if user_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?
    };

    let following_set: std::collections::HashSet<Uuid> = if let Some(ref v) = viewer {
        if user_ids.is_empty() {
            std::collections::HashSet::new()
        } else {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT following_id FROM follows WHERE follower_id = $1 AND following_id = ANY($2)",
            )
            .bind(v.id)
            .bind(&user_ids)
            .fetch_all(&c_pool)
            .await?
            .into_iter()
            .collect()
        }
    } else {
        std::collections::HashSet::new()
    };

    let users: Vec<serde_json::Value> = user_ids
        .iter()
        .map(|uid| {
            let info = authors.get(uid);
            let is_self = viewer.as_ref().map(|v| v.id == *uid).unwrap_or(false);
            serde_json::json!({
                "user_id": uid,
                "display_name": info.map(|a| a.display_name.clone()).unwrap_or_else(|| "Anonymous".into()),
                "avatar_url": info.and_then(|a| a.avatar_url.clone()),
                "is_following": following_set.contains(uid),
                "is_self": is_self,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "users": users,
        "page": page,
        "has_more": has_more,
    })))
}

async fn list_followers(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(q): Query<RelationshipQuery>,
) -> Result<impl IntoResponse, AppError> {
    list_relationship(jar, &state, profile_id, "followers", q.page.unwrap_or(1)).await
}

async fn list_following(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(q): Query<RelationshipQuery>,
) -> Result<impl IntoResponse, AppError> {
    list_relationship(jar, &state, profile_id, "following", q.page.unwrap_or(1)).await
}

// ─── WS3.1: per-user community profile data ─────────────────────────────

const PROFILE_PAGE_SIZE: i64 = 20;

#[derive(Deserialize)]
struct ProfilePageQuery {
    page: Option<i64>,
}

async fn list_user_posts(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(q): Query<ProfilePageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let viewer = middleware::get_current_user(&jar, &state.db).await;
    let c_pool = get_community_pool(&state)?;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * PROFILE_PAGE_SIZE;

    let posts = sqlx::query_as::<_, models::Post>(
        r#"
        SELECT p.* FROM posts p
        JOIN community_profiles cp ON p.user_id = cp.user_id
        WHERE p.user_id = $1
          AND p.is_hidden = false
          AND (cp.is_shadowbanned = false OR $2 = p.user_id)
        ORDER BY p.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(profile_id)
    .bind(viewer.as_ref().map(|v| v.id))
    .bind(PROFILE_PAGE_SIZE + 1)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let mut posts = posts;
    let has_more = posts.len() as i64 > PROFILE_PAGE_SIZE;
    if has_more {
        posts.truncate(PROFILE_PAGE_SIZE as usize);
    }

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;
    let badges = service::get_badges_batch(&c_pool, &user_ids).await?;

    // Reactions + bookmarks the viewer has placed on these posts.
    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();
    let (reacted_set, bookmarked_set): (
        std::collections::HashSet<Uuid>,
        std::collections::HashSet<Uuid>,
    ) = if let Some(ref v) = viewer {
        if post_ids.is_empty() {
            (
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
            )
        } else {
            let reacted = sqlx::query_scalar::<_, Uuid>(
                "SELECT post_id FROM reactions WHERE user_id = $1 AND post_id = ANY($2) AND reaction_type = 'fire'",
            )
            .bind(v.id)
            .bind(&post_ids)
            .fetch_all(&c_pool)
            .await?
            .into_iter()
            .collect();
            let bookmarked = sqlx::query_scalar::<_, Uuid>(
                "SELECT post_id FROM bookmarks WHERE user_id = $1 AND post_id = ANY($2)",
            )
            .bind(v.id)
            .bind(&post_ids)
            .fetch_all(&c_pool)
            .await?
            .into_iter()
            .collect();
            (reacted, bookmarked)
        }
    } else {
        (
            std::collections::HashSet::new(),
            std::collections::HashSet::new(),
        )
    };

    let feed: Vec<models::PostDisplay> = posts
        .iter()
        .map(|p| {
            let author = authors.get(&p.user_id);
            let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();
            let author_name = author
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Anonymous".into());
            map_to_post_display(
                p,
                author_name,
                author.and_then(|a| a.avatar_url.clone()),
                author_badges,
                reacted_set.contains(&p.id),
                bookmarked_set.contains(&p.id),
            )
        })
        .collect();

    Ok(Json(serde_json::json!({
        "posts": feed,
        "page": page,
        "has_more": has_more,
    })))
}

async fn list_user_comments(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(q): Query<ProfilePageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let _viewer = middleware::get_current_user(&jar, &state.db).await;
    let c_pool = get_community_pool(&state)?;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * PROFILE_PAGE_SIZE;

    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, chrono::DateTime<chrono::Utc>, String)>(
        r#"
        SELECT c.id, c.post_id, c.content, c.created_at,
               COALESCE(LEFT(p.content, 80), '')
        FROM comments c
        JOIN posts p ON p.id = c.post_id
        WHERE c.user_id = $1
          AND c.is_hidden = false
          AND p.is_hidden = false
        ORDER BY c.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(profile_id)
    .bind(PROFILE_PAGE_SIZE + 1)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let mut rows = rows;
    let has_more = rows.len() as i64 > PROFILE_PAGE_SIZE;
    if has_more {
        rows.truncate(PROFILE_PAGE_SIZE as usize);
    }

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, post_id, content, created_at, post_snippet)| {
            serde_json::json!({
                "id": id,
                "post_id": post_id,
                "content": content,
                "created_at": created_at,
                "post_snippet": post_snippet,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "comments": entries,
        "page": page,
        "has_more": has_more,
    })))
}

async fn list_user_media(
    _jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(q): Query<ProfilePageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * PROFILE_PAGE_SIZE;

    let rows = sqlx::query_as::<_, (Uuid, Option<Vec<String>>, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT id, image_urls, created_at FROM posts
        WHERE user_id = $1
          AND is_hidden = false
          AND image_urls IS NOT NULL
          AND array_length(image_urls, 1) > 0
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(profile_id)
    .bind(PROFILE_PAGE_SIZE + 1)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let mut rows = rows;
    let has_more = rows.len() as i64 > PROFILE_PAGE_SIZE;
    if has_more {
        rows.truncate(PROFILE_PAGE_SIZE as usize);
    }

    let mut media: Vec<serde_json::Value> = Vec::new();
    for (post_id, urls, created_at) in rows {
        for url in urls.unwrap_or_default() {
            media.push(serde_json::json!({
                "post_id": post_id,
                "url": crate::storage::service::rewrite_gcs_url(&url),
                "created_at": created_at,
            }));
        }
    }

    Ok(Json(serde_json::json!({
        "media": media,
        "page": page,
        "has_more": has_more,
    })))
}

async fn list_user_activity(
    _jar: CookieJar,
    State(state): State<AppState>,
    Path(profile_id): Path<Uuid>,
    Query(q): Query<ProfilePageQuery>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * PROFILE_PAGE_SIZE;
    let limit = PROFILE_PAGE_SIZE + 1;

    // Merge posts, comments, and xp_ledger entries into one timeline.
    let rows = sqlx::query_as::<_, (String, Uuid, Option<String>, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT 'post' AS kind, id AS entity_id, LEFT(content, 100) AS detail, created_at
        FROM posts WHERE user_id = $1 AND is_hidden = false
        UNION ALL
        SELECT 'comment' AS kind, id AS entity_id, LEFT(content, 100) AS detail, created_at
        FROM comments WHERE user_id = $1 AND is_hidden = false
        UNION ALL
        SELECT 'xp' AS kind, NULL::uuid AS entity_id,
               (amount::text || ' XP — ' || COALESCE(reason, '')) AS detail,
               created_at
        FROM xp_ledger WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(profile_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let mut rows = rows;
    let has_more = rows.len() as i64 > PROFILE_PAGE_SIZE;
    if has_more {
        rows.truncate(PROFILE_PAGE_SIZE as usize);
    }

    let entries: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(kind, entity_id, detail, created_at)| {
            serde_json::json!({
                "kind": kind,
                "entity_id": entity_id,
                "detail": detail,
                "created_at": created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "entries": entries,
        "page": page,
        "has_more": has_more,
    })))
}

async fn get_my_analytics(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    let posts_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE user_id = $1 AND created_at > NOW() - INTERVAL '30 days'",
    )
    .bind(user.id)
    .fetch_one(&c_pool)
    .await
    .unwrap_or(0);

    let reactions_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reactions r JOIN posts p ON r.post_id = p.id \
         WHERE p.user_id = $1 AND r.created_at > NOW() - INTERVAL '30 days'",
    )
    .bind(user.id)
    .fetch_one(&c_pool)
    .await
    .unwrap_or(0);

    let comments_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM comments c JOIN posts p ON c.post_id = p.id \
         WHERE p.user_id = $1 AND c.created_at > NOW() - INTERVAL '30 days'",
    )
    .bind(user.id)
    .fetch_one(&c_pool)
    .await
    .unwrap_or(0);

    let xp_30d: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM xp_ledger WHERE user_id = $1 AND created_at > NOW() - INTERVAL '30 days'",
    )
    .bind(user.id)
    .fetch_one(&c_pool)
    .await
    .unwrap_or(0);

    let profile_views_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM profile_views WHERE profile_user_id = $1 AND created_at > NOW() - INTERVAL '30 days'",
    )
    .bind(user.id)
    .fetch_one(&c_pool)
    .await
    .unwrap_or(0);

    let top_post: Option<(Uuid, String, i32)> = sqlx::query_as(
        "SELECT id, LEFT(content, 100), reaction_count FROM posts \
         WHERE user_id = $1 AND created_at > NOW() - INTERVAL '30 days' AND is_hidden = false \
         ORDER BY reaction_count DESC LIMIT 1",
    )
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await
    .unwrap_or(None);

    Ok(Json(serde_json::json!({
        "posts_30d": posts_30d,
        "reactions_received_30d": reactions_30d,
        "comments_received_30d": comments_30d,
        "xp_earned_30d": xp_30d,
        "profile_views_30d": profile_views_30d,
        "top_post": top_post.map(|(id, snippet, rc)| serde_json::json!({
            "post_id": id,
            "content_snippet": snippet,
            "reaction_count": rc,
        })),
    })))
}

/// Helper invoked by the profile page renderer to track views.
///
/// WS3.2: telemetry is fire-and-forget so a DB blip never breaks the page
/// render, but failures still surface via `tracing::warn!` (and forward to
/// Sentry via the tracing-sentry bridge) instead of being silently swallowed.
pub async fn record_profile_view(
    pool: &sqlx::PgPool,
    profile_user_id: Uuid,
    viewer_user_id: Option<Uuid>,
) {
    if Some(profile_user_id) == viewer_user_id {
        return; // Don't count self-views.
    }
    if let Err(e) =
        sqlx::query("INSERT INTO profile_views (profile_user_id, viewer_user_id) VALUES ($1, $2)")
            .bind(profile_user_id)
            .bind(viewer_user_id)
            .execute(pool)
            .await
    {
        tracing::warn!(
            profile_user_id = %profile_user_id,
            viewer_user_id = ?viewer_user_id,
            error = %e,
            "record_profile_view: insert failed"
        );
    }
}

// ─── Verified-owner request flow (14.8.16) ──────────────────────────────

#[derive(Deserialize)]
struct SubmitVerifiedOwnerRequest {
    pub asset_id: Uuid,
    pub evidence_url: Option<String>,
    pub note: Option<String>,
}

async fn submit_verified_owner_request(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<SubmitVerifiedOwnerRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM verified_owner_requests
         WHERE user_id = $1 AND asset_id = $2 AND status = 'pending'",
    )
    .bind(user.id)
    .bind(payload.asset_id)
    .fetch_optional(&c_pool)
    .await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "You already have a pending request for this asset.".into(),
        ));
    }

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO verified_owner_requests (user_id, asset_id, evidence_url, note)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(user.id)
    .bind(payload.asset_id)
    .bind(payload.evidence_url.as_deref())
    .bind(payload.note.as_deref())
    .fetch_one(&c_pool)
    .await?;

    Ok(Json(serde_json::json!({ "id": id, "status": "pending" })))
}

async fn list_my_verified_owner_requests(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, asset_id, status, evidence_url, note, reviewed_at, created_at
         FROM verified_owner_requests
         WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "asset_id": row.try_get::<Uuid, _>("asset_id").ok(),
                "status": row.try_get::<String, _>("status").unwrap_or_default(),
                "evidence_url": row.try_get::<Option<String>, _>("evidence_url").unwrap_or(None),
                "note": row.try_get::<Option<String>, _>("note").unwrap_or(None),
                "reviewed_at": row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("reviewed_at")
                    .unwrap_or(None),
                "created_at": row
                    .try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .ok(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "requests": items })))
}

#[derive(Deserialize)]
struct AdminReviewQuery {
    pub status: Option<String>,
}

async fn admin_list_verified_owner_requests(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminReviewQuery>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    let status_filter = query.status.unwrap_or_else(|| "pending".to_string());

    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, user_id, asset_id, status, evidence_url, note,
                reviewed_at, reviewer_id, admin_notes, created_at
         FROM verified_owner_requests
         WHERE status = $1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&status_filter)
    .fetch_all(&c_pool)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "user_id": row.try_get::<Uuid, _>("user_id").ok(),
                "asset_id": row.try_get::<Uuid, _>("asset_id").ok(),
                "status": row.try_get::<String, _>("status").unwrap_or_default(),
                "evidence_url": row.try_get::<Option<String>, _>("evidence_url").unwrap_or(None),
                "note": row.try_get::<Option<String>, _>("note").unwrap_or(None),
                "reviewed_at": row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("reviewed_at")
                    .unwrap_or(None),
                "reviewer_id": row.try_get::<Option<Uuid>, _>("reviewer_id").unwrap_or(None),
                "admin_notes": row.try_get::<Option<String>, _>("admin_notes").unwrap_or(None),
                "created_at": row
                    .try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .ok(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "requests": items })))
}

#[derive(Deserialize)]
struct AdminReviewVerifiedOwnerRequest {
    pub status: String,
    pub admin_notes: Option<String>,
}

async fn admin_review_verified_owner_request(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
    Json(payload): Json<AdminReviewVerifiedOwnerRequest>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    if !matches!(payload.status.as_str(), "approved" | "rejected") {
        return Err(AppError::BadRequest(
            "status must be 'approved' or 'rejected'.".into(),
        ));
    }

    let c_pool = get_community_pool(&state)?;
    let updated: Option<Uuid> = sqlx::query_scalar(
        "UPDATE verified_owner_requests SET
            status = $1,
            admin_notes = $2,
            reviewed_at = NOW(),
            reviewer_id = $3
         WHERE id = $4 AND status = 'pending'
         RETURNING user_id",
    )
    .bind(&payload.status)
    .bind(payload.admin_notes.as_deref())
    .bind(admin.user.id)
    .bind(request_id)
    .fetch_optional(&c_pool)
    .await?;

    if updated.is_none() {
        return Err(AppError::NotFound(
            "Request not found or already reviewed.".into(),
        ));
    }

    Ok(Json(serde_json::json!({
        "id": request_id,
        "status": payload.status,
    })))
}

// ─── Admin community settings (14.8.24) ─────────────────────────────────

async fn admin_get_community_settings(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.view")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT key, value, description, updated_at FROM community_settings ORDER BY key",
    )
    .fetch_all(&c_pool)
    .await?;

    let settings: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "key": row.try_get::<String, _>("key").unwrap_or_default(),
                "value": row.try_get::<String, _>("value").unwrap_or_default(),
                "description": row.try_get::<Option<String>, _>("description").unwrap_or(None),
                "updated_at": row
                    .try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                    .ok(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "settings": settings })))
}

#[derive(Deserialize)]
struct UpdateCommunitySettingsPayload {
    pub updates: Vec<CommunitySettingUpdate>,
}

#[derive(Deserialize)]
struct CommunitySettingUpdate {
    pub key: String,
    pub value: String,
}

async fn admin_update_community_settings(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateCommunitySettingsPayload>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    if payload.updates.is_empty() {
        return Ok(Json(serde_json::json!({ "updated": 0 })));
    }

    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;
    let mut updated_count: usize = 0;
    for update in &payload.updates {
        // Only existing keys are accepted (no schemaless write).
        let res = sqlx::query(
            "UPDATE community_settings SET value = $1, updated_by = $2, updated_at = NOW()
             WHERE key = $3",
        )
        .bind(&update.value)
        .bind(admin.user.id)
        .bind(&update.key)
        .execute(&mut *tx)
        .await?;
        if res.rows_affected() > 0 {
            updated_count += 1;
        }
    }
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "updated": updated_count })))
}

// ─── Direct messages (14.8.20) ──────────────────────────────────────────

/// Helper — returns (a, b) with a < b so callers can address the unique row.
fn dm_pair(viewer: Uuid, other: Uuid) -> (Uuid, Uuid) {
    if viewer < other {
        (viewer, other)
    } else {
        (other, viewer)
    }
}

/// Helper — returns true when either user has blocked the other.
async fn dm_block_exists(c_pool: &sqlx::PgPool, a: Uuid, b: Uuid) -> Result<bool, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM block_relationships
         WHERE (actor_user_id = $1 AND target_user_id = $2)
            OR (actor_user_id = $2 AND target_user_id = $1)",
    )
    .bind(a)
    .bind(b)
    .fetch_one(c_pool)
    .await?;
    Ok(count > 0)
}

async fn list_dm_threads(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT t.id, t.participant_a_id, t.participant_b_id, t.last_message_at,
                (
                    SELECT COUNT(*) FROM dm_messages m
                    WHERE m.thread_id = t.id
                      AND m.sender_id <> $1
                      AND m.read_at_recipient IS NULL
                ) AS unread_count,
                (
                    SELECT m.content FROM dm_messages m
                    WHERE m.thread_id = t.id
                    ORDER BY m.created_at DESC LIMIT 1
                ) AS last_message_preview
         FROM dm_threads t
         WHERE (t.participant_a_id = $1 AND t.deleted_at_a IS NULL)
            OR (t.participant_b_id = $1 AND t.deleted_at_b IS NULL)
         ORDER BY COALESCE(t.last_message_at, t.created_at) DESC
         LIMIT 100",
    )
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;

    let mut other_ids: Vec<Uuid> = Vec::with_capacity(rows.len());
    let mut pending: Vec<(
        Uuid,
        Uuid,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
        Option<String>,
    )> = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let a: Uuid = row.try_get("participant_a_id")?;
        let b: Uuid = row.try_get("participant_b_id")?;
        let last: Option<chrono::DateTime<chrono::Utc>> = row.try_get("last_message_at")?;
        let unread: i64 = row.try_get("unread_count")?;
        let preview: Option<String> = row.try_get("last_message_preview")?;
        let other = if a == user.id { b } else { a };
        other_ids.push(other);
        pending.push((id, other, last, unread, preview));
    }
    let authors = if other_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &other_ids)
            .await
            .unwrap_or_default()
    };

    let threads: Vec<serde_json::Value> = pending
        .into_iter()
        .map(|(id, other, last, unread, preview)| {
            let info = authors.get(&other);
            serde_json::json!({
                "thread_id": id,
                "other_user_id": other,
                "other_display_name": info
                    .map(|i| i.display_name.clone())
                    .unwrap_or_else(|| "Anonymous".into()),
                "other_avatar_url": info.and_then(|i| i.avatar_url.clone()),
                "last_message_at": last,
                "last_message_preview": preview,
                "unread_count": unread,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "threads": threads })))
}

#[derive(Deserialize)]
struct CreateDmThreadPayload {
    pub recipient_user_id: Uuid,
    pub content: String,
}

async fn create_dm_thread(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<CreateDmThreadPayload>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    if user.id == payload.recipient_user_id {
        return Err(AppError::BadRequest("Cannot DM yourself.".into()));
    }
    let trimmed = payload.content.trim();
    if trimmed.is_empty() || trimmed.len() > 4000 {
        return Err(AppError::BadRequest(
            "Message must be 1–4000 characters.".into(),
        ));
    }

    let c_pool = get_community_pool(&state)?;
    check_user_not_banned(&c_pool, user.id).await?;
    if dm_block_exists(&c_pool, user.id, payload.recipient_user_id).await? {
        return Err(AppError::Forbidden(
            "Direct messages are unavailable between you and this user.".into(),
        ));
    }

    let (a, b) = dm_pair(user.id, payload.recipient_user_id);
    let mut tx = c_pool.begin().await?;
    let thread_id: Uuid = sqlx::query_scalar(
        "INSERT INTO dm_threads (participant_a_id, participant_b_id, last_message_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (participant_a_id, participant_b_id)
         DO UPDATE SET last_message_at = NOW(),
                       deleted_at_a = NULL,
                       deleted_at_b = NULL
         RETURNING id",
    )
    .bind(a)
    .bind(b)
    .fetch_one(&mut *tx)
    .await?;

    let message_id: Uuid = sqlx::query_scalar(
        "INSERT INTO dm_messages (thread_id, sender_id, content)
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(thread_id)
    .bind(user.id)
    .bind(trimmed)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "thread_id": thread_id,
        "message_id": message_id,
    })))
}

async fn list_dm_messages(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(thread_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;
    let thread =
        sqlx::query("SELECT participant_a_id, participant_b_id FROM dm_threads WHERE id = $1")
            .bind(thread_id)
            .fetch_optional(&c_pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Thread not found.".into()))?;
    let a: Uuid = thread.try_get("participant_a_id")?;
    let b: Uuid = thread.try_get("participant_b_id")?;
    if user.id != a && user.id != b {
        return Err(AppError::Forbidden("Not a participant.".into()));
    }

    // Mark messages from the other side as read.
    let _ = sqlx::query(
        "UPDATE dm_messages SET read_at_recipient = NOW()
         WHERE thread_id = $1 AND sender_id <> $2 AND read_at_recipient IS NULL",
    )
    .bind(thread_id)
    .bind(user.id)
    .execute(&c_pool)
    .await;

    let rows = sqlx::query(
        "SELECT id, sender_id, content, created_at, read_at_recipient
         FROM dm_messages WHERE thread_id = $1
         ORDER BY created_at ASC LIMIT 500",
    )
    .bind(thread_id)
    .fetch_all(&c_pool)
    .await?;

    let messages: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "sender_id": row.try_get::<Uuid, _>("sender_id").ok(),
                "content": row.try_get::<String, _>("content").unwrap_or_default(),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
                "read_at_recipient": row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("read_at_recipient")
                    .unwrap_or(None),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "messages": messages })))
}

#[derive(Deserialize)]
struct PostDmMessagePayload {
    pub content: String,
}

async fn post_dm_message(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(thread_id): Path<Uuid>,
    Json(payload): Json<PostDmMessagePayload>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let trimmed = payload.content.trim();
    if trimmed.is_empty() || trimmed.len() > 4000 {
        return Err(AppError::BadRequest(
            "Message must be 1–4000 characters.".into(),
        ));
    }

    let c_pool = get_community_pool(&state)?;
    check_user_not_banned(&c_pool, user.id).await?;

    use sqlx::Row;
    let thread =
        sqlx::query("SELECT participant_a_id, participant_b_id FROM dm_threads WHERE id = $1")
            .bind(thread_id)
            .fetch_optional(&c_pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Thread not found.".into()))?;
    let a: Uuid = thread.try_get("participant_a_id")?;
    let b: Uuid = thread.try_get("participant_b_id")?;
    if user.id != a && user.id != b {
        return Err(AppError::Forbidden("Not a participant.".into()));
    }
    let other = if user.id == a { b } else { a };
    if dm_block_exists(&c_pool, user.id, other).await? {
        return Err(AppError::Forbidden(
            "Direct messages are unavailable between you and this user.".into(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO dm_messages (thread_id, sender_id, content)
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(thread_id)
    .bind(user.id)
    .bind(trimmed)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE dm_threads SET last_message_at = NOW() WHERE id = $1")
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({ "id": id })))
}

// ─── Block / mute self-service (14.8.2) ──────────────────────────────────
// A block disables visibility in both directions (the target's posts vanish
// from the actor's feed AND the actor's posts vanish from the target's feed);
// a mute is one-directional (only the actor stops seeing the target).

async fn block_user(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    if user.id == target_id {
        return Err(AppError::BadRequest("Cannot block yourself".into()));
    }
    let c_pool = get_community_pool(&state)?;
    sqlx::query(
        "INSERT INTO block_relationships (actor_user_id, target_user_id)
         VALUES ($1, $2)
         ON CONFLICT (actor_user_id, target_user_id) DO NOTHING",
    )
    .bind(user.id)
    .bind(target_id)
    .execute(&c_pool)
    .await?;
    // Blocking implies unfollowing in both directions so dangling follow rows
    // don't keep the target visible in the actor's "Following" feed.
    let _ = sqlx::query(
        "DELETE FROM follows WHERE (follower_id = $1 AND followee_id = $2)
                              OR (follower_id = $2 AND followee_id = $1)",
    )
    .bind(user.id)
    .bind(target_id)
    .execute(&c_pool)
    .await;
    Ok(Json(serde_json::json!({"success": true, "blocked": true})))
}

async fn unblock_user(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    sqlx::query("DELETE FROM block_relationships WHERE actor_user_id = $1 AND target_user_id = $2")
        .bind(user.id)
        .bind(target_id)
        .execute(&c_pool)
        .await?;
    Ok(Json(serde_json::json!({"success": true, "blocked": false})))
}

async fn mute_user(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    if user.id == target_id {
        return Err(AppError::BadRequest("Cannot mute yourself".into()));
    }
    let c_pool = get_community_pool(&state)?;
    sqlx::query(
        "INSERT INTO mute_relationships (actor_user_id, target_user_id)
         VALUES ($1, $2)
         ON CONFLICT (actor_user_id, target_user_id) DO NOTHING",
    )
    .bind(user.id)
    .bind(target_id)
    .execute(&c_pool)
    .await?;
    Ok(Json(serde_json::json!({"success": true, "muted": true})))
}

async fn unmute_user(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    sqlx::query("DELETE FROM mute_relationships WHERE actor_user_id = $1 AND target_user_id = $2")
        .bind(user.id)
        .bind(target_id)
        .execute(&c_pool)
        .await?;
    Ok(Json(serde_json::json!({"success": true, "muted": false})))
}

async fn list_blocks(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT target_user_id, created_at FROM block_relationships
         WHERE actor_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;
    let mut ids = Vec::with_capacity(rows.len());
    let mut payload = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("target_user_id")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        ids.push(id);
        payload.push(serde_json::json!({
            "target_user_id": id,
            "created_at": created_at,
        }));
    }
    let core_users =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &ids).await?;
    let enriched: Vec<serde_json::Value> = payload
        .into_iter()
        .zip(ids.iter())
        .map(|(mut row, id)| {
            if let Some(info) = core_users.get(id) {
                row["display_name"] = serde_json::json!(info.display_name);
                row["avatar_url"] = serde_json::json!(info.avatar_url);
            }
            row
        })
        .collect();
    Ok(Json(serde_json::json!({"blocks": enriched})))
}

async fn list_mutes(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT target_user_id, created_at FROM mute_relationships
         WHERE actor_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;
    let mut ids = Vec::with_capacity(rows.len());
    let mut payload = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("target_user_id")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
        ids.push(id);
        payload.push(serde_json::json!({
            "target_user_id": id,
            "created_at": created_at,
        }));
    }
    let core_users =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &ids).await?;
    let enriched: Vec<serde_json::Value> = payload
        .into_iter()
        .zip(ids.iter())
        .map(|(mut row, id)| {
            if let Some(info) = core_users.get(id) {
                row["display_name"] = serde_json::json!(info.display_name);
                row["avatar_url"] = serde_json::json!(info.avatar_url);
            }
            row
        })
        .collect();
    Ok(Json(serde_json::json!({"mutes": enriched})))
}

#[derive(Deserialize)]
struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>, // "users", "posts", "all"
    pub page: Option<i64>,
    // 14.8.19 — post filters (applied only when type=posts or type=all).
    // Ignored for user search.
    pub date_from: Option<String>, // ISO-8601 date or datetime, server-parsed
    pub date_to: Option<String>,
    pub author_id: Option<Uuid>,
    pub min_engagement: Option<i64>, // reactions + comments >= N
}

async fn search_community(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let viewer = middleware::get_current_user(&jar, &state.db).await;
    let c_pool = get_community_pool(&state)?;

    let limit = 20;
    let offset = (query.page.unwrap_or(1).max(1) - 1) * limit;

    let search_type = query.r#type.as_deref().unwrap_or("all");
    let search_term = format!("%{}%", query.q);

    let mut users_result = Vec::new();
    let mut posts_result = Vec::new();

    if search_type == "all" || search_type == "users" {
        users_result = sqlx::query_as::<_, crate::community::models::CommunityProfile>(
            r#"
            SELECT * FROM community_profiles
            WHERE is_shadowbanned = false
              AND is_community_banned = false
              AND bio ILIKE $1
            ORDER BY follower_count DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&search_term)
        .bind(limit)
        .bind(offset)
        .fetch_all(&c_pool)
        .await?;

        // Match with display_names in main DB!
        // Wait, display names are in the main database... it's better to fetch users whose display_name matches from main db!
        let user_matches_from_main = sqlx::query!(
            "SELECT u.id FROM users u JOIN user_profiles up ON u.id = up.user_id WHERE up.display_name ILIKE $1 LIMIT $2",
            &search_term, limit
        ).fetch_all(&state.db).await?;

        let matching_uids: Vec<Uuid> = user_matches_from_main.iter().map(|u| u.id).collect();

        if !matching_uids.is_empty() {
            let name_matched_users =
                sqlx::query_as::<_, crate::community::models::CommunityProfile>(
                    r#"
                SELECT * FROM community_profiles
                WHERE is_shadowbanned = false
                  AND is_community_banned = false
                  AND user_id = ANY($1)
                "#,
                )
                .bind(&matching_uids)
                .fetch_all(&c_pool)
                .await?;

            for u in name_matched_users {
                if !users_result
                    .iter()
                    .any(|existing| existing.user_id == u.user_id)
                {
                    users_result.push(u);
                }
            }
        }
    }

    if search_type == "all" || search_type == "posts" {
        // 14.8.19 — apply optional date / author / engagement filters.
        // Parse dates leniently: accept either YYYY-MM-DD or full RFC3339.
        let parse_date = |s: &str| -> Option<chrono::DateTime<chrono::Utc>> {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                return Some(dt.with_timezone(&chrono::Utc));
            }
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|ndt| ndt.and_utc())
        };
        let date_from = query.date_from.as_deref().and_then(parse_date);
        let date_to = query.date_to.as_deref().and_then(parse_date);
        let author_id = query.author_id;
        let min_engagement = query.min_engagement.unwrap_or(0).max(0);

        posts_result = sqlx::query_as::<_, crate::community::models::Post>(
            r#"
            SELECT p.* FROM posts p
            JOIN community_profiles cp ON p.user_id = cp.user_id
            WHERE p.is_hidden = false
              AND cp.is_shadowbanned = false
              AND cp.is_community_banned = false
              AND (p.content ILIKE $1 OR p.content_tags::text ILIKE $1)
              AND ($4::timestamptz IS NULL OR p.created_at >= $4)
              AND ($5::timestamptz IS NULL OR p.created_at <= $5)
              AND ($6::uuid IS NULL OR p.user_id = $6)
              AND (p.reaction_count + p.comment_count) >= $7
            ORDER BY
              CASE
                WHEN p.qa_status = 'official_answer' THEN 0
                WHEN p.qa_status = 'answered' THEN 1
                WHEN p.post_type IN ('question', 'due_diligence', 'resource') THEN 2
                ELSE 3
              END,
              p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&search_term)
        .bind(limit)
        .bind(offset)
        .bind(date_from)
        .bind(date_to)
        .bind(author_id)
        .bind(min_engagement)
        .fetch_all(&c_pool)
        .await?;
    }

    // Resolve Users
    let mut uids_to_fetch = std::collections::HashSet::new();
    for u in &users_result {
        uids_to_fetch.insert(u.user_id);
    }
    for p in &posts_result {
        uids_to_fetch.insert(p.user_id);
    }
    let uids_vec: Vec<Uuid> = uids_to_fetch.into_iter().collect();

    let authors = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &uids_vec,
    )
    .await?;
    let badges = crate::community::service::get_badges_batch(&c_pool, &uids_vec).await?;

    // Batch lookup of which result users the viewer already follows.
    let following_set: std::collections::HashSet<Uuid> = if let Some(v) = &viewer {
        let user_ids: Vec<Uuid> = users_result.iter().map(|u| u.user_id).collect();
        if user_ids.is_empty() {
            std::collections::HashSet::new()
        } else {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT following_id FROM follows WHERE follower_id = $1 AND following_id = ANY($2)",
            )
            .bind(v.id)
            .bind(&user_ids)
            .fetch_all(&c_pool)
            .await?
            .into_iter()
            .collect()
        }
    } else {
        std::collections::HashSet::new()
    };

    // Format response
    let mut users_formatted = Vec::new();
    for u in users_result {
        let auth = authors.get(&u.user_id);
        let is_self = viewer.as_ref().map(|v| v.id == u.user_id).unwrap_or(false);
        users_formatted.push(serde_json::json!({
            "user_id": u.user_id,
            "display_name": auth.map(|a| a.display_name.clone()).unwrap_or_else(|| "Anonymous".into()),
            "avatar_url": auth.and_then(|a| a.avatar_url.clone()),
            "bio": u.bio,
            "follower_count": u.follower_count,
            "badges": badges.get(&u.user_id).cloned().unwrap_or_default(),
            "is_following": following_set.contains(&u.user_id),
            "is_self": is_self,
        }));
    }

    let mut posts_formatted = Vec::new();
    for p in posts_result {
        let auth = authors.get(&p.user_id);
        let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();
        let author_name = auth
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Anonymous".into());

        posts_formatted.push(map_to_post_display(
            &p,
            author_name,
            auth.and_then(|a| a.avatar_url.clone()),
            author_badges,
            false,
            false,
        ));
    }

    Ok(Json(serde_json::json!({
        "users": users_formatted,
        "posts": posts_formatted,
    })))
}

// ─── XP Handlers (M4) ───────────────────────────────────────────────────────

async fn get_xp_summary(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let summary = crate::community::xp::get_xp_summary(&c_pool, user.id).await?;
    Ok(Json(summary))
}

#[derive(Deserialize)]
struct XpHistoryQuery {
    page: Option<i64>,
}

async fn get_xp_history(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(q): Query<XpHistoryQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let page = q.page.unwrap_or(1).max(1);
    let c_pool = get_community_pool(&state)?;
    let entries =
        crate::community::xp::get_xp_history(&c_pool, user.id, 20, (page - 1) * 20).await?;
    Ok(Json(serde_json::json!({"entries": entries, "page": page})))
}

// ─── Circle Handlers (M4) ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateCircleReq {
    name: String,
    description: Option<String>,
    emoji: Option<String>,
}

async fn create_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<CreateCircleReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    // Super admins skip both the level gate and the one-circle-per-owner
    // limit so they can seed/recover circles freely.
    let is_super_admin: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_roles ur
            JOIN roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1
              AND r.name = 'super_admin'
              AND ur.is_active = TRUE
        )
        "#,
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let c_pool = get_community_pool(&state)?;

    if !is_super_admin {
        // Level gate: Level 2 required to create a circle (M4-BE.10)
        crate::community::xp::check_level_gate(
            &c_pool,
            user.id,
            crate::community::xp::GatedFeature::CreateCircle,
        )
        .await?;
    }

    let circle = crate::community::circles::create_circle(
        &c_pool,
        user.id,
        &payload.name,
        payload.description.as_deref(),
        payload.emoji.as_deref(),
        is_super_admin,
    )
    .await?;

    // Award XP for creating a circle
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "circle_created",
        Some("Created a circle"),
        None,
    )
    .await;

    Ok(Json(circle))
}

/// Hydrate a list of `CircleMember` rows with display_name + avatar_url from
/// the cross-DB user bridge. Falls back to a generic label so the FE never
/// renders "Investor #abcdef" UUID stubs.
async fn enrich_circle_members(
    state: &AppState,
    members: Vec<crate::community::circles::CircleMember>,
) -> Vec<serde_json::Value> {
    if members.is_empty() {
        return Vec::new();
    }
    let user_ids: Vec<Uuid> = members.iter().map(|m| m.user_id).collect();
    let info_map = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &user_ids,
    )
    .await
    .unwrap_or_default();
    members
        .into_iter()
        .map(|m| {
            let info = info_map.get(&m.user_id);
            serde_json::json!({
                "user_id": m.user_id,
                "role": m.role,
                "joined_at": m.joined_at,
                "display_name": info
                    .map(|i| i.display_name.clone())
                    .unwrap_or_else(|| "Anonymous Investor".to_string()),
                "avatar_url": info.and_then(|i| i.avatar_url.clone()),
            })
        })
        .collect()
}

async fn get_my_circle(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::get_my_circle(&c_pool, user.id).await?;

    match circle {
        Some(c) => {
            let members = crate::community::circles::get_circle_members(&c_pool, c.id).await?;
            let enriched = enrich_circle_members(&state, members).await;
            Ok(Json(serde_json::json!({"circle": c, "members": enriched})))
        }
        None => Ok(Json(serde_json::json!({"circle": null, "members": []}))),
    }
}

async fn get_circle_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::get_circle(&c_pool, circle_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;
    let members = crate::community::circles::get_circle_members(&c_pool, circle_id).await?;
    let enriched = enrich_circle_members(&state, members).await;

    Ok(Json(
        serde_json::json!({"circle": circle, "members": enriched}),
    ))
}

#[derive(Deserialize)]
struct UpdateCircleReq {
    name: Option<String>,
    description: Option<String>,
    emoji: Option<String>,
    // CO.2: optional banner URL. Send empty string to clear, omit to leave alone.
    banner_url: Option<String>,
}

#[derive(Deserialize)]
struct CircleManageSettingsReq {
    name: Option<String>,
    description: Option<String>,
    avatar_emoji: Option<String>,
    slug: Option<String>,
    is_public: Option<bool>,
    visibility: Option<String>,
    join_policy: Option<String>,
    circle_type: Option<String>,
    category: Option<String>,
    language: Option<String>,
    location_text: Option<String>,
    rules_text: Option<String>,
    investment_disclaimer: Option<String>,
    join_approval_required: Option<bool>,
    auto_approve_verified_investors: Option<bool>,
    media_uploads_enabled: Option<bool>,
    polls_enabled: Option<bool>,
    anonymous_posting_enabled: Option<bool>,
    link_posting_enabled: Option<bool>,
    first_post_approval_enabled: Option<bool>,
    slow_mode_seconds: Option<i32>,
    blocked_words: Option<Vec<String>>,
    investment_risk_keywords: Option<Vec<String>>,
    allowed_post_types: Option<Vec<String>>,
    required_post_tags: Option<Vec<String>>,
    announcement_comments_enabled: Option<bool>,
    onboarding_enabled: Option<bool>,
}

#[derive(Deserialize)]
struct CircleOpsAlertActionReq {
    action: String,
    note: Option<String>,
    workflow_state: Option<String>,
}

#[derive(Deserialize)]
struct CircleReportBulkActionReq {
    action: String,
    report_ids: Vec<Uuid>,
    admin_notes: Option<String>,
}

fn normalize_manage_text(
    value: Option<String>,
    max_len: usize,
    label: &str,
) -> Result<Option<String>, AppError> {
    match value {
        Some(v) => {
            let trimmed = v.trim();
            if trimmed.chars().count() > max_len {
                return Err(AppError::BadRequest(format!(
                    "{} must be {} characters or fewer.",
                    label, max_len
                )));
            }
            Ok(if trimmed.is_empty() {
                Some(String::new())
            } else {
                Some(trimmed.to_string())
            })
        }
        None => Ok(None),
    }
}

fn validate_circle_slug(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let slug = value.trim().to_ascii_lowercase();
    let valid_len = (3..=60).contains(&slug.len());
    let valid_chars = slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
    let valid_edges = !slug.starts_with('-') && !slug.ends_with('-');
    if !(valid_len && valid_chars && valid_edges) {
        return Err(AppError::BadRequest(
            "Circle slug must be 3-60 lowercase letters, numbers, or hyphens and cannot start or end with a hyphen.".into(),
        ));
    }
    Ok(Some(slug))
}

fn ensure_manage_value_allowed(
    value: Option<String>,
    allowed: &[&str],
    label: &str,
) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = canonical_community_code(&value);
    if allowed.contains(&normalized.as_str()) {
        Ok(Some(normalized))
    } else {
        Err(AppError::BadRequest(format!("Invalid {}.", label)))
    }
}

fn normalize_manage_keywords(
    values: Option<Vec<String>>,
    label: &str,
) -> Result<Option<Vec<String>>, AppError> {
    let Some(values) = values else {
        return Ok(None);
    };
    if values.len() > 50 {
        return Err(AppError::BadRequest(format!(
            "{} can contain at most 50 entries.",
            label
        )));
    }

    let mut out = Vec::new();
    for value in values {
        let item = value.trim().to_ascii_lowercase();
        if item.is_empty() {
            continue;
        }
        if item.chars().count() > 80 {
            return Err(AppError::BadRequest(format!(
                "{} entries must be 80 characters or fewer.",
                label
            )));
        }
        if !out.iter().any(|existing| existing == &item) {
            out.push(item);
        }
    }
    Ok(Some(out))
}

fn normalize_manage_post_types(
    values: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, AppError> {
    let Some(values) = values else {
        return Ok(None);
    };
    let mut out = Vec::new();
    for value in values {
        let normalized = normalize_post_type(&value)?;
        if !out.iter().any(|existing| existing == &normalized) {
            out.push(normalized);
        }
    }
    if out.is_empty() {
        return Err(AppError::BadRequest(
            "At least one allowed post type is required.".into(),
        ));
    }
    Ok(Some(out))
}

async fn update_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<UpdateCircleReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::update_circle(
        &c_pool,
        circle_id,
        user.id,
        payload.name.as_deref(),
        payload.description.as_deref(),
        payload.emoji.as_deref(),
        payload.banner_url.as_deref(),
    )
    .await?;
    Ok(Json(circle))
}

async fn circle_manage_analytics_json(
    pool: &sqlx::PgPool,
    circle_id: Uuid,
) -> Result<serde_json::Value, AppError> {
    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT
          (SELECT COUNT(*)::BIGINT
             FROM posts
            WHERE circle_id = $1
              AND is_hidden = FALSE
              AND created_at >= NOW() - INTERVAL '7 days') AS posts_7d,
          (SELECT COUNT(*)::BIGINT
             FROM comments c
             JOIN posts p ON p.id = c.post_id
            WHERE p.circle_id = $1
              AND c.is_hidden = FALSE
              AND c.created_at >= NOW() - INTERVAL '7 days') AS comments_7d,
          (SELECT COUNT(DISTINCT activity.user_id)::BIGINT
             FROM (
               SELECT user_id FROM posts
                WHERE circle_id = $1
                  AND is_hidden = FALSE
                  AND created_at >= NOW() - INTERVAL '7 days'
               UNION
               SELECT c.user_id FROM comments c
                JOIN posts p ON p.id = c.post_id
               WHERE p.circle_id = $1
                 AND c.is_hidden = FALSE
                 AND c.created_at >= NOW() - INTERVAL '7 days'
             ) activity) AS active_members_7d,
          (SELECT COUNT(*)::BIGINT
             FROM content_reports cr
             JOIN posts p ON p.id = cr.post_id
            WHERE p.circle_id = $1
              AND cr.status = 'pending') AS pending_reports,
          (SELECT COUNT(*)::BIGINT
             FROM circle_members
            WHERE circle_id = $1) AS member_count,
          (SELECT COUNT(*)::BIGINT
             FROM community_audit_logs
            WHERE entity_type = 'circle'
              AND entity_id = $1
              AND created_at >= NOW() - INTERVAL '7 days') AS audit_events_7d
        "#,
    )
    .bind(circle_id)
    .fetch_one(pool)
    .await?;

    let pending_reports: i64 = row.try_get("pending_reports").unwrap_or(0);
    Ok(serde_json::json!({
        "posts_7d": row.try_get::<i64, _>("posts_7d").unwrap_or(0),
        "comments_7d": row.try_get::<i64, _>("comments_7d").unwrap_or(0),
        "active_members_7d": row.try_get::<i64, _>("active_members_7d").unwrap_or(0),
        "pending_reports": pending_reports,
        "member_count": row.try_get::<i64, _>("member_count").unwrap_or(0),
        "audit_events_7d": row.try_get::<i64, _>("audit_events_7d").unwrap_or(0),
        "report_backlog_status": if pending_reports > 0 { "attention" } else { "healthy" },
    }))
}

async fn get_circle_manage_summary(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;

    let row = sqlx::query(
        r#"
        SELECT id, name, description, avatar_emoji, slug, is_public,
               circle_type, visibility, join_policy, category, language,
               location_text, rules_text, investment_disclaimer,
               join_approval_required, auto_approve_verified_investors,
               allowed_post_types, required_post_tags,
               media_uploads_enabled, polls_enabled, anonymous_posting_enabled,
               link_posting_enabled, first_post_approval_enabled,
               slow_mode_seconds, blocked_words, investment_risk_keywords,
               announcement_comments_enabled, onboarding_enabled,
               analytics_enabled, updated_at
          FROM circles
         WHERE id = $1
        "#,
    )
    .bind(circle_id)
    .fetch_optional(&c_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;

    let analytics = circle_manage_analytics_json(&c_pool, circle_id).await?;

    let audit_rows = sqlx::query(
        r#"
        SELECT action, entity_type, entity_id, target_user_id, details, created_at
          FROM community_audit_logs
         WHERE entity_type = 'circle'
           AND entity_id = $1
         ORDER BY created_at DESC
         LIMIT 10
        "#,
    )
    .bind(circle_id)
    .fetch_all(&c_pool)
    .await?;

    let audit_log: Vec<serde_json::Value> = audit_rows
        .into_iter()
        .map(|audit| {
            serde_json::json!({
                "action": audit.try_get::<String, _>("action").unwrap_or_default(),
                "entity_type": audit.try_get::<String, _>("entity_type").unwrap_or_default(),
                "entity_id": audit.try_get::<Option<Uuid>, _>("entity_id").ok().flatten(),
                "target_user_id": audit.try_get::<Option<Uuid>, _>("target_user_id").ok().flatten(),
                "details": audit.try_get::<serde_json::Value, _>("details").unwrap_or_else(|_| serde_json::json!({})),
                "created_at": audit.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "role": role,
        "circle": {
            "id": row.try_get::<Uuid, _>("id").unwrap_or(circle_id),
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "description": row.try_get::<Option<String>, _>("description").ok().flatten(),
            "avatar_emoji": row.try_get::<Option<String>, _>("avatar_emoji").ok().flatten(),
            "slug": row.try_get::<String, _>("slug").unwrap_or_default(),
            "is_public": row.try_get::<bool, _>("is_public").unwrap_or(false),
            "circle_type": row.try_get::<String, _>("circle_type").unwrap_or_else(|_| "social".to_string()),
            "visibility": row.try_get::<String, _>("visibility").unwrap_or_else(|_| "private".to_string()),
            "join_policy": row.try_get::<String, _>("join_policy").unwrap_or_else(|_| "request".to_string()),
            "category": row.try_get::<Option<String>, _>("category").ok().flatten(),
            "language": row.try_get::<String, _>("language").unwrap_or_else(|_| "en".to_string()),
            "location_text": row.try_get::<Option<String>, _>("location_text").ok().flatten(),
            "rules_text": row.try_get::<Option<String>, _>("rules_text").ok().flatten(),
            "investment_disclaimer": row.try_get::<Option<String>, _>("investment_disclaimer").ok().flatten(),
            "join_approval_required": row.try_get::<bool, _>("join_approval_required").unwrap_or(false),
            "auto_approve_verified_investors": row.try_get::<bool, _>("auto_approve_verified_investors").unwrap_or(false),
            "allowed_post_types": row.try_get::<Vec<String>, _>("allowed_post_types").unwrap_or_default(),
            "required_post_tags": row.try_get::<Vec<String>, _>("required_post_tags").unwrap_or_default(),
            "media_uploads_enabled": row.try_get::<bool, _>("media_uploads_enabled").unwrap_or(true),
            "polls_enabled": row.try_get::<bool, _>("polls_enabled").unwrap_or(true),
            "anonymous_posting_enabled": row.try_get::<bool, _>("anonymous_posting_enabled").unwrap_or(false),
            "link_posting_enabled": row.try_get::<bool, _>("link_posting_enabled").unwrap_or(true),
            "first_post_approval_enabled": row.try_get::<bool, _>("first_post_approval_enabled").unwrap_or(false),
            "slow_mode_seconds": row.try_get::<i32, _>("slow_mode_seconds").unwrap_or(0),
            "blocked_words": row.try_get::<Vec<String>, _>("blocked_words").unwrap_or_default(),
            "investment_risk_keywords": row.try_get::<Vec<String>, _>("investment_risk_keywords").unwrap_or_default(),
            "announcement_comments_enabled": row.try_get::<bool, _>("announcement_comments_enabled").unwrap_or(true),
            "onboarding_enabled": row.try_get::<bool, _>("onboarding_enabled").unwrap_or(true),
            "analytics_enabled": row.try_get::<bool, _>("analytics_enabled").unwrap_or(true),
            "updated_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok(),
        },
        "analytics": analytics,
        "audit_log": audit_log,
    })))
}

async fn get_circle_analytics(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;
    let analytics = circle_manage_analytics_json(&c_pool, circle_id).await?;
    Ok(Json(serde_json::json!({
        "circle_id": circle_id,
        "role": role,
        "analytics": analytics,
    })))
}

async fn get_circle_ops_alerts(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;

    let rows = sqlx::query(
        r#"
        SELECT id,
               alert_type,
               severity,
               status,
               workflow_state,
               workflow_note,
               workflow_updated_at,
               workflow_updated_by,
               summary,
               details,
               created_at,
               resolved_at
          FROM circle_ops_alerts
         WHERE circle_id = $1
           AND status IN ('open', 'acknowledged')
         ORDER BY
           CASE severity
             WHEN 'critical' THEN 0
             WHEN 'warning' THEN 1
             ELSE 2
           END,
           created_at DESC
         LIMIT 50
        "#,
    )
    .bind(circle_id)
    .fetch_all(&c_pool)
    .await?;

    let alerts: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "alert_type": row.try_get::<String, _>("alert_type").unwrap_or_default(),
                "severity": row.try_get::<String, _>("severity").unwrap_or_else(|_| "info".to_string()),
                "status": row.try_get::<String, _>("status").unwrap_or_else(|_| "open".to_string()),
                "workflow_state": row.try_get::<String, _>("workflow_state").unwrap_or_else(|_| "triage".to_string()),
                "workflow_note": row.try_get::<Option<String>, _>("workflow_note").ok().flatten(),
                "workflow_updated_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("workflow_updated_at").ok().flatten(),
                "workflow_updated_by": row.try_get::<Option<Uuid>, _>("workflow_updated_by").ok().flatten(),
                "summary": row.try_get::<String, _>("summary").unwrap_or_default(),
                "details": row.try_get::<serde_json::Value, _>("details").unwrap_or_else(|_| serde_json::json!({})),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
                "resolved_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("resolved_at").ok().flatten(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "circle_id": circle_id,
        "role": role,
        "alerts": alerts,
    })))
}

async fn take_circle_ops_alert_action(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, alert_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CircleOpsAlertActionReq>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;

    let action = payload.action.trim().to_ascii_lowercase();
    if !matches!(
        action.as_str(),
        "acknowledge" | "resolve" | "set_workflow_state"
    ) {
        return Err(AppError::BadRequest(
            "Circle ops alert action must be acknowledge, resolve, or set_workflow_state.".into(),
        ));
    }
    let new_status = match action.as_str() {
        "resolve" => "resolved",
        "set_workflow_state" | "acknowledge" => "acknowledged",
        _ => "acknowledged",
    };
    let workflow_state = if action == "set_workflow_state" {
        Some(normalize_ops_alert_workflow_state(
            payload.workflow_state.as_deref(),
        )?)
    } else {
        None
    };
    let note = payload.note.unwrap_or_default().trim().to_string();
    if note.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "Alert action note must be 1000 characters or fewer.".into(),
        ));
    }

    let row = sqlx::query(
        r#"
        UPDATE circle_ops_alerts
           SET status = $3,
               resolved_at = CASE WHEN $3 = 'resolved' THEN NOW() ELSE resolved_at END,
               workflow_state = COALESCE($7::TEXT, workflow_state),
               workflow_note = CASE WHEN $7::TEXT IS NULL THEN workflow_note ELSE NULLIF($5, '') END,
               workflow_updated_at = CASE WHEN $7::TEXT IS NULL THEN workflow_updated_at ELSE NOW() END,
               workflow_updated_by = CASE WHEN $7::TEXT IS NULL THEN workflow_updated_by ELSE $6::UUID END,
               details = COALESCE(details, '{}'::JSONB) || JSONB_BUILD_OBJECT(
                 'last_action', $4,
                 'last_action_note', NULLIF($5, ''),
                 'last_action_by', $6,
                 'last_action_at', NOW(),
                 'workflow_state', COALESCE($7::TEXT, workflow_state)
               )
         WHERE id = $1
           AND circle_id = $2
           AND status IN ('open', 'acknowledged')
         RETURNING alert_type, severity, status, workflow_state
        "#,
    )
    .bind(alert_id)
    .bind(circle_id)
    .bind(new_status)
    .bind(&action)
    .bind(&note)
    .bind(user.id.to_string())
    .bind(workflow_state.as_deref())
    .fetch_optional(&c_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle ops alert not found.".into()))?;

    let alert_type = row.try_get::<String, _>("alert_type").unwrap_or_default();
    let severity = row.try_get::<String, _>("severity").unwrap_or_default();
    let workflow_state = row
        .try_get::<String, _>("workflow_state")
        .unwrap_or_else(|_| "triage".to_string());
    crate::community::audit::log(
        &c_pool,
        user.id,
        &format!("circle.ops_alert.{}", action),
        "circle_ops_alert",
        Some(alert_id),
        None,
        Some(serde_json::json!({
            "circle_id": circle_id,
            "alert_type": alert_type,
            "severity": severity,
            "status": new_status,
            "workflow_state": workflow_state,
            "has_note": !note.is_empty(),
        })),
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "status": new_status,
        "workflow_state": workflow_state,
    })))
}

async fn get_circle_report_queue(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;

    let rows = sqlx::query(
        r#"
        SELECT cr.id,
               cr.post_id,
               cr.reporter_id,
               cr.reason,
               cr.reporter_note,
               cr.status,
               cr.admin_notes,
               cr.created_at,
               p.user_id AS post_author_id,
               p.post_type,
               LEFT(COALESCE(p.content_sanitized, p.content), 500) AS post_content
          FROM content_reports cr
          JOIN posts p ON p.id = cr.post_id
         WHERE p.circle_id = $1
           AND cr.status = 'pending'
         ORDER BY cr.created_at ASC
         LIMIT 50
        "#,
    )
    .bind(circle_id)
    .fetch_all(&c_pool)
    .await?;

    let mut user_ids = std::collections::HashSet::new();
    for row in &rows {
        if let Ok(id) = row.try_get::<Uuid, _>("reporter_id") {
            user_ids.insert(id);
        }
        if let Ok(id) = row.try_get::<Uuid, _>("post_author_id") {
            user_ids.insert(id);
        }
    }
    let user_ids: Vec<Uuid> = user_ids.into_iter().collect();
    let users = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids)
        .await
        .unwrap_or_default();

    let reports: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            let reporter_id = row.try_get::<Uuid, _>("reporter_id").unwrap_or_default();
            let author_id = row.try_get::<Uuid, _>("post_author_id").unwrap_or_default();
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").unwrap_or_default(),
                "post_id": row.try_get::<Uuid, _>("post_id").unwrap_or_default(),
                "reporter_id": reporter_id,
                "reporter_name": users.get(&reporter_id)
                    .map(|info| info.display_name.clone())
                    .unwrap_or_else(|| "Unknown reporter".to_string()),
                "post_author_id": author_id,
                "post_author_name": users.get(&author_id)
                    .map(|info| info.display_name.clone())
                    .unwrap_or_else(|| "Unknown author".to_string()),
                "post_type": row.try_get::<String, _>("post_type").unwrap_or_else(|_| "general".to_string()),
                "post_content": row.try_get::<String, _>("post_content").unwrap_or_default(),
                "reason": row.try_get::<String, _>("reason").unwrap_or_default(),
                "reporter_note": row.try_get::<Option<String>, _>("reporter_note").ok().flatten(),
                "status": row.try_get::<String, _>("status").unwrap_or_else(|_| "pending".to_string()),
                "admin_notes": row.try_get::<Option<String>, _>("admin_notes").ok().flatten(),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "circle_id": circle_id,
        "role": role,
        "reports": reports,
    })))
}

async fn take_circle_report_action(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, report_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<models::AdminReportActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;

    if !matches!(payload.action.as_str(), "hide_post" | "dismiss_report") {
        return Err(AppError::BadRequest(
            "Circle report actions are limited to hide_post or dismiss_report.".into(),
        ));
    }

    let notes = payload
        .admin_notes
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    if notes.is_empty() {
        return Err(AppError::BadRequest(
            "Moderation notes are required.".to_string(),
        ));
    }
    if notes.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "Moderation notes must be 1000 characters or fewer.".to_string(),
        ));
    }

    let belongs_to_circle = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1
            FROM content_reports cr
            JOIN posts p ON p.id = cr.post_id
           WHERE cr.id = $1
             AND p.circle_id = $2
        )
        "#,
    )
    .bind(report_id)
    .bind(circle_id)
    .fetch_one(&c_pool)
    .await?;
    if !belongs_to_circle {
        return Err(AppError::NotFound(
            "Report not found for this Circle.".into(),
        ));
    }

    service::action_on_report(&c_pool, report_id, user.id, &payload.action, notes).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn take_circle_report_bulk_action(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<CircleReportBulkActionReq>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;

    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;

    if !matches!(payload.action.as_str(), "hide_posts" | "dismiss_reports") {
        return Err(AppError::BadRequest(
            "Circle bulk report action must be hide_posts or dismiss_reports.".into(),
        ));
    }

    let mut deduped_report_ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for report_id in payload.report_ids {
        if seen.insert(report_id) {
            deduped_report_ids.push(report_id);
        }
    }
    if deduped_report_ids.is_empty() {
        return Err(AppError::BadRequest(
            "At least one report must be selected.".into(),
        ));
    }
    if deduped_report_ids.len() > 50 {
        return Err(AppError::BadRequest(
            "Bulk report actions are limited to 50 reports.".into(),
        ));
    }

    let notes = payload
        .admin_notes
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_string();
    if notes.is_empty() {
        return Err(AppError::BadRequest(
            "Moderation notes are required.".to_string(),
        ));
    }
    if notes.chars().count() > 1000 {
        return Err(AppError::BadRequest(
            "Moderation notes must be 1000 characters or fewer.".to_string(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let rows = sqlx::query(
        r#"
        SELECT cr.id,
               cr.post_id,
               cr.status,
               p.user_id AS post_author_id,
               p.is_hidden,
               p.hidden_reason
          FROM content_reports cr
          JOIN posts p ON p.id = cr.post_id
         WHERE cr.id = ANY($1)
           AND p.circle_id = $2
         FOR UPDATE OF cr, p
        "#,
    )
    .bind(&deduped_report_ids)
    .bind(circle_id)
    .fetch_all(&mut *tx)
    .await?;

    if rows.len() != deduped_report_ids.len() {
        return Err(AppError::NotFound(
            "One or more reports were not found for this Circle.".into(),
        ));
    }

    let mut report_ids = Vec::with_capacity(rows.len());
    let mut post_ids = Vec::new();
    let mut target_user_ids = Vec::new();
    let mut previous_posts = Vec::new();
    let mut post_seen = std::collections::HashSet::new();
    for row in rows {
        let report_id: Uuid = row.try_get("id")?;
        let post_id: Uuid = row.try_get("post_id")?;
        let status: String = row.try_get("status")?;
        if status != "pending" {
            return Err(AppError::Conflict(
                "One or more reports have already been moderated.".into(),
            ));
        }

        report_ids.push(report_id);
        target_user_ids.push(row.try_get::<Uuid, _>("post_author_id")?);
        previous_posts.push(serde_json::json!({
            "report_id": report_id,
            "post_id": post_id,
            "is_hidden": row.try_get::<bool, _>("is_hidden")?,
            "hidden_reason": row.try_get::<Option<String>, _>("hidden_reason")?,
        }));
        if post_seen.insert(post_id) {
            post_ids.push(post_id);
        }
    }

    let (report_status, audit_action, hidden_reason) = match payload.action.as_str() {
        "hide_posts" => (
            "resolved",
            "circle.report.bulk_hide_posts",
            Some(format!("Hidden after bulk report triage: {}", notes)),
        ),
        "dismiss_reports" => ("dismissed", "circle.report.bulk_dismiss_reports", None),
        _ => unreachable!("bulk report action was allowlisted above"),
    };

    if let Some(reason) = hidden_reason.as_deref() {
        let post_result = sqlx::query(
            "UPDATE posts SET is_hidden = true, hidden_reason = $1, updated_at = NOW() WHERE id = ANY($2)",
        )
        .bind(reason)
        .bind(&post_ids)
        .execute(&mut *tx)
        .await?;
        if post_result.rows_affected() != post_ids.len() as u64 {
            return Err(AppError::Conflict(
                "One or more reported posts could not be updated.".into(),
            ));
        }
    }

    let report_result = sqlx::query(
        "UPDATE content_reports SET status = $1, admin_notes = $2, updated_at = NOW() WHERE id = ANY($3) AND status = 'pending'",
    )
    .bind(report_status)
    .bind(&notes)
    .bind(&report_ids)
    .execute(&mut *tx)
    .await?;
    if report_result.rows_affected() != report_ids.len() as u64 {
        return Err(AppError::Conflict(
            "One or more reports have already been moderated.".into(),
        ));
    }

    sqlx::query(
        r#"INSERT INTO community_audit_logs
           (actor_user_id, action, entity_type, entity_id, target_user_id, details)
           VALUES ($1, $2, 'circle_report_bulk_action', $3, $4, $5)"#,
    )
    .bind(user.id)
    .bind(audit_action)
    .bind(circle_id)
    .bind(Option::<Uuid>::None)
    .bind(serde_json::json!({
        "circle_id": circle_id,
        "action": payload.action,
        "report_ids": report_ids,
        "post_ids": post_ids,
        "target_user_ids": target_user_ids,
        "report_count": report_result.rows_affected(),
        "post_count": post_ids.len(),
        "new_report_status": report_status,
        "admin_notes": notes,
        "previous_posts": previous_posts,
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "action": audit_action,
        "report_count": report_result.rows_affected(),
        "post_count": post_ids.len(),
    })))
}

async fn update_circle_manage_settings(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<CircleManageSettingsReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_manage_access(&state, &c_pool, circle_id, user.id).await?;
    let is_platform_admin = role == "platform_admin";
    let is_owner = role == "owner" || is_platform_admin;
    let is_admin_or_owner = matches!(role.as_str(), "owner" | "admin" | "platform_admin");

    if payload.slug.is_some() && !is_owner {
        return Err(AppError::Forbidden(
            "Only the Circle owner or platform admin can change the Circle slug.".into(),
        ));
    }

    let admin_only_payload = payload.name.is_some()
        || payload.description.is_some()
        || payload.avatar_emoji.is_some()
        || payload.is_public.is_some()
        || payload.visibility.is_some()
        || payload.join_policy.is_some()
        || payload.circle_type.is_some()
        || payload.category.is_some()
        || payload.language.is_some()
        || payload.location_text.is_some()
        || payload.rules_text.is_some()
        || payload.investment_disclaimer.is_some()
        || payload.join_approval_required.is_some()
        || payload.auto_approve_verified_investors.is_some()
        || payload.media_uploads_enabled.is_some()
        || payload.polls_enabled.is_some()
        || payload.anonymous_posting_enabled.is_some()
        || payload.link_posting_enabled.is_some()
        || payload.allowed_post_types.is_some()
        || payload.required_post_tags.is_some()
        || payload.announcement_comments_enabled.is_some()
        || payload.onboarding_enabled.is_some();

    if admin_only_payload && !is_admin_or_owner {
        return Err(AppError::Forbidden(
            "Moderators can update moderation controls, but not Circle owner/admin settings."
                .into(),
        ));
    }

    let name = normalize_manage_text(payload.name, 100, "Circle name")?;
    if matches!(name.as_deref(), Some("")) {
        return Err(AppError::BadRequest("Circle name is required.".into()));
    }
    let description = normalize_manage_text(payload.description, 500, "Description")?;
    let avatar_emoji = normalize_manage_text(payload.avatar_emoji, 10, "Circle icon")?;
    let slug = validate_circle_slug(payload.slug)?;
    let visibility = ensure_manage_value_allowed(
        payload.visibility,
        &["public", "private", "hidden"],
        "visibility",
    )?;
    let join_policy = ensure_manage_value_allowed(
        payload.join_policy,
        &[
            "open",
            "request",
            "invite_only",
            "holder_only",
            "kyc_required",
        ],
        "join policy",
    )?;
    let circle_type = ensure_manage_value_allowed(
        payload.circle_type,
        &[
            "social",
            "asset",
            "topic",
            "expert",
            "private_investor",
            "official",
        ],
        "circle type",
    )?;
    let category = normalize_manage_text(payload.category, 80, "Category")?;
    let language = normalize_manage_text(payload.language, 16, "Language")?.and_then(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value.to_ascii_lowercase())
        }
    });
    let location_text = normalize_manage_text(payload.location_text, 120, "Location")?;
    let rules_text = normalize_manage_text(payload.rules_text, 5000, "Rules")?;
    let investment_disclaimer =
        normalize_manage_text(payload.investment_disclaimer, 2000, "Investment disclaimer")?;
    let blocked_words = normalize_manage_keywords(payload.blocked_words, "Blocked words")?;
    let investment_risk_keywords =
        normalize_manage_keywords(payload.investment_risk_keywords, "Investment risk keywords")?;
    let allowed_post_types = normalize_manage_post_types(payload.allowed_post_types)?;
    let required_post_tags = if payload.required_post_tags.is_some() {
        Some(normalize_post_tags(payload.required_post_tags)?)
    } else {
        None
    };
    let slow_mode_seconds = match payload.slow_mode_seconds {
        Some(value) if !(0..=86400).contains(&value) => {
            return Err(AppError::BadRequest(
                "Slow mode must be between 0 and 86400 seconds.".into(),
            ));
        }
        other => other,
    };

    use sqlx::Row;
    let updated = sqlx::query(
        r#"
        UPDATE circles SET
          name = COALESCE($2, name),
          avatar_emoji = COALESCE($3, avatar_emoji),
          description = CASE WHEN $4::BOOL THEN NULLIF($5, '') ELSE description END,
          slug = COALESCE($6, slug),
          is_public = COALESCE($7, is_public),
          visibility = COALESCE($8, visibility),
          join_policy = COALESCE($9, join_policy),
          circle_type = COALESCE($10, circle_type),
          category = CASE WHEN $11::BOOL THEN NULLIF($12, '') ELSE category END,
          language = COALESCE($13, language),
          location_text = CASE WHEN $14::BOOL THEN NULLIF($15, '') ELSE location_text END,
          rules_text = CASE WHEN $16::BOOL THEN NULLIF($17, '') ELSE rules_text END,
          investment_disclaimer = CASE WHEN $18::BOOL THEN NULLIF($19, '') ELSE investment_disclaimer END,
          join_approval_required = COALESCE($20, join_approval_required),
          auto_approve_verified_investors = COALESCE($21, auto_approve_verified_investors),
          media_uploads_enabled = COALESCE($22, media_uploads_enabled),
          polls_enabled = COALESCE($23, polls_enabled),
          anonymous_posting_enabled = COALESCE($24, anonymous_posting_enabled),
          link_posting_enabled = COALESCE($25, link_posting_enabled),
          first_post_approval_enabled = COALESCE($26, first_post_approval_enabled),
          slow_mode_seconds = COALESCE($27, slow_mode_seconds),
          blocked_words = CASE WHEN $28::BOOL THEN $29 ELSE blocked_words END,
          investment_risk_keywords = CASE WHEN $30::BOOL THEN $31 ELSE investment_risk_keywords END,
          allowed_post_types = CASE WHEN $32::BOOL THEN $33 ELSE allowed_post_types END,
          required_post_tags = CASE WHEN $34::BOOL THEN $35 ELSE required_post_tags END,
          announcement_comments_enabled = COALESCE($36, announcement_comments_enabled),
          onboarding_enabled = COALESCE($37, onboarding_enabled),
          updated_at = NOW()
        WHERE id = $1
        RETURNING id, slug
        "#,
    )
    .bind(circle_id)
    .bind(name.as_deref())
    .bind(avatar_emoji.as_deref())
    .bind(description.is_some())
    .bind(description.as_deref())
    .bind(slug.as_deref())
    .bind(payload.is_public)
    .bind(visibility.as_deref())
    .bind(join_policy.as_deref())
    .bind(circle_type.as_deref())
    .bind(category.is_some())
    .bind(category.as_deref())
    .bind(language.as_deref())
    .bind(location_text.is_some())
    .bind(location_text.as_deref())
    .bind(rules_text.is_some())
    .bind(rules_text.as_deref())
    .bind(investment_disclaimer.is_some())
    .bind(investment_disclaimer.as_deref())
    .bind(payload.join_approval_required)
    .bind(payload.auto_approve_verified_investors)
    .bind(payload.media_uploads_enabled)
    .bind(payload.polls_enabled)
    .bind(payload.anonymous_posting_enabled)
    .bind(payload.link_posting_enabled)
    .bind(payload.first_post_approval_enabled)
    .bind(slow_mode_seconds)
    .bind(blocked_words.is_some())
    .bind(blocked_words.as_ref())
    .bind(investment_risk_keywords.is_some())
    .bind(investment_risk_keywords.as_ref())
    .bind(allowed_post_types.is_some())
    .bind(allowed_post_types.as_ref())
    .bind(required_post_tags.is_some())
    .bind(required_post_tags.as_ref())
    .bind(payload.announcement_comments_enabled)
    .bind(payload.onboarding_enabled)
    .fetch_one(&c_pool)
    .await?;

    crate::community::audit::log(
        &c_pool,
        user.id,
        "circle.manage.update",
        "circle",
        Some(circle_id),
        None,
        Some(serde_json::json!({
            "role": role,
            "settings": {
                "name": name,
                "slug": slug,
                "visibility": visibility,
                "join_policy": join_policy,
                "circle_type": circle_type,
                "category": category,
                "language": language,
                "location_text": location_text,
                "join_approval_required": payload.join_approval_required,
                "auto_approve_verified_investors": payload.auto_approve_verified_investors,
                "media_uploads_enabled": payload.media_uploads_enabled,
                "polls_enabled": payload.polls_enabled,
                "anonymous_posting_enabled": payload.anonymous_posting_enabled,
                "link_posting_enabled": payload.link_posting_enabled,
                "first_post_approval_enabled": payload.first_post_approval_enabled,
                "slow_mode_seconds": slow_mode_seconds,
                "blocked_words_count": blocked_words.as_ref().map(|items| items.len()),
                "investment_risk_keywords_count": investment_risk_keywords.as_ref().map(|items| items.len()),
                "allowed_post_types": allowed_post_types,
                "required_post_tags": required_post_tags,
                "announcement_comments_enabled": payload.announcement_comments_enabled,
                "onboarding_enabled": payload.onboarding_enabled,
            }
        })),
    )
    .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "id": updated.try_get::<Uuid, _>("id").unwrap_or(circle_id),
        "slug": updated.try_get::<String, _>("slug").unwrap_or_default(),
    })))
}

async fn delete_own_circle_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::delete_own_circle(&c_pool, user.id, circle_id).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_circle_members(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let members = crate::community::circles::get_circle_members(&c_pool, circle_id).await?;
    let enriched = enrich_circle_members(&state, members).await;
    Ok(Json(serde_json::json!({"members": enriched})))
}

async fn join_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, user.id, "join").await?;

    let c_pool = get_community_pool(&state)?;

    // W3.1: Check token gate requirement before allowing join
    crate::community::circles::check_token_gate(&c_pool, &state.db, user.id, circle_id).await?;
    crate::community::circles::check_kyc_gate(&c_pool, &state.db, user.id, circle_id).await?;

    crate::community::circles::join_circle(&c_pool, user.id, circle_id).await?;

    // Award XP
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "circle_joined",
        Some("Joined a circle"),
        None,
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
struct LeaveCircleReq {
    circle_id: Uuid,
}

async fn leave_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<LeaveCircleReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, user.id, "leave").await?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::leave_circle(&c_pool, user.id, payload.circle_id).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
struct InviteReq {
    invitee_id: Uuid,
}

async fn send_circle_invite(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<InviteReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, user.id, "invite").await?;

    let c_pool = get_community_pool(&state)?;

    // Level gate: Level 3 required to invite (M4-BE.10)
    crate::community::xp::check_level_gate(
        &c_pool,
        user.id,
        crate::community::xp::GatedFeature::InviteToCircle,
    )
    .await?;

    let invite =
        crate::community::circles::send_invite(&c_pool, user.id, payload.invitee_id, circle_id)
            .await?;
    Ok(Json(invite))
}

async fn kick_circle_member(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((circle_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::kick_member(&c_pool, user.id, target_user_id, circle_id).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

// ─── M4-BE.11: Circle Role Management ──────────────────────────────────────

#[derive(Deserialize)]
struct UpdateRoleReq {
    user_id: Uuid,
    role: String, // "admin" | "verified_expert" | "member"
}

async fn update_circle_member_role(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<UpdateRoleReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::update_member_role(
        &c_pool,
        user.id,
        payload.user_id,
        circle_id,
        &payload.role,
    )
    .await?;

    Ok(Json(serde_json::json!({"success": true})))
}

// ─── M4-BE.12: Transfer Circle Ownership ───────────────────────────────────

#[derive(Deserialize)]
struct TransferOwnershipReq {
    new_owner_id: Uuid,
}

async fn transfer_circle_ownership(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<TransferOwnershipReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::transfer_ownership(
        &c_pool,
        user.id,
        payload.new_owner_id,
        circle_id,
    )
    .await?;

    // Notify the new owner
    let _ = crate::community::notifications::notify_user(
        &c_pool,
        payload.new_owner_id,
        Some(user.id),
        "circle_ownership_transferred",
        Some(circle_id),
        "You are now the owner of this circle!",
        Some(&format!("/community?tab=my-circle")),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

// ─── M4-BE.13: Circle Privacy Settings ─────────────────────────────────────

#[derive(Deserialize)]
struct UpdatePrivacyReq {
    is_public: bool,
}

async fn update_circle_privacy(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<UpdatePrivacyReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::update_circle_privacy(
        &c_pool,
        user.id,
        circle_id,
        payload.is_public,
    )
    .await?;

    Ok(Json(
        serde_json::json!({"success": true, "is_public": payload.is_public}),
    ))
}

// ─── W3.1: Token-Gated Circle Management ───────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct UpdateTokenGateReq {
    /// If None, clears the token gate
    asset_id: Option<Uuid>,
    /// Minimum value in cents (e.g. 100000 = $1,000)
    min_value_cents: Option<i64>,
}

/// POST /api/community/circles/:id/token-gate — set or clear the token gate
async fn update_circle_token_gate(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<UpdateTokenGateReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    // Validate min_value_cents is non-negative if provided
    if let Some(cents) = payload.min_value_cents {
        if cents < 0 {
            return Err(AppError::BadRequest(
                "min_value_cents must be non-negative".into(),
            ));
        }
    }

    let circle = crate::community::circles::update_token_gate(
        &c_pool,
        &state.db,
        user.id,
        circle_id,
        payload.asset_id,
        payload.min_value_cents,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "circle": circle,
    })))
}

// ─── M4-BE.15: Join Request Handlers ───────────────────────────────────────

/// POST /api/community/circles/:id/request — request to join a private circle
async fn request_to_join_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    // W3.1: Check token gate requirement before allowing join request
    crate::community::circles::check_token_gate(&c_pool, &state.db, user.id, circle_id).await?;
    crate::community::circles::check_kyc_gate(&c_pool, &state.db, user.id, circle_id).await?;

    let req = crate::community::circles::request_to_join(&c_pool, user.id, circle_id).await?;

    // Notify circle owner
    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM circles WHERE id = $1")
        .bind(circle_id)
        .fetch_optional(&c_pool)
        .await?;

    if let Some(oid) = owner_id {
        let _ = crate::community::notifications::notify_user(
            &c_pool,
            oid,
            Some(user.id),
            "circle_join_request",
            Some(circle_id),
            "Someone has requested to join your circle",
            Some(&format!("/community?tab=my-circle")),
        )
        .await;
    }

    Ok(Json(req))
}

/// DELETE /api/community/circles/:id/request — cancel your own join request
async fn cancel_join_request_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    // Find the pending request ID for this user + circle
    let req_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM circle_join_requests WHERE circle_id = $1 AND user_id = $2 AND status = 'pending'"
    )
    .bind(circle_id)
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await?;

    let req_id = req_id.ok_or_else(|| AppError::NotFound("No pending request found.".into()))?;
    crate::community::circles::cancel_join_request(&c_pool, user.id, req_id).await?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// GET /api/community/circles/:id/requests — list pending requests (owner/admin)
async fn list_join_requests(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let requests =
        crate::community::circles::get_pending_join_requests(&c_pool, user.id, circle_id).await?;

    // Enrich with names from core DB
    let user_ids: Vec<Uuid> = requests.iter().map(|r| r.user_id).collect();
    let names = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &user_ids,
    )
    .await?;

    let enriched: Vec<serde_json::Value> = requests.iter().map(|r| {
        let info = names.get(&r.user_id);
        serde_json::json!({
            "id": r.id,
            "circle_id": r.circle_id,
            "user_id": r.user_id,
            "user_name": info.map(|i| i.display_name.clone()).unwrap_or_else(|| "Unknown".into()),
            "user_avatar": info.and_then(|i| i.avatar_url.clone()),
            "status": r.status,
            "created_at": r.created_at,
        })
    }).collect();

    Ok(Json(serde_json::json!({"requests": enriched})))
}

/// GET /api/community/circles/requests/mine — my own pending join requests
async fn get_my_join_requests_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let requests = crate::community::circles::get_my_join_requests(&c_pool, user.id).await?;
    Ok(Json(serde_json::json!({"requests": requests})))
}

/// POST /api/community/circles/requests/:req_id/approve — approve a join request
async fn approve_join_request_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let approved_user_id =
        crate::community::circles::approve_join_request(&c_pool, user.id, request_id).await?;

    // Award XP to the new member
    let _ = crate::community::xp::award_xp(
        &c_pool,
        approved_user_id,
        "circle_joined",
        Some("Joined a circle via request"),
        None,
    )
    .await;

    // Notify the approved user
    let _ = crate::community::notifications::notify_user(
        &c_pool,
        approved_user_id,
        Some(user.id),
        "circle_request_approved",
        None,
        "Your request to join the circle has been approved! Welcome!",
        Some("/community?tab=my-circle"),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

/// POST /api/community/circles/requests/:req_id/decline — decline a join request
async fn decline_join_request_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(request_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let declined_user_id =
        crate::community::circles::decline_join_request(&c_pool, user.id, request_id).await?;

    // Notify the declined user
    let _ = crate::community::notifications::notify_user(
        &c_pool,
        declined_user_id,
        None,
        "circle_request_declined",
        None,
        "Your request to join the circle was not approved this time.",
        Some("/community?tab=my-circle"),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_my_invites(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let invites = crate::community::circles::get_my_invites(&c_pool, user.id).await?;
    Ok(Json(serde_json::json!({"invites": invites})))
}

async fn accept_invite(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(invite_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let invite_circle_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT circle_id FROM circle_invites
         WHERE id = $1
           AND invitee_id = $2
           AND status = 'pending'
           AND expires_at > NOW()",
    )
    .bind(invite_id)
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await?;
    if let Some(circle_id) = invite_circle_id {
        crate::community::circles::check_token_gate(&c_pool, &state.db, user.id, circle_id).await?;
        crate::community::circles::check_kyc_gate(&c_pool, &state.db, user.id, circle_id).await?;
    }
    crate::community::circles::accept_invite(&c_pool, user.id, invite_id).await?;

    // Award XP
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "circle_invite_accepted",
        Some("Accepted circle invite"),
        None,
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn decline_invite(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(invite_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::decline_invite(&c_pool, user.id, invite_id).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

// Phase 3 task 25 + roadmap 14.8.9: global user XP leaderboard. Decorates
// each entry with display_name + avatar_url + is_self for the viewer so the
// rendered table can highlight the viewer's row without an extra fetch.
//
// Accepts `period=week|month|alltime` to window XP aggregation and
// `scope=global` for spec-compatibility (only `global` is supported today).
#[derive(Deserialize)]
struct LeaderboardQuery {
    limit: Option<i64>,
    period: Option<String>,
    #[allow(dead_code)] // currently only `global` is supported; reserved for future "circle"
    scope: Option<String>,
}

async fn get_global_leaderboard(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<impl IntoResponse, AppError> {
    let viewer = middleware::get_current_user(&jar, &state.db).await;
    let c_pool = get_community_pool(&state)?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let period = crate::community::xp::LeaderboardPeriod::parse(query.period.as_deref());

    let entries =
        crate::community::xp::get_user_leaderboard_for_period(&c_pool, period, limit).await?;

    let user_ids: Vec<Uuid> = entries.iter().map(|e| e.user_id).collect();
    let authors = if user_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?
    };

    // Respect leaderboard_preferences.visible=false from the investor
    // leaderboard schema: users who opted out of public ranking get their
    // display name and avatar suppressed (replaced with an anonymized
    // "Investor #abcdef" placeholder). The viewer themselves is always
    // shown un-anonymized so they can find their own row.
    let hidden_user_ids = if user_ids.is_empty() {
        std::collections::HashSet::new()
    } else {
        fetch_hidden_leaderboard_user_ids(&state.db, &user_ids).await?
    };

    let viewer_id = viewer.as_ref().map(|v| v.id);
    let rows: Vec<serde_json::Value> = entries
        .iter()
        .enumerate()
        .map(|(idx, e)| {
            let info = authors.get(&e.user_id);
            let is_self = viewer_id == Some(e.user_id);
            let is_hidden = hidden_user_ids.contains(&e.user_id) && !is_self;
            let (display_name, avatar_url) = if is_hidden {
                (format!("Investor #{}", &e.user_id.to_string()[..6]), None)
            } else {
                (
                    info.map(|a| a.display_name.clone())
                        .unwrap_or_else(|| "Anonymous".into()),
                    info.and_then(|a| a.avatar_url.clone()),
                )
            };
            serde_json::json!({
                "rank": idx + 1,
                "user_id": e.user_id,
                "display_name": display_name,
                "avatar_url": avatar_url,
                "xp_total": e.xp_total,
                "level": e.level,
                "level_name": e.level_name,
                "login_streak": e.login_streak,
                "is_self": is_self,
                "anonymized": is_hidden,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "leaderboard": rows,
        "period": period.to_string(),
        "scope": "global",
    })))
}

/// Return the set of user IDs (from the input slice) whose
/// `leaderboard_preferences.visible` is explicitly false. Users without a
/// preferences row default to visible (no anonymization), matching the
/// public-by-default semantics elsewhere in the community surface.
async fn fetch_hidden_leaderboard_user_ids(
    main_pool: &sqlx::PgPool,
    user_ids: &[Uuid],
) -> Result<std::collections::HashSet<Uuid>, AppError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM leaderboard_preferences
         WHERE user_id = ANY($1) AND visible = FALSE",
    )
    .bind(user_ids)
    .fetch_all(main_pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

async fn get_circle_leaderboard(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let entries = crate::community::circles::get_circle_leaderboard(&c_pool, 20).await?;
    Ok(Json(serde_json::json!({"circles": entries})))
}

// ─── Property Reviews Handlers (M5) ───────────────────────────────────────

async fn list_asset_reviews(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let viewer_id = middleware::get_current_user(&jar, &state.db)
        .await
        .map(|u| u.id);
    let limit = q
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(20);
    let offset = q
        .get("offset")
        .and_then(|o| o.parse::<i64>().ok())
        .unwrap_or(0);

    let c_pool = get_community_pool(&state)?;
    let reviews = crate::community::reviews::list_reviews_for_asset(
        &c_pool, asset_id, viewer_id, limit, offset,
    )
    .await?;
    let stats = crate::community::reviews::get_review_stats(&c_pool, asset_id).await?;

    // Also get my review if logged in
    let my_review = if let Some(vid) = viewer_id {
        crate::community::reviews::get_my_review(&c_pool, vid, asset_id).await?
    } else {
        None
    };

    Ok(Json(serde_json::json!({
        "stats": stats,
        "reviews": reviews,
        "my_review": my_review,
    })))
}

#[derive(serde::Deserialize)]
struct UpsertReviewReq {
    rating: i16,
    content: String,
}

async fn upsert_asset_review(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
    Json(payload): Json<UpsertReviewReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let review = crate::community::reviews::upsert_review(
        &c_pool,
        &state.db,
        user.id,
        asset_id,
        payload.rating,
        &payload.content,
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "success": true, "review": review }),
    ))
}

async fn delete_asset_review(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>, // We use the asset ID for standardizing route structure but delete relies on lookup
    axum::extract::Query(_q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    // the route is /api/community/assets/:id/reviews, but a user only has one review per asset
    // So we can find the review by user_id and asset_id.
    let c_pool = get_community_pool(&state)?;

    // Grab review ID first
    let review = crate::community::reviews::get_my_review(&c_pool, user.id, asset_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Review not found".into()))?;

    crate::community::reviews::delete_review(&c_pool, user.id, review.id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn toggle_review_upvote(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(review_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let (is_upvoted, count) =
        crate::community::reviews::toggle_review_upvote(&c_pool, user.id, review_id).await?;

    Ok(Json(
        serde_json::json!({ "is_upvoted": is_upvoted, "helpful_count": count }),
    ))
}

// ─── Challenges Handlers (M5) ──────────────────────────────────────────────

async fn list_challenges(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let challenges =
        crate::community::challenges::list_challenges_for_user(&c_pool, user.id).await?;

    Ok(Json(serde_json::json!({ "challenges": challenges })))
}

async fn get_circle_announcements(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_read_access(&state, &c_pool, circle_id, Some(user.id)).await?;

    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT p.id,
               p.user_id,
               p.post_type,
               COALESCE(p.content_sanitized, p.content) AS content,
               COALESCE(p.content_tags, '{}'::TEXT[]) AS content_tags,
               p.is_pinned,
               p.comment_count,
               p.reaction_count,
               p.created_at,
               COALESCE(c.announcement_comments_enabled, TRUE) AS comments_enabled
        FROM posts p
        JOIN circles c ON c.id = p.circle_id
        WHERE p.circle_id = $1
          AND p.is_hidden = FALSE
          AND p.post_type IN ('announcement', 'official_update')
        ORDER BY p.is_pinned DESC, p.created_at DESC
        LIMIT 20
        "#,
    )
    .bind(circle_id)
    .fetch_all(&c_pool)
    .await?;

    let author_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|row| row.try_get::<Uuid, _>("user_id").ok())
        .collect();
    let authors = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &author_ids)
        .await
        .unwrap_or_default();

    let announcements: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let author_id: Uuid = row.try_get("user_id").unwrap_or_default();
            let author = authors.get(&author_id);
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "circle_id": circle_id,
                "post_type": row.try_get::<String, _>("post_type").unwrap_or_else(|_| "announcement".to_string()),
                "content": row.try_get::<String, _>("content").unwrap_or_default(),
                "content_tags": row.try_get::<Vec<String>, _>("content_tags").unwrap_or_default(),
                "is_pinned": row.try_get::<bool, _>("is_pinned").unwrap_or(false),
                "comments_enabled": row.try_get::<bool, _>("comments_enabled").unwrap_or(true),
                "comment_count": row.try_get::<i32, _>("comment_count").unwrap_or(0),
                "reaction_count": row.try_get::<i32, _>("reaction_count").unwrap_or(0),
                "author_name": author
                    .map(|info| info.display_name.clone())
                    .unwrap_or_else(|| "POOOL".to_string()),
                "author_avatar": author.and_then(|info| info.avatar_url.clone()),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "announcements": announcements,
        "scope": "circle",
        "circle_id": circle_id,
    })))
}

async fn get_circle_events(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_read_access(&state, &c_pool, circle_id, Some(user.id)).await?;
    let events = crate::community::amas::list_circle_amas(&c_pool, circle_id).await?;

    Ok(Json(serde_json::json!({
        "events": events,
        "scope": "circle",
        "circle_id": circle_id,
        "notifications_feature_flagged": true,
    })))
}

#[derive(Deserialize)]
struct CircleResourceCreateReq {
    title: String,
    description: Option<String>,
    resource_type: Option<String>,
    access_scope: Option<String>,
    url: Option<String>,
    storage_object_path: Option<String>,
    is_official: Option<bool>,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size_bytes: Option<i64>,
    sha256_hex: Option<String>,
    version_label: Option<String>,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    requires_download: Option<bool>,
    change_note: Option<String>,
    upload_status: Option<String>,
    retention_policy: Option<String>,
    retention_until: Option<chrono::DateTime<chrono::Utc>>,
    review_required_at: Option<chrono::DateTime<chrono::Utc>>,
    document_lifecycle_notes: Option<String>,
}

#[derive(Deserialize)]
struct CircleResourceUpdateReq {
    title: Option<String>,
    description: Option<String>,
    resource_type: Option<String>,
    access_scope: Option<String>,
    is_official: Option<bool>,
    is_active: Option<bool>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    upload_status: Option<String>,
    retention_policy: Option<String>,
    retention_until: Option<chrono::DateTime<chrono::Utc>>,
    review_required_at: Option<chrono::DateTime<chrono::Utc>>,
    document_lifecycle_notes: Option<String>,
}

#[derive(Deserialize)]
struct CircleResourceVersionReq {
    version_label: Option<String>,
    url: Option<String>,
    storage_object_path: Option<String>,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size_bytes: Option<i64>,
    sha256_hex: Option<String>,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    requires_download: Option<bool>,
    change_note: Option<String>,
    upload_status: Option<String>,
    retention_policy: Option<String>,
    retention_until: Option<chrono::DateTime<chrono::Utc>>,
    review_required_at: Option<chrono::DateTime<chrono::Utc>>,
    document_lifecycle_notes: Option<String>,
}

#[derive(Deserialize)]
struct CircleResourceVersionReviewReq {
    action: String,
    note: Option<String>,
}

#[derive(Deserialize)]
struct CircleResourceLifecycleReq {
    action: String,
    note: Option<String>,
    retention_policy: Option<String>,
    retention_until: Option<chrono::DateTime<chrono::Utc>>,
    review_required_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Default)]
struct CircleResourceUploadFields {
    title: Option<String>,
    description: Option<String>,
    resource_type: Option<String>,
    access_scope: Option<String>,
    is_official: bool,
    file_name: Option<String>,
    version_label: Option<String>,
    change_note: Option<String>,
    retention_policy: Option<String>,
    retention_until: Option<chrono::DateTime<chrono::Utc>>,
    review_required_at: Option<chrono::DateTime<chrono::Utc>>,
    document_lifecycle_notes: Option<String>,
}

fn is_circle_resource_admin_role(role: &str) -> bool {
    matches!(role, "owner" | "admin" | "platform_admin")
}

async fn ensure_circle_resource_admin_access(
    state: &AppState,
    pool: &sqlx::PgPool,
    circle_id: Uuid,
    user_id: Uuid,
) -> Result<String, AppError> {
    let role = ensure_circle_manage_access(state, pool, circle_id, user_id).await?;
    if is_circle_resource_admin_role(&role) {
        Ok(role)
    } else {
        Err(AppError::Forbidden(
            "Circle resource management requires owner, admin, or platform admin access.".into(),
        ))
    }
}

fn normalize_resource_required_text(
    value: &str,
    max_chars: usize,
    label: &str,
) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{label} is required.")));
    }
    if trimmed.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!(
            "{label} must be {max_chars} characters or fewer."
        )));
    }
    Ok(trimmed.to_string())
}

fn normalize_resource_optional_text(
    value: Option<String>,
    max_chars: usize,
    label: &str,
) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!(
            "{label} must be {max_chars} characters or fewer."
        )));
    }
    Ok(Some(trimmed.to_string()))
}

fn normalize_circle_resource_type(value: Option<String>) -> Result<String, AppError> {
    let normalized = value
        .as_deref()
        .map(canonical_community_code)
        .unwrap_or_else(|| "resource".to_string());
    match normalized.as_str() {
        "official_document" | "report" | "yield_report" | "guide" | "link" | "photo_update"
        | "community_resource" | "resource" => Ok(if normalized == "resource" {
            "community_resource".to_string()
        } else {
            normalized
        }),
        _ => Err(AppError::BadRequest("Invalid Circle resource type.".into())),
    }
}

fn normalize_circle_resource_access_scope(value: Option<String>) -> Result<String, AppError> {
    let normalized = value
        .as_deref()
        .map(canonical_community_code)
        .unwrap_or_else(|| "member".to_string());
    match normalized.as_str() {
        "public" | "member" | "holder_only" | "admin_only" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "Invalid Circle resource access scope.".into(),
        )),
    }
}

fn normalize_circle_resource_upload_status(
    value: Option<String>,
    default_status: &str,
) -> Result<String, AppError> {
    let normalized = value
        .as_deref()
        .map(canonical_community_code)
        .unwrap_or_else(|| default_status.to_string());
    match normalized.as_str() {
        "external" | "pending_upload" | "uploaded" | "rejected" | "expired" | "deleted" => {
            Ok(normalized)
        }
        _ => Err(AppError::BadRequest(
            "Invalid Circle resource upload status.".into(),
        )),
    }
}

fn normalize_circle_resource_retention_policy(value: Option<String>) -> Result<String, AppError> {
    let normalized = value
        .as_deref()
        .map(canonical_community_code)
        .unwrap_or_else(|| "standard".to_string());
    match normalized.as_str() {
        "standard" | "legal_hold" | "delete_after_expiry" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "Invalid Circle resource retention policy.".into(),
        )),
    }
}

fn normalize_circle_resource_lifecycle_action(value: &str) -> Result<String, AppError> {
    let normalized = canonical_community_code(value);
    match normalized.as_str() {
        "mark_reviewed"
        | "clear_review"
        | "mark_uploaded"
        | "mark_pending_upload"
        | "reject_upload"
        | "expire"
        | "soft_delete"
        | "restore"
        | "legal_hold"
        | "standard_retention"
        | "schedule_review" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "Invalid Circle resource lifecycle action.".into(),
        )),
    }
}

fn normalize_circle_resource_version_review_action(value: &str) -> Result<String, AppError> {
    let normalized = canonical_community_code(value);
    match normalized.as_str() {
        "approve" | "reject" | "mark_pending" => Ok(normalized),
        _ => Err(AppError::BadRequest(
            "Invalid Circle resource version review action.".into(),
        )),
    }
}

fn parse_circle_resource_upload_datetime(
    value: Option<String>,
    label: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    let Some(value) = normalize_resource_optional_text(value, 80, label)? else {
        return Ok(None);
    };
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|dt| Some(dt.with_timezone(&chrono::Utc)))
        .map_err(|_| AppError::BadRequest(format!("{label} must be an ISO-8601 timestamp.")))
}

fn parse_circle_resource_upload_bool(value: &str) -> bool {
    matches!(
        canonical_community_code(value).as_str(),
        "true" | "1" | "yes" | "on"
    )
}

fn normalize_circle_resource_source(
    url: Option<String>,
    storage_object_path: Option<String>,
    configured_bucket: Option<&str>,
) -> Result<(Option<String>, Option<String>), AppError> {
    let url = normalize_resource_optional_text(url, 2000, "Resource URL")?;
    let storage_object_path =
        normalize_resource_optional_text(storage_object_path, 1024, "Storage object path")?;

    match (url, storage_object_path) {
        (Some(url), None) => {
            if !is_safe_circle_resource_url(&url) {
                return Err(AppError::BadRequest(
                    "Resource URL must be http(s) or a safe relative path.".into(),
                ));
            }
            Ok((Some(url), None))
        }
        (None, Some(storage_path)) => {
            validate_circle_resource_storage_path_input(&storage_path, configured_bucket)?;
            Ok((None, Some(storage_path)))
        }
        (Some(_), Some(_)) => Err(AppError::BadRequest(
            "Provide either a Resource URL or a storage object path, not both.".into(),
        )),
        (None, None) => Err(AppError::BadRequest(
            "Resource URL or storage object path is required.".into(),
        )),
    }
}

fn validate_circle_resource_storage_path_input(
    value: &str,
    configured_bucket: Option<&str>,
) -> Result<(), AppError> {
    if value.chars().any(char::is_control) || value.contains("..") {
        return Err(AppError::BadRequest(
            "Storage object path contains unsafe characters.".into(),
        ));
    }
    if parse_circle_resource_storage_path(value, configured_bucket).is_err() {
        return Err(AppError::BadRequest(
            "Storage object path must be a valid gs:// path, configured-bucket object path, or existing GCS proxy path.".into(),
        ));
    }
    Ok(())
}

fn normalize_circle_resource_sha(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = normalize_resource_optional_text(value, 64, "SHA-256 hash")? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    let valid = normalized.len() == 64 && normalized.chars().all(|ch| ch.is_ascii_hexdigit());
    if !valid {
        return Err(AppError::BadRequest(
            "SHA-256 hash must be 64 lowercase hexadecimal characters.".into(),
        ));
    }
    Ok(Some(normalized))
}

fn validate_circle_resource_file_size(value: Option<i64>) -> Result<Option<i64>, AppError> {
    if value.is_some_and(|size| size < 0) {
        return Err(AppError::BadRequest(
            "File size must be zero or greater.".into(),
        ));
    }
    Ok(value)
}

async fn get_circle_resource_manage(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let rows = sqlx::query(
        r#"
        SELECT r.id,
               r.circle_id,
               r.asset_id,
               r.title,
               r.description,
               r.resource_type,
               r.access_scope,
               CASE WHEN r.storage_object_path IS NULL THEN r.url ELSE NULL END AS external_url,
               r.is_official,
               r.is_active,
               r.file_name,
               r.mime_type,
               r.file_size_bytes,
               r.sha256_hex,
               r.version_label,
               r.published_at,
               r.expires_at,
               r.requires_download,
               r.upload_status,
               r.retention_policy,
               r.retention_until,
               r.review_required_at,
               r.reviewed_at,
               r.reviewed_by,
               r.legal_hold,
               r.deleted_at,
               r.deleted_by,
               r.deletion_reason,
               r.document_lifecycle_notes,
               r.storage_object_path IS NOT NULL AS has_private_file,
               current_version.id AS current_version_id,
               COALESCE(version_counts.version_count, 0)::BIGINT AS version_count,
               r.created_at,
               r.updated_at
          FROM circle_resources r
          LEFT JOIN LATERAL (
            SELECT id
              FROM circle_resource_versions v
             WHERE v.resource_id = r.id
               AND v.is_current = TRUE
             ORDER BY v.created_at DESC
             LIMIT 1
          ) current_version ON TRUE
          LEFT JOIN (
            SELECT resource_id, COUNT(*)::BIGINT AS version_count
              FROM circle_resource_versions
             GROUP BY resource_id
          ) version_counts ON version_counts.resource_id = r.id
         WHERE r.circle_id = $1
         ORDER BY r.is_active DESC, r.is_official DESC, r.updated_at DESC, r.created_at DESC
         LIMIT 200
        "#,
    )
    .bind(circle_id)
    .fetch_all(&c_pool)
    .await?;

    let resources: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            let resource_id = row.try_get::<Uuid, _>("id").ok();
            serde_json::json!({
                "id": resource_id,
                "circle_id": row.try_get::<Uuid, _>("circle_id").ok(),
                "asset_id": row.try_get::<Option<Uuid>, _>("asset_id").ok().flatten(),
                "title": row.try_get::<String, _>("title").unwrap_or_default(),
                "description": row.try_get::<Option<String>, _>("description").ok().flatten(),
                "resource_type": row.try_get::<String, _>("resource_type").unwrap_or_else(|_| "community_resource".to_string()),
                "access_scope": row.try_get::<String, _>("access_scope").unwrap_or_else(|_| "member".to_string()),
                "external_url": row.try_get::<Option<String>, _>("external_url").ok().flatten(),
                "is_official": row.try_get::<bool, _>("is_official").unwrap_or(false),
                "is_active": row.try_get::<bool, _>("is_active").unwrap_or(false),
                "file_name": row.try_get::<Option<String>, _>("file_name").ok().flatten(),
                "mime_type": row.try_get::<Option<String>, _>("mime_type").ok().flatten(),
                "file_size_bytes": row.try_get::<Option<i64>, _>("file_size_bytes").ok().flatten(),
                "sha256_hex": row.try_get::<Option<String>, _>("sha256_hex").ok().flatten(),
                "version_label": row.try_get::<String, _>("version_label").unwrap_or_else(|_| "v1".to_string()),
                "published_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("published_at").ok(),
                "expires_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at").ok().flatten(),
                "requires_download": row.try_get::<bool, _>("requires_download").unwrap_or(true),
                "upload_status": row.try_get::<String, _>("upload_status").unwrap_or_else(|_| "external".to_string()),
                "retention_policy": row.try_get::<String, _>("retention_policy").unwrap_or_else(|_| "standard".to_string()),
                "retention_until": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("retention_until").ok().flatten(),
                "review_required_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("review_required_at").ok().flatten(),
                "reviewed_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("reviewed_at").ok().flatten(),
                "reviewed_by": row.try_get::<Option<Uuid>, _>("reviewed_by").ok().flatten(),
                "legal_hold": row.try_get::<bool, _>("legal_hold").unwrap_or(false),
                "deleted_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at").ok().flatten(),
                "deleted_by": row.try_get::<Option<Uuid>, _>("deleted_by").ok().flatten(),
                "deletion_reason": row.try_get::<Option<String>, _>("deletion_reason").ok().flatten(),
                "document_lifecycle_notes": row.try_get::<Option<String>, _>("document_lifecycle_notes").ok().flatten(),
                "has_private_file": row.try_get::<bool, _>("has_private_file").unwrap_or(false),
                "current_version_id": row.try_get::<Option<Uuid>, _>("current_version_id").ok().flatten(),
                "version_count": row.try_get::<i64, _>("version_count").unwrap_or(0),
                "delivery_url": resource_id.map(|id| circle_resource_delivery_url(circle_id, id)),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
                "updated_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "circle_id": circle_id,
        "role": role,
        "resources": resources,
        "storage_paths_hidden": true,
    })))
}

async fn create_circle_resource_manage(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<CircleResourceCreateReq>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let title = normalize_resource_required_text(&payload.title, 240, "Resource title")?;
    let description =
        normalize_resource_optional_text(payload.description, 2000, "Resource description")?;
    let resource_type = normalize_circle_resource_type(payload.resource_type)?;
    let access_scope = normalize_circle_resource_access_scope(payload.access_scope)?;
    let (url, storage_object_path) = normalize_circle_resource_source(
        payload.url,
        payload.storage_object_path,
        state.config.gcs_bucket.as_deref(),
    )?;
    let file_name = normalize_resource_optional_text(payload.file_name, 240, "File name")?;
    let mime_type = normalize_resource_optional_text(payload.mime_type, 120, "MIME type")?;
    let file_size_bytes = validate_circle_resource_file_size(payload.file_size_bytes)?;
    let sha256_hex = normalize_circle_resource_sha(payload.sha256_hex)?;
    let version_label =
        normalize_resource_optional_text(payload.version_label, 80, "Version label")?
            .unwrap_or_else(|| "v1".to_string());
    let change_note = normalize_resource_optional_text(payload.change_note, 1000, "Change note")?;
    let requires_download = payload.requires_download.unwrap_or(true);
    let default_upload_status = if storage_object_path.is_some() {
        "uploaded"
    } else {
        "external"
    };
    let upload_status =
        normalize_circle_resource_upload_status(payload.upload_status, default_upload_status)?;
    let retention_policy = normalize_circle_resource_retention_policy(payload.retention_policy)?;
    let document_lifecycle_notes = normalize_resource_optional_text(
        payload.document_lifecycle_notes,
        2000,
        "Document lifecycle notes",
    )?;
    let retention_until = payload.retention_until;
    let review_required_at = payload.review_required_at;

    let mut tx = c_pool.begin().await?;
    let asset_id: Option<Uuid> =
        sqlx::query_scalar("SELECT related_asset_id FROM circles WHERE id = $1 FOR UPDATE")
            .bind(circle_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Circle not found.".into()))?;

    let resource_row = sqlx::query(
        r#"
        INSERT INTO circle_resources (
          circle_id, asset_id, title, description, resource_type, access_scope,
          url, storage_object_path, is_official, is_active, created_by,
          file_name, mime_type, file_size_bytes, sha256_hex, version_label,
          published_at, expires_at, requires_download, upload_status,
          retention_policy, retention_until, review_required_at, legal_hold,
          document_lifecycle_notes
        )
        VALUES (
          $1, $2, $3, $4, $5, $6,
          $7, $8, $9, TRUE, $10,
          $11, $12, $13, $14, $15,
          COALESCE($16, NOW()), $17, $18, $19,
          $20, $21, $22, $23, $24
        )
        RETURNING id, version_label, published_at, expires_at
        "#,
    )
    .bind(circle_id)
    .bind(asset_id)
    .bind(&title)
    .bind(description.as_deref())
    .bind(&resource_type)
    .bind(&access_scope)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(payload.is_official.unwrap_or(false))
    .bind(user.id)
    .bind(file_name.as_deref())
    .bind(mime_type.as_deref())
    .bind(file_size_bytes)
    .bind(sha256_hex.as_deref())
    .bind(&version_label)
    .bind(payload.published_at)
    .bind(payload.expires_at)
    .bind(requires_download)
    .bind(&upload_status)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(retention_policy == "legal_hold")
    .bind(document_lifecycle_notes.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    let resource_id = resource_row.try_get::<Uuid, _>("id").unwrap_or_default();
    let published_at = resource_row
        .try_get::<chrono::DateTime<chrono::Utc>, _>("published_at")
        .ok();
    let expires_at = resource_row
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at")
        .ok()
        .flatten();

    sqlx::query(
        r#"
        INSERT INTO circle_resource_versions (
          resource_id, circle_id, version_label, url, storage_object_path,
          file_name, mime_type, file_size_bytes, sha256_hex, requires_download,
          published_at, expires_at, change_note, upload_status, retention_policy,
          retention_until, review_required_at, document_lifecycle_notes, is_current, created_by
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
          COALESCE($11, NOW()), $12, $13, $14, $15, $16, $17, $18, TRUE, $19
        )
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(&version_label)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(file_name.as_deref())
    .bind(mime_type.as_deref())
    .bind(file_size_bytes)
    .bind(sha256_hex.as_deref())
    .bind(requires_download)
    .bind(published_at)
    .bind(expires_at)
    .bind(change_note.as_deref())
    .bind(&upload_status)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(document_lifecycle_notes.as_deref())
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.create",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "role": role,
            "resource_type": resource_type,
            "access_scope": access_scope,
            "version_label": version_label,
            "has_private_file": storage_object_path.is_some(),
            "upload_status": upload_status,
            "retention_policy": retention_policy,
            "review_required_at": review_required_at,
            "retention_until": retention_until,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
    })))
}

async fn upload_circle_resource_file(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, user.id, "circle_resource_upload").await?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let mut fields = CircleResourceUploadFields::default();
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut client_mime = "application/octet-stream".to_string();
    let mut original_file_name: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart upload body.".into()))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name == "file" {
            if file_bytes.is_some() {
                return Err(AppError::BadRequest(
                    "Only one Circle resource file can be uploaded at a time.".into(),
                ));
            }
            client_mime = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            original_file_name = field.file_name().map(str::to_string);
            let mut field = field;
            file_bytes = Some(
                crate::storage::upload_helpers::read_field_capped(
                    &mut field,
                    MAX_CIRCLE_RESOURCE_UPLOAD_BYTES,
                    "Circle resource file",
                )
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?,
            );
            continue;
        }

        let mut field = field;
        let value_bytes =
            crate::storage::upload_helpers::read_field_capped(&mut field, 4096, &field_name)
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
        let value = String::from_utf8(value_bytes)
            .map_err(|_| AppError::BadRequest("Multipart field must be UTF-8.".into()))?;
        match field_name.as_str() {
            "title" => fields.title = Some(value),
            "description" => fields.description = Some(value),
            "resource_type" => fields.resource_type = Some(value),
            "access_scope" => fields.access_scope = Some(value),
            "is_official" => fields.is_official = parse_circle_resource_upload_bool(&value),
            "file_name" => fields.file_name = Some(value),
            "version_label" => fields.version_label = Some(value),
            "change_note" => fields.change_note = Some(value),
            "retention_policy" => fields.retention_policy = Some(value),
            "retention_until" => {
                fields.retention_until =
                    parse_circle_resource_upload_datetime(Some(value), "Retention until")?
            }
            "review_required_at" => {
                fields.review_required_at =
                    parse_circle_resource_upload_datetime(Some(value), "Review required at")?
            }
            "document_lifecycle_notes" => fields.document_lifecycle_notes = Some(value),
            _ => {}
        }
    }

    let file_bytes = file_bytes
        .ok_or_else(|| AppError::BadRequest("Circle resource file is required.".into()))?;
    if file_bytes.is_empty() {
        return Err(AppError::BadRequest(
            "Circle resource file cannot be empty.".into(),
        ));
    }
    let sniffed = crate::storage::service::sniff_mime(&file_bytes)
        .ok_or_else(|| AppError::BadRequest("Unsupported or unrecognized file format.".into()))?;
    if !crate::storage::service::mime_matches(&client_mime, sniffed) {
        tracing::warn!(
            circle_id = %circle_id,
            user_id = %user.id,
            claimed_mime = %client_mime,
            sniffed_mime = %sniffed,
            "Circle resource upload rejected: MIME mismatch"
        );
        return Err(AppError::BadRequest(
            "File content does not match declared type.".into(),
        ));
    }
    let mime_type = if client_mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        )
        && sniffed == "application/zip"
    {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()
    } else {
        sniffed.to_string()
    };
    crate::storage::service::validate_asset_doc_mime(&mime_type)?;

    let ext = crate::storage::service::extension_for_doc_mime(&mime_type);
    let fallback_title = original_file_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Circle resource file");
    let title = normalize_resource_required_text(
        fields.title.as_deref().unwrap_or(fallback_title),
        240,
        "Resource title",
    )?;
    let description =
        normalize_resource_optional_text(fields.description, 2000, "Resource description")?;
    let resource_type = normalize_circle_resource_type(fields.resource_type)?;
    let access_scope = normalize_circle_resource_access_scope(fields.access_scope)?;
    let file_name = normalize_resource_optional_text(
        fields.file_name.or(original_file_name),
        240,
        "File name",
    )?
    .unwrap_or_else(|| format!("{}.{}", title, ext));
    let version_label =
        normalize_resource_optional_text(fields.version_label, 80, "Version label")?
            .unwrap_or_else(|| "v1".to_string());
    let retention_policy = normalize_circle_resource_retention_policy(fields.retention_policy)?;
    let retention_until = fields.retention_until;
    let review_required_at = fields.review_required_at;
    let document_lifecycle_notes = normalize_resource_optional_text(
        fields.document_lifecycle_notes,
        2000,
        "Document lifecycle notes",
    )?;
    let file_size_bytes = file_bytes.len() as i64;
    let sha256_hex = crate::storage::service::sha256_hex(&file_bytes);
    let file_id = Uuid::new_v4();
    let object_path = format!(
        "community/circles/{}/resources/{}.{}",
        circle_id, file_id, ext
    );

    let (url, storage_object_path) = if let Some(bucket) = state.config.gcs_bucket.as_deref() {
        let upload = crate::storage::service::upload_private_with_markers(
            bucket,
            &object_path,
            file_bytes.clone(),
            &mime_type,
            crate::storage::service::PiiClass::B,
            Some(user.id),
        );
        match tokio::time::timeout(std::time::Duration::from_secs(15), upload).await {
            Ok(Ok(path)) => (None, Some(path)),
            Ok(Err(e)) => {
                tracing::error!(
                    circle_id = %circle_id,
                    error = %e,
                    "Circle resource GCS upload failed; trying local fallback"
                );
                let local_url =
                    crate::storage::service::upload_local(&object_path, file_bytes.clone()).await?;
                (Some(local_url), None)
            }
            Err(_) => {
                tracing::error!(
                    circle_id = %circle_id,
                    "Circle resource GCS upload timed out; trying local fallback"
                );
                let local_url =
                    crate::storage::service::upload_local(&object_path, file_bytes.clone()).await?;
                (Some(local_url), None)
            }
        }
    } else {
        let local_url = crate::storage::service::upload_local(&object_path, file_bytes).await?;
        (Some(local_url), None)
    };

    let mut tx = c_pool.begin().await?;
    let asset_id: Option<Uuid> =
        sqlx::query_scalar("SELECT related_asset_id FROM circles WHERE id = $1 FOR UPDATE")
            .bind(circle_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Circle not found.".into()))?;

    let resource_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO circle_resources (
          circle_id, asset_id, title, description, resource_type, access_scope,
          url, storage_object_path, is_official, is_active, created_by,
          file_name, mime_type, file_size_bytes, sha256_hex, version_label,
          published_at, requires_download, upload_status, retention_policy,
          retention_until, review_required_at, legal_hold, document_lifecycle_notes
        )
        VALUES (
          $1, $2, $3, $4, $5, $6,
          $7, $8, $9, TRUE, $10,
          $11, $12, $13, $14, $15,
          NOW(), TRUE, 'uploaded', $16,
          $17, $18, $19, $20
        )
        RETURNING id
        "#,
    )
    .bind(circle_id)
    .bind(asset_id)
    .bind(&title)
    .bind(description.as_deref())
    .bind(&resource_type)
    .bind(&access_scope)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(fields.is_official)
    .bind(user.id)
    .bind(&file_name)
    .bind(&mime_type)
    .bind(file_size_bytes)
    .bind(&sha256_hex)
    .bind(&version_label)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(retention_policy == "legal_hold")
    .bind(document_lifecycle_notes.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO circle_resource_versions (
          resource_id, circle_id, version_label, url, storage_object_path,
          file_name, mime_type, file_size_bytes, sha256_hex, requires_download,
          published_at, change_note, upload_status, retention_policy,
          retention_until, review_required_at, document_lifecycle_notes, is_current, created_by
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE,
          NOW(), 'Initial binary upload', 'uploaded', $10,
          $11, $12, $13, TRUE, $14
        )
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(&version_label)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(&file_name)
    .bind(&mime_type)
    .bind(file_size_bytes)
    .bind(&sha256_hex)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(document_lifecycle_notes.as_deref())
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.upload",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "role": role,
            "resource_type": resource_type,
            "access_scope": access_scope,
            "mime_type": mime_type,
            "file_size_bytes": file_size_bytes,
            "sha256_hex": sha256_hex,
            "has_private_file": storage_object_path.is_some(),
            "retention_policy": retention_policy,
            "review_required_at": review_required_at,
            "retention_until": retention_until,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
        "delivery_url": circle_resource_delivery_url(circle_id, resource_id),
    })))
}

async fn upload_circle_resource_version_file(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, resource_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, user.id, "circle_resource_upload").await?;

    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let existing_title: Option<String> =
        sqlx::query_scalar("SELECT title FROM circle_resources WHERE id = $1 AND circle_id = $2")
            .bind(resource_id)
            .bind(circle_id)
            .fetch_optional(&c_pool)
            .await?;
    let existing_title =
        existing_title.ok_or_else(|| AppError::NotFound("Circle resource not found.".into()))?;

    let mut fields = CircleResourceUploadFields::default();
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut client_mime = "application/octet-stream".to_string();
    let mut original_file_name: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart upload body.".into()))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name == "file" {
            if file_bytes.is_some() {
                return Err(AppError::BadRequest(
                    "Only one Circle resource file can be uploaded at a time.".into(),
                ));
            }
            client_mime = field
                .content_type()
                .unwrap_or("application/octet-stream")
                .to_string();
            original_file_name = field.file_name().map(str::to_string);
            let mut field = field;
            file_bytes = Some(
                crate::storage::upload_helpers::read_field_capped(
                    &mut field,
                    MAX_CIRCLE_RESOURCE_UPLOAD_BYTES,
                    "Circle resource version file",
                )
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?,
            );
            continue;
        }

        let mut field = field;
        let value_bytes =
            crate::storage::upload_helpers::read_field_capped(&mut field, 4096, &field_name)
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
        let value = String::from_utf8(value_bytes)
            .map_err(|_| AppError::BadRequest("Multipart field must be UTF-8.".into()))?;
        match field_name.as_str() {
            "file_name" => fields.file_name = Some(value),
            "version_label" => fields.version_label = Some(value),
            "change_note" => fields.change_note = Some(value),
            "retention_policy" => fields.retention_policy = Some(value),
            "retention_until" => {
                fields.retention_until =
                    parse_circle_resource_upload_datetime(Some(value), "Retention until")?
            }
            "review_required_at" => {
                fields.review_required_at =
                    parse_circle_resource_upload_datetime(Some(value), "Review required at")?
            }
            "document_lifecycle_notes" => fields.document_lifecycle_notes = Some(value),
            _ => {}
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| {
        AppError::BadRequest("Circle resource replacement file is required.".into())
    })?;
    if file_bytes.is_empty() {
        return Err(AppError::BadRequest(
            "Circle resource replacement file cannot be empty.".into(),
        ));
    }
    let sniffed = crate::storage::service::sniff_mime(&file_bytes)
        .ok_or_else(|| AppError::BadRequest("Unsupported or unrecognized file format.".into()))?;
    if !crate::storage::service::mime_matches(&client_mime, sniffed) {
        tracing::warn!(
            circle_id = %circle_id,
            resource_id = %resource_id,
            user_id = %user.id,
            claimed_mime = %client_mime,
            sniffed_mime = %sniffed,
            "Circle resource replacement upload rejected: MIME mismatch"
        );
        return Err(AppError::BadRequest(
            "File content does not match declared type.".into(),
        ));
    }
    let mime_type = if client_mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        )
        && sniffed == "application/zip"
    {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()
    } else {
        sniffed.to_string()
    };
    crate::storage::service::validate_asset_doc_mime(&mime_type)?;

    let ext = crate::storage::service::extension_for_doc_mime(&mime_type);
    let file_name = normalize_resource_optional_text(
        fields.file_name.or(original_file_name),
        240,
        "File name",
    )?
    .unwrap_or_else(|| format!("{}.{}", existing_title, ext));
    let version_label =
        normalize_resource_optional_text(fields.version_label, 80, "Version label")?
            .unwrap_or_else(|| "replacement".to_string());
    let change_note = normalize_resource_optional_text(fields.change_note, 1000, "Change note")?
        .unwrap_or_else(|| "Binary replacement upload".to_string());
    let retention_policy = normalize_circle_resource_retention_policy(fields.retention_policy)?;
    let retention_until = fields.retention_until;
    let review_required_at = fields.review_required_at;
    let document_lifecycle_notes = normalize_resource_optional_text(
        fields.document_lifecycle_notes,
        2000,
        "Document lifecycle notes",
    )?;
    let file_size_bytes = file_bytes.len() as i64;
    let sha256_hex = crate::storage::service::sha256_hex(&file_bytes);
    let file_id = Uuid::new_v4();
    let object_path = format!(
        "community/circles/{}/resources/{}/versions/{}.{}",
        circle_id, resource_id, file_id, ext
    );

    let (url, storage_object_path) = if let Some(bucket) = state.config.gcs_bucket.as_deref() {
        let upload = crate::storage::service::upload_private_with_markers(
            bucket,
            &object_path,
            file_bytes.clone(),
            &mime_type,
            crate::storage::service::PiiClass::B,
            Some(user.id),
        );
        match tokio::time::timeout(std::time::Duration::from_secs(15), upload).await {
            Ok(Ok(path)) => (None, Some(path)),
            Ok(Err(e)) => {
                tracing::error!(
                    circle_id = %circle_id,
                    resource_id = %resource_id,
                    error = %e,
                    "Circle resource replacement GCS upload failed; trying local fallback"
                );
                let local_url =
                    crate::storage::service::upload_local(&object_path, file_bytes.clone()).await?;
                (Some(local_url), None)
            }
            Err(_) => {
                tracing::error!(
                    circle_id = %circle_id,
                    resource_id = %resource_id,
                    "Circle resource replacement GCS upload timed out; trying local fallback"
                );
                let local_url =
                    crate::storage::service::upload_local(&object_path, file_bytes.clone()).await?;
                (Some(local_url), None)
            }
        }
    } else {
        let local_url = crate::storage::service::upload_local(&object_path, file_bytes).await?;
        (Some(local_url), None)
    };

    let mut tx = c_pool.begin().await?;
    let resource_exists: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM circle_resources WHERE id = $1 AND circle_id = $2 FOR UPDATE",
    )
    .bind(resource_id)
    .bind(circle_id)
    .fetch_optional(&mut *tx)
    .await?;
    if resource_exists.is_none() {
        return Err(AppError::NotFound("Circle resource not found.".into()));
    }

    sqlx::query("UPDATE circle_resource_versions SET is_current = FALSE WHERE resource_id = $1")
        .bind(resource_id)
        .execute(&mut *tx)
        .await?;

    let version_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO circle_resource_versions (
          resource_id, circle_id, version_label, url, storage_object_path,
          file_name, mime_type, file_size_bytes, sha256_hex, requires_download,
          published_at, change_note, upload_status, retention_policy,
          retention_until, review_required_at, document_lifecycle_notes, is_current, created_by
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9, TRUE,
          NOW(), $10, 'uploaded', $11, $12, $13, $14, TRUE, $15
        )
        RETURNING id
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(&version_label)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(&file_name)
    .bind(&mime_type)
    .bind(file_size_bytes)
    .bind(&sha256_hex)
    .bind(&change_note)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(document_lifecycle_notes.as_deref())
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE circle_resources
           SET url = $3,
               storage_object_path = $4,
               file_name = $5,
               mime_type = $6,
               file_size_bytes = $7,
               sha256_hex = $8,
               version_label = $9,
               published_at = NOW(),
               expires_at = NULL,
               requires_download = TRUE,
               upload_status = 'uploaded',
               retention_policy = $10,
               retention_until = $11,
               review_required_at = $12,
               legal_hold = $13,
               reviewed_at = NULL,
               reviewed_by = NULL,
               document_lifecycle_notes = COALESCE($14, document_lifecycle_notes),
               is_active = TRUE,
               deleted_at = NULL,
               deleted_by = NULL,
               deletion_reason = NULL,
               storage_deleted_at = NULL,
               storage_delete_attempts = 0,
               storage_delete_last_error = NULL,
               storage_delete_next_attempt_at = NULL,
               updated_at = NOW()
         WHERE id = $1
           AND circle_id = $2
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(&file_name)
    .bind(&mime_type)
    .bind(file_size_bytes)
    .bind(&sha256_hex)
    .bind(&version_label)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(retention_policy == "legal_hold")
    .bind(document_lifecycle_notes.as_deref())
    .execute(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.version.upload",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "version_id": version_id,
            "role": role,
            "version_label": version_label,
            "mime_type": mime_type,
            "file_size_bytes": file_size_bytes,
            "sha256_hex": sha256_hex,
            "has_private_file": storage_object_path.is_some(),
            "retention_policy": retention_policy,
            "review_required_at": review_required_at,
            "retention_until": retention_until,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
        "version_id": version_id,
        "delivery_url": circle_resource_delivery_url(circle_id, resource_id),
    })))
}

async fn update_circle_resource_manage(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, resource_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CircleResourceUpdateReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let title = match payload.title {
        Some(value) => Some(normalize_resource_required_text(
            &value,
            240,
            "Resource title",
        )?),
        None => None,
    };
    let description =
        normalize_resource_optional_text(payload.description, 2000, "Resource description")?;
    let resource_type = match payload.resource_type {
        Some(value) => Some(normalize_circle_resource_type(Some(value))?),
        None => None,
    };
    let access_scope = match payload.access_scope {
        Some(value) => Some(normalize_circle_resource_access_scope(Some(value))?),
        None => None,
    };
    let upload_status = match payload.upload_status {
        Some(value) => Some(normalize_circle_resource_upload_status(
            Some(value),
            "external",
        )?),
        None => None,
    };
    let retention_policy = match payload.retention_policy {
        Some(value) => Some(normalize_circle_resource_retention_policy(Some(value))?),
        None => None,
    };
    let lifecycle_retention_until = payload.retention_until;
    let lifecycle_review_required_at = payload.review_required_at;
    let document_lifecycle_notes = normalize_resource_optional_text(
        payload.document_lifecycle_notes,
        2000,
        "Document lifecycle notes",
    )?;

    if title.is_none()
        && description.is_none()
        && resource_type.is_none()
        && access_scope.is_none()
        && payload.is_official.is_none()
        && payload.is_active.is_none()
        && payload.expires_at.is_none()
        && upload_status.is_none()
        && retention_policy.is_none()
        && lifecycle_retention_until.is_none()
        && lifecycle_review_required_at.is_none()
        && document_lifecycle_notes.is_none()
    {
        return Err(AppError::BadRequest(
            "At least one resource field must be provided.".into(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let updated_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE circle_resources
           SET title = COALESCE($3, title),
               description = COALESCE($4, description),
               resource_type = COALESCE($5, resource_type),
               access_scope = COALESCE($6, access_scope),
               is_official = COALESCE($7, is_official),
               is_active = COALESCE($8, is_active),
               expires_at = COALESCE($9, expires_at),
               upload_status = COALESCE($10, upload_status),
               retention_policy = COALESCE($11, retention_policy),
               retention_until = COALESCE($12, retention_until),
               review_required_at = COALESCE($13, review_required_at),
               legal_hold = CASE
                 WHEN $11 = 'legal_hold' THEN TRUE
                 WHEN $11 IN ('standard', 'delete_after_expiry') THEN FALSE
                 ELSE legal_hold
               END,
               document_lifecycle_notes = COALESCE($14, document_lifecycle_notes),
               updated_at = NOW()
         WHERE id = $1
           AND circle_id = $2
         RETURNING id
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(title.as_deref())
    .bind(description.as_deref())
    .bind(resource_type.as_deref())
    .bind(access_scope.as_deref())
    .bind(payload.is_official)
    .bind(payload.is_active)
    .bind(payload.expires_at)
    .bind(upload_status.as_deref())
    .bind(retention_policy.as_deref())
    .bind(lifecycle_retention_until)
    .bind(lifecycle_review_required_at)
    .bind(document_lifecycle_notes.as_deref())
    .fetch_optional(&mut *tx)
    .await?;

    if updated_id.is_none() {
        return Err(AppError::NotFound("Circle resource not found.".into()));
    }

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.update",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "role": role,
            "changed": {
                "title": title.is_some(),
                "description": description.is_some(),
                "resource_type": resource_type,
                "access_scope": access_scope,
                "is_official": payload.is_official,
                "is_active": payload.is_active,
                "expires_at": payload.expires_at,
                "upload_status": upload_status,
                "retention_policy": retention_policy,
                "retention_until": lifecycle_retention_until,
                "review_required_at": lifecycle_review_required_at,
                "document_lifecycle_notes": document_lifecycle_notes.is_some(),
            }
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn update_circle_resource_lifecycle(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, resource_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CircleResourceLifecycleReq>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let action = normalize_circle_resource_lifecycle_action(&payload.action)?;
    let note = normalize_resource_optional_text(payload.note, 2000, "Lifecycle note")?;
    let retention_policy = match payload.retention_policy {
        Some(value) => Some(normalize_circle_resource_retention_policy(Some(value))?),
        None => None,
    };
    let lifecycle_retention_until = payload.retention_until;
    let lifecycle_review_required_at = payload.review_required_at;

    let mut tx = c_pool.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT id,
               storage_object_path IS NOT NULL AS has_private_file
          FROM circle_resources
         WHERE id = $1
           AND circle_id = $2
         FOR UPDATE
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle resource not found.".into()))?;

    let has_private_file = row.try_get::<bool, _>("has_private_file").unwrap_or(false);
    let restored_upload_status = if has_private_file {
        "uploaded"
    } else {
        "external"
    };

    match action.as_str() {
        "mark_reviewed" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET reviewed_at = NOW(),
                       reviewed_by = $3,
                       document_lifecycle_notes = COALESCE($4, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(user.id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "clear_review" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET reviewed_at = NULL,
                       reviewed_by = NULL,
                       document_lifecycle_notes = COALESCE($3, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "mark_uploaded" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET upload_status = 'uploaded',
                       deleted_at = NULL,
                       deleted_by = NULL,
                       deletion_reason = NULL,
                       is_active = TRUE,
                       document_lifecycle_notes = COALESCE($3, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "mark_pending_upload" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET upload_status = 'pending_upload',
                       document_lifecycle_notes = COALESCE($3, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "reject_upload" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET upload_status = 'rejected',
                       is_active = FALSE,
                       document_lifecycle_notes = COALESCE($3, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "expire" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET upload_status = 'expired',
                       is_active = FALSE,
                       expires_at = COALESCE(expires_at, NOW()),
                       document_lifecycle_notes = COALESCE($3, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "soft_delete" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET upload_status = 'deleted',
                       is_active = FALSE,
                       deleted_at = NOW(),
                       deleted_by = $3,
                       deletion_reason = COALESCE($4, deletion_reason),
                       document_lifecycle_notes = COALESCE($4, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(user.id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "restore" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET upload_status = $3,
                       is_active = TRUE,
                       deleted_at = NULL,
                       deleted_by = NULL,
                       deletion_reason = NULL,
                       document_lifecycle_notes = COALESCE($4, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(restored_upload_status)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "legal_hold" => {
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET retention_policy = 'legal_hold',
                       legal_hold = TRUE,
                       document_lifecycle_notes = COALESCE($3, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "standard_retention" => {
            let policy = retention_policy
                .clone()
                .unwrap_or_else(|| "standard".to_string());
            if policy == "legal_hold" {
                return Err(AppError::BadRequest(
                    "Use the legal_hold lifecycle action to place a document on legal hold.".into(),
                ));
            }
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET retention_policy = $3,
                       retention_until = COALESCE($4, retention_until),
                       legal_hold = FALSE,
                       document_lifecycle_notes = COALESCE($5, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(policy)
            .bind(lifecycle_retention_until)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        "schedule_review" => {
            let review_required_at = lifecycle_review_required_at.ok_or_else(|| {
                AppError::BadRequest("review_required_at is required for schedule_review.".into())
            })?;
            sqlx::query(
                r#"
                UPDATE circle_resources
                   SET review_required_at = $3,
                       reviewed_at = NULL,
                       reviewed_by = NULL,
                       document_lifecycle_notes = COALESCE($4, document_lifecycle_notes),
                       updated_at = NOW()
                 WHERE id = $1
                   AND circle_id = $2
                "#,
            )
            .bind(resource_id)
            .bind(circle_id)
            .bind(review_required_at)
            .bind(note.as_deref())
            .execute(&mut *tx)
            .await?;
        }
        _ => unreachable!("validated lifecycle action"),
    }

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.lifecycle",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "role": role,
            "action": action,
            "has_note": note.is_some(),
            "retention_policy": retention_policy,
            "retention_until": lifecycle_retention_until,
            "review_required_at": lifecycle_review_required_at,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
        "action": action,
    })))
}

fn circle_resource_version_field_label(field: &str) -> &'static str {
    match field {
        "version_label" => "Version label",
        "file_name" => "File name",
        "mime_type" => "MIME type",
        "file_size_bytes" => "File size",
        "sha256_hex" => "SHA-256",
        "requires_download" => "Requires download",
        "published_at" => "Published at",
        "expires_at" => "Expires at",
        "upload_status" => "Upload status",
        "retention_policy" => "Retention policy",
        "retention_until" => "Retention until",
        "review_required_at" => "Review required at",
        "review_status" => "Review status",
        _ => "Field",
    }
}

fn circle_resource_version_comparison(versions: &[serde_json::Value]) -> Option<serde_json::Value> {
    let current = versions.iter().find(|version| {
        version
            .get("is_current")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    })?;
    let candidate = versions.iter().find(|version| {
        !version
            .get("is_current")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    })?;
    let fields = [
        "version_label",
        "file_name",
        "mime_type",
        "file_size_bytes",
        "sha256_hex",
        "requires_download",
        "published_at",
        "expires_at",
        "upload_status",
        "retention_policy",
        "retention_until",
        "review_required_at",
        "review_status",
    ];
    let changed_fields: Vec<serde_json::Value> = fields
        .iter()
        .filter_map(|field| {
            let current_value = current
                .get(field)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let candidate_value = candidate
                .get(field)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if current_value == candidate_value {
                None
            } else {
                Some(serde_json::json!({
                    "field": field,
                    "label": circle_resource_version_field_label(field),
                    "current": current_value,
                    "candidate": candidate_value,
                }))
            }
        })
        .collect();

    Some(serde_json::json!({
        "current_version_id": current.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "candidate_version_id": candidate.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "candidate_label": candidate.get("version_label").cloned().unwrap_or(serde_json::Value::Null),
        "change_count": changed_fields.len(),
        "changed_fields": changed_fields,
    }))
}

async fn get_circle_resource_versions(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((circle_id, resource_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM circle_resources WHERE id = $1 AND circle_id = $2)",
    )
    .bind(resource_id)
    .bind(circle_id)
    .fetch_one(&c_pool)
    .await?;
    if !exists {
        return Err(AppError::NotFound("Circle resource not found.".into()));
    }

    let rows = sqlx::query(
        r#"
        SELECT id,
               resource_id,
               circle_id,
               version_label,
               CASE WHEN storage_object_path IS NULL THEN url ELSE NULL END AS external_url,
               storage_object_path IS NOT NULL AS has_private_file,
               file_name,
               mime_type,
               file_size_bytes,
               sha256_hex,
               requires_download,
               published_at,
               expires_at,
               change_note,
               upload_status,
               retention_policy,
               retention_until,
               review_required_at,
               review_status,
               reviewed_at,
               reviewed_by,
               review_note,
               document_lifecycle_notes,
               is_current,
               created_by,
               created_at
          FROM circle_resource_versions
         WHERE resource_id = $1
           AND circle_id = $2
         ORDER BY is_current DESC, created_at DESC
         LIMIT 100
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .fetch_all(&c_pool)
    .await?;

    let versions: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "resource_id": row.try_get::<Uuid, _>("resource_id").ok(),
                "circle_id": row.try_get::<Uuid, _>("circle_id").ok(),
                "version_label": row.try_get::<String, _>("version_label").unwrap_or_else(|_| "v1".to_string()),
                "external_url": row.try_get::<Option<String>, _>("external_url").ok().flatten(),
                "has_private_file": row.try_get::<bool, _>("has_private_file").unwrap_or(false),
                "file_name": row.try_get::<Option<String>, _>("file_name").ok().flatten(),
                "mime_type": row.try_get::<Option<String>, _>("mime_type").ok().flatten(),
                "file_size_bytes": row.try_get::<Option<i64>, _>("file_size_bytes").ok().flatten(),
                "sha256_hex": row.try_get::<Option<String>, _>("sha256_hex").ok().flatten(),
                "requires_download": row.try_get::<bool, _>("requires_download").unwrap_or(true),
                "published_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("published_at").ok(),
                "expires_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at").ok().flatten(),
                "change_note": row.try_get::<Option<String>, _>("change_note").ok().flatten(),
                "upload_status": row.try_get::<String, _>("upload_status").unwrap_or_else(|_| "external".to_string()),
                "retention_policy": row.try_get::<String, _>("retention_policy").unwrap_or_else(|_| "standard".to_string()),
                "retention_until": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("retention_until").ok().flatten(),
                "review_required_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("review_required_at").ok().flatten(),
                "review_status": row.try_get::<String, _>("review_status").unwrap_or_else(|_| "pending".to_string()),
                "reviewed_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("reviewed_at").ok().flatten(),
                "reviewed_by": row.try_get::<Option<Uuid>, _>("reviewed_by").ok().flatten(),
                "review_note": row.try_get::<Option<String>, _>("review_note").ok().flatten(),
                "document_lifecycle_notes": row.try_get::<Option<String>, _>("document_lifecycle_notes").ok().flatten(),
                "is_current": row.try_get::<bool, _>("is_current").unwrap_or(false),
                "created_by": row.try_get::<Option<Uuid>, _>("created_by").ok().flatten(),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
                "delivery_url": row.try_get::<Uuid, _>("id").ok().map(|version_id| circle_resource_version_delivery_url(circle_id, resource_id, version_id)),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "circle_id": circle_id,
        "resource_id": resource_id,
        "role": role,
        "comparison": circle_resource_version_comparison(&versions),
        "versions": versions,
        "storage_paths_hidden": true,
    })))
}

async fn create_circle_resource_version(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, resource_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CircleResourceVersionReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let version_label =
        normalize_resource_optional_text(payload.version_label, 80, "Version label")?
            .unwrap_or_else(|| "v1".to_string());
    let (url, storage_object_path) = normalize_circle_resource_source(
        payload.url,
        payload.storage_object_path,
        state.config.gcs_bucket.as_deref(),
    )?;
    let file_name = normalize_resource_optional_text(payload.file_name, 240, "File name")?;
    let mime_type = normalize_resource_optional_text(payload.mime_type, 120, "MIME type")?;
    let file_size_bytes = validate_circle_resource_file_size(payload.file_size_bytes)?;
    let sha256_hex = normalize_circle_resource_sha(payload.sha256_hex)?;
    let change_note = normalize_resource_optional_text(payload.change_note, 1000, "Change note")?;
    let requires_download = payload.requires_download.unwrap_or(true);
    let default_upload_status = if storage_object_path.is_some() {
        "uploaded"
    } else {
        "external"
    };
    let upload_status =
        normalize_circle_resource_upload_status(payload.upload_status, default_upload_status)?;
    let retention_policy = normalize_circle_resource_retention_policy(payload.retention_policy)?;
    let document_lifecycle_notes = normalize_resource_optional_text(
        payload.document_lifecycle_notes,
        2000,
        "Document lifecycle notes",
    )?;
    let retention_until = payload.retention_until;
    let review_required_at = payload.review_required_at;

    let mut tx = c_pool.begin().await?;
    let resource_exists: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM circle_resources WHERE id = $1 AND circle_id = $2 FOR UPDATE",
    )
    .bind(resource_id)
    .bind(circle_id)
    .fetch_optional(&mut *tx)
    .await?;
    if resource_exists.is_none() {
        return Err(AppError::NotFound("Circle resource not found.".into()));
    }

    sqlx::query("UPDATE circle_resource_versions SET is_current = FALSE WHERE resource_id = $1")
        .bind(resource_id)
        .execute(&mut *tx)
        .await?;

    let version_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO circle_resource_versions (
          resource_id, circle_id, version_label, url, storage_object_path,
          file_name, mime_type, file_size_bytes, sha256_hex, requires_download,
          published_at, expires_at, change_note, upload_status, retention_policy,
          retention_until, review_required_at, document_lifecycle_notes, is_current, created_by
        )
        VALUES (
          $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
          COALESCE($11, NOW()), $12, $13, $14, $15, $16, $17, $18, TRUE, $19
        )
        RETURNING id
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(&version_label)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(file_name.as_deref())
    .bind(mime_type.as_deref())
    .bind(file_size_bytes)
    .bind(sha256_hex.as_deref())
    .bind(requires_download)
    .bind(payload.published_at)
    .bind(payload.expires_at)
    .bind(change_note.as_deref())
    .bind(&upload_status)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(document_lifecycle_notes.as_deref())
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE circle_resources
           SET url = $3,
               storage_object_path = $4,
               file_name = $5,
               mime_type = $6,
               file_size_bytes = $7,
               sha256_hex = $8,
               version_label = $9,
               published_at = COALESCE($10, NOW()),
               expires_at = $11,
               requires_download = $12,
               upload_status = $13,
               retention_policy = $14,
               retention_until = $15,
               review_required_at = $16,
               legal_hold = $17,
               reviewed_at = NULL,
               reviewed_by = NULL,
               document_lifecycle_notes = COALESCE($18, document_lifecycle_notes),
               updated_at = NOW()
         WHERE id = $1
           AND circle_id = $2
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(url.as_deref())
    .bind(storage_object_path.as_deref())
    .bind(file_name.as_deref())
    .bind(mime_type.as_deref())
    .bind(file_size_bytes)
    .bind(sha256_hex.as_deref())
    .bind(&version_label)
    .bind(payload.published_at)
    .bind(payload.expires_at)
    .bind(requires_download)
    .bind(&upload_status)
    .bind(&retention_policy)
    .bind(retention_until)
    .bind(review_required_at)
    .bind(retention_policy == "legal_hold")
    .bind(document_lifecycle_notes.as_deref())
    .execute(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.version.create",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "version_id": version_id,
            "role": role,
            "version_label": version_label,
            "has_private_file": storage_object_path.is_some(),
            "has_change_note": change_note.is_some(),
            "upload_status": upload_status,
            "retention_policy": retention_policy,
            "review_required_at": review_required_at,
            "retention_until": retention_until,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
        "version_id": version_id,
    })))
}

async fn get_circle_resource_version_access(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((circle_id, resource_id, version_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Response, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT version.id,
               resource.title,
               version.url,
               version.storage_object_path,
               version.file_name,
               version.mime_type,
               version.requires_download
          FROM circle_resource_versions version
          JOIN circle_resources resource ON resource.id = version.resource_id
         WHERE version.id = $1
           AND version.resource_id = $2
           AND version.circle_id = $3
           AND resource.circle_id = $3
           AND (version.storage_object_path IS NULL OR version.storage_deleted_at IS NULL)
         LIMIT 1
        "#,
    )
    .bind(version_id)
    .bind(resource_id)
    .bind(circle_id)
    .fetch_optional(&c_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle resource version not found.".into()))?;

    if let Some(url) = row.try_get::<Option<String>, _>("url").ok().flatten() {
        let url = url.trim().to_string();
        if !is_safe_circle_resource_url(&url) {
            return Err(AppError::Internal(
                "Circle resource version has an unsafe delivery URL.".into(),
            ));
        }
        return Ok(Redirect::temporary(&url).into_response());
    }

    let storage_path = row
        .try_get::<Option<String>, _>("storage_object_path")
        .ok()
        .flatten()
        .ok_or_else(|| AppError::NotFound("Circle resource version file not found.".into()))?;
    let (bucket, object_path) =
        parse_circle_resource_storage_path(&storage_path, state.config.gcs_bucket.as_deref())?;

    match crate::storage::service::download_object(&bucket, &object_path).await {
        Ok((downloaded_content_type, data)) => {
            let title = row.try_get::<String, _>("title").unwrap_or_default();
            let file_name = row.try_get::<Option<String>, _>("file_name").ok().flatten();
            let filename =
                safe_circle_resource_filename(&title, file_name.as_deref(), &object_path);
            let content_type = row
                .try_get::<Option<String>, _>("mime_type")
                .ok()
                .flatten()
                .unwrap_or(downloaded_content_type);
            let requires_download = row.try_get::<bool, _>("requires_download").unwrap_or(true);
            let mut headers = HeaderMap::new();
            if let Ok(v) = content_type.parse() {
                headers.insert(header::CONTENT_TYPE, v);
            }
            headers.insert(
                header::CACHE_CONTROL,
                "private, max-age=0, no-store".parse().unwrap(),
            );
            headers.insert(
                header::HeaderName::from_static("x-content-type-options"),
                "nosniff".parse().unwrap(),
            );
            if requires_download || !content_type.starts_with("image/") {
                if let Ok(v) = format!("attachment; filename=\"{}\"", filename).parse() {
                    headers.insert(header::CONTENT_DISPOSITION, v);
                }
            }
            Ok((StatusCode::OK, headers, data).into_response())
        }
        Err(e) => {
            tracing::error!(
                resource_id = %resource_id,
                version_id = %version_id,
                circle_id = %circle_id,
                error = %e,
                "Circle resource version delivery failed"
            );
            Err(AppError::Internal(
                "Failed to fetch Circle resource version file.".into(),
            ))
        }
    }
}

async fn restore_circle_resource_version(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, resource_id, version_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let mut tx = c_pool.begin().await?;
    let version = sqlx::query(
        r#"
        SELECT version.id,
               version.version_label,
               version.url,
               version.storage_object_path,
               version.file_name,
               version.mime_type,
               version.file_size_bytes,
               version.sha256_hex,
               version.requires_download,
               version.published_at,
               version.expires_at,
               version.change_note,
               version.upload_status,
               version.retention_policy,
               version.retention_until,
               version.review_required_at,
               version.document_lifecycle_notes,
               version.storage_deleted_at,
               version.storage_object_path IS NOT NULL AS has_private_file
          FROM circle_resource_versions version
          JOIN circle_resources resource ON resource.id = version.resource_id
         WHERE version.id = $1
           AND version.resource_id = $2
           AND version.circle_id = $3
           AND resource.circle_id = $3
         FOR UPDATE OF version, resource
        "#,
    )
    .bind(version_id)
    .bind(resource_id)
    .bind(circle_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle resource version not found.".into()))?;

    let restored_url = version.try_get::<Option<String>, _>("url").ok().flatten();
    let restored_storage_object_path = version
        .try_get::<Option<String>, _>("storage_object_path")
        .ok()
        .flatten();
    let restored_storage_deleted_at = version
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("storage_deleted_at")
        .ok()
        .flatten();
    if restored_storage_object_path.is_some() && restored_storage_deleted_at.is_some() {
        return Err(AppError::BadRequest(
            "This Circle resource version's backing file has already been physically deleted."
                .into(),
        ));
    }
    let restored_file_name = version
        .try_get::<Option<String>, _>("file_name")
        .ok()
        .flatten();
    let restored_mime_type = version
        .try_get::<Option<String>, _>("mime_type")
        .ok()
        .flatten();
    let restored_file_size_bytes = version
        .try_get::<Option<i64>, _>("file_size_bytes")
        .ok()
        .flatten();
    let restored_sha256_hex = version
        .try_get::<Option<String>, _>("sha256_hex")
        .ok()
        .flatten();
    let version_label = version
        .try_get::<String, _>("version_label")
        .unwrap_or_else(|_| "v1".to_string());
    let restored_published_at = version
        .try_get::<chrono::DateTime<chrono::Utc>, _>("published_at")
        .unwrap_or_else(|_| chrono::Utc::now());
    let restored_expires_at = version
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at")
        .ok()
        .flatten();
    let restored_requires_download = version
        .try_get::<bool, _>("requires_download")
        .unwrap_or(true);
    let upload_status = version
        .try_get::<String, _>("upload_status")
        .unwrap_or_else(|_| "external".to_string());
    let retention_policy = version
        .try_get::<String, _>("retention_policy")
        .unwrap_or_else(|_| "standard".to_string());
    let restored_retention_until = version
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("retention_until")
        .ok()
        .flatten();
    let restored_review_required_at = version
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("review_required_at")
        .ok()
        .flatten();
    let restored_lifecycle_notes = version
        .try_get::<Option<String>, _>("document_lifecycle_notes")
        .ok()
        .flatten();
    let has_private_file = version
        .try_get::<bool, _>("has_private_file")
        .unwrap_or(false);

    sqlx::query("UPDATE circle_resource_versions SET is_current = FALSE WHERE resource_id = $1")
        .bind(resource_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE circle_resource_versions SET is_current = TRUE WHERE id = $1")
        .bind(version_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        UPDATE circle_resources
           SET url = $3,
               storage_object_path = $4,
               file_name = $5,
               mime_type = $6,
               file_size_bytes = $7,
               sha256_hex = $8,
               version_label = $9,
               published_at = $10,
               expires_at = $11,
               requires_download = $12,
               upload_status = $13,
               retention_policy = $14,
               retention_until = $15,
               review_required_at = $16,
               legal_hold = $17,
               document_lifecycle_notes = COALESCE($18, document_lifecycle_notes),
               is_active = TRUE,
               deleted_at = NULL,
               deleted_by = NULL,
               deletion_reason = NULL,
               storage_deleted_at = NULL,
               storage_delete_attempts = 0,
               storage_delete_last_error = NULL,
               storage_delete_next_attempt_at = NULL,
               updated_at = NOW()
         WHERE id = $1
           AND circle_id = $2
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(restored_url.as_deref())
    .bind(restored_storage_object_path.as_deref())
    .bind(restored_file_name.as_deref())
    .bind(restored_mime_type.as_deref())
    .bind(restored_file_size_bytes)
    .bind(restored_sha256_hex.as_deref())
    .bind(&version_label)
    .bind(restored_published_at)
    .bind(restored_expires_at)
    .bind(restored_requires_download)
    .bind(&upload_status)
    .bind(&retention_policy)
    .bind(restored_retention_until)
    .bind(restored_review_required_at)
    .bind(retention_policy == "legal_hold")
    .bind(restored_lifecycle_notes.as_deref())
    .execute(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.version.restore",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "version_id": version_id,
            "role": role,
            "version_label": version_label,
            "upload_status": upload_status,
            "has_private_file": has_private_file,
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
        "version_id": version_id,
        "delivery_url": circle_resource_delivery_url(circle_id, resource_id),
    })))
}

async fn review_circle_resource_version(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, resource_id, version_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(payload): Json<CircleResourceVersionReviewReq>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::Row;
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let role = ensure_circle_resource_admin_access(&state, &c_pool, circle_id, user.id).await?;

    let action = normalize_circle_resource_version_review_action(&payload.action)?;
    let review_status = match action.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        "mark_pending" => "pending",
        _ => "pending",
    };
    let note = normalize_resource_optional_text(payload.note, 2000, "Review note")?;
    if action == "reject" && note.is_none() {
        return Err(AppError::BadRequest(
            "Review note is required when rejecting a resource version.".into(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let version = sqlx::query(
        r#"
        SELECT version.id,
               version.version_label,
               version.storage_object_path,
               version.storage_deleted_at,
               version.is_current,
               version.upload_status
          FROM circle_resource_versions version
          JOIN circle_resources resource ON resource.id = version.resource_id
         WHERE version.id = $1
           AND version.resource_id = $2
           AND version.circle_id = $3
           AND resource.circle_id = $3
         FOR UPDATE OF version, resource
        "#,
    )
    .bind(version_id)
    .bind(resource_id)
    .bind(circle_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle resource version not found.".into()))?;

    let has_deleted_private_file = version
        .try_get::<Option<String>, _>("storage_object_path")
        .ok()
        .flatten()
        .is_some()
        && version
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("storage_deleted_at")
            .ok()
            .flatten()
            .is_some();
    if review_status == "approved" && has_deleted_private_file {
        return Err(AppError::BadRequest(
            "This Circle resource version's backing file has already been physically deleted."
                .into(),
        ));
    }

    sqlx::query(
        r#"
        UPDATE circle_resource_versions
           SET review_status = $2,
               reviewed_at = CASE WHEN $2 = 'pending' THEN NULL ELSE NOW() END,
               reviewed_by = CASE WHEN $2 = 'pending' THEN NULL ELSE $3 END,
               review_note = $4
         WHERE id = $1
        "#,
    )
    .bind(version_id)
    .bind(review_status)
    .bind(user.id)
    .bind(note.as_deref())
    .execute(&mut *tx)
    .await?;

    let is_current = version.try_get::<bool, _>("is_current").unwrap_or(false);
    let upload_status = version
        .try_get::<String, _>("upload_status")
        .unwrap_or_else(|_| "external".to_string());
    if is_current {
        sqlx::query(
            r#"
            UPDATE circle_resources
               SET reviewed_at = CASE WHEN $3 = 'pending' THEN NULL ELSE NOW() END,
                   reviewed_by = CASE WHEN $3 = 'pending' THEN NULL ELSE $4 END,
                   review_required_at = CASE WHEN $3 = 'approved' THEN NULL ELSE review_required_at END,
                   upload_status = CASE
                     WHEN $3 = 'rejected' THEN 'rejected'
                     WHEN $3 = 'approved' AND upload_status = 'rejected' THEN $6
                     ELSE upload_status
                   END,
                   document_lifecycle_notes = COALESCE($5, document_lifecycle_notes),
                   updated_at = NOW()
             WHERE id = $1
               AND circle_id = $2
            "#,
        )
        .bind(resource_id)
        .bind(circle_id)
        .bind(review_status)
        .bind(user.id)
        .bind(note.as_deref())
        .bind(&upload_status)
        .execute(&mut *tx)
        .await?;
    }

    let version_label = version
        .try_get::<String, _>("version_label")
        .unwrap_or_else(|_| "v1".to_string());

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "circle.resource.version.review",
        "circle_resource",
        Some(resource_id),
        None,
        serde_json::json!({
            "circle_id": circle_id,
            "version_id": version_id,
            "role": role,
            "action": action,
            "review_status": review_status,
            "version_label": version_label,
            "upload_status": upload_status,
            "is_current": is_current,
            "has_review_note": note.is_some(),
        }),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "resource_id": resource_id,
        "version_id": version_id,
        "review_status": review_status,
    })))
}

async fn get_circle_resources(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let (circle_asset_id, has_holding, is_member) =
        ensure_asset_circle_access(&state, &c_pool, circle_id, user.id).await?;
    let is_platform_admin =
        middleware::has_permission(&state.db, user.id, "community.manage").await;

    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT id,
               circle_id,
               asset_id,
               title,
               description,
               resource_type,
               access_scope,
               is_official,
               file_name,
               mime_type,
               file_size_bytes,
               sha256_hex,
               version_label,
               published_at,
               expires_at,
               storage_object_path IS NOT NULL AS has_private_file,
               created_at
        FROM circle_resources
        WHERE circle_id = $1
          AND is_active = TRUE
          AND (expires_at IS NULL OR expires_at > NOW())
          AND (
                access_scope = 'public'
             OR (access_scope = 'member' AND $2)
             OR (access_scope = 'holder_only' AND $3)
             OR (access_scope = 'admin_only' AND $4)
          )
        ORDER BY is_official DESC, created_at DESC
        LIMIT 100
        "#,
    )
    .bind(circle_id)
    .bind(is_member || has_holding || is_platform_admin)
    .bind(has_holding || is_platform_admin)
    .bind(is_platform_admin)
    .fetch_all(&c_pool)
    .await?;

    let resources: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let access_scope = row
                .try_get::<String, _>("access_scope")
                .unwrap_or_else(|_| "member".to_string());
            let resource_id = row.try_get::<Uuid, _>("id").ok();
            let has_private_file = row
                .try_get::<bool, _>("has_private_file")
                .unwrap_or(false);
            serde_json::json!({
                "id": resource_id,
                "circle_id": row.try_get::<Uuid, _>("circle_id").ok(),
                "asset_id": row.try_get::<Option<Uuid>, _>("asset_id").ok().flatten().or(circle_asset_id),
                "title": row.try_get::<String, _>("title").unwrap_or_default(),
                "description": row.try_get::<Option<String>, _>("description").ok().flatten(),
                "resource_type": row.try_get::<String, _>("resource_type").unwrap_or_else(|_| "resource".to_string()),
                "access_scope": access_scope,
                "is_official": row.try_get::<bool, _>("is_official").unwrap_or(false),
                "file_name": row.try_get::<Option<String>, _>("file_name").ok().flatten(),
                "mime_type": row.try_get::<Option<String>, _>("mime_type").ok().flatten(),
                "file_size_bytes": row.try_get::<Option<i64>, _>("file_size_bytes").ok().flatten(),
                "sha256_hex": row.try_get::<Option<String>, _>("sha256_hex").ok().flatten(),
                "version_label": row.try_get::<String, _>("version_label").unwrap_or_else(|_| "v1".to_string()),
                "published_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("published_at").ok(),
                "expires_at": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at").ok().flatten(),
                "has_private_file": has_private_file,
                "delivery_mode": if has_private_file { "api_stream" } else { "api_redirect" },
                "delivery_url": resource_id.map(|id| circle_resource_delivery_url(circle_id, id)),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "resources": resources,
        "scope": "circle",
        "circle_id": circle_id,
        "asset_id": circle_asset_id,
        "has_holding": has_holding,
        "is_member": is_member,
    })))
}

async fn get_circle_resource_access(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((circle_id, resource_id)): Path<(Uuid, Uuid)>,
) -> Result<Response, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let (_circle_asset_id, has_holding, is_member) =
        ensure_asset_circle_access(&state, &c_pool, circle_id, user.id).await?;
    let is_platform_admin =
        middleware::has_permission(&state.db, user.id, "community.manage").await;

    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT id,
               title,
               access_scope,
               url,
               storage_object_path,
               file_name,
               mime_type,
               requires_download
        FROM circle_resources
        WHERE id = $1
          AND circle_id = $2
          AND is_active = TRUE
          AND (expires_at IS NULL OR expires_at > NOW())
          AND (
                access_scope = 'public'
             OR (access_scope = 'member' AND $3)
             OR (access_scope = 'holder_only' AND $4)
             OR (access_scope = 'admin_only' AND $5)
          )
        LIMIT 1
        "#,
    )
    .bind(resource_id)
    .bind(circle_id)
    .bind(is_member || has_holding || is_platform_admin)
    .bind(has_holding || is_platform_admin)
    .bind(is_platform_admin)
    .fetch_optional(&c_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Circle resource not found.".into()))?;

    if let Some(url) = row.try_get::<Option<String>, _>("url").ok().flatten() {
        let url = url.trim().to_string();
        if !is_safe_circle_resource_url(&url) {
            return Err(AppError::Internal(
                "Circle resource has an unsafe delivery URL.".into(),
            ));
        }
        return Ok(Redirect::temporary(&url).into_response());
    }

    let storage_path = row
        .try_get::<Option<String>, _>("storage_object_path")
        .ok()
        .flatten()
        .ok_or_else(|| AppError::NotFound("Circle resource file not found.".into()))?;
    let (bucket, object_path) =
        parse_circle_resource_storage_path(&storage_path, state.config.gcs_bucket.as_deref())?;

    match crate::storage::service::download_object(&bucket, &object_path).await {
        Ok((downloaded_content_type, data)) => {
            let title = row.try_get::<String, _>("title").unwrap_or_default();
            let file_name = row.try_get::<Option<String>, _>("file_name").ok().flatten();
            let filename =
                safe_circle_resource_filename(&title, file_name.as_deref(), &object_path);
            let content_type = row
                .try_get::<Option<String>, _>("mime_type")
                .ok()
                .flatten()
                .unwrap_or(downloaded_content_type);
            let requires_download = row.try_get::<bool, _>("requires_download").unwrap_or(true);
            let mut headers = HeaderMap::new();
            if let Ok(v) = content_type.parse() {
                headers.insert(header::CONTENT_TYPE, v);
            }
            headers.insert(
                header::CACHE_CONTROL,
                "private, max-age=0, no-store".parse().unwrap(),
            );
            headers.insert(
                header::HeaderName::from_static("x-content-type-options"),
                "nosniff".parse().unwrap(),
            );
            if requires_download || !content_type.starts_with("image/") {
                if let Ok(v) = format!("attachment; filename=\"{}\"", filename).parse() {
                    headers.insert(header::CONTENT_DISPOSITION, v);
                }
            }
            Ok((StatusCode::OK, headers, data).into_response())
        }
        Err(e) => {
            tracing::error!(
                resource_id = %resource_id,
                circle_id = %circle_id,
                error = %e,
                "Circle resource delivery failed"
            );
            Err(AppError::Internal(
                "Failed to fetch Circle resource file.".into(),
            ))
        }
    }
}

fn circle_resource_delivery_url(circle_id: Uuid, resource_id: Uuid) -> String {
    format!(
        "/api/community/circles/{}/resources/{}/access",
        circle_id, resource_id
    )
}

fn circle_resource_version_delivery_url(
    circle_id: Uuid,
    resource_id: Uuid,
    version_id: Uuid,
) -> String {
    format!(
        "/api/community/circles/{}/resources/{}/versions/{}/access",
        circle_id, resource_id, version_id
    )
}

fn is_safe_circle_resource_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    !url.chars().any(char::is_control)
        && (lower.starts_with("https://")
            || lower.starts_with("http://")
            || (url.starts_with('/') && !url.starts_with("//")))
}

fn parse_circle_resource_storage_path(
    raw: &str,
    configured_bucket: Option<&str>,
) -> Result<(String, String), AppError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::Internal(
            "Circle resource storage path is empty.".into(),
        ));
    }

    let (bucket, object_path) = if let Some(rest) = raw.strip_prefix("gs://") {
        let mut parts = rest.splitn(2, '/');
        let bucket = parts.next().unwrap_or_default();
        let object_path = parts.next().unwrap_or_default();
        if bucket.is_empty() || object_path.is_empty() {
            return Err(AppError::Internal(
                "Circle resource storage path is invalid.".into(),
            ));
        }
        (bucket.to_string(), object_path.to_string())
    } else if let Some(rest) = raw.strip_prefix("/api/proxy/gcs/") {
        let mut parts = rest.splitn(2, '/');
        let bucket = parts.next().unwrap_or_default();
        let object_path = parts.next().unwrap_or_default();
        if bucket.is_empty() || object_path.is_empty() {
            return Err(AppError::Internal(
                "Circle resource proxy path is invalid.".into(),
            ));
        }
        (bucket.to_string(), object_path.to_string())
    } else {
        let bucket = configured_bucket.ok_or_else(|| {
            AppError::Internal("GCS bucket is not configured for Circle resources.".into())
        })?;
        (bucket.to_string(), raw.to_string())
    };

    if configured_bucket.is_some_and(|allowed| allowed != bucket) {
        return Err(AppError::NotFound("Circle resource not found.".into()));
    }
    if object_path.starts_with('/')
        || object_path.contains("..")
        || object_path.contains("//")
        || object_path.contains('\\')
        || object_path.chars().any(char::is_control)
    {
        return Err(AppError::BadRequest(
            "Circle resource path is invalid.".into(),
        ));
    }

    Ok((bucket, object_path))
}

fn safe_circle_resource_filename(
    title: &str,
    file_name: Option<&str>,
    object_path: &str,
) -> String {
    let candidate = file_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            object_path
                .rsplit('/')
                .next()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(title)
        });
    let cleaned = candidate
        .chars()
        .filter(|ch| !matches!(ch, '"' | '\r' | '\n' | '\\'))
        .collect::<String>();
    if cleaned.trim().is_empty() {
        "circle-resource".to_string()
    } else {
        cleaned
    }
}

async fn get_asset_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(asset_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let has_holding = user_has_asset_holding(&state.db, user.id, asset_id).await?;

    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT id,
               name,
               slug,
               description,
               member_count,
               visibility,
               join_policy,
               is_official,
               is_primary_asset_circle,
               token_gate_asset_id,
               related_asset_id,
               recent_post_count
        FROM circles
        WHERE (related_asset_id = $1 OR token_gate_asset_id = $1)
          AND circle_type = 'asset'
          AND visibility != 'hidden'
        ORDER BY is_primary_asset_circle DESC, is_official DESC, created_at ASC
        LIMIT 1
        "#,
    )
    .bind(asset_id)
    .fetch_optional(&c_pool)
    .await?;

    let Some(row) = row else {
        return Ok(Json(serde_json::json!({
            "circle": null,
            "asset_id": asset_id,
            "has_holding": has_holding,
        })));
    };

    let circle_id: Uuid = row.try_get("id").unwrap_or_default();
    let role = get_circle_member_role(&c_pool, circle_id, user.id).await?;
    let is_member = role.is_some();
    let join_policy = row
        .try_get::<String, _>("join_policy")
        .unwrap_or_else(|_| "request".to_string());
    let access_state = if is_member {
        "open"
    } else if has_holding || join_policy == "open" {
        "join"
    } else if join_policy == "holder_only" {
        "locked"
    } else {
        "request_access"
    };

    Ok(Json(serde_json::json!({
        "asset_id": asset_id,
        "has_holding": has_holding,
        "circle": {
            "id": circle_id,
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "slug": row.try_get::<String, _>("slug").unwrap_or_default(),
            "description": row.try_get::<Option<String>, _>("description").ok().flatten(),
            "member_count": row.try_get::<i32, _>("member_count").unwrap_or(0),
            "visibility": row.try_get::<String, _>("visibility").unwrap_or_else(|_| "private".to_string()),
            "join_policy": join_policy,
            "is_official": row.try_get::<bool, _>("is_official").unwrap_or(false),
            "is_primary_asset_circle": row.try_get::<bool, _>("is_primary_asset_circle").unwrap_or(false),
            "related_asset_id": row.try_get::<Option<Uuid>, _>("related_asset_id").ok().flatten(),
            "token_gate_asset_id": row.try_get::<Option<Uuid>, _>("token_gate_asset_id").ok().flatten(),
            "recent_post_count": row.try_get::<i32, _>("recent_post_count").unwrap_or(0),
            "is_member": is_member,
            "role": role,
            "access_state": access_state,
            "url": format!("/community/circle/{}", row.try_get::<String, _>("slug").unwrap_or_default()),
        }
    })))
}

async fn get_circle_challenges(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_read_access(&state, &c_pool, circle_id, Some(user.id)).await?;
    let challenges =
        crate::community::challenges::list_circle_challenges_for_user(&c_pool, user.id, circle_id)
            .await?;

    Ok(Json(serde_json::json!({
        "challenges": challenges,
        "scope": "circle",
        "circle_id": circle_id,
    })))
}

async fn get_circle_onboarding(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_read_access(&state, &c_pool, circle_id, Some(user.id)).await?;

    let role = get_circle_member_role(&c_pool, circle_id, user.id).await?;
    if role.is_none() {
        return Ok(Json(serde_json::json!({
            "enabled": false,
            "reason": "not_member",
            "steps": [],
            "is_completed": false,
        })));
    }

    let onboarding_enabled: bool =
        sqlx::query_scalar("SELECT COALESCE(onboarding_enabled, TRUE) FROM circles WHERE id = $1")
            .bind(circle_id)
            .fetch_one(&c_pool)
            .await?;

    if !onboarding_enabled {
        return Ok(Json(serde_json::json!({
            "enabled": false,
            "reason": "disabled",
            "steps": [],
            "is_completed": false,
        })));
    }

    sqlx::query(
        r#"
        INSERT INTO circle_onboarding_progress (circle_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (circle_id, user_id) DO NOTHING
        "#,
    )
    .bind(circle_id)
    .bind(user.id)
    .execute(&c_pool)
    .await?;

    use sqlx::Row;
    let row = sqlx::query(
        r#"
        SELECT rules_read,
               introduced_self,
               interests_selected,
               ama_followed,
               first_question_posted,
               is_completed,
               completed_at
        FROM circle_onboarding_progress
        WHERE circle_id = $1 AND user_id = $2
        "#,
    )
    .bind(circle_id)
    .bind(user.id)
    .fetch_one(&c_pool)
    .await?;

    let rules_read = row.try_get::<bool, _>("rules_read").unwrap_or(false);
    let introduced_self = row.try_get::<bool, _>("introduced_self").unwrap_or(false);
    let interests_selected = row
        .try_get::<bool, _>("interests_selected")
        .unwrap_or(false);
    let ama_followed = row.try_get::<bool, _>("ama_followed").unwrap_or(false);
    let first_question_posted = row
        .try_get::<bool, _>("first_question_posted")
        .unwrap_or(false);
    let is_completed = row.try_get::<bool, _>("is_completed").unwrap_or(false);

    let steps = vec![
        serde_json::json!({"code": "rules_read", "label": "Read the Circle rules", "completed": rules_read, "action": "confirm"}),
        serde_json::json!({"code": "introduced_self", "label": "Introduce yourself", "completed": introduced_self, "action": "confirm"}),
        serde_json::json!({"code": "interests_selected", "label": "Choose your interests", "completed": interests_selected, "action": "confirm"}),
        serde_json::json!({"code": "ama_followed", "label": "Follow an upcoming AMA", "completed": ama_followed, "action": "confirm"}),
        serde_json::json!({"code": "first_question_posted", "label": "Post your first question", "completed": first_question_posted, "action": "post_question"}),
    ];

    Ok(Json(serde_json::json!({
        "enabled": !is_completed,
        "is_completed": is_completed,
        "completed_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("completed_at").ok(),
        "steps": steps,
    })))
}

async fn update_circle_onboarding_step(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((circle_id, step)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_circle_write_access(&state, &c_pool, circle_id, user.id).await?;
    let step = normalize_circle_onboarding_step(&step)?;
    mark_circle_onboarding_step(&c_pool, user.id, circle_id, step).await?;

    let challenge_type = match step {
        "rules_read" => Some("circle_guide_read"),
        "introduced_self" => Some("circle_introduction"),
        "ama_followed" => Some("circle_ama_join"),
        _ => None,
    };
    if let Some(challenge_type) = challenge_type {
        let _ = crate::community::challenges::increment_circle_progress(
            &c_pool,
            user.id,
            circle_id,
            challenge_type,
            1,
        )
        .await;
    }

    Ok(Json(serde_json::json!({ "success": true, "step": step })))
}

// ─── Challenge Submissions (14.8.11 follow-up) ─────────────────────────────

#[derive(Deserialize)]
struct SubmitChallengeReq {
    content: String,
}

/// POST /api/community/challenges/:id/submit — create or replace the user's
/// entry for a `requirement_type='submission'` challenge.
async fn submit_challenge_entry(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(challenge_id): Path<Uuid>,
    Json(payload): Json<SubmitChallengeReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    let content = payload.content.trim();
    if content.is_empty() {
        return Err(AppError::BadRequest("Submission cannot be empty.".into()));
    }
    if content.chars().count() > 5_000 {
        return Err(AppError::BadRequest(
            "Submission must be 5,000 characters or fewer.".into(),
        ));
    }

    // Confirm challenge exists and is a submission-type, active.
    let kind: Option<(String, bool)> =
        sqlx::query_as("SELECT requirement_type, is_active FROM challenges WHERE id = $1")
            .bind(challenge_id)
            .fetch_optional(&c_pool)
            .await?;
    let (req_type, is_active) =
        kind.ok_or_else(|| AppError::NotFound("Challenge not found.".into()))?;
    if !is_active {
        return Err(AppError::BadRequest("Challenge is not active.".into()));
    }
    if req_type != "submission" {
        return Err(AppError::BadRequest(
            "This challenge does not accept submissions.".into(),
        ));
    }

    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO challenge_submissions (challenge_id, user_id, content)
        VALUES ($1, $2, $3)
        ON CONFLICT (challenge_id, user_id) DO UPDATE
            SET content = EXCLUDED.content, updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(challenge_id)
    .bind(user.id)
    .bind(content)
    .fetch_one(&c_pool)
    .await?;

    Ok(Json(
        serde_json::json!({ "submission_id": id, "status": "ok" }),
    ))
}

/// GET /api/community/challenges/:id/submissions — list submissions ordered
/// by votes desc; includes whether the current user has voted.
async fn list_challenge_submissions(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(challenge_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT s.id, s.user_id, s.content, s.vote_count, s.created_at,
               EXISTS (
                   SELECT 1 FROM challenge_submission_votes v
                   WHERE v.submission_id = s.id AND v.voter_id = $2
               ) AS has_voted
        FROM challenge_submissions s
        WHERE s.challenge_id = $1
        ORDER BY s.vote_count DESC, s.created_at ASC
        LIMIT 200
        "#,
    )
    .bind(challenge_id)
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;

    // Hydrate display_name + avatar via the user bridge in one batch.
    let user_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect();
    let info_map = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &user_ids,
    )
    .await
    .unwrap_or_default();

    let submissions: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let uid: Uuid = row.try_get("user_id").unwrap_or_default();
            let info = info_map.get(&uid);
            serde_json::json!({
                "id": row.try_get::<Uuid, _>("id").ok(),
                "user_id": uid,
                "display_name": info
                    .map(|i| i.display_name.clone())
                    .unwrap_or_else(|| "Anonymous Investor".to_string()),
                "avatar_url": info.and_then(|i| i.avatar_url.clone()),
                "content": row.try_get::<String, _>("content").unwrap_or_default(),
                "vote_count": row.try_get::<i32, _>("vote_count").unwrap_or(0),
                "has_voted": row.try_get::<bool, _>("has_voted").unwrap_or(false),
                "created_at": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "submissions": submissions })))
}

/// POST /api/community/challenges/submissions/:sid/vote — toggle vote.
async fn toggle_submission_vote(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(submission_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    // Don't allow self-voting (cheap sybil).
    let owner: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_id, challenge_id FROM challenge_submissions WHERE id = $1")
            .bind(submission_id)
            .fetch_optional(&c_pool)
            .await?;
    let (owner_id, challenge_id) =
        owner.ok_or_else(|| AppError::NotFound("Submission not found.".into()))?;
    if owner_id == user.id {
        return Err(AppError::BadRequest(
            "You cannot vote for your own submission.".into(),
        ));
    }

    // Toggle: delete if exists, otherwise insert.
    let existed: Option<()> = sqlx::query_scalar(
        "DELETE FROM challenge_submission_votes
         WHERE submission_id = $1 AND voter_id = $2 RETURNING TRUE",
    )
    .bind(submission_id)
    .bind(user.id)
    .fetch_optional(&c_pool)
    .await?
    .map(|_: bool| ());

    let has_voted = if existed.is_some() {
        false
    } else {
        sqlx::query(
            "INSERT INTO challenge_submission_votes (submission_id, voter_id) VALUES ($1, $2)",
        )
        .bind(submission_id)
        .bind(user.id)
        .execute(&c_pool)
        .await?;
        true
    };

    let vote_count: i32 =
        sqlx::query_scalar("SELECT vote_count FROM challenge_submissions WHERE id = $1")
            .bind(submission_id)
            .fetch_one(&c_pool)
            .await?;

    // Mirror vote count into challenge_progress so the existing list endpoint
    // and completion sweep treat votes as progress for submission challenges.
    let req_value: Option<i32> = sqlx::query_scalar(
        "SELECT requirement_value FROM challenges WHERE id = $1 AND requirement_type = 'submission'",
    )
    .bind(challenge_id)
    .fetch_optional(&c_pool)
    .await?;
    if let Some(req_value) = req_value {
        let _ = sqlx::query(
            r#"
            INSERT INTO challenge_progress (user_id, challenge_id, current_value, is_completed, completed_at)
            VALUES ($1, $2, LEAST($3, $4), $3 >= $4, CASE WHEN $3 >= $4 THEN NOW() END)
            ON CONFLICT (user_id, challenge_id) DO UPDATE
                SET current_value = LEAST(EXCLUDED.current_value, $4),
                    is_completed = (EXCLUDED.current_value >= $4),
                    completed_at = CASE
                        WHEN challenge_progress.completed_at IS NOT NULL THEN challenge_progress.completed_at
                        WHEN EXCLUDED.current_value >= $4 THEN NOW()
                        ELSE NULL
                    END,
                    updated_at = NOW()
            "#,
        )
        .bind(owner_id)
        .bind(challenge_id)
        .bind(vote_count)
        .bind(req_value)
        .execute(&c_pool)
        .await;
    }

    Ok(Json(
        serde_json::json!({ "has_voted": has_voted, "vote_count": vote_count }),
    ))
}

// ─── Notifications Handlers (M5) ────────────────────────────────────────────

async fn list_notifications(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(_q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let limit = _q
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(50);
    let offset = _q
        .get("offset")
        .and_then(|o| o.parse::<i64>().ok())
        .unwrap_or(0);

    let c_pool = get_community_pool(&state)?;
    let notifications = crate::community::notifications::get_my_notifications(
        &c_pool,
        &state.db,
        state.redis.as_ref(),
        user.id,
        limit,
        offset,
    )
    .await?;

    Ok(Json(serde_json::json!({ "notifications": notifications })))
}

async fn get_unread_notification_count(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db).await;
    if user.is_none() {
        return Ok(Json(serde_json::json!({ "count": 0 }))); // Fail silently for count
    }

    let c_pool = get_community_pool(&state)?;
    let count =
        crate::community::notifications::get_unread_count(&c_pool, user.unwrap().id).await?;

    Ok(Json(serde_json::json!({ "count": count })))
}

async fn mark_notification_read(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(notification_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::notifications::mark_as_read(&c_pool, user.id, notification_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// 14.8.15 per-column notification prefs REMOVED 2026-05-15. Canonical
// JSONB endpoint lives at `/api/community/notifications/preferences`.

async fn mark_all_notifications_read(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::notifications::mark_all_as_read(&c_pool, user.id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ─── AMA Handlers (M5) ──────────────────────────────────────────────

async fn list_amas(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let amas = crate::community::amas::list_amas(&c_pool).await?;
    Ok(Json(serde_json::json!({"amas": amas})))
}

async fn get_ama_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(ama_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_ama_read_access(&state, &c_pool, ama_id, user.id).await?;
    let detail = crate::community::amas::get_ama_detail(&c_pool, ama_id, user.id).await?;
    Ok(Json(detail))
}

#[derive(Deserialize)]
struct SubmitQuestionReq {
    question: String,
}

async fn submit_ama_question(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(ama_id): Path<Uuid>,
    Json(payload): Json<SubmitQuestionReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let q_text = payload.question.trim();
    if q_text.len() < 10 || q_text.len() > 500 {
        return Err(AppError::BadRequest(
            "Question must be between 10 and 500 characters.".into(),
        ));
    }

    let c_pool = get_community_pool(&state)?;
    ensure_ama_read_access(&state, &c_pool, ama_id, user.id).await?;
    let question =
        crate::community::amas::submit_question(&c_pool, ama_id, user.id, q_text).await?;

    // Award XP for submitting a question
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "ama_question",
        Some("Submitted an AMA question"),
        Some(10),
    )
    .await;

    Ok(Json(question))
}

async fn toggle_ama_upvote(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((ama_id, qid)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_ama_read_access(&state, &c_pool, ama_id, user.id).await?;
    let question_belongs_to_ama: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM ama_questions WHERE id = $1 AND ama_id = $2)",
    )
    .bind(qid)
    .bind(ama_id)
    .fetch_one(&c_pool)
    .await?;
    if !question_belongs_to_ama {
        return Err(AppError::NotFound("AMA question not found.".into()));
    }
    let added = crate::community::amas::toggle_upvote(&c_pool, qid, user.id).await?;
    Ok(Json(serde_json::json!({"upvoted": added})))
}

// ─── Admin AMA Handlers ─────────────────────────────────────────────

async fn admin_list_amas(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.view")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    let amas = crate::community::amas::list_amas_admin(&c_pool).await?;
    Ok(Json(serde_json::json!({"amas": amas})))
}

#[derive(Deserialize)]
struct CreateAmaReq {
    title: String,
    description: Option<String>,
    expert_name: String,
    expert_title: Option<String>,
    expert_avatar_url: Option<String>,
    #[serde(default)]
    banner_url: Option<String>,
    scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    status: Option<String>,
    circle_id: Option<Uuid>,
    asset_id: Option<Uuid>,
    #[serde(default)]
    rsvp_enabled: bool,
}

async fn admin_create_ama(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateAmaReq>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = admin.user.id;
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    let ama = crate::community::amas::create_ama(
        &c_pool,
        user_id,
        &payload.title,
        payload.description.as_deref(),
        &payload.expert_name,
        payload.expert_title.as_deref(),
        payload.expert_avatar_url.as_deref(),
        payload.banner_url.as_deref(),
        payload.scheduled_at,
        payload.status.as_deref(),
        payload.circle_id,
        payload.asset_id,
        payload.rsvp_enabled,
    )
    .await?;

    crate::community::audit::log(
        &c_pool,
        user_id,
        "ama.create",
        "ama",
        Some(ama.id),
        None,
        Some(serde_json::json!({
            "title": ama.title,
            "status": ama.status,
            "scheduled_at": ama.scheduled_at,
            "circle_id": ama.circle_id,
            "asset_id": ama.asset_id,
            "rsvp_enabled": ama.rsvp_enabled,
        })),
    )
    .await;

    Ok(Json(ama))
}

async fn admin_get_ama_detail(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(ama_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = admin.user.id;
    admin
        .require_permission(&state.db, "community.view")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    let detail = crate::community::amas::get_ama_detail_admin(&c_pool, ama_id, user_id).await?;
    Ok(Json(detail))
}

#[derive(Deserialize)]
struct UpdateAmaStatusReq {
    status: String,
}

async fn admin_update_ama_status(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(ama_id): Path<Uuid>,
    Json(payload): Json<UpdateAmaStatusReq>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = admin.user.id;
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::amas::update_ama_status(&c_pool, ama_id, &payload.status).await?;

    crate::community::audit::log(
        &c_pool,
        user_id,
        "ama.status_update",
        "ama",
        Some(ama_id),
        None,
        Some(serde_json::json!({"status": payload.status})),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
struct AnswerQuestionReq {
    answer: String,
}

async fn admin_answer_question(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((ama_id, qid)): Path<(Uuid, Uuid)>,
    Json(payload): Json<AnswerQuestionReq>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = admin.user.id;
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    let (target_user_id, _) =
        crate::community::amas::answer_question(&c_pool, ama_id, qid, user_id, &payload.answer)
            .await?;

    crate::community::audit::log(
        &c_pool,
        user_id,
        "ama.answer_question",
        "ama_question",
        Some(qid),
        Some(target_user_id),
        Some(serde_json::json!({"ama_id": ama_id})),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn admin_toggle_featured(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((ama_id, qid)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let user_id = admin.user.id;
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;
    let is_featured = crate::community::amas::toggle_featured(&c_pool, ama_id, qid).await?;

    crate::community::audit::log(
        &c_pool,
        user_id,
        if is_featured {
            "ama.question_feature"
        } else {
            "ama.question_unfeature"
        },
        "ama_question",
        Some(qid),
        None,
        Some(serde_json::json!({"ama_id": ama_id})),
    )
    .await;

    Ok(Json(serde_json::json!({"is_featured": is_featured})))
}

// ─── Admin Badge Handlers (M3-ADMIN.4) ──────────────────────────────

async fn log_community_admin_action_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor_user_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Option<Uuid>,
    target_user_id: Option<Uuid>,
    details: serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query(
        r#"INSERT INTO community_audit_logs (actor_user_id, action, entity_type, entity_id, target_user_id, details)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(actor_user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(target_user_id)
    .bind(details)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn validate_badge_code(code: &str) -> Result<String, AppError> {
    let code = code.trim().to_ascii_lowercase();
    let len = code.chars().count();
    let valid = len >= 2
        && len <= 50
        && code
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
        && code
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit());

    if !valid {
        return Err(AppError::BadRequest(
            "Badge code must be 2-50 lowercase letters, numbers, underscores, or hyphens."
                .to_string(),
        ));
    }

    Ok(code)
}

fn validate_badge_text(value: &str, field: &str, max_chars: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest(format!("{field} is required.")));
    }
    if value.chars().count() > max_chars {
        return Err(AppError::BadRequest(format!(
            "{field} must be {max_chars} characters or fewer."
        )));
    }
    Ok(value.to_string())
}

fn validate_optional_badge_text(
    value: Option<String>,
    field: &str,
    max_chars: usize,
) -> Result<Option<String>, AppError> {
    match value {
        Some(value) => validate_badge_text(&value, field, max_chars).map(Some),
        None => Ok(None),
    }
}

fn validate_badge_order(value: Option<i32>) -> Result<i32, AppError> {
    let order = value.unwrap_or(0);
    if !(0..=10_000).contains(&order) {
        return Err(AppError::BadRequest(
            "Display order must be between 0 and 10000.".to_string(),
        ));
    }
    Ok(order)
}

fn map_badge_write_error(err: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return AppError::Conflict("Badge code already exists.".to_string());
        }
    }
    AppError::Database(err)
}

async fn admin_list_badges(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.view")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;

    let c_pool = get_community_pool(&state)?;

    let badges = sqlx::query_as::<_, BadgeRow>(
        "SELECT id, code, name, description, icon, display_order, created_at FROM badges ORDER BY display_order ASC"
    )
    .fetch_all(&c_pool)
    .await?;

    // Get usage counts
    let counts: Vec<(Uuid, i64)> =
        sqlx::query_as("SELECT badge_id, COUNT(*)::BIGINT FROM user_badges GROUP BY badge_id")
            .fetch_all(&c_pool)
            .await?;

    let count_map: std::collections::HashMap<Uuid, i64> = counts.into_iter().collect();
    let awards = sqlx::query_as::<_, BadgeAwardRow>(
        "SELECT badge_id, user_id, earned_at FROM user_badges ORDER BY earned_at DESC",
    )
    .fetch_all(&c_pool)
    .await?;

    let mut award_map: std::collections::HashMap<Uuid, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    for award in awards {
        award_map
            .entry(award.badge_id)
            .or_default()
            .push(serde_json::json!({
                "user_id": award.user_id,
                "earned_at": award.earned_at,
            }));
    }

    let result: Vec<serde_json::Value> = badges
        .iter()
        .map(|b| {
            let mut awarded_users = award_map.remove(&b.id).unwrap_or_default();
            awarded_users.truncate(20);
            serde_json::json!({
                "id": b.id,
                "code": b.code,
                "name": b.name,
                "description": b.description,
                "icon": b.icon,
                "display_order": b.display_order,
                "created_at": b.created_at,
                "users_count": count_map.get(&b.id).copied().unwrap_or(0),
                "awarded_users": awarded_users,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({"badges": result})))
}

#[derive(sqlx::FromRow, serde::Serialize)]
struct BadgeRow {
    id: Uuid,
    code: String,
    name: String,
    description: String,
    icon: String,
    display_order: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct BadgeAwardRow {
    badge_id: Uuid,
    user_id: Uuid,
    earned_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct CreateBadgeReq {
    code: String,
    name: String,
    description: String,
    icon: String,
    display_order: Option<i32>,
}

async fn admin_create_badge(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateBadgeReq>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;
    let user = admin.user;
    let c_pool = get_community_pool(&state)?;
    let code = validate_badge_code(&payload.code)?;
    let name = validate_badge_text(&payload.name, "Badge name", 100)?;
    let description = validate_badge_text(&payload.description, "Badge description", 500)?;
    let icon = validate_badge_text(&payload.icon, "Badge icon", 20)?;
    let display_order = validate_badge_order(payload.display_order)?;
    let mut tx = c_pool.begin().await?;

    let badge = sqlx::query_as::<_, BadgeRow>(
        r#"INSERT INTO badges (code, name, description, icon, display_order)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, code, name, description, icon, display_order, created_at"#,
    )
    .bind(&code)
    .bind(&name)
    .bind(&description)
    .bind(&icon)
    .bind(display_order)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_badge_write_error)?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "badge.create",
        "badge",
        Some(badge.id),
        None,
        serde_json::json!({
            "code": badge.code,
            "name": badge.name,
            "display_order": badge.display_order,
        }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(badge))
}

#[derive(Deserialize)]
struct UpdateBadgeReq {
    name: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    display_order: Option<i32>,
}

async fn admin_update_badge(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(badge_id): Path<Uuid>,
    Json(payload): Json<UpdateBadgeReq>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;
    let user = admin.user;
    let c_pool = get_community_pool(&state)?;
    let name = validate_optional_badge_text(payload.name, "Badge name", 100)?;
    let description = validate_optional_badge_text(payload.description, "Badge description", 500)?;
    let icon = validate_optional_badge_text(payload.icon, "Badge icon", 20)?;
    let display_order = match payload.display_order {
        Some(value) => Some(validate_badge_order(Some(value))?),
        None => None,
    };

    if name.is_none() && description.is_none() && icon.is_none() && display_order.is_none() {
        return Err(AppError::BadRequest(
            "At least one badge field must be provided.".to_string(),
        ));
    }

    let mut tx = c_pool.begin().await?;
    let before = sqlx::query_as::<_, BadgeRow>(
        "SELECT id, code, name, description, icon, display_order, created_at FROM badges WHERE id = $1 FOR UPDATE",
    )
    .bind(badge_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Badge not found.".to_string()))?;

    let badge = sqlx::query_as::<_, BadgeRow>(
        r#"UPDATE badges SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            icon = COALESCE($3, icon),
            display_order = COALESCE($4, display_order)
           WHERE id = $5
           RETURNING id, code, name, description, icon, display_order, created_at"#,
    )
    .bind(name.as_deref())
    .bind(description.as_deref())
    .bind(icon.as_deref())
    .bind(display_order)
    .bind(badge_id)
    .fetch_one(&mut *tx)
    .await?;

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "badge.update",
        "badge",
        Some(badge.id),
        None,
        serde_json::json!({
            "before": {
                "name": before.name,
                "description": before.description,
                "icon": before.icon,
                "display_order": before.display_order,
            },
            "after": {
                "name": badge.name,
                "description": badge.description,
                "icon": badge.icon,
                "display_order": badge.display_order,
            }
        }),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({"success": true, "badge": badge})))
}

#[derive(Deserialize)]
struct GrantBadgeReq {
    badge_code: String,
}

async fn admin_grant_badge(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<GrantBadgeReq>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;
    let user = admin.user;
    let badge_code = validate_badge_code(&payload.badge_code)?;
    let target_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status <> 'deleted')",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !target_exists {
        return Err(AppError::NotFound("Target user not found.".to_string()));
    }

    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;

    let badge_id: Uuid = sqlx::query_scalar("SELECT id FROM badges WHERE code = $1")
        .bind(&badge_code)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Badge code '{}' not found", badge_code)))?;

    sqlx::query(
        "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let inserted_badge_id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING badge_id",
    )
    .bind(user_id)
    .bind(badge_id)
    .fetch_optional(&mut *tx)
    .await?;

    if inserted_badge_id.is_some() {
        log_community_admin_action_tx(
            &mut tx,
            user.id,
            "badge.grant",
            "badge",
            Some(badge_id),
            Some(user_id),
            serde_json::json!({"badge_code": badge_code}),
        )
        .await?;
    }

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "badge_code": badge_code,
        "already_granted": inserted_badge_id.is_none(),
    })))
}

async fn admin_revoke_badge(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((user_id, badge_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    admin
        .require_permission(&state.db, "community.manage")
        .await
        .map_err(|e| AppError::Forbidden(e.to_string()))?;
    let user = admin.user;
    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;
    let badge_code: String = sqlx::query_scalar("SELECT code FROM badges WHERE id = $1")
        .bind(badge_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Badge not found.".to_string()))?;

    let deleted = sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_id = $2")
        .bind(user_id)
        .bind(badge_id)
        .execute(&mut *tx)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "User does not currently hold this badge.".to_string(),
        ));
    }

    log_community_admin_action_tx(
        &mut tx,
        user.id,
        "badge.revoke",
        "badge",
        Some(badge_id),
        Some(user_id),
        serde_json::json!({"badge_code": badge_code}),
    )
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({"success": true})))
}

// ─── Admin User Detail Handler (M3-ADMIN.1) ─────────────────────────

async fn admin_get_user_detail(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    // Community profile
    let mut profile: Option<serde_json::Value> = sqlx::query_scalar(
        r#"SELECT json_build_object(
            'user_id', user_id,
            'bio', bio,
            'post_count', post_count,
            'follower_count', follower_count,
            'following_count', following_count,
            'xp_total', xp_total,
            'level', level,
            'level_name', level_name,
            'login_streak', login_streak,
            'is_community_banned', is_community_banned,
            'ban_reason', ban_reason,
            'warning_count', warning_count,
            'mod_notes', mod_notes,
            'muted_until', muted_until,
            'is_shadowbanned', is_shadowbanned,
            'created_at', created_at
        ) FROM community_profiles WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&c_pool)
    .await?;
    if profile.is_none() {
        return Err(AppError::NotFound("Community user not found.".to_string()));
    }

    if let Some(serde_json::Value::Object(profile_object)) = profile.as_mut() {
        if let Ok(user_info) =
            user_bridge::get_user_info(&state.db, state.redis.as_ref(), user_id).await
        {
            profile_object.insert(
                "display_name".to_string(),
                serde_json::json!(user_info.display_name),
            );
        }
    }

    // User badges
    let badges = sqlx::query_as::<_, BadgeRow>(
        r#"SELECT b.id, b.code, b.name, b.description, b.icon, b.display_order, ub.earned_at AS created_at
           FROM user_badges ub
           JOIN badges b ON b.id = ub.badge_id
           WHERE ub.user_id = $1
           ORDER BY b.display_order"#
    )
    .bind(user_id)
    .fetch_all(&c_pool)
    .await?;

    // Recent posts (last 10)
    let recent_posts: Vec<serde_json::Value> = sqlx::query_scalar(
        r#"SELECT json_build_object(
            'id', id, 'content', LEFT(content, 200), 'post_type', post_type,
            'is_hidden', is_hidden, 'created_at', created_at, 'reaction_count', reaction_count
        ) FROM posts WHERE user_id = $1 ORDER BY created_at DESC LIMIT 10"#,
    )
    .bind(user_id)
    .fetch_all(&c_pool)
    .await?;

    // XP summary
    let xp_summary = crate::community::xp::get_xp_summary(&c_pool, user_id)
        .await
        .ok();

    // Core user data (from main DB)
    let core_data: Option<serde_json::Value> = sqlx::query_scalar(
        r#"SELECT json_build_object(
            'id', u.id, 'email', u.email, 'status', u.status,
            'created_at', u.created_at,
            'first_name', p.first_name, 'last_name', p.last_name,
            'avatar_url', u.avatar_url
        ) FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        WHERE u.id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "user": core_data,
        "profile": profile,
        "badges": badges,
        "recent_posts": recent_posts,
        "xp_summary": xp_summary,
    })))
}

// ─── Admin Circle Handlers (M4-ADMIN) ──────────────────────────────────────

#[derive(Deserialize)]
struct AdminCircleListQuery {
    search: Option<String>,
    visibility: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn admin_list_circles(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Query(query): Query<AdminCircleListQuery>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;

    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let visibility = match query.visibility.as_deref() {
        Some("public") => Some("public"),
        Some("private") => Some("private"),
        Some("hidden") => Some("hidden"),
        Some("all") | None | Some("") => None,
        Some(_) => {
            return Err(AppError::BadRequest(
                "visibility must be public, private, hidden, or all".to_string(),
            ))
        }
    };
    let limit = query
        .limit
        .unwrap_or(ADMIN_CIRCLE_DEFAULT_LIMIT)
        .clamp(1, ADMIN_CIRCLE_MAX_LIMIT);
    let offset = query.offset.unwrap_or(0).max(0);

    let (circles, total, total_members, total_xp) =
        crate::community::circles::admin_get_circles(&c_pool, search, visibility, limit, offset)
            .await?;

    Ok(Json(serde_json::json!({
        "circles": circles,
        "total": total,
        "total_members": total_members,
        "total_xp": total_xp,
        "limit": limit,
        "offset": offset,
    })))
}

async fn admin_delete_circle(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::admin_delete_circle(&c_pool, circle_id, admin.user.id).await?;
    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

async fn admin_remove_circle_member(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((circle_id, target_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::admin_remove_member(&c_pool, circle_id, target_id).await?;
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "circle.remove_member",
        "circle",
        Some(circle_id),
        Some(target_id),
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "status": "removed" })))
}

async fn admin_get_circle_detail(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::get_circle(&c_pool, circle_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;
    let members = crate::community::circles::get_circle_members(&c_pool, circle_id).await?;
    let enriched = enrich_circle_members(&state, members).await;

    Ok(Json(serde_json::json!({
        "circle": circle,
        "members": enriched,
    })))
}

#[derive(serde::Deserialize)]
struct AdminUpdateCircleReq {
    name: Option<String>,
    description: Option<String>,
    avatar_emoji: Option<String>,
    is_public: Option<bool>,
    circle_type: Option<String>,
    visibility: Option<String>,
    join_policy: Option<String>,
    is_official: Option<bool>,
    kyc_required: Option<bool>,
    private_investor_club: Option<bool>,
    allow_cross_post: Option<bool>,
}

async fn admin_update_circle(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<AdminUpdateCircleReq>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::admin_force_update_circle(
        &c_pool,
        circle_id,
        payload.name.as_deref(),
        payload.description.as_deref(),
        payload.avatar_emoji.as_deref(),
        payload.is_public,
        payload.circle_type.as_deref(),
        payload.visibility.as_deref(),
        payload.join_policy.as_deref(),
        payload.is_official,
        payload.kyc_required,
        payload.private_investor_club,
        payload.allow_cross_post,
    )
    .await?;
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "circle.update",
        "circle",
        Some(circle_id),
        None,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "circle": circle })))
}

#[derive(serde::Deserialize)]
struct AdminTransferCircleReq {
    new_owner_id: Uuid,
}

async fn admin_transfer_circle(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<AdminTransferCircleReq>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::admin_force_transfer_circle(
        &c_pool,
        &state.db,
        circle_id,
        payload.new_owner_id,
    )
    .await?;
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "circle.transfer",
        "circle",
        Some(circle_id),
        Some(payload.new_owner_id),
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ─── Admin Leaderboard Handlers (M4-ADMIN) ─────────────────────────────────

async fn admin_get_leaderboard(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(_q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    require_community_view_or_manage(&state, &admin).await?;
    let limit = _q
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(100);
    let c_pool = get_community_pool(&state)?;
    let entries = crate::community::xp::get_user_leaderboard(&c_pool, limit).await?;
    Ok(Json(serde_json::json!({ "leaderboard": entries })))
}

#[derive(serde::Deserialize)]
struct AdminAwardXpReq {
    amount: i32,
    reason_label: String,
    description: String,
}

const MAX_ADMIN_XP_ADJUSTMENT: i32 = 10_000;
const MAX_ADMIN_XP_DESCRIPTION_LEN: usize = 200;

fn validate_admin_xp_adjustment(
    payload: &AdminAwardXpReq,
) -> Result<(&str, i32, String), AppError> {
    let reason = payload.reason_label.trim();
    if !matches!(reason, "admin_grant" | "admin_revoke") {
        return Err(AppError::BadRequest(
            "XP adjustment action must be admin_grant or admin_revoke.".to_string(),
        ));
    }

    if payload.amount == 0 {
        return Err(AppError::BadRequest(
            "XP adjustment amount must not be zero.".to_string(),
        ));
    }

    if payload.amount == i32::MIN || payload.amount.abs() > MAX_ADMIN_XP_ADJUSTMENT {
        return Err(AppError::BadRequest(format!(
            "XP adjustment amount must be between 1 and {}.",
            MAX_ADMIN_XP_ADJUSTMENT
        )));
    }

    if reason == "admin_grant" && payload.amount < 0 {
        return Err(AppError::BadRequest(
            "Grant adjustments must use a positive amount.".to_string(),
        ));
    }

    if reason == "admin_revoke" && payload.amount > 0 {
        return Err(AppError::BadRequest(
            "Revoke adjustments must use a negative amount.".to_string(),
        ));
    }

    let description = payload.description.trim();
    if description.is_empty() {
        return Err(AppError::BadRequest(
            "XP adjustment description is required.".to_string(),
        ));
    }

    if description.chars().count() > MAX_ADMIN_XP_DESCRIPTION_LEN {
        return Err(AppError::BadRequest(format!(
            "XP adjustment description must be {} characters or fewer.",
            MAX_ADMIN_XP_DESCRIPTION_LEN
        )));
    }

    Ok((reason, payload.amount, description.to_string()))
}

async fn admin_award_xp(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AdminAwardXpReq>,
) -> Result<impl IntoResponse, AppError> {
    require_community_manage(&state, &admin).await?;
    let (reason, amount, description) = validate_admin_xp_adjustment(&payload)?;

    let target_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND status <> 'deleted')",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;
    if !target_exists {
        return Err(AppError::NotFound("Target user not found.".to_string()));
    }

    let c_pool = get_community_pool(&state)?;
    let mut tx = c_pool.begin().await?;

    sqlx::query(
        "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let current_xp: i32 =
        sqlx::query_scalar("SELECT xp_total FROM community_profiles WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

    let new_xp = current_xp
        .checked_add(amount)
        .ok_or_else(|| AppError::BadRequest("XP adjustment would overflow.".to_string()))?;
    if new_xp < 0 {
        return Err(AppError::BadRequest(
            "XP adjustment cannot reduce the user's XP below zero.".to_string(),
        ));
    }

    let (new_level, new_level_name): (i32, String) = sqlx::query_as(
        "SELECT level, name FROM xp_levels WHERE min_xp <= $1 ORDER BY level DESC LIMIT 1",
    )
    .bind(new_xp)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or((1, "Seedling".to_string()));

    sqlx::query(
        "INSERT INTO xp_ledger (user_id, amount, reason, description) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(amount)
    .bind(reason)
    .bind(&description)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "UPDATE community_profiles SET xp_total = $1, level = $2, level_name = $3, updated_at = NOW() WHERE user_id = $4",
    )
    .bind(new_xp)
    .bind(new_level)
    .bind(&new_level_name)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO community_audit_logs (actor_user_id, action, entity_type, entity_id, target_user_id, details)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(admin.user.id)
    .bind("xp.adjust")
    .bind("user")
    .bind(Option::<Uuid>::None)
    .bind(user_id)
    .bind(serde_json::json!({
        "amount": amount,
        "reason": reason,
        "previous_xp": current_xp,
        "new_xp": new_xp,
        "new_level": new_level,
        "description": description
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(serde_json::json!({
        "status": "xp_adjusted",
        "amount": amount,
        "previous_xp": current_xp,
        "new_xp": new_xp,
        "level": new_level,
        "level_name": new_level_name
    })))
}

// ─── Admin Audit Log API (M2-ADMIN.7) ────────────────────────────────────────

async fn admin_get_community_audit_log(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let limit = q
        .get("limit")
        .and_then(|l| l.parse::<i64>().ok())
        .unwrap_or(50)
        .min(200);
    let offset = q
        .get("offset")
        .and_then(|o| o.parse::<i64>().ok())
        .unwrap_or(0);
    let entity_type_filter = q.get("entity_type").cloned();
    let action_filter = q.get("action").cloned();
    // Phase 2 task 17: filter by the user the audit entry was *about* so the
    // admin user-detail page can pull just that user's history.
    let target_user_filter = q
        .get("target_user_id")
        .and_then(|v| Uuid::parse_str(v).ok());

    // Build dynamic query
    let mut conditions = vec!["1=1".to_string()];
    if let Some(ref et) = entity_type_filter {
        conditions.push(format!("entity_type = '{}'", et.replace('\'', "")));
    }
    if let Some(ref act) = action_filter {
        conditions.push(format!("action = '{}'", act.replace('\'', "")));
    }
    if let Some(target) = target_user_filter {
        conditions.push(format!("target_user_id = '{}'", target));
    }
    let where_clause = conditions.join(" AND ");

    let sql = format!(
        "SELECT id, actor_user_id, action, entity_type, entity_id, target_user_id, details, created_at \
         FROM community_audit_logs WHERE {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
        where_clause, limit, offset
    );

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Option<Uuid>,
            String,
            String,
            Option<Uuid>,
            Option<Uuid>,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        ),
    >(&sql)
    .fetch_all(&c_pool)
    .await?;

    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.0,
                "actor_user_id": r.1,
                "action": r.2,
                "entity_type": r.3,
                "entity_id": r.4,
                "target_user_id": r.5,
                "details": r.6,
                "created_at": r.7,
            })
        })
        .collect();

    Ok(Json(
        serde_json::json!({ "logs": entries, "count": entries.len() }),
    ))
}

/// CO.13: stream the community audit log as CSV. Same filters as the JSON
/// endpoint (`entity_type`, `action`, `target_user_id`) so an admin can
/// download exactly what they're looking at on the page. Capped at 10k
/// rows per export to keep the response under Cloud Run's request size.
async fn admin_export_community_audit_log_csv(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let entity_type_filter = q.get("entity_type").cloned();
    let action_filter = q.get("action").cloned();
    let target_user_filter = q
        .get("target_user_id")
        .and_then(|v| Uuid::parse_str(v).ok());

    let mut conditions = vec!["1=1".to_string()];
    if let Some(ref et) = entity_type_filter {
        conditions.push(format!("entity_type = '{}'", et.replace('\'', "")));
    }
    if let Some(ref act) = action_filter {
        conditions.push(format!("action = '{}'", act.replace('\'', "")));
    }
    if let Some(target) = target_user_filter {
        conditions.push(format!("target_user_id = '{}'", target));
    }
    let where_clause = conditions.join(" AND ");

    let sql = format!(
        "SELECT id, actor_user_id, action, entity_type, entity_id, target_user_id, details, created_at \
         FROM community_audit_logs WHERE {} ORDER BY created_at DESC LIMIT 10000",
        where_clause
    );
    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Option<Uuid>,
            String,
            String,
            Option<Uuid>,
            Option<Uuid>,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        ),
    >(&sql)
    .fetch_all(&c_pool)
    .await?;

    fn csv_escape(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    let mut body = String::with_capacity(rows.len() * 200);
    body.push_str(
        "id,actor_user_id,action,entity_type,entity_id,target_user_id,details,created_at\n",
    );
    for r in rows {
        body.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            r.0,
            r.1.map(|v| v.to_string()).unwrap_or_default(),
            csv_escape(&r.2),
            csv_escape(&r.3),
            r.4.map(|v| v.to_string()).unwrap_or_default(),
            r.5.map(|v| v.to_string()).unwrap_or_default(),
            csv_escape(&r.6.to_string()),
            r.7.to_rfc3339(),
        ));
    }

    let filename = format!(
        "community-audit-log-{}.csv",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    Ok((
        axum::http::StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/csv; charset=utf-8".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        body,
    ))
}

// ─── Ban Appeals Handlers (M7-BE.5) ──────────────────────────────────────────

async fn submit_ban_appeal(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(payload): Json<models::CreateBanAppealReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    if payload.appeal_text.len() < 10 || payload.appeal_text.len() > 2000 {
        return Err(AppError::BadRequest(
            "Appeal text must be between 10 and 2000 characters.".into(),
        ));
    }

    // Check if the user is actually banned
    let is_banned: Option<bool> =
        sqlx::query_scalar("SELECT is_community_banned FROM community_profiles WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(&c_pool)
            .await?;

    if !is_banned.unwrap_or(false) {
        return Err(AppError::BadRequest("You are not currently banned.".into()));
    }

    // Check if they already have a pending appeal
    let existing_pending: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM ban_appeals WHERE user_id = $1 AND status = 'pending'")
            .bind(user.id)
            .fetch_optional(&c_pool)
            .await?;

    if existing_pending.is_some() {
        return Err(AppError::BadRequest(
            "You already have a pending ban appeal. Please wait for an admin to review it.".into(),
        ));
    }

    let appeal_id: Uuid = sqlx::query_scalar(
        "INSERT INTO ban_appeals (user_id, appeal_text) VALUES ($1, $2) RETURNING id",
    )
    .bind(user.id)
    .bind(payload.appeal_text)
    .fetch_one(&c_pool)
    .await?;

    crate::community::audit::log(
        &c_pool,
        user.id,
        "appeal.submit",
        "ban_appeal",
        Some(appeal_id),
        Some(user.id),
        Some(serde_json::json!({"status": "pending"})),
    )
    .await;

    Ok(Json(
        serde_json::json!({"success": true, "id": appeal_id, "message": "Appeal submitted successfully."}),
    ))
}

async fn get_ban_appeals(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(_q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    // Status filter
    let status_filter = _q.get("status").map(|s| s.as_str()).unwrap_or("pending");

    let records = sqlx::query(
        r#"
        SELECT a.id, a.user_id, a.appeal_text, a.status, a.admin_notes, a.created_at, a.resolved_at
        FROM ban_appeals a
        WHERE ($1 = 'all' OR a.status = $1)
        ORDER BY a.created_at ASC
        "#,
    )
    .bind(status_filter)
    .fetch_all(&c_pool)
    .await?;

    // We manually construct BanAppealDisplay and fetch names
    use sqlx::Row;
    let mut appeals = Vec::new();
    for rec in records {
        let user_id: Uuid = rec.try_get("user_id")?;
        let id: Uuid = rec.try_get("id")?;
        let appeal_text: String = rec.try_get("appeal_text")?;
        let r_status: String = rec.try_get("status")?;
        let admin_notes: Option<String> = rec.try_get("admin_notes")?;
        let created_at: chrono::DateTime<chrono::Utc> = rec.try_get("created_at")?;
        let resolved_at: Option<chrono::DateTime<chrono::Utc>> = rec.try_get("resolved_at")?;
        let name = user_bridge::get_user_info(&state.db, state.redis.as_ref(), user_id)
            .await
            .map(|u| u.display_name)
            .unwrap_or_else(|_| "Unknown".into());

        appeals.push(models::BanAppealDisplay {
            id,
            user_id,
            display_name: name,
            appeal_text,
            status: r_status,
            admin_notes,
            created_at,
            resolved_at,
        });
    }

    Ok(Json(serde_json::json!({"appeals": appeals})))
}

async fn review_ban_appeal(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(appeal_id): Path<Uuid>,
    Json(payload): Json<models::AdminReviewAppealReq>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    let status = match payload.action.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        _ => {
            return Err(AppError::BadRequest(
                "Action must be 'approve' or 'reject'".into(),
            ))
        }
    };

    let mut tx = c_pool.begin().await.map_err(AppError::Database)?;

    use sqlx::Row;
    let record = sqlx::query(
        "UPDATE ban_appeals SET status = $1, admin_notes = $2, resolved_at = NOW() WHERE id = $3 AND status = 'pending' RETURNING user_id"
    )
    .bind(status)
    .bind(&payload.admin_notes)
    .bind(appeal_id)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id: Uuid = match record {
        Some(r) => r.try_get("user_id")?,
        None => return Err(AppError::NotFound("Pending appeal not found".into())),
    };

    if status == "approved" {
        // Lift the ban
        sqlx::query("UPDATE community_profiles SET is_community_banned = false, ban_expires_at = NULL WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Notify the user
        crate::community::notifications::notify_user(
            &c_pool,
            user_id,
            None,
            "system_alert",
            None,
            "Ban Appeal Approved: Your community ban has been lifted. You can now post and interact again.",
            None
        ).await.ok();
    } else {
        // Notify the user of rejection
        crate::community::notifications::notify_user(
            &c_pool,
            user_id,
            None,
            "system_alert",
            None,
            "Ban Appeal Rejected: Your ban appeal was reviewed and rejected. The ban remains in place.",
            None
        ).await.ok();
    }

    tx.commit().await.map_err(AppError::Database)?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        if status == "approved" {
            "appeal.approve"
        } else {
            "appeal.reject"
        },
        "ban_appeal",
        Some(appeal_id),
        Some(user_id),
        Some(serde_json::json!({
            "status": status,
            "admin_notes": payload.admin_notes,
        })),
    )
    .await;

    Ok(Json(serde_json::json!({"success": true, "status": status})))
}

// ═══════════════════════════════════════════════════════════════════
// UX.6 — BOOKMARKS / SAVED POSTS
// ═══════════════════════════════════════════════════════════════════

/// Toggle bookmark on a post. If already bookmarked, removes it. Returns {"bookmarked": true/false}.
async fn toggle_bookmark(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_post_read_access(&state, &c_pool, post_id, Some(user.id)).await?;

    // Check if already bookmarked
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM bookmarks WHERE user_id = $1 AND post_id = $2")
            .bind(user.id)
            .bind(post_id)
            .fetch_optional(&c_pool)
            .await?;

    if let Some(_) = existing {
        // Remove bookmark
        sqlx::query("DELETE FROM bookmarks WHERE user_id = $1 AND post_id = $2")
            .bind(user.id)
            .bind(post_id)
            .execute(&c_pool)
            .await?;
        Ok(Json(serde_json::json!({"bookmarked": false})))
    } else {
        // Verify post exists and is not hidden
        let post_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1 AND is_hidden = false)",
        )
        .bind(post_id)
        .fetch_one(&c_pool)
        .await?;

        if !post_exists {
            return Err(AppError::NotFound("Post not found".into()));
        }

        // Add bookmark
        sqlx::query(
            "INSERT INTO bookmarks (user_id, post_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user.id)
        .bind(post_id)
        .execute(&c_pool)
        .await?;
        Ok(Json(serde_json::json!({"bookmarked": true})))
    }
}

/// Get bookmark status for a post
async fn get_bookmark_status(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    ensure_post_read_access(&state, &c_pool, post_id, Some(user.id)).await?;

    let is_bookmarked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM bookmarks WHERE user_id = $1 AND post_id = $2)",
    )
    .bind(user.id)
    .bind(post_id)
    .fetch_one(&c_pool)
    .await?;

    Ok(Json(serde_json::json!({"bookmarked": is_bookmarked})))
}

/// List all bookmarked posts for the current user (paginated, newest first)
#[derive(Deserialize)]
pub struct BookmarkQuery {
    pub page: Option<i64>,
}

// Phase 2 task 15: extracted so the HTMX feed-list partial can reuse the
// same data shape as the JSON /api/community/bookmarks endpoint.
pub async fn get_bookmark_feed_data(
    state: &AppState,
    user_id: Uuid,
    page: Option<i64>,
) -> Result<Vec<models::PostDisplay>, AppError> {
    let c_pool = get_community_pool(state)?;

    let limit: i64 = 20;
    let offset = (page.unwrap_or(1).max(1) - 1) * limit;

    let posts = sqlx::query_as::<_, models::Post>(
        r#"
        SELECT p.*
        FROM bookmarks b
        JOIN posts p ON b.post_id = p.id
        WHERE b.user_id = $1
          AND p.is_hidden = false
          AND (
            p.circle_id IS NULL
            OR EXISTS (
              SELECT 1
              FROM circles c
              WHERE c.id = p.circle_id
                AND (
                  c.is_public = true
                  OR EXISTS (
                    SELECT 1 FROM circle_members cm
                    WHERE cm.circle_id = p.circle_id AND cm.user_id = $1
                  )
                )
            )
          )
        ORDER BY b.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;
    let badges = service::get_badges_batch(&c_pool, &user_ids).await?;

    let mut feed = Vec::with_capacity(posts.len());
    for p in posts {
        let author = authors.get(&p.user_id);
        let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();
        let author_name = author
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Anonymous".into());
        feed.push(map_to_post_display(
            &p,
            author_name,
            author.and_then(|a| a.avatar_url.clone()),
            author_badges,
            false,
            true, // every post here is bookmarked by definition
        ));
    }
    Ok(feed)
}

async fn list_bookmarks(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<BookmarkQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let feed = get_bookmark_feed_data(&state, user.id, query.page).await?;
    Ok(Json(feed))
}

// ═══════════════════════════════════════════════════════════════════
// UX.11 — NATIVE POLLS & SURVEYS
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct VotePollReq {
    pub option_id: Uuid,
}

/// Vote on a poll option. Users can only vote once per poll (unless allows_multiple).
async fn vote_on_poll(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<VotePollReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    check_user_not_banned(&c_pool, user.id).await?;

    use sqlx::Row;

    // Get the poll for this post
    let poll_row =
        sqlx::query("SELECT id, allows_multiple, expires_at FROM polls WHERE post_id = $1")
            .bind(post_id)
            .fetch_optional(&c_pool)
            .await?
            .ok_or_else(|| AppError::NotFound("No poll found for this post".into()))?;

    let poll_id: Uuid = poll_row.try_get("id")?;
    let allows_multiple: bool = poll_row.try_get("allows_multiple")?;
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = poll_row.try_get("expires_at")?;

    // Check if poll has expired
    if let Some(exp) = expires_at {
        if exp < chrono::Utc::now() {
            return Err(AppError::BadRequest("This poll has expired".into()));
        }
    }

    // Verify the option belongs to this poll
    let option_valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM poll_options WHERE id = $1 AND poll_id = $2)",
    )
    .bind(payload.option_id)
    .bind(poll_id)
    .fetch_one(&c_pool)
    .await?;

    if !option_valid {
        return Err(AppError::BadRequest("Invalid poll option".into()));
    }

    let mut tx = c_pool.begin().await?;

    if !allows_multiple {
        // Remove any existing vote on this poll
        sqlx::query("DELETE FROM poll_votes WHERE poll_id = $1 AND user_id = $2")
            .bind(poll_id)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;
    } else {
        // Check for existing vote on the same option
        let already_voted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM poll_votes WHERE poll_id = $1 AND user_id = $2 AND option_id = $3)"
        )
        .bind(poll_id)
        .bind(user.id)
        .bind(payload.option_id)
        .fetch_one(&mut *tx)
        .await?;

        if already_voted {
            // Toggle off — remove this vote
            sqlx::query(
                "DELETE FROM poll_votes WHERE poll_id = $1 AND user_id = $2 AND option_id = $3",
            )
            .bind(poll_id)
            .bind(user.id)
            .bind(payload.option_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(Json(
                serde_json::json!({"voted": false, "option_id": payload.option_id}),
            ));
        }
    }

    // Insert new vote
    sqlx::query("INSERT INTO poll_votes (poll_id, option_id, user_id) VALUES ($1, $2, $3)")
        .bind(poll_id)
        .bind(payload.option_id)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(
        serde_json::json!({"voted": true, "option_id": payload.option_id}),
    ))
}

/// Get poll results for a post, including whether the current user has voted
async fn get_poll_results(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db).await;
    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;

    // Get poll
    let poll_row = sqlx::query(
        "SELECT id, question, allows_multiple, total_votes, expires_at FROM polls WHERE post_id = $1"
    )
    .bind(post_id)
    .fetch_optional(&c_pool)
    .await?;

    let poll_row = match poll_row {
        Some(r) => r,
        None => return Ok(Json(serde_json::json!(null))),
    };

    let poll_id: Uuid = poll_row.try_get("id")?;
    let question: String = poll_row.try_get("question")?;
    let allows_multiple: bool = poll_row.try_get("allows_multiple")?;
    let total_votes: i32 = poll_row.try_get("total_votes")?;
    let expires_at: Option<chrono::DateTime<chrono::Utc>> = poll_row.try_get("expires_at")?;

    let is_expired = expires_at.map(|e| e < chrono::Utc::now()).unwrap_or(false);

    // Get options with vote counts
    let option_rows = sqlx::query(
        "SELECT id, label, sort_order, vote_count FROM poll_options WHERE poll_id = $1 ORDER BY sort_order ASC"
    )
    .bind(poll_id)
    .fetch_all(&c_pool)
    .await?;

    // Get user's votes (if logged in)
    let user_voted_options: Vec<Uuid> = if let Some(ref u) = user {
        sqlx::query_scalar("SELECT option_id FROM poll_votes WHERE poll_id = $1 AND user_id = $2")
            .bind(poll_id)
            .bind(u.id)
            .fetch_all(&c_pool)
            .await?
    } else {
        vec![]
    };

    let has_voted = !user_voted_options.is_empty();

    let options: Vec<serde_json::Value> = option_rows
        .iter()
        .map(|r| {
            let opt_id: Uuid = r.try_get("id").unwrap_or_default();
            let label: String = r.try_get("label").unwrap_or_default();
            let vote_count: i32 = r.try_get("vote_count").unwrap_or(0);
            let pct = if total_votes > 0 {
                (vote_count as f64 / total_votes as f64 * 100.0).round() as i32
            } else {
                0
            };

            serde_json::json!({
                "id": opt_id,
                "label": label,
                "vote_count": vote_count,
                "percentage": pct,
                "user_voted": user_voted_options.contains(&opt_id),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "poll_id": poll_id,
        "question": question,
        "allows_multiple": allows_multiple,
        "total_votes": total_votes,
        "is_expired": is_expired,
        "expires_at": expires_at,
        "has_voted": has_voted,
        "options": options,
    })))
}

// ═══════════════════════════════════════════════════════════════════
// UX.4 — HASHTAG ARCHITECTURE
// ═══════════════════════════════════════════════════════════════════

/// Get trending hashtags (top 20 by post_count)
async fn get_trending_hashtags(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT id, tag, post_count FROM hashtags WHERE post_count > 0 ORDER BY post_count DESC LIMIT 20"
    )
    .fetch_all(&c_pool)
    .await?;

    let hashtags: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").unwrap_or_default(),
                "tag": r.try_get::<String, _>("tag").unwrap_or_default(),
                "post_count": r.try_get::<i32, _>("post_count").unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(hashtags))
}

// Phase 3 task 28: autocomplete suggestions for the post composer.

#[derive(Deserialize)]
struct SuggestQuery {
    pub q: String,
}

async fn suggest_hashtags(
    State(state): State<AppState>,
    Query(q): Query<SuggestQuery>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let prefix = q.q.trim().trim_start_matches('#').to_lowercase();
    if prefix.is_empty() {
        return Ok(Json(serde_json::json!({ "hashtags": [] })));
    }
    let pattern = format!("{}%", prefix);
    let rows = sqlx::query_as::<_, (String, i32)>(
        "SELECT tag, post_count FROM hashtags WHERE tag ILIKE $1 ORDER BY post_count DESC, tag ASC LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(&c_pool)
    .await?;
    let hashtags: Vec<serde_json::Value> = rows
        .iter()
        .map(|(tag, count)| serde_json::json!({ "tag": tag, "post_count": count }))
        .collect();
    Ok(Json(serde_json::json!({ "hashtags": hashtags })))
}

async fn suggest_mentions(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(q): Query<SuggestQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let prefix = q.q.trim().trim_start_matches('@');
    if prefix.is_empty() {
        return Ok(Json(serde_json::json!({ "users": [], "circles": [] })));
    }
    let circle_prefix = prefix.strip_prefix("circle/").unwrap_or(prefix);
    let pattern = format!("{}%", prefix);
    let circle_pattern = format!("{}%", circle_prefix);
    let rows = sqlx::query_as::<_, (Uuid, Option<String>, Option<String>)>(
        r#"
        SELECT u.id, up.display_name, u.avatar_url
        FROM users u
        JOIN user_profiles up ON up.user_id = u.id
        WHERE up.display_name ILIKE $1
          AND u.status <> 'deleted'
        ORDER BY up.display_name ASC
        LIMIT 10
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;
    let c_pool = get_community_pool(&state)?;
    use sqlx::Row;
    let circle_rows = sqlx::query(
        r#"
        SELECT c.id, c.slug, c.name, c.visibility, c.is_public, cm.role AS my_role
        FROM circles c
        LEFT JOIN circle_members cm
          ON cm.circle_id = c.id
         AND cm.user_id = $2
        WHERE (c.name ILIKE $1 OR c.slug ILIKE $1)
          AND (
            (c.visibility = 'public' AND c.is_public = TRUE)
            OR cm.user_id IS NOT NULL
          )
        ORDER BY
          CASE WHEN c.slug ILIKE $1 THEN 0 ELSE 1 END,
          c.member_count DESC,
          c.name ASC
        LIMIT 10
        "#,
    )
    .bind(&circle_pattern)
    .bind(user.id)
    .fetch_all(&c_pool)
    .await?;
    let users: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, name, avatar)| {
            serde_json::json!({
                "user_id": id,
                "display_name": name.clone().unwrap_or_default(),
                "avatar_url": avatar,
            })
        })
        .collect();
    let circles: Vec<serde_json::Value> = circle_rows
        .iter()
        .map(|row| {
            let slug = row.try_get::<String, _>("slug").unwrap_or_default();
            serde_json::json!({
                "circle_id": row.try_get::<Uuid, _>("id").ok(),
                "slug": slug,
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "visibility": row.try_get::<String, _>("visibility").unwrap_or_else(|_| "public".to_string()),
                "is_public": row.try_get::<bool, _>("is_public").unwrap_or(false),
                "my_role": row.try_get::<Option<String>, _>("my_role").ok().flatten(),
                "mention_token": format!("@circle/{}", slug),
            })
        })
        .collect();
    Ok(Json(
        serde_json::json!({ "users": users, "circles": circles }),
    ))
}

// ─── UX.8: Trending posts (sidebar widget) ──────────────────────────

#[derive(Deserialize)]
struct TrendingQuery {
    pub limit: Option<i64>,
}

/// GET /api/community/trending — top posts by engagement in last 7 days.
/// Score = reaction_count + 2 * comment_count (comments weighted higher
/// because they're a stronger engagement signal than a one-tap reaction).
async fn list_trending_posts(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(q): Query<TrendingQuery>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    let limit = q.limit.unwrap_or(3).clamp(1, 10);
    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT id, user_id, content, reaction_count, comment_count, created_at
        FROM posts
        WHERE is_hidden = FALSE
          AND created_at > NOW() - INTERVAL '7 days'
        ORDER BY (reaction_count + 2 * comment_count) DESC, created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&c_pool)
    .await?;

    let user_ids: Vec<Uuid> = rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect();
    let info_map = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &user_ids,
    )
    .await
    .unwrap_or_default();

    let posts: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let uid: Uuid = r.try_get("user_id").unwrap_or_default();
            let info = info_map.get(&uid);
            let content: String = r.try_get("content").unwrap_or_default();
            let snippet = if content.chars().count() > 140 {
                let mut s: String = content.chars().take(137).collect();
                s.push('…');
                s
            } else {
                content
            };
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok(),
                "author_name": info
                    .map(|i| i.display_name.clone())
                    .unwrap_or_else(|| "Anonymous".to_string()),
                "content": snippet,
                "reaction_count": r.try_get::<i32, _>("reaction_count").unwrap_or(0),
                "comment_count": r.try_get::<i32, _>("comment_count").unwrap_or(0),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "posts": posts })))
}

// ─── M6-FEAT.4: Member Directory ────────────────────────────────────

#[derive(Deserialize)]
struct MemberDirectoryQuery {
    /// Display-name substring (case-insensitive). Empty returns top members.
    pub q: Option<String>,
    /// `xp` (default), `recent`, `posts`. Anything else falls back to `xp`.
    pub sort: Option<String>,
    pub page: Option<i64>,
}

/// GET /api/community/members?q=&sort=&page= — paginated member list with
/// display_name, avatar, level, post count, follower count.
async fn list_community_members(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(q): Query<MemberDirectoryQuery>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;

    let page = q.page.unwrap_or(1).max(1);
    let limit: i64 = 30;
    let offset = (page - 1) * limit;
    let sort = q.sort.as_deref().unwrap_or("xp");
    let order_clause = match sort {
        "recent" => "cp.created_at DESC",
        "posts" => "cp.post_count DESC, cp.xp_total DESC",
        _ => "cp.xp_total DESC, cp.post_count DESC",
    };
    let q_text = q.q.as_deref().unwrap_or("").trim();

    use sqlx::Row;
    // First pass: get the user_ids + community-side stats. The display_name
    // filter happens in a second step against the user bridge so we don't
    // need to denormalise names into the community DB.
    let candidate_rows = sqlx::query(&format!(
        r#"
        SELECT cp.user_id, cp.xp_total, cp.level, cp.post_count, cp.follower_count, cp.created_at
        FROM community_profiles cp
        WHERE cp.is_shadowbanned = FALSE AND cp.is_community_banned = FALSE
        ORDER BY {order}
        LIMIT $1 OFFSET $2
        "#,
        order = order_clause,
    ))
    .bind(if q_text.is_empty() { limit } else { 200 }) // overfetch when filtering
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let user_ids: Vec<Uuid> = candidate_rows
        .iter()
        .filter_map(|r| r.try_get::<Uuid, _>("user_id").ok())
        .collect();
    let info_map = crate::community::user_bridge::get_users_info_batch(
        &state.db,
        state.redis.as_ref(),
        &user_ids,
    )
    .await
    .unwrap_or_default();

    let q_lower = q_text.to_lowercase();
    let mut members: Vec<serde_json::Value> = Vec::with_capacity(candidate_rows.len());
    for row in candidate_rows {
        let uid: Uuid = row.try_get("user_id").unwrap_or_default();
        let info = info_map.get(&uid);
        let display_name = info
            .map(|i| i.display_name.clone())
            .unwrap_or_else(|| "Anonymous Investor".to_string());
        if !q_lower.is_empty() && !display_name.to_lowercase().contains(&q_lower) {
            continue;
        }
        members.push(serde_json::json!({
            "user_id": uid,
            "display_name": display_name,
            "avatar_url": info.and_then(|i| i.avatar_url.clone()),
            "xp_total": row.try_get::<i64, _>("xp_total").unwrap_or(0),
            "level": row.try_get::<i32, _>("level").unwrap_or(1),
            "post_count": row.try_get::<i32, _>("post_count").unwrap_or(0),
            "follower_count": row.try_get::<i32, _>("follower_count").unwrap_or(0),
        }));
        if members.len() as i64 >= limit {
            break;
        }
    }

    Ok(Json(serde_json::json!({
        "members": members,
        "page": page,
        "page_size": limit,
    })))
}

/// `$` ticker autocomplete — published assets by name/slug prefix.
/// Reads from the Core DB (assets table); community DB pool not used.
async fn suggest_assets(
    State(state): State<AppState>,
    Query(q): Query<SuggestQuery>,
) -> Result<impl IntoResponse, AppError> {
    let prefix = q.q.trim().trim_start_matches('$');
    if prefix.is_empty() {
        return Ok(Json(serde_json::json!({ "assets": [] })));
    }
    let pattern = format!("%{}%", prefix.to_lowercase());
    let rows = sqlx::query_as::<_, (Uuid, String, String, String)>(
        r#"
        SELECT id, slug, title, asset_type
        FROM assets
        WHERE published = TRUE
          AND (LOWER(title) LIKE $1 OR LOWER(slug) LIKE $1)
        ORDER BY
          CASE WHEN LOWER(slug) LIKE $1 THEN 0 ELSE 1 END,
          title ASC
        LIMIT 10
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;
    let assets: Vec<serde_json::Value> = rows
        .iter()
        .map(|(id, slug, title, asset_type)| {
            serde_json::json!({
                "asset_id": id,
                "slug": slug,
                "title": title,
                "asset_type": asset_type,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "assets": assets })))
}

/// Get posts by a specific hashtag
#[derive(Deserialize)]
pub struct HashtagPostsQuery {
    pub page: Option<i64>,
}

// Phase 3 task 24: shared helper so both the JSON endpoint and the new SSR
// /community/hashtag/:tag page hit the same query.
pub async fn get_hashtag_feed_data(
    state: &AppState,
    tag: &str,
    page: Option<i64>,
) -> Result<(String, Vec<models::PostDisplay>), AppError> {
    let c_pool = get_community_pool(state)?;
    let clean_tag = tag.to_lowercase().trim_start_matches('#').to_string();
    let limit: i64 = 20;
    let offset = (page.unwrap_or(1).max(1) - 1) * limit;

    let posts = sqlx::query_as::<_, models::Post>(
        r#"
        SELECT p.*
        FROM posts p
        JOIN post_hashtags ph ON p.id = ph.post_id
        JOIN hashtags h ON ph.hashtag_id = h.id
        WHERE h.tag = $1 AND p.is_hidden = false
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&clean_tag)
    .bind(limit)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;
    let badges = service::get_badges_batch(&c_pool, &user_ids).await?;

    let mut feed = Vec::with_capacity(posts.len());
    for p in posts {
        let author = authors.get(&p.user_id);
        let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();
        let author_name = author
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Anonymous".into());
        feed.push(map_to_post_display(
            &p,
            author_name,
            author.and_then(|a| a.avatar_url.clone()),
            author_badges,
            false,
            false, // hashtag SSR helper has no viewer context yet
        ));
    }
    Ok((clean_tag, feed))
}

/// 14.8.13 — user-facing badge detail. Returns badge metadata, holder
/// count, and a short list of recent holders for SSR + JSON consumers.
pub async fn get_badge_detail_data(
    state: &AppState,
    badge_id: Uuid,
) -> Result<(serde_json::Value, Vec<serde_json::Value>), AppError> {
    let c_pool = get_community_pool(state)?;

    let badge = sqlx::query_as::<_, BadgeRow>(
        "SELECT id, code, name, description, icon, display_order, created_at
         FROM badges WHERE id = $1",
    )
    .bind(badge_id)
    .fetch_optional(&c_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Badge not found".into()))?;

    let holder_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_badges WHERE badge_id = $1")
            .bind(badge_id)
            .fetch_one(&c_pool)
            .await
            .unwrap_or(0);

    let recent_awards = sqlx::query_as::<_, BadgeAwardRow>(
        "SELECT badge_id, user_id, earned_at FROM user_badges
         WHERE badge_id = $1 ORDER BY earned_at DESC LIMIT 12",
    )
    .bind(badge_id)
    .fetch_all(&c_pool)
    .await
    .unwrap_or_default();

    let holder_ids: Vec<Uuid> = recent_awards.iter().map(|a| a.user_id).collect();
    let authors = if holder_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &holder_ids)
            .await
            .unwrap_or_default()
    };

    let recent_holders: Vec<serde_json::Value> = recent_awards
        .into_iter()
        .map(|a| {
            let info = authors.get(&a.user_id);
            serde_json::json!({
                "user_id": a.user_id,
                "display_name": info
                    .map(|i| i.display_name.clone())
                    .unwrap_or_else(|| "Anonymous".into()),
                "avatar_url": info.and_then(|i| i.avatar_url.clone()),
                "earned_at": a.earned_at,
            })
        })
        .collect();

    let badge_json = serde_json::json!({
        "id": badge.id,
        "code": badge.code,
        "name": badge.name,
        "description": badge.description,
        "icon": badge.icon,
        "display_order": badge.display_order,
        "created_at": badge.created_at,
        "holder_count": holder_count,
    });

    Ok((badge_json, recent_holders))
}

async fn get_badge_detail(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(badge_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let (badge, recent_holders) = get_badge_detail_data(&state, badge_id).await?;
    Ok(Json(serde_json::json!({
        "badge": badge,
        "recent_holders": recent_holders,
    })))
}

async fn get_posts_by_hashtag(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Query(query): Query<HashtagPostsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    let limit: i64 = 20;
    let offset = (query.page.unwrap_or(1).max(1) - 1) * limit;

    let clean_tag = tag.to_lowercase().trim_start_matches('#').to_string();

    let posts = sqlx::query_as::<_, models::Post>(
        r#"
        SELECT p.*
        FROM posts p
        JOIN post_hashtags ph ON p.id = ph.post_id
        JOIN hashtags h ON ph.hashtag_id = h.id
        WHERE h.tag = $1 AND p.is_hidden = false
        ORDER BY p.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&clean_tag)
    .bind(limit)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors =
        user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;
    let badges = service::get_badges_batch(&c_pool, &user_ids).await?;

    let mut feed = Vec::with_capacity(posts.len());
    for p in posts {
        let author = authors.get(&p.user_id);
        let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();

        let author_name = author
            .map(|a| a.display_name.clone())
            .unwrap_or_else(|| "Anonymous".into());

        feed.push(map_to_post_display(
            &p,
            author_name,
            author.and_then(|a| a.avatar_url.clone()),
            author_badges,
            false,
            false,
        ));
    }

    Ok(Json(serde_json::json!({
        "tag": clean_tag,
        "posts": feed,
    })))
}

// ═══════════════════════════════════════════════════════════════════════
// Multi-circle handlers (2026-05-16 rework). Discover / search / by-slug /
// my-circles / moderator-promote / ban / unban.
// ═══════════════════════════════════════════════════════════════════════

async fn discover_circles_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let _ = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let payload = crate::community::circles::discover_circles(&c_pool).await?;

    // Gather all circle ids surfaced in any discover section, then
    // batch-hydrate member previews so each card can render face avatars
    // without a follow-up roundtrip.
    let mut ids: Vec<Uuid> = Vec::with_capacity(
        payload.featured.len()
            + payload.trending.len()
            + payload.new.len()
            + payload.public.len()
            + payload.private.len()
            + payload.asset.len()
            + payload.holder_only.len()
            + payload.official.len()
            + payload.kyc_gated.len(),
    );
    for row in payload
        .featured
        .iter()
        .chain(payload.trending.iter())
        .chain(payload.new.iter())
        .chain(payload.public.iter())
        .chain(payload.private.iter())
        .chain(payload.asset.iter())
        .chain(payload.holder_only.iter())
        .chain(payload.official.iter())
        .chain(payload.kyc_gated.iter())
    {
        ids.push(row.id);
    }
    ids.sort();
    ids.dedup();
    let previews = crate::community::circles::get_member_previews(
        &c_pool,
        &state.db,
        state.redis.as_ref(),
        &ids,
        5,
    )
    .await;

    // Augment each row with its member_preview slice.
    let attach = |rows: &Vec<crate::community::circles::CircleCardRow>| -> Vec<serde_json::Value> {
        rows.iter()
            .map(|c| {
                let preview = previews.get(&c.id).cloned().unwrap_or_default();
                let mut v = serde_json::to_value(c).unwrap_or(serde_json::json!({}));
                if let serde_json::Value::Object(ref mut map) = v {
                    map.insert("member_preview".into(), serde_json::json!(preview));
                }
                v
            })
            .collect()
    };

    Ok(Json(serde_json::json!({
        "featured":   attach(&payload.featured),
        "trending":   attach(&payload.trending),
        "new":        attach(&payload.new),
        "public":     attach(&payload.public),
        "private":    attach(&payload.private),
        "asset":      attach(&payload.asset),
        "holder_only": attach(&payload.holder_only),
        "official":   attach(&payload.official),
        "kyc_gated":  attach(&payload.kyc_gated),
    })))
}

#[derive(Deserialize)]
struct SearchCirclesQuery {
    q: Option<String>,
    page: Option<i64>,
    per_page: Option<i64>,
}

async fn search_circles_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<SearchCirclesQuery>,
) -> Result<impl IntoResponse, AppError> {
    let _ = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let query = q.q.unwrap_or_default();
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(10).clamp(1, 50);
    let (rows, total) =
        crate::community::circles::search_circles(&c_pool, &query, page, per_page).await?;
    let ids: Vec<Uuid> = rows.iter().map(|c| c.id).collect();
    let previews = crate::community::circles::get_member_previews(
        &c_pool,
        &state.db,
        state.redis.as_ref(),
        &ids,
        5,
    )
    .await;
    let results: Vec<serde_json::Value> = rows
        .iter()
        .map(|c| {
            let preview = previews.get(&c.id).cloned().unwrap_or_default();
            let mut v = serde_json::to_value(c).unwrap_or(serde_json::json!({}));
            if let serde_json::Value::Object(ref mut map) = v {
                map.insert("member_preview".into(), serde_json::json!(preview));
            }
            v
        })
        .collect();
    Ok(Json(serde_json::json!({
        "results": results,
        "page": page,
        "per_page": per_page,
        "total": total,
        "total_pages": ((total as f64) / (per_page as f64)).ceil() as i64,
    })))
}

async fn get_circle_by_slug_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let viewer = middleware::get_current_user(&jar, &state.db)
        .await
        .map(|u| u.id);
    let c_pool = get_community_pool(&state)?;
    let (circle, role) =
        crate::community::circles::get_circle_by_slug(&c_pool, &slug, viewer).await?;
    Ok(Json(serde_json::json!({
        "circle": circle,
        "my_role": role,
    })))
}

async fn list_my_circles_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let rows = crate::community::circles::list_my_circles(&c_pool, user.id).await?;
    let payload: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(c, role)| {
            serde_json::json!({
                "circle": c,
                "role": role,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "circles": payload })))
}

#[derive(Deserialize)]
struct ModeratorReq {
    moderator: bool,
}

async fn set_moderator_handler(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<ModeratorReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let actor = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, actor.id, "moderator").await?;
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::set_member_moderator(
        &c_pool,
        actor.id,
        circle_id,
        user_id,
        payload.moderator,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "role": if payload.moderator { "moderator" } else { "member" },
    })))
}

#[derive(Deserialize)]
struct BanReq {
    user_id: Uuid,
    reason: Option<String>,
}

async fn ban_member_handler(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<BanReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let actor = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    require_community_rate_limit(&state, actor.id, "ban").await?;
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::ban_member(
        &c_pool,
        actor.id,
        circle_id,
        payload.user_id,
        payload.reason,
    )
    .await?;
    Ok(Json(serde_json::json!({"success": true})))
}

async fn unban_member_handler(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((circle_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let actor = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::unban_member(&c_pool, actor.id, circle_id, user_id).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

/// GET /api/community/circles/:id/bans — list active bans for the circle.
/// Owner/admin/moderator only (mirrors the visibility on the settings page).
async fn list_circle_bans_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let actor = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    let bans = crate::community::circles::list_circle_bans(&c_pool, actor.id, circle_id).await?;
    Ok(Json(serde_json::json!({ "bans": bans })))
}

#[derive(Deserialize)]
struct ProfileBannerReq {
    /// Set to `None`/`null` or omit to clear the banner.
    banner_url: Option<String>,
}

/// PUT /api/community/profile/banner — save the Facebook-style cover-photo URL
/// to the caller's community profile. The actual image upload happens via
/// `/api/upload/post-image` (returns a URL); this endpoint just persists the
/// reference. Pass `banner_url: null` (or an empty string) to clear.
async fn set_profile_banner_handler(
    jar: CookieJar,
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<ProfileBannerReq>,
) -> Result<impl IntoResponse, AppError> {
    require_csrf_header(&headers, &jar)?;
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;
    let c_pool = get_community_pool(&state)?;
    // Trim + length-cap (1024 chars matches DB CHECK constraint).
    let cleaned: Option<String> = payload.banner_url.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else if t.len() > 1024 {
            None
        } else {
            Some(t)
        }
    });
    crate::community::circles::set_profile_banner(&c_pool, user.id, cleaned.as_deref()).await?;
    Ok(Json(serde_json::json!({
        "success": true,
        "banner_url": cleaned,
    })))
}
