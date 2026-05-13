-- Rollback for 131_villa_expense_categories.sql.
-- Safe pre-deploy. Post-deploy: prefer RENAME if any villa_deduction_policy row references codes.

DROP TABLE IF EXISTS villa_expense_categories CASCADE;
DELETE FROM _schema_migrations WHERE filename = '131_villa_expense_categories.sql';
