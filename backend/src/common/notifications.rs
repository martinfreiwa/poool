//! Phase-3 P1: in-app notifications inbox helpers.
//!
//! Centralises writes to the `notifications` table so producers can call
//! a single typed function instead of scattering `INSERT INTO notifications`
//! across the codebase. Bell-icon + dropdown read via the API in
//! `crate::inbox::router()`.
//!
//! See `database/183_notifications_inbox.sql` for the schema extensions.

use crate::error::AppError;
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Insert one notification for `user_id`. Returns the new row id.
///
/// `event_type` must match the CHECK constraint in mig 183 (e.g.
/// `affiliate_commission_earned`, `team_invitation_accepted`).
/// `title` is the headline shown in the dropdown row (≤180 chars).
/// `body` is optional preview text.
/// `link_url` is optional deep-link target on click.
/// `metadata` is an open JSON payload; renderers may inspect keys.
pub async fn enqueue_notification(
    pool: &PgPool,
    user_id: Uuid,
    event_type: &str,
    title: &str,
    body: Option<&str>,
    link_url: Option<&str>,
    metadata: serde_json::Value,
) -> Result<Uuid, AppError> {
    // Cap by char count (not bytes) — byte-slicing on `&str` panics when the
    // 180th byte sits inside a multi-byte UTF-8 sequence (emoji, CJK, accents).
    let clipped_title: String = title.chars().take(180).collect();
    let id = sqlx::query_scalar!(
        r#"INSERT INTO notifications
              (user_id, type, title, message, link_url, metadata, is_read)
           VALUES ($1, $2, $3, $4, $5, $6, false)
           RETURNING id"#,
        user_id,
        event_type,
        clipped_title,
        body,
        link_url,
        metadata,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("notification insert failed: {}", e)))?;
    Ok(id)
}

// ─── Read-path API row types (shared with inbox routes) ────────────────────

#[derive(FromRow, Serialize)]
pub struct InboxRow {
    pub id: Uuid,
    pub event_type: String,
    pub title: String,
    pub body: Option<String>,
    pub link_url: Option<String>,
    pub metadata: serde_json::Value,
    pub is_read: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Cursor-paginated list. `before` is the `created_at` of the last row
/// the client already has; pass `None` for the first page.
pub async fn list_notifications_for_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    before: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<InboxRow>, AppError> {
    let limit_clamped = limit.clamp(1, 100);
    let rows = match before {
        Some(ts) => sqlx::query_as::<_, InboxRow>(
            r#"SELECT id, type AS event_type, title, message AS body, link_url,
                      metadata, is_read, created_at
                 FROM notifications
                WHERE user_id = $1 AND created_at < $2
                ORDER BY created_at DESC, id DESC
                LIMIT $3"#,
        )
        .bind(user_id)
        .bind(ts)
        .bind(limit_clamped)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?,
        None => sqlx::query_as::<_, InboxRow>(
            r#"SELECT id, type AS event_type, title, message AS body, link_url,
                      metadata, is_read, created_at
                 FROM notifications
                WHERE user_id = $1
                ORDER BY created_at DESC, id DESC
                LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit_clamped)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?,
    };
    Ok(rows)
}

pub async fn unread_count_for_user(pool: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    sqlx::query_scalar!(
        "SELECT COUNT(*)::BIGINT AS \"c!\"
           FROM notifications
          WHERE user_id = $1 AND is_read = FALSE",
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))
}

/// Mark one notification read. Scoped by user_id so a request bearer
/// cannot mark another user's notification as read.
pub async fn mark_one_read(
    pool: &PgPool,
    user_id: Uuid,
    notification_id: Uuid,
) -> Result<bool, AppError> {
    let updated = sqlx::query!(
        "UPDATE notifications SET is_read = TRUE
          WHERE id = $1 AND user_id = $2 AND is_read = FALSE",
        notification_id,
        user_id,
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(updated.rows_affected() > 0)
}

pub async fn mark_all_read(pool: &PgPool, user_id: Uuid) -> Result<u64, AppError> {
    let updated = sqlx::query!(
        "UPDATE notifications SET is_read = TRUE
          WHERE user_id = $1 AND is_read = FALSE",
        user_id
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(updated.rows_affected())
}
