-- 148 rollback — restore migration 146 defaults if needed.
-- Re-enables Villa-Returns flag + pilot bool for every asset. Use only if
-- rolling back to pre-148 state for testing.

UPDATE platform_settings
SET value = 'on'
WHERE key = 'villa_returns.enabled'
  AND value = 'off';

UPDATE assets
SET villa_returns_pilot = TRUE
WHERE villa_returns_pilot = FALSE;
