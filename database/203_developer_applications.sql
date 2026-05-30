-- Migration 203 — developer_applications: persistent record of every
-- "Become a Developer" submission, gated on admin review + KYC verification.
--
-- Closes critical onboarding gaps from the 2026-05-19 developer-pages audit
-- (C-1, C-2, C-3):
--
--   • C-1: POST /api/developer/apply auto-granted the `developer` role on a
--          100%-client-controlled body and persisted only 2 of 11 fields.
--          Anyone authenticated could self-promote.
--   • C-2: POST /api/developer/draft also auto-granted the `developer` role
--          as a side-effect of the very first draft. Same self-promotion.
--   • C-3: No part of onboarding was gated on KYC verification — Didit was
--          wired but not consulted.
--
-- Fix: every "Become a Developer" submission now lands in this table with
-- status='pending'. An admin must call POST /api/admin/developer-applications/
-- :id/approve, which (a) verifies the applicant's latest kyc_records row is
-- 'approved' and (b) then — and only then — grants the `developer` role.
--
-- Idempotent.

BEGIN;

CREATE TABLE IF NOT EXISTS developer_applications (
    id                          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                     UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Persist every field the application form sends. Names mirror the
    -- frontend payload in developer-onboarding.html so the apply handler
    -- can do a direct field-for-field INSERT.
    first_name                  TEXT,
    last_name                   TEXT,
    phone                       TEXT,
    whatsapp                    TEXT,
    nationality                 TEXT,
    country                     TEXT,
    website                     TEXT,
    assets_count                TEXT,
    asset_value                 TEXT,
    monthly_income              TEXT,
    bio                         TEXT,

    -- Review machinery.
    status                      TEXT         NOT NULL DEFAULT 'pending'
                                CHECK (status IN ('pending', 'approved', 'rejected', 'needs_kyc')),
    submitted_at                TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    reviewed_by                 UUID         REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at                 TIMESTAMPTZ,
    review_notes                TEXT,
    -- Snapshot of the kyc_records.verified_at value at approval time, so we
    -- preserve evidence even if the KYC record is later reset.
    kyc_verified_at             TIMESTAMPTZ
);

COMMENT ON TABLE developer_applications IS
  'Each submission of the "Become a Developer" form. Admin must approve before the '
  '`developer` role is granted (C-1/C-2/C-3 fix, 2026-05-19 audit). Approval is '
  'further gated on the applicant having an `approved` kyc_records row.';

COMMENT ON COLUMN developer_applications.status IS
  'pending → admin has not reviewed yet. '
  'needs_kyc → admin tried to approve but KYC was not verified. '
  'approved → developer role granted at reviewed_at. '
  'rejected → admin declined; see review_notes.';

-- Admin queue index — used by GET /api/admin/developer-applications?status=pending.
CREATE INDEX IF NOT EXISTS idx_developer_applications_status_submitted
  ON developer_applications (status, submitted_at DESC);

-- Per-user lookup — used to show the applicant their pending state.
CREATE INDEX IF NOT EXISTS idx_developer_applications_user
  ON developer_applications (user_id, submitted_at DESC);

COMMIT;
