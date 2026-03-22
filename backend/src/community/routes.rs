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
    community::{models::*, service, user_bridge, validation},
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
    let user = middleware::get_current_user(&jar, &state.db)
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
        feed.push(AnnouncementDisplay {
            id: p.id,
            author_name: author
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Anonymous".into()),
            author_avatar: author.and_then(|a| a.avatar_url.clone()),
            category: "announcement".to_string(), // M1 MVP limitation logic
            content: p.content,
            image_urls: p.image_urls.unwrap_or_default(),
            reaction_count: p.reaction_count,
            comment_count: p.comment_count,
            is_pinned: p.is_pinned,
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

// ─── Router Configuration ────────────────────────────────────────────────────

pub fn router() -> Router<AppState> {
    Router::new()
        // Feed & Filter
        .route("/api/community/feed", get(get_feed))
        // Announcements
        .route(
            "/api/admin/community/announcements",
            post(create_announcement),
        )
        // Reactions
        .route("/api/community/posts/:id/reactions", post(toggle_reaction))
        // Comments
        .route(
            "/api/community/posts/:id/comments",
            get(get_comments).post(create_comment),
        )
        // Admin Stats
        .route("/api/admin/community/stats", get(get_admin_stats))
}
