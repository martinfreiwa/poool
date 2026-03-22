-- ═══════════════════════════════════════════════════════════════════════
-- Module 5: Expert AMAs (Ask Me Anything)
-- Tables: amas, ama_questions, ama_question_upvotes
-- ═══════════════════════════════════════════════════════════════════════

-- ─── AMAs ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS amas (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title           VARCHAR(300) NOT NULL,
    description     TEXT,
    expert_name     VARCHAR(200) NOT NULL,
    expert_title    VARCHAR(300),
    expert_avatar_url TEXT,
    status          VARCHAR(20) NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'scheduled', 'accepting_questions',
                                      'live', 'closed', 'archived')),
    scheduled_at    TIMESTAMPTZ,
    started_at      TIMESTAMPTZ,
    ended_at        TIMESTAMPTZ,
    max_questions   INTEGER NOT NULL DEFAULT 100,
    created_by      UUID NOT NULL,       -- Admin user_id
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_amas_status ON amas(status);
CREATE INDEX IF NOT EXISTS idx_amas_scheduled ON amas(scheduled_at DESC);

-- ─── AMA Questions ──────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ama_questions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ama_id          UUID NOT NULL REFERENCES amas(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    question        TEXT NOT NULL CHECK (char_length(question) BETWEEN 10 AND 500),
    answer          TEXT,                -- Filled in by expert during AMA
    answered_by     UUID,               -- Expert/Admin user_id
    answered_at     TIMESTAMPTZ,
    upvote_count    INTEGER NOT NULL DEFAULT 0,
    is_featured     BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ama_questions_ama ON ama_questions(ama_id);
CREATE INDEX IF NOT EXISTS idx_ama_questions_user ON ama_questions(user_id);
CREATE INDEX IF NOT EXISTS idx_ama_questions_upvotes ON ama_questions(ama_id, upvote_count DESC);

-- ─── AMA Question Upvotes ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ama_question_upvotes (
    question_id     UUID NOT NULL REFERENCES ama_questions(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY(question_id, user_id)
);

-- ─── Trigger: auto-update upvote_count ──────────────────────────────
CREATE OR REPLACE FUNCTION update_ama_upvote_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE ama_questions SET upvote_count = upvote_count + 1 WHERE id = NEW.question_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE ama_questions SET upvote_count = upvote_count - 1 WHERE id = OLD.question_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_ama_upvote_count ON ama_question_upvotes;
CREATE TRIGGER trg_ama_upvote_count
    AFTER INSERT OR DELETE ON ama_question_upvotes
    FOR EACH ROW EXECUTE FUNCTION update_ama_upvote_count();
