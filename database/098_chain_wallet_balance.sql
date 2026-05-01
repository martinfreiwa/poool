-- Migration 098: Track settlement-wallet gas balance for monitoring/admin UI.
--
-- The gas-monitor worker polls eth_getBalance on the settlement wallet
-- every 5 min and stores the latest reading here. The admin UI (Blockchain
-- Treasury page) reads this row to display the current MATIC balance and
-- low/critical alert state.
--
-- Single-row pattern per address — UPSERT on every write.

CREATE TABLE IF NOT EXISTS chain_wallet_balance (
    address     TEXT PRIMARY KEY,
    balance_wei TEXT NOT NULL,           -- TEXT because wei can exceed i64
    checked_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE chain_wallet_balance IS
    'Latest gas-balance reading for hot wallets (e.g. settlement wallet).
     Updated by gas_monitor worker every 5 min. Read by admin UI for
     low-balance alerting.';
