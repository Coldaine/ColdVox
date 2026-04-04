#!/usr/bin/env python3
"""Pre-commit hook: Warn about missing frontmatter and offer autofix."""
from __future__ import annotations

import sys
import subprocess
from datetime import date
from pathlib import Path
from typing import Optional

REQUIRED_KEYS = {
    "doc_type",
    "subsystem",
    "version",
    "status",
    "owners",
    "last_reviewed",
}

# Smart defaults based on file path
DOC_TYPE_INFERENCE = {
    "architecture": "architecture",
    "plans": "plan",
    "reference": "reference",
    "research": "research",
    "playbooks": "playbook",
    "tasks": "troubleshooting",
}

SUBSYSTEM_INFERENCE = {
    "audio": "audio",
    "foundation": "foundation",
    "gui": "gui",
    "stt": "stt",
    "text-injection": "text-injection",
    "vad": "vad",
}


def get_git_author() -> str:
    """Get git author name as a hint for owners field."""
    try:
        result = subprocess.run(
            ["git", "config", "user.name"],
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "Documentation Working Group"


def infer_doc_type(path: Path) -> str:
    """Infer doc_type from path."""
    for part in path.parts:
        if part in DOC_TYPE_INFERENCE:
            return DOC_TYPE_INFERENCE[part]
    return "reference"


def infer_subsystem(path: Path) -> str:
    """Infer subsystem from path."""
    # Check for domain paths like docs/domains/audio/
    if "domains" in path.parts:
        idx = path.parts.index("domains")
        if idx + 1 < len(path.parts):
            domain = path.parts[idx + 1]
            if domain in SUBSYSTEM_INFERENCE:
                return SUBSYSTEM_INFERENCE[domain]

    # Check for crate paths like docs/reference/crates/coldvox-audio/
    if "crates" in path.parts:
        idx = path.parts.index("crates")
        if idx + 1 < len(path.parts):
            crate = path.parts[idx + 1]
            # Strip coldvox- prefix
            subsystem = crate.replace("coldvox-", "")
            if subsystem in SUBSYSTEM_INFERENCE:
                return SUBSYSTEM_INFERENCE[subsystem]

    return "general"


def check_frontmatter(path: Path) -> tuple[bool, set[str]]:
    """Check if file has frontmatter with required keys. Returns (has_frontmatter, missing_keys)."""
    try:
        text = path.read_text(encoding="utf-8")
    except (FileNotFoundError, UnicodeDecodeError):
        return False, REQUIRED_KEYS

    if not text.startswith("---\n"):
        return False, REQUIRED_KEYS

    try:
        closing = text.index("\n---", 4)
    except ValueError:
        return False, REQUIRED_KEYS

    header_block = text[4:closing]
    keys = set()
    for line in header_block.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if ":" not in stripped:
            continue
        key = stripped.split(":", 1)[0].strip()
        keys.add(key)

    missing = REQUIRED_KEYS - keys
    return True, missing


def generate_frontmatter(path: Path) -> str:
    """Generate frontmatter block with smart defaults."""
    doc_type = infer_doc_type(path)
    subsystem = infer_subsystem(path)
    author = get_git_author()
    today = date.today().isoformat()

    return f"""---
doc_type: {doc_type}
subsystem: {subsystem}
version: 1.0.0
status: draft
owners: {author}
last_reviewed: {today}
---

"""


def fix_file(path: Path) -> bool:
    """Insert frontmatter if missing. Returns True if fixed."""
    has_fm, missing = check_frontmatter(path)

    if not has_fm:
        # No frontmatter at all - insert it
        content = path.read_text(encoding="utf-8")
        new_content = generate_frontmatter(path) + content
        path.write_text(new_content, encoding="utf-8")
        return True

    if missing:
        # Has frontmatter but missing keys - for now just warn
        # (Adding keys to existing YAML is more complex, skip for now)
        return False

    return False


def main() -> int:
    """Check docs/ markdown files for frontmatter."""
    # Check for --fix flag
    fix_mode = "--fix" in sys.argv
    args = [arg for arg in sys.argv[1:] if arg != "--fix"]

    files = [Path(arg) for arg in args]
    docs_markdown = [f for f in files if f.suffix == ".md" and f.parts[0] == "docs"]

    if not docs_markdown:
        return 0

    issues: list[tuple[Path, set[str]]] = []
    for path in docs_markdown:
        has_fm, missing = check_frontmatter(path)
        if not has_fm or missing:
            issues.append((path, missing))

    if not issues:
        if fix_mode:
            print("✅ All files have complete frontmatter!")
        return 0

    if fix_mode:
        # Auto-fix mode
        print("\n🔧 Auto-fixing frontmatter...\n")
        fixed_count = 0
        for path, missing in issues:
            if missing == REQUIRED_KEYS:
                # No frontmatter at all - we can fix this
                if fix_file(path):
                    print(f"  ✅ Added frontmatter to {path}")
                    fixed_count += 1
            else:
                print(f"  ⚠️  {path}: Has partial frontmatter, missing: {', '.join(sorted(missing))}")
                print(f"     Please manually add these keys.")

        if fixed_count > 0:
            print(f"\n✅ Fixed {fixed_count} file(s)! Review changes and re-stage them.")
        return 0

    # Warning mode (pre-commit)
    print("\n⚠️  WARNING: Documentation files missing required frontmatter:\n")
    for path, missing in issues:
        if missing == REQUIRED_KEYS:
            print(f"  - {path}: No frontmatter block")
        else:
            print(f"  - {path}: Missing keys: {', '.join(sorted(missing))}")

    print("\n📋 Required frontmatter keys:")
    print("   - doc_type: Type of document (architecture, plan, reference, etc.)")
    print("   - subsystem: Which subsystem this relates to (general, audio, gui, etc.)")
    print("   - version: Document version (usually 1.0.0 for new docs)")
    print("   - status: Document status (draft, active, deprecated)")
    print("   - owners: Who maintains this (your name or team)")
    print("   - last_reviewed: Date last reviewed (YYYY-MM-DD)")
    print("\n🔧 Auto-fix available! Run:")
    print(f"   python3 scripts/ensure_doc_frontmatter.py --fix {' '.join(str(p) for p, _ in issues)}")
    print("\n   Or manually add frontmatter following the template in docs/standards.md")

    # Don't block - just warn
    print("\n✅ This is a warning only. Commit will proceed.")
    print("   Please add frontmatter before pushing to avoid CI failure.\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
