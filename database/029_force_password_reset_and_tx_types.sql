-- ============================================================
-- Migration 029: Force password reset flag + expanded wallet tx types
-- ============================================================

-- 1. Add force_password_reset flag to user_settings
ALTER TABLE user_settings ADD COLUMN IF NOT EXISTS force_password_reset BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN user_settings.force_password_reset IS 'When TRUE, user must change password on next login';

-- 2. Expand wallet_transactions type CHECK to include admin adjustments
--    Drop the old constraint and recreate with admin_credit/admin_debit
ALTER TABLE wallet_transactions DROP CONSTRAINT IF EXISTS wallet_transactions_type_check;

ALTER TABLE wallet_transactions ADD CONSTRAINT wallet_transactions_type_check
    CHECK (type IN (
        'deposit', 'withdrawal', 'purchase',
        'sale', 'dividend', 'reward', 'refund', 'fee',
        'admin_credit', 'admin_debit'
    ));
