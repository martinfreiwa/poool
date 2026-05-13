-- 139 — Villa-Returns P1: link asset_documents to a specific operations period.
--
-- Receipts, invoices, bank statements (PDF §2) uploaded against a monthly log row.
-- Multiple documents per (asset, period, doc_type) allowed.

CREATE TABLE IF NOT EXISTS villa_period_documents (
    id                  BIGSERIAL PRIMARY KEY,
    asset_id            UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    period_year         INTEGER NOT NULL CHECK (period_year BETWEEN 2000 AND 2100),
    period_month        INTEGER NOT NULL CHECK (period_month BETWEEN 1 AND 12),
    log_id              BIGINT REFERENCES villa_operations_log(id),
    document_id         UUID NOT NULL REFERENCES asset_documents(id) ON DELETE CASCADE,
    doc_type            VARCHAR(40) NOT NULL,
    uploaded_by         UUID REFERENCES users(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (period_year, period_month, document_id)
);

CREATE INDEX IF NOT EXISTS idx_vpd_asset_period
    ON villa_period_documents (asset_id, period_year DESC, period_month DESC);

CREATE INDEX IF NOT EXISTS idx_vpd_log
    ON villa_period_documents (log_id) WHERE log_id IS NOT NULL;

COMMENT ON TABLE villa_period_documents IS 'Link table joining asset_documents to a (asset_id, period_year, period_month, doc_type). Surfaces period-grouped documents on property.html.';
