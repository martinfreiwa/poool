use crate::community::models::{ContentReport, Post};
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;
/// Ensures a community profile exists for a user.
pub async fn ensure_community_profile<'a, E>(executor: E, user_id: Uuid) -> Result<(), AppError>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    sqlx::query(
        "INSERT INTO community_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .execute(executor)
    .await?;
    Ok(())
}

/// Gets the chronological feed, paginated.
pub async fn get_community_feed(
    pool: &PgPool,
    category: Option<String>,
    only_following_user_id: Option<Uuid>,
    sort_by: Option<String>,
    limit: i64,
    offset: i64,
    // 14.8.2: when Some, the feed query also filters out posts authored by
    // anyone the current user has blocked OR muted, AND anyone who has
    // blocked the current user (reciprocal block).
    current_user_id: Option<Uuid>,
) -> Result<Vec<Post>, AppError> {
    let limit = limit.clamp(1, 50);

    let is_hot = sort_by.as_deref() == Some("hot");

    // Dynamic ordering:
    // If "hot", sort by engagement score (reactions + comments).
    // If "fresh" (default), sort by creation date.
    let order_clause = if is_hot {
        "ORDER BY p.is_pinned DESC, (p.reaction_count + p.comment_count * 2) DESC, p.created_at DESC"
    } else {
        "ORDER BY p.is_pinned DESC, p.created_at DESC"
    };

    // Reused inside both branches. When current_user_id is None the predicate
    // is vacuously true (NULL coalesced to '00000000…' avoids skipping rows).
    let block_mute_predicate = "
              AND ($CUR IS NULL OR p.user_id NOT IN (
                  SELECT target_user_id FROM block_relationships WHERE actor_user_id = $CUR
                  UNION SELECT actor_user_id FROM block_relationships WHERE target_user_id = $CUR
                  UNION SELECT target_user_id FROM mute_relationships WHERE actor_user_id = $CUR
              ))";

    let rows = if let Some(cat) = category {
        let query_str = format!(
            r#"
            SELECT p.*
            FROM posts p
            LEFT JOIN announcement_categories ac ON ac.post_id = p.id
            JOIN community_profiles cp ON p.user_id = cp.user_id
            WHERE p.is_hidden = false
              AND cp.is_shadowbanned = false
              -- CO.7: hide future-scheduled posts until their time arrives
              AND (p.scheduled_for IS NULL OR p.scheduled_for <= NOW())
              AND (ac.category = $1 OR $1 = '')
              AND ($2 IS NULL OR p.user_id IN (SELECT following_id FROM follows WHERE follower_id = $2))
              {block_mute}
            {order_clause}
            LIMIT $4 OFFSET $5
            "#,
            block_mute = block_mute_predicate.replace("$CUR", "$3"),
        );

        sqlx::query_as::<_, Post>(&query_str)
            .bind(cat)
            .bind(only_following_user_id)
            .bind(current_user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
    } else {
        let query_str = format!(
            r#"
            SELECT p.*
            FROM posts p
            JOIN community_profiles cp ON p.user_id = cp.user_id
            WHERE p.is_hidden = false
              AND cp.is_shadowbanned = false
              -- CO.7: hide future-scheduled posts until their time arrives
              AND (p.scheduled_for IS NULL OR p.scheduled_for <= NOW())
              AND ($1 IS NULL OR p.user_id IN (SELECT following_id FROM follows WHERE follower_id = $1))
              {block_mute}
            {order_clause}
            LIMIT $3 OFFSET $4
            "#,
            block_mute = block_mute_predicate.replace("$CUR", "$2"),
        );

        sqlx::query_as::<_, Post>(&query_str)
            .bind(only_following_user_id)
            .bind(current_user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
    };

    Ok(rows)
}

pub async fn get_announcements(
    pool: &PgPool,
    category: Option<String>,
    limit: i64,
) -> Result<Vec<crate::community::models::AnnouncementDisplay>, AppError> {
    let limit = limit.clamp(1, 50);

    let query_str = if category.is_some() && category.as_deref() != Some("") {
        r#"
        SELECT p.id, 'POOOL Official'::text as author_name, NULL::text as author_avatar,
               ac.category, p.content_sanitized, p.content, p.image_urls, 
               p.reaction_count, p.comment_count, p.is_pinned, p.created_at
        FROM posts p
        JOIN announcement_categories ac ON ac.post_id = p.id
        WHERE p.is_hidden = false AND ac.category = $1
        ORDER BY p.is_pinned DESC, p.created_at DESC
        LIMIT $2
        "#
    } else {
        r#"
        SELECT p.id, 'POOOL Official'::text as author_name, NULL::text as author_avatar,
               ac.category, p.content_sanitized, p.content, p.image_urls, 
               p.reaction_count, p.comment_count, p.is_pinned, p.created_at
        FROM posts p
        JOIN announcement_categories ac ON ac.post_id = p.id
        WHERE p.is_hidden = false
        ORDER BY p.is_pinned DESC, p.created_at DESC
        LIMIT $1
        "#
    };

    use sqlx::Row;
    let rows = if let Some(cat) = category {
        if cat.is_empty() {
            sqlx::query(query_str).bind(limit).fetch_all(pool).await?
        } else {
            sqlx::query(query_str)
                .bind(cat)
                .bind(limit)
                .fetch_all(pool)
                .await?
        }
    } else {
        sqlx::query(query_str).bind(limit).fetch_all(pool).await?
    };

    let mut results = Vec::new();
    for row in rows {
        let content_sanitized: Option<String> = row.get("content_sanitized");
        let content: String = row.get("content");
        let parsed_images: Vec<String> = row
            .try_get::<Option<Vec<String>>, _>("image_urls")
            .ok()
            .flatten()
            .unwrap_or_default();

        results.push(crate::community::models::AnnouncementDisplay {
            id: row.get("id"),
            author_name: row
                .try_get("author_name")
                .unwrap_or_else(|_| "POOOL Official".into()),
            author_avatar: row.try_get("author_avatar").ok().flatten(),
            category: row.get("category"),
            content: content_sanitized.unwrap_or(content),
            image_urls: parsed_images,
            reaction_count: row.get("reaction_count"),
            comment_count: row.get("comment_count"),
            is_pinned: row.get("is_pinned"),
            created_at: row.get("created_at"),
            created_at_display: row
                .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .format("%B %e, %Y")
                .to_string(),
        });
    }

    Ok(results)
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
    audit_details: serde_json::Value,
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

    sqlx::query(
        r#"
        INSERT INTO community_audit_logs
            (actor_user_id, action, entity_type, entity_id, target_user_id, details)
        VALUES ($1, 'announcement.create', 'announcement', $2, NULL, $3)
        "#,
    )
    .bind(user_id)
    .bind(post_id)
    .bind(audit_details)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(post_id)
}

pub struct ToggleReactionOutcome {
    pub added: bool,
    pub reaction_count: i32,
}

/// Toggle a reaction on a post.
pub async fn toggle_reaction(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
    reaction_type: String,
) -> Result<ToggleReactionOutcome, AppError> {
    const ALLOWED_REACTIONS: &[&str] = &["fire", "insightful", "clap", "green"];
    if !ALLOWED_REACTIONS.contains(&reaction_type.as_str()) {
        return Err(AppError::BadRequest("Invalid reaction type.".to_string()));
    }

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

    let added = if let Some(_) = existing {
        // Remove existing reaction (toggle off)
        sqlx::query(
            "DELETE FROM reactions WHERE post_id = $1 AND user_id = $2 AND reaction_type = $3",
        )
        .bind(post_id)
        .bind(user_id)
        .bind(&reaction_type)
        .execute(&mut *tx)
        .await?;
        false
    } else {
        // Insert new reaction
        sqlx::query("INSERT INTO reactions (post_id, user_id, reaction_type) VALUES ($1, $2, $3)")
            .bind(post_id)
            .bind(user_id)
            .bind(&reaction_type)
            .execute(&mut *tx)
            .await?;
        true
    };

    let reaction_count =
        sqlx::query_scalar::<_, i32>("SELECT reaction_count FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    tx.commit().await?;

    if added {
        // Find owner of the post and notify
        let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_optional(pool)
            .await?;

        if let Some(target_id) = owner_id {
            let notif_content = format!("Someone reacted with {} to your post.", reaction_type);
            let link = format!("/community/feed?post={}", post_id);
            let _ = crate::community::notifications::notify_user(
                pool,
                target_id,
                Some(user_id),
                "post_like",
                Some(post_id),
                &notif_content,
                Some(&link),
            )
            .await;
        }
    }

    Ok(ToggleReactionOutcome {
        added,
        reaction_count,
    })
}

/// Create a comment on a post (optionally as a reply to another comment).
///
/// 14.8.12 — when `parent_comment_id` is supplied, the parent must exist on
/// the same post AND must itself be a top-level comment (depth cap of 2 —
/// no reply to a reply). Replies still bump `posts.comment_count` so the
/// flat total reflects total engagement.
pub async fn create_comment(
    pool: &PgPool,
    post_id: Uuid,
    user_id: Uuid,
    content: String,
    content_sanitized: String,
    parent_comment_id: Option<Uuid>,
) -> Result<Uuid, AppError> {
    let mut tx = pool.begin().await?;

    if let Some(parent_id) = parent_comment_id {
        // Parent must exist on this post AND be top-level (parent.parent_id IS NULL).
        let parent: Option<(Uuid, Option<Uuid>)> =
            sqlx::query_as("SELECT post_id, parent_comment_id FROM comments WHERE id = $1")
                .bind(parent_id)
                .fetch_optional(&mut *tx)
                .await?;
        match parent {
            None => {
                return Err(AppError::NotFound("Parent comment not found.".to_string()));
            }
            Some((parent_post_id, parent_parent_id)) => {
                if parent_post_id != post_id {
                    return Err(AppError::BadRequest(
                        "Parent comment is on a different post.".to_string(),
                    ));
                }
                if parent_parent_id.is_some() {
                    return Err(AppError::BadRequest(
                        "Replies are limited to one level. Reply to the top-level comment instead."
                            .to_string(),
                    ));
                }
            }
        }
    }

    let comment_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO comments (post_id, user_id, content, content_sanitized, parent_comment_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(post_id)
    .bind(user_id)
    .bind(&content)
    .bind(&content_sanitized)
    .bind(parent_comment_id)
    .fetch_one(&mut *tx)
    .await?;

    let owner_id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE posts SET comment_count = comment_count + 1 WHERE id = $1 RETURNING user_id",
    )
    .bind(post_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    tx.commit().await?;

    let notif_content = "Someone commented on your post.";
    let link = format!("/community/feed?post={}", post_id);
    let _ = crate::community::notifications::notify_user(
        pool,
        owner_id,
        Some(user_id),
        "comment_reply",
        Some(post_id),
        notif_content,
        Some(&link),
    )
    .await;

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
                    return Err(AppError::BadRequest(
                        "Rate limit exceeded: Max 5 posts per hour.".into(),
                    ));
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
                return Err(AppError::BadRequest(
                    "Duplicate post detected. Please wait before posting the same content.".into(),
                ));
            }

            // Mark successful post creation in Redis limits
            let _: () = conn.incr(&rl_key, 1).await.unwrap_or(());
            let _: () = conn.expire(&rl_key, 3600).await.unwrap_or(());
            let _: () = conn.set_ex(&dup_key, "1", 300).await.unwrap_or(());
        }
    }

    let mut tx = pool.begin().await?;

    // Ensure user has a community profile (M2-BE.1)
    ensure_community_profile(&mut *tx, user_id).await?;

    // Moderate content
    let mod_result =
        crate::community::moderation::moderate_content(&req.content, is_high_level_user);

    // UX.16 — quote-repost validation. Reject self-quoting (silly) and
    // chains (one level deep). The quoted post must exist and not be
    // hidden. We don't enforce author block/mute here — the feed query
    // already hides those, so the quote card simply renders nothing.
    if let Some(quoted_id) = req.quoted_post_id {
        let quoted_meta: Option<(Uuid, bool, Option<Uuid>)> =
            sqlx::query_as("SELECT user_id, is_hidden, quoted_post_id FROM posts WHERE id = $1")
                .bind(quoted_id)
                .fetch_optional(&mut *tx)
                .await?;
        match quoted_meta {
            None => {
                return Err(AppError::BadRequest(
                    "The post you're quoting no longer exists.".into(),
                ));
            }
            Some((author_id, _, _)) if author_id == user_id => {
                return Err(AppError::BadRequest(
                    "You can't quote your own post — just edit it instead.".into(),
                ));
            }
            Some((_, true, _)) => {
                return Err(AppError::BadRequest(
                    "This post has been removed and can't be shared.".into(),
                ));
            }
            Some((_, _, Some(_))) => {
                return Err(AppError::BadRequest(
                    "Quote chains aren't supported — quote the original post instead.".into(),
                ));
            }
            _ => {}
        }
    }

    // CO.7 — validate scheduled timestamp. Cap how far ahead a post can be
    // queued so we don't accumulate forgotten zombie drafts indefinitely.
    let scheduled_for = match req.scheduled_for {
        Some(ts) => {
            let now = chrono::Utc::now();
            if ts <= now {
                return Err(AppError::BadRequest(
                    "scheduled_for must be in the future".into(),
                ));
            }
            if ts > now + chrono::Duration::days(60) {
                return Err(AppError::BadRequest(
                    "scheduled_for can be at most 60 days from now".into(),
                ));
            }
            Some(ts)
        }
        None => None,
    };

    let post_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO posts (user_id, post_type, content, content_sanitized, asset_id, image_urls, is_hidden, hidden_reason, disclaimer_shown, quoted_post_id, scheduled_for)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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
    .bind(req.quoted_post_id)
    .bind(scheduled_for)
    .fetch_one(&mut *tx)
    .await?;

    // UX.4: Extract and link hashtags from content
    extract_and_link_hashtags(&mut tx, &req.content, post_id).await?;

    // UX.11: Create poll if poll data is provided
    if let (Some(question), Some(options)) = (&req.poll_question, &req.poll_options) {
        if !question.is_empty() && options.len() >= 2 && options.len() <= 10 {
            let expires_at = req.poll_expires_hours.map(|hours| {
                chrono::Utc::now() + chrono::Duration::hours(hours.clamp(1, 168) as i64)
            });

            let poll_id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO polls (post_id, question, expires_at) VALUES ($1, $2, $3) RETURNING id"
            )
            .bind(post_id)
            .bind(question)
            .bind(expires_at)
            .fetch_one(&mut *tx)
            .await?;

            for (i, label) in options.iter().enumerate() {
                if !label.trim().is_empty() {
                    sqlx::query(
                        "INSERT INTO poll_options (poll_id, label, sort_order) VALUES ($1, $2, $3)",
                    )
                    .bind(poll_id)
                    .bind(label.trim())
                    .bind(i as i32)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }
    }

    tx.commit().await?;

    Ok(post_id)
}

/// UX.4: Extract hashtags from content and link them to the post.
/// Pattern: #word (alphanumeric + underscores, 1-100 chars).
async fn extract_and_link_hashtags(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    content: &str,
    post_id: Uuid,
) -> Result<(), AppError> {
    let mut seen = std::collections::HashSet::new();

    for word in content.split_whitespace() {
        if word.starts_with('#') && word.len() > 1 {
            let tag = word
                .trim_start_matches('#')
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_lowercase();
            if tag.is_empty() || tag.len() > 100 || seen.contains(&tag) {
                continue;
            }
            seen.insert(tag.clone());

            // Upsert the hashtag
            let hashtag_id: Uuid = sqlx::query_scalar(
                "INSERT INTO hashtags (tag) VALUES ($1) ON CONFLICT (tag) DO UPDATE SET tag = EXCLUDED.tag RETURNING id"
            )
            .bind(&tag)
            .fetch_one(&mut **tx)
            .await?;

            // Link post to hashtag
            sqlx::query(
                "INSERT INTO post_hashtags (post_id, hashtag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            )
            .bind(post_id)
            .bind(hashtag_id)
            .execute(&mut **tx)
            .await?;
        }
    }

    Ok(())
}

/// User reports a post.
pub async fn create_content_report(
    pool: &PgPool,
    post_id: Uuid,
    reporter_id: Uuid,
    reason: String,
    note: Option<String>,
) -> Result<Uuid, AppError> {
    // Cap user-supplied note length defensively; the textarea has a 500-char
    // maxlength but we don't trust client validation.
    let trimmed_note = note
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(500).collect::<String>());

    let report_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO content_reports (post_id, reporter_id, reason, reporter_note)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (post_id, reporter_id) DO UPDATE
        SET reporter_note = COALESCE(EXCLUDED.reporter_note, content_reports.reporter_note),
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(post_id)
    .bind(reporter_id)
    .bind(&reason)
    .bind(trimmed_note.as_deref())
    .fetch_one(pool)
    .await?;

    Ok(report_id)
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
    let post = sqlx::query("SELECT user_id, created_at FROM posts WHERE id = $1 FOR UPDATE")
        .bind(post_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let post_user_id: Uuid = post.try_get("user_id")?;
    let created_at: chrono::DateTime<chrono::Utc> = post.try_get("created_at")?;

    if post_user_id != user_id {
        return Err(AppError::Forbidden(
            "You can only edit your own posts".to_string(),
        ));
    }

    let now = chrono::Utc::now();
    if (now - created_at).num_minutes() > 15 {
        return Err(AppError::BadRequest(
            "Posts can only be edited within 15 minutes of creation".to_string(),
        ));
    }

    let mod_result =
        crate::community::moderation::moderate_content(&new_content, is_high_level_user);

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
pub async fn delete_user_post(pool: &PgPool, post_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    use sqlx::Row;

    let post = sqlx::query("SELECT user_id FROM posts WHERE id = $1 FOR UPDATE")
        .bind(post_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    let post_user_id: Uuid = post.try_get("user_id")?;

    if post_user_id != user_id {
        return Err(AppError::Forbidden(
            "You can only delete your own posts".to_string(),
        ));
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
        "SELECT * FROM content_reports WHERE status = 'pending' ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(reports)
}

pub async fn action_on_report(
    pool: &PgPool,
    report_id: Uuid,
    actor_user_id: Uuid,
    action: &str,
    notes: String,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    use sqlx::Row;

    let row = sqlx::query("SELECT post_id, status FROM content_reports WHERE id = $1 FOR UPDATE")
        .bind(report_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Report not found".into()))?;

    let post_id: Uuid = row.try_get("post_id")?;
    let status: String = row.try_get("status")?;
    if status != "pending" {
        return Err(AppError::Conflict(
            "Report has already been moderated".to_string(),
        ));
    }

    let post =
        sqlx::query("SELECT user_id, is_hidden, hidden_reason FROM posts WHERE id = $1 FOR UPDATE")
            .bind(post_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Reported post not found".into()))?;
    let author_id: Uuid = post.try_get("user_id")?;
    let previous_is_hidden: bool = post.try_get("is_hidden")?;
    let previous_hidden_reason: Option<String> = post.try_get("hidden_reason")?;

    let (report_status, audit_action, action_details) = match action {
        "hide_post" => {
            let hidden_reason = format!("Hidden after report: {}", notes);
            let result = sqlx::query(
                "UPDATE posts SET is_hidden = true, hidden_reason = $1, updated_at = NOW() WHERE id = $2",
            )
                .bind(&hidden_reason)
                .bind(post_id)
                .execute(&mut *tx)
                .await?;
            if result.rows_affected() != 1 {
                return Err(AppError::NotFound("Reported post not found".into()));
            }

            (
                "resolved",
                "report.hide_post",
                serde_json::json!({
                    "previous_post": {
                        "is_hidden": previous_is_hidden,
                        "hidden_reason": previous_hidden_reason,
                    },
                    "new_post": {
                        "is_hidden": true,
                        "hidden_reason": hidden_reason,
                    }
                }),
            )
        }
        "dismiss_report" => (
            "dismissed",
            "report.dismiss",
            serde_json::json!({
                "previous_post": {
                    "is_hidden": previous_is_hidden,
                    "hidden_reason": previous_hidden_reason,
                }
            }),
        ),
        "warn_user" => {
            ensure_community_profile(&mut *tx, author_id).await?;
            let profile = sqlx::query(
                "SELECT warning_count FROM community_profiles WHERE user_id = $1 FOR UPDATE",
            )
            .bind(author_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Community profile not found".into()))?;
            let previous_warning_count: i32 = profile.try_get("warning_count")?;

            let result = sqlx::query(
                "UPDATE community_profiles SET warning_count = warning_count + 1, updated_at = NOW() WHERE user_id = $1",
            )
                .bind(author_id)
                .execute(&mut *tx)
                .await?;
            if result.rows_affected() != 1 {
                return Err(AppError::NotFound("Community profile not found".into()));
            }

            (
                "resolved",
                "report.warn_user",
                serde_json::json!({
                    "previous_profile": {
                        "warning_count": previous_warning_count,
                    },
                    "new_profile": {
                        "warning_count": previous_warning_count + 1,
                    }
                }),
            )
        }
        "ban_user" => {
            ensure_community_profile(&mut *tx, author_id).await?;
            let profile = sqlx::query(
                "SELECT is_community_banned, ban_reason FROM community_profiles WHERE user_id = $1 FOR UPDATE",
            )
            .bind(author_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Community profile not found".into()))?;
            let previous_is_banned: bool = profile.try_get("is_community_banned")?;
            let previous_ban_reason: Option<String> = profile.try_get("ban_reason")?;

            let result = sqlx::query(
                "UPDATE community_profiles SET is_community_banned = true, ban_reason = $1, updated_at = NOW() WHERE user_id = $2",
            )
                .bind(&notes)
                .bind(author_id)
                .execute(&mut *tx)
                .await?;
            if result.rows_affected() != 1 {
                return Err(AppError::NotFound("Community profile not found".into()));
            }

            (
                "resolved",
                "report.ban_user",
                serde_json::json!({
                    "previous_profile": {
                        "is_community_banned": previous_is_banned,
                        "ban_reason": previous_ban_reason,
                    },
                    "new_profile": {
                        "is_community_banned": true,
                        "ban_reason": notes.clone(),
                    }
                }),
            )
        }
        _ => return Err(AppError::BadRequest("Invalid action type".into())),
    };

    let report_result = sqlx::query(
        "UPDATE content_reports SET status = $1, admin_notes = $2, updated_at = NOW() WHERE id = $3 AND status = 'pending'",
    )
        .bind(report_status)
        .bind(&notes)
        .bind(report_id)
        .execute(&mut *tx)
        .await?;
    if report_result.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "Report has already been moderated".to_string(),
        ));
    }

    sqlx::query(
        r#"INSERT INTO community_audit_logs
           (actor_user_id, action, entity_type, entity_id, target_user_id, details)
           VALUES ($1, $2, 'content_report', $3, $4, $5)"#,
    )
    .bind(actor_user_id)
    .bind(audit_action)
    .bind(report_id)
    .bind(author_id)
    .bind(serde_json::json!({
        "report_id": report_id,
        "post_id": post_id,
        "target_user_id": author_id,
        "action": action,
        "previous_report_status": status,
        "new_report_status": report_status,
        "admin_notes": notes,
        "action_details": action_details,
    }))
    .execute(&mut *tx)
    .await?;

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
         LIMIT 3",
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
    flair: Option<Option<String>>,
    is_public_profile: Option<bool>,
    allow_dms_from_strangers: Option<bool>,
) -> Result<(), AppError> {
    // Cap flair to 24 chars + trim. Empty string clears (passed as Some(None)).
    let flair_normalized: Option<Option<String>> = flair.map(|opt| {
        opt.and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.chars().take(24).collect())
            }
        })
    });

    // Make sure the row exists so partial updates don't quietly drop.
    ensure_community_profile(pool, user_id).await?;

    // CASE-based update so callers can choose to leave any field untouched
    // (None) vs explicitly set (Some(_)). Lets the FE PUT a single key
    // for instant per-toggle save.
    sqlx::query(
        r#"UPDATE community_profiles SET
            bio = COALESCE($1, bio),
            flair = CASE WHEN $2::BOOL THEN $3 ELSE flair END,
            is_public_profile = COALESCE($4, is_public_profile),
            allow_dms_from_strangers = COALESCE($5, allow_dms_from_strangers),
            updated_at = NOW()
           WHERE user_id = $6"#,
    )
    .bind(bio)
    .bind(flair_normalized.is_some())
    .bind(flair_normalized.flatten())
    .bind(is_public_profile)
    .bind(allow_dms_from_strangers)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// UX.17: dynamic "Top Contributor" set. Returns the user_ids that
/// currently sit in the top `limit` slots of the community by `xp_total`.
/// Cheap (one indexed query) and idempotent — callers can intersect with
/// the visible page of authors to flag who deserves the badge.
pub async fn get_top_contributor_set(
    pool: &PgPool,
    limit: i64,
) -> Result<std::collections::HashSet<Uuid>, AppError> {
    let limit = limit.clamp(1, 500);
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM community_profiles
         WHERE is_shadowbanned = FALSE AND is_community_banned = FALSE
         ORDER BY xp_total DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(ids.into_iter().collect())
}

/// UX.14: batch-resolve user_id → flair for feed/profile rendering.
/// Only returns entries for users who actually set a flair to keep the
/// downstream HashMap small. Failure is non-fatal; caller treats absence
/// as "no flair".
pub async fn get_flairs_batch(
    pool: &PgPool,
    user_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, String>, AppError> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT user_id, flair FROM community_profiles
         WHERE user_id = ANY($1) AND flair IS NOT NULL AND flair <> ''",
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;
    let mut out = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        let uid: Uuid = match r.try_get("user_id") {
            Ok(v) => v,
            Err(_) => continue,
        };
        let flair: String = r.try_get("flair").unwrap_or_default();
        if !flair.is_empty() {
            out.insert(uid, flair);
        }
    }
    Ok(out)
}

// ─── Gamification & Badges ──────────────────────────────────────────────────

/// Fetches badges for a batch of users (useful for feed rendering without N+1)
pub async fn get_badges_batch(
    pool: &PgPool,
    user_ids: &[Uuid],
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
         ORDER BY b.display_order ASC",
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<Uuid, Vec<String>> =
        std::collections::HashMap::with_capacity(user_ids.len());
    for r in badge_rows {
        let uid: Uuid = r.get("user_id");
        let icon: String = r.get("icon");
        map.entry(uid).or_default().push(icon);
    }

    Ok(map)
}

#[derive(serde::Serialize)]
pub struct BadgeDisplay {
    pub id: Uuid,
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
    /// UX.14: optional short flair (max 24 chars).
    pub flair: Option<String>,
    /// Privacy toggles surfaced for the /community/me/edit page.
    pub is_public_profile: bool,
    pub allow_dms_from_strangers: bool,
}

pub async fn is_following(
    pool: &PgPool,
    follower: Uuid,
    following: Uuid,
) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2)",
    )
    .bind(follower)
    .bind(following)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn get_user_profile(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<UserProfileDisplay, AppError> {
    // Ensure profile exists (Auto-Onboarding Fix)
    ensure_community_profile(pool, user_id).await?;

    use sqlx::Row;
    let row = sqlx::query(
        "SELECT bio, follower_count, following_count, post_count, flair,
                COALESCE(is_public_profile, TRUE) AS is_public_profile,
                COALESCE(allow_dms_from_strangers, TRUE) AS allow_dms_from_strangers
         FROM community_profiles WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| AppError::NotFound("Profile not found".into()))?;

    // Load Badges
    let badge_rows = sqlx::query(
        "SELECT b.id, b.code, b.name, b.icon
         FROM user_badges ub
         JOIN badges b ON ub.badge_id = b.id
         WHERE ub.user_id = $1
         ORDER BY b.display_order ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let badges = badge_rows
        .into_iter()
        .map(|r| BadgeDisplay {
            id: r.get("id"),
            code: r.get("code"),
            name: r.get("name"),
            icon: r.get("icon"),
        })
        .collect();

    Ok(UserProfileDisplay {
        user_id,
        bio: row.try_get("bio")?,
        follower_count: row.try_get("follower_count")?,
        following_count: row.try_get("following_count")?,
        post_count: row.try_get("post_count")?,
        badges,
        flair: row.try_get::<Option<String>, _>("flair").ok().flatten(),
        is_public_profile: row.try_get::<bool, _>("is_public_profile").unwrap_or(true),
        allow_dms_from_strangers: row
            .try_get::<bool, _>("allow_dms_from_strangers")
            .unwrap_or(true),
    })
}

pub async fn add_follow(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    // Insert follow logic
    let res = sqlx::query(
        "INSERT INTO follows (follower_id, following_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
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

        sqlx::query(
            "UPDATE community_profiles SET follower_count = follower_count + 1 WHERE user_id = $1",
        )
        .bind(following_id)
        .execute(&mut *tx)
        .await?;

        // Notify
        let notif_content = "Someone started following you.".to_string();
        let link = format!("/community/profile?user={}", follower_id);
        let _ = crate::community::notifications::notify_user(
            pool,
            following_id,
            Some(follower_id),
            "new_follower",
            None,
            &notif_content,
            Some(&link),
        )
        .await;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn remove_follow(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
) -> Result<(), AppError> {
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
    new_asset_id: Uuid,
) -> Result<(), AppError> {
    // 1. Get user total active investments count
    let total_investments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM investments WHERE user_id = $1 AND tokens_owned > 0",
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
            format!(
                "🚀 Milestone reached! I just completed my {}th investment in **{}**.",
                total_investments, asset_name
            )
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
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
    }

    Ok(())
}
