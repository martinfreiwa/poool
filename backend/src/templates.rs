/// MiniJinja template engine setup.
///
/// Loads HTML templates from the frontend/platform directory and renders them
/// with dynamic context (error messages, user data, etc.)
use minijinja::{path_loader, Environment};
use std::sync::Arc;

pub type Templates = Arc<Environment<'static>>;

pub fn create_engine() -> Templates {
    let mut env = Environment::new();

    // HTML autoescape for all .html/.htm/.xml templates — prevents XSS when
    // user-controlled strings are interpolated without an explicit `|safe`.
    env.set_auto_escape_callback(|name| {
        if name.ends_with(".html") || name.ends_with(".htm") || name.ends_with(".xml") {
            minijinja::AutoEscape::Html
        } else {
            minijinja::AutoEscape::None
        }
    });

    // Dynamically load all templates from frontend platform directory
    env.set_loader(path_loader("../frontend/platform"));

    // Detect environment for cache busting. Default to production behavior
    // when POOOL_ENV / BASE_URL are unset — dev-mode defaults could leak
    // cache-busting timestamps or other dev-only behavior into prod.
    let is_dev = matches!(
        std::env::var("POOOL_ENV").as_deref(),
        Ok("development") | Ok("dev") | Ok("local")
    ) || std::env::var("BASE_URL")
        .map(|url| url.contains("localhost"))
        .unwrap_or(false);

    let version = if is_dev {
        // Use startup timestamp — templates call now_ms() for per-request busting
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string()
    } else {
        // Use static version in production
        "1.0.27".to_string()
    };

    env.add_global("asset_version", version.clone());
    env.add_global("is_dev", is_dev);

    // Per-request cache-bust function for dev. Templates use:
    //   ?v={{ now_ms() if is_dev else asset_version }}
    // Returns millisecond timestamp so each page load gets a fresh URL in dev.
    env.add_function("now_ms", || -> String {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string()
    });

    // Custom filter: format an integer with comma separators (e.g. 1334000 → "1,334,000")
    env.add_filter("format_number", |value: i64| -> String {
        crate::common::currency::format_thousands(value)
    });

    // Custom filter: split a string by a delimiter (default "\n"). Used by the
    // risk-notification component to render multi-line CMS text as separate
    // strategy-item cards.
    env.add_filter(
        "split",
        |value: String, delim: Option<String>| -> Vec<String> {
            let d = delim.unwrap_or_else(|| "\n".to_string());
            value.split(d.as_str()).map(|s| s.to_string()).collect()
        },
    );

    // Custom filter: percent-encode a string for safe inclusion in a URL
    // query parameter. The post-card partial uses it for Twitter/X share
    // intent links; without it MiniJinja errored with "unknown filter:
    // urlencode" and the whole feed list render aborted.
    env.add_filter("urlencode", |value: String| -> String {
        // Encode everything except RFC-3986 unreserved chars so the result
        // is safe inside both query strings and path segments.
        const UNRESERVED: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut out = String::with_capacity(value.len());
        for &b in value.as_bytes() {
            if UNRESERVED.contains(&b) {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{:02X}", b));
            }
        }
        out
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
