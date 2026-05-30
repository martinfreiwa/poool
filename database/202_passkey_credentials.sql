-- Passkey / WebAuthn credentials
-- Each row is one registered passkey for a user (phone, laptop, hardware key…).
-- passkey_data stores the serialized webauthn-rs `Passkey` struct as JSONB so
-- sign_count and public-key material stay together.

CREATE TABLE IF NOT EXISTS passkey_credentials (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   TEXT NOT NULL UNIQUE,   -- base64url, indexed for fast lookup at login
    passkey_data    JSONB NOT NULL,         -- webauthn-rs Passkey (serialised)
    name            TEXT NOT NULL DEFAULT 'Passkey',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_passkey_credentials_user_id      ON passkey_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_passkey_credentials_credential_id ON passkey_credentials(credential_id);

-- Short-lived challenge state stored server-side between start and finish calls.
-- kind = 'register' | 'authenticate'
-- state_data holds the serialised webauthn-rs PasskeyRegistration /
-- DiscoverableAuthentication struct (danger-allow-state-serialisation).
-- Rows expire after 5 minutes and are cleaned up on finish or by a future job.

CREATE TABLE IF NOT EXISTS passkey_challenges (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID REFERENCES users(id) ON DELETE CASCADE,  -- NULL for unauthenticated auth start
    kind        TEXT NOT NULL CHECK (kind IN ('register', 'authenticate')),
    state_data  JSONB NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '5 minutes',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_passkey_challenges_expires ON passkey_challenges(expires_at);
