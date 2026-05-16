-- ══════════════════════════════════════════════════════════════
-- 181_deposit_source_of_funds.sql
--
-- AMLD5/6 Article 18 source-of-funds capture for fiat deposits.
--
-- Thresholds (both editable in Admin → Settings → Deposits):
--   deposit_sof_threshold_cents     ($3,000) — reason required
--   deposit_sof_doc_threshold_cents ($10,000) — supporting document required
--
-- The deposit modal collects the reason between the amount entry and the
-- wire-instructions step. Documents follow the same upload pipeline as
-- the proof-of-transfer file (GCS upload, 15-min signed URLs for admin).
-- ══════════════════════════════════════════════════════════════

ALTER TABLE deposit_requests
    ADD COLUMN IF NOT EXISTS source_of_funds_reason   VARCHAR(40),
    ADD COLUMN IF NOT EXISTS source_of_funds_detail   TEXT,
    ADD COLUMN IF NOT EXISTS source_of_funds_doc_path TEXT;

-- Allow only the curated enum values. NULL stays valid for legacy rows
-- and for deposits below the SoF threshold.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
         WHERE conname = 'deposit_requests_sof_reason_check'
    ) THEN
        ALTER TABLE deposit_requests
            ADD CONSTRAINT deposit_requests_sof_reason_check
            CHECK (
                source_of_funds_reason IS NULL
                OR source_of_funds_reason IN (
                    'salary', 'inheritance', 'business_income', 'investment',
                    'crypto_sale', 'gift', 'savings', 'other'
                )
            );
    END IF;
END$$;

-- Helps admin search for deposits that should have a declaration.
CREATE INDEX IF NOT EXISTS idx_deposit_requests_sof_reason
    ON deposit_requests(source_of_funds_reason)
    WHERE source_of_funds_reason IS NOT NULL;

INSERT INTO platform_settings (key, value, value_type, description) VALUES
  ('deposit_sof_threshold_cents',
   '300000',
   'number',
   'Above this amount (cents), the deposit modal requires a source-of-funds reason. Default: $3,000.'),
  ('deposit_sof_doc_threshold_cents',
   '1000000',
   'number',
   'Above this amount (cents), the deposit modal also requires a supporting document. Default: $10,000.')
ON CONFLICT (key) DO NOTHING;
