#!/usr/bin/env bash
set -euo pipefail

# Guidance script: decides whether to recommend enabling strict mode in CI.
# Logic: run src-only scan; if all counts zero for 3 consecutive cached runs (tracked in .tmp file), suggest CI change.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_FILE="${ROOT_DIR}/target/.deprecation_zero_streak"
mkdir -p "${ROOT_DIR}/target" || true

JSON=$(bash "${ROOT_DIR}/scripts/deprecation-scan.sh" --src-only --json)
echo "[auto-guidance] scan json: $JSON" >&2

NONZERO=$(echo "$JSON" | tr '{}' '\n' | tr ',' '\n' | grep ':' | awk -F: '{if ($2+0>0) print $0}') || true

if [ -n "$NONZERO" ]; then
  echo 0 > "$STATE_FILE"
  echo "[auto-guidance] Deprecated refs still present. Zero streak reset." >&2
  exit 0
fi

STREAK=0
if [ -f "$STATE_FILE" ]; then
  STREAK=$(cat "$STATE_FILE" || echo 0)
fi
STREAK=$((STREAK+1))
echo $STREAK > "$STATE_FILE"
echo "[auto-guidance] Zero streak: $STREAK" >&2

if [ $STREAK -ge 3 ]; then
  cat <<EOF
All deprecated auth flat field references have been zero for $STREAK consecutive runs.\n
Recommended next steps:\n  1. Add a CI step after tests: \n     bash scripts/deprecation-strict-check.sh\n  2. Remove 'auth-flat-fields' from default features (and eventually delete code).\n  3. Remove 'legacy-auth-flat' feature & LoginResponse schema.\n  4. Regenerate OpenAPI clients and announce in CHANGELOG (Phase 4).\nEOF
fi

exit 0
