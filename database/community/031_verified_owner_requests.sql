-- 14.8.16 — User-initiated verified-owner requests.
--
-- Users submit a request to be marked as a verified owner of an asset they
-- hold. Admin (or moderation) reviews and approves/rejects. Approval flips
-- the user's review-eligibility status (downstream service decides exactly
-- where the "verified owner" flag lives — at minimum this request table is
-- the audit trail).

CREATE TABLE verified_owner_requests (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL,
    asset_id      UUID NOT NULL,
    status        VARCHAR(20) NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending', 'approved', 'rejected')),
    evidence_url  TEXT,
    note          TEXT,
    reviewed_at   TIMESTAMPTZ,
    reviewer_id   UUID,
    admin_notes   TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- One pending request per (user, asset). Approved or rejected rows can
    -- coexist so the audit trail is preserved.
    CONSTRAINT verified_owner_requests_one_pending UNIQUE (user_id, asset_id, status)
);

CREATE INDEX idx_verified_owner_requests_user ON verified_owner_requests (user_id);
CREATE INDEX idx_verified_owner_requests_status ON verified_owner_requests (status);
