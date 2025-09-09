#!/usr/bin/env bash
set -euo pipefail

# Fails (exit 1) if any deprecated flattened auth fields / legacy LoginResponse references
# remain in production source (src/) tree.
# Intended to be wired into CI once counts are expected to be zero (Phase 4 readiness).

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

JSON=$(bash scripts/deprecation-scan.sh --src-only --json)
echo "[strict-check] scan json: $JSON" >&2

# Parse without jq (avoid dependency). Expect format: {"pattern":N,...}
NONZERO=$(echo "$JSON" | tr '{}' '\n' | tr ',' '\n' | grep ':' | awk -F: '{gsub(/"/,"",$1); if ($2+0>0) print $0}') || true

if [ -n "$NONZERO" ]; then
  echo "[strict-check] ❌ Non-zero deprecated references detected:" >&2
  echo "$NONZERO" >&2
  exit 1
fi

echo "[strict-check] ✅ All deprecated auth flat field references eliminated in src/ (safe to remove feature flags)." >&2
exit 0
