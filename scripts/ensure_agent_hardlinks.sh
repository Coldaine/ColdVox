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

src="$repo_root/CLAUDE.md"

dst1="$repo_root/.github/copilot-instructions.md"
dst2="$repo_root/.kilocode/rules/agents.md"
dst3="$repo_root/AGENTS.md"
dst4="$repo_root/GEMINI.md"

if [[ ! -f "$src" ]]; then
  echo "error: missing $src" >&2
  exit 1
fi

mkdir -p "$(dirname "$dst1")" "$(dirname "$dst2")"

say() {
  if [[ "$quiet" -eq 0 ]]; then
    echo "$@"
  fi
}

inode_of() {
  local path="$1"
  if stat -c '%i' "$path" >/dev/null 2>&1; then
    stat -c '%i' "$path"
    return 0
  fi
  if stat -f '%i' "$path" >/dev/null 2>&1; then
    stat -f '%i' "$path"
    return 0
  fi
  return 1
}

link_count_of() {
  local path="$1"
  if stat -c '%h' "$path" >/dev/null 2>&1; then
    stat -c '%h' "$path"
    return 0
  fi
  if stat -f '%l' "$path" >/dev/null 2>&1; then
    stat -f '%l' "$path"
    return 0
  fi
  return 1
}

link_or_symlink_or_copy() {
  local dst="$1"

  # Remove symlinks explicitly; ln -f replaces regular files but not all symlinks reliably.
  if [[ -L "$dst" ]]; then
    rm -f "$dst"
  fi

  # 1) Prefer hardlink.
  if ln -f "$src" "$dst" 2>/dev/null; then
    return 0
  fi

  # 2) Fallback to symlink (works across filesystems).
  local rel
  rel="$(python - <<'PY'
import os
import sys
src = sys.argv[1]
dst = sys.argv[2]
print(os.path.relpath(src, os.path.dirname(dst)))
PY
"$src" "$dst" 2>/dev/null || true)"

  if [[ -n "$rel" ]]; then
    if ln -sf "$rel" "$dst" 2>/dev/null; then
      echo "warning: could not hardlink $dst; created symlink instead" >&2
      return 0
    fi
  fi

  # 3) Last resort: copy contents.
  cp -f "$src" "$dst"
  echo "warning: could not hardlink or symlink $dst; copied contents instead" >&2
  return 0
}

is_hardlinked_pair() {
  local a="$1"
  local b="$2"

  local ia ib
  if ia="$(inode_of "$a")" && ib="$(inode_of "$b")"; then
    [[ "$ia" == "$ib" ]]
    return $?
  fi

  # Fallback: inode via ls -i (portable enough for typical *nix).
  ia="$(ls -di "$a" 2>/dev/null | awk '{print $1}')" || return 1
  ib="$(ls -di "$b" 2>/dev/null | awk '{print $1}')" || return 1
  [[ "$ia" == "$ib" ]]
}

ensure_pair() {
  local dst="$1"

  link_or_symlink_or_copy "$dst"

  if ! cmp -s "$src" "$dst"; then
    echo "error: $dst does not match $src" >&2
    exit 1
  fi

  if ! is_hardlinked_pair "$src" "$dst"; then
    if [[ "$require_hardlink" -eq 1 ]]; then
      echo "error: $dst is not hardlinked to $src (same contents, different inode)" >&2
      echo "hint: ensure both files are on the same filesystem; rerun scripts/ensure_agent_hardlinks.sh" >&2
      exit 2
    fi

    echo "warning: $dst is not hardlinked to $src (content matches)" >&2
  fi
}

ensure_pair "$dst1"
ensure_pair "$dst2"
ensure_pair "$dst3"
ensure_pair "$dst4"

src_inode="$(inode_of "$src" 2>/dev/null || true)"
src_links="$(link_count_of "$src" 2>/dev/null || true)"

if [[ -n "$src_inode" && -n "$src_links" ]]; then
  say "AGENTS mirrors hardlinked (inode=$src_inode links=$src_links)"
else
  say "AGENTS mirrors hardlinked"
fi
