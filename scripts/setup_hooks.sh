#!/bin/bash
# This script installs the project's git hooks.

# Exit on error
set -e

# The root directory of the git repository
GIT_ROOT=$(git rev-parse --show-toplevel)
HOOKS_DIR="$GIT_ROOT/.git/hooks"
SRC_HOOK_DIR="$GIT_ROOT/.git-hooks"

# Check if the user wants to skip hook installation
if [[ "$COLDVOX_SKIP_HOOKS" == "1" ]]; then
    echo "COLDVOX_SKIP_HOOKS is set. Skipping hook installation."
    exit 0
fi

echo "Installing ColdVox git hooks..."

# Ensure the .git/hooks directory exists
mkdir -p "$HOOKS_DIR"

# The name of our pre-commit hook script
HOOK_NAME="pre-commit-injection-tests"
# The target path for the symlink
TARGET_HOOK="$HOOKS_DIR/pre-commit"

# Create a symlink from the canonical pre-commit hook location
# to our script in the repository.
# We use a relative path for portability of the repository.
# The path from .git/hooks/pre-commit to .git-hooks/ is ../../.git-hooks
ln -sf "../../.git-hooks/$HOOK_NAME" "$TARGET_HOOK"

echo "Hook '$HOOK_NAME' installed as pre-commit hook."
echo "To disable, set COLDVOX_SKIP_HOOKS=1 or remove the symlink at $TARGET_HOOK."
