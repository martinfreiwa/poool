-- Migration 201: extend notifications.type whitelist for Phase 3 fresh.
--
-- Adds three affiliate-domain event types that producers in
-- `crate::rewards::notifications` emit:
--
--   * affiliate_commission_clawed_back  — refund-driven reversal of a
--     previously-earned commission. Fires once per (user, investment).
--   * affiliate_policy_update_required  — a policy version the affiliate
--     accepted earlier has been superseded; re-acceptance required.
--   * affiliate_tax_doc_required        — affiliate has earned commissions
--     but tax document is missing / about to block the next payout.
--
-- Idempotent. Safe to re-run.

BEGIN;

ALTER TABLE notifications
    DROP CONSTRAINT IF EXISTS notifications_type_check;

ALTER TABLE notifications
    ADD CONSTRAINT notifications_type_check
    CHECK (type IN (
        'kyc', 'investment', 'payout', 'system', 'promo',
        -- Phase-3 P1 originals (migration 183)
        'affiliate_commission_earned',
        'affiliate_payout_released',
        'team_invitation_accepted',
        'team_member_joined',
        'team_member_removed',
        'team_invitation_received',
        -- Phase-3 fresh additions
        'affiliate_commission_clawed_back',
        'affiliate_policy_update_required',
        'affiliate_tax_doc_required'
    ));

COMMIT;
