-- Migration 109: Alert assignee, audit trail, detection rules, snooze
-- Extends marketplace_alerts (mig 054) with assignee + snooze.
-- Adds: marketplace_alert_audit, marketplace_alert_rules.
-- Extends marketplace_watchlist to support arbitrary entity types
-- (user, wallet, asset, ip) — was user-only.

-- ── 1. Extend marketplace_alerts ────────────────────────────────────
ALTER TABLE marketplace_alerts
    ADD COLUMN IF NOT EXISTS assigned_to     UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS snoozed_until   TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS rule_id         UUID,
    ADD COLUMN IF NOT EXISTS escalated_at    TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_alerts_assignee
    ON marketplace_alerts(assigned_to)
    WHERE status IN ('new', 'acknowledged');

CREATE INDEX IF NOT EXISTS idx_alerts_snoozed
    ON marketplace_alerts(snoozed_until)
    WHERE snoozed_until IS NOT NULL;

-- ── 2. Extend marketplace_watchlist ─────────────────────────────────
ALTER TABLE marketplace_watchlist
    ADD COLUMN IF NOT EXISTS entity_type        VARCHAR(20) NOT NULL DEFAULT 'user'
                              CHECK (entity_type IN ('user', 'wallet', 'asset', 'ip')),
    ADD COLUMN IF NOT EXISTS entity_identifier  TEXT;

-- Backfill identifier for existing user-typed entries
UPDATE marketplace_watchlist
   SET entity_identifier = user_id::TEXT
 WHERE entity_identifier IS NULL AND user_id IS NOT NULL;

ALTER TABLE marketplace_watchlist
    ALTER COLUMN user_id DROP NOT NULL;

-- Drop the user-only unique index, add a generic one
DROP INDEX IF EXISTS idx_watchlist_user;
CREATE UNIQUE INDEX IF NOT EXISTS idx_watchlist_entity
    ON marketplace_watchlist(entity_type, entity_identifier)
    WHERE is_active = true;

-- ── 3. Audit trail (who did what to which alert) ────────────────────
CREATE TABLE IF NOT EXISTS marketplace_alert_audit (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_id     UUID NOT NULL REFERENCES marketplace_alerts(id) ON DELETE CASCADE,
    by_user_id   UUID NOT NULL REFERENCES users(id),
    action       VARCHAR(40) NOT NULL,
    details      JSONB,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_audit_alert
    ON marketplace_alert_audit(alert_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_alert_audit_user
    ON marketplace_alert_audit(by_user_id, created_at DESC);

-- ── 4. Detection rules ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS marketplace_alert_rules (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                VARCHAR(100) NOT NULL,
    category            VARCHAR(20) NOT NULL DEFAULT 'anomaly'
                        CHECK (category IN ('trading', 'compliance', 'system', 'anomaly')),
    severity            VARCHAR(15) NOT NULL DEFAULT 'warning'
                        CHECK (severity IN ('info', 'warning', 'critical')),
    threshold_text      TEXT,
    escalate_after_min  INTEGER NOT NULL DEFAULT 0,
    channel             VARCHAR(20) NOT NULL DEFAULT 'none'
                        CHECK (channel IN ('none', 'slack', 'email', 'sms', 'page')),
    enabled             BOOLEAN NOT NULL DEFAULT true,
    created_by          UUID REFERENCES users(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_alert_rules_name
    ON marketplace_alert_rules(name);

CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled
    ON marketplace_alert_rules(enabled, category)
    WHERE enabled = true;

-- ── 5. Wire alerts.rule_id FK after rules table exists ──────────────
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'fk_alerts_rule'
    ) THEN
        ALTER TABLE marketplace_alerts
            ADD CONSTRAINT fk_alerts_rule
            FOREIGN KEY (rule_id) REFERENCES marketplace_alert_rules(id) ON DELETE SET NULL;
    END IF;
END $$;
