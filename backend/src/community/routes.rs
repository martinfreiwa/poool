use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
    routing::{get, post},
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
}

#[derive(Deserialize)]
pub struct CreateAnnouncementReq {
    pub content: String,
    pub category: String,
    pub image_urls: Option<Vec<String>>,
    pub is_pinned: Option<bool>,
}

#[derive(Deserialize)]
pub struct ToggleReactionReq {
    pub reaction_type: String,
}

#[derive(Deserialize)]
pub struct CreateCommentReq {
    pub content: String,
}

/// Helper to assert the community database is available
fn get_community_pool(state: &AppState) -> Result<sqlx::PgPool, AppError> {
    state.community_db.clone().ok_or_else(|| {
        tracing::error!("Community DB is not configured, but a community route was hit.");
        AppError::Internal("Community Database is offline".to_string())
    })
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

async fn get_feed(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<FeedQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Auth check
    let _user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    let limit = 20;
    let offset = (query.page.unwrap_or(1).max(1) - 1) * limit;

    let posts = service::get_announcement_feed(&c_pool, query.category, limit, offset).await?;

    // Build user_ids list for batch fetching
    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors = user_bridge::get_users_info_batch(&state.db, &user_ids).await?;

    // Construct Display views
    let mut feed = Vec::with_capacity(posts.len());
    for p in posts {
        let author = authors.get(&p.user_id);
        feed.push(PostDisplay {
            id: p.id,
            author_name: author
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Anonymous".into()),
            author_id: p.user_id,
            author_avatar: author.and_then(|a| a.avatar_url.clone()),
            post_type: p.post_type.clone(),
            content: p.content_sanitized.unwrap_or(p.content),
            asset_id: p.asset_id,
            image_urls: p.image_urls.unwrap_or_default(),
            reaction_count: p.reaction_count,
            comment_count: p.comment_count,
            is_hidden: p.is_hidden,
            is_pinned: p.is_pinned,
            disclaimer_shown: p.disclaimer_shown,
            created_at: p.created_at,
        });
    }

    Ok(Json(feed))
}

async fn create_announcement(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateAnnouncementReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = admin.user;

    let c_pool = get_community_pool(&state)?;

    let clean_html = validation::sanitize_html_basic(&payload.content);

    let post_id = service::create_announcement(
        &c_pool,
        user.id,
        payload.content,
        clean_html,
        payload.category,
        payload.image_urls,
        payload.is_pinned.unwrap_or(false),
    )
    .await?;

    Ok(Json(serde_json::json!({ "id": post_id })))
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

    let added = service::toggle_reaction(&c_pool, post_id, user.id, payload.reaction_type).await?;

    Ok(Json(serde_json::json!({ "added": added })))
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

    let comment_id =
        service::create_comment(&c_pool, post_id, user.id, payload.content, clean_html).await?;

    Ok(Json(serde_json::json!({ "id": comment_id })))
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

    let comments = sqlx::query_as::<_, Comment>(
        "SELECT * FROM comments WHERE post_id = $1 AND is_hidden = false ORDER BY created_at ASC",
    )
    .bind(post_id)
    .fetch_all(&c_pool)
    .await?;

    // Batch map authors
    let user_ids: Vec<Uuid> = comments.iter().map(|c| c.user_id).collect();
    let authors = user_bridge::get_users_info_batch(&state.db, &user_ids).await?;

    let mut result = Vec::with_capacity(comments.len());
    for c in comments {
        let author = authors.get(&c.user_id);
        result.push(serde_json::json!({
            "id": c.id,
            "post_id": c.post_id,
            "author_name": author.map(|a| a.display_name.clone()).unwrap_or_else(|| "Anonymous".into()),
            "author_avatar": author.and_then(|a| a.avatar_url.clone()),
            "content": c.content,
            "helpful_count": c.helpful_count,
            "created_at": c.created_at,
        }));
    }

    Ok(Json(result))
}

async fn get_admin_stats(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    let total_posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(&c_pool)
        .await
        .unwrap_or((0,));
    
    let total_comments: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM comments")
        .fetch_one(&c_pool)
        .await
        .unwrap_or((0,));
        
    let total_reactions: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reactions")
        .fetch_one(&c_pool)
        .await
        .unwrap_or((0,));

    let active_profiles: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM community_profiles")
        .fetch_one(&c_pool)
        .await
        .unwrap_or((0,));

    Ok(Json(serde_json::json!({
        "total_posts": total_posts.0,
        "total_comments": total_comments.0,
        "total_reactions": total_reactions.0,
        "active_profiles": active_profiles.0,
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

    let c_pool = get_community_pool(&state)?;

    // We can assume high_level = false for now until M4 XP system is in place
    let is_high_level_user = false;

    let post_id = service::create_user_post(&c_pool, state.redis.as_ref(), user.id, payload, is_high_level_user).await?;

    Ok(Json(serde_json::json!({ "id": post_id })))
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

    service::update_user_post(&c_pool, post_id, user.id, payload.content, is_high_level_user).await?;

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

    let report_id = service::create_content_report(&c_pool, post_id, user.id, payload.reason).await?;

    Ok(Json(serde_json::json!({ "id": report_id })))
}

async fn get_reports(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
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
        let posts: Vec<models::Post> = sqlx::query_as(
            "SELECT * FROM posts WHERE id = ANY($1)"
        )
        .bind(&p_ids)
        .fetch_all(&c_pool)
        .await?;

        for p in posts {
            user_ids.insert(p.user_id);
            posts_map.insert(p.id, p);
        }
    }

    let user_ids_vec: Vec<Uuid> = user_ids.into_iter().collect();
    let authors = user_bridge::get_users_info_batch(&state.db, &user_ids_vec).await?;

    let mut response = Vec::with_capacity(pending_reports.len());

    for r in pending_reports {
        let reporter = authors.get(&r.reporter_id);
        
        let (post_author_id, post_author_name, post_content) = if let Some(post) = posts_map.get(&r.post_id) {
            let p_author = authors.get(&post.user_id);
            (
                post.user_id,
                p_author.map(|a| a.display_name.clone()).unwrap_or_else(|| "Unknown".into()),
                post.content_sanitized.clone().unwrap_or(post.content.clone())
            )
        } else {
            (Uuid::nil(), "Deleted Post".into(), "[Content Unavailable]".into())
        };

        response.push(models::AdminReportDisplay {
            id: r.id,
            post_id: r.post_id,
            reporter_id: r.reporter_id,
            reporter_name: reporter.map(|a| a.display_name.clone()).unwrap_or_else(|| "Unknown".into()),
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
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    Json(payload): Json<models::AdminReportActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    service::action_on_report(&c_pool, report_id, &payload.action, payload.admin_notes).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Serialize)]
pub struct TrendingAssetDisplay {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub post_count: i64,
}

async fn get_trending_assets(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let trending = service::get_trending_assets(&c_pool).await?;

    if trending.is_empty() {
        return Ok(Json(Vec::<TrendingAssetDisplay>::new()));
    }

    let asset_ids: Vec<Uuid> = trending.iter().map(|(id, _)| *id).collect();

    let assets: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, name, symbol FROM assets WHERE id = ANY($1)"
    )
    .bind(&asset_ids)
    .fetch_all(&state.db)
    .await?;

    let mut asset_map = std::collections::HashMap::new();
    for a in assets {
        asset_map.insert(a.0, (a.1, a.2));
    }

    let mut result = Vec::new();
    for (id, count) in trending {
        if let Some((name, symbol)) = asset_map.get(&id) {
            result.push(TrendingAssetDisplay {
                id,
                name: name.clone(),
                symbol: symbol.clone(),
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
    pub hidden_reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn admin_get_posts(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    let posts: Vec<models::Post> = sqlx::query_as("SELECT * FROM posts ORDER BY created_at DESC")
        .fetch_all(&c_pool)
        .await?;

    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors = user_bridge::get_users_info_batch(&state.db, &user_ids).await?;

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
            hidden_reason: p.hidden_reason.clone(),
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
    _admin: crate::admin::extractors::AdminUser,
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

    Ok(Json(serde_json::json!({ "success": true })))
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
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn admin_get_users(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
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

    let core_users = user_bridge::get_users_info_batch(&state.db, &user_ids).await?;

    let mut result = Vec::new();
    for row in rows {
        let u_id: Uuid = row.try_get("user_id")?;
        let is_community_banned: bool = row.try_get("is_community_banned")?;
        let ban_reason: Option<String> = row.try_get("ban_reason")?;
        let warning_count: i32 = row.try_get("warning_count")?;
        let post_count: i32 = row.try_get("post_count")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;

        let user_info = core_users.get(&u_id);
        
        result.push(AdminUserDisplay {
            user_id: u_id,
            display_name: user_info.map(|u| u.display_name.clone()).unwrap_or_else(|| "Unknown".into()),
            avatar_url: user_info.and_then(|u| u.avatar_url.clone()),
            is_community_banned,
            ban_reason,
            warning_count,
            post_count,
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
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<BanUserPayload>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE community_profiles SET is_community_banned = $1, ban_reason = $2 WHERE user_id = $3")
        .bind(payload.is_banned)
        .bind(&payload.reason)
        .bind(user_id)
        .execute(&c_pool)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ─── Router Configuration ────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        // Feed & Filter
        .route("/api/community/feed", get(get_feed))
        .route("/api/community/trending-assets", get(get_trending_assets))
        // Announcements
        .route(
            "/api/admin/community/announcements",
            post(create_announcement),
        )
        // User Posts
        .route("/api/community/posts", post(create_user_post))
        .route("/api/community/posts/:id", axum::routing::put(update_user_post).delete(delete_user_post))
        .route("/api/community/posts/:id/report", post(create_content_report))
        // Reactions
        .route("/api/community/posts/:id/reactions", post(toggle_reaction))
        // Comments
        .route(
            "/api/community/posts/:id/comments",
            get(get_comments).post(create_comment),
        )
        // Admin Stats & Moderation
        .route("/api/admin/community/stats", get(get_admin_stats))
        .route("/api/admin/community/reports", get(get_reports))
        .route("/api/admin/community/reports/:id/action", post(take_report_action))
        .route("/api/admin/community/posts", get(admin_get_posts))
        .route("/api/admin/community/posts/:id/hide", post(admin_hide_post))
        .route("/api/admin/community/users", get(admin_get_users))
        .route("/api/admin/community/users/:id/ban", post(admin_toggle_ban_user))
}
