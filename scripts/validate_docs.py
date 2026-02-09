#!/usr/bin/env python3
"""Validate documentation changes against the Master Documentation Playbook policies."""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import re
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple

CORE_KEYS = {"doc_type", "subsystem", "status"}
OPTIONAL_KEYS = {"version", "owners", "last_reviewed", "last_reviewer", "review_due"}
INFO_KEYS = {"freshness", "preservation"}

DATE_PATTERN = re.compile(r"^\d{4}-\d{2}-\d{2}$")

GRACE_PERIOD_END = dt.date(2026, 3, 11)

VALID_FRESHNESS = {"current", "aging", "stale", "historical", "dead"}
VALID_PRESERVATION = {"reference", "preserve", "summarize", "delete"}

APPROVED_EXCEPTIONS = {
    Path("README.md"),
    Path("CHANGELOG.md"),
    Path("AGENTS.md"),
    Path("CLAUDE.md"),
    Path("CODEX.md"),
    Path("COPILOT.md"),
    Path(".github/pull_request_template.md"),
}

APPROVED_PATTERNS = [
    "crates/*/README.md",
    ".github/**/*.md",
]


def git_diff_files(base: str, head: str) -> List[str]:
    """Get list of added or modified files (not deleted ones)."""
    result = subprocess.run(
        ["git", "diff", "--name-status", "--diff-filter=AM", base, head],
        capture_output=True,
        text=True,
        check=True,
    )
    return [
        line.split(None, 1)[1].strip()
        for line in result.stdout.splitlines()
        if line.strip()
    ]


def load_file(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def parse_frontmatter(path: Path) -> Tuple[bool, str, dict]:
    text = load_file(path)
    if not text:
        return False, "file missing or empty", {}
    if not text.startswith("---\n"):
        return False, "missing frontmatter delimiter", {}
    try:
        closing = text.index("\n---", 4)
    except ValueError:
        return False, "frontmatter not terminated", {}
    header_block = text[4:closing]
    values: dict = {}
    for line in header_block.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if ":" not in stripped:
            continue
        key, raw_value = stripped.split(":", 1)
        values[key.strip()] = raw_value.strip().strip("'\"")

    return True, "", values


def check_frontmatter(path: Path) -> Tuple[bool, List[str], List[str], dict]:
    """Check frontmatter. Returns (ok, errors, warnings, values)."""
    ok, message, values = parse_frontmatter(path)
    if not ok:
        return False, [message], [], values

    keys = set(values.keys())
    errors: List[str] = []
    warnings: List[str] = []

    missing_core = CORE_KEYS - keys
    if missing_core:
        errors.append(f"missing required keys: {', '.join(sorted(missing_core))}")

    missing_info = INFO_KEYS - keys
    if missing_info:
        if dt.date.today() >= GRACE_PERIOD_END:
            errors.append(
                f"missing informational keys: {', '.join(sorted(missing_info))} (grace period ended)"
            )
        else:
            warnings.append(
                f"missing informational keys: {', '.join(sorted(missing_info))} (required after {GRACE_PERIOD_END})"
            )

    freshness = values.get("freshness", "")
    if freshness and freshness not in VALID_FRESHNESS:
        errors.append(
            f"invalid freshness '{freshness}', must be one of: {', '.join(sorted(VALID_FRESHNESS))}"
        )

    preservation = values.get("preservation", "")
    if preservation and preservation not in VALID_PRESERVATION:
        errors.append(
            f"invalid preservation '{preservation}', must be one of: {', '.join(sorted(VALID_PRESERVATION))}"
        )

    if freshness == "dead" and "archive" not in path.parts:
        errors.append("freshness:dead must be in docs/archive/ directory")

    return len(errors) == 0, errors, warnings, values


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
            block_content = lines[block_start : idx - 1]
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
    parser.add_argument(
        "--all", action="store_true", help="Check all docs, not just changed files"
    )
    args = parser.parse_args()

    if args.all:
        docs_to_check = list(Path("docs").rglob("*.md"))
    else:
        changed = [Path(p) for p in git_diff_files(args.base, args.head)]
        docs_to_check = [
            p for p in changed if p.suffix == ".md" and p.parts[0] == "docs"
        ]

    errors: List[str] = []
    warnings: List[str] = []

    outside_markdown = []
    if not args.all:
        changed = [Path(p) for p in git_diff_files(args.base, args.head)]
        outside_markdown = [
            p for p in changed if p.suffix == ".md" and p.parts[0] != "docs"
        ]

    for path in outside_markdown:
        if path in APPROVED_EXCEPTIONS:
            continue
        path_str = str(path)
        if any(fnmatch.fnmatch(path_str, pattern) for pattern in APPROVED_PATTERNS):
            continue
        warnings.append(f"Markdown outside /docs (consider moving): {path}")

    for path in docs_to_check:
        ok, doc_errors, doc_warnings, values = check_frontmatter(path)
        for err in doc_errors:
            errors.append(f"Frontmatter error in {path}: {err}")
        for warn in doc_warnings:
            warnings.append(f"Frontmatter warning in {path}: {warn}")

        ok, message = lint_mermaid(path)
        if not ok:
            errors.append(f"Mermaid lint error in {path}: {message}")

        if Path("docs/reference/crates") in path.parents:
            ok, message = check_crate_index(path)
            if not ok:
                errors.append(f"Crate index error in {path}: {message}")

    if not args.all:
        tasks_changes = [p for p in docs_to_check if Path("docs/tasks") in p.parents]
        if tasks_changes and Path("docs/todo.md") not in docs_to_check:
            warnings.append(
                "Changes detected under docs/tasks/ without a corresponding docs/todo.md update."
            )

    for warning in warnings:
        print(f"::warning ::{warning}")

    if errors:
        for error in errors:
            print(f"::error ::{error}")
        sys.exit(1)

    print(f"✅ Validated {len(docs_to_check)} docs with no errors")
    if warnings:
        print(f"⚠️  {len(warnings)} warnings")


if __name__ == "__main__":
    main()
