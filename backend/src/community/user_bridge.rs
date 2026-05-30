use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBridgeInfo {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// Helper to build the display name dynamically if `display_name` is null.
fn build_display_name(
    display_name: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    email: Option<String>,
) -> String {
    if let Some(dn) = display_name.filter(|s| !s.trim().is_empty()) {
        return dn;
    }
    let first = first_name.unwrap_or_default();
    let last = last_name.unwrap_or_default();
    if first.is_empty() && last.is_empty() {
        email
            .as_deref()
            .and_then(|e| e.split('@').next())
            .unwrap_or("User")
            .to_string()
    } else {
        format!("{} {}", first, last).trim().to_string()
    }
}

fn normalize_email(email: impl Into<Option<String>>) -> Option<String> {
    email.into()
}

/// W3.4: derive a portfolio "tier" from total invested capital. Pure
/// function so we can compute it from any cents value (per-user batch sum
/// or a single-user sidebar lookup).
pub fn portfolio_tier_for_cents(cents: i64) -> Option<&'static str> {
    if cents >= 10_000_000 {
        Some("Platinum")
    }
    // ≥ $100k
    else if cents >= 1_000_000 {
        Some("Gold")
    }
    // ≥ $10k
    else if cents >= 100_000 {
        Some("Silver")
    }
    // ≥ $1k
    else if cents > 0 {
        Some("Bronze")
    } else {
        None
    }
}

/// W3.4: batch-resolve user_id → portfolio tier label. Single core-DB
/// query that sums purchase_value_cents per user. Skips users with zero
/// holdings so the resulting map only contains positive answers.
pub async fn get_portfolio_tiers_batch(
    core_pool: &PgPool,
    user_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, String>, AppError> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT user_id, COALESCE(SUM(purchase_value_cents), 0)::BIGINT
         FROM investments WHERE user_id = ANY($1)
         GROUP BY user_id",
    )
    .bind(user_ids)
    .fetch_all(core_pool)
    .await?;
    let mut out = std::collections::HashMap::with_capacity(rows.len());
    for (uid, total) in rows {
        if let Some(tier) = portfolio_tier_for_cents(total) {
            out.insert(uid, tier.to_string());
        }
    }
    Ok(out)
}

/// Fetches basic user information (Name + Avatar) from the Core DB.
/// Used to enrich community posts and comments on the fly.
/// FIX-F9: Caches in Redis for 5 minutes.
pub async fn get_user_info(
    core_pool: &PgPool,
    redis_pool: Option<&deadpool_redis::Pool>,
    user_id: Uuid,
) -> Result<UserBridgeInfo, AppError> {
    let cache_key = format!("community:user_bridge:{}", user_id);

    // 1. Try Redis Cache
    if let Some(pool) = redis_pool {
        if let Ok(mut conn) = pool.get().await {
            use redis::AsyncCommands;
            let cached: Option<String> = conn.get(&cache_key).await.unwrap_or(None);
            if let Some(json) = cached {
                if let Ok(info) = serde_json::from_str::<UserBridgeInfo>(&json) {
                    return Ok(info);
                }
            }
        }
    }

    // 2. Fallback to Core DB
    let row = sqlx::query!(
        r#"
        SELECT u.id as "id!", u.email, up.display_name, up.first_name, up.last_name, u.avatar_url 
        FROM users u
        LEFT JOIN user_profiles up ON u.id = up.user_id
        WHERE u.id = $1
        "#,
        user_id
    )
    .fetch_optional(core_pool)
    .await?;

    match row {
        Some(r) => {
            let info = UserBridgeInfo {
                user_id: r.id,
                display_name: build_display_name(
                    r.display_name,
                    r.first_name,
                    r.last_name,
                    normalize_email(r.email),
                ),
                avatar_url: r
                    .avatar_url
                    .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
            };

            // 3. Set Cache
            if let Some(pool) = redis_pool {
                if let Ok(mut conn) = pool.get().await {
                    use redis::AsyncCommands;
                    if let Ok(json) = serde_json::to_string(&info) {
                        let _: () = conn.set_ex(&cache_key, json, 300).await.unwrap_or(());
                    }
                }
            }

            Ok(info)
        }
        None => Err(AppError::NotFound("User not found in Core DB".to_string())),
    }
}

/// Batch fetch user information for multiple users.
/// Essential for feed rendering to avoid N+1 queries to the Core DB.
/// FIX-F9: Caches results in Redis (5 min TTL) to reduce DB load.
pub async fn get_users_info_batch(
    core_pool: &PgPool,
    redis_pool: Option<&deadpool_redis::Pool>,
    user_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, UserBridgeInfo>, AppError> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let mut result_map = std::collections::HashMap::with_capacity(user_ids.len());
    let mut missing_ids = Vec::new();

    // 1. Try Redis for each user if available
    if let Some(pool) = redis_pool {
        if let Ok(mut conn) = pool.get().await {
            use redis::AsyncCommands;
            let keys: Vec<String> = user_ids
                .iter()
                .map(|id| format!("community:user_bridge:{}", id))
                .collect();

            // Try mget
            let cached: Vec<Option<String>> = conn.mget(&keys).await.unwrap_or_default();
            // If the size is different, fallback entirely to avoiding index out of bounds
            if cached.len() == user_ids.len() {
                for (id, cached_json) in user_ids.iter().zip(cached.into_iter()) {
                    let mut found = false;
                    if let Some(json) = cached_json {
                        if let Ok(info) = serde_json::from_str::<UserBridgeInfo>(&json) {
                            result_map.insert(*id, info);
                            found = true;
                        }
                    }
                    if !found {
                        missing_ids.push(*id);
                    }
                }
            } else {
                // MGet failed or returned mismatched lengths -> fallback to all missing
                missing_ids.extend_from_slice(user_ids);
            }
        } else {
            missing_ids.extend_from_slice(user_ids);
        }
    } else {
        missing_ids.extend_from_slice(user_ids);
    }

    // 2. Fetch missing from DB
    if !missing_ids.is_empty() {
        let rows = sqlx::query!(
            r#"
            SELECT u.id as "id!", u.email, up.display_name, up.first_name, up.last_name, u.avatar_url 
            FROM users u
            LEFT JOIN user_profiles up ON u.id = up.user_id
            WHERE u.id = ANY($1)
            "#,
            &missing_ids[..]
        )
        .fetch_all(core_pool)
        .await?;

        // Extract and populate Redis in batch
        if let Some(pool) = redis_pool {
            if let Ok(mut conn) = pool.get().await {
                // Using a pipeline for efficient batch setting

                let mut p = redis::pipe();
                for r in &rows {
                    let info = UserBridgeInfo {
                        user_id: r.id,
                        display_name: build_display_name(
                            r.display_name.clone(),
                            r.first_name.clone(),
                            r.last_name.clone(),
                            normalize_email(r.email.clone()),
                        ),
                        avatar_url: r
                            .avatar_url
                            .clone()
                            .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
                    };
                    result_map.insert(r.id, info.clone());
                    let key = format!("community:user_bridge:{}", r.id);
                    if let Ok(json) = serde_json::to_string(&info) {
                        p.set_ex(&key, json, 300);
                    }
                }
                let _: () = p.query_async(&mut *conn).await.unwrap_or(());
            } else {
                for r in rows {
                    let info = UserBridgeInfo {
                        user_id: r.id,
                        display_name: build_display_name(
                            r.display_name,
                            r.first_name,
                            r.last_name,
                            normalize_email(r.email),
                        ),
                        avatar_url: r
                            .avatar_url
                            .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
                    };
                    result_map.insert(r.id, info);
                }
            }
        } else {
            for r in rows {
                let info = UserBridgeInfo {
                    user_id: r.id,
                    display_name: build_display_name(
                        r.display_name,
                        r.first_name,
                        r.last_name,
                        normalize_email(r.email),
                    ),
                    avatar_url: r
                        .avatar_url
                        .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
                };
                result_map.insert(r.id, info);
            }
        }
    }

    Ok(result_map)
}

/// Evicts the Redis cache entry for a single user's bridge info.
///
/// Call this after writing to `users.avatar_url` or `user_profiles.display_name`
/// so the community feed, comments, and any other surface that reads via
/// `get_user_info` / `get_users_info_batch` picks up the new value on next read
/// instead of serving stale data for up to 5 minutes.
///
/// Failures are intentionally swallowed: Redis is a cache, not a source of
/// truth. If eviction fails, the worst case is one user sees a stale avatar
/// for the 5-minute TTL window — never a write inconsistency.
pub async fn invalidate_user_cache(redis_pool: Option<&deadpool_redis::Pool>, user_id: Uuid) {
    let Some(pool) = redis_pool else {
        return;
    };
    let Ok(mut conn) = pool.get().await else {
        return;
    };
    use redis::AsyncCommands;
    let key = format!("community:user_bridge:{}", user_id);
    let _: Result<(), _> = conn.del(&key).await;
}
