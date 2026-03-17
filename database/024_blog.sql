-- =========================================================================
-- 024_blog.sql — Blog Tables (Authors, Categories, Articles, Relations)
-- =========================================================================
-- Supports: SEO metadata, structured data, content automation, tagging.
-- Follows the POOOL convention: UUIDs, TIMESTAMPTZ, text constraints.
-- =========================================================================

-- 1. blog_authors — Writers / content creators
CREATE TABLE IF NOT EXISTS blog_authors (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES users(id),          -- Link to platform user (nullable for guest authors)
    name            VARCHAR(200) NOT NULL,
    slug            VARCHAR(100) NOT NULL UNIQUE,
    bio             TEXT,
    avatar_url      VARCHAR(512),

    -- Social (used in Person schema markup)
    website_url     VARCHAR(512),
    twitter_handle  VARCHAR(100),
    linkedin_url    VARCHAR(512),
    facebook_url    VARCHAR(512),
    instagram_url   VARCHAR(512),
    whatsapp        VARCHAR(50),

    -- SEO
    expertise       TEXT[] DEFAULT '{}',

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_blog_authors_slug ON blog_authors(slug);
CREATE INDEX IF NOT EXISTS idx_blog_authors_user ON blog_authors(user_id);

-- 2. blog_categories — Topic buckets with optional hierarchy
CREATE TABLE IF NOT EXISTS blog_categories (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(100) NOT NULL UNIQUE,
    slug            VARCHAR(100) NOT NULL UNIQUE,
    description     TEXT,

    -- Hierarchy
    parent_id       UUID REFERENCES blog_categories(id),
    sort_order      INTEGER NOT NULL DEFAULT 0,

    -- SEO
    meta_title      VARCHAR(70),
    meta_description VARCHAR(160),
    og_image_url    VARCHAR(512),

    -- Display
    color           VARCHAR(7),         -- Accent hex: "#4F46E5"
    icon            VARCHAR(50),        -- Icon name: "building", "chart-line"

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_blog_categories_slug ON blog_categories(slug);
CREATE INDEX IF NOT EXISTS idx_blog_categories_parent ON blog_categories(parent_id);

-- 3. blog_articles — Core content table
CREATE TABLE IF NOT EXISTS blog_articles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            VARCHAR(255) NOT NULL UNIQUE,
    title           VARCHAR(300) NOT NULL,
    subtitle        VARCHAR(500),
    excerpt         TEXT NOT NULL,                      -- Used as meta description (max ~160 chars)
    content         TEXT NOT NULL,                      -- Full body in Markdown
    content_html    TEXT,                               -- Pre-rendered HTML (optional cache)

    -- SEO overrides
    meta_title      VARCHAR(70),
    meta_description VARCHAR(160),
    canonical_url   VARCHAR(512),
    og_image_url    VARCHAR(512),                       -- 1200×630 recommended

    -- Taxonomy
    author_id       UUID NOT NULL REFERENCES blog_authors(id),
    category_id     UUID NOT NULL REFERENCES blog_categories(id),
    tags            TEXT[] DEFAULT '{}',

    -- Display
    cover_image_url VARCHAR(512),
    reading_time_minutes INTEGER DEFAULT 5,

    -- Publishing
    status          VARCHAR(20) NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'review', 'scheduled', 'published', 'archived')),
    featured        BOOLEAN NOT NULL DEFAULT FALSE,

    -- Schema.org structured data type
    schema_type     VARCHAR(30) DEFAULT 'BlogPosting'
                    CHECK (schema_type IN ('Article', 'BlogPosting', 'HowTo', 'FAQPage', 'NewsArticle')),
    faq_data        JSONB,                              -- For FAQPage: [{"q":"...","a":"..."}]

    -- Timestamps
    published_at    TIMESTAMPTZ,
    scheduled_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_blog_articles_slug ON blog_articles(slug);
CREATE INDEX IF NOT EXISTS idx_blog_articles_status ON blog_articles(status);
CREATE INDEX IF NOT EXISTS idx_blog_articles_category ON blog_articles(category_id);
CREATE INDEX IF NOT EXISTS idx_blog_articles_author ON blog_articles(author_id);
CREATE INDEX IF NOT EXISTS idx_blog_articles_published ON blog_articles(published_at DESC)
    WHERE status = 'published';
CREATE INDEX IF NOT EXISTS idx_blog_articles_tags ON blog_articles USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_blog_articles_featured ON blog_articles(featured)
    WHERE featured = TRUE AND status = 'published';

-- 4. blog_article_relations — "Related articles" links
CREATE TABLE IF NOT EXISTS blog_article_relations (
    article_id      UUID NOT NULL REFERENCES blog_articles(id) ON DELETE CASCADE,
    related_id      UUID NOT NULL REFERENCES blog_articles(id) ON DELETE CASCADE,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (article_id, related_id),
    CHECK (article_id != related_id)
);

-- 5. Seed default categories
INSERT INTO blog_categories (name, slug, description, color, icon, sort_order) VALUES
    ('Investment Guides',   'investment-guides',   'Learn how to invest wisely in real-world assets.',       '#4F46E5', 'book-open',   1),
    ('Market Insights',     'market-insights',     'Analysis and trends in the real estate market.',         '#0EA5E9', 'chart-line',  2),
    ('Platform Updates',    'platform-updates',    'New features and announcements from POOOL.',             '#10B981', 'megaphone',   3),
    ('Real Estate',         'real-estate',         'Deep dives into property investment strategies.',        '#F59E0B', 'building',    4),
    ('Tokenization',        'tokenization',        'Understanding asset tokenization and fractional ownership.', '#8B5CF6', 'coins',   5)
ON CONFLICT (slug) DO NOTHING;
