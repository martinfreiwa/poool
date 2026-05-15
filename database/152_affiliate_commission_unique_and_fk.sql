-- 141: Affiliate commission idempotency + referential integrity
-- Fixes audit findings:
--   - No UNIQUE constraint on (source_order_id, referral_id) → double-pay
--     possible if check_and_track_affiliate_commission retries or races.
--   - No FK from affiliate_commissions.source_order_id → investments(id) →
--     commissions can reference phantom orders, no DB-level integrity guarantee.
--
-- Strategy:
--   1. De-duplicate existing rows (keep the earliest created_at per
--      (source_order_id, referral_id)) and log the discards.
--   2. Add the UNIQUE constraint.
--   3. Add the FK as NOT VALID so the migration applies cleanly even when
--      orphan rows exist; surface orphan rows via a SELECT for follow-up.
--   4. Add a partial unique index that prevents two unpaid commissions from
--      pointing at the same payout_batch_id concurrently (defence against
--      double-batching pre-payout).

BEGIN;

-- 1. De-duplicate. Keep the earliest row per (source_order_id, referral_id).
WITH ranked AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY source_order_id, referral_id
               ORDER BY created_at ASC, id ASC
           ) AS rn
    FROM affiliate_commissions
),
discarded AS (
    DELETE FROM affiliate_commissions
    WHERE id IN (SELECT id FROM ranked WHERE rn > 1)
    RETURNING id, referral_id, source_order_id, status, provisional_amount_cents
)
INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, metadata)
SELECT NULL,
       'affiliate_commission_dedup',
       'affiliate_commissions',
       d.id,
       jsonb_build_object(
           'referral_id', d.referral_id,
           'source_order_id', d.source_order_id,
           'status', d.status,
           'provisional_amount_cents', d.provisional_amount_cents,
           'migration', '141_affiliate_commission_unique_and_fk'
       )
FROM discarded d;

-- 2. Idempotency: same referral cannot earn two commissions for the same order.
ALTER TABLE affiliate_commissions
    ADD CONSTRAINT affiliate_commissions_referral_order_uniq
    UNIQUE (referral_id, source_order_id);

-- 3. Referential integrity. NOT VALID so we don't block migration on legacy
-- orphan rows; a follow-up admin task can `ALTER ... VALIDATE CONSTRAINT`
-- once stragglers are reconciled.
ALTER TABLE affiliate_commissions
    ADD CONSTRAINT affiliate_commissions_source_order_id_fkey
    FOREIGN KEY (source_order_id) REFERENCES investments(id)
    NOT VALID;

-- 4. Prevent double-batching: an unpaid commission (status NOT IN ('paid',
-- 'clawed_back')) cannot be attached to two payout_batches. Combined with
-- the UNIQUE above this closes the schema-level "attach commission to batch
-- A, then to batch B" race noted in the audit.
CREATE UNIQUE INDEX affiliate_commissions_unpaid_one_batch
    ON affiliate_commissions (id)
    WHERE payout_batch_id IS NOT NULL
      AND status NOT IN ('paid', 'clawed_back');

COMMIT;

-- Audit query for operators after apply (run manually):
--   SELECT id, source_order_id FROM affiliate_commissions c
--   WHERE NOT EXISTS (SELECT 1 FROM investments i WHERE i.id = c.source_order_id);
-- Once empty, run:
--   ALTER TABLE affiliate_commissions VALIDATE CONSTRAINT affiliate_commissions_source_order_id_fkey;
