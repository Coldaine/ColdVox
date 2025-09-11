#!/usr/bin/env bash
set -euo pipefail

# Runner health / provisioning contract verification.
# Fails fast if required cached assets or system libs are missing.

CACHE_DIR="${CACHE_DIR:-/home/coldaine/ActionRunnerCache/vosk-models}"
REQUIRED_MODEL_SMALL="vosk-model-small-en-us-0.15"
OPTIONAL_MODEL_LARGE="vosk-model-en-us-0.22"

echo "=== Runner Health Check ==="
echo "Date: $(date)"
echo "Hostname: $(hostname)"
echo "Cache Dir: $CACHE_DIR"

if [[ ! -d "$CACHE_DIR/$REQUIRED_MODEL_SMALL" ]]; then
  echo "❌ Required Vosk model missing: $CACHE_DIR/$REQUIRED_MODEL_SMALL" >&2
  exit 1
fi

# Basic structural checks for the small model
for sub in am conf graph ivector; do
  if [[ ! -d "$CACHE_DIR/$REQUIRED_MODEL_SMALL/$sub" ]]; then
    echo "❌ Missing subdir in required model: $sub" >&2
    exit 1
  fi
done

if [[ -d "$CACHE_DIR/$OPTIONAL_MODEL_LARGE" ]]; then
  echo "✅ Optional large model present"
else
  echo "ℹ️  Optional large model not present (ok)"
fi

# libvosk
"$(dirname "$0")/verify_libvosk.sh"

# Resource snapshot
echo "--- System Resources ---"
echo "Load: $(uptime)"
echo "Memory:"; (free -h || true)
echo "Disk (/):"; (df -h / || true)

echo "✅ Runner health check passed"
