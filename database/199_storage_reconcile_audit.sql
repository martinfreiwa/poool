-- Migration 199 — Storage reconciliation audit trail (Phase 3.3).
--
-- Persists every run of the DB↔GCS drift detector + each anomaly it
-- finds, so the operator has a single SQL query to triage and BSI C5
-- RB-15 (DR-Tests) gets attestation-grade evidence.
--
-- See docs/storage/03-backup-and-disaster-recovery.md → "Layer 3 —
-- Reconciliation Job" for the runbook.
--
-- Idempotent: safe to re-run.

BEGIN;

-- One row per reconciler invocation.
CREATE TABLE IF NOT EXISTS storage_reconcile_runs (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at     TIMESTAMPTZ,
    -- Which source table did this run scan? (kyc_documents, asset_documents, ...)
    source_table    TEXT        NOT NULL,
    -- Bucket that was sampled against.
    bucket          TEXT        NOT NULL,
    -- Counters populated as the run progresses; reset on retry.
    rows_scanned    INTEGER     NOT NULL DEFAULT 0,
    objects_scanned INTEGER     NOT NULL DEFAULT 0,
    missing_objects INTEGER     NOT NULL DEFAULT 0,
    orphan_objects  INTEGER     NOT NULL DEFAULT 0,
    hash_mismatches INTEGER     NOT NULL DEFAULT 0,
    size_mismatches INTEGER     NOT NULL DEFAULT 0,
    -- 'running' | 'success' | 'partial' | 'failed'
    status          TEXT        NOT NULL DEFAULT 'running'
                    CHECK (status IN ('running','success','partial','failed')),
    -- Free-form note: triggering operator, container id, error message on failure.
    note            TEXT,
    -- If an alert was raised, the Sentry event id (uuid string).
    sentry_event_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_storage_reconcile_runs_started_at
    ON storage_reconcile_runs (started_at DESC);

CREATE INDEX IF NOT EXISTS idx_storage_reconcile_runs_status
    ON storage_reconcile_runs (status)
    WHERE status IN ('running','failed','partial');

-- One row per anomaly. A clean run inserts zero rows here.
CREATE TABLE IF NOT EXISTS storage_reconcile_findings (
    id            BIGSERIAL    PRIMARY KEY,
    run_id        UUID         NOT NULL REFERENCES storage_reconcile_runs(id) ON DELETE CASCADE,
    -- The DB row id whose object was being checked (NULL when this finding
    -- is an orphan-object discovery with no DB row to point at).
    source_id     UUID,
    source_table  TEXT         NOT NULL,
    object_path   TEXT         NOT NULL,
    -- 'missing_object' | 'orphan_object' | 'hash_mismatch' | 'size_mismatch'
    kind          TEXT         NOT NULL
                  CHECK (kind IN ('missing_object','orphan_object','hash_mismatch','size_mismatch')),
    -- 'info' | 'warning' | 'critical'
    severity      TEXT         NOT NULL DEFAULT 'warning'
                  CHECK (severity IN ('info','warning','critical')),
    -- Detail JSON. For hash_mismatch: { expected_sha256, actual_crc32c, expected_size, actual_size }.
    detail        JSONB        NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    -- Triage flag: nulled until an operator acknowledges; once set, this
    -- finding is excluded from "open findings" dashboards.
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by UUID
);

CREATE INDEX IF NOT EXISTS idx_storage_reconcile_findings_run_id
    ON storage_reconcile_findings (run_id);

CREATE INDEX IF NOT EXISTS idx_storage_reconcile_findings_open
    ON storage_reconcile_findings (severity, created_at DESC)
    WHERE acknowledged_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_storage_reconcile_findings_kind
    ON storage_reconcile_findings (kind, created_at DESC);

COMMIT;
