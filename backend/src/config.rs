//! Application configuration loaded from environment variables.

/// Default USD → IDR exchange rate (fallback when live API is unreachable).
/// Used by: cart display, checkout FX conversion, invoice generation, currency formatting.
/// In production, the live rate from `get_usd_to_idr_rate()` takes precedence.
pub const DEFAULT_USD_TO_IDR_RATE: f64 = 15_500.0;
pub const DEFAULT_USD_TO_IDR_RATE_I64: i64 = 15_500;

/// Hash a plaintext token with SHA-256 and return hex string.
/// Used for password reset tokens, email verification tokens, and admin invitation tokens.
/// The raw token is sent to the user (via email link); only the hash is stored in the DB.
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub base_url: String,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub facebook_app_id: Option<String>,
    pub facebook_app_secret: Option<String>,
    /// Didit.me KYC provider API key.
    pub didit_api_key: Option<String>,
    /// Didit.me workflow ID.
    pub didit_workflow_id: Option<String>,
    /// Didit.me webhook secret for HMAC verification.
    pub didit_webhook_secret: Option<String>,
    /// Redis URL for caching (optional).
    pub redis_url: Option<String>,
    /// Read-replica database URL (optional — falls back to primary).
    pub database_replica_url: Option<String>,
    /// Community database URL (optional — separate DB for community features).
    pub community_database_url: Option<String>,
    /// Sentry DSN for error and performance monitoring (optional).
    pub sentry_dsn: Option<String>,
    /// Application environment (development, staging, production).
    pub app_env: String,
    /// GCS bucket name for user file uploads (avatars, KYC docs, property images).
    /// Set GCS_BUCKET_NAME in the environment.  If absent, upload endpoints return 503.
    pub gcs_bucket: Option<String>,
    /// Blog content source. Use "sanity" for Sanity CDN-backed public blog reads.
    pub blog_content_source: String,
    /// Sanity project ID for public blog content.
    pub sanity_project_id: String,
    /// Sanity dataset for public blog content.
    pub sanity_dataset: String,
    /// Sanity API version used for CDN queries.
    pub sanity_api_version: String,
    /// Optional Sanity Studio/Manage URL opened by the admin blog dashboard.
    pub sanity_studio_url: String,
    /// Server-side Sanity token for private admin reads, including drafts.
    pub sanity_read_token: Option<String>,
    /// Server-side Sanity token for Content Lake mutations and asset uploads.
    pub sanity_write_token: Option<String>,
    /// Optional Metabase base URL for admin analytics, e.g. https://metabase.example.com.
    pub metabase_base_url: String,
    /// Optional public Metabase dashboard path used for embedding.
    pub metabase_public_dashboard_path: String,
    /// Optional internal Metabase dashboard ID opened by the admin button.
    pub metabase_dashboard_id: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let sanity_project_id = env_or("SANITY_PROJECT_ID", "3y7eud93");
        let sanity_studio_url = env_optional("SANITY_STUDIO_URL")
            .unwrap_or_else(|| format!("https://www.sanity.io/manage/project/{sanity_project_id}"));

        Self {
            database_url: env_required("DATABASE_URL"),
            server_host: env_or("SERVER_HOST", "0.0.0.0"),
            server_port: std::env::var("PORT")
                .or_else(|_| std::env::var("SERVER_PORT"))
                .unwrap_or_else(|_| "8888".to_string())
                .parse()
                .expect("PORT/SERVER_PORT must be a valid number"),
            base_url: env_or("BASE_URL", "http://localhost:8888"),
            google_client_id: env_optional("GOOGLE_CLIENT_ID"),
            google_client_secret: env_optional("GOOGLE_CLIENT_SECRET"),
            facebook_app_id: env_optional("FACEBOOK_APP_ID"),
            facebook_app_secret: env_optional("FACEBOOK_APP_SECRET"),
            didit_api_key: env_optional("DIDIT_API_KEY"),
            didit_workflow_id: env_optional("DIDIT_WORKFLOW_ID"),
            didit_webhook_secret: env_optional("DIDIT_WEBHOOK_SECRET"),
            redis_url: env_optional("REDIS_URL"),
            database_replica_url: env_optional("DATABASE_REPLICA_URL"),
            community_database_url: env_optional("COMMUNITY_DATABASE_URL"),
            sentry_dsn: env_optional("SENTRY_DSN"),
            app_env: env_or("APP_ENV", "development"),
            gcs_bucket: env_optional("GCS_BUCKET_NAME"),
            blog_content_source: env_or("BLOG_CONTENT_SOURCE", "sanity"),
            sanity_project_id,
            sanity_dataset: env_or("SANITY_DATASET", "production"),
            sanity_api_version: env_or("SANITY_API_VERSION", "2026-04-24"),
            sanity_studio_url,
            sanity_read_token: env_optional("SANITY_READ_TOKEN"),
            sanity_write_token: env_optional("SANITY_WRITE_TOKEN"),
            metabase_base_url: env_optional("METABASE_BASE_URL")
                .unwrap_or_default()
                .trim_end_matches('/')
                .to_string(),
            metabase_public_dashboard_path: env_optional("METABASE_PUBLIC_DASHBOARD_PATH")
                .unwrap_or_default(),
            metabase_dashboard_id: env_optional("METABASE_DASHBOARD_ID").unwrap_or_default(),
        }
    }

    /// Returns true if Google OAuth is properly configured.
    pub fn google_oauth_enabled(&self) -> bool {
        self.google_client_id.is_some() && self.google_client_secret.is_some()
    }

    /// Returns true if Facebook OAuth is properly configured.
    pub fn facebook_oauth_enabled(&self) -> bool {
        self.facebook_app_id.is_some() && self.facebook_app_secret.is_some()
    }
}

fn env_required(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{} must be set in .env or environment", key))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_optional(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}
