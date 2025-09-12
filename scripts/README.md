# Documentation Validation Tools

This directory contains tools for validating the ColdVox documentation structure and links.

## validate_docs.py

A Python script that validates all links in the hierarchical documentation index and ensures consistency between the index and actual files.

### Features:
- Validates that all links in `docs/hierarchical/index.md` point to existing files
- Checks that all actual documentation files are referenced in the index
- Handles special `CODE:repo://` link formats
- Provides detailed output of missing or unreferenced files

### Usage:
```bash
./scripts/validate_docs.sh
```

Or directly:
```bash
python3 ./scripts/validate_docs.py
```

### Output:
- ✅ Found: Files that exist and are properly linked
- ❌ Missing: Files referenced in index.md but don't exist
- ⚠️ Unreferenced: Files that exist but aren't in index.md

## Integration

The validation script can be integrated into CI/CD pipelines to ensure documentation consistency:

```bash
# In CI script
./scripts/validate_docs.sh
```

## Maintenance

When adding new documentation files:
1. Create the documentation file in the appropriate directory
2. Add an entry to `docs/hierarchical/index.md`
3. Run the validation script to verify everything is consistent