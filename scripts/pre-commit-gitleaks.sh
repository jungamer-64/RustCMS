#!/usr/bin/env bash
set -euo pipefail
# Run gitleaks detect on the repo root and fail commit if anything found
GITLEAKS_CMD="gitleaks detect --source . --report-format json --report-path security/gitleaks-pre-commit.json"
echo "Running gitleaks pre-commit check..."
mkdir -p security
if command -v gitleaks >/dev/null 2>&1; then
  if $GITLEAKS_CMD 2> security/gitleaks-pre-commit.log; then
    echo "No secrets detected by gitleaks."
    exit 0
  else
    echo "gitleaks detected potential secrets. See security/gitleaks-pre-commit.json and security/gitleaks-pre-commit.log"
    echo "Aborting commit. If this is a false positive, update .gitleaks.toml or the docs and try again."
    exit 1
  fi
else
  echo "gitleaks is not installed. To enable pre-commit scanning, install gitleaks (https://github.com/zricethezav/gitleaks) or skip pre-commit hook." >&2
  # Do not block commit if gitleaks isn't installed locally; only warn.
  exit 0
fi
