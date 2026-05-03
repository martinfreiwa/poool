-- Migration 119: Editable risk-notification items for the property page.
--
-- The legacy `risk_notification` TEXT column holds a single newline-joined
-- string and renders as bare paragraphs. Operators want a structured list
-- with a title and body per row (matching the legacy hardcoded layout:
-- "Developer Issues:", "Natural Events:", "Investment Duration:" each with
-- their own paragraph). Stored as a JSONB array so admins can add/remove
-- entries without further migrations. Each element has the shape:
--   { "title": "Developer Issues", "body": "Unforeseen problems..." }
--
-- When NULL or empty, the template falls back to splitting the legacy
-- `risk_notification` TEXT field on newlines (and ultimately the hardcoded
-- three items if that is also empty).

ALTER TABLE assets
    ADD COLUMN IF NOT EXISTS risk_notification_items JSONB;

COMMENT ON COLUMN assets.risk_notification_items IS
    'Optional JSONB array of {title,body} objects rendered as the Risk Notification sub-items on the property detail page. NULL/empty falls back to the legacy risk_notification TEXT field, then the hardcoded three items.';
