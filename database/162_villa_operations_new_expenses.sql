-- 162 — Villa-Returns P2: add structured expense columns to villa_operations_log.
--
-- New fields (all BIGINT cents, IDR-native, DEFAULT 0):
--   expense_property_tax_idr_cents  — Pajak Bumi dan Bangunan (PBB); annual, amortize monthly
--   expense_insurance_idr_cents     — fire, liability, contents insurance
--   expense_accounting_idr_cents    — notary, audit, accountant fees; annual, amortize monthly
--   expense_internet_idr_cents      — WiFi, Netflix, TV subscriptions for guests
--   expense_capex_idr_cents         — renovation / refurbishment (CapEx)
--
-- NOTE ON CAPEX: capital expenditure is NOT included in total_opex_idr_cents and
-- therefore does NOT reduce distributable_idr_cents. It is stored for transparency
-- and reporting but is treated as a separate cash-flow line, not an operating expense.
-- The four new OpEx fields (property_tax, insurance, accounting, internet) ARE
-- included in total_opex when rows are recomputed going forward.
--
-- Existing published rows keep their current total_opex / distributable values — they
-- are NOT retroactively recomputed. The new columns simply default to 0.

ALTER TABLE villa_operations_log
    ADD COLUMN IF NOT EXISTS expense_property_tax_idr_cents  BIGINT NOT NULL DEFAULT 0
        CHECK (expense_property_tax_idr_cents >= 0),
    ADD COLUMN IF NOT EXISTS expense_insurance_idr_cents      BIGINT NOT NULL DEFAULT 0
        CHECK (expense_insurance_idr_cents >= 0),
    ADD COLUMN IF NOT EXISTS expense_accounting_idr_cents     BIGINT NOT NULL DEFAULT 0
        CHECK (expense_accounting_idr_cents >= 0),
    ADD COLUMN IF NOT EXISTS expense_internet_idr_cents       BIGINT NOT NULL DEFAULT 0
        CHECK (expense_internet_idr_cents >= 0),
    ADD COLUMN IF NOT EXISTS expense_capex_idr_cents          BIGINT NOT NULL DEFAULT 0
        CHECK (expense_capex_idr_cents >= 0);

COMMENT ON COLUMN villa_operations_log.expense_property_tax_idr_cents
    IS 'Pajak Bumi dan Bangunan (PBB) — annual Indonesian land & building tax, amortized monthly';
COMMENT ON COLUMN villa_operations_log.expense_insurance_idr_cents
    IS 'Property insurance: fire, liability, contents — monthly portion of annual premium';
COMMENT ON COLUMN villa_operations_log.expense_accounting_idr_cents
    IS 'Accounting, audit, notary and legal compliance costs — amortized monthly';
COMMENT ON COLUMN villa_operations_log.expense_internet_idr_cents
    IS 'Internet (WiFi) and guest-facing streaming subscriptions (Netflix, TV, etc.)';
COMMENT ON COLUMN villa_operations_log.expense_capex_idr_cents
    IS 'Capital expenditure: renovation / refurbishment. NOT included in total_opex — tracked separately for transparency';
