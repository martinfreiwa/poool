-- 146 — Villa-Returns A7: feature-flag seed + per-asset pilot toggle.
--
-- Plan §6.1 — rollout state is gated by `platform_settings.villa_returns.enabled`
-- (off | shadow | on) and a per-asset `villa_returns_pilot` flag. Default state
-- on dev is `'on'` for the single test asset; production should override the
-- pilot flag per-asset before exposing.

INSERT INTO platform_settings (key, value, value_type, description)
VALUES (
    'villa_returns.enabled',
    'on',
    'string',
    'Villa-Returns rollout state: off (legacy reads only), shadow (write both, return legacy), on (read new layer)'
)
ON CONFLICT (key) DO NOTHING;

ALTER TABLE assets
    ADD COLUMN IF NOT EXISTS villa_returns_pilot BOOLEAN NOT NULL DEFAULT TRUE;

COMMENT ON COLUMN assets.villa_returns_pilot IS 'Per-asset opt-in for Villa-Returns read path. Default TRUE on dev; flip to FALSE per asset before adding to a real investor cohort.';
