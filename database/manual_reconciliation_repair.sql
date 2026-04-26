-- ============================================================
-- Manual Reconciliation Repair
-- ============================================================
-- Purpose:
--   Repair local/staging reconciliation drift caused by seed/test data that
--   directly mutates wallet balances or asset counters.
--
-- Safety:
--   1. Review the DRY RUN result sets first.
--   2. Run the repair section only after confirming this is not production
--      unless finance/compliance has approved the source-of-truth choices.
--
-- Source of truth used by this script:
--   - Cash wallet balances are treated as authoritative for local/staging
--     repair; balancing admin_credit/admin_debit ledger rows are inserted.
--   - Investment ownership is treated as authoritative when it can be
--     reconciled by asset availability alone.
--   - Sold-token counters are treated as authoritative when investments are
--     missing; missing tokens are assigned to the seed/admin user.
--   - Impossible token states, where investments exceed total token supply,
--     are reported and left untouched.
--
-- Usage:
--   psql -d poool -f database/manual_reconciliation_repair.sql
-- ============================================================

BEGIN;

CREATE TEMP TABLE recon_cash_drift AS
SELECT
    w.id AS wallet_id,
    w.user_id,
    u.email,
    w.wallet_type,
    w.currency,
    w.balance_cents,
    COALESCE(SUM(wt.amount_cents) FILTER (WHERE wt.status = 'completed'), 0)::bigint AS ledger_cents,
    w.balance_cents - COALESCE(SUM(wt.amount_cents) FILTER (WHERE wt.status = 'completed'), 0)::bigint AS delta_cents
FROM wallets w
JOIN users u ON u.id = w.user_id
LEFT JOIN wallet_transactions wt ON wt.wallet_id = w.id
WHERE w.wallet_type = 'cash'
  AND w.currency = 'USD'
GROUP BY w.id, w.user_id, u.email, w.wallet_type, w.currency, w.balance_cents
HAVING w.balance_cents != COALESCE(SUM(wt.amount_cents) FILTER (WHERE wt.status = 'completed'), 0)::bigint;

CREATE TEMP TABLE recon_token_drift AS
SELECT
    a.id AS asset_id,
    a.title,
    a.tokens_total,
    a.tokens_available,
    a.token_price_cents,
    a.tokens_total - a.tokens_available AS sold_tokens,
    COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int AS investment_tokens,
    (a.tokens_total - a.tokens_available)
        - COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int AS delta_tokens
FROM assets a
LEFT JOIN investments i ON i.asset_id = a.id
WHERE a.funding_status IN ('funding_open', 'funding_in_progress', 'funded')
GROUP BY a.id, a.title, a.tokens_total, a.tokens_available, a.token_price_cents
HAVING (a.tokens_total - a.tokens_available)
    != COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int;

-- DRY RUN: review these rows before trusting the repair.
SELECT 'cash_drift' AS section, *
FROM recon_cash_drift
ORDER BY ABS(delta_cents) DESC, email;

SELECT
    'token_drift' AS section,
    *,
    CASE
        WHEN delta_tokens > 0 THEN 'insert_missing_seed_investment'
        WHEN investment_tokens BETWEEN 0 AND tokens_total THEN 'recompute_tokens_available'
        ELSE 'manual_review_investments_exceed_supply'
    END AS repair_action
FROM recon_token_drift
ORDER BY title;

-- Repair cash ledgers to match wallet balances.
INSERT INTO wallet_transactions (
    wallet_id,
    type,
    status,
    amount_cents,
    currency,
    description,
    external_ref_id,
    completed_at
)
SELECT
    wallet_id,
    CASE WHEN delta_cents >= 0 THEN 'admin_credit' ELSE 'admin_debit' END,
    'completed',
    delta_cents,
    currency,
    'Manual reconciliation repair: align ledger with wallet balance',
    'manual-reconciliation:' || wallet_id::text || ':' || txid_current()::text,
    NOW()
FROM recon_cash_drift
WHERE delta_cents != 0;

-- Repair missing investment rows when the asset counter says more tokens are sold
-- than investment ownership records show.
INSERT INTO investments (
    user_id,
    asset_id,
    tokens_owned,
    purchase_value_cents,
    current_value_cents,
    status
)
SELECT
    (SELECT id FROM users WHERE email = 'jonas.freiwald@poool.app' LIMIT 1),
    asset_id,
    delta_tokens,
    delta_tokens::bigint * token_price_cents,
    delta_tokens::bigint * token_price_cents,
    'active'
FROM recon_token_drift
WHERE delta_tokens > 0
  AND (SELECT id FROM users WHERE email = 'jonas.freiwald@poool.app' LIMIT 1) IS NOT NULL
ON CONFLICT (user_id, asset_id) DO UPDATE
SET tokens_owned = investments.tokens_owned + EXCLUDED.tokens_owned,
    purchase_value_cents = investments.purchase_value_cents + EXCLUDED.purchase_value_cents,
    current_value_cents = investments.current_value_cents + EXCLUDED.current_value_cents,
    status = CASE
        WHEN investments.status = 'exited' THEN EXCLUDED.status
        ELSE investments.status
    END,
    updated_at = NOW();

-- Repair asset availability when investment ownership can be represented within
-- the existing total token supply.
UPDATE assets a
SET tokens_available = a.tokens_total - d.investment_tokens,
    funding_status = CASE
        WHEN d.investment_tokens = a.tokens_total THEN 'funded'
        WHEN a.funding_status = 'funded' THEN 'funding_in_progress'
        ELSE a.funding_status
    END,
    updated_at = NOW()
FROM recon_token_drift d
WHERE d.asset_id = a.id
  AND d.delta_tokens < 0
  AND d.investment_tokens BETWEEN 0 AND a.tokens_total;

-- Final verification result sets.
SELECT
    'remaining_cash_drift' AS section,
    COUNT(*) AS rows,
    COALESCE(SUM(delta_cents), 0)::bigint AS total_delta_cents
FROM (
    SELECT
        w.id,
        w.balance_cents - COALESCE(SUM(wt.amount_cents) FILTER (WHERE wt.status = 'completed'), 0)::bigint AS delta_cents
    FROM wallets w
    LEFT JOIN wallet_transactions wt ON wt.wallet_id = w.id
    WHERE w.wallet_type = 'cash'
      AND w.currency = 'USD'
    GROUP BY w.id, w.balance_cents
    HAVING w.balance_cents != COALESCE(SUM(wt.amount_cents) FILTER (WHERE wt.status = 'completed'), 0)::bigint
) drift;

SELECT
    'remaining_token_drift' AS section,
    a.id,
    a.title,
    a.tokens_total,
    a.tokens_available,
    a.tokens_total - a.tokens_available AS sold_tokens,
    COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int AS investment_tokens,
    (a.tokens_total - a.tokens_available)
        - COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int AS delta_tokens,
    CASE
        WHEN COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int > a.tokens_total
            THEN 'manual_review_investments_exceed_supply'
        ELSE 'unexpected_remaining_drift'
    END AS reason
FROM assets a
LEFT JOIN investments i ON i.asset_id = a.id
WHERE a.funding_status IN ('funding_open', 'funding_in_progress', 'funded')
GROUP BY a.id, a.title, a.tokens_total, a.tokens_available
HAVING (a.tokens_total - a.tokens_available)
    != COALESCE(SUM(i.tokens_owned) FILTER (WHERE i.status != 'exited'), 0)::int
ORDER BY a.title;

COMMIT;
