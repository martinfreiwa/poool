-- 186_step_up_sessions.sql
--
-- Durable fallback for 15-minute step-up 2FA sessions. Redis remains the
-- fast path, but Redis is optional in the app config, so sensitive
-- operations must not become impossible when Redis is absent.

CREATE TABLE IF NOT EXISTS step_up_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action VARCHAR(40) NOT NULL CHECK (action IN ('withdraw', 'trade', 'pm', 'pwd')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, action)
);

CREATE INDEX IF NOT EXISTS idx_step_up_sessions_user_action_expires
    ON step_up_sessions(user_id, action, expires_at DESC);
