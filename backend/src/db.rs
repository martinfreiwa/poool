/// Database connection pool initialization.
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

/// Build PgConnectOptions from the DATABASE_URL.
/// On Cloud Run, we auto-detect the Cloud SQL Unix socket under /cloudsql/.
/// On local dev, we use a standard TCP connection from the DATABASE_URL.
fn build_connect_options(database_url: &str) -> PgConnectOptions {
    // Cloud Run + --add-cloudsql-instances mounts sockets at /cloudsql/<connection-name>
    // The CLOUD_SQL_SOCKET_PATH env var can override the auto-detection.
    let socket_dir = std::env::var("CLOUD_SQL_SOCKET_PATH").ok().or_else(|| {
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
    });

    if let Some(socket_path) = socket_dir {
        // Parse user/password/dbname from DATABASE_URL but switch to socket transport.
        // The DATABASE_URL in this case is only used for credentials, not the host.
        let base: PgConnectOptions = database_url
            .parse()
            .unwrap_or_else(|_| PgConnectOptions::new());

        tracing::info!("Using Cloud SQL Unix socket at: {}", socket_path);

        // .socket() sets the host to the directory; sqlx will look for .s.PGSQL.5432 inside it
        base.socket(socket_path)
    } else {
        // Standard TCP connection (local dev via DATABASE_URL)
        database_url.parse().expect("Invalid DATABASE_URL")
    }
}

pub async fn create_pool(database_url: &str) -> PgPool {
    let connect_options = build_connect_options(database_url);

    tracing::info!("Connecting to database...");

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
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(300))
            .connect_with(opts)
            .await
        {
            Ok(p) => break p,
            Err(e) if attempt < max_attempts => {
                let backoff = std::time::Duration::from_secs(2u64.pow(attempt));
                tracing::warn!(
                    "Database connection attempt {}/{} failed, retrying in {}s: {}",
                    attempt,
                    max_attempts,
                    backoff.as_secs(),
                    e
                );
                tokio::time::sleep(backoff).await;
            }
            Err(e) => {
                panic!(
                    "Failed to connect to PostgreSQL after {} attempts. Is the database running? Last error: {}",
                    max_attempts, e
                );
            }
        }
    };

    // Verify connection works with retry
    let mut verify_attempts = 0;
    loop {
        verify_attempts += 1;
        match sqlx::query("SELECT 1").execute(&pool).await {
            Ok(_) => {
                tracing::info!("Database connection established ✓");
                break;
            }
            Err(e) if verify_attempts < 3 => {
                tracing::warn!(
                    "Database connection test failed (attempt {}/3): {}",
                    verify_attempts,
                    e
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => {
                panic!("Database connection test failed after 3 attempts: {}", e);
            }
        }
    }

    pool
}
