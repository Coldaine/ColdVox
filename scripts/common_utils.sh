#!/usr/bin/env bash
# Common logging and helper utilities for ColdVox shell scripts.
# Source this file (alongside config.env) to share consistent behaviour.

# Detect whether we should emit ANSI colours. Honour explicit opt-out or non-TTY outputs.
if [[ -t 1 && -z "${COLDVOX_LOG_NO_COLOR:-}" ]]; then
  _COLDVOX_COLOR_RESET="\033[0m"
  _COLDVOX_COLOR_RED="\033[0;31m"
  _COLDVOX_COLOR_GREEN="\033[0;32m"
  _COLDVOX_COLOR_YELLOW="\033[1;33m"
  _COLDVOX_COLOR_BLUE="\033[0;34m"
  _COLDVOX_COLOR_CYAN="\033[0;36m"
else
  _COLDVOX_COLOR_RESET=""
  _COLDVOX_COLOR_RED=""
  _COLDVOX_COLOR_GREEN=""
  _COLDVOX_COLOR_YELLOW=""
  _COLDVOX_COLOR_BLUE=""
  _COLDVOX_COLOR_CYAN=""
fi

_coldvox_log_prefix() {
  if [[ -n "${COLDVOX_LOG_PREFIX:-}" ]]; then
    printf '%s ' "${COLDVOX_LOG_PREFIX}"
  fi
}

_coldvox_emit() {
  local colour="$1"
  local icon="$2"
  local label="$3"
  local stream="$4"
  shift 4
  local message="$*"
  if [[ -n "$label" ]]; then
    printf '%b%s%s %s%b\n' "${colour}" "$(_coldvox_log_prefix)" "$icon" "$message" "${_COLDVOX_COLOR_RESET}" >&"$stream"
  else
    printf '%b%s%s%b\n' "${colour}" "$(_coldvox_log_prefix)" "$message" "${_COLDVOX_COLOR_RESET}" >&"$stream"
  fi
}

log_step() {
  _coldvox_emit "${_COLDVOX_COLOR_CYAN}" "==>" "STEP" 1 "$*"
}

log_info() {
  _coldvox_emit "${_COLDVOX_COLOR_BLUE}" "ℹ️" "INFO" 1 "$*"
}

log_warn() {
  _coldvox_emit "${_COLDVOX_COLOR_YELLOW}" "⚠️" "WARN" 2 "$*"
}

log_error() {
  _coldvox_emit "${_COLDVOX_COLOR_RED}" "❌" "ERROR" 2 "$*"
}

log_success() {
  _coldvox_emit "${_COLDVOX_COLOR_GREEN}" "✅" "OK" 1 "$*"
}

log_verbose() {
  if [[ "${COLDVOX_LOG_VERBOSE:-0}" == "1" ]]; then
    _coldvox_emit "${_COLDVOX_COLOR_BLUE}" "🔍" "VERBOSE" 1 "$*"
  fi
}

log_debug() {
  if [[ "${COLDVOX_LOG_DEBUG:-0}" == "1" ]]; then
    _coldvox_emit "${_COLDVOX_COLOR_BLUE}" "🐛" "DEBUG" 1 "$*"
  fi
}

fail() {
  log_error "$*"
  exit 1
}

require_command() {
  local missing=()
  for cmd in "$@"; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
      missing+=("$cmd")
    fi
  done
  if [[ ${#missing[@]} -gt 0 ]]; then
    fail "Missing required command(s): ${missing[*]}"
  fi
}
