-- 178_trigger_transition_table_alias_fix.sql
-- ──────────────────────────────────────────────────────────────────────────
-- Hotfix: mig 169 + 170 wrote `affiliate_commissions_counter_sync_*`
-- function bodies that reference `new_table` / `old_table`, but mig 163's
-- CREATE TRIGGER declares the transition tables as `new_rows` / `old_rows`.
--
-- Every commission INSERT/UPDATE/DELETE was therefore failing the trigger:
--   ERROR: relation "new_table" does not exist
-- The bug went unnoticed in dev seeds because the v3 seed pre-disables
-- triggers via direct UPDATE on affiliate_live_counters, but any real
-- check_and_track_affiliate_commission run inside a tx crashes.
--
-- Fix: rewrite the three functions with the correct alias names. Bodies
-- are otherwise identical to mig 170.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

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
    FROM new_rows nt
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
        FROM new_rows n JOIN old_rows o ON o.id = n.id
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
        FROM old_rows ot
        GROUP BY ot.payout_user_id, ot.currency
    ) agg
    WHERE lc.payout_user_id = agg.payout_user_id
      AND lc.currency       = agg.currency;
    RETURN NULL;
END $$;

COMMIT;
