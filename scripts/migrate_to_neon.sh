#!/usr/bin/env bash
# Migrate Cloud SQL → Neon PostgreSQL
# Run this once from your local machine.
# Prerequisites: gcloud, cloud-sql-proxy, pg_dump, psql

set -euo pipefail

PROJECT_ID="my-project-35266-489713"
INSTANCE="my-project-35266-489713:europe-west1:poool-db"
DB_NAME="poool"
DUMP_FILE="/tmp/poool_dump_$(date +%Y%m%d_%H%M%S).sql"

# ── 1. Get current DB credentials from Secret Manager ──────────
echo "Fetching DATABASE_URL from Secret Manager..."
RAW_URL=$(gcloud secrets versions access latest \
  --secret=poool-database-url \
  --project="$PROJECT_ID")
# Extract user and password from unix-socket URL
# Format: postgres://user:pass@/dbname?host=/cloudsql/...
DB_USER=$(echo "$RAW_URL" | grep -oP '(?<=://)[^:]+')
DB_PASS=$(echo "$RAW_URL" | grep -oP '(?<=://[^:]{1,64}:)[^@]+')

# ── 2. Start Cloud SQL Auth Proxy ───────────────────────────────
echo "Starting Cloud SQL Auth Proxy on port 5433..."
cloud-sql-proxy "$INSTANCE" --port=5433 &
PROXY_PID=$!
trap "kill $PROXY_PID 2>/dev/null; echo 'Proxy stopped.'" EXIT
sleep 3

# ── 3. pg_dump from Cloud SQL ───────────────────────────────────
echo "Dumping database to $DUMP_FILE..."
PGPASSWORD="$DB_PASS" pg_dump \
  --host=127.0.0.1 \
  --port=5433 \
  --username="$DB_USER" \
  --dbname="$DB_NAME" \
  --no-owner \
  --no-acl \
  --format=plain \
  --file="$DUMP_FILE"
echo "Dump done: $(du -sh "$DUMP_FILE" | cut -f1)"

# ── 4. Import to Neon ───────────────────────────────────────────
echo ""
echo "=== NEON SETUP ==="
echo "1. Go to https://neon.tech → create free project → region: EU (Frankfurt)"
echo "2. Copy the connection string (postgres://user:pass@host/dbname?sslmode=require)"
echo ""
read -rp "Paste your Neon connection string: " NEON_URL

echo "Importing dump into Neon..."
psql "$NEON_URL" \
  --file="$DUMP_FILE" \
  --single-transaction \
  2>&1 | tail -20

echo "Import done!"

# ── 5. Update Secret Manager ────────────────────────────────────
echo ""
echo "Updating SECRET poool-database-url to Neon URL..."
echo -n "$NEON_URL" | gcloud secrets versions add poool-database-url \
  --data-file=- \
  --project="$PROJECT_ID"
echo "Secret updated!"

echo ""
echo "=== DONE ==="
echo "Next steps:"
echo "  1. Run a deploy: gh workflow run deploy.yml"
echo "  2. Check https://platform.poool.app/ready → status should be 'ok'"
echo "  3. If site works: delete Cloud SQL instance in GCP Console"
echo "     https://console.cloud.google.com/sql/instances?project=$PROJECT_ID"
echo "  4. Delete Redis Memorystore if it exists:"
echo "     https://console.cloud.google.com/memorystore/redis/instances?project=$PROJECT_ID"
