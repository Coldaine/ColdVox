#!/usr/bin/env bash
set -euo pipefail

repo_root="$({
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
})"

if [[ ! -d "$repo_root/.git" ]]; then
  echo "error: not a git checkout (missing $repo_root/.git)" >&2
  exit 1
fi

hooks_path="$repo_root/.githooks"

if [[ ! -d "$hooks_path" ]]; then
  echo "error: missing $hooks_path (expected repo-tracked hooks)" >&2
  exit 1
fi

chmod +x "$hooks_path/post-checkout" "$hooks_path/post-merge" "$repo_root/scripts/ensure_agent_hardlinks.sh"

git -C "$repo_root" config core.hooksPath .githooks

echo "Enabled repo hooks via core.hooksPath=.githooks"
"$repo_root/scripts/ensure_agent_hardlinks.sh"
