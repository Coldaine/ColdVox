#!/usr/bin/env bash
set -euo pipefail

quiet=0
require_hardlink=0

for arg in "$@"; do
  case "$arg" in
    --quiet) quiet=1 ;;
    --require-hardlink) require_hardlink=1 ;;
    *)
      echo "error: unknown arg: $arg" >&2
      echo "usage: $0 [--quiet] [--require-hardlink]" >&2
      exit 64
      ;;
  esac
done

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

ensure_args=()
if [[ "$quiet" -eq 1 ]]; then ensure_args+=(--quiet); fi
if [[ "$require_hardlink" -eq 1 ]]; then ensure_args+=(--require-hardlink); fi

"$repo_root/scripts/ensure_agent_hardlinks.sh" "${ensure_args[@]}"
