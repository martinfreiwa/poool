-- Add profile and tax data columns to the affiliates table for the onboarding wizard

ALTER TABLE affiliates 
ADD COLUMN traffic_source VARCHAR(50),
ADD COLUMN audience_size VARCHAR(50),
ADD COLUMN main_url TEXT,
ADD COLUMN phone_number VARCHAR(50),
ADD COLUMN tax_id VARCHAR(50),
ADD COLUMN company_name VARCHAR(255);

ALTER TABLE affiliate_referrals
ADD COLUMN sub_id VARCHAR(100),
ADD COLUMN utm_source VARCHAR(100);
