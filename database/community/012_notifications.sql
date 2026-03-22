-- Module 5: In-App Notifications

CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    actor_id UUID, -- NULL if triggered by the platform
    type VARCHAR(50) NOT NULL, -- e.g. 'new_follower', 'post_like', 'comment_reply', 'level_up', 'ama_answer', 'challenge_completed'
    entity_id UUID,            -- e.g. post_id, comment_id, or ama_id
    content TEXT NOT NULL,     -- e.g. "Martin liked your post" or "You leveled up to Seedling!"
    link_url TEXT,             -- Optional direct link to the content (e.g., /community/feed?post=123)
    is_read BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id) WHERE is_read = false;
