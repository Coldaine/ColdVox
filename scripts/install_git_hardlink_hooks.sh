#!/usr/bin/env bash
set -euo pipefail

quiet=0
if [[ "${1:-}" == "--quiet" ]]; then
  quiet=1
fi

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

prev_hooks_path="$(git -C "$repo_root" config --local --get core.hooksPath || true)"
git -C "$repo_root" config --local core.hooksPath .githooks

if [[ "$quiet" -eq 0 ]]; then
  if [[ -n "$prev_hooks_path" && "$prev_hooks_path" != ".githooks" ]]; then
    echo "Updated repo hooks via core.hooksPath: $prev_hooks_path -> .githooks"
  else
    echo "Enabled repo hooks via core.hooksPath=.githooks"
  fi
fi

"$repo_root/scripts/ensure_agent_hardlinks.sh" ${quiet:+--quiet}
