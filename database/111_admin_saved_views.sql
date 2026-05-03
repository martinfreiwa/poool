-- 111_admin_saved_views.sql
-- Admin-scoped saved filter views for marketplace orders + other list pages.
--
-- One row per saved view per admin. `scope` partitions views by page
-- (e.g. 'marketplace_orders', 'marketplace_trades') so the same name can
-- coexist across pages.
--
-- The `preset` JSONB stores arbitrary filter state — frontend defines the
-- shape per scope (status / side / sort / order / q / etc.).

CREATE TABLE IF NOT EXISTS admin_saved_views (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope           TEXT NOT NULL,
    name            TEXT NOT NULL,
    preset          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT admin_saved_views_name_per_scope UNIQUE (user_id, scope, name)
);

CREATE INDEX IF NOT EXISTS idx_admin_saved_views_user_scope
    ON admin_saved_views (user_id, scope);

COMMENT ON TABLE  admin_saved_views IS 'Per-admin filter presets for list pages.';
COMMENT ON COLUMN admin_saved_views.scope  IS 'Page identifier (e.g. marketplace_orders).';
COMMENT ON COLUMN admin_saved_views.preset IS 'Arbitrary JSON filter state — frontend defines shape.';
