CREATE TABLE posts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,  -- Referenz auf Core-DB users.id (kein FK!)
    post_type       VARCHAR(20) NOT NULL DEFAULT 'announcement'
                    CHECK (post_type IN ('general', 'market_insight', 'milestone',
                                         'farm_update', 'announcement', 'review')),
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 5000),
    content_sanitized TEXT,          -- Ammonia-bereinigter Content
    asset_id        UUID,            -- Optional: Referenz auf Core-DB assets.id
    image_urls      TEXT[],          -- GCS-Pfade zu Post-Bildern (max 4)
    is_pinned       BOOLEAN NOT NULL DEFAULT false,
    is_hidden       BOOLEAN NOT NULL DEFAULT false,
    hidden_reason   TEXT,
    disclaimer_shown BOOLEAN NOT NULL DEFAULT false,
    reaction_count  INTEGER NOT NULL DEFAULT 0,   -- Denormalisiert für Performance
    comment_count   INTEGER NOT NULL DEFAULT 0,   -- Denormalisiert für Performance
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
