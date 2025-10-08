#!/usr/bin/env bash
set -euo pipefail

# Graphite batch split helper (file-based routing)
# NOTE: This automates clean, file-path-based splits. Mixed hunks within a file
# still require an interactive pass (gt split --by-hunk) to avoid misrouting.
# Use this as a first pass, then finish interactively.

ANCHOR_BRANCH=${ANCHOR_BRANCH:-"anchor/oct-06-2025"}
BASE_BRANCH=${BASE_BRANCH:-"main"}
REMOTE=${REMOTE:-"origin"}

BRANCHES=(
  "01-config-settings"
  "02-audio-capture"
  "03-vad"
  "04-stt"
  "05-app-runtime-wav"
  "06-text-injection"
  "07-testing"
  "08-logging-observability"
  "09-docs-changelog"
)

# Routing matrix (file globs). Keep in sync with docs.
# IMPORTANT: These are path-level filters; any file containing mixed-domain hunks
# should be reviewed afterwards interactively.
read -r -d '' ROUTE_01 <<'EOF'
config/**
crates/app/src/lib.rs
crates/app/src/main.rs
crates/coldvox-foundation/**
crates/app/tests/settings_test.rs
EOF

read -r -d '' ROUTE_02 <<'EOF'
crates/coldvox-audio/**
crates/app/src/audio/mod.rs
crates/app/src/audio/vad_adapter.rs
crates/app/src/audio/vad_processor.rs
EOF

read -r -d '' ROUTE_03 <<'EOF'
crates/coldvox-vad/**
crates/coldvox-vad-silero/**
crates/app/src/vad.rs
EOF

read -r -d '' ROUTE_04 <<'EOF'
crates/coldvox-stt/**
crates/coldvox-stt-vosk/**
crates/app/src/stt/processor.rs
crates/app/src/stt/vosk.rs
crates/app/src/stt/persistence.rs
crates/app/src/stt/plugin_manager.rs
crates/app/src/stt/session.rs
crates/app/src/stt/types.rs
EOF

read -r -d '' ROUTE_05 <<'EOF'
crates/app/src/runtime.rs
crates/app/src/audio/wav_file_loader.rs
crates/app/src/stt/tests/end_to_end_wav.rs
EOF

read -r -d '' ROUTE_06 <<'EOF'
crates/coldvox-text-injection/**
EOF

read -r -d '' ROUTE_07 <<'EOF'
**/tests/**
examples/**
EOF

read -r -d '' ROUTE_08 <<'EOF'
crates/coldvox-telemetry/**
crates/app/src/bin/*.rs
EOF

read -r -d '' ROUTE_09 <<'EOF'
docs/**
CHANGELOG.md
README.md
CLAUDE.md
agents.md
.github/**
Cargo.lock
EOF

routes_for_branch() {
  case "$1" in
    01-config-settings) echo "$ROUTE_01" ;;
    02-audio-capture) echo "$ROUTE_02" ;;
    03-vad) echo "$ROUTE_03" ;;
    04-stt) echo "$ROUTE_04" ;;
    05-app-runtime-wav) echo "$ROUTE_05" ;;
    06-text-injection) echo "$ROUTE_06" ;;
    07-testing) echo "$ROUTE_07" ;;
    08-logging-observability) echo "$ROUTE_08" ;;
    09-docs-changelog) echo "$ROUTE_09" ;;
    *) echo "" ;;
  esac
}

require_clean_tree() {
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Error: working tree or index is not clean. Commit or stash first." >&2
    exit 1
  fi
}

ensure_base_and_anchor() {
  git fetch "$REMOTE"
  # Ensure we’re on anchor and it’s tracked.
  git checkout "$ANCHOR_BRANCH"
  gt track || true
  # Verify parent is main.
  gt track --parent "$BASE_BRANCH" "$ANCHOR_BRANCH" || true
}

create_branch_if_missing() {
  local branch="$1" parent="$2"
  if ! git rev-parse --verify "$branch" >/dev/null 2>&1; then
    echo "Creating branch $branch off $parent"
    git checkout -b "$branch" "$parent"
  else
    echo "Branch $branch already exists"
  fi
}

track_branch_stack() {
  local branch="$1" parent="$2"
  git checkout "$branch"
  gt track --parent "$parent" "$branch" || true
}

apply_paths_from_anchor() {
  local branch="$1" paths="$2"
  echo "Routing files to $branch:"
  printf "%s\n" "$paths" | while read -r p; do
    [[ -z "$p" ]] && continue
    echo "  - $p"
    # Use pathspec from anchor; if glob doesn’t match, skip.
    # git checkout <tree-ish> -- <paths> supports globs from current workdir.
    git checkout "$ANCHOR_BRANCH" -- "$p" 2>/dev/null || true
  done
  # Commit if changes were applied
  if ! git diff --quiet; then
    git add -A
    git commit -m "split($branch): route file-based changes per matrix"
  else
    echo "No changes staged for $branch"
  fi
}

push_branch() {
  local branch="$1"
  git push -u "$REMOTE" "$branch"
}

# Main
require_clean_tree
ensure_base_and_anchor

# Create stack branches and apply routing in order
prev="$BASE_BRANCH"
for b in "${BRANCHES[@]}"; do
  create_branch_if_missing "$b" "$prev"
  track_branch_stack "$b" "$prev"
  apply_paths_from_anchor "$b" "$(routes_for_branch "$b")"
  push_branch "$b"
  prev="$b"
done

# Final: return to anchor branch for interactive clean-up of mixed hunks
git checkout "$ANCHOR_BRANCH"

echo "\nBatch file-based split complete. Next steps:"
echo "  1) Run 'gt log' to inspect stack order; use 'gt reorder' if needed."
echo "  2) Run 'gt split --by-hunk' on $ANCHOR_BRANCH to mop up mixed hunks (prefer higher domain)."
echo "  3) Create /tmp/split-validation.log with ambiguous hunks and decisions."
echo "  4) For each branch: 'git checkout <branch> && cargo check' to validate builds."