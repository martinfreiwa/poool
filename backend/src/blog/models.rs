use serde::{Deserialize, Serialize};

// ── Response models ───────────────────────────────────────────────

/// A single blog article returned by the API.
#[derive(Debug, Serialize, Deserialize)]
pub struct ArticleResponse {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub excerpt: String,
    pub content: Option<String>, // Only included in single-article responses
    pub content_html: Option<String>,

    // SEO
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,

    // Taxonomy
    pub author: AuthorSummary,
    pub category: CategorySummary,
    pub tags: Vec<String>,

    // Display
    pub cover_image_url: Option<String>,
    pub reading_time_minutes: i32,
    pub featured: bool,
    pub share_links: ArticleShareLinks,

    // Schema
    pub schema_type: String,
    pub faq_data: Option<serde_json::Value>,

    // Timestamps
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Optional per-article social share destinations.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArticleShareLinks {
    pub whatsapp_url: Option<String>,
    pub facebook_url: Option<String>,
    pub x_url: Option<String>,
    pub instagram_url: Option<String>,
    pub linkedin_url: Option<String>,
}

/// Minimal author info embedded in article responses.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthorSummary {
    pub id: String,
    pub name: String,
    pub initials: String,
    pub slug: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub facebook_url: Option<String>,
    pub instagram_url: Option<String>,
    pub whatsapp: Option<String>,
}

/// Minimal category info embedded in article responses.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategorySummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

/// Full author detail (for /api/blog/authors/:slug).
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub facebook_url: Option<String>,
    pub instagram_url: Option<String>,
    pub whatsapp: Option<String>,
    pub expertise: Vec<String>,
    pub article_count: i64,
}

/// Full category detail (for /api/blog/categories).
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub article_count: i64,
}

/// Paginated list envelope.
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedArticles {
    pub articles: Vec<ArticleResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

pub fn author_initials(name: &str) -> String {
    let initials: String = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect();

    if initials.is_empty() {
        "P".to_string()
    } else {
        initials.to_uppercase()
    }
}

// ── Request models ────────────────────────────────────────────────

/// Body for POST /api/blog/articles
#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub slug: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub excerpt: String,
    pub content: String,
    pub content_html: Option<String>,

    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,

    pub author_id: String,   // UUID as string
    pub category_id: String, // UUID as string
    pub tags: Option<Vec<String>>,

    pub cover_image_url: Option<String>,
    pub reading_time_minutes: Option<i32>,
    pub featured: Option<bool>,

    pub schema_type: Option<String>,
    pub faq_data: Option<serde_json::Value>,

    pub status: Option<String>, // draft | review | scheduled | published
    pub published_at: Option<String>,
    pub scheduled_at: Option<String>,
}

/// Body for PUT /api/blog/articles/:id
#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub slug: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub content_html: Option<String>,

    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,

    pub author_id: Option<String>,
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,

    pub cover_image_url: Option<String>,
    pub reading_time_minutes: Option<i32>,
    pub featured: Option<bool>,

    pub schema_type: Option<String>,
    pub faq_data: Option<serde_json::Value>,

    pub status: Option<String>,
    pub published_at: Option<String>,
    pub scheduled_at: Option<String>,
}
