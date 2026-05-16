-- ══════════════════════════════════════════════════════════════
-- 182_sanctions_rescreening.sql
--
-- AMLD5/6 + OFAC ongoing-diligence: continuous re-screening of
-- approved users against sanctions / PEP lists.
--
-- Initial onboarding screening is handled by the KYC provider
-- (kyc/didit.rs). This migration adds:
--
--   sanctions_rescreening_log   — audit trail of every re-screening run
--                                  per user (status, summary, timestamp)
--   compliance_alerts           — actionable hits that compliance staff
--                                  must triage. One row per (user, kind,
--                                  open-or-closed) — closed_at NULL means
--                                  the alert is still open.
--
-- Both tables are append-mostly; old logs can be pruned after the
-- regulatory retention window (5y EU, 5y US).
-- ══════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS sanctions_rescreening_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          VARCHAR(20) NOT NULL
                    CHECK (status IN ('clear', 'hit', 'error', 'skipped')),
    provider        VARCHAR(40) NOT NULL,
    summary         TEXT,
    raw_response    JSONB,
    checked_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sanctions_log_user_time
    ON sanctions_rescreening_log(user_id, checked_at DESC);

CREATE INDEX IF NOT EXISTS idx_sanctions_log_status
    ON sanctions_rescreening_log(status, checked_at DESC)
    WHERE status IN ('hit', 'error');

-- Compliance-alerts is the work-queue. It is intentionally kind/severity-
-- typed so the same table can carry sanctions hits today and transaction-
-- monitoring findings tomorrow (P0-1).
CREATE TABLE IF NOT EXISTS compliance_alerts (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind             VARCHAR(40) NOT NULL
                     CHECK (kind IN (
                         'sanctions_hit', 'pep_hit', 'adverse_media',
                         'velocity_anomaly', 'structuring', 'manual_review'
                     )),
    severity         VARCHAR(10) NOT NULL DEFAULT 'medium'
                     CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    summary          TEXT NOT NULL,
    details          JSONB,
    -- Reference back to the source so triage can pull the raw evidence
    source_log_id    UUID REFERENCES sanctions_rescreening_log(id) ON DELETE SET NULL,
    -- Workflow state
    assigned_to      UUID REFERENCES users(id) ON DELETE SET NULL,
    closed_at        TIMESTAMPTZ,
    closed_by        UUID REFERENCES users(id) ON DELETE SET NULL,
    close_reason     TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_compliance_alerts_open
    ON compliance_alerts(severity, created_at DESC)
    WHERE closed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_compliance_alerts_user
    ON compliance_alerts(user_id, created_at DESC);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger
         WHERE tgname = 'set_updated_at'
           AND tgrelid = 'compliance_alerts'::regclass
    ) THEN
        CREATE TRIGGER set_updated_at BEFORE UPDATE ON compliance_alerts
        FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
    END IF;
END$$;

-- Re-screening cadence is admin-tunable. Default 30 days strikes the
-- standard EU/US bank rhythm without DoS'ing the screening API.
INSERT INTO platform_settings (key, value, value_type, description) VALUES
  ('sanctions_rescreening_interval_days',
   '30',
   'number',
   'How often each approved user is re-screened against sanctions / PEP lists. Default 30 days.')
ON CONFLICT (key) DO NOTHING;

-- Compliance staff need to read alerts + close them. Re-uses the
-- 'compliance' role added in migration 006.
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm
  FROM roles r,
       (VALUES ('compliance.read'), ('compliance.write')) AS p(perm)
 WHERE r.name = 'compliance'
ON CONFLICT DO NOTHING;
