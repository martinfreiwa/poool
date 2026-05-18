-- Adds an optional reporter-supplied note to content_reports.
-- Surfaced in the user-facing report modal (community-feed Report Post),
-- read-only context for moderators alongside admin_notes.
ALTER TABLE content_reports
    ADD COLUMN IF NOT EXISTS reporter_note TEXT;
