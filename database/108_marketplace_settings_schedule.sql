-- ============================================================
-- 108: Marketplace Settings Scheduling
-- Allows admins to schedule a settings change for a future time.
-- A leader-locked worker applies the change at apply_at and writes
-- an audit_log entry. Cancellation supported until applied.
-- ============================================================

CREATE TABLE IF NOT EXISTS marketplace_settings_schedule (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scheduled_state JSONB NOT NULL,
    apply_at        TIMESTAMPTZ NOT NULL,
    status          VARCHAR(15) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'applied', 'cancelled', 'failed')),
    created_by      UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    applied_at      TIMESTAMPTZ,
    error_message   TEXT,
    note            TEXT
);

CREATE INDEX IF NOT EXISTS idx_mp_settings_schedule_due
    ON marketplace_settings_schedule(apply_at)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_mp_settings_schedule_status
    ON marketplace_settings_schedule(status, apply_at DESC);
