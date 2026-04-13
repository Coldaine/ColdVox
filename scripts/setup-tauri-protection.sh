#!/usr/bin/env bash
set -euo pipefail

repo="$(gh repo view --json nameWithOwner -q '.nameWithOwner')"

gh api "repos/${repo}/branches/tauri-base/protection" --method PUT \
  --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "Repo Integrity Checks",
      "Check (stable)",
      "Check (1.90)",
      "Lint & Format",
      "Test"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": false
}
EOF
