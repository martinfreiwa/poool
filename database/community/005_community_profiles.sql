CREATE TABLE community_profiles (
    user_id         UUID PRIMARY KEY,    -- 1:1 mit Core-DB users.id
    bio             TEXT CHECK (char_length(bio) <= 300),
    is_community_banned BOOLEAN NOT NULL DEFAULT false,
    ban_reason      TEXT,
    ban_expires_at  TIMESTAMPTZ,
    warning_count   INTEGER NOT NULL DEFAULT 0,
    post_count      INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert
    follower_count  INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert
    following_count INTEGER NOT NULL DEFAULT 0,     -- Denormalisiert
    login_streak    INTEGER NOT NULL DEFAULT 0,      -- Aktuelle Login-Streak in Tagen
    last_login_date DATE,                            -- Letzter Login-Tag (für Streak)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
