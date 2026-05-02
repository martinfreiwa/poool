-- ═══════════════════════════════════════════════════════════════
-- 101_sovereign_wallet_binding.sql
-- Sovereign-wallet model: user-supplied + signature-proved Ethereum
-- addresses replace the fake-hash placeholder.
--
-- Three changes:
--
-- 1. New table `wallet_binding_challenges` — single-use SIWE nonces. The
--    backend issues a nonce, the user signs it in their wallet, the
--    backend verifies the EIP-191 signature, then marks the challenge
--    consumed and stamps `users.chain_wallet_address`.
--
-- 2. Clear ALL existing `chain_wallet_address` values. The previous
--    addresses were derived via DefaultHasher from the user UUID and
--    have NO private key. Tokens already sent to those addresses are
--    unrecoverable (this is OK because we're still on testnet — verify
--    before running this in production).
--
-- 3. Add a UNIQUE partial index on `chain_wallet_address` so the same
--    on-chain address can never bind to two POOOL accounts.
--
-- 🔴 PRODUCTION NOTE: clearing chain_wallet_address forces every
-- KYC-approved user to re-bind a real wallet via the SIWE endpoint
-- before they can settle on-chain. Communicate this in advance.
-- ═══════════════════════════════════════════════════════════════

-- 1. SIWE challenge store.
CREATE TABLE IF NOT EXISTS wallet_binding_challenges (
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    address      VARCHAR(42) NOT NULL,
    nonce        VARCHAR(64) NOT NULL,           -- 32-byte random, hex-encoded
    expires_at   TIMESTAMPTZ NOT NULL,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, address)
);

CREATE INDEX IF NOT EXISTS idx_wallet_binding_active
    ON wallet_binding_challenges(user_id)
    WHERE consumed_at IS NULL;

-- 2. Wipe legacy fake addresses. Keeping the column itself in place.
-- Also reset whitelist timestamps so the worker re-whitelists when a
-- real wallet is bound. CHANGE TO COMMENT IF ROLLING TO PROD WITH USERS.
UPDATE users
   SET chain_wallet_address = NULL,
       chain_whitelisted_at = NULL
 WHERE chain_wallet_address IS NOT NULL;

-- 3. One Ethereum address can map to at most one POOOL account.
CREATE UNIQUE INDEX IF NOT EXISTS uq_users_chain_wallet_address
    ON users(LOWER(chain_wallet_address))
    WHERE chain_wallet_address IS NOT NULL;
