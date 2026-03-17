-- Migration 026: Add is_2fa_verified column to user_sessions
-- This was previously an inline HOTFIX in main.rs — now a proper migration.
ALTER TABLE user_sessions ADD COLUMN IF NOT EXISTS is_2fa_verified BOOLEAN NOT NULL DEFAULT FALSE;
