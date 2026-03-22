CREATE TABLE comments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    content         TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 2000),
    content_sanitized TEXT,
    helpful_count   INTEGER NOT NULL DEFAULT 0,
    is_hidden       BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
