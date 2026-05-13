-- 142 — Villa-Returns P1: developer-villa assignment table (W1 onboarding gap-fix).
--
-- A Developer user can only submit operations / CapEx / forecast suggestions for villas
-- they are explicitly linked to. Row-level enforcement happens in the DeveloperUser
-- extractor on the backend (auth middleware checks for an active row in this table).
-- Append-only: revokes flip effective_until rather than deleting, for audit.

CREATE TABLE IF NOT EXISTS developer_asset_links (
    id                  BIGSERIAL PRIMARY KEY,
    developer_user_id   UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    asset_id            UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    effective_from      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_until     TIMESTAMPTZ,
    granted_by          UUID REFERENCES users(id),
    revoked_by          UUID REFERENCES users(id),
    revoked_at          TIMESTAMPTZ,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT dal_period_sane CHECK (effective_until IS NULL OR effective_until > effective_from)
);

-- A user can only have one active link to a given asset at a time.
CREATE UNIQUE INDEX IF NOT EXISTS uq_dal_active_link
    ON developer_asset_links (developer_user_id, asset_id)
    WHERE effective_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_dal_developer_active
    ON developer_asset_links (developer_user_id) WHERE effective_until IS NULL;

CREATE INDEX IF NOT EXISTS idx_dal_asset_active
    ON developer_asset_links (asset_id) WHERE effective_until IS NULL;

-- Append-only guard: links may be revoked (effective_until set) but never deleted or fully rewritten.
CREATE OR REPLACE FUNCTION fn_developer_asset_links_guard()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'developer_asset_links is append-only; revoke by setting effective_until instead (id=%)', OLD.id;
    END IF;

    -- Allow only effective_until / revoked_by / revoked_at / notes to be set on UPDATE.
    IF  NEW.developer_user_id IS DISTINCT FROM OLD.developer_user_id
     OR NEW.asset_id          IS DISTINCT FROM OLD.asset_id
     OR NEW.effective_from    IS DISTINCT FROM OLD.effective_from
     OR NEW.granted_by        IS DISTINCT FROM OLD.granted_by
    THEN
        RAISE EXCEPTION 'developer_asset_links: only effective_until/revoked_by/revoked_at/notes may be updated (id=%)', OLD.id;
    END IF;

    -- Once revoked, no further mutation.
    IF OLD.effective_until IS NOT NULL THEN
        RAISE EXCEPTION 'developer_asset_links: revoked link is immutable (id=%)', OLD.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_developer_asset_links_guard ON developer_asset_links;
CREATE TRIGGER trg_developer_asset_links_guard
    BEFORE UPDATE OR DELETE ON developer_asset_links
    FOR EACH ROW EXECUTE FUNCTION fn_developer_asset_links_guard();

COMMENT ON TABLE developer_asset_links IS 'Authorises a Developer user to submit data for a specific villa. DeveloperUser extractor checks for an active row before any /api/developer/villas/:id/... write.';
