-- Migration 100: Per-asset tick size override.
--
-- Default tick size lives in `marketplace:settings` Redis key (5¢). For
-- assets with low token prices (e.g. 5¢ token, 5¢ tick = no price granularity)
-- we need a finer tick. For high-value assets ($1000 tokens) a coarser tick
-- avoids dust orders.
--
-- Per-asset override: NULL = use platform default. Otherwise this value (in
-- cents) wins. Validation in `validate_runtime_settings_for_order` reads
-- this column and falls back to settings when null.

ALTER TABLE assets
    ADD COLUMN IF NOT EXISTS tick_size_cents INTEGER
        CHECK (tick_size_cents IS NULL OR tick_size_cents > 0);

COMMENT ON COLUMN assets.tick_size_cents IS
    'Per-asset price granularity (cents). NULL = use platform default from
     marketplace:settings. Overrides allow finer ticks on cheap tokens and
     coarser ticks on expensive ones.';
