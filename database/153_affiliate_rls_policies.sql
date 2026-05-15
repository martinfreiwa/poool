-- 142: Row-level security on affiliate/PII tables (defense in depth)
--
-- Audit finding: a `grep` for `ENABLE ROW LEVEL SECURITY` / `CREATE POLICY`
-- across the entire `database/` directory returned 0 hits. None of the
-- affiliate or PII tables were protected. If a non-owner role is ever
-- exposed (Supabase anon/authenticated, a leaked PostgREST proxy, a
-- read-replica with a separate user, an analytics role), it would have
-- full table access including encrypted tax IDs, phone numbers, IPs.
--
-- This migration enables RLS and installs least-privilege policies. The
-- backend connects as the table owner, which bypasses RLS by default
-- (Postgres doesn't apply RLS to owners unless `FORCE ROW LEVEL SECURITY`
-- is set). That means today these policies are inert — the backend keeps
-- working exactly as before. The policies activate the moment a non-owner
-- role queries these tables, or whenever an operator runs
-- `ALTER TABLE ... FORCE ROW LEVEL SECURITY` to enforce them on the
-- backend too. The policy text assumes the application sets
-- `app.current_user_id` per request before any user-scoped query — this
-- is a known follow-up; until then, FORCE must remain off.

BEGIN;

-- Helper: read the current request user id from a session var. Returns
-- NULL when unset (which means policies evaluate to false for that path).
CREATE OR REPLACE FUNCTION app_current_user_id() RETURNS uuid
LANGUAGE sql STABLE AS $$
    SELECT NULLIF(current_setting('app.current_user_id', true), '')::uuid
$$;

-- ── affiliates (PII: tax_id, phone, address, etc.) ──────────────────────
ALTER TABLE affiliates ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS affiliates_self_read ON affiliates;
CREATE POLICY affiliates_self_read ON affiliates
    FOR SELECT USING (user_id = app_current_user_id());
DROP POLICY IF EXISTS affiliates_self_update ON affiliates;
CREATE POLICY affiliates_self_update ON affiliates
    FOR UPDATE USING (user_id = app_current_user_id())
    WITH CHECK (user_id = app_current_user_id());

-- ── affiliate_referrals (per-affiliate visibility) ──────────────────────
ALTER TABLE affiliate_referrals ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS affiliate_referrals_owner_read ON affiliate_referrals;
CREATE POLICY affiliate_referrals_owner_read ON affiliate_referrals
    FOR SELECT USING (affiliate_id = app_current_user_id());

-- ── affiliate_commissions (financial; per-affiliate visibility) ─────────
ALTER TABLE affiliate_commissions ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS affiliate_commissions_owner_read ON affiliate_commissions;
CREATE POLICY affiliate_commissions_owner_read ON affiliate_commissions
    FOR SELECT USING (affiliate_id = app_current_user_id());

-- ── affiliate_payout_requests (per-affiliate visibility) ────────────────
ALTER TABLE affiliate_payout_requests ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS affiliate_payout_requests_owner_read ON affiliate_payout_requests;
CREATE POLICY affiliate_payout_requests_owner_read ON affiliate_payout_requests
    FOR SELECT USING (affiliate_id = app_current_user_id());
DROP POLICY IF EXISTS affiliate_payout_requests_owner_insert ON affiliate_payout_requests;
CREATE POLICY affiliate_payout_requests_owner_insert ON affiliate_payout_requests
    FOR INSERT WITH CHECK (affiliate_id = app_current_user_id());

-- ── affiliate_materials (per-affiliate visibility) ──────────────────────
ALTER TABLE affiliate_materials ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS affiliate_materials_owner_read ON affiliate_materials;
CREATE POLICY affiliate_materials_owner_read ON affiliate_materials
    FOR SELECT USING (affiliate_id = app_current_user_id());
DROP POLICY IF EXISTS affiliate_materials_owner_write ON affiliate_materials;
CREATE POLICY affiliate_materials_owner_write ON affiliate_materials
    FOR INSERT WITH CHECK (affiliate_id = app_current_user_id());

-- ── affiliate_policy_acceptances (legal log; per-affiliate read; immutable) ─
ALTER TABLE affiliate_policy_acceptances ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS affiliate_policy_acceptances_owner_read ON affiliate_policy_acceptances;
CREATE POLICY affiliate_policy_acceptances_owner_read ON affiliate_policy_acceptances
    FOR SELECT USING (affiliate_id = app_current_user_id());
DROP POLICY IF EXISTS affiliate_policy_acceptances_owner_insert ON affiliate_policy_acceptances;
CREATE POLICY affiliate_policy_acceptances_owner_insert ON affiliate_policy_acceptances
    FOR INSERT WITH CHECK (affiliate_id = app_current_user_id());
-- No UPDATE/DELETE policies → defense-in-depth append-only enforcement.

-- ── investment_disclosures_log (per-user read; append-only) ─────────────
ALTER TABLE investment_disclosures_log ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS investment_disclosures_log_owner_read ON investment_disclosures_log;
CREATE POLICY investment_disclosures_log_owner_read ON investment_disclosures_log
    FOR SELECT USING (user_id = app_current_user_id());
DROP POLICY IF EXISTS investment_disclosures_log_owner_insert ON investment_disclosures_log;
CREATE POLICY investment_disclosures_log_owner_insert ON investment_disclosures_log
    FOR INSERT WITH CHECK (user_id = app_current_user_id());

-- ── Admin-only tables (no per-user policy) ──────────────────────────────
-- Enabling RLS without policies = deny-all for any non-owner role. The
-- backend owner still bypasses. Future admin-role connections must use
-- `SECURITY DEFINER` functions or be granted explicit policies.
ALTER TABLE payout_batches ENABLE ROW LEVEL SECURITY;
ALTER TABLE affiliate_conduct_incidents ENABLE ROW LEVEL SECURITY;
ALTER TABLE referral_clicks ENABLE ROW LEVEL SECURITY;

COMMIT;

-- Operator follow-up to fully enforce (after app sets app.current_user_id
-- on every user-scoped DB session):
--   ALTER TABLE affiliates                    FORCE ROW LEVEL SECURITY;
--   ALTER TABLE affiliate_referrals           FORCE ROW LEVEL SECURITY;
--   ALTER TABLE affiliate_commissions         FORCE ROW LEVEL SECURITY;
--   ALTER TABLE affiliate_payout_requests     FORCE ROW LEVEL SECURITY;
--   ALTER TABLE affiliate_materials           FORCE ROW LEVEL SECURITY;
--   ALTER TABLE affiliate_policy_acceptances  FORCE ROW LEVEL SECURITY;
--   ALTER TABLE investment_disclosures_log    FORCE ROW LEVEL SECURITY;
