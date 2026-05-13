-- 137 — Villa-Returns P1: CapEx events tracked separately from monthly OpEx (PDF §3, §9).
--
-- Q8 lock-in: CapEx never reduces a single month's distributable. Approved CapEx
-- surfaces to admin on the next valuation entry (B2) as "Approved CapEx since
-- last valuation: $X" — admin uses it as an input but not as an automatic delta.

CREATE TABLE IF NOT EXISTS villa_capex_events (
    id                  BIGSERIAL PRIMARY KEY,
    asset_id            UUID    NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    event_date          DATE    NOT NULL,
    amount_idr_cents    BIGINT  NOT NULL CHECK (amount_idr_cents > 0),
    amount_usd_cents    BIGINT  NOT NULL DEFAULT 0,
    currency_code       CHAR(3) NOT NULL DEFAULT 'IDR',
    fx_rate_idr_to_usd_bps INTEGER NOT NULL DEFAULT 1 CHECK (fx_rate_idr_to_usd_bps > 0),
    category            VARCHAR(40) NOT NULL,
    description         TEXT    NOT NULL,
    evidence_doc_id     UUID REFERENCES asset_documents(id),

    status              VARCHAR(20) NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft','submitted','approved','rejected','superseded')),
    supersedes_id       BIGINT REFERENCES villa_capex_events(id),
    correction_reason   TEXT,
    submitted_by        UUID REFERENCES users(id),
    submitted_at        TIMESTAMPTZ,
    approved_by         UUID REFERENCES users(id),
    approved_at         TIMESTAMPTZ,
    rejected_reason     TEXT,
    rejected_at         TIMESTAMPTZ,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT vce_approver_differs CHECK (approved_by IS NULL OR approved_by <> submitted_by)
);

CREATE INDEX IF NOT EXISTS idx_vce_asset_date
    ON villa_capex_events (asset_id, event_date DESC);

CREATE INDEX IF NOT EXISTS idx_vce_pending
    ON villa_capex_events (status, submitted_at) WHERE status = 'submitted';

-- After-approve immutability: approved CapEx events can only be superseded, not edited.
CREATE OR REPLACE FUNCTION fn_villa_capex_events_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'villa_capex_events is append-only; DELETE not permitted (id=%)', OLD.id;
    END IF;

    IF OLD.status = 'approved' THEN
        IF NEW.status NOT IN ('approved','superseded') THEN
            RAISE EXCEPTION 'approved CapEx cannot revert to status=% (id=%)', NEW.status, OLD.id;
        END IF;
        IF NEW.status = 'superseded' THEN
            IF  NEW.amount_idr_cents IS DISTINCT FROM OLD.amount_idr_cents
             OR NEW.event_date       IS DISTINCT FROM OLD.event_date
             OR NEW.asset_id         IS DISTINCT FROM OLD.asset_id
            THEN
                RAISE EXCEPTION 'approved CapEx data is immutable; only status flip allowed (id=%)', OLD.id;
            END IF;
        END IF;
    END IF;

    IF OLD.status = 'superseded' THEN
        RAISE EXCEPTION 'superseded CapEx is immutable (id=%)', OLD.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_villa_capex_events_guard ON villa_capex_events;
CREATE TRIGGER trg_villa_capex_events_guard
    BEFORE UPDATE OR DELETE ON villa_capex_events
    FOR EACH ROW EXECUTE FUNCTION fn_villa_capex_events_guard();

COMMENT ON TABLE villa_capex_events IS 'CapEx events kept separate from monthly OpEx so they never silently reduce monthly payouts. Q8 lock-in.';
