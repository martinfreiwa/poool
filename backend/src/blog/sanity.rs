use std::collections::HashMap;

use ammonia::Builder;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;

use super::models::{
    author_initials, ArticleResponse, ArticleShareLinks, AuthorResponse, AuthorSummary,
    CategoryResponse, CategorySummary, PaginatedArticles,
};

#[derive(Debug, Deserialize)]
struct SanityEnvelope<T> {
    result: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SanityArticle {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_rev")]
    revision: Option<String>,
    slug: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
    excerpt: Option<String>,
    body: Option<Value>,
    content: Option<String>,
    meta_title: Option<String>,
    meta_description: Option<String>,
    canonical_url: Option<String>,
    og_image_url: Option<String>,
    cover_image_url: Option<String>,
    cover_image_asset_ref: Option<String>,
    reading_time_minutes: Option<i32>,
    featured: Option<bool>,
    share_links: Option<SanityShareLinks>,
    status: Option<String>,
    schema_type: Option<String>,
    faq_data: Option<Value>,
    tags: Option<Vec<String>>,
    published_at: Option<String>,
    scheduled_at: Option<String>,
    language: Option<String>,
    locale: Option<String>,
    lang: Option<String>,
    translations: Option<Vec<SanityArticleTranslation>>,
    translation_metadata: Option<Vec<SanityArticleTranslation>>,
    #[serde(rename = "_createdAt")]
    created_at: Option<String>,
    #[serde(rename = "_updatedAt")]
    updated_at: Option<String>,
    author: Option<SanityAuthor>,
    category: Option<SanityCategory>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct SanityShareLinks {
    whatsapp_url: Option<String>,
    facebook_url: Option<String>,
    x_url: Option<String>,
    instagram_url: Option<String>,
    linkedin_url: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SanityArticleTranslation {
    #[serde(rename = "_id")]
    id: Option<String>,
    slug: Option<String>,
    title: Option<String>,
    status: Option<String>,
    language: Option<String>,
    locale: Option<String>,
    lang: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SanityAuthor {
    #[serde(rename = "_id")]
    id: Option<String>,
    name: Option<String>,
    slug: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
    website_url: Option<String>,
    twitter_handle: Option<String>,
    linkedin_url: Option<String>,
    facebook_url: Option<String>,
    instagram_url: Option<String>,
    whatsapp: Option<String>,
    expertise: Option<Vec<String>>,
    article_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SanityCategory {
    #[serde(rename = "_id")]
    id: Option<String>,
    name: Option<String>,
    slug: Option<String>,
    description: Option<String>,
    color: Option<String>,
    icon: Option<String>,
    meta_title: Option<String>,
    meta_description: Option<String>,
    article_count: Option<i64>,
}

/// Overview data used by the admin blog dashboard.
#[derive(Debug, Serialize)]
pub struct AdminBlogOverview {
    /// Number of published articles visible through the public Sanity dataset.
    pub published_count: i64,
    /// Draft count is not available without a private token.
    pub draft_count: Option<i64>,
    /// Number of categories in Sanity.
    pub category_count: i64,
    /// Most recent published article timestamp.
    pub latest_published_at: Option<String>,
    /// Sanity project ID.
    pub project_id: String,
    /// Sanity dataset name.
    pub dataset: String,
    /// URL opened by admin create/edit actions.
    pub studio_url: String,
    /// Whether private read queries can include drafts and private IDs.
    pub private_reads_enabled: bool,
    /// Whether write endpoints can mutate Sanity documents.
    pub writes_enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminBlogArticle {
    pub id: String,
    pub draft_id: Option<String>,
    pub published_id: String,
    pub revision: Option<String>,
    pub slug: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub excerpt: String,
    pub body: Value,
    pub body_text: String,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub cover_image_url: Option<String>,
    pub cover_image_asset_ref: Option<String>,
    pub author: Option<AdminTaxonomySummary>,
    pub category: Option<AdminTaxonomySummary>,
    pub tags: Vec<String>,
    pub featured: bool,
    pub share_links: ArticleShareLinks,
    pub status: String,
    pub published_at: Option<String>,
    pub scheduled_at: Option<String>,
    pub schema_type: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub live_url: Option<String>,
    pub studio_url: String,
    pub translation_status: Vec<AdminTranslationStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTranslationStatus {
    pub code: String,
    pub label: String,
    pub present: bool,
    pub document_id: Option<String>,
    pub status: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTaxonomySummary {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminBlogArticleList {
    pub articles: Vec<AdminBlogArticle>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminBlogImportResult {
    pub dry_run: bool,
    pub source_articles: i64,
    pub source_authors: i64,
    pub source_categories: i64,
    pub existing_sanity_slugs: Vec<String>,
    pub imported_articles: i64,
    pub imported_authors: i64,
    pub imported_categories: i64,
    pub skipped_articles: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminArticleInput {
    pub title: String,
    pub slug: String,
    pub subtitle: Option<String>,
    pub excerpt: String,
    pub body: Option<Value>,
    pub body_text: Option<String>,
    pub meta_title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub cover_image_url: Option<String>,
    pub cover_image_asset_ref: Option<String>,
    pub author_id: Option<String>,
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub featured: Option<bool>,
    pub share_links: Option<ArticleShareLinks>,
    pub status: Option<String>,
    pub published_at: Option<String>,
    pub scheduled_at: Option<String>,
    pub schema_type: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTaxonomyInput {
    pub id: Option<String>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i32>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub website_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub linkedin_url: Option<String>,
    pub facebook_url: Option<String>,
    pub instagram_url: Option<String>,
    pub whatsapp: Option<String>,
    pub expertise: Option<Vec<String>>,
}

/// Public Sanity CDN client.
pub struct SanityClient {
    http: reqwest::Client,
    project_id: String,
    dataset: String,
    api_version: String,
    studio_url: String,
    read_token: Option<String>,
    write_token: Option<String>,
}

impl SanityClient {
    /// Build a Sanity client from application config.
    pub fn from_config(config: &Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            project_id: config.sanity_project_id.clone(),
            dataset: config.sanity_dataset.clone(),
            api_version: config.sanity_api_version.clone(),
            studio_url: config.sanity_studio_url.clone(),
            read_token: config.sanity_read_token.clone(),
            write_token: config.sanity_write_token.clone(),
        }
    }

    /// List published articles from Sanity.
    pub async fn list_articles(
        &self,
        page: i64,
        per_page: i64,
        category_slug: Option<&str>,
        tag: Option<&str>,
        featured_only: bool,
    ) -> Result<PaginatedArticles> {
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);
        let start = (page - 1) * per_page;
        let end = start + per_page;
        let filter = article_filter(category_slug, tag, featured_only);

        let count_query = format!("count(*[{}])", filter);
        let total: i64 = self.query(&count_query).await?;

        let query = format!(
            "*[{filter}] | order(publishedAt desc) [{start}...{end}] {{ {} }}",
            article_projection()
        );
        let rows: Vec<SanityArticle> = self.query(&query).await?;
        let articles = rows.into_iter().map(map_article).collect();
        let total_pages = if total > 0 {
            (total + per_page - 1) / per_page
        } else {
            0
        };

        Ok(PaginatedArticles {
            articles,
            total,
            page,
            per_page,
            total_pages,
        })
    }

    /// Get one published article by slug.
    pub async fn get_article_by_slug(&self, slug: &str) -> Result<Option<ArticleResponse>> {
        if !is_safe_slug(slug) {
            return Ok(None);
        }

        let query = format!(
            "*[{} && slug.current == {}][0] {{ {} }}",
            article_filter(None, None, false),
            groq_string(slug),
            article_projection()
        );
        let article: Option<SanityArticle> = self.query(&query).await?;
        Ok(article.map(map_article))
    }

    /// Get recent articles excluding a slug.
    pub async fn get_recent_articles(
        &self,
        exclude_slug: &str,
        limit: i64,
    ) -> Result<Vec<ArticleResponse>> {
        let limit = limit.clamp(1, 20);
        let query = format!(
            "*[{} && slug.current != {}] | order(publishedAt desc) [0...{}] {{ {} }}",
            article_filter(None, None, false),
            groq_string(exclude_slug),
            limit,
            article_projection()
        );
        let rows: Vec<SanityArticle> = self.query(&query).await?;
        Ok(rows.into_iter().map(map_article).collect())
    }

    /// List blog categories from Sanity.
    pub async fn list_categories(&self) -> Result<Vec<CategoryResponse>> {
        let query = format!(
            r#"*[_type == "category"] | order(coalesce(sortOrder, 9999) asc, name asc) {{
                _id,
                name,
                "slug": slug.current,
                description,
                color,
                icon,
                metaTitle,
                metaDescription,
                "articleCount": count(*[{} && references(^._id)])
            }}"#,
            article_filter(None, None, false)
        );
        let rows: Vec<SanityCategory> = self.query(&query).await?;
        Ok(rows.into_iter().map(map_category).collect())
    }

    /// List blog authors from Sanity.
    pub async fn list_authors(&self) -> Result<Vec<AuthorResponse>> {
        let query = format!(
            r#"*[_type == "author"] | order(name asc) {{
                _id,
                name,
                "slug": slug.current,
                bio,
                "avatarUrl": avatar.asset->url,
                websiteUrl,
                twitterHandle,
                linkedinUrl,
                facebookUrl,
                instagramUrl,
                whatsapp,
                expertise,
                "articleCount": count(*[{} && references(^._id)])
            }}"#,
            article_filter(None, None, false)
        );
        let rows: Vec<SanityAuthor> = self.query(&query).await?;
        Ok(rows.into_iter().map(map_author).collect())
    }

    /// Get admin dashboard overview metrics.
    pub async fn admin_overview(&self) -> Result<AdminBlogOverview> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct OverviewResult {
            published_count: i64,
            category_count: i64,
            latest_published_at: Option<String>,
        }

        let query = format!(
            r#"{{
                "publishedCount": count(*[{}]),
                "categoryCount": count(*[_type == "category"]),
                "latestPublishedAt": *[{}] | order(publishedAt desc)[0].publishedAt
            }}"#,
            article_filter(None, None, false),
            article_filter(None, None, false)
        );
        let result: OverviewResult = self.query(&query).await?;
        Ok(AdminBlogOverview {
            published_count: result.published_count,
            draft_count: None,
            category_count: result.category_count,
            latest_published_at: result.latest_published_at,
            project_id: self.project_id.clone(),
            dataset: self.dataset.clone(),
            studio_url: self.studio_url.clone(),
            private_reads_enabled: self.read_token.is_some() || self.write_token.is_some(),
            writes_enabled: self.write_token.is_some(),
        })
    }

    /// List all Sanity article states for the admin CMS, including drafts.
    pub async fn admin_list_articles(
        &self,
        page: i64,
        per_page: i64,
        status: Option<&str>,
        category_slug: Option<&str>,
        search: Option<&str>,
    ) -> Result<AdminBlogArticleList> {
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);
        let query = format!(
            r#"*[_type == "article"] | order(coalesce(publishedAt, _updatedAt) desc) {{ {} }}"#,
            article_projection()
        );
        let rows: Vec<SanityArticle> = self.admin_query(&query).await?;
        let mut articles = group_admin_articles(rows, &self.studio_url, None);

        if let Some(wanted) = status.filter(|s| !s.is_empty()) {
            articles.retain(|article| article.status == wanted);
        }
        if let Some(slug) = category_slug.filter(|s| is_safe_slug(s)) {
            articles.retain(|article| article.category.as_ref().is_some_and(|c| c.slug == slug));
        }
        if let Some(term) = search.map(str::trim).filter(|s| !s.is_empty()) {
            let needle = term.to_lowercase();
            articles.retain(|article| {
                [
                    article.title.as_str(),
                    article.slug.as_str(),
                    article.excerpt.as_str(),
                    article
                        .author
                        .as_ref()
                        .map(|a| a.name.as_str())
                        .unwrap_or(""),
                    article
                        .category
                        .as_ref()
                        .map(|c| c.name.as_str())
                        .unwrap_or(""),
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&needle))
            });
        }

        let total = articles.len() as i64;
        let start = ((page - 1) * per_page) as usize;
        let end = (start + per_page as usize).min(articles.len());
        let page_articles = if start >= articles.len() {
            Vec::new()
        } else {
            articles.drain(start..end).collect()
        };
        let total_pages = if total > 0 {
            (total + per_page - 1) / per_page
        } else {
            0
        };

        Ok(AdminBlogArticleList {
            articles: page_articles,
            total,
            page,
            per_page,
            total_pages,
        })
    }

    /// Get one article for the admin editor, preferring the draft if present.
    pub async fn admin_get_article(&self, id: &str) -> Result<Option<AdminBlogArticle>> {
        let base_id = published_id(id);
        let query = format!(
            r#"*[_type == "article" && _id in [{}, {}]] {{ {} }}"#,
            groq_string(&base_id),
            groq_string(&draft_id(&base_id)),
            article_projection()
        );
        let rows: Vec<SanityArticle> = self.admin_query(&query).await?;
        Ok(group_admin_articles(rows, &self.studio_url, None)
            .into_iter()
            .next())
    }

    /// Create a new draft article in Sanity.
    pub async fn admin_create_article(&self, input: AdminArticleInput) -> Result<AdminBlogArticle> {
        validate_article_input(&input)?;
        let base_id = uuid::Uuid::new_v4().to_string();
        let doc = article_input_to_document(draft_id(&base_id), input, None, false);
        self.mutate(json!([{ "create": doc }])).await?;
        self.admin_get_article(&base_id)
            .await?
            .context("Created article could not be reloaded")
    }

    /// Save article edits as a Sanity draft.
    pub async fn admin_update_article(
        &self,
        id: &str,
        input: AdminArticleInput,
    ) -> Result<AdminBlogArticle> {
        validate_article_input(&input)?;
        let base_id = published_id(id);
        let existing = self
            .admin_get_article(&base_id)
            .await?
            .context("Article not found")?;

        if let (Some(expected), Some(current)) =
            (input.revision.as_deref(), existing.revision.as_deref())
        {
            if expected != current {
                bail!("Article was modified by another editor; refresh before saving");
            }
        }

        let doc = article_input_to_document(draft_id(&base_id), input, Some(existing), false);
        self.mutate(json!([{ "createOrReplace": doc }])).await?;
        self.admin_get_article(&base_id)
            .await?
            .context("Updated article could not be reloaded")
    }

    /// Publish a draft by copying it to the published root ID and deleting the draft.
    pub async fn admin_publish_article(&self, id: &str) -> Result<AdminBlogArticle> {
        let base_id = published_id(id);
        let draft = self
            .load_raw_article(&draft_id(&base_id))
            .await?
            .or(self.load_raw_article(&base_id).await?)
            .context("Article not found")?;
        let now = chrono::Utc::now().to_rfc3339();
        let mut doc = raw_article_to_document(draft, base_id.clone());
        set_object_value(&mut doc, "status", json!("published"));
        if doc.get("publishedAt").and_then(Value::as_str).is_none() {
            set_object_value(&mut doc, "publishedAt", json!(now));
        }
        self.mutate(json!([
            { "createOrReplace": doc },
            { "delete": { "id": draft_id(&base_id) } }
        ]))
        .await?;
        self.admin_get_article(&base_id)
            .await?
            .context("Published article could not be reloaded")
    }

    /// Take an article down publicly while preserving an editable draft.
    pub async fn admin_unpublish_article(&self, id: &str) -> Result<AdminBlogArticle> {
        self.move_published_to_draft(id, "taken_down").await
    }

    /// Archive an article and remove any public version.
    pub async fn admin_archive_article(&self, id: &str) -> Result<AdminBlogArticle> {
        self.move_published_to_draft(id, "archived").await
    }

    /// Restore an archived/taken-down article to a normal draft.
    pub async fn admin_restore_article(&self, id: &str) -> Result<AdminBlogArticle> {
        let base_id = published_id(id);
        let source = self
            .load_raw_article(&draft_id(&base_id))
            .await?
            .or(self.load_raw_article(&base_id).await?)
            .context("Article not found")?;
        let mut doc = raw_article_to_document(source, draft_id(&base_id));
        set_object_value(&mut doc, "status", json!("draft"));
        self.mutate(json!([{ "createOrReplace": doc }])).await?;
        self.admin_get_article(&base_id)
            .await?
            .context("Restored article could not be reloaded")
    }

    /// List categories through private API for admin screens.
    pub async fn admin_list_categories(&self) -> Result<Vec<CategoryResponse>> {
        let query = format!(
            r#"*[_type == "category"] | order(coalesce(sortOrder, 9999) asc, name asc) {{
                _id, name, "slug": slug.current, description, color, icon, metaTitle, metaDescription,
                "articleCount": count(*[_type == "article" && references(^._id)])
            }}"#
        );
        let rows: Vec<SanityCategory> = self.admin_query(&query).await?;
        Ok(rows.into_iter().map(map_category).collect())
    }

    /// List authors through private API for admin screens.
    pub async fn admin_list_authors(&self) -> Result<Vec<AuthorResponse>> {
        let query = r#"*[_type == "author"] | order(name asc) {
            _id, name, "slug": slug.current, bio, "avatarUrl": coalesce(avatar.asset->url, avatarUrl),
            websiteUrl, twitterHandle, linkedinUrl, facebookUrl, instagramUrl, whatsapp, expertise,
            "articleCount": count(*[_type == "article" && references(^._id)])
        }"#;
        let rows: Vec<SanityAuthor> = self.admin_query(query).await?;
        Ok(rows.into_iter().map(map_author).collect())
    }

    /// Create or update a Sanity category.
    pub async fn admin_save_category(&self, input: AdminTaxonomyInput) -> Result<CategoryResponse> {
        validate_taxonomy_input(&input)?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let doc = json!({
            "name": input.name,
            "slug": { "_type": "slug", "current": slugify(&input.slug) },
            "description": input.description,
            "color": input.color,
            "sortOrder": input.sort_order.unwrap_or(0),
        });
        if input.id.is_some() {
            self.mutate(json!([{ "patch": { "id": id, "set": doc } }]))
                .await?;
        } else {
            let mut doc = doc;
            set_object_value(&mut doc, "_id", json!(id));
            set_object_value(&mut doc, "_type", json!("category"));
            self.mutate(json!([{ "create": doc }])).await?;
        }
        let query = format!(
            r#"*[_type == "category" && _id == {}][0] {{
                _id, name, "slug": slug.current, description, color, icon, metaTitle, metaDescription,
                "articleCount": count(*[_type == "article" && references(^._id)])
            }}"#,
            groq_string(&id)
        );
        let row: SanityCategory = self.admin_query(&query).await?;
        Ok(map_category(row))
    }

    /// Create or update a Sanity author.
    pub async fn admin_save_author(&self, input: AdminTaxonomyInput) -> Result<AuthorResponse> {
        validate_taxonomy_input(&input)?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let doc = json!({
            "_id": id,
            "_type": "author",
            "name": input.name,
            "slug": { "_type": "slug", "current": slugify(&input.slug) },
            "bio": input.bio.or(input.description),
            "avatarUrl": input.avatar_url,
            "websiteUrl": input.website_url,
            "twitterHandle": input.twitter_handle.map(|handle| handle.trim_start_matches('@').to_string()),
            "linkedinUrl": input.linkedin_url,
            "facebookUrl": input.facebook_url,
            "instagramUrl": input.instagram_url,
            "whatsapp": input.whatsapp,
            "expertise": input.expertise.unwrap_or_default(),
        });
        self.mutate(json!([{ "createOrReplace": doc }])).await?;
        let query = format!(
            r#"*[_type == "author" && _id == {}][0] {{
                _id, name, "slug": slug.current, bio, "avatarUrl": coalesce(avatar.asset->url, avatarUrl),
                websiteUrl, twitterHandle, linkedinUrl, facebookUrl, instagramUrl, whatsapp, expertise,
                "articleCount": count(*[_type == "article" && references(^._id)])
            }}"#,
            groq_string(&id)
        );
        let row: SanityAuthor = self.admin_query(&query).await?;
        Ok(map_author(row))
    }

    /// Upload an image asset to Sanity.
    pub async fn admin_upload_image(
        &self,
        bytes: Vec<u8>,
        filename: Option<String>,
        content_type: Option<String>,
    ) -> Result<Value> {
        let token = self
            .write_token
            .as_deref()
            .context("SANITY_WRITE_TOKEN is not configured")?;
        let url = format!(
            "https://{}.api.sanity.io/v{}/assets/images/{}",
            self.project_id, self.api_version, self.dataset
        );
        let mut request = self
            .http
            .post(url)
            .bearer_auth(token)
            .header(
                reqwest::header::CONTENT_TYPE,
                content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
            )
            .body(bytes);
        if let Some(filename) = filename.filter(|name| !name.is_empty()) {
            request = request.query(&[("filename", filename)]);
        }
        let envelope: Value = request
            .send()
            .await
            .context("Sanity asset upload request failed")?
            .error_for_status()
            .context("Sanity asset upload returned an error status")?
            .json()
            .await
            .context("Sanity asset upload response could not be decoded")?;
        Ok(envelope)
    }

    /// Import existing DB blog tables into Sanity.
    pub async fn import_database_blog(
        &self,
        pool: &sqlx::PgPool,
        dry_run: bool,
    ) -> Result<AdminBlogImportResult> {
        use sqlx::Row;

        let authors = sqlx::query(
            "SELECT id::text, name, slug, bio, avatar_url, website_url, twitter_handle, linkedin_url, facebook_url, instagram_url, whatsapp, expertise FROM blog_authors ORDER BY created_at ASC",
        )
        .fetch_all(pool)
        .await
        .context("Failed to load DB blog authors")?;
        let categories = sqlx::query(
            "SELECT id::text, name, slug, description, color, icon, sort_order, meta_title, meta_description FROM blog_categories ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(pool)
        .await
        .context("Failed to load DB blog categories")?;
        let articles = sqlx::query(
            "SELECT id::text, slug, title, subtitle, excerpt, content, content_html, meta_title, meta_description, canonical_url, og_image_url, author_id::text, category_id::text, tags, cover_image_url, reading_time_minutes, featured, schema_type, faq_data, status, published_at::text, scheduled_at::text, created_at::text, updated_at::text FROM blog_articles ORDER BY created_at ASC",
        )
        .fetch_all(pool)
        .await
        .context("Failed to load DB blog articles")?;

        let slugs_query = r#"*[_type == "article" && defined(slug.current)].slug.current"#;
        let existing_slugs: Vec<String> = self.admin_query(slugs_query).await.unwrap_or_default();
        let existing: std::collections::HashSet<String> = existing_slugs.iter().cloned().collect();

        let mut result = AdminBlogImportResult {
            dry_run,
            source_articles: articles.len() as i64,
            source_authors: authors.len() as i64,
            source_categories: categories.len() as i64,
            existing_sanity_slugs: existing_slugs,
            imported_articles: 0,
            imported_authors: 0,
            imported_categories: 0,
            skipped_articles: 0,
            errors: Vec::new(),
        };

        if dry_run {
            result.skipped_articles = articles
                .iter()
                .filter_map(|row| row.try_get::<String, _>("slug").ok())
                .filter(|slug| existing.contains(slug))
                .count() as i64;
            return Ok(result);
        }

        let mut mutations = Vec::new();
        for row in authors {
            let id: String = row.try_get("id")?;
            mutations.push(json!({
                "createOrReplace": {
                    "_id": id,
                    "_type": "author",
                    "name": row.try_get::<String, _>("name").unwrap_or_default(),
                    "slug": { "_type": "slug", "current": row.try_get::<String, _>("slug").unwrap_or_default() },
                    "bio": row.try_get::<Option<String>, _>("bio").unwrap_or(None),
                    "avatarUrl": row.try_get::<Option<String>, _>("avatar_url").unwrap_or(None),
                    "websiteUrl": row.try_get::<Option<String>, _>("website_url").unwrap_or(None),
                    "twitterHandle": row.try_get::<Option<String>, _>("twitter_handle").unwrap_or(None),
                    "linkedinUrl": row.try_get::<Option<String>, _>("linkedin_url").unwrap_or(None),
                    "facebookUrl": row.try_get::<Option<String>, _>("facebook_url").unwrap_or(None),
                    "instagramUrl": row.try_get::<Option<String>, _>("instagram_url").unwrap_or(None),
                    "whatsapp": row.try_get::<Option<String>, _>("whatsapp").unwrap_or(None),
                    "expertise": row.try_get::<Vec<String>, _>("expertise").unwrap_or_default(),
                }
            }));
            result.imported_authors += 1;
        }
        for row in categories {
            let id: String = row.try_get("id")?;
            mutations.push(json!({
                "createOrReplace": {
                    "_id": id,
                    "_type": "category",
                    "name": row.try_get::<String, _>("name").unwrap_or_default(),
                    "slug": { "_type": "slug", "current": row.try_get::<String, _>("slug").unwrap_or_default() },
                    "description": row.try_get::<Option<String>, _>("description").unwrap_or(None),
                    "color": row.try_get::<Option<String>, _>("color").unwrap_or(None),
                    "icon": row.try_get::<Option<String>, _>("icon").unwrap_or(None),
                    "sortOrder": row.try_get::<i32, _>("sort_order").unwrap_or(0),
                    "metaTitle": row.try_get::<Option<String>, _>("meta_title").unwrap_or(None),
                    "metaDescription": row.try_get::<Option<String>, _>("meta_description").unwrap_or(None),
                }
            }));
            result.imported_categories += 1;
        }
        for row in articles {
            let slug: String = row.try_get("slug")?;
            let id: String = row.try_get("id")?;
            let existing_article_id = if existing.contains(&slug) {
                self.find_article_id_by_slug(&slug).await?
            } else {
                None
            };
            let sanity_id = existing_article_id.unwrap_or_else(|| id.clone());
            let status: String = row
                .try_get("status")
                .unwrap_or_else(|_| "draft".to_string());
            let published = status == "published";
            let doc_id = if published {
                published_id(&sanity_id)
            } else {
                draft_id(&sanity_id)
            };
            let content: String = row.try_get("content").unwrap_or_default();
            mutations.push(json!({
                "createOrReplace": {
                    "_id": doc_id,
                    "_type": "article",
                    "title": row.try_get::<String, _>("title").unwrap_or_default(),
                    "slug": { "_type": "slug", "current": slug },
                    "subtitle": row.try_get::<Option<String>, _>("subtitle").unwrap_or(None),
                    "excerpt": row.try_get::<String, _>("excerpt").unwrap_or_default(),
                    "body": text_to_portable_blocks(&content),
                    "bodyText": content,
                    "metaTitle": row.try_get::<Option<String>, _>("meta_title").unwrap_or(None),
                    "metaDescription": row.try_get::<Option<String>, _>("meta_description").unwrap_or(None),
                    "canonicalUrl": row.try_get::<Option<String>, _>("canonical_url").unwrap_or(None),
                    "ogImageUrl": row.try_get::<Option<String>, _>("og_image_url").unwrap_or(None),
                    "author": { "_type": "reference", "_ref": row.try_get::<String, _>("author_id").unwrap_or_default() },
                    "category": { "_type": "reference", "_ref": row.try_get::<String, _>("category_id").unwrap_or_default() },
                    "tags": row.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
                    "coverImageUrl": row.try_get::<Option<String>, _>("cover_image_url").unwrap_or(None),
                    "readingTimeMinutes": row.try_get::<Option<i32>, _>("reading_time_minutes").unwrap_or(Some(5)).unwrap_or(5),
                    "featured": row.try_get::<bool, _>("featured").unwrap_or(false),
                    "schemaType": row.try_get::<Option<String>, _>("schema_type").unwrap_or(Some("BlogPosting".to_string())).unwrap_or_else(|| "BlogPosting".to_string()),
                    "faqData": row.try_get::<Option<Value>, _>("faq_data").unwrap_or(None),
                    "status": if published { "published" } else { status.as_str() },
                    "publishedAt": normalize_datetime(row.try_get::<Option<String>, _>("published_at").unwrap_or(None)),
                    "scheduledAt": normalize_datetime(row.try_get::<Option<String>, _>("scheduled_at").unwrap_or(None)),
                }
            }));
            if existing.contains(&slug) {
                result.skipped_articles += 1;
            } else {
                result.imported_articles += 1;
            }
        }

        for chunk in mutations.chunks(50) {
            self.mutate(Value::Array(chunk.to_vec())).await?;
        }

        Ok(result)
    }

    async fn move_published_to_draft(&self, id: &str, status: &str) -> Result<AdminBlogArticle> {
        let base_id = published_id(id);
        let source = self
            .load_raw_article(&draft_id(&base_id))
            .await?
            .or(self.load_raw_article(&base_id).await?)
            .context("Article not found")?;
        let mut doc = raw_article_to_document(source, draft_id(&base_id));
        set_object_value(&mut doc, "status", json!(status));
        set_object_value(&mut doc, "publishedAt", Value::Null);
        self.mutate(json!([
            { "createOrReplace": doc },
            { "delete": { "id": base_id } }
        ]))
        .await?;
        self.admin_get_article(id)
            .await?
            .context("Article could not be reloaded")
    }

    async fn load_raw_article(&self, id: &str) -> Result<Option<SanityArticle>> {
        let query = format!(
            r#"*[_type == "article" && _id == {}][0] {{ {} }}"#,
            groq_string(id),
            article_projection()
        );
        self.private_query(&query).await
    }

    async fn find_article_id_by_slug(&self, slug: &str) -> Result<Option<String>> {
        let query = format!(
            r#"*[_type == "article" && slug.current == {}][0]._id"#,
            groq_string(slug)
        );
        self.admin_query(&query).await
    }

    async fn private_query<T>(&self, query: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let token = self
            .read_token
            .as_deref()
            .or(self.write_token.as_deref())
            .context("SANITY_READ_TOKEN or SANITY_WRITE_TOKEN is not configured")?;
        let url = format!(
            "https://{}.api.sanity.io/v{}/data/query/{}",
            self.project_id, self.api_version, self.dataset
        );
        let envelope: SanityEnvelope<T> = self
            .http
            .get(url)
            .bearer_auth(token)
            .query(&[("query", query)])
            .send()
            .await
            .context("Sanity private query request failed")?
            .error_for_status()
            .context("Sanity private query returned an error status")?
            .json()
            .await
            .context("Sanity private query response could not be decoded")?;
        Ok(envelope.result)
    }

    async fn admin_query<T>(&self, query: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        if self.read_token.is_some() || self.write_token.is_some() {
            self.private_query(query).await
        } else {
            self.query(query).await
        }
    }

    async fn mutate(&self, mutations: Value) -> Result<Value> {
        let token = self
            .write_token
            .as_deref()
            .context("SANITY_WRITE_TOKEN is not configured")?;
        let url = format!(
            "https://{}.api.sanity.io/v{}/data/mutate/{}",
            self.project_id, self.api_version, self.dataset
        );
        let result: Value = self
            .http
            .post(url)
            .bearer_auth(token)
            .query(&[
                ("returnDocuments", "true"),
                ("visibility", "sync"),
                ("autoGenerateArrayKeys", "true"),
                ("tag", "poool.admin.blog"),
            ])
            .json(&json!({ "mutations": mutations }))
            .send()
            .await
            .context("Sanity mutation request failed")?
            .error_for_status()
            .context("Sanity mutation returned an error status")?
            .json()
            .await
            .context("Sanity mutation response could not be decoded")?;
        Ok(result)
    }

    async fn query<T>(&self, query: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!(
            "https://{}.apicdn.sanity.io/v{}/data/query/{}",
            self.project_id, self.api_version, self.dataset
        );
        let envelope: SanityEnvelope<T> = self
            .http
            .get(url)
            .query(&[("query", query)])
            .send()
            .await
            .context("Sanity CDN request failed")?
            .error_for_status()
            .context("Sanity CDN returned an error status")?
            .json()
            .await
            .context("Sanity CDN response could not be decoded")?;

        Ok(envelope.result)
    }
}

fn article_filter(category_slug: Option<&str>, tag: Option<&str>, featured_only: bool) -> String {
    let mut parts = vec![
        r#"_type == "article""#.to_string(),
        r#"!(_id in path("drafts.**"))"#.to_string(),
        "defined(slug.current)".to_string(),
        "defined(publishedAt)".to_string(),
        r#"coalesce(status, "published") == "published""#.to_string(),
        "publishedAt <= now()".to_string(),
    ];

    if let Some(slug) = category_slug.filter(|s| is_safe_slug(s)) {
        parts.push(format!("category->slug.current == {}", groq_string(slug)));
    }

    if let Some(tag) = tag.filter(|s| is_safe_tag(s)) {
        parts.push(format!("{} in tags[]", groq_string(tag)));
    }

    if featured_only {
        parts.push("featured == true".to_string());
    }

    parts.join(" && ")
}

fn article_projection() -> &'static str {
    r#"
        _id,
        _rev,
        "slug": slug.current,
        title,
        subtitle,
        excerpt,
        body,
        "content": pt::text(body),
        "metaTitle": metaTitle,
        "metaDescription": metaDescription,
        canonicalUrl,
        "ogImageUrl": coalesce(ogImage.asset->url, coverImage.asset->url),
        "coverImageUrl": coalesce(coverImage.asset->url, coverImageUrl),
        "coverImageAssetRef": coverImage.asset._ref,
        readingTimeMinutes,
        featured,
        shareLinks,
        status,
        schemaType,
        faqData,
        tags,
        language,
        locale,
        lang,
        "translations": coalesce(translations[]->{
            _id,
            "slug": slug.current,
            title,
            status,
            language,
            locale,
            lang
        }, []),
        "translationMetadata": coalesce(*[_type == "translation.metadata" && references(^._id)][0].translations[].value->{
            _id,
            "slug": slug.current,
            title,
            status,
            language,
            locale,
            lang
        }, []),
        "publishedAt": publishedAt,
        "scheduledAt": scheduledAt,
        _createdAt,
        _updatedAt,
        author->{
            _id,
            name,
            "slug": slug.current,
            bio,
            "avatarUrl": avatar.asset->url,
            websiteUrl,
            twitterHandle,
            linkedinUrl,
            facebookUrl,
            instagramUrl,
            whatsapp,
            expertise
        },
        category->{
            _id,
            name,
            "slug": slug.current,
            description,
            color,
            icon,
            metaTitle,
            metaDescription
        }
    "#
}

fn group_admin_articles(
    rows: Vec<SanityArticle>,
    studio_url: &str,
    base_url: Option<&str>,
) -> Vec<AdminBlogArticle> {
    let mut grouped: HashMap<String, (Option<SanityArticle>, Option<SanityArticle>)> =
        HashMap::new();

    for row in rows {
        let is_draft = row.id.starts_with("drafts.");
        let key = published_id(&row.id);
        let entry = grouped.entry(key).or_insert((None, None));
        if is_draft {
            entry.0 = Some(row);
        } else {
            entry.1 = Some(row);
        }
    }

    let mut articles: Vec<AdminBlogArticle> = grouped
        .into_iter()
        .filter_map(|(base_id, (draft, published))| {
            let has_draft = draft.is_some();
            let has_published = published.is_some();
            let status = derive_admin_status(draft.as_ref(), published.as_ref());
            let preferred = draft.as_ref().or(published.as_ref())?;
            let slug = preferred.slug.clone().unwrap_or_default();
            let body_text = preferred
                .content
                .clone()
                .unwrap_or_else(|| plain_text_from_portable(&preferred.body));
            Some(AdminBlogArticle {
                id: base_id.clone(),
                draft_id: has_draft.then(|| draft_id(&base_id)),
                published_id: base_id.clone(),
                revision: preferred.revision.clone(),
                slug: slug.clone(),
                title: preferred
                    .title
                    .clone()
                    .unwrap_or_else(|| "Untitled".to_string()),
                subtitle: preferred.subtitle.clone(),
                excerpt: preferred.excerpt.clone().unwrap_or_default(),
                body: preferred
                    .body
                    .clone()
                    .unwrap_or_else(|| text_to_portable_blocks(&body_text)),
                body_text,
                meta_title: preferred.meta_title.clone(),
                meta_description: preferred.meta_description.clone(),
                canonical_url: preferred.canonical_url.clone(),
                og_image_url: preferred.og_image_url.clone(),
                cover_image_url: preferred.cover_image_url.clone(),
                cover_image_asset_ref: preferred.cover_image_asset_ref.clone(),
                author: preferred.author.as_ref().map(admin_author_summary),
                category: preferred.category.as_ref().map(admin_category_summary),
                tags: preferred.tags.clone().unwrap_or_default(),
                featured: preferred.featured.unwrap_or(false),
                share_links: preferred
                    .share_links
                    .clone()
                    .map(article_share_links_from_sanity)
                    .unwrap_or_default(),
                status: if has_published && has_draft && status == "published" {
                    "changes_pending".to_string()
                } else {
                    status
                },
                published_at: preferred.published_at.clone(),
                scheduled_at: preferred.scheduled_at.clone(),
                schema_type: preferred
                    .schema_type
                    .clone()
                    .unwrap_or_else(|| "BlogPosting".to_string()),
                created_at: preferred.created_at.clone(),
                updated_at: preferred.updated_at.clone(),
                live_url: base_url
                    .map(|url| format!("{}/blog/{}", url.trim_end_matches('/'), slug)),
                studio_url: sanity_edit_url(studio_url, &base_id),
                translation_status: translation_status_for(preferred),
            })
        })
        .collect();

    articles.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    articles
}

fn derive_admin_status(draft: Option<&SanityArticle>, published: Option<&SanityArticle>) -> String {
    let preferred_status = draft
        .and_then(|d| d.status.as_deref())
        .or_else(|| published.and_then(|p| p.status.as_deref()));

    match preferred_status {
        Some("archived") => return "archived".to_string(),
        Some("taken_down") => return "taken_down".to_string(),
        Some("scheduled") => return "scheduled".to_string(),
        _ => {}
    }

    if published.is_some() && draft.is_some() {
        return "changes_pending".to_string();
    }
    if let Some(published) = published {
        if let Some(date) = published.published_at.as_deref() {
            if chrono::DateTime::parse_from_rfc3339(date)
                .map(|dt| dt.with_timezone(&chrono::Utc) > chrono::Utc::now())
                .unwrap_or(false)
            {
                return "scheduled".to_string();
            }
        }
        return "published".to_string();
    }

    "draft".to_string()
}

fn translation_status_for(article: &SanityArticle) -> Vec<AdminTranslationStatus> {
    let candidates = article.translations.as_deref().unwrap_or(&[]).iter().chain(
        article
            .translation_metadata
            .as_deref()
            .unwrap_or(&[])
            .iter(),
    );

    [("id", "Indonesian"), ("de", "German"), ("ru", "Russian")]
        .into_iter()
        .map(|(code, label)| {
            let self_matches = article_language_matches(
                code,
                [
                    article.language.as_deref(),
                    article.locale.as_deref(),
                    article.lang.as_deref(),
                ],
            );
            let translation = candidates.clone().find(|translation| {
                article_language_matches(
                    code,
                    [
                        translation.language.as_deref(),
                        translation.locale.as_deref(),
                        translation.lang.as_deref(),
                    ],
                )
            });

            AdminTranslationStatus {
                code: code.to_string(),
                label: label.to_string(),
                present: self_matches || translation.is_some(),
                document_id: translation.and_then(|t| t.id.clone()),
                status: translation
                    .and_then(|t| t.status.clone())
                    .or_else(|| self_matches.then(|| article.status.clone()).flatten()),
                slug: translation
                    .and_then(|t| t.slug.clone())
                    .or_else(|| self_matches.then(|| article.slug.clone()).flatten()),
                title: translation
                    .and_then(|t| t.title.clone())
                    .or_else(|| self_matches.then(|| article.title.clone()).flatten()),
            }
        })
        .collect()
}

fn article_language_matches<'a>(
    required_code: &str,
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> bool {
    values.into_iter().flatten().any(|value| {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        normalized == required_code
            || normalized.starts_with(&format!("{required_code}-"))
            || language_name_to_code(&normalized) == Some(required_code)
    })
}

fn language_name_to_code(value: &str) -> Option<&'static str> {
    match value {
        "indonesian" | "bahasa indonesia" | "indonesia" => Some("id"),
        "german" | "deutsch" => Some("de"),
        "russian" | "русский" | "russisch" => Some("ru"),
        _ => None,
    }
}

fn admin_author_summary(author: &SanityAuthor) -> AdminTaxonomySummary {
    AdminTaxonomySummary {
        id: author.id.clone().unwrap_or_else(|| "unknown".to_string()),
        name: author
            .name
            .clone()
            .unwrap_or_else(|| "POOOL Editorial".to_string()),
        slug: author
            .slug
            .clone()
            .unwrap_or_else(|| "poool-editorial".to_string()),
    }
}

fn admin_category_summary(category: &SanityCategory) -> AdminTaxonomySummary {
    AdminTaxonomySummary {
        id: category
            .id
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string()),
        name: category
            .name
            .clone()
            .unwrap_or_else(|| "Insights".to_string()),
        slug: category
            .slug
            .clone()
            .unwrap_or_else(|| "insights".to_string()),
    }
}

fn article_input_to_document(
    id: String,
    input: AdminArticleInput,
    existing: Option<AdminBlogArticle>,
    publishing: bool,
) -> Value {
    let body = input
        .body
        .unwrap_or_else(|| text_to_portable_blocks(input.body_text.as_deref().unwrap_or_default()));
    let published_at = if publishing {
        input
            .published_at
            .clone()
            .or_else(|| Some(chrono::Utc::now().to_rfc3339()))
    } else {
        input.published_at.clone()
    };

    json!({
        "_id": id,
        "_type": "article",
        "title": input.title,
        "slug": { "_type": "slug", "current": slugify(&input.slug) },
        "subtitle": input.subtitle,
        "excerpt": input.excerpt,
        "body": body,
        "bodyText": input.body_text,
        "metaTitle": input.meta_title,
        "metaDescription": input.meta_description,
        "canonicalUrl": input.canonical_url,
        "ogImageUrl": input.og_image_url,
        "coverImageUrl": input.cover_image_url,
        "coverImage": input.cover_image_asset_ref.as_ref().map(|asset_ref| json!({
            "_type": "image",
            "asset": { "_type": "reference", "_ref": asset_ref }
        })),
        "shareLinks": input.share_links.map(sanity_share_links_from_article),
        "author": input.author_id.or_else(|| existing.as_ref().and_then(|a| a.author.as_ref().map(|x| x.id.clone()))).map(|id| json!({
            "_type": "reference",
            "_ref": id
        })),
        "category": input.category_id.or_else(|| existing.as_ref().and_then(|a| a.category.as_ref().map(|x| x.id.clone()))).map(|id| json!({
            "_type": "reference",
            "_ref": id
        })),
        "tags": input.tags.unwrap_or_default(),
        "featured": input.featured.unwrap_or(false),
        "status": if publishing { "published".to_string() } else { input.status.unwrap_or_else(|| "draft".to_string()) },
        "publishedAt": published_at,
        "scheduledAt": input.scheduled_at,
        "schemaType": input.schema_type.unwrap_or_else(|| "BlogPosting".to_string()),
    })
}

fn raw_article_to_document(row: SanityArticle, id: String) -> Value {
    json!({
        "_id": id,
        "_type": "article",
        "title": row.title,
        "slug": row.slug.map(|slug| json!({ "_type": "slug", "current": slug })),
        "subtitle": row.subtitle,
        "excerpt": row.excerpt,
        "body": row.body,
        "bodyText": row.content,
        "metaTitle": row.meta_title,
        "metaDescription": row.meta_description,
        "canonicalUrl": row.canonical_url,
        "ogImageUrl": row.og_image_url,
        "coverImageUrl": row.cover_image_url,
        "coverImage": row.cover_image_asset_ref.as_ref().map(|asset_ref| json!({
            "_type": "image",
            "asset": { "_type": "reference", "_ref": asset_ref }
        })),
        "shareLinks": row.share_links.map(article_share_links_from_sanity),
        "author": row.author.and_then(|a| a.id).map(|id| json!({ "_type": "reference", "_ref": id })),
        "category": row.category.and_then(|c| c.id).map(|id| json!({ "_type": "reference", "_ref": id })),
        "tags": row.tags.unwrap_or_default(),
        "featured": row.featured.unwrap_or(false),
        "status": row.status.unwrap_or_else(|| "draft".to_string()),
        "publishedAt": row.published_at,
        "scheduledAt": row.scheduled_at,
        "schemaType": row.schema_type.unwrap_or_else(|| "BlogPosting".to_string()),
        "faqData": row.faq_data,
    })
}

fn set_object_value(doc: &mut Value, key: &str, value: Value) {
    if let Some(object) = doc.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

fn validate_article_input(input: &AdminArticleInput) -> Result<()> {
    if input.title.trim().is_empty() {
        bail!("Title is required");
    }
    if !is_safe_slug(&slugify(&input.slug)) {
        bail!("Slug is required and may only contain lowercase letters, numbers, and hyphens");
    }
    if input.excerpt.trim().is_empty() {
        bail!("Excerpt is required");
    }
    Ok(())
}

fn validate_taxonomy_input(input: &AdminTaxonomyInput) -> Result<()> {
    if input.name.trim().is_empty() {
        bail!("Name is required");
    }
    if !is_safe_slug(&slugify(&input.slug)) {
        bail!("Slug is required and may only contain lowercase letters, numbers, and hyphens");
    }
    Ok(())
}

fn published_id(id: &str) -> String {
    id.strip_prefix("drafts.").unwrap_or(id).to_string()
}

fn draft_id(id: &str) -> String {
    let base = published_id(id);
    format!("drafts.{base}")
}

fn sanity_edit_url(studio_url: &str, id: &str) -> String {
    if studio_url.is_empty() || studio_url.contains("sanity.io/manage") {
        return studio_url.to_string();
    }
    format!(
        "{}/desk/article;{}",
        studio_url.trim_end_matches('/'),
        url::form_urlencoded::byte_serialize(id.as_bytes()).collect::<String>()
    )
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn normalize_datetime(value: Option<String>) -> Option<String> {
    let value = value?;
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&value) {
        return Some(dt.with_timezone(&chrono::Utc).to_rfc3339());
    }
    if let Ok(dt) = chrono::DateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f%#z") {
        return Some(dt.with_timezone(&chrono::Utc).to_rfc3339());
    }
    if let Ok(dt) = chrono::DateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f%z") {
        return Some(dt.with_timezone(&chrono::Utc).to_rfc3339());
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(dt.and_utc().to_rfc3339());
    }
    Some(value)
}

fn text_to_portable_blocks(text: &str) -> Value {
    let blocks: Vec<Value> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (style, content) = if let Some(rest) = line.strip_prefix("### ") {
                ("h3", rest)
            } else if let Some(rest) = line.strip_prefix("## ") {
                ("h2", rest)
            } else if let Some(rest) = line.strip_prefix("> ") {
                ("blockquote", rest)
            } else {
                ("normal", line)
            };
            json!({
                "_type": "block",
                "_key": uuid::Uuid::new_v4().to_string().replace('-', ""),
                "style": style,
                "markDefs": [],
                "children": [{
                    "_type": "span",
                    "_key": uuid::Uuid::new_v4().to_string().replace('-', ""),
                    "text": content,
                    "marks": []
                }]
            })
        })
        .collect();
    Value::Array(blocks)
}

fn plain_text_from_portable(value: &Option<Value>) -> String {
    value
        .as_ref()
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .map(|block| {
                    block
                        .get("children")
                        .and_then(Value::as_array)
                        .map(|children| {
                            children
                                .iter()
                                .filter_map(|child| child.get("text").and_then(Value::as_str))
                                .collect::<String>()
                        })
                        .unwrap_or_default()
                })
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

fn article_share_links_from_sanity(links: SanityShareLinks) -> ArticleShareLinks {
    ArticleShareLinks {
        whatsapp_url: clean_public_url(links.whatsapp_url),
        facebook_url: clean_public_url(links.facebook_url),
        x_url: clean_public_url(links.x_url),
        instagram_url: clean_public_url(links.instagram_url),
        linkedin_url: clean_public_url(links.linkedin_url),
    }
}

fn sanity_share_links_from_article(links: ArticleShareLinks) -> Value {
    json!({
        "whatsappUrl": clean_public_url(links.whatsapp_url),
        "facebookUrl": clean_public_url(links.facebook_url),
        "xUrl": clean_public_url(links.x_url),
        "instagramUrl": clean_public_url(links.instagram_url),
        "linkedinUrl": clean_public_url(links.linkedin_url),
    })
}

fn clean_public_url(value: Option<String>) -> Option<String> {
    value.map(|item| item.trim().to_string()).filter(|item| {
        let lower = item.to_ascii_lowercase();
        lower.starts_with("https://") || lower.starts_with("http://")
    })
}

fn map_article(row: SanityArticle) -> ArticleResponse {
    let content_html = row.body.as_ref().map(portable_text_to_safe_html);
    ArticleResponse {
        id: row.id,
        slug: row.slug.unwrap_or_default(),
        title: row.title.unwrap_or_else(|| "Untitled".to_string()),
        subtitle: row.subtitle,
        excerpt: row.excerpt.unwrap_or_default(),
        content: row.content,
        content_html,
        meta_title: row.meta_title,
        meta_description: row.meta_description,
        canonical_url: row.canonical_url,
        og_image_url: row.og_image_url,
        author: row
            .author
            .map(map_author_summary)
            .unwrap_or_else(|| AuthorSummary {
                id: "unknown".to_string(),
                name: "POOOL Editorial".to_string(),
                initials: "PE".to_string(),
                slug: "poool-editorial".to_string(),
                avatar_url: None,
                bio: None,
                website_url: None,
                twitter_handle: None,
                linkedin_url: None,
                facebook_url: None,
                instagram_url: None,
                whatsapp: None,
            }),
        category: row
            .category
            .map(map_category_summary)
            .unwrap_or_else(|| CategorySummary {
                id: "uncategorized".to_string(),
                name: "Insights".to_string(),
                slug: "insights".to_string(),
                color: None,
                icon: None,
            }),
        tags: row.tags.unwrap_or_default(),
        cover_image_url: row.cover_image_url,
        reading_time_minutes: row.reading_time_minutes.unwrap_or(5).max(1),
        featured: row.featured.unwrap_or(false),
        share_links: row
            .share_links
            .map(article_share_links_from_sanity)
            .unwrap_or_default(),
        schema_type: row.schema_type.unwrap_or_else(|| "BlogPosting".to_string()),
        faq_data: row.faq_data,
        published_at: row.published_at,
        created_at: row.created_at.unwrap_or_default(),
        updated_at: row.updated_at.unwrap_or_default(),
    }
}

fn map_author_summary(author: SanityAuthor) -> AuthorSummary {
    let name = author.name.unwrap_or_else(|| "POOOL Editorial".to_string());
    AuthorSummary {
        id: author.id.unwrap_or_else(|| "unknown".to_string()),
        initials: author_initials(&name),
        name,
        slug: author.slug.unwrap_or_else(|| "poool-editorial".to_string()),
        avatar_url: author.avatar_url,
        bio: author.bio,
        website_url: author.website_url,
        twitter_handle: author.twitter_handle,
        linkedin_url: author.linkedin_url,
        facebook_url: author.facebook_url,
        instagram_url: author.instagram_url,
        whatsapp: author.whatsapp,
    }
}

fn map_category_summary(category: SanityCategory) -> CategorySummary {
    CategorySummary {
        id: category.id.unwrap_or_else(|| "uncategorized".to_string()),
        name: category.name.unwrap_or_else(|| "Insights".to_string()),
        slug: category.slug.unwrap_or_else(|| "insights".to_string()),
        color: category.color,
        icon: category.icon,
    }
}

fn map_category(category: SanityCategory) -> CategoryResponse {
    CategoryResponse {
        id: category.id.unwrap_or_else(|| "uncategorized".to_string()),
        name: category.name.unwrap_or_else(|| "Insights".to_string()),
        slug: category.slug.unwrap_or_else(|| "insights".to_string()),
        description: category.description,
        color: category.color,
        icon: category.icon,
        meta_title: category.meta_title,
        meta_description: category.meta_description,
        article_count: category.article_count.unwrap_or(0),
    }
}

fn map_author(author: SanityAuthor) -> AuthorResponse {
    AuthorResponse {
        id: author.id.unwrap_or_else(|| "unknown".to_string()),
        name: author.name.unwrap_or_else(|| "POOOL Editorial".to_string()),
        slug: author.slug.unwrap_or_else(|| "poool-editorial".to_string()),
        bio: author.bio,
        avatar_url: author.avatar_url,
        website_url: author.website_url,
        twitter_handle: author.twitter_handle,
        linkedin_url: author.linkedin_url,
        facebook_url: author.facebook_url,
        instagram_url: author.instagram_url,
        whatsapp: author.whatsapp,
        expertise: author.expertise.unwrap_or_default(),
        article_count: author.article_count.unwrap_or(0),
    }
}

fn portable_text_to_safe_html(value: &Value) -> String {
    let mut html = String::new();
    if let Some(blocks) = value.as_array() {
        let mut open_list: Option<&str> = None;
        for block in blocks {
            let list_tag = block.get("listItem").and_then(Value::as_str).map(|item| {
                if item == "number" {
                    "ol"
                } else {
                    "ul"
                }
            });

            match (open_list, list_tag) {
                (Some(current), Some(next)) if current == next => {}
                (Some(current), Some(next)) => {
                    html.push_str("</");
                    html.push_str(current);
                    html.push('>');
                    html.push('<');
                    html.push_str(next);
                    html.push('>');
                    open_list = Some(next);
                }
                (None, Some(next)) => {
                    html.push('<');
                    html.push_str(next);
                    html.push('>');
                    open_list = Some(next);
                }
                (Some(current), None) => {
                    html.push_str("</");
                    html.push_str(current);
                    html.push('>');
                    open_list = None;
                }
                (None, None) => {}
            }

            html.push_str(&block_to_html(block));
        }

        if let Some(current) = open_list {
            html.push_str("</");
            html.push_str(current);
            html.push('>');
        }
    }

    Builder::default()
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
        ])
        .add_tag_attributes("a", ["href", "target"])
        .link_rel(Some("noopener noreferrer"))
        .clean(&html)
        .to_string()
}

fn block_to_html(block: &Value) -> String {
    let block_type = block
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if block_type != "block" {
        return String::new();
    }

    let style = block
        .get("style")
        .and_then(Value::as_str)
        .unwrap_or("normal");
    let children = block_children_html(block);

    if block.get("listItem").and_then(Value::as_str).is_some() {
        return format!("<li>{}</li>", children);
    }

    match style {
        "h2" => format!("<h2>{}</h2>", children),
        "h3" => format!("<h3>{}</h3>", children),
        "h4" => format!("<h4>{}</h4>", children),
        "blockquote" => format!("<blockquote>{}</blockquote>", children),
        _ => format!("<p>{}</p>", children),
    }
}

fn block_children_html(block: &Value) -> String {
    block
        .get("children")
        .and_then(Value::as_array)
        .map(|children| {
            let mark_defs = collect_mark_defs(block);
            children
                .iter()
                .map(|child| span_to_html(child, &mark_defs))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn span_to_html(span: &Value, mark_defs: &HashMap<String, String>) -> String {
    let text = span
        .get("text")
        .and_then(Value::as_str)
        .map(escape_html)
        .unwrap_or_default();
    let mut output = text;

    if let Some(marks) = span.get("marks").and_then(Value::as_array) {
        for mark in marks.iter().filter_map(Value::as_str) {
            output = match mark {
                "strong" => format!("<strong>{}</strong>", output),
                "em" => format!("<em>{}</em>", output),
                "code" => format!("<code>{}</code>", output),
                key => {
                    if let Some(href) = mark_defs.get(key) {
                        format!(
                            r#"<a href="{}" target="_blank" rel="noopener noreferrer">{}</a>"#,
                            escape_attr(href),
                            output
                        )
                    } else {
                        output
                    }
                }
            };
        }
    }

    output
}

fn collect_mark_defs(block: &Value) -> HashMap<String, String> {
    block
        .get("markDefs")
        .and_then(Value::as_array)
        .map(|defs| {
            defs.iter()
                .filter_map(|def| {
                    let key = def.get("_key")?.as_str()?;
                    let href = def.get("href")?.as_str()?;
                    Some((key.to_string(), href.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn groq_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn is_safe_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_safe_tag(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ')
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    escape_html(value).replace('"', "&quot;")
}
