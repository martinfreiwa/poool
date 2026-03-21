-- Migration 054: Create marketplace_alerts + marketplace_watchlist tables
-- Purpose: Fraud detection alerts and admin watchlist for suspicious users
-- Ref: Masterplan §4.2 Mig054

CREATE TABLE marketplace_alerts (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_type   VARCHAR(50) NOT NULL,
    severity     VARCHAR(15) NOT NULL DEFAULT 'warning'
                 CHECK (severity IN ('info', 'warning', 'critical')),
    asset_id     UUID REFERENCES assets(id),
    user_id      UUID REFERENCES users(id),
    trade_id     UUID REFERENCES trade_history(id),
    message      TEXT NOT NULL,
    metadata     JSONB,
    status       VARCHAR(15) NOT NULL DEFAULT 'new'
                 CHECK (status IN ('new', 'acknowledged', 'resolved', 'false_positive')),
    resolved_by  UUID REFERENCES users(id),
    resolved_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Active alerts queue
CREATE INDEX idx_alerts_status ON marketplace_alerts(status)
    WHERE status IN ('new', 'acknowledged');

-- Severity-based ordering for new alerts
CREATE INDEX idx_alerts_severity ON marketplace_alerts(severity, created_at DESC)
    WHERE status = 'new';

-- Alerts per user
CREATE INDEX idx_alerts_user ON marketplace_alerts(user_id);

-- Admin watchlist for suspicious users
CREATE TABLE marketplace_watchlist (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users(id),
    reason     TEXT NOT NULL,
    added_by   UUID NOT NULL REFERENCES users(id),
    is_active  BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Only one active watchlist entry per user
CREATE UNIQUE INDEX idx_watchlist_user ON marketplace_watchlist(user_id)
    WHERE is_active = true;
