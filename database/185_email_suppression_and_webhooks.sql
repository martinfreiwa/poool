-- 185: Email suppression list + provider_id index for webhook updates
--
-- Bounce suppression: once a recipient address hard-bounces or
-- complains, the outbox worker must stop attempting future sends.
-- Resend (and every reputable ESP) will block our sender domain if
-- we keep hammering known-bad addresses, hurting deliverability for
-- everyone else. The suppression list is keyed by lowercase email
-- so casing differences don't bypass the check.
--
-- A NULL `cleared_at` means the suppression is active. Admins can
-- manually un-suppress (set cleared_at = NOW()) after the recipient
-- confirms the bounce was transient — e.g. a full mailbox now drained.

CREATE TABLE IF NOT EXISTS email_suppressions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email             VARCHAR(255) NOT NULL,
    reason            VARCHAR(50) NOT NULL
                      CHECK (reason IN ('hard_bounce', 'soft_bounce',
                                        'spam_complaint', 'manual')),
    bounce_count      INTEGER NOT NULL DEFAULT 1 CHECK (bounce_count > 0),
    last_event_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    provider_event_id VARCHAR(255),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cleared_at        TIMESTAMPTZ,
    cleared_by_admin  UUID REFERENCES users(id)
);

-- Lowercase-unique active suppression per address. Partial index lets
-- a previously-cleared address get re-suppressed without a constraint
-- collision on the historical row.
CREATE UNIQUE INDEX IF NOT EXISTS idx_email_suppressions_active_lower
    ON email_suppressions (LOWER(email))
    WHERE cleared_at IS NULL;

-- Lookup index for the worker's pre-send check.
CREATE INDEX IF NOT EXISTS idx_email_suppressions_lower
    ON email_suppressions (LOWER(email));

-- email_logs.provider_id was added back in migration 008 but never
-- indexed. The Resend webhook handler does
-- `UPDATE email_logs ... WHERE provider_id = $1` on every event;
-- without an index that's a full-table scan per webhook hit.
CREATE INDEX IF NOT EXISTS idx_email_logs_provider_id
    ON email_logs (provider_id)
    WHERE provider_id IS NOT NULL;
