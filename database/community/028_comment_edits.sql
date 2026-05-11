-- 14.8.5 — Comment editing.
--
-- Two new columns on the existing `comments` table:
--   edited_at         — non-NULL once a comment has been edited at least
--                       once. Frontend shows an "Edited" indicator on
--                       comment rows where this is non-NULL.
--   original_content  — preserves the comment's original text for
--                       moderation review. Captured on the first edit
--                       (subsequent edits leave it untouched).
--
-- No backfill needed: existing rows stay edited_at IS NULL.

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS edited_at TIMESTAMPTZ NULL,
    ADD COLUMN IF NOT EXISTS original_content TEXT NULL;
