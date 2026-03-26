/// MiniJinja template engine setup.
///
/// Loads HTML templates from the frontend/platform directory and renders them
/// with dynamic context (error messages, user data, etc.)
use minijinja::{path_loader, Environment};
use std::sync::Arc;

pub type Templates = Arc<Environment<'static>>;

pub fn create_engine() -> Templates {
    let mut env = Environment::new();

    // Dynamically load all templates from frontend platform directory
    env.set_loader(path_loader("../frontend/platform"));

    // Detect environment for cache busting
    let is_dev = matches!(
        std::env::var("POOOL_ENV").as_deref(),
        Ok("development") | Ok("dev") | Ok("local")
    ) || std::env::var("BASE_URL")
        .map(|url| url.contains("localhost"))
        .unwrap_or(true); // default to dev when BASE_URL is not set

    let version = if is_dev {
        // Use current timestamp as version in dev mode to break cache
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string()
    } else {
        // Use static version in production
        "1.0.15".to_string()
    };

    env.add_global("asset_version", version.clone());
    env.add_global("is_dev", is_dev);

    // Custom filter: format an integer with comma separators (e.g. 1334000 → "1,334,000")
    env.add_filter("format_number", |value: i64| -> String {
        crate::common::currency::format_thousands(value)
    });

    // Auth templates are now loaded dynamically via path_loader from frontend/platform
    // This ensures consistency with the design system and shared head.html.
    tracing::info!(
        "Template engine initialized (dev_mode={}, version={}) ✓",
        is_dev,
        version
    );
    Arc::new(env)
}
