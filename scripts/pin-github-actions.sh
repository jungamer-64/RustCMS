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
    sha=""
    # try commit endpoint (works for branches/tags/SHAs)
  resp=$(curl -s -w "\n%{http_code}" -H "Authorization: token ${GITHUB_TOKEN}" "https://api.github.com/repos/${owner}/${repo}/commits/${ref}") || resp=""
  body=$(printf '%s' "$resp" | head -n -1)
  code=$(printf '%s' "$resp" | tail -n1)
    if [ "$code" = "200" ]; then
      sha=$(printf '%s' "$body" | jq -r '.sha // empty') || sha=""
    fi

    # fallback: try git ref for tags (refs/tags/<ref>)
    if [ -z "$sha" ]; then
  resp=$(curl -s -w "\n%{http_code}" -H "Authorization: token ${GITHUB_TOKEN}" "https://api.github.com/repos/${owner}/${repo}/git/ref/tags/${ref}") || resp=""
  body=$(printf '%s' "$resp" | head -n -1)
  code=$(printf '%s' "$resp" | tail -n1)
      if [ "$code" = "200" ]; then
        # object may be tag object or commit
        obj_sha=$(printf '%s' "$body" | jq -r '.object.sha // empty') || obj_sha=""
        # if annotated tag, need to dereference
        if [ -n "$obj_sha" ]; then
          # fetch the object to get commit sha
          obj_resp=$(curl -s -H "Authorization: token ${GITHUB_TOKEN}" "https://api.github.com/repos/${owner}/${repo}/git/tags/${obj_sha}") || obj_resp=""
          obj_type=$(printf '%s' "$obj_resp" | jq -r '.object.type // empty' 2>/dev/null || true)
          if [ -n "$obj_type" ] && [ "$obj_type" = "commit" ]; then
            sha=$(printf '%s' "$obj_resp" | jq -r '.object.sha // empty') || sha=""
          else
            sha="$obj_sha"
          fi
        fi
      fi
    fi

    # final fallback: try releases/tags endpoint
    if [ -z "$sha" ]; then
  resp=$(curl -s -w "\n%{http_code}" -H "Authorization: token ${GITHUB_TOKEN}" "https://api.github.com/repos/${owner}/${repo}/releases/tags/${ref}") || resp=""
  body=$(printf '%s' "$resp" | head -n -1)
  code=$(printf '%s' "$resp" | tail -n1)
      if [ "$code" = "200" ]; then
        # releases endpoint does not return commit sha directly; try tag_name -> git/refs/tags
        rel_tag=$(printf '%s' "$body" | jq -r '.tag_name // empty') || rel_tag=""
        if [ -n "$rel_tag" ]; then
          resp2=$(curl -s -w "\n%{http_code}" -H "Authorization: token ${GITHUB_TOKEN}" "https://api.github.com/repos/${owner}/${repo}/git/ref/tags/${rel_tag}") || resp2=""
          body2=$(printf '%s' "$resp2" | sed -n '1,$p' | sed '\$d')
          code2=$(printf '%s' "$resp2" | tail -n1)
          if [ "$code2" = "200" ]; then
            sha=$(printf '%s' "$body2" | jq -r '.object.sha // empty') || sha=""
          fi
        fi
      fi
    fi

    if [ -z "$sha" ]; then
      echo "FAILED"
      echo "  -> Could not resolve ${owner_repo}@${ref}; skipping" >> "$summary_file"
      continue
    fi
    echo "ok -> $sha"

    # perform in-file replacement and add comment preserving indentation
    # find lines like "    - uses: owner/repo@ref" and replace with comment + pinned uses
    # escape only slash and ampersand for safe use in perl substitution
    escaped_owner_repo_ref="$(printf '%s' "${owner_repo}@${ref}" | sed -e 's/[\/&]/\\&/g')"
    escaped_owner_repo_sha="$(printf '%s' "${owner_repo}@${sha}" | sed -e 's/[\/&]/\\&/g')"

      # insert comment line with same indentation before the uses line and replace ref
      # We will rewrite the file safely: for each line, if it matches the exact uses pattern,
      # print a comment line then the replaced uses line; otherwise print the line unchanged.
      tmpfile="${wf}.tmp"
      awk -v target="${owner_repo}@${ref}" -v replacement="${owner_repo}@${sha}" '
        { if (index($0, "uses:") && index($0, target)) {
            match($0, /^([ \t]*)-[ \t]*uses:[ \t]*/, m);
            indent=(m[1] ? m[1] : "");
            print indent "# original uses: " target;
            pos = index($0, target);
            prefix = substr($0, 1, pos-1);
            suffix = substr($0, pos + length(target));
            print prefix replacement suffix;
          } else print $0 }
      ' "$wf" > "$tmpfile" && mv "$tmpfile" "$wf"

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