-- Migration 055: Create reconciliation_reports table
-- Purpose: Store daily balance reconciliation check results
-- Ref: Masterplan §4.2 Mig055

CREATE TABLE reconciliation_reports (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date             DATE NOT NULL UNIQUE,

    -- Cash balance
    total_wallet_cents      BIGINT NOT NULL,     -- SUM(wallets.balance_cents + held_balance_cents)
    total_deposits_cents    BIGINT NOT NULL,      -- SUM(deposits)
    total_withdrawals_cents BIGINT NOT NULL,      -- SUM(withdrawals)
    total_purchases_cents   BIGINT NOT NULL,      -- SUM(primary market purchases)
    cash_delta_cents        BIGINT NOT NULL,      -- Expected - Actual (must be 0!)

    -- Fee balance
    total_fees_earned_cents BIGINT NOT NULL,
    fee_wallet_cents        BIGINT NOT NULL,
    fee_delta_cents         BIGINT NOT NULL,      -- Expected - Actual (must be 0!)

    -- Token integrity
    token_mismatches        INTEGER NOT NULL DEFAULT 0,  -- Number of assets with delta != 0
    token_details           JSONB,                       -- Details per asset on mismatch

    -- Status
    status                  VARCHAR(15) NOT NULL DEFAULT 'pass'
                            CHECK (status IN ('pass', 'warning', 'fail')),
    notes                   TEXT,

    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
