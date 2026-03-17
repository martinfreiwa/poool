use crate::auth::middleware;
use crate::auth::routes::AppState;
use axum::response::{Html, IntoResponse, Redirect};
use axum_extra::extract::cookie::CookieJar;
use minijinja::context;
use tracing::error;

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

    // Render using Minijinja to resolve {% include %}
    match state.templates.get_template(file) {
        Ok(template) => match template.render(context! { user => user }) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                error!("Template rendering error for {}: {}", file, e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Internal Server Error: {}</h1>", e)),
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

/// Helper: serve an admin-protected HTML page.
///
/// Checks if the user is authenticated AND has an 'admin' or 'super_admin' role.
/// If not an admin, redirects to the marketplace (if authenticated) or login.
pub async fn serve_admin_protected(
    jar: CookieJar,
    state: &AppState,
    file: &str,
) -> axum::response::Response {
    // Check authentication
    if !middleware::is_authenticated(&jar, &state.db).await {
        return Redirect::to("/auth/login").into_response();
    }

    // Check admin role
    if !middleware::is_admin(&jar, &state.db).await {
        tracing::warn!("Non-admin user attempted to access admin page: {}", file);
        return Redirect::to("/marketplace").into_response();
    }

    // Render using Minijinja
    match state.templates.get_template(file) {
        Ok(template) => match template.render(context! {}) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                error!("Template rendering error for admin {}: {}", file, e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Internal Server Error: {}</h1>", e)),
                )
                    .into_response()
            }
        },
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            Html("<h1>Page not found</h1>".to_string()),
        )
            .into_response(),
    }
}
