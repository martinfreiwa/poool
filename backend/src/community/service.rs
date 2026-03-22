use crate::community::models::{Comment, Post};
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

/// Gets the chronological announcement feed, paginated.
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
            JOIN announcement_categories ac ON ac.post_id = p.id
            WHERE p.post_type = 'announcement'
              AND p.is_hidden = false
              AND ac.category = $1
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
            WHERE post_type = 'announcement'
              AND is_hidden = false
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
