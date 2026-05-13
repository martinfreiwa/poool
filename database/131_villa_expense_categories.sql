-- 131 — Villa-Returns P1: lookup table for permitted expense categories (PDF §2).
--
-- A small fixed catalog. Used by villa_deduction_policy to whitelist categories per villa
-- and by the operations entry form for category labels and ordering.

CREATE TABLE IF NOT EXISTS villa_expense_categories (
    code         VARCHAR(40) PRIMARY KEY,
    label        VARCHAR(120) NOT NULL,
    description  TEXT,
    is_default   BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order   INTEGER NOT NULL DEFAULT 100,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO villa_expense_categories (code, label, description, sort_order) VALUES
    ('cleaning',     'Cleaning',              'All monthly cleaning costs.',                              10),
    ('maintenance',  'Maintenance / repairs', 'Ongoing maintenance of the villa.',                        20),
    ('utilities',    'Utilities',             'Electricity, water, internet.',                            30),
    ('staff',        'Staff & security',      'Personnel, housekeeping and security costs.',              40),
    ('pool_garden',  'Pool & garden',         'Recurring property care costs.',                           50),
    ('pest',         'Pest control',          'Pest control services.',                                   60),
    ('mgmt_fee',     'Management fee',        'Management company fee paid this period.',                 70),
    ('ota_fees',     'OTA fees',              'Fees from Airbnb, Booking.com, Agoda, ...',                80),
    ('payment_fees', 'Payment fees',          'Payment provider charges.',                                90),
    ('refunds',      'Refunds & cancellations','Refunds or guest-related revenue corrections.',          100),
    ('other',        'Other operating costs', 'Non-standard but contractually permitted operating costs.',110)
ON CONFLICT (code) DO NOTHING;
