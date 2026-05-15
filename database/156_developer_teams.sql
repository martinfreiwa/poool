-- 156: Developer team & team-membership tables
--
-- Phase 1 of Developer-Team-Affiliate-Programm.
--
-- Modell:
--   * Ein Developer kann mehrere Teams haben (multi-team future-proofing),
--     aber genau EIN aktives Default-Team. Heute erzeugen wir genau 1
--     Team je Developer beim Onboarding-Approve.
--   * Ein User kann zu jeder Zeit nur in EINEM aktiven Team (über alle
--     Developer hinweg) Mitglied sein. Wechsel: erst aus Team A entfernen,
--     dann in Team B aufnehmen.
--   * Beitritts-Flow erlaubt zwei Wege:
--       (a) Developer lädt per Email-Token ein  → status='invited'
--       (b) User self-requestet via Developer-Slug → 'pending_developer_approval'
--     Developer bestätigt → 'active'.
--
-- Skalierung:
--   * Indexe auf alle Lookup-Pfade (Owner, Member, Team).
--   * Partial-Unique-Indexes statt voller UNIQUE-Constraints, damit
--     gelöschte / entfernte Rows die Slot-Belegung freigeben.
--   * Keine Cascade auf Member-Soft-Delete: bestehende Commissions sollen
--     auch nach Member-Entfernung historisch zuordenbar bleiben. Daher
--     kein ON DELETE CASCADE auf users(id), sondern soft-delete via status.

BEGIN;

CREATE TABLE developer_teams (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_user_id    UUID NOT NULL REFERENCES users(id),
    display_name         TEXT NOT NULL,
    public_slug          VARCHAR(40),                              -- für Self-Request-Flow; NULL = unsichtbar
    is_default           BOOLEAN NOT NULL DEFAULT true,
    status               VARCHAR(20) NOT NULL DEFAULT 'active'
                         CHECK (status IN ('active', 'paused', 'terminated')),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    terminated_at        TIMESTAMPTZ,
    terminated_reason    TEXT
);

CREATE UNIQUE INDEX one_default_team_per_developer
    ON developer_teams (developer_user_id)
    WHERE is_default = true AND status <> 'terminated';

CREATE UNIQUE INDEX developer_teams_public_slug_uniq
    ON developer_teams (LOWER(public_slug))
    WHERE public_slug IS NOT NULL AND status <> 'terminated';

CREATE INDEX idx_developer_teams_owner ON developer_teams (developer_user_id);
CREATE INDEX idx_developer_teams_status ON developer_teams (status) WHERE status <> 'terminated';

CREATE TABLE developer_team_memberships (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id                  UUID NOT NULL REFERENCES developer_teams(id),
    user_id                  UUID NOT NULL REFERENCES users(id),
    role                     VARCHAR(20) NOT NULL DEFAULT 'member'
                             CHECK (role IN ('member', 'manager')),
    status                   VARCHAR(30) NOT NULL DEFAULT 'invited'
                             CHECK (status IN (
                                 'invited',                     -- Developer hat eingeladen, User noch nicht gehandelt
                                 'pending_developer_approval',  -- User self-requestet, Developer muss bestätigen
                                 'active',                       -- vollständig im Team
                                 'removed'                       -- soft-delete, historisch zuordenbar
                             )),
    invitation_token_hash    TEXT,                              -- SHA-256 des Tokens, nicht das Token selbst
    invitation_expires_at    TIMESTAMPTZ,
    invited_by_user_id       UUID REFERENCES users(id),
    invited_at               TIMESTAMPTZ DEFAULT NOW(),
    joined_at                TIMESTAMPTZ,
    removed_at               TIMESTAMPTZ,
    removed_reason           TEXT,
    removed_by_user_id       UUID REFERENCES users(id),
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Strikt: ein User darf nur in EINEM nicht-removed Team gleichzeitig sein.
-- Verhindert Multi-Team-Attribution und ambiguous payout routing.
CREATE UNIQUE INDEX one_active_membership_per_user
    ON developer_team_memberships (user_id)
    WHERE status IN ('invited', 'pending_developer_approval', 'active');

CREATE INDEX idx_team_memberships_team_status ON developer_team_memberships (team_id, status);
CREATE INDEX idx_team_memberships_user ON developer_team_memberships (user_id, status);
CREATE INDEX idx_team_memberships_invitation_token ON developer_team_memberships (invitation_token_hash)
    WHERE invitation_token_hash IS NOT NULL AND status = 'invited';

-- Updated_at-Trigger für beide Tabellen
CREATE OR REPLACE FUNCTION dev_team_set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END $$;

CREATE TRIGGER trg_developer_teams_updated_at
    BEFORE UPDATE ON developer_teams
    FOR EACH ROW EXECUTE FUNCTION dev_team_set_updated_at();

CREATE TRIGGER trg_developer_team_memberships_updated_at
    BEFORE UPDATE ON developer_team_memberships
    FOR EACH ROW EXECUTE FUNCTION dev_team_set_updated_at();

-- RLS: Developer darf eigene Teams sehen. Member darf eigene Membership sehen.
-- Aktiviert RLS aber nutzt owner-bypass solange app.current_user_id nicht
-- pro Session gesetzt wird (analog Migration 153).
ALTER TABLE developer_teams ENABLE ROW LEVEL SECURITY;
CREATE POLICY developer_teams_owner_read ON developer_teams
    FOR SELECT USING (developer_user_id = app_current_user_id());

ALTER TABLE developer_team_memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY team_memberships_self_read ON developer_team_memberships
    FOR SELECT USING (user_id = app_current_user_id());
CREATE POLICY team_memberships_developer_read ON developer_team_memberships
    FOR SELECT USING (
        team_id IN (
            SELECT id FROM developer_teams WHERE developer_user_id = app_current_user_id()
        )
    );

COMMIT;
