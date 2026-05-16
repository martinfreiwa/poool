-- Migration 200: per-category email opt-out preferences.
--
-- Phase-7: extends the single boolean `user_settings.email_notifications`
-- into a granular per-category opt-out. Categories mirror the email
-- event-class buckets used in `common::email::is_optional_email_event`:
--
--   * affiliate          — commission_earned, payout_released
--   * team               — team_invitation_received/accepted, member_*
--   * assets             — asset_funded, dividend_payout
--   * statements         — monthly_statement, annual_statement
--   * milestones         — milestone_* (rewards / tier upgrades)
--
-- Each is a separate boolean column so the worker can `WHERE
-- email_pref_<class> = TRUE` cheaply. Existing rows default to TRUE
-- (current behaviour) so the migration is non-breaking for any user who
-- hasn't customised.
--
-- A `NULL` global `email_notifications` keeps backwards compatibility —
-- the dispatcher's existing fail-open semantics treat NULL as TRUE.

ALTER TABLE user_settings
    ADD COLUMN IF NOT EXISTS email_pref_affiliate   BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS email_pref_team        BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS email_pref_assets      BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS email_pref_statements  BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS email_pref_milestones  BOOLEAN NOT NULL DEFAULT TRUE;

COMMENT ON COLUMN user_settings.email_pref_affiliate IS
  'Phase-7: opt-out toggle for affiliate emails (commission_earned, payout_released, application_*). FALSE → dispatcher skips.';
COMMENT ON COLUMN user_settings.email_pref_team IS
  'Phase-7: opt-out toggle for team-membership emails (invitations, approvals, removals).';
COMMENT ON COLUMN user_settings.email_pref_assets IS
  'Phase-7: opt-out toggle for asset / investment emails (funded, dividend, monthly statement).';
COMMENT ON COLUMN user_settings.email_pref_statements IS
  'Phase-7: opt-out toggle for periodic statements (monthly/annual rollups).';
COMMENT ON COLUMN user_settings.email_pref_milestones IS
  'Phase-7: opt-out toggle for milestone / rewards / tier-upgrade emails.';
