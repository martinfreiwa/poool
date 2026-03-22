-- Module 5: Verified Property Reviews

CREATE TABLE asset_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL, -- Logical FK to poool.assets
    user_id UUID NOT NULL,  -- Logical FK to poool.users
    rating SMALLINT NOT NULL CHECK (rating >= 1 AND rating <= 5),
    content TEXT NOT NULL,
    is_owner BOOLEAN NOT NULL DEFAULT false,
    helpful_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(asset_id, user_id)
);

CREATE TABLE review_upvotes (
    review_id UUID NOT NULL REFERENCES asset_reviews(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (review_id, user_id)
);

CREATE INDEX idx_asset_reviews_asset_id ON asset_reviews(asset_id);
CREATE INDEX idx_asset_reviews_rating ON asset_reviews(rating);
