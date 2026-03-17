use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;

use super::service;
use crate::auth::middleware;
use crate::auth::routes::AppState;

async fn require_user_id(
    jar: &CookieJar,
    state: &AppState,
) -> Result<uuid::Uuid, axum::response::Response> {
    match middleware::get_current_user(jar, &state.db).await {
        Some(user) => Ok(user.id),
        None => Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Not authenticated"})),
        )
            .into_response()),
    }
}

pub async fn get_portfolio_handler(
    jar: CookieJar,
    State(state): State<AppState>,
) -> axum::response::Response {
    let user_id = match require_user_id(&jar, &state).await {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match service::get_portfolio(&state.db, user_id).await {
        Ok(portfolio) => Json(portfolio).into_response(),
        Err(e) => {
            tracing::error!("Failed to get portfolio for user {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load portfolio."})),
            )
                .into_response()
        }
    }
}

/// GET /portfolio — Render the portfolio page.
pub async fn page_portfolio(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    let user = match middleware::get_current_user(&jar, &state.db).await {
        Some(u) => u,
        None => return axum::response::Redirect::to("/auth/login").into_response(),
    };

    let portfolio_json = match service::get_portfolio(&state.db, user.id).await {
        Ok(p) => serde_json::to_string(&p).unwrap_or_else(|_| "null".to_string()),
        Err(e) => {
            tracing::error!("Failed to fetch portfolio data for SSR: {}", e);
            "null".to_string()
        }
    };

    match state.templates.get_template("portfolio.html") {
        Ok(template) => {
            match template
                .render(minijinja::context! { user => user, portfolio_json => portfolio_json })
            {
                Ok(content) => axum::response::Html(content).into_response(),
                Err(e) => axum::response::Html(format!("<h1>Internal Server Error: {}</h1>", e))
                    .into_response(),
            }
        }
        Err(_) => axum::response::Html("<h1>Page not found</h1>".to_string()).into_response(),
    }
}

/// GET /transactions — Render the transactions list.
pub async fn page_transactions(jar: CookieJar, State(state): State<AppState>) -> impl IntoResponse {
    crate::common::routes_helper::serve_protected(jar, &state, "transactions.html").await
}
