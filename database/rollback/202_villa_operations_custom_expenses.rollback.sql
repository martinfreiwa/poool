-- 202 rollback — drop the JSONB custom-expense breakdown column.
-- Safe: column is additive and nullable; recompute() and compute_totals()
-- continue to use expense_other_idr_cents as the authoritative subtotal.

ALTER TABLE villa_operations_log
    DROP COLUMN IF EXISTS expense_other_notes;
