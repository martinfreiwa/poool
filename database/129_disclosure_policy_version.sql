-- 19.9 — Add policy_version to investment_disclosures_log.
--
-- The masterplan calls out timestamp, IP, AND policy version as the three
-- immutable fields. The first two were already stored; this adds the version
-- column so we can prove which disclosure text the user actually agreed to
-- when a policy is updated. Defaults to '1.0' for backfilled rows so the
-- existing log retains a stable identifier instead of NULL.

ALTER TABLE investment_disclosures_log
    ADD COLUMN IF NOT EXISTS policy_version VARCHAR(20) NOT NULL DEFAULT '1.0';

-- Seed the canonical version in platform_settings so it can be bumped without
-- a code change. The checkout flow reads this key when inserting a new row.
INSERT INTO platform_settings (key, value, value_type, description)
VALUES ('legal_disclosure_version', '1.0', 'string', '19.9 — Version stamp written to investment_disclosures_log on every checkout.')
ON CONFLICT (key) DO NOTHING;
