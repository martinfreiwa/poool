-- ============================================================
-- POOOL Platform
-- Migration 007: Support Ticket Replies
-- ============================================================

CREATE TABLE IF NOT EXISTS support_ticket_replies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id       UUID NOT NULL REFERENCES support_tickets(id) ON DELETE CASCADE,
    author_id       UUID NOT NULL REFERENCES users(id),
    author_name     VARCHAR(200) NOT NULL,
    author_role     VARCHAR(20) NOT NULL DEFAULT 'user'
                    CHECK (author_role IN ('customer', 'user', 'agent', 'admin')),
    type            VARCHAR(30) NOT NULL DEFAULT 'reply'
                    CHECK (type IN ('initial', 'reply', 'internal_note')),
    content         TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ticket_replies_ticket ON support_ticket_replies(ticket_id);
CREATE INDEX IF NOT EXISTS idx_ticket_replies_author ON support_ticket_replies(author_id);
