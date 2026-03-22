-- Module 3: Social Layer
-- M3-DB.1, M3-DB.2, M3-DB.3, M3-DB.4

-- 1. Alters to community_profiles (M3-DB.3)
-- Already added in 005_community_profiles.sql: bio, follower_count, following_count

-- 2. Follows Table (M3-DB.1)
CREATE TABLE follows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_id UUID NOT NULL,
    following_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT self_follow_check CHECK (follower_id != following_id),
    UNIQUE (follower_id, following_id)
);

CREATE INDEX idx_follows_follower ON follows(follower_id);
CREATE INDEX idx_follows_following ON follows(following_id);

-- 3. Badges (M3-DB.2)
CREATE TABLE badges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL, -- e.g., 'early_adopter', 'first_investment'
    name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    icon VARCHAR(20) NOT NULL, -- e.g. 🏅, 🐋, 🌲
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE user_badges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    badge_id UUID NOT NULL REFERENCES badges(id) ON DELETE CASCADE,
    earned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, badge_id)
);

CREATE INDEX idx_user_badges_user_id ON user_badges(user_id);

-- 4. Seed Badge Definitions (M3-DB.4)
INSERT INTO badges (code, name, description, icon, display_order) VALUES
('early_adopter', 'Early Adopter', 'Joined POOOL directly after Beta.', '🚀', 1),
('first_investment', 'First Timber', 'First investment in a Timber asset.', '🌲', 2),
('cocoa_expert', 'Cocoa Aficionado', 'Invested in at least 3 distinct Cocoa properties.', '🍫', 3),
('diversified', 'Diversified Portfolio', 'Holds >3 asset categories natively.', '🌍', 4),
('whale', 'Platinum Backer', 'Accumulated >$10k across active pools.', '🐋', 5),
('connector', 'Connector', 'Has successfully referred 5 investors.', '🤝', 6),
('scholar', 'Scholar', 'Passed the highest-score investor educational KYC tier.', '🎓', 7),
('dividend_king', 'Dividend Collector', 'Received over 10 separate payout events.', '👑', 8),
('yield_farmer', 'Yield Farmer', 'Secured 3 assets with consecutive >10% annual yields.', '🚜', 9),
('helpful_voice', 'Helpful Voice', 'Received 50 "Helpful" reactions on community posts.', '💡', 10),
('market_guru', 'Market Guru', 'Shared 10 Market Insights highly reacted to.', '📈', 11),
('1yr_anniversary', 'One Year Club', 'Active for exactly 1 full year on platform.', '🎂', 12),
('5yr_anniversary', 'Five Year Club', 'Solid POOOL supporter for over half a decade.', '🏛', 13),
('eco_warrior', 'Eco Warrior', 'Invested only in certified ESG-tier assets.', '♻️', 14),
('pioneer', 'Pioneer', 'Funded a pool while it was below 10% target.', '🧭', 15),
('community_mod', 'Moderator', 'Active community mod fixing broken links & spam.', '🛡', 16);
