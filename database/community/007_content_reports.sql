CREATE TABLE content_reports (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id         UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    reporter_id     UUID NOT NULL,  -- Referenz auf Core-DB users.id
    reason          TEXT NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'resolved', 'dismissed')),
    admin_notes     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, reporter_id) -- Ein User kann einen Post nur 1x reporten
);

CREATE INDEX idx_content_reports_post_id ON content_reports(post_id);
CREATE INDEX idx_content_reports_status ON content_reports(status);
