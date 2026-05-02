-- ============================================================
-- 103: Asset Change Requests — Reviewer Assignment
-- ============================================================
-- Adds optional reviewer-assignment column so admins can
-- claim/route pending change requests during triage.
-- ============================================================

ALTER TABLE asset_change_requests
    ADD COLUMN IF NOT EXISTS assigned_to UUID REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_acr_assigned_to
    ON asset_change_requests(assigned_to)
    WHERE assigned_to IS NOT NULL;
