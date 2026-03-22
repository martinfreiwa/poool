use crate::community::models::{Post, ContentReport};
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

/// Gets the chronological feed, paginated.
pub async fn get_announcement_feed(
    pool: &PgPool,
    category: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Post>, AppError> {
    let limit = limit.clamp(1, 50);

    let rows = if let Some(cat) = category {
        sqlx::query_as::<_, Post>(
            r#"
            SELECT p.*
            FROM posts p
            LEFT JOIN announcement_categories ac ON ac.post_id = p.id
            WHERE p.is_hidden = false
              AND (ac.category = $1 OR $1 = '')
            ORDER BY p.is_pinned DESC, p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(cat)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Post>(
            r#"
            SELECT *
            FROM posts
            WHERE is_hidden = false
            ORDER BY is_pinned DESC, created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok(rows)
}

/// Admin creates an announcement.
pub async fn create_announcement(
    pool: &PgPool,
    user_id: Uuid,
    content: String,
    content_sanitized: String,
    category: String,
    image_urls: Option<Vec<String>>,
    is_pinned: bool,
) -> Result<Uuid, AppError> {
    let mut tx = pool.begin().await?;

    let post_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO posts (user_id, post_type, content, content_sanitized, image_urls, is_pinned)
        VALUES ($1, 'announcement', $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&content)
    .bind(&content_sanitized)
    .bind(image_urls.as_deref())
    .bind(is_pinned)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO announcement_categories (post_id, category)
        VALUES ($1, $2)
        "#,
    )
    .bind(post_id)
    .bind(&category)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(post_id)
}

/// Toggle a reaction on a post. Returns true if added, false if removed.
pub async fn toggle_reaction(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
    reaction_type: String,
) -> Result<bool, AppError> {
    // Attempt to insert
    let inserted = sqlx::query(
        r#"
        INSERT INTO reactions (post_id, user_id, reaction_type)
        VALUES ($1, $2, $3)
        ON CONFLICT (post_id, user_id, reaction_type) DO NOTHING
        "#,
    )
    .bind(post_id)
    .bind(user_id)
    .bind(&reaction_type)
    .execute(pool)
    .await?;

    if inserted.rows_affected() > 0 {
        return Ok(true);
    }

    // Reaction already existed, so we remove it (toggle off)
    sqlx::query(
        r#"
        DELETE FROM reactions
        WHERE post_id = $1 AND user_id = $2 AND reaction_type = $3
        "#,
    )
    .bind(post_id)
    .bind(user_id)
    .bind(&reaction_type)
    .execute(pool)
    .await?;

    Ok(false)
}

/// Create a comment on a post
pub async fn create_comment(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
    content: String,
    content_sanitized: String,
) -> Result<Uuid, AppError> {
    let comment_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO comments (post_id, user_id, content, content_sanitized)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(post_id)
    .bind(user_id)
    .bind(&content)
    .bind(&content_sanitized)
    .fetch_one(pool)
    .await?;

    // Increment post comment_count (since M1 doesn't have a DB trigger for comments yet, we do it in code,
    // though the Masterplan might add a trigger later).
    sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;

    Ok(comment_id)
}

/// User creates a post.
pub async fn create_user_post(
    pool: &PgPool,
    redis: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
    req: crate::community::models::CreatePostRequest,
    is_high_level_user: bool,
) -> Result<Uuid, AppError> {

    // --- Post Rate Limiting (M2-BE.7) ---
    if let Some(redis_pool) = redis {
        use redis::AsyncCommands;
        if let Ok(mut conn) = redis_pool.get().await {
            let rl_key = format!("community:ratelimit:posts:{}", user_id);
            // 1) Rate Limit: max 5 posts per hour
            let count: Option<i64> = conn.get(&rl_key).await.unwrap_or(None);
            if let Some(c) = count {
                if c >= 5 {
                    return Err(AppError::BadRequest("Rate limit exceeded: Max 5 posts per hour.".into()));
                }
            }

            // 2) Duplicate-Detection: check last post hash in 5 minutes
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(req.content.as_bytes());
            let content_hash = format!("{:x}", hasher.finalize());
            let dup_key = format!("community:dup:{}:{}", user_id, content_hash);
            let is_dup: Option<String> = conn.get(&dup_key).await.unwrap_or(None);
            if is_dup.is_some() {
                return Err(AppError::BadRequest("Duplicate post detected. Please wait before posting the same content.".into()));
            }

            // Mark successful post creation in Redis limits
            let _ : () = conn.incr(&rl_key, 1).await.unwrap_or(());
            let _ : () = conn.expire(&rl_key, 3600).await.unwrap_or(());
            let _ : () = conn.set_ex(&dup_key, "1", 300).await.unwrap_or(());
        }
    }

    let mut tx = pool.begin().await?;

    // Moderate content
    let mod_result = crate::community::moderation::moderate_content(&req.content, is_high_level_user);

    let post_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO posts (user_id, post_type, content, content_sanitized, asset_id, image_urls, is_hidden, hidden_reason, disclaimer_shown)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&req.post_type)
    .bind(&req.content)
    .bind(&mod_result.sanitized_content)
    .bind(req.asset_id)
    .bind(req.image_urls.as_deref())
    .bind(mod_result.is_flagged)
    .bind(&mod_result.flag_reason)
    .bind(mod_result.needs_disclaimer)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(post_id)
}

/// User reports a post.
pub async fn create_content_report(
    pool: &PgPool,
    post_id: Uuid,
    reporter_id: Uuid,
    reason: String,
) -> Result<Uuid, AppError> {
    let report_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO content_reports (post_id, reporter_id, reason)
        VALUES ($1, $2, $3)
        ON CONFLICT (post_id, reporter_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(post_id)
    .bind(reporter_id)
    .bind(&reason)
    .fetch_optional(pool)
    .await?;

    // If it was already reported, just return a dummy UUID (or the actual one if we fetched it, but ON CONFLICT DO NOTHING returns nothing)
    Ok(report_id.unwrap_or_else(Uuid::new_v4))
}

/// Edit a user post (must be within 15 minutes of creation)
pub async fn update_user_post(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
    new_content: String,
    is_high_level_user: bool,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    use sqlx::Row;

    // Check ownership and time
    let post = sqlx::query(
        "SELECT user_id, created_at FROM posts WHERE id = $1 FOR UPDATE"
    )
    .bind(post_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let post_user_id: Uuid = post.try_get("user_id")?;
    let created_at: chrono::DateTime<chrono::Utc> = post.try_get("created_at")?;

    if post_user_id != user_id {
        return Err(AppError::Forbidden("You can only edit your own posts".to_string()));
    }

    let now = chrono::Utc::now();
    if (now - created_at).num_minutes() > 15 {
        return Err(AppError::BadRequest("Posts can only be edited within 15 minutes of creation".to_string()));
    }

    let mod_result = crate::community::moderation::moderate_content(&new_content, is_high_level_user);

    sqlx::query(
        r#"
        UPDATE posts 
        SET content = $1, content_sanitized = $2, is_hidden = $3, hidden_reason = $4, disclaimer_shown = $5, updated_at = NOW()
        WHERE id = $6
        "#
    )
    .bind(&new_content)
    .bind(&mod_result.sanitized_content)
    .bind(mod_result.is_flagged)
    .bind(&mod_result.flag_reason)
    .bind(mod_result.needs_disclaimer)
    .bind(post_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Delete a user post (must be owner)
pub async fn delete_user_post(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    use sqlx::Row;

    let post = sqlx::query(
        "SELECT user_id FROM posts WHERE id = $1 FOR UPDATE"
    )
    .bind(post_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let post_user_id: Uuid = post.try_get("user_id")?;

    if post_user_id != user_id {
        return Err(AppError::Forbidden("You can only delete your own posts".to_string()));
    }

    sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_pending_reports(pool: &PgPool) -> Result<Vec<ContentReport>, AppError> {
    let reports = sqlx::query_as::<_, ContentReport>(
        "SELECT * FROM content_reports WHERE status = 'pending' ORDER BY created_at ASC"
    )
    .fetch_all(pool)
    .await?;

    Ok(reports)
}

pub async fn action_on_report(
    pool: &PgPool,
    report_id: Uuid,
    action: &str,
    notes: Option<String>,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    // Get the report to find the post relative to it.
    // Use manual query approach instead of macro to ensure cross-db compat
    let row = sqlx::query("SELECT post_id FROM content_reports WHERE id = $1")
        .bind(report_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Report not found".into()))?;

    use sqlx::Row;
    let post_id: Uuid = row.try_get("post_id")?;

    match action {
        "hide_post" => {
            sqlx::query("UPDATE posts SET is_hidden = true, hidden_reason = 'Moderator action' WHERE id = $1")
                .bind(post_id)
                .execute(&mut *tx)
                .await?;
            
            sqlx::query("UPDATE content_reports SET status = 'resolved', admin_notes = $1, updated_at = NOW() WHERE id = $2")
                .bind(notes)
                .bind(report_id)
                .execute(&mut *tx)
                .await?;
        }
        "dismiss_report" => {
            sqlx::query("UPDATE content_reports SET status = 'dismissed', admin_notes = $1, updated_at = NOW() WHERE id = $2")
                .bind(notes)
                .bind(report_id)
                .execute(&mut *tx)
                .await?;
        }
        _ => return Err(AppError::BadRequest("Invalid action type".into())),
    }

    tx.commit().await?;
    Ok(())
}


pub async fn get_trending_assets(pool: &PgPool) -> Result<Vec<(Uuid, i64)>, AppError> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT asset_id, count(*) as post_count \
         FROM posts \
         WHERE asset_id IS NOT NULL \
         GROUP BY asset_id \
         ORDER BY post_count DESC \
         LIMIT 3"
    )
    .fetch_all(pool)
    .await?;

    let mut trending = Vec::new();
    for row in rows {
        let asset_id: Uuid = row.try_get("asset_id")?;
        let count: i64 = row.try_get("post_count")?;
        trending.push((asset_id, count));
    }

    Ok(trending)
}
