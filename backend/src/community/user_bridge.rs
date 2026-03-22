use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct UserBridgeInfo {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Fetches basic user information (Name + Avatar) from the Core DB.
/// Used to enrich community posts and comments on the fly.
pub async fn get_user_info(core_pool: &PgPool, user_id: Uuid) -> Result<UserBridgeInfo, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT u.id as "id!", up.display_name, u.avatar_url 
        FROM users u
        LEFT JOIN user_profiles up ON u.id = up.user_id
        WHERE u.id = $1
        "#,
        user_id
    )
    .fetch_optional(core_pool)
    .await?;

    match row {
        Some(r) => Ok(UserBridgeInfo {
            user_id: r.id,
            display_name: r
                .display_name
                .unwrap_or_else(|| "Anonymous User".to_string()),
            avatar_url: r.avatar_url,
        }),
        None => Err(AppError::NotFound("User not found in Core DB".to_string())),
    }
}

/// Batch fetch user information for multiple users.
/// Essential for feed rendering to avoid N+1 queries to the Core DB.
pub async fn get_users_info_batch(
    core_pool: &PgPool,
    user_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, UserBridgeInfo>, AppError> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let rows = sqlx::query!(
        r#"
        SELECT u.id as "id!", up.display_name, u.avatar_url 
        FROM users u
        LEFT JOIN user_profiles up ON u.id = up.user_id
        WHERE u.id = ANY($1)
        "#,
        user_ids
    )
    .fetch_all(core_pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        map.insert(
            r.id,
            UserBridgeInfo {
                user_id: r.id,
                display_name: r
                    .display_name
                    .unwrap_or_else(|| "Anonymous User".to_string()),
                avatar_url: r.avatar_url,
            },
        );
    }

    Ok(map)
}
