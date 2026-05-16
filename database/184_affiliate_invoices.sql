-- Migration 184: affiliate invoices.
--
-- Phase-3 P1: every payout batch issues an invoice row so the affiliate can
-- (a) download a printable HTML invoice (browser → "Save as PDF"), and
-- (b) reconcile their accounting in a single place. The HTML pattern
-- mirrors `templates/pdf-tax-report.html` — no server-side PDF binary
-- needed for MVP; headless-Chrome / Typst rendering is Phase 4.
--
-- Each row corresponds to ONE (batch_id, affiliate_id) pair. Idempotent
-- creation: if a payout batch is retried, the same invoice row is
-- updated rather than duplicated.
--
-- Invoice numbers are sequential per calendar year, scoped globally
-- (NOT per-affiliate) so an accountant has a single monotonic register
-- to follow. Format: `POL-{YYYY}-{seq:06}` (e.g. `POL-2026-000017`).

CREATE TABLE IF NOT EXISTS affiliate_invoices (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number   VARCHAR(40) NOT NULL UNIQUE,
    -- Source payout batch + recipient. Together they're naturally unique
    -- but we keep an explicit partial unique below for idempotency.
    payout_batch_id  UUID NOT NULL REFERENCES payout_batches(id) ON DELETE RESTRICT,
    affiliate_id     UUID NOT NULL REFERENCES affiliates(user_id) ON DELETE RESTRICT,
    amount_cents     BIGINT NOT NULL CHECK (amount_cents >= 0),
    currency         CHAR(3) NOT NULL DEFAULT 'EUR' CHECK (currency ~ '^[A-Z]{3}$'),
    commission_count INTEGER NOT NULL CHECK (commission_count >= 0),
    issued_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Snapshot of recipient details at issue time. Email + name come
    -- from the user record; bank metadata from `developer_teams` if any.
    recipient_email      VARCHAR(255),
    recipient_full_name  VARCHAR(255),
    bank_account_holder  VARCHAR(255),
    bank_iban_last4      VARCHAR(4),
    bank_bic             VARCHAR(11),
    bank_country         VARCHAR(2),
    -- Optional GCS path for the rendered PDF once Phase 4 PDF generation
    -- lands. For now this column stays NULL and the printable HTML is
    -- generated on-demand at `/affiliate/invoices/:id`.
    pdf_gcs_path     TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Idempotency: one invoice per (batch, affiliate). A retried batch payout
-- finds the existing row via ON CONFLICT and updates it.
CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_invoices_batch_affiliate
    ON affiliate_invoices (payout_batch_id, affiliate_id);

CREATE INDEX IF NOT EXISTS idx_affiliate_invoices_affiliate_issued
    ON affiliate_invoices (affiliate_id, issued_at DESC);

-- Per-year sequence used by the `next_invoice_number` helper. Postgres
-- sequences are autonomous (don't roll back with a failed tx), which is
-- exactly what we want for a monotonic accounting register.
CREATE SEQUENCE IF NOT EXISTS affiliate_invoice_seq_2026 START 1;

-- Generate the next invoice number for the calling year. Caller is
-- responsible for selecting the right sequence (this helper resolves
-- automatically from `EXTRACT(YEAR FROM NOW())`).
CREATE OR REPLACE FUNCTION next_affiliate_invoice_number()
RETURNS TEXT
LANGUAGE plpgsql
AS $$
DECLARE
    year_part TEXT := to_char(NOW(), 'YYYY');
    seq_name  TEXT := 'affiliate_invoice_seq_' || year_part;
    seq_val   BIGINT;
BEGIN
    -- Lazy-create the sequence for new years so we don't ship a
    -- migration every January. Sequence is owned by the calling DB user.
    EXECUTE format('CREATE SEQUENCE IF NOT EXISTS %I START 1', seq_name);
    EXECUTE format('SELECT nextval(%L)', seq_name) INTO seq_val;
    RETURN format('POL-%s-%s', year_part, lpad(seq_val::text, 6, '0'));
END;
$$;

CREATE TRIGGER set_affiliate_invoices_updated_at
    BEFORE UPDATE ON affiliate_invoices
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

COMMENT ON TABLE affiliate_invoices IS
  'Phase-3 P1 affiliate payout invoice register. One row per (payout_batch_id, affiliate_id). Sequential invoice numbers per calendar year via next_affiliate_invoice_number().';
