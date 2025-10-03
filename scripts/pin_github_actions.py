#!/usr/bin/env python3
"""Resolve GitHub Action refs to full commit SHAs and pin workflows.

Usage: export GITHUB_TOKEN=...; python3 scripts/pin_github_actions.py
"""
import os
import re
import sys
import json
from urllib.request import Request, urlopen
from urllib.error import HTTPError

TOKEN = os.environ.get('GITHUB_TOKEN')
if not TOKEN:
    print('Set GITHUB_TOKEN in env first', file=sys.stderr)
    sys.exit(1)

WORKFLOW_GLOB = '.github/workflows'


def gh_api_get(url):
    # Security: Validate URL to prevent file:// scheme injection
    if not url.startswith('https://api.github.com/'):
        raise ValueError(f'Invalid GitHub API URL: {url}')

    req = Request(url)
    req.add_header('Authorization', f'token {TOKEN}')
    req.add_header('Accept', 'application/vnd.github.v3+json')
    try:
        with urlopen(req, timeout=20) as resp:
            return resp.read().decode('utf-8'), resp.getcode()
    except HTTPError as e:
        try:
            body = e.read().decode('utf-8')
        except Exception:
            body = ''
        return body, e.code
    except Exception as e:
        return str(e), None


def resolve_sha(owner, repo, ref):
    """Try common GitHub API endpoints to resolve a ref to a commit SHA."""
    # try commits endpoint (works for branches, tags, and SHAs)
    body, code = gh_api_get(f'https://api.github.com/repos/{owner}/{repo}/commits/{ref}')
    if code == 200:
        try:
            return json.loads(body).get('sha')
        except Exception:
            pass
    # try git ref tags (handles lightweight and annotated tags)
    body, code = gh_api_get(f'https://api.github.com/repos/{owner}/{repo}/git/ref/tags/{ref}')
    if code == 200:
        try:
            obj = json.loads(body).get('object', {})
            obj_sha = obj.get('sha')
            if obj_sha:
                # if annotated tag, fetch tag object to resolve to commit
                body2, code2 = gh_api_get(f'https://api.github.com/repos/{owner}/{repo}/git/tags/{obj_sha}')
                if code2 == 200:
                    try:
                        o = json.loads(body2).get('object', {})
                        return o.get('sha') or obj_sha
                    except Exception:
                        return obj_sha
                return obj_sha
        except Exception:
            pass
    # try releases by tag name
    body, code = gh_api_get(f'https://api.github.com/repos/{owner}/{repo}/releases/tags/{ref}')
    if code == 200:
        try:
            tag = json.loads(body).get('tag_name')
            if tag:
                body2, code2 = gh_api_get(f'https://api.github.com/repos/{owner}/{repo}/git/ref/tags/{tag}')
                if code2 == 200:
                    try:
                        obj_sha = json.loads(body2).get('object', {}).get('sha')
                        return obj_sha
                    except Exception:
                        pass
        except Exception:
            pass
    return None


def process_file(path):
    changed = False
    with open(path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # match lines like:
    #   - uses: owner/repo@ref
    #   uses: owner/repo@ref
    #   <indent>- uses: owner/repo@ref
    pattern = re.compile(r'^(?P<indent>\s*)(?P<prefix>-?\s*uses:\s*)(?P<action>[^@\s]+)@(?P<ref>\S+)', re.IGNORECASE)
    out = []
    for ln in lines:
        m = pattern.match(ln)
        if not m:
            out.append(ln)
            continue
        action = m.group('action')
        ref = m.group('ref')
        indent = m.group('indent')
        # prefix = m.group('prefix')  # Not used currently, reserved for future use
        # local actions like ./ or . should be skipped
        if action.startswith('.') or action.startswith('./'):
            out.append(ln)
            continue
        # already pinned to full sha
        if re.fullmatch(r'[0-9a-f]{40}', ref, re.IGNORECASE):
            out.append(ln)
            continue
        # not a typical owner/repo action (skip)
        if '/' not in action:
            out.append(ln)
            continue
        owner, repo = action.split('/', 1)
        print(f'Resolving {action}@{ref} ... ', end='', flush=True)
        sha = resolve_sha(owner, repo, ref)
        if not sha:
            print('FAILED')
            out.append(ln)
            continue
        print(sha)
        # insert comment with original and replace the ref with the sha
        out.append(f"{indent}# original uses: {action}@{ref}\n")
        # replace only the first @ref occurrence on the line
        replaced = re.sub(r'@(\S+)', f'@{sha}', ln, count=1)
        out.append(replaced)
        changed = True

    if changed:
        # backup original
        with open(path + '.bak', 'w', encoding='utf-8') as b:
            b.writelines(lines)
        with open(path, 'w', encoding='utf-8') as f:
            f.writelines(out)
    return changed


def main():
    import glob
    files = glob.glob(os.path.join(WORKFLOW_GLOB, '*.yml'))
    if not files:
        print('No workflow files')
        return
    any_changed = False
    for p in files:
        print('Processing', p)
        changed = process_file(p)
        if changed:
            any_changed = True
            print('Modified', p)
    if not any_changed:
        print('No changes')


if __name__ == '__main__':
    main()
