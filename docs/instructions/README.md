# Agent Instructions

This directory contains instructions for AI agents working on the ColdVox project.

## Multi-Agent PR Fix Execution

### File: `multi-agent-pr-fix-prompt.md`

Complete prompt for a team of 3 specialized agents to execute the PR review fixes.

**Team Roles:**
- **Debug-Lead:** Issue identification, code fixes, compilation verification
- **Arch-Lead:** Architectural fixes, design reviews, code quality validation
- **Ops-Lead:** CI/CD, integration testing, coordination, logging

**What They'll Do:**
1. Fix Vosk CI checksum (affects all PRs)
2. Add missing module declarations (#126)
3. Fix compilation errors (#129)
4. Resolve circular dependencies (#124/#127)
5. Fix hardcoded device issue (#130)
6. Analyze PR #134 for duplicate work
7. Generate detailed execution log with reviews

**Expected Output:**
- Commits pushed to PR branches with fixes
- Detailed execution log at `docs/review/fix-execution-log-2025-10-08.md`
- PR readiness assessment
- Recommendations for next steps

### Quick Start

**For Claude Code:**
```bash
# From project root
cat docs/instructions/multi-agent-pr-fix-prompt.md
# Copy the entire prompt and provide to agent team
```

**For Other AI Systems:**
Copy the entire contents of `multi-agent-pr-fix-prompt.md` and provide to your multi-agent system, ensuring:
- Agents have git access
- Agents can checkout PR branches
- Agents can push commits
- Agents can run cargo commands

### Prerequisites

- ✅ Review summary exists: `docs/review/pr-review-summary-2025-10-08.md`
- ✅ Working directory: `/home/coldaine/Desktop/ColdVoxRefactorTwo/ColdVox`
- ✅ Git access configured
- ✅ Rust toolchain available (`cargo` command)
- ✅ PR branches exist on remote

### Expected Timeline

**Estimated Duration:** 8-12 hours of agent work (can run in parallel with human review)

**Breakdown:**
- Batch 1 (CI): 1-2 hours
- Batch 2 (Modules): 30-45 min
- Batch 3 (Circular Dep): 20-30 min
- Batch 4 (Portability): 30-45 min
- Batch 5 (Validation): 1-2 hours

### Monitoring Progress

Agents will create and update: `docs/review/fix-execution-log-2025-10-08.md`

Check this file for:
- Real-time progress updates
- Task assignments and completions
- Review approvals
- Integration test results
- Blockers or issues

### After Completion

Review the execution log and verify:
1. All 5 blocking issues addressed
2. All fixes have peer review approval
3. PR readiness assessment provided
4. PR #134 disposition decided

Then proceed with merge strategy as recommended.

## Related Documentation

- **Review Summary:** `docs/review/pr-review-summary-2025-10-08.md` (59KB, 40+ pages)
- **Execution Log:** `docs/review/fix-execution-log-2025-10-08.md` (created by agents)
- **Project Status:** `docs/PROJECT_STATUS.md`
