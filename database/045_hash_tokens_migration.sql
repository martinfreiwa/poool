-- Migration: Invalidate any existing raw-text tokens
-- After this migration, tokens are stored as SHA-256 hex hashes.
-- Old tokens stored as plaintext will no longer match any hash,
-- effectively invalidating them. Users will need to request new tokens.

-- Password reset tokens: invalidate all unused ones  
UPDATE password_reset_tokens SET used_at = NOW() WHERE used_at IS NULL;

-- Email verification tokens: delete all outstanding ones
DELETE FROM email_verification_tokens;

-- Admin invitations: invalidate all pending ones (they need to be resent)
DELETE FROM admin_invitations WHERE accepted_at IS NULL;
