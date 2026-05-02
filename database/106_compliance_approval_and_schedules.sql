-- 4-Eye Approval & Auto-Schedule for Compliance Exports
--
-- compliance_export_request:
--   PII exports (travel_rule today; extensible) require a SECOND admin
--   to approve before a download token is issued. The token is single-use
--   and time-bounded.
--
-- compliance_export_schedule:
--   Cron-style auto-generation of regulatory exports (e.g. OJK quarterly
--   on the first day of the next quarter, with email to compliance@).
--   Cron logic lives outside the DB; this table stores definition + state.

CREATE TABLE IF NOT EXISTS compliance_export_request (
    id              BIGSERIAL PRIMARY KEY,
    export_type     TEXT        NOT NULL,
    period_label    TEXT        NOT NULL,
    period_start    DATE,
    period_end      DATE,
    requested_by    UUID        NOT NULL REFERENCES users(id),
    requested_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    requested_reason TEXT,
    status          TEXT        NOT NULL DEFAULT 'pending',
                                                  -- 'pending' | 'approved' | 'denied' | 'used' | 'expired' | 'cancelled'
    decided_by      UUID        REFERENCES users(id),
    decided_at      TIMESTAMPTZ,
    decision_notes  TEXT,
    download_token  TEXT,                          -- random 32-byte hex; null until approved
    token_expires_at TIMESTAMPTZ,
    used_at         TIMESTAMPTZ,
    used_audit_id   BIGINT REFERENCES compliance_export_audit(id) ON DELETE SET NULL,
    CONSTRAINT compliance_export_request_self_approval
        CHECK (decided_by IS NULL OR decided_by <> requested_by)
);

CREATE INDEX IF NOT EXISTS idx_compliance_export_request_pending
    ON compliance_export_request (status, requested_at DESC)
    WHERE status = 'pending';
CREATE UNIQUE INDEX IF NOT EXISTS idx_compliance_export_request_token
    ON compliance_export_request (download_token)
    WHERE download_token IS NOT NULL;


CREATE TABLE IF NOT EXISTS compliance_export_schedule (
    id              BIGSERIAL PRIMARY KEY,
    export_type     TEXT        NOT NULL,
    cadence         TEXT        NOT NULL,         -- 'quarterly' | 'annually' | 'monthly' | 'weekly'
    delivery_email  TEXT        NOT NULL,
    format          TEXT        NOT NULL DEFAULT 'csv',  -- 'csv' | 'json'
    enabled         BOOLEAN     NOT NULL DEFAULT TRUE,
    created_by      UUID        NOT NULL REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_run_at     TIMESTAMPTZ,
    last_run_status TEXT,
    last_run_error  TEXT,
    next_run_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_compliance_export_schedule_due
    ON compliance_export_schedule (enabled, next_run_at)
    WHERE enabled = TRUE;
