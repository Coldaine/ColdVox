#!/usr/bin/env bash
set -euo pipefail

repo_root="$({
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
})"

src="$repo_root/AGENTS.md"

dst1="$repo_root/.github/copilot-instructions.md"
dst2="$repo_root/.kilocode/rules/agents.md"

if [[ ! -f "$src" ]]; then
  echo "error: missing $src" >&2
  exit 1
fi

mkdir -p "$(dirname "$dst1")" "$(dirname "$dst2")"

# Replace destination files with hard links to AGENTS.md.
# This is safe even if Git doesn't preserve hardlinks across clones: rerun anytime.
ln -f "$src" "$dst1"
ln -f "$src" "$dst2"

# Show inode + link count for confirmation.
stat -c '%i %h %n' "$src" "$dst1" "$dst2"
