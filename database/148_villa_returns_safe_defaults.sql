-- 148 — Villa-Returns: force safe defaults for production rollout.
--
-- Migration 146 seeds `platform_settings.villa_returns.enabled='on'` and adds
-- `assets.villa_returns_pilot` column defaulting to TRUE. That combination
-- makes the investor read path (assets/villa_performance.rs) return new-layer
-- data for every asset on first boot — but legacy assets have zero rows in
-- `villa_operations_log`, so the UI surfaces would show zeros until operations
-- are explicitly published per asset.
--
-- This migration flips both to the "safe" state:
--   - platform_settings.villa_returns.enabled = 'off'  (read path returns zeros consistently as "feature disabled" semantics)
--   - assets.villa_returns_pilot = FALSE for all rows
--
-- After this migration applies, an operator must explicitly:
--   1. UPDATE platform_settings SET value='shadow' WHERE key='villa_returns.enabled';
--   2. Pick a pilot asset: UPDATE assets SET villa_returns_pilot=TRUE WHERE id='<uuid>';
--   3. Publish operations through admin UI.
--   4. Flip enabled='on' once verified.
--
-- Idempotent: only flips rows currently in the unsafe default state.

UPDATE platform_settings
SET value = 'off'
WHERE key = 'villa_returns.enabled'
  AND value = 'on';

UPDATE assets
SET villa_returns_pilot = FALSE
WHERE villa_returns_pilot = TRUE;
