-- ============================================================
-- Migration 018: GCS File Storage Support
-- Adds kyc_documents table for user-uploaded identity documents.
-- Avatar URLs are already stored in users.avatar_url (VARCHAR(512)).
-- Property images already use asset_images / asset_documents tables.
-- ============================================================

-- ────────────────────────────────────────────────────────────────
-- kyc_documents
--
-- Stores metadata about identity documents uploaded by users as part
-- of the manual KYC flow.  The actual files live in GCS under:
--   gs://{GCS_BUCKET_NAME}/kyc/{user_id}/{uuid}.{ext}
--
-- The `gcs_path`  column stores the raw "gs://" path (safe to store
-- in the DB, never served directly to users).
--
-- Signed URLs (valid 15 min) are generated on-demand in the backend
-- whenever an admin or the user needs to view the document.
-- ────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS kyc_documents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- What kind of document was uploaded
    document_type   VARCHAR(50) NOT NULL
                    CHECK (document_type IN (
                        'passport', 'national_id', 'driving_licence',
                        'proof_of_address', 'other'
                    )),

    -- GCS object path, e.g. "gs://poool-user-uploads/kyc/{uid}/{file-id}.pdf"
    gcs_path        TEXT NOT NULL,

    -- Review state (set by admin or automated KYC flow)
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected')),

    rejection_reason TEXT,

    -- Optional link back to a kyc_records row (if document is
    -- associated with a full KYC session via Didit/manual)
    kyc_record_id   UUID REFERENCES kyc_records(id) ON DELETE SET NULL,

    uploaded_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at     TIMESTAMPTZ
);

CREATE INDEX idx_kyc_docs_user   ON kyc_documents(user_id);
CREATE INDEX idx_kyc_docs_status ON kyc_documents(status);

-- ────────────────────────────────────────────────────────────────
-- Widen avatar_url on users if it's still 512 chars
-- (already VARCHAR(512) in migration 001 – safe to re-run with IF EXISTS)
-- ────────────────────────────────────────────────────────────────
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'users' AND column_name = 'avatar_url'
          AND character_maximum_length < 1024
    ) THEN
        ALTER TABLE users ALTER COLUMN avatar_url TYPE VARCHAR(1024);
    END IF;
END
$$;
