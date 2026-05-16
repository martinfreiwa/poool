#!/usr/bin/env bash
# Local dev runner with auto-reload.
#
# Watches:
#   - backend/src       (Rust source)         → triggers cargo build + restart
#   - frontend/platform (templates, JS, CSS)  → triggers binary restart only
#                                                 (no Rust recompile needed)
#
# Why this works for "I edited an .html / .css / .js file and want to see it":
#   * cargo-watch detects the file change.
#   * `cargo run` checks freshness — if no .rs changed, no recompile → fast.
#   * The new process recreates the MiniJinja Environment from disk
#     (cache cleared) and bumps the dev-mode `asset_version` timestamp,
#     so every CSS/JS URL becomes `?v=<new_ts>` and browsers stop serving
#     the old cached copy.
#
# Why we don't use a fancier inline template-autoreload: cargo-watch already
# gives us atomic correctness (restart = full reset of every cache) and a
# 2-3s rebuild on HTML-only changes. No extra dependency on minijinja-autoreload.
#
# Usage:
#   ./scripts/dev_watch.sh                   # foreground, Ctrl-C to stop
#   ./scripts/dev_watch.sh > /tmp/dev.log &  # background
#
# Requires: cargo-watch (install with `cargo install cargo-watch` if missing).

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT/backend"

if ! command -v cargo-watch >/dev/null 2>&1; then
  echo "ERROR: cargo-watch not found. Install with:"
  echo "    cargo install cargo-watch"
  exit 1
fi

# Free port 8888 if anything is squatting on it (previous run, IDE, etc.)
PID="$(lsof -ti:8888 2>/dev/null || true)"
if [ -n "${PID:-}" ]; then
  echo "[dev_watch] killing existing process on :8888 (pid $PID)"
  kill "$PID" 2>/dev/null || true
  sleep 1
fi

# -w  watch path (relative to backend/, where this command runs)
# -i  ignore glob (don't restart on noise — .sqlx changes, target/, etc.)
# -x  execute on change
# --clear leaves a clean terminal between rebuilds
echo "[dev_watch] starting cargo-watch — edits to backend/src or frontend/platform will hot-restart"
exec cargo watch \
  -w src \
  -w ../frontend/platform \
  -w ../database \
  -i 'target/**' \
  -i '.sqlx/**' \
  -i 'frontend/platform/_archive/**' \
  -i 'frontend/platform/**/*.bak' \
  --clear \
  -x 'run --bin poool-backend'
