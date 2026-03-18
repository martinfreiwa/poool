-- 043: Asset page view tracking for developer dashboard analytics
CREATE TABLE IF NOT EXISTS asset_views (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id    UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    viewed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_asset_views_asset_id ON asset_views(asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_views_user_id ON asset_views(user_id);
CREATE INDEX IF NOT EXISTS idx_asset_views_viewed_at ON asset_views(viewed_at);
