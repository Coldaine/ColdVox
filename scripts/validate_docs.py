#!/usr/bin/env python3
"""Validate documentation changes against the Master Documentation Playbook policies."""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import Iterable, List, Tuple

REQUIRED_KEYS = {
    "doc_type",
    "subsystem",
    "version",
    "status",
    "owners",
    "last_reviewed",
}

APPROVED_EXCEPTIONS = {
    Path("README.md"),
    Path("CHANGELOG.md"),
    Path("CLAUDE.md"),
}


def git_diff_files(base: str, head: str) -> List[str]:
    """Get list of added or modified files (not deleted ones)."""
    result = subprocess.run(
        ["git", "diff", "--name-status", "--diff-filter=AM", base, head],
        capture_output=True,
        text=True,
        check=True,
    )
    # Extract just the file paths (skip the status column)
    return [line.split(None, 1)[1].strip() for line in result.stdout.splitlines() if line.strip()]


def load_file(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def check_frontmatter(path: Path) -> Tuple[bool, str]:
    text = load_file(path)
    if not text:
        return False, "file missing or empty"
    if not text.startswith("---\n"):
        return False, "missing frontmatter delimiter"
    try:
        closing = text.index("\n---", 4)
    except ValueError:
        return False, "frontmatter not terminated"
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
    if missing:
        return False, f"missing keys: {', '.join(sorted(missing))}"
    return True, ""


def lint_mermaid(path: Path) -> Tuple[bool, str]:
    text = load_file(path)
    if not text:
        return True, ""
    errors: List[str] = []
    lines = text.splitlines()
    in_block = False
    block_start = 0
    for idx, line in enumerate(lines, start=1):
        if line.strip().startswith("```mermaid") and not in_block:
            in_block = True
            block_start = idx
            continue
        if line.strip().startswith("```") and in_block:
            block_content = lines[block_start:idx - 1]
            if not any(l.strip() for l in block_content):
                errors.append(f"empty mermaid block starting at line {block_start}")
            in_block = False
    if in_block:
        errors.append(f"unterminated mermaid block starting at line {block_start}")
    if errors:
        return False, "; ".join(errors)
    return True, ""


def check_crate_index(path: Path) -> Tuple[bool, str]:
    text = load_file(path)
    if "crates/" not in text or "README.md" not in text:
        return False, "crate index missing README link"
    return True, ""


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base")
    parser.add_argument("head")
    args = parser.parse_args()

    changed = [Path(p) for p in git_diff_files(args.base, args.head)]

    errors: List[str] = []
    warnings: List[str] = []

    docs_changed = [p for p in changed if p.suffix == ".md" and p.parts[0] == "docs"]
    outside_markdown = [p for p in changed if p.suffix == ".md" and p.parts[0] != "docs"]

    for path in outside_markdown:
        if path in APPROVED_EXCEPTIONS:
            continue
        errors.append(f"Markdown outside /docs is not allowed: {path}")

    for path in docs_changed:
        ok, message = check_frontmatter(path)
        if not ok:
            errors.append(f"Frontmatter error in {path}: {message}")
        ok, message = lint_mermaid(path)
        if not ok:
            errors.append(f"Mermaid lint error in {path}: {message}")
        if Path("docs/reference/crates") in path.parents:
            ok, message = check_crate_index(path)
            if not ok:
                errors.append(f"Crate index error in {path}: {message}")

    tasks_changes = [p for p in docs_changed if Path("docs/tasks") in p.parents]
    if tasks_changes and Path("docs/todo.md") not in docs_changed:
        warnings.append(
            "Changes detected under docs/tasks/ without a corresponding docs/todo.md update."
        )

    for warning in warnings:
        print(f"::warning ::{warning}")

    if errors:
        for error in errors:
            print(f"::error ::{error}")
        sys.exit(1)


if __name__ == "__main__":
    main()
