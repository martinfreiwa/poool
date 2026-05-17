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

    /// Atomic dual-key check (audit M#9). Both keys must be under their
    /// respective limits to allow the request. If either is over, the
    /// shared limit increment is rolled back for the other key — preserving
    /// the all-or-nothing semantics needed for dual-tier auth gates
    /// (IP-bucket × email-bucket).
    ///
    /// Two separate `check()` calls have a TOCTOU gap during which an
    /// attacker can burst between the IP and email checks, consuming a
    /// slot in the first but bypassing the second.
    async fn check_dual(&self, key_a: &str, key_b: &str) -> Result<(), u64>;

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

    async fn check_dual(&self, key_a: &str, key_b: &str) -> Result<(), u64> {
        // Hold ONE lock across both keys → no other check() can interleave.
        let mut map = self.inner.lock().await;
        let now = Instant::now();
        let cutoff = now - self.window;

        // Snapshot both buckets after expiring stale entries.
        let count_a = {
            let ts = map.entry(key_a.to_string()).or_default();
            ts.retain(|t| *t > cutoff);
            ts.len()
        };
        let count_b = {
            let ts = map.entry(key_b.to_string()).or_default();
            ts.retain(|t| *t > cutoff);
            ts.len()
        };

        if count_a >= self.max_requests {
            let oldest = map
                .get(key_a)
                .and_then(|v| v.first().copied())
                .unwrap_or(now);
            let retry_after = self.window.as_secs() - now.duration_since(oldest).as_secs();
            return Err(retry_after.max(1));
        }
        if count_b >= self.max_requests {
            let oldest = map
                .get(key_b)
                .and_then(|v| v.first().copied())
                .unwrap_or(now);
            let retry_after = self.window.as_secs() - now.duration_since(oldest).as_secs();
            return Err(retry_after.max(1));
        }

        // Both under limit — commit BOTH atomically (lock still held).
        map.get_mut(key_a).expect("entry exists").push(now);
        map.get_mut(key_b).expect("entry exists").push(now);
        Ok(())
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
                // Fail CLOSED for auth rate limiting. If the counter store is
                // unavailable we cannot distinguish "first attempt" from
                // "thousandth attempt", so allowing the request would hand
                // attackers a free brute-force window whenever Redis flaps.
                tracing::error!("Redis rate limiter unavailable, rejecting request: {}", e);
                return Err(30);
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
            .arg(format!(
                "{}:{}",
                now_ms,
                uuid::Uuid::new_v4().as_u128() & 0xFFFF
            ))
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
                // Fail CLOSED. See rationale above.
                tracing::error!("Redis rate limit check failed, rejecting request: {}", e);
                Err(30)
            }
        }
    }

    async fn check_dual(&self, key_a: &str, key_b: &str) -> Result<(), u64> {
        // Audit M#9: one Lua script — Redis evaluates atomically, no
        // pipeline race window. Script returns 0 on success, or the
        // retry-after seconds on failure (positive integer).
        const SCRIPT: &str = r#"
            local key_a = KEYS[1]
            local key_b = KEYS[2]
            local now_ms = tonumber(ARGV[1])
            local cutoff_ms = tonumber(ARGV[2])
            local max_req = tonumber(ARGV[3])
            local window_s = tonumber(ARGV[4])
            local member = ARGV[5]

            redis.call('ZREMRANGEBYSCORE', key_a, '-inf', cutoff_ms)
            redis.call('ZREMRANGEBYSCORE', key_b, '-inf', cutoff_ms)

            local count_a = redis.call('ZCARD', key_a)
            local count_b = redis.call('ZCARD', key_b)

            if count_a >= max_req then
                local oldest = redis.call('ZRANGE', key_a, 0, 0, 'WITHSCORES')
                local oldest_ms = tonumber(oldest[2]) or now_ms
                local retry = math.max(1, math.floor((oldest_ms + window_s * 1000 - now_ms) / 1000))
                return retry
            end
            if count_b >= max_req then
                local oldest = redis.call('ZRANGE', key_b, 0, 0, 'WITHSCORES')
                local oldest_ms = tonumber(oldest[2]) or now_ms
                local retry = math.max(1, math.floor((oldest_ms + window_s * 1000 - now_ms) / 1000))
                return retry
            end

            redis.call('ZADD', key_a, now_ms, member .. ':a')
            redis.call('ZADD', key_b, now_ms, member .. ':b')
            redis.call('EXPIRE', key_a, window_s + 1)
            redis.call('EXPIRE', key_b, window_s + 1)
            return 0
        "#;

        let redis_key_a = format!("rl:{}", key_a);
        let redis_key_b = format!("rl:{}", key_b);
        let window_ms = self.window.as_millis() as i64;

        let mut conn = match self.pool.get().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(
                    "Redis rate limiter unavailable, rejecting dual check: {}",
                    e
                );
                return Err(30);
            }
        };

        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_ms = now_ms - window_ms;
        let member = format!("{}:{}", now_ms, uuid::Uuid::new_v4().as_u128() & 0xFFFF);

        // Use raw EVAL (avoids the optional `script` cargo feature).
        let result: Result<i64, _> = redis::cmd("EVAL")
            .arg(SCRIPT)
            .arg(2_i64) // numkeys
            .arg(&redis_key_a)
            .arg(&redis_key_b)
            .arg(now_ms)
            .arg(cutoff_ms)
            .arg(self.max_requests as i64)
            .arg(self.window.as_secs() as i64)
            .arg(&member)
            .query_async(&mut *conn)
            .await;

        match result {
            Ok(0) => Ok(()),
            Ok(retry) => Err(retry as u64),
            Err(e) => {
                tracing::error!("Redis dual rate limit check failed, rejecting: {}", e);
                Err(30)
            }
        }
    }

    async fn cleanup(&self) {
        // Redis handles cleanup via TTL, no-op
    }
}

// ─── No-op Backend ──────────────────────────────────────────────

struct NoopBackend;

#[async_trait::async_trait]
impl RateLimitBackend for NoopBackend {
    async fn check(&self, _key: &str) -> Result<usize, u64> {
        Ok(usize::MAX)
    }
    async fn check_dual(&self, _key_a: &str, _key_b: &str) -> Result<(), u64> {
        Ok(())
    }
    async fn cleanup(&self) {}
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

    /// Create a no-op rate limiter that always allows requests.
    pub fn disabled() -> Self {
        Self {
            backend: Arc::new(NoopBackend),
        }
    }

    /// Check if a request from `key` should be allowed.
    pub async fn check(&self, key: &str) -> Result<usize, u64> {
        self.backend.check(key).await
    }

    /// Atomic dual-key check (audit M#9). Use this for two-tier auth
    /// gates (IP × email) so an attacker can't burst between separate
    /// `check()` calls.
    pub async fn check_dual(&self, key_a: &str, key_b: &str) -> Result<(), u64> {
        self.backend.check_dual(key_a, key_b).await
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

    #[tokio::test]
    async fn test_check_dual_allows_when_both_under_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        assert!(limiter.check_dual("ip:1", "email:a").await.is_ok());
        assert!(limiter.check_dual("ip:1", "email:a").await.is_ok());
    }

    #[tokio::test]
    async fn test_check_dual_blocks_when_first_key_over() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        // Burn the first key via single-check.
        assert!(limiter.check("ip:1").await.is_ok());
        // Dual now fails on ip:1 before touching email:a.
        assert!(limiter.check_dual("ip:1", "email:a").await.is_err());
    }

    #[tokio::test]
    async fn test_check_dual_blocks_when_second_key_over() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.check("email:a").await.is_ok());
        // ip:2 has capacity; email:a is full → dual fails.
        assert!(limiter.check_dual("ip:2", "email:a").await.is_err());
    }

    #[tokio::test]
    async fn test_check_dual_rollback_on_either_failure() {
        // Critical for audit M#9: if the second key is over, we must NOT
        // have consumed a slot on the first key. Two failed dual checks
        // with a never-yet-counted first key must still leave its bucket
        // with capacity for a follow-up direct check.
        let limiter = RateLimiter::new(2, Duration::from_secs(60));
        // Fill email bucket.
        assert!(limiter.check("email:a").await.is_ok());
        assert!(limiter.check("email:a").await.is_ok());
        // Now dual check with fresh IP — email side is full → dual fails,
        // but `ip:9` must NOT have been incremented.
        assert!(limiter.check_dual("ip:9", "email:a").await.is_err());
        assert!(limiter.check_dual("ip:9", "email:a").await.is_err());
        // Direct checks on ip:9 should still have full capacity.
        assert!(limiter.check("ip:9").await.is_ok());
        assert!(limiter.check("ip:9").await.is_ok());
        assert!(limiter.check("ip:9").await.is_err()); // now full
    }
}
