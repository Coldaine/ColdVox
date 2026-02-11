#!/usr/bin/env python3
"""Backfill review metadata in docs frontmatter.

Adds missing `last_reviewer` and `review_due` keys for docs that already have frontmatter.
"""
from __future__ import annotations

import argparse
import datetime as dt
from pathlib import Path


def update_file(path: Path, review_due: str, apply_changes: bool) -> bool:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return False
    try:
        close_idx = text.index("\n---", 4)
    except ValueError:
        return False

    header = text[4:close_idx]
    body = text[close_idx + 4 :]
    lines = header.splitlines()
    keys = {line.split(":", 1)[0].strip() for line in lines if ":" in line}
    changed = False

    owners_value = "Documentation Working Group"
    for line in lines:
        if line.startswith("owners:"):
            owners_value = line.split(":", 1)[1].strip()
            break

    if "last_reviewer" not in keys:
        lines.append(f"last_reviewer: {owners_value}")
        changed = True
    if "review_due" not in keys:
        lines.append(f"review_due: {review_due}")
        changed = True

    if changed and apply_changes:
        new_header = "\n".join(lines)
        path.write_text(f"---\n{new_header}\n---{body}", encoding="utf-8")

    return changed


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="Write changes to files")
    parser.add_argument(
        "--days",
        type=int,
        default=90,
        help="Review due offset in days for files missing review_due",
    )
    args = parser.parse_args()

    review_due = (dt.date.today() + dt.timedelta(days=args.days)).isoformat()
    changed_files = []
    for path in sorted(Path("docs").rglob("*.md")):
        if path.name == "index.md":
            continue
        if update_file(path, review_due, args.apply):
            changed_files.append(path)

    mode = "updated" if args.apply else "would update"
    print(f"{mode} {len(changed_files)} docs files")
    for path in changed_files:
        print(path.as_posix())


if __name__ == "__main__":
    main()
