-- 107: Admin notes per trade — append-only audit log
-- Lets admin team annotate trades during fraud investigations and
-- provide context for compliance review. Immutable once written.

CREATE TABLE IF NOT EXISTS trade_admin_notes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trade_id    UUID NOT NULL REFERENCES trade_history(id) ON DELETE CASCADE,
    author_id   UUID NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    content     TEXT NOT NULL CHECK (char_length(content) BETWEEN 1 AND 2000),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_trade_admin_notes_trade
    ON trade_admin_notes (trade_id, created_at DESC);
