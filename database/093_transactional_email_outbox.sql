-- Migration 093: Generic transactional email outbox
-- Durable delivery with retry for KYC, deposit, and other transactional events.

CREATE TABLE IF NOT EXISTS transactional_email_outbox (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_type      VARCHAR(100) NOT NULL,
    recipient_email VARCHAR(255) NOT NULL,
    subject         VARCHAR(500) NOT NULL,
    html_body       TEXT NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued', 'sending', 'sent', 'failed')),
    attempts        INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error      TEXT,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transactional_email_outbox_ready
    ON transactional_email_outbox (status, next_attempt_at, created_at)
    WHERE status IN ('queued', 'failed');

CREATE INDEX IF NOT EXISTS idx_transactional_email_outbox_user
    ON transactional_email_outbox (user_id);

CREATE TRIGGER set_transactional_email_outbox_updated_at
    BEFORE UPDATE ON transactional_email_outbox
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
