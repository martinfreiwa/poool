-- database/022_bank_system_field.sql
-- Add bank_system column to track which banking system the account uses
-- e.g. 'ach' (US), 'bacs' (UK), 'sepa' (EU), 'bsb' (AU), 'ifsc' (IN), 'swift' (international)

ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS bank_system VARCHAR(20);
