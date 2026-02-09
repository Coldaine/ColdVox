#!/usr/bin/env python3
"""Pre-commit hook: Warn about missing frontmatter and offer autofix."""

from __future__ import annotations

import sys
from datetime import date, timedelta
from pathlib import Path

CORE_KEYS = {"doc_type", "subsystem", "status"}
INFO_KEYS = {"freshness", "preservation"}

DOC_TYPE_INFERENCE = {
    "architecture": "architecture",
    "plans": "plan",
    "reference": "reference",
    "research": "research",
    "playbooks": "playbook",
    "tasks": "troubleshooting",
    "history": "history",
}

SUBSYSTEM_INFERENCE = {
    "audio": "audio",
    "foundation": "foundation",
    "gui": "gui",
    "stt": "stt",
    "text-injection": "text-injection",
    "vad": "vad",
    "telemetry": "telemetry",
}


def infer_doc_type(path: Path) -> str:
    for part in path.parts:
        if part in DOC_TYPE_INFERENCE:
            return DOC_TYPE_INFERENCE[part]
    return "reference"


def infer_subsystem(path: Path) -> str:
    if "domains" in path.parts:
        idx = path.parts.index("domains")
        if idx + 1 < len(path.parts):
            domain = path.parts[idx + 1]
            if domain in SUBSYSTEM_INFERENCE:
                return SUBSYSTEM_INFERENCE[domain]
    if "crates" in path.parts:
        idx = path.parts.index("crates")
        if idx + 1 < len(path.parts):
            crate = path.parts[idx + 1]
            subsystem = crate.replace("coldvox-", "")
            if subsystem in SUBSYSTEM_INFERENCE:
                return SUBSYSTEM_INFERENCE[subsystem]
    return "general"


def check_frontmatter(path: Path) -> tuple[bool, set[str]]:
    """Check if file has frontmatter with required keys."""
    try:
        text = path.read_text(encoding="utf-8")
    except (FileNotFoundError, UnicodeDecodeError):
        return False, CORE_KEYS | INFO_KEYS

    if not text.startswith("---\n"):
        return False, CORE_KEYS | INFO_KEYS

    try:
        closing = text.index("\n---", 4)
    except ValueError:
        return False, CORE_KEYS | INFO_KEYS

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

    missing = (CORE_KEYS | INFO_KEYS) - keys
    return True, missing


def generate_frontmatter(path: Path) -> str:
    doc_type = infer_doc_type(path)
    subsystem = infer_subsystem(path)
    freshness = (
        "historical" if "archive" in path.parts or "history" in path.parts else "stale"
    )
    preservation = "preserve"

    if "archive" in path.parts:
        status = "archived"
    else:
        status = "draft"

    return f"""---
doc_type: {doc_type}
subsystem: {subsystem}
status: {status}
freshness: {freshness}
preservation: {preservation}
---

"""


def fix_file(path: Path) -> bool:
    has_fm, missing = check_frontmatter(path)

    if not has_fm:
        content = path.read_text(encoding="utf-8")
        new_content = generate_frontmatter(path) + content
        path.write_text(new_content, encoding="utf-8")
        return True

    return False


def main() -> int:
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
            print("[OK] All files have complete frontmatter!")
        return 0

    if fix_mode:
        print("\n[FIX] Auto-fixing frontmatter...\n")
        fixed_count = 0
        for path, missing in issues:
            if missing == CORE_KEYS | INFO_KEYS:
                if fix_file(path):
                    print(f"  [OK] Added frontmatter to {path}")
                    fixed_count += 1
            else:
                print(
                    f"  [WARN] {path}: Has partial frontmatter, missing: {', '.join(sorted(missing))}"
                )
                print(f"     Please manually add these keys.")

        if fixed_count > 0:
            print(
                f"\n[OK] Fixed {fixed_count} file(s)! Review changes and re-stage them."
            )
        return 0

    print("\n[WARN] WARNING: Documentation files missing required frontmatter:\n")
    for path, missing in issues:
        if missing == CORE_KEYS | INFO_KEYS:
            print(f"  - {path}: No frontmatter block")
        else:
            print(f"  - {path}: Missing keys: {', '.join(sorted(missing))}")

    print("\n[INFO] Required frontmatter keys:")
    print("   - doc_type: Type of document (architecture, plan, reference, etc.)")
    print("   - subsystem: Which subsystem this relates to (general, audio, gui, etc.)")
    print("   - status: Document status (draft, active, archived)")
    print("   - freshness: Accuracy state (current, aging, stale, historical, dead)")
    print(
        "   - preservation: Value classification (reference, preserve, summarize, delete)"
    )
    print("\n[FIX] Auto-fix available! Run:")
    print(
        f"   python scripts/ensure_doc_frontmatter.py --fix {' '.join(str(p) for p, _ in issues)}"
    )

    print("\n[OK] This is a warning only. Commit will proceed.")
    print("   Please add frontmatter before pushing to avoid CI failure.\n")

    return 0


if __name__ == "__main__":
    sys.exit(main())
