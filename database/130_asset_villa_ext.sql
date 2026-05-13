-- 130 — Villa-Returns P1: extend `assets` with tokenization, payout, reserve, and currency fields.
--
-- Forward-compatible: all columns nullable or with safe DEFAULTs. No data loss path.
-- See drafts/villa-returns-implementation-plan.md §3.3 for the full schema design.
--
-- Q1 lock-in: native_currency_code defaults to 'IDR' (most villas operate in IDR; USD is derived display).
-- Q4 lock-in: allow_developer_submission defaults TRUE so developers can submit from day one;
--             per-asset kill-switch retained for incident response.

ALTER TABLE assets
  ADD COLUMN IF NOT EXISTS tokenized_pct_bps          INTEGER
    CHECK (tokenized_pct_bps IS NULL OR (tokenized_pct_bps BETWEEN 0 AND 10000)),
  ADD COLUMN IF NOT EXISTS tokens_payout_eligible     INTEGER
    CHECK (tokens_payout_eligible IS NULL OR tokens_payout_eligible >= 0),
  ADD COLUMN IF NOT EXISTS tokens_owner_retained      INTEGER
    CHECK (tokens_owner_retained IS NULL OR tokens_owner_retained >= 0),
  ADD COLUMN IF NOT EXISTS reserve_pct_bps            INTEGER NOT NULL DEFAULT 500
    CHECK (reserve_pct_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS payout_frequency           VARCHAR(20) NOT NULL DEFAULT 'monthly'
    CHECK (payout_frequency IN ('monthly','quarterly','annual')),
  ADD COLUMN IF NOT EXISTS payout_currency            CHAR(3) NOT NULL DEFAULT 'USD',
  ADD COLUMN IF NOT EXISTS distribution_record_day    INTEGER NOT NULL DEFAULT 1
    CHECK (distribution_record_day BETWEEN 1 AND 28),
  ADD COLUMN IF NOT EXISTS withholding_tax_bps        INTEGER NOT NULL DEFAULT 0
    CHECK (withholding_tax_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS allow_developer_submission BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS native_currency_code       CHAR(3) NOT NULL DEFAULT 'IDR',
  ADD COLUMN IF NOT EXISTS mgmt_fee_bps               INTEGER
    CHECK (mgmt_fee_bps IS NULL OR (mgmt_fee_bps BETWEEN 0 AND 10000));

COMMENT ON COLUMN assets.tokenized_pct_bps          IS 'PDF §7: portion of the villa sold via POOOL (10000 = 100%). NAV = (value * tokenized_pct / 10000) / (tokens_total - tokens_owner_retained).';
COMMENT ON COLUMN assets.tokens_payout_eligible     IS 'Tokens that receive monthly distributions. tokens_total minus locked/owner-retained.';
COMMENT ON COLUMN assets.tokens_owner_retained      IS 'Tokens kept by the asset owner outside the investor pool.';
COMMENT ON COLUMN assets.reserve_pct_bps            IS 'Default reserve allocation as bps of net rental. Per-period override in villa_operations_log.reserve_override_idr_cents.';
COMMENT ON COLUMN assets.payout_currency            IS 'Currency in which investors receive distributions (USD, IDR, USDT, ...).';
COMMENT ON COLUMN assets.distribution_record_day    IS 'Day of month (1-28) at which holdings snapshot for distribution eligibility.';
COMMENT ON COLUMN assets.withholding_tax_bps        IS 'SPV-level tax withheld before distribution. Per-investor tax handled separately.';
COMMENT ON COLUMN assets.allow_developer_submission IS 'Per-asset kill-switch for developer self-submission. Default TRUE (Q4 lock-in).';
COMMENT ON COLUMN assets.native_currency_code       IS 'Currency villa books are kept in. IDR by default (Q1 lock-in). USD derived at publish.';
COMMENT ON COLUMN assets.mgmt_fee_bps               IS 'Contractually agreed management fee as bps of gross or net rental (per contract). The actual fee paid each month is captured in villa_operations_log.mgmt_fee_idr_cents.';
