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

3. **Apply any new migrations to production** (via Cloud SQL Proxy):
   - Ensure Cloud SQL Proxy is running:
```bash
cloud-sql-proxy my-project-35266-489713:europe-west1:poool-db --port=5433 2>&1
```
   - Then run pending migrations:
```bash
cd /Users/martin/Projects/poool && PGPASSWORD=p00lPr0dDb2026 psql -h 127.0.0.1 -p 5433 -U postgres -d poool -f database/latest_migration.sql
```

4. **Compare production schema with local** to catch missing tables/columns:
```bash
# List all tables on production
PGPASSWORD=p00lPr0dDb2026 psql -h 127.0.0.1 -p 5433 -U postgres -d poool -c "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name;"

# Compare with local
psql -d poool -c "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name;"
```

## Deploy

5. **Deploy to Cloud Run**:
```bash
cd /Users/martin/Projects/poool && gcloud run deploy poool-backend --source . --region europe-west1 --project my-project-35266-489713 --allow-unauthenticated
```

## Post-Deploy Verification

6. **Check that the service is healthy**:
```bash
curl -s -o /dev/null -w "HTTP %{http_code}" https://poool-backend-745757325286.europe-west1.run.app/marketplace
```

7. **Check Cloud Run logs for errors**:
```bash
gcloud run services logs read poool-backend --region=europe-west1 --project=my-project-35266-489713 --limit=10 2>&1 | grep -i error
```
