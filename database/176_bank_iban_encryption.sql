-- 176_bank_iban_encryption.sql
-- ──────────────────────────────────────────────────────────────────────────
-- B-P0-1 fix: encrypt `developer_teams.bank_iban` at rest via AES-256-GCM
-- (handled in Rust, same pattern as `affiliates.tax_id_encrypted` —
-- backend/src/rewards/service.rs::encrypt_tax_id_payload).
--
-- Strategy:
--   1. Add encrypted column + cached last4 + key_version.
--   2. Backfill: encrypt existing plaintext rows. Application reads
--      `BANK_IBAN_ENCRYPTION_KEY` env, encrypts each row, writes back.
--      (Cannot encrypt server-side via SQL — AEAD nonce + key in Vault.)
--   3. Drop CHECK constraint on plaintext length (encrypted blobs longer).
--   4. Drop plaintext `bank_iban` column AFTER backfill verification.
--      → split into mig 177 so operators control timing.
--
-- This migration is reversible: dropping bank_iban_encrypted is safe
-- because plaintext column still exists during the transition window.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

ALTER TABLE developer_teams
    ADD COLUMN IF NOT EXISTS bank_iban_encrypted    TEXT,
    ADD COLUMN IF NOT EXISTS bank_iban_last4        VARCHAR(4),
    ADD COLUMN IF NOT EXISTS bank_iban_key_version  SMALLINT NOT NULL DEFAULT 1;

COMMENT ON COLUMN developer_teams.bank_iban_encrypted IS
    'AES-256-GCM ciphertext envelope: "iban:v1:<base64-nonce>:<base64-ciphertext>". Key from BANK_IBAN_ENCRYPTION_KEY env (32-byte). Encrypted/decrypted in Rust — never in SQL.';
COMMENT ON COLUMN developer_teams.bank_iban_last4 IS
    'Last 4 chars of compacted IBAN. Safe to expose (used in masked display).';
COMMENT ON COLUMN developer_teams.bank_iban_key_version IS
    'Encryption key version. Bump when rotating BANK_IBAN_ENCRYPTION_KEY and re-encrypting rows.';

-- Helpful index for "is bank set?" checks without touching the cipher blob.
CREATE INDEX IF NOT EXISTS idx_developer_teams_bank_iban_set
    ON developer_teams (id) WHERE bank_iban_encrypted IS NOT NULL;

COMMIT;

\echo '── plaintext rows still present (need backfill via Rust admin task) ──'
SELECT COUNT(*) AS plaintext_count,
       COUNT(*) FILTER (WHERE bank_iban_encrypted IS NOT NULL) AS already_encrypted
  FROM developer_teams
 WHERE bank_iban IS NOT NULL;
