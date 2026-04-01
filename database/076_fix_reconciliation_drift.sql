-- ============================================================
-- Fix Reconciliation Drift: Sync Ledgers with Seed Balances
-- ============================================================

-- Fix 1: Wallets without matching ledger transactions (e.g. from seed data)
-- Insert an 'admin_credit' to bridge the gap so the ledger matches the wallet balance exactly.
INSERT INTO wallet_transactions (wallet_id, type, status, amount_cents, currency, description)
SELECT 
    w.id, 
    'admin_credit', 
    'completed', 
    w.balance_cents - COALESCE((SELECT SUM(amount_cents) FROM wallet_transactions WHERE wallet_id = w.id AND status = 'completed'), 0), 
    w.currency, 
    'System balance reconciliation (seed fix)'
FROM wallets w
WHERE w.balance_cents != COALESCE((SELECT SUM(amount_cents) FROM wallet_transactions WHERE wallet_id = w.id AND status = 'completed'), 0);

-- Fix 2: Assets where sold tokens exceed investment records (e.g. from seed data)
-- Assign the missing tokens to the admin/seed user to balance the token supply.
INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status)
SELECT 
    (SELECT id FROM users WHERE email = 'jonas.freiwald@poool.app' LIMIT 1),
    a.id,
    a.tokens_total - a.tokens_available - COALESCE((SELECT SUM(tokens_owned)::int FROM investments WHERE asset_id = a.id AND status != 'exited'), 0),
    (a.tokens_total - a.tokens_available - COALESCE((SELECT SUM(tokens_owned)::int FROM investments WHERE asset_id = a.id AND status != 'exited'), 0)) * a.token_price_cents,
    (a.tokens_total - a.tokens_available - COALESCE((SELECT SUM(tokens_owned)::int FROM investments WHERE asset_id = a.id AND status != 'exited'), 0)) * a.token_price_cents,
    'active'
FROM assets a
WHERE a.funding_status IN ('funding_open', 'funding_in_progress', 'funded')
  AND a.tokens_total - a.tokens_available - COALESCE((SELECT SUM(tokens_owned)::int FROM investments WHERE asset_id = a.id AND status != 'exited'), 0) > 0
  AND (SELECT id FROM users WHERE email = 'jonas.freiwald@poool.app' LIMIT 1) IS NOT NULL;

