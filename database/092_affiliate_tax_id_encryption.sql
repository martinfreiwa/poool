-- Store affiliate Tax IDs as encrypted payloads with only a display suffix.
-- Existing plaintext tax_id values are treated as legacy and cleared on the next
-- successful onboarding/settings write after the user re-enters the Tax ID.

ALTER TABLE affiliates
    ADD COLUMN IF NOT EXISTS tax_id_encrypted TEXT,
    ADD COLUMN IF NOT EXISTS tax_id_last4 VARCHAR(4);

CREATE INDEX IF NOT EXISTS idx_affiliates_tax_id_encrypted_present
    ON affiliates (user_id)
    WHERE tax_id_encrypted IS NOT NULL;
