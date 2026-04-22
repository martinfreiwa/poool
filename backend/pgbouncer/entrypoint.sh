#!/bin/sh
# ═══════════════════════════════════════════════════════════════════
# PgBouncer Sidecar Entrypoint Script
#
# 1. Parses DATABASE_URL to extract host/port/dbname/user/password
# 2. Generates pgbouncer.ini with the real connection details
# 3. Starts PgBouncer in the background
# 4. Starts the POOOL backend as the main process
#
# Phase 0.5 — Infrastructure
# ═══════════════════════════════════════════════════════════════════

set -e

# ── Parse DATABASE_URL ────────────────────────────────────────────
# Expected format: postgres://user:password@host:port/dbname?params
# or: postgres://user@host:port/dbname

if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL is not set. Cannot configure PgBouncer."
    exec /app/poool-backend
fi

# Check if PgBouncer should be enabled (disabled by default for local dev)
if [ "${PGBOUNCER_ENABLED:-true}" = "false" ]; then
    echo "PgBouncer disabled (PGBOUNCER_ENABLED=false). Starting backend directly."
    exec /app/poool-backend
fi

# Check if pgbouncer binary exists
if ! command -v pgbouncer >/dev/null 2>&1; then
    echo "WARN: pgbouncer binary not found. Starting backend directly."
    exec /app/poool-backend
fi

# Extract components from DATABASE_URL using shell parsing
# Remove postgres:// prefix
DB_STRIPPED="${DATABASE_URL#postgres://}"
DB_STRIPPED="${DB_STRIPPED#postgresql://}"

# Extract user info (before @)
DB_USERINFO="${DB_STRIPPED%%@*}"
DB_HOSTPATH="${DB_STRIPPED#*@}"

# User and password
DB_USER="${DB_USERINFO%%:*}"
if echo "$DB_USERINFO" | grep -q ':'; then
    DB_PASS="${DB_USERINFO#*:}"
else
    DB_PASS=""
fi

# Extract host:port/dbname
DB_HOSTPORT="${DB_HOSTPATH%%/*}"
DB_NAMEQUERY="${DB_HOSTPATH#*/}"
DB_NAME="${DB_NAMEQUERY%%\?*}"

# Host and port
DB_HOST="${DB_HOSTPORT%%:*}"
if echo "$DB_HOSTPORT" | grep -q ':'; then
    DB_PORT="${DB_HOSTPORT#*:}"
else
    DB_PORT="5432"
fi

# Override host if specified in query string parameters (e.g. for Cloud SQL unix sockets)
if echo "$DB_NAMEQUERY" | grep -q 'host='; then
    DB_HOST_OVERRIDE=$(echo "$DB_NAMEQUERY" | grep -o 'host=[^&]*' | cut -d= -f2-)
    if [ -n "$DB_HOST_OVERRIDE" ]; then
        DB_HOST="$DB_HOST_OVERRIDE"
        echo "PgBouncer: Extracted host override from query string: $DB_HOST"
    fi
fi

echo "PgBouncer: Proxying to ${DB_HOST}:${DB_PORT}/${DB_NAME} as ${DB_USER}"

# ── Generate PgBouncer config ─────────────────────────────────────

PGBOUNCER_DIR="/tmp/pgbouncer"
mkdir -p "$PGBOUNCER_DIR"

cat > "$PGBOUNCER_DIR/pgbouncer.ini" <<EOF
[databases]
${DB_NAME} = host=${DB_HOST} port=${DB_PORT} dbname=${DB_NAME} user=${DB_USER} password='${DB_PASS}'

[pgbouncer]
listen_addr = 127.0.0.1
listen_port = 6432
auth_type = trust
auth_file = $PGBOUNCER_DIR/userlist.txt
pool_mode = session
default_pool_size = ${PGBOUNCER_POOL_SIZE:-25}
min_pool_size = ${PGBOUNCER_MIN_POOL:-5}
max_client_conn = ${PGBOUNCER_MAX_CLIENT:-100}
max_db_connections = ${PGBOUNCER_MAX_DB:-30}
reserve_pool_size = 5
reserve_pool_timeout = 3
server_idle_timeout = 120
server_connect_timeout = 5
server_login_retry = 3
ignore_startup_parameters = extra_float_digits, options
server_reset_query = DISCARD ALL
query_timeout = 60
query_wait_timeout = 30
client_idle_timeout = 300
log_connections = 0
log_disconnections = 0
log_pooler_errors = 1
stats_period = 60
admin_users =
unix_socket_dir = /tmp
logfile = /tmp/pgbouncer.log
pidfile = /tmp/pgbouncer.pid
EOF

# ── Generate userlist.txt ─────────────────────────────────────────
cat > "$PGBOUNCER_DIR/userlist.txt" <<EOF
"${DB_USER}" "${DB_PASS}"
EOF

# ── Start PgBouncer ───────────────────────────────────────────────
echo "Starting PgBouncer on 127.0.0.1:6432..."
pgbouncer -d "$PGBOUNCER_DIR/pgbouncer.ini" 2>&1 || {
    echo "WARN: PgBouncer failed to start. Starting backend without connection pooling proxy."
    exec /app/poool-backend
}

# Give PgBouncer a moment to bind the port
sleep 1

# ── Rewrite DATABASE_URL to point to PgBouncer ───────────────────
if [ -n "$DB_PASS" ]; then
    export DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@127.0.0.1:6432/${DB_NAME}"
else
    export DATABASE_URL="postgres://${DB_USER}@127.0.0.1:6432/${DB_NAME}"
fi

echo "PgBouncer started. Backend will connect via 127.0.0.1:6432."
echo "Starting POOOL backend..."

# ── Start the backend ─────────────────────────────────────────────
exec /app/poool-backend
