#!/usr/bin/env python3
"""
Validate that all docs under docs/domains/<domain>/ include the domain short code in the filename.

Rules:
- Each domain has a short code declared in the domain's overview frontmatter as `domain_code: <code>`.
- All Markdown files (except an overview redirect stub) must include `<code>-` as a filename prefix.
- Old names left in place should carry a redirect frontmatter key pointing to the new file.

Exit non-zero with actionable errors if violations are found.
"""
from __future__ import annotations
import sys
import re
import os
import glob
from typing import Dict, Tuple

ROOT = os.path.dirname(os.path.dirname(__file__))
DOCS = os.path.join(ROOT, 'docs', 'domains')

FRONTMATTER_RE = re.compile(r'^---\s*$')
KEYVAL_RE = re.compile(r'^(?P<k>[A-Za-z0-9_\-]+):\s*(?P<v>.*)$')

def read_frontmatter(path: str) -> Dict[str, str]:
    fm: Dict[str, str] = {}
    try:
        with open(path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except Exception:
        return fm

    if not lines or not FRONTMATTER_RE.match(lines[0].strip()):
        return fm
    # find end
    end = None
    for i in range(1, min(len(lines), 200)):
        if FRONTMATTER_RE.match(lines[i].strip()):
            end = i
            break
    if end is None:
        return fm
    for line in lines[1:end]:
        m = KEYVAL_RE.match(line.strip())
        if m:
            fm[m.group('k')] = m.group('v')
    return fm

def find_overview(domain_dir: str) -> Tuple[str | None, Dict[str,str]]:
    # Prefer code-prefixed overview, fall back to overview.md
    candidates = []
    for f in glob.glob(os.path.join(domain_dir, '*.md')):
        base = os.path.basename(f)
        if base in ('overview.md', 'index.md') or base.endswith('-overview.md'):
            candidates.append(f)
    # If none match pattern, try any file with title that looks like overview
    if not candidates:
        for f in glob.glob(os.path.join(domain_dir, '*.md')):
            fm = read_frontmatter(f)
            title = fm.get('title','').lower()
            if 'overview' in title:
                return f, fm
    for f in sorted(candidates):
        return f, read_frontmatter(f)
    return None, {}

def main() -> int:
    errors: list[str] = []
    if not os.path.isdir(DOCS):
        print(f"No docs/domains directory found at {DOCS}")
        return 0

    # Known default codes if frontmatter missing (fallback)
    default_codes = {
        'text-injection': 'ti',
        'stt': 'stt',
        'vad': 'vad',
        'gui': 'gui',
        'audio': 'aud',
        'foundation': 'fdn',
        'telemetry': 'tele',
    }

    for domain in sorted(os.listdir(DOCS)):
        domain_dir = os.path.join(DOCS, domain)
        if not os.path.isdir(domain_dir):
            continue

        overview_path, fm = find_overview(domain_dir)
        code = fm.get('domain_code') or default_codes.get(domain)
        if not code:
            errors.append(f"[{domain}] Missing domain_code in overview frontmatter and no default mapping exists. Add 'domain_code: <code>' to the overview.")
            continue

        # Enforce filenames include code-
        for fn in sorted(os.listdir(domain_dir)):
            if not fn.endswith('.md'):
                continue
            # Allow old names with redirect frontmatter
            path = os.path.join(domain_dir, fn)
            if not fn.startswith(f"{code}-"):
                fm2 = read_frontmatter(path)
                if 'redirect' in fm2:
                    # Old file acting as stub is allowed
                    continue
                # Overview exception if the overview is not yet renamed but has domain_code
                if overview_path and os.path.normpath(path) == os.path.normpath(overview_path):
                    # Encourage rename but don't fail hard if it carries domain_code
                    errors.append(f"[{domain}] Overview file '{fn}' should be renamed to '{code}-overview.md' (has domain_code but old name). Add a redirect stub at the old path.")
                    continue
                errors.append(f"[{domain}] File '{fn}' must include '{code}-' prefix or contain a 'redirect:' frontmatter to the new location.")

    if errors:
        print("Documentation domain naming validation FAILED:\n")
        for e in errors:
            print(f"- {e}")
        return 2
    else:
        print("Documentation domain naming validation PASS")
        return 0

if __name__ == '__main__':
    sys.exit(main())
