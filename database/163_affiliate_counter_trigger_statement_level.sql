-- 163: Replace row-level affiliate_commissions counter trigger with
-- STATEMENT-level transition tables to fix bulk-insert lock contention.
--
-- Audit P0 #12: AFTER INSERT … FOR EACH ROW serializes UPDATEs on the
-- single `affiliate_live_counters` row per payout_user_id. Bulk inserts
-- (admin backfill, viral creator, batch reward computation) lock the
-- same row N times in a row.
--
-- Fix: AFTER … FOR EACH STATEMENT triggers that aggregate the transition
-- tables (REFERENCING NEW TABLE AS ins / OLD TABLE AS del) and emit a
-- single UPSERT per affected payout_user_id per statement.
--
-- Math: pending/payable/paid/clawed_back buckets are recomputed as deltas
-- by status-bucket mapping; lifetime is delta-per-row INSERT-or-DELETE.
-- Same as 161 but vectorized.

BEGIN;

-- INSERT statement trigger ----------------------------------------------------
CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_ins()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    -- Insert counter rows that don't yet exist (idempotent).
    INSERT INTO affiliate_live_counters (payout_user_id)
    SELECT DISTINCT payout_user_id FROM new_rows
    ON CONFLICT (payout_user_id) DO NOTHING;

    -- Aggregate per payout_user_id and apply deltas in one UPDATE per user.
    UPDATE affiliate_live_counters lc
       SET pending_commission_cents     = lc.pending_commission_cents     + agg.pending_delta,
           payable_commission_cents     = lc.payable_commission_cents     + agg.payable_delta,
           paid_commission_cents        = lc.paid_commission_cents        + agg.paid_delta,
           clawed_back_cents            = lc.clawed_back_cents            + agg.clawed_delta,
           lifetime_commission_cents    = lc.lifetime_commission_cents    + agg.lifetime_delta,
           last_updated                 = NOW()
      FROM (
          SELECT
              payout_user_id,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'pending'     THEN provisional_amount_cents ELSE 0 END)::bigint AS pending_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'payable'     THEN provisional_amount_cents ELSE 0 END)::bigint AS payable_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'paid'        THEN provisional_amount_cents ELSE 0 END)::bigint AS paid_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'clawed_back' THEN provisional_amount_cents ELSE 0 END)::bigint AS clawed_delta,
              SUM(provisional_amount_cents)::bigint AS lifetime_delta
          FROM new_rows
          GROUP BY payout_user_id
      ) AS agg
     WHERE lc.payout_user_id = agg.payout_user_id;

    RETURN NULL;
END $$;

-- UPDATE statement trigger ---------------------------------------------------
-- Handles status transitions AND payout_user_id changes (rare). Compares
-- transition tables row-by-row via a JOIN on the primary key.
CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_upd()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    -- Ensure counters exist for every payout_user_id mentioned in NEW (might be
    -- a brand-new payout_user_id if we rotated ownership).
    INSERT INTO affiliate_live_counters (payout_user_id)
    SELECT DISTINCT payout_user_id FROM new_rows
    ON CONFLICT (payout_user_id) DO NOTHING;
    INSERT INTO affiliate_live_counters (payout_user_id)
    SELECT DISTINCT payout_user_id FROM old_rows
    ON CONFLICT (payout_user_id) DO NOTHING;

    -- Subtract OLD contributions from the OLD payout_user_id, then add NEW
    -- contributions to the NEW payout_user_id. Done as two passes so the
    -- math survives same-user updates (where the deltas cancel correctly).
    UPDATE affiliate_live_counters lc
       SET pending_commission_cents     = lc.pending_commission_cents     - agg.pending_delta,
           payable_commission_cents     = lc.payable_commission_cents     - agg.payable_delta,
           paid_commission_cents        = lc.paid_commission_cents        - agg.paid_delta,
           clawed_back_cents            = lc.clawed_back_cents            - agg.clawed_delta,
           lifetime_commission_cents    = lc.lifetime_commission_cents    - agg.lifetime_delta,
           last_updated                 = NOW()
      FROM (
          SELECT
              payout_user_id,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'pending'     THEN provisional_amount_cents ELSE 0 END)::bigint AS pending_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'payable'     THEN provisional_amount_cents ELSE 0 END)::bigint AS payable_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'paid'        THEN provisional_amount_cents ELSE 0 END)::bigint AS paid_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'clawed_back' THEN provisional_amount_cents ELSE 0 END)::bigint AS clawed_delta,
              SUM(provisional_amount_cents)::bigint AS lifetime_delta
          FROM old_rows
          GROUP BY payout_user_id
      ) AS agg
     WHERE lc.payout_user_id = agg.payout_user_id;

    UPDATE affiliate_live_counters lc
       SET pending_commission_cents     = lc.pending_commission_cents     + agg.pending_delta,
           payable_commission_cents     = lc.payable_commission_cents     + agg.payable_delta,
           paid_commission_cents        = lc.paid_commission_cents        + agg.paid_delta,
           clawed_back_cents            = lc.clawed_back_cents            + agg.clawed_delta,
           lifetime_commission_cents    = lc.lifetime_commission_cents    + agg.lifetime_delta,
           last_updated                 = NOW()
      FROM (
          SELECT
              payout_user_id,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'pending'     THEN provisional_amount_cents ELSE 0 END)::bigint AS pending_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'payable'     THEN provisional_amount_cents ELSE 0 END)::bigint AS payable_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'paid'        THEN provisional_amount_cents ELSE 0 END)::bigint AS paid_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'clawed_back' THEN provisional_amount_cents ELSE 0 END)::bigint AS clawed_delta,
              SUM(provisional_amount_cents)::bigint AS lifetime_delta
          FROM new_rows
          GROUP BY payout_user_id
      ) AS agg
     WHERE lc.payout_user_id = agg.payout_user_id;

    RETURN NULL;
END $$;

-- DELETE statement trigger ---------------------------------------------------
CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_del()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    UPDATE affiliate_live_counters lc
       SET pending_commission_cents     = lc.pending_commission_cents     - agg.pending_delta,
           payable_commission_cents     = lc.payable_commission_cents     - agg.payable_delta,
           paid_commission_cents        = lc.paid_commission_cents        - agg.paid_delta,
           clawed_back_cents            = lc.clawed_back_cents            - agg.clawed_delta,
           lifetime_commission_cents    = lc.lifetime_commission_cents    - agg.lifetime_delta,
           last_updated                 = NOW()
      FROM (
          SELECT
              payout_user_id,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'pending'     THEN provisional_amount_cents ELSE 0 END)::bigint AS pending_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'payable'     THEN provisional_amount_cents ELSE 0 END)::bigint AS payable_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'paid'        THEN provisional_amount_cents ELSE 0 END)::bigint AS paid_delta,
              SUM(CASE WHEN affiliate_status_bucket(status) = 'clawed_back' THEN provisional_amount_cents ELSE 0 END)::bigint AS clawed_delta,
              SUM(provisional_amount_cents)::bigint AS lifetime_delta
          FROM old_rows
          GROUP BY payout_user_id
      ) AS agg
     WHERE lc.payout_user_id = agg.payout_user_id;

    RETURN NULL;
END $$;

-- Swap row-level trigger for statement-level
DROP TRIGGER IF EXISTS trg_affiliate_commissions_counter_sync ON affiliate_commissions;
DROP FUNCTION IF EXISTS affiliate_commissions_counter_sync();

CREATE TRIGGER trg_affiliate_commissions_counter_sync_ins
    AFTER INSERT ON affiliate_commissions
    REFERENCING NEW TABLE AS new_rows
    FOR EACH STATEMENT EXECUTE FUNCTION affiliate_commissions_counter_sync_ins();

CREATE TRIGGER trg_affiliate_commissions_counter_sync_upd
    AFTER UPDATE ON affiliate_commissions
    REFERENCING OLD TABLE AS old_rows NEW TABLE AS new_rows
    FOR EACH STATEMENT EXECUTE FUNCTION affiliate_commissions_counter_sync_upd();

CREATE TRIGGER trg_affiliate_commissions_counter_sync_del
    AFTER DELETE ON affiliate_commissions
    REFERENCING OLD TABLE AS old_rows
    FOR EACH STATEMENT EXECUTE FUNCTION affiliate_commissions_counter_sync_del();

-- Reseed live_counters as a safety net (idempotent).
TRUNCATE affiliate_live_counters;
INSERT INTO affiliate_live_counters
    (payout_user_id,
     lifetime_commission_cents,
     pending_commission_cents,
     payable_commission_cents,
     paid_commission_cents,
     clawed_back_cents)
SELECT
    payout_user_id,
    COALESCE(SUM(provisional_amount_cents), 0),
    COALESCE(SUM(provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(status) = 'pending'),     0),
    COALESCE(SUM(provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(status) = 'payable'),     0),
    COALESCE(SUM(provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(status) = 'paid'),        0),
    COALESCE(SUM(provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(status) = 'clawed_back'), 0)
FROM affiliate_commissions
GROUP BY payout_user_id;

COMMIT;
