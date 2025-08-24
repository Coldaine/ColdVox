#!/usr/bin/env bash
set -euo pipefail

UPSTREAM_REPO=${UPSTREAM_REPO:-"https://github.com/Coldaine/ColdVox-voice_activity_detector"}
WORKDIR=$(mktemp -d)
trap 'rm -rf "${WORKDIR}"' EXIT

echo "Cloning upstream: ${UPSTREAM_REPO}"
git clone --depth 1 "${UPSTREAM_REPO}" "${WORKDIR}/vad-upstream" >/dev/null 2>&1 || {
  echo "Clone failed. Check network or repo URL." >&2
  exit 1
}

pushd "${WORKDIR}/vad-upstream" >/dev/null
if git remote get-url upstream >/dev/null 2>&1; then
  git fetch upstream main --depth 1 >/dev/null 2>&1 || true
fi
popd >/dev/null

diff_file="${HOME}/vad-upstream-changes.diff"

echo "Generating diff vs. current vendored copy..."

git --no-pager diff --no-index \
  "${WORKDIR}/vad-upstream" \
  "$(cd "$(dirname "$0")/.." && pwd)/Forks/ColdVox-voice_activity_detector" \
  > "${diff_file}" || true

wc -l "${diff_file}"
echo "Review changes in ${diff_file}"
