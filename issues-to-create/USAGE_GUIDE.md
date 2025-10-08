# Usage Guide for Issue Templates

This guide explains how to create GitHub issues from the templates in this directory.

## Overview

This directory contains 20 pre-written issue templates that document problems in the `crates/coldvox-text-injection` codebase. These were split from a single monolithic issue to make the work more manageable.

## Files in This Directory

- **P0-01 through P0-07**: Critical correctness and reliability issues (7 files)
- **P1-08 through P1-15**: Performance and maintainability improvements (8 files)
- **P2-16 through P2-20**: Structure, testing, and documentation enhancements (5 files)
- **README.md**: Introduction and manual creation instructions
- **SUMMARY.md**: Comprehensive analysis and statistics
- **INDEX.md**: Quick reference table with dependencies
- **USAGE_GUIDE.md**: This file
- **create-issues.sh**: Automated script for bulk issue creation

## Method 1: Automated Creation (Recommended)

Use the provided shell script to create all issues at once:

```bash
cd issues-to-create

# Dry run first to preview what will be created
./create-issues.sh dry-run

# If everything looks good, create the issues
./create-issues.sh
```

**Prerequisites:**
- GitHub CLI (`gh`) must be installed: https://cli.github.com/
- You must be authenticated: `gh auth login`
- You must have issue creation permissions on the repository

**What the script does:**
1. Extracts title and labels from each template's frontmatter
2. Removes frontmatter and uses the body as issue content
3. Creates issues in priority order (P0, then P1, then P2)
4. Provides a summary at the end

## Method 2: Manual Creation

If you prefer manual control or don't have CLI access:

### Step-by-Step Process

1. **Open GitHub Issues page:**
   ```
   https://github.com/Coldaine/ColdVox/issues/new
   ```

2. **For each template file:**
   
   a. Open the file (e.g., `P0-01-cooldowns-not-per-app.md`)
   
   b. Copy the title from the frontmatter:
   ```yaml
   title: "[P0] Cooldowns not per-app: is_in_cooldown() checks any app"
   ```
   
   c. Copy the content below the second `---` line (skip frontmatter)
   
   d. Paste into GitHub issue editor
   
   e. Add labels from the frontmatter:
   ```yaml
   labels: ["bug", "priority:P0", "component:text-injection"]
   ```

3. **Repeat** for all 20 files

### Recommended Order

Create issues in this order for logical dependency tracking:
1. P0 issues (most critical)
2. P1 issues (performance)
3. P2 issues (quality)

## Method 3: Bulk Import via GitHub API

For programmatic creation, you can use the GitHub REST API:

```bash
# Example for one issue
TITLE="[P0] Cooldowns not per-app: is_in_cooldown() checks any app"
BODY=$(sed -n '/^---$/,/^---$/!p' P0-01-cooldowns-not-per-app.md | sed '1,/^---$/d')

gh api \
  --method POST \
  -H "Accept: application/vnd.github+json" \
  /repos/Coldaine/ColdVox/issues \
  -f title="$TITLE" \
  -f body="$BODY" \
  -f labels='["bug","priority:P0","component:text-injection"]'
```

See `create-issues.sh` for a complete implementation.

## After Creating Issues

### 1. Add to Project Board (Optional)

Organize the issues on a GitHub Project board:

```bash
# Create a project
gh project create --owner Coldaine --title "Text Injection Refactoring"

# Add issues to project (requires project number)
gh project item-add <project-number> --owner Coldaine --url <issue-url>
```

### 2. Create Milestones (Optional)

Group issues into milestones for sprint planning:

```bash
gh milestone create "Text Injection - Sprint 1" --repo Coldaine/ColdVox
gh milestone create "Text Injection - Sprint 2" --repo Coldaine/ColdVox
# ... etc
```

### 3. Assign Issues

Assign issues to developers:

```bash
gh issue edit <issue-number> --add-assignee <github-username>
```

### 4. Link Dependencies

Reference related issues in comments:
- "Depends on #123"
- "Blocks #456"
- "Related to #789"

### 5. Update Issue Descriptions

As work progresses, you may want to:
- Add more detail based on investigation
- Update priority if needed
- Add links to PRs that address the issue
- Close issues as they're completed

## Issue Tracking Workflow

### For Each Issue:

1. **Created** - Issue exists with label and priority
2. **Triaged** - Assigned to developer, added to sprint
3. **In Progress** - Developer working on fix
4. **In Review** - PR submitted, awaiting review
5. **Done** - PR merged, issue closed

### Example Labels:

- Priority: `priority:P0`, `priority:P1`, `priority:P2`
- Status: `in-progress`, `blocked`, `needs-review`
- Type: `bug`, `enhancement`, `refactor`, `performance`
- Component: `component:text-injection`

## Tips for Success

### 1. Don't Create All at Once
Consider creating issues in phases:
- **Phase 1**: P0 issues only (7 issues)
- **Phase 2**: P1 issues after P0 work starts (8 issues)
- **Phase 3**: P2 issues when approaching quality phase (5 issues)

### 2. Customize Labels
Adjust the labels to match your repository's conventions:
- Change `priority:P0` to `priority:critical` if you prefer
- Add status labels like `awaiting-triage`
- Add domain labels like `async`, `performance`, `testing`

### 3. Reference the Original Issue
In each created issue, consider adding:
```markdown
> Split from original issue #XXX: "Crazy enormous issue needs to be parceled out"
```

### 4. Update Templates Before Creating
Review each template and:
- Add any new findings since templates were created
- Update code references if files have moved
- Add links to related issues if they already exist

### 5. Track Progress
Use the INDEX.md file to track which issues have been:
- Created (✓)
- Assigned (→ @username)
- Started (▶)
- Completed (✅)

Example:
```markdown
- [✓→@alice▶] P0-01: Cooldowns not per-app
- [✓→@bob] P0-02: "unknown_app" hardcoded
- [✓] P0-03: Metrics mutex poisoning
```

## Troubleshooting

### "gh command not found"
Install GitHub CLI: https://cli.github.com/

### "Authentication required"
Run: `gh auth login`

### "Permission denied"
You need write access to the repository to create issues.

### "API rate limit exceeded"
Wait an hour or authenticate with a token that has higher limits.

### Script doesn't work
Check that:
- You're in the `issues-to-create` directory
- The script is executable: `chmod +x create-issues.sh`
- All markdown files exist: `ls P*.md`

## Getting Help

If you encounter issues:
1. Check the README.md for basic instructions
2. Review SUMMARY.md for context on the issues
3. Consult INDEX.md for the dependency graph
4. Open a discussion or issue in the repository

## Cleanup

After all issues are created and work is complete:

1. **Archive this directory** (optional):
   ```bash
   git mv issues-to-create issues-to-create.archive
   ```

2. **Or keep for reference** - It serves as historical documentation

3. **Update the original issue** to reference the new issues:
   ```markdown
   This issue has been split into 20 separate issues: #XXX through #XXX
   See the issues-to-create/ directory for details.
   ```

---

**Questions?** Contact the repository maintainers or open a discussion.

*Last updated: 2025-10-08*
