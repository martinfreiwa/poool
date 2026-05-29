-- 170_affiliate_commission_currency.sql
-- ──────────────────────────────────────────────────────────────────────────
-- F26 fix: add explicit `currency` ISO-4217 code to monetary affiliate tables.
--
-- Until now `affiliate_commissions.*_cents` and `affiliate_live_counters.*_cents`
-- were single-currency by convention only — the column comment said "USD" but
-- there was no enforcement. The platform recently flipped to EUR (de-DE locale
-- on the frontend), and any future multi-currency rollout (e.g. a USD-listed
-- asset for US offerings) would silently mix EUR cents with USD cents in
-- the same aggregate.
--
-- Strategy:
--   1. Add `currency CHAR(3) NOT NULL DEFAULT 'EUR'` to:
--        affiliate_commissions
--        affiliate_live_counters    (one row per (payout_user_id, currency))
--        affiliate_daily_rollups    (one row per (rollup_date, link_id, currency))
--      Backfill existing rows with 'EUR' (current platform contract).
--   2. Add CHECK constraint: currency matches `^[A-Z]{3}$` so typos fail at insert.
--   3. Extend the unique-key on live_counters to include currency. Old
--      single-row-per-user becomes "one row per currency the dev has earned in".
--   4. Update statement-level triggers (from mig 169) to compute the bucket
--      per-currency by reading NEW.currency.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

-- ── 1. Add currency columns ──────────────────────────────────────────────
ALTER TABLE affiliate_commissions
    ADD COLUMN IF NOT EXISTS currency CHAR(3) NOT NULL DEFAULT 'EUR';
ALTER TABLE affiliate_commissions
    DROP CONSTRAINT IF EXISTS affiliate_commissions_currency_iso;
ALTER TABLE affiliate_commissions
    ADD CONSTRAINT affiliate_commissions_currency_iso
    CHECK (currency ~ '^[A-Z]{3}$');

ALTER TABLE affiliate_live_counters
    ADD COLUMN IF NOT EXISTS currency CHAR(3) NOT NULL DEFAULT 'EUR';
ALTER TABLE affiliate_live_counters
    DROP CONSTRAINT IF EXISTS affiliate_live_counters_currency_iso;
ALTER TABLE affiliate_live_counters
    ADD CONSTRAINT affiliate_live_counters_currency_iso
    CHECK (currency ~ '^[A-Z]{3}$');

ALTER TABLE affiliate_daily_rollups
    ADD COLUMN IF NOT EXISTS currency CHAR(3) NOT NULL DEFAULT 'EUR';
ALTER TABLE affiliate_daily_rollups
    DROP CONSTRAINT IF EXISTS affiliate_daily_rollups_currency_iso;
ALTER TABLE affiliate_daily_rollups
    ADD CONSTRAINT affiliate_daily_rollups_currency_iso
    CHECK (currency ~ '^[A-Z]{3}$');

-- ── 2. Extend live_counters PK so multi-currency works ───────────────────
-- Old PK was (payout_user_id). New is (payout_user_id, currency).
-- All existing rows backfill as EUR by the column default → unique pair stays.
ALTER TABLE affiliate_live_counters
    DROP CONSTRAINT IF EXISTS affiliate_live_counters_pkey;
ALTER TABLE affiliate_live_counters
    ADD CONSTRAINT affiliate_live_counters_pkey PRIMARY KEY (payout_user_id, currency);

-- ── 3. Extend daily_rollups PK ───────────────────────────────────────────
ALTER TABLE affiliate_daily_rollups
    DROP CONSTRAINT IF EXISTS affiliate_daily_rollups_pkey;
ALTER TABLE affiliate_daily_rollups
    ADD CONSTRAINT affiliate_daily_rollups_pkey PRIMARY KEY (rollup_date, link_id, currency);

-- ── 4. Patch the live-counter triggers to GROUP BY currency too ──────────
CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_ins()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO affiliate_live_counters AS lc
        (payout_user_id, currency,
         lifetime_revenue_cents, lifetime_commission_cents,
         pending_commission_cents, payable_commission_cents,
         paid_commission_cents, clawed_back_cents, last_updated)
    SELECT nt.payout_user_id, nt.currency,
           COALESCE(SUM(nt.gross_amount_cents), 0),
           COALESCE(SUM(nt.provisional_amount_cents), 0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'pending'), 0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'payable'), 0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'paid'),    0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'clawed_back'), 0),
           NOW()
    FROM new_table nt
    GROUP BY nt.payout_user_id, nt.currency
    ON CONFLICT (payout_user_id, currency) DO UPDATE SET
        lifetime_revenue_cents   = lc.lifetime_revenue_cents   + EXCLUDED.lifetime_revenue_cents,
        lifetime_commission_cents= lc.lifetime_commission_cents+ EXCLUDED.lifetime_commission_cents,
        pending_commission_cents = lc.pending_commission_cents + EXCLUDED.pending_commission_cents,
        payable_commission_cents = lc.payable_commission_cents + EXCLUDED.payable_commission_cents,
        paid_commission_cents    = lc.paid_commission_cents    + EXCLUDED.paid_commission_cents,
        clawed_back_cents        = lc.clawed_back_cents        + EXCLUDED.clawed_back_cents,
        last_updated             = NOW();
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_upd()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE r RECORD;
BEGIN
    FOR r IN
        SELECT n.payout_user_id, n.currency,
               COALESCE(n.gross_amount_cents, 0) - COALESCE(o.gross_amount_cents, 0) AS d_revenue,
               COALESCE(n.provisional_amount_cents, 0) - COALESCE(o.provisional_amount_cents, 0) AS d_lifetime,
               COALESCE(o.provisional_amount_cents, 0) AS old_amt,
               COALESCE(n.provisional_amount_cents, 0) AS new_amt,
               affiliate_status_bucket(o.status) AS old_bucket,
               affiliate_status_bucket(n.status) AS new_bucket
        FROM new_table n JOIN old_table o ON o.id = n.id
        WHERE o.payout_user_id = n.payout_user_id
          AND o.currency       = n.currency
    LOOP
        UPDATE affiliate_live_counters SET
            lifetime_revenue_cents    = lifetime_revenue_cents + r.d_revenue,
            lifetime_commission_cents = lifetime_commission_cents + r.d_lifetime,
            pending_commission_cents  = pending_commission_cents
                - CASE WHEN r.old_bucket = 'pending' THEN r.old_amt ELSE 0 END
                + CASE WHEN r.new_bucket = 'pending' THEN r.new_amt ELSE 0 END,
            payable_commission_cents  = payable_commission_cents
                - CASE WHEN r.old_bucket = 'payable' THEN r.old_amt ELSE 0 END
                + CASE WHEN r.new_bucket = 'payable' THEN r.new_amt ELSE 0 END,
            paid_commission_cents     = paid_commission_cents
                - CASE WHEN r.old_bucket = 'paid'    THEN r.old_amt ELSE 0 END
                + CASE WHEN r.new_bucket = 'paid'    THEN r.new_amt ELSE 0 END,
            clawed_back_cents         = clawed_back_cents
                - CASE WHEN r.old_bucket = 'clawed_back' THEN r.old_amt ELSE 0 END
                + CASE WHEN r.new_bucket = 'clawed_back' THEN r.new_amt ELSE 0 END,
            last_updated = NOW()
        WHERE payout_user_id = r.payout_user_id
          AND currency       = r.currency;
    END LOOP;
    RETURN NULL;
END $$;

CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_del()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    UPDATE affiliate_live_counters lc SET
        lifetime_revenue_cents    = lc.lifetime_revenue_cents    - agg.gross_sum,
        lifetime_commission_cents = lc.lifetime_commission_cents - agg.amt_sum,
        pending_commission_cents  = lc.pending_commission_cents  - agg.pending_sum,
        payable_commission_cents  = lc.payable_commission_cents  - agg.payable_sum,
        paid_commission_cents     = lc.paid_commission_cents     - agg.paid_sum,
        clawed_back_cents         = lc.clawed_back_cents         - agg.clawed_sum,
        last_updated = NOW()
    FROM (
        SELECT ot.payout_user_id, ot.currency,
               COALESCE(SUM(ot.gross_amount_cents), 0) AS gross_sum,
               COALESCE(SUM(ot.provisional_amount_cents), 0) AS amt_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'pending'), 0) AS pending_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'payable'), 0) AS payable_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'paid'),    0) AS paid_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'clawed_back'), 0) AS clawed_sum
        FROM old_table ot
        GROUP BY ot.payout_user_id, ot.currency
    ) agg
    WHERE lc.payout_user_id = agg.payout_user_id
      AND lc.currency       = agg.currency;
    RETURN NULL;
END $$;

COMMENT ON COLUMN affiliate_commissions.currency IS
    'ISO-4217 currency code (3 uppercase letters). Stamped at INSERT from orders.currency. Live counters aggregate per currency.';
COMMENT ON COLUMN affiliate_live_counters.currency IS
    'ISO-4217 currency code. One counter row per (payout_user_id, currency) pair.';
COMMENT ON COLUMN affiliate_daily_rollups.currency IS
    'ISO-4217 currency code. One rollup row per (rollup_date, link_id, currency) so multi-currency reports aggregate correctly.';

COMMIT;

SELECT 'commissions' AS tbl, currency, COUNT(*) FROM affiliate_commissions GROUP BY currency
UNION ALL
SELECT 'live_counters', currency, COUNT(*) FROM affiliate_live_counters GROUP BY currency
UNION ALL
SELECT 'daily_rollups', currency, COUNT(*) FROM affiliate_daily_rollups GROUP BY currency
ORDER BY tbl, currency;
