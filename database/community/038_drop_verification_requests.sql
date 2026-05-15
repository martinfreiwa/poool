-- 2026-05-15: drop the legacy statement-only verification_requests table.
--
-- The admin UI never read from this table — every user submission was
-- silently orphaned. The asset-linked `verified_owner_requests` table
-- (added by 14.8.16) is the canonical flow and has its own admin review
-- pipeline that flips `community_profiles.is_verified_owner` directly.
--
-- Backfill of `is_verified_owner` from any historical approved rows in
-- this table was already done by migration 034, so no data is lost.

DROP TABLE IF EXISTS verification_requests CASCADE;
