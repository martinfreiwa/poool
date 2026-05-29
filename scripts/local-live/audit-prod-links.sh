#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-.}"

matches="$(
  rg -n \
    'https://platform\.poool\.app|https://www\.poool\.app' \
    "$ROOT/frontend/platform" \
    "$ROOT/backend/src" \
    "$ROOT/backend/templates" \
    2>/dev/null || true
)"

if [[ -n "$matches" ]]; then
  printf '%s\n' "$matches"
  printf '\nProduction-domain links found. Replace navigational/app URLs with relative URLs or a central BASE_URL helper before treating localhost as live-like.\n' >&2
  exit 1
fi

printf 'No hardcoded production-domain links found in app/template surfaces.\n'
