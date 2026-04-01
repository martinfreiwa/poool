-- Phase 18: Affiliate & Referral Subsystem (Core Schema)
-- Implements Zero-Trust Compliance-by-Code Referral Gating

-- Affiliate Profile & Compliance Gates
CREATE TABLE affiliates (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    referral_code VARCHAR(20) UNIQUE NOT NULL,
    current_tier VARCHAR(20) DEFAULT 'Access',
    commission_rate_bps INTEGER DEFAULT 50, -- 0.50%
    tax_recipient_class VARCHAR(30) CHECK (tax_recipient_class IN ('id_individual', 'id_entity', 'foreign', 'pending')),
    is_tax_ready BOOLEAN DEFAULT false,
    status VARCHAR(20) DEFAULT 'pending_approval' CHECK (status IN ('pending_onboarding', 'pending_approval', 'active', 'suspended', 'terminated')),
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Referral Tracking State Machine
CREATE TABLE affiliate_referrals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id UUID REFERENCES affiliates(user_id),
    referred_user_id UUID UNIQUE REFERENCES users(id),
    qualifying_investment_id UUID REFERENCES investments(id),
    status VARCHAR(30) DEFAULT 'attributed' CHECK (status IN ('attributed', 'registered', 'kyc_approved', 'first_investment_done', 'under_holdback', 'qualified', 'disqualified', 'reversed')),
    holdback_expires_at TIMESTAMPTZ,
    disqualifying_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Provisional & Payable Commissions
CREATE TABLE affiliate_commissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referral_id UUID REFERENCES affiliate_referrals(id),
    affiliate_id UUID REFERENCES affiliates(user_id),
    source_order_id UUID NOT NULL,
    provisional_amount_cents BIGINT NOT NULL,
    status VARCHAR(30) DEFAULT 'provisionally_tracked' CHECK (status IN ('provisionally_tracked', 'on_hold', 'payable', 'paid', 'frozen', 'clawback_pending', 'clawed_back')),
    payout_batch_id UUID,
    tier_at_execution VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Immutable Legal Acceptance Log
CREATE TABLE affiliate_policy_acceptances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id UUID REFERENCES affiliates(user_id),
    policy_name VARCHAR(50) NOT NULL,
    policy_version VARCHAR(20) NOT NULL,
    ip_address VARCHAR(45),
    accepted_at TIMESTAMPTZ DEFAULT NOW()
);

-- Checkout Disclosure Log (Proof of Independent Decision)
CREATE TABLE investment_disclosures_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    order_id UUID NOT NULL,
    is_referral_user BOOLEAN NOT NULL,
    agreed_to_general BOOLEAN NOT NULL,
    agreed_to_referral BOOLEAN, -- NULL for direct users
    ip_address VARCHAR(45),
    agreed_at TIMESTAMPTZ DEFAULT NOW()
);
