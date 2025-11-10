#!/usr/bin/env bash
set -euo pipefail

# Apply a GitHub label (default: "jules") to a list of issues in this repo.
# Requires: GitHub CLI (gh) authenticated with repo:write permissions.
#
# Usage:
#   ./scripts/label_jules.sh [owner/repo] [label] [color] [description] [assignee]
#
# Examples:
#   ./scripts/label_jules.sh Coldaine/ColdVox jules BFD4F2 "Owned by Jules"
#   ./scripts/label_jules.sh Coldaine/ColdVox jules BFD4F2 "Owned by Jules" jules-gh-handle
#

REPO="${1:-Coldaine/ColdVox}"
LABEL="${2:-jules}"
COLOR="${3:-BFD4F2}"
DESC="${4:-Owned by Jules}"
ASSIGNEE="${5:-}"

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: GitHub CLI 'gh' not found. Install from https://cli.github.com/ and authenticate with 'gh auth login'." >&2
  exit 1
fi

echo "Repo:      $REPO"
echo "Label:     $LABEL"
echo "Color:     $COLOR"
echo "Desc:      $DESC"
if [[ -n "$ASSIGNEE" ]]; then
  echo "Assignee:  $ASSIGNEE"
fi
echo

# Create the label if it doesn't exist
if ! gh label list -R "$REPO" --limit 200 | awk '{print $1}' | grep -qx "$LABEL"; then
  echo "Creating label '$LABEL'..."
  gh label create "$LABEL" -R "$REPO" --color "$COLOR" --description "$DESC"
else
  echo "Label '$LABEL' already exists."
fi

# Issues we marked in the master plan for #jules ownership
ISSUES=(
  171
  224 222 223
  40 162 173
  42 44 45 46 47
  208 209 210 211 212 213 215
  226 228 229 230
)

for i in "${ISSUES[@]}"; do
  echo "Applying label '$LABEL' to #$i..."
  gh issue edit "$i" -R "$REPO" --add-label "$LABEL"
done

if [[ -n "$ASSIGNEE" ]]; then
  for i in "${ISSUES[@]}"; do
    echo "Adding assignee '$ASSIGNEE' to #$i..."
    gh issue edit "$i" -R "$REPO" --add-assignee "$ASSIGNEE"
  done
fi

echo "Done."
