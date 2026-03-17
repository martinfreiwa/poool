use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use axum_extra::extract::cookie::CookieJar;
use minijinja::context;

use crate::auth::middleware;
use crate::auth::routes::AppState;

use super::service;

// ── SSR Page Handlers (public, no auth) ──────────────────────────

/// GET /blog — Blog index page (server-rendered HTML).
pub async fn page_blog_index(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let tag = params.get("tag").map(|s| s.as_str());

    // Fetch articles (gracefully degrade to empty if tables don't exist yet)
    let result = match service::list_articles(&state.db, page, 12, None, tag, false).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Blog index: failed to load articles: {}", e);
            super::models::PaginatedArticles {
                articles: vec![],
                total: 0,
                page: 1,
                per_page: 12,
                total_pages: 0,
            }
        }
    };

    // Fetch featured article
    let featured = service::list_articles(&state.db, 1, 1, None, None, true)
        .await
        .ok()
        .and_then(|f| f.articles.into_iter().next());

    // Fetch categories
    let categories = service::list_categories(&state.db)
        .await
        .unwrap_or_default();

    let base_url = state.config.base_url.clone();

    match state.templates.get_template("blog/index.html") {
        Ok(template) => match template.render(context! {
            articles => result.articles,
            total_pages => result.total_pages,
            page => page,
            featured => featured,
            categories => categories,
            base_url => base_url,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Blog index template render error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Render Error: {}</h1>", e)),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!("Blog index template not found: {}", e);
            (
                StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response()
        }
    }
}

/// GET /blog/:slug — Single article page (server-rendered).
pub async fn page_blog_article(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> axum::response::Response {
    let article = match service::get_article_by_slug(&state.db, &slug).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Html("<h1>Article not found</h1>".to_string()),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Blog article '{}': {}", slug, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<h1>Server Error</h1>".to_string()),
            )
                .into_response();
        }
    };

    let base_url = state.config.base_url.clone();
    let current_slug = article.slug.clone();

    // Fetch recent articles for right sidebar (up to 5, excluding current)
    let other_articles = service::get_recent_articles(&state.db, &current_slug, 5)
        .await
        .unwrap_or_default();

    // Fetch "also interested in" articles for the bottom (up to 3, excluding current)
    let related = service::get_recent_articles(&state.db, &current_slug, 3)
        .await
        .unwrap_or_default();

    match state.templates.get_template("blog/article.html") {
        Ok(template) => match template.render(context! {
            article => article,
            base_url => base_url,
            other_articles => other_articles,
            related => related,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Blog article template render error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Render Error: {}</h1>", e)),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!("Blog article template not found: {}", e);
            (
                StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response()
        }
    }
}

/// GET /blog/category/:slug — Category listing page (server-rendered).
pub async fn page_blog_category(
    State(state): State<AppState>,
    Path(category_slug): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);

    let result = match service::list_articles(
        &state.db,
        page,
        12,
        Some(&category_slug),
        None,
        false,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Blog category '{}': {}", category_slug, e);
            super::models::PaginatedArticles {
                articles: vec![],
                total: 0,
                page: 1,
                per_page: 12,
                total_pages: 0,
            }
        }
    };

    let categories = service::list_categories(&state.db)
        .await
        .unwrap_or_default();
    let base_url = state.config.base_url.clone();

    match state.templates.get_template("blog/index.html") {
        Ok(template) => match template.render(context! {
            articles => result.articles,
            total_pages => result.total_pages,
            page => page,
            categories => categories,
            active_category => category_slug,
            base_url => base_url,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Blog category template render error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("<h1>Render Error: {}</h1>", e)),
                )
                    .into_response()
            }
        },
        Err(e) => {
            tracing::error!("Blog category template not found: {}", e);
            (
                StatusCode::NOT_FOUND,
                Html("<h1>Page not found</h1>".to_string()),
            )
                .into_response()
        }
    }
}

/// GET /api/blog/articles — List published articles.
/// Query params: ?page=1&per_page=12&category=investment-guides&tag=bali&featured=true
pub async fn list_articles(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: i64 = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(12)
        .clamp(1, 100);
    let category = params.get("category").map(|s| s.as_str());
    let tag = params.get("tag").map(|s| s.as_str());
    let featured = params
        .get("featured")
        .map(|s| s == "true" || s == "1")
        .unwrap_or(false);

    match service::list_articles(&state.db, page, per_page, category, tag, featured).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => {
            tracing::error!("Failed to list blog articles: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load articles."})),
            )
                .into_response()
        }
    }
}

/// GET /api/blog/articles/:slug — Get a single article by slug.
pub async fn get_article(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> axum::response::Response {
    match service::get_article_by_slug(&state.db, &slug).await {
        Ok(Some(article)) => Json(article).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Article not found."})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get blog article '{}': {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load article."})),
            )
                .into_response()
        }
    }
}

/// GET /api/blog/categories — List all categories.
pub async fn list_categories(State(state): State<AppState>) -> axum::response::Response {
    match service::list_categories(&state.db).await {
        Ok(cats) => Json(serde_json::json!({"categories": cats})).into_response(),
        Err(e) => {
            tracing::error!("Failed to list blog categories: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load categories."})),
            )
                .into_response()
        }
    }
}

/// GET /api/blog/authors — List all authors.
pub async fn list_authors(State(state): State<AppState>) -> axum::response::Response {
    match service::list_authors(&state.db).await {
        Ok(authors) => Json(serde_json::json!({"authors": authors})).into_response(),
        Err(e) => {
            tracing::error!("Failed to list blog authors: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to load authors."})),
            )
                .into_response()
        }
    }
}

// ── Admin-only endpoints ──────────────────────────────────────────

/// Helper: require admin auth for write endpoints.
async fn require_admin(jar: &CookieJar, state: &AppState) -> Result<(), axum::response::Response> {
    if middleware::is_admin(jar, &state.db).await {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Admin access required."})),
        )
            .into_response())
    }
}

/// POST /api/blog/articles — Create a new article (admin only).
pub async fn create_article(
    jar: CookieJar,
    State(state): State<AppState>,
    Json(req): Json<super::models::CreateArticleRequest>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(&jar, &state).await {
        return resp;
    }

    // Basic validation
    if req.slug.is_empty() || req.title.is_empty() || req.excerpt.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "slug, title, and excerpt are required."})),
        )
            .into_response();
    }

    match service::create_article(&state.db, &req).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"id": id, "slug": req.slug, "status": "created"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to create blog article: {}", e);
            let msg = if e.to_string().contains("duplicate key") {
                "An article with this slug already exists."
            } else {
                "Failed to create article."
            };
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": msg})),
            )
                .into_response()
        }
    }
}

/// PUT /api/blog/articles/:id — Update an article (admin only).
pub async fn update_article(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<super::models::UpdateArticleRequest>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(&jar, &state).await {
        return resp;
    }

    match service::update_article(&state.db, &id, &req).await {
        Ok(true) => Json(serde_json::json!({"status": "updated", "id": id})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Article not found."})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to update blog article {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to update article."})),
            )
                .into_response()
        }
    }
}

/// DELETE /api/blog/articles/:id — Archive an article (admin only, soft delete).
pub async fn delete_article(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(&jar, &state).await {
        return resp;
    }

    match service::archive_article(&state.db, &id).await {
        Ok(true) => Json(serde_json::json!({"status": "archived", "id": id})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Article not found."})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to archive blog article {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to archive article."})),
            )
                .into_response()
        }
    }
}

/// POST /api/blog/articles/:id/publish — Publish an article (admin only).
pub async fn publish_article(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> axum::response::Response {
    if let Err(resp) = require_admin(&jar, &state).await {
        return resp;
    }

    match service::publish_article(&state.db, &id).await {
        Ok(true) => Json(serde_json::json!({
            "status": "published",
            "id": id,
            "message": "Article published successfully."
        }))
        .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Article not found or already published."})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to publish blog article {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to publish article."})),
            )
                .into_response()
        }
    }
}
