-- ============================================================
-- POOOL Platform – Migration 013: Admin Background Jobs & Idempotency
-- Adds missing tables and columns identified during the
-- Admin Dashboard audit.
-- ============================================================

-- ============================================================
-- 1. background_jobs (System Health & Operations)
-- ============================================================
CREATE TABLE IF NOT EXISTS background_jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_name        VARCHAR(255) NOT NULL,
    status          VARCHAR(30) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled')),
    attempts        INTEGER NOT NULL DEFAULT 0,
    max_attempts    INTEGER NOT NULL DEFAULT 3,
    payload         JSONB,
    error_message   TEXT,
    run_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_background_jobs_status ON background_jobs(status);
CREATE INDEX IF NOT EXISTS idx_background_jobs_run_at ON background_jobs(run_at);

-- ============================================================
-- 2. idempotency_keys
-- ============================================================
CREATE TABLE IF NOT EXISTS idempotency_keys (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key                 VARCHAR(255) NOT NULL UNIQUE,
    user_id             UUID REFERENCES users(id) ON DELETE SET NULL,
    request_path        VARCHAR(512) NOT NULL,
    request_method      VARCHAR(10) NOT NULL,
    request_payload     JSONB,
    response_status     INTEGER,
    response_body       JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ NOT NULL DEFAULT (NOW() + interval '24 hours')
);
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_key ON idempotency_keys(key);
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_user ON idempotency_keys(user_id);

-- ============================================================
-- 3. Add missing columns to user_settings
-- ============================================================
ALTER TABLE user_settings ADD COLUMN IF NOT EXISTS currency VARCHAR(3) DEFAULT 'USD';
ALTER TABLE user_settings ADD COLUMN IF NOT EXISTS timezone VARCHAR(100) DEFAULT 'UTC';

-- ============================================================
-- 4. Add missing columns to kyc_records
-- ============================================================
ALTER TABLE kyc_records ADD COLUMN IF NOT EXISTS risk_score INTEGER;

-- ============================================================
-- 5. webhook_logs
-- ============================================================
CREATE TABLE IF NOT EXISTS webhook_logs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider            VARCHAR(50) NOT NULL,
    endpoint            VARCHAR(255) NOT NULL,
    http_status         INTEGER,
    payload             JSONB,
    processed           BOOLEAN NOT NULL DEFAULT FALSE,
    error_message       TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_provider ON webhook_logs(provider);
CREATE INDEX IF NOT EXISTS idx_webhook_logs_created ON webhook_logs(created_at);

-- ============================================================
-- 6. Add missing columns to payment_methods
-- ============================================================
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS bank_country VARCHAR(3);
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS routing_number VARCHAR(50);

-- ============================================================
-- 7. Add missing columns to support_tickets
-- ============================================================
ALTER TABLE support_tickets ADD COLUMN IF NOT EXISTS category VARCHAR(50) DEFAULT 'general'
CHECK (category IN (
    'general', 'account', 'deposits', 'withdrawals',
    'investments', 'kyc', 'technical', 'billing', 'other'
));

-- ============================================================
-- Trigger for updated_at on background_jobs
-- ============================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'set_updated_at' AND tgrelid = 'background_jobs'::regclass
    ) THEN
        CREATE TRIGGER set_updated_at BEFORE UPDATE ON background_jobs
        FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
    END IF;
END;
$$;

-- ============================================================
-- Done! 🎉
-- ============================================================
