-- ═══════════════════════════════════════════════════════════════════
-- Migration 008: Email Engine & Marketing Hub
-- Creates tables for email templates and logs, and adds Resend API Key
-- ═══════════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS email_templates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(255) NOT NULL UNIQUE,
    subject         VARCHAR(255) NOT NULL,
    html_template   TEXT NOT NULL,
    text_template   TEXT,
    version         INTEGER DEFAULT 1,
    description     VARCHAR(255),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_logs (
    id              BIGSERIAL PRIMARY KEY,
    user_id         UUID REFERENCES users(id) ON DELETE SET NULL,
    template_id     UUID REFERENCES email_templates(id) ON DELETE SET NULL,
    recipient_email VARCHAR(255) NOT NULL,
    subject         VARCHAR(255) NOT NULL,
    status          VARCHAR(50) NOT NULL, -- Sent, Delivered, Opened, Clicked, Bounced
    sent_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add Resend API key to platform_settings
INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('resend_api_key', '', 'string', 'API key for Resend email provider')
ON CONFLICT (key) DO NOTHING;
