-- 149 — Villa-Returns C3: annual document link table.
--
-- Tax statements + annual reports (PDF §3) uploaded against a villa year.
-- Year-keyed sibling of villa_period_documents — annual documents have no
-- month, so they get their own table rather than a nullable period_month on
-- the monthly link table (keeps the monthly table's NOT NULL invariant clean).

CREATE TABLE IF NOT EXISTS villa_annual_documents (
    id              BIGSERIAL PRIMARY KEY,
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    period_year     INTEGER NOT NULL CHECK (period_year BETWEEN 2000 AND 2100),
    document_id     UUID NOT NULL REFERENCES asset_documents(id) ON DELETE CASCADE,
    doc_type        VARCHAR(40) NOT NULL,
    uploaded_by     UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (period_year, document_id)
);

CREATE INDEX IF NOT EXISTS idx_vad_asset_year
    ON villa_annual_documents (asset_id, period_year DESC);

COMMENT ON TABLE villa_annual_documents IS 'Link table joining asset_documents to a (asset_id, period_year, doc_type) for annual documents (tax statements, annual reports). Year-keyed sibling of villa_period_documents.';
