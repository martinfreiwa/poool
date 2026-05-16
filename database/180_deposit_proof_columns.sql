-- ══════════════════════════════════════════════════════════════
-- 180_deposit_proof_columns.sql
--
-- Capture mandatory proof-of-transfer for fiat deposits.
--
-- Flow:
--   1. User opens deposit modal, enters amount, sees bank-wire details
--      + the unique reference. Modal requires a proof upload (PDF/PNG/JPG)
--      before the request can be submitted.
--   2. handle_deposit (wallet/routes.rs) reads the multipart form, uploads
--      the proof file to GCS, and persists the gs:// URI here.
--   3. Admin "Deposits" page shows a "View proof" button that mints a
--      time-limited signed URL via storage::service::generate_signed_url.
--   4. Admin confirms the deposit -> wallet credited.
--
-- Columns are nullable so historical rows (pre-180) stay valid.
-- ══════════════════════════════════════════════════════════════

ALTER TABLE deposit_requests
    ADD COLUMN IF NOT EXISTS proof_gcs_path     TEXT,
    ADD COLUMN IF NOT EXISTS proof_uploaded_at  TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS user_notes         TEXT;

-- Speeds up the admin filter "deposits awaiting verification with proof".
CREATE INDEX IF NOT EXISTS idx_deposit_requests_proof_present
    ON deposit_requests(status)
    WHERE proof_gcs_path IS NOT NULL;
