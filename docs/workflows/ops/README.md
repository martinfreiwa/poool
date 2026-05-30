# Operations And Cross-Cutting Workflows

Purpose: Cover deployment, local operations, live confidence, and global edge cases shared by all roles.

Roles: Developer, Admin, Operator.

Primary pages and surfaces:
- `.agent/workflows/deploy.md`, `.agent/workflows/push.md`, `.agent/workflows/start-kyc-local.md`
- `/health`, `/admin/system`, `/admin/storage`, `/admin/audit-logs`, `/admin/reports`
- PgBouncer, Cloud SQL, Redis, Sentry, storage, email/webhook providers, local PostgreSQL.

Backend/API surfaces:
- Health checks, system jobs, storage reconciliation/retention, webhooks, email webhooks, payment/KYC webhooks, maintenance, cache/log rotation, reports, audit logs.

Prerequisites:
- Local backend, database, and optional Redis are controllable.
- Live/staging operations require explicit user approval.
- No production mutation is run unless explicitly approved.

Steps:
1. Verify local backend start/stop, migrations, SQLx cache, cargo checks/tests, and fixture seeding.
2. Verify PgBouncer/Cloud SQL production rules: backend points to PgBouncer, session pooling, ignored startup params, credentials, and statement cache behavior.
3. Verify Redis unavailable/empty/degraded states show safe user-facing behavior and rebuild/retry paths where implemented.
4. Verify Sentry/health/logging/alerts capture critical failures without leaking secrets.
5. Verify storage reconciliation, retention arm/run, upload links, and GCS-disabled states.
6. Verify webhook replay/receipt for email, payment, and KYC in local/staging sink.
7. Verify live read-only confidence pass using `docs/workflows/cross-role/live-read-only-confidence-pass.md`.
8. Verify global edge cases across representative pages: auth boundaries, CSRF, mobile layout, accessibility, network failure, empty state, authorization error, destructive confirmation, audit log, money-in-cents, and cleanup.

Expected Result:
- Operational workflows are repeatable, safe by default, and explicitly separate local/staging/live actions.
- Global edge cases are tested once per role category and cited by product workflows.

Coverage Matrix:

| Area | Expected Result |
|------|-----------------|
| Local dev | Backend, DB, migrations, and tests are reproducible. |
| Deployment | PgBouncer/Cloud SQL/Redis/Sentry assumptions are checked. |
| Storage/webhooks | Upload and inbound-event paths are auditable. |
| Live pass | Read-only confidence runs without mutation. |
| Edge cases | Cross-cutting states are classified and reusable. |

Negative Cases:
- DB down, Redis down, storage disabled, webhook signature invalid, email sink unavailable, stale migration, failed health check, unauthorized admin system action, and accidental live mutation attempt.

Audit / DB / Financial Checks:
- Ops actions affecting user, financial, storage, or system state write audit/operational logs.
- No live money mutation is performed without explicit approval.

Cleanup:
- Stop local background jobs, remove disposable fixtures, clear generated exports containing personal data, and document any retained logs.

