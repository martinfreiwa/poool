-- 193_users_status_frozen_constraint.sql
--
-- The application uses `users.status = 'frozen'` for withdrawal-velocity
-- auto-freezes and admin account controls. Some existing databases were
-- created before `frozen` was added to the canonical schema, so their
-- `users_status_check` constraint still rejects the value.

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_status_check;

ALTER TABLE users
    ADD CONSTRAINT users_status_check
    CHECK (status IN ('active', 'suspended', 'deleted', 'frozen'));
