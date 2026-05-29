-- 202 — Villa operations: capture custom-expense breakdown (name + amount).
--
-- C-5 fix: prior to this migration, the operations submit form folded all
-- user-typed "other" expense row names into a single expense_other_idr_cents
-- bucket on the client before POSTing, so investors and admins reviewing the
-- log never saw what the "other" expenses were for. This column stores the
-- per-row breakdown as a JSONB array of {name, amount_idr_cents} entries.
--
-- Additive only: the existing expense_other_idr_cents column keeps its
-- semantics (total of catch-all + custom rows) and continues to feed
-- recompute() and the server-side compute_totals() subtotal logic.

ALTER TABLE villa_operations_log
    ADD COLUMN IF NOT EXISTS expense_other_notes JSONB NULL;

COMMENT ON COLUMN villa_operations_log.expense_other_notes IS
    'Array of {name, amount_idr_cents} for the named "other" expense rows the developer added. NULL if none.';
