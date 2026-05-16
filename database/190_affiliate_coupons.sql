-- Migration 190: coupon codes attributed to affiliate links.
--
-- Phase-4 industry-standard (Rewardful / FirstPromoter): some affiliates
-- (especially influencers) drive conversions via promo codes rather than
-- tracking links. The user enters `INFLUENCER10` at checkout → the
-- attribution path uses the code's `affiliate_link_id` instead of the
-- `?ref=` cookie path.
--
-- A coupon code:
--   * MAY grant the customer a discount (handled by the checkout module;
--     we just store the optional `discount_bps` for display).
--   * ALWAYS routes commission to the linked affiliate.
--
-- Soft delete via `is_active` so historical attributions still resolve
-- when the code is retired. Case-insensitive uniqueness (lowercase
-- functional index) so `INFLUENCER10` == `influencer10` == `Influencer10`.

CREATE TABLE IF NOT EXISTS affiliate_coupons (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The literal coupon string the customer types at checkout. Stored
    -- case-preserving for display; matched case-insensitively.
    code                VARCHAR(40) NOT NULL
                        CHECK (length(code) BETWEEN 3 AND 40
                               AND code ~ '^[A-Za-z0-9_-]+$'),
    -- Affiliate link the commission routes through. Cascade delete so a
    -- removed link tidies its coupons.
    affiliate_link_id   UUID NOT NULL REFERENCES affiliate_links(id)
                        ON DELETE CASCADE,
    -- Optional customer-facing discount (basis points off the order
    -- subtotal). Checkout module reads this when present. NULL = no
    -- discount; the coupon is purely an attribution shortcut.
    discount_bps        INTEGER CHECK (discount_bps IS NULL
                                        OR (discount_bps BETWEEN 0 AND 10000)),
    -- Optional validity window. NULL = open-ended on that side.
    valid_from          TIMESTAMPTZ,
    valid_to            TIMESTAMPTZ,
    -- Optional global usage cap. NULL = unlimited; INT = stops attribution
    -- after `usage_count` reaches `max_uses`.
    max_uses            INTEGER CHECK (max_uses IS NULL OR max_uses > 0),
    usage_count         INTEGER NOT NULL DEFAULT 0,
    is_active           BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by_user_id  UUID REFERENCES users(id),
    CONSTRAINT affiliate_coupons_window
        CHECK (valid_from IS NULL OR valid_to IS NULL OR valid_from <= valid_to)
);

-- Case-insensitive uniqueness across active coupons. Inactive coupons
-- may share a code with an active one — useful for "retired then
-- re-issued under same string" workflows.
CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_coupons_code_lower
    ON affiliate_coupons (LOWER(code))
    WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_affiliate_coupons_link
    ON affiliate_coupons (affiliate_link_id)
    WHERE is_active = TRUE;

CREATE TRIGGER set_affiliate_coupons_updated_at
    BEFORE UPDATE ON affiliate_coupons
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- Atomic "consume one usage" — the checkout calls this to both validate
-- the code AND atomically reserve the slot. Returns the matched coupon
-- row (with affiliate_link_id) on success, or NULL on miss / exhausted /
-- expired / inactive. Single round-trip, no race window.
CREATE OR REPLACE FUNCTION consume_affiliate_coupon(p_code TEXT)
RETURNS TABLE (
    coupon_id          UUID,
    affiliate_link_id  UUID,
    discount_bps       INTEGER,
    code               VARCHAR
)
LANGUAGE plpgsql
AS $$
BEGIN
    RETURN QUERY
    UPDATE affiliate_coupons c
       SET usage_count = c.usage_count + 1,
           updated_at  = NOW()
     WHERE c.id = (
         SELECT inner_c.id
           FROM affiliate_coupons inner_c
          WHERE LOWER(inner_c.code) = LOWER(p_code)
            AND inner_c.is_active = TRUE
            AND (inner_c.valid_from IS NULL OR inner_c.valid_from <= NOW())
            AND (inner_c.valid_to   IS NULL OR inner_c.valid_to   >= NOW())
            AND (inner_c.max_uses   IS NULL OR inner_c.usage_count < inner_c.max_uses)
          ORDER BY inner_c.created_at ASC
          LIMIT 1
          FOR UPDATE SKIP LOCKED
     )
     RETURNING c.id, c.affiliate_link_id, c.discount_bps, c.code;
END;
$$;

COMMENT ON TABLE affiliate_coupons IS
  'Phase-4: coupon codes that route commission to an affiliate link. Use `consume_affiliate_coupon(code)` to atomically validate + reserve a usage slot in one round-trip.';
