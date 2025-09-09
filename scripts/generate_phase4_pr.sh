#!/usr/bin/env bash
set -euo pipefail

# Generates a draft Phase 4 removal PR markdown after verifying src-only scan = zero.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

JSON=$(bash scripts/deprecation-scan.sh --src-only --json)
NONZERO=$(echo "$JSON" | tr '{}' '\n' | tr ',' '\n' | grep ':' | awk -F: '{if ($2+0>0) print $0}') || true

if [ -n "$NONZERO" ]; then
  echo "[phase4-pr] ❌ Cannot generate PR draft; deprecated references remain:" >&2
  echo "$NONZERO" >&2
  exit 1
fi

OUT=PHASE4_REMOVAL_PR_DRAFT.md
cat > "$OUT" <<'MD'
# Phase 4: Remove Deprecated Auth Flat Fields & Legacy LoginResponse

## Summary
All deprecated flattened auth token fields and legacy LoginResponse references have been eliminated from production source. This PR removes backward-compatibility surfaces slated for 3.0.0.

## Removals
- Cargo feature `auth-flat-fields`
- Cargo feature `legacy-auth-flat`
- Struct fields (deprecated): access_token, refresh_token, biscuit_token, expires_in, session_id, token (top-level in AuthSuccessResponse)
- Struct `LoginResponse`

## Migration Confirmation
- SDKs already updated to use `response.tokens.*`
- No remaining references (validated by `scripts/deprecation-scan.sh --src-only --json`)
- Metrics counters: `auth_flat_fields_legacy_usage_total` & `legacy_login_response_conversion_total` stable at 0 for N days (fill in)

## Checklist
- [ ] Remove feature flags from `[features]` in Cargo.toml
- [ ] Delete flattened fields from `AuthSuccessResponse` implementation
- [ ] Delete `LoginResponse` and related OpenAPI conditional
- [ ] Remove legacy sections from README & docs/AUTH_MIGRATION_V2.md (retain historical note in CHANGELOG)
- [ ] Update OpenAPI examples (remove notes about deprecated fields)
- [ ] Regenerate and commit OpenAPI artifacts if applicable
- [ ] Bump version: 3.0.0
- [ ] Update CHANGELOG with breaking changes section
- [ ] Run full CI matrix (default + minimal) green

## Validation Commands
```bash
bash scripts/deprecation-scan.sh --src-only --json
cargo test --all --no-fail-fast
```

## Rollout / Communication Plan
- Publish release notes highlighting removal & migration path
- Pin previous minor for consumers needing temporary overlap

## Post-merge Follow-ups
- [ ] Remove metrics dashboard panels tied to deprecated counters after grace period (optional)
- [ ] Archive PHASE4_REMOVAL_PR_DRAFT.md (auto-deleted by PR merge)
MD

echo "[phase4-pr] Draft created: $OUT" >&2
exit 0
