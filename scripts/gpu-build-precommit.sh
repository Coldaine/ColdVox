#!/usr/bin/env bash
# GPU-gated build + test compilation for pre-commit.
# Conditions:
#   - Skip if CI is set (handled by CI pipeline already)
#   - Require nvidia-smi
#   - Require at least one NVIDIA GPU with >= 12288 MiB total memory (12 GiB)
#   - If requirements met: run cargo build (workspace) and cargo test --no-run (compile tests)
# Exit codes:
#   0 = success or intentionally skipped
#   1 = hard failure (build/compile error)
#   2 = environment unmet (treated as skip)

set -euo pipefail

log() { echo "[gpu-build] $*"; }
warn() { echo "[gpu-build][warn] $*" >&2; }
err() { echo "[gpu-build][error] $*" >&2; }

# Skip in CI to avoid duplicate work
if [[ "${CI:-}" == "true" ]]; then
  log "CI environment detected; skipping (handled by workflows)."
  exit 0
fi

# Require nvidia-smi
if ! command -v nvidia-smi >/dev/null 2>&1; then
  log "nvidia-smi not found; skipping (no NVIDIA GPU)."
  exit 0
fi

# Query GPU names and memory (in MiB)
# Format: index,name,memory.total
IFS=$'\n' read -r -d '' -a gpu_info < <(nvidia-smi --query-gpu=index,name,memory.total --format=csv,noheader,nounits 2>/dev/null && printf '\0' || true)
if [[ ${#gpu_info[@]} -eq 0 ]]; then
  log "No GPU lines returned; skipping."
  exit 0
fi

min_mem_mib=12288
selected=""
for line in "${gpu_info[@]}"; do
  # Example line: 0, NVIDIA GeForce RTX 3090, 24576
  idx=$(echo "$line" | cut -d',' -f1 | xargs)
  name=$(echo "$line" | cut -d',' -f2 | xargs)
  mem=$(echo "$line" | cut -d',' -f3 | xargs)
  if [[ -n "$mem" && "$mem" =~ ^[0-9]+$ ]]; then
    if (( mem >= min_mem_mib )); then
      selected="$idx,$name,$mem"
      break
    fi
  fi
done

if [[ -z "$selected" ]]; then
  log "No NVIDIA GPU with >= ${min_mem_mib}MiB VRAM detected; skipping."
  exit 0
fi

sel_name=$(echo "$selected" | cut -d',' -f2 | xargs)
sel_mem=$(echo "$selected" | cut -d',' -f3 | xargs)
log "Using GPU: $sel_name (${sel_mem} MiB VRAM)"

# Respect optional skip var
if [[ "${SKIP_GPU_BUILD_TESTS:-}" == "1" ]]; then
  log "SKIP_GPU_BUILD_TESTS=1 set; skipping build/tests."
  exit 0
fi

# Perform build + test compilation
log "Running cargo build (workspace)"
if ! cargo build --workspace --locked; then
  err "cargo build failed"
  exit 1
fi

log "Compiling tests (no run)"
if ! cargo test --workspace --no-run --locked; then
  err "cargo test --no-run failed"
  exit 1
fi

log "Success (build + test compile)"
exit 0
