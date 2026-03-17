-- Migration 017: Campaign & SubID Tracking for Affiliates

-- 1. Add subid columns to referral clicks and tracking
ALTER TABLE referral_clicks ADD COLUMN subid VARCHAR(255);
ALTER TABLE referral_tracking ADD COLUMN subid VARCHAR(255);

-- 2. Index for analytics filtering by campaign/subid
CREATE INDEX idx_referral_clicks_subid ON referral_clicks(subid);
CREATE INDEX idx_referral_tracking_subid ON referral_tracking(subid);
