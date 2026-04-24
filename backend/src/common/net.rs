//! Network / proxy helpers — centralizes the logic for resolving a trusted
//! client IP. Untrusted `X-Forwarded-For` values must never be used for rate
//! limiting, lockout keys, audit logs, or geo decisions; otherwise an attacker
//! trivially bypasses per-IP controls by supplying their own header.
//!
//! Trust is gated on the `TRUST_PROXY_XFF` environment variable. Only set it
//! to `true` / `1` when the application sits behind a reverse proxy you
//! control (Cloud Run, Fly, nginx, Cloudflare in full-proxy) that strips and
//! rewrites client-supplied `X-Forwarded-For`. In every other environment
//! (direct bind, local dev, uncontrolled ingress) leave it unset so header
//! values are ignored.
use axum::http::HeaderMap;

/// Resolve a client identifier suitable for rate-limit bucketing and audit
/// logging. Returns the trusted IP when available, otherwise "unknown".
///
/// Using "unknown" as the fallback is intentional — it collapses all
/// untrusted requests into a single bucket, which is safer (shared rate limit)
/// than letting a spoofed IP carve out its own unshared bucket.
pub fn client_ip(headers: &HeaderMap) -> String {
    if !proxy_trust_enabled() {
        return "unknown".to_string();
    }
    // Leftmost X-Forwarded-For is the original client when a trusted proxy
    // appends its own hop to the chain (standard behavior).
    if let Some(val) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return val.to_string();
    }
    if let Some(val) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return val.to_string();
    }
    "unknown".to_string()
}

fn proxy_trust_enabled() -> bool {
    std::env::var("TRUST_PROXY_XFF")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        })
        .unwrap_or(false)
}
