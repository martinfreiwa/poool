-- ══════════════════════════════════════════════════════════════
-- Add business details to platform_settings for PDF documents
-- Required for all exported PDF/print documents per Indonesian law
-- ══════════════════════════════════════════════════════════════

INSERT INTO platform_settings (key, value, value_type, description)
VALUES 
  ('company_legal_name', 'PT POOOL Finance Indonesia', 'string', 'Official registered company name for legal documents'),
  ('company_address', 'Jl. Sunset Road No. 88', 'string', 'Company street address'),
  ('company_city', 'Seminyak, Bali', 'string', 'Company city'),
  ('company_postal', '80361', 'string', 'Company postal code'),
  ('company_country', 'Indonesia', 'string', 'Company country'),
  ('company_npwp', '00.000.000.0-000.000', 'string', 'Company NPWP (tax ID number)'),
  ('company_nib', '0000000000000', 'string', 'Company NIB (business registration number)'),
  ('company_ojk_license', '', 'string', 'OJK license number (if applicable)'),
  ('company_phone', '+62 361 000 0000', 'string', 'Company phone number'),
  ('company_website', 'https://poool.finance', 'string', 'Company website URL')
ON CONFLICT (key) DO NOTHING;
