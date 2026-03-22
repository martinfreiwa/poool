use crate::community::models::{Post, ContentReport};
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

/// Gets the chronological feed, paginated.
pub async fn get_community_feed(
    pool: &PgPool,
    category: Option<String>,
    only_following_user_id: Option<Uuid>,
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
              AND ($2 IS NULL OR p.user_id IN (SELECT following_id FROM follows WHERE follower_id = $2))
            ORDER BY p.is_pinned DESC, p.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(cat)
        .bind(only_following_user_id)
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
              AND ($1 IS NULL OR user_id IN (SELECT following_id FROM follows WHERE follower_id = $1))
            ORDER BY is_pinned DESC, created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(only_following_user_id)
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
    let mut tx = pool.begin().await?;

    // Check if reaction already exists (with advisory lock via FOR UPDATE)
    let existing = sqlx::query(
        "SELECT id FROM reactions WHERE post_id = $1 AND user_id = $2 AND reaction_type = $3 FOR UPDATE"
    )
    .bind(post_id)
    .bind(user_id)
    .bind(&reaction_type)
    .fetch_optional(&mut *tx)
    .await?;

    let added = if existing.is_some() {
        // Remove existing reaction (toggle off)
        sqlx::query(
            "DELETE FROM reactions WHERE post_id = $1 AND user_id = $2 AND reaction_type = $3"
        )
        .bind(post_id)
        .bind(user_id)
        .bind(&reaction_type)
        .execute(&mut *tx)
        .await?;
        false
    } else {
        // Insert new reaction
        sqlx::query(
            "INSERT INTO reactions (post_id, user_id, reaction_type) VALUES ($1, $2, $3)"
        )
        .bind(post_id)
        .bind(user_id)
        .bind(&reaction_type)
        .execute(&mut *tx)
        .await?;
        true
    };

    tx.commit().await?;
    Ok(added)
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

// ─── Social Layer Functions (M3) ────────────────────────────────────────────────────────

pub async fn update_user_profile(
    pool: &PgPool,
    user_id: Uuid,
    bio: Option<String>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE community_profiles SET bio = $1, updated_at = NOW() WHERE user_id = $2")
        .bind(bio)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ─── Gamification & Badges ──────────────────────────────────────────────────

/// Fetches badges for a batch of users (useful for feed rendering without N+1)
pub async fn get_badges_batch(
    pool: &PgPool, 
    user_ids: &[Uuid]
) -> Result<std::collections::HashMap<Uuid, Vec<String>>, AppError> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    use sqlx::Row;
    let badge_rows = sqlx::query(
        "SELECT ub.user_id, b.icon 
         FROM user_badges ub 
         JOIN badges b ON ub.badge_id = b.id 
         WHERE ub.user_id = ANY($1) 
         ORDER BY b.display_order ASC"
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<Uuid, Vec<String>> = std::collections::HashMap::with_capacity(user_ids.len());
    for r in badge_rows {
        let uid: Uuid = r.get("user_id");
        let icon: String = r.get("icon");
        map.entry(uid).or_default().push(icon);
    }

    Ok(map)
}

#[derive(serde::Serialize)]
pub struct BadgeDisplay {
    pub code: String,
    pub name: String,
    pub icon: String,
}

#[derive(serde::Serialize)]
pub struct UserProfileDisplay {
    pub user_id: Uuid,
    pub bio: Option<String>,
    pub follower_count: i32,
    pub following_count: i32,
    pub post_count: i32,
    pub badges: Vec<BadgeDisplay>,
}

pub async fn is_following(pool: &PgPool, follower: Uuid, following: Uuid) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2)")
        .bind(follower)
        .bind(following)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn get_user_profile(pool: &PgPool, user_id: Uuid) -> Result<UserProfileDisplay, AppError> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT bio, follower_count, following_count, post_count 
         FROM community_profiles WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| AppError::NotFound("Profile not found".into()))?;

    // Load Badges
    let badge_rows = sqlx::query(
        "SELECT b.code, b.name, b.icon 
         FROM user_badges ub 
         JOIN badges b ON ub.badge_id = b.id 
         WHERE ub.user_id = $1 
         ORDER BY b.display_order ASC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let badges = badge_rows.into_iter().map(|r| BadgeDisplay {
        code: r.get("code"),
        name: r.get("name"),
        icon: r.get("icon"),
    }).collect();

    Ok(UserProfileDisplay {
        user_id,
        bio: row.try_get("bio")?,
        follower_count: row.try_get("follower_count")?,
        following_count: row.try_get("following_count")?,
        post_count: row.try_get("post_count")?,
        badges,
    })
}

pub async fn add_follow(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    // Insert follow logic
    let res = sqlx::query(
        "INSERT INTO follows (follower_id, following_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    )
    .bind(follower_id)
    .bind(following_id)
    .execute(&mut *tx)
    .await?;

    // Only update counts if actually newly inserted
    if res.rows_affected() > 0 {
        sqlx::query("UPDATE community_profiles SET following_count = following_count + 1 WHERE user_id = $1")
            .bind(follower_id)
            .execute(&mut *tx).await?;
        
        sqlx::query("UPDATE community_profiles SET follower_count = follower_count + 1 WHERE user_id = $1")
            .bind(following_id)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn remove_follow(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let res = sqlx::query("DELETE FROM follows WHERE follower_id = $1 AND following_id = $2")
        .bind(follower_id)
        .bind(following_id)
        .execute(&mut *tx)
        .await?;

    if res.rows_affected() > 0 {
        sqlx::query("UPDATE community_profiles SET following_count = GREATEST(0, following_count - 1) WHERE user_id = $1")
            .bind(follower_id)
            .execute(&mut *tx).await?;
        
        sqlx::query("UPDATE community_profiles SET follower_count = GREATEST(0, follower_count - 1) WHERE user_id = $1")
            .bind(following_id)
            .execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

// ─── Milestone Engine ───────────────────────────────────────────────────────

pub async fn trigger_investment_milestones(
    core_pool: &PgPool,
    community_pool: &PgPool,
    user_id: Uuid,
    new_asset_id: Uuid
) -> Result<(), AppError> {
    // 1. Get user total active investments count
    let total_investments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM investments WHERE user_id = $1 AND tokens_owned > 0"
    )
    .bind(user_id)
    .fetch_one(core_pool)
    .await?;

    // 2. Get Asset Name
    let asset_name: String = sqlx::query_scalar("SELECT name FROM assets WHERE id = $1")
        .bind(new_asset_id)
        .fetch_one(core_pool)
        .await?;

    let predefined_milestones = vec![1, 5, 10, 25, 50];

    if predefined_milestones.contains(&total_investments) {
        let content = if total_investments == 1 {
            format!("🎉 I just made my very first investment on POOOL in **{}**! Excited to join the community.", asset_name)
        } else {
            format!("🚀 Milestone reached! I just completed my {}th investment in **{}**.", total_investments, asset_name)
        };

        let sanitized = crate::community::validation::sanitize_html_basic(&content);
        let tags = " <span class=\"feed-post-badge\" style=\"background:#F0FDF4;color:#027A48;border:1px solid #D1FADF;\">Verified Owner</span> <span class=\"feed-post-badge\" style=\"background:#FFF9C4;color:#F57F17;border:1px solid #FFF59D;\">Milestone 🎉</span>";
        let finalized_content = format!("{}{}", sanitized, tags);

        // create_user_post handles adding to DB and updating profile counts!
        // We will call the DB insert directly to bypass the route redis limits.
        
        let mut tx = community_pool.begin().await?;
        let _post_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO posts (user_id, post_type, content, content_sanitized, asset_id, image_urls, is_hidden, disclaimer_shown) 
             VALUES ($1, $2, $3, $4, $5, $6, false, false) RETURNING id"
        )
        .bind(user_id)
        .bind("general")
        .bind(&finalized_content)
        .bind(&finalized_content)
        .bind(new_asset_id)
        .bind::<Option<Vec<String>>>(None)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE community_profiles SET post_count = post_count + 1 WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx).await?;

        tx.commit().await?;
    }

    Ok(())
}
