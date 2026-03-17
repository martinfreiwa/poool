-- ═══════════════════════════════════════════════════════════════════
-- Migration 024: Add display_name to leaderboard_preferences
-- ═══════════════════════════════════════════════════════════════════

ALTER TABLE leaderboard_preferences ADD COLUMN IF NOT EXISTS display_name VARCHAR(50) DEFAULT NULL;
