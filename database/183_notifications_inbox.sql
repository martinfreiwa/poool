-- Migration 183: extend the existing `notifications` table for Phase-3 P1
-- in-app inbox.
--
-- Pre-Phase-3 the table existed but was scoped to KYC / investment /
-- payout / system / promo only. The Phase-3 inbox surfaces affiliate
-- events (commission earned, payout released, team member joined) plus
-- generic actor metadata. We extend rather than replace so existing
-- producers keep working.
--
-- Changes:
--   1. Add `metadata JSONB DEFAULT '{}'` for structured per-event payload
--      (amount_cents, currency, team_name, member_email, etc.).
--   2. Replace the restrictive type CHECK with a wider whitelist that
--      includes the affiliate / team events.
--   3. Add a per-user unread-count index that doesn't already exist
--      (the existing `idx_notifications_unread` is fine — no-op).
--   4. Helper function `purge_old_notifications(days)` for retention.
--
-- Idempotent. Safe to re-run.

ALTER TABLE notifications
    ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}';

ALTER TABLE notifications
    DROP CONSTRAINT IF EXISTS notifications_type_check;
ALTER TABLE notifications
    ADD CONSTRAINT notifications_type_check
    CHECK (type IN (
        'kyc', 'investment', 'payout', 'system', 'promo',
        -- Phase-3 P1: affiliate inbox events
        'affiliate_commission_earned',
        'affiliate_payout_released',
        'team_invitation_accepted',
        'team_member_joined',
        'team_member_removed',
        'team_invitation_received'
    ));

CREATE OR REPLACE FUNCTION purge_old_notifications(retain_days INT DEFAULT 90)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    purged BIGINT;
BEGIN
    WITH d AS (
        DELETE FROM notifications
         WHERE is_read = TRUE
           AND created_at < NOW() - (retain_days || ' days')::INTERVAL
         RETURNING id
    )
    SELECT COUNT(*) INTO purged FROM d;
    RETURN purged;
END;
$$;

COMMENT ON TABLE notifications IS
  'Phase-3 P1 in-app notifications feed. Surfaced via topbar bell icon. Cursor-paginated by (created_at, id) DESC.';
COMMENT ON COLUMN notifications.metadata IS
  'Structured per-event payload (e.g. amount_cents, currency, team_name, member_email). Frontend renderers may inspect specific keys per event type.';
