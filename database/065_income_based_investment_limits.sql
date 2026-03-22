-- Layer: Phase 17.2 — Income-Based Investment Limits
-- This migration adds the annual_income_cents field to user_profiles 
-- and ensures every user has an entry in the investment_limits table.

-- 1. Add annual_income_cents to user_profiles
ALTER TABLE user_profiles ADD COLUMN IF NOT EXISTS annual_income_cents BIGINT;

-- 2. Create a function to initialize or update investment limits based on income
CREATE OR REPLACE FUNCTION update_user_investment_limit()
RETURNS TRIGGER AS $$
DECLARE
    income_val BIGINT;
    limit_cents BIGINT;
    current_yr INTEGER;
BEGIN
    income_val := NEW.annual_income_cents;
    current_yr := EXTRACT(YEAR FROM NOW())::INTEGER;

    -- If income <= $50,000 (5,000,000 cents): Limit is 5% of income
    -- If income > $50,000: Limit is 10% of income
    -- Default/Minimum limit if income is NULL: $2,500
    
    IF income_val IS NULL OR income_val = 0 THEN
        limit_cents := 250000; -- $2,500.00
    ELSIF income_val <= 5000000 THEN
        limit_cents := (income_val * 5) / 100;
    ELSE
        limit_cents := (income_val * 10) / 100;
    END IF;

    -- Upsert the limit for the current year
    INSERT INTO investment_limits (user_id, annual_limit_cents, limit_year)
    VALUES (NEW.user_id, limit_cents, current_yr)
    ON CONFLICT (user_id, limit_year) DO UPDATE
    SET annual_limit_cents = limit_cents, updated_at = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 3. Trigger on user_profiles
DROP TRIGGER IF EXISTS trg_update_investment_limit ON user_profiles;
CREATE TRIGGER trg_update_investment_limit
AFTER INSERT OR UPDATE OF annual_income_cents ON user_profiles
FOR EACH ROW EXECUTE FUNCTION update_user_investment_limit();

-- 4. Initial backfill for existing profiles
DO $$
BEGIN
    -- This will trigger the function for all existing users
    UPDATE user_profiles SET updated_at = NOW() WHERE annual_income_cents IS NULL;
END $$;
