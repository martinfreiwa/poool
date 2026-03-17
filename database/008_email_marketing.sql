-- ============================================================
-- POOOL Platform
-- Migration 008: Email Engine & Marketing Hub
-- ============================================================

CREATE TABLE IF NOT EXISTS email_templates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(100) NOT NULL UNIQUE,
    subject         VARCHAR(255) NOT NULL,
    html_template   TEXT NOT NULL,
    text_template   TEXT,
    version         INTEGER NOT NULL DEFAULT 1,
    description     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_logs (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    template_id     UUID REFERENCES email_templates(id),
    subject         VARCHAR(255) NOT NULL,
    recipient_email VARCHAR(255) NOT NULL,
    status          VARCHAR(50) NOT NULL DEFAULT 'sent'
                    CHECK (status IN ('queued', 'sent', 'delivered', 'opened', 'clicked', 'bounced', 'failed', 'spam_complaint')),
    provider_id     VARCHAR(255),
    error_message   TEXT,
    sent_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at    TIMESTAMPTZ,
    opened_at       TIMESTAMPTZ,
    clicked_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_email_logs_user ON email_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_email_logs_status ON email_logs(status);
CREATE INDEX IF NOT EXISTS idx_email_logs_sent_at ON email_logs(sent_at DESC);

-- Helper: updated_at trigger function for email_templates
CREATE TRIGGER set_updated_at BEFORE UPDATE ON email_templates FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

