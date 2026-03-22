use super::extractors::AdminUser;
use crate::auth::routes::AppState;
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Redirect},
};

/// GET /admin/  Admin dashboard (protected, requires admin role).
pub async fn page_admin_dashboard(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // AdminUser extractor already verified admin access
    render_admin_template(&state, "admin/index.html")
}

/// GET /admin/{any}.html  Serve admin sub-pages (protected).
pub async fn page_admin_generic(
    _admin: AdminUser,
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    let path = req.uri().path();
    let relative = path.trim_start_matches('/');

    // Path traversal protection
    if relative.contains("..") || !relative.starts_with("admin/") {
        return Redirect::to("/admin/").into_response();
    }

    // If the path doesn't end with .html, append it so clean URLs resolve correctly
    let file = if relative.ends_with(".html") {
        relative.to_string()
    } else if relative.ends_with('/') {
        format!("{}index.html", relative)
    } else {
        format!("{}.html", relative)
    };

    render_admin_template(&state, &file)
}

/// Render an admin template. Admin access is assumed to be already verified
/// by the `AdminUser` extractor.
fn render_admin_template(state: &AppState, file: &str) -> axum::response::Response {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/poool_debug.txt")
    {
        let _ = writeln!(f, "Attempting to load template for: {}", file);
    }
    use axum::response::Html;
    match state.templates.get_template(file) {
        Ok(template) => match template.render(minijinja::context! {}) {
            Ok(content) => Html(content).into_response(),
            Err(e) => {
                tracing::error!("Template rendering error for admin {}: {}", file, e);
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Internal Server Error: {}</h1>", e)),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!("Template GET error for admin file {}: {:?}", file, e);
            (
                axum::http::StatusCode::NOT_FOUND,
                Html(format!("<h1>Page not found</h1><p>Debug info: Tried file '{}', minijinja error: {}</p>", file, e)),
            )
                .into_response()
        }
    }
}
