-- 134 — Villa-Returns P1: append-only annual valuations (PDF §3).
--
-- Admin/Valuer responsibility. Each valuation is a discrete event with appraiser
-- evidence (linked to asset_documents). NAV recomputation happens off this table.

CREATE TABLE IF NOT EXISTS villa_valuations (
    id                  BIGSERIAL PRIMARY KEY,
    asset_id            UUID NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    valuation_date      DATE NOT NULL,
    valuation_idr_cents BIGINT  NOT NULL CHECK (valuation_idr_cents > 0),
    valuation_usd_cents BIGINT  NOT NULL DEFAULT 0,
    currency_code       CHAR(3) NOT NULL DEFAULT 'IDR',
    fx_rate_idr_to_usd_bps INTEGER NOT NULL DEFAULT 1 CHECK (fx_rate_idr_to_usd_bps > 0),
    valuation_method    VARCHAR(50) NOT NULL
        CHECK (valuation_method IN ('sales_comparison','income','cost','external_appraisal','other')),
    appraiser_name      VARCHAR(200),
    appraiser_user_id   UUID REFERENCES users(id),
    comparables         JSONB,
    notes               TEXT,
    evidence_doc_id     UUID REFERENCES asset_documents(id),

    status              VARCHAR(20) NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft','submitted','approved','published','superseded','rejected')),
    supersedes_id       BIGINT REFERENCES villa_valuations(id),
    correction_reason   TEXT,
    submitted_by        UUID REFERENCES users(id),
    submitted_at        TIMESTAMPTZ,
    approved_by         UUID REFERENCES users(id),
    approved_at         TIMESTAMPTZ,
    rejected_reason     TEXT,
    rejected_at         TIMESTAMPTZ,
    published_at        TIMESTAMPTZ,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT vv_approver_differs CHECK (approved_by IS NULL OR approved_by <> submitted_by),
    CONSTRAINT vv_correction_reason_required
        CHECK (supersedes_id IS NULL OR (correction_reason IS NOT NULL AND length(correction_reason) > 0))
);

CREATE INDEX IF NOT EXISTS idx_vv_asset_valuation_date
    ON villa_valuations (asset_id, valuation_date DESC, recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_vv_pending_review
    ON villa_valuations (status, submitted_at) WHERE status IN ('submitted','approved');

CREATE INDEX IF NOT EXISTS idx_vv_supersedes
    ON villa_valuations (supersedes_id) WHERE supersedes_id IS NOT NULL;

-- Same append-only guard as villa_operations_log.
CREATE OR REPLACE FUNCTION fn_villa_valuations_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'villa_valuations is append-only; DELETE is not permitted (id=%, asset_id=%)', OLD.id, OLD.asset_id;
    END IF;

    IF OLD.status = 'published' THEN
        IF NEW.status NOT IN ('published','superseded') THEN
            RAISE EXCEPTION 'published valuation cannot revert to status=% (id=%)', NEW.status, OLD.id;
        END IF;
        IF NEW.status = 'superseded' THEN
            IF  NEW.valuation_idr_cents  IS DISTINCT FROM OLD.valuation_idr_cents
             OR NEW.valuation_date       IS DISTINCT FROM OLD.valuation_date
             OR NEW.asset_id             IS DISTINCT FROM OLD.asset_id
             OR NEW.published_at         IS DISTINCT FROM OLD.published_at
            THEN
                RAISE EXCEPTION 'published valuation data is immutable; only status flip to superseded is allowed (id=%)', OLD.id;
            END IF;
        END IF;
    END IF;

    IF OLD.status = 'superseded' THEN
        RAISE EXCEPTION 'superseded valuation is immutable (id=%)', OLD.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_villa_valuations_guard ON villa_valuations;
CREATE TRIGGER trg_villa_valuations_guard
    BEFORE UPDATE OR DELETE ON villa_valuations
    FOR EACH ROW EXECUTE FUNCTION fn_villa_valuations_guard();

COMMENT ON TABLE villa_valuations IS 'Append-only annual valuations (PDF §3). NAV = (valuation_idr_cents * assets.tokenized_pct_bps / 10000) / (assets.tokens_total - assets.tokens_owner_retained) per PDF §7.';
