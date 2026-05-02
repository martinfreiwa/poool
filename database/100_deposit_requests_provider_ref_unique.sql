-- ═══════════════════════════════════════════════════════════════
-- 100_deposit_requests_provider_ref_unique.sql
-- Enforce UNIQUE on deposit_requests.provider_reference.
--
-- Why: confirm_deposit() in backend/src/payments/service.rs is built around
-- the assumption that provider_reference uniquely identifies a deposit. The
-- function does:
--   SELECT … WHERE provider_reference = $1 FOR UPDATE
-- and treats the result as the canonical row. Without a UNIQUE constraint,
-- two rows with the same provider_reference could exist (admin double-entry,
-- buggy import, race in create_deposit_request) and only one would ever
-- get its webhook applied — the other stays 'pending' forever.
--
-- Also closes the door on a webhook-replay attack vector: an attacker who
-- gets the platform to create two deposit_requests rows pointing at the
-- same external reference could front-run the webhook to whichever user
-- they prefer.
--
-- Allowing NULL provider_reference (legacy rows where the provider didn't
-- return a reference yet). UNIQUE in Postgres treats multiple NULLs as
-- distinct, so this is safe.
-- ═══════════════════════════════════════════════════════════════

-- Drop the plain index first; the UNIQUE constraint creates its own.
DROP INDEX IF EXISTS idx_deposit_req_provider_ref;

-- Use a partial unique index: NULLs are excluded so legacy rows without a
-- reference don't collide with each other.
CREATE UNIQUE INDEX IF NOT EXISTS uq_deposit_req_provider_ref
    ON deposit_requests(provider_reference)
    WHERE provider_reference IS NOT NULL;
