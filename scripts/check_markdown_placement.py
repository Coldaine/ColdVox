#!/usr/bin/env python3
"""Pre-commit hook: Block Markdown files outside docs/ except approved exceptions."""
from __future__ import annotations

import sys
from pathlib import Path

APPROVED_EXCEPTIONS = {
    Path("README.md"),
    Path("CHANGELOG.md"),
    Path("CLAUDE.md"),
    Path(".github/pull_request_template.md"),
}

# Pattern-based exceptions
def is_crate_readme(path: Path) -> bool:
    """Check if path is a crate README under crates/*/README.md."""
    return (
        len(path.parts) >= 3
        and path.parts[0] == "crates"
        and path.name == "README.md"
    )

def is_pr_assessment(path: Path) -> bool:
    """Check if path is a PR assessment file (PR-NNN-*.md at root)."""
    return (
        len(path.parts) == 1
        and path.name.startswith("PR-")
        and path.suffix == ".md"
    )


def main() -> int:
    """Check all staged markdown files are in docs/ or approved exceptions."""
    files = [Path(arg) for arg in sys.argv[1:]]
    markdown_files = [f for f in files if f.suffix == ".md"]

    errors: list[Path] = []
    for path in markdown_files:
        # Approved root-level exceptions
        if path in APPROVED_EXCEPTIONS:
            continue
        # Crate READMEs are allowed
        if is_crate_readme(path):
            continue
        # PR assessment files are allowed
        if is_pr_assessment(path):
            continue
        # Must be under docs/
        if path.parts[0] != "docs":
            errors.append(path)

    if errors:
        print("\n❌ ERROR: Markdown files outside /docs are not allowed:\n")
        for path in errors:
            print(f"  - {path}")
        print("\n📖 See docs/standards.md for approved exceptions.")
        print("   Move these files to docs/ or add them to APPROVED_EXCEPTIONS.\n")
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
