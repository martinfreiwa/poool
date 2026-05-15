-- 165: Hot-path indices for team_customers / team_products queries.
--
-- Audit P1: team_customers ORDER BY ar.created_at DESC LIMIT N runs as
-- index-scan-then-sort on the existing `idx_referrals_link` (no created_at
-- column). team_products filters `ac.created_at::date BETWEEN ...` which
-- is non-sargable. Both need a covering composite index.
--
-- These are pure additions — no data rewrite, just CREATE INDEX
-- CONCURRENTLY-friendly (we use plain CREATE here because the local DB
-- is small; production deploys should swap to CONCURRENTLY).

BEGIN;

-- Reports: ORDER BY created_at DESC for a team — covering index on
-- (link_id, created_at DESC) lets the planner skip the sort step.
CREATE INDEX IF NOT EXISTS idx_referrals_link_created_desc
    ON affiliate_referrals (link_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_commissions_link_created_desc
    ON affiliate_commissions (link_id, created_at DESC);

COMMIT;
