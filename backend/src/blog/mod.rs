/// Blog article, author, and category data models.
pub mod models;
/// HTTP route handlers for the blog API.
pub mod routes;
/// Business logic: database queries for articles, authors, categories.
pub mod service;

use crate::auth::routes::AppState;
use axum::{
    routing::{get, post, put},
    Router,
};

/// Compose all blog-domain routes into a single mountable [`Router`].
pub fn router() -> Router<AppState> {
    use routes::*;
    Router::new()
        // SSR pages (public, server-rendered HTML)
        .route("/blog", get(page_blog_index))
        .route("/blog/", get(page_blog_index))
        .route("/blog/:slug", get(page_blog_article))
        .route("/blog/category/:slug", get(page_blog_category))
        // Public JSON API endpoints
        .route("/api/blog/articles", get(list_articles))
        .route("/api/blog/articles/:slug", get(get_article))
        .route("/api/blog/categories", get(list_categories))
        .route("/api/blog/authors", get(list_authors))
        // Admin write endpoints (separate path prefix to avoid Axum param conflicts)
        .route("/api/blog/admin/articles", post(create_article))
        .route(
            "/api/blog/admin/articles/:id",
            put(update_article).delete(delete_article),
        )
        .route(
            "/api/blog/admin/articles/:id/publish",
            post(publish_article),
        )
}
