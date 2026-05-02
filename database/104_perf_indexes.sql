-- Performance indexes for hot admin endpoints.
-- Each query identified from Cloud Run latency logs (1.4-2.9s server-side).

-- /api/admin/approvals: ORDER BY created_at DESC LIMIT 200
CREATE INDEX IF NOT EXISTS idx_approval_created_at
    ON admin_approval_requests (created_at DESC);

-- /api/admin/notifications: ORDER BY n.created_at DESC LIMIT 200
CREATE INDEX IF NOT EXISTS idx_notifications_created_at
    ON notifications (created_at DESC);

-- /api/admin/community/reports: WHERE status='pending' ORDER BY created_at ASC
CREATE INDEX IF NOT EXISTS idx_content_reports_status_created
    ON content_reports (status, created_at ASC);

-- /api/admin/rewards/affiliates/pending: LATERAL subquery on
-- affiliate_payout_requests WHERE affiliate_id=$ AND status IN (...) ORDER BY requested_at
CREATE INDEX IF NOT EXISTS idx_affiliate_payout_requests_affiliate_status_requested
    ON affiliate_payout_requests (affiliate_id, status, requested_at ASC);

-- Same endpoint: WHERE ac.status = 'payable' (high-volume table, narrow filter)
CREATE INDEX IF NOT EXISTS idx_affiliate_commissions_status_payable
    ON affiliate_commissions (status)
    WHERE status = 'payable';
