-- 155: Mark legacy referral_tracking as deprecated / read-only
--
-- Audit finding GAP-07: legacy `referral_tracking` and new
-- `affiliate_referrals` ran in parallel. Both systems credited users on
-- different conditions ($1000 threshold for legacy, commission-on-order
-- for affiliate). A user attributed via both tables could earn rewards
-- twice for the same investment activity.
--
-- This migration is the schema half of the GAP-07 close. The application
-- code removed:
--   - INSERT into referral_tracking on signup (auth/routes.rs)
--   - check_and_qualify_referral() called from payments completion path
--   - api_admin_referral_update PATCH endpoint
-- Existing rows are kept intact for historical dashboards (leaderboard,
-- community, user rewards page) and for the double-payout guard in
-- check_and_track_affiliate_commission.
--
-- Schema action: a table-level COMMENT documents the deprecation so
-- future engineers and SQL tools see it.

COMMENT ON TABLE referral_tracking IS
    'DEPRECATED 2026-05-15 (GAP-07): read-only history. New referral attribution writes to affiliate_referrals. No application code should INSERT or UPDATE this table.';
