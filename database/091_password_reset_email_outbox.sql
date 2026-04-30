-- Migration 091: Durable password reset email outbox
-- Ensures password reset email delivery can be retried after token creation.

CREATE TABLE IF NOT EXISTS password_reset_email_outbox (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    password_reset_token_id UUID NOT NULL REFERENCES password_reset_tokens(id) ON DELETE CASCADE,
    recipient_email     VARCHAR(255) NOT NULL,
    subject             VARCHAR(255) NOT NULL,
    html_body           TEXT NOT NULL,
    status              VARCHAR(20) NOT NULL DEFAULT 'queued'
                        CHECK (status IN ('queued', 'sending', 'sent', 'failed')),
    attempts            INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error          TEXT,
    next_attempt_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at             TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_password_reset_email_outbox_ready
    ON password_reset_email_outbox (status, next_attempt_at, created_at)
    WHERE status IN ('queued', 'failed');

CREATE INDEX IF NOT EXISTS idx_password_reset_email_outbox_token
    ON password_reset_email_outbox (password_reset_token_id);

CREATE TRIGGER set_password_reset_email_outbox_updated_at
    BEFORE UPDATE ON password_reset_email_outbox
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
