use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ─── Models ─────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub actor_id: Option<Uuid>,      // who acted
    pub actor_name: Option<String>,  // Denormalized/Joined
    pub actor_avatar: Option<String>,
    pub r#type: String,              // new_follower, post_like, level_up, etc.
    pub entity_id: Option<Uuid>,     // post.id or user.id
    pub content: String,
    pub link_url: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

// ─── Core Logic ─────────────────────────────────────────────────────

/// Helper to trigger a new notification internally.
pub async fn notify_user(
    pool: &PgPool,
    user_id: Uuid,
    actor_id: Option<Uuid>,
    type_code: &str,
    entity_id: Option<Uuid>,
    content: &str,
    link_url: Option<&str>,
) -> Result<(), AppError> {
    if Some(user_id) == actor_id {
        return Ok(()); // Don't notify yourself
    }

    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, actor_id, type, entity_id, content, link_url)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(user_id)
    .bind(actor_id)
    .bind(type_code)
    .bind(entity_id)
    .bind(content)
    .bind(link_url)
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper method to fetch notifications for the current user
pub async fn get_my_notifications(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<Notification>, AppError> {
    let limit = limit.clamp(1, 100);

    let rows = sqlx::query_as::<_, Notification>(
        r#"
        SELECT 
            n.id, n.user_id, n.actor_id,
            n.type, n.entity_id, n.content, n.link_url, n.is_read, n.created_at,
            cp.display_name AS actor_name,
            cp.avatar_url AS actor_avatar
        FROM notifications n
        LEFT JOIN community_profiles cp ON cp.user_id = n.actor_id
        WHERE n.user_id = $1
        ORDER BY n.created_at DESC
        LIMIT $2
        "#
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Get unread notification count
pub async fn get_unread_count(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<i64, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Mark a single notification as read
pub async fn mark_as_read(
    pool: &PgPool,
    user_id: Uuid,
    notification_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("UPDATE notifications SET is_read = true WHERE id = $1 AND user_id = $2")
        .bind(notification_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mark ALL notifications as read
pub async fn mark_all_as_read(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("UPDATE notifications SET is_read = true WHERE user_id = $1 AND is_read = false")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
