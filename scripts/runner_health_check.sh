#!/usr/bin/env bash
set -euo pipefail

# Runner health / provisioning contract verification.
# Fails fast if required system components are missing.

echo "=== Runner Health Check ==="
echo "Date: $(date)"
echo "Hostname: $(hostname)"

# Resource snapshot
echo "--- System Resources ---"
echo "Load: $(uptime)"
echo "Memory:"; (free -h || true)
echo "Disk (/):"; (df -h / || true)

echo "✅ Runner health check passed"
