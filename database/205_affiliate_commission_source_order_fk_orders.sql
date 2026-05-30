-- 205: Ensure affiliate commission source_order_id points at orders(id).
--
-- Migration 152 originally added a NOT VALID FK to investments(id). Checkout
-- commission creation has used orders.id as source_order_id since the wallet
-- checkout path was introduced, and later validation scripts document the
-- intended orders(id) relationship. Some local databases can still carry the
-- old validated investments FK, which silently breaks referral commissions.

BEGIN;

ALTER TABLE affiliate_commissions
    DROP CONSTRAINT IF EXISTS affiliate_commissions_source_order_id_fkey;

ALTER TABLE affiliate_commissions
    ADD CONSTRAINT affiliate_commissions_source_order_id_fkey
    FOREIGN KEY (source_order_id) REFERENCES orders(id) NOT VALID;

COMMIT;
