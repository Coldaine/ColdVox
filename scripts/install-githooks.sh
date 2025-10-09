#!/usr/bin/env bash
set -euo pipefail

# Install repository-git hooks (makes it easy for contributors to enable pre-commit hooks)
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.githooks"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

if [[ ! -d "$HOOKS_DIR" ]]; then
  echo "No .githooks directory found in repo root" >&2
  exit 1
fi

echo "Installing git hooks from $HOOKS_DIR to $GIT_HOOKS_DIR"
mkdir -p "$GIT_HOOKS_DIR"

for hook in "$HOOKS_DIR"/*; do
  hook_name=$(basename "$hook")
  target="$GIT_HOOKS_DIR/$hook_name"
  echo " - Installing $hook_name"
  cp "$hook" "$target"
  chmod +x "$target"
done

echo "Done. Hooks installed. To enable them for this repo run:"
echo "  cd $REPO_ROOT && ./scripts/install-githooks.sh"
