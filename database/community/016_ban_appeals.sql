CREATE TABLE ban_appeals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    appeal_text TEXT NOT NULL CHECK (char_length(appeal_text) BETWEEN 10 AND 2000),
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    admin_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX idx_ban_appeals_status ON ban_appeals (status);
