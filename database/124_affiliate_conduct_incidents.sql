-- 124_affiliate_conduct_incidents.sql
-- Phase 1 affiliate program: persuasion / sales-conduct incident reports.
--
-- Supports blueprint Point 2 monitoring requirement: detect and record
-- one-to-one persuasion behavior, advisory selling, side-deal indicators,
-- objection-handling patterns, and other prohibited sales-conduct signals.
--
-- This is the data-capture layer. Reports may come from:
--   - customer/user complaints,
--   - internal staff/support escalation,
--   - admin moderation review of public affiliate content,
--   - automated content scanning (future).
--
-- Distinct from `affiliate_fraud_flags` (referral-ring / IP-overlap automated detection).

BEGIN;

CREATE TABLE IF NOT EXISTS affiliate_conduct_incidents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id    UUID NOT NULL REFERENCES affiliates(user_id) ON DELETE CASCADE,
    -- Type of conduct flagged (free-form for v1; UI provides a controlled list)
    incident_type   VARCHAR(64) NOT NULL,
    -- Severity: minor | serious | critical (per blueprint Point 4 §F)
    severity        VARCHAR(16) NOT NULL DEFAULT 'minor'
                      CHECK (severity IN ('minor', 'serious', 'critical')),
    -- Status: open | under_review | resolved | dismissed
    status          VARCHAR(16) NOT NULL DEFAULT 'open'
                      CHECK (status IN ('open', 'under_review', 'resolved', 'dismissed')),
    -- Source of the report
    source          VARCHAR(32) NOT NULL DEFAULT 'manual'
                      CHECK (source IN ('user_complaint', 'support_escalation',
                                         'admin_review', 'automated', 'manual')),
    -- Free-text description of the incident (what was observed)
    description     TEXT NOT NULL,
    -- Optional URL/screenshot reference for evidence (e.g. social post URL)
    evidence_url    TEXT,
    -- Optional content snippet (the offending wording)
    content_snippet TEXT,
    -- Action taken on this incident: warning | content_correction | takedown |
    --   retraining | freeze | suspension | clawback_started | permanent_removal | none
    action_taken    VARCHAR(32),
    -- Reporter (admin user id or NULL if external complaint)
    reported_by     UUID REFERENCES users(id),
    -- Reviewer (admin user id when status moved to resolved/dismissed)
    reviewed_by     UUID REFERENCES users(id),
    reviewed_at     TIMESTAMPTZ,
    -- Free-form review notes
    review_notes    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_affiliate_conduct_incidents_affiliate_id
    ON affiliate_conduct_incidents (affiliate_id);
CREATE INDEX IF NOT EXISTS idx_affiliate_conduct_incidents_status
    ON affiliate_conduct_incidents (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_affiliate_conduct_incidents_severity
    ON affiliate_conduct_incidents (severity, status);

-- Trigger to auto-update updated_at
CREATE OR REPLACE FUNCTION trg_affiliate_conduct_incidents_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS affiliate_conduct_incidents_set_updated_at
    ON affiliate_conduct_incidents;
CREATE TRIGGER affiliate_conduct_incidents_set_updated_at
    BEFORE UPDATE ON affiliate_conduct_incidents
    FOR EACH ROW
    EXECUTE FUNCTION trg_affiliate_conduct_incidents_updated_at();

COMMIT;
