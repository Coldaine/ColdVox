#!/usr/bin/env python3
"""
Telemetry Schema Validator for ColdVox

Validates that metrics follow the naming convention:
  coldvox.{subsystem}.{metric_name}.{unit}

Exit codes:
  0 - All metrics follow the schema
  1 - Schema violations found

Usage:
  ./scripts/validate_telemetry_schema.py [--fix]
"""

import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List, Tuple


@dataclass
class MetricViolation:
    file: Path
    line: int
    metric_name: str
    issue: str
    suggestion: str


# Valid subsystems (should match crate names and domain codes)
VALID_SUBSYSTEMS = {
    "pipeline",  # Pipeline-level metrics
    "stt",  # Speech-to-text
    "vad",  # Voice activity detection
    "audio",  # Audio capture/processing
    "text_injection",  # Text injection
    "gui",  # GUI/Overlay
    "telemetry",  # Telemetry self-monitoring
}

# Valid units (non-exhaustive, add as needed)
VALID_UNITS = {
    # Time
    "us",
    "ms",
    "s",
    # Count
    "total",
    "count",
    # Data
    "bytes",
    "kb",
    "mb",
    # Percentage
    "pct",
    "percent",
    # Rate
    "fps",
    "hz",
    # Boolean/state
    "bool",
    "state",
    # Level
    "db",
    "level",
}

# Legacy metric names that are grandfathered in (TODO: migrate these)
LEGACY_METRICS = {
    "capture_frames",
    "chunker_frames",
    "capture_errors",
    "chunker_errors",
    "current_peak",
    "current_rms",
    "audio_level_db",
    "stt_failover_count",
    "stt_total_errors",
    "stt_unload_count",
    "stt_transcription_success",
    "end_to_end_ms",
}

# Pattern to find metric names in Rust code
METRIC_PATTERNS = [
    # counter!("name", ...)
    re.compile(r'counter!\s*\(\s*"([^"]+)"'),
    # gauge!("name", ...)
    re.compile(r'gauge!\s*\(\s*"([^"]+)"'),
    # histogram!("name", ...)
    re.compile(r'histogram!\s*\(\s*"([^"]+)"'),
    # Metric names in string literals (heuristic)
    re.compile(r'"(coldvox_[a-z_]+)"'),
    # Atomic store names like stt_transcription_success.store(...)
    re.compile(r"(\w+)\.store\s*\("),
    # Arc<AtomicU64> field names in structs
    re.compile(r"pub\s+\w+:\s*Arc<Atomic\w+>,\s*//\s*(\w+)"),
]


def validate_metric_name(name: str) -> Tuple[bool, str, str]:
    """
    Validate a metric name against the schema.

    Returns: (is_valid, issue, suggestion)
    """
    # Skip legacy metrics
    if name in LEGACY_METRICS:
        return True, "", ""

    # Skip non-metric strings
    if not (
        name.startswith("coldvox")
        or name.startswith("stt_")
        or name.startswith("capture_")
        or name.startswith("chunker_")
        or name.startswith("vad_")
    ):
        return True, "", ""  # Not a metric we care about

    # Check for new schema: coldvox.{subsystem}.{name}.{unit}
    if name.startswith("coldvox_"):
        parts = name.split("_")
        if len(parts) < 3:
            return (
                False,
                "Too few components",
                f"Use format: coldvox_{{subsystem}}_{{name}}_{{unit}}",
            )

        subsystem = parts[1]
        if subsystem not in VALID_SUBSYSTEMS:
            return (
                False,
                f"Invalid subsystem '{subsystem}'",
                f"Use one of: {VALID_SUBSYSTEMS}",
            )

        # Check for unit suffix
        unit = parts[-1]
        if unit not in VALID_UNITS and not any(u in unit for u in VALID_UNITS):
            return (
                False,
                f"Missing/invalid unit suffix",
                f"Add unit suffix from: {VALID_UNITS}",
            )

    return True, "", ""


def find_metrics_in_file(file_path: Path) -> List[Tuple[int, str]]:
    """Find all metric names in a Rust source file."""
    metrics = []
    content = file_path.read_text(encoding="utf-8")

    for line_num, line in enumerate(content.split("\n"), 1):
        for pattern in METRIC_PATTERNS:
            for match in pattern.finditer(line):
                metric_name = match.group(1)
                metrics.append((line_num, metric_name))

    return metrics


def scan_crate(crate_path: Path) -> List[MetricViolation]:
    """Scan a crate for metric naming violations."""
    violations = []

    src_dir = crate_path / "src"
    if not src_dir.exists():
        return violations

    for rust_file in src_dir.rglob("*.rs"):
        metrics = find_metrics_in_file(rust_file)
        for line_num, metric_name in metrics:
            is_valid, issue, suggestion = validate_metric_name(metric_name)
            if not is_valid:
                violations.append(
                    MetricViolation(
                        file=rust_file,
                        line=line_num,
                        metric_name=metric_name,
                        issue=issue,
                        suggestion=suggestion,
                    )
                )

    return violations


def main() -> int:
    """Main entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="Validate ColdVox telemetry schema")
    parser.add_argument(
        "--fix", action="store_true", help="Suggest fixes (not implemented)"
    )
    parser.add_argument("--crate", type=str, help="Specific crate to scan")
    args = parser.parse_args()

    repo_root = Path(__file__).parent.parent
    crates_dir = repo_root / "crates"

    all_violations = []

    if args.crate:
        crate_path = crates_dir / args.crate
        if not crate_path.exists():
            print(f"Error: Crate '{args.crate}' not found at {crate_path}")
            return 1
        all_violations = scan_crate(crate_path)
    else:
        # Scan telemetry crates specifically
        for crate_name in [
            "coldvox-telemetry",
            "coldvox-stt",
            "coldvox-audio",
            "coldvox-vad",
        ]:
            crate_path = crates_dir / crate_name
            if crate_path.exists():
                violations = scan_crate(crate_path)
                all_violations.extend(violations)

    if not all_violations:
        print("✅ All metrics follow the naming schema!")
        print(f"\nSchema: coldvox.{{subsystem}}.{{metric_name}}.{{unit}}")
        print(f"Valid subsystems: {', '.join(sorted(VALID_SUBSYSTEMS))}")
        return 0

    print(f"❌ Found {len(all_violations)} schema violation(s):\n")

    for v in all_violations:
        rel_path = v.file.relative_to(repo_root)
        print(f"  {rel_path}:{v.line}")
        print(f"    Metric: '{v.metric_name}'")
        print(f"    Issue:  {v.issue}")
        print(f"    Fix:    {v.suggestion}")
        print()

    print("\nTo fix these issues:")
    print("1. Rename metrics to follow: coldvox_{subsystem}_{name}_{unit}")
    print("2. Or add legacy metric to LEGACY_METRICS in this script")
    print("3. Update docs/domains/telemetry/tele-observability-playbook.md")

    return 1


if __name__ == "__main__":
    sys.exit(main())
