CREATE TABLE circle_join_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, accepted, declined
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_circle_join_requests_unique ON circle_join_requests(circle_id, user_id) WHERE status = 'pending';
