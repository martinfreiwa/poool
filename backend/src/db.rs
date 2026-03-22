/// Database connection pool initialization.
///
/// Phase 1.1: Dual pool setup (core_primary, core_replica, community)
/// Phase 1.2: Connection pool tuning (30 primary, 15 replica)
/// Phase 1.3: Read-your-writes pattern via Redis flag
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

// ─── Pool Tuning Constants (Phase 1.2) ──────────────────────────

/// Primary pool: handles all writes and reads-after-write.
const PRIMARY_MAX_CONNECTIONS: u32 = 30;
const PRIMARY_MIN_CONNECTIONS: u32 = 5;
const PRIMARY_ACQUIRE_TIMEOUT_SECS: u64 = 5;
const PRIMARY_IDLE_TIMEOUT_SECS: u64 = 120;

/// Replica pool: read-only queries that can tolerate slight lag.
const REPLICA_MAX_CONNECTIONS: u32 = 15;
const REPLICA_MIN_CONNECTIONS: u32 = 2;
const REPLICA_ACQUIRE_TIMEOUT_SECS: u64 = 5;
const REPLICA_IDLE_TIMEOUT_SECS: u64 = 120;

/// Community pool: separate database for community features.
const COMMUNITY_MAX_CONNECTIONS: u32 = 15;
const COMMUNITY_MIN_CONNECTIONS: u32 = 2;
const COMMUNITY_ACQUIRE_TIMEOUT_SECS: u64 = 5;
const COMMUNITY_IDLE_TIMEOUT_SECS: u64 = 120;

/// Redis key TTL for read-your-writes (Phase 1.3).
#[allow(dead_code)]
const RECENT_WRITE_TTL_SECS: u64 = 2;

// ─── Database Pools (Phase 1.1) ─────────────────────────────────

/// All configured database connection pools.
pub struct DatabasePools {
    /// Core primary pool – reads + writes. Always available.
    pub primary: PgPool,
    /// Core replica pool – read-only. Optional (falls back to primary).
    pub replica: Option<PgPool>,
    /// Community database pool – separate DB for community features. Optional.
    pub community: Option<PgPool>,
}

/// Build PgConnectOptions from a database URL.
/// On Cloud Run, we auto-detect the Cloud SQL Unix socket under /cloudsql/.
/// On local dev, we use a standard TCP connection from the DATABASE_URL.
///
/// When PgBouncer is enabled (PGBOUNCER_ENABLED=true), skip socket auto-detection
/// because PgBouncer handles the upstream connection to Cloud SQL. The backend
/// should connect to PgBouncer via TCP (127.0.0.1:6432).
fn build_connect_options(database_url: &str) -> PgConnectOptions {
    // If PgBouncer is enabled, the entrypoint.sh has already rewritten DATABASE_URL
    // to point to 127.0.0.1:6432. We must NOT auto-detect the Cloud SQL socket
    // or we'd bypass PgBouncer entirely, causing prepared statement conflicts.
    let pgbouncer_enabled = std::env::var("PGBOUNCER_ENABLED")
        .map(|v| v == "true")
        .unwrap_or(false);

    let socket_dir = if pgbouncer_enabled {
        tracing::info!("PgBouncer enabled — skipping Cloud SQL socket auto-detection");
        None
    } else {
        // Cloud Run + --add-cloudsql-instances mounts sockets at /cloudsql/<connection-name>
        // The CLOUD_SQL_SOCKET_PATH env var can override the auto-detection.
        std::env::var("CLOUD_SQL_SOCKET_PATH").ok().or_else(|| {
            let path = "/cloudsql";
            if std::path::Path::new(path).exists() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            let p = entry.path().to_string_lossy().into_owned();
                            tracing::info!("Auto-detected Cloud SQL socket dir: {}", p);
                            return Some(p);
                        }
                    }
                }
            }
            None
        })
    };

    if let Some(socket_path) = socket_dir {
        // Parse user/password/dbname from DATABASE_URL but switch to socket transport.
        // The DATABASE_URL in this case is only used for credentials, not the host.
        let base: PgConnectOptions = database_url
            .parse::<PgConnectOptions>()
            .unwrap_or_else(|_| PgConnectOptions::new())
            .statement_cache_capacity(0);

        tracing::info!("Using Cloud SQL Unix socket at: {}", socket_path);

        // .socket() sets the host to the directory; sqlx will look for .s.PGSQL.5432 inside it
        base.socket(socket_path)
    } else {
        // TCP connection — either local dev or PgBouncer proxy
        database_url
            .parse::<PgConnectOptions>()
            .expect("Invalid DATABASE_URL")
            .statement_cache_capacity(0)
    }
}

/// Create a single connection pool with the given tuning parameters.
/// Retries with exponential backoff for Cloud Run cold starts.
async fn create_pool_with_options(
    database_url: &str,
    label: &str,
    max_connections: u32,
    min_connections: u32,
    acquire_timeout_secs: u64,
    idle_timeout_secs: u64,
) -> PgPool {
    let connect_options = build_connect_options(database_url);
    tracing::info!("Connecting to {} database...", label);

    // Retry the initial connection with exponential backoff.
    // On Cloud Run the Cloud SQL Auth Proxy may need a few seconds to
    // establish the Unix-socket before our app can connect.  Without
    // retries the container panics immediately and Cloud Run marks the
    // revision as failed ("container failed to start").
    let max_attempts: u32 = 5;
    let mut attempt: u32 = 0;
    let pool = loop {
        attempt += 1;
        let opts = connect_options.clone();
        match PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(min_connections)
            .acquire_timeout(std::time::Duration::from_secs(acquire_timeout_secs))
            .idle_timeout(std::time::Duration::from_secs(idle_timeout_secs))
            .connect_with(opts)
            .await
        {
            Ok(p) => break p,
            Err(e) if attempt < max_attempts => {
                let backoff = std::time::Duration::from_secs(2u64.pow(attempt));
                tracing::warn!(
                    "{} database connection attempt {}/{} failed, retrying in {}s: {}",
                    label,
                    attempt,
                    max_attempts,
                    backoff.as_secs(),
                    e
                );
                tokio::time::sleep(backoff).await;
            }
            Err(e) => {
                panic!(
                    "Failed to connect to {} database after {} attempts. Is the database running? Last error: {}",
                    label, max_attempts, e
                );
            }
        }
    };

    // Verify connection works
    let mut verify_attempts = 0;
    loop {
        verify_attempts += 1;
        match sqlx::query("SELECT 1").execute(&pool).await {
            Ok(_) => {
                tracing::info!(
                    "{} database connection established ✓ (max={}, min={})",
                    label,
                    max_connections,
                    min_connections
                );
                break;
            }
            Err(e) if verify_attempts < 3 => {
                tracing::warn!(
                    "{} database connection test failed (attempt {}/3): {}",
                    label,
                    verify_attempts,
                    e
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                panic!(
                    "{} database connection test failed after 3 attempts: {}",
                    label, e
                );
            }
        }
    }

    pool
}

/// Backward-compatible: create a single primary pool from DATABASE_URL.
/// Used by existing code that only needs one pool.
#[allow(dead_code)]
pub async fn create_pool(database_url: &str) -> PgPool {
    create_pool_with_options(
        database_url,
        "primary",
        PRIMARY_MAX_CONNECTIONS,
        PRIMARY_MIN_CONNECTIONS,
        PRIMARY_ACQUIRE_TIMEOUT_SECS,
        PRIMARY_IDLE_TIMEOUT_SECS,
    )
    .await
}

/// Create all configured database pools (Phase 1.1).
///
/// - `primary` is always created from `DATABASE_URL`.
/// - `replica` is created from `DATABASE_REPLICA_URL` if set.
/// - `community` is created from `COMMUNITY_DATABASE_URL` if set.
pub async fn create_pools(config: &crate::config::Config) -> DatabasePools {
    let primary = create_pool_with_options(
        &config.database_url,
        "primary",
        PRIMARY_MAX_CONNECTIONS,
        PRIMARY_MIN_CONNECTIONS,
        PRIMARY_ACQUIRE_TIMEOUT_SECS,
        PRIMARY_IDLE_TIMEOUT_SECS,
    )
    .await;

    let replica = if let Some(ref url) = config.database_replica_url {
        tracing::info!("Replica database URL configured — creating read pool");
        Some(
            create_pool_with_options(
                url,
                "replica",
                REPLICA_MAX_CONNECTIONS,
                REPLICA_MIN_CONNECTIONS,
                REPLICA_ACQUIRE_TIMEOUT_SECS,
                REPLICA_IDLE_TIMEOUT_SECS,
            )
            .await,
        )
    } else {
        tracing::info!("No DATABASE_REPLICA_URL — reads will use primary pool");
        None
    };

    let community = if let Some(ref url) = config.community_database_url {
        tracing::info!("Community database URL configured — creating community pool");
        Some(
            create_pool_with_options(
                url,
                "community",
                COMMUNITY_MAX_CONNECTIONS,
                COMMUNITY_MIN_CONNECTIONS,
                COMMUNITY_ACQUIRE_TIMEOUT_SECS,
                COMMUNITY_IDLE_TIMEOUT_SECS,
            )
            .await,
        )
    } else {
        tracing::info!("No COMMUNITY_DATABASE_URL — community features disabled or using primary");
        None
    };

    DatabasePools {
        primary,
        replica,
        community,
    }
}

// ─── Read-Your-Writes (Phase 1.3) ──────────────────────────────

/// Return the best pool for read queries, respecting read-your-writes.
///
/// If a recent write flag exists in Redis for this user, returns the primary
/// pool to avoid stale reads from the replica. Otherwise returns the replica
/// (or primary if no replica is configured).
///
/// Falls back to primary if Redis is unavailable.
#[allow(dead_code)]
pub async fn read_pool<'a>(
    primary: &'a PgPool,
    replica: Option<&'a PgPool>,
    redis: Option<&deadpool_redis::Pool>,
    user_id: &uuid::Uuid,
) -> &'a PgPool {
    // No replica configured → always use primary
    let replica = match replica {
        Some(r) => r,
        None => return primary,
    };

    // Check Redis for recent-write flag
    if let Some(redis_pool) = redis {
        let key = format!("recent_write:{}", user_id);
        if let Ok(mut conn) = redis_pool.get().await {
            let exists: Result<i32, redis::RedisError> =
                redis::cmd("EXISTS").arg(&key).query_async(&mut *conn).await;
            match exists {
                Ok(1) => {
                    tracing::debug!(
                        "Read-your-writes: routing user {} to primary (recent write)",
                        user_id
                    );
                    return primary;
                }
                Ok(_) => {
                    // No recent write → safe to use replica
                }
                Err(e) => {
                    tracing::warn!("Redis EXISTS check failed, falling back to primary: {}", e);
                    return primary;
                }
            }
        } else {
            tracing::warn!(
                "Failed to get Redis connection for read routing, falling back to primary"
            );
            return primary;
        }
    } else {
        // No Redis → can't track writes, use replica optimistically
        // This is fine for non-financial reads
    }

    replica
}

/// Mark that a user recently performed a write operation.
///
/// Sets a Redis flag with a 2-second TTL so read queries for this user
/// are routed to the primary pool (read-your-writes consistency).
///
/// Silently no-ops if Redis is unavailable.
#[allow(dead_code)]
pub async fn mark_recent_write(redis: Option<&deadpool_redis::Pool>, user_id: &uuid::Uuid) {
    let redis_pool = match redis {
        Some(r) => r,
        None => return,
    };

    let key = format!("recent_write:{}", user_id);
    if let Ok(mut conn) = redis_pool.get().await {
        let result: Result<(), redis::RedisError> = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .arg("EX")
            .arg(RECENT_WRITE_TTL_SECS)
            .query_async(&mut *conn)
            .await;

        if let Err(e) = result {
            tracing::warn!(
                "Failed to set recent_write flag for user {}: {}",
                user_id,
                e
            );
        }
    }
}
