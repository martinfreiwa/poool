-- 177_rls_monitoring.sql
-- ──────────────────────────────────────────────────────────────────────────
-- B3 fix: monitor + safely-document the RLS state on affiliate tables.
--
-- Background:
--   * Mig 153 added RLS policies referencing `app_current_user_id()`.
--   * Backend connects as table OWNER → Postgres skips RLS unless
--     `FORCE ROW LEVEL SECURITY` is set. Migration 164 documented this.
--   * Risk: a future migration / DBA action that flips FORCE without
--     wiring `app.current_user_id` per-session would lock the app out.
--
-- This migration:
--   1. Adds a view that surfaces every RLS-enabled-but-NOT-forced table so
--      operators can SELECT * FROM rls_status_audit and see at-a-glance
--      which tables are in the "inert" state.
--   2. Adds an explicit DB-level comment on each affected table warning
--      against `FORCE ROW LEVEL SECURITY` without app-session wiring.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

-- 1. Audit view.
CREATE OR REPLACE VIEW rls_status_audit AS
SELECT
    n.nspname        AS schema_name,
    c.relname        AS table_name,
    c.relrowsecurity AS rls_enabled,
    c.relforcerowsecurity AS rls_forced,
    CASE
        WHEN c.relrowsecurity AND NOT c.relforcerowsecurity THEN 'INERT (owner bypass)'
        WHEN c.relrowsecurity AND c.relforcerowsecurity THEN 'ENFORCED'
        ELSE 'OFF'
    END AS effective_state,
    (SELECT COUNT(*) FROM pg_policy p WHERE p.polrelid = c.oid) AS policy_count
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE c.relkind = 'r'
  AND n.nspname NOT IN ('pg_catalog', 'information_schema')
ORDER BY effective_state, table_name;

COMMENT ON VIEW rls_status_audit IS
    'B3: per-table RLS state. `INERT (owner bypass)` = policies exist but app connects as owner so checks are skipped. Flipping to FORCE without wiring app.current_user_id per session WILL lock the backend out of those tables.';

-- 2. Per-table warnings.
COMMENT ON TABLE affiliates IS
    'RLS policies present (mig 153) but INERT: backend connects as table owner, no per-session app.current_user_id. Do NOT enable FORCE ROW LEVEL SECURITY without first wiring session-level user-id via SET LOCAL "app.current_user_id" = ... at request start.';
COMMENT ON TABLE affiliate_referrals IS
    'RLS policies present (mig 153) but INERT. See affiliates comment.';
COMMENT ON TABLE affiliate_commissions IS
    'RLS policies present (mig 153) but INERT. See affiliates comment.';
COMMENT ON TABLE developer_teams IS
    'RLS policies present (mig 156) but INERT. See affiliates comment.';
COMMENT ON TABLE developer_team_memberships IS
    'RLS policies present (mig 156) but INERT. See affiliates comment.';
COMMENT ON TABLE affiliate_links IS
    'RLS policies present (mig 157) but INERT. See affiliates comment.';

COMMIT;

\echo '── RLS state — INERT tables ──'
SELECT table_name, rls_enabled, rls_forced, effective_state, policy_count
  FROM rls_status_audit
 WHERE effective_state = 'INERT (owner bypass)'
 ORDER BY table_name;
