use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
    routing::{get, post, put, delete},
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

/// FIX-F7: Check if user is community-banned before allowing write operations
async fn check_user_not_banned(pool: &sqlx::PgPool, user_id: Uuid) -> Result<(), AppError> {
    let is_banned: Option<bool> = sqlx::query_scalar(
        "SELECT is_community_banned FROM community_profiles WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if is_banned == Some(true) {
        return Err(AppError::Forbidden(
            "Your community access has been suspended. Contact support for more information.".to_string()
        ));
    }
    Ok(())
}

// ─── Route Handlers ──────────────────────────────────────────────────────────

async fn get_feed(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<FeedQuery>,
) -> Result<impl IntoResponse, AppError> {
    // Auth check
    let user = middleware::get_current_user(&jar, &state.db).await;
    
    // Determine if we need to enforce auth based on query
    if query.feed_mode.as_deref() == Some("following") && user.is_none() {
         return Err(AppError::Unauthorized("You must be logged in to view your following feed.".into()));
    }

    let c_pool = get_community_pool(&state)?;

    let limit = 20;
    let offset = (query.page.unwrap_or(1).max(1) - 1) * limit;

    let only_following_user_id = if query.feed_mode.as_deref() == Some("following") {
        user.as_ref().map(|u| u.id)
    } else {
        None
    };

    let posts = service::get_community_feed(&c_pool, query.category, only_following_user_id, limit, offset).await?;

    // Build user_ids list for batch fetching
    let user_ids: Vec<Uuid> = posts.iter().map(|p| p.user_id).collect();
    let authors = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;
    let badges = service::get_badges_batch(&c_pool, &user_ids).await?;

    // Construct Display views
    let mut feed = Vec::with_capacity(posts.len());
    for p in posts {
        let author = authors.get(&p.user_id);
        let author_badges = badges.get(&p.user_id).cloned().unwrap_or_default();

        feed.push(PostDisplay {
            id: p.id,
            author_name: author
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Anonymous".into()),
            author_id: p.user_id,
            author_avatar: author.and_then(|a| a.avatar_url.clone()),
            author_badges,
            post_type: p.post_type.clone(),
            content: p.content_sanitized.unwrap_or(p.content),
            asset_id: p.asset_id,
            image_urls: p.image_urls.unwrap_or_default(),
            reaction_count: p.reaction_count,
            comment_count: p.comment_count,
            is_hidden: p.is_hidden,
            is_pinned: p.is_pinned,
            disclaimer_shown: p.disclaimer_shown,
            verified_owner: false, // FIX-F4: Computed per-post; feed doesn't have this context yet
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

    // FIX-F7: Check ban before allowing reaction
    check_user_not_banned(&c_pool, user.id).await?;

    let added = service::toggle_reaction(&c_pool, post_id, user.id, payload.reaction_type).await?;

    // Award XP only when reaction is added (not removed)
    if added {
        let _ = crate::community::xp::award_xp(&c_pool, user.id, "reaction_given", Some("Reacted to a post"), None).await;
    }

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

    // FIX-F7: Check ban before allowing comment
    check_user_not_banned(&c_pool, user.id).await?;

    // FIX-CRL: Comment rate limiting (30 comments/hour via Redis)
    if let Some(redis_pool) = state.redis.as_ref() {
        use redis::AsyncCommands;
        if let Ok(mut conn) = redis_pool.get().await {
            let rl_key = format!("community:ratelimit:comments:{}", user.id);
            let count: Option<i64> = conn.get(&rl_key).await.unwrap_or(None);
            if let Some(c) = count {
                if c >= 30 {
                    return Err(AppError::BadRequest("Rate limit exceeded: Max 30 comments per hour.".into()));
                }
            }
            let _: () = conn.incr(&rl_key, 1).await.unwrap_or(());
            let _: () = conn.expire(&rl_key, 3600).await.unwrap_or(());
        }
    }

    let comment_id =
        service::create_comment(&c_pool, post_id, user.id, payload.content, clean_html).await?;

    // Award XP for comment
    let _ = crate::community::xp::award_xp(&c_pool, user.id, "comment_created", Some("Posted a comment"), None).await;

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
    let authors = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

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

    // FIX-F7: Check ban before allowing post creation
    check_user_not_banned(&c_pool, user.id).await?;

    // We can assume high_level = false for now until M4 XP system is in place
    let is_high_level_user = false;

    // M3-BE.7 Dynamic Asset-Owner Tags Check — boolean flag, NOT HTML injection (FIX-F4)
    let mut verified_owner = false;

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
            "SELECT a.name FROM investments i JOIN assets a ON i.asset_id = a.id WHERE i.user_id = $1 AND i.tokens_owned > 0"
        )
        .bind(user.id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        for name in owned_assets {
            if payload.content.to_lowercase().contains(&name.to_lowercase()) {
                verified_owner = true;
                break;
            }
        }
    }

    let post_id = service::create_user_post(&c_pool, state.redis.as_ref(), user.id, payload, is_high_level_user).await?;

    // Award XP for post creation
    let _ = crate::community::xp::award_xp(&c_pool, user.id, "post_created", Some("Created a post"), None).await;

    Ok(Json(serde_json::json!({ "id": post_id, "verified_owner": verified_owner })))
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
    let authors = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids_vec).await?;

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
    let authors = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

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
    let comments: Vec<models::Comment> = sqlx::query_as("SELECT * FROM comments WHERE post_id = $1 ORDER BY created_at ASC")
        .bind(post_id)
        .fetch_all(&c_pool)
        .await?;

    // 3. Fetch Reactions
    let reactions: Vec<models::Reaction> = sqlx::query_as("SELECT * FROM reactions WHERE post_id = $1 ORDER BY created_at DESC")
        .bind(post_id)
        .fetch_all(&c_pool)
        .await?;

    // 4. Fetch Reports
    let reports: Vec<models::ContentReport> = sqlx::query_as(
        "SELECT * FROM content_reports WHERE post_id = $1 ORDER BY created_at DESC"
    )
    .bind(post_id)
    .fetch_all(&c_pool)
    .await?;

    // Collect all unique user IDs to fetch names
    let mut user_ids = std::collections::HashSet::new();
    user_ids.insert(p.user_id);
    for c in &comments { user_ids.insert(c.user_id); }
    for r in &reactions { user_ids.insert(r.user_id); }
    for rep in &reports { user_ids.insert(rep.reporter_id); }

    let user_ids_vec: Vec<Uuid> = user_ids.into_iter().collect();
    let authors = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids_vec).await?;

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
        hidden_reason: p.hidden_reason.clone(),
        created_at: p.created_at,
    };

    // Format Comments
    let mut comments_display = Vec::new();
    for c in comments {
        let name = authors.get(&c.user_id).map(|a| a.display_name.clone()).unwrap_or_else(|| "Unknown".into());
        comments_display.push(serde_json::json!({
            "id": c.id,
            "user_id": c.user_id,
            "author_name": name,
            "content": c.content_sanitized.unwrap_or(c.content),
            "is_hidden": c.is_hidden,
            "created_at": c.created_at,
        }));
    }

    // Format Reactions
    let mut reactions_display = Vec::new();
    for r in reactions {
        let name = authors.get(&r.user_id).map(|a| a.display_name.clone()).unwrap_or_else(|| "Unknown".into());
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
        let name = authors.get(&rep.reporter_id).map(|a| a.display_name.clone()).unwrap_or_else(|| "Unknown".into());
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

    let core_users = user_bridge::get_users_info_batch(&state.db, state.redis.as_ref(), &user_ids).await?;

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
        .route("/api/admin/community/posts/:id", get(admin_get_post_detail))
        .route("/api/admin/community/posts/:id/hide", post(admin_hide_post))
        .route("/api/admin/community/users", get(admin_get_users))
        .route("/api/admin/community/users/:id/ban", post(admin_toggle_ban_user))
        // Social Layer
        .route("/api/community/profile/me", get(get_profile_me))
        .route("/api/community/profile", put(update_profile))
        .route("/api/community/profile/:id", get(get_profile))
        .route("/api/community/follow/:id", post(follow_user))
        .route("/api/community/follow/:id", delete(unfollow_user))
        // XP System (M4)
        .route("/api/community/xp", get(get_xp_summary))
        .route("/api/community/xp/history", get(get_xp_history))
        // Circles (M4)
        .route("/api/community/circles", post(create_circle))
        .route("/api/community/circles/me", get(get_my_circle))
        .route("/api/community/circles/leaderboard", get(get_circle_leaderboard))
        .route("/api/community/circles/:id", get(get_circle_detail))
        .route("/api/community/circles/:id", put(update_circle))
        .route("/api/community/circles/:id/members", get(get_circle_members))
        .route("/api/community/circles/:id/join", post(join_circle))
        .route("/api/community/circles/leave", post(leave_circle))
        .route("/api/community/circles/:id/invite", post(send_circle_invite))
        .route("/api/community/circles/:id/kick/:user_id", post(kick_circle_member))
        .route("/api/community/invites", get(get_my_invites))
        .route("/api/community/invites/:id/accept", post(accept_invite))
        .route("/api/community/invites/:id/decline", post(decline_invite))
        // Expert AMAs (M5)
        .route("/api/community/amas", get(list_amas))
        .route("/api/community/amas/:id", get(get_ama_detail))
        .route("/api/community/amas/:id/questions", post(submit_ama_question))
        .route("/api/community/amas/:id/questions/:qid/upvote", post(toggle_ama_upvote))
        // Admin AMAs
        .route("/api/admin/community/amas", get(admin_list_amas).post(admin_create_ama))
        .route("/api/admin/community/amas/:id/status", post(admin_update_ama_status))
        .route("/api/admin/community/amas/:id/questions/:qid/answer", post(admin_answer_question))
        .route("/api/admin/community/amas/:id/questions/:qid/feature", post(admin_toggle_featured))
        // Admin Badges (M3-ADMIN)
        .route("/api/admin/community/badges", get(admin_list_badges).post(admin_create_badge))
        .route("/api/admin/community/badges/:id", put(admin_update_badge))
        .route("/api/admin/community/users/:id/badge", post(admin_grant_badge))
        .route("/api/admin/community/users/:id/badge/:badge_id", delete(admin_revoke_badge))
        // Admin User Detail (M3-ADMIN)
        .route("/api/admin/community/users/:id/detail", get(admin_get_user_detail))
}

// ─── Social Handlers ─────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct UpdateProfileReq {
    pub bio: Option<String>,
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

    Ok(Json(serde_json::json!({
        "user_id": profile.user_id,
        "bio": profile.bio,
        "post_count": profile.post_count,
        "follower_count": profile.follower_count,
        "following_count": profile.following_count,
        "badges": profile.badges,
    })))
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
    crate::community::service::update_user_profile(&c_pool, user.id, payload.bio).await?;

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
    let bridge_info = crate::community::user_bridge::get_user_info(&state.db, state.redis.as_ref(), profile_id).await.unwrap_or_else(|_| {
        crate::community::user_bridge::UserBridgeInfo {
            user_id: profile_id,
            display_name: "Anonymous User".to_string(),
            avatar_url: None,
        }
    });

    let is_following = if let Some(u) = user {
        crate::community::service::is_following(&c_pool, u.id, profile_id).await?
    } else {
        false
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
        "is_following": is_following
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
    let _ = crate::community::xp::award_xp(&c_pool, target_id, "follow_gained", Some("Gained a new follower"), None).await;

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
    let entries = crate::community::xp::get_xp_history(&c_pool, user.id, 20, (page - 1) * 20).await?;
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

    let c_pool = get_community_pool(&state)?;

    // Level gate: Level 2 required to create a circle (M4-BE.10)
    crate::community::xp::check_level_gate(&c_pool, user.id, crate::community::xp::GatedFeature::CreateCircle).await?;

    let circle = crate::community::circles::create_circle(
        &c_pool, user.id, &payload.name, payload.description.as_deref(), payload.emoji.as_deref()
    ).await?;

    // Award XP for creating a circle
    let _ = crate::community::xp::award_xp(&c_pool, user.id, "circle_created", Some("Created a circle"), None).await;

    Ok(Json(circle))
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
            Ok(Json(serde_json::json!({"circle": c, "members": members})))
        },
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
    let circle = crate::community::circles::get_circle(&c_pool, circle_id).await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;
    let members = crate::community::circles::get_circle_members(&c_pool, circle_id).await?;

    Ok(Json(serde_json::json!({"circle": circle, "members": members})))
}

#[derive(Deserialize)]
struct UpdateCircleReq {
    name: Option<String>,
    description: Option<String>,
    emoji: Option<String>,
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
        &c_pool, circle_id, user.id,
        payload.name.as_deref(), payload.description.as_deref(), payload.emoji.as_deref()
    ).await?;
    Ok(Json(circle))
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
    Ok(Json(serde_json::json!({"members": members})))
}

async fn join_circle(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::join_circle(&c_pool, user.id, circle_id).await?;

    // Award XP
    let _ = crate::community::xp::award_xp(&c_pool, user.id, "circle_joined", Some("Joined a circle"), None).await;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn leave_circle(
    jar: CookieJar,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    crate::community::circles::leave_circle(&c_pool, user.id).await?;
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

    let c_pool = get_community_pool(&state)?;

    // Level gate: Level 3 required to invite (M4-BE.10)
    crate::community::xp::check_level_gate(&c_pool, user.id, crate::community::xp::GatedFeature::InviteToCircle).await?;

    let invite = crate::community::circles::send_invite(&c_pool, user.id, payload.invitee_id, circle_id).await?;
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
    crate::community::circles::accept_invite(&c_pool, user.id, invite_id).await?;

    // Award XP
    let _ = crate::community::xp::award_xp(&c_pool, user.id, "circle_invite_accepted", Some("Accepted circle invite"), None).await;

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
        return Err(AppError::BadRequest("Question must be between 10 and 500 characters.".into()));
    }

    let c_pool = get_community_pool(&state)?;
    let question = crate::community::amas::submit_question(&c_pool, ama_id, user.id, q_text).await?;

    // Award XP for submitting a question
    let _ = crate::community::xp::award_xp(&c_pool, user.id, "ama_question", Some("Submitted an AMA question"), Some(10)).await;

    Ok(Json(question))
}

async fn toggle_ama_upvote(
    jar: CookieJar,
    State(state): State<AppState>,
    Path((_, qid)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;
    let added = crate::community::amas::toggle_upvote(&c_pool, qid, user.id).await?;
    Ok(Json(serde_json::json!({"upvoted": added})))
}

// ─── Admin AMA Handlers ─────────────────────────────────────────────

async fn admin_list_amas(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let _user = admin.user;

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
    scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
    status: Option<String>,
}

async fn admin_create_ama(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateAmaReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = admin.user;

    let c_pool = get_community_pool(&state)?;
    let ama = crate::community::amas::create_ama(
        &c_pool,
        user.id,
        &payload.title,
        payload.description.as_deref(),
        &payload.expert_name,
        payload.expert_title.as_deref(),
        payload.expert_avatar_url.as_deref(),
        payload.scheduled_at,
        payload.status.as_deref(),
    ).await?;

    Ok(Json(ama))
}

#[derive(Deserialize)]
struct UpdateAmaStatusReq {
    status: String,
}

async fn admin_update_ama_status(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(ama_id): Path<Uuid>,
    Json(payload): Json<UpdateAmaStatusReq>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    crate::community::amas::update_ama_status(&c_pool, ama_id, &payload.status).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
struct AnswerQuestionReq {
    answer: String,
}

async fn admin_answer_question(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((_, qid)): Path<(Uuid, Uuid)>,
    Json(payload): Json<AnswerQuestionReq>,
) -> Result<impl IntoResponse, AppError> {
    let user = admin.user;

    let c_pool = get_community_pool(&state)?;
    crate::community::amas::answer_question(&c_pool, qid, user.id, &payload.answer).await?;
    Ok(Json(serde_json::json!({"success": true})))
}

async fn admin_toggle_featured(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((_, qid)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let is_featured = crate::community::amas::toggle_featured(&c_pool, qid).await?;
    Ok(Json(serde_json::json!({"is_featured": is_featured})))
}

// ─── Admin Badge Handlers (M3-ADMIN.4) ──────────────────────────────

async fn admin_list_badges(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let _user = admin.user;
    let c_pool = get_community_pool(&state)?;

    let badges = sqlx::query_as::<_, BadgeRow>(
        "SELECT id, code, name, description, icon, display_order, created_at FROM badges ORDER BY display_order ASC"
    )
    .fetch_all(&c_pool)
    .await?;

    // Get usage counts
    let counts: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT badge_id, COUNT(*)::BIGINT FROM user_badges GROUP BY badge_id"
    )
    .fetch_all(&c_pool)
    .await?;

    let count_map: std::collections::HashMap<Uuid, i64> = counts.into_iter().collect();

    let result: Vec<serde_json::Value> = badges.iter().map(|b| {
        serde_json::json!({
            "id": b.id,
            "code": b.code,
            "name": b.name,
            "description": b.description,
            "icon": b.icon,
            "display_order": b.display_order,
            "created_at": b.created_at,
            "users_count": count_map.get(&b.id).copied().unwrap_or(0),
        })
    }).collect();

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
    let _user = admin.user;
    let c_pool = get_community_pool(&state)?;

    let badge = sqlx::query_as::<_, BadgeRow>(
        r#"INSERT INTO badges (code, name, description, icon, display_order)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, code, name, description, icon, display_order, created_at"#
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.icon)
    .bind(payload.display_order.unwrap_or(0))
    .fetch_one(&c_pool)
    .await?;

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
    let _user = admin.user;
    let c_pool = get_community_pool(&state)?;

    sqlx::query(
        r#"UPDATE badges SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            icon = COALESCE($3, icon),
            display_order = COALESCE($4, display_order)
           WHERE id = $5"#
    )
    .bind(payload.name.as_deref())
    .bind(payload.description.as_deref())
    .bind(payload.icon.as_deref())
    .bind(payload.display_order)
    .bind(badge_id)
    .execute(&c_pool)
    .await?;

    Ok(Json(serde_json::json!({"success": true})))
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
    let _user = admin.user;
    let c_pool = get_community_pool(&state)?;

    let badge_id: Uuid = sqlx::query_scalar("SELECT id FROM badges WHERE code = $1")
        .bind(&payload.badge_code)
        .fetch_optional(&c_pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Badge code '{}' not found", payload.badge_code)))?;

    sqlx::query(
        "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
    .bind(user_id)
    .bind(badge_id)
    .execute(&c_pool)
    .await?;

    Ok(Json(serde_json::json!({"success": true, "badge_code": payload.badge_code})))
}

async fn admin_revoke_badge(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((user_id, badge_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    let _user = admin.user;
    let c_pool = get_community_pool(&state)?;

    sqlx::query("DELETE FROM user_badges WHERE user_id = $1 AND badge_id = $2")
        .bind(user_id)
        .bind(badge_id)
        .execute(&c_pool)
        .await?;

    Ok(Json(serde_json::json!({"success": true})))
}

// ─── Admin User Detail Handler (M3-ADMIN.1) ─────────────────────────

async fn admin_get_user_detail(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let _user = admin.user;
    let c_pool = get_community_pool(&state)?;

    // Community profile
    let profile: Option<serde_json::Value> = sqlx::query_scalar(
        r#"SELECT json_build_object(
            'user_id', user_id,
            'display_name', display_name,
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
            'created_at', created_at
        ) FROM community_profiles WHERE user_id = $1"#
    )
    .bind(user_id)
    .fetch_optional(&c_pool)
    .await?;

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
        ) FROM posts WHERE user_id = $1 ORDER BY created_at DESC LIMIT 10"#
    )
    .bind(user_id)
    .fetch_all(&c_pool)
    .await?;

    // XP summary
    let xp_summary = crate::community::xp::get_xp_summary(&c_pool, user_id).await.ok();

    // Core user data (from main DB)
    let core_data: Option<serde_json::Value> = sqlx::query_scalar(
        r#"SELECT json_build_object(
            'id', u.id, 'email', u.email, 'status', u.status,
            'created_at', u.created_at,
            'first_name', p.first_name, 'last_name', p.last_name,
            'avatar_url', u.avatar_url
        ) FROM users u
        LEFT JOIN user_profiles p ON p.user_id = u.id
        WHERE u.id = $1"#
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
