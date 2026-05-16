-- ══════════════════════════════════════════════════════════════
-- 183_transaction_monitoring.sql
--
-- Rule-based transaction monitoring (P0-1).
--
-- A small set of rules runs hourly across all wallet movements and
-- opens compliance_alerts (introduced in migration 182) when patterns
-- consistent with money-laundering / structuring / fraud appear.
--
-- Tables:
--   transaction_monitoring_rules      — rule catalogue, admin-tunable
--                                        thresholds. Disabled rules
--                                        stay in the DB so we keep the
--                                        decision history.
--   transaction_monitoring_findings   — every match emitted by the
--                                        engine, append-only. Linked to
--                                        compliance_alerts.
--
-- Each rule is identified by a stable string code (e.g. `large_deposit`,
-- `rapid_deposits`) so code references survive primary-key rotations.
-- ══════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS transaction_monitoring_rules (
    code            VARCHAR(64) PRIMARY KEY,
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    severity        VARCHAR(10) NOT NULL DEFAULT 'medium'
                    CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    -- JSON config — each rule defines its own thresholds (cents,
    -- counts, hours). Keeps the schema generic so adding a rule
    -- doesn't require an ALTER.
    config          JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by      UUID REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS transaction_monitoring_findings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rule_code       VARCHAR(64) NOT NULL REFERENCES transaction_monitoring_rules(code),
    severity        VARCHAR(10) NOT NULL DEFAULT 'medium'
                    CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    summary         TEXT NOT NULL,
    details         JSONB,
    -- Link to compliance_alerts so triage isn't split across two tables
    alert_id        UUID REFERENCES compliance_alerts(id) ON DELETE SET NULL,
    -- Window the rule looked at (so two runs over the same window don't
    -- double-emit; the engine dedupes by (user_id, rule_code, window_end))
    window_start    TIMESTAMPTZ NOT NULL,
    window_end      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (user_id, rule_code, window_end)
);

CREATE INDEX IF NOT EXISTS idx_tx_monitoring_findings_user
    ON transaction_monitoring_findings(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_tx_monitoring_findings_rule
    ON transaction_monitoring_findings(rule_code, created_at DESC);

-- Seed the initial rule set. Threshold defaults match common
-- AMLD/FinCEN heuristics; compliance can tune them via the
-- transaction_monitoring_rules.config column.
INSERT INTO transaction_monitoring_rules (code, name, description, enabled, severity, config) VALUES
  (
    'large_deposit',
    'Large single deposit',
    'A single deposit at or above the threshold. Standard CTR (Currency Transaction Report) tripwire.',
    TRUE, 'medium',
    '{"threshold_cents": 1000000}'::jsonb
  ),
  (
    'rapid_deposits',
    'Rapid sequential deposits',
    'N+ pending or paid deposits within the rolling window. Catches deposit-spam attempts and structuring.',
    TRUE, 'high',
    '{"count_threshold": 5, "window_hours": 24}'::jsonb
  ),
  (
    'structuring_deposits',
    'Possible structuring',
    'Multiple deposits each just below the CTR threshold within a short window — classic structuring pattern.',
    TRUE, 'high',
    '{"upper_cents": 1000000, "lower_cents": 800000, "count_threshold": 3, "window_hours": 168}'::jsonb
  ),
  (
    'withdraw_new_bank',
    'Withdrawal to a freshly added bank',
    'User withdraws to a bank account added within the cooldown window — common money-mule signal.',
    TRUE, 'high',
    '{"cooldown_hours": 168}'::jsonb
  ),
  (
    'velocity_spike',
    'Volume spike vs baseline',
    'Deposits or withdrawals in the last 24h exceed N times the user''s 30-day daily average.',
    TRUE, 'medium',
    '{"multiplier": 10, "min_baseline_cents": 50000}'::jsonb
  )
ON CONFLICT (code) DO NOTHING;

INSERT INTO platform_settings (key, value, value_type, description) VALUES
  ('tx_monitoring_interval_minutes',
   '60',
   'number',
   'How often the transaction-monitoring rule engine runs. Default 60 minutes.')
ON CONFLICT (key) DO NOTHING;
