-- 18.5 — webhook_events: durable log of inbound webhook deliveries.
--
-- Every webhook the platform receives (Stripe, OCBC, Midtrans, etc.) lands
-- in this table BEFORE business logic runs so we can:
--   1. Replay a delivery later if the handler crashed.
--   2. Audit which provider sent what when, with the raw signed payload.
--   3. Distinguish "delivery received but handler errored" from
--      "delivery never arrived", which is otherwise indistinguishable.
--
-- The handler workflow is:
--   - On request: INSERT row with status='received', payload (JSONB), and
--     event_type pulled from the provider's header / payload field.
--   - After processing: UPDATE status to 'processed', or 'error' + populate
--     `error_message`.
--
-- `provider_event_id` is the provider's idempotency key (Stripe: event.id;
-- OCBC: transaction ref). The UNIQUE on (provider, provider_event_id) is
-- partial because some providers don't supply a stable id for every event.

CREATE TABLE IF NOT EXISTS webhook_events (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider            VARCHAR(30) NOT NULL
                        CHECK (provider IN ('stripe', 'ocbc', 'midtrans', 'mangopay', 'sumsub', 'manual')),
    event_type          VARCHAR(80) NOT NULL,
    provider_event_id   VARCHAR(255),
    payload             JSONB NOT NULL,
    signature_valid     BOOLEAN NOT NULL DEFAULT FALSE,
    status              VARCHAR(20) NOT NULL DEFAULT 'received'
                        CHECK (status IN ('received', 'processed', 'error', 'ignored')),
    processed_at        TIMESTAMPTZ,
    error_message       TEXT,
    related_entity_type VARCHAR(40),   -- e.g. 'deposit_request', 'withdrawal_request', 'kyc_record'
    related_entity_id   UUID,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_webhook_events_provider_status
    ON webhook_events(provider, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_webhook_events_event_type
    ON webhook_events(event_type, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_webhook_events_related
    ON webhook_events(related_entity_type, related_entity_id)
    WHERE related_entity_id IS NOT NULL;

-- Partial unique on (provider, provider_event_id) — only enforces uniqueness
-- when the provider supplied an id. Providers that don't supply one (manual,
-- some legacy OCBC payloads) write NULL and are allowed to duplicate.
CREATE UNIQUE INDEX IF NOT EXISTS idx_webhook_events_idempotency
    ON webhook_events(provider, provider_event_id)
    WHERE provider_event_id IS NOT NULL;
