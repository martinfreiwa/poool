-- Migration 186: server-side request fingerprint on clicks + attributions.
--
-- Phase-3 P1: enable fraud detection beyond IP. The audit flagged that
-- a single user with a clean VPN can spin up many fake referrals because
-- the only signal is `referral_clicks.ip_address`.
--
-- We add a 64-char SHA-256 hex column to both:
--   * `referral_clicks`    — snapshot at landing time
--   * `affiliate_referrals` — snapshot at signup-time attribution
--
-- Mismatch between the two (same click but different fingerprint at
-- signup) is an actionable fraud signal — a real human keeps the same
-- browser between landing and signup.
--
-- The hash is computed in Rust from User-Agent + Accept-Language + the
-- first three octets of the IP. Not cryptographically PII-safe (it's
-- reversible if the original headers are known) but indexable + tiny
-- enough to scan billions of rows efficiently.

-- referral_clicks is partitioned; ALTER on the parent cascades to
-- existing partitions automatically.
ALTER TABLE referral_clicks
    ADD COLUMN IF NOT EXISTS fingerprint_hash CHAR(64);

-- Used by the (future) anomaly scanner: "give me clicks where the
-- fingerprint is shared across N distinct codes in a 24h window".
CREATE INDEX IF NOT EXISTS idx_referral_clicks_fingerprint
    ON referral_clicks (fingerprint_hash, created_at DESC)
    WHERE fingerprint_hash IS NOT NULL;

ALTER TABLE affiliate_referrals
    ADD COLUMN IF NOT EXISTS click_fingerprint_hash CHAR(64);

ALTER TABLE affiliate_referrals
    ADD COLUMN IF NOT EXISTS signup_fingerprint_hash CHAR(64);

CREATE INDEX IF NOT EXISTS idx_affiliate_referrals_fingerprint
    ON affiliate_referrals (signup_fingerprint_hash)
    WHERE signup_fingerprint_hash IS NOT NULL;

COMMENT ON COLUMN referral_clicks.fingerprint_hash IS
  'Phase-3 P1: SHA-256 hex of user-agent || accept-language || ip /24.';
COMMENT ON COLUMN affiliate_referrals.click_fingerprint_hash IS
  'Phase-3 P1: copy of the originating click''s fingerprint.';
COMMENT ON COLUMN affiliate_referrals.signup_fingerprint_hash IS
  'Phase-3 P1: fingerprint recorded at signup-time attribution. If different from click_fingerprint_hash it''s a fraud signal.';
