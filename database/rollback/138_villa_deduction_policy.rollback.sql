-- Rollback for 138_villa_deduction_policy.sql.

DROP TRIGGER IF EXISTS trg_villa_deduction_policy_guard ON villa_deduction_policy;
DROP FUNCTION IF EXISTS fn_villa_deduction_policy_guard();
DROP TABLE IF EXISTS villa_deduction_policy CASCADE;
DELETE FROM _schema_migrations WHERE filename = '138_villa_deduction_policy.sql';
