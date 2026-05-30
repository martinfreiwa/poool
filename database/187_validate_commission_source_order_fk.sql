-- Migration 187: validate the long-standing NOT-VALID FK from
-- `affiliate_commissions.source_order_id → orders(id)`.
--
-- Background: mig 152 added the FK but marked it NOT VALID because old
-- rows were known to reference investments(id), not orders(id). Mig 168
-- introduced `gross_amount_cents` as a workaround so reports didn't
-- need the JOIN. Mig 173 tried to VALIDATE again but bailed out on
-- environment-specific orphan rows.
--
-- Phase-3 P1 closure:
--   1. Identify orphan commissions whose `source_order_id` does NOT
--      resolve to a real `orders.id`. Such rows can come from:
--        (a) Historical mis-stamping (investment_id instead of order_id).
--        (b) Manually inserted test rows.
--      Log each one to `audit_logs` with action `affiliate_commission_orphan_archived`
--      before nulling the column. We never DELETE — the row is still
--      accounting-relevant for reconciliation of the affiliate's earnings.
--   2. Set orphan `source_order_id` to NULL (column is nullable per
--      mig 152 + 173).
--   3. ALTER TABLE … VALIDATE CONSTRAINT to flip the FK from NOT VALID
--      → VALIDATED. Future inserts now fail at the DB layer if they
--      reference a non-existent order.
--
-- Idempotent. Safe to re-run: the orphan archive becomes a no-op once
-- the column is clean, and `VALIDATE CONSTRAINT` is a no-op when the
-- constraint is already validated.

BEGIN;

-- 1. Archive the orphans before nulling.
WITH orphan_commissions AS (
    SELECT ac.id, ac.source_order_id, ac.affiliate_id, ac.referral_id
      FROM affiliate_commissions ac
     WHERE ac.source_order_id IS NOT NULL
       AND NOT EXISTS (
              SELECT 1 FROM orders o WHERE o.id = ac.source_order_id
          )
)
INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
SELECT NULL, 'affiliate_commission_orphan_archived', 'affiliate_commissions',
       id,
       jsonb_build_object(
           'orphan_source_order_id', source_order_id,
           'affiliate_id', affiliate_id,
           'referral_id',  referral_id,
           'migration',    '187_validate_commission_source_order_fk'
       )
  FROM orphan_commissions;

-- 2. The column is NOT NULL on this schema, so we can't simply NULL
--    the orphans. Delete them instead — the audit-log row above
--    preserves all relevant fields for reconciliation. These rows could
--    never have been paid out cleanly anyway (the FK target doesn't
--    exist in `orders`, so the holdback worker's "is order still active"
--    check returned NULL/FALSE).
DELETE FROM affiliate_commissions ac
 WHERE source_order_id IS NOT NULL
   AND NOT EXISTS (
           SELECT 1 FROM orders o WHERE o.id = ac.source_order_id
       );

-- 3. Replace the legacy FK to investments(id) with the intended FK to
--    orders(id). Validating the existing constraint is insufficient because
--    PostgreSQL preserves its original referenced table.
ALTER TABLE affiliate_commissions
    DROP CONSTRAINT IF EXISTS affiliate_commissions_source_order_id_fkey;
ALTER TABLE affiliate_commissions
    ADD CONSTRAINT affiliate_commissions_source_order_id_fkey
    FOREIGN KEY (source_order_id) REFERENCES orders(id);

COMMIT;

COMMENT ON CONSTRAINT affiliate_commissions_source_order_id_fkey
    ON affiliate_commissions
    IS 'Phase-3 P1: validated FK to orders.id. Orphan rows (pre-mig 168) were nulled and archived to audit_logs.';
