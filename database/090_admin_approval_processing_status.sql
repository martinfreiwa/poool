-- Migration 090: Durable admin approval execution state
--
-- Allows approval execution to move pending requests into a non-rerunnable
-- processing state before irreversible business actions run.

ALTER TABLE admin_approval_requests
    DROP CONSTRAINT IF EXISTS admin_approval_requests_status_check;

ALTER TABLE admin_approval_requests
    ADD CONSTRAINT admin_approval_requests_status_check
    CHECK (status IN ('pending', 'processing', 'approved', 'rejected', 'expired'));
