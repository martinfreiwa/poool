-- Migration 014: Performance Optimization Indexes
-- Resolving common query performance issues identified in audit.

-- 1. Faster portfolio / active investment filtering
CREATE INDEX IF NOT EXISTS idx_investments_status ON investments(status);

-- 2. Faster Marketplace rendering for published / featured properties
CREATE INDEX IF NOT EXISTS idx_assets_published_featured ON assets(published, featured);

-- 3. Faster Order analytics and item lookups
CREATE INDEX IF NOT EXISTS idx_order_items_asset ON order_items(asset_id);

-- 4. Faster notification filtering by type (KYC, payments, etc.)
CREATE INDEX IF NOT EXISTS idx_notifications_type ON notifications(type);
