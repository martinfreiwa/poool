-- Migration 194: multi-payout-method per affiliate (Phase 4).
--
-- Pre-Phase-4 the only payout target was the SEPA IBAN on
-- `developer_teams`. Industry-standard (Wise, PayPal, Stripe Connect,
-- USDC) requires per-affiliate, per-method selection so affiliates
-- outside the EU SEPA zone can get paid too.
--
-- Each row is one configured destination. An affiliate may have
-- multiple rows (e.g. SEPA + USDC as backup); `is_default = TRUE`
-- selects the one the payout-batch worker uses. A partial-unique index
-- guarantees at-most-one default per affiliate.
--
-- Identifier handling:
--   * `identifier_encrypted` — AES-256-GCM envelope for sensitive
--     identifiers (IBAN, account number). Encryption uses the existing
--     `BANK_IBAN_ENCRYPTION_KEY` helper from `rewards::service`.
--   * `identifier_last4` — cached last-4 chars for masked display.
--   * `identifier_plain` — non-sensitive identifiers (PayPal email,
--     Wise email) that the user can already see in the UI; no point
--     encrypting these.
--
-- Method-type-specific validation lives in Rust (`validate_payout_method`).
-- Schema CHECK just enforces the whitelist + an at-least-one-identifier
-- guard.
--
-- Idempotent. Safe to re-run.

CREATE TABLE IF NOT EXISTS affiliate_payout_methods (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id          UUID NOT NULL REFERENCES affiliates(user_id) ON DELETE CASCADE,
    method_type           VARCHAR(20) NOT NULL
                          CHECK (method_type IN
                              ('sepa_iban', 'paypal_email', 'wise_email',
                               'usdc_wallet', 'stripe_connect')),
    -- One of `identifier_encrypted` (encrypted blob) OR
    -- `identifier_plain` (non-sensitive plaintext) must be set. Both can
    -- be set for redundancy but only one is canonical per method.
    identifier_encrypted  TEXT,
    identifier_plain      VARCHAR(255),
    identifier_last4      VARCHAR(8),
    key_version           INTEGER,
    -- Optional user-friendly label ("Backup PayPal", "Wise USD").
    label                 VARCHAR(80),
    is_default            BOOLEAN NOT NULL DEFAULT FALSE,
    is_active             BOOLEAN NOT NULL DEFAULT TRUE,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT affiliate_payout_methods_has_identifier
        CHECK (identifier_encrypted IS NOT NULL OR identifier_plain IS NOT NULL)
);

-- One default per affiliate at any time.
CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_payout_methods_default
    ON affiliate_payout_methods (affiliate_id)
    WHERE is_default = TRUE AND is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_affiliate_payout_methods_affiliate
    ON affiliate_payout_methods (affiliate_id, is_active);

CREATE TRIGGER set_affiliate_payout_methods_updated_at
    BEFORE UPDATE ON affiliate_payout_methods
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

COMMENT ON TABLE affiliate_payout_methods IS
  'Phase-4: per-affiliate payout destinations (SEPA / PayPal / Wise / USDC / Stripe Connect). One is_default = TRUE per active set selects the payout-batch target.';
