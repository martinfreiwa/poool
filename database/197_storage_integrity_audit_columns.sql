-- ═══════════════════════════════════════════════════════════════════
-- Migration 178: Storage Integrity + Audit Columns
-- ═══════════════════════════════════════════════════════════════════
--
-- Adds the three columns every uploaded file should carry to satisfy:
--   - ISO 27001 A.12.2.1 / DSGVO Art. 32 (integrity): `content_sha256`
--   - BAIT 8.3 / GwG §10 (audit trail): `uploaded_ip`, `uploaded_user_agent`
--
-- Backfill strategy:
--   - Existing rows keep `content_sha256 = NULL`. The application treats
--     NULL as "unknown" and skips integrity verification for legacy rows
--     until the first re-download triggers a one-shot back-fill (see
--     `storage::service::ensure_checksum`).
--   - `uploaded_ip` / `uploaded_user_agent` default to NULL for legacy
--     rows — there is no way to reconstruct them; they're tracked
--     prospectively only.
--
-- Two-table change because both kyc_documents AND asset_documents are
-- production storage tables. Same shape applied for code-uniformity.

-- ── kyc_documents ──────────────────────────────────────────────────
ALTER TABLE kyc_documents
  ADD COLUMN IF NOT EXISTS content_sha256        CHAR(64),
  ADD COLUMN IF NOT EXISTS content_size_bytes    BIGINT,
  ADD COLUMN IF NOT EXISTS uploaded_ip           INET,
  ADD COLUMN IF NOT EXISTS uploaded_user_agent   TEXT;

COMMENT ON COLUMN kyc_documents.content_sha256
  IS 'Hex-encoded SHA-256 of the uploaded bytes. NULL = legacy row pre-2026-05-16.';
COMMENT ON COLUMN kyc_documents.content_size_bytes
  IS 'Original byte length of the upload (verifies against GCS object metadata).';
COMMENT ON COLUMN kyc_documents.uploaded_ip
  IS 'Source IP of the upload — BAIT 8.3 audit trail.';
COMMENT ON COLUMN kyc_documents.uploaded_user_agent
  IS 'User-Agent string at upload time — additional audit context.';

-- ── asset_documents ────────────────────────────────────────────────
ALTER TABLE asset_documents
  ADD COLUMN IF NOT EXISTS content_sha256        CHAR(64),
  ADD COLUMN IF NOT EXISTS uploaded_ip           INET,
  ADD COLUMN IF NOT EXISTS uploaded_user_agent   TEXT,
  ADD COLUMN IF NOT EXISTS uploaded_by_user_id   UUID REFERENCES users(id) ON DELETE SET NULL;

COMMENT ON COLUMN asset_documents.content_sha256
  IS 'Hex-encoded SHA-256 of the uploaded bytes. NULL = legacy row.';
COMMENT ON COLUMN asset_documents.uploaded_ip
  IS 'Source IP of the upload — BAIT 8.3 audit trail.';
COMMENT ON COLUMN asset_documents.uploaded_user_agent
  IS 'User-Agent string at upload time.';
COMMENT ON COLUMN asset_documents.uploaded_by_user_id
  IS 'Developer / admin who uploaded the doc. Soft FK (ON DELETE SET NULL) — preserves audit row even after user-delete.';

-- ── Index for de-duplication queries ────────────────────────────────
-- Allows "have we already stored a file with this hash?" lookups in O(log n)
-- without scanning the full table.
CREATE INDEX IF NOT EXISTS idx_kyc_documents_sha256
  ON kyc_documents(content_sha256) WHERE content_sha256 IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_asset_documents_sha256
  ON asset_documents(content_sha256) WHERE content_sha256 IS NOT NULL;
