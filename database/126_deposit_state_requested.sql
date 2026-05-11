-- 18.1 — Add `requested` state to deposit_requests.
--
-- The current flow inserts straight to 'pending' on order creation. The
-- masterplan calls for an additional 'requested' state that represents the
-- user's intent before the payment provider has acknowledged the deposit
-- (e.g. Stripe Payment Intent created but no webhook received yet). 'pending'
-- then specifically means "provider has acknowledged, awaiting clearance".
--
-- This migration extends the existing CHECK constraint without backfilling
-- — existing rows in `pending` keep their meaning. New rows opting into the
-- expanded state machine should INSERT with status='requested' and update to
-- 'pending' on first webhook ack.

ALTER TABLE deposit_requests
    DROP CONSTRAINT IF EXISTS deposit_requests_status_check;

ALTER TABLE deposit_requests
    ADD CONSTRAINT deposit_requests_status_check
    CHECK (status IN ('requested', 'pending', 'paid', 'expired', 'failed', 'cancelled'));

-- Index supports the webhook reconciliation path which looks up "requested"
-- rows by provider + provider_reference (the value Stripe writes into the
-- PaymentIntent metadata).
CREATE INDEX IF NOT EXISTS idx_deposit_req_status_provider
    ON deposit_requests(provider, status)
    WHERE status IN ('requested', 'pending');
