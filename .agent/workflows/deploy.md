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

5. **Deploy to Cloud Run via Zero-Downtime Safe Rollout**:
   - This bash script deploys a "shadow" revision (`--no-traffic`), fetches its unique URL, tests the `/health` endpoint, and only routes live user traffic if the test passes.
   - If the health check fails, the new revision is silently discarded and no users are affected.

```bash
cd /Users/martin/Projects/poool
cat << 'EOF' > deploy_safe.sh
#!/bin/bash
set -e

echo "🚀 Deploying new revision without traffic..."
gcloud run deploy poool-backend \
    --source . \
    --region europe-west1 \
    --project my-project-35266-489713 \
    --allow-unauthenticated \
    --no-traffic \
    --tag staging

echo "🔍 Fetching staging URL..."
# We sleep a moment to make sure traffic config is propagated
sleep 3
STAGING_URL=$(gcloud run services describe poool-backend --region europe-west1 --project my-project-35266-489713 --format="json" | grep -o '"url": "[^"]*"' | grep 'staging---' | cut -d'"' -f4 | head -n 1)

if [ -z "$STAGING_URL" ]; then
    echo "❌ Could not parse staging URL."
    exit 1
fi

echo "🏥 Checking health at $STAGING_URL/health"
# Attempt health check up to 3 times
for i in {1..3}; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$STAGING_URL/health" || echo "failed")
    if [ "$STATUS" = "200" ]; then
        break
    fi
    echo "Attempt $i: Status $STATUS... retrying in 5 seconds."
    sleep 5
done

if [ "$STATUS" = "200" ]; then
    echo "✅ Health check passed (HTTP 200). Routing 100% traffic to new staging revision..."
    gcloud run services update-traffic poool-backend \
        --region europe-west1 \
        --project my-project-35266-489713 \
        --to-tags staging=100
    echo "🎉 Deployment completely successful."
else
    echo "🧯 Health check final failure (HTTP $STATUS)! Traffic NOT updated."
    echo "💡 Rollback is automatic. The buggy revision will not receive production traffic."
    exit 1
fi
EOF
chmod +x deploy_safe.sh
./deploy_safe.sh
```

## Post-Deploy Verification

6. **Check that the live service is still healthy**:
```bash
curl -s -o /dev/null -w "HTTP %{http_code}" https://poool-backend-745757325286.europe-west1.run.app/marketplace
```

7. **Check Cloud Run logs for recent errors on the new revision**:
```bash
gcloud run services logs read poool-backend --region=europe-west1 --project=my-project-35266-489713 --limit=15 2>&1 | grep -i error
```
