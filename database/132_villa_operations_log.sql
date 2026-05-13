-- 132 — Villa-Returns P1: append-only monthly operations log (PDF §2).
--
-- Time-travel design (see drafts/villa-returns-implementation-plan.md §3.1):
--   - Rows are append-only for data fields. Pre-publish, status may flip
--     (draft → submitted → approved → published) on the same row.
--   - Once status='published', the only permitted change is status='superseded'
--     (enforced by trigger). Corrections insert a NEW row with supersedes_id set.
--   - All monetary values stored as BIGINT cents. IDR is native (Q1).
--     USD derived at publish using fx_rate_idr_to_usd_bps frozen with the row.
-- Q5 lock-in: approver_differs CHECK enforces 4-eyes.

CREATE TABLE IF NOT EXISTS villa_operations_log (
    id                              BIGSERIAL PRIMARY KEY,
    asset_id                        UUID    NOT NULL REFERENCES assets(id) ON DELETE RESTRICT,
    period_year                     INTEGER NOT NULL CHECK (period_year BETWEEN 2000 AND 2100),
    period_month                    INTEGER NOT NULL CHECK (period_month BETWEEN 1 AND 12),

    -- Rental + occupancy (PDF §2)
    gross_rental_idr_cents          BIGINT  NOT NULL DEFAULT 0 CHECK (gross_rental_idr_cents >= 0),
    currency_code                   CHAR(3) NOT NULL DEFAULT 'IDR',
    fx_rate_idr_to_usd_bps          INTEGER NOT NULL DEFAULT 1
                                    CHECK (fx_rate_idr_to_usd_bps > 0),
    gross_rental_usd_cents          BIGINT  NOT NULL DEFAULT 0,
    nights_available                INTEGER NOT NULL DEFAULT 0
                                    CHECK (nights_available BETWEEN 0 AND 31),
    nights_booked                   INTEGER NOT NULL DEFAULT 0
                                    CHECK (nights_booked BETWEEN 0 AND 31),
    occupancy_bps                   INTEGER GENERATED ALWAYS AS
                                      (CASE WHEN nights_available > 0
                                            THEN (nights_booked * 10000) / nights_available
                                            ELSE 0 END) STORED,
    adr_idr_cents                   BIGINT  GENERATED ALWAYS AS
                                      (CASE WHEN nights_booked > 0
                                            THEN gross_rental_idr_cents / nights_booked
                                            ELSE 0 END) STORED,
    adr_usd_cents                   BIGINT  GENERATED ALWAYS AS
                                      (CASE WHEN nights_booked > 0
                                            THEN gross_rental_usd_cents / nights_booked
                                            ELSE 0 END) STORED,

    -- Expense breakdown (IDR-native, per PDF §2)
    expense_cleaning_idr_cents      BIGINT  NOT NULL DEFAULT 0 CHECK (expense_cleaning_idr_cents >= 0),
    expense_maintenance_idr_cents   BIGINT  NOT NULL DEFAULT 0 CHECK (expense_maintenance_idr_cents >= 0),
    expense_utilities_idr_cents     BIGINT  NOT NULL DEFAULT 0 CHECK (expense_utilities_idr_cents >= 0),
    expense_staff_idr_cents         BIGINT  NOT NULL DEFAULT 0 CHECK (expense_staff_idr_cents >= 0),
    expense_pool_garden_idr_cents   BIGINT  NOT NULL DEFAULT 0 CHECK (expense_pool_garden_idr_cents >= 0),
    expense_pest_idr_cents          BIGINT  NOT NULL DEFAULT 0 CHECK (expense_pest_idr_cents >= 0),
    expense_other_idr_cents         BIGINT  NOT NULL DEFAULT 0 CHECK (expense_other_idr_cents >= 0),
    ota_fees_idr_cents              BIGINT  NOT NULL DEFAULT 0 CHECK (ota_fees_idr_cents >= 0),
    payment_fees_idr_cents          BIGINT  NOT NULL DEFAULT 0 CHECK (payment_fees_idr_cents >= 0),
    refunds_idr_cents               BIGINT  NOT NULL DEFAULT 0 CHECK (refunds_idr_cents >= 0),
    mgmt_fee_idr_cents              BIGINT  NOT NULL DEFAULT 0 CHECK (mgmt_fee_idr_cents >= 0),

    -- Derived totals (IDR + USD, frozen at publish)
    total_opex_idr_cents            BIGINT  NOT NULL DEFAULT 0,
    total_opex_usd_cents            BIGINT  NOT NULL DEFAULT 0,
    net_rental_income_idr_cents     BIGINT  NOT NULL DEFAULT 0,
    net_rental_income_usd_cents     BIGINT  NOT NULL DEFAULT 0,
    reserve_override_idr_cents      BIGINT,
    reserve_applied_idr_cents       BIGINT  NOT NULL DEFAULT 0 CHECK (reserve_applied_idr_cents >= 0),
    platform_fee_idr_cents          BIGINT  NOT NULL DEFAULT 0 CHECK (platform_fee_idr_cents >= 0),
    withholding_idr_cents           BIGINT  NOT NULL DEFAULT 0 CHECK (withholding_idr_cents >= 0),
    distributable_idr_cents         BIGINT  NOT NULL DEFAULT 0,
    distributable_usd_cents         BIGINT  NOT NULL DEFAULT 0,

    -- Workflow + versioning
    status                          VARCHAR(20) NOT NULL DEFAULT 'draft'
                                    CHECK (status IN ('draft','submitted','approved','published','superseded')),
    supersedes_id                   BIGINT REFERENCES villa_operations_log(id),
    correction_reason               TEXT,
    submitted_by                    UUID REFERENCES users(id),
    submitted_at                    TIMESTAMPTZ,
    approved_by                     UUID REFERENCES users(id),
    approved_at                     TIMESTAMPTZ,
    rejected_reason                 TEXT,
    rejected_at                     TIMESTAMPTZ,
    published_at                    TIMESTAMPTZ,
    recorded_at                     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT vol_approver_differs CHECK (approved_by IS NULL OR approved_by <> submitted_by),
    CONSTRAINT vol_correction_reason_required
        CHECK (supersedes_id IS NULL OR (correction_reason IS NOT NULL AND length(correction_reason) > 0))
);

-- Hot path: as-of read for a single (asset, period). Returns latest row recorded <= as_of.
CREATE INDEX IF NOT EXISTS idx_vol_asset_period_recorded
    ON villa_operations_log (asset_id, period_year DESC, period_month DESC, recorded_at DESC);

-- Approvals queue.
CREATE INDEX IF NOT EXISTS idx_vol_pending_review
    ON villa_operations_log (status, submitted_at) WHERE status IN ('submitted','approved');

-- Supersession chain navigation.
CREATE INDEX IF NOT EXISTS idx_vol_supersedes
    ON villa_operations_log (supersedes_id) WHERE supersedes_id IS NOT NULL;

-- Append-only enforcement: after publish, only status flips to 'superseded' are allowed.
CREATE OR REPLACE FUNCTION fn_villa_operations_log_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'villa_operations_log is append-only; DELETE is not permitted (id=%, asset_id=%)', OLD.id, OLD.asset_id;
    END IF;

    IF OLD.status = 'published' THEN
        IF NEW.status NOT IN ('published','superseded') THEN
            RAISE EXCEPTION 'published row cannot revert to status=% (id=%, asset_id=%)', NEW.status, OLD.id, OLD.asset_id;
        END IF;
        IF NEW.status = 'superseded' THEN
            -- Allow only the status flip; reject any other column drift.
            IF  NEW.gross_rental_idr_cents     IS DISTINCT FROM OLD.gross_rental_idr_cents
             OR NEW.distributable_idr_cents    IS DISTINCT FROM OLD.distributable_idr_cents
             OR NEW.net_rental_income_idr_cents IS DISTINCT FROM OLD.net_rental_income_idr_cents
             OR NEW.period_year                IS DISTINCT FROM OLD.period_year
             OR NEW.period_month               IS DISTINCT FROM OLD.period_month
             OR NEW.asset_id                   IS DISTINCT FROM OLD.asset_id
             OR NEW.published_at               IS DISTINCT FROM OLD.published_at
            THEN
                RAISE EXCEPTION 'published row data is immutable; only status flip to superseded is allowed (id=%)', OLD.id;
            END IF;
        END IF;
    END IF;

    -- superseded rows are fully frozen
    IF OLD.status = 'superseded' THEN
        RAISE EXCEPTION 'superseded row is immutable (id=%, asset_id=%)', OLD.id, OLD.asset_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_villa_operations_log_guard ON villa_operations_log;
CREATE TRIGGER trg_villa_operations_log_guard
    BEFORE UPDATE OR DELETE ON villa_operations_log
    FOR EACH ROW EXECUTE FUNCTION fn_villa_operations_log_guard();

COMMENT ON TABLE villa_operations_log IS 'Append-only monthly operations log for villas (PDF §2). Status flips up to publish are allowed on the same row; after publish the row is immutable and corrections insert a new row with supersedes_id set.';
