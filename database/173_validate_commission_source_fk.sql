-- 173_validate_commission_source_fk.sql
-- ──────────────────────────────────────────────────────────────────────────
-- F24 fix: attempt to validate `affiliate_commissions_source_order_id_fkey`,
-- but stay NOT VALID if orphans exist. Surfaces orphan count via a
-- table-row-count audit + writes a system audit_log entry so ops know.
--
-- Background: mig 152 added FK source_order_id → investments(id) as NOT VALID.
-- Bug 12 fix replaced it with → orders(id) NOT VALID. This migration tries
-- to promote the constraint to VALID once the operator cleans up any orphan
-- rows. Until then, the constraint enforces on new writes (NOT VALID still
-- catches new INSERTs/UPDATEs) but skips the existing-row scan.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

-- 1. Count orphans and report via audit_log. Don't raise an exception —
-- this migration must apply cleanly so deployments can progress.
DO $$
DECLARE
    orphan_count INT;
BEGIN
    SELECT COUNT(*) INTO orphan_count
      FROM affiliate_commissions ac
     WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.id = ac.source_order_id);

    IF orphan_count > 0 THEN
        RAISE NOTICE 'F24: % orphan affiliate_commissions reference non-existent orders. FK stays NOT VALID. Run cleanup, then re-attempt VALIDATE via the trailing operator script.', orphan_count;
        -- Write to audit_logs so operators see this on their dashboards.
        INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
        VALUES (
            NULL,
            'migration_warning',
            'affiliate_commissions',
            gen_random_uuid(),
            jsonb_build_object(
                'migration', '173_validate_commission_source_fk',
                'orphan_count', orphan_count,
                'note', 'Legacy orphan commissions reference non-existent orders. Investigate via the audit query.'
            )
        );
    ELSE
        -- Zero orphans → promote NOT VALID → VALID.
        RAISE NOTICE 'F24: zero orphans, promoting FK to VALID.';
        ALTER TABLE affiliate_commissions
            VALIDATE CONSTRAINT affiliate_commissions_source_order_id_fkey;
    END IF;
END $$;

COMMIT;

-- ── Operator cleanup script (run manually after investigation) ────────────
--
-- 1. Audit the orphans:
--   SELECT ac.id, ac.source_order_id, ac.payout_user_id, ac.status,
--          ac.provisional_amount_cents, ac.created_at
--     FROM affiliate_commissions ac
--    WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.id = ac.source_order_id)
--    ORDER BY ac.created_at;
--
-- 2. For each orphan, decide:
--    (a) DELETE if it was test/debug data and never paid.
--    (b) UPDATE source_order_id to the correct orders.id if the order
--        existed but its UUID got rewritten by a faulty migration.
--    (c) Create a synthetic orders row matching the historical conversion
--        if you need to preserve audit trail but the order itself is gone.
--
-- 3. After zero orphans:
--    ALTER TABLE affiliate_commissions
--       VALIDATE CONSTRAINT affiliate_commissions_source_order_id_fkey;
--
-- 4. Verify:
--    SELECT convalidated FROM pg_constraint
--     WHERE conname = 'affiliate_commissions_source_order_id_fkey';
--    -- should return `t`

SELECT conname, convalidated, pg_get_constraintdef(oid)
  FROM pg_constraint
 WHERE conrelid = 'affiliate_commissions'::regclass
   AND conname  = 'affiliate_commissions_source_order_id_fkey';
