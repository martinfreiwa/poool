-- 18.6, 18.7, 18.8 — Withdrawal safety controls.
--
-- These rows seed `platform_settings` with the knobs the wallet handler
-- consults at runtime. Defaults match the masterplan numbers — operators
-- can tune via the existing admin platform_settings UI without code change.

-- Configurable withdrawal limits (cents, hours).
INSERT INTO platform_settings (key, value, value_type, description)
VALUES
    ('withdrawal_daily_cap_cents',       '1000000', 'number', '18.6 — Max cumulative withdrawal value per user per UTC calendar day. 1_000_000c = $10,000.'),
    ('withdrawal_velocity_threshold',    '3',       'number', '18.7 — More than this many withdrawal_requests within 24h triggers an auto-freeze pending admin review.'),
    ('withdrawal_velocity_window_hours', '24',      'number', '18.7 — Sliding window (in hours) for the velocity counter.'),
    ('new_account_cooldown_hours',       '72',      'number', '18.8 — First N hours after KYC verified, withdrawals are capped at new_account_max_withdraw_cents.'),
    ('new_account_max_withdraw_cents',   '100000',  'number', '18.8 — Per-withdrawal cap during the new-account cooldown window. 100_000c = $1,000.')
ON CONFLICT (key) DO NOTHING;
