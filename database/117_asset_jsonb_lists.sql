-- Migration 117: Editable property-page info badges (the four-tile row right
-- under the property categories on the property detail page).
--
-- Stored as a JSONB array of objects so admins can add/remove/reorder badges
-- without further migrations. Each element has the shape:
--   {
--     "icon_url": "/static/images/prop-details/ID.webp",
--     "title":    "Indonesia, Bali",
--     "subtitle": "A mature real estate market with a high return on investment"
--   }
--
-- When NULL or empty, the property template renders the existing hardcoded
-- four badges (country, rented, yield, gross-yield) as a fallback.

ALTER TABLE assets
    ADD COLUMN IF NOT EXISTS info_badges    JSONB,
    ADD COLUMN IF NOT EXISTS leasing_items  JSONB;

COMMENT ON COLUMN assets.info_badges IS
    'Optional JSONB array of {icon_url,title,subtitle} objects rendered as info badges on the property detail page. NULL falls back to the legacy hardcoded badges.';
COMMENT ON COLUMN assets.leasing_items IS
    'Optional JSONB array of {title,description} objects rendered as the Leasing Strategy sub-cards. NULL falls back to the legacy hardcoded items.';
