-- Migration 019: Admin Dashboard Optimization Indexes
-- Resolving missing indexes for newly added queries in admin dashboard.

-- 1. Faster querying for user signups over time
CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at);
CREATE INDEX IF NOT EXISTS idx_users_status ON users(status);

-- 2. Faster querying for wallet transactions (deposits over time)
CREATE INDEX IF NOT EXISTS idx_wallet_tx_type_status_created ON wallet_transactions(type, status, created_at);

-- 3. Faster filtering of KYC records by status
CREATE INDEX IF NOT EXISTS idx_kyc_records_status ON kyc_records(status);

-- 4. Faster filtering of deposit requests by status
CREATE INDEX IF NOT EXISTS idx_deposit_requests_status ON deposit_requests(status);

-- 5. Faster asset filtering for marketplace and admin
CREATE INDEX IF NOT EXISTS idx_assets_funding_status ON assets(funding_status);
CREATE INDEX IF NOT EXISTS idx_assets_asset_type ON assets(asset_type);

-- 6. Faster order analytics
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_created_at ON orders(created_at);

-- 7. Faster rewards tracking querying
CREATE INDEX IF NOT EXISTS idx_referral_tracking_referrer ON referral_tracking(referrer_id);
