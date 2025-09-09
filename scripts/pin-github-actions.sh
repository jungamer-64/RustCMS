#!/usr/bin/env bash
set -euo pipefail

# 必要: GITHUB_TOKEN 環境変数が設定されていること
if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "Set GITHUB_TOKEN (PAT) in the environment first."
  exit 1
fi

ROOT="$(pwd)"
WORKFLOWS=$(git ls-files .github/workflows/*.yml || true)
if [ -z "$WORKFLOWS" ]; then
  echo "No workflow files found under .github/workflows"
  exit 0
fi

echo "Scanning workflows: $WORKFLOWS"
summary_file="$(mktemp)"
echo "Pinned changes summary" > "$summary_file"

for wf in $WORKFLOWS; do
  echo "Processing $wf"
  # extract uses entries like owner/repo@ref (ignore local actions like ./ or repo-relative)
  mapfile -t uses_lines < <(grep -oP '(?<=uses:\s)[^[:space:]]+' "$wf" | grep '/')
  if [ ${#uses_lines[@]} -eq 0 ]; then
    echo "  no external uses found"
    continue
  fi

  cp "$wf" "${wf}.bak"
  modified=0

  for u in "${uses_lines[@]}"; do
    # skip if uses a local action (starts with . or ./)
    if [[ "$u" =~ ^[./] ]]; then
      continue
    fi
    owner_repo="${u%@*}"
    ref="${u#*@}"
    owner="${owner_repo%%/*}"
    repo="${owner_repo#*/}"

    # skip if ref already full sha
    if [[ "$ref" =~ ^[0-9a-f]{40}$ ]]; then
      echo "  $u already pinned"
      continue
    fi

    echo -n "  resolving $owner_repo@$ref ... "
    sha=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" "https://api.github.com/repos/${owner}/${repo}/commits/${ref}" | jq -r '.sha // empty')
    if [ -z "$sha" ]; then
      echo "FAILED"
      echo "  -> Could not resolve ${owner_repo}@${ref}; skipping" >> "$summary_file"
      continue
    fi
    echo "ok -> $sha"

    # perform in-file replacement and add comment preserving indentation
    # find lines like "    - uses: owner/repo@ref" and replace with comment + pinned uses
    escaped_owner_repo_ref="$(printf '%s' "${owner_repo}@${ref}" | sed -e 's/[]\/$*.^[]/\\&/g')"
    escaped_owner_repo_sha="$(printf '%s' "${owner_repo}@${sha}" | sed -e 's/[]\/$*.^[]/\\&/g')"

    # insert comment line with same indentation before the uses line and replace ref
    perl -0777 -pe "s/^([ \\t]*- +uses: +)${escaped_owner_repo_ref}/# original uses: ${owner_repo}@${ref}\n\$1${escaped_owner_repo_sha}/mg" -i "$wf"

    modified=1
    echo "  replaced ${owner_repo}@${ref} -> ${owner_repo}@${sha}" >> "$summary_file"
  done

  if [ "$modified" -eq 1 ]; then
    echo "Modified $wf (backup at ${wf}.bak)"
  else
    rm -f "${wf}.bak"
  fi
done

echo "Done. Summary:"
cat "$summary_file"
echo "You can review changes with: git diff"
echo "If OK: git add <files> && git commit -m 'Pin GitHub Actions to full SHAs'"