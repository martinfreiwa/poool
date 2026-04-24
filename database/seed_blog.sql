-- Seed a test author
INSERT INTO blog_authors (id, name, slug, bio, avatar_url, twitter_handle, linkedin_url, expertise)
VALUES (
    '11111111-1111-1111-1111-111111111111',
    'Martin Freiwald',
    'martin-freiwald',
    'Co-founder of POOOL. Passionate about real estate tokenization and fractional investing.',
    '/images/martin_pfp.png',
    'poool_finance',
    'https://www.linkedin.com/company/poool-finance/',
    '{"real estate", "tokenization", "fintech"}'
) ON CONFLICT (slug) DO NOTHING;

-- Seed a test article
INSERT INTO blog_articles (
    slug,
    title,
    subtitle,
    excerpt,
    content,
    author_id,
    category_id,
    tags,
    cover_image_url,
    reading_time_minutes,
    status,
    published_at
)
VALUES (
    'real-estate-101',
    'Real Estate Investing 101: A Beginner''s Guide to Fractional Ownership',
    'How to start building wealth with as little as $50.',
    'Dive deep into the mechanics of fractional real estate ownership and why it''s the future of property investing.',
    '# Introduction to Fractional Real Estate

Investing in real estate has traditionally been an exclusive club for those with significant capital. With fractional ownership, the game has fundamentally changed.

## What is Fractional Ownership?

Instead of buying a $500,000 property outright, it is broken down into digital shares (or tokens), meaning you can buy a slice of that property for as little as $50.

### The Benefits
1. **Lower barrier to entry**: Anyone can invest.
2. **Diversification**: Spread your risk across multiple properties or cities.
3. **Liquidity**: Digital tokens can be traded much faster than traditional real estate.

> "Tokenization is not a buzzword; it is a profound upgrade to our financial plumbing."

If you''re looking to get started, you are in the right place. Dive into the POOOL marketplace today to see live fractional properties.',
    '11111111-1111-1111-1111-111111111111',
    (SELECT id FROM blog_categories WHERE slug = 'investment-guides'),
    '{"real estate", "guide", "beginners"}',
    '/images/villa1.webp',
    4,
    'published',
    NOW()
) ON CONFLICT (slug) DO NOTHING;
