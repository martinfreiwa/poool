-- CO.2: custom circle banner. URL points at the GCS-uploaded image
-- (or the local fallback during dev). NULL = use the default decorative
-- background defined in CSS.

ALTER TABLE circles
    ADD COLUMN IF NOT EXISTS banner_url TEXT NULL;
