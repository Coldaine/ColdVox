#!/usr/bin/env python3
"""Bulk apply classification metadata to documentation frontmatter."""

import re
from pathlib import Path

# Classification Map
# path_pattern -> {metadata}
CLASSIFICATIONS = [
    # HISTORICAL
    (r"docs/history/.*", {"freshness": "historical", "preservation": "delete"}),
    (r"docs/archive/.*", {"freshness": "historical", "preservation": "delete"}),
    (
        r"docs/research/checkpoints/.*",
        {"freshness": "historical", "preservation": "delete"},
    ),
    (r"docs/research/logs/.*", {"freshness": "historical", "preservation": "delete"}),
    # DEAD
    (
        r"docs/archive/reference/crates/coldvox-stt.md",
        {"freshness": "dead", "preservation": "summarize"},
    ),
    (
        r"docs/archive/plans/gui/raw-gui-plan.md",
        {"freshness": "dead", "preservation": "delete"},
    ),
    (
        r"docs/archive/plans/gui/comprehensive-gui-plan.md",
        {"freshness": "dead", "preservation": "delete"},
    ),
    (
        r"docs/archive/plans/gui/aspirational-gui-plan.md",
        {"freshness": "dead", "preservation": "delete"},
    ),
    # CURRENT / REFERENCE
    (
        r"docs/plans/critical-action-plan.md",
        {
            "freshness": "current",
            "preservation": "reference",
            "summary": "Source of truth for currently broken vs working features",
        },
    ),
    (
        r"docs/plans/2026-02-09-pr-triage-action-plan.md",
        {
            "freshness": "current",
            "preservation": "reference",
            "summary": "Active triage plan for today's work",
        },
    ),
    (
        r"docs/MasterDocumentationPlaybook.md",
        {
            "freshness": "current",
            "preservation": "reference",
            "summary": "Canonical documentation policy and retention rules",
        },
    ),
    (
        r"docs/standards.md",
        {
            "freshness": "current",
            "preservation": "reference",
            "summary": "Canonical frontmatter schema and validation rules",
        },
    ),
    (
        r"docs/dev/CI/architecture.md",
        {
            "freshness": "current",
            "preservation": "reference",
            "summary": "Rationale for self-hosted vs GitHub-hosted CI split",
        },
    ),
    (
        r"docs/research/pr-reports/PR-259-moonshine-implementation-status.md",
        {
            "freshness": "current",
            "preservation": "preserve",
            "summary": "Implementation details for Moonshine STT backend",
        },
    ),
    # PRESERVE (STALE but valuable)
    (
        r"docs/architecture.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "High-level architecture and tiered STT vision",
            "signals": "['always-on', 'tiered-stt', 'decoupled-threading']",
        },
    ),
    (
        r"docs/observability-playbook.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "OTel span naming and metrics taxonomy",
            "signals": "['otel', 'metrics', 'tracing']",
        },
    ),
    (
        r"docs/domains/text-injection/ti-async-safety-analysis.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "Deep analysis of 6 race conditions in text injection",
            "signals": "['race-condition', 'lock-ordering', 'async-safety']",
        },
    ),
    (
        r"docs/plans/text-injection/opus-code-inject.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "Implementation patterns for Wayland vkbd, EIS, KWin",
            "signals": "['wayland-vkbd', 'portal-eis', 'kwin-fakeinput']",
        },
    ),
    (
        r"docs/stt-parakeet-integration-plan.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "API contract analysis and gap analysis for Parakeet",
            "signals": "['stt-api', 'parakeet']",
        },
    ),
    (
        r"docs/domains/audio/aud-pipewire-design.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "PipeWire detection and ALSA fallback strategy",
            "signals": "['pipewire', 'alsa', 'audio-routing']",
        },
    ),
    (
        r"docs/playbooks/testing/llm-test-debugging-playbook.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "LLM-assisted test debugging methodology",
        },
    ),
    (
        r"docs/issues/pyo3_instability.md",
        {
            "freshness": "stale",
            "preservation": "preserve",
            "summary": "Risks of Python 3.13 + PyO3 0.27 free-threading",
        },
    ),
    # SUMMARIZE / ARCHIVE
    (
        r"docs/research/pr-reports/PR-temp-.*",
        {"freshness": "historical", "preservation": "delete"},
    ),
    (r"docs/repo/editor.md", {"freshness": "stale", "preservation": "delete"}),
    (r"docs/repo/gitignore.md", {"freshness": "stale", "preservation": "delete"}),
]


def update_frontmatter(path: Path, metadata: dict):
    if not path.exists():
        return False

    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        # No frontmatter - we could add it, but for now just skip
        return False

    try:
        closing = text.index("\n---", 4)
    except ValueError:
        return False

    header_block = text[4:closing]
    lines = header_block.splitlines()

    # Parse existing keys
    keys = {}
    for line in lines:
        if ":" in line:
            k, v = line.split(":", 1)
            keys[k.strip()] = v.strip()

    # Merge metadata
    changed = False
    for k, v in metadata.items():
        if k not in keys or keys[k] != str(v):
            keys[k] = str(v)
            changed = True

    if not changed:
        return False

    # Reconstruct header
    new_lines = []
    # Keep some order: doc_type, subsystem, status, freshness, preservation, summary, then others
    priority = [
        "doc_type",
        "subsystem",
        "status",
        "freshness",
        "preservation",
        "summary",
        "signals",
    ]
    for k in priority:
        if k in keys:
            new_lines.append(f"{k}: {keys.pop(k)}")

    for k in sorted(keys.keys()):
        new_lines.append(f"{k}: {keys[k]}")

    new_header = "---\n" + "\n".join(new_lines) + "\n---"
    new_text = new_header + text[closing + 4 :]

    path.write_text(new_text, encoding="utf-8")
    return True


def main():
    docs_root = Path("docs")
    updated_count = 0

    # Process files matching patterns
    for pattern, metadata in CLASSIFICATIONS:
        regex = re.compile(pattern)
        for path in docs_root.rglob("*.md"):
            rel_path = path.as_posix()
            if regex.match(rel_path):
                if update_frontmatter(path, metadata):
                    print(f"  [OK] Updated {rel_path}")
                    updated_count += 1

    # Catch-all: Anything in docs/ that hasn't been classified as freshness: current/stale/historical
    # gets freshness: stale, preservation: preserve by default if it's not archived
    for path in docs_root.rglob("*.md"):
        rel_path = path.as_posix()
        if "index.md" in rel_path:
            continue

        text = path.read_text(encoding="utf-8")
        if not text.startswith("---\n"):
            continue

        try:
            closing = text.index("\n---", 4)
            header = text[4:closing]
            if "freshness:" in header:
                continue

            # Default classification
            metadata = {
                "freshness": "stale",
                "preservation": "preserve",
                "status": "draft",
            }
            if "archive" in rel_path or "history" in rel_path:
                metadata["freshness"] = "historical"
                metadata["preservation"] = "delete"
                metadata["status"] = "archived"

            if update_frontmatter(path, metadata):
                print(f"  [DEFAULT] Classified {rel_path}")
                updated_count += 1
        except (ValueError, UnicodeDecodeError) as exc:
            print(f"  [SKIP] {rel_path}: {exc}")
            continue

    print(f"\n[DONE] Updated {updated_count} files.")


if __name__ == "__main__":
    main()
