-- Migration 113: Fraud Finding Assignments
-- Tracks ops ownership of syndicate-fraud findings (circular rings, IP overlaps, etc).
-- Findings have no DB id (re-derived per scan), so we key by a stable string identifier
-- composed in the application layer (e.g. "ring:<sorted-uuid-pair>", "ip:<ip>:<ids>").

CREATE TABLE IF NOT EXISTS fraud_finding_assignments (
    finding_id        TEXT PRIMARY KEY,
    scan_type         VARCHAR(32) NOT NULL,
    assignee_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    assigned_by       UUID REFERENCES users(id) ON DELETE SET NULL,
    note              TEXT,
    status            VARCHAR(20) NOT NULL DEFAULT 'open'
                      CHECK (status IN ('open', 'investigating', 'resolved', 'dismissed')),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fraud_finding_assignments_assignee
    ON fraud_finding_assignments(assignee_user_id);
CREATE INDEX IF NOT EXISTS idx_fraud_finding_assignments_status
    ON fraud_finding_assignments(status);
