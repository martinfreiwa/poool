#!/usr/bin/env bash
# Master runner: every community-feature e2e bash script, sequential.
# Aggregates pass/fail across all suites + exits non-zero on any failure.

set -u
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

SUITES=(
  "test_feed_e2e.sh"
  "test_follow_e2e.sh"
  "test_dm_e2e.sh"
  "test_block_mute_e2e.sh"
  "test_notifications_e2e.sh"
  "test_bookmarks_e2e.sh"
  "test_low_priority_e2e.sh"
)

TOTAL_PASS=0
TOTAL_FAIL=0
FAILED_SUITES=()

for suite in "${SUITES[@]}"; do
  printf "\n\033[1;35m═══ %s ═══\033[0m\n" "$suite"
  OUT=$("$SCRIPT_DIR/$suite" 2>&1) || true
  # Parse "passed: N" and "failed: N" from the suite summary block.
  # Strip ANSI color sequences before parsing the summary block.
  CLEAN=$(printf '%s\n' "$OUT" | sed -E 's/\x1b\[[0-9;]*m//g')
  PASS=$(printf '%s\n' "$CLEAN" | awk '/passed:/ {gsub(/[^0-9]/,"",$NF); print $NF; exit}')
  FAIL=$(printf '%s\n' "$CLEAN" | awk '/failed:/ {gsub(/[^0-9]/,"",$NF); print $NF; exit}')
  PASS="${PASS:-0}"
  FAIL="${FAIL:-0}"
  TOTAL_PASS=$((TOTAL_PASS + PASS))
  TOTAL_FAIL=$((TOTAL_FAIL + FAIL))
  if [ "$FAIL" -gt 0 ] || [ "$PASS" -eq 0 ]; then
    FAILED_SUITES+=("$suite ($PASS✓ / $FAIL✗)")
    printf '%s\n' "$OUT" | tail -15
  else
    printf "  \033[0;32m✓ %s: %d passed\033[0m\n" "$suite" "$PASS"
  fi
done

echo
printf "\033[1m═══ TOTAL ═══\033[0m\n"
printf "  \033[0;32mpassed:\033[0m %d\n" "$TOTAL_PASS"
printf "  \033[0;31mfailed:\033[0m %d\n" "$TOTAL_FAIL"
if [ "${#FAILED_SUITES[@]}" -gt 0 ]; then
  printf "  \033[0;31mfailed suites:\033[0m\n"
  for s in "${FAILED_SUITES[@]}"; do printf "    - %s\n" "$s"; done
fi
[ "$TOTAL_FAIL" -eq 0 ] && exit 0 || exit 1
