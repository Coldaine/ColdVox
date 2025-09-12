#!/usr/bin/env python3
"""
ColdVox Feature Testing Framework

A comprehensive test runner for testing all feature combinations in Rust workspaces.
This script systematically tests feature-gated code paths to identify bugs and
platform-specific issues that standard CI might miss.
"""

import argparse
import subprocess
import sys
import os
import json
import time
from typing import List, Dict, Set, Tuple, Optional
from itertools import chain, combinations
from pathlib import Path
import tomllib
from dataclasses import dataclass
from enum import Enum


class TestStrategy(Enum):
    """Testing strategies for feature combinations."""
    DEFAULT_ONLY = "default-only"
    NO_DEFAULT = "no-default"
    EACH_FEATURE = "each-feature"
    POWERSET = "powerset"
    CURATED = "curated"


@dataclass
class TestResult:
    """Result of a single test run."""
    features: str
    command: str
    success: bool
    duration: float
    output: str = ""
    error: str = ""


class FeatureTestRunner:
    """Main test runner class for feature combination testing."""

    def __init__(self, workspace_root: Path, verbose: bool = False):
        self.workspace_root = workspace_root
        self.verbose = verbose
        self.results: List[TestResult] = []

    def parse_cargo_toml(self, crate_path: Path) -> Dict:
        """Parse a Cargo.toml file to extract features and metadata."""
        cargo_toml = crate_path / "Cargo.toml"
        if not cargo_toml.exists():
            raise FileNotFoundError(f"Cargo.toml not found at {cargo_toml}")

        with open(cargo_toml, "rb") as f:
            return tomllib.load(f)

    def get_features(self, crate_path: Path) -> Tuple[List[str], List[str]]:
        """
        Extract features from a crate's Cargo.toml.
        Returns: (all_features, default_features)
        """
        config = self.parse_cargo_toml(crate_path)
        features_section = config.get("features", {})

        all_features = [f for f in features_section.keys() if f != "default"]
        default_features = features_section.get("default", [])

        return all_features, default_features

    def powerset(self, items: List[str]) -> List[List[str]]:
        """Generate all possible combinations (powerset) of features."""
        return list(chain.from_iterable(
            combinations(items, r) for r in range(len(items) + 1)
        ))

    def get_curated_combinations(self, crate_name: str, all_features: List[str]) -> List[List[str]]:
        """
        Get curated feature combinations for specific crates.
        These are hand-picked combinations that are most likely to reveal issues.
        """
        curated = {
            "coldvox-app": [
                [],  # Default features
                ["silero"],
                # ["level3"], # Legacy feature, not actively tested
                ["vosk"],
                ["text-injection"],
                ["silero", "vosk"],
                # ["level3", "vosk"], # Legacy feature, not actively tested
                ["silero", "text-injection"],
                # ["level3", "text-injection"], # Legacy feature, not actively tested
                ["silero", "vosk", "text-injection"],
                # ["level3", "vosk", "text-injection"], # Legacy feature, not actively tested
                ["text-injection-atspi"],
                ["text-injection-clipboard"],
                ["text-injection-enigo"],
                ["text-injection-ydotool"],
                ["text-injection-kdotool"],
            ],
            "coldvox-vad": [
                [],
                # ["level3"], # Legacy feature, not actively tested
            ],
            "coldvox-text-injection": [
                [],
                ["atspi"],
                ["wl_clipboard"],
                ["enigo"],
                ["kdotool"],
                ["ydotool"],
                ["enigo"],
                ["all-backends"],
                ["linux-desktop"],
                ["atspi", "wl_clipboard"],
                ["enigo"],
            ],
            "coldvox-vad-silero": [
                [],
                ["silero"],
            ],
            "coldvox-stt-vosk": [
                [],
                ["vosk"],
            ],
        }

        # Return curated combinations if available, otherwise use sensible defaults
        if crate_name in curated:
            return curated[crate_name]

        # Default curated set for unknown crates
        if len(all_features) <= 3:
            return self.powerset(all_features)
        else:
            # For crates with many features, test each individually plus some pairs
            individual = [[f] for f in all_features]
            some_pairs = list(combinations(all_features[:4], 2))
            return [[]] + individual + some_pairs

    def generate_test_combinations(
        self,
        strategy: TestStrategy,
        all_features: List[str],
        crate_name: str = None
    ) -> List[Tuple[bool, List[str]]]:
        """
        Generate test combinations based on the selected strategy.
        Returns list of tuples: (use_default_features, feature_list)
        """
        combinations = []

        if strategy == TestStrategy.DEFAULT_ONLY:
            combinations = [(True, [])]

        elif strategy == TestStrategy.NO_DEFAULT:
            combinations = [(False, [])]

        elif strategy == TestStrategy.EACH_FEATURE:
            combinations = [(False, [])]  # No features
            combinations += [(False, [f]) for f in all_features]

        elif strategy == TestStrategy.POWERSET:
            all_combos = self.powerset(all_features)
            combinations = [(False, list(combo)) for combo in all_combos]
            combinations.append((True, []))  # Also test with defaults

        elif strategy == TestStrategy.CURATED:
            curated_combos = self.get_curated_combinations(crate_name or "unknown", all_features)
            combinations = [(False, list(combo)) for combo in curated_combos]
            combinations.append((True, []))  # Also test with defaults

        return combinations

    def build_cargo_command(
        self,
        package: Optional[str],
        use_defaults: bool,
        features: List[str],
        test_args: List[str]
    ) -> List[str]:
        """Build the cargo test command with appropriate flags."""
        cmd = ["cargo", "test"]

        if package:
            cmd.extend(["-p", package])

        if not use_defaults:
            cmd.append("--no-default-features")

        if features:
            cmd.extend(["--features", ",".join(features)])

        cmd.extend(test_args)

        return cmd

    def run_test(self, cmd: List[str]) -> TestResult:
        """Execute a test command and return the result."""
        cmd_str = " ".join(cmd)
        print(f"\n{'=' * 80}")
        print(f"Running: {cmd_str}")
        print(f"{'=' * 80}")

        start_time = time.time()

        try:
            result = subprocess.run(
                cmd,
                cwd=self.workspace_root,
                capture_output=True,
                text=True,
                timeout=600  # 10 minute timeout
            )

            duration = time.time() - start_time
            success = result.returncode == 0

            if success:
                print(f"✅ SUCCESS (took {duration:.2f}s)")
            else:
                print(f"❌ FAILURE (took {duration:.2f}s)")
                if self.verbose:
                    print(f"STDOUT:\n{result.stdout}")
                    print(f"STDERR:\n{result.stderr}")

            features = ""
            for i, arg in enumerate(cmd):
                if arg == "--features" and i + 1 < len(cmd):
                    features = cmd[i + 1]
                    break

            return TestResult(
                features=features or "default",
                command=cmd_str,
                success=success,
                duration=duration,
                output=result.stdout,
                error=result.stderr
            )

        except subprocess.TimeoutExpired:
            duration = time.time() - start_time
            print(f"❌ TIMEOUT (after {duration:.2f}s)")
            return TestResult(
                features="",
                command=cmd_str,
                success=False,
                duration=duration,
                error="Test timed out after 600 seconds"
            )
        except Exception as e:
            duration = time.time() - start_time
            print(f"❌ ERROR: {e}")
            return TestResult(
                features="",
                command=cmd_str,
                success=False,
                duration=duration,
                error=str(e)
            )

    def test_package(
        self,
        package: str,
        strategy: TestStrategy,
        test_args: List[str]
    ) -> List[TestResult]:
        """Test a specific package with the given strategy."""
        # Find the package directory
        if package:
            package_path = self.workspace_root / "crates" / package.replace("coldvox-", "")
            if not package_path.exists():
                package_path = self.workspace_root / "crates" / package
            if not package_path.exists():
                raise ValueError(f"Package {package} not found in workspace")
        else:
            package_path = self.workspace_root

        try:
            all_features, default_features = self.get_features(package_path)
        except Exception as e:
            print(f"Warning: Could not parse features from {package_path}: {e}")
            all_features, default_features = [], []

        print(f"\n🔍 Testing package: {package or 'workspace'}")
        print(f"   Available features: {all_features}")
        print(f"   Default features: {default_features}")
        print(f"   Strategy: {strategy.value}")

        combinations = self.generate_test_combinations(strategy, all_features, package)
        results = []

        for i, (use_defaults, features) in enumerate(combinations, 1):
            print(f"\n[{i}/{len(combinations)}] Testing combination...")

            cmd = self.build_cargo_command(package, use_defaults, features, test_args)
            result = self.run_test(cmd)
            results.append(result)
            self.results.append(result)

        return results

    def print_summary(self):
        """Print a summary of all test results."""
        print("\n" + "=" * 80)
        print("TEST SUMMARY")
        print("=" * 80)

        total = len(self.results)
        passed = sum(1 for r in self.results if r.success)
        failed = total - passed

        print(f"\nTotal tests: {total}")
        print(f"✅ Passed: {passed}")
        print(f"❌ Failed: {failed}")

        if failed > 0:
            print(f"\n{'FAILED COMBINATIONS':^80}")
            print("-" * 80)
            for r in self.results:
                if not r.success:
                    print(f"  Features: {r.features or 'default'}")
                    print(f"  Command:  {r.command}")
                    if r.error and self.verbose:
                        print(f"  Error: {r.error[:200]}...")
                    print()

        total_duration = sum(r.duration for r in self.results)
        print(f"\nTotal test time: {total_duration:.2f}s")

        return failed == 0


def main():
    """Main entry point for the test runner."""
    parser = argparse.ArgumentParser(
        description="ColdVox Feature Testing Framework - Test all feature combinations"
    )

    parser.add_argument(
        "-p", "--package",
        help="Specific package to test (e.g., 'app', 'coldvox-vad')"
    )

    parser.add_argument(
        "--strategy",
        type=str,
        choices=[s.value for s in TestStrategy],
        default=TestStrategy.CURATED.value,
        help="Testing strategy (default: curated)"
    )

    parser.add_argument(
        "--workspace-root",
        type=Path,
        default=Path.cwd(),
        help="Path to workspace root (default: current directory)"
    )

    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Show detailed output including test stdout/stderr"
    )

    parser.add_argument(
        "--json-output",
        type=Path,
        help="Save results to JSON file"
    )

    parser.add_argument(
        "test_args",
        nargs=argparse.REMAINDER,
        help="Additional arguments to pass to cargo test"
    )

    args = parser.parse_args()

    # Find workspace root if not specified
    workspace_root = args.workspace_root
    if not (workspace_root / "Cargo.toml").exists():
        print(f"Error: No Cargo.toml found at {workspace_root}")
        print("Please run from workspace root or specify --workspace-root")
        sys.exit(1)

    # Create runner and execute tests
    runner = FeatureTestRunner(workspace_root, verbose=args.verbose)
    strategy = TestStrategy(args.strategy)

    try:
        runner.test_package(args.package, strategy, args.test_args)
        success = runner.print_summary()

        # Save JSON output if requested
        if args.json_output:
            results_data = [
                {
                    "features": r.features,
                    "command": r.command,
                    "success": r.success,
                    "duration": r.duration,
                    "output": r.output if args.verbose else "",
                    "error": r.error if args.verbose else ""
                }
                for r in runner.results
            ]

            with open(args.json_output, "w") as f:
                json.dump(results_data, f, indent=2)

            print(f"\nResults saved to {args.json_output}")

        sys.exit(0 if success else 1)

    except Exception as e:
        print(f"\n❌ Fatal error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
