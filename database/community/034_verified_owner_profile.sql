-- WS1.3: denormalize verified-owner status onto the profile so newly created
-- posts inherit the flag automatically without re-running the
-- admin_review_verification_request bulk update.

ALTER TABLE community_profiles
    ADD COLUMN IF NOT EXISTS is_verified_owner BOOLEAN NOT NULL DEFAULT FALSE;

-- Backfill from any previously approved requests.
UPDATE community_profiles cp
SET is_verified_owner = TRUE
WHERE EXISTS (
    SELECT 1 FROM verification_requests vr
    WHERE vr.user_id = cp.user_id AND vr.status = 'approved'
);
