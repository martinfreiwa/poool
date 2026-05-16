-- Migration 182: extend transactional_email_outbox.status to allow 'skipped'.
--
-- Phase-2 P0: when a recipient has opted out of optional notifications
-- (List-Unsubscribe header / settings toggle), the dispatcher no longer
-- delivers the row but still wants to record the disposition rather than
-- delete the queued message. New terminal state: 'skipped'.
--
-- Idempotent. Safe to re-run.

ALTER TABLE transactional_email_outbox
    DROP CONSTRAINT IF EXISTS transactional_email_outbox_status_check;

ALTER TABLE transactional_email_outbox
    ADD CONSTRAINT transactional_email_outbox_status_check
    CHECK (status IN ('queued', 'sending', 'sent', 'failed', 'skipped'));
