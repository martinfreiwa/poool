-- 167: Bank-account / payout-destination columns on developer_teams
--
-- The team owner (developer) needs a place to declare where the consolidated
-- team commission should be paid out. Today the platform tracks the AMOUNT
-- (affiliate_live_counters.payable_commission_cents) and the EARLIEST date
-- (affiliate_live_counters.next_payout_date) but has no field for the
-- destination account, so payouts cannot actually be initiated.
--
-- This migration adds plain-text columns intentionally — the existing
-- tax-id encryption pattern (Aes256Gcm + TAX_ID_ENCRYPTION_KEY env var,
-- see backend/src/rewards/service.rs) should be applied to bank_iban
-- before production traffic. Tracked as a follow-up.
--
-- All columns are NULLable so existing teams keep working until the owner
-- fills the form. The UPDATE handler in rewards/team_routes.rs validates
-- format before INSERT.

BEGIN;

ALTER TABLE developer_teams
    ADD COLUMN bank_account_holder TEXT,
    ADD COLUMN bank_iban            TEXT,   -- TODO: encrypt with AES-256-GCM (see TaxIdStorage)
    ADD COLUMN bank_bic             VARCHAR(11),
    ADD COLUMN bank_name            TEXT,
    ADD COLUMN bank_country         VARCHAR(2);  -- ISO-3166-1 alpha-2

-- Compact length sanity-checks; format validation happens application-side.
ALTER TABLE developer_teams
    ADD CONSTRAINT developer_teams_bank_account_holder_len
        CHECK (bank_account_holder IS NULL OR char_length(bank_account_holder) BETWEEN 1 AND 120),
    ADD CONSTRAINT developer_teams_bank_iban_len
        CHECK (bank_iban IS NULL OR char_length(bank_iban) BETWEEN 5 AND 64),
    ADD CONSTRAINT developer_teams_bank_bic_len
        CHECK (bank_bic IS NULL OR char_length(bank_bic) BETWEEN 8 AND 11),
    ADD CONSTRAINT developer_teams_bank_name_len
        CHECK (bank_name IS NULL OR char_length(bank_name) BETWEEN 1 AND 120),
    ADD CONSTRAINT developer_teams_bank_country_iso2
        CHECK (bank_country IS NULL OR bank_country ~ '^[A-Z]{2}$');

COMMIT;
