-- Villa-Returns — Phase 3 Step C2 pilot cut-over.
-- Run against PRODUCTION after deploy 1dbbae5 (migrations 130-148 applied).
--
-- Post-deploy state: platform_settings.villa_returns.enabled = 'off',
-- every asset villa_returns_pilot = FALSE (migration 148 safe defaults).
-- This script moves ONE asset into the pilot.
--
-- Procedure: run PART 1, eyeball the candidates, paste the chosen id into
-- PART 2's \set line, then run PART 2 inside the transaction.

-- ── PART 1 — review candidates ───────────────────────────────────────
-- Bali / Indonesia real-estate assets, fewest investors first so the
-- pilot has live read traffic but a small blast radius.
SELECT a.id,
       a.title,
       a.location_city,
       a.funding_status,
       COUNT(DISTINCT i.user_id)               AS investor_count,
       COUNT(DISTINCT vol.id)                   AS villa_ops_rows
FROM assets a
LEFT JOIN investments i        ON i.asset_id = a.id
LEFT JOIN villa_operations_log vol ON vol.asset_id = a.id
WHERE a.asset_type = 'real_estate'
  AND (a.location_country = 'ID' OR a.location_country = 'Indonesia')
GROUP BY a.id, a.title, a.location_city, a.funding_status
ORDER BY investor_count ASC, a.created_at ASC
LIMIT 10;

-- ── PART 2 — flip the pilot (transaction-wrapped) ────────────────────
-- Replace the UUID below with the chosen asset id from PART 1.
\set pilot_id '00000000-0000-0000-0000-000000000000'

BEGIN;

-- Shadow mode: publish handler writes BOTH new layer + legacy
-- asset_financials. Investor read path still gated per-asset.
UPDATE platform_settings
SET value = 'shadow'
WHERE key = 'villa_returns.enabled';

-- Enable the pilot asset only.
UPDATE assets
SET villa_returns_pilot = TRUE
WHERE id = :'pilot_id';

-- Verify: expect 1 row, villa_returns_pilot = t.
SELECT id, title, villa_returns_pilot
FROM assets
WHERE id = :'pilot_id';

-- Verify flag.
SELECT key, value FROM platform_settings WHERE key = 'villa_returns.enabled';

-- Inspect the two SELECTs above. If correct:  COMMIT;
-- If wrong:  ROLLBACK;
COMMIT;

-- ── Rollback (if pilot misbehaves within the 14-day window) ──────────
-- UPDATE platform_settings SET value='off' WHERE key='villa_returns.enabled';
-- UPDATE assets SET villa_returns_pilot=FALSE WHERE id=:'pilot_id';
