-- Module 5: Gamified Challenges & Progress

CREATE TABLE challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    xp_reward INTEGER NOT NULL DEFAULT 0,
    badge_reward VARCHAR(255),
    requirement_type VARCHAR(50) NOT NULL, -- 'buy_asset', 'write_review', 'login_streak', 'invite_friends', 'join_circle'
    requirement_value INTEGER NOT NULL DEFAULT 1,
    frequency VARCHAR(20) NOT NULL DEFAULT 'one_time', -- 'one_time', 'daily', 'weekly'
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE challenge_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    current_value INTEGER NOT NULL DEFAULT 0,
    is_completed BOOLEAN NOT NULL DEFAULT false,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, challenge_id)
);

CREATE INDEX idx_challenge_progress_user_id ON challenge_progress(user_id);
CREATE INDEX idx_challenges_active ON challenges(is_active);

-- Insert some default one-time challenges
INSERT INTO challenges (title, description, xp_reward, requirement_type, requirement_value, frequency)
VALUES 
    ('First Steps', 'Complete your KYC and verify your identity.', 100, 'kyc_approved', 1, 'one_time'),
    ('First Investment', 'Make your first property investment.', 250, 'buy_asset', 1, 'one_time'),
    ('Community Voice', 'Write a verified review for a property you own.', 50, 'write_review', 1, 'one_time'),
    ('Social Butterfly', 'Join a community circle.', 50, 'join_circle', 1, 'one_time'),
    ('Consistent Investor', 'Log in for 5 consecutive days.', 100, 'login_streak', 5, 'one_time');
