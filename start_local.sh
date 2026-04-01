#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# start_local.sh — Start the POOOL platform on localhost
#
# Usage:  ./start_local.sh
#
# What this does:
#   1. Checks PostgreSQL is running
#   2. Creates the database if it doesn't exist
#   3. Runs any pending migrations (including the new affiliate system ones)
#   4. Builds and starts the Axum backend (serves the full platform)
#
# Prereqs (one-time setup):
#   brew install rustup-init postgresql@16
#   rustup-init && source ~/.cargo/env
#   brew services start postgresql@16
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/backend"
DB_NAME="poool"

# ── Colour helpers ────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
ok()   { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }
err()  { echo -e "${RED}✗${NC} $1"; exit 1; }

echo ""
echo "  🏦  POOOL – Local Development Server"
echo "  ─────────────────────────────────────"
echo ""

# ── 1. Check prerequisites ────────────────────────────────────────────────────
command -v cargo  >/dev/null 2>&1 || err "Rust/Cargo not found. Install via: curl https://sh.rustup.rs -sSf | sh"
command -v psql   >/dev/null 2>&1 || err "psql not found. Install via: brew install postgresql@16"
ok "Prerequisites found"

# ── 2. Check PostgreSQL is running ────────────────────────────────────────────
if ! pg_isready -q 2>/dev/null; then
    warn "PostgreSQL doesn't appear to be running."
    echo "  Start it with: brew services start postgresql@16"
    echo "  Then re-run this script."
    exit 1
fi
ok "PostgreSQL is running"

# ── 3. Ensure database exists ─────────────────────────────────────────────────
if ! psql -lqt 2>/dev/null | cut -d \| -f 1 | grep -qw "$DB_NAME"; then
    echo "  Creating database '$DB_NAME'..."
    createdb "$DB_NAME" && ok "Database '$DB_NAME' created"
else
    ok "Database '$DB_NAME' exists"
fi

# ── 4. Migrations run automatically at server startup ────────────────────────
# The backend reads all .sql files in ../database/ at startup and applies
# any that haven't been applied yet (tracked in _schema_migrations table).
# No manual psql needed!
ok "Migrations will apply automatically on first startup"

# ── 5. Build CSS bundle (only if bundle.css is missing/stale) ─────────────────
BUNDLE="$SCRIPT_DIR/frontend/platform/static/css/bundle.css"
BUILD_SCRIPT="$SCRIPT_DIR/frontend/platform/static/css/build-bundle.sh"
if [ ! -f "$BUNDLE" ]; then
    echo "  Building CSS bundle..."
    bash "$BUILD_SCRIPT" && ok "CSS bundle built"
else
    ok "CSS bundle exists"
fi

# ── 6. Start the backend ──────────────────────────────────────────────────────
echo ""
echo "  Building and starting backend (this may take a minute on first run)..."
echo "  ──────────────────────────────────────────────────────────────────────"
echo ""

cd "$BACKEND_DIR"

# SQLX_OFFLINE=true is set in .env — uses pre-baked query cache so no live DB
# connection is needed at compile time.
cargo run
