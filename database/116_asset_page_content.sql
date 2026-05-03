-- Migration 116: Editable property-page content fields.
--
-- Adds CMS-style fields to `assets` so admins can populate the property detail
-- page sections currently hardcoded in `frontend/platform/property.html` and
-- its components (investment type, leasing strategy, calculator defaults,
-- developer card, risk notification).
--
-- All columns are nullable; the property page renders existing hardcoded
-- copy as a fallback when the field is NULL.

ALTER TABLE assets
    -- Investment Type section
    ADD COLUMN IF NOT EXISTS investment_type             VARCHAR(50),
    ADD COLUMN IF NOT EXISTS investment_type_description TEXT,

    -- Leasing Strategy section
    ADD COLUMN IF NOT EXISTS leasing_strategy_type        VARCHAR(50),
    ADD COLUMN IF NOT EXISTS leasing_strategy_description TEXT,

    -- Risk Notification section
    ADD COLUMN IF NOT EXISTS risk_notification TEXT,

    -- Investment Calculator defaults (sliders + chart seed values)
    ADD COLUMN IF NOT EXISTS default_investment_amount_cents BIGINT,
    ADD COLUMN IF NOT EXISTS default_value_growth_bps        INTEGER,
    ADD COLUMN IF NOT EXISTS default_rental_yield_bps        INTEGER,

    -- Property Developer card (per-asset, not the developer user profile)
    ADD COLUMN IF NOT EXISTS developer_logo_url    VARCHAR(512),
    ADD COLUMN IF NOT EXISTS developer_name        VARCHAR(255),
    ADD COLUMN IF NOT EXISTS developer_description TEXT,
    ADD COLUMN IF NOT EXISTS developer_website     VARCHAR(512),
    ADD COLUMN IF NOT EXISTS developer_facebook    VARCHAR(512),
    ADD COLUMN IF NOT EXISTS developer_instagram   VARCHAR(512),
    ADD COLUMN IF NOT EXISTS developer_youtube     VARCHAR(512);

COMMENT ON COLUMN assets.investment_type IS
    'Short label for property investment type, e.g. "Full ownership", "Fractional", "Buy-to-let".';
COMMENT ON COLUMN assets.investment_type_description IS
    'Long-form description shown in the Investment Type section of the property page.';
COMMENT ON COLUMN assets.leasing_strategy_type IS
    'Short label for leasing strategy, e.g. "Long-term rental", "Short-term/Airbnb", "Mixed".';
COMMENT ON COLUMN assets.leasing_strategy_description IS
    'Long-form description shown in the Leasing Strategy section of the property page.';
COMMENT ON COLUMN assets.risk_notification IS
    'Asset-specific risk disclosure shown in the Risk Notification section.';
COMMENT ON COLUMN assets.default_investment_amount_cents IS
    'Default value for the Investment Calculator amount slider, in USD cents.';
COMMENT ON COLUMN assets.default_value_growth_bps IS
    'Default annual property value growth for the calculator, in basis points (1000 = 10%).';
COMMENT ON COLUMN assets.default_rental_yield_bps IS
    'Default annual rental yield for the calculator, in basis points. Falls back to annual_yield_bps when NULL.';
COMMENT ON COLUMN assets.developer_logo_url IS
    'Logo image URL for the Property Developer card on the property page.';
