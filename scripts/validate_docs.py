#!/usr/bin/env python3
"""
ColdVox Documentation Link Validator

This script validates that all links in the hierarchical documentation
point to existing files and that the index.md is consistent with
the actual file structure.
"""

import os
import re
import sys
from pathlib import Path

def validate_links():
    """Validate all links in the documentation."""
    docs_root = Path("/home/coldaine/Projects/Worktrees/ColdVox2/docs/hierarchical")
    
    if not docs_root.exists():
        print(f"Error: Documentation root not found at {docs_root}")
        return False
    
    # Read index.md
    index_path = docs_root / "index.md"
    if not index_path.exists():
        print(f"Error: index.md not found at {index_path}")
        return False
    
    with open(index_path, 'r') as f:
        index_content = f.read()
    
    # Extract all links from index.md
    links = re.findall(r'\[.*?\]\((.*?)\)', index_content)
    
    print(f"Found {len(links)} links in index.md")
    
    # Validate each link
    missing_files = []
    for link in links:
        # Handle the special CODE:repo:// links
        if link.startswith("CODE:repo://"):
            # Convert to actual file path
            actual_path = link.replace("CODE:repo://", "IMP6/CODE:repo:/")
            file_path = docs_root / actual_path
        else:
            file_path = docs_root / link
        
        if not file_path.exists():
            missing_files.append(link)
            print(f"  ❌ Missing: {link}")
        else:
            print(f"  ✅ Found: {link}")
    
    # Also check that all actual files are referenced in index.md
    actual_files = []
    for root, dirs, files in os.walk(docs_root):
        for file in files:
            if file.endswith('.md') and file != 'index.md' and file != 'README.md' and file != 'ARCHITECTURE_OVERVIEW.md':
                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, docs_root)
                actual_files.append(rel_path)
    
    print(f"\nFound {len(actual_files)} actual documentation files")
    
    # Check for unreferenced files
    unreferenced_files = []
    for file_path in actual_files:
        # Convert file path to link format for matching
        if "CODE:repo:/" in file_path:
            # Convert to CODE:repo:// format
            link_format = file_path.replace("CODE:repo:/", "CODE:repo://")
        else:
            link_format = file_path
            
        if link_format not in links:
            unreferenced_files.append(file_path)
            print(f"  ⚠️  Unreferenced: {file_path}")
    
    # Summary
    print(f"\n=== Validation Summary ===")
    print(f"Total links in index.md: {len(links)}")
    print(f"Missing files: {len(missing_files)}")
    print(f"Unreferenced files: {len(unreferenced_files)}")
    
    if missing_files:
        print(f"\n❌ FAILED: {len(missing_files)} missing files")
        return False
    elif unreferenced_files:
        print(f"\n⚠️  WARNING: {len(unreferenced_files)} unreferenced files")
        return True
    else:
        print(f"\n✅ SUCCESS: All links are valid")
        return True

if __name__ == "__main__":
    success = validate_links()
    sys.exit(0 if success else 1)