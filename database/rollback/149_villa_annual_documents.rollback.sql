-- 149 rollback — drop the annual document link table.
-- Safe: villa_annual_documents is a pure link table; rows reference
-- asset_documents which are unaffected.

DROP TABLE IF EXISTS villa_annual_documents;
