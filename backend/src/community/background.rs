use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Periodically checks the post creation velocity of assets and flags if > 5 posts in 10 mins.
pub async fn monitor_asset_velocity(community_pool: PgPool, core_pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5)); // Every 5 minutes

    loop {
        interval.tick().await;

        if let Err(e) = check_velocity(&community_pool, &core_pool).await {
            tracing::error!("Failed to check asset velocity: {:?}", e);
        }
    }
}

async fn check_velocity(c_pool: &PgPool, core_pool: &PgPool) -> Result<(), crate::error::AppError> {
    use sqlx::Row;

    // Check last 10 minutes
    let rows = sqlx::query(
        "SELECT asset_id, count(*) as post_count \
         FROM posts \
         WHERE asset_id IS NOT NULL \
         AND created_at >= NOW() - INTERVAL '10 minutes' \
         GROUP BY asset_id \
         HAVING count(*) >= 5"
    )
    .fetch_all(c_pool)
    .await?;

    for row in rows {
        let asset_id: Uuid = row.try_get("asset_id")?;
        let count: i64 = row.try_get("post_count")?;

        tracing::warn!("Pump & Dump Warning: Asset {} has {} mentions in the last 10 minutes!", asset_id, count);

        // Alert Admins by creating a special admin notification or just logging for now.
        // If an admin_alerts table is added later, we can insert into it.
        tracing::error!("PUMP & DUMP ALERT: Asset {} mentioned {} times in 10 mins", asset_id, count);
    }

    Ok(())
}
