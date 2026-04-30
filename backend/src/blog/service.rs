use sqlx::PgPool;

use super::models::*;

fn clean_public_url(value: Option<String>) -> Option<String> {
    value.map(|item| item.trim().to_string()).filter(|item| {
        let lower = item.to_ascii_lowercase();
        lower.starts_with("https://") || lower.starts_with("http://")
    })
}

fn sanitize_article_html(html: &str) -> String {
    ammonia::Builder::default()
        .add_tags([
            "h2",
            "h3",
            "h4",
            "p",
            "blockquote",
            "ul",
            "ol",
            "li",
            "strong",
            "em",
            "code",
            "pre",
            "a",
            "br",
            "hr",
            "table",
            "thead",
            "tbody",
            "tr",
            "th",
            "td",
            "img",
        ])
        .add_tag_attributes("a", ["href", "target", "title"])
        .add_tag_attributes("img", ["src", "alt", "title", "width", "height", "loading"])
        .link_rel(Some("noopener noreferrer"))
        .clean(html)
        .to_string()
}

/// Render markdown source to HTML. Used as fallback when content_html is missing.
fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    let parser = Parser::new_ext(md, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

// ── Article Queries ───────────────────────────────────────────────

/// List published articles with pagination and optional filters.
pub async fn list_articles(
    pool: &PgPool,
    page: i64,
    per_page: i64,
    category_slug: Option<&str>,
    tag: Option<&str>,
    featured_only: bool,
) -> Result<PaginatedArticles, sqlx::Error> {
    let offset = (page - 1) * per_page;

    // Count total matching articles
    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM blog_articles a
           LEFT JOIN blog_categories c ON a.category_id = c.id
           WHERE a.status = 'published'
             AND ($1::text IS NULL OR c.slug = $1)
             AND ($2::text IS NULL OR $2 = ANY(a.tags))
             AND ($3::bool = FALSE OR a.featured = TRUE)"#,
    )
    .bind(category_slug)
    .bind(tag)
    .bind(featured_only)
    .fetch_one(pool)
    .await?;

    // Fetch article rows with joined author + category
    let rows = sqlx::query(
        r#"SELECT
             a.id::text AS article_id,
             a.slug, a.title, a.subtitle, a.excerpt,
             a.meta_title, a.meta_description, a.canonical_url, a.og_image_url,
             a.cover_image_url, a.reading_time_minutes, a.featured,
             a.schema_type, a.faq_data,
             a.tags,
             a.published_at::text, a.created_at::text AS a_created_at, a.updated_at::text AS a_updated_at,
             -- Author
             au.id::text AS author_id, au.name AS author_name, au.slug AS author_slug,
             au.avatar_url AS author_avatar, au.bio AS author_bio,
             au.website_url AS author_website, au.twitter_handle AS author_twitter,
             au.linkedin_url AS author_linkedin,
             au.facebook_url AS author_facebook, au.instagram_url AS author_instagram,
             au.whatsapp AS author_whatsapp,
             -- Category
             c.id::text AS category_id, c.name AS category_name, c.slug AS category_slug,
             c.color AS category_color, c.icon AS category_icon
           FROM blog_articles a
           JOIN blog_authors au ON a.author_id = au.id
           JOIN blog_categories c ON a.category_id = c.id
           WHERE a.status = 'published'
             AND ($1::text IS NULL OR c.slug = $1)
             AND ($2::text IS NULL OR $2 = ANY(a.tags))
             AND ($3::bool = FALSE OR a.featured = TRUE)
           ORDER BY a.published_at DESC NULLS LAST
           LIMIT $4 OFFSET $5"#,
    )
    .bind(category_slug)
    .bind(tag)
    .bind(featured_only)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    let articles: Vec<ArticleResponse> = rows
        .iter()
        .map(|r| ArticleResponse {
            id: r.get("article_id"),
            slug: r.get("slug"),
            title: r.get("title"),
            subtitle: r.get("subtitle"),
            excerpt: r.get("excerpt"),
            content: None, // List endpoint omits full content
            content_html: None,
            meta_title: r.get("meta_title"),
            meta_description: r.get("meta_description"),
            canonical_url: r.get("canonical_url"),
            og_image_url: r.get("og_image_url"),
            author: AuthorSummary {
                id: r.get("author_id"),
                name: r.get("author_name"),
                initials: author_initials(&r.get::<String, _>("author_name")),
                slug: r.get("author_slug"),
                avatar_url: r
                    .get::<Option<String>, _>("author_avatar")
                    .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
                bio: r.get("author_bio"),
                website_url: clean_public_url(r.get("author_website")),
                twitter_handle: r.get("author_twitter"),
                linkedin_url: clean_public_url(r.get("author_linkedin")),
                facebook_url: clean_public_url(r.get("author_facebook")),
                instagram_url: clean_public_url(r.get("author_instagram")),
                whatsapp: r.get("author_whatsapp"),
            },
            category: CategorySummary {
                id: r.get("category_id"),
                name: r.get("category_name"),
                slug: r.get("category_slug"),
                color: r.get("category_color"),
                icon: r.get("category_icon"),
            },
            tags: r.get::<Vec<String>, _>("tags"),
            cover_image_url: r
                .get::<Option<String>, _>("cover_image_url")
                .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
            reading_time_minutes: r.get::<Option<i32>, _>("reading_time_minutes").unwrap_or(5),
            featured: r.get::<Option<bool>, _>("featured").unwrap_or(false),
            share_links: ArticleShareLinks::default(),
            schema_type: r
                .get::<Option<String>, _>("schema_type")
                .unwrap_or_else(|| "BlogPosting".to_string()),
            faq_data: r.get("faq_data"),
            published_at: r.get("published_at"),
            created_at: r.get("a_created_at"),
            updated_at: r.get("a_updated_at"),
        })
        .collect();

    let total_pages = if per_page > 0 {
        (total + per_page - 1) / per_page
    } else {
        1
    };

    Ok(PaginatedArticles {
        articles,
        total,
        page,
        per_page,
        total_pages,
    })
}

/// Get a single article by slug (public — only published).
pub async fn get_article_by_slug(
    pool: &PgPool,
    slug: &str,
) -> Result<Option<ArticleResponse>, sqlx::Error> {
    let row = sqlx::query(
        r#"SELECT
             a.id::text AS article_id,
             a.slug, a.title, a.subtitle, a.excerpt, a.content, a.content_html,
             a.meta_title, a.meta_description, a.canonical_url, a.og_image_url,
             a.cover_image_url, a.reading_time_minutes, a.featured,
             a.schema_type, a.faq_data,
             a.tags,
             a.published_at::text, a.created_at::text AS a_created_at, a.updated_at::text AS a_updated_at,
             au.id::text AS author_id, au.name AS author_name, au.slug AS author_slug,
             au.avatar_url AS author_avatar, au.bio AS author_bio,
             au.website_url AS author_website, au.twitter_handle AS author_twitter,
             au.linkedin_url AS author_linkedin,
             au.facebook_url AS author_facebook, au.instagram_url AS author_instagram,
             au.whatsapp AS author_whatsapp,
             c.id::text AS category_id, c.name AS category_name, c.slug AS category_slug,
             c.color AS category_color, c.icon AS category_icon
           FROM blog_articles a
           JOIN blog_authors au ON a.author_id = au.id
           JOIN blog_categories c ON a.category_id = c.id
           WHERE a.slug = $1 AND a.status = 'published'"#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    use sqlx::Row;
    Ok(row.map(|r| ArticleResponse {
        id: r.get("article_id"),
        slug: r.get("slug"),
        title: r.get("title"),
        subtitle: r.get("subtitle"),
        excerpt: r.get("excerpt"),
        content: r.get("content"),
        content_html: {
            let existing: Option<String> = r.get("content_html");
            existing
                .filter(|s| !s.trim().is_empty())
                .map(|html| sanitize_article_html(&html))
                .or_else(|| {
                    r.get::<Option<String>, _>("content")
                        .filter(|s| !s.trim().is_empty())
                        .map(|md| sanitize_article_html(&render_markdown(&md)))
                })
        },
        meta_title: r.get("meta_title"),
        meta_description: r.get("meta_description"),
        canonical_url: r.get("canonical_url"),
        og_image_url: r.get("og_image_url"),
        author: AuthorSummary {
            id: r.get("author_id"),
            name: r.get("author_name"),
            initials: author_initials(&r.get::<String, _>("author_name")),
            slug: r.get("author_slug"),
            avatar_url: r
                .get::<Option<String>, _>("author_avatar")
                .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
            bio: r.get("author_bio"),
            website_url: clean_public_url(r.get("author_website")),
            twitter_handle: r.get("author_twitter"),
            linkedin_url: clean_public_url(r.get("author_linkedin")),
            facebook_url: clean_public_url(r.get("author_facebook")),
            instagram_url: clean_public_url(r.get("author_instagram")),
            whatsapp: r.get("author_whatsapp"),
        },
        category: CategorySummary {
            id: r.get("category_id"),
            name: r.get("category_name"),
            slug: r.get("category_slug"),
            color: r.get("category_color"),
            icon: r.get("category_icon"),
        },
        tags: r.get::<Vec<String>, _>("tags"),
        cover_image_url: r
            .get::<Option<String>, _>("cover_image_url")
            .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
        reading_time_minutes: r.get::<Option<i32>, _>("reading_time_minutes").unwrap_or(5),
        featured: r.get::<Option<bool>, _>("featured").unwrap_or(false),
        share_links: ArticleShareLinks::default(),
        schema_type: r
            .get::<Option<String>, _>("schema_type")
            .unwrap_or_else(|| "BlogPosting".to_string()),
        faq_data: r.get("faq_data"),
        published_at: r.get("published_at"),
        created_at: r.get("a_created_at"),
        updated_at: r.get("a_updated_at"),
    }))
}

/// Create a new article. Returns the created article's id.
pub async fn create_article(
    pool: &PgPool,
    req: &CreateArticleRequest,
) -> Result<String, sqlx::Error> {
    let author_id: uuid::Uuid = req
        .author_id
        .parse()
        .map_err(|_| sqlx::Error::Protocol("Invalid author_id UUID".to_string()))?;
    let category_id: uuid::Uuid = req
        .category_id
        .parse()
        .map_err(|_| sqlx::Error::Protocol("Invalid category_id UUID".to_string()))?;
    let tags = req.tags.clone().unwrap_or_default();
    let status = req.status.as_deref().unwrap_or("draft");
    let reading_time = req.reading_time_minutes.unwrap_or(5);
    let featured = req.featured.unwrap_or(false);
    let schema_type = req.schema_type.as_deref().unwrap_or("BlogPosting");

    // Parse optional timestamps
    let published_at: Option<chrono::DateTime<chrono::Utc>> =
        req.published_at.as_ref().and_then(|s| s.parse().ok());
    let scheduled_at: Option<chrono::DateTime<chrono::Utc>> =
        req.scheduled_at.as_ref().and_then(|s| s.parse().ok());

    let id: String = sqlx::query_scalar(
        r#"INSERT INTO blog_articles (
             slug, title, subtitle, excerpt, content, content_html,
             meta_title, meta_description, canonical_url, og_image_url,
             author_id, category_id, tags,
             cover_image_url, reading_time_minutes, featured,
             schema_type, faq_data,
             status, published_at, scheduled_at
           ) VALUES (
             $1, $2, $3, $4, $5, $6,
             $7, $8, $9, $10,
             $11, $12, $13,
             $14, $15, $16,
             $17, $18,
             $19, $20, $21
           ) RETURNING id::text"#,
    )
    .bind(&req.slug)
    .bind(&req.title)
    .bind(&req.subtitle)
    .bind(&req.excerpt)
    .bind(&req.content)
    .bind(&req.content_html)
    .bind(&req.meta_title)
    .bind(&req.meta_description)
    .bind(&req.canonical_url)
    .bind(&req.og_image_url)
    .bind(author_id)
    .bind(category_id)
    .bind(&tags)
    .bind(&req.cover_image_url)
    .bind(reading_time)
    .bind(featured)
    .bind(schema_type)
    .bind(&req.faq_data)
    .bind(status)
    .bind(published_at)
    .bind(scheduled_at)
    .fetch_one(pool)
    .await?;

    Ok(id)
}

/// Update an existing article by UUID string. Returns true if a row was updated.
pub async fn update_article(
    pool: &PgPool,
    article_id: &str,
    req: &UpdateArticleRequest,
) -> Result<bool, sqlx::Error> {
    let uuid: uuid::Uuid = article_id
        .parse()
        .map_err(|_| sqlx::Error::Protocol("Invalid article_id UUID".to_string()))?;

    // Build dynamic SET clauses (only update provided fields)
    // For simplicity we update all columns, using COALESCE to keep existing values.
    let author_id: Option<uuid::Uuid> = req.author_id.as_ref().and_then(|s| s.parse().ok());
    let category_id: Option<uuid::Uuid> = req.category_id.as_ref().and_then(|s| s.parse().ok());
    let published_at: Option<chrono::DateTime<chrono::Utc>> =
        req.published_at.as_ref().and_then(|s| s.parse().ok());
    let scheduled_at: Option<chrono::DateTime<chrono::Utc>> =
        req.scheduled_at.as_ref().and_then(|s| s.parse().ok());

    let result = sqlx::query(
        r#"UPDATE blog_articles SET
             slug               = COALESCE($2, slug),
             title              = COALESCE($3, title),
             subtitle           = COALESCE($4, subtitle),
             excerpt            = COALESCE($5, excerpt),
             content            = COALESCE($6, content),
             content_html       = COALESCE($7, content_html),
             meta_title         = COALESCE($8, meta_title),
             meta_description   = COALESCE($9, meta_description),
             canonical_url      = COALESCE($10, canonical_url),
             og_image_url       = COALESCE($11, og_image_url),
             author_id          = COALESCE($12, author_id),
             category_id        = COALESCE($13, category_id),
             tags               = COALESCE($14, tags),
             cover_image_url    = COALESCE($15, cover_image_url),
             reading_time_minutes = COALESCE($16, reading_time_minutes),
             featured           = COALESCE($17, featured),
             schema_type        = COALESCE($18, schema_type),
             faq_data           = COALESCE($19, faq_data),
             status             = COALESCE($20, status),
             published_at       = COALESCE($21, published_at),
             scheduled_at       = COALESCE($22, scheduled_at),
             updated_at         = NOW()
           WHERE id = $1"#,
    )
    .bind(uuid)
    .bind(&req.slug)
    .bind(&req.title)
    .bind(&req.subtitle)
    .bind(&req.excerpt)
    .bind(&req.content)
    .bind(&req.content_html)
    .bind(&req.meta_title)
    .bind(&req.meta_description)
    .bind(&req.canonical_url)
    .bind(&req.og_image_url)
    .bind(author_id)
    .bind(category_id)
    .bind(&req.tags)
    .bind(&req.cover_image_url)
    .bind(req.reading_time_minutes)
    .bind(req.featured)
    .bind(&req.schema_type)
    .bind(&req.faq_data)
    .bind(&req.status)
    .bind(published_at)
    .bind(scheduled_at)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Soft-delete: set article status to 'archived'.
pub async fn archive_article(pool: &PgPool, article_id: &str) -> Result<bool, sqlx::Error> {
    let uuid: uuid::Uuid = article_id
        .parse()
        .map_err(|_| sqlx::Error::Protocol("Invalid article_id UUID".to_string()))?;

    let result = sqlx::query(
        "UPDATE blog_articles SET status = 'archived', updated_at = NOW() WHERE id = $1",
    )
    .bind(uuid)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Publish an article: set status='published' and published_at=NOW() if not already set.
pub async fn publish_article(pool: &PgPool, article_id: &str) -> Result<bool, sqlx::Error> {
    let uuid: uuid::Uuid = article_id
        .parse()
        .map_err(|_| sqlx::Error::Protocol("Invalid article_id UUID".to_string()))?;

    let result = sqlx::query(
        r#"UPDATE blog_articles SET
             status = 'published',
             published_at = COALESCE(published_at, NOW()),
             updated_at = NOW()
           WHERE id = $1 AND status != 'published'"#,
    )
    .bind(uuid)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Get recent articles (excluding a given slug) for sidebar/recommendations.
pub async fn get_recent_articles(
    pool: &PgPool,
    exclude_slug: &str,
    limit: i64,
) -> Result<Vec<ArticleResponse>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT
             a.id::text AS article_id,
             a.slug, a.title, a.subtitle, a.excerpt,
             a.meta_title, a.meta_description, a.canonical_url, a.og_image_url,
             a.cover_image_url, a.reading_time_minutes, a.featured,
             a.schema_type, a.faq_data,
             a.tags,
             a.published_at::text, a.created_at::text AS a_created_at, a.updated_at::text AS a_updated_at,
             au.id::text AS author_id, au.name AS author_name, au.slug AS author_slug,
             au.avatar_url AS author_avatar, au.bio AS author_bio,
             au.website_url AS author_website, au.twitter_handle AS author_twitter,
             au.linkedin_url AS author_linkedin,
             au.facebook_url AS author_facebook, au.instagram_url AS author_instagram,
             au.whatsapp AS author_whatsapp,
             c.id::text AS category_id, c.name AS category_name, c.slug AS category_slug,
             c.color AS category_color, c.icon AS category_icon
           FROM blog_articles a
           JOIN blog_authors au ON a.author_id = au.id
           JOIN blog_categories c ON a.category_id = c.id
           WHERE a.status = 'published' AND a.slug != $1
           ORDER BY a.published_at DESC NULLS LAST
           LIMIT $2"#,
    )
    .bind(exclude_slug)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    Ok(rows
        .iter()
        .map(|r| ArticleResponse {
            id: r.get("article_id"),
            slug: r.get("slug"),
            title: r.get("title"),
            subtitle: r.get("subtitle"),
            excerpt: r.get("excerpt"),
            content: None,
            content_html: None,
            meta_title: r.get("meta_title"),
            meta_description: r.get("meta_description"),
            canonical_url: r.get("canonical_url"),
            og_image_url: r.get("og_image_url"),
            author: AuthorSummary {
                id: r.get("author_id"),
                name: r.get("author_name"),
                initials: author_initials(&r.get::<String, _>("author_name")),
                slug: r.get("author_slug"),
                avatar_url: r
                    .get::<Option<String>, _>("author_avatar")
                    .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
                bio: r.get("author_bio"),
                website_url: clean_public_url(r.get("author_website")),
                twitter_handle: r.get("author_twitter"),
                linkedin_url: clean_public_url(r.get("author_linkedin")),
                facebook_url: clean_public_url(r.get("author_facebook")),
                instagram_url: clean_public_url(r.get("author_instagram")),
                whatsapp: r.get("author_whatsapp"),
            },
            category: CategorySummary {
                id: r.get("category_id"),
                name: r.get("category_name"),
                slug: r.get("category_slug"),
                color: r.get("category_color"),
                icon: r.get("category_icon"),
            },
            tags: r.get::<Vec<String>, _>("tags"),
            cover_image_url: r
                .get::<Option<String>, _>("cover_image_url")
                .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
            reading_time_minutes: r.get::<Option<i32>, _>("reading_time_minutes").unwrap_or(5),
            featured: r.get::<Option<bool>, _>("featured").unwrap_or(false),
            share_links: ArticleShareLinks::default(),
            schema_type: r
                .get::<Option<String>, _>("schema_type")
                .unwrap_or_else(|| "BlogPosting".to_string()),
            faq_data: r.get("faq_data"),
            published_at: r.get("published_at"),
            created_at: r.get("a_created_at"),
            updated_at: r.get("a_updated_at"),
        })
        .collect())
}

// ── Category / Author Queries ─────────────────────────────────────

/// List all categories with article counts.
pub async fn list_categories(pool: &PgPool) -> Result<Vec<CategoryResponse>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT
             c.id::text, c.name, c.slug, c.description, c.color, c.icon,
             c.meta_title, c.meta_description,
             COUNT(a.id) FILTER (WHERE a.status = 'published') AS article_count
           FROM blog_categories c
           LEFT JOIN blog_articles a ON a.category_id = c.id
           GROUP BY c.id
           ORDER BY c.sort_order"#,
    )
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    Ok(rows
        .iter()
        .map(|r| CategoryResponse {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            description: r.get("description"),
            color: r.get("color"),
            icon: r.get("icon"),
            meta_title: r.get("meta_title"),
            meta_description: r.get("meta_description"),
            article_count: r.get::<Option<i64>, _>("article_count").unwrap_or(0),
        })
        .collect())
}

/// List all authors with article counts.
pub async fn list_authors(pool: &PgPool) -> Result<Vec<AuthorResponse>, sqlx::Error> {
    let rows = sqlx::query(
        r#"SELECT
             au.id::text, au.name, au.slug, au.bio, au.avatar_url,
             au.website_url, au.twitter_handle, au.linkedin_url,
             au.facebook_url, au.instagram_url, au.whatsapp,
             au.expertise,
             COUNT(a.id) FILTER (WHERE a.status = 'published') AS article_count
           FROM blog_authors au
           LEFT JOIN blog_articles a ON a.author_id = au.id
           GROUP BY au.id
           ORDER BY au.name"#,
    )
    .fetch_all(pool)
    .await?;

    use sqlx::Row;
    Ok(rows
        .iter()
        .map(|r| AuthorResponse {
            id: r.get("id"),
            name: r.get("name"),
            slug: r.get("slug"),
            bio: r.get("bio"),
            avatar_url: r
                .get::<Option<String>, _>("avatar_url")
                .map(|u| crate::storage::service::rewrite_gcs_url(&u)),
            website_url: clean_public_url(r.get("website_url")),
            twitter_handle: r.get("twitter_handle"),
            linkedin_url: clean_public_url(r.get("linkedin_url")),
            facebook_url: clean_public_url(r.get("facebook_url")),
            instagram_url: clean_public_url(r.get("instagram_url")),
            whatsapp: r.get("whatsapp"),
            expertise: r.get::<Vec<String>, _>("expertise"),
            article_count: r.get::<Option<i64>, _>("article_count").unwrap_or(0),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{clean_public_url, render_markdown, sanitize_article_html};

    #[test]
    fn sanitize_article_html_removes_scripts_events_and_unsafe_urls() {
        let html = r#"
            <h2>Safe heading</h2>
            <script>alert("x")</script>
            <p onclick="alert(1)">Body <strong>text</strong></p>
            <a href="javascript:alert(1)" target="_blank">bad</a>
            <img src="javascript:alert(1)" onerror="alert(1)" alt="bad">
            <iframe src="https://example.com"></iframe>
        "#;

        let cleaned = sanitize_article_html(html);

        assert!(cleaned.contains("<h2>Safe heading</h2>"));
        assert!(cleaned.contains("<strong>text</strong>"));
        assert!(!cleaned.contains("<script"));
        assert!(!cleaned.contains("onclick"));
        assert!(!cleaned.contains("javascript:"));
        assert!(!cleaned.contains("onerror"));
        assert!(!cleaned.contains("<iframe"));
    }

    #[test]
    fn markdown_fallback_html_is_sanitizable() {
        let rendered =
            render_markdown("[bad](javascript:alert(1))\n\n<img src=x onerror=alert(1)>");
        let cleaned = sanitize_article_html(&rendered);

        assert!(!cleaned.contains("javascript:"));
        assert!(!cleaned.contains("onerror"));
    }

    #[test]
    fn clean_public_url_allows_only_http_urls() {
        assert_eq!(
            clean_public_url(Some(" https://example.com/profile ".to_string())),
            Some("https://example.com/profile".to_string())
        );
        assert_eq!(
            clean_public_url(Some("javascript:alert(1)".to_string())),
            None
        );
        assert_eq!(
            clean_public_url(Some("mailto:test@example.com".to_string())),
            None
        );
    }
}
