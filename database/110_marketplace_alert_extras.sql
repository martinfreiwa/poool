-- Migration 110: Saved alert views (per-user), rule mute schedules, alert daily counts matview.
-- Builds on mig 109 (alert rules, audit, snooze).

-- ── 1. Per-user saved filter views ──────────────────────────────────
CREATE TABLE IF NOT EXISTS marketplace_alert_views (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        VARCHAR(80) NOT NULL,
    state       JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alert_views_user
    ON marketplace_alert_views(user_id, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_alert_views_user_name
    ON marketplace_alert_views(user_id, name);

-- ── 2. Rule mute schedule (cron-like windows when rule should not fire) ─
ALTER TABLE marketplace_alert_rules
    ADD COLUMN IF NOT EXISTS mute_schedule  JSONB;
-- Example: {"weekends": true, "hours": [22,23,0,1,2,3,4,5,6]}
-- Interpreted by escalation worker; null/empty = always active.

-- ── 3. Daily counts materialized view for sparklines ────────────────
CREATE MATERIALIZED VIEW IF NOT EXISTS marketplace_alert_daily_counts AS
SELECT
    DATE_TRUNC('day', created_at)::DATE AS day,
    severity,
    COUNT(*)::INTEGER AS count
FROM marketplace_alerts
WHERE created_at > NOW() - INTERVAL '90 days'
GROUP BY 1, 2;

CREATE INDEX IF NOT EXISTS idx_alert_daily_counts_day
    ON marketplace_alert_daily_counts(day DESC);

-- Refresh recipe (run from cron or background worker):
--   REFRESH MATERIALIZED VIEW CONCURRENTLY marketplace_alert_daily_counts;
-- Need a unique index for CONCURRENTLY:
CREATE UNIQUE INDEX IF NOT EXISTS uq_alert_daily_counts
    ON marketplace_alert_daily_counts(day, severity);
