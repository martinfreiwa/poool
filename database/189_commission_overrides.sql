-- Migration 189: per-asset / per-affiliate commission rate overrides.
--
-- Phase-4 industry-standard (Tapfiliate / PartnerStack / Impact):
-- different assets sometimes need different commission rates.
-- Examples:
--   * "Asset X is a high-margin villa — pay 5% instead of the tier default."
--   * "Influencer Jane gets 3% on every asset; she negotiated up."
--   * "Holiday-2026 campaign assets pay 4% for 30 days."
--
-- We model this as a small table of overrides scoped by:
--   - `affiliate_id`  (optional — applies to ONE affiliate when set)
--   - `asset_id`      (optional — applies to ONE asset when set)
-- At least ONE of the two must be set. The commission engine picks the
-- most specific match: (affiliate, asset) > (affiliate, *) > (*, asset)
-- > tier default. Tied specificity is broken by `priority DESC` (admin-
-- tuned tiebreaker) then `created_at DESC`.
--
-- Validity window (`valid_from`, `valid_to`) lets a campaign rate auto-
-- expire without manual cleanup.
--
-- Idempotent. Safe to re-run.

CREATE TABLE IF NOT EXISTS affiliate_commission_overrides (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Either or both must be set. CHECK below enforces.
    affiliate_id        UUID REFERENCES affiliates(user_id) ON DELETE CASCADE,
    asset_id            UUID REFERENCES assets(id)          ON DELETE CASCADE,
    -- The override itself.
    commission_rate_bps INTEGER NOT NULL
                        CHECK (commission_rate_bps BETWEEN 0 AND 10000),
    -- Optional time window. NULL on either side = open-ended.
    valid_from          TIMESTAMPTZ,
    valid_to            TIMESTAMPTZ,
    -- Tiebreaker for ties in specificity (higher wins).
    priority            INTEGER NOT NULL DEFAULT 0,
    -- Free-form note shown in admin UI / audit log.
    reason              TEXT,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by_user_id  UUID REFERENCES users(id),
    CONSTRAINT affiliate_commission_overrides_scope
        CHECK (affiliate_id IS NOT NULL OR asset_id IS NOT NULL),
    CONSTRAINT affiliate_commission_overrides_window
        CHECK (valid_from IS NULL OR valid_to IS NULL OR valid_from <= valid_to)
);

-- Hot-path index for the commission engine's lookup:
--   "find the active override matching (this_affiliate, this_asset, NOW())"
CREATE INDEX IF NOT EXISTS idx_affiliate_commission_overrides_lookup
    ON affiliate_commission_overrides
       (affiliate_id, asset_id, priority DESC, created_at DESC)
    WHERE is_active = TRUE;

-- Avoid duplicate "exact (affiliate, asset)" overrides that would
-- otherwise need the priority tiebreaker. A new exact-match override
-- supersedes the old by either bumping priority OR deactivating the
-- prior one — the unique guarantees we don't accumulate clutter.
CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_commission_overrides_exact
    ON affiliate_commission_overrides (affiliate_id, asset_id)
    WHERE is_active = TRUE
      AND affiliate_id IS NOT NULL
      AND asset_id IS NOT NULL;

CREATE TRIGGER set_affiliate_commission_overrides_updated_at
    BEFORE UPDATE ON affiliate_commission_overrides
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

COMMENT ON TABLE affiliate_commission_overrides IS
  'Phase-4: per-affiliate and/or per-asset commission rate overrides. Picked by the commission engine in (affiliate,asset) → (affiliate,*) → (*,asset) → tier-default precedence.';
