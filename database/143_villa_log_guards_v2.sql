-- 143 — Villa-Returns P2: tighten append-only guard functions on the log tables.
--
-- The original guards in migrations 132/134/137 had a hole: an UPDATE that left
-- OLD.status = NEW.status = 'published' (no status flip) fell through to
-- RETURN NEW, allowing silent data mutation on already-published rows.
-- This migration replaces the guard functions so:
--   - Published rows may only transition to 'superseded'.
--   - In any case, when OLD.status = 'published' the data fields must be unchanged.
--   - Superseded rows remain fully immutable.
-- Idempotent via CREATE OR REPLACE FUNCTION; triggers already point at these names.

CREATE OR REPLACE FUNCTION fn_villa_operations_log_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'villa_operations_log is append-only; DELETE is not permitted (id=%, asset_id=%)', OLD.id, OLD.asset_id;
    END IF;

    IF OLD.status = 'published' THEN
        IF NEW.status <> 'superseded' THEN
            RAISE EXCEPTION 'published row can only transition to superseded (id=%, attempted status=%)', OLD.id, NEW.status;
        END IF;
        IF  NEW.gross_rental_idr_cents     IS DISTINCT FROM OLD.gross_rental_idr_cents
         OR NEW.distributable_idr_cents    IS DISTINCT FROM OLD.distributable_idr_cents
         OR NEW.net_rental_income_idr_cents IS DISTINCT FROM OLD.net_rental_income_idr_cents
         OR NEW.total_opex_idr_cents       IS DISTINCT FROM OLD.total_opex_idr_cents
         OR NEW.reserve_applied_idr_cents  IS DISTINCT FROM OLD.reserve_applied_idr_cents
         OR NEW.platform_fee_idr_cents     IS DISTINCT FROM OLD.platform_fee_idr_cents
         OR NEW.withholding_idr_cents      IS DISTINCT FROM OLD.withholding_idr_cents
         OR NEW.fx_rate_idr_to_usd_bps     IS DISTINCT FROM OLD.fx_rate_idr_to_usd_bps
         OR NEW.gross_rental_usd_cents     IS DISTINCT FROM OLD.gross_rental_usd_cents
         OR NEW.distributable_usd_cents    IS DISTINCT FROM OLD.distributable_usd_cents
         OR NEW.period_year                IS DISTINCT FROM OLD.period_year
         OR NEW.period_month               IS DISTINCT FROM OLD.period_month
         OR NEW.asset_id                   IS DISTINCT FROM OLD.asset_id
         OR NEW.published_at               IS DISTINCT FROM OLD.published_at
         OR NEW.correction_reason          IS DISTINCT FROM OLD.correction_reason
        THEN
            RAISE EXCEPTION 'published row data is immutable; only status flip to superseded is allowed (id=%)', OLD.id;
        END IF;
    END IF;

    IF OLD.status = 'superseded' THEN
        RAISE EXCEPTION 'superseded row is immutable (id=%, asset_id=%)', OLD.id, OLD.asset_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION fn_villa_valuations_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'villa_valuations is append-only; DELETE is not permitted (id=%, asset_id=%)', OLD.id, OLD.asset_id;
    END IF;

    IF OLD.status = 'published' THEN
        IF NEW.status <> 'superseded' THEN
            RAISE EXCEPTION 'published valuation can only transition to superseded (id=%, attempted status=%)', OLD.id, NEW.status;
        END IF;
        IF  NEW.valuation_idr_cents IS DISTINCT FROM OLD.valuation_idr_cents
         OR NEW.valuation_usd_cents IS DISTINCT FROM OLD.valuation_usd_cents
         OR NEW.valuation_date      IS DISTINCT FROM OLD.valuation_date
         OR NEW.valuation_method    IS DISTINCT FROM OLD.valuation_method
         OR NEW.asset_id            IS DISTINCT FROM OLD.asset_id
         OR NEW.published_at        IS DISTINCT FROM OLD.published_at
         OR NEW.appraiser_user_id   IS DISTINCT FROM OLD.appraiser_user_id
         OR NEW.evidence_doc_id     IS DISTINCT FROM OLD.evidence_doc_id
        THEN
            RAISE EXCEPTION 'published valuation data is immutable; only status flip to superseded is allowed (id=%)', OLD.id;
        END IF;
    END IF;

    IF OLD.status = 'superseded' THEN
        RAISE EXCEPTION 'superseded valuation is immutable (id=%)', OLD.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

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
        -- Whether staying approved or flipping to superseded, data must be unchanged.
        IF  NEW.amount_idr_cents IS DISTINCT FROM OLD.amount_idr_cents
         OR NEW.event_date       IS DISTINCT FROM OLD.event_date
         OR NEW.asset_id         IS DISTINCT FROM OLD.asset_id
         OR NEW.category         IS DISTINCT FROM OLD.category
        THEN
            RAISE EXCEPTION 'approved CapEx data is immutable; only status flip allowed (id=%)', OLD.id;
        END IF;
    END IF;

    IF OLD.status = 'superseded' THEN
        RAISE EXCEPTION 'superseded CapEx is immutable (id=%)', OLD.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
