---
doc_type: plan
subsystem: ci
status: active
last_reviewed: 2026-03-31
---

# Agentic Evidence Assessor: System Architecture & Spec

## Overview

This document specifies the implementation of a "Shadow Mode Agentic Evidence Assessor" for ColdVox. A Gemini-powered CI agent runs on every PR to audit whether the PR provides empirical evidence for its material claims and detects semantic drift between code and documentation.

**Shadow Mode**: The assessor is non-blocking in Phase 1. It writes a Markdown report to the GitHub Step Summary. It does not comment on the PR, does not set a check status, and cannot block merges. It is advisory only.

## System Components

```
┌─────────────────────────────────────────────────────────────────┐
│  GitHub Actions: agentic-evidence-preview.yml                   │
│                                                                 │
│  Trigger: pull_request (opened, synchronize, ready_for_review)  │
│                                                                 │
│  Step 1: actions/checkout (fetch-depth: 0)                      │
│  Step 2: Fetch base branch ref explicitly                       │
│  Step 3: Generate git diff (base...HEAD), truncate at 2000 ln  │
│  Step 4: Collect docs index (ls docs/ recursive, head 100ln)   │
│  Step 5: Extract anchor docs (northstar.md, AGENTS.md)          │
│  Step 6: Compose full prompt (instructions + all context)       │
│  Step 7: npx @google/gemini-cli --model gemini-2.5-flash       │
│  Step 8: Write report → $GITHUB_STEP_SUMMARY                   │
└─────────────────────────────────────────────────────────────────┘
```

## GitHub Actions Configuration

### Permissions Required

```yaml
permissions:
  contents: read
  pull-requests: read
```

`contents: read` is required for checkout and to read file contents.
`pull-requests: read` is required to access PR metadata (title, body) via `github.event.pull_request`.

No write permissions are needed since the agent writes only to `$GITHUB_STEP_SUMMARY` (which is write-access controlled by the runner, not a permissions scope).

### Secrets Required

| Secret | Purpose |
|--------|---------|
| `GEMINI_API_KEY` | Authenticate requests to Google Gemini API |

The `GITHUB_TOKEN` is automatically provided by Actions and used implicitly (no explicit injection needed for read-only operations in Phase 1).

### Runner

`ubuntu-latest` (GitHub-hosted). The assessor does not need hardware access, GPU, or the self-hosted Fedora/Nobara runner. It runs fast general CI work only.

## Git Diff Strategy

### Why `fetch-depth: 0`

Without `fetch-depth: 0`, `actions/checkout` performs a shallow clone (depth 1). A shallow clone only contains the tip commit. Running `git diff origin/main...HEAD` on a shallow clone will fail with "fatal: no commits between" or produce an empty diff because `origin/main` has no common ancestor in the local history.

With `fetch-depth: 0`, the full history is available. The three-dot diff (`origin/main...HEAD`) produces exactly the commits added by this PR, which is what the agent needs.

### Base Branch Reference

In a PR context, `github.event.pull_request.base.ref` contains the target branch (e.g., `main`). The checkout action fetches this automatically. The diff command should use:

```bash
git diff "origin/${{ github.event.pull_request.base.ref }}...HEAD"
```

This is more robust than hardcoding `origin/main` because it handles PRs targeting release branches.

### Diff Truncation

Diffs larger than 2000 lines are truncated before passing to the model, with a note appended. This prevents token budget exhaustion and keeps latency acceptable. The truncation preserves the beginning of the diff (file headers and early hunks), which typically contain the most structurally significant changes.

## Prompt Architecture

The assessor operates in an **autonomous agentic mode** using Gemini CLI's YOLO approval mode. Instead of pre-gathering all context in bash and sending a massive prompt, the CI runner provides the PR title, body, and base branch as environment variables.

The agent is then invoked with:
`npx @google/gemini-cli --approval-mode=yolo -p "..."`

This allows the agent to:
- Use its tools to actively run `git diff origin/main...HEAD`.
- Search the workspace, read files, and explore documentation dynamically.
- Reason about the code changes in a much deeper way than a static prompt allows.
- Produce a highly context-aware Markdown report.

The system instructions for the agent live at `.github/prompts/evidence-assessor.md`. See that file for the full Chain-of-Thought instructions.

## Output Format

The agent must produce a Markdown report with these sections:

```markdown
## PR Evidence Assessment Report

**PR:** [title]
**Verdict:** EVIDENCE_PRESENT | EVIDENCE_WEAK | EVIDENCE_MISSING

### Material Claims Found
- Claim 1: [quoted text from PR description]
- Claim 2: ...

### Evidence Audit
| Claim | Evidence Found | Tier | Notes |
|-------|---------------|------|-------|
| ...   | ✅/⚠️/❌     | 1-5  | ...   |

### Semantic Drift Detected
- [Subsystem]: [description of drift, or "None detected"]

### Assessment Notes
[Brief reasoning, 3-5 sentences max]
```

The agent is instructed to cite ONLY evidence visible in the provided diff and docs. It must NOT hallucinate tests it did not see or infer evidence from general knowledge.

## Token Budget

| Item | Estimated Tokens |
|------|-----------------|
| Instructions (evidence-assessor.md) | ~800 |
| PR title + body | ~500 |
| git diff (truncated at 2000 lines) | ~5,000 |
| docs index (ls output) | ~200 |
| northstar.md excerpt | ~600 |
| AGENTS.md excerpt | ~600 |
| **Total Input** | **~7,700** |
| Output report | ~1,000 |

Gemini 2.5 Flash supports 1M token context. This implementation is well within budget.

## Failure Modes and Mitigations

| Failure | Mitigation |
|---------|-----------|
| `GEMINI_API_KEY` not set | Workflow step fails fast with clear error; PR proceeds unblocked |
| Gemini API rate limit | `continue-on-error: true` on the assessor job; report notes API failure |
| Diff too large (> 2000 lines) | Truncation with note; assessment may be incomplete |
| Agent produces malformed output | Report is written as-is to Step Summary; human reviewer sees raw output |
| PR has no description | Agent reports "no material claims found"; verdict is EVIDENCE_MISSING by default |

## Phase 2 Considerations (Future)

Phase 2 would gate merges on the assessor verdict. This requires:
- Setting a check status (requires `statuses: write` or `checks: write` permission)
- Defining the threshold (all EVIDENCE_MISSING claims require override)
- Adding a `pr-evidence-override` label bypass for intentional exceptions

Phase 2 is explicitly out of scope for this implementation. Phase 1 builds the evidence record to calibrate Phase 2 thresholds.
