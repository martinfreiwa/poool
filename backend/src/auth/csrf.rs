use axum::{
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use std::collections::HashMap;
use url::form_urlencoded;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CsrfToken(pub String);

/// CSRF Protection Middleware
///
/// Implements Double Submit Cookie pattern:
/// 1. Reads `csrf_token` cookie. If missing, generates it and adds to `Set-Cookie`.
/// 2. For mutating requests (POST, PUT, DELETE, PATCH), checks for `X-CSRF-Token` header.
/// 3. If header is missing, checks query string for `csrf_token` (used for standard `<form>` submits).
/// 4. Validates the provided token against the cookie.
pub async fn csrf_middleware(
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();
    // Only skip CSRF for static asset paths and webhook endpoints (the latter
    // are authenticated by HMAC signature, not cookies). Locale-prefixed
    // routes ("/en/...", "/id/...") serve real mutating handlers and MUST
    // enforce CSRF — otherwise any form posted to "/en/auth/login" bypassed
    // the double-submit check entirely.
    if path.starts_with("/static/")
        || path.starts_with("/fonts/")
        || path.starts_with("/static/images/")
        || path.starts_with("/webhook")
        || path.starts_with("/api/webhooks/")
    {
        return Ok(next.run(req).await);
    }

    let mut current_token = jar.get("csrf_token").map(|c| c.value().to_string());

    let mut needs_cookie_set = false;

    // Generate token if not present
    if current_token.is_none() {
        let new_token = Uuid::new_v4().to_string();
        current_token = Some(new_token);
        needs_cookie_set = true;
    }

    let token = current_token.unwrap();
    req.extensions_mut().insert(CsrfToken(token.clone()));

    // Validate token for state-changing methods
    let method = req.method().clone();
    if [Method::POST, Method::PUT, Method::DELETE, Method::PATCH].contains(&method) {
        let mut is_valid = false;

        // 1. Check HTTP Header (used by HTMX and Fetch)
        if let Some(header_val) = req.headers().get("X-CSRF-Token") {
            if let Ok(header_str) = header_val.to_str() {
                if header_str == token {
                    is_valid = true;
                }
            }
        }

        // 2. Check form body (used by HTML forms with <input type="hidden" name="csrf_token">)
        if !is_valid {
            if let Some(content_type) = req.headers().get(header::CONTENT_TYPE) {
                if let Ok(ct_str) = content_type.to_str() {
                    if ct_str.starts_with("application/x-www-form-urlencoded") {
                        let (parts, body) = req.into_parts();
                        if let Ok(bytes) = axum::body::to_bytes(body, 2 * 1024 * 1024).await {
                            let params: HashMap<String, String> =
                                form_urlencoded::parse(&bytes).into_owned().collect();
                            if let Some(body_token) = params.get("csrf_token") {
                                if body_token == &token {
                                    is_valid = true;
                                }
                            }
                            req = Request::from_parts(parts, axum::body::Body::from(bytes));
                        } else {
                            req = Request::from_parts(parts, axum::body::Body::empty());
                        }
                    }
                }
            }
        }

        // 3. Check Query String if Header and Body are absent (used by plain HTML form submissions without body payload fallback)
        if !is_valid {
            if let Some(query) = req.uri().query() {
                let params: HashMap<String, String> = form_urlencoded::parse(query.as_bytes())
                    .into_owned()
                    .collect();
                if let Some(query_token) = params.get("csrf_token") {
                    if query_token == &token {
                        is_valid = true;
                    }
                }
            }
        }

        if !is_valid {
            tracing::warn!("CSRF token validation failed for {}", path);
            sentry::with_scope(
                |scope| {
                    scope.set_tag("security.event", "csrf_failure");
                    scope.set_tag("request.path", &path);
                    scope.set_tag("request.method", method.to_string());
                },
                || {
                    sentry::capture_message(
                        &format!("CSRF validation failed: {} {}", method, path),
                        sentry::Level::Warning,
                    );
                },
            );
            // Return JSON for API endpoints so the frontend can parse it
            if path.starts_with("/api/") {
                return Ok((
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({
                        "error": "CSRF token missing or invalid. Please refresh the page and try again.",
                        "success": false,
                        "message": "CSRF token missing or invalid. Please refresh the page and try again."
                    })),
                )
                    .into_response());
            }
            if req
                .headers()
                .get("HX-Request")
                .and_then(|value| value.to_str().ok())
                .map(|value| value.eq_ignore_ascii_case("true"))
                .unwrap_or(false)
            {
                return Ok((
                    StatusCode::FORBIDDEN,
                    Html(r#"<div class="auth-error-message" role="alert" aria-live="assertive" tabindex="-1">Security check failed. Please refresh the page and try again.</div>"#),
                )
                    .into_response());
            }
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // Process request
    let mut response = next.run(req).await;

    // Attach new cookie if it was generated
    if needs_cookie_set {
        let cookie = Cookie::build(("csrf_token", token.clone()))
            .path("/")
            .http_only(false) // Must be readable by Javascript for fetch/htmx generic appending
            .secure(cookie_is_secure())
            .same_site(axum_extra::extract::cookie::SameSite::Lax)
            // 24h max lifetime — a token stolen via XSS/subdomain is valid
            // for at most a day instead of a year. Forms will re-issue on
            // page load and long-lived sessions re-mint on next request.
            .max_age(time::Duration::days(1));

        let cookie_string = cookie.to_string();
        response.headers_mut().append(
            header::SET_COOKIE,
            cookie_string
                .parse()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
    }

    Ok(response)
}

/// Build a cookie that expires the existing `csrf_token` immediately.
///
/// Add this to the returned `CookieJar` on privilege changes (login, logout,
/// 2FA verify/enroll). The browser drops the old token; the next request
/// falls through `csrf_middleware` and receives a fresh token bound to the
/// new session. Prevents session-fixation reuse of a pre-login CSRF token.
pub fn rotation_cookie() -> Cookie<'static> {
    Cookie::build(("csrf_token", ""))
        .path("/")
        .http_only(false)
        .secure(cookie_is_secure())
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(0))
        .build()
}

fn cookie_is_secure() -> bool {
    // Explicitly non-secure if POOOL_ENV says dev/local
    if matches!(
        std::env::var("POOOL_ENV").as_deref(),
        Ok("development") | Ok("dev") | Ok("local")
    ) {
        return false;
    }
    // Also non-secure if BASE_URL is plain http:// (local dev default)
    if let Ok(base) = std::env::var("BASE_URL") {
        if base.starts_with("http://") {
            return false;
        }
    }
    true
}
