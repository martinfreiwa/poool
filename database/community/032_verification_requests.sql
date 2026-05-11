-- Phase 3 task 32: verified-owner badge request flow.
--
-- Users submit a request to be marked as a "Verified Owner" of one of their
-- assets. The request carries a free-form statement plus an optional proof
-- image URL (uploaded via /api/upload/post-image). An admin reviews and
-- approves or rejects via /api/admin/community/verification-requests.

CREATE TABLE IF NOT EXISTS verification_requests (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    statement   TEXT NOT NULL,
    proof_url   TEXT,
    status      TEXT NOT NULL DEFAULT 'pending',
    admin_notes TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_verification_requests_user
    ON verification_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_verification_requests_pending
    ON verification_requests(status, created_at DESC)
    WHERE status = 'pending';

-- Verified-owner flag on posts, surfaced as the "Verified Owner" pill on
-- the post card. The admin-review handler bulk-flips this for every post
-- the approved user has authored.
ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS verified_owner BOOLEAN NOT NULL DEFAULT FALSE;
