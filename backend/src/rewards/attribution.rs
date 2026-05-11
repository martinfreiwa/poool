//! Affiliate attribution middleware — Phase 19.2.
//!
//! Captures `?ref=<code>` on any inbound request and writes a 30-day
//! HttpOnly cookie so the user is still attributed if they bounce
//! around the marketing site before registering. The signup handler
//! (`auth::routes::signup_submit`) already reads `REFERRAL_COOKIE` and
//! falls through to `attribute_affiliate_referral`, so wiring this
//! middleware is the only missing piece of the attribution funnel.
//!
//! Behavior:
//!   - Read `ref` from URI query string.
//!   - Strip to `[a-zA-Z0-9_-|]` and clamp to ≤80 chars (we accept the
//!     `code|subid|utm_source` composite format the signup handler
//!     already parses).
//!   - If the request already carries a non-empty `poool_referral` cookie,
//!     do NOT overwrite — first-touch attribution wins.
//!   - Set the cookie HttpOnly, SameSite=Lax, 30-day Max-Age.
//!
//! The middleware does NOT validate the code against the `affiliates`
//! table here — that validation already happens at registration time in
//! `attribute_affiliate_referral`. Cookie writes are cheap; an invalid
//! code just silently no-ops at signup, which is the correct shape
//! (avoid leaking which codes are valid via a different cookie response).

use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::IntoResponse,
};

const REFERRAL_COOKIE: &str = "poool_referral";
const MAX_REF_LEN: usize = 80;
const COOKIE_MAX_AGE_SECS: i64 = 60 * 60 * 24 * 30; // 30 days

/// Axum middleware that captures `?ref=<code>` and writes a 30-day cookie.
pub async fn capture_referral(req: Request, next: Next) -> impl IntoResponse {
    // Cheapest path: no query string → skip without parsing.
    let captured = req.uri().query().and_then(|q| {
        for pair in q.split('&') {
            let mut it = pair.splitn(2, '=');
            if it.next()? == "ref" {
                let raw = it.next().unwrap_or("");
                if raw.is_empty() {
                    return None;
                }
                // Decode + sanitize.
                let decoded = url_decode_pct(raw);
                let cleaned = sanitize(&decoded);
                if cleaned.is_empty() {
                    return None;
                }
                return Some(cleaned);
            }
        }
        None
    });

    // First-touch wins: if the request already carries a non-empty cookie,
    // don't overwrite. Re-clicks of a different affiliate's link should
    // NOT steal attribution from the user's earlier landing.
    let already_set = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|cookies| {
            cookies.split(';').any(|c| {
                let c = c.trim();
                c.starts_with(&format!("{}=", REFERRAL_COOKIE))
                    && c.len() > REFERRAL_COOKIE.len() + 1
            })
        })
        .unwrap_or(false);

    let mut response = next.run(req).await;

    if let Some(code) = captured {
        if !already_set {
            let cookie_str = format!(
                "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                REFERRAL_COOKIE, code, COOKIE_MAX_AGE_SECS
            );
            if let Ok(hv) = HeaderValue::from_str(&cookie_str) {
                response.headers_mut().append(header::SET_COOKIE, hv);
            }
        }
    }

    response
}

/// Strip everything outside `[A-Za-z0-9_\-|]` and clamp length.
fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len().min(MAX_REF_LEN));
    for ch in input.chars() {
        if out.len() >= MAX_REF_LEN {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '|' {
            out.push(ch);
        }
    }
    out
}

/// Minimal percent-decode for the subset of escapes we care about (alphanumerics,
/// `|`, `-`, `_`, `%`, space). Anything we don't recognise stays in place and is
/// then filtered out by `sanitize`.
fn url_decode_pct(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 2 < bytes.len() {
            let h = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]));
            if let (Some(hi), Some(lo)) = h {
                out.push((hi * 16 + lo) as char);
                i += 3;
                continue;
            }
        } else if b == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_unsafe_chars() {
        assert_eq!(sanitize("abc123"), "abc123");
        assert_eq!(sanitize("abc|sub|utm"), "abc|sub|utm");
        assert_eq!(sanitize("abc<script>"), "abcscript");
        assert_eq!(sanitize("a/b\\c"), "abc");
    }

    #[test]
    fn sanitize_clamps_length() {
        let long = "a".repeat(200);
        assert_eq!(sanitize(&long).len(), MAX_REF_LEN);
    }

    #[test]
    fn url_decode_handles_percent_escapes() {
        assert_eq!(url_decode_pct("a%7Cb"), "a|b");
        assert_eq!(url_decode_pct("plus+sign"), "plus sign");
        assert_eq!(url_decode_pct("noop"), "noop");
    }
}
