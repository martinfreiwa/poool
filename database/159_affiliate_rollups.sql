-- 159: Pre-Aggregierte Rollups + Live-Counter für Dashboard-Performance
--
-- Problem: bei wachsendem affiliate_commissions (10M+ Rows in 3 Jahren bei
-- erwartetem Volume) wird ein GROUP BY für Dashboard-Tiles teuer. Lösung:
--   * affiliate_daily_rollups — pro Tag/Link/Attribution/Payout vorbereitete
--     Aggregate (Clicks, Signups, Qualified, Gross-Revenue, Commission).
--     Worker läuft alle 15 Minuten und schreibt Delta-Updates.
--   * affiliate_live_counters — pro payout_user_id O(1)-Counter für
--     Dashboard-Tile "Mein Gesamtumsatz" / "Pending Commission".
--     Updates fließen synchron bei Status-Wechseln (Service-Layer schreibt).
--
-- Beide Tabellen sind ableitbar — bei Korruption komplett rebuildbar aus
-- den Source-Tables. Nicht der Source-of-Truth, aber der Hot-Read-Path.

BEGIN;

CREATE TABLE affiliate_daily_rollups (
    rollup_date          DATE       NOT NULL,
    link_id              UUID       NOT NULL REFERENCES affiliate_links(id),
    payout_user_id       UUID       NOT NULL,
    attribution_user_id  UUID       NOT NULL,
    team_id              UUID,                     -- NULL für personal-Links
    link_type            VARCHAR(20) NOT NULL,
    clicks_count         INTEGER    NOT NULL DEFAULT 0,
    signups_count        INTEGER    NOT NULL DEFAULT 0,
    qualified_count      INTEGER    NOT NULL DEFAULT 0,
    gross_revenue_cents  BIGINT     NOT NULL DEFAULT 0,
    commission_cents     BIGINT     NOT NULL DEFAULT 0,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (rollup_date, link_id)
);

-- Dashboard-Lookups: "Letzte 30 Tage für Payout-User X"
CREATE INDEX idx_rollups_payout_date_desc
    ON affiliate_daily_rollups (payout_user_id, rollup_date DESC);

-- Developer-Dashboard: "Letzte 30 Tage für Team Y, per Member breakdown"
CREATE INDEX idx_rollups_team_date_desc
    ON affiliate_daily_rollups (team_id, rollup_date DESC)
    WHERE team_id IS NOT NULL;

-- Member-Self-View: "Was hat Mitarbeiter Z geworben"
CREATE INDEX idx_rollups_attribution_date_desc
    ON affiliate_daily_rollups (attribution_user_id, rollup_date DESC);

CREATE TABLE affiliate_live_counters (
    payout_user_id              UUID PRIMARY KEY REFERENCES users(id),
    lifetime_revenue_cents      BIGINT NOT NULL DEFAULT 0,
    lifetime_commission_cents   BIGINT NOT NULL DEFAULT 0,
    pending_commission_cents    BIGINT NOT NULL DEFAULT 0,   -- provisionally_tracked + under_holdback
    payable_commission_cents    BIGINT NOT NULL DEFAULT 0,
    paid_commission_cents       BIGINT NOT NULL DEFAULT 0,
    clawed_back_cents           BIGINT NOT NULL DEFAULT 0,
    last_updated                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- One-time Backfill aus Bestandsdaten
INSERT INTO affiliate_live_counters
    (payout_user_id,
     lifetime_revenue_cents, lifetime_commission_cents,
     pending_commission_cents, payable_commission_cents,
     paid_commission_cents, clawed_back_cents)
SELECT
    ac.payout_user_id,
    0,
    COALESCE(SUM(ac.provisional_amount_cents), 0),
    COALESCE(SUM(ac.provisional_amount_cents) FILTER (
        WHERE ac.status IN ('provisionally_tracked', 'on_hold')
    ), 0),
    COALESCE(SUM(ac.provisional_amount_cents) FILTER (
        WHERE ac.status = 'payable'
    ), 0),
    COALESCE(SUM(ac.provisional_amount_cents) FILTER (
        WHERE ac.status = 'paid'
    ), 0),
    COALESCE(SUM(ac.provisional_amount_cents) FILTER (
        WHERE ac.status = 'clawed_back'
    ), 0)
FROM affiliate_commissions ac
GROUP BY ac.payout_user_id
ON CONFLICT (payout_user_id) DO NOTHING;

-- RLS: nur Owner darf eigene Counter sehen.
ALTER TABLE affiliate_live_counters ENABLE ROW LEVEL SECURITY;
CREATE POLICY live_counters_owner_read ON affiliate_live_counters
    FOR SELECT USING (payout_user_id = app_current_user_id());

ALTER TABLE affiliate_daily_rollups ENABLE ROW LEVEL SECURITY;
CREATE POLICY rollups_payout_read ON affiliate_daily_rollups
    FOR SELECT USING (payout_user_id = app_current_user_id());
CREATE POLICY rollups_team_owner_read ON affiliate_daily_rollups
    FOR SELECT USING (
        team_id IN (
            SELECT id FROM developer_teams WHERE developer_user_id = app_current_user_id()
        )
    );
CREATE POLICY rollups_attribution_read ON affiliate_daily_rollups
    FOR SELECT USING (attribution_user_id = app_current_user_id());

COMMIT;
