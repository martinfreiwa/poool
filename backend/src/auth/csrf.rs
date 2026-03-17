use axum::{
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use std::collections::HashMap;
use url::form_urlencoded;
use uuid::Uuid;

/// CSRF Protection Middleware
///
/// Implements Double Submit Cookie pattern:
/// 1. Reads `csrf_token` cookie. If missing, generates it and adds to `Set-Cookie`.
/// 2. For mutating requests (POST, PUT, DELETE, PATCH), checks for `X-CSRF-Token` header.
/// 3. If header is missing, checks query string for `csrf_token` (used for standard `<form>` submits).
/// 4. Validates the provided token against the cookie.
pub async fn csrf_middleware(
    jar: CookieJar,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if path.starts_with("/static/")
        || path.starts_with("/fonts/")
        || path.starts_with("/images/")
        || path.starts_with("/en/")
        || path.starts_with("/id/")
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

    // Validate token for state-changing methods
    let method = req.method();
    if [Method::POST, Method::PUT, Method::DELETE, Method::PATCH].contains(method) {
        let mut is_valid = false;

        // 1. Check HTTP Header (used by HTMX and Fetch)
        if let Some(header_val) = req.headers().get("X-CSRF-Token") {
            if let Ok(header_str) = header_val.to_str() {
                if header_str == token {
                    is_valid = true;
                }
            }
        }

        // 2. Check Query String if Header is absent (used by plain HTML form submissions)
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
                    scope.set_tag("request.path", path);
                    scope.set_tag("request.method", &method.to_string());
                },
                || {
                    sentry::capture_message(
                        &format!("CSRF validation failed: {} {}", method, path),
                        sentry::Level::Warning,
                    );
                },
            );
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
            .max_age(time::Duration::days(365));

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
