-- ══════════════════════════════════════════════════════════════
-- 179_deposit_bank_wire_settings.sql
--
-- Seed platform_settings with bank-wire details used to render the
-- deposit modal and the post-deposit instructions. These values
-- replace the hardcoded strings in:
--   backend/src/payments/service.rs::create_deposit_request
--   frontend/platform/static/js/wallet.js::showDepositInstructionsModal
--
-- Editing these values via Admin → Settings → Deposits takes effect
-- on the next deposit request; no redeploy required.
-- ══════════════════════════════════════════════════════════════

INSERT INTO platform_settings (key, value, value_type, description) VALUES
  ('deposit_bank_name',          'Deutsche Bank AG',         'string', 'Bank name shown on deposit instructions'),
  ('deposit_account_holder',     'POOOL GmbH',               'string', 'Account holder shown on deposit instructions'),
  ('deposit_iban',               'DE89370400440532013000',   'string', 'IBAN shown on deposit instructions'),
  ('deposit_bic',                'DEUTDEDB',                 'string', 'BIC / SWIFT code shown on deposit instructions'),
  ('deposit_bank_address',       '',                         'string', 'Optional bank address shown on deposit instructions'),
  ('deposit_reference_prefix',   'POOOL',                    'string', 'Prefix used when generating the unique deposit reference'),
  ('deposit_processing_hours',   '24',                       'number', 'Expected hours until a deposit is credited after wire is received'),
  ('deposit_min_amount_cents',   '5000',                     'number', 'Minimum deposit amount in cents (default $50.00)'),
  ('deposit_max_amount_cents',   '10000000',                 'number', 'Maximum single deposit amount in cents (default $100,000)')
ON CONFLICT (key) DO NOTHING;
