-- ═══════════════════════════════════════════════════════════════
-- 099_chain_nonce_state.sql
-- DB-backed nonce manager for the on-chain settlement signer.
--
-- Why: previously the worker fetched eth_getTransactionCount("latest")
-- per send. Under concurrency (settlement worker + reconciler retry, or
-- two replicas) two requests would observe the same nonce and one TX
-- would be silently dropped from the mempool — leading to lost
-- settlements that would never confirm.
--
-- This table holds one row per signer address. Reads use SELECT … FOR
-- UPDATE so concurrent callers serialize on it. The worker initializes
-- next_nonce from the chain on first use and increments locally per
-- broadcast.
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS chain_nonce_state (
    signer_address  VARCHAR(42) PRIMARY KEY,   -- 0x-prefixed lowercase
    next_nonce      BIGINT NOT NULL CHECK (next_nonce >= 0),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
