use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Json},
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
    let mut mentions = std::collections::HashSet::new();
    for word in content.split_whitespace() {
        if word.starts_with('@') && word.len() > 1 {
            let mention = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
            if mention.len() > 1 {
                mentions.insert(mention[1..].to_string()); // skip '@'
            }
        }
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
    if let Ok(url_regex) = regex::Regex::new(r"https?://[^\s<]+") {
        if let Some(mat) = url_regex.find(&content) {
            let url = mat.as_str().to_string();

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap_or_default();

            if let Ok(res) = client.get(&url).send().await {
                if let Ok(html) = res.text().await {
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

pub fn map_to_post_display(
    p: &models::Post,
    author_name: String,
    author_avatar: Option<String>,
    author_badges: Vec<String>,
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
    let re = regex::Regex::new(r"(#[\w\u00C0-\u024F]+|@[\w\u00C0-\u024F_-]+)").unwrap();
    let rendered_content = if p.post_type == "announcement" {
        raw_content.clone()
    } else {
        re.replace_all(&raw_content, |caps: &regex::Captures| {
            let matched = &caps[0];
            if matched.starts_with('#') {
                let tag = matched[1..].to_lowercase();
                format!("<span class='hashtag-tag' style='color: var(--btn-primary-bg, #0000FF); font-weight: 600; cursor: pointer; transition: opacity 0.2s;' hx-get='/community/partials/feed/list?hashtag={}' hx-target='#community-feed-container'>{}</span>", tag, matched)
            } else {
                let user = &matched[1..];
                format!("<span class='mention-tag' style='color: #7F56D9; font-weight: 600; cursor: pointer; transition: opacity 0.2s;' hx-get='/community/partials/feed/list?mention={}' hx-target='#community-feed-container'>{}</span>", user, matched)
            }
        }).into_owned()
    };

    let image_urls = p
        .image_urls
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|u| crate::storage::service::rewrite_gcs_url(&u))
        .collect();

    PostDisplay {
        id: p.id,
        author_name,
        author_initials,
        author_id: p.user_id,
        author_avatar,
        author_badges,
        post_type: p.post_type.clone(),
        content: raw_content,
        rendered_content,
        asset_id: p.asset_id,
        image_urls,
        link_preview: p.link_preview.clone(),
        link_preview_domain,
        reaction_count: p.reaction_count,
        comment_count: p.comment_count,
        is_hidden: p.is_hidden,
        is_pinned: p.is_pinned,
        disclaimer_shown: p.disclaimer_shown,
        verified_owner: false,
        created_at: p.created_at,
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

    let posts = service::get_community_feed(
        &c_pool,
        query.category.clone(),
        only_following_user_id,
        query.sort_by.clone(),
        limit,
        offset,
    )
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
        ));
    }

    Ok(feed)
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

    let response = map_to_post_display(
        &p,
        author_name,
        author_info.and_then(|a| a.avatar_url.clone()),
        author_badges,
    );

    Ok(Json(response))
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
        let _ = crate::community::xp::award_xp(
            &c_pool,
            user.id,
            "reaction_given",
            Some("Reacted to a post"),
            None,
        )
        .await;
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

    if let Some(reason) = validation::check_automod(&payload.content) {
        // M6-BE.1 Auto Mod
        return Err(AppError::Forbidden(format!(
            "Content violation: {}",
            reason
        )));
    }

    // FIX-F7: Check ban before allowing comment
    check_user_not_banned(&c_pool, user.id).await?;

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
    )
    .await?;

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

    let total_circles: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM circles")
        .fetch_one(&c_pool)
        .await
        .unwrap_or((0,));

    let total_xp: (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(current_xp), 0) FROM user_xp_totals")
            .fetch_one(&c_pool)
            .await
            .unwrap_or((0,));

    let pending_reports_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM content_reports WHERE status = 'pending'")
            .fetch_one(&c_pool)
            .await
            .unwrap_or((0,));

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

    let c_pool = get_community_pool(&state)?;

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

    // Award XP for post creation
    let _ = crate::community::xp::award_xp(
        &c_pool,
        user.id,
        "post_created",
        Some("Created a post"),
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

    Ok(Json(
        serde_json::json!({ "id": post_id, "verified_owner": verified_owner }),
    ))
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
        service::create_content_report(&c_pool, post_id, user.id, payload.reason).await?;

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
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    Json(payload): Json<models::AdminReportActionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    service::action_on_report(&c_pool, report_id, &payload.action, payload.admin_notes).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ─── ADMIN CHALLENGES ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct CreateChallengeReq {
    pub title: String,
    pub description: String,
    pub xp_reward: i32,
    pub badge_reward: Option<String>,
    pub requirement_type: String, // e.g., "buy_asset", "invite_friend"
    pub requirement_value: i32,
    pub frequency: String, // "once", "daily", "weekly"
}

async fn admin_list_challenges(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let _ = admin;
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
    let _ = admin;
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
    let _ = admin;
    let c_pool = get_community_pool(&state)?;

    crate::community::challenges::admin_toggle_challenge(&c_pool, id, payload.is_active).await?;

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

    let assets: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, name, symbol FROM assets WHERE id = ANY($1)")
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

    let posts: Vec<models::Post> = sqlx::query_as("SELECT * FROM posts ORDER BY created_at DESC")
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

    let action = if payload.is_banned {
        "user.ban"
    } else {
        "user.unban"
    };
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        action,
        "user",
        None,
        Some(user_id),
        Some(serde_json::json!({"reason": payload.reason})),
    )
    .await;

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
    let c_pool = get_community_pool(&state)?;

    let muted_until = payload
        .hours
        .map(|h| chrono::Utc::now() + chrono::Duration::hours(h as i64));

    sqlx::query("UPDATE community_profiles SET muted_until = $1 WHERE user_id = $2")
        .bind(muted_until)
        .bind(user_id)
        .execute(&c_pool)
        .await?;

    let action = if payload.hours.is_some() {
        "user.mute"
    } else {
        "user.unmute"
    };
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        action,
        "user",
        None,
        Some(user_id),
        Some(serde_json::json!({"hours": payload.hours})),
    )
    .await;

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
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE community_profiles SET is_shadowbanned = $1 WHERE user_id = $2")
        .bind(payload.is_shadowbanned)
        .bind(user_id)
        .execute(&c_pool)
        .await?;

    let action = if payload.is_shadowbanned {
        "user.shadowban"
    } else {
        "user.unshadowban"
    };
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        action,
        "user",
        None,
        Some(user_id),
        None,
    )
    .await;

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
    let c_pool = get_community_pool(&state)?;

    sqlx::query(
        "UPDATE community_profiles SET warning_count = warning_count + 1 WHERE user_id = $1",
    )
    .bind(user_id)
    .execute(&c_pool)
    .await?;

    // Create an in-app notification for the warning
    crate::community::notifications::notify_user(
        &c_pool,
        user_id,
        None,
        "system_alert",
        None,
        &format!("Warning from Admin: {}", payload.reason),
        None,
    )
    .await?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "user.warn",
        "user",
        None,
        Some(user_id),
        Some(serde_json::json!({"reason": payload.reason})),
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[derive(serde::Deserialize)]
pub struct UpdateModNotesPayload {
    pub notes: String,
}

async fn admin_update_mod_notes(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateModNotesPayload>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE community_profiles SET mod_notes = $1 WHERE user_id = $2")
        .bind(&payload.notes)
        .bind(user_id)
        .execute(&c_pool)
        .await?;

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

async fn admin_get_comments(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(_q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let limit: i64 = _q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(200);
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
    Json(_payload): Json<HideCommentPayload>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE comments SET is_hidden = true WHERE id = $1") // missing hidden_reason in comment schema? just set hidden
        .bind(comment_id)
        .execute(&c_pool)
        .await?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "comment.hide",
        "comment",
        Some(comment_id),
        None,
        None,
    )
    .await;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn admin_delete_comment(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(comment_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    sqlx::query("DELETE FROM comments WHERE id = $1")
        .bind(comment_id)
        .execute(&c_pool)
        .await?;

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "comment.delete",
        "comment",
        Some(comment_id),
        None,
        None,
    )
    .await;

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
    let c_pool = get_community_pool(&state)?;

    sqlx::query("UPDATE comments SET is_pinned = $1 WHERE id = $2")
        .bind(payload.is_pinned)
        .bind(comment_id)
        .execute(&c_pool)
        .await?;

    let action = if payload.is_pinned {
        "comment.pin"
    } else {
        "comment.unpin"
    };
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        action,
        "comment",
        Some(comment_id),
        None,
        None,
    )
    .await;

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
            post(create_announcement),
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
        .route(
            "/api/admin/community/reports/:id/action",
            post(take_report_action),
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
        .route(
            "/api/community/circles/leaderboard",
            get(get_circle_leaderboard),
        )
        .route("/api/community/circles/:id", get(get_circle_detail))
        .route(
            "/api/community/circles/:id",
            put(update_circle).delete(delete_own_circle_handler),
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
        .route("/api/community/invites", get(get_my_invites))
        .route("/api/community/invites/:id/accept", post(accept_invite))
        .route("/api/community/invites/:id/decline", post(decline_invite))
        // Property Reviews (M5)
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
        // Admin Audit Log (M2-ADMIN.7)
        .route(
            "/api/admin/community/audit-log",
            get(admin_get_community_audit_log),
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
    let bridge_info =
        crate::community::user_bridge::get_user_info(&state.db, state.redis.as_ref(), profile_id)
            .await
            .unwrap_or_else(|_| crate::community::user_bridge::UserBridgeInfo {
                user_id: profile_id,
                display_name: "Anonymous User".to_string(),
                avatar_url: None,
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
    let _ = crate::community::xp::award_xp(
        &c_pool,
        target_id,
        "follow_gained",
        Some("Gained a new follower"),
        None,
    )
    .await;

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

#[derive(Deserialize)]
struct SearchQuery {
    pub q: String,
    pub r#type: Option<String>, // "users", "posts", "all"
    pub page: Option<i64>,
}

async fn search_community(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let _user = middleware::get_current_user(&jar, &state.db).await;
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
        // Tag search vs content search
        posts_result = sqlx::query_as::<_, crate::community::models::Post>(
            r#"
            SELECT p.* FROM posts p
            JOIN community_profiles cp ON p.user_id = cp.user_id
            WHERE p.is_hidden = false 
              AND cp.is_shadowbanned = false
              AND cp.is_community_banned = false
              AND (p.content ILIKE $1 OR p.content_tags::text ILIKE $1)
            ORDER BY p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(&search_term)
        .bind(limit)
        .bind(offset)
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

    // Format response
    let mut users_formatted = Vec::new();
    for u in users_result {
        let auth = authors.get(&u.user_id);
        users_formatted.push(serde_json::json!({
            "user_id": u.user_id,
            "display_name": auth.map(|a| a.display_name.clone()).unwrap_or_else(|| "Anonymous".into()),
            "avatar_url": auth.and_then(|a| a.avatar_url.clone()),
            "bio": u.bio,
            "follower_count": u.follower_count,
            "badges": badges.get(&u.user_id).cloned().unwrap_or_default(),
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

    let c_pool = get_community_pool(&state)?;

    // Level gate: Level 2 required to create a circle (M4-BE.10)
    crate::community::xp::check_level_gate(
        &c_pool,
        user.id,
        crate::community::xp::GatedFeature::CreateCircle,
    )
    .await?;

    let circle = crate::community::circles::create_circle(
        &c_pool,
        user.id,
        &payload.name,
        payload.description.as_deref(),
        payload.emoji.as_deref(),
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

    Ok(Json(
        serde_json::json!({"circle": circle, "members": members}),
    ))
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
        &c_pool,
        circle_id,
        user.id,
        payload.name.as_deref(),
        payload.description.as_deref(),
        payload.emoji.as_deref(),
    )
    .await?;
    Ok(Json(circle))
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

    // W3.1: Check token gate requirement before allowing join
    crate::community::circles::check_token_gate(&c_pool, &state.db, user.id, circle_id).await?;

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
    role: String, // "admin" | "member"
}

async fn update_circle_member_role(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<UpdateRoleReq>,
) -> Result<impl IntoResponse, AppError> {
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
    )
    .await?;

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
    let counts: Vec<(Uuid, i64)> =
        sqlx::query_as("SELECT badge_id, COUNT(*)::BIGINT FROM user_badges GROUP BY badge_id")
            .fetch_all(&c_pool)
            .await?;

    let count_map: std::collections::HashMap<Uuid, i64> = counts.into_iter().collect();

    let result: Vec<serde_json::Value> = badges
        .iter()
        .map(|b| {
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
           RETURNING id, code, name, description, icon, display_order, created_at"#,
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
           WHERE id = $5"#,
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
        .ok_or_else(|| {
            AppError::NotFound(format!("Badge code '{}' not found", payload.badge_code))
        })?;

    sqlx::query(
        "INSERT INTO user_badges (user_id, badge_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(badge_id)
    .execute(&c_pool)
    .await?;

    Ok(Json(
        serde_json::json!({"success": true, "badge_code": payload.badge_code}),
    ))
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
        ) FROM community_profiles WHERE user_id = $1"#,
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

async fn admin_list_circles(
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let circles = crate::community::circles::admin_get_all_circles(&c_pool).await?;
    Ok(Json(serde_json::json!({ "circles": circles })))
}

async fn admin_delete_circle(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::admin_delete_circle(&c_pool, circle_id).await?;
    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "circle.delete",
        "circle",
        Some(circle_id),
        None,
        None,
    )
    .await;
    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

async fn admin_remove_circle_member(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path((circle_id, target_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
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
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::get_circle(&c_pool, circle_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Circle not found".into()))?;
    let members = crate::community::circles::get_circle_members(&c_pool, circle_id).await?;

    Ok(Json(serde_json::json!({
        "circle": circle,
        "members": members,
    })))
}

#[derive(serde::Deserialize)]
struct AdminUpdateCircleReq {
    name: Option<String>,
    description: Option<String>,
    avatar_emoji: Option<String>,
    is_public: Option<bool>,
}

async fn admin_update_circle(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(circle_id): Path<Uuid>,
    Json(payload): Json<AdminUpdateCircleReq>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;
    let circle = crate::community::circles::admin_force_update_circle(
        &c_pool,
        circle_id,
        payload.name.as_deref(),
        payload.description.as_deref(),
        payload.avatar_emoji.as_deref(),
        payload.is_public,
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
    let c_pool = get_community_pool(&state)?;
    crate::community::circles::admin_force_transfer_circle(
        &c_pool,
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
    _admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(_q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
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

async fn admin_award_xp(
    admin: crate::admin::extractors::AdminUser,
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AdminAwardXpReq>,
) -> Result<impl IntoResponse, AppError> {
    let c_pool = get_community_pool(&state)?;

    let amount_awarded = crate::community::xp::award_xp(
        &c_pool,
        user_id,
        &payload.reason_label,
        Some(&payload.description),
        Some(payload.amount),
    )
    .await?;

    if amount_awarded != 0 {
        // Evaluate level up
        crate::community::xp::update_user_level(&c_pool, user_id).await?;
    }

    crate::community::audit::log(
        &c_pool,
        admin.user.id,
        "xp.award",
        "user",
        None,
        Some(user_id),
        Some(serde_json::json!({"amount": payload.amount, "reason": payload.reason_label})),
    )
    .await;

    Ok(Json(
        serde_json::json!({ "status": "xp_awarded", "amount": amount_awarded }),
    ))
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

    // Build dynamic query
    let mut conditions = vec!["1=1".to_string()];
    if let Some(ref et) = entity_type_filter {
        conditions.push(format!("entity_type = '{}'", et.replace('\'', "")));
    }
    if let Some(ref act) = action_filter {
        conditions.push(format!("action = '{}'", act.replace('\'', "")));
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

    sqlx::query("INSERT INTO ban_appeals (user_id, appeal_text) VALUES ($1, $2)")
        .bind(user.id)
        .bind(payload.appeal_text)
        .execute(&c_pool)
        .await?;

    Ok(Json(
        serde_json::json!({"success": true, "message": "Appeal submitted successfully."}),
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
    // Unused admin variable
    let _a = admin;
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

async fn list_bookmarks(
    jar: CookieJar,
    State(state): State<AppState>,
    Query(query): Query<BookmarkQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = middleware::get_current_user(&jar, &state.db)
        .await
        .ok_or_else(|| AppError::Unauthorized("Auth needed".into()))?;

    let c_pool = get_community_pool(&state)?;

    let limit: i64 = 20;
    let offset = (query.page.unwrap_or(1).max(1) - 1) * limit;

    let posts = sqlx::query_as::<_, models::Post>(
        r#"
        SELECT p.*
        FROM bookmarks b
        JOIN posts p ON b.post_id = p.id
        WHERE b.user_id = $1 AND p.is_hidden = false
        ORDER BY b.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(user.id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&c_pool)
    .await?;

    // Build user_ids list for batch fetching
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
        ));
    }

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

/// Get posts by a specific hashtag
#[derive(Deserialize)]
pub struct HashtagPostsQuery {
    pub page: Option<i64>,
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
        ));
    }

    Ok(Json(serde_json::json!({
        "tag": clean_tag,
        "posts": feed,
    })))
}
