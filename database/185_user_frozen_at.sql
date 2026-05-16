-- ══════════════════════════════════════════════════════════════
-- 185_user_frozen_at.sql
--
-- P1-8 — Stamp when an account was frozen so the admin queue can
-- prioritize older freezes and the user-facing "request review"
-- flow can show "your account has been frozen for N hours/days".
-- ══════════════════════════════════════════════════════════════

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS frozen_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS frozen_reason VARCHAR(64),
    ADD COLUMN IF NOT EXISTS unfreeze_requested_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_users_frozen
    ON users(frozen_at DESC)
    WHERE status = 'frozen';
