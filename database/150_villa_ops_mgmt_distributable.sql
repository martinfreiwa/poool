-- 150 — Villa-Returns: add mgmt_reported_distributable_idr_cents to villa_operations_log.
--
-- PDF §2 lists "Distributable property amount" as a management-company monthly input:
-- the actual amount they transferred to the owner/SPV after their own deductions.
-- This is distinct from distributable_idr_cents (POOOL's calculated value) and is used
-- for reconciliation during admin review.
-- Nullable: management company may omit it; admin flags any discrepancy.

ALTER TABLE villa_operations_log
    ADD COLUMN IF NOT EXISTS mgmt_reported_distributable_idr_cents BIGINT;

COMMENT ON COLUMN villa_operations_log.mgmt_reported_distributable_idr_cents
    IS 'Developer-submitted: actual amount transferred by management company to SPV/owner (PDF §2 "Distributable property amount"). Used by admin to reconcile against POOOL-calculated distributable_idr_cents.';
