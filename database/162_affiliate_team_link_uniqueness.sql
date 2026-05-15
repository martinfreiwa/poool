-- 162: Partial UNIQUE for affiliate_links to close the race-condition gap.
--
-- Audit P0 #6: `create_team_business_link` and `create_personal_link` do an
-- idempotency-check + INSERT on separate queries with no row lock. Two
-- concurrent callers (double-click magic-link, retried POST) can both pass
-- the existence check and create duplicate active links for the same
-- (team, member) pair — fanning commission rollups.
--
-- Fix: DB-level partial-unique indices. Rely on them as the source of
-- truth; application can pre-check for UX but the DB guarantees no dupes.

BEGIN;

-- Personal: one active personal link per attribution user.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_personal_link_per_user
    ON affiliate_links (attribution_user_id)
    WHERE link_type = 'personal' AND status = 'active';

-- Team-business: one active business link per (team_id, attribution_user_id).
CREATE UNIQUE INDEX IF NOT EXISTS uniq_team_business_link_per_member
    ON affiliate_links (team_id, attribution_user_id)
    WHERE link_type = 'team_business' AND status = 'active';

COMMIT;
