#!/usr/bin/env bash
set -euo pipefail

# Simple verification that libvosk is provisioned on the self-hosted runner.
# Contract (cache-as-contract model):
#   - /usr/local/lib/libvosk.so must exist (installed out-of-band, never by CI)
#   - /usr/local/include/vosk_api.h should exist
#   - ldconfig should list libvosk

LIB_PATH="/usr/local/lib/libvosk.so"
HEADER_PATH="/usr/local/include/vosk_api.h"

fail() { echo "❌ libvosk verification failed: $1" >&2; exit 1; }

if [[ ! -f "$LIB_PATH" ]]; then
  fail "Missing $LIB_PATH (provision this on the runner)."
fi

if [[ ! -f "$HEADER_PATH" ]]; then
  echo "⚠️  Warning: Header not found at $HEADER_PATH (development headers recommended)." >&2
fi

# Ensure dynamic linker is aware
if ! ldconfig -p | grep -q "libvosk.so"; then
  echo "ℹ️  libvosk not present in ldconfig cache, attempting ldconfig refresh (requires sudo)" >&2
  if command -v sudo >/dev/null 2>&1; then
    sudo ldconfig || true
  fi
  if ! ldconfig -p | grep -q "libvosk.so"; then
    echo "⚠️  libvosk still not in ldconfig output; continuing (RUST may still link via absolute path)." >&2
  fi
fi

echo "✅ libvosk present: $LIB_PATH"
