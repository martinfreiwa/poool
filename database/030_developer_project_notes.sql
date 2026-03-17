-- 030: Developer project admin notes history
-- Each admin can add notes to a developer project submission.
-- Notes are immutable once written (append-only history).

CREATE TABLE IF NOT EXISTS developer_project_notes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id   UUID NOT NULL REFERENCES developer_projects(id) ON DELETE CASCADE,
    author_id    UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    content      TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dev_project_notes_project ON developer_project_notes(project_id, created_at DESC);
