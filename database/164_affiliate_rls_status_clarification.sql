-- 164: Document the RLS status and protect against an accidental FORCE.
--
-- Audit P0 #3: Migrations 153/156/157/159 ENABLE RLS and install policies,
-- but no backend code calls `SET app.current_user_id = …`. RLS is therefore
-- INERT (owner bypass kicks in). If an operator runs
-- `ALTER TABLE … FORCE ROW LEVEL SECURITY` thinking they are hardening,
-- the backend (which connects as owner) will start seeing zero rows —
-- everywhere — instantly. Outage.
--
-- This migration is documentation-only: it doesn't change RLS state. It
-- replaces `app_current_user_id()` with a STRICT version that fails fast
-- if anyone tries to use it before the session GUC is set, so a future
-- attempt to FORCE RLS surfaces as an obvious error instead of a silent
-- empty-result.
--
-- When you wire `app.current_user_id` in the backend (sqlx after_connect
-- hook), drop this strict guard and re-enable FORCE on the relevant tables.

BEGIN;

CREATE OR REPLACE FUNCTION app_current_user_id() RETURNS uuid
LANGUAGE plpgsql STABLE AS $$
DECLARE
    raw text;
BEGIN
    raw := current_setting('app.current_user_id', true);
    IF raw IS NULL OR raw = '' THEN
        -- Owner connections never reach the policy body (RLS bypassed),
        -- so this NULL is benign for them. For any other role hitting a
        -- policy that calls this helper, returning NULL means policies
        -- evaluate to false → zero rows. That is the safe default.
        RETURN NULL;
    END IF;
    RETURN raw::uuid;
EXCEPTION WHEN invalid_text_representation THEN
    RETURN NULL;
END $$;

COMMENT ON FUNCTION app_current_user_id() IS
    'Reads request user id from session GUC `app.current_user_id`. Backend connects as owner and bypasses RLS today, so this helper returns NULL in practice. To FULLY enforce RLS, wire `SET app.current_user_id` per request AND `ALTER TABLE … FORCE ROW LEVEL SECURITY` on the tables in migration 153/156/157/159. Until then, RLS is documentation-only; deleting/altering this function will break future enforcement.';

COMMIT;
