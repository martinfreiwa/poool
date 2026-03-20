ALTER TABLE investments DROP CONSTRAINT IF EXISTS investments_status_check;
ALTER TABLE investments ADD CONSTRAINT investments_status_check CHECK (status IN ('active', 'funded', 'rented', 'payout_pending', 'in_process', 'funding_in_progress', 'exited', 'failed'));
