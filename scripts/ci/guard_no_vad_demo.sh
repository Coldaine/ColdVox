#!/bin/bash
# CI Guard: Ensure vad_demo is not reintroduced
# This script fails if any references to vad_demo are found in the repository

set -e

echo "Checking for vad_demo references..."

# Use ripgrep if available, otherwise fall back to grep
if command -v rg >/dev/null 2>&1; then
    # Exclude target/, .git/, and this script itself
    MATCHES=$(rg -n --hidden --iglob '!*target/*' --iglob '!*scripts/ci/guard_no_vad_demo.sh' -i 'vad_demo|vad demo|vad\s+demo' . || true)
else
    # Fallback to grep
    MATCHES=$(grep -Rni --exclude-dir=target --exclude-dir=.git --exclude=scripts/ci/guard_no_vad_demo.sh 'vad_demo\|vad demo' . || true)
fi

if [ -n "$MATCHES" ]; then
    echo "ERROR: Found references to vad_demo in the repository:"
    echo "$MATCHES"
    echo ""
    echo "Please remove all references to vad_demo and use test_silero_wav instead."
    exit 1
else
    echo "✓ No vad_demo references found. CI guard passed."
fi
