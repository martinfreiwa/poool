-- 138 — Villa-Returns P1: per-asset deduction policy (PDF §4 "permitted expense deductions").
--
-- Defines which expense categories are contractually permitted per asset, with optional
-- caps. Append-only — every policy change inserts a new row with a new effective_from
-- date. The active policy at any moment T is: the row with the largest effective_from <= T.

CREATE TABLE IF NOT EXISTS villa_deduction_policy (
    id              BIGSERIAL PRIMARY KEY,
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    effective_from  DATE NOT NULL,
    allowed_codes   TEXT[] NOT NULL,
    per_category_cap_bps JSONB,
    notes           TEXT,
    set_by          UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, effective_from)
);

CREATE INDEX IF NOT EXISTS idx_vdp_asset_effective
    ON villa_deduction_policy (asset_id, effective_from DESC);

-- Append-only guard: never UPDATE or DELETE policies; correct by inserting a new effective_from.
CREATE OR REPLACE FUNCTION fn_villa_deduction_policy_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'villa_deduction_policy is append-only; insert a new row with a later effective_from instead (id=%)', OLD.id;
    END IF;
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'villa_deduction_policy is append-only; DELETE not permitted (id=%)', OLD.id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_villa_deduction_policy_guard ON villa_deduction_policy;
CREATE TRIGGER trg_villa_deduction_policy_guard
    BEFORE UPDATE OR DELETE ON villa_deduction_policy
    FOR EACH ROW EXECUTE FUNCTION fn_villa_deduction_policy_guard();

COMMENT ON TABLE villa_deduction_policy IS 'Per-asset whitelist of permitted expense categories with effective_from dates. Past published rows are unaffected by later policy changes.';
