-- 186: Per-workflow enable/disable for the transactional email engine
--
-- Lets admins toggle entire categories of mail (e.g. milestone celebrations,
-- abandoned-cart drips) off without code changes. The check fires inside
-- the outbox worker and inside `trigger_transactional_email`, so disabling
-- a workflow stops *new* sends but doesn't drain rows already enqueued.
--
-- Mandatory events (security, legal, payment-completion) cannot be
-- disabled — the `workflow_is_enabled` helper short-circuits them via the
-- in-code mandatory list (`common::email::is_mandatory_event`). Admin UI
-- greys those toggle switches out.

CREATE TABLE IF NOT EXISTS email_workflow_settings (
    event_type        VARCHAR(100) PRIMARY KEY,
    enabled           BOOLEAN      NOT NULL DEFAULT TRUE,
    note              TEXT,
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_by_admin  UUID REFERENCES users(id)
);

-- Audit-trail trigger so we can recover from accidental mass-disable.
-- Reuses the existing `set_updated_at` pattern from migration 008.
DROP TRIGGER IF EXISTS set_email_workflow_settings_updated_at ON email_workflow_settings;
CREATE TRIGGER set_email_workflow_settings_updated_at
    BEFORE UPDATE ON email_workflow_settings
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
