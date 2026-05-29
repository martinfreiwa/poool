use crate::auth::middleware;
use crate::auth::routes::AppState;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::cookie::CookieJar;
use minijinja::context;
use tracing::error;

fn user_display_name(email: &str) -> String {
    email
        .split('@')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("User")
        .to_string()
}

/// Helper: serve a protected HTML page from the platform frontend.
///
/// Checks the session cookie against the database.
/// Returns the HTML content if authenticated, redirects to login otherwise.
pub async fn serve_protected(
    jar: CookieJar,
    state: &AppState,
    file: &str,
) -> axum::response::Response {
    // Check authentication
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    // Fetch affiliate status
    let affiliate_status: String =
        sqlx::query_scalar("SELECT status FROM affiliates WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "unregistered".to_string());

    let is_developer = file.starts_with("developer");
    let user_display_name = user_display_name(&user.email);

    // Render using Minijinja to resolve {% include %}
    match state.templates.get_template(file) {
        Ok(template) => match template.render(context! {
            user => user,
            user_display_name => user_display_name,
            affiliate_status => affiliate_status,
            is_developer => is_developer,
        }) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                error!("Template rendering error for {}: {}", file, e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Internal Server Error</h1>".to_string()),
                )
                    .into_response()
            }
        },
        Err(e) => {
            error!("Template not found for {}: {}", file, e);
            (
                axum::http::StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response()
        }
    }
}

pub async fn serve_protected_with_context<T: serde::Serialize>(
    jar: CookieJar,
    state: &AppState,
    file: &str,
    extra_context: T,
) -> axum::response::Response {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return Redirect::to("/auth/login").into_response(),
    };

    let mut map = match serde_json::to_value(&extra_context) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    };
    if let Ok(u_val) = serde_json::to_value(&user) {
        map.insert("user".to_string(), u_val);
    }
    map.entry("user_display_name".to_string())
        .or_insert_with(|| serde_json::json!(user_display_name(&user.email)));

    let affiliate_status: String =
        sqlx::query_scalar("SELECT status FROM affiliates WHERE user_id = $1")
            .bind(user.id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "unregistered".to_string());
    map.insert(
        "affiliate_status".to_string(),
        serde_json::json!(affiliate_status),
    );
    map.entry("is_developer".to_string())
        .or_insert_with(|| serde_json::json!(file.starts_with("developer")));

    match state.templates.get_template(file) {
        Ok(template) => match template.render(map) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                // Walk the error source chain — MiniJinja's Display omits the
                // root cause when the failure was inside an include.
                let mut chain = format!("{:#}", e);
                let mut cur: Option<&dyn std::error::Error> = std::error::Error::source(&e);
                while let Some(src) = cur {
                    chain.push_str(&format!("\n caused by: {}", src));
                    cur = std::error::Error::source(src);
                }
                error!("Template rendering error for {}: {}", file, chain);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Internal Server Error</h1>".to_string()),
                )
                    .into_response()
            }
        },
        Err(e) => {
            error!("Template not found for {}: {}", file, e);
            (
                axum::http::StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response()
        }
    }
}

pub async fn serve_public_with_context<T: serde::Serialize>(
    jar: CookieJar,
    state: &AppState,
    file: &str,
    extra_context: T,
) -> axum::response::Response {
    let mut map = match serde_json::to_value(&extra_context) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    };

    // Optionally inject user if logged in, but DO NOT redirect if missing
    if let Some(user) = middleware::get_current_user(&jar, &state.db).await {
        if let Ok(u_val) = serde_json::to_value(&user) {
            map.insert("user".to_string(), u_val);
        }
        map.entry("user_display_name".to_string())
            .or_insert_with(|| serde_json::json!(user_display_name(&user.email)));

        let affiliate_status: String =
            sqlx::query_scalar("SELECT status FROM affiliates WHERE user_id = $1")
                .bind(user.id)
                .fetch_optional(&state.db)
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "unregistered".to_string());
        map.insert(
            "affiliate_status".to_string(),
            serde_json::json!(affiliate_status),
        );
    }

    map.entry("is_developer".to_string())
        .or_insert_with(|| serde_json::json!(file.starts_with("developer")));

    match state.templates.get_template(file) {
        Ok(template) => match template.render(map) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                error!("Template rendering error for public {}: {}", file, e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Internal Server Error</h1>".to_string()),
                )
                    .into_response()
            }
        },
        Err(e) => {
            error!("Template not found for {}: {}", file, e);
            (
                axum::http::StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response()
        }
    }
}
