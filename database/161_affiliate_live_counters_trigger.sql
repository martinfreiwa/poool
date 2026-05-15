-- 161: Auto-Sync affiliate_live_counters über DB-Trigger
--
-- Statt jeden Status-Übergang in Service-Code zu duplizieren (Holdback-
-- Worker, Payout-Batch, Clawback, …), kapselt ein einziger AFTER
-- INSERT/UPDATE/DELETE-Trigger die Counter-Logik. Damit ist garantiert,
-- dass die Live-Tile-Zahlen IMMER mit der Source-of-Truth in
-- affiliate_commissions konsistent bleiben — egal welcher Code-Pfad sie
-- ändert (auch direkte SQL-Reparaturen durch Admins).
--
-- Status-Mapping (Bucket-Auswirkung):
--   provisionally_tracked / on_hold → pending_commission_cents
--   payable                          → payable_commission_cents
--   paid                             → paid_commission_cents
--   clawed_back                      → clawed_back_cents
--   frozen / clawback_pending        → pending (konservativ — bleibt sichtbar)
--   disqualified (falls vorhanden)   → nichts (commission "verfällt")
--
-- lifetime_commission_cents zählt jede Commission **bei Insert** einmal.
-- Bei Status-Wechseln verändert sich lifetime nicht. Bei DELETE wird
-- lifetime entsprechend reduziert.
--
-- Delta-basiert: trigger berechnet (new_bucket_delta - old_bucket_delta)
-- und addiert auf, anstelle eines kompletten Recomputes.

BEGIN;

CREATE OR REPLACE FUNCTION affiliate_status_bucket(status_text TEXT)
RETURNS TEXT
LANGUAGE plpgsql IMMUTABLE AS $$
BEGIN
    RETURN CASE status_text
        WHEN 'provisionally_tracked' THEN 'pending'
        WHEN 'on_hold'               THEN 'pending'
        WHEN 'frozen'                THEN 'pending'
        WHEN 'clawback_pending'      THEN 'pending'
        WHEN 'payable'               THEN 'payable'
        WHEN 'paid'                  THEN 'paid'
        WHEN 'clawed_back'           THEN 'clawed_back'
        ELSE NULL  -- 'disqualified' o.ä.: verfallen, keine Bucket-Wirkung
    END;
END $$;

CREATE OR REPLACE FUNCTION affiliate_commissions_counter_sync()
RETURNS TRIGGER
LANGUAGE plpgsql AS $$
DECLARE
    new_bucket TEXT;
    old_bucket TEXT;
    new_amt    BIGINT;
    old_amt    BIGINT;
    new_payout UUID;
    old_payout UUID;
BEGIN
    -- Werte aus NEW / OLD ableiten
    IF TG_OP = 'INSERT' THEN
        new_bucket := affiliate_status_bucket(NEW.status);
        new_amt    := NEW.provisional_amount_cents;
        new_payout := NEW.payout_user_id;
        old_bucket := NULL;
        old_amt    := 0;
        old_payout := NULL;
    ELSIF TG_OP = 'UPDATE' THEN
        new_bucket := affiliate_status_bucket(NEW.status);
        new_amt    := NEW.provisional_amount_cents;
        new_payout := NEW.payout_user_id;
        old_bucket := affiliate_status_bucket(OLD.status);
        old_amt    := OLD.provisional_amount_cents;
        old_payout := OLD.payout_user_id;
    ELSIF TG_OP = 'DELETE' THEN
        new_bucket := NULL;
        new_amt    := 0;
        new_payout := NULL;
        old_bucket := affiliate_status_bucket(OLD.status);
        old_amt    := OLD.provisional_amount_cents;
        old_payout := OLD.payout_user_id;
    END IF;

    -- Counter-Zeile sicherstellen
    IF new_payout IS NOT NULL THEN
        INSERT INTO affiliate_live_counters (payout_user_id) VALUES (new_payout)
            ON CONFLICT (payout_user_id) DO NOTHING;
    END IF;
    IF old_payout IS NOT NULL AND old_payout IS DISTINCT FROM new_payout THEN
        INSERT INTO affiliate_live_counters (payout_user_id) VALUES (old_payout)
            ON CONFLICT (payout_user_id) DO NOTHING;
    END IF;

    -- Falls payout_user_id wechselt: alten Bucket reduzieren, neuen erhöhen
    IF old_payout IS NOT NULL AND old_payout IS DISTINCT FROM new_payout THEN
        UPDATE affiliate_live_counters
        SET pending_commission_cents     = pending_commission_cents     - CASE WHEN old_bucket = 'pending'     THEN old_amt ELSE 0 END,
            payable_commission_cents     = payable_commission_cents     - CASE WHEN old_bucket = 'payable'     THEN old_amt ELSE 0 END,
            paid_commission_cents        = paid_commission_cents        - CASE WHEN old_bucket = 'paid'        THEN old_amt ELSE 0 END,
            clawed_back_cents            = clawed_back_cents            - CASE WHEN old_bucket = 'clawed_back' THEN old_amt ELSE 0 END,
            lifetime_commission_cents    = lifetime_commission_cents    - old_amt,
            last_updated                 = NOW()
        WHERE payout_user_id = old_payout;

        UPDATE affiliate_live_counters
        SET pending_commission_cents     = pending_commission_cents     + CASE WHEN new_bucket = 'pending'     THEN new_amt ELSE 0 END,
            payable_commission_cents     = payable_commission_cents     + CASE WHEN new_bucket = 'payable'     THEN new_amt ELSE 0 END,
            paid_commission_cents        = paid_commission_cents        + CASE WHEN new_bucket = 'paid'        THEN new_amt ELSE 0 END,
            clawed_back_cents            = clawed_back_cents            + CASE WHEN new_bucket = 'clawed_back' THEN new_amt ELSE 0 END,
            lifetime_commission_cents    = lifetime_commission_cents    + new_amt,
            last_updated                 = NOW()
        WHERE payout_user_id = new_payout;
    ELSE
        -- Standard-Fall: gleicher payout_user_id, eventuell anderer Bucket oder Betrag
        IF new_payout IS NOT NULL THEN
            UPDATE affiliate_live_counters
            SET pending_commission_cents = pending_commission_cents
                + CASE WHEN new_bucket = 'pending'     THEN new_amt ELSE 0 END
                - CASE WHEN old_bucket = 'pending'     THEN old_amt ELSE 0 END,
                payable_commission_cents = payable_commission_cents
                + CASE WHEN new_bucket = 'payable'     THEN new_amt ELSE 0 END
                - CASE WHEN old_bucket = 'payable'     THEN old_amt ELSE 0 END,
                paid_commission_cents    = paid_commission_cents
                + CASE WHEN new_bucket = 'paid'        THEN new_amt ELSE 0 END
                - CASE WHEN old_bucket = 'paid'        THEN old_amt ELSE 0 END,
                clawed_back_cents        = clawed_back_cents
                + CASE WHEN new_bucket = 'clawed_back' THEN new_amt ELSE 0 END
                - CASE WHEN old_bucket = 'clawed_back' THEN old_amt ELSE 0 END,
                lifetime_commission_cents = lifetime_commission_cents
                + (new_amt - old_amt),
                last_updated = NOW()
            WHERE payout_user_id = new_payout;
        ELSIF old_payout IS NOT NULL THEN
            -- DELETE-Fall ohne payout-Wechsel
            UPDATE affiliate_live_counters
            SET pending_commission_cents = pending_commission_cents     - CASE WHEN old_bucket = 'pending'     THEN old_amt ELSE 0 END,
                payable_commission_cents = payable_commission_cents     - CASE WHEN old_bucket = 'payable'     THEN old_amt ELSE 0 END,
                paid_commission_cents    = paid_commission_cents        - CASE WHEN old_bucket = 'paid'        THEN old_amt ELSE 0 END,
                clawed_back_cents        = clawed_back_cents            - CASE WHEN old_bucket = 'clawed_back' THEN old_amt ELSE 0 END,
                lifetime_commission_cents = lifetime_commission_cents - old_amt,
                last_updated = NOW()
            WHERE payout_user_id = old_payout;
        END IF;
    END IF;

    RETURN COALESCE(NEW, OLD);
END $$;

DROP TRIGGER IF EXISTS trg_affiliate_commissions_counter_sync ON affiliate_commissions;
CREATE TRIGGER trg_affiliate_commissions_counter_sync
    AFTER INSERT OR UPDATE OR DELETE ON affiliate_commissions
    FOR EACH ROW EXECUTE FUNCTION affiliate_commissions_counter_sync();

-- Reseed live_counters aus Source-of-Truth (idempotent, safe rerun)
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
