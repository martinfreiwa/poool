//! Rate limiter for authentication endpoints.
//!
//! Supports two backends:
//! - **In-memory** (default): HashMap-based, resets on restart, not shared across instances.
//! - **Redis** (when REDIS_URL is set): Shared across Cloud Run instances, survives restarts.
//!
//! Both implement the same sliding-window algorithm: max N requests per key per window.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

// ─── Trait ──────────────────────────────────────────────────────

/// Abstract rate limiter interface.
#[async_trait::async_trait]
pub trait RateLimitBackend: Send + Sync {
    /// Check if a request from `key` should be allowed.
    /// Returns `Ok(remaining)` if allowed, `Err(retry_after_secs)` if rate-limited.
    async fn check(&self, key: &str) -> Result<usize, u64>;

    /// Clean up stale entries (no-op for Redis which uses TTL).
    async fn cleanup(&self);
}

// ─── In-Memory Backend ──────────────────────────────────────────

/// In-memory rate limiter using a HashMap.
pub struct InMemoryBackend {
    inner: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl InMemoryBackend {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }
    }
}

#[async_trait::async_trait]
impl RateLimitBackend for InMemoryBackend {
    async fn check(&self, key: &str) -> Result<usize, u64> {
        let mut map = self.inner.lock().await;
        let now = Instant::now();
        let cutoff = now - self.window;

        let timestamps = map.entry(key.to_string()).or_default();
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.max_requests {
            let oldest = timestamps.first().copied().unwrap_or(now);
            let retry_after = self.window.as_secs() - now.duration_since(oldest).as_secs();
            Err(retry_after.max(1))
        } else {
            timestamps.push(now);
            Ok(self.max_requests - timestamps.len())
        }
    }

    async fn cleanup(&self) {
        let mut map = self.inner.lock().await;
        let cutoff = Instant::now() - self.window;
        map.retain(|_, timestamps| {
            timestamps.retain(|t| *t > cutoff);
            !timestamps.is_empty()
        });
    }
}

// ─── Redis Backend ──────────────────────────────────────────────

/// Redis-backed rate limiter using sorted sets with sliding window.
///
/// Each key gets a Redis sorted set where members are unique request IDs
/// and scores are Unix timestamps in milliseconds. TTL is set on the key
/// to auto-expire after the window passes.
pub struct RedisBackend {
    pool: deadpool_redis::Pool,
    max_requests: usize,
    window: Duration,
}

impl RedisBackend {
    pub fn new(pool: deadpool_redis::Pool, max_requests: usize, window: Duration) -> Self {
        Self {
            pool,
            max_requests,
            window,
        }
    }
}

#[async_trait::async_trait]
impl RateLimitBackend for RedisBackend {
    async fn check(&self, key: &str) -> Result<usize, u64> {
        let redis_key = format!("rl:{}", key);
        let window_ms = self.window.as_millis() as i64;

        let mut conn = match self.pool.get().await {
            Ok(c) => c,
            Err(e) => {
                // If Redis is down, fail open (allow the request)
                tracing::warn!("Redis rate limiter unavailable, allowing request: {}", e);
                return Ok(self.max_requests);
            }
        };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_ms = now_ms - window_ms;

        // Use a Redis pipeline for atomicity:
        // 1. Remove expired entries (ZREMRANGEBYSCORE)
        // 2. Count current entries (ZCARD)
        // 3. Add new entry if under limit (ZADD)
        // 4. Set TTL on key (EXPIRE)
        let result: Result<(i64,), _> = redis::pipe()
            .atomic()
            .cmd("ZREMRANGEBYSCORE")
            .arg(&redis_key)
            .arg("-inf")
            .arg(cutoff_ms)
            .ignore()
            .cmd("ZCARD")
            .arg(&redis_key)
            .cmd("ZADD")
            .arg(&redis_key)
            .arg(now_ms)
            .arg(format!("{}:{}", now_ms, uuid::Uuid::new_v4().as_u128() & 0xFFFF))
            .ignore()
            .cmd("EXPIRE")
            .arg(&redis_key)
            .arg(self.window.as_secs() as i64 + 1)
            .ignore()
            .query_async(&mut *conn)
            .await;

        match result {
            Ok((count,)) => {
                if count as usize >= self.max_requests {
                    // Over limit — remove the entry we just added
                    let _: Result<(), _> = redis::cmd("ZREMRANGEBYSCORE")
                        .arg(&redis_key)
                        .arg(now_ms)
                        .arg(now_ms)
                        .query_async(&mut *conn)
                        .await;

                    // Calculate retry-after from oldest entry
                    let oldest: Result<Vec<(String, f64)>, _> = redis::cmd("ZRANGEBYSCORE")
                        .arg(&redis_key)
                        .arg("-inf")
                        .arg("+inf")
                        .arg("WITHSCORES")
                        .arg("LIMIT")
                        .arg(0i64)
                        .arg(1i64)
                        .query_async(&mut *conn)
                        .await;

                    let retry_after = match oldest {
                        Ok(entries) if !entries.is_empty() => {
                            let oldest_ms = entries[0].1 as i64;
                            let expires_ms = oldest_ms + window_ms;
                            ((expires_ms - now_ms) / 1000).max(1) as u64
                        }
                        _ => self.window.as_secs(),
                    };

                    Err(retry_after)
                } else {
                    Ok(self.max_requests - count as usize - 1)
                }
            }
            Err(e) => {
                tracing::warn!("Redis rate limit check failed, allowing request: {}", e);
                Ok(self.max_requests) // Fail open
            }
        }
    }

    async fn cleanup(&self) {
        // Redis handles cleanup via TTL, no-op
    }
}

// ─── Unified RateLimiter Wrapper ────────────────────────────────

/// Unified rate limiter that wraps either in-memory or Redis backend.
#[derive(Clone)]
pub struct RateLimiter {
    backend: Arc<dyn RateLimitBackend>,
}

impl RateLimiter {
    /// Create a new in-memory rate limiter (default).
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            backend: Arc::new(InMemoryBackend::new(max_requests, window)),
        }
    }

    /// Create a Redis-backed rate limiter.
    pub fn new_redis(pool: deadpool_redis::Pool, max_requests: usize, window: Duration) -> Self {
        Self {
            backend: Arc::new(RedisBackend::new(pool, max_requests, window)),
        }
    }

    /// Check if a request from `key` should be allowed.
    pub async fn check(&self, key: &str) -> Result<usize, u64> {
        self.backend.check(key).await
    }

    /// Periodically clean up stale entries.
    pub async fn cleanup(&self) {
        self.backend.cleanup().await;
    }
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.check("1.2.3.4").await.is_ok());
        assert!(limiter.check("1.2.3.4").await.is_ok());
        assert!(limiter.check("1.2.3.4").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        assert!(limiter.check("1.2.3.4").await.is_ok());
        assert!(limiter.check("1.2.3.4").await.is_ok());
        assert!(limiter.check("1.2.3.4").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.check("1.2.3.4").await.is_ok());
        assert!(limiter.check("5.6.7.8").await.is_ok()); // Different IP
        assert!(limiter.check("1.2.3.4").await.is_err()); // Same IP, over limit
    }
}
