#!/usr/bin/env bash
set -euo pipefail
# Thin wrapper (2025-09) around canonical integrity verifier.
# Accepts same arguments; defers to verify-model-integrity.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/verify-model-integrity.sh" "${@}" || exit $?
