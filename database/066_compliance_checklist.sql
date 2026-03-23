-- 066: Add compliance_checklist JSONB column to developer_projects
-- Stores the admin's manual compliance checklist state so it persists across page reloads.
-- Example value: {"chk-kyc":true,"chk-legal":true,"chk-spv":false,...}

ALTER TABLE developer_projects
  ADD COLUMN IF NOT EXISTS compliance_checklist JSONB DEFAULT '{}'::jsonb;

COMMENT ON COLUMN developer_projects.compliance_checklist
  IS 'Admin compliance checklist state (JSON object of checkbox_id → boolean). Persisted so it survives page reloads.';
