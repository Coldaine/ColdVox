#!/usr/bin/env python3
"""Append documentation changes to docs/revision_log.csv for auditing."""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import subprocess
from pathlib import Path
from typing import Dict, List

ACTION_MAP: Dict[str, str] = {
    "A": "added",
    "M": "modified",
    "D": "deleted",
    "R": "renamed",
    "C": "copied",
}


def git_diff_status(base: str, head: str) -> List[str]:
    result = subprocess.run(
        ["git", "diff", "--name-status", base, head],
        capture_output=True,
        text=True,
        check=True,
    )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def parse_status(line: str) -> List[tuple[str, Path]]:
    parts = line.split("\t")
    status = parts[0]
    if status.startswith("R") or status.startswith("C"):
        # Format: R100\told\tnew
        new_path = Path(parts[-1])
        return [(status[0], new_path)]
    code = status[0]
    path = Path(parts[1])
    return [(code, path)]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base")
    parser.add_argument("head")
    parser.add_argument("actor")
    args = parser.parse_args()

    status_lines = git_diff_status(args.base, args.head)
    entries: List[tuple[str, Path]] = []
    for line in status_lines:
        entries.extend(parse_status(line))

    docs_entries = [item for item in entries if item[1].suffix == ".md" and item[1].parts[0] == "docs"]
    if not docs_entries:
        return

    log_path = Path("docs/revision_log.csv")
    timestamp = dt.datetime.utcnow().isoformat(timespec="seconds") + "Z"

    with log_path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        for code, path in docs_entries:
            action = ACTION_MAP.get(code, "modified")
            writer.writerow([timestamp, args.actor, str(path), action, "CI validation"])


if __name__ == "__main__":
    main()
