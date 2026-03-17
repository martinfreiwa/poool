-- ============================================================
-- POOOL Platform
-- Migration 020: Support Tickets Enhancements (SLA, Analytics, Attachments)
-- ============================================================

-- 1. Enhance support_tickets table

-- Update status constraint to include 'waiting_on_customer'
ALTER TABLE support_tickets 
DROP CONSTRAINT IF EXISTS support_tickets_status_check;

ALTER TABLE support_tickets 
ADD CONSTRAINT support_tickets_status_check 
CHECK (status IN ('open', 'in_progress', 'waiting_on_customer', 'resolved', 'closed'));

-- Add new columns
ALTER TABLE support_tickets
ADD COLUMN IF NOT EXISTS category VARCHAR(50),
ADD COLUMN IF NOT EXISTS metadata JSONB,
ADD COLUMN IF NOT EXISTS sla_breach_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS sla_alert_sent BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS csat_score SMALLINT CHECK (csat_score >= 1 AND csat_score <= 5),
ADD COLUMN IF NOT EXISTS csat_feedback TEXT;

-- 2. Create support_ticket_attachments table
CREATE TABLE IF NOT EXISTS support_ticket_attachments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reply_id        UUID NOT NULL REFERENCES support_ticket_replies(id) ON DELETE CASCADE,
    file_url        VARCHAR(512) NOT NULL,
    file_type       VARCHAR(100),
    file_size_bytes BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_support_attachments_reply ON support_ticket_attachments(reply_id);
