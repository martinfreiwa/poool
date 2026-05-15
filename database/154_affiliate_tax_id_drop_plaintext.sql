-- 143: Drop legacy plaintext tax_id, add tax_id_key_version
--
-- Audit findings closed by this migration:
--   - 092 added `tax_id_encrypted` / `tax_id_last4` but never dropped the
--     plaintext `tax_id VARCHAR(50)` column. Legacy plaintext values were
--     allowed to linger until the user re-saved their settings.
--   - Admin endpoints (api_admin_affiliates_pending, dupe-check CTE) still
--     read the plaintext column directly, so the column was an active PII
--     surface, not a dormant remnant.
--   - No key-version metadata. Rotating TAX_ID_ENCRYPTION_KEY would have
--     invalidated every stored row with no way to identify rows still on
--     the old key.
--
-- Strategy:
--   1. Add `tax_id_key_version SMALLINT NOT NULL DEFAULT 1`. Backfill to 1
--      for rows whose ciphertext starts with the current `tax_id:v1:` prefix.
--   2. For any affiliate still holding a plaintext `tax_id` but no
--      encrypted payload, force `is_tax_ready = false`. The next save in
--      affiliate-settings encrypts and clears the legacy column anyway;
--      flipping `is_tax_ready` ensures payouts are blocked until that
--      happens, so we never disclose unencrypted PII to admins by accident.
--   3. Drop `tax_id`. The column index, if any, is implicitly removed.
--
-- After apply, key rotation is a per-row UPDATE setting
-- tax_id_key_version = N and re-encrypting under key N — no schema change
-- needed.

BEGIN;

-- 1. Add version column with a safe default for existing rows.
ALTER TABLE affiliates
    ADD COLUMN IF NOT EXISTS tax_id_key_version SMALLINT NOT NULL DEFAULT 1;

-- 2. Force is_tax_ready=false for any row still on legacy plaintext only.
WITH legacy_holders AS (
    SELECT user_id
    FROM affiliates
    WHERE tax_id IS NOT NULL
      AND tax_id <> ''
      AND tax_id_encrypted IS NULL
),
unready AS (
    UPDATE affiliates
       SET is_tax_ready = false,
           updated_at = NOW()
     WHERE user_id IN (SELECT user_id FROM legacy_holders)
   RETURNING user_id
)
INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
SELECT u.user_id,
       'affiliate_legacy_tax_id_cleared',
       'affiliates',
       u.user_id,
       jsonb_build_object(
           'reason', 'Migration 143 dropped legacy plaintext tax_id column. is_tax_ready forced to false until affiliate re-saves encrypted tax id.',
           'migration', '143_affiliate_tax_id_drop_plaintext'
       )
FROM unready u;

-- 3. Drop the plaintext column. Index `idx_affiliates_tax_id` (if added by
-- an older migration) drops automatically with the column.
ALTER TABLE affiliates
    DROP COLUMN tax_id;

-- 4. Replace the (pointless) partial index from 092 — it indexed user_id
-- WHERE tax_id_encrypted IS NOT NULL, but user_id is already the PK so
-- the index added nothing. Replace with one that supports the rotation
-- workflow ("find rows still on key version N").
DROP INDEX IF EXISTS idx_affiliates_tax_id_encrypted_present;
CREATE INDEX IF NOT EXISTS idx_affiliates_tax_id_key_version
    ON affiliates (tax_id_key_version)
    WHERE tax_id_encrypted IS NOT NULL;

COMMIT;
