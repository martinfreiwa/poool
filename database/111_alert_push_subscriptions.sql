-- Migration 111: Web push subscriptions for alert notifications.

CREATE TABLE IF NOT EXISTS marketplace_alert_push_subscriptions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    endpoint     TEXT NOT NULL,
    p256dh       TEXT NOT NULL,
    auth         TEXT NOT NULL,
    user_agent   TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_alert_push_endpoint
    ON marketplace_alert_push_subscriptions(endpoint);

CREATE INDEX IF NOT EXISTS idx_alert_push_user
    ON marketplace_alert_push_subscriptions(user_id);
