use deadpool_redis::{Config, Pool, Runtime};

/// Initializes the Redis connection pool.
pub async fn init_pool(redis_url: Option<String>) -> Option<Pool> {
    if let Some(url) = redis_url {
        let cfg = Config::from_url(url);
        match cfg.create_pool(Some(Runtime::Tokio1)) {
            Ok(pool) => {
                tracing::info!("Redis pool initialized successfully.");
                Some(pool)
            }
            Err(e) => {
                tracing::warn!("Failed to create Redis pool: {}", e);
                None
            }
        }
    } else {
        tracing::info!("No REDIS_URL provided. Operating without caching backend.");
        None
    }
}
