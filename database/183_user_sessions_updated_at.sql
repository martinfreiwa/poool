-- 183: user_sessions.updated_at for session-token rotation audit
--
-- `rotate_session_token` in backend/src/auth/service.rs:409 sets
-- updated_at = NOW() when minting a new session token (post-2FA step-up).
-- The column was missing → step-up TOTP verification 500'd on production
-- with: `column "updated_at" of relation "user_sessions" does not exist`.

BEGIN;

ALTER TABLE user_sessions
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

COMMIT;
