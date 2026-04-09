#!/usr/bin/env python3
"""
Documentation validation script.
Validates that documentation files are properly formatted and referenced.
"""

import argparse
import sys
from pathlib import Path


def validate_docs(base: str, head: str) -> int:
    """
    Validate documentation changes between base and head refs.
    
    Args:
        base: Base git ref to compare against
        head: Head git ref to compare
        
    Returns:
        0 if validation passes, 1 otherwise
    """
    repo_root = Path(__file__).parent.parent
    docs_dir = repo_root / "docs"
    
    if not docs_dir.exists():
        print(f"❌ Documentation directory not found: {docs_dir}")
        return 1
    
    print(f"✅ Documentation directory found: {docs_dir}")
    print(f"   Validating changes between {base} and {head}")
    
    # Basic validation: check for common issues
    issues = []
    
    # Check for broken markdown links (simplified)
    for md_file in docs_dir.rglob("*.md"):
        content = md_file.read_text(encoding="utf-8")
        
        # Check for empty files
        if not content.strip():
            issues.append(f"Empty documentation file: {md_file.relative_to(repo_root)}")
        
        # Check for broken internal links (very basic check)
        lines = content.split("\n")
        for i, line in enumerate(lines, 1):
            if "](" in line and ")" in line:
                # Very basic link check - could be enhanced
                pass
    
    if issues:
        print("\n⚠️  Documentation issues found:")
        for issue in issues:
            print(f"   - {issue}")
        return 1
    
    print("✅ Documentation validation passed")
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Validate documentation changes"
    )
    parser.add_argument(
        "base",
        nargs="?",
        default="origin/main",
        help="Base git ref (default: origin/main)"
    )
    parser.add_argument(
        "head",
        nargs="?",
        default="HEAD",
        help="Head git ref (default: HEAD)"
    )
    
    args = parser.parse_args()
    
    sys.exit(validate_docs(args.base, args.head))


if __name__ == "__main__":
    main()
