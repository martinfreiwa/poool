-- Migration 200 — KYC retention machinery (GwG §8, Phase 4.1).
--
-- The German Geldwäschegesetz (GwG) §8 mandates that KYC identification
-- documents are retained for 5 years (default) to 10 years (extended)
-- after the business relationship with the customer has ended. DSGVO
-- Art. 5(1)(e) ("Speicherbegrenzung") requires us to delete them as soon
-- as the retention window expires — both retention and timely deletion
-- are mandatory.
--
-- This migration adds the machinery to express + enforce both:
--   • users.business_relationship_ended_at    → trigger that arms the clock
--   • kyc_documents.retention_until           → exact delete-deadline
--   • kyc_documents.deleted_at / deletion_reason → soft-delete trace
--   • Index for the nightly retention-worker scan
--
-- The actual delete worker lives in `backend/src/storage/retention.rs`.
-- See `docs/storage/04-compliance-and-retention.md` for the runbook.
--
-- Idempotent.

BEGIN;

-- ── users: explicit business-relationship-end timestamp ────────────
-- Set when the user account is "ended" by either:
--   (a) account deletion (user-initiated, DSGVO Art. 17)
--   (b) admin off-boarding (e.g. KYC permanently failed)
--   (c) auto-deactivation after extended inactivity (>24 months)
-- The retention clock starts here. Leave NULL while the relationship is active.
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS business_relationship_ended_at TIMESTAMPTZ;

COMMENT ON COLUMN users.business_relationship_ended_at IS
  'GwG §8 trigger: business-relationship end timestamp. NULL = active. '
  'When set, all kyc_documents.retention_until for this user is computed = this + 5y.';

CREATE INDEX IF NOT EXISTS idx_users_business_relationship_ended_at
  ON users (business_relationship_ended_at)
  WHERE business_relationship_ended_at IS NOT NULL;

-- ── kyc_documents: retention + soft-delete columns ────────────────
ALTER TABLE kyc_documents
  ADD COLUMN IF NOT EXISTS retention_until  TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS deleted_at       TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS deletion_reason  TEXT;

COMMENT ON COLUMN kyc_documents.retention_until IS
  'GwG §8 delete-deadline: computed = users.business_relationship_ended_at + 5 years. '
  'NULL = retention clock not yet armed (relationship still active).';
COMMENT ON COLUMN kyc_documents.deleted_at IS
  'Soft-delete timestamp set by retention worker (or DSGVO user-delete). '
  'GCS object is deleted at the same time. Row retained for audit trail.';
COMMENT ON COLUMN kyc_documents.deletion_reason IS
  'Why this row was deleted: gwg_retention_expired | dsgvo_user_request | admin_purge';

-- Worker scan index: WHERE retention_until IS NOT NULL AND deleted_at IS NULL
-- ORDER BY retention_until. Partial keeps the index tiny (most rows have
-- retention_until = NULL while users are active).
CREATE INDEX IF NOT EXISTS idx_kyc_documents_retention_due
  ON kyc_documents (retention_until)
  WHERE retention_until IS NOT NULL AND deleted_at IS NULL;

-- Worker reconciliation index: list everything the worker has deleted
-- for the audit dashboard.
CREATE INDEX IF NOT EXISTS idx_kyc_documents_deleted_at
  ON kyc_documents (deleted_at DESC)
  WHERE deleted_at IS NOT NULL;

-- ── Worker run-log table ──────────────────────────────────────────
-- Same shape as storage_reconcile_runs (migration 199), one row per
-- retention-worker invocation. Powers the BSI C5 ORG-08 retention-test
-- attestation.
CREATE TABLE IF NOT EXISTS kyc_retention_runs (
    id                 UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    started_at         TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    finished_at        TIMESTAMPTZ,
    rows_considered    INTEGER      NOT NULL DEFAULT 0,
    rows_due           INTEGER      NOT NULL DEFAULT 0,
    rows_deleted       INTEGER      NOT NULL DEFAULT 0,
    gcs_deletes_ok     INTEGER      NOT NULL DEFAULT 0,
    gcs_deletes_failed INTEGER      NOT NULL DEFAULT 0,
    status             TEXT         NOT NULL DEFAULT 'running'
                       CHECK (status IN ('running','success','partial','failed')),
    note               TEXT,
    -- Dry-run mode: count what would be deleted without touching anything.
    -- True for routine canaries, false for the actual nightly run.
    dry_run            BOOLEAN      NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_kyc_retention_runs_started_at
  ON kyc_retention_runs (started_at DESC);

-- ── Helper function: arm retention clock for a user ────────────────
-- Called by the DSGVO user-delete handler + by the admin off-boarding
-- flow. Setting business_relationship_ended_at + computing
-- kyc_documents.retention_until is a single transaction so the two
-- never drift.
CREATE OR REPLACE FUNCTION arm_kyc_retention_for_user(
    p_user_id UUID,
    p_retention_years INTEGER DEFAULT 5
) RETURNS INTEGER
LANGUAGE plpgsql AS $$
DECLARE
    affected INTEGER;
    end_at TIMESTAMPTZ;
BEGIN
    -- 1. Mark the relationship as ended (idempotent: don't overwrite an
    -- earlier end-date, the first one is the legally binding trigger).
    UPDATE users
       SET business_relationship_ended_at = COALESCE(business_relationship_ended_at, NOW())
     WHERE id = p_user_id
     RETURNING business_relationship_ended_at INTO end_at;

    IF end_at IS NULL THEN
        RAISE EXCEPTION 'user % not found', p_user_id;
    END IF;

    -- 2. Compute retention_until on every KYC doc for this user that
    --    doesn't already have one. Idempotent.
    UPDATE kyc_documents
       SET retention_until = end_at + (p_retention_years || ' years')::interval
     WHERE user_id = p_user_id
       AND retention_until IS NULL
       AND deleted_at IS NULL;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

COMMENT ON FUNCTION arm_kyc_retention_for_user(UUID, INTEGER) IS
  'GwG §8: arm the retention clock for a user after their business relationship ends. '
  'Sets users.business_relationship_ended_at and computes kyc_documents.retention_until.';

COMMIT;
