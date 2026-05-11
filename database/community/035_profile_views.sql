-- WS3.1: track profile-page views so the analytics tab can show "X visited
-- your profile in the last 30 days". Anonymous viewers leave a NULL row so
-- we still capture traffic (the dashboard never exposes anonymous viewer
-- ids).

CREATE TABLE IF NOT EXISTS profile_views (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_user_id UUID NOT NULL,
    viewer_user_id  UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_profile_views_profile
    ON profile_views(profile_user_id, created_at DESC);
