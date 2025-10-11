#!/usr/bin/env bash
# Enhanced error handling for runner health checks (2025-10-11)
# Added proper status checks and error reporting per refactoring recommendations
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=common_utils.sh
source "${SCRIPT_DIR}/common_utils.sh"
# shellcheck source=config.env
source "${SCRIPT_DIR}/config.env"

CACHE_DIR="${CACHE_DIR:-${RUNNER_CACHE_DIR_ALT}}"
REQUIRED_MODEL_SMALL="${VOSK_MODEL_NAME}"
OPTIONAL_MODEL_LARGE="${RUNNER_OPTIONAL_MODEL_NAME}"

log_step "Runner Health Check"
log_info "Date: $(date)"
log_info "Hostname: $(hostname)"
log_info "Cache Dir: ${CACHE_DIR}"

if [[ ! -d "${CACHE_DIR}/${REQUIRED_MODEL_SMALL}" ]]; then
  log_error "Required Vosk model missing: ${CACHE_DIR}/${REQUIRED_MODEL_SMALL}"
  exit 1
fi

# Basic structural checks for the small model
read -r -a _required_subdirs <<< "${VOSK_MODEL_REQUIRED_SUBDIRS}"
for sub in "${_required_subdirs[@]}"; do
  if [[ ! -d "${CACHE_DIR}/${REQUIRED_MODEL_SMALL}/${sub}" ]]; then
    log_error "Missing subdir in required model: ${sub}"
    exit 1
  fi
done

if [[ -d "${CACHE_DIR}/${OPTIONAL_MODEL_LARGE}" ]]; then
  log_success "Optional large model present"
else
  log_info "Optional large model not present (ok)"
fi

# libvosk with enhanced error checking
VERIFY_LIBVOSK_SCRIPT="${SCRIPT_DIR}/verify_libvosk.sh"
if [[ ! -f "${VERIFY_LIBVOSK_SCRIPT}" ]]; then
    log_error "Required script missing: ${VERIFY_LIBVOSK_SCRIPT}"
    exit 1
fi
if [[ ! -x "${VERIFY_LIBVOSK_SCRIPT}" ]]; then
    log_error "Script not executable: ${VERIFY_LIBVOSK_SCRIPT}"
    exit 1
fi
"${VERIFY_LIBVOSK_SCRIPT}"

# Resource snapshot
log_step "System Resources"
log_info "Load: $(uptime)"
log_info "Memory usage:"
free -h || true
log_info "Disk usage (/):"
df -h / || true

log_success "Runner health check passed"
