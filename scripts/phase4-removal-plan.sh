#!/usr/bin/env bash
set -euo pipefail

echo "[phase4] Scanning for deprecated auth flatten & legacy artifacts"

echo "--- Flattened auth field references (excluding canonical definition) ---"
grep -R -n -E '\b(access_token|refresh_token|biscuit_token|expires_in|session_id|token)\b' src/ \
  | grep -v 'auth_response.rs' || true

echo "--- LoginResponse references (should only exist under feature gate) ---"
grep -R -n 'LoginResponse' src/ || true

echo "--- legacy admin token references ---"
grep -R -n 'legacy-admin-token' ./Cargo.toml src/ || true

echo "--- Summary guidance ---"
cat <<'EOS'
Planned Phase 4 (3.0.0) removal steps:
1. Remove feature gates: legacy-auth-flat, auth-flat-fields, legacy-admin-token.
2. Delete flattened fields from AuthSuccessResponse & related tests (auth_success_flatten_*).
3. Remove LoginResponse struct & conditional OpenAPI insertion.
4. Purge admin token utilities and feature, ensure Biscuit permissions cover admin flows.
5. Drop CI deprecated-scan job or repurpose for other deprecations.
6. Update docs: AUTH_MIGRATION_V2.md (mark complete), CHANGELOG breaking section.
7. Run this script again; expect zero matches except historical docs/changelog.
EOS

echo "[phase4] Scan complete"