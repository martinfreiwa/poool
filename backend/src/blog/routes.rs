#![allow(clippy::items_after_test_module)]

use axum::{
    extract::{Multipart, Path, State},
    http::{header::CONTENT_TYPE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
};
use minijinja::context;
use serde::Deserialize;
use serde_json::json;

use crate::admin::extractors::{AdminUser, ApiError};
use crate::auth::middleware;
use crate::auth::routes::AppState;

use super::{
    sanity::{AdminArticleInput, AdminTaxonomyInput, SanityClient},
    service,
};

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

    let result = match list_articles_for_source(&state, page, 12, None, tag, false).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Blog index: failed to load articles: {}", e);
            return blog_unavailable_response();
        }
    };

    // Fetch featured article
    let featured = list_articles_for_source(&state, 1, 1, None, None, true)
        .await
        .ok()
        .and_then(|f| f.articles.into_iter().next());

    let categories = match list_categories_for_source(&state).await {
        Ok(categories) => categories,
        Err(e) => {
            tracing::error!("Blog index: failed to load categories: {}", e);
            return blog_unavailable_response();
        }
    };

    let base_url = state.config.base_url.clone();
    let page_title = "Blog".to_string();
    let page_description =
        "Expert insights on fractional real estate investing, asset tokenization, and building wealth with POOOL."
            .to_string();
    let canonical_path = "/blog".to_string();

    match state.templates.get_template("blog/index.html") {
        Ok(template) => match template.render(context! {
            articles => result.articles,
            total_pages => result.total_pages,
            page => page,
            featured => featured,
            categories => categories,
            active_category_detail => None::<super::models::CategoryResponse>,
            page_title => page_title,
            page_description => page_description,
            canonical_path => canonical_path,
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
    if !is_safe_public_slug(&slug) {
        return (
            StatusCode::NOT_FOUND,
            Html("<h1>Article not found</h1>".to_string()),
        )
            .into_response();
    }

    let article = match get_article_for_source(&state, &slug).await {
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
    let canonical_url = effective_canonical_url(
        article.canonical_url.as_deref(),
        &base_url,
        &format!("/blog/{}", article.slug),
    );
    let meta_title = non_empty(article.meta_title.as_deref())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{} | POOOL Blog", article.title));
    let meta_description = non_empty(article.meta_description.as_deref())
        .or_else(|| non_empty(Some(article.excerpt.as_str())))
        .unwrap_or("Insights on fractional real estate investing, tokenization, and building wealth with POOOL.")
        .to_string();
    let og_image_url = non_empty(article.og_image_url.as_deref())
        .or_else(|| non_empty(article.cover_image_url.as_deref()))
        .map(str::to_string);
    let blog_schema_json = blog_posting_schema(&article, &canonical_url);
    let breadcrumb_schema_json = breadcrumb_schema(&article, &canonical_url, &base_url);
    let faq_schema_json = faq_schema(article.faq_data.as_ref());

    // Fetch recent articles for right sidebar (up to 5, excluding current)
    let other_articles = get_recent_articles_for_source(&state, &current_slug, 5)
        .await
        .unwrap_or_default();

    // Fetch "also interested in" articles for the bottom (up to 3, excluding current)
    let related = get_recent_articles_for_source(&state, &current_slug, 3)
        .await
        .unwrap_or_default();

    match state.templates.get_template("blog/article.html") {
        Ok(template) => match template.render(context! {
            article => article,
            base_url => base_url,
            meta_title => meta_title,
            meta_description => meta_description,
            canonical_url => canonical_url,
            og_image_url => og_image_url,
            blog_schema_json => blog_schema_json,
            breadcrumb_schema_json => breadcrumb_schema_json,
            faq_schema_json => faq_schema_json,
            other_articles => other_articles,
            related => related,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Blog article template render error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Internal Server Error</h1>".to_string()),
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
    if !is_safe_public_slug(&category_slug) {
        return (
            StatusCode::NOT_FOUND,
            Html("<h1>Category not found</h1>".to_string()),
        )
            .into_response();
    }

    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);

    let result =
        match list_articles_for_source(&state, page, 12, Some(&category_slug), None, false).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Blog category '{}': {}", category_slug, e);
                return blog_unavailable_response();
            }
        };

    let categories = match list_categories_for_source(&state).await {
        Ok(categories) => categories,
        Err(e) => {
            tracing::error!(
                "Blog category '{}': failed to load categories: {}",
                category_slug,
                e
            );
            return blog_unavailable_response();
        }
    };
    let active_category_detail = categories
        .iter()
        .find(|category| category.slug == category_slug)
        .cloned();
    let page_title = active_category_detail
        .as_ref()
        .map(|category| format!("{} Articles", category.name))
        .unwrap_or_else(|| "Blog Category".to_string());
    let page_description = active_category_detail
        .as_ref()
        .and_then(|category| {
            category
                .meta_description
                .as_deref()
                .or(category.description.as_deref())
        })
        .unwrap_or("Expert insights from the POOOL blog.")
        .to_string();
    let canonical_path = format!("/blog/category/{}", category_slug);
    let base_url = state.config.base_url.clone();

    match state.templates.get_template("blog/index.html") {
        Ok(template) => match template.render(context! {
            articles => result.articles,
            total_pages => result.total_pages,
            page => page,
            categories => categories,
            active_category => category_slug,
            active_category_detail => active_category_detail,
            page_title => page_title,
            page_description => page_description,
            canonical_path => canonical_path,
            base_url => base_url,
        }) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Blog category template render error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<h1>Internal Server Error</h1>".to_string()),
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

fn is_safe_public_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|item| !item.is_empty())
}

fn effective_canonical_url(explicit: Option<&str>, base_url: &str, path: &str) -> String {
    if let Some(url) = non_empty(explicit) {
        return url.to_string();
    }
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

fn blog_posting_schema(article: &super::models::ArticleResponse, canonical_url: &str) -> String {
    let mut schema = json!({
        "@context": "https://schema.org",
        "@type": article.schema_type,
        "headline": article.title,
        "description": article.excerpt,
        "author": {
            "@type": "Person",
            "name": article.author.name,
        },
        "publisher": {
            "@type": "Organization",
            "name": "POOOL",
            "logo": {
                "@type": "ImageObject",
                "url": "https://poool.finance/static/images/icons/logo-pool.svg",
            },
        },
        "datePublished": article.published_at,
        "dateModified": article.updated_at,
        "mainEntityOfPage": canonical_url,
    });

    if let Some(image) = non_empty(article.cover_image_url.as_deref()) {
        schema["image"] = json!(image);
    }
    if let Some(author_url) = non_empty(article.author.website_url.as_deref()) {
        schema["author"]["url"] = json!(author_url);
    }

    serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string())
}

fn breadcrumb_schema(
    article: &super::models::ArticleResponse,
    canonical_url: &str,
    base_url: &str,
) -> String {
    let base = base_url.trim_end_matches('/');
    serde_json::to_string(&json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": [
            { "@type": "ListItem", "position": 1, "name": "Home", "item": format!("{}/", base) },
            { "@type": "ListItem", "position": 2, "name": "Blog", "item": format!("{}/blog", base) },
            {
                "@type": "ListItem",
                "position": 3,
                "name": article.category.name,
                "item": format!("{}/blog/category/{}", base, article.category.slug),
            },
            { "@type": "ListItem", "position": 4, "name": article.title, "item": canonical_url },
        ],
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn faq_schema(faq_data: Option<&serde_json::Value>) -> Option<String> {
    let items = faq_data?.as_array()?;
    let main_entity: Vec<serde_json::Value> = items
        .iter()
        .filter_map(|item| {
            let question = item.get("q").and_then(serde_json::Value::as_str)?.trim();
            let answer = item.get("a").and_then(serde_json::Value::as_str)?.trim();
            if question.is_empty() || answer.is_empty() {
                return None;
            }
            Some(json!({
                "@type": "Question",
                "name": question,
                "acceptedAnswer": {
                    "@type": "Answer",
                    "text": answer,
                },
            }))
        })
        .collect();

    if main_entity.is_empty() {
        return None;
    }

    serde_json::to_string(&json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": main_entity,
    }))
    .ok()
}

#[derive(Debug, Deserialize)]
pub struct BlogNewsletterRequest {
    email: String,
}

/// GET /blog/feed.xml — Public RSS feed for published blog articles.
pub async fn blog_feed_xml(State(state): State<AppState>) -> axum::response::Response {
    let base_url = state.config.base_url.trim_end_matches('/').to_string();
    let articles = match list_articles_for_source(&state, 1, 50, None, None, false).await {
        Ok(result) => result.articles,
        Err(e) => {
            tracing::error!("Blog feed: failed to load articles: {}", e);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [(CONTENT_TYPE, "application/xml; charset=utf-8")],
                r#"<?xml version="1.0" encoding="UTF-8"?><error>Blog feed temporarily unavailable</error>"#,
            )
                .into_response();
        }
    };

    let xml = render_blog_rss(&base_url, &articles);
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        xml,
    )
        .into_response()
}

fn blog_unavailable_response() -> axum::response::Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Html(
            r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Blog temporarily unavailable | POOOL</title>
    <meta name="robots" content="noindex" />
    <link rel="stylesheet" href="/static/css/fonts.css" />
    <link rel="stylesheet" href="/static/css/blog.css" />
</head>
<body>
    <main class="blog-main">
        <section class="blog-empty blog-empty--error" role="status" aria-live="polite">
            <h1>Blog temporarily unavailable</h1>
            <p>We could not load the latest articles right now. Please try again shortly.</p>
            <a href="/blog" class="blog-pagination__btn">Retry</a>
        </section>
    </main>
</body>
</html>"#
                .to_string(),
        ),
    )
        .into_response()
}

fn render_blog_rss(base_url: &str, articles: &[super::models::ArticleResponse]) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
"#,
    );
    xml.push_str("<title>POOOL Blog</title>\n");
    xml.push_str("<link>");
    xml.push_str(&xml_escape(&format!("{base_url}/blog")));
    xml.push_str("</link>\n");
    xml.push_str("<description>Insights on fractional real estate investing, asset tokenization, and building wealth with POOOL.</description>\n");
    xml.push_str("<language>en</language>\n");
    xml.push_str("<atom:link href=\"");
    xml.push_str(&xml_escape(&format!("{base_url}/blog/feed.xml")));
    xml.push_str("\" rel=\"self\" type=\"application/rss+xml\" />\n");
    xml.push_str("<lastBuildDate>");
    xml.push_str(&chrono::Utc::now().to_rfc2822());
    xml.push_str("</lastBuildDate>\n");

    for article in articles {
        let link = format!("{base_url}/blog/{}", article.slug);
        xml.push_str("<item>\n");
        xml.push_str("<title>");
        xml.push_str(&xml_escape(&article.title));
        xml.push_str("</title>\n");
        xml.push_str("<link>");
        xml.push_str(&xml_escape(&link));
        xml.push_str("</link>\n");
        xml.push_str("<guid isPermaLink=\"true\">");
        xml.push_str(&xml_escape(&link));
        xml.push_str("</guid>\n");
        xml.push_str("<description>");
        xml.push_str(&xml_escape(&article.excerpt));
        xml.push_str("</description>\n");
        xml.push_str("<category>");
        xml.push_str(&xml_escape(&article.category.name));
        xml.push_str("</category>\n");
        if let Some(pub_date) = article.published_at.as_deref().and_then(rss_pub_date) {
            xml.push_str("<pubDate>");
            xml.push_str(&pub_date);
            xml.push_str("</pubDate>\n");
        }
        xml.push_str("</item>\n");
    }

    xml.push_str("</channel>\n</rss>\n");
    xml
}

fn rss_pub_date(value: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.to_rfc2822())
        .or_else(|_| {
            chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%#z")
                .map(|dt| dt.to_rfc2822())
        })
        .or_else(|_| {
            chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%z")
                .map(|dt| dt.to_rfc2822())
        })
        .ok()
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blog::models::{ArticleResponse, ArticleShareLinks, AuthorSummary, CategorySummary};

    fn sample_article() -> ArticleResponse {
        ArticleResponse {
            id: "article-1".to_string(),
            slug: "rss-test".to_string(),
            title: "R&D <Update>".to_string(),
            subtitle: None,
            excerpt: "Fractional ownership & yield updates".to_string(),
            content: None,
            content_html: None,
            meta_title: None,
            meta_description: None,
            canonical_url: None,
            og_image_url: None,
            author: AuthorSummary {
                id: "author-1".to_string(),
                name: "POOOL".to_string(),
                initials: "P".to_string(),
                slug: "poool".to_string(),
                avatar_url: None,
                bio: None,
                website_url: None,
                twitter_handle: None,
                linkedin_url: None,
                facebook_url: None,
                instagram_url: None,
                whatsapp: None,
            },
            category: CategorySummary {
                id: "category-1".to_string(),
                name: "Market & Insights".to_string(),
                slug: "market-insights".to_string(),
                color: None,
                icon: None,
            },
            tags: vec![],
            cover_image_url: None,
            reading_time_minutes: 4,
            featured: false,
            share_links: ArticleShareLinks::default(),
            schema_type: "BlogPosting".to_string(),
            faq_data: None,
            published_at: Some("2026-04-28T08:00:00Z".to_string()),
            created_at: "2026-04-28T08:00:00Z".to_string(),
            updated_at: "2026-04-28T08:00:00Z".to_string(),
        }
    }

    #[test]
    fn rss_feed_escapes_article_fields() {
        let xml = render_blog_rss("https://platform.poool.app", &[sample_article()]);

        assert!(xml.contains("<rss version=\"2.0\""));
        assert!(xml.contains("https://platform.poool.app/blog/rss-test"));
        assert!(xml.contains("R&amp;D &lt;Update&gt;"));
        assert!(xml.contains("Fractional ownership &amp; yield updates"));
        assert!(xml.contains("Market &amp; Insights"));
        assert!(xml.contains("<pubDate>Tue, 28 Apr 2026 08:00:00 +0000</pubDate>"));
    }

    #[test]
    fn public_slug_rejects_path_like_values() {
        assert!(is_safe_public_slug("market-insights"));
        assert!(is_safe_public_slug("market_insights"));
        assert!(!is_safe_public_slug(""));
        assert!(!is_safe_public_slug("../admin"));
        assert!(!is_safe_public_slug("market/insights"));
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

    match list_articles_for_source(&state, page, per_page, category, tag, featured).await {
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
    match get_article_for_source(&state, &slug).await {
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
    match list_categories_for_source(&state).await {
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
    match list_authors_for_source(&state).await {
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

/// POST /api/blog/newsletter — Persist a public newsletter signup request.
pub async fn subscribe_newsletter(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<BlogNewsletterRequest>,
) -> axum::response::Response {
    let email = req.email.trim().to_lowercase();
    if let Err(err) = crate::common::validation::validate_email(&email) {
        return err.into_response();
    }

    let client_ip = crate::common::net::client_ip(&headers);
    for key in [
        format!("blog_newsletter:ip:{}", client_ip),
        format!("blog_newsletter:email:{}", email),
    ] {
        if let Err(retry_after) = state.auth_rate_limiter.check(&key).await {
            return crate::error::AppError::RateLimited(retry_after).into_response();
        }
    }

    let metadata = serde_json::json!({
        "source": "blog_article_newsletter",
        "delivery": "queued_for_marketing_export",
    });

    if let Err(err) = sqlx::query(
        r#"INSERT INTO email_logs (subject, recipient_email, status, error_message)
           VALUES ($1, $2, 'queued', $3)"#,
    )
    .bind("POOOL Blog newsletter signup")
    .bind(&email)
    .bind(metadata.to_string())
    .execute(&state.db)
    .await
    {
        tracing::error!("Failed to persist blog newsletter signup: {}", err);
        return crate::error::AppError::Database(err).into_response();
    }

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "queued",
            "message": "Subscription request received."
        })),
    )
        .into_response()
}

/// GET /api/admin/blog/overview — Admin blog dashboard metrics.
pub async fn admin_blog_overview(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.view").await?;

    let client = SanityClient::from_config(&state.config);
    let mut overview = client.admin_overview().await.map_err(|e| {
        tracing::error!("Failed to load Sanity blog overview: {:?}", e);
        ApiError::Internal("Failed to load blog overview".to_string())
    })?;
    if overview.private_reads_enabled {
        if let Ok(list) = client.admin_list_articles(1, 100, None, None, None).await {
            overview.published_count = list
                .articles
                .iter()
                .filter(|article| article.status == "published")
                .count() as i64;
            overview.draft_count = Some(
                list.articles
                    .iter()
                    .filter(|article| {
                        article.status == "draft" || article.status == "changes_pending"
                    })
                    .count() as i64,
            );
        }
    }

    Ok(Json(serde_json::json!({ "overview": overview })))
}

/// GET /api/admin/blog/articles — Admin list of published Sanity articles.
pub async fn admin_blog_articles(
    admin: AdminUser,
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.view").await?;

    let page: i64 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: i64 = params
        .get("per_page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(50)
        .clamp(1, 100);
    let category = params.get("category").map(|s| s.as_str());
    let status = params.get("status").map(|s| s.as_str());
    let search = params.get("q").map(|s| s.as_str());

    let client = SanityClient::from_config(&state.config);
    let articles = client
        .admin_list_articles(page, per_page, status, category, search)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load Sanity admin articles: {:?}", e);
            ApiError::Internal("Failed to load blog articles".to_string())
        })?;
    let categories = client.admin_list_categories().await.unwrap_or_default();
    let authors = client.admin_list_authors().await.unwrap_or_default();

    Ok(Json(serde_json::json!({
        "articles": articles.articles,
        "total": articles.total,
        "page": articles.page,
        "per_page": articles.per_page,
        "total_pages": articles.total_pages,
        "categories": categories,
        "authors": authors,
        "studio_url": state.config.sanity_studio_url,
    })))
}

/// GET /api/admin/blog/articles/:id — Load one article for the admin editor.
pub async fn admin_blog_get_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.view").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_get_article(&id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to load article: {e:?}")))?
        .ok_or_else(|| ApiError::NotFound("Article not found".to_string()))?;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// POST /api/admin/blog/articles — Create a draft article in Sanity.
pub async fn admin_blog_create_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(input): Json<AdminArticleInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.edit").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_create_article(input)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.article.create",
        &article.id,
        &article.status,
    )
    .await;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// PUT /api/admin/blog/articles/:id — Save article edits as a Sanity draft.
pub async fn admin_blog_update_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<AdminArticleInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.edit").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_update_article(&id, input)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.article.update",
        &article.id,
        &article.status,
    )
    .await;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// POST /api/admin/blog/articles/:id/publish — Publish a Sanity article.
pub async fn admin_blog_publish_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.publish").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_publish_article(&id)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.article.publish",
        &article.id,
        &article.status,
    )
    .await;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// POST /api/admin/blog/articles/:id/unpublish — Take down a Sanity article.
pub async fn admin_blog_unpublish_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.publish").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_unpublish_article(&id)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.article.unpublish",
        &article.id,
        &article.status,
    )
    .await;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// POST /api/admin/blog/articles/:id/archive — Archive a Sanity article.
pub async fn admin_blog_archive_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.archive").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_archive_article(&id)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.article.archive",
        &article.id,
        &article.status,
    )
    .await;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// POST /api/admin/blog/articles/:id/restore — Restore an archived article to draft.
pub async fn admin_blog_restore_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.edit").await?;
    let client = SanityClient::from_config(&state.config);
    let article = client
        .admin_restore_article(&id)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.article.restore",
        &article.id,
        &article.status,
    )
    .await;
    Ok(Json(serde_json::json!({ "article": article })))
}

/// POST /api/admin/blog/assets — Upload a cover image to Sanity.
pub async fn admin_blog_upload_asset(
    admin: AdminUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.edit").await?;

    const MAX_BLOG_ASSET_BYTES: usize = 8 * 1024 * 1024;

    let mut bytes = None;
    let mut filename = None;
    let mut content_type = None;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("Invalid multipart upload".to_string()))?
    {
        if field.name() == Some("file") {
            filename = field.file_name().map(ToString::to_string);
            content_type = field.content_type().map(ToString::to_string);
            // Stream chunk-by-chunk so an oversized upload is rejected before
            // the whole body sits in memory. `field.bytes()` would buffer the
            // entire payload first.
            let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
            loop {
                match field.chunk().await {
                    Ok(Some(chunk)) => {
                        if buf.len().saturating_add(chunk.len()) > MAX_BLOG_ASSET_BYTES {
                            return Err(ApiError::BadRequest(
                                "Image must be 8 MB or smaller".to_string(),
                            ));
                        }
                        buf.extend_from_slice(&chunk);
                    }
                    Ok(None) => break,
                    Err(_) => {
                        return Err(ApiError::BadRequest("Invalid upload body".to_string()));
                    }
                }
            }
            bytes = Some(buf);
            break;
        }
    }

    let bytes = bytes.ok_or_else(|| ApiError::BadRequest("Missing file field".to_string()))?;

    let client = SanityClient::from_config(&state.config);
    let asset = client
        .admin_upload_image(bytes, filename, content_type)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.asset.upload",
        "sanity.imageAsset",
        "uploaded",
    )
    .await;
    Ok(Json(serde_json::json!({ "asset": asset })))
}

/// GET /api/admin/blog/authors — List Sanity authors for admin.
pub async fn admin_blog_list_authors(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.view").await?;
    let authors = SanityClient::from_config(&state.config)
        .admin_list_authors()
        .await
        .map_err(map_sanity_admin_error)?;
    Ok(Json(serde_json::json!({ "authors": authors })))
}

/// POST /api/admin/blog/authors — Create a Sanity author.
pub async fn admin_blog_save_author(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(input): Json<AdminTaxonomyInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.edit").await?;
    let author = SanityClient::from_config(&state.config)
        .admin_save_author(input)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(&state, &admin, "blog.author.save", &author.id, "saved").await;
    Ok(Json(serde_json::json!({ "author": author })))
}

/// GET /api/admin/blog/categories — List Sanity categories for admin.
pub async fn admin_blog_list_categories(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.view").await?;
    let categories = SanityClient::from_config(&state.config)
        .admin_list_categories()
        .await
        .map_err(map_sanity_admin_error)?;
    Ok(Json(serde_json::json!({ "categories": categories })))
}

/// POST /api/admin/blog/categories — Create a Sanity category.
pub async fn admin_blog_save_category(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(input): Json<AdminTaxonomyInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.edit").await?;
    let category = SanityClient::from_config(&state.config)
        .admin_save_category(input)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(&state, &admin, "blog.category.save", &category.id, "saved").await;
    Ok(Json(serde_json::json!({ "category": category })))
}

/// POST /api/admin/blog/import/db-to-sanity/dry-run — Preview DB blog import.
pub async fn admin_blog_import_dry_run(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.import").await?;
    let result = SanityClient::from_config(&state.config)
        .import_database_blog(&state.db, true)
        .await
        .map_err(map_sanity_admin_error)?;
    Ok(Json(serde_json::json!({ "import": result })))
}

/// POST /api/admin/blog/import/db-to-sanity — Import DB blog records into Sanity.
pub async fn admin_blog_import_run(
    admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_blog_permission(&admin, &state, "blog.import").await?;
    let result = SanityClient::from_config(&state.config)
        .import_database_blog(&state.db, false)
        .await
        .map_err(map_sanity_admin_error)?;
    audit_blog_action(
        &state,
        &admin,
        "blog.import.db_to_sanity",
        "blog_articles",
        "imported",
    )
    .await;
    Ok(Json(serde_json::json!({ "import": result })))
}

async fn require_blog_permission(
    admin: &AdminUser,
    state: &AppState,
    permission: &str,
) -> Result<(), ApiError> {
    if middleware::has_permission(&state.db, admin.user.id, permission).await
        || middleware::has_permission(&state.db, admin.user.id, "blog.manage").await
    {
        Ok(())
    } else {
        Err(ApiError::Forbidden(format!(
            "Missing permission: {permission}"
        )))
    }
}

fn map_sanity_admin_error(err: anyhow::Error) -> ApiError {
    let message = err.to_string();
    if message.contains("not configured") {
        ApiError::BadRequest(message)
    } else if message.contains("modified by another editor") {
        ApiError::Conflict(message)
    } else if message.contains("not found") || message.contains("Article not found") {
        ApiError::NotFound(message)
    } else {
        tracing::error!("Sanity admin operation failed: {:?}", err);
        ApiError::Internal("Sanity operation failed".to_string())
    }
}

async fn audit_blog_action(
    state: &AppState,
    admin: &AdminUser,
    action: &str,
    entity_id: &str,
    status: &str,
) {
    let _ = sqlx::query(
        r#"INSERT INTO audit_logs (actor_user_id, action, entity_type, new_state)
           VALUES ($1, $2, 'blog', $3)"#,
    )
    .bind(admin.user.id)
    .bind(action)
    .bind(serde_json::json!({ "entity_id": entity_id, "status": status }))
    .execute(&state.db)
    .await;
}

fn use_sanity_content(state: &AppState) -> bool {
    state
        .config
        .blog_content_source
        .eq_ignore_ascii_case("sanity")
}

async fn list_articles_for_source(
    state: &AppState,
    page: i64,
    per_page: i64,
    category_slug: Option<&str>,
    tag: Option<&str>,
    featured_only: bool,
) -> anyhow::Result<super::models::PaginatedArticles> {
    if use_sanity_content(state) {
        SanityClient::from_config(&state.config)
            .list_articles(page, per_page, category_slug, tag, featured_only)
            .await
    } else {
        service::list_articles(&state.db, page, per_page, category_slug, tag, featured_only)
            .await
            .map_err(anyhow::Error::from)
    }
}

async fn get_article_for_source(
    state: &AppState,
    slug: &str,
) -> anyhow::Result<Option<super::models::ArticleResponse>> {
    if use_sanity_content(state) {
        SanityClient::from_config(&state.config)
            .get_article_by_slug(slug)
            .await
    } else {
        service::get_article_by_slug(&state.db, slug)
            .await
            .map_err(anyhow::Error::from)
    }
}

async fn get_recent_articles_for_source(
    state: &AppState,
    exclude_slug: &str,
    limit: i64,
) -> anyhow::Result<Vec<super::models::ArticleResponse>> {
    if use_sanity_content(state) {
        SanityClient::from_config(&state.config)
            .get_recent_articles(exclude_slug, limit)
            .await
    } else {
        service::get_recent_articles(&state.db, exclude_slug, limit)
            .await
            .map_err(anyhow::Error::from)
    }
}

async fn list_categories_for_source(
    state: &AppState,
) -> anyhow::Result<Vec<super::models::CategoryResponse>> {
    if use_sanity_content(state) {
        SanityClient::from_config(&state.config)
            .list_categories()
            .await
    } else {
        service::list_categories(&state.db)
            .await
            .map_err(anyhow::Error::from)
    }
}

async fn list_authors_for_source(
    state: &AppState,
) -> anyhow::Result<Vec<super::models::AuthorResponse>> {
    if use_sanity_content(state) {
        SanityClient::from_config(&state.config)
            .list_authors()
            .await
    } else {
        service::list_authors(&state.db)
            .await
            .map_err(anyhow::Error::from)
    }
}

// ── Admin-only endpoints ──────────────────────────────────────────

/// POST /api/blog/articles — Create a new article (admin only).
pub async fn create_article(
    admin: AdminUser,
    State(state): State<AppState>,
    Json(req): Json<super::models::CreateArticleRequest>,
) -> axum::response::Response {
    // Basic validation
    if req.slug.is_empty() || req.title.is_empty() || req.excerpt.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "slug, title, and excerpt are required."})),
        )
            .into_response();
    }

    match service::create_article(&state.db, &req).await {
        Ok(id) => {
            audit_blog_action(&state, &admin, "blog.article.create", &id, "created").await;
            (
                StatusCode::CREATED,
                Json(serde_json::json!({"id": id, "slug": req.slug, "status": "created"})),
            )
                .into_response()
        }
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
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<super::models::UpdateArticleRequest>,
) -> axum::response::Response {
    match service::update_article(&state.db, &id, &req).await {
        Ok(true) => {
            audit_blog_action(&state, &admin, "blog.article.update", &id, "updated").await;
            Json(serde_json::json!({"status": "updated", "id": id})).into_response()
        }
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
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> axum::response::Response {
    match service::archive_article(&state.db, &id).await {
        Ok(true) => {
            audit_blog_action(&state, &admin, "blog.article.archive", &id, "archived").await;
            Json(serde_json::json!({"status": "archived", "id": id})).into_response()
        }
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
    admin: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> axum::response::Response {
    match service::publish_article(&state.db, &id).await {
        Ok(true) => {
            audit_blog_action(&state, &admin, "blog.article.publish", &id, "published").await;
            Json(serde_json::json!({
                "status": "published",
                "id": id,
                "message": "Article published successfully."
            }))
            .into_response()
        }
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
