-- 157: affiliate_links — zentrale Link-Entität (Personal vs. Team-Business)
--
-- Splittet den heutigen "1 Affiliate = 1 Code"-Ansatz auf in:
--   * Personal-Links: attribution_user_id = payout_user_id
--   * Team-Business-Links: attribution_user_id ≠ payout_user_id, team_id NOT NULL
--
-- Dazu erhalten affiliate_referrals + affiliate_commissions + referral_clicks
-- die neue link_id-Spalte plus die getrennten attribution_user_id /
-- payout_user_id-Felder. Bestehende Rows werden in genau eine synthetische
-- Personal-Link-Row je Affiliate gebackfilled.
--
-- Skalierung:
--   * Composite-Indexe für jeden Dashboard- und Worker-Lookup.
--   * Partial-Index-Filter (status='active') hält Index klein.
--   * affiliates.referral_code bleibt vorerst als denormalisierte
--     Convenience-Spalte (Backfill setzt sie identisch mit
--     affiliate_links.code des Personal-Links).
--
-- Backfill ist idempotent (alle INSERTs nutzen ON CONFLICT DO NOTHING bzw.
-- werden über NULL-Filter geschützt).

BEGIN;

-- ── 1) affiliate_links ─────────────────────────────────────────────────────
CREATE TABLE affiliate_links (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code                 VARCHAR(32) NOT NULL,
    link_type            VARCHAR(20) NOT NULL
                         CHECK (link_type IN ('personal', 'team_business')),
    attribution_user_id  UUID NOT NULL REFERENCES users(id),
    payout_user_id       UUID NOT NULL REFERENCES users(id),
    team_id              UUID REFERENCES developer_teams(id),
    status               VARCHAR(20) NOT NULL DEFAULT 'active'
                         CHECK (status IN ('active', 'inactive', 'suspended')),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deactivated_at       TIMESTAMPTZ,
    deactivated_reason   TEXT,
    CONSTRAINT affiliate_links_shape_check CHECK (
        (link_type = 'personal'
            AND team_id IS NULL
            AND attribution_user_id = payout_user_id)
     OR (link_type = 'team_business'
            AND team_id IS NOT NULL
            AND attribution_user_id <> payout_user_id)
    )
);

CREATE UNIQUE INDEX affiliate_links_code_uniq ON affiliate_links (code);
CREATE INDEX idx_affiliate_links_attribution_active
    ON affiliate_links (attribution_user_id) WHERE status = 'active';
CREATE INDEX idx_affiliate_links_payout_active
    ON affiliate_links (payout_user_id) WHERE status = 'active';
CREATE INDEX idx_affiliate_links_team_active
    ON affiliate_links (team_id) WHERE team_id IS NOT NULL AND status = 'active';
CREATE INDEX idx_affiliate_links_type_status ON affiliate_links (link_type, status);

CREATE TRIGGER trg_affiliate_links_updated_at
    BEFORE UPDATE ON affiliate_links
    FOR EACH ROW EXECUTE FUNCTION dev_team_set_updated_at();

ALTER TABLE affiliate_links ENABLE ROW LEVEL SECURITY;
CREATE POLICY affiliate_links_attribution_read ON affiliate_links
    FOR SELECT USING (attribution_user_id = app_current_user_id());
CREATE POLICY affiliate_links_payout_read ON affiliate_links
    FOR SELECT USING (payout_user_id = app_current_user_id());
CREATE POLICY affiliate_links_team_owner_read ON affiliate_links
    FOR SELECT USING (
        team_id IN (
            SELECT id FROM developer_teams WHERE developer_user_id = app_current_user_id()
        )
    );

-- ── 2) Backfill: Personal-Link je bestehender affiliates-Row ───────────────
-- Erzeugt für jeden existierenden Affiliate genau eine Personal-Link-Row,
-- die exakt seinen referral_code übernimmt. So bleiben alle bestehenden
-- URLs gültig.
INSERT INTO affiliate_links
    (id, code, link_type, attribution_user_id, payout_user_id, team_id, status, created_at)
SELECT
    gen_random_uuid(),
    a.referral_code,
    'personal',
    a.user_id,
    a.user_id,
    NULL,
    CASE WHEN a.status = 'active' THEN 'active' ELSE 'inactive' END,
    a.created_at
FROM affiliates a
WHERE NOT EXISTS (
    SELECT 1 FROM affiliate_links al WHERE al.code = a.referral_code
);

-- ── 3) Extension auf affiliate_referrals ───────────────────────────────────
ALTER TABLE affiliate_referrals
    ADD COLUMN link_id              UUID REFERENCES affiliate_links(id),
    ADD COLUMN attribution_user_id  UUID REFERENCES users(id),
    ADD COLUMN payout_user_id       UUID REFERENCES users(id);

-- Backfill: bestehende Referrals zeigen auf den synthetischen Personal-Link
-- ihres heutigen affiliate_id (das = payout = attribution für Personal).
UPDATE affiliate_referrals ar
SET link_id              = al.id,
    attribution_user_id  = ar.affiliate_id,
    payout_user_id       = ar.affiliate_id
FROM affiliate_links al
WHERE al.attribution_user_id = ar.affiliate_id
  AND al.link_type = 'personal'
  AND ar.link_id IS NULL;

-- Erzwinge NOT NULL nach Backfill
ALTER TABLE affiliate_referrals
    ALTER COLUMN link_id             SET NOT NULL,
    ALTER COLUMN attribution_user_id SET NOT NULL,
    ALTER COLUMN payout_user_id      SET NOT NULL;

-- Indexe für Dashboard- und Worker-Queries
CREATE INDEX idx_referrals_link ON affiliate_referrals (link_id);
CREATE INDEX idx_referrals_payout_status ON affiliate_referrals (payout_user_id, status);
CREATE INDEX idx_referrals_attribution_created
    ON affiliate_referrals (attribution_user_id, created_at DESC);

-- Sanity: attribution ≠ referred (Self-Referral), payout ≠ referred
ALTER TABLE affiliate_referrals
    ADD CONSTRAINT referrals_no_self_attribution CHECK (referred_user_id <> attribution_user_id),
    ADD CONSTRAINT referrals_no_self_payout      CHECK (referred_user_id <> payout_user_id);

-- ── 4) Extension auf affiliate_commissions ─────────────────────────────────
ALTER TABLE affiliate_commissions
    ADD COLUMN link_id              UUID REFERENCES affiliate_links(id),
    ADD COLUMN attribution_user_id  UUID REFERENCES users(id),
    ADD COLUMN payout_user_id       UUID REFERENCES users(id);

UPDATE affiliate_commissions ac
SET link_id              = ar.link_id,
    attribution_user_id  = ar.attribution_user_id,
    payout_user_id       = ar.payout_user_id
FROM affiliate_referrals ar
WHERE ar.id = ac.referral_id
  AND ac.link_id IS NULL;

ALTER TABLE affiliate_commissions
    ALTER COLUMN link_id             SET NOT NULL,
    ALTER COLUMN attribution_user_id SET NOT NULL,
    ALTER COLUMN payout_user_id      SET NOT NULL;

-- Hot-Path-Indexe für Payout-Aggregation, Holdback-Worker, Reporting
CREATE INDEX idx_commissions_payout_status_created
    ON affiliate_commissions (payout_user_id, status, created_at DESC);
CREATE INDEX idx_commissions_attribution_created
    ON affiliate_commissions (attribution_user_id, created_at DESC);
CREATE INDEX idx_commissions_link_status
    ON affiliate_commissions (link_id, status);

-- ── 5) referral_clicks: FK auf legacy referral_codes lockern, link_id hinzu ─
-- Heute zwingt referral_clicks_code_fkey jede Click-Row eine referral_codes-
-- Row zu haben. Für Team-Business-Codes existieren die nicht. Außerdem
-- wollen wir auf link_id pivotieren.
ALTER TABLE referral_clicks DROP CONSTRAINT IF EXISTS referral_clicks_code_fkey;

ALTER TABLE referral_clicks
    ADD COLUMN link_id UUID REFERENCES affiliate_links(id);

-- Backfill (Variante B, vom User gewählt): link_id aus code-join setzen,
-- soweit der Code zu einem (synthetischen) Personal-Link gehört. Legacy
-- referral_codes-Codes ohne Affiliate bleiben mit link_id=NULL.
UPDATE referral_clicks rc
SET link_id = al.id
FROM affiliate_links al
WHERE al.code = rc.code
  AND rc.link_id IS NULL;

CREATE INDEX idx_referral_clicks_link
    ON referral_clicks (link_id, created_at DESC)
    WHERE link_id IS NOT NULL;

COMMIT;
