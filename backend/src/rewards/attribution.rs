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
// F21 fix: aligned with the 90-day first-conversion window enforced in
// check_and_track_affiliate_commission. A 30-day cookie silently broke
// attribution for any user who clicked but took 31-90 days to register.
const COOKIE_MAX_AGE_SECS: i64 = 60 * 60 * 24 * 90; // 90 days

/// Phase-2 P0: server-readable consent cookie. Set by
/// `static/js/cookie-consent.js` alongside the localStorage record.
///
/// Value is a `+`-joined flag list such as:
///   `essential`                              — strictly necessary only
///   `essential+analytics`                    — analytics opt-in
///   `essential+analytics+marketing`          — full opt-in
///   `essential+marketing`                    — marketing without analytics
///
/// We test for the substring `marketing` since that's the bucket affiliate
/// attribution lives in. Affiliate tracking is unambiguously marketing /
/// behavioural under ePrivacy + GDPR — no legitimate-interest fall-back.
pub const CONSENT_COOKIE: &str = "poool_consent";

/// Phase-3 P1: server-side request fingerprint.
///
/// Returns the lowercase hex SHA-256 of:
///   `<user-agent> || \n || <accept-language> || \n || <ip /24 prefix>`
///
/// The IP is reduced to its /24 subnet so a household NAT and a mobile
/// carrier (small CGNAT) share the same fingerprint as long as User-Agent
/// + Accept-Language match. That's the property we want — a legitimate
///   human's browser stays stable between landing and signup; a bot
///   rotating UA strings inside the same /24 won't.
///
/// Empty inputs are tolerated (returns the hash of empty headers). The
/// only failure mode is "missing all signal" which still yields a stable
/// (if unhelpful) hash that's at least matchable cross-row.
/// Phase-3 P1: lightweight crawler / bot detector.
///
/// Returns `true` when the request looks like an automated visitor we
/// should EXCLUDE from click attribution + the `referral_clicks` table.
/// Detection is intentionally narrow: we only catch self-identified bots
/// via well-known User-Agent substrings. Headless browsers that lie
/// about their UA are out of scope for this filter (covered by the
/// fingerprint anomaly scan instead).
///
/// List sourced from the union of:
///   * Google's crawler list (`googlebot`, `mediapartners-google`)
///   * Bing / Yandex / Baidu / DuckDuck / Apple
///   * Common social-share previewers (`facebookexternalhit`, `twitterbot`,
///     `linkedinbot`, `slackbot`, `discordbot`, `whatsapp`, `telegrambot`)
///   * Headless / scraper signatures (`headlesschrome`, `puppeteer`,
///     `selenium`, `phantomjs`, `playwright`, `httpclient`, `curl`,
///     `wget`, `python-requests`)
///   * Site-uptime monitors (`uptimerobot`, `pingdom`, `statuscake`)
pub fn is_bot_user_agent(ua: &str) -> bool {
    let ua = ua.to_ascii_lowercase();
    const BOT_SIGNATURES: &[&str] = &[
        // Search engines
        "googlebot",
        "mediapartners-google",
        "adsbot-google",
        "bingbot",
        "yandex",
        "baiduspider",
        "duckduckbot",
        "applebot",
        // Social previewers
        "facebookexternalhit",
        "facebookcatalog",
        "twitterbot",
        "linkedinbot",
        "slackbot",
        "discordbot",
        "whatsapp",
        "telegrambot",
        "skypeuripreview",
        // Headless / scrapers
        "headlesschrome",
        "puppeteer",
        "selenium",
        "phantomjs",
        "playwright",
        "httpclient",
        "python-requests",
        "go-http-client",
        "node-fetch",
        "okhttp",
        // Generic
        "bot/",
        "crawl",
        "spider",
        "scrap",
        "monitor",
        "curl/",
        "wget/",
        // Uptime monitors
        "uptimerobot",
        "pingdom",
        "statuscake",
        "siteimprove",
    ];
    BOT_SIGNATURES.iter().any(|sig| ua.contains(sig))
}

pub fn compute_request_fingerprint(headers: &axum::http::HeaderMap, ip: &str) -> String {
    use sha2::{Digest, Sha256};
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let lang = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // IP /24 reduction: drop the last octet for IPv4, drop the last
    // hextet for IPv6. Anything malformed → use as-is.
    let ip_class = if ip.contains(':') {
        // IPv6 — keep first 4 hextets (= /64 — proper subnet boundary).
        ip.split(':').take(4).collect::<Vec<_>>().join(":")
    } else {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() == 4 {
            format!("{}.{}.{}", parts[0], parts[1], parts[2])
        } else {
            ip.to_string()
        }
    };
    let mut h = Sha256::new();
    h.update(ua.as_bytes());
    h.update(b"\n");
    h.update(lang.as_bytes());
    h.update(b"\n");
    h.update(ip_class.as_bytes());
    let bytes = h.finalize();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for b in bytes.iter() {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

/// Returns `true` when the request carries explicit marketing consent.
/// Conservative default: missing cookie → `false`. ePrivacy Art. 5(3)
/// requires opt-in, not opt-out, so silence is rejection.
pub fn has_marketing_consent(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|cookies| {
            cookies.split(';').any(|c| {
                let c = c.trim();
                if let Some(v) = c.strip_prefix(&format!("{}=", CONSENT_COOKIE)) {
                    // Tolerate URL-encoded `+` (`%2B`) since some browsers
                    // encode the delimiter when writing via document.cookie.
                    v.to_ascii_lowercase().contains("marketing")
                } else {
                    false
                }
            })
        })
        .unwrap_or(false)
}

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

    // Phase-2 P0: ePrivacy gate. The referral cookie is behavioural
    // marketing, so we must NOT set it without explicit opt-in. The check
    // happens against the request headers (consent set by an earlier
    // banner interaction). No consent → silently skip; the `?ref` param
    // simply doesn't stick.
    let has_consent = has_marketing_consent(req.headers());

    let mut response = next.run(req).await;

    if let Some(code) = captured {
        if !already_set && has_consent {
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
