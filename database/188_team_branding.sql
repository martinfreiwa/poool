-- Migration 188: per-team email branding (Phase 4).
--
-- Industry standard (Tapfiliate / PartnerStack): the team owner uploads a
-- logo + picks an accent color → the invite email and the public join
-- page render with the team's brand instead of generic POOOL chrome.
-- Increases acceptance rate because invitees see a familiar logo.
--
-- Columns added to `developer_teams`:
--   * `logo_url`           — HTTPS URL to a hosted logo PNG/SVG. Sanitised
--                            at write time (length cap + scheme check).
--                            NULL → fall back to the POOOL wordmark.
--   * `accent_color`       — hex string `#RRGGBB`. CHECK constraint enforces
--                            6-hex-digit pattern. NULL → fall back to
--                            `#0000FF` (Electric Blue).
--   * `email_from_display` — sender name override, e.g. "Acme Property".
--                            Length-capped to 80 chars to fit RFC 5322
--                            display-name limits.
--
-- All three are optional; existing teams continue to render the default
-- POOOL chrome until the team owner opts in via the settings page.

ALTER TABLE developer_teams
    ADD COLUMN IF NOT EXISTS logo_url            VARCHAR(512),
    ADD COLUMN IF NOT EXISTS accent_color        CHAR(7),
    ADD COLUMN IF NOT EXISTS email_from_display  VARCHAR(80);

-- CHECK is applied as a separate ALTER so the IF NOT EXISTS on the
-- column add doesn't blow up on a re-run that already added the column.
ALTER TABLE developer_teams
    DROP CONSTRAINT IF EXISTS developer_teams_accent_color_hex;
ALTER TABLE developer_teams
    ADD CONSTRAINT developer_teams_accent_color_hex
    CHECK (accent_color IS NULL OR accent_color ~ '^#[0-9A-Fa-f]{6}$');

COMMENT ON COLUMN developer_teams.logo_url IS
  'Phase-4: optional brand logo URL (HTTPS) shown in invite emails + public join page. NULL → POOOL wordmark fallback.';
COMMENT ON COLUMN developer_teams.accent_color IS
  'Phase-4: optional brand accent hex color (`#RRGGBB`). Used for the email CTA button + page accents. NULL → #0000FF default.';
COMMENT ON COLUMN developer_teams.email_from_display IS
  'Phase-4: optional RFC 5322 display name for invite emails (e.g. "Acme Property"). NULL → "POOOL".';
