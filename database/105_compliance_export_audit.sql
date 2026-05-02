-- Compliance Export Audit Log
-- Every regulatory CSV export (OJK, Travel-Rule, Tax) writes one row here.
-- Required for OJK/Bappebti regulator audits and to detect re-generations
-- where late-arriving trades change historical reports.

CREATE TABLE IF NOT EXISTS compliance_export_audit (
    id              BIGSERIAL PRIMARY KEY,
    export_type     TEXT        NOT NULL,         -- 'ojk_quarterly' | 'travel_rule' | 'tax_fiscal'
    period_label    TEXT        NOT NULL,         -- e.g. '2026-Q1', '2025-01-01..2025-12-31', 'FY2025'
    period_start    DATE,                          -- inclusive
    period_end      DATE,                          -- exclusive
    requested_by    UUID        NOT NULL REFERENCES users(id),
    requested_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    row_count       BIGINT      NOT NULL,
    byte_size       BIGINT      NOT NULL,
    content_sha256  TEXT        NOT NULL,         -- hex digest of CSV body
    submission_status TEXT      NOT NULL DEFAULT 'generated',
                                                  -- 'generated' | 'submitted' | 'superseded'
    submitted_at    TIMESTAMPTZ,
    submitted_by    UUID        REFERENCES users(id),
    notes           TEXT
);

CREATE INDEX IF NOT EXISTS idx_compliance_export_audit_type_period
    ON compliance_export_audit (export_type, period_label, requested_at DESC);

CREATE INDEX IF NOT EXISTS idx_compliance_export_audit_recent
    ON compliance_export_audit (requested_at DESC);
