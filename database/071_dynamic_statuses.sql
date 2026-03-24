-- Migration to remove hardcoded status CHECK constraints

-- Allow dynamic investment statuses
ALTER TABLE investments DROP CONSTRAINT IF EXISTS investments_status_check;

-- Allow dynamic asset funding statuses
ALTER TABLE assets DROP CONSTRAINT IF EXISTS assets_funding_status_check;
