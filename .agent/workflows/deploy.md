---
description: Deploy to Cloud Run with all pre-flight checks
---

# Deploy to Cloud Run

// turbo-all

This workflow ensures the production deployment works correctly by running all prerequisite steps.

## Pre-Flight Checks

1. **Update SQLx offline cache** (required after any SQL query changes in Rust code):
```bash
cd /Users/martin/Projects/poool/backend && cargo sqlx prepare
```

2. **Verify it compiles locally** before pushing to Cloud Build:
```bash
cd /Users/martin/Projects/poool/backend && SQLX_OFFLINE=true cargo check --release 2>&1 | tail -5
```

3. **Verify PgBouncer compatibility** — check that `db.rs` respects `PGBOUNCER_ENABLED`:
```bash
cd /Users/martin/Projects/poool && rg "PGBOUNCER_ENABLED" backend/src/db.rs
```
   - Must see `PGBOUNCER_ENABLED` check that skips Cloud SQL socket auto-detection.
   - If missing, the deployment WILL fail with `prepared statement already exists`.

4. **Verify Dockerfile has PGBOUNCER_ENABLED=true**:
```bash
cd /Users/martin/Projects/poool && grep "PGBOUNCER_ENABLED" Dockerfile
```
   - Must show `ENV PGBOUNCER_ENABLED=true`.

5. **Verify PgBouncer entrypoint has ignore_startup_parameters**:
```bash
cd /Users/martin/Projects/poool && grep "ignore_startup_parameters" pgbouncer/entrypoint.sh
```
   - Must include `extra_float_digits` and `options`.

6. **Apply any new migrations to production** (via Cloud SQL Proxy):
   - Ensure Cloud SQL Proxy is running:
```bash
cloud-sql-proxy my-project-35266-489713:europe-west1:poool-db --port=5433 2>&1
```
   - Then run pending migrations:
```bash
cd /Users/martin/Projects/poool && PGPASSWORD=p00lPr0dDb2026 psql -h 127.0.0.1 -p 5433 -U postgres -d poool -f database/latest_migration.sql
```

7. **Compare production schema with local** to catch missing tables/columns:
```bash
# List all tables on production
PGPASSWORD=p00lPr0dDb2026 psql -h 127.0.0.1 -p 5433 -U postgres -d poool -c "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name;"

# Compare with local
psql -d poool -c "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name;"
```

## Deploy

8. **Commit and push** (Cloud Build triggers automatically on push to `main`):
```bash
cd /Users/martin/Projects/poool && git add . && git commit -m "deploy: <description>" && git push
```

## Post-Deploy Verification

9. **Wait for Cloud Build to finish** (~6 minutes):
```bash
sleep 180 && gcloud builds list --limit=1 --project=my-project-35266-489713
```
   - Confirm STATUS is `SUCCESS`. If `FAILURE`, check the build log.

10. **Check Cloud Run logs for errors on the new revision**:
```bash
gcloud run services logs read poool-backend --region=europe-west1 --project=my-project-35266-489713 --limit=20 2>&1 | grep -iE "error|panic|failed"
```
   - **Zero tolerance** for `prepared statement already exists` — if seen, PgBouncer bypass is back.
   - **Zero tolerance** for `GLIBC` errors — if seen, Dockerfile builder image is wrong.
   - **Zero tolerance** for `trust authentication failed` — if seen, PgBouncer credentials are wrong.

11. **Verify the live service is responding**:
```bash
curl -s -o /dev/null -w "HTTP %{http_code}" https://platform.poool.app/auth/login
```
   - Must return `HTTP 200`.

12. **Test Google OAuth** by visiting `https://platform.poool.app/auth/google` in a browser.

---

## Architecture: PgBouncer + Cloud SQL (DO NOT BREAK)

```
┌─────────────────────────────────────────────────────────┐
│  Cloud Run Container                                     │
│                                                          │
│  ┌──────────────┐    TCP 127.0.0.1:6432    ┌──────────┐ │
│  │ POOOL Backend │ ──────────────────────► │ PgBouncer │ │
│  │  (Rust/Axum)  │                         │ (sidecar) │ │
│  └──────────────┘                          └─────┬────┘ │
│                                                   │      │
│                            Unix socket: /cloudsql/...    │
│                                                   │      │
└───────────────────────────────────────────────────┼──────┘
                                                    │
                                              ┌─────▼─────┐
                                              │ Cloud SQL  │
                                              │ PostgreSQL │
                                              └───────────┘
```

### Critical Rules:
1. **Backend → PgBouncer → Cloud SQL.** The backend must NEVER connect directly to the Unix socket.
2. **`PGBOUNCER_ENABLED=true`** in Dockerfile tells `db.rs` to skip socket auto-detection.
3. **`pool_mode = session`** in PgBouncer config — mandatory for `sqlx` prepared statements to work without collision.
4. **`ignore_startup_parameters = extra_float_digits, options`** in PgBouncer config prevents sqlx connection rejection.
5. **`entrypoint.sh` rewrites `DATABASE_URL`** to `127.0.0.1:6432` before starting the backend.

### Known Failure Modes:
| Symptom | Cause | Fix |
|---------|-------|-----|
| `prepared statement "sqlx_s_N" already exists` | Backend bypassing PgBouncer (connecting to socket directly) | Ensure `PGBOUNCER_ENABLED=true` and `db.rs` checks it |
| `unsupported startup parameter: extra_float_digits` | PgBouncer rejecting sqlx connection params | Add `ignore_startup_parameters` to PgBouncer config |
| `"trust" authentication failed` | PgBouncer not passing credentials upstream | Add `user=` and `password=` to `[databases]` line |
| `GLIBC_2.39 not found` | Builder image glibc newer than runtime | Pin builder to `rust:1-bookworm` matching `debian:bookworm-slim` |
| Container fails to start (timeout) | Any of the above causing crash loop | Check logs with `gcloud run services logs read` |
