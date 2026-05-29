-- 169_commission_round2_fixes.sql
-- ──────────────────────────────────────────────────────────────────────────
-- Second round of logic-bug fixes after deeper audit. Covers SQL-side of:
--   F2  — `lifetime_revenue_cents` never updated by triggers
--   F12 — terminated-team links never reactivated on resume
--   F16 — paused teams keep accepting attribution
-- Code-side bugs (F1, F3, F4, F5, F6, F7, F8, F10, F13, F14, F18, F20, F21)
-- are patched in the corresponding Rust files in this same change.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

-- ── F2: lifetime_revenue_cents must be maintained by the same trigger that
--    tracks commission deltas. The trigger logic from mig 163 only touched
--    the four bucket columns (pending/payable/paid/clawed_back) and
--    lifetime_commission_cents. Now we extend to lifetime_revenue_cents
--    using the NEW gross_amount_cents column (mig 168).
-- We CREATE OR REPLACE all three statement-level trigger functions so the
-- delta math stays consistent.
CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_ins()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO affiliate_live_counters AS lc
        (payout_user_id,
         lifetime_revenue_cents, lifetime_commission_cents,
         pending_commission_cents, payable_commission_cents,
         paid_commission_cents, clawed_back_cents, last_updated)
    SELECT nt.payout_user_id,
           COALESCE(SUM(nt.gross_amount_cents), 0),
           COALESCE(SUM(nt.provisional_amount_cents), 0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'pending'), 0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'payable'), 0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'paid'),    0),
           COALESCE(SUM(nt.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(nt.status) = 'clawed_back'), 0),
           NOW()
    FROM new_table nt
    GROUP BY nt.payout_user_id
    ON CONFLICT (payout_user_id) DO UPDATE SET
        lifetime_revenue_cents   = lc.lifetime_revenue_cents   + EXCLUDED.lifetime_revenue_cents,
        lifetime_commission_cents= lc.lifetime_commission_cents+ EXCLUDED.lifetime_commission_cents,
        pending_commission_cents = lc.pending_commission_cents + EXCLUDED.pending_commission_cents,
        payable_commission_cents = lc.payable_commission_cents + EXCLUDED.payable_commission_cents,
        paid_commission_cents    = lc.paid_commission_cents    + EXCLUDED.paid_commission_cents,
        clawed_back_cents        = lc.clawed_back_cents        + EXCLUDED.clawed_back_cents,
        last_updated             = NOW();
    RETURN NULL;
END $$;

-- UPDATE: net delta from OLD → NEW (lifetime_revenue is per-row, doesn't
-- change on status transitions, but does change if gross_amount_cents is
-- corrected — rare but possible).
CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync_upd()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE r RECORD;
BEGIN
    FOR r IN
        SELECT n.payout_user_id,
               -- revenue delta: gross can shift if a commission is re-priced
               COALESCE(n.gross_amount_cents, 0) - COALESCE(o.gross_amount_cents, 0) AS d_revenue,
               -- lifetime commission delta (provisional amount can change)
               COALESCE(n.provisional_amount_cents, 0) - COALESCE(o.provisional_amount_cents, 0) AS d_lifetime,
               -- bucket flips: subtract from old bucket, add to new bucket
               COALESCE(o.provisional_amount_cents, 0) AS old_amt,
               COALESCE(n.provisional_amount_cents, 0) AS new_amt,
               affiliate_status_bucket(o.status) AS old_bucket,
               affiliate_status_bucket(n.status) AS new_bucket
        FROM new_table n JOIN old_table o ON o.id = n.id
        WHERE o.payout_user_id = n.payout_user_id
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
        WHERE payout_user_id = r.payout_user_id;
    END LOOP;
    RETURN NULL;
END $$;

-- DELETE: subtract row's revenue + bucket from counters.
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
        SELECT ot.payout_user_id,
               COALESCE(SUM(ot.gross_amount_cents), 0) AS gross_sum,
               COALESCE(SUM(ot.provisional_amount_cents), 0) AS amt_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'pending'), 0) AS pending_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'payable'), 0) AS payable_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'paid'),    0) AS paid_sum,
               COALESCE(SUM(ot.provisional_amount_cents) FILTER (WHERE affiliate_status_bucket(ot.status) = 'clawed_back'), 0) AS clawed_sum
        FROM old_table ot
        GROUP BY ot.payout_user_id
    ) agg
    WHERE lc.payout_user_id = agg.payout_user_id;
    RETURN NULL;
END $$;

-- One-time backfill: lifetime_revenue_cents from existing commissions.
UPDATE affiliate_live_counters lc
   SET lifetime_revenue_cents = COALESCE(s.sum_cents, 0),
       last_updated = NOW()
  FROM (
       SELECT payout_user_id, SUM(gross_amount_cents) AS sum_cents
         FROM affiliate_commissions
        GROUP BY payout_user_id
  ) s
 WHERE s.payout_user_id = lc.payout_user_id;


-- ── F16: paused teams should stop attribution. Easiest: cascade-pause links
--    on team pause (mirroring the terminate cascade from mig 168).
CREATE OR REPLACE FUNCTION cascade_terminate_team_links()
RETURNS TRIGGER AS $$
BEGIN
    -- terminated → deactivate links (already in mig 168)
    IF NEW.status = 'terminated' AND OLD.status <> 'terminated' THEN
        UPDATE affiliate_links
           SET status = 'inactive',
               deactivated_at = NOW(),
               deactivated_reason = COALESCE(deactivated_reason,
                                             'team_terminated_' || NEW.id::text),
               updated_at = NOW()
         WHERE team_id = NEW.id
           AND status = 'active';
    END IF;

    -- paused → suspend links (reversible). Use status='suspended' which is
    -- documented but unused previously.
    IF NEW.status = 'paused' AND OLD.status <> 'paused' THEN
        UPDATE affiliate_links
           SET status = 'suspended',
               deactivated_at = NOW(),
               deactivated_reason = COALESCE(deactivated_reason,
                                             'team_paused_' || NEW.id::text),
               updated_at = NOW()
         WHERE team_id = NEW.id
           AND status = 'active';
    END IF;

    -- active (resume from paused) → reactivate ONLY links that were
    -- suspended by THIS team's pause. Don't touch admin-suspended ones.
    IF NEW.status = 'active' AND OLD.status = 'paused' THEN
        UPDATE affiliate_links
           SET status = 'active',
               deactivated_at = NULL,
               deactivated_reason = NULL,
               updated_at = NOW()
         WHERE team_id = NEW.id
           AND status = 'suspended'
           AND deactivated_reason = 'team_paused_' || NEW.id::text;
    END IF;

    -- F12: active (resume from terminated) → reactivate the deactivated
    -- links. Same pattern: only links that were deactivated BY this
    -- specific termination get restored.
    IF NEW.status = 'active' AND OLD.status = 'terminated' THEN
        UPDATE affiliate_links
           SET status = 'active',
               deactivated_at = NULL,
               deactivated_reason = NULL,
               updated_at = NOW()
         WHERE team_id = NEW.id
           AND status = 'inactive'
           AND deactivated_reason = 'team_terminated_' || NEW.id::text;
    END IF;

    RETURN NEW;
END $$ LANGUAGE plpgsql;

COMMENT ON FUNCTION cascade_terminate_team_links IS
    'Team-status changes cascade to its team_business affiliate_links: terminated → inactive, paused → suspended, active (from paused/terminated) → reactivate the links this team itself deactivated. Admin-suspended links survive untouched.';

COMMIT;

SELECT u.email, lc.lifetime_revenue_cents/100.0 AS revenue_eur,
       lc.lifetime_commission_cents/100.0 AS commission_eur
  FROM affiliate_live_counters lc
  JOIN users u ON u.id = lc.payout_user_id
 WHERE u.email = 'support@traffic-creator.com';
