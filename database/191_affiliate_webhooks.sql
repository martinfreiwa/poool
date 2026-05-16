-- Migration 191: per-affiliate event webhooks (Phase 4).
--
-- Replaces the legacy single-URL `affiliates.postback_url` with a
-- proper subscription model:
--   * Multiple endpoints per affiliate (e.g. one for `commission_earned`,
--     another for `payout_released`).
--   * HMAC-SHA256 signed payloads (header `X-Poool-Signature`).
--   * Durable retry via existing `affiliate_postback_outbox` (this
--     migration adds `subscription_id` to bind a queued outbox row to a
--     specific endpoint so per-endpoint deactivation drains cleanly).
--
-- We DON'T drop `affiliates.postback_url` yet — backward-compat for the
-- existing personal-affiliate flow. Phase-5 deprecation once all callers
-- migrate.
--
-- Event-type whitelist enforced in the Rust dispatcher (no DB CHECK so
-- adding new event types doesn't require a migration).
--
-- Idempotent. Safe to re-run.

CREATE TABLE IF NOT EXISTS affiliate_webhook_subscriptions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- HTTPS-only endpoint. The dispatcher's SSRF guard re-validates the
    -- URL on every send so a later URL flip doesn't bypass the check.
    url             TEXT NOT NULL CHECK (url LIKE 'https://%'),
    -- Comma-separated event list, e.g. 'commission_earned,payout_released'.
    -- The literal '*' means subscribe to all events.
    event_types     TEXT NOT NULL CHECK (length(event_types) BETWEEN 1 AND 500),
    -- HMAC secret. 64 hex chars (256 bits). Caller generates + stores
    -- their copy at create time. Never returned in list endpoints.
    secret_hash     CHAR(64) NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    -- Free-form label shown in admin UI ("CRM webhook", "Zapier").
    description     VARCHAR(160),
    last_success_at TIMESTAMPTZ,
    last_failure_at TIMESTAMPTZ,
    last_status_code INTEGER,
    failure_count   INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_affiliate_webhook_subs_user
    ON affiliate_webhook_subscriptions (user_id)
    WHERE is_active = TRUE;

-- Bind queued outbox rows to a specific subscription so deactivating a
-- single endpoint drains cleanly. Column is NULLABLE for back-compat
-- with rows queued before this migration ran.
ALTER TABLE affiliate_postback_outbox
    ADD COLUMN IF NOT EXISTS subscription_id UUID
        REFERENCES affiliate_webhook_subscriptions(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_affiliate_postback_outbox_subscription
    ON affiliate_postback_outbox (subscription_id)
    WHERE subscription_id IS NOT NULL;

CREATE TRIGGER set_affiliate_webhook_subs_updated_at
    BEFORE UPDATE ON affiliate_webhook_subscriptions
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

COMMENT ON TABLE affiliate_webhook_subscriptions IS
  'Phase-4: per-affiliate, per-event-type webhook subscriptions with HMAC-SHA256 signed payloads. Replaces single-URL affiliates.postback_url.';
