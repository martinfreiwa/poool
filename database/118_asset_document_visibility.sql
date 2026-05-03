-- Admin-controlled visibility for investor-facing property documents.
-- Existing investor-visible document categories are backfilled to preserve
-- current published property pages after the migration.

ALTER TABLE asset_documents
  ADD COLUMN IF NOT EXISTS is_investor_visible BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE asset_documents
SET is_investor_visible = TRUE
WHERE document_type IN (
  'proof_of_title',
  'legal_basis',
  'building_permit',
  'site_plan',
  'expose',
  'appraisal',
  'financial',
  'floor_plan',
  'other'
)
AND is_investor_visible = FALSE;

CREATE INDEX IF NOT EXISTS idx_asset_docs_asset_investor_visible
  ON asset_documents(asset_id, is_investor_visible)
  WHERE is_investor_visible = TRUE;

COMMENT ON COLUMN asset_documents.is_investor_visible IS
  'Admin-controlled flag deciding whether the document is shown and downloadable on investor-facing property pages.';
