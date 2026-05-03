-- 114_kyc_assignments.sql
-- KYC reviewer assignments + 4-eyes principle support.
-- Adds: assigned_to (current owner), reviewed_by (first reviewer for 4-eyes lock).
-- Audit log entries for assign/escalate already flow through existing `audit_logs`.

ALTER TABLE kyc_records
    ADD COLUMN IF NOT EXISTS assigned_to       UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS assigned_at       TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS reviewed_by       UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS reviewed_at       TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_kyc_assigned_to ON kyc_records(assigned_to)
    WHERE assigned_to IS NOT NULL;

COMMENT ON COLUMN kyc_records.assigned_to IS 'Current ops owner of this case';
COMMENT ON COLUMN kyc_records.reviewed_by IS 'First reviewer (4-eyes: approver must differ)';
