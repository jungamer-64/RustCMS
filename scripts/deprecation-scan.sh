#!/usr/bin/env bash
set -euo pipefail

# Deprecation scan utility
# Purpose:
#   Surfacing remaining usages of soon-to-be removed flattened auth response fields
#   and legacy LoginResponse struct before 3.0.0 removal.
# Output modes:
#   default  -> lists each match with file:line
#   --counts -> per pattern counts only (also enabled via SCAN_COUNTS=1)
# Exit code:
#   Always 0 (informational) unless --strict is passed AND any matches exist.
# Usage examples:
#   scripts/deprecation-scan.sh
#   scripts/deprecation-scan.sh --counts
#   scripts/deprecation-scan.sh --strict

STRICT=0
COUNTS_MODE=0
SRC_ONLY=0
JSON_MODE=0
for arg in "$@"; do
  case "$arg" in
    --counts) COUNTS_MODE=1 ;;
    --strict) STRICT=1 ;;
    --src-only) SRC_ONLY=1 ;;
    --json) JSON_MODE=1 ;;
    *) echo "[deprecation-scan] unknown arg: $arg" >&2; exit 2 ;;
  esac
done

if [ "${SCAN_COUNTS:-0}" = "1" ]; then
  COUNTS_MODE=1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[deprecation-scan] scanning for deprecated auth flattened fields & legacy login schema..." >&2

PATTERNS=(
  "\\.access_token" \
  "\\.refresh_token" \
  "\\.biscuit_token" \
  "\\.expires_in" \
  "\\.session_id" \
  "\\.token" \
  "LoginResponse"
)

IGNORE_DIRS=(target uploads .git .github/ISSUE_TEMPLATE)

IGNORE_EXPR="$(IFS='|'; echo "${IGNORE_DIRS[*]// /|}")"

matches_found=0

declare -A COUNTS
for pat in "${PATTERNS[@]}"; do
  if rg --version >/dev/null 2>&1; then
    # Use ripgrep, ignore directories, skip binary, no colors.
    OUT=$(rg -n --color=never -e "$pat" \
      --glob '!target' --glob '!uploads' --glob '!.git' --glob '!.github/ISSUE_TEMPLATE' || true)
  else
    # Grep fallback: -I to ignore binary, prune ignored dirs.
    OUT=$(grep -R -n -I -E "$pat" . \
      | grep -vE "$IGNORE_EXPR" || true)
  fi

    if [ -n "$OUT" ]; then
      # Optional src-only filter (keep only ./src/ paths)
      if [ $SRC_ONLY -eq 1 ]; then
        OUT=$(printf '%s\n' "$OUT" | grep '^src/' || true)
      fi
    fi

    if [ -n "$OUT" ]; then
      matches_found=1
      if [ $COUNTS_MODE -eq 0 ] && [ $JSON_MODE -eq 0 ]; then
        echo "--- pattern: $pat"; echo "$OUT"; echo
      fi
      COUNT=$(printf '%s\n' "$OUT" | grep -c . || true)
      COUNTS["$pat"]=$COUNT
    else
      COUNTS["$pat"]=0
    fi
done

  if [ $COUNTS_MODE -eq 1 ] && [ $JSON_MODE -eq 0 ]; then
    echo "pattern,count"
    for pat in "${PATTERNS[@]}"; do
      echo "$pat,${COUNTS[$pat]}"
    done
    echo
  fi

  if [ $JSON_MODE -eq 1 ]; then
    printf '{'
    first=1
    for pat in "${PATTERNS[@]}"; do
      if [ $first -eq 0 ]; then printf ','; fi
      first=0
      printf '"%s":%s' "$pat" "${COUNTS[$pat]}"
    done
    printf '}'
    echo
  fi

if [ $matches_found -eq 1 ]; then
  echo "[deprecation-scan] NOTE: occurrences inside tests or docs can be acceptable; production src/ should migrate to tokens.*" >&2
else
  echo "[deprecation-scan] ✅ no matches found (good candidate for removing flat fields)" >&2
fi

echo "[deprecation-scan] Hint: run under no-flat CI matrix (AUTH_FLAT=0) to ensure builds without deprecated fields." >&2
if [ $SRC_ONLY -eq 1 ]; then
  echo "[deprecation-scan] (src-only mode) counts limited to production source tree" >&2
fi

if [ $STRICT -eq 1 ] && [ $matches_found -eq 1 ]; then
  exit 1
fi

exit 0
