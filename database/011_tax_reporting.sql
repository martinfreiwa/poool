-- Table for automated tax reporting, P&L, and capital gains
CREATE TABLE tax_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    fiscal_year INT NOT NULL,
    total_investment_cents BIGINT NOT NULL DEFAULT 0,
    total_dividends_cents BIGINT NOT NULL DEFAULT 0,
    capital_gains_cents BIGINT NOT NULL DEFAULT 0,
    withholding_tax_cents BIGINT NOT NULL DEFAULT 0,
    pdf_url TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, generated, failed
    generated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Ensure a single report per user per fiscal year
CREATE UNIQUE INDEX idx_tax_reports_user_year ON tax_reports(user_id, fiscal_year);
CREATE INDEX idx_tax_reports_user ON tax_reports(user_id);
